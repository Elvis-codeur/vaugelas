# Data licences / Licences des données

The program's code is under the MIT licence ([LICENSE](LICENSE)). The data files it downloads are separate works with
their own licences (decided by the author on 2026-11-10):

| file | licence | text |
|---|---|---|
| `model.frcm` (energy model N1) | **CC BY-SA 4.0** | [LICENSES/CC-BY-SA-4.0.txt](LICENSES/CC-BY-SA-4.0.txt) |
| `resources.tar.gz` (`resources/`: counts, homophone lists, confusion pairs) | **CC BY-SA 4.0** | [LICENSES/CC-BY-SA-4.0.txt](LICENSES/CC-BY-SA-4.0.txt) |
| `lexicon.tsv` and the files built from it (`lex/`, `lexicon.tsv.lem.frclex`) | **MPL-2.0**, as received | [LICENSES/MPL-2.0.txt](LICENSES/MPL-2.0.txt) |

Le code est sous licence MIT. Le modèle et l'archive de ressources sont sous **CC BY-SA 4.0** : vous pouvez les
utiliser, les modifier et les redistribuer, même commercialement, en citant les sources ci-dessous et en publiant
vos versions modifiées sous la même licence. Le lexique garde sa licence d'origine, **MPL-2.0**.

## Attribution (CC BY-SA 4.0: model and resources)

Vaugelas model N1 and resources (data set `data-3`), © 2026 Elvis-codeur, licensed CC BY-SA 4.0. Made with:

- **French Wikipedia** — text by Wikipedia contributors, CC BY-SA 4.0 (https://fr.wikipedia.org); training sentences.
- **Lexique 3.83** — B. New, C. Pallier et al., http://www.lexique.org, CC BY-SA 4.0; homophone lists
  (`interp/homophones_lex_1.0*.json`, `interp/nearhom.json`) and the model's pronunciation atoms (`snd=`).
- **Grammalecte French lexicon** — "Lexique des formes fléchies du français", Grammalecte v7.7, Olivier R. /
  Dicollecte, MPL-2.0 (https://grammalecte.net); word readings and tags, and the lemma strings in the model's
  vocabulary (`lem=`). Those strings remain available under MPL-2.0.
- **multilingual-gec** — J. Cavallotti, Apache-2.0 (https://huggingface.co/datasets/juancavallotti/multilingual-gec),
  through its French subset akufeldt/fr-gec-dataset; training sentences.
- **Project Gutenberg** — public-domain French books (https://www.gutenberg.org); training sentences and word counts.
- Training sentences and confusion examples written with **DeepSeek** (terms allow training on outputs); the confusion
  partners in `interp/cw_pairs.json` come from them. No training data or resource was written by Claude.

Changed versions of the model or of the resources must be shared under CC BY-SA 4.0 and keep this attribution.
The full record of how these licences were chosen: [docs/licence-research.md](docs/licence-research.md) and
[RIGHTS-CHECKLIST.md](RIGHTS-CHECKLIST.md).
