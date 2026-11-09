//! text -> sentences -> v14 records -> model scores, all in Rust.
//!
//! One sentence goes through exactly the steps of fr-check-infer `frc-infer v14` (`v14::process`):
//! `sentence_filter::clean` -> `tokenize` -> `sentence_filter::skip` -> `record_pf` (fr-check lints as trigger atoms,
//! v2 candidate lists) -> `encode` -> scores. Every step is a pure function of the sentence and the read-only tables,
//! so a `Scorer` can be shared between threads.
use crate::model::cands::Gen;
use crate::model::record::Builder;
use crate::model::resources::Resources;
use crate::model::Model;
use crate::rules::Checker;
use crate::text::characterize::{sent_split, tokenize, Lexicon};
use crate::text::natural::Natural;
use crate::text::sentence_filter::{clean, skip, Skip};
use std::path::{Path, PathBuf};

/// Where the built lexicon tables are cached.
#[derive(Clone, Debug, Default)]
pub enum LexCache {
    /// next to the TSV (`lexicon.tsv.lem.frclex`)
    #[default]
    NextToTsv,
    /// at this path
    At(PathBuf),
    /// no cache: build the tables from the TSV at every start (~6 s)
    Off,
}

/// The read-only tables of the v14 record path: lexicon (characterizer + natural.py tables) and the corpus resources.
pub struct Tables { pub lex: Lexicon, pub nat: Natural, pub res: Resources }

impl Tables {
    /// `lexicon_tsv` with the lemma atom on (the model is trained with it); `resources`: the directory holding `interp/`
    /// and `ngram/`. The lexicon tables are cached (see `text::lexcache`) and rebuilt only when lexicon.tsv changes.
    pub fn load(lexicon_tsv: &Path, resources: &Path, cache: &LexCache) -> Result<Tables, String> {
        let (lx, res) = rayon::join(|| Self::lexicon(lexicon_tsv, cache), || Resources::load(resources));
        let (lex, nat) = lx.map_err(|e| format!("{}: {e}", lexicon_tsv.display()))?;
        Ok(Tables { lex, nat, res: res? })
    }

    fn lexicon(tsv: &Path, cache: &LexCache) -> std::io::Result<(Lexicon, Natural)> {
        Ok(match cache {
            LexCache::Off => {
                let text = crate::text::pystr::universal_newlines(std::fs::read_to_string(tsv)?);
                (Lexicon::from_tsv_with(&text, true), Natural::from_tsv(&text))
            }
            c => {
                let at = if let LexCache::At(p) = c { Some(p.as_path()) } else { None };
                let (lex, nat, _) = crate::text::lexcache::load_or_build(tsv, true, at)?;
                (lex, nat)
            }
        })
    }
}

/// One scored sentence.
#[derive(Clone, Debug)]
pub struct Scored {
    /// the tokens of the cleaned sentence
    pub words: Vec<String>,
    /// why the model does not score this sentence (English, foreign, markup, no word); `scores` is then empty
    pub skip: Option<Skip>,
    /// per word (at most the model's 96 first): (score 0..1, lowest-energy candidate form when the word has a set)
    pub scores: Vec<(f64, Option<String>)>,
}

/// Byte span of each word of a sentence in the checked text (None: not located).
pub type Spans = Vec<Option<(usize, usize)>>;

/// The model side of the checker: candidate generator + record builder + model, over borrowed tables.
pub struct Scorer<'a> { gen: Gen<'a>, ck: &'a Checker, model: &'a Model }

