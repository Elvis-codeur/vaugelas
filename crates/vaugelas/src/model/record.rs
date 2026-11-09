//! Port of training-design/ebm_data2.py for the v14 flag set (FRC_TRIG on, FRC_LEXHOM_NOSAME=1, FRC_LEXHOM_GROUP=1,
//! FRC_GEN3=1, FRC_LMFEAT=1, FRC_MAXF=16), inference part only: the evaluation record `record_pf` (v2 candidate lists,
//! fr-check triggers, no gold). Ported from fr-check-infer `frc_prep::ebm_data2` (commit cc7d94e); the training
//! records (`record` with gold, `record_all`) stay there.
//!
//! | Rust                      | Python                                                       |
//! |---------------------------|--------------------------------------------------------------|
//! | `frc_lints`               | natural.py `Frc.gram` (in-process checker, same JSON offsets as `frc serve`) |
//! | `token_spans`             | `token_spans(text, ws)`                                      |
//! | `triggers`                | `triggers(text, ws)`                                         |
//! | `lm_atoms`                | `lm_atoms(ws, i, forms)`                                     |
//! | `Builder::cand_list`      | `cand_list(ws, i, gold=None)`: v2 list + fr-check suggestions, `decorate` |
//! | `decorate`                | its last steps: snd= group, cw= identity, fq=/lm=/capr=       |
//! | `Builder::record_pf`      | `record_pf(text, ws, err=-1, src)`                           |
//!
//! The raw text of a sentence (what fr-check checks and the tokens are located in) is passed explicitly; Python finds
//! it again through `_TEXT[tuple(ws)]`, filled by the patched `C.tokenize` just before, which is the same text.
//! The fr-check rule filter (`IGN`) belongs to the model (see `crate::model::profile`): `Builder::ignore`.
use crate::text::characterize::Lexicon;
use crate::model::drops::Drops;
use crate::model::cands::{cap_first, closed_pos, midsent, over_maxf, Cand, CandSet, Gen, Sent, MAXC};
use crate::text::pystr::{has_w, lower, repr_str, str_isupper};
use crate::model::resources::Resources;
use crate::rules::{CheckOptions, Checker};
use std::collections::HashMap;

/// `frc serve` without `--lazy` / `--strict-typo` (what natural.Frc runs).
const SERVE_OPTS: CheckOptions = CheckOptions { spell_suggestions: true, strict_typography: false };

