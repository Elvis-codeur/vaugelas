//! The lexical banks, contexts and held-out lists of `archive/python-2026-11-12/lexpairs.py`, verbatim.
//! Generated once from that file (see tools/gen_lexpairs_data.py); edit the Python archive first, then this.

pub struct Bank { pub sub: &'static str, pub family: &'static str, pub en: &'static str, pub right: &'static str, pub wrong: &'static str, pub note: &'static str }

pub const BANKS: [Bank; 27] = [
    Bank { sub: "works", family: "glossary_sense", en: "works (of God, of the law)", right: "œuvres", wrong: "travaux", note: "'works' in the theological sense is 'œuvres'; 'travaux' is manual labour" },
    Bank { sub: "purpose", family: "glossary_sense", en: "purpose, design (decree)", right: "dessein", wrong: "dessin", note: "decree/purpose is 'dessein'; 'dessin' is a drawing" },
    Bank { sub: "servant", family: "glossary_sense", en: "servant (of the Lord)", right: "serviteur", wrong: "ouvrier", note: "'serviteur' for the Lord's servant; 'ouvrier' is a workman" },
    Bank { sub: "grace", family: "glossary_sense", en: "grace (unmerited favour)", right: "grâce", wrong: "faveur", note: "theological 'grâce', not the generic 'faveur'" },
    Bank { sub: "scripture", family: "glossary_sense", en: "Scripture (the writings)", right: "Écriture", wrong: "écriture", note: "the corpus is 'l'Écriture' capitalised as the collection; 'écriture' is handwriting" },
    Bank { sub: "covenant", family: "glossary_sense", en: "covenant", right: "alliance", wrong: "contrat", note: "biblical 'alliance', not the commercial 'contrat'" },
    Bank { sub: "church", family: "glossary_sense", en: "the church (institution)", right: "Église", wrong: "temple", note: "the institution is 'l'Église'; 'temple' is a building" },
    Bank { sub: "repentance", family: "glossary_sense", en: "repentance", right: "repentance", wrong: "regret", note: "'repentance' is the doctrine; 'regret' is an emotion" },
    Bank { sub: "elder", family: "glossary_sense", en: "elder (office)", right: "ancien", wrong: "aîné", note: "the office is 'ancien'; 'aîné' is the elder sibling" },
    Bank { sub: "sensible", family: "false_friend", en: "a sensible propounding", right: "un exposé raisonnable", wrong: "une exposition sensible", note: "'sensible' in English is 'raisonnable'; the French 'sensible' means sensitive" },
    Bank { sub: "conversation", family: "false_friend", en: "conversation (= conduct, manner of life)", right: "conduite", wrong: "conversation", note: "17th-c. 'conversation' is 'conduite', not a chat" },
    Bank { sub: "freely", family: "false_friend", en: "freely (of his own accord)", right: "librement", wrong: "gratuitement", note: "'freely' here is 'librement'; the glossary row 'gratuitement' names cost, not freedom" },
    Bank { sub: "prevent", family: "false_friend", en: "to prevent (= go before)", right: "précéder", wrong: "empêcher", note: "'prevent' in the old sense is 'précéder', not 'empêcher'" },
    Bank { sub: "suffer", family: "false_friend", en: "to suffer (= allow)", right: "permettre", wrong: "souffrir", note: "'suffer' as 'allow' is 'permettre'" },
    Bank { sub: "flesh", family: "false_friend", en: "flesh (human nature)", right: "chair", wrong: "viande", note: "theological 'chair'; 'viande' is meat" },
    Bank { sub: "ghost", family: "false_friend", en: "the Holy Ghost", right: "l'Esprit", wrong: "le fantôme", note: "the Holy Ghost is 'l'Esprit', never 'le fantôme'" },
    Bank { sub: "attend", family: "false_friend", en: "to attend (= wait for)", right: "attendre", wrong: "assister", note: "'attend' as 'wait for' is 'attendre'; 'assister à' is to be present at" },
    Bank { sub: "continuance", family: "register", en: "with continuance (persisting in sin)", right: "persistance", wrong: "persévérance", note: "'persévérance' names the virtue; the sense is 'persistance'" },
    Bank { sub: "fear", family: "register", en: "the fear of God", right: "crainte", wrong: "peur", note: "the theological register is 'crainte'" },
    Bank { sub: "pride", family: "register", en: "pride (the sin)", right: "orgueil", wrong: "fierté", note: "'fierté' can be legitimate; the sin is 'orgueil'" },
    Bank { sub: "certainly", family: "calque", en: "it is certainly too much for us", right: "il est assurément trop pour nous", wrong: "il est certainement trop pour nous", note: "the English adverb modifies the judgement, not the verb" },
    Bank { sub: "made_a", family: "calque", en: "is made a mode of the revelation", right: "on fait de lui un mode de la révélation", wrong: "il est fait un mode de la révélation", note: "French needs 'on fait de … un …', not 'est fait un …'" },
    Bank { sub: "insist_that", family: "calque", en: "they insist that man cannot cooperate", right: "ils affirment que l'homme ne peut coopérer", wrong: "ils insistent sur ce que l'homme ne peut coopérer", note: "'insister sur ce que' + indicative is a calque; French says 'affirmer que'" },
    Bank { sub: "still", family: "misread_english", en: "still (= nevertheless)", right: "pourtant", wrong: "immobile", note: "'still' as 'nevertheless' is 'pourtant'" },
    Bank { sub: "present", family: "misread_english", en: "the present life", right: "la vie actuelle", wrong: "la vie cadeau", note: "'present' as 'current' is 'actuel', not 'cadeau'" },
    Bank { sub: "state", family: "misread_english", en: "state (condition)", right: "état", wrong: "État", note: "the condition is 'état'; 'État' is the polity" },
    Bank { sub: "mean", family: "misread_english", en: "to mean (= intend)", right: "vouloir dire", wrong: "moyen", note: "'to mean' is 'vouloir dire', not the noun 'moyen'" },
];

