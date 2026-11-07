//! Data files: where they live, how they are downloaded and verified.
//!
//! Downloaded (release assets, gzip-compressed, sha256 checked after decompression):
//!   model.frcm   the energy model (66 MB)
//!   lexicon.tsv  the French lexicon (20 MB; 515,637 forms with lemma and morphological tags)
//! Built locally from lexicon.tsv, then checked against known sha256 values:
//!   lex/         the rule checker's FST lexicon (words.fst, recs.bin, gen.fst, lemmas.bin, tags.txt; 7 MB)
//!   lexicon.tsv.lem.frclex   the model's lexicon tables (72 MB cache, written at the first start)
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Release that holds the data files (override with VAUGELAS_DATA_URL, a directory URL ending in `/`).
pub const DATA_URL: &str = "https://github.com/Elvis-codeur/vaugelas/releases/download/data-1/";

pub struct Asset { pub name: &'static str, pub sha256: &'static str, pub size: u64 }

pub const ASSETS: [Asset; 2] = [
    Asset { name: "model.frcm", sha256: "ea9f56253fdd4e1af96fdc27cc846de31dffa4f5b70e139468fc542598df36e3", size: 66_189_897 },
    Asset { name: "lexicon.tsv", sha256: "4cbad5eec76835dafc19a6863dc980f576618cc1b2d54828fa52186b7cf657d8", size: 20_072_620 },
];

/// sha256 of the FST lexicon files built from lexicon.tsv (identical to fr-check's frc-build output)
pub const LEX_FILES: [(&str, &str); 5] = [
    ("gen.fst", "0b899d6e5078b320043ed9174666743657240f27468c077c4df18a7ca9caa9e0"),
    ("lemmas.bin", "6d8507df387f95c39039616428eba37977a08db74f946af4a7140612fac7451f"),
    ("recs.bin", "443ddcbb47e76556610ab58c810818dbf82414087bdc874dc0395087ad0c4175"),
    ("tags.txt", "0079d5c88b31508f6383a515eee33c43632129b07d7d918216a0b5f2b96b499a"),
    ("words.fst", "bc0a7cfe2360a68eb5b610956a47c7ac26c0c7268fcf06768bc5f51adfd02190"),
];

/// The data directory: --data DIR, else VAUGELAS_DATA, else the platform's per-user data directory.
pub fn data_dir(explicit: Option<&str>) -> PathBuf {
    if let Some(d) = explicit { return PathBuf::from(d); }
    if let Some(d) = std::env::var_os("VAUGELAS_DATA") { return PathBuf::from(d); }
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        if let Some(d) = env("LOCALAPPDATA") { return d.join("vaugelas"); }
    } else if cfg!(target_os = "macos") {
        if let Some(h) = env("HOME") { return h.join("Library/Application Support/vaugelas"); }
    } else {
        if let Some(d) = env("XDG_DATA_HOME") { return d.join("vaugelas"); }
        if let Some(h) = env("HOME") { return h.join(".local/share/vaugelas"); }
    }
    PathBuf::from("vaugelas-data")
}

pub struct Files { pub model: PathBuf, pub lexicon: PathBuf, pub lex_dir: PathBuf }

impl Files {
    pub fn in_dir(d: &Path) -> Files { Files { model: d.join("model.frcm"), lexicon: d.join("lexicon.tsv"), lex_dir: d.join("lex") } }
    pub fn complete(&self) -> bool {
        self.model.is_file() && self.lexicon.is_file() && LEX_FILES.iter().all(|(f, _)| self.lex_dir.join(f).is_file())
    }
}

pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop { let n = f.read(&mut buf)?; if n == 0 { break; } h.update(&buf[..n]); }
    Ok(hex(&h.finalize()))
}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

/// Write `reader` (gzip-decompressed when `gz`) to `dest` through a temporary file, checking the sha256.
fn install_stream(mut reader: Box<dyn Read>, gz: bool, a: &Asset, dest: &Path, progress: bool) -> Result<(), String> {
    if gz { reader = Box::new(flate2::read::GzDecoder::new(reader)); }
    let tmp = dest.with_extension("part");
    let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let (mut done, mut shown) = (0u64, 0u64);
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("{}: {e}", a.name))?;
        if n == 0 { break; }
        h.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if progress && done - shown >= 4 << 20 { shown = done; eprint!("\r  {}: {} / {} MB", a.name, done >> 20, a.size >> 20); }
    }
    out.flush().map_err(|e| e.to_string())?; drop(out);
    if progress { eprintln!("\r  {}: {} MB          ", a.name, done >> 20); }
    let got = hex(&h.finalize());
    if got != a.sha256 {
        std::fs::remove_file(&tmp).ok();
        return Err(format!("{}: sha256 {got} does not match the expected {} (corrupted or wrong file)", a.name, a.sha256));
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Install every data file into `dir`: from a local directory (`from`, holding `NAME` or `NAME.gz`) or by download.
/// Files already present with the right sha256 are kept. Then builds the rule checker's lexicon and checks it.
pub fn install(dir: &Path, from: Option<&Path>, force: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let base = std::env::var("VAUGELAS_DATA_URL").unwrap_or_else(|_| DATA_URL.to_string());
    eprintln!("data directory: {}", dir.display());
    for a in &ASSETS {
        let dest = dir.join(a.name);
        if !force && dest.is_file() && sha256_file(&dest).map_err(|e| e.to_string())? == a.sha256 {
            eprintln!("  {}: present, sha256 ok", a.name); continue;
        }
        match from {
            Some(src) => {
                let (p, gz) = if src.join(a.name).is_file() { (src.join(a.name), false) } else { (src.join(format!("{}.gz", a.name)), true) };
                let f = std::fs::File::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                eprintln!("  {}: copying from {}", a.name, p.display());
                install_stream(Box::new(f), gz, a, &dest, false)?;
            }
            None => {
                let url = format!("{base}{}.gz", a.name);
                eprintln!("  {}: downloading {url}", a.name);
                let resp = ureq::get(&url).call().map_err(|e| format!("download failed ({url}): {e}"))?;
                install_stream(Box::new(resp.into_reader()), true, a, &dest, true)?;
            }
        }
        eprintln!("  {}: sha256 ok", a.name);
    }
    build_lex(dir)
}

/// Build `lex/` from lexicon.tsv (skipped when present and correct) and check every file's sha256.
pub fn build_lex(dir: &Path) -> Result<(), String> {
    let f = Files::in_dir(dir);
    let ok = |d: &Path| LEX_FILES.iter().all(|(n, h)| sha256_file(&d.join(n)).map(|x| x == *h).unwrap_or(false));
    if ok(&f.lex_dir) { eprintln!("  lex/: present, sha256 ok"); return Ok(()); }
    eprintln!("  lex/: building the rule checker's lexicon from lexicon.tsv");
    vaugelas::rules::build::build_lexicon(&f.lexicon, &f.lex_dir).map_err(|e| format!("building lex/: {e}"))?;
    if !ok(&f.lex_dir) { return Err("lex/: built files do not have the expected sha256".into()); }
    eprintln!("  lex/: built, sha256 ok");
    Ok(())
}