/// One fr-check lint as `frc serve` reports it: character offset and length in the checked text.
#[derive(Clone, Debug)]
pub struct Lint { pub offset: usize, pub len: usize, pub rule: &'static str, pub suggestions: Vec<String> }

/// `Frc.gram(s)`: the grammar lints of one line (rules matching `ignore` dropped and counted as
/// `frc_lints_ignored:<rule>`). Python refuses a text with a line break (it would desynchronise the `serve` pipe); so
/// does this.
pub fn frc_lints(ck: &Checker, text: &str, ignore: &[String], dr: &mut Drops) -> Result<Vec<Lint>, String> {
    if text.contains('\n') || text.contains('\r') {
        return Err(format!("newline in fr-check input would desynchronise replies: {}", repr_str(&text.chars().take(80).collect::<String>())));
    }
    let mut out = Vec::new();
    for l in ck.check_opts(text, &SERVE_OPTS) {
        let up = l.rule.to_uppercase();
        if ignore.iter().any(|k| up.contains(k.as_str())) { dr.count(&format!("frc_lints_ignored:{}", l.rule)); continue; }
        let cs = text[..l.start].chars().count();
        let ce = text[..l.end].chars().count();
        out.push(Lint { offset: cs, len: ce - cs, rule: l.rule, suggestions: l.suggestions });
    }
    Ok(out)
}

/// `token_spans(text, ws)`: character span of each token, by sequential search in the text (with ' as ’, which is
/// what the tokenizer does); None from the first token that cannot be found or lies more than 3 characters ahead
/// (counted: `spans_unlocated` tokens, `sentences_spans_lost`).
pub fn token_spans(text: &str, ws: &[String], dr: &mut Drops) -> Vec<Option<(usize, usize)>> {
    let t: Vec<char> = text.chars().map(|c| if c == '\'' { '’' } else { c }).collect();
    let mut out: Vec<Option<(usize, usize)>> = Vec::with_capacity(ws.len());
    let mut pos = 0usize;
    for w in ws {
        let wc: Vec<char> = w.chars().collect();
        let k = if wc.len() > t.len() { None } else { (pos..=t.len() - wc.len()).find(|&k| t[k..k + wc.len()] == wc[..]) };
        match k {
            Some(k) if !(k - pos > 3 && !out.is_empty()) => { out.push(Some((k, k + wc.len()))); pos = k + wc.len(); }
            _ => {
                dr.add("spans_unlocated", (ws.len() - out.len()) as i64);
                dr.count_ex("sentences_spans_lost", || format!("{} in {}", repr_str(w), repr_str(&t.iter().take(100).collect::<String>())));
                out.resize(ws.len(), None);
                return out;
            }
        }
    }
    out
}

/// What fr-check says about each token: rules (in lint order, duplicates kept) and single-token suggestions.
#[derive(Clone, Debug, Default)]
pub struct Triggers { pub rules: Vec<Vec<&'static str>>, pub sugg: Vec<Vec<String>> }

/// `triggers(text, ws)` on the lints of `text` (`frc_lints`), with its counters.
pub fn triggers(lints: &[Lint], text: &str, ws: &[String], dr: &mut Drops) -> Triggers {
    let sp = token_spans(text, ws, dr);
    dr.count("frc_calls");
    let mut t = Triggers { rules: vec![Vec::new(); ws.len()], sugg: vec![Vec::new(); ws.len()] };
    for m in lints {
        let (a, b) = (m.offset, m.offset + m.len);
        let cov: Vec<usize> = sp.iter().enumerate().filter_map(|(i, x)| x.filter(|x| x.0 < b && x.1 > a).map(|_| i)).collect();
        dr.count("frc_lints");
        if cov.is_empty() { dr.count_ex("lints_unmapped", || format!("{} @{a}: {}", m.rule, repr_str(&text.chars().take(80).collect::<String>()))); }
        for &i in &cov { t.rules[i].push(m.rule); }
        if cov.len() == 1 {
            let sg: Vec<&String> = m.suggestions.iter().filter(|x| !x.is_empty()).collect();
            if sg.iter().any(|x| x.contains(' ')) { dr.count_ex("sugg_multiword", || m.rule.to_string()); }
            let wl = lower(&ws[cov[0]]);
            t.sugg[cov[0]].extend(sg.into_iter().filter(|x| !x.contains(' ') && lower(x) != wl).cloned());
        }
    }
    t
}

/// One evaluation record: per word its readings (+ `frc=<RULE>` trigger atoms), the words, the candidate sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub feats: Vec<Vec<String>>,
    pub words: Vec<String>,
    pub cands: Vec<CandSet>,
}

/// Python `round(x)` for a float: nearest integer, ties to even.
fn py_round(x: f64) -> i64 { x.round_ties_even() as i64 }

/// `lm_atoms(ws, i, forms)`: per candidate "|fq=..|lm=.." (+ "|capr=.." for a capitalised candidate mid-sentence)
pub fn lm_atoms(res: &Resources, s: &Sent, i: usize, forms: &[&str]) -> Vec<String> {
    let p = if i == 0 { "<s>" } else { s.low[i - 1].as_str() };
    let n = if i + 1 < s.low.len() { s.low[i + 1].as_str() } else { "</s>" };
    let big = |k: String| *res.bigrams.get(&k).unwrap_or(&0);
    let sc = |x: &str| big(format!("{p} {x}")) + big(format!("{x} {n}"));
    let wl = s.low[i].as_str();
    let sw = sc(wl);
    let mid = midsent(s.ws, i);
    forms.iter().map(|f| {
        let fl = lower(f);
        let uni = *res.unigrams.get(&fl).unwrap_or(&0);
        let fq = ((((1 + uni) as f64).log2() / 2.0) as i64).min(9);   // min(9, int(log2(1 + n) / 2))
        let lm = if fl == wl { 0 } else { py_round((((sc(&fl) + 1) as f64) / ((sw + 1) as f64)).log2() / 2.0).clamp(-3, 3) };
        let mut a = format!("|fq={fq}|lm={lm}");
        if mid && f.chars().next().is_some_and(crate::text::pystr::is_upper) && !str_isupper(f) {
            let (c, l) = res.capstats2.get(&fl).copied().unwrap_or((0, uni));
            a.push_str(&format!("|capr={}", py_round((10 * c) as f64 / (c + l).max(1) as f64)));
        }
        a
    }).collect()
}

/// The last steps of ebm_data2 `cand_list` on a non-empty list: FRC_LEXHOM_GROUP snd=<pronunciation> atoms
/// (NOSAME: no snd=same), GEN3g identity atoms (cw=<form> on closed-class candidates and on candidates whose
/// readings are identical), FRC_LMFEAT corpus atoms (fq=, lm=, capr=).
pub fn decorate(res: &Resources, s: &Sent, i: usize, cs: &mut [Cand], dr: &mut Drops) {
    // FRC_LEXHOM_GROUP (NOSAME: no snd=same): snd=<pronunciation> on every member of the typed word's group
    let wl = s.low[i].as_str();
    if let (Some(hs), Some(ph)) = (res.lexhom.get(wl), res.lexhom_phon.get(wl)) {
        if !hs.is_empty() && !ph.is_empty() {                // `if hs and ph:`
            let grp: Vec<String> = hs.iter().map(|x| lower(x)).chain(std::iter::once(wl.to_string())).collect();
            let t = format!("|snd={ph}");
            for (f, rd) in cs.iter_mut() {
                if grp.contains(&lower(f)) { for r in rd.iter_mut() { if !r.contains(&t) { r.push_str(&t); } } }
            }
        }
    }
    // GEN3g: closed-class candidates -> identity atom cw=<form>; then identical reading lists -> identity atom
    for (f, rd) in cs.iter_mut() {
        if !rd.is_empty() && rd.iter().filter(|r| r.starts_with("pos=")).all(|r| closed_pos(r)) {
            let fl = lower(f);
            for r in rd.iter_mut() { if !r.contains("|cw=") { r.push_str("|cw="); r.push_str(&fl); } }
        }
    }
    let mut cnt: HashMap<Vec<String>, usize> = HashMap::new();
    for (_, rd) in cs.iter() { *cnt.entry(rd.clone()).or_insert(0) += 1; }
    if cnt.values().any(|&v| v > 1) {
        let dup: Vec<bool> = cs.iter().map(|(_, rd)| cnt[rd] > 1).collect();
        for ((f, rd), d) in cs.iter_mut().zip(dup) {
            if d { let fl = lower(f); for r in rd.iter_mut() { r.push_str("|cw="); r.push_str(&fl); } }
        }
        dr.count("cands_identity_added");
    }
    // FRC_LMFEAT: corpus atoms on every candidate reading
    let forms: Vec<&str> = cs.iter().map(|x| x.0.as_str()).collect();
    let at = lm_atoms(res, s, i, &forms);
    for ((_, rd), a) in cs.iter_mut().zip(at) { for r in rd.iter_mut() { r.push_str(&a); } }
    for (f, rd) in cs.iter() {
        for r in rd { if over_maxf(r) { dr.warn("reading_over_maxf", format!("{f}: {r}")); } }
    }
}

/// The record builder: generator, checker and the trigger rule filter.
pub struct Builder<'a> {
    pub gen: &'a Gen<'a>,
    pub ck: &'a Checker,
    /// upper-case substrings of fr-check rule names whose lints are not model inputs (natural.py `IGN`)
    pub ignore: &'a [String],
}

