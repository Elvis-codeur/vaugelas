//! `lexpairs.py` — synthetic lexical minimal pairs (word choice / glossary sense).
//!
//! A faithful port of `archive/python-2026-11-12/lexpairs.py` (213 lines), including its `random.Random(seed)`
//! stream, so the output is byte-identical to the Python generator for the same `--per-subtype`, `--seed` and
//! context-table revision. Every sentence is written in the generator; nothing is read from a corpus.
//!
//! `--contexts 8` reproduces the earlier revision (git 3693633) that produced the 1,080-record
//! `data/lexpairs.jsonl`; the archived generator uses all 16 contexts and produces a different 1,080-record file.
//! Both are parity-checked (see tests/ and `make parity`).

use crate::pyrandom::PyRandom;
use crate::lexpairs_data::{Bank, BANKS, CONTEXTS, HELD_NAMES, HELD_SUBTYPES, PLACES, TRAIN_NAMES};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// The number of context templates in the archived generator (the pre-12:55 revision used the first 8).
pub const MAX_CONTEXTS: usize = CONTEXTS.len();

/// One generated pair: the Python dict's fields, in its insertion order.
struct Rec {
    id: String,
    family: &'static str,
    subtype: &'static str,
    en: &'static str,
    fr_right: String,
    fr_wrong: String,
    span_right: &'static str,
    span_wrong: &'static str,
    note: &'static str,
    entity: String,
    place: String,
    split: &'static str,
}

impl Rec {
    fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("id".into(), json!(self.id));
        m.insert("family".into(), json!(self.family));
        m.insert("subtype".into(), json!(self.subtype));
        m.insert("en".into(), json!(self.en));
        m.insert("fr_right".into(), json!(self.fr_right));
        m.insert("fr_wrong".into(), json!(self.fr_wrong));
        m.insert("span_right".into(), json!(self.span_right));
        m.insert("span_wrong".into(), json!(self.span_wrong));
        m.insert("note".into(), json!(self.note));
        m.insert("entity".into(), json!(self.entity));
        m.insert("place".into(), json!(self.place));
        m.insert("split".into(), json!(self.split));
        Value::Object(m)
    }
}

fn fmt(tpl: &str, right: &str, name: &str, place: &str, en: &str) -> String {
    tpl.replace("{s}", right).replace("{n}", name).replace("{p}", place).replace("{e}", en)
}

/// `records(per_subtype, seed)` with the context table truncated to `n_contexts`.
pub fn records(per_subtype: usize, seed: u64, n_contexts: usize) -> Vec<Value> {
    assert!((1..=CONTEXTS.len()).contains(&n_contexts), "--contexts must be 1..={}", CONTEXTS.len());
    let mut rng = PyRandom::new(seed);
    let mut out: Vec<Rec> = Vec::new();
    for Bank { sub, family, en, right, wrong, note } in BANKS {
        for k in 0..per_subtype {
            let tpl = CONTEXTS[k % n_contexts];
            let (split, name): (&str, &str) = if !HELD_SUBTYPES.contains(&sub) {
                if k % 5 == 4 {
                    ("probe_names", rng.choice(&HELD_NAMES))
                } else {
                    ("train", rng.choice(&TRAIN_NAMES))
                }
            } else {
                ("probe_family", rng.choice(&TRAIN_NAMES))
            };
            let place = rng.choice(&PLACES);
            out.push(Rec {
                id: format!("lex-{sub}-{k:03}"),
                family,
                subtype: sub,
                en,
                fr_right: fmt(tpl, right, name, place, en),
                fr_wrong: fmt(tpl, wrong, name, place, en),
                span_right: right,
                span_wrong: wrong,
                note,
                entity: name.to_string(),
                place: place.to_string(),
                split,
            });
        }
    }
    rng.shuffle(&mut out);
    out.into_iter().map(|r| r.to_value()).collect()
}

/// The JSONL body: `"\n".join(json.dumps(r, ensure_ascii=False) for r in recs) + "\n"`.
pub fn body(recs: &[Value]) -> String {
    let mut s = String::new();
    for r in recs {
        s.push_str(&vaugelas::text::pyjson::dumps(r));
        s.push('\n');
    }
    s
}

/// `sha256(body)`.
pub fn sha256(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The dataset card, byte for byte as `lexpairs.py` writes it (the sha256 and all counts included).
pub fn card(recs: &[Value], body: &str, per_subtype: usize, seed: u64, sha: &str) -> String {
    let mut by_split: Vec<(&str, usize)> = Vec::new(); // first-appearance order, then sorted
    let mut fams: Vec<(&str, Vec<&str>)> = Vec::new();
    for r in recs {
        let split = r["split"].as_str().unwrap();
        match by_split.iter_mut().find(|x| x.0 == split) {
            Some(x) => x.1 += 1,
            None => by_split.push((split, 1)),
        }
        let fam = r["family"].as_str().unwrap();
        let sub = r["subtype"].as_str().unwrap();
        match fams.iter_mut().find(|x| x.0 == fam) {
            Some(x) => {
                if !x.1.contains(&sub) {
                    x.1.push(sub)
                }
            }
            None => fams.push((fam, vec![sub])),
        }
    }
    by_split.sort_by_key(|x| x.0);
    fams.sort_by_key(|x| x.0);
    for f in fams.iter_mut() {
        f.1.sort_unstable();
    }
    let chars = body.chars().count() as f64;
    let c: Vec<String> = vec![
        "# Lexical minimal pairs — dataset card".into(),
        String::new(),
        format!(
            "Generated by `lexpairs.py` (seed {seed}, {per_subtype} contexts per sub-type). Deterministic: sha256 of `lexpairs.jsonl` = `{sha}`."
        ),
        String::new(),
        format!("- records: {} ({} lines, {:.0} KiB)", recs.len(), recs.len(), chars / 1024.0),
        format!("- families: {}", fams.len()),
        format!(
            "- per family: {}",
            fams.iter()
                .map(|(k, v)| format!("{k} {} sub-types ({})", v.len(), v.join(", ")))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        format!(
            "- splits: {}",
            by_split.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ")
        ),
        String::new(),
        "## Held out (never in train)".into(),
        format!("- probe_family sub-types: {}", {
            let mut h = HELD_SUBTYPES.to_vec();
            h.sort_unstable();
            h.join(", ")
        }),
        format!("- probe_names names: {}", HELD_NAMES.join(", ")),
        String::new(),
        "## What a record is".into(),
        "`fr_right` and `fr_wrong` are the same sentence except for one slot: the correct rendering and a real".into(),
        "French word a model produced for the same English sense. `note` states the deciding condition.".into(),
        String::new(),
        "## Provenance and licence".into(),
        "Every sentence is written in `lexpairs.py` from the sense definitions recorded in the project's defect".into(),
        "catalogue; no corpus sentence and no Claude-written text is used. CC BY-SA 4.0, like the project's model".into(),
        "and data (pending owner confirmation).".into(),
        String::new(),
    ];
    c.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_counts() {
        let r = records(4, 20261112, 16);
        assert_eq!(r.len(), 27 * 4);
        assert!(r.iter().any(|x| x["id"] == "lex-works-000"));
    }

    #[test]
    fn deterministic() {
        assert_eq!(body(&records(6, 7, 16)), body(&records(6, 7, 16)));
        assert_ne!(body(&records(6, 7, 16)), body(&records(6, 8, 16)));
    }
}
