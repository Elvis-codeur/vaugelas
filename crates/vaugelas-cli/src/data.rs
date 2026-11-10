//! Data files: where they live, how they are installed and verified.
//!
//! Installed from the data release (gzip-compressed, sha256 checked after decompression):
//!   model.frcm        the energy model H1 (78 MB)
//!   lexicon.tsv       the French lexicon (20 MB; 515,637 forms with lemma and morphological tags)
//!   resources.tar.gz  the model's corpus tables, unpacked into resources/ (interp/ + ngram/, 26 MB; every file checked)
//! Built locally from lexicon.tsv, then checked against known sha256 values:
//!   lex/              the rule checker's FST lexicon (words.fst, recs.bin, gen.fst, lemmas.bin, tags.txt; 7 MB)
//!   lexicon.tsv.lem.frclex   the model's lexicon tables (72 MB cache, written at the first start)
//! A finished install writes `DATA_SET` into `data-set.txt`; another value there (an older install) means "install again"
//! (files already correct are kept).
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Name of this set of data files; also the tag of the GitHub release that holds them.
pub const DATA_SET: &str = "data-2";

/// Release that holds the data files (override with VAUGELAS_DATA_URL, a directory URL ending in `/`).
pub const DATA_URL: &str = "https://github.com/Elvis-codeur/vaugelas/releases/download/data-2/";

/// One data file: its name, and the sha256 and size of its uncompressed content.
pub struct Asset { pub name: &'static str, pub sha256: &'static str, pub size: u64 }

pub const MODEL: Asset = Asset { name: "model.frcm", sha256: "87be81fb4deda34a768a4a4c636b7059e94e996434beb0b292a0655a53b106d2", size: 77_546_586 };
pub const LEXICON: Asset = Asset { name: "lexicon.tsv", sha256: "4cbad5eec76835dafc19a6863dc980f576618cc1b2d54828fa52186b7cf657d8", size: 20_072_620 };

/// The archive that holds `RESOURCE_FILES` (paths relative to `resources/`).
pub const RESOURCES_ARCHIVE: &str = "resources.tar.gz";

/// sha256 of every file of resources/ (the corpus tables of training-design build/interp and build/ngram)
pub const RESOURCE_FILES: [(&str, &str); 11] = [
    ("interp/capstats.json", "42f315db7eab23a0222efe195ff1afd83774813ac5f8602a4defe1b0f49f7730"),
    ("interp/ctx_bigrams.json", "cc0961369de71b4751a57bba1b484025d20fc32a181151361f0a24f3091441e5"),
    ("interp/ctx_trigrams.json", "d526422ce9202219f9a38813bcc83713f4a4a5ee81c330e801e7149183b62a93"),
    ("interp/cw_pairs.json", "0af3dc76f75404de1dc1256b66a3f336b57447325cc04ae4e6431c811f8df179"),
    ("interp/homophones_lex_1.0.json", "d53858deb1f64a1aed4ce56d2cb629c830569c946cf54f7c8bc9a2b9267ac8ab"),
    ("interp/homophones_lex_1.0.phon.json", "825038156134a1961ea85167690ec21cff09ddddfc9f8436872acba973bfcad0"),
    ("interp/mined_pairs.json", "56b2f6dc8efe3b642bed2289281dab5c97db4bdba1c5f2947a1b8f9e28acfd87"),
    ("interp/nearhom.json", "76b17701ba8f58d5a097aa5c3f7344815f1459262cdb2d4003f4404ab57e5e27"),
    ("ngram/bigrams.json", "d0288cc03b5279a9b57c5f4b103a1f1ae95827b532499092533e539d03a7f5a7"),
    ("ngram/capstats2.json", "d15184e04b26f1dea9df92e2e7af0a3b2df6bb993304c69aa0ddf052e651782c"),
    ("ngram/unigrams.json", "ca0450d097c1182b71ef8ec920f00db7a14bfead702b693b035ecc84a173c677"),
];

/// sha256 of the FST lexicon files built from lexicon.tsv (identical to fr-check's frc-build output)
pub const LEX_FILES: [(&str, &str); 5] = [
    ("gen.fst", "0b899d6e5078b320043ed9174666743657240f27468c077c4df18a7ca9caa9e0"),
    ("lemmas.bin", "6d8507df387f95c39039616428eba37977a08db74f946af4a7140612fac7451f"),
    ("recs.bin", "443ddcbb47e76556610ab58c810818dbf82414087bdc874dc0395087ad0c4175"),
    ("tags.txt", "0079d5c88b31508f6383a515eee33c43632129b07d7d918216a0b5f2b96b499a"),
    ("words.fst", "bc0a7cfe2360a68eb5b610956a47c7ac26c0c7268fcf06768bc5f51adfd02190"),
];

