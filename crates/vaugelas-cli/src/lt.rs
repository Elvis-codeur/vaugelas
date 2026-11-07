//! LanguageTool-compatible HTTP API (the subset clients use): `POST /v2/check`, `GET /v2/languages`.
//! Offsets and lengths are in UTF-16 code units, as in LanguageTool (Java strings), and refer to the submitted
//! document: `text`, or for annotated `data` the concatenation of every `text` and `markup` piece.
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use vaugelas::report::{rule_description, Issue, Settings, Vaugelas};

pub const MAX_CHARS: usize = 200_000;

/// The submitted document: what is checked (`plain`) and how plain byte offsets map back to the document.
pub struct Doc {
    pub plain: String,
    /// document text (plain text, or text + markup for annotated data)
    pub original: String,
    /// (plain start, plain end, original start, original end, is text): pieces in order
    segs: Vec<(usize, usize, usize, usize, bool)>,
}

impl Doc {
    pub fn from_text(t: &str) -> Doc { Doc { plain: t.to_string(), original: t.to_string(), segs: vec![(0, t.len(), 0, t.len(), true)] } }

    /// LanguageTool `data`: {"annotation": [{"text": "..."}, {"markup": "<b>", "interpretAs": "..."}]}
    pub fn from_annotation(data: &str) -> Result<Doc, String> {
        let v: Value = serde_json::from_str(data).map_err(|e| format!("invalid 'data' JSON: {e}"))?;
        let ann = v["annotation"].as_array().ok_or("'data' needs an 'annotation' array")?;
        let mut d = Doc { plain: String::new(), original: String::new(), segs: Vec::new() };
        for a in ann {
            let (p0, o0) = (d.plain.len(), d.original.len());
            if let Some(t) = a.get("text").and_then(Value::as_str) {
                d.plain.push_str(t); d.original.push_str(t);
                d.segs.push((p0, d.plain.len(), o0, d.original.len(), true));
            } else if let Some(m) = a.get("markup").and_then(Value::as_str) {
                d.plain.push_str(a.get("interpretAs").and_then(Value::as_str).unwrap_or(""));
                d.original.push_str(m);
                d.segs.push((p0, d.plain.len(), o0, d.original.len(), false));
            } else { return Err("each annotation needs 'text' or 'markup'".into()); }
        }
        Ok(d)
    }

    /// plain byte offset -> original byte offset (`end`: the offset closes a span)
    fn map(&self, p: usize, end: bool) -> usize {
        for &(ps, pe, os, oe, text) in &self.segs {
            let inside = if end { p > ps && p <= pe } else { p >= ps && p < pe };
            if inside { return if text { os + (p - ps) } else if end { oe } else { os }; }
        }
        if end { self.original.len() } else { self.segs.iter().find(|s| s.0 >= p).map_or(self.original.len(), |s| s.2) }
    }
}

/// byte offset -> UTF-16 offset table of `s` (valid at char boundaries)
fn utf16_table(s: &str) -> Vec<u32> {
    let mut t = vec![0u32; s.len() + 1];
    let mut u = 0u32;
    for (b, c) in s.char_indices() { t[b] = u; u += c.len_utf16() as u32; }
    t[s.len()] = u;
    t
}

fn context(plain: &str, start: usize, end: usize) -> Value {
    const W: usize = 40;
    let mut cs = start; for _ in 0..W { match plain[..cs].chars().next_back() { Some(c) if c != '\n' => cs -= c.len_utf8(), _ => break } }
    let mut ce = end; for _ in 0..W { match plain[ce..].chars().next() { Some(c) if c != '\n' => ce += c.len_utf8(), _ => break } }
    let u16len = |s: &str| s.encode_utf16().count();
    json!({"text": &plain[cs..ce], "offset": u16len(&plain[cs..start]), "length": u16len(&plain[start..end])})
}

fn sentence_of(plain: &str, start: usize) -> &str {
    let a = plain[..start].rfind('\n').map_or(0, |i| i + 1);
    let b = plain[start..].find('\n').map_or(plain.len(), |i| start + i);
    &plain[a..b]
}

