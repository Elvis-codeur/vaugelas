//! Binary cache of the built `Lexicon` + `Natural` tables (issue #18: startup was ~4 s and ~680 MB, all spent
//! rebuilding these tables from lexicon.tsv at every start).
//!
//! The cache holds the finished tables, so loading it is a plain read: no parsing of tags, ranking or suffix
//! counting. It is tied to its source: the header records the format version, the lemma flag and the size and
//! modification time of lexicon.tsv; any difference means the cache is ignored and rebuilt.
//! Format: little-endian; strings and lists are a u32 length followed by their bytes / items.
use crate::text::characterize::{KeyInfo, Lexicon};
use crate::text::fxhash::FxMap;
use crate::text::natural::{Natural, Reading};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"FRCLEX\0\x01";

/// Identity of the source file: (size, mtime in ns).
pub fn source_id(tsv: &Path) -> io::Result<(u64, u128)> {
    let m = std::fs::metadata(tsv)?;
    let t = m.modified()?.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    Ok((m.len(), t))
}

/// Default cache path: next to the TSV, `lexicon.tsv` -> `lexicon.tsv.lem.frclex` (or `.nolem.frclex`).
pub fn default_path(tsv: &Path, lemma: bool) -> PathBuf {
    let mut s = tsv.as_os_str().to_owned();
    s.push(if lemma { ".lem.frclex" } else { ".nolem.frclex" });
    PathBuf::from(s)
}

struct W<T: Write>(T);
impl<T: Write> W<T> {
    fn u8(&mut self, v: u8) -> io::Result<()> { self.0.write_all(&[v]) }
    fn u32(&mut self, v: u32) -> io::Result<()> { self.0.write_all(&v.to_le_bytes()) }
    fn len(&mut self, v: usize) -> io::Result<()> { self.u32(u32::try_from(v).expect("table too large")) }
    fn str(&mut self, s: &str) -> io::Result<()> { self.len(s.len())?; self.0.write_all(s.as_bytes()) }
    fn ids(&mut self, v: &[u32]) -> io::Result<()> { self.len(v.len())?; for x in v { self.u32(*x)?; } Ok(()) }
    fn strs(&mut self, v: &[String]) -> io::Result<()> { self.len(v.len())?; for s in v { self.str(s)?; } Ok(()) }
}

struct R<'a> { b: &'a [u8], i: usize }
fn bad() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "truncated or corrupt lexicon cache") }
impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let s = self.b.get(self.i..self.i + n).ok_or_else(bad)?; self.i += n; Ok(s)
    }
    fn u8(&mut self) -> io::Result<u8> { Ok(self.take(1)?[0]) }
    fn u32(&mut self) -> io::Result<u32> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn len(&mut self) -> io::Result<usize> { Ok(self.u32()? as usize) }
    fn str(&mut self) -> io::Result<String> {
        let n = self.len()?; std::str::from_utf8(self.take(n)?).map(str::to_owned).map_err(|_| bad())
    }
    fn ids(&mut self) -> io::Result<Vec<u32>> {
        let n = self.len()?; let raw = self.take(n.checked_mul(4).ok_or_else(bad)?)?;
        Ok(raw.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
    }
    fn strs(&mut self) -> io::Result<Vec<String>> { let n = self.len()?; (0..n).map(|_| self.str()).collect() }
}

fn write_map_ids<T: Write, V: AsRef<[u32]>>(w: &mut W<T>, m: &FxMap<String, V>) -> io::Result<()> {
    let mut ks: Vec<&String> = m.keys().collect(); ks.sort(); // deterministic file
    w.len(ks.len())?;
    for k in ks { w.str(k)?; w.ids(m[k].as_ref())?; }
    Ok(())
}
fn read_map_ids(r: &mut R) -> io::Result<FxMap<String, Vec<u32>>> {
    let n = r.len()?; let mut m = FxMap::default(); m.reserve(n);
    for _ in 0..n { let k = r.str()?; m.insert(k, r.ids()?); }
    Ok(m)
}

