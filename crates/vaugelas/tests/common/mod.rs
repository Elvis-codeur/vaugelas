//! The installed data files for the data-dependent tests: VAUGELAS_DATA, else <repo>/data (as `vaugelas
//! download-model` lays them out). None (the test skips itself) when they are not there: the data is not in git.
use std::path::{Path, PathBuf};
use vaugelas::model::pipeline::LexCache;
use vaugelas::report::{Data, DataPaths};

pub fn data_dir() -> Option<PathBuf> {
    let d = std::env::var("VAUGELAS_DATA").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    let complete = d.join("lex/words.fst").is_file() && d.join("model.frcm").is_file() && d.join("lexicon.tsv").is_file()
        && d.join("resources/interp").is_dir() && d.join("resources/ngram").is_dir();
    if complete { Some(d) } else { eprintln!("skipping: no data at {}", d.display()); None }
}

#[allow(dead_code)] // not every test file loads the whole data set
pub fn load() -> Option<Data> {
    let d = data_dir()?;
    let p = DataPaths { lex_dir: &d.join("lex"), lexicon_tsv: &d.join("lexicon.tsv"), resources: &d.join("resources"), model: &d.join("model.frcm") };
    Some(Data::load(&p, false, &LexCache::NextToTsv).unwrap())
}

#[allow(dead_code)]
pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)).unwrap()
}
