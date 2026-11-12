//! Quotation and reference integrity (#19): the apparatus a reader checks first and the checker never looked at.
//!
//! Recorded failures this is written from (`2026-11-10-prompt-ceiling/reference/review.jsonl` and the judged model
//! outputs):
//! - a closing guillemet with no opening one (`… ta gloire. »`, Ministral r330644);
//! - the same reference twice in one passage (`[Psaumes 63:3] … Psaumes 63:3, 2`, Gemma r330644);
//! - the source's English citation style kept verbatim (`1 Cor. x. 11, Rom. xv. 4, iv. 23`, r332480; `2 Cor.
//!   5:19-21`, r313915);
//! - Segond text under KJV verse numbers, i.e. bare verse numbers printed inside the prose (`10 Mon bouclier … 11
//!   Dieu est un juste juge`, r326124);
//! - a reference's verse list glued together without the space French requires (`Ézéchiel 8:6,13,15`, r313650).
//!
//! These rules read the raw text, not the tokens, and they are the reason [`super::mask`] lets `FR_REF_*` findings
//! through a mask: the apparatus is their subject.
//!
//! Determinism: pure functions of the text; a finding names the condition that decided it.
use crate::rules::mask::{is_book_abbrev, is_roman};
use super::*;

/// One parsed verse reference: `Psaumes 63:3`, `Rom. xv. 4`, `1 Cor. ix. 10`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Ref {
    pub start: usize,
    pub end: usize,
    pub book: String,
    pub chapter: String,
    pub verses: Vec<String>,
}

/// Every `Book chapter:verse` reference in `text`, in order.
///
/// A reference is a `digits:digits` anchor, optionally preceded by a book name or an abbreviation (with a leading
/// ordinal: `1 Jean 5:16`, `II Tim. iii. 16`), followed by the user's list (`63:3, 2`) and range (`5:19-21`) forms.
pub(super) fn references(text: &str) -> Vec<Ref> {
    let b = text.as_bytes();
    let mut out: Vec<Ref> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() { i += 1; continue; }
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() { j += 1; }
        if b.get(j) != Some(&b':') { i += 1; continue; }
        let mut k = j + 1;
        while k < b.len() && b[k].is_ascii_digit() { k += 1; }
        if k == j + 1 { i += 1; continue; }
        let chapter = &text[i..j];
        let mut verses: Vec<String> = vec![text[j + 1..k].to_string()];
        loop {
            let mut m = k;
            while m < b.len() && b[m] == b' ' { m += 1; }
            if b.get(m) == Some(&b',') {
                let mut e = m + 1;
                while e < b.len() && b[e] == b' ' { e += 1; }
                let vs = e;
                while e < b.len() && b[e].is_ascii_digit() { e += 1; }
                if e > vs { verses.push(text[vs..e].to_string()); k = e; continue; }
            }
            if b.get(m) == Some(&b'-') {
                let mut e = m + 1;
                let vs = e;
                while e < b.len() && b[e].is_ascii_digit() { e += 1; }
                if e > vs { verses.push(text[vs..e].to_string()); k = e; continue; }
            }
            break;
        }
        // backward over the book name / abbreviation
        let mut start = i;
        let mut book = String::new();
        for _ in 0..3 {
            let mut ws = start;
            while ws > 0 && b[ws - 1] == b' ' { ws -= 1; }
            let we = ws;
            let mut p = ws;
            while p > 0 && (b[p - 1].is_ascii_alphanumeric() || b[p - 1] == b'.') { p -= 1; }
            if p == we { break; }
            let w = &text[p..we];
            if is_roman(&w.to_lowercase()) || is_book_abbrev(w) || (w.chars().next().is_some_and(|c| c.is_uppercase()) && w.chars().count() > 1) {
                start = p;
                if book.is_empty() && !w.chars().all(|c| c.is_ascii_digit()) { book = w.trim_end_matches('.').to_string(); }
            } else {
                break;
            }
        }
        let mut end = k;
        if b.get(end) == Some(&b'.') && !b.get(end + 1).is_some_and(|c| c.is_ascii_digit()) { end += 1; }
        if !book.is_empty() {
            out.push(Ref { start, end, book, chapter: chapter.to_string(), verses });
        }
        i = end;
    }
    out
}

