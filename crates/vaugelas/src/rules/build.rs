//! Build the rule checker's FST lexicon directory (words.fst, recs.bin, gen.fst, lemmas.bin, tags.txt) from
//! lexicon.tsv (rows: form \t lemma \t tag). Same bytes as fr-check's `frc-build`.
use fst::{MapBuilder, SetBuilder};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Counts of what was written: (rows, lemmas, tags).
pub struct BuildStats { pub rows: usize, pub lemmas: usize, pub tags: usize }

fn io_err<E: std::fmt::Display>(e: E) -> std::io::Error { std::io::Error::other(e.to_string()) }

/// Build the FST lexicon of `tsv` into directory `out` (created if missing).
pub fn build_lexicon(tsv: &Path, out: &Path) -> std::io::Result<BuildStats> {
    let text = std::fs::read_to_string(tsv)?;
    std::fs::create_dir_all(out)?;

    let mut lemma_ids: HashMap<String, u32> = HashMap::new();
    let mut lemma_list: Vec<String> = Vec::new();
    let mut tag_ids: HashMap<String, u16> = HashMap::new();
    let mut tag_list: Vec<String> = Vec::new();
    let mut rows: Vec<(String, u32, u16)> = Vec::new();

    for line in text.lines() {
        let mut f = line.split('\t');
        let (Some(flex), Some(lemma), Some(tag)) = (f.next(), f.next(), f.next()) else { continue };
        let tag: String = tag.split('/').next().unwrap_or("").chars().filter(|c| *c != '!').collect();
        let lid = *lemma_ids.entry(lemma.to_string()).or_insert_with(|| { lemma_list.push(lemma.to_string()); (lemma_list.len() - 1) as u32 });
        let tid = *tag_ids.entry(tag.clone()).or_insert_with(|| { tag_list.push(tag.clone()); (tag_list.len() - 1) as u16 });
        rows.push((flex.to_string(), lid, tid));
    }
    rows.sort();
    rows.dedup();

    // words.fst + recs.bin
    let mut recs: Vec<u8> = vec![0]; // offset 0 reserved
    let mut mb = MapBuilder::new(BufWriter::new(std::fs::File::create(out.join("words.fst"))?)).map_err(io_err)?;
    let mut i = 0;
    while i < rows.len() {
        let mut j = i;
        while j < rows.len() && rows[j].0 == rows[i].0 { j += 1; }
        let n = j - i;
        if n >= 256 { return Err(io_err(format!("too many readings for {}", rows[i].0))); }
        let off = recs.len() as u64;
        recs.push(n as u8);
        for r in &rows[i..j] {
            let mut v = r.1;
            loop {
                let b = (v & 0x7f) as u8; v >>= 7;
                if v == 0 { recs.push(b); break } else { recs.push(b | 0x80) }
            }
            recs.extend_from_slice(&r.2.to_le_bytes());
        }
        mb.insert(rows[i].0.as_bytes(), off).map_err(io_err)?;
        i = j;
    }
    mb.finish().map_err(io_err)?;
    std::fs::write(out.join("recs.bin"), &recs)?;

    // gen.fst: lemma \0 form \0 tagid for N, A, V
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for (flex, lid, tid) in &rows {
        let c = tag_list[*tid as usize].trim_start_matches(':').chars().next().unwrap_or(' ');
        if c == 'N' || c == 'A' || c == 'V' {
            let mut k = lemma_list[*lid as usize].as_bytes().to_vec();
            k.push(0); k.extend_from_slice(flex.as_bytes()); k.push(0);
            k.extend_from_slice(tid.to_string().as_bytes());
            keys.push(k);
        }
    }
    keys.sort();
    keys.dedup();
    let mut sb = SetBuilder::new(BufWriter::new(std::fs::File::create(out.join("gen.fst"))?)).map_err(io_err)?;
    for k in &keys { sb.insert(k).map_err(io_err)?; }
    sb.finish().map_err(io_err)?;

    // lemmas.bin: [count u32][offsets u32 x (count+1)][bytes]
    let mut w = BufWriter::new(std::fs::File::create(out.join("lemmas.bin"))?);
    w.write_all(&(lemma_list.len() as u32).to_le_bytes())?;
    let mut off = 0u32;
    w.write_all(&off.to_le_bytes())?;
    for l in &lemma_list { off += l.len() as u32; w.write_all(&off.to_le_bytes())?; }
    for l in &lemma_list { w.write_all(l.as_bytes())?; }
    w.flush()?;
    drop(w);
    std::fs::write(out.join("tags.txt"), tag_list.join("\n"))?;
    Ok(BuildStats { rows: rows.len(), lemmas: lemma_list.len(), tags: tag_list.len() })
}
