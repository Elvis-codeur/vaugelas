//! The lexical and corpus tables the v14 candidate generator loads from `build/interp` and `build/ngram`
//! (ebm_cands2.py `_mined`, `_lexhom`, `LEXHOM_PHON`, `_cwpairs`, `_ctx`, `CAPSTATS`; ebm_data2.py `_ngram`).
//!
//! Location: one directory holding `interp/` and `ngram/` (the data file `resources.tar.gz`, unpacked). Every file the
//! v14 flag set needs must exist: a missing table would silently change the candidate lists (Python audit 14), so loading fails instead. This includes
//! `interp/ctx_trigrams.json`, which Python merely warns about (`ctx_trigrams_missing`) and then builds different
//! context promotions (ctx=1) without: 123 of the 2,932 kit records changed when it was absent.
//!
//! All counts are integers in the JSON files; a non-integer value is an error rather than a silent conversion.
use std::collections::HashMap;
use std::path::Path;

/// A form -> list-of-forms table (lexical homophones, near-homophones, confusion pairs).
pub type FormLists = HashMap<String, Vec<String>>;

#[derive(Default)]
pub struct Resources {
    /// `MINED`: typed form -> confusion forms mined from the training errors (n >= 3, no "ne"/"pas", no spaces)
    pub mined: FormLists,
    /// `LEXHOM`: form -> lexical homophones (homophones_lex_1.0.json)
    pub lexhom: FormLists,
    /// `LEXHOM_PHON`: form -> pronunciation of its homophone group (homophones_lex_1.0.phon.json)
    pub lexhom_phon: HashMap<String, String>,
    /// `CW_PAIRS`: lemma or form -> Claude-written confusion partners (cw_pairs.json; Python keeps them as sets)
    pub cw_pairs: FormLists,
    /// `NEARHOM`: form -> near-homophones (nearhom.json)
    pub nearhom: FormLists,
    /// `BIG`: "w1 w2" (ctx_bigrams.json) and "w1 w2 w3" (ctx_trigrams.json) -> count in the training books
    pub ctx_big: HashMap<String, i64>,
    /// `CAPSTATS` (interp/capstats.json): lower-case form -> [capitalised mid-sentence, lower-case] counts
    pub capstats: HashMap<String, (i64, i64)>,
    /// ebm_data2 `_ngram()[0]`: lower-case form -> count (ngram/unigrams.json)
    pub unigrams: HashMap<String, i64>,
    /// ebm_data2 `_ngram()[1]`: "w1 w2" -> count (ngram/bigrams.json)
    pub bigrams: HashMap<String, i64>,
    /// ebm_data2 `_ngram()[2]` (ngram/capstats2.json)
    pub capstats2: HashMap<String, (i64, i64)>,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn pairs(m: HashMap<String, [i64; 2]>) -> HashMap<String, (i64, i64)> {
    m.into_iter().map(|(k, [a, b])| (k, (a, b))).collect()
}

/// ebm_cands2.py `_mined()`: `[[w, c, n], ...]` -> {w: [c, ...]} for n >= 3, without "n’"/"n'"/"ne"/"pas" and
/// multi-word corrections.
fn mined(path: &Path) -> Result<FormLists, String> {
    let rows: Vec<(String, String, i64)> = parse(path)?;
    let skip = ["n’", "n'", "ne", "pas"];
    let mut out: FormLists = HashMap::new();
    for (w, c, n) in rows {
        if n < 3 || skip.contains(&w.as_str()) || skip.contains(&c.as_str()) || c.contains(' ') { continue; }
        out.entry(w).or_default().push(c);
    }
    Ok(out)
}

impl Resources {
    /// Load every table from `dir/interp` and `dir/ngram`.
    pub fn load(dir: &Path) -> Result<Resources, String> {
        let i = dir.join("interp");
        let g = dir.join("ngram");
        let mut ctx_big: HashMap<String, i64> = parse(&i.join("ctx_bigrams.json"))?;
        let tri: HashMap<String, i64> = parse(&i.join("ctx_trigrams.json"))?;
        ctx_big.extend(tri); // Python: big.update(json.load(ctx_trigrams.json))
        Ok(Resources {
            mined: mined(&i.join("mined_pairs.json"))?,
            lexhom: parse(&i.join("homophones_lex_1.0.json"))?,
            lexhom_phon: parse(&i.join("homophones_lex_1.0.phon.json"))?,
            cw_pairs: parse(&i.join("cw_pairs.json"))?,
            nearhom: parse(&i.join("nearhom.json"))?,
            ctx_big,
            capstats: pairs(parse(&i.join("capstats.json"))?),
            unigrams: parse(&g.join("unigrams.json"))?,
            bigrams: parse(&g.join("bigrams.json"))?,
            capstats2: pairs(parse(&g.join("capstats2.json"))?),
        })
    }
}