/// The three files a user downloads (compressed names), in the order they are installed.
pub fn download_names() -> [String; 3] { [format!("{}.gz", MODEL.name), format!("{}.gz", LEXICON.name), RESOURCES_ARCHIVE.to_string()] }

/// The data directory: --data DIR, else VAUGELAS_DATA, else the platform's per-user data directory.
pub fn data_dir(explicit: Option<&str>, from_settings: Option<&Path>) -> PathBuf {
    if let Some(d) = explicit { return PathBuf::from(d); }
    if let Some(d) = std::env::var_os("VAUGELAS_DATA") { return PathBuf::from(d); }
    if let Some(d) = from_settings { return d.to_path_buf(); }
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

pub struct Files { pub dir: PathBuf, pub model: PathBuf, pub lexicon: PathBuf, pub resources: PathBuf, pub lex_dir: PathBuf }

impl Files {
    pub fn in_dir(d: &Path) -> Files {
        Files { dir: d.to_path_buf(), model: d.join(MODEL.name), lexicon: d.join(LEXICON.name), resources: d.join("resources"), lex_dir: d.join("lex") }
    }
    fn stamp(&self) -> PathBuf { self.dir.join("data-set.txt") }
    /// Every file present and installed by this version's data set.
    pub fn complete(&self) -> bool {
        let stamped = std::fs::read_to_string(self.stamp()).is_ok_and(|s| s.trim() == DATA_SET);
        stamped && self.model.is_file() && self.lexicon.is_file()
            && RESOURCE_FILES.iter().all(|(f, _)| self.resources.join(f).is_file())
            && LEX_FILES.iter().all(|(f, _)| self.lex_dir.join(f).is_file())
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

fn has_sha(p: &Path, sha: &str) -> bool { sha256_file(p).is_ok_and(|x| x == sha) }

/// Copy `reader` to `dest` through a temporary file, checking the sha256 of what is written.
fn write_checked(reader: &mut dyn Read, name: &str, sha: &str, size: u64, dest: &Path, progress: bool) -> Result<(), String> {
    if let Some(p) = dest.parent() { std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?; }
    let tmp = dest.with_extension("part");
    let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let (mut done, mut shown) = (0u64, 0u64);
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("{name}: {e}"))?;
        if n == 0 { break; }
        h.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", tmp.display()))?;
        done += n as u64;
        if progress && done - shown >= 4 << 20 { shown = done; eprint!("\r  {name}: {} / {} MB", done >> 20, size >> 20); }
    }
    out.flush().map_err(|e| format!("{}: {e}", tmp.display()))?;
    drop(out);
    if progress { eprintln!("\r  {name}: {} MB          ", done >> 20); }
    let got = hex(&h.finalize());
    if got != sha {
        std::fs::remove_file(&tmp).ok();
        return Err(format!("{name}: sha256 {got} does not match the expected {sha} (corrupted or wrong file)"));
    }
    std::fs::rename(&tmp, dest).map_err(|e| format!("{}: {e}", dest.display()))
}

/// Unpack the resource archive (a gzip-compressed tar) into `dir`: only the files of `RESOURCE_FILES`, each checked.
fn unpack_resources(reader: Box<dyn Read>, dir: &Path) -> Result<(), String> {
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(reader));
    let mut seen: Vec<&str> = Vec::new();
    for e in ar.entries().map_err(|e| format!("{RESOURCES_ARCHIVE}: {e}"))? {
        let mut e = e.map_err(|e| format!("{RESOURCES_ARCHIVE}: {e}"))?;
        if !e.header().entry_type().is_file() { continue; }
        let path = e.path().map_err(|e| format!("{RESOURCES_ARCHIVE}: {e}"))?.to_string_lossy().replace('\\', "/");
        let path = path.trim_start_matches("./").trim_start_matches("resources/");
        let Some((name, sha)) = RESOURCE_FILES.iter().find(|(f, _)| *f == path) else { continue };
        let size = e.header().size().unwrap_or(0);
        write_checked(&mut e, name, sha, size, &dir.join(name), false)?;
        seen.push(name);
    }
    let missing: Vec<&str> = RESOURCE_FILES.iter().map(|(f, _)| *f).filter(|f| !seen.contains(f)).collect();
    if !missing.is_empty() { return Err(format!("{RESOURCES_ARCHIVE}: missing {}", missing.join(", "))); }
    Ok(())
}

/// Where one data file comes from: a local directory holding `NAME` or `NAME.gz` (`from`), else the data release.
fn open_source(name: &str, from: Option<&Path>, base: &str) -> Result<(Box<dyn Read>, bool, String), String> {
    match from {
        Some(src) => {
            let (p, gz) = if src.join(name).is_file() { (src.join(name), false) } else { (src.join(format!("{name}.gz")), true) };
            let f = std::fs::File::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok((Box::new(f), gz, format!("copying from {}", p.display())))
        }
        None => {
            let url = format!("{base}{name}.gz");
            let resp = ureq::get(&url).call().map_err(|e| format!("download failed ({url}): {e}"))?;
            Ok((Box::new(resp.into_reader()), true, format!("downloading {url}")))
        }
    }
}

