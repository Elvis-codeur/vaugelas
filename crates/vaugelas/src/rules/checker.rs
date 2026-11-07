//! Spelling + grammar rules over morphological readings.
use crate::rules::lexicon::*;
use crate::rules::tokenizer::{tokenize, Tok};
use std::path::Path;

mod hard;

#[derive(Clone, Debug)]
pub struct Lint {
    pub start: usize, // byte offsets into the checked text
    pub end: usize,
    pub rule: &'static str,
    pub message: String,
    pub suggestions: Vec<String>,
}

pub struct Checker {
    pub lex: Lexicon,
}

/// What `check_opts` computes.
#[derive(Clone, Copy, Debug)]
pub struct CheckOptions {
    /// Compute spelling suggestions now (eager). When false, FR_SPELL lints come back with no
    /// suggestions; ask for them later with `Checker::suggest_spelling`.
    pub spell_suggestions: bool,
    /// Also report typography that is only a convention in plain text: a normal space where a
    /// (narrow) non-breaking space is expected, `...` instead of `…`, straight apostrophes.
    pub strict_typography: bool,
}

impl Default for CheckOptions {
    fn default() -> Self { CheckOptions { spell_suggestions: true, strict_typography: false } }
}

const CLITICS: [&str; 17] = [
    "ne", "n’", "me", "m’", "te", "t’", "se", "s’", "le", "la", "l’", "les", "lui", "leur", "y", "en", "nous",
];
const CLITICS2: [&str; 1] = ["vous"];
const ADVERBS: [&str; 22] = [
    "très", "trop", "si", "tout", "bien", "vraiment", "assez", "plus", "moins", "aussi", "fort", "peu",
    "déjà", "encore", "toujours", "souvent", "pas", "jamais", "enfin", "tant", "point", "guère",
];
const PREPS: &[&str] = &[
    "de", "d’", "du", "des", "à", "au", "aux", "en", "dans", "sur", "avec", "pour", "par", "sans", "sous",
    "chez", "vers", "contre", "entre", "devant", "derrière", "près", "auprès", "autour", "loin", "parmi",
    "malgré", "pendant", "durant", "depuis", "selon", "envers", "hors", "sauf", "outre", "après", "avant",
    "dès", "face", "lors", "via", "au-dessus", "au-dessous", "à côté",
];
/// Nouns heading time expressions ("cette nuit", "le lendemain"): such a noun phrase is not a direct object.
const TIME_NOUNS: &[&str] = &[
    "nuit", "matin", "soir", "jour", "journée", "semaine", "année", "an", "mois", "veille", "lendemain", "fois", "heure",
    "après-midi", "été", "hiver", "printemps", "automne", "week-end", "lundi", "mardi", "mercredi", "jeudi", "vendredi",
    "samedi", "dimanche", "soirée", "matinée", "nuitée", "siècle", "instant", "moment",
];
const COLLECTIVE: [&str; 24] = [
    "plupart", "majorité", "minorité", "moitié", "tiers", "quart", "quantité", "nombre", "partie", "totalité",
    "reste", "série", "foule", "multitude", "infinité", "groupe", "ensemble", "paire", "couple", "dizaine",
    "centaine", "douzaine", "millier", "million",
];
/// Participles that stay invariable after "que" + avoir (no real direct object).
const PP_INVARIABLE: [&str; 14] = [
    "coûter", "valoir", "durer", "peser", "vivre", "courir", "mesurer", "régner", "être", "dormir", "marcher",
    "pleuvoir", "falloir", "pouvoir",
];
/// Verbs whose pronoun is an indirect object: participle stays invariable (ils se sont parlé).
const PP_INVARIABLE_PRON: &[&str] = &[
    "coûter", "valoir", "durer", "peser", "être", "dire", "demander", "écrire", "parler", "téléphoner", "plaire",
    "déplaire", "rire", "sourire", "mentir", "nuire", "succéder", "ressembler", "convenir", "suffire", "complaire",
    "répondre", "promettre", "permettre", "faire", "souhaiter", "rendre", "reprocher", "accorder", "succéder",
    // references disagree ("elle s'est joué(e) de lui"): accept both forms, flag neither
    "jouer",
];

fn is_fin(m: &Morph) -> bool { m.pos == Pos::Verb && m.moods & M_FIN != 0 }
fn has_fin(t: &Tok) -> bool { t.morph.iter().any(is_fin) }
fn has_pos(t: &Tok, p: Pos) -> bool { t.morph.iter().any(|m| m.pos == p) }
fn all_pos(t: &Tok, p: Pos) -> bool { !t.morph.is_empty() && t.morph.iter().all(|m| m.pos == p) }
fn adjish(m: &Morph) -> bool { m.pos == Pos::Adj || (m.pos == Pos::Verb && m.moods & M_Q != 0) }
fn single(bits: u8) -> bool { bits == 1 || bits == 2 }

fn det_info(t: &Tok) -> Option<(u8, u8)> {
    let (mut g, mut n, mut any) = (0u8, 0u8, false);
    for m in &t.morph { if m.pos.is_det() { any = true; g |= m.gender; n |= m.number; } }
    if any { return Some((g, n)); }
    // contractions tagged only as prepositions, and cardinal numbers above one
    match t.lower.as_str() {
        "au" | "du" => Some((G_M, N_S)),
        "aux" | "des" => Some((3, N_P)),
        "deux" | "trois" | "quatre" | "cinq" | "six" | "sept" | "huit" | "neuf" | "dix" | "onze" | "douze" | "quinze" | "vingt" | "trente" | "cent" | "mille" | "plusieurs" => Some((3, N_P)),
        _ => None,
    }
}

fn pron_pn(l: &str) -> Option<(u8, u8)> {
    Some(match l {
        "je" | "j’" => (1, 1), "tu" => (2, 1), "il" | "elle" | "on" => (4, 1),
        "nous" => (1, 2), "vous" => (2, 2), "ils" | "elles" => (4, 2),
        _ => return None,
    })
}
fn pron_gn(l: &str) -> Option<(u8, u8)> {
    Some(match l { "il" => (G_M, N_S), "elle" => (G_F, N_S), "ils" => (G_M, N_P), "elles" => (G_F, N_P), _ => return None })
}

fn match_case(orig: &str, s: &str) -> String {
    let n = orig.chars().count();
    let first_up = orig.chars().next().map_or(false, |c| c.is_uppercase());
    if n > 1 && orig.chars().all(|c| !c.is_lowercase()) { return s.to_uppercase(); }
    if first_up {
        let mut c = s.chars();
        return match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() };
    }
    s.to_string()
}

struct Np { head: usize, end: usize, gender: u8, number: u8, alt: u8 }

impl Checker {
    pub fn open(dir: &Path) -> Result<Checker, Box<dyn std::error::Error>> {
        Ok(Checker { lex: Lexicon::open(dir)? })
    }

    fn prepare(&self, tok: &mut Tok) {
        let (known, morph) = self.lex.analyze(&tok.text);
        if known { tok.known = true; tok.morph = morph; return; }
        if tok.text.chars().any(|c| c.is_ascii_digit()) { tok.known = true; return; }
        if tok.text.contains('-') {
            // verb + inverted subject pronoun: "avez-vous", "a-t-il", "auraient-il" -> verb readings + pronoun features
            let parts: Vec<&str> = tok.lower.split('-').collect();
            if parts.len() >= 2 {
                if let Some(pn) = pron_pn(parts[parts.len() - 1]).or(if parts[parts.len() - 1] == "ce" { Some((4, 1)) } else { None }) {
                    let (k, m) = self.lex.analyze(parts[0]);
                    // "préviens-nous", "dites-vous" after an imperative: nous/vous are objects, not inverted subjects
                    if matches!(parts[parts.len() - 1], "nous" | "vous") && m.iter().any(|x| x.moods & M_E != 0) && parts.len() == 2 && !m.iter().any(|x| is_fin(x) && x.person & pn.0 != 0 && x.number & pn.1 != 0) {
                        tok.known = true; tok.compound = true; return;
                    }
                    let verb: Vec<Morph> = m.into_iter().filter(|x| is_fin(x)).collect();
                    if k && !verb.is_empty() && parts[1..parts.len() - 1].iter().all(|p| *p == "t") {
                        tok.known = true; tok.morph = verb; tok.inv = Some(pn); return;
                    }
                }
            }
            let ok = tok.text.split('-').all(|p| p.is_empty() || p == "t" || self.lex.known(p));
            if ok { tok.known = true; tok.compound = true; }
        }
    }

    pub fn check(&self, text: &str) -> Vec<Lint> { self.check_opts(text, &CheckOptions::default()) }

    /// Spelling suggestions for one word (for callers using `spell_suggestions: false`).
    pub fn suggest_spelling(&self, word: &str) -> Vec<String> {
        self.lex.suggest(word, 5).into_iter().map(|s| match_case(word, &s)).collect()
    }

