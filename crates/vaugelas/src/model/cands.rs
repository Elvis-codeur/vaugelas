//! Port of training/ebm_data.py candidate generation for inference (`record` with err = -1) and for the golden test
//! (`pairs`, gold form at the error). Python stays the reference: the parity test diffs the two on final-2 and tune.
//!   HOMO / HOMO_OF, NOUNS, paradigm_low (priority order + plausibility filter), paradigm, known_sorted, readings
//!   (capitalised fallback through Characterizer.sentence), cand_list (MAXC = 8, gold kept), record.
use crate::text::characterize::{Key, Lexicon, Tok};
use crate::text::natural::{Natural, Reading, E, IF, IP, IQ, IS, K, Q, SP, SQ, Y};
use crate::text::pystr::{has_w, lower, str_isupper, upper};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

pub const MAXC: usize = 8;

const HOMO: [&[&str]; 27] = [
    &["a", "à"], &["et", "est"], &["son", "sont"], &["ou", "où"], &["on", "ont"], &["ce", "se"], &["ces", "ses"], &["la", "là"],
    &["dont", "que", "qui"], &["tout", "tous", "toute", "toutes"], &["leur", "leurs"], &["quel", "quelle", "quels", "quelles", "qu’elle", "qu’elles"],
    &["le", "la", "les"], &["ce", "cet", "cette", "ces"], &["mon", "ma", "mes"], &["ton", "ta", "tes"], &["son", "sa", "ses"],
    &["notre", "nôtre", "nos"], &["votre", "vôtre", "vos"], &["lequel", "laquelle", "lesquels", "lesquelles"],
    &["auquel", "à laquelle", "auxquels", "auxquelles"], &["y", "en"], &["tel", "telle", "tels", "telles"],
    &["même", "mêmes"], &["quelque", "quelques", "quel que", "quelle que"], &["plutôt", "plus tôt"], &["quand", "quant", "qu’en"],
];

/// One candidate set: (word index, gold index, [(form, reading keys)]) -- the `cands` entries of ebm_data.
pub type CandSet = (usize, usize, Vec<(String, Vec<Key>)>);

pub struct Cands<'a> {
    pub lex: &'a Lexicon,
    pub nat: &'a Natural,
    nouns: HashMap<u32, Vec<u32>>,
    homo_of: HashMap<&'static str, BTreeSet<&'static str>>,
    para: RefCell<HashMap<String, Vec<String>>>,
}

fn chars_prefix(s: &str, n: usize) -> &str { match s.char_indices().nth(n) { Some((b, _)) => &s[..b], None => s } }

impl<'a> Cands<'a> {
    pub fn new(lex: &'a Lexicon, nat: &'a Natural) -> Self {
        let mut nouns: HashMap<u32, Vec<u32>> = HashMap::new();
        for (id, r) in nat.readings.iter().enumerate() { if r.pos == 'N' { nouns.entry(r.lem).or_default().push(id as u32); } }
        let mut homo_of: HashMap<&'static str, BTreeSet<&'static str>> = HashMap::new();
        for s in HOMO { for w in s.iter() { homo_of.entry(*w).or_default().extend(s.iter().copied()); } }
        Cands { lex, nat, nouns, homo_of, para: RefCell::new(HashMap::new()) }
    }

    fn r(&self, id: &u32) -> &Reading { &self.nat.readings[*id as usize] }

    /// Forget the memoised paradigms once more than `max` words are cached (a long-running server sees an unbounded
    /// vocabulary). Results do not change: the memo only saves recomputation.
    pub fn limit_cache(&self, max: usize) {
        let mut p = self.para.borrow_mut();
        if p.len() > max { p.clear(); }
    }

