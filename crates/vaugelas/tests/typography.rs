use vaugelas::rules::typography::check;

fn rules(s: &str, strict: bool) -> Vec<&'static str> { check(s, strict).into_iter().map(|l| l.rule).collect() }

#[test]
fn missing_space_before_double_punctuation() {
    let l = check("Vraiment? Oui!", false);
    assert_eq!(l.len(), 2);
    assert!(l.iter().all(|x| x.rule == "FR_TYPO_SPACE_BEFORE"));
    assert_eq!(l[0].suggestions[0], "\u{202F}?");
    assert_eq!(rules("Attention: voici", false), ["FR_TYPO_SPACE_BEFORE"]);
}

#[test]
fn normal_space_is_fine_unless_strict() {
    assert!(rules("Vraiment ? Oui ! Attention : voici ; fin", false).is_empty());
    assert_eq!(rules("Vraiment ? Oui", true), ["FR_TYPO_NBSP"]);
    assert!(rules("Vraiment\u{202F}? Oui\u{00A0}: non", true).is_empty());
}

#[test]
fn exceptions_times_urls_smileys_and_sequences() {
    assert!(rules("Rendez-vous à 10:30 précises", false).is_empty());
    assert!(rules("Voir https://exemple.fr/page?id=3 demain", false).is_empty());
    assert!(rules("Merci :) et ;-) aussi", false).is_empty());
    assert_eq!(rules("Quoi ?!", false), Vec::<&str>::new());
}

#[test]
fn guillemets_need_inner_spaces() {
    assert_eq!(rules("Il dit «bonjour» puis part.", false), ["FR_TYPO_GUILLEMET", "FR_TYPO_GUILLEMET"]);
    assert!(rules("Il dit « bonjour » puis part.", false).is_empty());
    assert_eq!(rules("Il dit « bonjour » puis part.", true), ["FR_TYPO_NBSP", "FR_TYPO_NBSP"]);
}

#[test]
fn missing_space_after_punctuation() {
    assert!(rules("Attention :voici", false).contains(&"FR_SPACE_AFTER"));
    assert_eq!(rules("C'est la fin.Le jour se lève.", false), ["FR_SPACE_AFTER"]);
    assert!(rules("M.Dupont a 3.5 euros, cf. U.S.A.", false).is_empty());
}

#[test]
fn double_spaces_and_strict_conventions() {
    assert_eq!(rules("Il  vient.", false), ["FR_DOUBLE_SPACE"]);
    assert!(rules("    indentation", false).is_empty());
    assert_eq!(rules("Et puis... rien", true), ["FR_TYPO_ELLIPSIS"]);
    assert_eq!(rules("l'enfant", true), ["FR_TYPO_APOS"]);
    assert!(rules("Et puis... l'enfant", false).is_empty());
}
