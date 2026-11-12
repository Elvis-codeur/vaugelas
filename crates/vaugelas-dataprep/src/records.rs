//! The record side of the data preparation: `ebm_data.py` / `ebm_data2.py`'s pair and text paths, on top of the
//! ported candidate generator (`vaugelas::model::cands::Gen`) and record builder (`Builder`).
//!
//! Output records are the Python dicts `{"feats", "words", "err", "src", "cands"}` (plus `kind`/`side`/`item` for a
//! pair), serialised by `vaugelas::text::pyjson` exactly like `json.dumps(..., ensure_ascii=False)`. Work is
//! independent per input line, so it runs on the rayon pool and keeps the input order.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;
use vaugelas::model::cands::{CandSet, Gen};
use vaugelas::model::profile::Profile;
use vaugelas::model::record::Builder;
use vaugelas::model::pipeline::{LexCache, Tables};
use vaugelas::rules::Checker;
use vaugelas::text::characterize::tokenize;

fn cands_value(cands: Vec<CandSet>) -> Value {
    Value::Array(
        cands
            .into_iter()
            .map(|(i, g, cs)| {
                let cs = cs
                    .into_iter()
                    .map(|(f, rd)| Value::Array(vec![Value::String(f), Value::Array(rd.into_iter().map(Value::String).collect())]))
                    .collect();
                Value::Array(vec![json!(i), json!(g), Value::Array(cs)])
            })
            .collect(),
    )
}

fn record_value(feats: Vec<Vec<String>>, words: &[String], err: i64, src: &str, cands: Vec<CandSet>, extra: &[(&str, Value)]) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("feats".into(), Value::Array(feats.into_iter().map(|v| Value::Array(v.into_iter().map(Value::String).collect())).collect()));
    m.insert("words".into(), Value::Array(words.iter().map(|w| Value::String(w.clone())).collect()));
    m.insert("err".into(), json!(err));
    m.insert("src".into(), json!(src));
    m.insert("cands".into(), cands_value(cands));
    for (k, v) in extra {
        m.insert((*k).into(), v.clone());
    }
    Value::Object(m)
}

/// The tables the record path needs (no model weights: only its header's rule filter).
struct Ctx {
    tables: Tables,
    ck: Checker,
    ignore: Vec<String>,
}

fn load(data: &Path) -> Result<Ctx, String> {
    let head = vaugelas::model::read_header(&data.join("model.frcm").to_string_lossy())?;
    let profile = Profile::from_header(&head)?;
    let t0 = Instant::now();
    let (tables, ck) = rayon::join(
        || Tables::load(&data.join("lexicon.tsv"), &data.join("resources"), &LexCache::NextToTsv),
        || Checker::open(&data.join("lex")).map_err(|e| e.to_string()),
    );
    let tables = tables?;
    let ck = ck?;
    eprintln!("data loaded in {:.1} s ({})", t0.elapsed().as_secs_f64(), profile.name);
    Ok(Ctx { tables, ck, ignore: profile.frc_ignore })
}

pub struct Stats {
    pub records: usize,
    pub secs: f64,
}

/// Write the body as plain text, or gzip when the path ends in `.gz` (like `featdata.wopen`).
pub fn write_out(path: &Path, body: &str) -> Result<(), String> {
    use std::io::Write;
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut w: Box<dyn Write> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::write::GzEncoder::new(f, flate2::Compression::default()))
    } else {
        Box::new(std::io::BufWriter::new(f))
    };
    w.write_all(body.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    w.flush().map_err(|e| format!("{}: {e}", path.display()))
}