/// Write the tables to `path` (atomically: temp file + rename).
pub fn save(path: &Path, src: (u64, u128), lemma: bool, lex: &Lexicon, nat: &Natural) -> io::Result<()> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    {
        let mut w = W(io::BufWriter::new(std::fs::File::create(&tmp)?));
        w.0.write_all(MAGIC)?; w.u8(lemma as u8)?;
        w.0.write_all(&src.0.to_le_bytes())?; w.0.write_all(&src.1.to_le_bytes())?;
        // Lexicon (keys_json is derived from keys on load)
        w.strs(&lex.keys)?;
        w.len(lex.info.len())?;
        for k in &lex.info {
            w.u8(k.det as u8 | (k.pron as u8) << 1 | (k.propn as u8) << 2 | (k.has_pron as u8) << 3 | (k.n_or_a as u8) << 4
                 | (k.open_stripped.is_some() as u8) << 5)?;
            w.u32(k.open_stripped.unwrap_or(0))?;
            w.strs(&k.det_n)?;
        }
        write_map_ids(&mut w, &lex.read)?;
        write_map_ids(&mut w, &lex.suffix)?;
        for k in [lex.k_punct, lex.k_num, lex.k_deictic, lex.k_propn, lex.k_n, lex.k_a] { w.u32(k)?; }
        let mut pn: Vec<(&String, &u32)> = lex.k_propn_n.iter().collect(); pn.sort();
        w.len(pn.len())?; for (k, v) in pn { w.str(k)?; w.u32(*v)?; }
        // Natural
        w.len(nat.readings.len())?;
        for r in &nat.readings {
            w.str(&r.form)?; w.u32(r.lem)?; w.u32(r.pos as u32)?; w.u8(r.g)?; w.u8(r.n)?; w.u8(r.p)?;
            w.0.write_all(&r.mo.to_le_bytes())?;
        }
        w.strs(&nat.lemmas)?;
        write_map_ids(&mut w, &nat.read)?;
        let mut bl: Vec<(&u32, &Vec<u32>)> = nat.bylem.iter().collect(); bl.sort();
        w.len(bl.len())?; for (k, v) in bl { w.u32(*k)?; w.ids(v)?; }
        w.0.write_all(b"END\0")?;
        w.0.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

/// Read the tables back; `Ok(None)` when the cache is for another source file or lemma setting.
pub fn load(path: &Path, src: (u64, u128), lemma: bool) -> io::Result<Option<(Lexicon, Natural)>> {
    let mut b = Vec::new(); std::fs::File::open(path)?.read_to_end(&mut b)?;
    let mut r = R { b: &b, i: 0 };
    if r.take(8)? != MAGIC { return Ok(None); }
    if r.u8()? != lemma as u8 { return Ok(None); }
    let size = u64::from_le_bytes(r.take(8)?.try_into().unwrap());
    let mtime = u128::from_le_bytes(r.take(16)?.try_into().unwrap());
    if (size, mtime) != src { return Ok(None); }
    let keys = r.strs()?;
    let keys_json = keys.iter().map(|k| { let mut o = String::new(); crate::text::pyjson::write_str(&mut o, k); o }).collect();
    let n = r.len()?; let mut info = Vec::with_capacity(n);
    for _ in 0..n {
        let f = r.u8()?; let os = r.u32()?;
        info.push(KeyInfo { det: f & 1 != 0, pron: f & 2 != 0, propn: f & 4 != 0, has_pron: f & 8 != 0, n_or_a: f & 16 != 0,
                            open_stripped: (f & 32 != 0).then_some(os), det_n: r.strs()? });
    }
    let read = read_map_ids(&mut r)?.into_iter().map(|(k, v)| (k, v.into_boxed_slice())).collect();
    let suffix = read_map_ids(&mut r)?;
    let mut k6 = [0u32; 6]; for k in k6.iter_mut() { *k = r.u32()?; }
    let n = r.len()?; let mut k_propn_n = FxMap::default();
    for _ in 0..n { let k = r.str()?; k_propn_n.insert(k, r.u32()?); }
    let lex = Lexicon { keys, keys_json, info, read, suffix, k_punct: k6[0], k_num: k6[1], k_deictic: k6[2], k_propn: k6[3],
                        k_n: k6[4], k_a: k6[5], k_propn_n };
    let n = r.len()?; let mut readings = Vec::with_capacity(n);
    for _ in 0..n {
        let form = r.str()?; let lem = r.u32()?; let pos = char::from_u32(r.u32()?).ok_or_else(bad)?;
        let (g, nn, p) = (r.u8()?, r.u8()?, r.u8()?); let mo = u16::from_le_bytes(r.take(2)?.try_into().unwrap());
        readings.push(Reading { form, lem, pos, g, n: nn, p, mo });
    }
    let lemmas = r.strs()?;
    let nread = read_map_ids(&mut r)?;
    let n = r.len()?; let mut bylem = FxMap::default();
    for _ in 0..n { let k = r.u32()?; bylem.insert(k, r.ids()?); }
    if r.take(4)? != b"END\0" || r.i != b.len() { return Err(bad()); }
    Ok(Some((lex, Natural { readings, lemmas, read: nread, bylem })))
}

/// Load from the cache when it matches `tsv`, else build from the TSV and write the cache (best effort: a read-only
/// directory only costs the rebuild). `cache` = None uses `default_path`. Returns (lexicon, natural, came from cache).
pub fn load_or_build(tsv: &Path, lemma: bool, cache: Option<&Path>) -> io::Result<(Lexicon, Natural, bool)> {
    let src = source_id(tsv)?;
    let path = cache.map(Path::to_path_buf).unwrap_or_else(|| default_path(tsv, lemma));
    match load(&path, src, lemma) {
        Ok(Some((l, n))) => return Ok((l, n, true)),
        Ok(None) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("lexicon cache {}: {e}; rebuilding", path.display()),
    }
    let text = crate::text::pystr::universal_newlines(std::fs::read_to_string(tsv)?);
    let (lex, nat) = rayon::join(|| Lexicon::from_tsv_with(&text, lemma), || Natural::from_tsv(&text));
    drop(text);
    if let Err(e) = save(&path, src, lemma, &lex, &nat) {
        eprintln!("lexicon cache not written ({}: {e}); next start rebuilds again", path.display());
    }
    Ok((lex, nat, false))
}
