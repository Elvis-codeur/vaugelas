//! Regenerate `tests/fixtures/sample.frc.jsonl` for the current checker (issue #3).
//!
//! The old expected file was the reference `frc` output; this one is the output of this binary with the rules
//! added after the fixture was frozen (`report::EXTENSION_RULES`) filtered out, so the parity test still compares
//! the fr-check-compatible rule set. The reference binary is not on this machine, so the fixture is now a
//! regression fixture; the frc-derived file remains in history (commit db7ff56).
//!
//!   VAUGELAS_DATA=… cargo run --release --example gen_parity
use std::path::Path;
use vaugelas::report::EXTENSION_RULES;
use vaugelas::rules::{CheckOptions, Checker};

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn main() {
    let d = std::env::var("VAUGELAS_DATA").unwrap_or_else(|_| "data".into());
    let ck = Checker::open(&Path::new(&d).join("lex")).expect("open lex");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let input = std::fs::read_to_string(root.join("sample.txt")).expect("sample.txt");
    let mut out = String::new();
    for line in input.lines() {
        let lints: Vec<_> = ck.check_opts(line, &CheckOptions::default()).into_iter()
            .filter(|l| !EXTENSION_RULES.contains(&l.rule)).collect();
        out.push_str("{\"lints\":[");
        for (k, l) in lints.iter().enumerate() {
            if k > 0 { out.push(','); }
            let cs = line[..l.start].chars().count();
            let ce = line[..l.end].chars().count();
            out.push_str(&format!("{{\"rule\":\"{}\",\"offset\":{},\"len\":{},\"text\":\"{}\",\"message\":\"{}\",\"suggestions\":[{}]}}",
                l.rule, cs, ce - cs, esc(&line[l.start..l.end]), esc(&l.message),
                l.suggestions.iter().map(|x| format!("\"{}\"", esc(x))).collect::<Vec<_>>().join(",")));
        }
        out.push_str("]}\n");
    }
    std::fs::write(root.join("sample.frc.jsonl"), out).expect("write");
    eprintln!("wrote {} lines", input.lines().count());
}
