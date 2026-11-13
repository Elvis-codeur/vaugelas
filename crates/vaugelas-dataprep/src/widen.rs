//! The widened candidate set of the owner plan step 3.2: written form, same-lemma paradigm, lexconf cross-lemma
//! pairs, supplied glossary renderings, and the top-k lexicon forms for the context, capped (32 by default).
//!
//! Everything here is additive: the shipped `Gen::cand_list` is untouched and still returns at most `MAXC` (16)
//! candidates for inference; this module only builds the larger experimental set. Ordering is deterministic:
//! written form, the base list, lexconf, the supplied forms, then the context-ranked forms (score desc, form asc),
//! first occurrence wins, truncated at `maxc`.

use std::collections::BTreeSet;
use vaugelas::model::cands::{Cand, CandSet, Gen, Sent};
use vaugelas::model::resources::FormLists;

pub struct Widener<'a> {
    pub gen: &'a Gen<'a>,
    /// the confusion table to add (the data directory's `lexpairs_conf.json`, or `--lexconf`)
    pub lexconf: &'a FormLists,
    pub maxc: usize,
    /// how many context-ranked forms to add at most
    pub topk: usize,
    /// the document's own pool (glossary renderings, recurring terms, entities), added at every slot
    pub doc_forms: &'a [String],
    /// how many document-pool forms to add at most
    pub doc_topk: usize,
}

impl<'a> Widener<'a> {
    fn push(&self, s: &Sent, i: usize, cs: &mut Vec<Cand>, form: &str) {
        let form = form.trim();
        if form.is_empty() || cs.len() >= self.maxc || cs.iter().any(|x| x.0 == form) {
            return;
        }
        let rd = self.gen.readings(form, s, i);
        if rd.is_empty() {
            return;
        }
        cs.push((form.to_string(), rd));
    }

    /// The widened set at word `i`; `extra` holds forms supplied by the document (glossary renderings).
    pub fn widen(&self, s: &Sent, i: usize, extra: &[String]) -> CandSet {
        let low = &s.low[i];
        let mut cs: Vec<Cand> = self.gen.cand_list(s, i).map(|x| x.2).unwrap_or_default();
        if cs.is_empty() {
            let rd = self.gen.readings(&s.ws[i], s, i);
            if !rd.is_empty() {
                cs.push((s.ws[i].clone(), rd));
            }
        }
        // candidate 0 must be the written form
        if cs.first().map(|x| x.0.as_str()) != Some(s.ws[i].as_str()) {
            if let Some(k) = cs.iter().position(|x| x.0 == s.ws[i]) {
                let x = cs.remove(k);
                cs.insert(0, x);
            } else if self.gen.readings(&s.ws[i], s, i).len() > 0 {
                cs.insert(0, (s.ws[i].clone(), self.gen.readings(&s.ws[i], s, i)));
            }
        }
        if let Some(v) = self.lexconf.get(low) {
            for f in v {
                self.push(s, i, &mut cs, f);
            }
        }
        for f in extra {
            self.push(s, i, &mut cs, f);
        }
        // cheap context ranking over the confusion space the resources already hold (lexical, near, mined, cw,
        // lexconf, edit-distance near forms): score = count(prev, form) + count(form, next) in the training books
        let mut pool: BTreeSet<String> = BTreeSet::new();
        for table in [&self.gen.res.lexhom, &self.gen.res.nearhom, &self.gen.res.cw_pairs, &self.gen.res.mined] {
            if let Some(v) = table.get(low) {
                pool.extend(v.iter().cloned());
            }
        }
        if let Some(v) = self.lexconf.get(low) {
            pool.extend(v.iter().cloned());
        }
        pool.extend(self.gen.near_known(low));
        let prev = if i > 0 { Some(s.low[i - 1].as_str()) } else { None };
        let next = if i + 1 < s.low.len() { Some(s.low[i + 1].as_str()) } else { None };
        let score = |f: &str| -> i64 {
            let fl = f.to_lowercase();
            let mut n = 0;
            if let Some(p) = prev {
                n += *self.gen.res.bigrams.get(&format!("{p} {fl}")).unwrap_or(&0);
            }
            if let Some(q) = next {
                n += *self.gen.res.bigrams.get(&format!("{fl} {q}")).unwrap_or(&0);
            }
            n
        };
        let mut ranked: Vec<String> = pool.into_iter().collect();
        ranked.sort_by(|a, b| score(b).cmp(&score(a)).then_with(|| a.cmp(b)));
        let mut added = 0usize;
        for f in &ranked {
            if added >= self.topk || cs.len() >= self.maxc {
                break;
            }
            let before = cs.len();
            self.push(s, i, &mut cs, f);
            if cs.len() > before {
                added += 1;
            }
        }
        // the document's own knowledge: glossary renderings, recurring terms and entities, ranked by the same
        // cheap context score; this is what a reader of that book could propose
        let mut doc: Vec<&String> = self.doc_forms.iter().filter(|f| !f.trim().is_empty()).collect();
        doc.sort_by(|a, b| score(b).cmp(&score(a)).then_with(|| a.cmp(b)));
        let mut dadded = 0usize;
        for f in doc {
            if dadded >= self.doc_topk || cs.len() >= self.maxc {
                break;
            }
            let before = cs.len();
            self.push(s, i, &mut cs, f);
            if cs.len() > before {
                dadded += 1;
            }
        }
        cs.truncate(self.maxc);
        if std::env::var("WIDEN_DEBUG").is_ok_and(|v| v == s.ws[i]) {
            eprintln!("WIDEN {} at {} extra={:?} forms={:?}", s.ws[i], i, extra, cs.iter().map(|x| x.0.clone()).collect::<Vec<_>>());
        }
        (i, 0, cs)
    }
}

/// The margin used in step 3: `E(written) - softmin(E(alternatives))`, where softmin = -log sum_{a>0} exp(-E_a).
/// Positive = the written form is less plausible than the alternatives taken together.
pub fn margin(energies: &[f32]) -> f32 {
    if energies.len() < 2 {
        return 0.0;
    }
    let m = energies[1..].iter().map(|x| -x).fold(f32::NEG_INFINITY, f32::max);
    let z: f32 = energies[1..].iter().map(|x| (-x - m).exp()).sum();
    energies[0] - (-(m + z.ln()))
}
