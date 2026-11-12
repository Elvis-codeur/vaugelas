//! Text that is not French prose and must not be checked as if it were.
//!
//! Our register is Scripture-quoting theological prose: every passage carries verse references, footnote
//! letters preserved inline (`vrai,b`), bracketed footnote markers (`[114]`), markdown anchors
//! (`[Back to Top](#Content)`) and Roman numerals in citations (`Rom. xv. 4`). Checked as prose, that apparatus
//! produced 46 of the 55 flags Vaugelas raised on the 55-passage reference corpus, none of them a translation
//! defect (`2026-11-10-prompt-ceiling/vaugelas/VAUGELAS-EVALUATION.md`). This module computes the byte ranges to
//! leave alone, and the rule report drops every finding that overlaps one — except the `FR_REF_*` rules, which
//! are about the apparatus and read the raw text.
//!
//! What is masked:
//! - a markdown link or anchor: `[label](#target)`;
//! - a bracketed footnote marker: `[114]`, `[2]`;
//! - a footnote letter glued to punctuation: the `,b` of `vrai,b`, the `;r` of `gloire ;r` (letters `a`–`t`);
//! - a citation: `Genèse 3:8`, `Psaumes 63:3, 2`, `(1 Cor. x. 11, Rom. xv. 4, iv. 23)`, `(Col. i. 9, 10)`,
//!   with the Roman numeral that stands for a chapter (`xv`, `iv`, `iii`).
//!
//! Latin quotations are handled where the lexicon is: [`super::checker`] does not report an unknown word when
//! the word next to it is unknown too (a run of foreign words), and a single unknown word with no plausible
//! neighbour is reported as an invented word rather than as a misspelling.
//!
//! Determinism: a pure function of the text; no state, no map iteration.

/// Why a range is masked. Recorded so a diagnostic can say what was skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `[label](#target)`
    Markdown,
    /// `[114]`, `[2]`
    FootnoteMarker,
    /// `,b` in `vrai,b`
    FootnoteLetter,
    /// `Genèse 3:8`, `(1 Cor. x. 11, Rom. xv. 4)` and the Roman numerals inside it
    Citation,
}

/// A byte range of the checked text that is apparatus, not prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mask {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

impl Mask {
    /// True when a finding over `start..end` overlaps this mask.
    pub fn overlaps(&self, start: usize, end: usize) -> bool { start < self.end && end > self.start }
}

/// Every masked range of `text`, sorted by start byte.
pub fn ranges(text: &str) -> Vec<Mask> {
    let mut v = Vec::new();
    markdown(text, &mut v);
    footnote_markers(text, &mut v);
    footnote_letters(text, &mut v);
    citations(text, &mut v);
    v.sort_by_key(|m| (m.start, m.end));
    v.dedup_by_key(|m| (m.start, m.end));
    v
}

