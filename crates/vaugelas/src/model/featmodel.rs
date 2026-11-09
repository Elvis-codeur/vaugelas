//! Model inputs: training-design featmodel.py (`Vocab`, `token_extras`, `surface_extras`, `context_extras`,
//! `encode`) and ebm_model.py (`encode`, `enc_reads`) at inference (no corruption, no lemma dropout), as atom ids.
//! Ported from fr-check-infer (frc-infer `featmodel` + frc-prep `featmodel`, commit cc7d94e).
//!
//! The encoding comes from the model file's header (`enc`, written by training/export_ebm.py --enc):
//!   * v14 (training-design-v14): MAXF 16 atoms per reading, XW 6 extra slots per word =
//!     frc=<RULE> (first 2) + cw=/pc= + midcap/elided (FRC_SURF=1, FRC_SENT=1) + coord/inv/lit (FRC_CTX=1) + guessed;
//!   * round 3 (no `enc` in the header, models such as ebm_ds): MAXF 12, 4 extra slots = cw=/pc= + guessed, MAXK 8.
//!     Vaugelas builds v14 records only, so `Model::load` refuses round-3 models.
use crate::model::cands::{closed_pos, SENT_START};
use crate::text::pystr::{is_upper, lower, str_isupper};
use serde_json::Value;
use std::collections::HashMap;

/// The input encoding of one model (featmodel module constants and FRC_* switches it was trained with).
#[derive(Clone, Debug, PartialEq)]
pub struct Enc {
    /// readings per word (`MAXR`)
    pub maxr: usize,
    /// atoms per reading (`MAXF`)
    pub maxf: usize,
    /// words per sentence (`MAXT`)
    pub maxt: usize,
    /// extra-atom slots per word (`XW`)
    pub xw: usize,
    /// candidates per set (`ebm_model.MAXK`, cfg["maxk"])
    pub maxk: usize,
    /// v14 extras (frc=, surface, context); false = round-3 extras (cw=/pc= + guessed)
    pub v14: bool,
    /// FRC_SURF: midcap / elided
    pub surf: bool,
    /// FRC_CTX: coord / inv / lit
    pub ctx: bool,
    /// FRC_SENT: a word after . ! ? « — : … starts a sentence (surface_extras)
    pub sent: bool,
}

impl Enc {
    /// round-3 encoding (featmodel.py of 2026-11-05: MAXF 12, 4 extras)
    pub fn round3(maxk: usize) -> Enc { Enc { maxr: 6, maxf: 12, maxt: 96, xw: 4, maxk, v14: false, surf: false, ctx: false, sent: false } }

    /// from the model header: `enc` when present, else round 3
    pub fn from_header(head: &Value) -> Result<Enc, String> {
        let maxk = head["cfg"].get("maxk").and_then(Value::as_u64).unwrap_or(8) as usize;
        let Some(e) = head.get("enc") else { return Ok(Enc::round3(maxk)) };
        let n = |k: &str| e.get(k).and_then(Value::as_u64).map(|x| x as usize).ok_or(format!("model header: enc.{k} missing"));
        let b = |k: &str| e.get(k).and_then(Value::as_bool).ok_or(format!("model header: enc.{k} missing"));
        Ok(Enc { maxr: n("maxr")?, maxf: n("maxf")?, maxt: n("maxt")?, xw: n("xw")?, maxk, v14: b("trig")?, surf: b("surf")?, ctx: b("ctx")?, sent: b("sent")? })
    }
}

/// featmodel.Vocab (lookup only)
pub struct Vocab {
    pub itos: Vec<String>,
    stoi: HashMap<String, u32>,
    lem_unk: u32,
}

impl Vocab {
    pub fn new(itos: Vec<String>) -> Vocab {
        let stoi: HashMap<String, u32> = itos.iter().enumerate().map(|(i, s)| (s.clone(), i as u32)).collect();
        let lem_unk = *stoi.get("lem=?").unwrap_or(&2);
        Vocab { itos, stoi, lem_unk }
    }
    /// `Vocab.get(s)`: unknown "lem=..." -> "lem=?" (or <unk> when the vocabulary has no "lem=?"), else <unk> (2)
    pub fn get(&self, a: &str) -> u32 {
        match self.stoi.get(a) { Some(&i) => i, None => if a.starts_with("lem=") { self.lem_unk } else { 2 } }
    }
    pub fn contains(&self, a: &str) -> bool { self.stoi.contains_key(a) }
    pub fn len(&self) -> usize { self.itos.len() }
    pub fn is_empty(&self) -> bool { self.itos.is_empty() }
}


/// One candidate set as given to `encode`: (word index, gold index, [(form, reading keys)]).
pub type SetRef<F> = (usize, usize, Vec<(F, Vec<F>)>);

