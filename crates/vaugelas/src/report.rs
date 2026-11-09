//! Rules + model in one report, one entry per word.
//!
//! The text is checked line by line (like `frc check FILE`, one document per line). On each line the rule checker
//! gives its lints; the energy model scores every word of every sentence and flags those above the threshold. A second
//! rule on the same word, or a model flag on a word that a rule already covers, is merged into the first entry (its
//! suggestions are added to the replacements when new), so each word appears once. Nothing else combines the two: the report is the plain union of
//! the rules and the model (the learned combiner of v14 was fitted for that model only and is not used).
use crate::model::pipeline::{LexCache, Scorer, Tables};
use crate::model::Model;
use crate::rules::{CheckOptions, Checker};
use std::path::Path;

/// Rule id given to the model's flags.
pub const MODEL_RULE: &str = "VAUGELAS_MODEL";

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// a word is flagged by the model when its score is above this (0..1; higher = fewer, surer flags);
    /// None = the model's own default (`Model::profile`)
    pub threshold: Option<f64>,
    pub rules: bool,
    pub model: bool,
    /// compute spelling suggestions (off = faster, FR_SPELL lints without suggestions)
    pub spell_suggestions: bool,
    /// also report typographic conventions (non-breaking spaces, `…`, `’`)
    pub strict_typography: bool,
}

