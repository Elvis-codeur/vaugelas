//! Vaugelas Corrector: a French grammar checker.
//!
//! Two checkers, one report:
//! - [`rules`]: hand-written rules over a memory-mapped FST lexicon (agreement, homophones, participles, moods,
//!   spelling, typography). Fast (well under a millisecond per sentence) and precise.
//! - [`model`]: a paradigm-energy neural model that scores every word against the other forms of its paradigm and
//!   proposes the most plausible one. It reads the rule checker's lints as inputs. Pure-Rust CPU inference.
//!
//! [`report::Vaugelas`] runs both and merges their findings, one entry per word.
pub mod model;
pub mod report;
pub mod rules;
pub mod text;