/// `ebm_data2.py pairs OUT ITEMS.jsonl`: for every item its good record, and for a valid pair its bad record with the
/// gold form at the changed position. `item` is the input line index (Python's `k`).
pub fn pairs(items: &Path, out: &Path, data: &Path) -> Result<Stats, String> {
    let ctx = load(data)?;
    let gen = Gen::new(&ctx.tables.lex, &ctx.tables.nat, &ctx.tables.res);
    let b = Builder::new(&gen, &ctx.ck, &ctx.ignore);
    let text = std::fs::read_to_string(items).map_err(|e| format!("{}: {e}", items.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let t0 = Instant::now();
    let bodies: Vec<String> = lines
        .par_iter()
        .enumerate()
        .map(|(k, line)| {
            let it: Value = serde_json::from_str(line).map_err(|e| format!("{}: {e}", items.display()))?;
            let id = it["id"].as_str().ok_or("item without id")?.to_string();
            let kind = it["kind"].as_str().unwrap_or("pair");
            let good = it["good"].as_str().ok_or("item without good")?;
            let mut out = String::new();
            let ws = tokenize(b.gen.lex, good);
            let (feats, cands) = b.record_parts(good, &ws, None)?;
            out.push_str(&vaugelas::text::pyjson::dumps(&record_value(feats, &ws, -1, &id, cands, &[("kind", json!(kind)), ("side", json!("good")), ("item", json!(k))])));
            out.push('\n');
            if kind == "pair" && it.get("valid").and_then(Value::as_bool).unwrap_or(false) {
                let bad = it["bad"].as_str().ok_or("pair item without bad")?;
                let wb = tokenize(b.gen.lex, bad);
                if wb.len() != ws.len() {
                    eprintln!("pair_len_mismatch: {id}");
                } else if let Some(e) = (0..ws.len()).find(|&j| ws[j] != wb[j]) {
                    let (feats, cands) = b.record_parts(bad, &wb, Some((e, ws[e].as_str())))?;
                    out.push_str(&vaugelas::text::pyjson::dumps(&record_value(feats, &wb, e as i64, &id, cands, &[("kind", json!("pair")), ("side", json!("bad")), ("item", json!(k))])));
                    out.push('\n');
                } else {
                    eprintln!("pair_identical: {id}");
                }
            }
            Ok(out)
        })
        .collect::<Result<Vec<String>, String>>()?;
    let secs = t0.elapsed().as_secs_f64();
    let body = bodies.concat();
    write_out(out, &body)?;
    let records = body.lines().count();
    Ok(Stats { records, secs })
}

/// `cands`: the evaluation record (`record_pf`, no gold) for each input line. A line is JSON with a `text`/`sentence`
/// field (an `id`/`template` names the source), or plain text; one record per line.
pub fn cands(texts: &Path, out: &Path, data: &Path) -> Result<Stats, String> {
    let ctx = load(data)?;
    let gen = Gen::new(&ctx.tables.lex, &ctx.tables.nat, &ctx.tables.res);
    let b = Builder::new(&gen, &ctx.ck, &ctx.ignore);
    let text = std::fs::read_to_string(texts).map_err(|e| format!("{}: {e}", texts.display()))?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let stem = texts.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "cands".into());
    let t0 = Instant::now();
    let bodies: Vec<String> = lines
        .par_iter()
        .enumerate()
        .map(|(k, line)| {
            let (src, text) = if line.trim_start().starts_with('{') {
                let v: Value = serde_json::from_str(line).map_err(|e| format!("{}: {e}", texts.display()))?;
                let src = v
                    .get("template")
                    .or_else(|| v.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{stem}-{k}"));
                let t = v.get("text").or_else(|| v.get("sentence")).and_then(Value::as_str).ok_or("line without text/sentence")?;
                (src, t.to_string())
            } else {
                (format!("{stem}-{k}"), (*line).to_string())
            };
            let ws = tokenize(b.gen.lex, &text);
            let (feats, cands) = b.record_parts(&text, &ws, None)?;
            let mut s = vaugelas::text::pyjson::dumps(&record_value(feats, &ws, -1, &src, cands, &[]));
            s.push('\n');
            Ok(s)
        })
        .collect::<Result<Vec<String>, String>>()?;
    let secs = t0.elapsed().as_secs_f64();
    let body = bodies.concat();
    write_out(out, &body)?;
    let records = body.lines().count();
    Ok(Stats { records, secs })
}