impl<'a> Builder<'a> {
    pub fn new(gen: &'a Gen<'a>, ck: &'a Checker, ignore: &'a [String]) -> Self { Builder { gen, ck, ignore } }

    pub fn lex(&self) -> &'a Lexicon { self.gen.lex }

    /// ebm_data2 `cand_list(ws, i, gold=None)`: the v2/GEN3/GEN4 list, fr-check suggestions `sugg`, homophone group
    /// atom, identity atoms, corpus atoms.
    pub fn cand_list(&self, s: &Sent, i: usize, sugg: &[String], dr: &mut Drops) -> Option<CandSet> {
        let g = self.gen;
        let ws = s.ws;
        let mut c = g.cand_list_dr(s, i, dr);
        if !sugg.is_empty() {
            let mut cs: Vec<(String, Vec<String>)> = match &c {
                Some(c) => c.2.clone(),
                None => {
                    let rd = g.readings(&ws[i], s, i);
                    vec![(ws[i].clone(), if rd.is_empty() { vec!["pos=UNK".to_string()] } else { rd })]
                }
            };
            if cs[0].1.is_empty() { dr.count_ex("sugg_written_unread", || ws[i].clone()); }
            if !cs[0].1.is_empty() {
                let wcap = ws[i].chars().next().is_some_and(crate::text::pystr::is_upper);
                for f in sugg {
                    let f = if wcap { cap_first(f) } else { f.clone() };
                    let fl = lower(&f);
                    let k = if let Some(k) = cs.iter().position(|x| x.0 == f) { k }
                        else if let Some(k) = cs.iter().position(|x| lower(&x.0) == fl) { k }
                        else {
                            let rd = g.reads3(&f, s, i, false);
                            if rd.is_empty() { dr.count_ex("sugg_no_reading", || f.clone()); continue; }
                            cs.insert(1, (f, rd));
                            dr.count("sugg_inserted");
                            1
                        };
                    for r in cs[k].1.iter_mut() { if !r.ends_with("|sugg=1") { r.push_str("|sugg=1"); } }
                }
                if cs.len() > MAXC { dr.count_ex("sugg_pushed_out", || cs[MAXC].0.clone()); }
                cs.truncate(MAXC);
                if cs.len() >= 2 { c = Some((i, 0, cs)); }
            }
        }
        let (ci, cg, mut cs) = c?;
        decorate(self.gen.res, s, i, &mut cs, dr);
        Some((ci, cg, cs))
    }