impl<'a> Scorer<'a> {
    /// `ck`: the fr-check rule checker whose lints are model inputs (the same one the report uses).
    pub fn new(t: &'a Tables, ck: &'a Checker, model: &'a Model) -> Scorer<'a> { Scorer { gen: Gen::new(&t.lex, &t.nat, &t.res), ck, model } }

    pub fn model(&self) -> &'a Model { self.model }

    /// One sentence through the v14 path (frc-infer `v14::process`). Err when the text holds a line break.
    pub fn score_sentence(&self, text: &str) -> Result<Scored, String> {
        let c = clean(text);
        let words = tokenize(self.gen.lex, &c);
        if let Some(sk) = skip(&words, self.gen.nat) { return Ok(Scored { words, skip: Some(sk), scores: Vec::new() }); }
        let b = Builder::new(&self.gen, self.ck, &self.model.profile.frc_ignore);
        let r = b.record_pf(&c, &words)?;
        let scores = self.model.score_parts(&r.feats, &r.words, &r.cands).into_iter().enumerate().map(|(i, (s, best))| {
            let sug = best.and_then(|k| r.cands.iter().find(|c| c.0 == i).map(|c| c.2[k].0.clone()));
            (s, sug)
        }).collect();
        Ok(Scored { words, skip: None, scores })
    }

    /// Every non-blank sentence of a one-line text (`sent_split`), with its byte offset in `text` and the byte span of
    /// each of its words in `text` (None for a word that cannot be located). A sentence that cannot be scored (a line
    /// break inside it) is left out.
    pub fn score_line(&self, text: &str) -> Vec<(Scored, Spans)> {
        sent_split(text).into_iter().filter(|s| !s.trim().is_empty()).filter_map(|s| {
            let offset = s.as_ptr() as usize - text.as_ptr() as usize;
            let sc = self.score_sentence(s).ok()?;
            let spans = token_spans(s, &sc.words).into_iter().map(|x| x.map(|(a, b)| (offset + a, offset + b))).collect();
            Some((sc, spans))
        }).collect()
    }
}

/// Byte spans of the tokens of `tokenize(clean(sentence))` inside the raw `sentence`, in order. A token is matched at
/// the first position at or after the end of the previous one, with `'` in the text standing for the tokenizer's `’`;
/// what `clean` removed (`_`, `[12]`, `[Note …]`, runs of spaces) is skipped that way. None for a token not found.
pub fn token_spans(sentence: &str, toks: &[String]) -> Spans {
    let chars: Vec<(usize, char)> = sentence.char_indices().collect();
    let same = |c: char, t: char| c == t || (t == '’' && c == '\'');
    let mut pos = 0usize;                   // index into `chars`
    toks.iter().map(|t| {
        let tc: Vec<char> = t.chars().collect();
        if tc.is_empty() || tc.len() > chars.len() { return None; }
        let k = (pos..=chars.len() - tc.len()).find(|&k| tc.iter().enumerate().all(|(j, &x)| same(chars[k + j].1, x)))?;
        pos = k + tc.len();
        let end = chars.get(pos).map_or(sentence.len(), |c| c.0);
        Some((chars[k].0, end))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::token_spans;
    fn v(xs: &[&str]) -> Vec<String> { xs.iter().map(|x| x.to_string()).collect() }

    #[test]
    fn spans_follow_elision_apostrophes_and_cleaning() {
        let s = "L'enfant  dit-il « oui ».";
        let sp = token_spans(s, &v(&["L’", "enfant", "dit", "-il", "«", "oui", "»", "."]));
        let got: Vec<&str> = sp.iter().map(|x| { let (a, b) = x.unwrap(); &s[a..b] }).collect();
        assert_eq!(got, vec!["L'", "enfant", "dit", "-il", "«", "oui", "»", "."]);
        // clean() removed "_" and "[12]": the words are still found in the raw text
        let s = "Une _liberté_ rare[12] ici.";
        let sp = token_spans(s, &v(&["Une", "liberté", "rare", "ici", "."]));
        let got: Vec<&str> = sp.iter().map(|x| { let (a, b) = x.unwrap(); &s[a..b] }).collect();
        assert_eq!(got, vec!["Une", "liberté", "rare", "ici", "."]);
        assert_eq!(token_spans("abc", &v(&["abd"])), vec![None]);
    }
}