/// Atom ids of one sentence: per word its readings (each <= maxf ids, at most maxr readings) and its extra atoms
/// (<= xw ids); per candidate set (word index, candidates' readings).
#[derive(Clone, Debug, PartialEq)]
pub struct Encoded {
    pub t: usize,
    pub reads: Vec<Vec<Vec<u32>>>,
    pub extras: Vec<Vec<u32>>,
    pub sets: Vec<(usize, Vec<Vec<Vec<u32>>>)>,
}

fn key_ids(v: &Vocab, key: &str, maxf: usize) -> Vec<u32> { key.split('|').take(maxf).map(|a| v.get(a)).collect() }

/// ebm_model.encode(r, v, rng=None, 0.0) (and featmodel.encode inside it), as ids.
/// `cands` entries: (word index, gold index, [(form, reading keys)]).
pub fn encode<S: AsRef<str>, F: AsRef<str>>(enc: &Enc, v: &Vocab, words: &[S], feats: &[Vec<F>], cands: &[SetRef<F>]) -> Encoded {
    let t = words.len().min(enc.maxt);
    let mut reads = Vec::with_capacity(t);
    let mut extras = Vec::with_capacity(t);
    for i in 0..t {
        let fs: Vec<&str> = feats[i].iter().map(|x| x.as_ref()).collect();
        let word = words[i].as_ref();
        let guessed = fs.contains(&"guessed");
        let rd: Vec<&str> = fs.iter().copied().filter(|x| *x != "guessed" && !(enc.v14 && x.starts_with("frc="))).take(enc.maxr).collect();
        reads.push(rd.iter().map(|k| key_ids(v, k, enc.maxf)).collect::<Vec<_>>());
        let mut ex: Vec<String> = Vec::new();
        if enc.v14 {
            ex.extend(fs.iter().filter(|x| x.starts_with("frc=")).take(2).map(|x| x.to_string()));
            ex.extend(token_extras(word, &rd));
            let prev = if i > 0 { Some(words[i - 1].as_ref()) } else { None };
            let mut sc: Vec<&str> = Vec::new();
            if enc.surf { sc.extend(surface_extras(word, i, prev, enc.sent)); }
            if enc.ctx { sc.extend(context_extras(words, feats, i)); }
            ex.extend(sc.into_iter().filter(|x| v.contains(x)).map(String::from));
        } else {
            ex.extend(token_extras(word, &rd));
        }
        if guessed { ex.push("guessed".into()); }
        extras.push(ex.iter().take(enc.xw).map(|e| v.get(e)).collect());
    }
    let mut sets = Vec::new();
    for (i, g, cs) in cands {
        if *i >= t { continue; }
        let k = cs.len().min(enc.maxk);
        if *g >= k { continue; }
        sets.push((*i, cs[..k].iter().map(|(_, keys)| keys.iter().take(enc.maxr).map(|x| key_ids(v, x.as_ref(), enc.maxf)).collect()).collect()));
    }
    Encoded { t, reads, extras, sets }
}

/// `token_extras(word, readings)`: pc=<punct> for punctuation, cw=<word> for closed-class words
pub fn token_extras<S: AsRef<str>>(word: &str, reads: &[S]) -> Option<String> {
    if reads.len() == 1 && reads[0].as_ref() == "pos=PUNCT" { return Some(format!("pc={word}")); }
    if !reads.is_empty() && reads.iter().map(|r| r.as_ref()).filter(|r| r.starts_with("pos=")).all(closed_pos) {
        return Some(format!("cw={}", lower(word)));
    }
    None
}

/// `surface_extras(word, t, prev)` (FRC_SURF=1): midcap = capital not at a sentence start (and not all caps);
/// elided = ends with ’
pub fn surface_extras(word: &str, t: usize, prev: Option<&str>, sent: bool) -> Vec<&'static str> {
    let mut ex = Vec::new();
    let start = t == 0 || (sent && prev.is_some_and(|p| SENT_START.contains(&p)));
    if !start && word.chars().next().is_some_and(is_upper) && !str_isupper(word) { ex.push("midcap"); }
    if word.ends_with('’') && word.chars().count() > 1 { ex.push("elided"); }
    ex
}

/// the part-of-speech set of word k (`pos(k)`); empty out of range
fn pos_set<S: AsRef<str>>(feats: &[Vec<S>], k: isize) -> Vec<&str> {
    if k < 0 || k as usize >= feats.len() { return Vec::new(); }
    let mut v: Vec<&str> = feats[k as usize].iter().map(|r| r.as_ref()).filter(|r| r.starts_with("pos="))
        .map(|r| r.split('|').next().unwrap_or("").get(4..).unwrap_or("")).collect();
    v.sort_unstable(); v.dedup();
    v
}
/// the mood set of word k (`moods(k)`); empty out of range
fn mood_set<S: AsRef<str>>(feats: &[Vec<S>], k: isize) -> Vec<&str> {
    if k < 0 || k as usize >= feats.len() { return Vec::new(); }
    let mut v: Vec<&str> = Vec::new();
    for r in &feats[k as usize] {
        for a in r.as_ref().split('|') { if let Some(m) = a.strip_prefix("mood=") { v.extend(m.split('+')); } }
    }
    v.sort_unstable(); v.dedup();
    v
}