    /// The fr-check triggers of the raw text (`_with_trig`): lints, then rules and suggestions per token.
    fn triggers(&self, text: &str, ws: &[String], dr: &mut Drops) -> Result<Triggers, String> {
        let lints = frc_lints(self.ck, text, self.ignore, dr)?;
        Ok(triggers(&lints, text, ws, dr))
    }

    /// `FD.feats(ws)` + `frc=<RULE>` atoms (first occurrence order) on every token a lint covers.
    fn feats(&self, s: &Sent, trig: &Triggers) -> Vec<Vec<String>> {
        let lex = self.gen.lex;
        let mut feats: Vec<Vec<String>> = s.base.iter().map(|t| {
            let mut v: Vec<String> = t.r.iter().map(|k| lex.keys[*k as usize].clone()).collect();
            if t.guessed { v.push("guessed".into()); }
            v
        }).collect();
        for (i, rs) in trig.rules.iter().enumerate() {
            let mut seen: Vec<&str> = Vec::new();                 // dict.fromkeys(rs): first occurrence order
            for r in rs { if !seen.contains(r) { seen.push(r); } }
            feats[i].extend(seen.into_iter().map(|r| format!("frc={r}")));
        }
        feats
    }

    /// `record_pf(text, ws, -1, src)`: the evaluation record exactly as inference builds it (no gold). Err when the
    /// text holds a line break (Python refuses it too: it would desynchronise `frc serve`).
    pub fn record_pf(&self, text: &str, ws: &[String]) -> Result<Record, String> {
        let dr = &mut Drops::new();
        let trig = self.triggers(text, ws, dr)?;
        let s = Sent::new(self.gen.lex, ws);
        let cands = ws.iter().enumerate().filter(|(_, w)| has_w(w)).filter_map(|(i, _)| self.cand_list(&s, i, &trig.sugg[i], dr)).collect();
        Ok(Record { feats: self.feats(&s, &trig), words: ws.to_vec(), cands })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    fn v(xs: &[&str]) -> Vec<String> { xs.iter().map(|x| x.to_string()).collect() }
    #[test]
    fn spans_follow_the_tokenizer() {
        // the tokenizer turns ' into ’ and splits elisions: spans are found on the raw text
        let ws = v(&["L’", "homme", "qu’", "il", "voit", "."]);
        let mut dr = Drops::new();
        assert_eq!(token_spans("L'homme qu'il voit.", &ws, &mut dr),
                   vec![Some((0, 2)), Some((2, 7)), Some((8, 11)), Some((11, 13)), Some((14, 18)), Some((18, 19))]);
        // a token more than 3 characters ahead: lost from there on
        let ws = v(&["a", "b", "c"]);
        assert_eq!(token_spans("a xxxxx b c", &ws, &mut dr), vec![Some((0, 1)), None, None]);
        // the first token may be anywhere
        assert_eq!(token_spans("------ a", &v(&["a"]), &mut dr), vec![Some((7, 8))]);
    }
    #[test]
    fn homophone_group_and_identity_atoms() {
        let mut res = Resources::default();
        res.lexhom.insert("la".into(), v(&["là"]));
        res.lexhom_phon.insert("la".into(), "la".into());
        let ws = v(&["Il", "voit", "la", "porte"]);
        let s = Sent { ws: &ws, low: ws.iter().map(|w| lower(w)).collect(), base: Vec::new() };
        let c = |f: &str, r: &[&str]| (f.to_string(), v(r));
        let mut cs = vec![c("la", &["pos=DET|sub=def|g=f|n=s"]), c("là", &["pos=ADV"]), c("le", &["pos=DET|sub=def|g=m|n=s"]),
                          c("x", &["pos=N|g=m"]), c("y", &["pos=N|g=m"])];
        decorate(&res, &s, 2, &mut cs, &mut Drops::new());
        assert_eq!(cs, vec![
            c("la", &["pos=DET|sub=def|g=f|n=s|snd=la|cw=la|fq=0|lm=0"]),   // group member + closed class
            c("là", &["pos=ADV|snd=la|fq=0|lm=0"]),                          // group member, open class
            c("le", &["pos=DET|sub=def|g=m|n=s|cw=le|fq=0|lm=0"]),          // closed class only
            c("x", &["pos=N|g=m|cw=x|fq=0|lm=0"]),                          // identical readings: identity on both
            c("y", &["pos=N|g=m|cw=y|fq=0|lm=0"]),
        ]);
    }
    #[test]
    fn corpus_atom_buckets() {
        // fq = min(9, int(log2(1 + n) / 2)); lm = clamp(round(log2((s(f)+1)/(s(w)+1)) / 2), -3, 3), ties to even;
        // capr = round(10 c / max(1, c + l)), ties to even
        let mut res = Resources::default();
        for (k, n) in [("x", 3), ("y", 4), ("z", 1 << 20), ("mars", 9)] { res.unigrams.insert(k.into(), n); }
        res.bigrams.insert("le x".into(), 1);       // s(x) = 1: (1+1)/(0+1) = 2 -> log2 = 1 -> 0.5 -> 0 (half to even)
        res.bigrams.insert("le y".into(), 7);       // (7+1)/1 = 8 -> 3 -> 1.5 -> 2
        res.bigrams.insert("z .".into(), 1000);     // log2(1001) / 2 = 4.98 -> 5 -> clamped to 3
        res.capstats2.insert("mars".into(), (1, 3)); // 10 * 1 / 4 = 2.5 -> 2
        let ws = v(&["le", "w", "."]);
        let s = Sent { ws: &ws, low: ws.clone(), base: Vec::new() };
        let a = lm_atoms(&res, &s, 1, &["w", "x", "y", "z", "Mars", "MARS"]);
        assert_eq!(a, v(&["|fq=0|lm=0", "|fq=1|lm=0", "|fq=1|lm=2", "|fq=9|lm=3", "|fq=1|lm=0|capr=2", "|fq=1|lm=0"]));
        // after a sentence end there is no capital question
        let ws = v(&[".", "w"]);
        let s = Sent { ws: &ws, low: ws.clone(), base: Vec::new() };
        assert_eq!(lm_atoms(&res, &s, 1, &["Mars"]), v(&["|fq=1|lm=0"]));
    }
    #[test]
    fn python_round_is_half_even() {
        assert_eq!(py_round(0.5), 0);
        assert_eq!(py_round(1.5), 2);
        assert_eq!(py_round(2.5), 2);
        assert_eq!(py_round(-0.5), 0);
        assert_eq!(py_round(-1.5), -2);
    }
}
