//! Parity of the energy model with the reference implementation (Python features + PyTorch, and the research Rust
//! port `frc-infer golden`), on the held-out evaluation sets. Heavy and data-dependent, so `#[ignore]`d; run with
//! `tools/parity.sh` (sets the variables below) or:
//!   VAUGELAS_DATA=DIR                 model.frcm + lexicon.tsv
//!   VAUGELAS_PARITY_ITEMS=items.jsonl the evaluation items ({"good", "bad", "kind", "valid"})
//!   VAUGELAS_PARITY_REF=scores.jsonl  reference per-word scores, one JSON list per record (frc-infer golden output)
//!   VAUGELAS_PARITY_PY=rows.jsonl.gz  optional: Python ebm_data.py records ({words, feats, err, cands}) to diff
//!   cargo test --release -p vaugelas --test parity_model -- --ignored --nocapture
//! Pass: every record's scores line is byte-identical to the reference, and (with _PY) 0 record mismatches.
use serde_json::{json, Value};
use std::io::BufRead;
use vaugelas::model::cands::Cands;
use vaugelas::model::pipeline::Pipeline;
use vaugelas::model::{open_lines, Model};
use vaugelas::text::characterize::tokenize;

fn env(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }

#[test]
#[ignore]
fn scores_identical_to_reference() {
    let (Some(data), Some(items), Some(refp)) = (env("VAUGELAS_DATA"), env("VAUGELAS_PARITY_ITEMS"), env("VAUGELAS_PARITY_REF")) else {
        eprintln!("skipping: set VAUGELAS_DATA, VAUGELAS_PARITY_ITEMS, VAUGELAS_PARITY_REF"); return;
    };
    let p = Pipeline::load(&format!("{data}/lexicon.tsv"));
    let m = Model::load(&format!("{data}/model.frcm")).unwrap();
    let c = Cands::new(&p.lex, &p.nat);
    // ebm_data.py pairs(): the good sentence (err = -1) and, for valid pairs of equal length, the bad sentence with
    // its first differing word as the error and the good word as gold
    let mut rows: Vec<(Vec<String>, Option<usize>, Option<String>)> = Vec::new();
    for l in std::fs::read_to_string(&items).unwrap().lines() {
        let it: Value = serde_json::from_str(l).unwrap();
        let ws = tokenize(&p.lex, it["good"].as_str().unwrap());
        rows.push((ws.clone(), None, None));
        if it["kind"] == "pair" && it.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
            let wb = tokenize(&p.lex, it["bad"].as_str().unwrap());
            if let Some(e) = ws.iter().zip(&wb).position(|(x, y)| x != y) {
                if wb.len() == ws.len() { rows.push((wb, Some(e), Some(ws[e].clone()))); }
            }
        }
    }
    let reference: Vec<String> = std::fs::read_to_string(&refp).unwrap().lines().map(String::from).collect();
    let py: Option<Vec<Value>> = env("VAUGELAS_PARITY_PY").map(|f| open_lines(&f).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap()).collect());
    assert_eq!(rows.len(), reference.len(), "record count differs from the reference");
    let (mut diff, mut rec_bad, mut words, mut maxd) = (0usize, 0usize, 0usize, 0f64);
    for (k, (ws, err, gold)) in rows.iter().enumerate() {
        let (toks, cs) = c.record(ws, *err, gold.as_deref());
        let feats = p.feats(&toks);
        if let Some(py) = &py {
            let pr = &py[k];
            let rc: Value = json!(cs.iter().map(|(i, g, v)| json!([i, g, v.iter().map(|(f, ks)| json!([f, ks.iter().map(|k| p.lex.keys[*k as usize].as_str()).collect::<Vec<_>>()])).collect::<Vec<_>>()])).collect::<Vec<_>>());
            let ok = pr["words"] == json!(ws) && pr["feats"] == json!(feats) && pr["err"].as_i64().unwrap() == err.map_or(-1, |e| e as i64) && pr["cands"] == rc;
            if !ok { rec_bad += 1; if rec_bad <= 5 { eprintln!("record {k} differs from Python: {ws:?}"); } }
        }
        let wr: Vec<&str> = ws.iter().map(|w| w.as_str()).collect();
        let s: Vec<f32> = m.score_parts(&feats, &wr, &p.cand_refs(&cs)).into_iter().map(|x| x.0).collect();
        let line = serde_json::to_string(&s).unwrap();
        words += s.len();
        if line != reference[k] {
            diff += 1;
            let r: Vec<f64> = serde_json::from_str(&reference[k]).unwrap();
            for (a, b) in s.iter().zip(&r) { maxd = maxd.max((*a as f64 - b).abs()); }
            if diff <= 5 { eprintln!("record {k}: scores differ\n  new {line}\n  ref {}", reference[k]); }
        }
    }
    eprintln!("{}", json!({"records": rows.len(), "scored_words": words, "lines_not_byte_identical": diff, "max_abs_diff": maxd,
                           "python_record_mismatches": py.as_ref().map(|_| rec_bad)}));
    assert_eq!(diff, 0);
    assert_eq!(rec_bad, 0);
}
