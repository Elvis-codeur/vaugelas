//! Python `str` and `re` character semantics, exactly as the reference interpreter has them.
//!
//! The reference scripts run on CPython 3.12 (Unicode 15.0); Rust's `char` methods follow a newer Unicode and
//! different definitions (`char::is_alphabetic` is the Alphabetic property, Python's `isalpha` is the L* categories;
//! regex `\w` includes combining marks, Python's does not; Python's `\s` includes U+001C..U+001F). So nothing here
//! uses Rust's own tables: every predicate and case mapping comes from `pyunicode_data.rs`, generated from the
//! interpreter itself by `prep/tools/gen_pyunicode.py`.
use crate::text::pyunicode_data::{IGNORECASE_FIXES, LOWER_MAP, RANGES, SIMPLE_LOWER_MAP, UPPER_MAP};
use std::sync::OnceLock;

pub const W: u16 = 1;
pub const S: u16 = 2;
pub const ALPHA: u16 = 4;
pub const DIGIT: u16 = 8;
pub const DEC: u16 = 16;
pub const UPPER: u16 = 32;
pub const LOWER: u16 = 64;
pub const TITLE: u16 = 128;
pub const SRECASED: u16 = 256;
pub const SIG_IGN: u16 = 512;
pub const SIG_CASED: u16 = 1024;
pub const LMAP: u16 = 2048;
pub const UMAP: u16 = 4096;
pub const SLMAP: u16 = 8192;

fn bmp() -> &'static [u16] {
    static T: OnceLock<Vec<u16>> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = vec![0u16; 0x10000];
        for (k, &(start, f)) in RANGES.iter().enumerate() {
            if start >= 0x10000 { break; }
            let end = RANGES.get(k + 1).map_or(0x10000, |r| r.0.min(0x10000));
            for cp in start..end { t[cp as usize] = f; }
        }
        t
    })
}

/// Python's flags for one character.
#[inline]
pub fn flags(c: char) -> u16 {
    let cp = c as u32;
    if cp < 0x10000 { return bmp()[cp as usize]; }
    let k = RANGES.partition_point(|r| r.0 <= cp);
    RANGES[k - 1].1
}

#[inline] pub fn is_w(c: char) -> bool { flags(c) & W != 0 }
#[inline] pub fn is_space(c: char) -> bool { flags(c) & S != 0 }
#[inline] pub fn is_alpha(c: char) -> bool { flags(c) & ALPHA != 0 }
#[inline] pub fn is_digit(c: char) -> bool { flags(c) & DIGIT != 0 }
#[inline] pub fn is_decimal(c: char) -> bool { flags(c) & DEC != 0 }
/// `c.isupper()` for one character (Py_UNICODE_ISUPPER).
#[inline] pub fn is_upper(c: char) -> bool { flags(c) & UPPER != 0 }

/// `s.isalpha()`
pub fn str_isalpha(s: &str) -> bool { !s.is_empty() && s.chars().all(is_alpha) }
/// `s.isdigit()`
pub fn str_isdigit(s: &str) -> bool { !s.is_empty() && s.chars().all(is_digit) }
/// `s.isupper()`: no lower-case or title-case character, and at least one upper-case one.
pub fn str_isupper(s: &str) -> bool {
    let mut cased = false;
    for c in s.chars() {
        let f = flags(c);
        if f & (LOWER | TITLE) != 0 { return false; }
        if f & UPPER != 0 { cased = true; }
    }
    cased
}
/// `re.search(r"\w", s) is not None`
pub fn has_w(s: &str) -> bool { s.chars().any(is_w) }
/// `s.strip()`
pub fn strip(s: &str) -> &str { s.trim_matches(is_space) }

