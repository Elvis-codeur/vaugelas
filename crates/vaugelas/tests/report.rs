//! The merged report: one entry per word, offsets per line, settings (needs the data files; skipped without them).
mod common;
use vaugelas::report::{Settings, Source, Vaugelas, MODEL_RULE};

#[test]
fn one_entry_per_word_and_line_offsets() {
    let Some(d) = common::load() else { return };
    let vg = Vaugelas::new(&d);
    assert_eq!(vg.default_threshold(), 0.9935, "N1's threshold comes from the model");
    assert_eq!(d.model.profile.frc_ignore, ["SPELL", "TYPO", "SPACE", "NBSP", "DUP", "DOUBLE"]);
    let text = "Ceci est correct.\nLes élections ont été annulés par le juge.\r\nIl a manger toute la tarte.";
    let issues = vg.check(text, &Settings::default());
    let words: Vec<&str> = issues.iter().map(|i| &text[i.start..i.end]).collect();
    assert!(words.contains(&"annulés") && words.contains(&"manger"), "{words:?}");
    for w in ["annulés", "manger"] { assert_eq!(words.iter().filter(|x| **x == w).count(), 1, "{w} reported once"); }
    let a = issues.iter().find(|i| &text[i.start..i.end] == "annulés").unwrap();
    assert!(a.replacements.contains(&"annulées".to_string()));
    // a model flag never overlaps a rule entry (it is merged into it instead)
    for i in issues.iter().filter(|i| i.source == Source::Model) {
        assert!(issues.iter().filter(|j| j.source != Source::Model).all(|j| i.end <= j.start || j.end <= i.start));
    }

    // two rules on the same word (attribute agreement + plural after être): still one entry, first rule's suggestion first
    let t2 = "Les rues étaient désert pendant la nuit.";
    let d2: Vec<_> = vg.check(t2, &Settings::default()).into_iter().filter(|i| &t2[i.start..i.end] == "désert").collect();
    assert_eq!(d2.len(), 1, "{d2:?}");
    assert_eq!(d2[0].replacements.first().map(String::as_str), Some("désertes"), "{d2:?}");

    // rules only / model only
    let r = vg.check(text, &Settings { model: false, ..Settings::default() });
    assert!(r.iter().all(|i| i.source == Source::Rules));
    let m = vg.check(text, &Settings { rules: false, ..Settings::default() });
    assert!(!m.is_empty() && m.iter().all(|i| i.rule == MODEL_RULE && i.score.is_some_and(|s| s > 0.9935)));
    // threshold 1.0: the model flags nothing
    let none = vg.check(text, &Settings { rules: false, threshold: Some(1.0), ..Settings::default() });
    assert!(none.is_empty());
}

#[test]
fn model_flags_are_the_scores_above_the_threshold() {
    let Some(d) = common::load() else { return };
    let vg = Vaugelas::new(&d);
    // a word in raw text the sentence filter cleans ("_" marks, a note call) is still found and reported in place
    let text = "Les _élections_ ont été annulés[3] par le juge.";
    let flags = vg.check(text, &Settings { rules: false, ..Settings::default() });
    let shown: Vec<&str> = flags.iter().map(|i| &text[i.start..i.end]).collect();
    assert_eq!(shown, ["annulés"], "{flags:?}");
    // with threshold 0 every scored word above 0 is a flag, at the same place as its token
    let s = "Il a manger toute la tarte.";
    let sc = vg.scorer.score_sentence(s).unwrap();
    let all = vg.check(s, &Settings { rules: false, threshold: Some(0.0), ..Settings::default() });
    assert_eq!(all.len(), sc.scores.iter().filter(|x| x.0 > 0.0).count());
    // English and foreign sentences are not scored
    assert!(vg.scorer.score_sentence("The cat is on the table and the dog is in the garden.").unwrap().skip.is_some());
}
