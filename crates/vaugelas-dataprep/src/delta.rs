//! Step 3 of the owner plan: the margin `E(written) - softmin(E(alternatives))` on the **widened** candidate set,
//! with the current weights (no training), plus the per-word dump used to calibrate the threshold and compute the
//! 55-passage buckets. All model work is Rust; only the metrics are computed by the analysis script.

use crate::widen::{margin, Widener};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;
use vaugelas::model::cands::{Gen, Sent};
use vaugelas::model::drops::Drops;
use vaugelas::model::featmodel::encode;
use vaugelas::model::pipeline::LexCache;
use vaugelas::model::record::{frc_lints, triggers, Builder};
use vaugelas::report::{Data, DataPaths};
use vaugelas::rules::Checker;
use vaugelas::text::characterize::tokenize;
use vaugelas::text::pystr::has_w;

pub struct Ctx {
    pub data: Data,
    pub maxc: usize,
}

pub fn load(data: &Path, lexconf: Option<&Path>, maxc: usize) -> Result<Ctx, String> {
    let paths = DataPaths {
        lex_dir: &data.join("lex"),
        lexicon_tsv: &data.join("lexicon.tsv"),
        resources: &data.join("resources"),
        model: &data.join("model.frcm"),
    };
    let t0 = Instant::now();
    let mut d = Data::load(&paths, false, &LexCache::NextToTsv)?;
    if let Some(p) = lexconf {
        d.tables.res.set_lexconf(p)?;
    }
    d.model.set_maxk(maxc);
    eprintln!("model + tables loaded in {:.1} s ({}, maxk {maxc})", t0.elapsed().as_secs_f64(), d.model.profile.name);
    Ok(Ctx { data: d, maxc })
}

#[derive(Clone)]
pub struct WordScore {
    pub delta: f32,
    pub n: usize,
    pub best: Option<String>,
    pub gold: Option<usize>,
}

/// Score every word that has a candidate set: widened set, one encode, energies, margin. `extra_at` supplies forms
/// the document itself gives (a glossary rendering) at one position.
pub fn sentence_margins(
    ctx: &Ctx,
    gen: &Gen,
    text: &str,
    ws: &[String],
    extra_at: Option<(usize, &[String])>,
    gold: Option<&str>,
) -> Result<Vec<Option<WordScore>>, String> {
    let d = &ctx.data;
    let w = Widener { gen, lexconf: &d.tables.res.lexconf, maxc: ctx.maxc, topk: 12 };
    let s = Sent::new(&d.tables.lex, ws);
    let ck: &Checker = &d.rules;
    let dr = &mut Drops::new();
    let lints = frc_lints(ck, text, &d.model.profile.frc_ignore, dr)?;
    let trig = triggers(&lints, text, ws, dr);
    let b = Builder::new(gen, ck, &d.model.profile.frc_ignore);
    let feats = b.feats(&s, &trig);
    let mut cands = Vec::new();
    for i in 0..ws.len() {
        if !has_w(&ws[i]) {
            continue;
        }
        let extra: Vec<String> = match extra_at {
            Some((j, forms)) if j == i => forms.to_vec(),
            _ => Vec::new(),
        };
        let set = w.widen(&s, i, &extra);
        if set.2.len() >= 2 {
            cands.push(set);
        }
    }
    if cands.is_empty() {
        return Ok(vec![None; ws.len()]);
    }
    let enc = encode(&d.model.enc, &d.model.vocab, ws, &feats, &cands);
    let mut out = vec![None; ws.len()];
    for (i, e, best) in d.model.set_energies(&enc) {
        let forms = cands.iter().find(|(j, _, _)| *j == i).map(|(_, _, cs)| cs);
        let best_form = best.and_then(|k| forms.and_then(|cs| cs.get(k)).map(|x| x.0.clone()));
        let gold_idx = gold.and_then(|g| {
            forms.and_then(|cs| cs.iter().position(|x| x.0 == g).or_else(|| cs.iter().position(|x| x.0.to_lowercase() == g.to_lowercase())))
        });
        out[i] = Some(WordScore { delta: margin(&e), n: e.len(), best: best_form, gold: gold_idx });
    }
    Ok(out)
}

