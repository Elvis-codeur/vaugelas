# Data-preparation parity — Rust `vaugelas-dataprep` vs the archived Python

Owner rule of 2026-11-12: the data-prep stack is Rust only; equivalence (not byte identity) is the acceptance bar,
with every difference named. This page is the evidence. It is produced by `make parity`
(`tools/parity.py`) and committed next to the code it judges.

Environment: Python 3.12.3 (the ThinkPad, the interpreter that produced the archived datasets), Rust 1.99.1,
`seed = 20261112`, random-stream contract `pyrandom-mt19937-v1` (CPython MT19937 + `_randbelow`, pinned by
`crates/vaugelas-dataprep/tests/pyrandom.json`). The two generators' outputs are **byte-identical**, `CARD.md`
included. The record path is **equivalent with two named differences** (below), both caused by the Python research
script being older than the v14 inference path the Rust code ports.

## Datasets

| check | Python sha256 | Rust sha256 | result |
|---|---|---|---|
| `lexpairs` 1,080 records, archived generator (40/sub-type, 16 contexts) | `ae10a3859f167fc6…` | `ae10a3859f167fc6…` | **identical** |
| `lexpairs` 1,080 — `CARD.md` | `8565a2a64c1f1153…` | `8565a2a64c1f1153…` | **identical** |
| `lexpairs` 1,620 records (60/sub-type, 16 contexts) | `95cc3570a3635f15…` | `95cc3570a3635f15…` | **identical** |
| `lexpairs` 1,620 — `CARD.md` | `1ab7ab7512d67590…` | `1ab7ab7512d67590…` | **identical** |
| `lexpairs` 1,080 records, legacy 8-context revision (= committed `data/lexpairs.jsonl`) | `f28bf5e26ff92012…` | `f28bf5e26ff92012…` | **identical** |
| `lexpairs` 1,080 legacy — `CARD.md` | `5ff6321e3013c913…` | `5ff6321e3013c913…` | **identical** |
| `product-pairs` 1,989 items (13,388 carriers) | `f4a8b93c5c5a00f0…` | `f4a8b93c5c5a00f0…` | **identical** |

Full hashes: `ae10a3859f167fc65510451f54b7d69966d7f83779ce14e742b147a8d5db003e`,
`95cc3570a3635f15db1ff16d5ecc648c8b28ab5c0c97a6c465727820c7d7b974`,
`f28bf5e26ff92012919f9136e46283a3b3bcd5a4617278b2fad2040b6ad865ae`,
`f4a8b93c5c5a00f062e7fab4f73eba3ac84e17739d85b238fc15292657b8276c`.
The goldens are in `crates/vaugelas-dataprep/tests/parity/`; `cargo test -p vaugelas-dataprep` checks them, and
`tools/parity.py --python-dir <archive>` re-derives them from the archived Python by running both generators.

**The 1,080 dataset in `data/lexpairs.jsonl` is a stale Python output.** It was generated at 11:56 by the pre-12:55
revision of `lexpairs.py` (8 context templates, git `3693633`); the archived 213-line script has 16 templates and
therefore produces a *different* 1,080-record file (`ae10…`). Both are reproduced byte-for-byte from Rust
(`--contexts 8` selects the legacy table). Nothing is silently substituted: the old file is kept, and the parity table
names which revision each hash belongs to.

## Candidate records (`pairs`)

60 items from the product batch (119 records: 60 good, 59 bad — `prod-0005` is dropped for a token-length mismatch,
as Python warns) through `ebm_data2.py pairs` on the ThinkPad with the v14 flag set the Rust path implements
(`FRC_LEMMA=1 FRC_TRIG=1 FRC_SENT=1 FRC_GEN3=1 FRC_GEN4=1 FRC_LEXHOM=… FRC_LEXHOM_GROUP=1 FRC_LEXHOM_NOSAME=1
FRC_LMFEAT=1 FRC_MAXF=16 FRC_CW_PAIRS=<data-3> FRC_IGNORE=SPELL,TYPO,SPACE,NBSP,DUP,DOUBLE`), same `lexicon.tsv`
(sha256 `4cbad5ee…`), same `interp/` + `ngram/` tables, same N1 model header.

