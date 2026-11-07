//! Python `json.dumps(..., ensure_ascii=False)` string literals, byte for byte (the lexicon keeps every reading key
//! in this form): `\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, other controls as `\u00xx` (lower-case hex), everything
//! else (including U+2028 and DEL) raw.

/// Append `s` as a JSON string literal (with quotes).
pub fn write_str(o: &mut String, s: &str) {
    o.push('"');
    let b = s.as_bytes();
    let mut start = 0;
    for (i, &c) in b.iter().enumerate() {
        let esc: &str = match c {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            0x08 => "\\b",
            0x0c => "\\f",
            c if c < 0x20 => "",
            _ => continue,
        };
        o.push_str(&s[start..i]);
        if esc.is_empty() { o.push_str(&format!("\\u{:04x}", c)); } else { o.push_str(esc); }
        start = i + 1;
    }
    o.push_str(&s[start..]);
    o.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strings_escape_like_python() {
        let mut o = String::new();
        write_str(&mut o, "a\"b\\c\n\u{1}\u{7f}é\u{2028}");
        assert_eq!(o, "\"a\\\"b\\\\c\\n\\u0001\u{7f}é\u{2028}\"");
    }
}