/// `delta-pairs`: every mined pair, margin of the wrong word in the bad sentence and of the correct word in the good
/// sentence at the same changed position. Glossary items also get the reviewed rendering as a candidate at the slot
/// (the document supplies that glossary), disagreement items do not.
pub fn delta_pairs(items: &Path, out: &Path, data: &Path, lexconf: Option<&Path>, maxc: usize) -> Result<usize, String> {
    let ctx = load(data, lexconf, maxc)?;
    let gen = Gen::new(&ctx.data.tables.lex, &ctx.data.tables.nat, &ctx.data.tables.res);
    let text = std::fs::read_to_string(items).map_err(|e| format!("{}: {e}", items.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let out_rows: Vec<Result<String, String>> = lines
        .par_iter()
        .map(|line| {
            let it: Value = serde_json::from_str(line).map_err(|e| format!("{items:?}: {e}"))?;
            let good = it["good"].as_str().ok_or("item without good")?;
            let bad = it["bad"].as_str().ok_or("item without bad")?;
            let right = it["right"].as_str().ok_or("item without right")?;
            let source = it["source"].as_str().unwrap_or("");
            let ws = tokenize(&ctx.data.tables.lex, good);
            let wb = tokenize(&ctx.data.tables.lex, bad);
            let e = if ws.len() != wb.len() { None } else { (0..ws.len()).find(|&j| ws[j] != wb[j]) };
            let Some(e) = e else { return Ok(String::new()) };
            // the model reads at most `enc.maxt` words of a sentence; a word beyond that is not scorable
            if e >= ctx.data.model.enc.maxt { return Ok(String::new()); }
            let extra: Vec<String> = if source == "review_glossary" { vec![right.to_string()] } else { Vec::new() };
            let sb = sentence_margins(&ctx, &gen, bad, &wb, Some((e, &extra)), Some(right))?;
            let sg = sentence_margins(&ctx, &gen, good, &ws, Some((e, &extra)), Some(right))?;
            let (d_bad, n_bad, gold_bad, best_bad) = match &sb[e] {
                Some(x) => (x.delta, x.n, x.gold, x.best.clone()),
                None => (0.0, 0, None, None),
            };
            let (d_good, n_good) = match &sg[e] {
                Some(x) => (x.delta, x.n),
                None => (0.0, 0),
            };
            let v = json!({
                "id": it["id"], "source": source, "split": it["split"], "doc": it["doc"],
                "run": it["run"], "ref_id": it["ref_id"], "right": right, "wrong": it["wrong"],
                "index": e, "delta_bad": d_bad, "delta_good": d_good, "n_bad": n_bad, "n_good": n_good,
                "gold_in_bad": gold_bad, "best_bad": best_bad,
            });
            Ok(vaugelas::text::pyjson::dumps(&v))
        })
        .collect();
    let mut body = String::new();
    let mut n = 0;
    for r in out_rows {
        let s = r?;
        if !s.is_empty() {
            body.push_str(&s);
            body.push('\n');
            n += 1;
        }
    }
    crate::records::write_out(out, &body)?;
    Ok(n)
}

/// `train-records`: widened-set training records (JSONL, gzip when the path ends in `.gz`) for a pair file: one bad
/// record with the gold at the changed position (only when the widened set proposes it — no gold injection) and one
/// good record (all sets gold=written) per item. The training script reads `{feats, words, err, src, cands}`.
pub fn train_records(items: &Path, out: &Path, data: &Path, lexconf: Option<&Path>, maxc: usize) -> Result<(usize, usize), String> {
    let ctx = load(data, lexconf, maxc)?;
    let gen = Gen::new(&ctx.data.tables.lex, &ctx.data.tables.nat, &ctx.data.tables.res);
    let w = Widener { gen: &gen, lexconf: &ctx.data.tables.res.lexconf, maxc, topk: 12 };
    let text = std::fs::read_to_string(items).map_err(|e| format!("{}: {e}", items.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let rows: Vec<Result<String, String>> = lines
        .par_iter()
        .map(|line| {
            let it: Value = serde_json::from_str(line).map_err(|e| format!("{items:?}: {e}"))?;
            let good = it["good"].as_str().ok_or("item without good")?;
            let bad = it["bad"].as_str().ok_or("item without bad")?;
            let right = it["right"].as_str().ok_or("item without right")?;
            let source = it["source"].as_str().unwrap_or("");
            let ws = tokenize(&ctx.data.tables.lex, good);
            let wb = tokenize(&ctx.data.tables.lex, bad);
            let e = if ws.len() != wb.len() { None } else { (0..ws.len()).find(|&j| ws[j] != wb[j]) };
            let Some(e) = e else { return Ok(String::new()) };
            if e >= ctx.data.model.enc.maxt { return Ok(String::new()); }
            let extra: Vec<String> = if source == "review_glossary" { vec![right.to_string()] } else { Vec::new() };
            let mut out = String::new();
            let build = |text: &str, words: &[String], at: Option<usize>| -> Result<(Value, Option<usize>), String> {
                let s = Sent::new(&ctx.data.tables.lex, words);
                let dr = &mut Drops::new();
                let lints = frc_lints(&ctx.data.rules, text, &ctx.data.model.profile.frc_ignore, dr)?;
                let trig = triggers(&lints, text, words, dr);
                let b = Builder::new(&gen, &ctx.data.rules, &ctx.data.model.profile.frc_ignore);
                let feats = b.feats(&s, &trig);
                let mut cands = Vec::new();
                let mut gold = None;
                for i in 0..words.len() {
                    if !has_w(&words[i]) { continue; }
                    let ex: Vec<String> = match at { Some(j) if j == i => extra.clone(), _ => Vec::new() };
                    let set = w.widen(&s, i, &ex);
                    if set.2.len() < 2 { continue; }
                    let g = if Some(i) == at {
                        gold = set.2.iter().position(|x| x.0 == right).or_else(|| set.2.iter().position(|x| x.0.to_lowercase() == right.to_lowercase()));
                        gold.unwrap_or(0)
                    } else { 0 };
                    cands.push((i, g, set.2));
                }
                let v = json!({"feats": feats, "words": words, "cands": cands.iter().map(|(i, g, cs)| json!([i, g, cs])).collect::<Vec<_>>()});
                Ok((v, gold))
            };
            let (mut rb, gold) = build(bad, &wb, Some(e))?;
            if gold.is_none() { return Ok(String::new()); }  // no candidate set with the gold: useless for training
            rb["err"] = json!(e);
            rb["src"] = json!(format!("{}/bad", it["id"].as_str().unwrap_or("?")));
            out.push_str(&vaugelas::text::pyjson::dumps(&rb));
            out.push('\n');
            let (mut rg, _) = build(good, &ws, None)?;
            rg["err"] = json!(-1);
            rg["src"] = json!(format!("{}/good", it["id"].as_str().unwrap_or("?")));
            out.push_str(&vaugelas::text::pyjson::dumps(&rg));
            out.push('\n');
            Ok(out)
        })
        .collect();
    let mut body = String::new();
    let (mut n_bad, mut n_good) = (0, 0);
    for r in rows {
        let s = r?;
        if s.is_empty() { continue; }
        n_bad += s.lines().filter(|l| l.contains("\"err\": ") && !l.contains("\"err\": -1")).count();
        n_good += s.lines().filter(|l| l.contains("\"err\": -1")).count();
        body.push_str(&s);
    }
    crate::records::write_out(out, &body)?;
    Ok((n_bad, n_good))
}

/// `delta-text`: one passage per line -> per-sentence, per-word margins with character offsets (the offsets the
/// evaluation corpus and the checker report use). Sentences are split exactly like the checker (`sent_split`).
pub fn delta_text(texts: &Path, out: &Path, data: &Path, lexconf: Option<&Path>, maxc: usize) -> Result<usize, String> {
    let ctx = load(data, lexconf, maxc)?;
    let gen = Gen::new(&ctx.data.tables.lex, &ctx.data.tables.nat, &ctx.data.tables.res);
    let text = std::fs::read_to_string(texts).map_err(|e| format!("{}: {e}", texts.display()))?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let rows: Vec<Result<String, String>> = lines
        .par_iter()
        .enumerate()
        .map(|(k, line)| {
            let mut sents = Vec::new();
            for sent in vaugelas::text::characterize::sent_split(line) {
                if sent.trim().is_empty() { continue; }
                let byte = sent.as_ptr() as usize - line.as_ptr() as usize;
                let start = line[..byte].chars().count();
                let nchars = sent.chars().count();
                let ws = tokenize(&ctx.data.tables.lex, sent);
                let sc = sentence_margins(&ctx, &gen, sent, &ws, None, None)?;
                // char span of each token in the sentence (sequential search, ' as ’ like the tokenizer)
                let chars: Vec<char> = sent.chars().collect();
                let mut pos = 0usize;
                let mut spans: Vec<Option<(usize, usize)>> = Vec::new();
                for w in &ws {
                    let wc: Vec<char> = w.chars().map(|c| if c == '\'' { '’' } else { c }).collect();
                    let hit = if wc.len() > chars.len() { None } else {
                        (pos..=chars.len() - wc.len()).find(|&j| chars[j..j + wc.len()] == wc[..])
                    };
                    match hit {
                        Some(j) => { spans.push(Some((j, j + wc.len()))); pos = j + wc.len(); }
                        None => { spans.push(None); }
                    }
                }
                sents.push(json!({
                    "start": start, "end": start + nchars, "text": sent, "words": ws,
                    "word_spans": spans.iter().map(|x| x.map_or(Value::Null, |(a, b)| json!([a, b]))).collect::<Vec<_>>(),
                    "deltas": sc.iter().map(|x| x.as_ref().map_or(Value::Null, |s| json!(s.delta))).collect::<Vec<_>>(),
                    "sizes": sc.iter().map(|x| x.as_ref().map_or(0, |s| s.n)).collect::<Vec<_>>(),
                    "best": sc.iter().map(|x| x.as_ref().and_then(|s| s.best.clone()).map_or(Value::Null, Value::String)).collect::<Vec<_>>(),
                }));
            }
            Ok(vaugelas::text::pyjson::dumps(&json!({ "line": k, "text": line, "sentences": sents })))
        })
        .collect();
    let mut body = String::new();
    let mut n = 0;
    for r in rows {
        body.push_str(&r?);
        body.push('\n');
        n += 1;
    }
    crate::records::write_out(out, &body)?;
    Ok(n)
}
