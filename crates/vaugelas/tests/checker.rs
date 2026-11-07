//! Lexicon-backed tests. They skip themselves when the data (`vaugelas download-model`) is not installed.
use vaugelas::rules::Checker;
use std::path::Path;

fn checker() -> Option<Checker> {
    // the data directory of `vaugelas download-model` (VAUGELAS_DATA, else <repo>/data)
    let dir = std::env::var("VAUGELAS_DATA").map(|d| format!("{d}/lex")).unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lex").into());
    if !Path::new(&dir).join("words.fst").exists() { eprintln!("skipping: no lexicon at {dir}"); return None; }
    Some(Checker::open(Path::new(&dir)).unwrap())
}

fn rules(ck: &Checker, s: &str) -> Vec<&'static str> { ck.check(s).into_iter().map(|l| l.rule).collect() }

#[test]
fn flags_known_errors_with_the_right_fix() {
    let Some(ck) = checker() else { return };
    let l = ck.check("Il a manger toute la tarte.");
    assert!(l.iter().any(|x| x.rule == "FR_AVOIR_INF" && x.suggestions.contains(&"mangé".to_string())));
    let l = ck.check("Les élèves de la classe chante une chanson.");
    assert!(l.iter().any(|x| x.rule == "FR_SUBJ_VERB" && x.suggestions.contains(&"chantent".to_string())));
    let l = ck.check("Les lettres que j'ai écrit hier sont parties.");
    assert!(l.iter().any(|x| x.rule == "FR_PP_COD" && x.suggestions.contains(&"écrites".to_string())));
}

#[test]
fn correct_sentences_stay_clean() {
    let Some(ck) = checker() else { return };
    for s in [
        "Il a mangé toute la tarte avant que nous arrivions.",
        "Les lettres que j'ai écrites hier sont parties ce matin.",
        "Les enfants que la voisine garde chaque mercredi mangent trop de bonbons.",
        "Elle s'est lavée, puis elle s'est coiffée devant le miroir.",
        "Quelle que soit la raison, il faut que tu viennes.",
        "La plupart des personnes interrogées étaient satisfaites.",
        "Son père et sa mère faisaient partie des serviteurs.",
    ] {
        assert!(rules(&ck, s).is_empty(), "false positive on: {s}: {:?}", rules(&ck, s));
    }
}

#[test]
fn skips_a_relative_clause_to_reach_the_main_verb() {
    let Some(ck) = checker() else { return };
    assert!(rules(&ck, "Les enfants que la voisine garde chaque mercredi mange trop de bonbons.").contains(&"FR_SUBJ_VERB"));
}

#[test]
fn lazy_mode_defers_spelling_suggestions() {
    let Some(ck) = checker() else { return };
    let opts = vaugelas::rules::CheckOptions { spell_suggestions: false, ..Default::default() };
    let l = ck.check_opts("Cette situation est vraiment embarassante.", &opts);
    let spell: Vec<_> = l.iter().filter(|x| x.rule == "FR_SPELL").collect();
    assert_eq!(spell.len(), 1);
    assert!(spell[0].suggestions.is_empty());
    assert_eq!(ck.suggest_spelling("embarassante").first().map(String::as_str), Some("embarrassante"));
    assert_eq!(ck.suggest_spelling("Embarassante").first().map(String::as_str), Some("Embarrassante"));
}

#[test]
fn eager_mode_still_ranks_swaps_and_morphological_repairs_first() {
    let Some(ck) = checker() else { return };
    assert_eq!(ck.suggest_spelling("chevals").first().map(String::as_str), Some("chevaux"));
    assert_eq!(ck.suggest_spelling("mangons").first().map(String::as_str), Some("mangeons"));
    assert!(ck.suggest_spelling("prolbème").contains(&"problème".to_string()));
}


#[test]
fn agreement_gaps_from_minimal_pair_benchmarks() {
    let Some(ck) = checker() else { return };
    let flags = |s: &str, w: &str| ck.check(s).iter().any(|l| l.rule == "FR_SUBJ_VERB" && &s[l.start..l.end] == w);
    assert!(flags("Les auteurs qui aiment les gardes parle.", "parle"), "main verb after a qui relative");
    assert!(flags("Les auteurs qui aime les gardes parlent.", "aime"), "verb of the qui relative");
    assert!(flags("Les officiers aiment le cinéma et aime le théâtre.", "aime"), "coordinated verb");
    assert!(flags("Les auteurs devant les gardes est vieux.", "est"), "across a devant PP");
    assert!(flags("C'est la ville où vivait les ouvriers.", "vivait"), "inverted subject after où");
    let l = ck.check("La maison a été vendu hier.");
    assert!(l.iter().any(|x| x.rule == "FR_ATTR_AGREE" && x.suggestions.contains(&"vendue".to_string())), "a été + participle");
}

#[test]
fn agreement_gap_rules_leave_correct_sentences_alone() {
    let Some(ck) = checker() else { return };
    for s in [
        "Les livres que lisent les enfants sont nombreux.",
        "Le fils des voisins qui habitent en face est venu.",
        "Les officiers aiment le cinéma et regardent des films.",
        "C'est la ville où vivaient les ouvriers.",
        "La maison a été vendue hier.",
        "Les auteurs qui aiment les gardes parlent.",
        "Pierre mange une pomme et boit un verre d'eau.",
    ] {
        let r: Vec<_> = ck.check(s).into_iter().map(|l| l.rule).collect();
        assert!(r.is_empty(), "false positive on: {s}: {r:?}");
    }
}


#[test]
fn first_or_second_person_verb_needs_a_matching_pronoun() {
    let Some(ck) = checker() else { return };
    let flags = |s: &str, w: &str| ck.check(s).iter().any(|l| l.rule == "FR_SUBJ_VERB" && &s[l.start..l.end] == w);
    assert!(flags("L'auteur lui-même le traduirai en arabe.", "traduirai"));
    assert!(flags("Le roman raconte une histoire où s'affrontez les services secrets.", "affrontez"));
    for s in ["Je pense que nous partirons demain.", "Allez, venez avec moi !", "Prenez la deuxième rue à gauche.", "C'est moi qui ai raison.", "Si vous voulez, partons."] {
        let r: Vec<_> = ck.check(s).into_iter().map(|l| l.rule).collect();
        assert!(r.is_empty(), "false positive on: {s}: {r:?}");
    }
}
