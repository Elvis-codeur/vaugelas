//! Byte parity of the Rust `product-pairs` generator.
//!
//! * the 16-sentence fixture is always checked against its Python output (same generator, same seed);
//! * the real 1,989-item batch 1 is checked when `PARITY_CARRIERS` points at the clean-text carrier JSONL
//!   (`datasets/cleantext/books_dev_texts.jsonl`), because that corpus is deliberately not in the repository.
//!
//! The wall-time line of `CARD.md` is measured and therefore not compared; the dataset is.
use std::io::Read;
use vaugelas_dataprep::product;

fn gunzip(p: &str) -> Vec<u8> {
    let f = std::fs::File::open(p).unwrap();
    let mut v = Vec::new();
    flate2::read::GzDecoder::new(f).read_to_end(&mut v).unwrap();
    v
}

fn lines(p: &str) -> Vec<String> {
    std::fs::read_to_string(p).unwrap().lines().map(str::to_string).collect()
}

#[test]
fn fixture_matches_python() {
    let dir = format!("{}/tests/parity", env!("CARGO_MANIFEST_DIR"));
    let p = product::generate(&lines(&format!("{dir}/carriers-small.jsonl")), 12, vaugelas_dataprep::DEFAULT_SEED);
    assert_eq!(product::sha256(&p.body), "5a50407d296ec64caa4b34e32e935b17bc6bd48f52cae326a6b9b71a055a9529");
    assert_eq!(p.body.as_bytes(), gunzip(&format!("{dir}/product-items-small.jsonl.gz")).as_slice());
    assert_eq!(p.items.len(), 12);
    assert_eq!(p.carriers, 16);
}

#[test]
fn real_batch_matches_python() {
    let Ok(src) = std::env::var("PARITY_CARRIERS") else {
        eprintln!("SKIP real_batch_matches_python: set PARITY_CARRIERS to the clean-text JSONL (the corpus is not in the repo)");
        return;
    };
    let dir = format!("{}/tests/parity", env!("CARGO_MANIFEST_DIR"));
    let p = product::generate(&lines(&src), 1989, vaugelas_dataprep::DEFAULT_SEED);
    assert_eq!(p.carriers, 13388);
    assert_eq!(product::sha256(&p.body), "f4a8b93c5c5a00f062e7fab4f73eba3ac84e17739d85b238fc15292657b8276c");
    assert_eq!(p.body.as_bytes(), gunzip(&format!("{dir}/product-items-1989.jsonl.gz")).as_slice());
}
