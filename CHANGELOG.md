# Changelog

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
