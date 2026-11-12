//! Byte parity of the Rust `lexpairs` generator against the Python generator's outputs.
//!
//! Three datasets are checked, all produced by Python (the two 1,080 variants and the 1,620 one), and their
//! dataset cards. `sha256` constants are the ones the Python scripts printed, so a change in the Python archive
//! (or in this port) fails here even if the golden file were regenerated.
use std::io::Read;
use vaugelas_dataprep::lexpairs;

fn gunzip(p: &str) -> Vec<u8> {
    let f = std::fs::File::open(p).unwrap();
    let mut v = Vec::new();
    flate2::read::GzDecoder::new(f).read_to_end(&mut v).unwrap();
    v
}

fn check(per: usize, contexts: usize, file: &str, sha: &str, card_sha: &str) {
    let body = lexpairs::body(&lexpairs::records(per, vaugelas_dataprep::DEFAULT_SEED, contexts));
    assert_eq!(lexpairs::sha256(&body), sha, "{per}/{contexts}: dataset sha256");
    let want = gunzip(&format!("{}/tests/parity/{file}", env!("CARGO_MANIFEST_DIR")));
    assert_eq!(body.as_bytes(), want.as_slice(), "{per}/{contexts}: bytes vs Python");
    let recs = lexpairs::records(per, vaugelas_dataprep::DEFAULT_SEED, contexts);
    let card = lexpairs::card(&recs, &body, per, vaugelas_dataprep::DEFAULT_SEED, sha);
    assert_eq!(lexpairs::sha256(&card), card_sha, "{per}/{contexts}: CARD.md sha256");
}

#[test]
fn archived_generator_1080() {
    // the 213-line archived script, 40 per sub-type: ae10a385… (data/lexpairs.jsonl predates it, see the next test)
    check(40, 16, "lexpairs-1080.jsonl.gz", "ae10a3859f167fc65510451f54b7d69966d7f83779ce14e742b147a8d5db003e", "8565a2a64c1f1153ed360ab961b9d7196c195fa697e8caef60124750b56bdf79");
}

#[test]
fn archived_generator_1620() {
    check(60, 16, "lexpairs-1620.jsonl.gz", "95cc3570a3635f15db1ff16d5ecc648c8b28ab5c0c97a6c465727820c7d7b974", "1ab7ab7512d6759030bd97f520b25bd255b1de27ce7b70a64f365c9cfc47c361");
}

#[test]
fn legacy_1080_with_eight_contexts() {
    // the pre-12:55 revision (git 3693633) that produced the committed data/lexpairs.jsonl: f28bf5e2…
    check(40, 8, "lexpairs-1080-legacy8.jsonl.gz", "f28bf5e26ff92012919f9136e46283a3b3bcd5a4617278b2fad2040b6ad865ae", "5ff6321e3013c913ea3a161f3cb6ecb848f1ea52a7ffecc94cdaf1714de2d6f1");
}

#[test]
fn seed_changes_output() {
    let a = lexpairs::body(&lexpairs::records(2, 20261112, 16));
    let b = lexpairs::body(&lexpairs::records(2, 20261113, 16));
    assert_ne!(lexpairs::sha256(&a), lexpairs::sha256(&b));
}