const CLAUSE_END: [&str; 6] = [",", ";", ":", ".", "!", "?"];

/// `context_extras(words, feats, t)` (FRC_CTX=1, v14): coord / inv / lit
pub fn context_extras<S: AsRef<str>, F: AsRef<str>>(words: &[S], feats: &[Vec<F>], t: usize) -> Vec<&'static str> {
    let mut ex = Vec::new();
    let ti = t as isize;
    let has = |k: isize, p: &str| pos_set(feats, k).contains(&p);
    let w = |k: usize| words[k].as_ref();
    let ft = &feats[t];
    let all_aq = !ft.is_empty() && ft.iter().map(|r| r.as_ref()).filter(|r| r.starts_with("pos="))
        .all(|r| r.starts_with("pos=A") || (r.contains("pos=V") && r.contains("mood=Q")));
    if all_aq && t >= 3 && has(ti - 1, "N") {               // N (DET) et|ou|ni (DET) N [ADJ]
        for j in t.saturating_sub(5)..t - 1 {
            let jl = lower(w(j));
            let ji = j as isize;
            let lo = (ji - 3).max(0);
            if ["et", "ou", "ni"].contains(&jl.as_str()) && !(ji + 1..ti).any(|k| has(k, "V"))
                && (lo..ji).any(|k| has(k, "N")) && !(lo..ji).any(|k| has(k, "V")) {
                ex.push("coord");
                break;
            }
        }
    }
    let p = pos_set(feats, ti);
    let mo = mood_set(feats, ti);
    if p == ["V"] && mo.iter().any(|m| ["Ip", "Iq", "If", "Is", "K", "Sp"].contains(m)) && !mo.contains(&"E") && has(ti + 1, "DET") {
        let mut k = ti - 1;
        let mut prep = false;
        while k >= 0 && !CLAUSE_END.contains(&w(k as usize)) && !((has(k, "N") || has(k, "PRON")) && !prep) {
            if has(k, "PREP") { prep = true; }
            k -= 1;
        }
        let e2 = if has(ti + 3, "A") { ti + 3 } else { ti + 2 };
        let n = words.len() as isize;
        let incise = t > 0 && [",", "—"].contains(&w(t - 1)) && has(ti + 2, "N")
            && (e2 + 1 >= n || [",", ";", ":", ".", "!", "?", "»"].contains(&w((e2 + 1) as usize)));
        let fronted = prep && (k < 0 || CLAUSE_END.contains(&w(k as usize))) && has(k + 1, "PREP");
        if fronted || incise { ex.push("inv"); }
    }
    if mo.iter().any(|m| *m == "Sq" || *m == "Is") { ex.push("lit"); }
    ex
}

#[cfg(test)]
mod tests {
    use super::*;
    fn f(xs: &[&[&str]]) -> Vec<Vec<String>> { xs.iter().map(|r| r.iter().map(|x| x.to_string()).collect()).collect() }
    #[test]
    fn surface() {
        assert_eq!(surface_extras("Paris", 3, Some("à"), true), vec!["midcap"]);
        assert!(surface_extras("La", 3, Some("."), true).is_empty());
        assert_eq!(surface_extras("La", 3, Some("."), false), vec!["midcap"]);
        assert!(surface_extras("ONU", 3, Some("à"), true).is_empty());
        assert_eq!(surface_extras("qu’", 0, None, true), vec!["elided"]);
        assert!(surface_extras("’", 2, Some("a"), true).is_empty());
    }
    #[test]
    fn token() {
        assert_eq!(token_extras(",", &["pos=PUNCT"]).as_deref(), Some("pc=,"));
        assert_eq!(token_extras("Les", &["pos=DET|sub=def|n=p", "pos=PRON|sub=obj"]).as_deref(), Some("cw=les"));
        assert_eq!(token_extras("porte", &["pos=N|g=f", "pos=V|mood=Ip"]), None);
        assert_eq!(token_extras::<&str>("x", &[]), None);
    }
    #[test]
    fn context() {
        // "le lien et la solidarité collectifs": coordination before an adjective
        let w = ["le", "lien", "et", "la", "solidarité", "collectifs"];
        let fe = f(&[&["pos=DET|sub=def"], &["pos=N|g=m"], &["pos=CONJ|sub=coord"], &["pos=DET|sub=def"], &["pos=N|g=f"], &["pos=A|g=m|n=p"]]);
        assert_eq!(context_extras(&w, &fe, 5), vec!["coord"]);
        // incise: ", répondit le garçon ."
        let w = ["Non", ",", "répondit", "le", "garçon", "."];
        let fe = f(&[&["pos=ADV"], &["pos=PUNCT"], &["pos=V|mood=Is"], &["pos=DET|sub=def"], &["pos=N|g=m"], &["pos=PUNCT"]]);
        assert_eq!(context_extras(&w, &fe, 2), vec!["inv", "lit"]);
    }
}
