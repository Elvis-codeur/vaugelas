# Rights checklist (for the owner)

Open questions to settle **before the repository becomes public or the data files go beyond a few friends**. No
licence has been chosen: that is the owner's decision ([LICENSE-TBD.md](LICENSE-TBD.md) has the facts found and the
options). Nothing here is legal advice. Tick an item when it is answered and write the answer next to it.

## Data and model

Research of 2026-11-10 (primary texts; not legal advice): see the findings below each item.

- [x] **Lexicon (Grammalecte dictionary): MPL-2.0, confirmed.** The author publishes the same lexicon separately as
      `lexique-grammalecte-fr-v7.7.zip` (grammalecte.net, file `dic/`), built 2025-12-15 15:47, one minute before
      the `fr-allvars` dictionary we dumped (15:48:13); its header says "Licence : MPL v2.0" with the MPL notice, and
      no "Incompatible With Secondary Licenses" mark. Debian's `hunspell-fr` copyright file also lists the Dicollecte
      dictionaries as MPL-2.0. (The Grammalecte *program* package says GPL 3 for itself.) *Then:* ship the MPL-2.0
      text with the data release; credit "Lexique des formes fléchies du français, Grammalecte v7.7" in NOTICE.md.
- [x] **DeepSeek terms: allowed.** DeepSeek Terms of Use 4.2(3) and Open Platform Terms: "You may apply the Inputs
      and Outputs of the Services to a wide range of use cases, including personal use, academic research, derivative
      product development, training other models (such as model distillation), etc." Outputs are assigned to the user
      (4.2(2)). (The owner also confirmed from his own use.)
- [ ] **Anthropic (Claude) terms: open, owner decision.** The Claude-written data (cw, cw2, `cw_pairs.json`,
      the test fixtures) was written in Claude Code under the owner's Claude Max subscription, so the **Consumer
      Terms** apply (effective 2025-10-08). Outputs: "we assign to you all our right, title, and interest (if any) in
      Outputs", so publishing the fixtures and `cw_pairs.json` is fine. Training: section 3 forbids using the Services
      "To develop any products or services that compete with our Services, including to develop or train any
      artificial intelligence or machine learning algorithms or models". Read narrowly (training *as part of* a
      competing product) a French grammar checker is not a competitor of Claude; read broadly it forbids training
      any model. (The Commercial Terms, for API customers, say "to train competing AI models", which is narrower.)
      *Options:* ask Anthropic in writing before publishing the model; or, for the public model, retrain without the
      Claude-written data (cw, cw2) and drop `cw_pairs.json` from the resources.
- [ ] **Wikipedia (CC BY-SA 4.0): open, owner decision.** Creative Commons' guidance on AI training: "CC licenses apply
      only when copyright permission is required. If exceptions or limitations apply, then the CC license terms don't
      apply" (the EU text-and-data-mining exception, art. 4 of directive 2019/790, may apply), but also: if models
      "are based on ShareAlike content and they will be shared publicly", BY-SA "require[s] that adaptations be shared
      under the same license"; attribution "could be a simple link to the source of the dataset". Whether weights are
      an adaptation is legally unsettled. *Safe choice:* publish the model weights under CC BY-SA 4.0 with attribution
      to French Wikipedia; this satisfies share-alike in every reading and keeps the model fully open.
- [ ] **Lexique 3.83: CC BY-SA 4.0 (found 2026-11-10, missing from earlier records).** lexique.org: "Lexique est
      distribuée sous une licence Creative Commons Attribution – Partage dans les mêmes conditions 4.0". The homophone
      lists in `resources/` (`interp/homophones_lex_1.0*.json`, `interp/nearhom.json`) are made from it, and the
      model's `snd=` pronunciation atoms come from it. *Then:* those resource files under CC BY-SA 4.0 with credit;
      one more reason for CC BY-SA 4.0 on the model.
- [x] **French GEC set: Apache-2.0 upstream.** `juancavallotti/multilingual-gec` (Hugging Face) is Apache-2.0;
      `akufeldt/fr-gec-dataset` is its French subset and states no licence of its own. Credit both in NOTICE.md.
- [ ] **Project Gutenberg books.** Public domain in the USA; the model and the word counts in `resources/` are
      derived from them, which the Gutenberg licence header does not restrict (it applies to redistributing the
      texts). Check author death dates if the model is published for other jurisdictions.
- [ ] **UD treebanks.** Not used by H1 as far as the training records show (the research repository lists H1's data
      lineage); UD French ParTUT is non-commercial and excluded everywhere. Confirm before publishing.
- [ ] **Model weights licence.** Recommendation: CC BY-SA 4.0 (Wikipedia and Lexique share-alike), with a
      note that the vocabulary's lemma strings come from the MPL-2.0 lexicon. Resources archive: CC BY-SA 4.0 (the
      Lexique-derived lists must be), the lexicon itself stays MPL-2.0. Decide after the Anthropic item.

## Code

- [x] **Rule checker origin: no Grammalecte rule was copied (checked 2026-11-10).** Compared with Grammalecte 2.3's
      rules (`gc_rules.py`, `gc_rules_graph.py`, `gc_functions.py`, 4.3 MB): none of our 479 French message strings
      appears in it (8 share only stock grammar phrases such as "s'accorde avec le sujet"); our 36 word lists share
      no run of more than 8 consecutive words, and those runs are natural orders (days of the week, "ne me te se");
      no Grammalecte module or data (`phonet_data`, `mfsp_data`, `conj_data`) is used by the app or its data
      build; the fr-check history names Grammalecte only as a benchmark. Grammalecte code is used only by tools that
      do not ship: `tools/dump_lexicon.py` (its MPL-2.0 dictionary reader) and `eval/run_tools.py` (baseline runs).
      The MIT licence holds.
- [x] **Code licence**: MIT (owner, 2026-11-10). The Rust dependencies are all permissive (NOTICE.md).

## Sharing decisions (not rights, but decide before sharing widely)

- [x] `tests/fixtures/sample.txt` held sentences of the held-out test set final-2. Replaced on 2026-11-09 by 1,006
      sentences written by Claude (issue #3); the rule checker is still byte-identical to `frc` on them.
- [ ] The old `sample.txt` and `sample.frc.jsonl` are still in git history (commit db7ff56, on `main` and on GitHub).
      Keep that history, or rewrite it before sharing the repository?
- [ ] Name "Vaugelas": no trademark search has been done.
