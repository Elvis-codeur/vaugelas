//! Python `json.dumps(..., ensure_ascii=False)` output, byte for byte, plus the float formatting the scripts print.
//!
//! * separators `", "` and `": "` (or, with `indent`, `","` + newline and `": "`), keys in insertion order
//!   (serde_json is built with `preserve_order`, so a parsed record keeps its key order like a Python dict);
//! * strings: `\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, other controls as `\u00xx` (lower-case hex), everything
//!   else (including U+2028 and DEL) raw;
//! * floats: `float.__repr__` (shortest round-trip digits; exponent form below 1e-4 and from 1e16 on, `e-05`).
use serde_json::Value;

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

/// Python `repr(float)`.
pub fn float_repr(x: f64) -> String {
    if x.is_nan() { return "NaN".into(); }
    if x.is_infinite() { return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() }; }
    if x == 0.0 { return if x.is_sign_negative() { "-0.0".into() } else { "0.0".into() }; }
    // Rust's `{:e}` gives the shortest round-trip digits, like Python's dtoa mode 0.
    let e = format!("{:e}", x.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let decpt = exp + 1;
    let mut o = String::new();
    if x < 0.0 { o.push('-'); }
    if decpt <= -4 || decpt > 16 {
        o.push_str(&digits[..1]);
        if digits.len() > 1 { o.push('.'); o.push_str(&digits[1..]); }
        let e = decpt - 1;
        o.push_str(&format!("e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs()));
    } else if decpt <= 0 {
        o.push_str("0.");
        for _ in 0..(-decpt) { o.push('0'); }
        o.push_str(&digits);
    } else if (decpt as usize) < digits.len() {
        o.push_str(&digits[..decpt as usize]);
        o.push('.');
        o.push_str(&digits[decpt as usize..]);
    } else {
        o.push_str(&digits);
        for _ in 0..(decpt as usize - digits.len()) { o.push('0'); }
        o.push_str(".0");
    }
    o
}

fn write_number(o: &mut String, n: &serde_json::Number) {
    if let Some(i) = n.as_i64() { o.push_str(&i.to_string()); }
    else if let Some(u) = n.as_u64() { o.push_str(&u.to_string()); }
    else { o.push_str(&float_repr(n.as_f64().unwrap())); }
}

/// `json.dumps(v, ensure_ascii=False)` appended to `o`.
pub fn dumps_into(o: &mut String, v: &Value) {
    match v {
        Value::Null => o.push_str("null"),
        Value::Bool(b) => o.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => write_number(o, n),
        Value::String(s) => write_str(o, s),
        Value::Array(a) => {
            o.push('[');
            for (k, x) in a.iter().enumerate() {
                if k > 0 { o.push_str(", "); }
                dumps_into(o, x);
            }
            o.push(']');
        }
        Value::Object(m) => {
            o.push('{');
            for (k, (key, x)) in m.iter().enumerate() {
                if k > 0 { o.push_str(", "); }
                write_str(o, key);
                o.push_str(": ");
                dumps_into(o, x);
            }
            o.push('}');
        }
    }
}

pub fn dumps(v: &Value) -> String {
    let mut o = String::new();
    dumps_into(&mut o, v);
    o
}

/// `json.dump(v, f, ensure_ascii=False, indent=n)`.
pub fn dumps_indent(v: &Value, indent: usize) -> String {
    fn rec(o: &mut String, v: &Value, ind: usize, level: usize) {
        let nl = |o: &mut String, l: usize| { o.push('\n'); for _ in 0..ind * l { o.push(' '); } };
        match v {
            Value::Array(a) if !a.is_empty() => {
                o.push('[');
                for (k, x) in a.iter().enumerate() {
                    if k > 0 { o.push(','); }
                    nl(o, level + 1);
                    rec(o, x, ind, level + 1);
                }
                nl(o, level);
                o.push(']');
            }
            Value::Object(m) if !m.is_empty() => {
                o.push('{');
                for (k, (key, x)) in m.iter().enumerate() {
                    if k > 0 { o.push(','); }
                    nl(o, level + 1);
                    write_str(o, key);
                    o.push_str(": ");
                    rec(o, x, ind, level + 1);
                }
                nl(o, level);
                o.push('}');
            }
            _ => dumps_into(o, v),
        }
    }
    let mut o = String::new();
    rec(&mut o, v, indent, 0);
    o
}

/// Python truthiness of a JSON value (`if r.get("frc"):`).
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_none_or(|x| x != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(m)) => !m.is_empty(),
    }
}

/// Python `round(x, nd)` for floats: correctly rounded (half-even on the exact binary value), then re-parsed.
pub fn py_round(x: f64, nd: usize) -> f64 {
    format!("{:.*}", nd, x).parse().unwrap()
}

/// Python `f"{x:.{nd}f}"`.
pub fn fmt_f(x: f64, nd: usize) -> String {
    format!("{:.*}", nd, x)
}

/// Python `str(v)` for the scalar JSON values the scripts format into text (`f"{f}:{s}"`).
pub fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True".into() } else { "False".into() },
        Value::Number(n) => { let mut o = String::new(); write_number(&mut o, n); o }
        other => dumps(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn float_repr_matches_python() {
        for (x, s) in [(1.0, "1.0"), (0.5, "0.5"), (1e16, "1e+16"), (1e15, "1000000000000000.0"), (0.0001, "0.0001"),
                       (0.00001, "1e-05"), (-2.5e-7, "-2.5e-07"), (123.456, "123.456"), (1.5e300, "1.5e+300"),
                       (0.1 + 0.2, "0.30000000000000004")] {
            assert_eq!(float_repr(x), s);
        }
    }
    #[test]
    fn strings_escape_like_python() {
        let mut o = String::new();
        write_str(&mut o, "a\"b\\c\n\u{1}\u{7f}é\u{2028}");
        assert_eq!(o, "\"a\\\"b\\\\c\\n\\u0001\u{7f}é\u{2028}\"");
    }
    #[test]
    fn round_half_even_on_exact_value() {
        assert_eq!(py_round(1.0 / 64.0, 5), 0.01562);
        assert_eq!(py_round(0.375, 2), 0.38);
        assert_eq!(fmt_f(0.125, 2), "0.12");
    }
}
