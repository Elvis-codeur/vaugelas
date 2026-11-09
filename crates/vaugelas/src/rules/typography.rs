//! French typography, checked on the raw text (no lexicon needed).
//!
//! Default rules report unambiguous errors: no space at all before `; : ! ?` or inside `« »`,
//! a missing space after `; : ! ?` or after a sentence-final period, and double spaces.
//! Strict rules (opt-in) report conventions of typeset French that plain text often ignores:
//! a normal space where a non-breaking one is expected, `...` instead of `…`, straight apostrophes.
use crate::rules::checker::Lint;
use crate::rules::tokenizer::skip_ranges;

const NBSP: char = '\u{00A0}';
const NNBSP: char = '\u{202F}';

fn lint(start: usize, end: usize, rule: &'static str, message: &str, sugg: String) -> Lint {
    Lint { start, end, rule, message: message.to_string(), suggestions: vec![sugg] }
}

pub fn check(text: &str, strict: bool) -> Vec<Lint> {
    let skips = skip_ranges(text);
    let skipped = |b: usize| skips.iter().any(|&(s, e)| b >= s && b < e);
    let ch: Vec<(usize, char)> = text.char_indices().collect();
    let n = ch.len();
    let end_of = |k: usize| if k + 1 < n { ch[k + 1].0 } else { text.len() };
    let mut out = Vec::new();

    for k in 0..n {
        let (b, c) = ch[k];
        if skipped(b) { continue; }
        let prev = if k > 0 { Some(ch[k - 1].1) } else { None };
        let next = if k + 1 < n { Some(ch[k + 1].1) } else { None };
        match c {
            ';' | ':' | '!' | '?' => {
                let space = if c == ':' { NBSP } else { NNBSP };
                // 10:30, emoticons, "?!" sequences
                let time = c == ':' && prev.is_some_and(|p| p.is_ascii_digit()) && next.is_some_and(|x| x.is_ascii_digit());
                let smiley = matches!(c, ':' | ';') && matches!(next, Some(')') | Some('(') | Some('-') | Some('D') | Some('P'));
                if time || smiley { continue; }
                match prev {
                    Some(p) if !p.is_whitespace() && !matches!(p, '!' | '?' | ';' | ':' | '(' | '[' | '«' | '"' | '“') => {
                        out.push(lint(b, end_of(k), "FR_TYPO_SPACE_BEFORE", "Espace (insécable) requise avant cette ponctuation.", format!("{space}{c}")));
                    }
                    Some(' ') if strict => {
                        let sb = ch[k - 1].0;
                        out.push(lint(sb, b, "FR_TYPO_NBSP", "Espace insécable attendue avant cette ponctuation.", space.to_string()));
                    }
                    _ => {}
                }
                if let Some(x) = next {
                    if x.is_alphabetic() && !(c == ':' && prev.is_some_and(|p| p.is_ascii_digit())) {
                        out.push(lint(b, end_of(k), "FR_SPACE_AFTER", "Espace manquante après cette ponctuation.", format!("{c} ")));
                    }
                }
            }
            '«' => match next {
                Some(x) if x.is_alphanumeric() => out.push(lint(b, end_of(k), "FR_TYPO_GUILLEMET", "Espace insécable requise après « .", format!("«{NBSP}"))),
                Some(' ') if strict => out.push(lint(ch[k + 1].0, end_of(k + 1), "FR_TYPO_NBSP", "Espace insécable attendue après « .", NBSP.to_string())),
                _ => {}
            },
            '»' => match prev {
                Some(p) if p.is_alphanumeric() || matches!(p, '.' | ',' | '!' | '?' | '…') => out.push(lint(b, end_of(k), "FR_TYPO_GUILLEMET", "Espace insécable requise avant » .", format!("{NBSP}»"))),
                Some(' ') if strict => out.push(lint(ch[k - 1].0, b, "FR_TYPO_NBSP", "Espace insécable attendue avant » .", NBSP.to_string())),
                _ => {}
            },
            '.' => {
                // "fin.Le" (but not "M.Dupont", "U.S.A", "3.5", "etc...")
                if let (Some(p), Some(x)) = (prev, next) {
                    if p.is_lowercase() && x.is_uppercase() && k >= 2 && ch[k - 2].1.is_alphabetic() {
                        out.push(lint(b, end_of(k), "FR_SPACE_AFTER", "Espace manquante après le point.", ". ".to_string()));
                    }
                }
                if strict && next == Some('.') && k + 2 < n && ch[k + 2].1 == '.' && prev != Some('.') {
                    out.push(lint(b, end_of(k + 2), "FR_TYPO_ELLIPSIS", "Utilisez le caractère « … ».", "…".to_string()));
                }
            }
            ' ' => {
                // start of a run of 2+ spaces between two visible characters
                if prev.is_some_and(|p| !p.is_whitespace()) && next == Some(' ') {
                    let mut j = k;
                    while j < n && ch[j].1 == ' ' { j += 1; }
                    if j < n && !ch[j].1.is_whitespace() {
                        out.push(lint(b, ch[j].0, "FR_DOUBLE_SPACE", "Espaces multiples.", " ".to_string()));
                    }
                }
            }
            '\'' if strict
                && prev.is_some_and(|p| p.is_alphabetic()) && next.is_some_and(|x| x.is_alphabetic()) => {
                    out.push(lint(b, end_of(k), "FR_TYPO_APOS", "Apostrophe typographique « ’ ».", "’".to_string()));
                }
            _ => {}
        }
    }
    out
}
