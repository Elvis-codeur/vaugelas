//! Text analysis for the energy model: tokenizer, sentence filter, characterizer (readings of every word, suffix
//! guesser, lemma atom), lexicon tables and their on-disk cache. Ported from fr-check-infer `frc_prep` (commit cc7d94e),
//! itself proven byte-identical to the Python reference (training-design/characterize.py, natural.py,
//! sentence_filter.py), with the Python string semantics it depends on (`pystr`, `pyunicode_data`, `pyjson`).
pub mod characterize;
pub mod fxhash;
pub mod lexcache;
pub mod natural;
pub mod pyjson;
pub mod pystr;
#[rustfmt::skip]
pub mod pyunicode_data;
pub mod sentence_filter;
