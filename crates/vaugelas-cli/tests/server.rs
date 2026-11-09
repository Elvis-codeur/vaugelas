//! Smoke test of `vaugelas serve` through its LanguageTool-compatible API (needs the data files; skipped without them).
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

fn data() -> Option<PathBuf> {
    let d = std::env::var("VAUGELAS_DATA").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    let complete = d.join("lex/words.fst").is_file() && d.join("model.frcm").is_file() && d.join("lexicon.tsv").is_file() && d.join("resources/ngram").is_dir();
    if complete { Some(d) } else { eprintln!("skipping: no data at {}", d.display()); None }
}

struct Kill(Child);
impl Drop for Kill { fn drop(&mut self) { self.0.kill().ok(); self.0.wait().ok(); } }

#[test]
fn languagetool_api_round_trip() {
    let Some(d) = data() else { return };
    let port = 18000 + (std::process::id() % 1000) as u16;
    let _srv = Kill(Command::new(env!("CARGO_BIN_EXE_vaugelas")).args(["serve", "--port", &port.to_string(), "--threads", "2", "--data"]).arg(&d).spawn().unwrap());
    let base = format!("http://127.0.0.1:{port}");
    let t0 = Instant::now();
    while ureq::get(&format!("{base}/health")).call().is_err() {
        assert!(t0.elapsed() < Duration::from_secs(120), "server did not start");
        std::thread::sleep(Duration::from_millis(250));
    }
    let langs: Value = ureq::get(&format!("{base}/v2/languages")).call().unwrap().into_string().map(|s| serde_json::from_str(&s).unwrap()).unwrap();
    assert!(langs.as_array().unwrap().iter().any(|l| l["longCode"] == "fr-FR"));

    // "😀 " before the text: 2 UTF-16 units, so offsets must be UTF-16 (LanguageTool), not chars or bytes
    let text = "😀 Les élections ont été annulés par le juge.";
    let r: Value = ureq::post(&format!("{base}/v2/check")).send_form(&[("language", "fr-FR"), ("text", text)]).unwrap().into_string().map(|s| serde_json::from_str(&s).unwrap()).unwrap();
    let m = r["matches"].as_array().unwrap();
    let hit = m.iter().find(|x| x["replacements"].as_array().unwrap().iter().any(|v| v["value"] == "annulées")).expect("annulés flagged");
    let units: Vec<u16> = text.encode_utf16().collect();
    let (o, l) = (hit["offset"].as_u64().unwrap() as usize, hit["length"].as_u64().unwrap() as usize);
    assert_eq!(String::from_utf16(&units[o..o + l]).unwrap(), "annulés");
    assert!(hit["rule"]["id"].is_string() && hit["rule"]["category"]["id"].is_string() && hit["message"].is_string());

    // disabledRules removes a rule's matches
    let id = hit["rule"]["id"].as_str().unwrap().to_string();
    let r2: Value = ureq::post(&format!("{base}/v2/check")).send_form(&[("language", "fr"), ("text", text), ("disabledRules", &id)]).unwrap().into_string().map(|s| serde_json::from_str(&s).unwrap()).unwrap();
    assert!(r2["matches"].as_array().unwrap().iter().all(|x| x["rule"]["id"] != id.as_str()));

    // a correct sentence: no match; another language: 400
    let ok: Value = ureq::post(&format!("{base}/v2/check")).send_form(&[("language", "fr"), ("text", "Ceci est une phrase correcte.")]).unwrap().into_string().map(|s| serde_json::from_str(&s).unwrap()).unwrap();
    assert_eq!(ok["matches"].as_array().unwrap().len(), 0, "{ok}");
    match ureq::post(&format!("{base}/v2/check")).send_form(&[("language", "de-DE"), ("text", "Hallo")]) {
        Err(ureq::Error::Status(400, _)) => {}
        other => panic!("expected 400, got {other:?}"),
    }
}