pub fn match_json(doc: &Doc, t16: &[u32], i: &Issue) -> Value {
    let (os, oe) = (doc.map(i.start, false), doc.map(i.end, true).max(doc.map(i.start, false)));
    let mut m = json!({
        "message": i.message,
        "shortMessage": "",
        "replacements": i.replacements.iter().map(|r| json!({"value": r})).collect::<Vec<_>>(),
        "offset": t16[os],
        "length": t16[oe] - t16[os],
        "context": context(&doc.plain, i.start, i.end),
        "sentence": sentence_of(&doc.plain, i.start),
        "type": {"typeName": "Other"},
        "rule": {
            "id": i.rule,
            "description": rule_description(&i.rule),
            "issueType": i.category.issue_type(),
            "category": {"id": i.category.id(), "name": i.category.name()},
        },
        "ignoreForIncompleteSentence": false,
        "contextForSureMatch": 0,
    });
    m["vaugelas"] = json!({"source": i.source.as_str(), "score": i.score});
    m
}

pub fn languages() -> Value {
    json!([
        {"name": "French", "code": "fr", "longCode": "fr"},
        {"name": "French (France)", "code": "fr", "longCode": "fr-FR"},
        {"name": "French (Belgium)", "code": "fr", "longCode": "fr-BE"},
        {"name": "French (Canada)", "code": "fr", "longCode": "fr-CA"},
        {"name": "French (Switzerland)", "code": "fr", "longCode": "fr-CH"},
    ])
}

fn software() -> Value {
    json!({"name": "Vaugelas", "version": env!("CARGO_PKG_VERSION"), "buildDate": "", "apiVersion": 1, "premium": false, "status": ""})
}

fn split_list(v: Option<&String>) -> HashSet<String> {
    v.map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}

/// Handle `/v2/check` parameters; Err((status, message)).
pub fn check(vg: &Vaugelas, cands: &vaugelas::model::cands::Cands, params: &[(String, String)], base: &Settings) -> Result<Value, (u16, String)> {
    let get = |k: &str| params.iter().find(|(a, _)| a == k).map(|(_, v)| v);
    let lang = get("language").map(String::as_str).unwrap_or("fr");
    let code = if lang == "auto" || lang.eq_ignore_ascii_case("fr") || lang.to_ascii_lowercase().starts_with("fr-") { lang } else {
        return Err((400, format!("Error: '{lang}' is not a language code known to Vaugelas (French only: fr, fr-FR, fr-BE, fr-CA, fr-CH, auto).")));
    };
    let doc = match (get("text"), get("data")) {
        (Some(t), _) => Doc::from_text(t),
        (None, Some(d)) => Doc::from_annotation(d).map_err(|e| (400, format!("Error: {e}")))?,
        (None, None) => return Err((400, "Error: Missing 'text' or 'data' parameter".into())),
    };
    if doc.plain.chars().count() > MAX_CHARS { return Err((413, format!("Error: text too long (maximum {MAX_CHARS} characters)"))); }
    let disabled = split_list(get("disabledRules"));
    let enabled = split_list(get("enabledRules"));
    let enabled_only = get("enabledOnly").map_or(false, |v| v == "true");
    let dis_cat = split_list(get("disabledCategories"));
    let issues = vg.check(cands, &doc.plain, base);
    let t16 = utf16_table(&doc.original);
    let matches: Vec<Value> = issues.iter()
        .filter(|i| !disabled.contains(&i.rule) && !dis_cat.contains(i.category.id()) && (!enabled_only || enabled.contains(&i.rule)))
        .map(|i| match_json(&doc, &t16, i)).collect();
    let lname = |c: &str| match c.to_ascii_lowercase().as_str() { "fr-fr" => "French (France)", "fr-be" => "French (Belgium)", "fr-ca" => "French (Canada)", "fr-ch" => "French (Switzerland)", _ => "French" };
    let shown = if code == "auto" { "fr" } else { code };
    Ok(json!({
        "software": software(),
        "warnings": {"incompleteResults": false},
        "language": {"name": lname(shown), "code": shown,
                     "detectedLanguage": {"name": "French", "code": "fr", "confidence": 1.0}},
        "matches": matches,
    }))
}