    pub fn check_opts(&self, text: &str, opts: &CheckOptions) -> Vec<Lint> {
        let mut t = tokenize(text);
        for tok in t.iter_mut() { if tok.word { self.prepare(tok); } }
        let mut o: Vec<Lint> = Vec::new();
        self.spell(&t, opts.spell_suggestions, &mut o);
        self.dup(&t, text, &mut o);
        self.punct(&t, &mut o);
        o.extend(crate::rules::typography::check(text, opts.strict_typography));
        self.det_noun(&t, &mut o);
        self.noun_adj(&t, &mut o);
        self.subject_verb(&t, &mut o);
        self.qui_relative(&t, &mut o);
        self.orphan_person(&t, &mut o);
        self.inversion(&t, &mut o);
        self.pp_cod(&t, &mut o);
        self.pp_pronominal(&t, &mut o);
        self.avoir_inf(&t, &mut o);
        self.inf_pp(&t, &mut o);
        self.subjunctive(&t, &mut o);
        self.si_cond(&t, &mut o);
        self.adv_verb(&t, &mut o);
        self.etre_plural_attr(&t, &mut o);
        self.hard_rules(&t, &mut o);
        self.homophones(&t, &mut o);
        o.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));
        o.dedup_by(|a, b| a.start == b.start && a.end == b.end && a.rule == b.rule);
        o
    }

    // ---------- helpers ----------
    fn lemma_of(&self, m: &Morph) -> &str { self.lex.lemma(m.lemma) }
    fn fin_lemma(&self, t: &Tok, l: &str) -> bool { t.morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == l) }
    fn same(t: &[Tok], i: usize, j: usize) -> bool { j < t.len() && t[j].sid == t[i].sid }
    fn word_at(t: &[Tok], i: usize) -> bool { i < t.len() && t[i].word }

    fn skip_clitics(&self, t: &[Tok], mut j: usize, from: usize) -> usize {
        while j < t.len() && t[j].word && t[j].sid == t[from].sid
            && (CLITICS.contains(&t[j].lower.as_str()) || CLITICS2.contains(&t[j].lower.as_str())) { j += 1; }
        j
    }

    fn forms_where(&self, lemma: &str, pred: impl Fn(&Morph) -> bool) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for (f, m) in self.lex.forms(lemma) { if pred(&m) && !v.contains(&f) { v.push(f); } }
        v
    }

    /// adjective or participle form of `lemma` for gender/number
    fn inflect_adj(&self, lemma: &str, part: bool, g: u8, n: u8) -> Vec<String> {
        let g = if single(g) { g } else { G_M };
        let kind = |m: &Morph| if part { m.pos == Pos::Verb && m.moods & M_Q != 0 } else { m.pos == Pos::Adj };
        let exact = self.forms_where(lemma, |m| kind(m) && m.gender == g && m.number == n);
        if !exact.is_empty() { return exact; }
        self.forms_where(lemma, |m| kind(m) && m.gender & g != 0 && m.number & n != 0)
    }

    fn mk(&self, t: &Tok, rule: &'static str, msg: &str, sugg: Vec<String>) -> Lint {
        Lint { start: t.start, end: t.end, rule, message: msg.to_string(),
               suggestions: sugg.into_iter().map(|s| match_case(&t.text, &s)).collect() }
    }

    // ---------- noun phrases ----------
    fn np_core(&self, t: &[Tok], i: usize) -> Option<Np> {
        if !Self::word_at(t, i) { return None; }
        if matches!(t[i].lower.as_str(), "de" | "d’") { return None; }
        let (dg, dn) = det_info(&t[i])?;
        let mut j = i + 1;
        let mut last = i;
        let (mut pure, mut any) = (None::<usize>, None::<usize>);
        while Self::word_at(t, j) && Self::same(t, i, j) && j <= i + 5 {
            let tk = &t[j];
            // a preposition or an auxiliary ends the noun phrase even when it is also a noun ("devant", "a", "été")
            if PREPS.contains(&tk.lower.as_str()) || self.fin_lemma(tk, "être") || self.fin_lemma(tk, "avoir") { break; }
            if det_info(tk).is_some() && !tk.morph.iter().any(adjish) { break; } // "toute la longue cérémonie": "la" is not a noun here
            if has_pos(tk, Pos::PronSubj) { break; } // "Quels beaux taille-crayons tu as achetés"
            // "Dans la vallée vit des loups", "Sous le pont passe ...": a finite-verb homograph after the head ends the phrase
            if (pure.is_some() || any.is_some()) && has_fin(tk) && !tk.morph.iter().any(adjish) { break; }
            let not_head = ADVERBS.contains(&tk.lower.as_str()) || matches!(tk.lower.as_str(), "fois" | "être" | "plus" | "moins" | "mieux" | "peu" | "bien");
            let nounish = !not_head && tk.morph.iter().any(|m| m.pos == Pos::Noun && m.number & dn != 0);
            let nounish_any = !not_head && has_pos(tk, Pos::Noun);
            let aj = tk.morph.iter().any(adjish);
            let adv = ADVERBS.contains(&tk.lower.as_str()) && has_pos(tk, Pos::Adv);
            if !(nounish_any || aj || adv) { break; }
            if nounish && !aj && pure.is_none() { pure = Some(j); }
            if nounish && any.is_none() { any = Some(j); }
            last = j;
            j += 1;
        }
        let head = pure.or(any)?;
        let (mut ng, mut nn) = (0u8, 0u8);
        for m in &t[head].morph { if m.pos == Pos::Noun && m.number & dn != 0 { ng |= m.gender; nn |= m.number; } }
        let mut number = if single(dn) { dn } else if single(nn) { nn } else { 3 };
        let mut gender = if single(ng) { ng } else if single(dg) && ng == 3 { dg } else if ng == 0 { dg } else { ng };
        if t[i].lower == "quelque" && t[head].lower == "chose" { gender = G_M; } // "quelque chose" is neuter -> masculine
        if t[i].lower == "quelque" && Self::word_at(t, i + 1) && matches!(t[i + 1].lower.as_str(), "deux" | "trois" | "dix" | "vingt" | "cinquante" | "cent" | "mille") { number = N_P; }
        Some(Np { head, end: last, gender, number, alt: 0 })
    }

    fn np_at(&self, t: &[Tok], i: usize) -> Option<Np> {
        let mut np = self.np_core(t, i)?;
        let mut pp_used = false;
        let coll = t[np.head].morph.iter().any(|m| m.pos == Pos::Noun && COLLECTIVE.contains(&self.lemma_of(m)))
            || matches!(t[np.head].lower.as_str(), "vingtaine" | "trentaine" | "quarantaine" | "cinquantaine" | "soixantaine" | "quinzaine" | "huitaine" | "majorité" | "minorité" | "masse" | "poignée" | "kyrielle" | "flopée");
        if coll { np.alt = 3; } // "un grand nombre dorment", "la plupart sont": plural verb accepted
        for _ in 0..4 {
            let k = np.end + 1;
            if !Self::word_at(t, k) || !Self::same(t, i, k) { break; }
            let lw = t[k].lower.as_str();
            if !PREPS.contains(&lw) { break; }
            let mut start = if matches!(lw, "du" | "des" | "au" | "aux") { k } else { k + 1 };
            // "près de la gare", "loin des villes": locative + de
            if !matches!(lw, "de" | "d’" | "du" | "des") && Self::word_at(t, start) && matches!(t[start].lower.as_str(), "de" | "d’") { start += 1; }
            if let Some(n2) = self.np_core(t, start) {
                if coll { np.alt = 3; }
                np.end = n2.end;
                pp_used = true;
                continue;
            }
            if Self::word_at(t, start) && Self::same(t, i, start) && has_pos(&t[start], Pos::Noun) && !has_fin(&t[start]) && det_info(&t[start]).is_none() {
                if coll { np.alt = 3; }
                let mut e = start;
                while Self::word_at(t, e + 1) && t[e + 1].morph.iter().any(adjish) && !has_fin(&t[e + 1]) && !has_pos(&t[e + 1], Pos::Prep) { e += 1; }
                np.end = e;
                pp_used = true;
                continue;
            }
            break;
        }
        // coordination: NP et NP => plural (not after a prepositional phrase: "du ciel et de la terre")
        loop {
            let k = np.end + 1;
            if !pp_used && Self::word_at(t, k) && Self::same(t, i, k) && t[k].lower == "et" {
                if let Some(n2) = self.np_core(t, k + 1) {
                    np.end = n2.end; np.number = N_P; np.gender = if np.gender == G_F && n2.gender == G_F { G_F } else { G_M }; np.alt = 0;
                    continue;
                }
            }
            break;
        }
        Some(np)
    }

    /// True when position i can start a clause (so an NP there is a subject).
    fn subj_ctx(&self, t: &[Tok], i: usize) -> bool {
        if t[i].sent_start || i == 0 { return true; }
        let p = &t[i - 1];
        if !p.word { return matches!(p.text.as_str(), "," | "«" | "—" | "(" | ":" | ";" | "\"" | "“"); }
        p.sid == t[i].sid && p.morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::ConjS))
    }

    // ---------- spelling ----------
    fn spell(&self, t: &[Tok], suggest: bool, o: &mut Vec<Lint>) {
        for k in 0..t.len() {
            let tk = &t[k];
            if !tk.word || tk.known || tk.skip || tk.compound { continue; }
            if tk.text.chars().count() < 2 { continue; }
            let first_up = tk.text.chars().next().map_or(false, |c| c.is_uppercase());
            if first_up { continue; } // proper nouns, acronyms, sentence-initial names
            if tk.text.chars().any(|c| !c.is_alphabetic() && c != '’' && c != '\'' && c != '-') { continue; }
            let sugg = if suggest { self.lex.suggest(&tk.text, 5) } else { Vec::new() };
            o.push(self.mk(tk, "FR_SPELL", "Faute d’orthographe possible.", sugg));
        }
    }

    fn dup(&self, t: &[Tok], text: &str, o: &mut Vec<Lint>) {
        const OK: [&str; 15] = ["nous", "vous", "très", "tout", "bien", "plus", "si", "peu", "pas", "non", "oui", "ça", "que", "coûte", "en"];
        for k in 0..t.len().saturating_sub(1) {
            let (a, b) = (&t[k], &t[k + 1]);
            if a.word && b.word && a.sid == b.sid && a.lower == b.lower && a.lower.chars().count() > 1
                && !OK.contains(&a.lower.as_str()) && !a.skip && a.lower.chars().all(|c| c.is_alphabetic() || c == '’') {
                o.push(Lint { start: a.start, end: b.end, rule: "FR_DUP", message: "Mot répété.".into(),
                              suggestions: vec![text[a.start..a.end].to_string()] });
            }
        }
    }

    fn punct(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for k in 1..t.len() {
            let tk = &t[k];
            if tk.word || tk.skip { continue; }
            let prev = &t[k - 1];
            if matches!(tk.text.as_str(), "," | ".") && tk.space_before && prev.word {
                o.push(Lint { start: prev.end, end: tk.end, rule: "FR_SPACE_BEFORE", message: "Pas d’espace avant cette ponctuation.".into(),
                              suggestions: vec![tk.text.clone()] });
            }
            if tk.text == "," && !tk.space_before && prev.word && k + 1 < t.len() && t[k + 1].word && !t[k + 1].space_before
                && !t[k + 1].text.chars().next().map_or(false, |c| c.is_ascii_digit()) && !prev.skip {
                o.push(Lint { start: tk.start, end: tk.end, rule: "FR_SPACE_AFTER", message: "Espace manquant après la virgule.".into(),
                              suggestions: vec![", ".into()] });
            }
        }
    }

    // ---------- agreement inside the noun phrase ----------
    fn det_noun(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !Self::word_at(t, i + 1) || !Self::same(t, i, i + 1) { continue; }
            let num_det = matches!(t[i].lower.as_str(), "deux" | "trois" | "quatre" | "cinq" | "six" | "sept" | "huit" | "neuf" | "dix" | "douze" | "vingt" | "cent" | "mille" | "plusieurs");
            let Some((dg, dn)) = det_info(&t[i]).or(if num_det { Some((3, N_P)) } else { None }) else { continue };
            if t[i].lower == "tout" { continue; } // "tout yeux, tout oreilles"
            if num_det && i > 0 && t[i - 1].word && t[i - 1].lower == "le" { continue; } // date: "le vingt août"
            if i > 0 && t[i - 1].word && (t[i - 1].morph.iter().any(|m| m.pos == Pos::DetNum)
                || matches!(t[i - 1].lower.as_str(), "cent" | "mille" | "vingt" | "trente" | "quarante" | "cinquante" | "soixante" | "et")) { continue; }
            // determiner readings only (not a pronoun use such as "les" in "il les mange")
            if t[i].morph.iter().any(|m| matches!(m.pos, Pos::PronSubj)) { continue; }
            let nx = &t[i + 1];
            if !all_pos(nx, Pos::Noun) { continue; }
            let (mut ng, mut nn) = (0u8, 0u8);
            for m in &nx.morph { ng |= m.gender; nn |= m.number; }
            if t[i].lower == "un" || t[i].lower == "une" {
                // numeral/pronoun ambiguity: still check plural noun
            }
            if single(dn) && single(nn) && dn != nn && !matches!(t[i].lower.as_str(), "l’" | "d’" | "qu’") {
                let target = dn;
                let mut sug = Vec::new();
                for m in &nx.morph {
                    sug.extend(self.forms_where(self.lemma_of(m), |f| f.pos == Pos::Noun && f.number == target && f.gender & m.gender != 0));
                    if !sug.is_empty() { break; }
                }
                o.push(self.mk(nx, "FR_DET_NOUN_NUM", "Le nom doit s’accorder en nombre avec le déterminant.", sug));
                continue;
            }
            if single(dg) && single(ng) && dg != ng && matches!(t[i].lower.as_str(), "le" | "la" | "un" | "une" | "ce" | "cette" | "mon" | "ma" | "ton" | "ta" | "son" | "sa") {
                let swap = match t[i].lower.as_str() {
                    "le" => "la", "la" => "le", "un" => "une", "une" => "un", "ce" => "cette", "cette" => "ce",
                    "mon" => "ma", "ma" => "mon", "ton" => "ta", "ta" => "ton", "son" => "sa", _ => "son",
                };
                o.push(self.mk(&t[i], "FR_DET_NOUN_GEN", "Le déterminant doit s’accorder en genre avec le nom.", vec![swap.to_string()]));
            }
        }
    }

    fn noun_adj(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            let Some(np) = self.np_core(t, i) else { continue };
            if !single(np.number) { continue; }
            if t[np.head].lower == "air" { continue; } // "avoir l'air heureuses": the adjective follows the subject
            // the NP is a complement ("les sorties de l'eau connues"): the adjective may agree with the noun before "de"
            let higher: Option<(u8, u8)> = if i >= 2 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "de" | "d’" | "à" | "en" | "pour" | "sur" | "dans" | "avec" | "par") {
                (i.saturating_sub(8)..i - 1).filter_map(|s0| self.np_core(t, s0)).find(|n0| n0.end == i - 2).map(|n0| (n0.gender, n0.number))
            } else if matches!(t[i].lower.as_str(), "du" | "des" | "au" | "aux") && i >= 1 {
                (i.saturating_sub(7)..i).filter_map(|s0| self.np_core(t, s0)).find(|n0| n0.end == i - 1).map(|n0| (n0.gender, n0.number))
            } else { None };
            // coordinated nouns: "une jupe et un manteau neufs" (plural, masculine unless both feminine) or nearest-noun agreement
            let mut coord: Option<(u8, u8)> = None;
            if i > 0 && t[i - 1].word && t[i - 1].morph.iter().any(|m| m.pos == Pos::ConjC) {
                if !matches!(t[i - 1].lower.as_str(), "et" | "ou") || i < 2 { continue; }
                match (i.saturating_sub(8)..i - 1).filter_map(|s0| self.np_core(t, s0)).find(|n0| n0.end == i - 2) {
                    Some(n0) if single(n0.gender) && single(np.gender) => coord = Some((if n0.gender == G_F && np.gender == G_F { G_F } else { G_M }, N_P)),
                    _ => continue,
                }
            }
            if i > 0 && t[i - 1].text == "," { continue; }
            let mut a = np.head + 1;
            while a <= np.end {
                let ta = &t[a];
                if !ta.known || has_fin(ta) || !ta.morph.iter().any(adjish) { break; }
                if ta.morph.iter().any(|m| m.pos == Pos::Noun) && !has_pos(ta, Pos::Adj) { break; }
                if ta.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv | Pos::ConjC | Pos::ConjS) || m.pos.is_det() || m.moods & M_P != 0) { break; }
                if self.colour_compound(t, a) { break; } // "bleu clair", "vert foncé", "marron": invariable
                if matches!(ta.lower.as_str(), "tel" | "telle" | "tels" | "telles" | "tout" | "toute" | "tous" | "toutes" | "même" | "mêmes" | "autre" | "autres" | "quelque" | "quelques") { break; }
                let agrees = |g: u8, n: u8| ta.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0 || !single(g)));
                if coord.map_or(false, |(g, n)| agrees(g, n)) || higher.map_or(false, |(g, n)| agrees(g, n)) { a += 1; continue; }
                if !agrees(np.gender, np.number) {
                    let Some(m0) = ta.morph.iter().find(|m| adjish(m)) else { break };
                    let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, np.gender, np.number);
                    o.push(self.mk(ta, "FR_NOUN_ADJ", "L’adjectif doit s’accorder avec le nom.", sug));
                }
                a += 1;
            }
            // "les conditions de travail décente": an adjective right after a prepositional chain
            if let Some(full) = self.np_at(t, i) {
                let e = full.end + 1;
                if full.end > np.end && Self::word_at(t, e) && t[e].sid == t[i].sid && single(full.number) {
                    let ta = &t[e];
                    let last = &t[full.end];
                    if ta.known && has_pos(ta, Pos::Adj) && !has_fin(ta) && !has_pos(ta, Pos::Noun) && !self.colour_compound(t, e)
                        && !ta.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv | Pos::ConjC | Pos::ConjS) || m.pos.is_det()) {
                        let ok = |g: u8, n: u8| ta.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0 || !single(g)));
                        let lg = last.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender);
                        let ln = last.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number);
                        if !ok(full.gender, full.number) && !(ln != 0 && ok(lg, ln)) {
                            if let Some(m0) = ta.morph.iter().find(|m| adjish(m)) {
                                let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, full.gender, full.number);
                                o.push(self.mk(ta, "FR_NOUN_ADJ", "L’adjectif doit s’accorder avec le nom.", sug));
                            }
                        }
                    }
                }
            }
        }
    }

    // ---------- subject / verb, attribute ----------
    fn verb_forms_for(&self, v: &Tok, person: u8, number: u8) -> Vec<String> {
        let Some(r) = v.morph.iter().find(|m| is_fin(m)) else { return vec![] };
        let mood = [M_IP, M_IQ, M_IF, M_IS, M_K, M_SQ, M_SP].into_iter().find(|b| r.moods & b != 0).unwrap_or(M_IP);
        self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & mood != 0 && f.person & person != 0 && f.number & number != 0)
    }

    fn subject_verb(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word { continue; }
            // pronoun subject
            if let Some((p, n)) = pron_pn(&t[i].lower) {
                // "nous"/"vous" are often objects (qui vous empêche, dont nous a parlé): subject only at a clause start
                let skip_obj = matches!(t[i].lower.as_str(), "nous" | "vous") && !(t[i].sent_start || i == 0
                    || (!t[i - 1].word && matches!(t[i - 1].text.as_str(), ";" | ":" | "«" | "—"))
                    || (t[i - 1].word && matches!(t[i - 1].lower.as_str(), "si" | "quand" | "lorsque" | "comme" | "puisque")))
                    || (i.saturating_sub(2)..i).any(|k| t[k].word && matches!(t[k].lower.as_str(), "qui" | "que" | "qu’" | "dont" | "où"));
                let in_coord = i > 0 && t[i - 1].word && (matches!(t[i - 1].lower.as_str(), "et" | "ou" | "ni") || PREPS.contains(&t[i - 1].lower.as_str()) || has_pos(&t[i - 1], Pos::Prep));
                if !skip_obj && !in_coord {
                    let j = self.skip_clitics(t, i + 1, i);
                    if Self::word_at(t, j) && Self::same(t, i, j) && has_fin(&t[j]) && t[j].inv.is_none() {
                        let agrees = t[j].morph.iter().any(|m| is_fin(m) && m.person & p != 0 && m.number & n != 0);
                        if !agrees {
                            let sug = self.verb_forms_for(&t[j], p, n);
                            o.push(self.mk(&t[j], "FR_SUBJ_VERB", "Le verbe doit s’accorder avec son sujet.", sug));
                        }
                        // attribute agreement: elle est content
                        if let Some((g, nn)) = pron_gn(&t[i].lower) { self.attribute(t, i, j, g, nn, o); }
                        if agrees { self.vp_coord(t, i, j, p, n, o); }
                    }
                }
                continue;
            }
            // NP subject
            if !self.subj_ctx(t, i) || matches!(t[i].lower.as_str(), "de" | "d’") { continue; }
            // second conjunct of a coordination ("son père et sa mère ..."): the subject starts earlier
            if i >= 2 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "et" | "ou" | "ni")
                && t[i - 2].sid == t[i].sid
                && (!t[i - 2].word || t[i - 2].morph.iter().any(|m| matches!(m.pos, Pos::Noun | Pos::Proper | Pos::Adj | Pos::PronSubj | Pos::PronObj | Pos::PronOther) || adjish(m))) { continue; }
            // a list of subjects started at the beginning of the sentence: "le protestantisme, le jansénisme, le rationalisme étaient ..."
            if i >= 1 && t[i - 1].text == "," {
                let ss = (0..i).rev().find(|&k| t[k].sent_start).unwrap_or(0);
                if t[ss].word && det_info(&t[ss]).is_some() && !(ss..i).any(|k| t[k].word && has_fin(&t[k]) && t[k].morph.iter().all(|m| m.pos == Pos::Verb)) { continue; }
            }
            if matches!(t[i].lower.as_str(), "quel" | "quelle" | "quels" | "quelles" | "combien") { continue; } // interrogative phrase
            if i >= 2 && matches!(t[i - 1].lower.as_str(), "que" | "qu’") && matches!(t[i - 2].lower.as_str(), "tel" | "telle" | "tels" | "telles" | "ainsi") { continue; }
            let Some(np) = self.np_at(t, i) else { continue };
            if TIME_NOUNS.contains(&t[np.head].lower.as_str()) && t.get(np.end + 1).map_or(false, |x| x.text == ",") { continue; } // "Chaque été, ..."
            if Self::word_at(t, np.end + 2) && matches!(t[np.end + 1].lower.as_str(), "et" | "ou") && t[np.end + 2].lower.split('-').next().map_or(false, |p| matches!(p, "moi" | "toi" | "nous" | "vous" | "lui" | "elle" | "eux" | "elles")) { continue; }
            if Self::word_at(t, np.end + 1) && has_pos(&t[np.end + 1], Pos::PronSubj) && !has_pos(&t[np.end + 1], Pos::PronObj) { continue; }
            if Self::word_at(t, np.end + 1) && matches!(t[np.end + 1].lower.as_str(), "nous" | "vous") && t[np.end + 1].sid == t[i].sid
                && Self::word_at(t, np.end + 2) && t[np.end + 2].morph.iter().any(|m| is_fin(m) && m.number == N_P && m.person & 3 != 0) { continue; }
            let Some(j) = self.main_verb_after(t, i, np.end + 1) else { continue };
            if !Self::word_at(t, j) || !Self::same(t, i, j) { continue; }
            let v = &t[j];
            if v.inv.is_some() { continue; } // "Quelles villes avez-vous ...": the attached pronoun is the subject
            if v.lower == "compris" && j > 0 && t[j - 1].lower == "y" { continue; } // "y compris"
            if !has_fin(v) || !(v.morph.iter().all(|m| m.pos == Pos::Verb) || self.fin_lemma(v, "être") || self.fin_lemma(v, "avoir")) { continue; }
            if np.number != 3 {
                let ok = |n: u8| v.morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & n != 0);
                if !ok(np.number) && !(np.alt != 0 && ok(np.alt)) {
                    let sug = self.verb_forms_for(v, 4, np.number);
                    o.push(self.mk(v, "FR_SUBJ_VERB", "Le verbe doit s’accorder avec son sujet.", sug));
                } else if np.alt == 0 {
                    self.vp_coord(t, i, j, 4, np.number, o);
                }
            }
            if np.alt == 0 { self.attribute(t, i, j, np.gender, np.number, o); }
        }
    }

    /// "Les enfants jouent dehors et rentrent tard": a verb coordinated after the subject's verb shares its subject.
    fn vp_coord(&self, t: &[Tok], i: usize, j: usize, p: u8, n: u8, o: &mut Vec<Lint>) {
        let lim = (j + 20).min(t.len());
        let mut k = j + 1;
        while k < lim && t[k].sid == t[i].sid {
            let x = &t[k];
            if !x.word { return; } // never cross punctuation: ", et nous ..." starts a new clause
            if matches!(x.lower.as_str(), "qui" | "que" | "qu’" | "dont" | "où" | "si" | "quand" | "lorsque" | "comme" | "parce" | "car" | "puisque")
                || x.morph.iter().any(|m| m.pos == Pos::PronSubj) || self.verbish(x) { return; }
            if matches!(x.lower.as_str(), "et" | "ou" | "puis" | "mais") {
                let first = match t.get(k + 1) { Some(f) if f.word => f, _ => return };
                if pron_pn(&first.lower).is_some() || matches!(first.lower.as_str(), "elle" | "elles" | "ce" | "c’" | "cela" | "ça")
                    || det_info(first).is_some() || first.morph.iter().any(|m| matches!(m.pos, Pos::Proper | Pos::Noun) && !has_fin(first)) { return; }
                let mut v = k + 1;
                while Self::word_at(t, v) && CLITICS.contains(&t[v].lower.as_str()) && t[v].lower != "nous" { v += 1; }
                if Self::word_at(t, v) && Self::same(t, i, v) && self.verbish(&t[v]) && !t[v].morph.iter().any(|m| m.pos == Pos::PronSubj || m.pos.is_det())
                    && !(t[v].morph.iter().any(|m| m.moods & M_E != 0) && !t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0)) // "et prenons garde"
                    && t[v].morph.iter().any(|m| is_fin(m))
                    && !t[v].morph.iter().any(|m| is_fin(m) && m.person & p != 0 && m.number & n != 0) {
                    let sug = self.verb_forms_for(&t[v], p, n);
                    o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Ce verbe coordonné doit s’accorder avec le même sujet.", sug));
                }
                return;
            }
            k += 1;
        }
    }

    /// "les auteurs qui aime ...": the verb of a subject relative agrees with its antecedent
    /// (the noun right before "qui", or the head of its noun phrase).
    fn qui_relative(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for q in 1..t.len() {
            if !t[q].word || t[q].lower != "qui" { continue; }
            let a = q - 1;
            if !t[a].word || t[a].sid != t[q].sid { continue; }
            if !t[a].morph.iter().any(|m| matches!(m.pos, Pos::Noun | Pos::Proper | Pos::Adj))
                || t[a].morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::ConjS | Pos::Prep | Pos::Adv) || m.pos.is_det()) { continue; }
            // every noun of the chain before "qui" is a possible antecedent; lists and coordination allow the plural;
            // a pronoun antecedent (celui des frères qui ...) disables the check
            let mut allowed = 0u8;
            let mut head_n = 0u8;
            let mut k = q;
            let mut steps = 0;
            while k > 0 && steps < 16 {
                k -= 1; steps += 1;
                let x = &t[k];
                if x.sid != t[q].sid { break; }
                if !x.word { if x.text == "," { allowed |= N_P; continue; } break; }
                if matches!(x.lower.as_str(), "et" | "ou" | "ni") { allowed |= N_P; continue; }
                if let Some((_, dn)) = det_info(x) { if single(dn) { allowed |= dn; } continue; }
                if x.morph.iter().any(|m| matches!(m.pos, Pos::PronDem | Pos::PronOther | Pos::PronSubj | Pos::PronObj | Pos::ConjS)) && !has_pos(x, Pos::Noun) { allowed = 3; break; }
                if self.verbish(x) { break; }
                if x.morph.iter().any(|m| m.pos == Pos::Proper) && !has_pos(x, Pos::Noun) { allowed |= N_S; continue; }
                let mut nn = 0u8;
                for m in &x.morph { if m.pos == Pos::Noun { nn |= m.number; } }
                if nn != 0 { allowed |= nn; head_n = nn; continue; }
                if x.morph.iter().any(|m| matches!(m.pos, Pos::Adj | Pos::Prep | Pos::Adv) || adjish(m) || m.moods & (M_Y | M_P) != 0) { continue; }
                break;
            }
            if allowed == 0 || allowed == 3 { continue; }
            let ok_set = allowed;
            let v = self.skip_clitics(t, q + 1, q);
            if !Self::word_at(t, v) || !Self::same(t, q, v) || !self.verbish(&t[v]) { continue; }
            if !t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0) { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & ok_set != 0) { continue; }
            let target = if single(head_n) { head_n } else { allowed };
            let sug = self.verb_forms_for(&t[v], 4, target);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Le verbe de la relative s’accorde avec l’antécédent de « qui ».", sug));
        }
    }

    /// Inverted subject: "la ville où vivaient les ouvriers", "les livres que lisent les enfants",
    /// "où aurait pu trouver refuge la jeune fille".
    fn inversion(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !matches!(t[i].lower.as_str(), "où" | "que" | "qu’" | "dont") { continue; }
            if t[i].lower != "où" && !(i > 0 && t[i - 1].word && t[i - 1].morph.iter().any(|m| matches!(m.pos, Pos::Noun | Pos::Adj))) { continue; }
            let mut v = i + 1;
            while Self::word_at(t, v) && matches!(t[v].lower.as_str(), "se" | "s’" | "y" | "en") { v += 1; } // "dont se sert"
            if !Self::word_at(t, v) || !Self::same(t, i, v) { continue; }
            let Some(n) = self.inverted_np(t, v, true) else { continue };
            let sug = self.verb_forms_for(&t[v], 4, n);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Le verbe s’accorde avec son sujet inversé.", sug));
        }
    }

    /// A verb in the 1st or 2nd person needs je/tu/nous/vous (or moi/toi) somewhere before it in the sentence:
    /// "L'auteur lui-même le traduirai" -> "traduira". Imperatives at a clause start are allowed.
    fn orphan_person(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for v in 0..t.len() {
            let x = &t[v];
            if !x.word || x.skip || x.morph.is_empty() || x.inv.is_some() { continue; } // "avez-vous": licensed by its own pronoun
            if !x.sent_start && x.text.chars().next().map_or(false, |c| c.is_uppercase()) { continue; } // proper name
            if !x.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & (M_FIN | M_E) != 0) { continue; }
            if !x.morph.iter().any(is_fin) || x.morph.iter().any(|m| is_fin(m) && m.person & 4 != 0) { continue; }
            // imperative: first word of its clause, possibly after "ne"/clitics
            if x.morph.iter().any(|m| m.moods & M_E != 0) {
                let mut b = v;
                while b > 0 && t[b - 1].word && t[b - 1].sid == x.sid && (CLITICS.contains(&t[b - 1].lower.as_str()) || ADVERBS.contains(&t[b - 1].lower.as_str()) || matches!(t[b - 1].lower.as_str(), "surtout" | "donc" | "alors" | "maintenant" | "ensuite" | "plutôt")) { b -= 1; }
                if b == 0 || t[b].sent_start || !t[b - 1].word || t[b - 1].sid != x.sid
                    || matches!(t[b - 1].lower.as_str(), "et" | "ou" | "mais" | "puis" | "donc" | "alors") { continue; }
            }
            let pron = |k: usize| t[k].word && matches!(t[k].lower.as_str(), "je" | "j’" | "j" | "tu" | "nous" | "vous" | "moi" | "toi" | "on");
            let licensed = (0..v).rev().take_while(|&k| t[k].sid == x.sid).any(pron)
                // inversion with a spaced hyphen: "resterons - nous"
                || (v + 2 < t.len() && !t[v + 1].word && t[v + 1].text == "-" && pron(v + 2))
                || (v + 1 < t.len() && pron(v + 1) && matches!(t[v + 1].lower.as_str(), "je" | "tu" | "nous" | "vous"))
                // "toi qui es", "Notre Père qui es aux cieux": the antecedent of "qui" can be 1st/2nd person
                || (v > 0 && t[v - 1].word && t[v - 1].lower == "qui");
            if licensed { continue; }
            let n = x.morph.iter().filter(|m| is_fin(m)).fold(0u8, |a, m| a | m.number);
            let sug = self.verb_forms_for(x, 4, if single(n) { n } else { N_S });
            o.push(self.mk(x, "FR_SUBJ_VERB", "Verbe à la 1re ou 2e personne sans sujet je/tu/nous/vous.", sug));
        }
    }

    fn verbish(&self, x: &Tok) -> bool {
        x.word && has_fin(x) && (self.fin_lemma(x, "être") || self.fin_lemma(x, "avoir")
            || x.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & (M_Q | M_Y | M_P) == 0))
    }

    /// A finite verb candidate; a noun/verb homograph counts when it follows a noun or pronoun ("la voisine garde").
    fn fin_cand(&self, t: &[Tok], j: usize) -> bool {
        let x = &t[j];
        if !x.word || !has_fin(x) { return false; }
        // "qu'elle a sues", "qu'il a faites": a participle right after an auxiliary is not a new finite verb
        if j > 0 && t[j - 1].word && (self.fin_lemma(&t[j - 1], "avoir") || self.fin_lemma(&t[j - 1], "être")) && x.morph.iter().any(|m| m.moods & M_Q != 0) { return false; }
        if self.verbish(x) { return true; }
        j > 0 && t[j - 1].word && !t[j - 1].morph.iter().any(|m| m.pos.is_det())
            && t[j - 1].morph.iter().any(|m| matches!(m.pos, Pos::Noun | Pos::Proper | Pos::PronSubj | Pos::PronObj))
    }

    /// Index of the main verb of the subject NP ending right before `k`, skipping an object relative clause
    /// ("que la voisine garde ...") or a comma-delimited parenthetical.
    fn main_verb_after(&self, t: &[Tok], i: usize, mut k: usize) -> Option<usize> {
        for _ in 0..2 {
            let tk = t.get(k)?;
            if tk.sid != t[i].sid { return None; }
            if tk.word && matches!(tk.lower.as_str(), "que" | "qu’" | "qui") {
                let mut j = k + 1;
                let mut rel = None;
                while j < t.len() && j <= k + 8 && t[j].sid == t[i].sid {
                    if !t[j].word { return None; }
                    if self.fin_cand(t, j) && !CLITICS.contains(&t[j].lower.as_str()) { rel = Some(j); break; }
                    if matches!(t[j].lower.as_str(), "qui" | "dont" | "où" | "que" | "qu’") || t[j].morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::ConjS)) { return None; }
                    j += 1;
                }
                let f = rel?;
                let mut m = f + 1;
                while m < t.len() && m <= f + 14 && t[m].sid == t[i].sid {
                    let x = &t[m];
                    if !x.word { return None; }
                    if self.fin_cand(t, m) && !CLITICS.contains(&x.lower.as_str()) { return Some(m); }
                    if matches!(x.lower.as_str(), "qui" | "que" | "qu’" | "dont" | "où") || x.morph.iter().any(|mm| matches!(mm.pos, Pos::ConjC | Pos::ConjS | Pos::PronSubj)) { return None; }
                    m += 1;
                }
                return None;
            }
            if !tk.word && matches!(tk.text.as_str(), "—" | "–" | "(") {
                let close = if tk.text == "(" { ")" } else { tk.text.as_str() };
                let c = (k + 1..(k + 25).min(t.len())).find(|&j| !t[j].word && t[j].text == close)?;
                k = c + 1;
                continue;
            }
            if !tk.word && tk.text == "," {
                let first = t.get(k + 1)?;
                let starter = first.word && (first.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::ConjS | Pos::Adv))
                    || has_fin(first) || first.morph.iter().any(adjish)
                    || matches!(first.lower.as_str(), "qui" | "dont" | "comme" | "selon" | "malgré" | "après" | "avant" | "lorsque" | "quand" | "ainsi"));
                if !starter { return None; }
                let mut c = None;
                for j in (k + 2)..=(k + 20).min(t.len().saturating_sub(1)) {
                    if t[j].sid != t[i].sid { break; }
                    if !t[j].word && matches!(t[j].text.as_str(), ";" | ":" | "(" | "«") { return None; }
                    if !t[j].word && t[j].text == "," { c = Some(j); break; }
                }
                k = c? + 1;
                continue;
            }
            break;
        }
        let j = self.skip_clitics(t, k, i);
        if Self::word_at(t, j) && Self::same(t, i, j) { Some(j) } else { None }
    }

    /// A plural form of "être" needs a plural attribute, whatever the subject is.
    fn etre_plural_attr(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !t[i].morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == "être" && m.person & 4 != 0 && m.number == N_P && m.moods & M_SP == 0) { continue; }
            if matches!(t[i].lower.as_str(), "sont" | "étaient" | "seront" | "seraient" | "furent") == false { continue; }
            if t[i].morph.iter().any(|m| m.number & N_S != 0 && is_fin(m)) { continue; }
            if i > 0 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "se" | "s’" | "nous" | "vous" | "me" | "m’" | "te" | "t’") { continue; } // pronominal: own rules
            let mut k = i + 1;
            while Self::word_at(t, k) && Self::same(t, i, k) && (ADVERBS.contains(&t[k].lower.as_str()) || matches!(t[k].lower.as_str(), "ne" | "n’")) { k += 1; }
            if !Self::word_at(t, k) || !Self::same(t, i, k) { continue; }
            let a = &t[k];
            if !a.known || !a.morph.iter().any(adjish) { continue; }
            if a.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv | Pos::ConjC | Pos::ConjS | Pos::PronSubj | Pos::PronObj) || m.pos.is_det()) { continue; }
            if a.morph.iter().any(|m| (adjish(m) || m.pos == Pos::Noun) && m.number & N_P != 0) { continue; }
            if a.morph.iter().any(is_fin) { continue; }
            let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { continue };
            let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, m0.gender, N_P);
            o.push(self.mk(a, "FR_ETRE_PLURAL", "Après « sont », l’attribut est au pluriel.", sug));
        }
    }

    /// subject (gender, number) + être + adjective/participle
    fn attribute(&self, t: &[Tok], i: usize, j: usize, g: u8, n: u8, o: &mut Vec<Lint>) {
        if !single(n) { return; }
        let mut j = j;
        let copula = ["être", "sembler", "paraître", "devenir", "rester", "demeurer", "redevenir"].iter().any(|l| self.fin_lemma(&t[j], l));
        if !copula {
            // compound passive / passé composé of être: "a été vendue", "ont été élus"
            if self.fin_lemma(&t[j], "avoir") && Self::word_at(t, j + 1) && t[j + 1].lower == "été" { j += 1; } else { return; }
        }
        if j > 0 && matches!(t[j - 1].lower.as_str(), "s’" | "se") { return; } // reflexive: handled by FR_PP_PRONOMINAL
        let mut k = j + 1;
        if k < t.len() && !t[k].word && matches!(t[k].text.as_str(), "," | "—") {
            let close = t[k].text.clone();
            if let Some(c) = (k + 1..(k + 15).min(t.len())).find(|&x| !t[x].word && t[x].text == close) { k = c + 1; }
        }
        while Self::word_at(t, k) && Self::same(t, i, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) { k += 1; }
        // "sont restés calmes", "est devenue célèbre": the participle of rester/devenir, then the real attribute
        if Self::word_at(t, k + 1) && t[k].morph.iter().any(|m| m.moods & M_Q != 0 && matches!(self.lemma_of(m), "rester" | "devenir" | "demeurer" | "redevenir"))
            && t[k + 1].morph.iter().any(adjish) && !has_pos(&t[k + 1], Pos::Noun) {
            let b = &t[k + 1];
            if !b.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv) || m.pos.is_det()) && !b.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0 || !single(g))) {
                if let Some(m0) = b.morph.iter().find(|m| adjish(m)) {
                    let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, g, n);
                    o.push(self.mk(b, "FR_ATTR_AGREE", "L’attribut doit s’accorder avec le sujet.", sug));
                }
            }
        }
        if !Self::word_at(t, k) || !Self::same(t, i, k) { return; }
        let a = &t[k];
        if !a.known || !a.morph.iter().any(adjish) { return; }
        if a.lower == "autre" && Self::word_at(t, k + 1) && t[k + 1].lower == "chose" { return; }
        if a.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv | Pos::ConjC | Pos::ConjS) || m.pos.is_det()) { return; }
        if a.morph.iter().any(|m| (adjish(m) || m.pos == Pos::Noun) && m.number & n != 0 && (m.gender & g != 0 || !single(g))) { return; }
        let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { return };
        let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, g, n);
        o.push(self.mk(a, "FR_ATTR_AGREE", "L’attribut doit s’accorder avec le sujet.", sug));
    }

    // ---------- participles ----------
    fn pp_cod(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            let Some(np) = self.np_core(t, i) else { continue };
            let q = np.end + 1;
            if !Self::word_at(t, q) || !matches!(t[q].lower.as_str(), "que" | "qu’") || !Self::same(t, i, q) { continue; }
            if !single(np.number) || !single(np.gender) { continue; }
            if np.gender == G_M && np.number == N_S { continue; }
            if matches!(t[i].lower.as_str(), "de" | "d’") { continue; }
            if i > 0 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "et" | "ou" | "ni") { continue; } // coordinated antecedent
            let mut j = q + 1;
            if j < t.len() && !t[j].word && matches!(t[j].text.as_str(), "," | "—" | "(") {
                let close = match t[j].text.as_str() { "(" => ")", x => x };
                if let Some(c) = (j + 1..(j + 20).min(t.len())).find(|&x| !t[x].word && t[x].text == close) { j = c + 1; }
            }
            if !Self::word_at(t, j) { continue; }
            if pron_pn(&t[j].lower).is_some() { j += 1; }
            else if let Some(n2) = self.np_core(t, j) { j = n2.end + 1; }
            else { continue; }
            j = self.skip_clitics(t, j, i);
            if !Self::word_at(t, j) || !self.fin_lemma(&t[j], "avoir") { continue; }
            let mut k = j + 1;
            let mut adv = 0;
            while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) && adv < 2 { k += 1; adv += 1; }
            if !Self::word_at(t, k) || !Self::same(t, i, k) { continue; }
            let p = &t[k];
            let qr: Vec<&Morph> = p.morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || has_fin(p) && !qr.iter().any(|m| m.gender == G_M) { continue; }
            if qr.iter().any(|m| PP_INVARIABLE.contains(&self.lemma_of(m))) { continue; }
            // "les résultats qu'elle a voulu (obtenir)": implied infinitive, both forms accepted
            if qr.iter().any(|m| matches!(self.lemma_of(m), "vouloir" | "devoir" | "croire" | "penser" | "prévoir" | "demander" | "permettre" | "falloir" | "savoir")) { continue; }
            // a direct object or an infinitive right after the participle: no agreement with the antecedent
            if Self::word_at(t, k + 2) && matches!(t[k + 1].lower.as_str(), "à" | "de" | "d’") && t[k + 2].morph.iter().any(|m| m.moods & M_Y != 0) { continue; }
            if let Some(nx) = t.get(k + 1) {
                let time_phrase = det_info(nx).is_some() && t.get(k + 2).map_or(false, |h| TIME_NOUNS.contains(&h.lower.as_str()));
                if nx.word && nx.sid == p.sid && !time_phrase && (det_info(nx).is_some() || nx.morph.iter().any(|m| m.moods & M_Y != 0 || m.pos == Pos::Noun || m.pos == Pos::Proper)) { continue; }
            }
            if qr.iter().any(|m| m.gender & np.gender != 0 && m.number & np.number != 0 && (m.gender != 3 || m.number != 3)) { continue; }
            if Self::word_at(t, k + 1) && t[k + 1].morph.iter().any(|m| m.moods & M_Y != 0) && qr.iter().any(|m| matches!(self.lemma_of(m), "faire" | "laisser" | "voir" | "entendre" | "sentir")) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, np.gender, np.number);
            o.push(self.mk(p, "FR_PP_COD", "Le participe passé s’accorde avec le COD placé avant.", sug));
        }
    }

    fn pp_pronominal(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(2) {
            let refl = matches!(t[i].lower.as_str(), "s’" | "se") || (matches!(t[i].lower.as_str(), "nous" | "vous") && t[i - 1].lower == t[i].lower);
            if !refl || !Self::word_at(t, i + 1) || !self.fin_lemma(&t[i + 1], "être") { continue; }
            let subj = if !t[i - 1].word { None }
                else if let Some(gn) = pron_gn(&t[i - 1].lower) { Some(gn) }
                else if t[i - 1].lower == "nous" && t[i].lower == "nous" { Some((3u8, N_P)) }
                else {
                    // noun phrase ending right before "se": "Les écoliers se sont levés", "La fenêtre s'est ouverte"
                    (i.saturating_sub(8)..i).filter(|&s0| self.subj_ctx(t, s0)).filter_map(|s0| self.np_at(t, s0)).find(|np| np.end == i - 1).map(|np| (np.gender, np.number))
                };
            let Some((g, n)) = subj else { continue };
            if !single(n) { continue; }
            if (i.saturating_sub(6)..i).any(|k| t[k].sid == t[i].sid && matches!(t[k].lower.as_str(), "que" | "qu’")) { continue; } // COD relative
            if g == G_M && n == N_S { continue; }
            let p = &t[i + 2];
            if !p.word || !Self::same(t, i, i + 2) { continue; }
            let qr: Vec<&Morph> = p.morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| PP_INVARIABLE_PRON.contains(&self.lemma_of(m))) { continue; }
            if qr.iter().any(|m| m.gender & g != 0 && m.number & n != 0 && (m.gender != 3 || m.number != 3)) { continue; }
            // a following direct object leaves the participle invariable
            let obj_follows = match t.get(i + 3) {
                None => false,
                Some(x) if !x.word => false,
                Some(x) if matches!(x.lower.as_str(), "de" | "d’" | "du" | "dès") => false,
                Some(x) if det_info(x).is_some() && t.get(i + 4).map_or(false, |h| TIME_NOUNS.contains(&h.lower.as_str()) || matches!(h.lower.as_str(), "long" | "côté" | "cours" | "bord")) => false,
                Some(x) => x.sid == p.sid && !x.morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::Prep | Pos::Adv)) || x.morph.iter().any(|m| m.pos.is_det() || m.pos == Pos::ConjS),
            };
            if obj_follows { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, g, n);
            o.push(self.mk(p, "FR_PP_PRONOMINAL", "Accord du participe passé avec le sujet (verbe pronominal).", sug));
        }
    }

    fn avoir_inf(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !Self::word_at(t, i + 1) || !Self::same(t, i, i + 1) { continue; }
            let aux = self.fin_lemma(&t[i], "avoir") || t[i].lower == "avoir";
            if !aux { continue; }
            let nx = &t[i + 1];
            if !nx.lower.ends_with("er") { continue; }
            let Some(m) = nx.morph.iter().find(|m| m.pos == Pos::Verb && m.moods & M_Y != 0 && self.lemma_of(m).ends_with("er")) else { continue };
            let sug = self.forms_where(self.lemma_of(m), |f| f.pos == Pos::Verb && f.moods & M_Q != 0 && f.gender == G_M && f.number == N_S);
            o.push(self.mk(nx, "FR_AVOIR_INF", "Après « avoir », on attend un participe passé.", sug));
        }
    }

    fn inf_pp(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const TRIG: [&str; 11] = ["aller", "pouvoir", "vouloir", "devoir", "savoir", "laisser", "aimer", "préférer", "détester", "oser", "espérer"];
        for i in 0..t.len() {
            if !t[i].word { continue; }
            // "de"/"d'" only right after a verb ("il rêve de voyager"), not the partitive "rien de caché", "quoi de plus lié"
            let de_after_verb = matches!(t[i].lower.as_str(), "de" | "d’") && i > 0 && t[i - 1].word && t[i - 1].morph.iter().any(|m| m.pos == Pos::Verb && (is_fin(m) || m.moods & (M_Y | M_Q) != 0)) && !has_pos(&t[i - 1], Pos::Noun);
            let trig = matches!(t[i].lower.as_str(), "pour" | "sans") || de_after_verb
                || t[i].morph.iter().any(|m| is_fin(m) && TRIG.contains(&self.lemma_of(m)));
            if !trig { continue; }
            let mut j = self.skip_clitics(t, i + 1, i);
            while Self::word_at(t, j) && matches!(t[j].lower.as_str(), "pas" | "plus" | "jamais" | "bien") { j += 1; }
            if !Self::word_at(t, j) || !Self::same(t, i, j) { continue; }
            let p = &t[j];
            if !p.known || p.morph.iter().any(|m| m.pos == Pos::Noun || is_fin(m) || m.moods & M_Y != 0) { continue; }
            if p.text.chars().next().map_or(false, |c| c.is_uppercase()) { continue; }
            if !(p.lower.ends_with('é') || p.lower.ends_with("és") || p.lower.ends_with("ée") || p.lower.ends_with("ées")) { continue; }
            let Some(m) = p.morph.iter().find(|m| m.pos == Pos::Verb && m.moods & M_Q != 0 && self.lemma_of(m).ends_with("er")) else { continue };
            o.push(self.mk(p, "FR_INF_PP", "Après ce mot, on attend un infinitif.", vec![self.lemma_of(m).to_string()]));
        }
    }

    // ---------- mood ----------
    fn subjunctive(&self, t: &[Tok], o: &mut Vec<Lint>) {
        let trig: [&[&str]; 15] = [
            &["il", "faut", "que"], &["il", "faut", "qu’"], &["bien", "que"], &["bien", "qu’"], &["pour", "que"], &["pour", "qu’"],
            &["afin", "que"], &["afin", "qu’"], &["avant", "que"], &["avant", "qu’"], &["sans", "que"], &["sans", "qu’"],
            &["quoique"], &["quoiqu’"], &["à", "moins", "que"],
        ];
        for e in 0..t.len() {
            if !t[e].word { continue; }
            let hit = trig.iter().find(|tr| e + 1 >= tr.len() && tr.iter().enumerate().all(|(k, w)| t[e + 1 - tr.len() + k].lower == *w && t[e + 1 - tr.len() + k].sid == t[e].sid));
            let Some(tr) = hit else { continue };
            if tr[0] == "bien" {
                let st = e + 1 - tr.len();
                let ctx = st == 0 || t[st].sent_start || (!t[st - 1].word && matches!(t[st - 1].text.as_str(), "," | ";" | ":"))
                    || (t[st - 1].word && t[st - 1].morph.iter().any(|m| m.pos == Pos::ConjC));
                if !ctx { continue; }
            }
            let s = e + 1;
            if !Self::word_at(t, s) { continue; }
            let Some((p, n)) = pron_pn(&t[s].lower) else { continue };
            let mut j = s + 1;
            while Self::word_at(t, j) && matches!(t[j].lower.as_str(), "ne" | "n’") { j += 1; }
            if !Self::word_at(t, j) || !Self::same(t, s, j) { continue; }
            let v = &t[j];
            let ok_subj = v.morph.iter().any(|m| m.moods & (M_SP | M_SQ) != 0 && m.person & p != 0 && m.number & n != 0);
            let ind = v.morph.iter().find(|m| m.pos == Pos::Verb && m.moods & (M_IP | M_IQ | M_IF | M_K) != 0 && m.person & p != 0 && m.number & n != 0);
            if ok_subj { continue; }
            let Some(r) = ind else { continue };
            let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_SP != 0 && f.person & p != 0 && f.number & n != 0);
            o.push(self.mk(v, "FR_SUBJONCTIF", "Le subjonctif est requis ici.", sug));
        }
    }

    fn si_cond(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !(t[i].lower == "si" || t[i].lower == "s’") { continue; }
            if !(t[i].sent_start || (i > 0 && t[i - 1].text == ",")) { continue; }
            let s = i + 1;
            if !Self::word_at(t, s) { continue; }
            let mut sp = pron_pn(&t[s].lower);
            if t[i].lower == "s’" && !matches!(t[s].lower.as_str(), "il" | "ils") { sp = None; }
            let Some((p, n)) = sp else { continue };
            let j = self.skip_clitics(t, s + 1, i);
            if !Self::word_at(t, j) || !Self::same(t, i, j) { continue; }
            let v = &t[j];
            let cond = v.morph.iter().find(|m| m.pos == Pos::Verb && m.moods & M_K != 0 && m.person & p != 0 && m.number & n != 0);
            let imp = v.morph.iter().any(|m| m.moods & (M_IQ | M_IP) != 0 && m.person & p != 0 && m.number & n != 0);
            let (Some(r), false) = (cond, imp) else { continue };
            let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_IQ != 0 && f.person & p != 0 && f.number & n != 0);
            o.push(self.mk(v, "FR_SI_COND", "Après « si », pas de conditionnel : utilisez l’imparfait.", sug));
        }
    }

    /// "très satisfais" -> "très satisfait": a degree adverb cannot modify a conjugated verb.
    fn adv_verb(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "très" | "tellement") { continue; }
            let j = i + 1;
            if !Self::word_at(t, j) || !Self::same(t, i, j) { continue; }
            let v = &t[j];
            if v.morph.is_empty() || !v.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & (M_Q | M_Y | M_P) == 0) { continue; }
            let lemma = self.lemma_of(&v.morph[0]).to_string();
            let mut cands: Vec<(u32, String)> = Vec::new();
            for (f, m) in self.lex.forms(&lemma) {
                if adjish(&m) && !cands.iter().any(|(_, x)| *x == f) { cands.push((damerau(&v.lower, &f), f)); }
            }
            cands.sort();
            let sug: Vec<String> = cands.into_iter().take(3).map(|(_, f)| f).collect();
            o.push(self.mk(v, "FR_ADV_VERB", "Après « très », on attend un adjectif ou un participe, pas un verbe conjugué.", sug));
        }
    }

    // ---------- homophones and small confusions ----------
    fn homophones(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            let tk = &t[i];
            if !tk.word || tk.skip { continue; }
            let prev = if i > 0 && t[i - 1].word && t[i - 1].sid == tk.sid { Some(&t[i - 1]) } else { None };
            let next = if Self::word_at(t, i + 1) && t[i + 1].sid == tk.sid { Some(&t[i + 1]) } else { None };
            match tk.lower.as_str() {
                "à" => {
                    if let (Some(p), Some(_)) = (prev, next) {
                        let after_prep = i >= 2 && t[i - 2].word && PREPS.contains(&t[i - 2].lower.as_str());
                        if matches!(p.lower.as_str(), "il" | "elle" | "on") && !after_prep {
                            o.push(self.mk(tk, "FR_A_ACCENT", "« a » (verbe avoir) s’écrit sans accent.", vec!["a".into()]));
                        }
                    }
                }
                "a" => {
                    if let (Some(p), Some(n)) = (prev, next) {
                        let after_verb = !p.morph.is_empty() && p.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_FIN != 0)
                            && !self.fin_lemma(p, "avoir") && !p.morph.iter().any(|m| m.moods & M_Q != 0);
                        let before_inf = n.morph.iter().any(|m| m.pos == Pos::Verb && m.moods & M_Y != 0)
                            && !n.morph.iter().any(|m| m.moods & M_Q != 0)
                            && !matches!(p.lower.as_str(), "il" | "elle" | "on" | "y" | "n’" | "ne" | "l’" | "qui" | "c’" | "ce" | "j’" | "lui" | "t’");
                        let after_verb = after_verb && !n.morph.iter().any(|m| m.moods & M_Q != 0) && !matches!(n.lower.as_str(), "été" | "eu");
                        if after_verb || before_inf || (self.fin_lemma(p, "aller") && (n.morph.iter().any(|m| m.pos.is_det()) || n.text.chars().next().map_or(false, |c| c.is_uppercase()))) {
                            o.push(self.mk(tk, "FR_A_ACCENT", "Préposition « à » avec accent.", vec!["à".into()]));
                        }
                    }
                }
                "ont" => {
                    if let Some(n) = next {
                        let plural_prev = prev.map_or(false, |p| matches!(p.lower.as_str(), "ils" | "elles" | "ceux" | "celles" | "qui" | "les" | "eux")
                            || p.morph.iter().any(|m| m.pos == Pos::Noun && m.number == N_P) || p.morph.iter().any(|m| m.pos == Pos::PronObj));
                        if !plural_prev && !n.morph.is_empty() && n.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_FIN != 0 && m.moods & (M_Q | M_Y | M_P) == 0) {
                            o.push(self.mk(tk, "FR_ON_ONT", "« on » (pronom) plutôt que « ont ».", vec!["on".into()]));
                        }
                    }
                }
                "ça" => {
                    if let Some(n) = next {
                        if all_pos(n, Pos::Noun) && n.morph.iter().all(|m| m.number & N_S != 0) && n.known {
                            o.push(self.mk(tk, "FR_CA_SA", "« sa » (déterminant possessif) devant un nom.", vec!["sa".into()]));
                        }
                    }
                }
                "son" => {
                    if let (Some(p), Some(n)) = (prev, next) {
                        let plural_subj = matches!(p.lower.as_str(), "ils" | "elles" | "ceux" | "celles") || p.morph.iter().any(|m| m.pos == Pos::Noun && m.number == N_P);
                        let verb_only = !n.morph.is_empty() && n.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_Q != 0);
                        if (plural_subj && n.morph.iter().any(|m| m.pos == Pos::Verb && m.moods & M_Q != 0) && !n.morph.iter().any(|m| m.pos == Pos::Noun && m.number == N_S)) || verb_only {
                            o.push(self.mk(tk, "FR_SONT_SON", "« sont » (verbe être) plutôt que « son ».", vec!["sont".into()]));
                        }
                    }
                }
                "on" => {
                    if let Some(p) = prev {
                        if matches!(p.lower.as_str(), "il" | "elle" | "ils" | "elles") {
                            o.push(self.mk(tk, "FR_ON_ONT", "« ont » (verbe avoir) : confusion avec « on ».", vec!["ont".into()]));
                        }
                    }
                }
                "sont" => {
                    if let Some(n) = next {
                        let before_ok = match prev { None => true, Some(p) => p.morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::Prep)) } ||
                            (i > 0 && !t[i - 1].word);
                        let sing_noun = all_pos(n, Pos::Noun) && n.morph.iter().all(|m| m.number == N_S);
                        if before_ok && sing_noun {
                            o.push(self.mk(tk, "FR_SONT_SON", "« son » (déterminant possessif) plutôt que « sont ».", vec!["son".into()]));
                        }
                    }
                }
                "se" => {
                    let est = |x: &Tok| matches!(x.lower.as_str(), "est" | "était" | "sera");
                    if let Some(n) = next {
                        if n.known && all_pos(n, Pos::Noun) && !est(n) {
                            o.push(self.mk(tk, "FR_CE_SE", "« ce » (déterminant démonstratif) devant un nom.", vec!["ce".into()]));
                        }
                        let ne = matches!(n.lower.as_str(), "n’" | "ne") && Self::word_at(t, i + 2) && est(&t[i + 2]);
                        if est(n) || ne {
                            o.push(self.mk(tk, "FR_CE_SE", "« ce » (démonstratif) plutôt que « se ».", vec!["ce".into()]));
                        }
                    }
                }
                "sa" => {
                    let bad = match t.get(i + 1) {
                        None => true,
                        Some(n) if !n.word => matches!(n.text.as_str(), "," | "." | "?" | "!" | ";" | ":" | "…"),
                        Some(n) => n.sid == tk.sid && (n.morph.iter().any(|m| m.pos == Pos::ConjC)
                            || (has_fin(n) && !n.morph.iter().any(|m| matches!(m.pos, Pos::Noun | Pos::Adj)))),
                    };
                    if bad {
                        o.push(self.mk(tk, "FR_CA_SA", "« ça » (pronom démonstratif) plutôt que « sa ».", vec!["ça".into()]));
                    }
                }
                "leur" => {
                    if let Some(n) = next {
                        if all_pos(n, Pos::Noun) && n.morph.iter().all(|m| m.number == N_P) {
                            o.push(self.mk(tk, "FR_LEUR", "« leurs » devant un nom pluriel.", vec!["leurs".into()]));
                        }
                    }
                }
                "leurs" => {
                    if let Some(n) = next {
                        if all_pos(n, Pos::Noun) && n.morph.iter().all(|m| m.number == N_S) {
                            o.push(self.mk(tk, "FR_LEUR", "« leur » devant un nom singulier.", vec!["leur".into()]));
                        }
                    }
                }
                "le" | "la" | "les" => {
                    if let Some(n) = next {
                        if n.lower == "l’" {
                            o.push(Lint { start: tk.start, end: n.end, rule: "FR_ELIDED_DUP", message: "Déterminant répété.".into(), suggestions: vec![n.text.clone()] });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