/// A directory that holds every data file a user downloads by hand (`download_names`, or the model and lexicon
/// uncompressed): the program's own folder, then the current folder.
pub fn local_source() -> Option<PathBuf> {
    let has = |d: &Path| {
        let one = |n: &str| d.join(n).is_file() || d.join(format!("{n}.gz")).is_file();
        one(MODEL.name) && one(LEXICON.name) && d.join(RESOURCES_ARCHIVE).is_file()
    };
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    [exe_dir, std::env::current_dir().ok()].into_iter().flatten().find(|d| has(d))
}

/// Install every data file into `dir`: from a local directory (`from`, holding `NAME` or `NAME.gz`) or by download.
/// Files already present with the right sha256 are kept (`force`: install them again). Then builds the rule checker's
/// lexicon, checks it, and stamps the directory with `DATA_SET`.
pub fn install(dir: &Path, from: Option<&Path>, force: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let f = Files::in_dir(dir);
    std::fs::remove_file(f.stamp()).ok();
    let base = std::env::var("VAUGELAS_DATA_URL").unwrap_or_else(|_| DATA_URL.to_string());
    eprintln!("data directory: {}", dir.display());
    for a in [&MODEL, &LEXICON] {
        let dest = dir.join(a.name);
        if !force && has_sha(&dest, a.sha256) { eprintln!("  {}: present, sha256 ok", a.name); continue; }
        let (r, gz, what) = open_source(a.name, from, &base)?;
        eprintln!("  {}: {what}", a.name);
        let mut r: Box<dyn Read> = if gz { Box::new(flate2::read::GzDecoder::new(r)) } else { r };
        write_checked(&mut r, a.name, a.sha256, a.size, &dest, from.is_none())?;
        eprintln!("  {}: sha256 ok", a.name);
    }
    if !force && RESOURCE_FILES.iter().all(|(n, h)| has_sha(&f.resources.join(n), h)) {
        eprintln!("  resources/: present, sha256 ok");
    } else {
        let (r, what) = match from {
            Some(src) => {
                let p = src.join(RESOURCES_ARCHIVE);
                let file = std::fs::File::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                (Box::new(file) as Box<dyn Read>, format!("copying from {}", p.display()))
            }
            None => {
                let url = format!("{base}{RESOURCES_ARCHIVE}");
                let resp = ureq::get(&url).call().map_err(|e| format!("download failed ({url}): {e}"))?;
                (resp.into_reader() as Box<dyn Read>, format!("downloading {url}"))
            }
        };
        eprintln!("  resources/: {what}");
        unpack_resources(r, &f.resources)?;
        eprintln!("  resources/: {} files, sha256 ok", RESOURCE_FILES.len());
    }
    build_lex(dir)?;
    std::fs::write(f.stamp(), format!("{DATA_SET}\n")).map_err(|e| format!("{}: {e}", f.stamp().display()))
}

/// Build `lex/` from lexicon.tsv (skipped when present and correct) and check every file's sha256.
pub fn build_lex(dir: &Path) -> Result<(), String> {
    let f = Files::in_dir(dir);
    let ok = |d: &Path| LEX_FILES.iter().all(|(n, h)| has_sha(&d.join(n), h));
    if ok(&f.lex_dir) { eprintln!("  lex/: present, sha256 ok"); return Ok(()); }
    eprintln!("  lex/: building the rule checker's lexicon from lexicon.tsv");
    vaugelas::rules::build::build_lexicon(&f.lexicon, &f.lex_dir).map_err(|e| format!("building lex/: {e}"))?;
    if !ok(&f.lex_dir) { return Err("lex/: built files do not have the expected sha256".into()); }
    eprintln!("  lex/: built, sha256 ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tar.gz holding `files` (path, content).
    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()));
        for (p, c) in files {
            let mut h = tar::Header::new_gnu();
            h.set_size(c.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, p, *c).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn resource_archive_is_checked() {
        let tmp = std::env::temp_dir().join(format!("vaugelas-test-{}", std::process::id()));
        // wrong content: refused, and nothing is left under the final name
        let bad = archive(&[("resources/interp/capstats.json", b"{}")]);
        let e = unpack_resources(Box::new(std::io::Cursor::new(bad)), &tmp).unwrap_err();
        assert!(e.contains("does not match"), "{e}");
        assert!(!tmp.join("interp/capstats.json").exists());
        // files outside the list are not written (only listed names are, under `dir`); a missing file is reported
        let other = archive(&[("resources/notes.txt", b"x")]);
        let e = unpack_resources(Box::new(std::io::Cursor::new(other)), &tmp).unwrap_err();
        assert!(e.contains("missing interp/capstats.json"), "{e}");
        assert!(!tmp.join("notes.txt").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
