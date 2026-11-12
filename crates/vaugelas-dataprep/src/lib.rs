//! Vaugelas data preparation in Rust.
//!
//! This crate replaces the archived Python generators (`archive/python-2026-11-12/`):
//!   * [`pyrandom`] — CPython's `random.Random` stream (MT19937, `_randbelow`, `shuffle`, `choice`);
//!   * [`lexpairs`] — `lexpairs.py`, the synthetic lexical minimal pairs (1,080 / 1,620 records);
//!   * [`product`] — `product_pairs.py`, minimal pairs inside real book sentences (the 1,989-item batch 1);
//!   * [`records`] — the `pairs` / `cands` paths of `ebm_data.py` / `ebm_data2.py` over the ported candidate
//!     generator (`vaugelas::model`) and record builder.
//!
//! The generators are deterministic functions of a seed; the parity tests compare their bytes against the Python
//! outputs (`tests/`), and `vaugelas-dataprep --help` documents the command line.
pub mod delta;
pub mod lexpairs;
pub mod lexpairs_data;
pub mod product;
pub mod product_data;
pub mod pyrandom;
pub mod records;
pub mod widen;

/// The seed the archived generators used by default (both Python scripts default to 20261112).
pub const DEFAULT_SEED: u64 = 20261112;

/// The random-stream contract of this port: CPython's `random.Random` (MT19937 + `_randbelow`), as implemented in
/// [`pyrandom`] and pinned by `tests/pyrandom.json`. Recorded in `--version` and in the parity table; a change to the
/// stream means a new version, never a silent change of produced datasets.
pub const ALGORITHM_VERSION: &str = "pyrandom-mt19937-v1";
