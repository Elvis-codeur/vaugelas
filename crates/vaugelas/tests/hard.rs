//! One good/bad pair per hard-phenomenon rule: the bad sentence must be flagged, the good one must stay clean.
use vaugelas::rules::Checker;
use std::path::Path;

fn checker() -> Option<Checker> {
    // the data directory of `vaugelas download-model` (VAUGELAS_DATA, else <repo>/data)
    let dir = std::env::var("VAUGELAS_DATA").map(|d| format!("{d}/lex")).unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lex").into());
    if !Path::new(&dir).join("words.fst").exists() { eprintln!("skipping: no lexicon at {dir}"); return None; }
    Some(Checker::open(Path::new(&dir)).unwrap())
}

const PAIRS: &[(&str, &str, &str)] = &[
    // (good, bad, word expected to be flagged in bad)
    ("Dans la vallée vivaient des loups.", "Dans la vallée vivait des loups.", "vivait"),
    ("Où vont ces trains ?", "Où va ces trains ?", "va"),
    ("« Nous partons », répondirent les élèves.", "« Nous partons », répondit les élèves.", "répondit"),
    ("C'est moi qui ai raison.", "C'est moi qui a raison.", "a"),
    ("Plus d'un élève a échoué.", "Plus d'un élève ont échoué.", "ont"),
    ("Beaucoup de gens pensent le contraire.", "Beaucoup de gens pense le contraire.", "pense"),
    ("Toi et moi partirons demain.", "Toi et moi partiront demain.", "partiront"),
    ("Les amis, la famille, tout le monde était là.", "Les amis, la famille, tout le monde étaient là.", "étaient"),
    ("Quelles villes avez-vous visitées ?", "Quelles villes avez-vous visité ?", "visité"),
    ("Des pommes, j'en ai mangé trois.", "Des pommes, j'en ai mangées trois.", "mangées"),
    ("Elle a acheté des fleurs.", "Elle a achetées des fleurs.", "achetées"),
    ("Elles se sont téléphoné.", "Elles se sont téléphonées.", "téléphonées"),
    ("Elle s'est lavé les mains.", "Elle s'est lavée les mains.", "lavée"),
    ("Fatiguées par le voyage, les filles se sont couchées tôt.", "Fatigué par le voyage, les filles se sont couchées tôt.", "Fatigué"),
    ("Étant donné les circonstances, nous partons.", "Étant données les circonstances, nous partons.", "données"),
    ("Ces questions semblent difficiles.", "Ces questions semblent difficile.", "difficile"),
    ("Une affaire des plus délicates.", "Une affaire des plus délicate.", "délicate"),
    ("Des yeux marron.", "Des yeux marrons.", "marrons"),
    ("Une heure et demie.", "Une heure et demi.", "demi"),
    ("Elle est toute contente de partir.", "Elle est tout contente de partir.", "tout"),
    ("Quelle que soit la raison, il viendra.", "Quelque soit la raison, il viendra.", "Quelque"),
    ("Quatre-vingts personnes sont venues.", "Quatre-vingt personnes sont venues.", "Quatre-vingt"),
    ("Je doute qu'il puisse venir.", "Je doute qu'il peut venir.", "peut"),
    ("Je ne pense pas qu'il vienne.", "Je ne pense pas qu'il vient.", "vient"),
    ("Après qu'il est parti, nous avons dîné.", "Après qu'il soit parti, nous avons dîné.", "soit"),
    ("Si j'avais le temps, je viendrais.", "Si j'avais le temps, je viendrai.", "viendrai"),
    ("Mange ta soupe !", "Manges ta soupe !", "Manges"),
    ("On n'a rien vu.", "On a rien vu.", "On a"),
    ("Aurait-il compris ?", "Auraient-il compris ?", "Auraient-il"),
];

#[test]
fn hard_phenomena_pairs() {
    let Some(ck) = checker() else { return };
    let mut fails = Vec::new();
    for (good, bad, word) in PAIRS {
        let g: Vec<_> = ck.check(good).into_iter().filter(|l| !l.rule.starts_with("FR_SPELL") && !l.rule.starts_with("FR_TYPO")).map(|l| l.rule).collect();
        if !g.is_empty() { fails.push(format!("false positive on: {good} {g:?}")); }
        let hit = ck.check(bad).iter().any(|l| &bad[l.start..l.end] == *word);
        if !hit { fails.push(format!("missed «{word}» in: {bad}")); }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn hard_phenomena_negatives() {
    let Some(ck) = checker() else { return };
    for s in [
        "Il reste trois places dans le train.",
        "Il est arrivé des choses étranges.",
        "On est partis tôt, ma sœur et moi.",
        "Les efforts qu'il a fallu ont été immenses.",
        "La maison qu'il a fait construire est grande.",
        "Je pense qu'il viendra demain.",
        "J'espère qu'il viendra.",
        "Si tu veux, je viendrai.",
        "Ces fleurs, je les ai cueillies hier.",
        "Elles se sont lavées avant de partir.",
        "Va à l'école.",
        "Vas-y vite !",
        "On a plus de temps maintenant.",
        "Dans ce cas, mélangez les œufs.",
    ] {
        let r: Vec<_> = ck.check(s).into_iter().filter(|l| !l.rule.starts_with("FR_SPELL") && !l.rule.starts_with("FR_TYPO")).map(|l| l.rule).collect();
        assert!(r.is_empty(), "false positive on: {s}: {r:?}");
    }
}
