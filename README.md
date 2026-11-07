# Vaugelas

*Correcteur de grammaire française · French grammar checker*

[Français](#français) · [English](#english)

---

## Français

### Qu'est-ce que c'est ?

Vaugelas relit un texte français et signale les fautes de grammaire et d'orthographe : accords (sujet-verbe,
participes passés, adjectifs), homophones (*a / à*, *on / ont*, *ces / ses*…), modes (subjonctif, conditionnel),
mots inconnus et typographie. Pour chaque faute, il propose une correction quand il en a une.

Il combine deux correcteurs et donne **un seul rapport, une entrée par mot** :

- **des règles** écrites à la main, très rapides (moins d'un dixième de milliseconde par phrase) et précises ;
- **un modèle appris** (un réseau de neurones « à énergie ») : il compare chaque mot aux autres formes du même mot
  (*annulés / annulées / annulé…*) et signale celui qui ne va pas dans la phrase, avec la forme la plus plausible.

Tout fonctionne **sur votre ordinateur**, sans connexion et sans envoyer votre texte ailleurs, une fois les
données téléchargées.

Ce qu'il sait faire et ne sait pas faire, avec des exemples : [docs/capabilities.md](docs/capabilities.md).

### Télécharger

1. Prenez le fichier de votre système sur la page des versions (*Releases*) du projet :
   - Windows : `vaugelas-…-windows-x86_64.zip`
   - macOS (puce Apple M1/M2/M3…) : `vaugelas-…-macos-arm64.tar.gz` ; macOS (Intel) : `…-macos-x86_64.tar.gz`
   - Linux : `vaugelas-…-linux-x86_64.tar.gz`
2. Décompressez-le. Il contient un seul programme, `vaugelas` (`vaugelas.exe` sous Windows).
3. Au premier lancement, Vaugelas télécharge ses données (environ 64 Mo : le modèle, 61 Mo, et le lexique, 3 Mo),
   vérifie leur empreinte (sha256) et les range dans votre dossier utilisateur. Vous pouvez aussi le faire à
   l'avance avec `vaugelas download-model`.

Les données occupent environ 165 Mo sur le disque : modèle 66 Mo, lexique 20 Mo, lexique compilé des règles
7 Mo et tables du modèle 72 Mo (créées au premier démarrage, une seule fois). Elles sont rangées dans :
Windows `%LOCALAPPDATA%\vaugelas`, macOS `~/Library/Application Support/vaugelas`, Linux `~/.local/share/vaugelas`.
Pour un autre dossier : option `--data DOSSIER` ou variable `VAUGELAS_DATA`.

### Utiliser

Ouvrez un terminal (Windows : *Invite de commandes* ou *PowerShell*) dans le dossier du programme :

```
vaugelas check "Les élections ont été annulés par le juge."
```
```
ligne 1, colonne 23 : « annulés »  →  annulées
    L’attribut doit s’accorder avec le sujet.  [FR_ATTR_AGREE, rules+model, score 1.000]
```

- Vérifier un fichier texte : `vaugelas check -f mon_texte.txt`
- Résultat en JSON (pour d'autres programmes) : `vaugelas check --json "…"`
- Moins d'alertes (le modèle ne signale que ce dont il est très sûr) : `--threshold 0.99`. Plus d'alertes :
  `--threshold 0.9`. La valeur par défaut, 0.9571, donne environ 1,5 % de fausses alertes sur nos phrases de test.
- Règles seulement (très rapide) : `--rules-only` ; modèle seulement : `--model-only`.
- Conventions typographiques (espaces insécables, `…`, `’`) : `--strict-typo`.
- Aide : `vaugelas --help`.

### Dans votre éditeur (serveur compatible LanguageTool)

Vaugelas peut tourner en arrière-plan comme un serveur **LanguageTool**. Les extensions et modules qui savent
parler à un serveur LanguageTool fonctionnent alors avec lui : LTeX (VS Code, Neovim…), l'extension LanguageTool
des navigateurs, des modules pour Obsidian, LibreOffice, Emacs, etc.

```
vaugelas serve --port 8081
```

Dans les réglages de l'extension, choisissez un serveur « local » ou « personnalisé » et donnez l'adresse
`http://localhost:8081` (selon l'extension : `http://localhost:8081/v2`). Exemple pour LTeX dans VS Code :
`"ltex.languageToolHttpServerUri": "http://localhost:8081"` et `"ltex.language": "fr"`.

Le serveur charge le modèle et le lexique une seule fois au démarrage (1 à 2 secondes), puis répond à chaque
requête. Il n'écoute que votre ordinateur (`127.0.0.1`), sauf avec `--host 0.0.0.0`.

Ce qui est pris en charge : `POST /v2/check` (champs `language`, `text` ou `data` annoté, `disabledRules`,
`enabledRules` + `enabledOnly`, `disabledCategories`), `GET /v2/languages`. Positions en unités UTF-16 comme
LanguageTool. Seul le français est accepté (`fr`, `fr-FR`, `fr-BE`, `fr-CA`, `fr-CH`, `auto`).

### Vitesse et mémoire

Mesuré sur un portable de 2013 (Intel i5-4300M, 2 cœurs / 4 fils, 7 Go de RAM), une machine qui faisait autre
chose en même temps (charge 5 à 7) : les chiffres sur une machine récente seront meilleurs.

| | par phrase (médiane) | 4 fils | mémoire |
|---|---:|---:|---:|
| règles seules | 0,07 ms | — | ~50 Mo |
| modèle + règles, depuis le texte | 12,4 ms | ~100 phrases/s | ~540 Mo au démarrage, ~670 Mo en pointe |
| idem, modèle int8 (`--int8`) | 14,1 ms | ~74 phrases/s | ~20 Mo de moins |

Le programme fait environ 5 Mo. Le serveur occupe environ 490 Mo de mémoire. Démarrage : environ 1 s, sauf la toute première fois (environ 6 s, le temps de créer les tables du modèle).

Qualité, sur les phrases de test « final-2 » (jamais vues pendant l'entraînement), avec erreurs difficiles : les
règles seules trouvent 49,2 % des fautes (1,6 % de fausses alertes), règles + modèle **67,5 %** (3,1 %), contre
61,9 % (6,1 %) pour règles + LanguageTool. Sur 20 000 phrases correctes de livres, le modèle signale 9,6 phrases
sur 1 000. Attention : ces phrases de test ont été construites en partie à partir des fautes que les règles
manquent, ce qui avantage un modèle qui les complète.

### Licence

Pas encore choisie : voir [LICENSE-TBD.md](LICENSE-TBD.md) et [NOTICE.md](NOTICE.md). Le lexique vient du
dictionnaire de Grammalecte (Olivier R.) et a sa propre licence.

---

## English

### What it is

Vaugelas proofreads French text and points out grammar and spelling mistakes: agreement (subject-verb, past
participles, adjectives), homophones (*a / à*, *on / ont*…), moods (subjunctive, conditional), unknown words and
typography, with a correction when it has one.

It combines two checkers into **one report, one entry per word**:
- **hand-written rules**, very fast (under 0.1 ms per sentence) and precise;
- **a learned model** (a paradigm-energy neural network) that compares each word with the other forms of the same
  word and flags the one that does not fit, together with the most plausible form.

Everything runs **on your computer**, offline once the data is downloaded. Your text is never sent anywhere.
Strengths and weaknesses, with examples: [docs/capabilities.md](docs/capabilities.md).

### Download

1. From the project's *Releases* page, take the file for your system: `…-windows-x86_64.zip`,
   `…-macos-arm64.tar.gz` (Apple silicon), `…-macos-x86_64.tar.gz` (Intel Mac), `…-linux-x86_64.tar.gz`.
2. Unpack it. It holds one program, `vaugelas` (`vaugelas.exe` on Windows).
3. On first run Vaugelas downloads its data (about 64 MB: model 61 MB, lexicon 3 MB, compressed), checks its sha256 and
   stores it in your user data folder. You can do this in advance with `vaugelas download-model`. The data uses
   about 165 MB on disk. Use `--data DIR` or `VAUGELAS_DATA` to put it somewhere else.

### Run

```
vaugelas check "Les élections ont été annulés par le juge."
vaugelas check -f my_text.txt
vaugelas check --json "…"              # machine-readable output
vaugelas check --threshold 0.99 "…"    # fewer, surer model flags (default 0.9571 ≈ 1.5% false alarms)
vaugelas check --rules-only "…"        # rules only (very fast); --model-only for the model only
vaugelas --help
```

### Use it from an editor (LanguageTool-compatible server)

```
vaugelas serve --port 8081
```
Point any LanguageTool client at `http://localhost:8081`: LTeX (VS Code, Neovim…), the LanguageTool browser
extension, plug-ins for Obsidian, LibreOffice and Emacs. Some clients want the `/v2` suffix. Example for LTeX:
`"ltex.languageToolHttpServerUri": "http://localhost:8081"`.
Supported: `POST /v2/check` (`language`, `text` or annotated `data`, `disabledRules`, `enabledRules` +
`enabledOnly`, `disabledCategories`) and `GET /v2/languages`. Offsets are in UTF-16 code units, as in LanguageTool.
French only. Each match also carries `"vaugelas": {"source": "rules" | "model" | "rules+model", "score": …}`.
Model and lexicon are loaded once at start. The server binds to `127.0.0.1` unless `--host` is given.

The server uses [tiny_http](https://crates.io/crates/tiny_http): a small, mature, synchronous HTTP server with
no async runtime. That is enough for a local service answering one editor, and it keeps the binary small and the
build simple on all four platforms.

### Speed and memory

2013 laptop (i5-4300M, 2 cores / 4 threads, 7 GB), never idle during the measurements (load average 5-7):

| | per sentence (p50) | 4 threads | memory |
|---|---:|---:|---:|
| rules only | 0.07 ms | — | ~50 MB |
| model + rules, from text | 12.4 ms | ~100 sentences/s | ~540 MB after start, ~670 MB peak |
| same, int8 model (`--int8`) | 14.1 ms | ~74 sentences/s | ~20 MB less |

The program is about 5 MB. The server uses about 490 MB of memory and answers a one-sentence request in about 22 ms. Startup is about 1 s. The very first start takes about 6 s while the model's lexicon tables are built and cached.
Quality on the held-out final-2 set: rules alone 49.2% detection at 1.6% false alarms, rules + model **67.5%** at 3.1%,
rules + LanguageTool 61.9% at 6.1%. On clean book text the model flags 9.6 sentences per 1,000. Caveat:
final-2 was built partly from errors the rules miss, which favours a model that complements them.

### For developers

```
cargo build --release            # binary: target/release/vaugelas
cargo test --release             # data-dependent tests skip when the data is not installed
vaugelas download-model --from DIR   # install from local files (model.frcm, lexicon.tsv, or their .gz)
```
- `crates/vaugelas`: the library (`rules`: fr-check rule checker; `model`: energy-model inference;
  `text`: characterizer and lexicon tables; `report`: the merged report).
- `crates/vaugelas-cli`: the `vaugelas` program (CLI, data download, LanguageTool-compatible server).
- Parity with the research code (2026-11-07):
  `tests/fixtures/`: 1,006 sentences, rule lints identical to `frc` and the 123 model flags identical to
  `frc-infer`, 0 lines differ. `tools/parity.sh` checks per-word scores on final-2 (5,105 records, 57,854
  words) and tune (9,366 records, 117,960 words): every line is byte-identical to the verified reference (max
  difference 0.0), and the records match the Python `ebm_data.py` rows (0 mismatches).
- **Data release.** The data is not in git. Publish it once as release `data-1` with `model.frcm.gz` and
  `lexicon.tsv.gz` (`gzip -9k`, then `gh release create data-1 model.frcm.gz lexicon.tsv.gz`). The sha256
  values of the *uncompressed* files are in `crates/vaugelas-cli/src/data.rs`. `VAUGELAS_DATA_URL` points the
  download at another location. While the repository is private, anonymous downloads of release assets fail:
  use `--from DIR` instead.
- Binary releases: push a tag `vX.Y.Z`. `.github/workflows/release.yml` builds Linux x86_64, Windows x86_64,
  macOS x86_64 and arm64 into a draft release.

### Licence

Not chosen yet: see [LICENSE-TBD.md](LICENSE-TBD.md) and [NOTICE.md](NOTICE.md). The lexicon comes from the
Grammalecte dictionary (Olivier R.) and carries its own licence.
