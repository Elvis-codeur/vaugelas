//! `product_pairs.py` — minimal pairs inside real product passages (the 1,989-item batch 1).
//!
//! A faithful port of `archive/python-2026-11-12/product_pairs.py` (148 lines): the same carrier filter, the same
//! `re.findall(r"\w+(?:['’]\w+)?|[^\w\s]+")` tokenisation, the same `random.Random(seed)` stream (`shuffle`, then
//! one `choice` per carrier), the same `re.sub(r"\b…\b", …, count=1)` replacement and the same `prod-NNNN` ids.
//! Carrier sentences are public-domain book sentences from the clean-text slice; the corpus stays on the ThinkPad.
//!
//! Byte-for-byte parity with Python is checked by `tests/product_golden.rs` against the committed 1,989-item golden.

use crate::product_data::BANKS;
use crate::pyrandom::PyRandom;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Python `str.isalnum() or c == "_"` (the `\w` of the `re` module for str patterns).
pub fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Python `str.isspace()` (it also accepts the four ASCII separators U+001C..U+001F).
pub fn space_char(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `re.findall(r"\w+(?:['’]\w+)?|[^\w\s]+", text)`.
pub fn tokens(text: &str) -> Vec<String> {
    let cs: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < cs.len() {
        let c = cs[i];
        if word_char(c) {
            let mut j = i;
            while j < cs.len() && word_char(cs[j]) {
                j += 1;
            }
            if j + 1 < cs.len() && (cs[j] == '\'' || cs[j] == '’') && word_char(cs[j + 1]) {
                let mut k = j + 2;
                while k < cs.len() && word_char(cs[k]) {
                    k += 1;
                }
                out.push(cs[i..k].iter().collect());
                i = k;
            } else {
                out.push(cs[i..j].iter().collect());
                i = j;
            }
        } else if space_char(c) {
            i += 1;
        } else {
            let mut j = i;
            while j < cs.len() && !word_char(cs[j]) && !space_char(cs[j]) {
                j += 1;
            }
            out.push(cs[i..j].iter().collect());
            i = j;
        }
    }
    out
}

/// `KEEP = re.compile(r"^(?=.*[a-zà-ÿ])[^\"“”()\[\]{}<>]{25,220}$")`, without a regex crate (no lookahead): a length in
/// code points between 25 and 220, at least one French lower-case letter, and none of the excluded characters.
pub fn keep(text: &str) -> bool {
    let n = text.chars().count();
    if !(25..=220).contains(&n) {
        return false;
    }
    let has_lower = text.chars().any(|c| c.is_ascii_lowercase() || ('à'..='ÿ').contains(&c));
    let excluded = |c: char| matches!(c, '"' | '“' | '”' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>');
    has_lower && !text.chars().any(excluded)
}

/// `PAIRS`: lower-cased form -> its partner. Built in bank order, later entries overwriting earlier ones (the Python
/// `PAIRS[w.lower()] = c; PAIRS[c.lower()] = w` loop).
pub fn pairs() -> HashMap<String, String> {
    let mut m = HashMap::new();
    for (w, c) in BANKS {
        m.insert(w.to_lowercase(), c.to_string());
        m.insert(c.to_lowercase(), w.to_string());
    }
    m
}

/// Python `s[:1].upper() + s[1:]`.
fn cap_first(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + it.as_str(),
    }
}

/// `re.sub(r"\b" + re.escape(right) + r"\b", wrong, text, count=1)` for a `right` that starts and ends with a word
/// character (every bank form and every token does). Returns None when the pattern is absent (Python returns the
/// unchanged text; the caller compares and skips).
fn replace_word(text: &str, right: &str, wrong: &str) -> Option<String> {
    let cs: Vec<char> = text.chars().collect();
    let r: Vec<char> = right.chars().collect();
    if r.is_empty() || r.len() > cs.len() {
        return None;
    }
    let boundary_before = |i: usize| i == 0 || !word_char(cs[i - 1]);
    let boundary_after = |i: usize| i + r.len() == cs.len() || !word_char(cs[i + r.len()]);
    for start in 0..=cs.len() - r.len() {
        if cs[start..start + r.len()] == r[..] && boundary_before(start) && boundary_after(start) {
            let mut out = String::with_capacity(text.len());
            out.extend(&cs[..start]);
            out.push_str(wrong);
            out.extend(&cs[start + r.len()..]);
            return Some(out);
        }
    }
    None
}

/// One produced dataset (items and the card's measured parts).
pub struct ProductPairs {
    pub items: Vec<Value>,
    pub body: String,
    pub carriers: usize,
    pub load_secs: f64,
    pub build_secs: f64,
}

/// `main()`'s body: carriers -> shuffled -> one pair each until `n`.
pub fn generate(src_lines: &[String], n: usize, seed: u64) -> ProductPairs {
    let t0 = std::time::Instant::now();
    let pairs = pairs();
    // Python reads the file line by line and appends; the rayon map over indexed lines keeps that order.
    use rayon::prelude::*;
    let mut carriers: Vec<String> = src_lines
        .par_iter()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            let t = v.get("text").and_then(Value::as_str).unwrap_or("").trim().replace('\n', " ");
            (keep(&t) && t.matches(' ').count() >= 4).then_some(t)
        })
        .collect();
    let load_secs = t0.elapsed().as_secs_f64();
    let mut rng = PyRandom::new(seed);
    rng.shuffle(&mut carriers);

    let t1 = std::time::Instant::now();
    let mut items: Vec<Value> = Vec::new();
    for text in &carriers {
        let toks = tokens(text);
        let cand: Vec<&String> = toks.iter().filter(|w| pairs.contains_key(&w.to_lowercase())).collect();
        if cand.is_empty() {
            continue;
        }
        let k = rng.randbelow(cand.len() as u64) as usize;
        let w = cand[k];
        let right = w.as_str();
        let mut wrong = pairs[&w.to_lowercase()].clone();
        if right.chars().next().is_some_and(|c| c.is_uppercase()) {
            wrong = cap_first(&wrong);
        }
        let Some(bad_s) = replace_word(text, right, &wrong) else { continue };
        if bad_s == *text {
            continue;
        }
        let split = if items.len().is_multiple_of(10) { "probe" } else { "train" };
        let mut m = Map::new();
        m.insert("id".into(), json!(format!("prod-{:04}", items.len())));
        m.insert("kind".into(), json!("pair"));
        m.insert("valid".into(), json!(true));
        m.insert("good".into(), json!(text));
        m.insert("bad".into(), json!(bad_s));
        m.insert("explanation".into(), json!(format!("{right} -> {wrong} in a real book sentence")));
        m.insert("split".into(), json!(split));
        items.push(Value::Object(m));
        if items.len() >= n {
            break;
        }
    }
    let build_secs = t1.elapsed().as_secs_f64();
    let mut body = String::new();
    for it in &items {
        body.push_str(&vaugelas::text::pyjson::dumps(it));
        body.push('\n');
    }
    ProductPairs { carriers: carriers.len(), items, body, load_secs, build_secs }
}

