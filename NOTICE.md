# NOTICE

Vaugelas Corrector. Copyright the author (Elvis-codeur).

The code in this repository is licensed under the GNU Affero General Public License, version 3 or (at your option)
any later version: see LICENSE. The data files below are not covered by that licence; each keeps its own terms
(LICENSE-TBD.md).

## Data (downloaded separately, not part of this repository)

- **Lexicon** (`lexicon.tsv`, and the files built from it: `lex/`, `lexicon.tsv.lem.frclex`). This is a dump of
  the French dictionary "Français (Toutes variantes)" (`fr-allvars`, 515,637 entries, built 2025-12-15) shipped
  with **Grammalecte 2.3.0** by Olivier R. (http://grammalecte.net, dictionary from the Dicollecte project).
  Grammalecte is distributed under GPL-3.0, and its dictionary data under its own licence (MPL-2.0 according to
  upstream; to be verified, see LICENSE-TBD.md).
- **Model** (`model.frcm`, energy model H1). Paradigm-energy model trained by the author on: public-domain French
  books from Project Gutenberg (https://www.gutenberg.org), including homophone-swap pairs made from them; sentences
  from the author's generators; French Wikipedia sentences and a French grammatical-error-correction set (the
  "natural" stream); training sentences written with DeepSeek; confusion examples written with Claude (Anthropic);
  readings from the lexicon above. Its vocabulary holds lemma strings from the lexicon.
- **Resources** (`resources.tar.gz`, unpacked into `resources/`): tables the model's candidate lists read.
  Word and word-pair counts from the training books (`ngram/`, `interp/ctx_*.json`, `interp/capstats.json`);
  homophone and near-homophone lists derived from the lexicon (`interp/homophones_lex_1.0*.json`,
  `interp/nearhom.json`); confusion pairs mined from the training errors (`interp/mined_pairs.json`); confusion
  partners written with Claude (`interp/cw_pairs.json`).

The rights questions these raise are listed in [RIGHTS-CHECKLIST.md](RIGHTS-CHECKLIST.md).

## Design credits

- The rule checker follows the design of Harper (https://github.com/Automattic/harper, Apache-2.0). Its rule
  knowledge draws on Grammalecte and LanguageTool (https://languagetool.org, LGPL-2.1). No code from these
  projects is included (to be confirmed by the author, see LICENSE-TBD.md).
- The HTTP API copies the shape of LanguageTool's public `/v2` API so that existing clients can use it.

## Rust dependencies compiled into the binary

Generated from `cargo tree -e normal --target all` (all platforms; build-only crates excluded). None is GPL.

| crate | licence |
|---|---|
| adler2 v2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| ascii v1.1.0 | Apache-2.0 OR MIT |
| base64 v0.22.1 | MIT OR Apache-2.0 |
| block-buffer v0.10.4 | MIT OR Apache-2.0 |
| cfg-if v1.0.5 | MIT OR Apache-2.0 |
| chunked_transfer v1.5.0 | MIT OR Apache-2.0 |
| cpufeatures v0.2.17 | MIT OR Apache-2.0 |
| crc32fast v1.5.4 | MIT OR Apache-2.0 |
| crossbeam-deque v0.8.8 | MIT OR Apache-2.0 |
| crossbeam-epoch v0.9.21 | MIT OR Apache-2.0 |
| crossbeam-utils v0.8.23 | MIT OR Apache-2.0 |
| crypto-common v0.1.7 | MIT OR Apache-2.0 |
| digest v0.10.7 | MIT OR Apache-2.0 |
| displaydoc v0.2.7 | MIT OR Apache-2.0 |
| either v1.19.0 | MIT OR Apache-2.0 |
| equivalent v1.0.3 | Apache-2.0 OR MIT |
| filetime v0.2.29 | MIT/Apache-2.0 |
| flate2 v1.1.10 | MIT OR Apache-2.0 |
| foldhash v0.2.0 | Zlib |
| form_urlencoded v1.2.2 | MIT OR Apache-2.0 |
| fst v0.4.7 | Unlicense/MIT |
| generic-array v0.14.7 | MIT |
| getrandom v0.2.17 | MIT OR Apache-2.0 |
| hashbrown v0.17.1 | MIT OR Apache-2.0 |
| httpdate v1.0.3 | MIT OR Apache-2.0 |
| icu_collections v2.4.0 | Unicode-3.0 |
| icu_locale_core v2.4.0 | Unicode-3.0 |
| icu_normalizer_data v2.4.0 | Unicode-3.0 |
| icu_normalizer v2.4.0 | Unicode-3.0 |
| icu_properties_data v2.4.0 | Unicode-3.0 |
| icu_properties v2.4.0 | Unicode-3.0 |
| icu_provider v2.4.0 | Unicode-3.0 |
| idna_adapter v1.2.2 | Apache-2.0 OR MIT |
| idna v1.1.0 | MIT OR Apache-2.0 |
| indexmap v2.14.2 | Apache-2.0 OR MIT |
| itoa v1.0.18 | MIT OR Apache-2.0 |
| libc v0.2.191 | MIT OR Apache-2.0 |
| litemap v0.8.3 | Unicode-3.0 |
| log v0.4.35 | MIT OR Apache-2.0 |
| matrixmultiply v0.3.11 | MIT/Apache-2.0 |
| memchr v2.8.3 | Unlicense OR MIT |
| memmap2 v0.9.12 | MIT OR Apache-2.0 |
| miniz_oxide v0.9.1 | MIT OR Zlib OR Apache-2.0 |
| once_cell v1.21.4 | MIT OR Apache-2.0 |
| percent-encoding v2.3.2 | MIT OR Apache-2.0 |
| potential_utf v0.1.6 | Unicode-3.0 |
| proc-macro2 v1.0.107 | MIT OR Apache-2.0 |
| quote v1.0.47 | MIT OR Apache-2.0 |
| rawpointer v0.2.1 | MIT/Apache-2.0 |
| rayon-core v1.13.0 | MIT OR Apache-2.0 |
| rayon v1.12.0 | MIT OR Apache-2.0 |
| ring v0.17.14 | Apache-2.0 AND ISC |
| rustls-pki-types v1.15.1 | MIT OR Apache-2.0 |
| rustls v0.23.46 | Apache-2.0 OR ISC OR MIT |
| rustls-webpki v0.103.16 | ISC |
| serde_core v1.0.229 | MIT OR Apache-2.0 |
| serde_derive v1.0.229 | MIT OR Apache-2.0 |
| serde_json v1.0.154 | MIT OR Apache-2.0 |
| serde v1.0.229 | MIT OR Apache-2.0 |
| sha2 v0.10.9 | MIT OR Apache-2.0 |
| simd-adler32 v0.3.10 | MIT |
| smallvec v1.16.5 | MIT OR Apache-2.0 |
| stable_deref_trait v1.2.1 | MIT OR Apache-2.0 |
| subtle v2.6.1 | BSD-3-Clause |
| synstructure v0.14.0 | MIT |
| syn v3.0.9 | MIT OR Apache-2.0 |
| tar v0.4.46 | MIT OR Apache-2.0 |
| tiny_http v0.12.0 | MIT OR Apache-2.0 |
| tinystr v0.8.5 | Unicode-3.0 |
| typenum v1.20.1 | MIT OR Apache-2.0 |
| unicode-ident v1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| untrusted v0.9.0 | ISC |
| ureq v2.12.1 | MIT OR Apache-2.0 |
| url v2.5.8 | MIT OR Apache-2.0 |
| utf8_iter v1.1.0 | Apache-2.0 OR MIT |
| utf8-ranges v1.0.5 | Unlicense/MIT |
| utf_types v1.0.0 | Apache-2.0 OR MIT |
| wasi v0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| webpki-roots v0.26.11 | CDLA-Permissive-2.0 |
| webpki-roots v1.0.11 | CDLA-Permissive-2.0 |
| windows_aarch64_gnullvm v0.52.6 | MIT OR Apache-2.0 |
| windows_aarch64_msvc v0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnullvm v0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnu v0.52.6 | MIT OR Apache-2.0 |
| windows_i686_msvc v0.52.6 | MIT OR Apache-2.0 |
| windows-sys v0.52.0 | MIT OR Apache-2.0 |
| windows-targets v0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnullvm v0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnu v0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_msvc v0.52.6 | MIT OR Apache-2.0 |
| writeable v0.6.4 | Unicode-3.0 |
| yoke-derive v0.8.4 | Unicode-3.0 |
| yoke v0.8.4 | Unicode-3.0 |
| zerofrom-derive v0.1.8 | Unicode-3.0 |
| zerofrom v0.1.8 | Unicode-3.0 |
| zeroize v1.9.1 | Apache-2.0 OR MIT |
| zerotrie v0.2.5 | Unicode-3.0 |
| zerovec-derive v0.11.7 | Unicode-3.0 |
| zerovec v0.11.9 | Unicode-3.0 |
| zlib-rs v0.6.8 | Zlib |
| zmij v1.0.23 | MIT |
