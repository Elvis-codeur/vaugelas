//! Record-path parity: `vaugelas-dataprep pairs` against Python `ebm_data2.py pairs` on the same 60 items.
//!
//! The Python reference (`pairs-reference60.jsonl.gz`) was built on the ThinkPad with the research repository's
//! `ebm_data2.py` and the v14 flag set the Rust path implements: `FRC_LEMMA=1 FRC_TRIG=1 FRC_SENT=1 FRC_GEN3=1
//! FRC_GEN4=1 FRC_LEXHOM=honophones_lex_1.0.json FRC_LEXHOM_GROUP=1 FRC_LEXHOM_NOSAME=1 FRC_LMFEAT=1 FRC_MAXF=16
//! FRC_CW_PAIRS=<data-3 cw_pairs.json> FRC_IGNORE=SPELL,TYPO,SPACE,NBSP,DUP,DOUBLE`, against the same
//! `lexicon.tsv` (sha256 4cbad5ee…), `build/interp` tables and N1 model header.
//!
//! Result: 113 of 119 records identical, 6 differing by exactly two documented causes, both older-in-Python:
//!   * `invented` — the Rust checker's `FR_INVENTED` rule (PR #21) flags a token the Nov-8 `frc` binary did not; the extra atom `frc=FR_INVENTED` is the only difference;
//!   * `capr` — `lm_atoms` in the research `ebm_data2.py` adds `|capr=` after any sentence-opening punctuation, the v14 path (and this port) suppresses it at a sentence start (`midsent`); readings differ only by `|capr=N`.
//!
//! A third cause fails the test. The test needs the model data (`VAUGELAS_DATA`, see the CLI `--data`).
use serde_json::Value;
use std::io::Read;
use vaugelas_dataprep::records::pairs;

fn gunzip(p: &str) -> Vec<u8> {
    let f = std::fs::File::open(p).unwrap();
    let mut v = Vec::new();
    flate2::read::GzDecoder::new(f).read_to_end(&mut v).unwrap();
    v
}

fn strip_capr(v: &Value) -> Value {
    let mut v = v.clone();
    fn fix(s: &mut String) {
        while let Some(i) = s.find("|capr=") {
            let rest = &s[i + 6..];
            let n = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            s.replace_range(i..i + 6 + n, "");
        }
    }
    if let Some(cands) = v.get_mut("cands").and_then(Value::as_array_mut) {
        for c in cands.iter_mut() {
            for cand in c[2].as_array_mut().unwrap().iter_mut() {
                let arr = cand[1].as_array_mut().unwrap();
                for x in arr.iter_mut() {
                    let mut s = x.as_str().unwrap().to_string();
                    fix(&mut s);
                    *x = Value::String(s);
                }
            }
        }
    }
    v
}

fn strip_invented(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(feats) = v.get_mut("feats").and_then(Value::as_array_mut) {
        for f in feats.iter_mut().filter_map(Value::as_array_mut) {
            f.retain(|x| x != "frc=FR_INVENTED");
        }
    }
    v
}

#[test]
fn records_match_python_with_named_differences() {
    let Some(data) = std::env::var_os("VAUGELAS_DATA") else {
        eprintln!("SKIP records_match_python_with_named_differences: set VAUGELAS_DATA to the model data directory");
        return;
    };
    let dir = format!("{}/tests/parity", env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("vaugelas-pairs-test-{}.jsonl", std::process::id()));
    let st = pairs(std::path::Path::new(&format!("{dir}/pairs-items60.jsonl")), &out, std::path::Path::new(&data), None).unwrap();
    assert_eq!(st.records, 119);
    let rust: Vec<Value> = std::fs::read_to_string(&out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let py_text = String::from_utf8(gunzip(&format!("{dir}/pairs-reference60.jsonl.gz"))).unwrap();
    let py: Vec<Value> = py_text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(rust.len(), py.len());
    let (mut same, mut invented, mut capr) = (0, 0, 0);
    for (k, (a, b)) in rust.iter().zip(&py).enumerate() {
        if a == b {
            same += 1;
        } else if strip_invented(a) == strip_invented(b) {
            invented += 1;
        } else if strip_capr(a) == strip_capr(b) {
            capr += 1;
        } else {
            panic!("record {k} ({} err {}): difference beyond the two documented causes\nrust: {}\npython: {}", a["src"], a["err"], a, b);
        }
    }
    assert_eq!((same, invented, capr), (113, 4, 2), "identical / FR_INVENTED / capr");
    std::fs::remove_file(&out).ok();
}

/// `FRC_LEXPAIRS_CONF` is ported: with the 143 confusions derived from batch 1 (the original file was a temporary
/// artifact), the gold form reaches the candidate list on 45 of the 59 bad records. Without it: 1. The archived
/// pre-GEN3 Python build of the same batch reaches 50/59: the v14 lists are longer (10.0 vs 8.8 candidates), so five
/// pri-8 confusions fall past MAXC=16. That difference is reported, not hidden (PARITY.md).
#[test]
fn lexconf_reaches_the_gold() {
    let Some(data) = std::env::var_os("VAUGELAS_DATA") else {
        eprintln!("SKIP lexconf_reaches_the_gold: set VAUGELAS_DATA");
        return;
    };
    let dir = format!("{}/tests/parity", env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("vaugelas-lexconf-test-{}.jsonl", std::process::id()));
    let conf_path = format!("{dir}/lexconf-batch1.json");
    let items_path = format!("{dir}/pairs-items60.jsonl");
    pairs(std::path::Path::new(&items_path), &out, std::path::Path::new(&data), Some(std::path::Path::new(&conf_path))).unwrap();
    let rows: Vec<Value> = std::fs::read_to_string(&out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let bad: Vec<&Value> = rows.iter().filter(|r| r["err"].as_i64().unwrap() >= 0).collect();
    let gold = bad.iter().filter(|r| {
        let e = r["err"].as_u64().unwrap() as usize;
        r["cands"].as_array().unwrap().iter().any(|c| c[0].as_u64().unwrap() as usize == e && c[1].as_u64().unwrap() > 0)
    }).count();
    assert_eq!((rows.len(), bad.len(), gold), (119, 59, 45));
    std::fs::remove_file(&out).ok();
}
