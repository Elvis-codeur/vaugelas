//! The parts of training-design/ebm_data.py the candidate generators share: `HOMO` / `HOMO_OF`, `NOUNS` and the v1
//! paradigm `paradigm_low` (kept unchanged by ebm_cands2.py, which ranks every v1 form at priority 20).
//!
//! Moved here from infer/src/cands.rs (issue #18, where it is proven equal to Python on final-2 and tune) so that
//! the v14 generator (`ebm_cands2`) and the ebm_ds inference path use one implementation.
use crate::text::natural::{Natural, Reading, E, IF, IP, IQ, IS, K, Q, SP, SQ, Y};
use crate::text::pystr::lower;
use std::collections::{BTreeSet, HashMap};

/// `HOMO`: closed-class homophone / agreement groups.
pub const HOMO: [&[&str]; 27] = [
    &["a", "à"], &["et", "est"], &["son", "sont"], &["ou", "où"], &["on", "ont"], &["ce", "se"], &["ces", "ses"], &["la", "là"],
    &["dont", "que", "qui"], &["tout", "tous", "toute", "toutes"], &["leur", "leurs"], &["quel", "quelle", "quels", "quelles", "qu’elle", "qu’elles"],
    &["le", "la", "les"], &["ce", "cet", "cette", "ces"], &["mon", "ma", "mes"], &["ton", "ta", "tes"], &["son", "sa", "ses"],
    &["notre", "nôtre", "nos"], &["votre", "vôtre", "vos"], &["lequel", "laquelle", "lesquels", "lesquelles"],
    &["auquel", "à laquelle", "auxquels", "auxquelles"], &["y", "en"], &["tel", "telle", "tels", "telles"],
    &["même", "mêmes"], &["quelque", "quelques", "quel que", "quelle que"], &["plutôt", "plus tôt"], &["quand", "quant", "qu’en"],
];

/// ebm_data.py module tables derived from the lexicon.
pub struct EbmData {
    /// `NOUNS`: lemma id -> its noun readings (lexicon file order)
    pub nouns: HashMap<u32, Vec<u32>>,
    /// `HOMO_OF`: word -> union of the HOMO groups it belongs to
    pub homo_of: HashMap<&'static str, BTreeSet<&'static str>>,
}

fn chars_prefix(s: &str, n: usize) -> &str { match s.char_indices().nth(n) { Some((b, _)) => &s[..b], None => s } }

impl EbmData {
    pub fn new(nat: &Natural) -> Self {
        let mut nouns: HashMap<u32, Vec<u32>> = HashMap::new();
        for (id, r) in nat.readings.iter().enumerate() { if r.pos == 'N' { nouns.entry(r.lem).or_default().push(id as u32); } }
        let mut homo_of: HashMap<&'static str, BTreeSet<&'static str>> = HashMap::new();
        for s in HOMO { for w in s.iter() { homo_of.entry(*w).or_default().extend(s.iter().copied()); } }
        EbmData { nouns, homo_of }
    }

    /// `paradigm_low(low)`: competing forms of a lower-cased word in priority order (sorted by (priority, form) like
    /// Python's `sorted(out)`). No cache here; callers that want Python's lru_cache keep their own.
    pub fn paradigm_low(&self, nat: &Natural, low: &str) -> Vec<String> {
        let rd = |id: &u32| -> &Reading { &nat.readings[*id as usize] };
        let mut pri: HashMap<String, u8> = HashMap::new();
        let mut add = |forms: &mut dyn Iterator<Item = String>, k: u8| {
            for f in forms { let e = pri.entry(f).or_insert(k); if *e > k { *e = k; } }
        };
        if let Some(s) = self.homo_of.get(low) { add(&mut s.iter().map(|x| x.to_string()), 0); }
        let rs: &[u32] = nat.read.get(low).map_or(&[], |v| v.as_slice());
        let nominal = rs.iter().any(|i| matches!(rd(i).pos, 'N' | 'A'));
        let empty: Vec<u32> = Vec::new();
        for i in rs {
            let r = rd(i);
            let lem = &nat.lemmas[r.lem as usize];
            let (pos, g, p, mo) = (r.pos, r.g, r.p, r.mo);
            if pos == 'V' {
                let forms: Vec<&Reading> = nat.bylem.get(&r.lem).unwrap_or(&empty).iter().map(rd).collect();
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
                let v: Vec<String> = nat.bylem.get(&r.lem).unwrap_or(&empty).iter().map(rd).filter(|f| f.pos == 'A').map(|f| f.form.clone()).collect();
                add(&mut v.into_iter(), 1);
            } else if pos == 'N' {
                let v: Vec<String> = self.nouns.get(&r.lem).unwrap_or(&empty).iter().map(rd).filter(|f| f.g & g != 0).map(|f| f.form.clone()).collect();
                add(&mut v.into_iter(), 1);
            }
        }
        let n = low.chars().count();
        let stem = chars_prefix(low, if n > 4 { n - 4 } else { 1 });
        let in_homo = self.homo_of.contains_key(low);
        let mut out: Vec<(u8, String)> = pri.into_iter().filter(|(f, k)| {
            if f == low || f.contains(' ') || f.contains('-') || *f != lower(f) || (f.chars().count() < 2 && !in_homo) { return false; }
            matches!(k, 0 | 2 | 3 | 4) || f.starts_with(stem)
        }).map(|(f, k)| (k, f)).collect();
        out.sort();
        out.into_iter().map(|(_, f)| f).collect()
    }
}