    /// `paradigm_low(low)`: competing forms in priority order (sorted by (priority, form) like Python's sorted(out))
    pub fn paradigm_low(&self, low: &str) -> Vec<String> {
        if let Some(v) = self.para.borrow().get(low) { return v.clone(); }
        let mut pri: HashMap<String, u8> = HashMap::new();
        let mut add = |forms: &mut dyn Iterator<Item = String>, k: u8| {
            for f in forms { let e = pri.entry(f).or_insert(k); if *e > k { *e = k; } }
        };
        if let Some(s) = self.homo_of.get(low) { add(&mut s.iter().map(|x| x.to_string()), 0); }
        let rs: Vec<u32> = self.nat.read.get(low).cloned().unwrap_or_default();
        let nominal = rs.iter().any(|i| matches!(self.r(i).pos, 'N' | 'A'));
        let empty: Vec<u32> = Vec::new();
        for i in &rs {
            let r = self.r(i).clone();
            let lem = &self.nat.lemmas[r.lem as usize];
            let (pos, g, p, mo) = (r.pos, r.g, r.p, r.mo);
            if pos == 'V' {
                let forms: Vec<&Reading> = self.nat.bylem.get(&r.lem).unwrap_or(&empty).iter().map(|j| self.r(j)).collect();
                if mo & Q != 0 {
                    add(&mut forms.iter().filter(|f| f.pos == 'V' && f.mo & Q != 0).map(|f| f.form.clone()), 1);
                    if lem.ends_with("er") {
                        let mut v = vec![lem.clone()];
                        v.extend(forms.iter().filter(|f| f.pos == 'V' && f.mo & IP != 0 && f.p & 2 != 0 && f.n & 2 != 0).map(|f| f.form.clone()));
                        add(&mut v.into_iter(), 0);
                    }
                } else if mo & Y != 0 && lem.ends_with("er") {
                    add(&mut forms.iter().filter(|f| f.pos == 'V' && f.mo & Q != 0 && f.g & 1 != 0 && f.n & 1 != 0).map(|f| f.form.clone()), 0);
                    add(&mut forms.iter().filter(|f| f.pos == 'V' && f.mo & IP != 0 && f.p & 2 != 0 && f.n & 2 != 0).map(|f| f.form.clone()), 0);
                } else if mo & (IP | IQ | IF | K | SP | IS | SQ | E) != 0 {
                    // swap = {Ip: {Sp, Iq, If}, Sp: {Ip}, If: {K, Ip}, K: {If, Iq}, Iq: {K, Ip}, Is: {Iq}, E: {Ip}}
                    let swap: [(u16, u16); 7] = [(IP, SP | IQ | IF), (SP, IP), (IF, K | IP), (K, IF | IQ), (IQ, K | IP), (IS, IQ), (E, IP)];
                    let tm: u16 = swap.iter().filter(|(m, _)| mo & m != 0).map(|(m, _)| *m).fold(0, |a, b| a | b);
                    let near: u16 = swap.iter().filter(|(m, _)| tm & m != 0).fold(tm, |a, (_, s)| a | s);
                    let (mut same, mut other) = (Vec::new(), Vec::new());
                    for f in &forms {
                        if f.pos != 'V' || f.form.contains('-') { continue; }
                        if f.mo & tm != 0 && f.p & p == 0 && f.p != 0 { same.push(f.form.clone()); }
                        else if f.mo & near != 0 && f.p & p != 0 { other.push(f.form.clone()); }
                    }
                    add(&mut same.into_iter(), if nominal { 3 } else { 2 });
                    add(&mut other.into_iter(), if nominal { 4 } else { 3 });
                }
            } else if pos == 'A' {
                let v: Vec<String> = self.nat.bylem.get(&r.lem).unwrap_or(&empty).iter().map(|j| self.r(j)).filter(|f| f.pos == 'A').map(|f| f.form.clone()).collect();
                add(&mut v.into_iter(), 1);
            } else if pos == 'N' {
                let v: Vec<String> = self.nouns.get(&r.lem).unwrap_or(&empty).iter().map(|j| self.r(j)).filter(|f| f.g & g != 0).map(|f| f.form.clone()).collect();
                add(&mut v.into_iter(), 1);
            }
        }
        let n = low.chars().count();
        let stem = chars_prefix(low, if n > 4 { n - 4 } else { 1 }.max(1));
        let in_homo = self.homo_of.contains_key(low);
        let mut out: Vec<(u8, String)> = pri.into_iter().filter(|(f, k)| {
            if f == low || f.contains(' ') || f.contains('-') || *f != lower(f) || (f.chars().count() < 2 && !in_homo) { return false; }
            matches!(k, 0 | 2 | 3 | 4) || f.starts_with(stem)
        }).map(|(f, k)| (k, f)).collect();
        out.sort();
        let v: Vec<String> = out.into_iter().map(|(_, f)| f).collect();
        self.para.borrow_mut().insert(low.to_string(), v.clone());
        v
    }

    /// `readings(form, toks, i)`
    fn readings(&self, form: &str, toks: &[String], i: usize) -> Vec<Key> {
        let ks = self.lex.known(form);
        if !ks.is_empty() { return ks; }
        let first: String = form.chars().take(1).collect();
        if !str_isupper(&first) { return Vec::new(); }
        let mut t2: Vec<String> = toks.to_vec(); t2[i] = form.to_string();
        self.lex.sentence(&t2)[i].r.clone()
    }

    /// `cand_list(ws, i, gold)`
    pub fn cand_list(&self, ws: &[String], i: usize, gold: Option<&str>) -> Option<CandSet> {
        let w = &ws[i];
        let mut alts = self.paradigm_low(&lower(w));
        if let Some(g) = gold { if lower(g) != lower(w) && !alts.iter().any(|a| a == g) { alts.push(g.to_string()); } }
        if alts.is_empty() { return None; }
        if alts.len() > MAXC - 1 {
            let keep: Vec<String> = match gold { Some(g) => alts.iter().filter(|a| lower(a) == lower(g)).cloned().collect(), None => Vec::new() };
            let mut rest: Vec<String> = alts.iter().filter(|a| !keep.contains(a)).cloned().collect();
            rest.truncate(MAXC - 1 - keep.len());
            alts = keep.into_iter().chain(rest).collect();
        }
        let wcap = str_isupper(&w.chars().take(1).collect::<String>());
        let mut forms = vec![w.clone()];
        for a in alts {
            forms.push(if wcap { let mut c = a.chars(); let f = c.next().unwrap(); upper(&f.to_string()) + c.as_str() } else { a });
        }
        let cs: Vec<(String, Vec<Key>)> = forms.into_iter().map(|f| { let r = self.readings(&f, ws, i); (f, r) }).filter(|c| !c.1.is_empty()).collect();
        if cs.len() < 2 || &cs[0].0 != w { return None; }
        let g = match gold { None => 0, Some(gd) => cs.iter().position(|c| lower(&c.0) == lower(gd))? };
        Some((i, g, cs))
    }

    /// `record(ws, err, ...)` without the bookkeeping fields: (characterised tokens, candidate sets)
    pub fn record(&self, ws: &[String], err: Option<usize>, gold: Option<&str>) -> (Vec<Tok>, Vec<CandSet>) {
        let mut cands = Vec::new();
        for (i, w) in ws.iter().enumerate() {
            if !has_w(w) || (str_isupper(&w.chars().take(1).collect::<String>()) && i > 0) { continue; }
            if let Some(c) = self.cand_list(ws, i, if Some(i) == err { gold } else { None }) { cands.push(c); }
        }
        (self.lex.sentence(ws), cands)
    }
}
