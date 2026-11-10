//! `report::RULES` lists every rule the rule checker can emit, once (the settings file checks rule names against it).
use std::collections::BTreeSet;
use std::path::Path;
use vaugelas::report::{RULES, MODEL_RULE};

/// every string literal `"FR_..."` in the rule checker's sources (a name ending in `_` is a prefix, not a rule)
fn emitted(dir: &Path, out: &mut BTreeSet<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() { emitted(&p, out); continue; }
        if p.extension().is_none_or(|x| x != "rs") { continue; }
        let s = std::fs::read_to_string(&p).unwrap();
        for (k, _) in s.match_indices("\"FR_") {
            let name: String = s[k + 1..].chars().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_').collect();
            if s[k + 1 + name.len()..].starts_with('"') && !name.ends_with('_') { out.insert(name); }
        }
    }
}

#[test]
fn every_emitted_rule_is_listed_once() {
    let mut found = BTreeSet::new();
    emitted(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules"), &mut found);
    assert!(found.len() >= 40, "{found:?}");
    let listed: BTreeSet<String> = RULES.iter().map(|(r, _)| r.to_string()).collect();
    assert_eq!(listed.len(), RULES.len(), "a rule is listed twice");
    let missing: Vec<_> = found.difference(&listed).collect();
    assert!(missing.is_empty(), "rules emitted but not in report::RULES: {missing:?}");
    let extra: Vec<_> = listed.iter().filter(|r| *r != MODEL_RULE && !found.contains(*r)).collect();
    assert!(extra.is_empty(), "rules in report::RULES that nothing emits: {extra:?}");
}
