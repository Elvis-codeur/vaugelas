//! Rule parity on `tests/fixtures/sample.txt`: 1,006 sentences generated for this test by `tools/make_sample.py`
//! (templates and word lists written for it; no sentence from any corpus, evaluation or training set).
//!
//! `sample.frc.jsonl` was the reference output `frc --lex lex check --json sample.txt` (fr-check, branch
//! feat/rust-infer, c17445e). The old fixture held held-out test sentences and was replaced (issue #3); the
//! expected file is now regenerated from this binary by `cargo run --release --example gen_parity`, with the rules
//! added after the fixture was frozen (`report::EXTENSION_RULES`) filtered out on both sides, so the comparison
//! still covers the fr-check-compatible rule set. The reference binary is not on this machine, so this is a
//! regression fixture: it fails when a change to an existing rule changes its output.
//! Skipped when the data files are not installed (VAUGELAS_DATA, else <repo>/data).
mod common;
use common::fixture;
use vaugelas::report::EXTENSION_RULES;
use vaugelas::rules::{CheckOptions, Checker};

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
    // the rules added after the fixture was frozen have no counterpart in the reference output
    let mut lints = ck.check_opts(text, opts);
    lints.retain(|l| !EXTENSION_RULES.contains(&l.rule));
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
    let Some(d) = common::data_dir() else { return };
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
