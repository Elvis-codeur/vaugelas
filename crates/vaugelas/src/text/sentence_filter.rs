//! Port of training-design/sentence_filter.py: which sentences the checker scores at all, and the markup removed
//! before tokenizing (`clean`). The regular expressions are implemented by hand with Python `re` semantics
//! (Unicode `\s`, `\d`, `\w`/`\b`, `re.IGNORECASE` case folding from `pystr`).
use crate::text::natural::Natural;
use crate::text::pystr::{ci_eq, has_w, is_decimal, is_digit, is_space, is_w, lower, strip};

/// `EN`: English function words.
const EN: [&str; 26] = ["the", "and", "of", "to", "is", "in", "for", "with", "that", "it", "his", "was", "he", "are", "be", "this",
                        "as", "by", "my", "when", "which", "from", "were", "have", "not", "but"];
/// `FR_TOO`: also French (or common in French text), never counted as English.
const FR_TOO: [&str; 7] = ["est", "in", "as", "be", "but", "not", "par"];

/// Why a sentence is not scored (`skip` returns its name; Python returns the string or None).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip { Empty, English, Foreign, Markup }

impl Skip {
    pub fn name(self) -> &'static str {
        match self { Skip::Empty => "empty", Skip::English => "english", Skip::Foreign => "foreign", Skip::Markup => "markup" }
    }
}

/// STRIP = `\[\s*\d+\s*\]|_|\[\s*Note[^\]]*\]` tried at `cs[p]`: the end of the match (exclusive), if any.
fn strip_at(cs: &[char], p: usize) -> Option<usize> {
    let n = cs.len();
    if cs[p] == '_' { return Some(p + 1); }
    if cs[p] != '[' { return None; }
    let skip_ws = |mut j: usize| { while j < n && is_space(cs[j]) { j += 1; } j };
    // alternative 1: [ \s* \d+ \s* ]   (\s and \d never overlap, so no backtracking is needed)
    let j = skip_ws(p + 1);
    let mut k = j;
    while k < n && is_decimal(cs[k]) { k += 1; }
    if k > j {
        let k = skip_ws(k);
        if k < n && cs[k] == ']' { return Some(k + 1); }
    }
    // alternative 3: [ \s* Note [^\]]* ]   (greedy run of non-']' then ']' = up to the first ']')
    let j = skip_ws(p + 1);
    if j + 4 <= n && cs[j..j + 4] == ['N', 'o', 't', 'e'] {
        let mut k = j + 4;
        while k < n && cs[k] != ']' { k += 1; }
        if k < n { return Some(k + 1); }
    }
    None
}

/// `clean(text)`: `re.sub(r"\s{2,}", " ", STRIP.sub(" ", text)).strip()`.
pub fn clean(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let mut a: Vec<char> = Vec::with_capacity(cs.len());
    let mut p = 0;
    while p < cs.len() {
        match strip_at(&cs, p) {
            Some(e) => { a.push(' '); p = e; }
            None => { a.push(cs[p]); p += 1; }
        }
    }
    let mut b = String::with_capacity(text.len());
    let mut p = 0;
    while p < a.len() {
        if is_space(a[p]) {
            let mut e = p + 1;
            while e < a.len() && is_space(a[e]) { e += 1; }
            if e - p >= 2 { b.push(' '); } else { b.push(a[p]); }
            p = e;
        } else {
            b.push(a[p]);
            p += 1;
        }
    }
    strip(&b).to_string()
}

/// `\b` at position `p` of `cs` (Unicode word characters).
fn boundary(cs: &[char], p: usize) -> bool {
    let before = p > 0 && is_w(cs[p - 1]);
    let after = p < cs.len() && is_w(cs[p]);
    before != after
}

/// `cs[p..]` starts with `lit` under re.IGNORECASE; returns the end.
fn ci_lit(cs: &[char], p: usize, lit: &str) -> Option<usize> {
    let mut j = p;
    for c in lit.chars() {
        if j >= cs.len() || !ci_eq(cs[j], c) { return None; }
        j += 1;
    }
    Some(j)
}

/// MARK = `\||\bcf\b|\bNote \d|\bibid\b|\bop\. ?cit` (re.I): does it match anywhere in `s`?
pub fn mark_search(s: &str) -> bool {
    let cs: Vec<char> = s.chars().collect();
    for p in 0..=cs.len() {
        if p < cs.len() && cs[p] == '|' { return true; }
        if !boundary(&cs, p) { continue; }
        if let Some(e) = ci_lit(&cs, p, "cf") { if boundary(&cs, e) { return true; } }
        if let Some(e) = ci_lit(&cs, p, "note ") { if e < cs.len() && is_decimal(cs[e]) { return true; } }
        if let Some(e) = ci_lit(&cs, p, "ibid") { if boundary(&cs, e) { return true; } }
        if let Some(e) = ci_lit(&cs, p, "op.") {
            let e = if e < cs.len() && cs[e] == ' ' { e + 1 } else { e };
            if ci_lit(&cs, e, "cit").is_some() { return true; }
        }
    }
    false
}

/// `skip(words, READ)` -> reason, or None when the sentence is scored.
pub fn skip(words: &[String], nat: &Natural) -> Option<Skip> {
    let low: Vec<String> = words.iter().filter(|w| has_w(w)).map(|w| lower(w)).collect();
    if low.is_empty() { return Some(Skip::Empty); }
    let en = low.iter().filter(|w| EN.contains(&w.as_str()) && !FR_TOO.contains(&w.as_str())).count();
    if en >= 2 { return Some(Skip::English); }
    let unk = low.iter().filter(|w| !nat.read.contains_key(w.as_str()) && !w.chars().next().is_some_and(is_digit)).count();
    if unk as f64 / low.len() as f64 > 0.35 && low.len() >= 4 { return Some(Skip::Foreign); }
    if mark_search(&words.join(" ")) { return Some(Skip::Markup); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_examples() {
        // from the kit (ref_filter_tokens): italics marks and runs of spaces
        assert_eq!(clean("Franc._) SOMMAIRE."), "Franc. ) SOMMAIRE.");
        assert_eq!(clean("que _liberté_ n'est"), "que liberté n'est");
        assert_eq!(clean("cabane. *       *  x"), "cabane. * * x");
        assert_eq!(clean("national[B], doux[ 12 ]. [Note: voir] fin"), "national[B], doux . fin");
        assert_eq!(clean("[Note sans fin"), "[Note sans fin");
        assert_eq!(clean("a\u{2003}\u{2003}b c\td"), "a b c\td"); // one \s is kept as is
    }
    #[test]
    fn mark_examples() {
        assert!(mark_search("voir cf . p"));
        assert!(mark_search("CF ."));
        assert!(!mark_search("cfa"));
        assert!(mark_search("Note 3"));
        assert!(mark_search("note ٣")); // \d is any Unicode decimal digit
        assert!(!mark_search("Notes 3"));
        assert!(mark_search("op.cit"));
        assert!(mark_search("Op. Cit."));
        assert!(mark_search("a | b"));
        assert!(!mark_search("ibidem"));
        assert!(mark_search("ibid ."));
    }
}
