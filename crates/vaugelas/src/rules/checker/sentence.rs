//! Sentence-level rules written from the errors the small translation models actually make.
//!
//! Every rule below comes from a recorded defect (issue #18, provenance
//! `agentic-reasoning-translator/experiments/2026-11-10-prompt-ceiling/`): the string that was wrong, the text
//! around it, and why a reader called it wrong. A rule that was not written from a recorded failure is not here.
//!
//! What the existing checker could not do, and what each rule adds:
//! - `FR_DET_NOUN_GEN` only looked at an adjacent determiner and noun, so `tous les malédictions` (feminine noun
//!   behind `les`) was invisible. [`Checker::quant_det_noun`] reads the quantifier through the article.
//! - no rule looked across a coordination, so `nos prières, dessein, promesses` (one noun left in the singular in a
//!   plural list) passed. [`Checker::noun_list_number`] walks the list.
//! - no rule looked at a sentence-initial bare plural subject, so `Conseils peuvent être donnés` passed.
//!   [`Checker::partitive_subject`].
//! - the elision apostrophe was not checked with a space after it: `j' reconnais`. [`Checker::apostrophe_space`].
//! - an infinitive used as a noun (`le murmurer et se quereller`) has noun position but verb morphology.
//!   [`Checker::infinitive_as_noun`].
//! - `si … mais qu'elle …` adds a `que` the conditional clause cannot take. [`Checker::si_mais_que`].
//!
//! Determinism: each finding records the deciding condition in its message; the rules are pure functions of the
//! token sequence and read neither ordering-sensitive collections nor hash maps.
use super::*;

/// Determinatives that can follow a quantifier: "tous **les** hommes", "toute **la** terre".
const ARTICLES: &[&str] = &["le", "la", "les", "l’", "du", "de", "d’", "des", "un", "une"];

/// Elided forms whose apostrophe must not be followed by a space.
const ELIDED: &[&str] = &["j’", "l’", "d’", "qu’", "c’", "n’", "m’", "t’", "s’"];

/// Infinitives that are ordinary nouns and must not be read as one used for a verb: "le dîner", "le pouvoir".
const NOUN_INFINITIVES: &[&str] = &[
    "dîner", "déjeuner", "souper", "goûter", "pouvoir", "savoir", "devoir", "vouloir", "être", "avoir", "faire",
    "dire", "rire", "sourire", "vivre", "boire", "manger", "parler", "souvenir", "venir", "avenir", "coucher",
    "lever", "reposer", "sortir", "rentrer", "passer", "toucher", "dire", "plaisir", "loisir", "manière",
];

fn noun_number(t: &Tok) -> u8 { t.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number) }
fn noun_gender(t: &Tok) -> u8 { t.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender) }
fn infinite(t: &Tok) -> bool { t.morph.iter().any(|m| m.pos == Pos::Verb && m.moods & M_Y != 0) }
fn only_infinitive(t: &Tok) -> bool {
    !t.morph.is_empty() && t.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_Y != 0)
}

impl Checker {
    pub(super) fn sentence_rules(&self, t: &[Tok], o: &mut Vec<Lint>) {
        self.quant_det_noun(t, o);
        self.noun_list_number(t, o);
        self.partitive_subject(t, o);
        self.apostrophe_space(t, o);
        self.infinitive_as_noun(t, o);
        self.si_mais_que(t, o);
    }

