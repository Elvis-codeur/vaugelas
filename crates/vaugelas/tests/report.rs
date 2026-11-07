//! The merged report: one entry per word, offsets per line (needs the data files; skipped without them).
use std::path::{Path, PathBuf};
use vaugelas::model::pipeline::LexCache;
use vaugelas::report::{Settings, Source, Vaugelas, MODEL_RULE};

fn load() -> Option<Vaugelas> {
    let d = std::env::var("VAUGELAS_DATA").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    if !d.join("lex/words.fst").is_file() || !d.join("model.frcm").is_file() { eprintln!("skipping: no data at {}", d.display()); return None; }
    Some(Vaugelas::load(&d.join("lex"), &d.join("lexicon.tsv"), &d.join("model.frcm"), false, &LexCache::NextToTsv).unwrap())
}

#[test]
fn one_entry_per_word_and_line_offsets() {
    let Some(vg) = load() else { return };
    let c = vg.cands();
    let text = "Ceci est correct.\nLes élections ont été annulés par le juge.\r\nIl a manger toute la tarte.";
    let issues = vg.check(&c, text, &Settings::default());
    let words: Vec<&str> = issues.iter().map(|i| &text[i.start..i.end]).collect();
    assert!(words.contains(&"annulés") && words.contains(&"manger"), "{words:?}");
    for w in ["annulés", "manger"] { assert_eq!(words.iter().filter(|x| **x == w).count(), 1, "{w} reported once"); }
    let a = issues.iter().find(|i| &text[i.start..i.end] == "annulés").unwrap();
    assert!(a.replacements.contains(&"annulées".to_string()));
    // a model flag never overlaps a rule entry (it is merged into it instead)
    for i in issues.iter().filter(|i| i.source == Source::Model) {
        assert!(issues.iter().filter(|j| j.source != Source::Model).all(|j| i.end <= j.start || j.end <= i.start));
    }

    // rules only / model only
    let r = vg.check(&c, text, &Settings { model: false, ..Settings::default() });
    assert!(r.iter().all(|i| i.source == Source::Rules));
    let m = vg.check(&c, text, &Settings { rules: false, ..Settings::default() });
    assert!(!m.is_empty() && m.iter().all(|i| i.rule == MODEL_RULE));
    // threshold 1.0: the model flags nothing
    let none = vg.check(&c, text, &Settings { rules: false, threshold: 1.0, ..Settings::default() });
    assert!(none.is_empty());
}
