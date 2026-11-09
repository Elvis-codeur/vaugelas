# Rights checklist (for the owner)

Open questions to settle **before the repository becomes public or the data files go beyond a few friends**. No
licence has been chosen: that is the owner's decision ([LICENSE-TBD.md](LICENSE-TBD.md) has the facts found and the
options). Nothing here is legal advice. Tick an item when it is answered and write the answer next to it.

## Data and model

- [ ] **Lexicon (Grammalecte dictionary).** Confirm on grammalecte.net or the dictionary's own README that the French
      dictionary is MPL-2.0, and whether it is marked "Incompatible With Secondary Licenses".
      *Touches:* `lexicon.tsv`, `lex/`, the cache, the model's vocabulary (lemma atoms), the homophone lists in
      `resources/`. *Then:* add the dictionary's licence text to the data release and keep the credit in NOTICE.md.
- [ ] **DeepSeek terms.** The model was trained partly on sentences written by DeepSeek, and
      `tests/fixtures/sample.txt` is DeepSeek-written. Check that DeepSeek's terms allow publishing those outputs and
      a model trained on them.
- [ ] **Anthropic (Claude) terms.** The model was trained partly on examples written by Claude (cw, cw2);
      `resources/interp/cw_pairs.json` and `tests/fixtures/sentences.tsv` were written by Claude. Check Anthropic's
      terms for publishing outputs and models trained on them.
- [ ] **Wikipedia (CC-BY-SA).** The "natural" training stream holds French Wikipedia sentences. Decide whether
      share-alike and attribution apply to the model weights, and how to credit them.
- [ ] **French GEC set** (akufeldt/fr-gec-dataset, from juancavallotti/multilingual-gec), also in the natural
      stream: no licence stated on the split. Check, or ask the authors.
- [ ] **Project Gutenberg books.** Public domain in the USA; the model and the word counts in `resources/` are
      derived from them, which the Gutenberg licence header does not restrict (it applies to redistributing the
      texts). Check author death dates if the model is published for other jurisdictions.
- [ ] **UD treebanks.** Not used by H1 as far as the training records show (the research repository lists H1's data
      lineage); UD French ParTUT is non-commercial and excluded everywhere. Confirm before publishing.
- [ ] **Model weights licence.** Choose only after the five items above: the weights inherit the strictest of them.

## Code

- [ ] **Rule checker origin.** Were any rules translated from Grammalecte's GPL rule files (`rules.grx`)? If yes, the
      rule checker is likely GPL-derived (LICENSE-TBD option A); if not, options B, C, D are open.
- [ ] **Code licence**: A / B / C / D in LICENSE-TBD.md. The Rust dependencies are all permissive (NOTICE.md).

## Sharing decisions (not rights, but decide before sharing widely)

- [ ] `tests/fixtures/sample.txt` holds sentences of the held-out test set final-2. Keep it in a repository shared
      with others, or replace it with other sentences?
- [ ] Name "Vaugelas": no trademark search has been done.