    /// "tous les malédictions": the quantifier agrees in gender with the noun it quantifies, even with an
    /// article between them. Deciding condition: the quantifier's gender is unambiguous and differs from the
    /// following noun's.
    fn quant_det_noun(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || !Self::same(t, i, i + 2) { continue; }
            let (qg, _qn) = match t[i].lower.as_str() {
                "tout" => (G_M, N_S), "toute" => (G_F, N_S), "tous" => (G_M, N_P), "toutes" => (G_F, N_P),
                _ => continue,
            };
            if !ARTICLES.contains(&t[i + 1].lower.as_str()) { continue; }
            if !Self::word_at(t, i + 2) { continue; }
            // the noun may be preceded by adjectives: "toutes les grandes maisons"
            let mut j = i + 2;
            while Self::word_at(t, j) && j <= i + 3 && t[j].morph.iter().any(adjish) && !has_pos(&t[j], Pos::Noun) { j += 1; }
            if !Self::word_at(t, j) || !Self::same(t, i, j) || !has_pos(&t[j], Pos::Noun) { continue; }
            let ng = noun_gender(&t[j]);
            if !single(ng) || ng == qg { continue; }
            let sug = match (t[i].lower.as_str(), ng) {
                ("tous", G_F) => "toutes", ("toutes", G_M) => "tous", ("tout", G_F) => "toute", ("toute", G_M) => "tout",
                _ => continue,
            };
            o.push(self.mk(&t[i], "FR_DET_NOUN_GEN",
                "Le quantificateur doit s’accorder en genre avec le nom qu’il détermine.", vec![sug.to_string()]));
        }
    }

    /// A plural determiner opens a coordinated list: every noun in the list must be plural.
    /// `nos prières, dessein, promesses` — deciding condition: the determiner's number is unambiguous, the
    /// first noun agrees with it, and a later noun of the same list is unambiguously singular.
    fn noun_list_number(&self, t: &[Tok], o: &mut Vec<Lint>) {
        let mut i = 0;
        while i + 1 < t.len() {
            i += 1;
            if !t[i - 1].word { continue; }
            let Some((_dg, dn)) = det_info(&t[i - 1]) else { continue };
            if !single(dn) { continue; }
            let Some(first) = (i..(i + 2).min(t.len())).find(|&k| Self::same(t, i - 1, k) && Self::word_at(t, k)
                && all_pos(&t[k], Pos::Noun)) else { continue };
            let fn_num = noun_number(&t[first]);
            if !single(fn_num) || fn_num != dn { continue; }
            let mut k = first + 1;
            let mut listed = 0usize;
            loop {
                // adjectives attached to the previous noun
                while k < t.len() && Self::same(t, i - 1, k) && t[k].word && t[k].morph.iter().any(adjish) && !has_fin(&t[k]) && !all_pos(&t[k], Pos::Noun) { k += 1; }
                if k >= t.len() || !Self::same(t, i - 1, k) { break; }
                let sep = !t[k].word && t[k].text == "," || t[k].word && matches!(t[k].lower.as_str(), "et" | "ou");
                if !sep { break; }
                k += 1;
                if k >= t.len() || !Self::word_at(t, k) || !Self::same(t, i - 1, k) || !all_pos(&t[k], Pos::Noun) { break; }
                let w = &t[k];
                let nn = noun_number(w);
                listed += 1;
                if single(nn) && nn != dn {
                    let Some(m) = w.morph.iter().find(|m| m.pos == Pos::Noun) else { break };
                    let sug = self.forms_where(self.lemma_of(m), |f| f.pos == Pos::Noun && f.number == dn);
                    if let Some(s) = sug.into_iter().next() {
                        o.push(self.mk(w, "FR_DET_NOUN_NUM",
                            "Le nom doit s’accorder en nombre avec le déterminant de la liste.", vec![s]));
                    }
                    break; // one finding per list
                }
                k += 1;
                if listed > 8 { break; }
            }
        }
    }

    /// `Conseils peuvent être donnés`: a sentence opens on a bare plural common noun immediately followed by a
    /// third-person plural finite verb, so the partitive article is missing. Deciding condition: the opening
    /// noun has only noun readings, is unambiguously plural, is not a proper noun, and the very next word is an
    /// unambiguous 3rd-plural verb (a comma or a conjunction would make it a coordination).
    fn partitive_subject(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            let w = &t[i];
            if !w.word || !w.sent_start || w.morph.iter().any(|m| m.pos == Pos::Proper) { continue; }
            if !all_pos(w, Pos::Noun) || w.text.chars().count() < 3 { continue; }
            let n = noun_number(w);
            if !single(n) || n != N_P { continue; }
            let v = &t[i + 1];
            if !Self::word_at(t, i + 1) || !Self::same(t, i, i + 1) || !v.morph.iter().all(|m| m.pos == Pos::Verb) { continue; }
            let ok = v.morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number == N_P);
            if !ok { continue; }
            o.push(self.mk(w, "FR_PARTITIVE", "Article partitif manquant devant un sujet pluriel.",
                vec![format!("Des {}", w.lower)]));
        }
    }

    /// `j' reconnais`: an elision apostrophe takes no space before the next word. The tokenizer splits such a
    /// form into `j` + `'` + `reconnais` when the apostrophe is followed by a space, so the rule reads the
    /// three-token sequence.
    fn apostrophe_space(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const BASE: &[&str] = &["j", "l", "d", "qu", "c", "n", "m", "t", "s"];
        let push = |start: usize, end: usize, o: &mut Vec<Lint>| {
            o.push(Lint {
                start,
                end,
                rule: "FR_TYPO_APOS_SPACE",
                message: "Pas d’espace après l’apostrophe d’élision.".to_string(),
                suggestions: vec![String::new()],
            });
        };
        for i in 0..t.len().saturating_sub(1) {
            let w = &t[i];
            if !w.word || !ELIDED.contains(&w.lower.as_str()) { continue; }
            let nx = &t[i + 1];
            if !nx.word || !nx.space_before || !Self::same(t, i, i + 1) { continue; }
            push(w.end.saturating_sub(1), nx.start, o);
        }
        for i in 0..t.len().saturating_sub(2) {
            let w = &t[i];
            if !w.word || !BASE.contains(&w.lower.as_str()) { continue; }
            let ap = &t[i + 1];
            if ap.word || !matches!(ap.text.as_str(), "'" | "’") || ap.start != w.end { continue; }
            let nx = &t[i + 2];
            if !nx.word || !nx.space_before || !Self::same(t, i, i + 2) { continue; }
            push(ap.start, nx.start, o);
        }
    }

    /// `le murmurer et se quereller`: a determiner followed by an infinitive that has no noun reading, then a
    /// coordinated infinitive. Deciding condition: both verbs are unambiguously infinitives (no noun reading on
    /// the first), which is what makes the determiner's noun phrase head a verb.
    fn infinitive_as_noun(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(3) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "le" | "la" | "les" | "l’" | "un" | "une" | "des") { continue; }
            if !Self::same(t, i, i + 1) || !Self::word_at(t, i + 1) { continue; }
            let v1 = &t[i + 1];
            if !only_infinitive(v1) || NOUN_INFINITIVES.contains(&v1.lower.as_str()) { continue; }
            let mut k = i + 2;
            if !(k < t.len() && Self::same(t, i, k) && t[k].word && matches!(t[k].lower.as_str(), "et" | "ou")) { continue; }
            k += 1;
            // "et se quereller": a clitic may precede the second infinitive
            while k < t.len() && Self::same(t, i, k) && t[k].word && CLITICS.contains(&t[k].lower.as_str()) { k += 1; }
            if k >= t.len() || !Self::same(t, i, k) || !Self::word_at(t, k) || !infinite(&t[k]) { continue; }
            let words: Vec<&str> = t[i + 1..=k].iter().map(|x| x.text.as_str()).collect();
            o.push(Lint {
                start: t[i].start,
                end: t[k].end,
                rule: "FR_INF_NOUN",
                message: "Un infinitif ne peut pas être le nom d’un groupe nominal : « le fait de … ».".to_string(),
                suggestions: vec![format!("le fait de {}", words.join(" "))],
            });
        }
    }

    /// `si … mais qu'elle éclate`: a `si` conditional clause cannot be continued by `mais que` + a finite verb.
    /// Deciding condition: a `si` opens a clause, the same sentence later has `mais` immediately before `qu’`.
    fn si_mais_que(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !Self::word_at(t, i) || t[i].lower != "si" { continue; }
            let clause_start = t[i].sent_start || i == 0
                || (!t[i - 1].word && matches!(t[i - 1].text.as_str(), "," | ";" | ":" | "«" | "—"));
            if !clause_start { continue; }
            let mut k = i + 1;
            while k + 1 < t.len() && t[k].sid == t[i].sid {
                if t[k].word && t[k].lower == "mais" && Self::word_at(t, k + 1) && matches!(t[k + 1].lower.as_str(), "que" | "qu’") {
                    let after = t.get(k + 2);
                    // the conditional clause has to be closed by a comma before "mais"
                    let closed = (i + 1..k).any(|j| !t[j].word && t[j].text == ",");
                    if closed && after.is_some_and(|a| a.word && (has_fin(a) || a.morph.iter().any(|m| m.pos == Pos::PronSubj))) {
                        o.push(Lint {
                            start: t[k].start,
                            end: t[k].end,
                            rule: "FR_SI_MAIS_QUE",
                            message: "Une proposition en « si » ne se poursuit pas par « mais que ».".to_string(),
                            suggestions: vec!["et".to_string()],
                        });
                    }
                    break;
                }
                k += 1;
            }
        }
    }
}
