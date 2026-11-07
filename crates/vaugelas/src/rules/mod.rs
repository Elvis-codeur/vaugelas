//! The rule checker (fr-check): memory-mapped FST lexicon, French tokenizer, grammar, spelling and typography rules.
pub mod build;
pub mod checker;
pub mod lexicon;
pub mod tokenizer;
pub mod typography;
pub use checker::{CheckOptions, Checker, Lint};
