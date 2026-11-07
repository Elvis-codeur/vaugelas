use vaugelas::rules::tokenizer::tokenize;

fn words(s: &str) -> Vec<String> {
    tokenize(s).into_iter().filter(|t| t.word).map(|t| t.text).collect()
}

#[test]
fn splits_elision() {
    assert_eq!(words("l'enfant qu’il a vu"), ["l'", "enfant", "qu’", "il", "a", "vu"]);
}

#[test]
fn keeps_hyphenated_words_and_aujourdhui() {
    assert_eq!(words("peut-être aujourd'hui est-ce"), ["peut-être", "aujourd'hui", "est-ce"]);
}

#[test]
fn sentence_ids_advance_after_terminal_punctuation() {
    let t = tokenize("Il vient. Elle part ! Et nous ?");
    let sid = |w: &str| t.iter().find(|x| x.text == w).unwrap().sid;
    assert_eq!(sid("Il"), 0);
    assert_eq!(sid("Elle"), 1);
    assert_eq!(sid("Et"), 2);
    assert!(t.iter().find(|x| x.text == "Elle").unwrap().sent_start);
}

#[test]
fn decimal_number_does_not_end_a_sentence() {
    let t = tokenize("Il a 3.5 euros");
    assert!(t.iter().all(|x| x.sid == 0));
}

#[test]
fn urls_and_emails_are_skipped() {
    let t = tokenize("écrire à jean@exemple.fr pour voir");
    assert!(t.iter().filter(|x| x.text == "jean" || x.text == "exemple").all(|x| x.skip));
    assert!(!t.iter().find(|x| x.text == "voir").unwrap().skip);
}

#[test]
fn byte_offsets_slice_back_to_the_token_text() {
    let s = "Les élèves—ils ont chanté, déjà.";
    for t in tokenize(s) { assert_eq!(&s[t.start..t.end], t.text); }
}