/// Run the HTTP server; blocks forever.
pub fn serve(vg: Arc<Vaugelas>, host: &str, port: u16, threads: usize, settings: Settings) -> Result<(), String> {
    let server = Arc::new(tiny_http::Server::http((host, port)).map_err(|e| format!("cannot listen on {host}:{port}: {e}"))?);
    eprintln!("Vaugelas listens on http://{host}:{port}/v2/  ({threads} threads; LanguageTool-compatible: POST /v2/check, GET /v2/languages)");
    let hs: Vec<_> = (0..threads).map(|_| {
        let (server, vg) = (server.clone(), vg.clone());
        std::thread::spawn(move || {
            let cands = vg.cands();
            loop {
                let Ok(rq) = server.recv() else { continue };
                handle(&vg, &cands, rq, &settings);
            }
        })
    }).collect();
    for h in hs { h.join().ok(); }
    Ok(())
}

fn header(k: &str, v: &str) -> tiny_http::Header { tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap() }

fn respond(rq: tiny_http::Request, status: u16, ctype: &str, body: String) {
    let r = tiny_http::Response::from_string(body).with_status_code(status)
        .with_header(header("Content-Type", ctype))
        .with_header(header("Access-Control-Allow-Origin", "*"))
        .with_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
        .with_header(header("Access-Control-Allow-Headers", "Content-Type, Accept"));
    rq.respond(r).ok();
}

fn handle(vg: &Vaugelas, cands: &vaugelas::model::cands::Cands, mut rq: tiny_http::Request, settings: &Settings) {
    let url = rq.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let path = path.trim_end_matches('/').to_string();
    let method = rq.method().clone();
    use tiny_http::Method::*;
    match (&method, path.as_str()) {
        (Options, _) => respond(rq, 204, "text/plain", String::new()),
        (Get, "/v2/languages") => respond(rq, 200, "application/json", languages().to_string()),
        (Get, "") | (Get, "/v2") | (Get, "/health") =>
            respond(rq, 200, "text/plain; charset=utf-8", format!("Vaugelas {} - LanguageTool-compatible API at /v2/check\n", env!("CARGO_PKG_VERSION"))),
        (Get, "/v2/check") | (Post, "/v2/check") => {
            let mut params: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes()).into_owned().collect();
            if method == Post {
                let mut body = Vec::new();
                use std::io::Read;
                if rq.as_reader().take(8 * MAX_CHARS as u64 + 65536).read_to_end(&mut body).is_err() {
                    return respond(rq, 400, "text/plain", "Error: cannot read the request body".into());
                }
                let json_body = rq.headers().iter().any(|h| h.field.equiv("Content-Type") && h.value.as_str().starts_with("application/json"));
                if json_body {
                    if let Ok(Value::Object(o)) = serde_json::from_slice::<Value>(&body) {
                        for (k, v) in o { params.push((k, match v { Value::String(s) => s, v => v.to_string() })); }
                    }
                } else {
                    params.extend(form_urlencoded::parse(&body).into_owned());
                }
            }
            match check(vg, cands, &params, settings) {
                Ok(v) => respond(rq, 200, "application/json", v.to_string()),
                Err((st, msg)) => respond(rq, st, "text/plain; charset=utf-8", msg),
            }
        }
        _ => respond(rq, 404, "text/plain", format!("Error: unknown path {path}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_offsets() {
        let t = utf16_table("é😀a");
        assert_eq!((t[0], t[2], t[6], t[7]), (0, 1, 3, 4));
    }
    #[test]
    fn annotation_maps_back_to_markup_positions() {
        let d = Doc::from_annotation(r#"{"annotation":[{"text":"Il "},{"markup":"<b>"},{"text":"mange"},{"markup":"</b>"},{"markup":"<p>","interpretAs":"\n\n"},{"text":"Fin."}]}"#).unwrap();
        assert_eq!(d.plain, "Il mange\n\nFin.");
        assert_eq!(d.original, "Il <b>mange</b><p>Fin.");
        let (s, e) = (3, 8); // "mange" in plain
        assert_eq!(&d.original[d.map(s, false)..d.map(e, true)], "mange");
        assert_eq!(&d.original[d.map(10, false)..d.map(14, true)], "Fin.");
    }
    #[test]
    fn context_window() {
        let c = context("abc def ghi", 4, 7);
        assert_eq!(c["text"], "abc def ghi"); assert_eq!(c["offset"], 4); assert_eq!(c["length"], 3);
    }
}