| result | records | cause |
|---|---|---|
| identical | **113 / 119** | — |
| `frc=FR_INVENTED` atom only | 4 | the Rust checker's PR #21 rule flags *mejbour*, *amoris*; the Nov-8 `frc` binary the reference used has no such rule |
| `|capr=` atom only | 2 | research `ebm_data2.lm_atoms` adds the capitalisation ratio after any sentence-opening punctuation; the v14 path (and this port) suppresses it at a sentence start (`midsent`) |
| anything else | **0** | — |

The test (`crates/vaugelas-dataprep/tests/pairs_golden.rs`) fails on a third cause. Readers should treat the two
named differences as version skew, not equivalence defects: both make the Rust record *closer* to what N1 was
trained and scored with (the v14 path), while the Python reference is the older research script.

### Lexical confusions (`FRC_LEXPAIRS_CONF`)

The archived product batch injected its lexical slots with `FRC_LEXPAIRS_CONF`, a Python-only hook; the Rust `Gen`
did not read it. It is ported now (`--lexconf FILE`, format `[[wrong, right, n], ...]`, `n` ignored, rank 8 and exempt
from the "plausible" filters, as in `ebm_cands2._lexconf`). The original file was a temporary artifact and no longer
exists; it was re-derived from batch 1 itself (the unique changed-token pairs, **143**, the number the dataset card
records) and committed as `tests/parity/lexconf-batch1.json` for the test.

| 60-item sample | gold visible at the error |
|---|---|
| Rust v14, no conf | 1 / 59 |
| Rust v14 + conf | **45 / 59 (76.3%)** |
| archived pre-GEN3 Python + same conf | 50 / 59 (84.7%; the full historical batch was 82.8%) |

The 119 records have the same `words`, `err` and `src`; the five-record gap is the v14 generator's longer lists
(10.0 candidates per set vs 8.8) pushing rank-8 confusions past `MAXC = 16`. That is a ranking-vs-recall difference of
the newer generator, named here rather than tuned away: raising the rank would diverge from the Python algorithm and
from what the shipped model reads.

The **archived 1,962 product-passage records** (built 14:17 with `FRC_CANDS2=1` + a 143-pair
`FRC_LEXPAIRS_CONF` resource) are **not** reproduced: the Rust `Gen` is the v14 GEN3/GEN4 generator, the historical
build used the pre-GEN3 v2 generator, and the `FRC_LEXPAIRS_CONF` file was a temporary artifact (not in either
repository). Records made by the Rust CLI from the same 1,989 items are therefore *regenerated* records of the same
sentences, not a copy of that file; this is stated here rather than papered over. Reproducing that exact file would
require porting the pre-GEN3 generator branch and re-deriving the confusions, which the owner did not ask for and the
current inference path does not use.

## Speed

Measured, wall clock, same machines:

| step | Python | Rust | speedup |
|---|---|---|---|
| `lexpairs` 1,080 | 0.07 s | 0.02 s | 3.5× |
| `lexpairs` 1,620 | 0.08 s | 0.02 s | 4× |
| `product-pairs` 1,989 (4-core ThinkPad) | 0.40 s | 0.11 s | 3.6× |
| `pairs` 3,951 records — build phase | 11 s (prior card, same batch) | 0.34 s | 32× |
| `pairs` 3,951 records — end to end (incl. data load) | 11 s | 1.6 s | 6.9× |

Rayon scaling of the record path (39,510 records, 16-core laptop, `--jobs N`; output sha identical at every N):

| jobs | 1 | 2 | 4 | 8 | 16 |
|---|---|---|---|---|---|
| build phase | 36.95 s | 17.35 s | 9.22 s | 4.73 s | 3.56 s |
| end-to-end wall | 45.0 s | 25.1 s | 14.5 s | 6.8 s | 5.9 s |

10.4× on the build phase at 16 threads; the end-to-end figure is bounded by loading the lexicon tables (~1.2 s).

## Reproduce

```
make dataprep                      # build the CLI
make parity                        # lexpairs + pairs (needs VAUGELAS_DATA)
PARITY_CARRIERS=/path/books_dev_texts.jsonl make parity    # adds product-pairs
make parity PYTHON=/path/archive/python-2026-11-12        # adds live Python re-derivation
```

`cargo test -p vaugelas-dataprep` runs the same checks without the corpus (the 1,989 batch test skips with a
message unless `PARITY_CARRIERS` is set).
