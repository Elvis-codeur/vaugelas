//! text -> sentences -> tokens -> characterised readings + candidate sets -> model scores, all in Rust.
use crate::model::cands::{CandSet, Cands};
use crate::model::{CandRef, Model};
use crate::text::characterize::{sent_split, tokenize, Lexicon, Tok};
use crate::text::natural::Natural;
use std::path::Path;

pub struct Pipeline { pub lex: Lexicon, pub nat: Natural }

/// one flagged word: (sentence index, word index, word, score, suggested form)
pub struct Flag { pub sent: usize, pub word: usize, pub text: String, pub score: f32, pub suggestion: Option<String> }

/// One scored sentence: its byte offset in the text, its words, per word (score, lowest-energy form).
pub struct ScoredSentence { pub offset: usize, pub words: Vec<String>, pub scores: Vec<(f32, Option<String>)> }

/// Where the built lexicon tables are cached.
#[derive(Clone, Debug, Default)]
pub enum LexCache {
    /// next to the TSV (`lexicon.tsv.lem.frclex`)
    #[default]
    NextToTsv,
    /// at this path
    At(std::path::PathBuf),
    /// no cache: build the tables from the TSV at every start (~6 s)
    Off,
}

impl Pipeline {
    /// lexicon.tsv with the lemma atom on (the energy model is trained with it). The built tables are cached (see
    /// `text::lexcache`) and rebuilt only when lexicon.tsv changes. `VAUGELAS_LEX_CACHE=PATH|off` overrides.
    pub fn load(lexicon_tsv: &str) -> Pipeline {
        let cache = match std::env::var("VAUGELAS_LEX_CACHE").ok().as_deref() {
            Some("off") => LexCache::Off,
            Some(p) => LexCache::At(p.into()),
            None => LexCache::NextToTsv,
        };
        Self::load_with(Path::new(lexicon_tsv), &cache).expect("lexicon.tsv")
    }

    pub fn load_with(tsv: &Path, cache: &LexCache) -> std::io::Result<Pipeline> {
        Ok(match cache {
            LexCache::Off => {
                let text = crate::text::pystr::universal_newlines(std::fs::read_to_string(tsv)?);
                Pipeline { lex: Lexicon::from_tsv_with(&text, true), nat: Natural::from_tsv(&text) }
            }
            c => {
                let at = if let LexCache::At(p) = c { Some(p.as_path()) } else { None };
                let (lex, nat, _) = crate::text::lexcache::load_or_build(tsv, true, at)?;
                Pipeline { lex, nat }
            }
        })
    }

    /// feats as strings (featdata.feats: readings + "guessed")
    pub fn feats<'a>(&'a self, toks: &[Tok]) -> Vec<Vec<&'a str>> {
        toks.iter().map(|t| {
            let mut v: Vec<&str> = t.r.iter().map(|k| self.lex.keys[*k as usize].as_str()).collect();
            if t.guessed { v.push("guessed"); }
            v
        }).collect()
    }

    pub fn cand_refs<'a>(&'a self, cs: &'a [CandSet]) -> Vec<CandRef<'a>> {
        cs.iter().map(|(i, g, v)| (*i, *g, v.iter().map(|(f, ks)| (f.as_str(), ks.iter().map(|k| self.lex.keys[*k as usize].as_str()).collect())).collect())).collect()
    }

    /// score one sentence: per word (score, suggestion = lowest-energy candidate form)
    pub fn score_sentence(&self, model: &Model, cands: &Cands, s: &str) -> (Vec<String>, Vec<(f32, Option<String>)>) {
        let ws = tokenize(&self.lex, s);
        let (toks, cs) = cands.record(&ws, None, None);
        let feats = self.feats(&toks); let words: Vec<&str> = ws.iter().map(|w| w.as_str()).collect();
        let refs = self.cand_refs(&cs);
        let sc = model.score_parts(&feats, &words, &refs);
        let per: Vec<(f32, Option<String>)> = sc.into_iter().enumerate().map(|(i, (s, best))| {
            let sug = best.and_then(|b| refs.iter().find(|c| c.0 == i).map(|c| c.2[b].0.to_string()));
            (s, sug)
        }).collect();
        (ws, per)
    }

    /// score every non-blank sentence of a text, with the sentence's byte offset in `text`
    pub fn score_sentences(&self, model: &Model, cands: &Cands, text: &str) -> Vec<ScoredSentence> {
        sent_split(text).into_iter().filter(|s| !s.trim().is_empty()).map(|s| {
            let offset = s.as_ptr() as usize - text.as_ptr() as usize;
            let (words, scores) = self.score_sentence(model, cands, s);
            ScoredSentence { offset, words, scores }
        }).collect()
    }

    /// score every sentence of a text; returns per sentence (words, per-word (score, suggestion))
    pub fn score_text(&self, model: &Model, cands: &Cands, text: &str) -> Vec<(Vec<String>, Vec<(f32, Option<String>)>)> {
        self.score_sentences(model, cands, text).into_iter().map(|s| (s.words, s.scores)).collect()
    }

    pub fn flags(&self, model: &Model, cands: &Cands, text: &str, thr: f32) -> Vec<Flag> {
        let mut out = Vec::new();
        for (si, (ws, per)) in self.score_text(model, cands, text).into_iter().enumerate() {
            for (wi, (s, sug)) in per.into_iter().enumerate() {
                if s > thr { out.push(Flag { sent: si, word: wi, text: ws[wi].clone(), score: s, suggestion: sug.filter(|x| *x != ws[wi]) }); }
            }
        }
        out
    }
}

/// Byte spans of the tokens of `tokenize(sentence)` inside `sentence` (tokens are contiguous pieces of the text,
/// with `'` written as `’`); None for a token that cannot be found.
pub fn token_spans(sentence: &str, toks: &[String]) -> Vec<Option<(usize, usize)>> {
    let mut pos = 0usize;
    toks.iter().map(|t| {
        let rest = &sentence[pos..];
        let skip: usize = rest.chars().take_while(|c| crate::text::pystr::is_space(*c)).map(char::len_utf8).sum();
        let start = pos + skip;
        let mut end = start;
        let mut it = sentence[start..].chars();
        for tc in t.chars() {
            match it.next() {
                Some(c) if c == tc || (tc == '’' && c == '\'') => end += c.len_utf8(),
                _ => return None,
            }
        }
        pos = end;
        Some((start, end))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::token_spans;
    #[test]
    fn spans_follow_elision_and_apostrophes() {
        let s = "L'enfant  dit-il « oui ».";
        let toks: Vec<String> = ["L’", "enfant", "dit", "-il", "«", "oui", "»", "."].iter().map(|x| x.to_string()).collect();
        let sp = token_spans(s, &toks);
        let got: Vec<&str> = sp.iter().map(|x| { let (a, b) = x.unwrap(); &s[a..b] }).collect();
        assert_eq!(got, vec!["L'", "enfant", "dit", "-il", "«", "oui", "»", "."]);
        assert_eq!(token_spans("abc", &["abd".to_string()]), vec![None]);
    }
}