impl Checker {
    /// Rules about one citation as it is written: its style and its comma spacing. The rules that need the
    /// whole passage (a repeated reference, verse numbers, quotation balance) run in [`Self::document_rules`].
    pub(super) fn reference_rules(&self, text: &str, o: &mut Vec<Lint>, strict: bool) {
        self.english_citation_style(text, o);
        if strict { self.reference_comma(text, o); }
    }

    /// Guillemets are balanced over the whole document: a quotation may open in one paragraph and close in the
    /// next (two of the 55 reference passages do exactly that). Only a closing mark with no opening one before
    /// it is reported — the recorded failure (`… ta gloire. »`, Ministral r330644). An opening mark left open is
    /// not reported: at chunk level the closing mark is often in the next chunk.
    pub fn document_rules(&self, text: &str) -> Vec<Lint> {
        let mut out = Vec::new();
        self.duplicate_references(text, &mut out);
        self.verse_numbers(text, &mut out);
        let mut depth: i32 = 0;
        for (b, c) in text.char_indices() {
            match c {
                '«' => depth += 1,
                '»' => {
                    depth -= 1;
                    if depth < 0 {
                        out.push(Lint {
                            start: b,
                            end: b + c.len_utf8(),
                            rule: "FR_REF_QUOTE",
                            message: "Guillemet fermant sans guillemet ouvrant.".to_string(),
                            suggestions: vec!["« ".to_string()],
                        });
                        depth = 0;
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// A reference printed twice in one passage is apparatus carried over, not a second citation.
    fn duplicate_references(&self, text: &str, o: &mut Vec<Lint>) {
        let mut seen: Vec<(usize, Ref)> = Vec::new();
        for r in references(text) {
            // "Psaumes 63:3" then "Psaumes 63:3, 2": the same verse, so the same citation carried over. The
            // match is on book + chapter + a shared verse, not on the exact printed list.
            let dup = seen.iter().any(|(_, s)| {
                s.book.eq_ignore_ascii_case(&r.book) && s.chapter == r.chapter
                    && s.verses.iter().any(|v| r.verses.contains(v))
            });
            if dup {
                o.push(Lint {
                    start: r.start,
                    end: r.end,
                    rule: "FR_REF_DUP",
                    message: format!("Référence répétée dans le passage ({}) : elle n’est citée qu’une fois.", text[r.start..r.end].trim()),
                    suggestions: Vec::new(),
                });
            } else {
                seen.push((r.start, r));
            }
        }
    }

    /// The source's English citation style kept verbatim: an English book abbreviation with a Roman chapter
    /// (`Cor. x. 11`, `Tim. iii. 16`) or a Chapter.Verse form (`xiv. 6`).
    fn english_citation_style(&self, text: &str, o: &mut Vec<Lint>) {
        let b = text.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if !b[i].is_ascii_alphabetic() { i += 1; continue; }
            let ws = i;
            while i < b.len() && b[i].is_ascii_alphabetic() { i += 1; }
            let we = i;
            if b.get(we) != Some(&b'.') { continue; }
            let w = &text[ws..we];
            if !is_book_abbrev(w) { i = we; continue; }
            // Roman chapter, optional period, then a verse number: "x. 11", "xv. 4", "iii. 16"
            let mut p = we + 1;
            while p < b.len() && b[p] == b' ' { p += 1; }
            let rs = p;
            while p < b.len() && b[p].is_ascii_alphabetic() { p += 1; }
            let mut q = p;
            if b.get(q) == Some(&b'.') { q += 1; }
            let mut r = q;
            while r < b.len() && b[r] == b' ' { r += 1; }
            let vs = r;
            while r < b.len() && b[r].is_ascii_digit() { r += 1; }
            let roman = &text[rs..p];
            if r > vs && is_roman(roman) {
                o.push(Lint {
                    start: ws,
                    end: r,
                    rule: "FR_REF_STYLE",
                    message: "Référence au style anglais (abréviation et chapitre en chiffres romains) dans un texte français.".to_string(),
                    suggestions: Vec::new(),
                });
                i = r;
            } else {
                i = we;
            }
        }
    }

    /// Bare verse numbers inside the prose: a line that starts with `10 Mon …` and no period after the number,
    /// twice or more in the passage. That is a Bible text printed with its verse numbers, not a numbered list.
    fn verse_numbers(&self, text: &str, o: &mut Vec<Lint>) {
        let mut found: Vec<(usize, usize, String)> = Vec::new();
        let mut off = 0usize;
        for line in text.split('\n') {
            let lead = line.len() - line.trim_start().len();
            let t = line.trim_start();
            let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() && digits.len() <= 3 {
                let rest = &t[digits.len()..];
                let after_ok = rest.starts_with(' ') && rest.trim_start().chars().next().is_some_and(|c| c.is_uppercase());
                // "10 Mon" is a verse number; "1. Introduction" (period) and "10:30" are not
                if after_ok && !rest.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
                    found.push((off + lead, off + lead + digits.len(), digits));
                }
            }
            off += line.len() + 1;
        }
        if found.len() < 2 { return; }
        // consecutive: 10, 11, 12 … (the model keeps the source's numbering, which is sequential)
        let nums: Vec<u32> = found.iter().filter_map(|(_, _, d)| d.parse().ok()).collect();
        let consecutive = nums.windows(2).filter(|w| w[1] == w[0] + 1).count() * 2 >= nums.len().saturating_sub(1);
        if !consecutive { return; }
        for (start, end, _) in found {
            o.push(Lint {
                start,
                end,
                rule: "FR_REF_VERSE_NUM",
                message: "Numéro de verset imprimé dans le texte : la numérotation de l’édition source reste dans le passage.".to_string(),
                suggestions: Vec::new(),
            });
        }
    }

    /// `Ézéchiel 8:6,13,15`: the commas of a verse list lose their space. Only inside a `chapter:verse` list, so
    /// a decimal comma (`1,5`) and a normal comma between numbers are left alone.
    fn reference_comma(&self, text: &str, o: &mut Vec<Lint>) {
        let b = text.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if !b[i].is_ascii_digit() { i += 1; continue; }
            let mut j = i;
            while j < b.len() && b[j].is_ascii_digit() { j += 1; }
            if b.get(j) != Some(&b':') { i = j; continue; }
            let mut k = j + 1;
            while k < b.len() && b[k].is_ascii_digit() { k += 1; }
            // only inside a list: `8:6,13`
            if b.get(k) != Some(&b',') || !b.get(k + 1).is_some_and(|c| c.is_ascii_digit()) { i = k; continue; }
            while b.get(k) == Some(&b',') && b.get(k + 1).is_some_and(|c| c.is_ascii_digit()) {
                o.push(Lint {
                    start: k,
                    end: k + 1,
                    rule: "FR_REF_COMMA",
                    message: "Espace manquante après la virgule dans une liste de versets.".to_string(),
                    suggestions: vec![", ".to_string()],
                });
                k += 1;
                while k < b.len() && b[k].is_ascii_digit() { k += 1; }
            }
            i = k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_reference_with_its_verse_list() {
        let r = references("Psaumes 63:3, 2.");
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].book, "Psaumes");
        assert_eq!(r[0].verses, vec!["3", "2"]);
    }

    #[test]
    fn parses_an_abbreviated_reference() {
        let r = references("(1 Cor. 10:11)");
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].book, "Cor");
        assert_eq!(r[0].chapter, "10");
        assert_eq!(r[0].verses, vec!["11"]);
    }

    #[test]
    fn plain_digits_are_not_a_reference() {
        assert!(references("il a 3:4 de chances").is_empty());
        assert!(references("en 1910, 12 hommes").is_empty());
    }
}