fn map_lookup<'a>(m: &'a [(u32, &'a str)], c: char) -> &'a str {
    let cp = c as u32;
    let k = m.binary_search_by_key(&cp, |e| e.0).expect("mapping flag without a mapping");
    m[k].1
}

/// `s.lower()` (full case mapping, with the Final_Sigma rule of `str.lower`).
pub fn lower(s: &str) -> String {
    if s.is_ascii() { return s.to_ascii_lowercase(); }
    let mut o = String::with_capacity(s.len());
    let has_sigma = s.contains('\u{3a3}');
    let chars: Vec<char> = if has_sigma { s.chars().collect() } else { Vec::new() };
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii() { o.push(c.to_ascii_lowercase()); continue; }
        if c == '\u{3a3}' { o.push(final_sigma(&chars, i)); continue; }
        if flags(c) & LMAP != 0 { o.push_str(map_lookup(&LOWER_MAP, c)); } else { o.push(c); }
    }
    o
}

/// CPython `handle_capital_sigma`.
fn final_sigma(chars: &[char], i: usize) -> char {
    let mut j = i as isize - 1;
    let mut c = '\0';
    while j >= 0 {
        c = chars[j as usize];
        if flags(c) & SIG_IGN == 0 { break; }
        j -= 1;
    }
    let mut fin = j >= 0 && flags(c) & SIG_CASED != 0;
    if fin && i + 1 < chars.len() {
        let mut j = i + 1;
        while j < chars.len() {
            c = chars[j];
            if flags(c) & SIG_IGN == 0 { break; }
            j += 1;
        }
        fin = j == chars.len() || flags(c) & SIG_CASED == 0;
    }
    if fin { '\u{3c2}' } else { '\u{3c3}' }
}

/// `s.upper()` (full case mapping).
pub fn upper(s: &str) -> String {
    if s.is_ascii() { return s.to_ascii_uppercase(); }
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() { o.push(c.to_ascii_uppercase()); continue; }
        if flags(c) & UMAP != 0 { o.push_str(map_lookup(&UPPER_MAP, c)); } else { o.push(c); }
    }
    o
}

/// `_sre.unicode_tolower` (simple lower-case mapping used by `re.IGNORECASE`).
pub fn simple_lower(c: char) -> u32 {
    let cp = c as u32;
    if flags(c) & SLMAP == 0 { return cp; }
    let k = SIMPLE_LOWER_MAP.binary_search_by_key(&cp, |e| e.0).expect("simple lower");
    SIMPLE_LOWER_MAP[k].1
}

/// Does text character `t` match the pattern literal `p` under `re.IGNORECASE` (Unicode str pattern)?
/// re._compiler: an uncased literal matches itself; a cased one compiles to `lower(p)` plus its `_EXTRA_CASES`, and
/// the engine compares `unicode_tolower(t)` against that set.
pub fn ci_eq(t: char, p: char) -> bool {
    if flags(p) & SRECASED == 0 { return t == p; }
    let lp = simple_lower(p);
    let lt = simple_lower(t);
    if lt == lp { return true; }
    match IGNORECASE_FIXES.binary_search_by_key(&lp, |e| e.0) {
        Ok(k) => IGNORECASE_FIXES[k].1.contains(&lt),
        Err(_) => false,
    }
}

/// Python `str.splitlines`-free line handling of text-mode files: `open(..., encoding=...)` with universal newlines
/// turns `\r\n` and lone `\r` into `\n` before the program sees the text.
pub fn universal_newlines(s: String) -> String {
    if !s.contains('\r') { return s; }
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Python `repr(str)` (used where the scripts `print` a dict).
pub fn repr_str(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut o = String::with_capacity(s.len() + 2);
    o.push(q);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c == q => { o.push('\\'); o.push(c); }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => o.push_str(&format!("\\x{:02x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push(q);
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_semantics_differ_from_rust_where_expected() {
        assert!(is_space('\u{1c}'));          // Python isspace, not Rust White_Space
        assert!(!is_w('\u{301}'));            // combining acute: not \w in Python
        assert!(is_w('²') && is_digit('²') && !is_decimal('²'));
        assert_eq!(lower("İ"), "i\u{307}");
        assert_eq!(lower("ΑΣ"), "ας");
        assert_eq!(lower("ΑΣΑ"), "ασα");
        assert_eq!(upper("ß"), "SS");
        assert!(ci_eq('ſ', 's') && ci_eq('İ', 'i') && ci_eq('À', 'à') && !ci_eq('a', 'à'));
        assert!(str_isupper("ÉTÉ") && !str_isupper("ÉTé") && !str_isupper("12"));
    }
}