impl Default for Settings {
    fn default() -> Self { Settings { threshold: None, rules: true, model: true, spell_suggestions: true, strict_typography: false } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source { Rules, Model, Both }

impl Source {
    pub fn as_str(self) -> &'static str { match self { Source::Rules => "rules", Source::Model => "model", Source::Both => "rules+model" } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category { Grammar, Typos, Typography, Casing }

impl Category {
    pub fn of_rule(rule: &str) -> Category {
        match rule {
            "FR_SPELL" => Category::Typos,
            "FR_CAPITAL" => Category::Casing,
            r if r.starts_with("FR_TYPO_") || matches!(r, "FR_DOUBLE_SPACE" | "FR_SPACE_AFTER" | "FR_SPACE_BEFORE") => Category::Typography,
            _ => Category::Grammar,
        }
    }
    /// LanguageTool category id
    pub fn id(self) -> &'static str { match self { Category::Grammar => "GRAMMAR", Category::Typos => "TYPOS", Category::Typography => "TYPOGRAPHY", Category::Casing => "CASING" } }
    pub fn name(self) -> &'static str { match self { Category::Grammar => "Grammaire", Category::Typos => "Orthographe", Category::Typography => "Typographie", Category::Casing => "Majuscules" } }
    /// LanguageTool issueType
    pub fn issue_type(self) -> &'static str { match self { Category::Grammar => "grammar", Category::Typos => "misspelling", Category::Typography => "typographical", Category::Casing => "typographical" } }
}

/// One finding. `start..end` are byte offsets into the checked text.
#[derive(Clone, Debug)]
pub struct Issue {
    pub start: usize,
    pub end: usize,
    pub rule: String,
    pub message: String,
    pub replacements: Vec<String>,
    pub source: Source,
    /// model score of the word (0..1) when the model flagged it
    pub score: Option<f64>,
    pub category: Category,
}

/// Short French description of a rule (LanguageTool's `rule.description`).
pub fn rule_description(rule: &str) -> &'static str {
    match rule {
        MODEL_RULE => "Forme douteuse (modèle)",
        "FR_SPELL" => "Mot inconnu",
        "FR_SUBJ_VERB" => "Accord sujet-verbe",
        "FR_ATTR_AGREE" => "Accord de l’attribut",
        "FR_DET_NOUN_GEN" => "Accord en genre du déterminant",
        "FR_DET_NOUN_NUM" => "Accord en nombre du déterminant",
        "FR_NOUN_ADJ" => "Accord du nom et de l’adjectif",
        "FR_ETRE_PLURAL" => "Accord avec être",
        "FR_PP_AVOIR" | "FR_PP_COD" => "Accord du participe passé avec avoir",
        "FR_PP_PRONOMINAL" => "Accord du participe passé pronominal",
        "FR_PP_INVARIABLE" => "Participe passé invariable",
        "FR_AVOIR_INF" | "FR_INF_PP" | "FR_ER_E" => "Infinitif ou participe passé",
        "FR_PPRESENT" => "Participe présent",
        "FR_SUBJONCTIF" => "Subjonctif attendu",
        "FR_INDICATIF" => "Indicatif attendu",
        "FR_SI_COND" => "Si + conditionnel",
        "FR_IMPERATIF" => "Impératif",
        "FR_HOMOPHONE" | "FR_A_ACCENT" | "FR_CA_SA" | "FR_CE_SE" | "FR_ON_ONT" | "FR_SONT_SON" => "Homophones",
        "FR_ON_NEG" | "FR_NE_QUE" => "Négation",
        "FR_TOUT" => "Accord de tout",
        "FR_QUEL_QUE" => "Quel que / quelque",
        "FR_LEUR" => "Leur / leurs",
        "FR_DONT" | "FR_LEQUEL" => "Pronom relatif",
        "FR_PRONOM" => "Pronom",
        "FR_PREP" => "Préposition",
        "FR_ADV_VERB" => "Adverbe et verbe",
        "FR_COLOUR" => "Adjectif de couleur",
        "FR_COMPOUND" => "Mot composé",
        "FR_NUMBER" => "Nombres",
        "FR_DUP" | "FR_ELIDED_DUP" => "Mot répété",
        "FR_CAPITAL" => "Majuscule",
        "FR_DOUBLE_SPACE" => "Espaces multiples",
        "FR_SPACE_AFTER" => "Espace après la ponctuation",
        "FR_SPACE_BEFORE" | "FR_TYPO_SPACE_BEFORE" => "Espace avant la ponctuation",
        "FR_TYPO_NBSP" => "Espace insécable",
        "FR_TYPO_APOS" => "Apostrophe typographique",
        "FR_TYPO_ELLIPSIS" => "Points de suspension",
        "FR_TYPO_GUILLEMET" => "Guillemets",
        _ => "Règle de grammaire",
    }
}

/// Everything loaded from the data directory, read-only: rule checker, lexicon tables + resources, model.
pub struct Data { pub rules: Checker, pub tables: Tables, pub model: Model }

/// The data files (see the CLI's `data` module for where they live).
pub struct DataPaths<'p> {
    /// FST lexicon of the rule checker
    pub lex_dir: &'p Path,
    /// lexicon of the model
    pub lexicon_tsv: &'p Path,
    /// corpus resources of the model's candidate lists (`interp/`, `ngram/`)
    pub resources: &'p Path,
    /// model.frcm
    pub model: &'p Path,
}

impl Data {
    /// `int8`: model weights stored as int8 (less memory, scores differ slightly).
    pub fn load(p: &DataPaths, int8: bool, cache: &LexCache) -> Result<Data, String> {
        let rules = Checker::open(p.lex_dir).map_err(|e| format!("{}: {e}", p.lex_dir.display()))?;
        let model = Model::load_opts(&p.model.to_string_lossy(), int8).map_err(|e| format!("{}: {e}", p.model.display()))?;
        let tables = Tables::load(p.lexicon_tsv, p.resources, cache)?;
        Ok(Data { rules, tables, model })
    }
}

/// The checker over loaded `Data`. Holds no mutable state: share one between threads (`std::thread::scope`).
pub struct Vaugelas<'a> { pub data: &'a Data, pub scorer: Scorer<'a> }

impl<'a> Vaugelas<'a> {
    pub fn new(data: &'a Data) -> Vaugelas<'a> { Vaugelas { data, scorer: Scorer::new(&data.tables, &data.rules, &data.model) } }

    /// The threshold used when `Settings::threshold` is None: the model's own.
    pub fn default_threshold(&self) -> f64 { self.data.model.profile.threshold }

    /// Check `text`; issues sorted by position.
    pub fn check(&self, text: &str, s: &Settings) -> Vec<Issue> {
        let mut out = Vec::new();
        let mut off = 0usize;
        for raw in text.split('\n') {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if !line.trim().is_empty() { out.extend(self.check_line(line, s).into_iter().map(|mut i| { i.start += off; i.end += off; i })); }
            off += raw.len() + 1;
        }
        out
    }

    fn check_line(&self, line: &str, s: &Settings) -> Vec<Issue> {
        let mut issues: Vec<Issue> = Vec::new();
        if s.rules {
            let opts = CheckOptions { spell_suggestions: s.spell_suggestions, strict_typography: s.strict_typography };
            for l in self.data.rules.check_opts(line, &opts) {
                // a second rule on the same word: its suggestions join the first entry (one entry per word)
                if let Some(i) = issues.iter_mut().find(|i| i.start == l.start && i.end == l.end) {
                    for x in l.suggestions { if !i.replacements.contains(&x) { i.replacements.push(x); } }
                    continue;
                }
                issues.push(Issue { start: l.start, end: l.end, rule: l.rule.to_string(), message: l.message, replacements: l.suggestions,
                                    source: Source::Rules, score: None, category: Category::of_rule(l.rule) });
            }
        }
        if s.model {
            let thr = s.threshold.unwrap_or_else(|| self.default_threshold());
            for (sent, spans) in self.scorer.score_line(line) {
                for (wi, (score, sug)) in sent.scores.iter().enumerate() {
                    if *score <= thr { continue; }
                    let Some((start, end)) = spans[wi] else { continue };
                    let word = &sent.words[wi];
                    let sug = sug.clone().filter(|x| x != word);
                    if let Some(r) = issues.iter_mut().find(|i| i.source != Source::Model && i.start < end && start < i.end) {
                        r.source = Source::Both; r.score = Some(*score);
                        if let Some(x) = sug { if !r.replacements.contains(&x) { r.replacements.push(x); } }
                        continue;
                    }
                    let shown = &line[start..end];
                    let message = match &sug {
                        Some(x) => format!("Forme douteuse : « {shown} ». Forme proposée : « {x} »."),
                        None => format!("Forme douteuse : « {shown} »."),
                    };
                    issues.push(Issue { start, end, rule: MODEL_RULE.into(), message, replacements: sug.into_iter().collect(),
                                        source: Source::Model, score: Some(*score), category: Category::Grammar });
                }
            }
        }
        issues.sort_by_key(|i| (i.start, i.end));
        issues
    }
}