/// `[label](target)` or `[label][ref]`: a markdown link is not prose in any of its parts.
fn markdown(text: &str, v: &mut Vec<Mask>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'[' { i += 1; continue; }
        let Some(close) = b[i + 1..].iter().position(|&c| c == b']').map(|p| i + 1 + p) else { i += 1; continue };
        // `[label](` or `[label][`
        let next = b.get(close + 1).copied();
        if matches!(next, Some(b'(') | Some(b'[')) {
            if next == Some(b'(') {
                if let Some(end) = b[close + 2..].iter().position(|&c| c == b')').map(|p| close + 2 + p + 1) {
                    v.push(Mask { start: i, end, kind: Kind::Markdown });
                    i = end;
                    continue;
                }
            } else if let Some(end) = b[close + 2..].iter().position(|&c| c == b']').map(|p| close + 2 + p + 1) {
                v.push(Mask { start: i, end, kind: Kind::Markdown });
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

/// `[114]`: a footnote marker, a bare bracketed number with no link target.
fn footnote_markers(text: &str, v: &mut Vec<Mask>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'[' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() { j += 1; }
            if j > i + 1 && b.get(j) == Some(&b']') {
                v.push(Mask { start: i, end: j + 1, kind: Kind::FootnoteMarker });
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
}

/// A single lowercase letter glued to `, ; : .` is the source's footnote apparatus kept inline (`vrai,b`, `;u`).
fn footnote_letters(text: &str, v: &mut Vec<Mask>) {
    let c: Vec<(usize, char)> = text.char_indices().collect();
    for k in 1..c.len() {
        let (pb, p) = c[k];
        if !matches!(p, ',' | ';' | ':' | '.') { continue; }
        // the punctuation must follow a word or a space (the source prints "gloire ;r" as well as "vrai,b")
        if !c[k - 1].1.is_alphanumeric() && !c[k - 1].1.is_whitespace()
            && !matches!(c[k - 1].1, '»' | '”' | ')' | '\'') { continue; }
        let Some(&(lb, l)) = c.get(k + 1) else { continue };
        if !l.is_ascii_lowercase() { continue; }
        // and must be followed by a boundary, not by more letters of a word ("et,il" is a real missing space)
        let after = c.get(k + 2).map(|x| x.1);
        if after.is_some_and(|a| a.is_alphanumeric() || a == '\'' || a == '’') { continue; }
        v.push(Mask { start: pb, end: lb + l.len_utf8(), kind: Kind::FootnoteLetter });
    }
}

/// English book abbreviations that carry a period (`Cor.`, `Tim.`, `Rom.`, `Ps.`, `Gen.`…).
const BOOK_ABBREV: &[&str] = &[
    "Gen", "Ex", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "Sam", "Kgs", "Chron", "Ezra", "Neh", "Esth",
    "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer", "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad",
    "Jonah", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal", "Matt", "Mark", "Luke", "John", "Acts", "Rom",
    "Cor", "Gal", "Eph", "Phil", "Col", "Thess", "Tim", "Titus", "Phlm", "Heb", "Jas", "Pet", "Jude", "Rev",
    "I", "II", "III", "1", "2", "3",
];

/// Is `w` a book abbreviation (case-insensitively) with or without its period?
pub(crate) fn is_book_abbrev(w: &str) -> bool {
    let w = w.trim_end_matches('.');
    BOOK_ABBREV.iter().any(|a| a.eq_ignore_ascii_case(w))
}

/// Roman numeral (lowercase, the form citations use for a chapter): `i`, `iv`, `xv`, `xii`.
pub(crate) fn is_roman(w: &str) -> bool {
    !w.is_empty() && w.len() <= 6 && w.chars().all(|c| "ivxlcdm".contains(c))
}

/// Byte span of the word ending/starting at a position, for the citation scanner.
fn word_before(b: &[u8], mut i: usize) -> Option<(usize, usize)> {
    while i > 0 && b[i - 1].is_ascii_whitespace() { i -= 1; }
    let end = i;
    // bytes >= 0x80 are the continuation and lead bytes of a French letter (É, è, ç), so they belong to the word
    while i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.' || b[i - 1] >= 0x80) { i -= 1; }
    if i == end { None } else { Some((i, end)) }
}

fn skip_ws(b: &[u8], mut i: usize) -> usize { while i < b.len() && b[i] == b' ' { i += 1; } i }

/// Citations: a chapter:verse anchor (`3:8`, `63:3`), a parenthesis that holds one, and a book
/// abbreviation followed by a Roman numeral chapter (`Cor. x. 11`).
fn citations(text: &str, v: &mut Vec<Mask>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() { i += 1; continue; }
        // `digits : digits` — the anchor of a verse reference
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() { j += 1; }
        if b.get(j) != Some(&b':') { i += 1; continue; }
        let mut k = j + 1;
        while k < b.len() && b[k].is_ascii_digit() { k += 1; }
        if k == j + 1 { i += 1; continue; }
        // extend the user's list form `63:3, 2` and the range form `5:19-21`
        loop {
            let mut m = skip_ws(b, k);
            if b.get(m) == Some(&b'-') || text[m..].starts_with('\u{2013}') {
                m += 1;
                let m2 = skip_ws(b, m);
                let mut e = m2;
                while e < b.len() && b[e].is_ascii_digit() { e += 1; }
                if e > m2 { k = e; continue; }
            }
            if b.get(m) == Some(&b',') {
                let m2 = skip_ws(b, m + 1);
                let mut e = m2;
                while e < b.len() && b[e].is_ascii_digit() { e += 1; }
                if e > m2 { k = e; continue; }
            }
            break;
        }
        // backward over the book: "Genèse 3:8", "1 Jean 5:16", "II Tim. iii. 16" (roman handled below)
        let mut start = i;
        for _ in 0..3 {
            let Some((ws, we)) = word_before(b, start) else { break };
            let w = &text[ws..we];
            if is_roman(&w.to_lowercase()) || is_book_abbrev(w) {
                start = ws;
            } else if w.chars().next().is_some_and(|c| c.is_uppercase()) && w.len() > 1 {
                start = ws; // a book name: "Genèse", "Psaumes", "Hébreux"
            } else {
                break;
            }
        }
        let mut end = k;
        // a trailing period that closes the citation ("Genèse 3:8.")
        if b.get(end) == Some(&b'.') && !b.get(end + 1).is_some_and(|c| c.is_ascii_digit()) { end += 1; }
        let inside_parens = start > 0 && b[start - 1] == b'(' && b.get(end) == Some(&b')');
        if inside_parens {
            v.push(Mask { start: start - 1, end: end + 1, kind: Kind::Citation });
            i = end + 1;
        } else {
            v.push(Mask { start, end, kind: Kind::Citation });
            i = end;
        }
    }
    // A parenthesis holding several references: "(1 Cor. x. 11, Rom. xv. 4, iv. 23, 1 Cor. ix. 10, iv. 6)".
    // Mask the whole parenthesised span when it holds an abbreviation or a Roman chapter.
    let mut depth: Vec<usize> = Vec::new();
    for (p, &c) in b.iter().enumerate() {
        match c {
            b'(' => depth.push(p),
            b')' => {
                if let Some(s) = depth.pop() {
                    let inner = &text[s + 1..p];
                    let has_abbrev = inner
                        .split(|c: char| !c.is_alphanumeric() && c != '.')
                        .any(|w| w.ends_with('.') && is_book_abbrev(w))
                        || inner.split_whitespace().any(is_book_abbrev);
                    let has_roman = inner.split(|c: char| !c.is_alphanumeric()).any(|w| is_roman(&w.to_lowercase()) && w.len() > 1);
                    let has_verse = inner.contains(':');
                    if has_abbrev || has_roman || has_verse {
                        v.push(Mask { start: s, end: p + 1, kind: Kind::Citation });
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masked(text: &str) -> Vec<&str> {
        ranges(text).into_iter().map(|m| &text[m.start..m.end]).collect()
    }

    #[test]
    fn footnote_letter_glued_to_punctuation() {
        assert_eq!(masked("vrai,b et très juste,q"), vec![",b", ",q"]);
        assert_eq!(masked("la gloire ;r de Dieu"), vec![";r"]);
        // a real missing space before a word is not apparatus
        assert!(masked("et,il").is_empty());
    }

    #[test]
    fn bracketed_markers_and_markdown() {
        assert_eq!(masked("la foi[114] et"), vec!["[114]"]);
        assert_eq!(masked("[Back to Top](#Content)"), vec!["[Back to Top](#Content)"]);
    }

    #[test]
    fn citations_keep_their_roman_numerals_out_of_the_check() {
        let got = masked("(1 Cor. x. 11, Rom. xv. 4, iv. 23, 1 Cor. ix. 10, iv. 6)");
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].starts_with("(1 Cor."));
        assert_eq!(masked("Ézéchiel 8:6,13,15"), vec!["Ézéchiel 8:6,13,15"]);
        assert_eq!(masked("Psaumes 63:3, 2."), vec!["Psaumes 63:3, 2."]);
        assert_eq!(masked("(Hébreux 4:13)"), vec!["(Hébreux 4:13)"]);
    }

    #[test]
    fn plain_prose_is_not_masked() {
        for s in ["Il a mangé une pomme, puis il est parti.", "Le monde est un pont ; passe dessus.", "10 Mon bouclier est en Dieu"] {
            assert!(masked(s).is_empty(), "{s}: {:?}", masked(s));
        }
    }
}
