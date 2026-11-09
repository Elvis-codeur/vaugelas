//! French tokenizer: words (with elision split: `l’enfant` -> `l’` + `enfant`), punctuation, sentences.
use crate::rules::lexicon::Morph;

#[derive(Clone, Debug)]
pub struct Tok {
    pub text: String,
    pub lower: String, // lowercase, typographic apostrophe
    pub start: usize,  // byte offsets
    pub end: usize,
    pub word: bool,
    pub known: bool,
    pub compound: bool,
    pub skip: bool, // inside a URL / e-mail / path
    pub morph: Vec<Morph>,
    pub sid: u32,
    pub sent_start: bool,
    pub space_before: bool,
    /// inverted subject pronoun attached to a verb ("avez-vous", "a-t-il"): (person bits, number bits)
    pub inv: Option<(u8, u8)>,
}

fn new_tok(text: &str, start: usize, end: usize, word: bool, space_before: bool) -> Tok {
    Tok {
        text: text.to_string(),
        lower: text.replace('\'', "’").to_lowercase(),
        start, end, word, known: false, compound: false, skip: false,
        morph: Vec::new(), sid: 0, sent_start: false, space_before, inv: None,
    }
}

const JOIN_ELISION: [&str; 4] = ["aujourd", "quelqu", "presqu", "prud"];

pub fn tokenize(text: &str) -> Vec<Tok> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let byte_at = |k: usize| if k < n { chars[k].0 } else { text.len() };
    let mut toks: Vec<Tok> = Vec::new();
    let (mut i, mut space) = (0usize, true);
    while i < n {
        let c = chars[i].1;
        if c.is_whitespace() { i += 1; space = true; continue; }
        if c.is_alphanumeric() {
            let mut j = i + 1;
            while j < n {
                let cj = chars[j].1;
                if cj.is_alphanumeric() { j += 1; continue; }
                if cj == '-' && j + 1 < n && chars[j + 1].1.is_alphanumeric() { j += 1; continue; }
                if (cj == '\'' || cj == '’') && j + 1 < n && chars[j + 1].1.is_alphabetic() && chars[j - 1].1.is_alphabetic() {
                    let so_far = text[byte_at(i)..byte_at(j)].to_lowercase();
                    // "aujourd'hui"; and "a-t'il", "va-t'en": keep the euphonic t / elided te inside the word
                    if JOIN_ELISION.contains(&so_far.as_str()) || so_far.ends_with("-t") || so_far.ends_with("-d") { j += 1; continue; }
                    j += 1;
                    break;
                }
                break;
            }
            let (s, e) = (byte_at(i), byte_at(j));
            toks.push(new_tok(&text[s..e], s, e, true, space));
            i = j; space = false; continue;
        }
        let (s, e) = (byte_at(i), byte_at(i + 1));
        toks.push(new_tok(&text[s..e], s, e, false, space));
        i += 1; space = false;
    }
    // sentences
    let (mut sid, mut pending) = (0u32, true);
    let len = toks.len();
    for k in 0..len {
        toks[k].sid = sid;
        if toks[k].word && pending { toks[k].sent_start = true; pending = false; }
        if !toks[k].word && matches!(toks[k].text.as_str(), "." | "!" | "?" | "…") {
            let boundary = k + 1 == len || {
                let nx = &toks[k + 1];
                if nx.word { nx.text.chars().next().is_some_and(|c| c.is_uppercase()) }
                else { matches!(nx.text.as_str(), "«" | "—" | "(" | "\"" | "“" | "-" | "–") }
            };
            if boundary { sid += 1; pending = true; }
        }
    }
    for (s, e) in skip_ranges(text) {
        for t in toks.iter_mut() { if t.start >= s && t.end <= e { t.skip = true; } }
    }
    toks
}

/// Byte ranges of whitespace-delimited chunks that look like URLs, e-mails or paths
/// (contain `@`, `://`, `www.`, `/`, `\\` or `_`). Rules leave them alone.
pub fn skip_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut cs: Option<usize> = None;
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (b, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = cs.take() { ranges.push((s, b)); }
        } else if cs.is_none() { cs = Some(b); }
    }
    if let Some(s) = cs { ranges.push((s, text.len())); }
    ranges.retain(|&(s, e)| {
        let ch = &text[s..e];
        ch.contains('@') || ch.contains("://") || ch.contains("www.") || ch.contains('/') || ch.contains('\\') || ch.contains('_')
    });
    ranges
}