pub fn sha256(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The card. The wall times are measured and therefore differ between runs (the JSONL does not); the parity tool
/// compares the dataset, not this line.
pub fn card(p: &ProductPairs, sha: &str, total_secs: f64) -> String {
    let probe = p.items.iter().filter(|x| x["split"] == "probe").count();
    [
        "# Product-passage pairs — batch 1".to_string(),
        String::new(),
        format!("carriers: {} clean book sentences; items: {}; split probe {}", p.carriers, p.items.len(), probe),
        format!("sha256 items.jsonl: `{sha}`"),
        String::new(),
        format!("wall time: load {:.1}s, build {:.1}s, total {:.1}s", p.load_secs, p.build_secs, total_secs),
        String::new(),
        "Each bad sentence differs from its good carrier by exactly one real French word, in real book context.".to_string(),
    ]
    .join("\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizer_matches_python_examples() {
        assert_eq!(tokens("L’homme qu’il voit, dit-il."), vec!["L’homme", "qu’il", "voit", ",", "dit", "-", "il", "."]);
        assert_eq!(tokens("c'est"), vec!["c'est"]);
        assert_eq!(tokens("fin'"), vec!["fin", "'"]);
    }

    #[test]
    fn keep_filter() {
        assert!(!keep("Trop court."));
        assert!(keep("Une phrase française assez longue pour le filtre, avec des mots."));
        assert!(!keep("Une phrase française assez longue pour le filtre (mais) avec des mots."));
        assert!(!keep("TOUT EN MAJUSCULES SANS MINUSCULE ICI, POUR LE FILTRE."));
    }

    #[test]
    fn replacement_respects_word_boundaries() {
        assert_eq!(replace_word("la foi et la foi", "foi", "foie"), Some("la foie et la foi".into()));
        assert_eq!(replace_word("foie", "foi", "foie"), None);
    }
}
