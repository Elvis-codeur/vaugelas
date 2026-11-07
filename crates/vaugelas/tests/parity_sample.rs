//! The rule checker and the model give exactly what the research binaries gave on `tests/fixtures/sample.txt`
//! (1,006 sentences from the final-2 and tuning rounds plus hand-written ones):
//!   sample.frc.jsonl        `frc --lex lex check --json sample.txt` (fr-check, branch feat/rust-infer)
//!   sample.frc-infer.jsonl  `frc-infer batch model.frcm lexicon.tsv sample.txt` (threshold 0.9571)
//! Skipped when the data files are not installed (VAUGELAS_DATA, else <repo>/data).
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use vaugelas::model::cands::Cands;
use vaugelas::model::pipeline::Pipeline;
use vaugelas::model::Model;
use vaugelas::rules::{CheckOptions, Checker};

fn data() -> Option<PathBuf> {
    let d = std::env::var("VAUGELAS_DATA").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    if d.join("lex/words.fst").is_file() && d.join("model.frcm").is_file() { Some(d) } else { eprintln!("skipping: no data at {}", d.display()); None }
}

fn fixture(name: &str) -> String { std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)).unwrap() }

/// frc's `json_line`, verbatim
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"),
                  c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)), c => o.push(c) }
    }
    o
}
fn json_line(ck: &Checker, text: &str, opts: &CheckOptions) -> String {
    let lints = ck.check_opts(text, opts);
    let mut s = String::from("{\"lints\":[");
    for (k, l) in lints.iter().enumerate() {
        if k > 0 { s.push(','); }
        let cs = text[..l.start].chars().count();
        let ce = text[..l.end].chars().count();
        s.push_str(&format!("{{\"rule\":\"{}\",\"offset\":{},\"len\":{},\"text\":\"{}\",\"message\":\"{}\",\"suggestions\":[{}]}}",
            l.rule, cs, ce - cs, esc(&text[l.start..l.end]), esc(&l.message),
            l.suggestions.iter().map(|x| format!("\"{}\"", esc(x))).collect::<Vec<_>>().join(",")));
    }
    s.push_str("]}");
    s
}

#[test]
fn rules_identical_to_frc() {
    let Some(d) = data() else { return };
    let ck = Checker::open(&d.join("lex")).unwrap();
    let (input, expected) = (fixture("sample.txt"), fixture("sample.frc.jsonl"));
    let exp: Vec<&str> = expected.lines().collect();
    let lines: Vec<&str> = input.lines().collect();
    assert_eq!(lines.len(), exp.len());
    let mut diff = 0;
    for (l, e) in lines.iter().zip(&exp) {
        let got = json_line(&ck, l, &CheckOptions::default());
        if got != *e { diff += 1; if diff <= 5 { eprintln!("differs on {l}\n  new {got}\n  ref {e}"); } }
    }
    eprintln!("rules: {} lines, {diff} differ", lines.len());
    assert_eq!(diff, 0);
}

#[test]
fn model_flags_identical_to_frc_infer() {
    let Some(d) = data() else { return };
    let p = Pipeline::load(d.join("lexicon.tsv").to_str().unwrap());
    let m = Model::load(d.join("model.frcm").to_str().unwrap()).unwrap();
    let c = Cands::new(&p.lex, &p.nat);
    let (input, expected) = (fixture("sample.txt"), fixture("sample.frc-infer.jsonl"));
    let exp: Vec<&str> = expected.lines().collect();
    let lines: Vec<&str> = input.lines().collect();
    assert_eq!(lines.len(), exp.len());
    let (mut diff, mut nflags) = (0, 0);
    for (l, e) in lines.iter().zip(&exp) {
        let fl: Vec<Value> = p.flags(&m, &c, l, 0.9571).into_iter().map(|f| json!({"sentence": f.sent, "word": f.word, "text": f.text, "score": f.score, "suggestion": f.suggestion})).collect();
        nflags += fl.len();
        let got = json!({"text": l, "flags": fl}).to_string();
        if got != *e { diff += 1; if diff <= 5 { eprintln!("differs\n  new {got}\n  ref {e}"); } }
    }
    eprintln!("model: {} lines, {nflags} flags, {diff} differ", lines.len());
    assert_eq!(diff, 0);
}