pub const CONTEXTS: [&str; 16] = [
    "L'apôtre montre que {e} n'appartient pas à l'homme, mais à {s} de Dieu seul.",
    "Ainsi {n} comprend que {s} précède toute décision humaine, et il l'enseigne à {p}.",
    "Ce chapitre traite de {s} ; {n} y revient trois fois avant de conclure.",
    "On ne peut séparer {s} de la promesse, écrit {n} dans sa lettre à {p}.",
    "La question de {s} occupe tout le second livre, et {n} la reprend à {p}.",
    "Si l'on nie {s}, toute la doctrine tombe, rappelle {n} aux fidèles de {p}.",
    "C'est ici que {s} devient visible : l'homme ne peut rien sans Dieu, dit {n}.",
    "Après avoir établi {s}, {n} passe à la pratique et instruit {p}.",
    "Toute la dispute porte sur {s} : {n} l'affirme, et {p} le conteste encore.",
    "{n} ne traite de {s} qu'après avoir rappelé la promesse, et {p} l'a bien noté.",
    "On demande à {n} comment concilier {s} avec la liberté de l'homme ; il répond à {p}.",
    "L'objection contre {s} est ancienne, mais {n} la juge sans force devant {p}.",
    "Ce que {n} appelle {s} n'est pas une opinion d'école, écrit-il de {p}.",
    "Il faut d'abord définir {s}, puis montrer comment {n} l'applique à {p}.",
    "Sur {s}, {n} refuse les deux excès, et il le fait voir à {p}.",
    "La difficulté de {s} arrête {n} un moment, puis il reprend son exposé à {p}.",
];

pub const TRAIN_NAMES: [&str; 16] = [
    "Jean", "Marie", "Pierre", "Paul", "Anne", "Luc", "Marthe", "Thomas", "Claire", "Étienne", "Catherine", "Michel", "Hélène", "Vincent", "Suzanne", "Antoine",
];
pub const HELD_NAMES: [&str; 16] = [
    "Anselme", "Bathilde", "Célestin", "Dorothée", "Eusèbe", "Fulbert", "Geneviève", "Hugues", "Isabeau", "Jocelin", "Léonce", "Mathilde", "Norbert", "Odile", "Pétronille", "Quentin",
];
pub const PLACES: [&str; 12] = [
    "Lyon", "Genève", "Rouen", "Paris", "Strasbourg", "Nîmes", "Montauban", "Sedan", "La Rochelle", "Orange", "Saumur", "Charenton",
];
pub const HELD_SUBTYPES: [&str; 7] = [
    "conversation", "covenant", "ghost", "made_a", "present", "pride", "purpose",
];
