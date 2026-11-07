# Licence: to be decided by the owner

No licence has been chosen for Vaugelas yet. Until one is, all rights are reserved by the author and the repository
stays private. This file lists what was found about the origin of each part and the options that fit those
constraints. Nothing here is legal advice. The facts were checked against local copies on 2026-11-07, and
the points marked **to verify** still need checking against upstream.

## What is in a release, and where it comes from

| part | in git? | origin | licence found |
|---|---|---|---|
| Rule checker code (`crates/vaugelas/src/rules/`) | yes | fr-check (owner's own code, Rust). Its README says: "The lexicon and rule knowledge come from Grammalecte (code GPLv3 ...). The architecture is modelled on Harper (Apache-2.0) and on LanguageTool (LGPL-2.1)." No source file names Grammalecte, Harper or LanguageTool, and no copied code was found by searching the source. | none chosen. **To verify:** were any rules translated from Grammalecte's GPL rule files (`gc_lang/fr/rules.grx`)? If so, that code may be a derivative work of GPL-3.0 code. |
| Model inference and text analysis code (`src/model/`, `src/text/`) | yes | owner's own code: Rust ports of the owner's Python training code (training-design/*.py) | none chosen |
| `lexicon.tsv` (20 MB, download) | **no** | Dump of Grammalecte 2.3.0's compiled dictionary `grammalecte/graphspell/_dictionaries/fr-allvars.json` ("Français (Toutes variantes)", built 2025-12-15 15:48:13, 515,637 entries), made with fr-check's `tools/dump_lexicon.py` (`IBDAWG(dic).select()` → `form<TAB>lemma<TAB>tags`). Local copy of the source: `repos/grammalecte-cli/src/`. | The Grammalecte package README says "License: GPL 3". The graphspell engine file `dawg.py` says "License: MPL 2". The French dictionary itself (Dicollecte project, Olivier R.) is, to my knowledge, published under **MPL-2.0** on grammalecte.net. **To verify on grammalecte.net / the dictionary's own README:** the licence of the dictionary data, and whether it is "Incompatible With Secondary Licenses". |
| FST lexicon `lex/` (words.fst, recs.bin, gen.fst, lemmas.bin, tags.txt, 7 MB) | **no**, built on the user's machine from lexicon.tsv | the same data as lexicon.tsv, in another form | same as lexicon.tsv (a derived form of the dictionary) |
| `lexicon.tsv.lem.frclex` (72 MB cache) | **no**, built at first start | lexicon.tsv plus derived tables | same as lexicon.tsv |
| `model.frcm` (66 MB, download) | **no** | Paradigm-energy model `ebm_ds` (16.4M parameters), trained by the owner. Training data: (1) public-domain French books from Project Gutenberg (authors born 1760-1900; `training-design/fetch_books.py`); (2) sentences made by the owner's generators; (3) training sentences written by DeepSeek (LLM API); (4) readings and tags from the Grammalecte lexicon. UD French treebanks were used by earlier models but not by this one (morning report 2026-11-06, section 5.4). | none chosen. The vocabulary inside the file holds lexicon-derived atoms (`lem=<lemma>`, tag atoms), so the file contains part of the dictionary. **To verify:** whether DeepSeek's terms of service allow publishing a model trained on its outputs. Project Gutenberg texts are public domain in the US. Their trademark/licence header applies to redistributing the texts, not to a model trained on them. |
| Test fixture `tests/fixtures/sample.txt` | yes | 1,006 sentences: final-2 and tuning rounds (written by DeepSeek for the hardloop evaluation) plus 13 written by hand | same DeepSeek question as above |
| Rust dependencies (compiled into the binary) | via Cargo.lock | crates.io | permissive only, listed in NOTICE.md (MIT, Apache-2.0, ISC, Zlib, Unicode-3.0, BSD-3-Clause, CDLA-Permissive-2.0 for the TLS root certificates, Unlicense, 0BSD). Dual-licensed crates can be used under MIT or Apache-2.0. None is GPL. |

## Constraints that follow

1. **Data and code can have different licences.** The data files are not in git and the binary does not embed them.
   They are separate downloads, so the code licence does not have to match the dictionary licence, as long as
   the code is not a derivative of GPL code (point 2).
2. **If rule code was translated from Grammalecte's GPL rules**, the rule checker is likely a derivative work and has to
   be GPL-3.0-or-later (or GPL-3.0-compatible, e.g. AGPL-3.0). If the rules were written independently, using
   Grammalecte only as a *source of facts* (which errors exist), the code is free of that constraint.
3. **The dictionary (MPL-2.0, to verify)** can be redistributed under MPL-2.0, which is file-level copyleft. Copies
   of `lexicon.tsv` and of the files derived from it (FST lexicon, cache, and arguably the model's vocabulary) have
   to stay under MPL-2.0 and keep its notice. MPL-2.0 can be combined with GPL code (section 3.3) unless the
   dictionary is marked "Incompatible With Secondary Licenses".
4. **The model** carries the dictionary's terms for the vocabulary it contains, plus whatever DeepSeek's terms say
   about models trained on its outputs.
5. **Name.** "Vaugelas" is the name of the 17th-century grammarian Claude Favre de Vaugelas (public domain). A
   trademark search has not been done.

## Options

| option | code | data (lexicon, FST, cache) | model | when it fits |
|---|---|---|---|---|
| A. All GPL | GPL-3.0-or-later | MPL-2.0 (as received) | GPL-3.0-or-later or MPL-2.0 | the safe choice if any rule code came from Grammalecte. Same family as Grammalecte. Bars closed-source embedding. |
| B. MPL everywhere | MPL-2.0 | MPL-2.0 | MPL-2.0 | one licence for all three. File-level copyleft: changes to these files stay open, while embedding in other software (including proprietary) is allowed. Only possible if the code is not GPL-derived. |
| C. Permissive code | MIT OR Apache-2.0 (Rust convention) | MPL-2.0, separate download | MPL-2.0, or CC-BY-4.0 for the weights if the dictionary licence allows | widest reuse of the code (editor plug-ins, other checkers). Only possible if the code is not GPL-derived. |
| D. LGPL code | LGPL-2.1-or-later (like LanguageTool) | MPL-2.0 | MPL-2.0 | a middle way: library copyleft, linking allowed |

For every option: keep NOTICE.md (credits: Grammalecte/Dicollecte dictionary by Olivier R., Project Gutenberg, the
Rust dependencies), ship the dictionary's licence text with the data release, and say clearly in the README that
the data has its own licence.

## Decisions needed from the owner

1. Was any rule code translated from Grammalecte (this decides between A and B/C/D)?
2. Licence for the code: A / B / C / D.
3. Licence for the model weights, after checking DeepSeek's terms.
4. Confirm the dictionary licence upstream (MPL-2.0?), then add its licence text to the data release.
