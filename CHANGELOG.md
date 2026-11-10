# Changelog

## [0.3.0] - 2026-11-10

New model: **N1**, trained without any data written by Claude (data set `data-3`; an installation of 0.2.x downloads
the new model and resources on the first start, the lexicon is unchanged).

### Changed
- The energy model is now **N1** (`n1_dw_scratch40k`, 76 MB) instead of H1. The owner chose to stay clear of
  Anthropic's terms of service: every Claude-written training example (about 4,700 error pairs) and the confusion
  partners made from them (`resources/interp/cw_pairs.json`) were replaced by DeepSeek-written equivalents (same
  families and counts), and the model was trained from scratch on H1's recipe (40,000 steps).
- Default threshold **0.9935**: the model alone flags 66 of 18,703 correct book sentences (3.5 per 1,000), the same
  budget as H1's 0.9914.
- Measured on the hard test set final-2 at that threshold, the model alone finds **47.8%** of the errors (H1: 51.4%):
  weaker on noun forms (17.9% vs 40.7%) and capitals (16.7% vs 35.0%), stronger on verb forms (52.5% vs 22.5%) and
  elision; at an equal false-alarm rate on final-2 the two models are level (53.3% vs 53.8% at 2%). Many noun-form and
  capital errors are caught by the rules anyway (spelling of invented plurals such as *caillous*, capitals of months
  and days). Details: docs/capabilities.md.
- The research implementation and the program give identical scores (parity test, 343 sentences, max difference 0).

## [0.2.1] - 2026-11-10

Same model (H1) and same data files (data set `data-2`): an installation of 0.2.0 keeps its data.

### Added
- **Settings file** `settings.toml` (#9): every setting of `check` and `serve` (model threshold, rules / model on or
  off, spelling suggestions, strict typography, int8, rules and categories never to report, data folder, server host,
  port and threads), written once instead of typed on every command. `vaugelas settings --init` creates it with every
  setting explained (French and English); `vaugelas settings` shows the file in use and the settings in force;
  `vaugelas rules` lists the rule names. Read from `--config FILE`, `VAUGELAS_CONFIG`, next to the program, or the
  user's settings folder. Command-line options take precedence. A mistake in the file is an error that names it.
- `disabled_rules` / `disabled_categories` now apply to `vaugelas check` too (before: server requests only).

### Changed
- Licences (#7): the code is under the MIT licence (LICENSE); the model and the resources archive under CC BY-SA 4.0,
  the lexicon under MPL-2.0 (DATA-LICENSES.md, texts in LICENSES/). Release archives ship them.

### Fixed
- `vaugelas --version` (and `-V`) printed the help instead of the version when given without a command (#5).

## [0.2.0] - 2026-11-09

New model: **H1**. The data files change (data set `data-2`); an installation from 0.1.0 updates itself on the first
start (files already correct are kept).

### Changed
- The energy model is now **H1** (`h1_hom_warm8k`, 78 MB) instead of `ebm_ds`. H1 was trained on stratified data and
  on 22,030 homophone-swap pairs. On the project's hard test set final-2 the model alone finds 51.4% of the errors at
  the default threshold, and 55.1% on the homophone test (a test that favours it: see docs/capabilities.md). On
  18,703 correct sentences of books it flags about 3.5 per 1,000.
- The model now reads the rule checker's findings as inputs, and builds its candidate forms with the research
  pipeline of the v14 models (homophone groups, corpus counts). Its per-word scores are identical to the research
  implementation (0 difference on 343 test sentences, 3,256 words).
- Default threshold **0.9914** (was 0.9571, which belonged to `ebm_ds`). It now comes from the model, as does the list
  of rule findings the model reads; `vaugelas info` shows both.
- When two rules flag the same word, the report shows one entry with both suggestions (it showed two entries).

### Added
- First start installs the data from the program's own folder (or the current folder) when the three data files are
  there: friends of a private repository can install without a download.
- New data file `resources.tar.gz` (the model's corpus tables), every file checked by sha256.
- `vaugelas score`: per-word model scores, one sentence per line (JSON lines).
- docs/capabilities.md and .en.md: what Vaugelas does well and badly, with numbers and examples.
- RIGHTS-CHECKLIST.md and SHARING.md for the owner; this changelog; RELEASING.md.
- CI runs clippy (no warnings allowed). The release workflow builds and tests the four platforms on pull requests
  that change it, adds the data files to the draft release, and uses the CHANGELOG section as release notes.

### Removed
- Support for round-3 models (`ebm_ds`): Vaugelas now refuses a model without the v14 input encoding, and refuses an
  unknown model instead of guessing its settings.

### Fixed
- The macOS builds used retired GitHub runners (`macos-13`, retired 2025-12-04; `macos-14`, retired 2026-11-02); they
  now use `macos-15-intel` (Intel) and `macos-15` (Apple silicon).

## [0.1.0] - 2026-11-07

First version (not released). Rules (fr-check) and the energy model `ebm_ds` in one Rust program: `vaugelas check`,
a LanguageTool-compatible server, data download with sha256 checks, CI for Linux and a release workflow for Linux,
Windows and macOS.
