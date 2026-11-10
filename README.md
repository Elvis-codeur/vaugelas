# Vaugelas

*Correcteur de grammaire française · French grammar checker*

[Français](#français) · [English](#english)

---

## Français

### Qu'est-ce que c'est ?

Vaugelas relit un texte français et signale les fautes : accords (sujet-verbe, participes passés, adjectifs),
homophones (*a / à*, *son / sont*, *ces / ses*, *vert / verre*…), modes (subjonctif, conditionnel), infinitif ou
participe (*manger / mangé*), majuscules, mots inconnus et typographie. Pour chaque faute, il propose une correction
quand il en a une.

Il réunit deux correcteurs et donne **un seul rapport, une entrée par mot** :

- **des règles** écrites à la main, très rapides et précises ;
- **un modèle appris** (un réseau de neurones « à énergie », version N1, entraînée sans aucune donnée écrite par Claude) : il compare chaque mot aux autres formes
  qu'il pourrait prendre (*annulés / annulées / annulé…*, *vert / verre / vers*) et signale celui qui ne va pas dans la
  phrase, avec la forme la plus plausible. Il lit aussi ce que les règles ont trouvé.

Tout fonctionne **sur votre ordinateur**, sans connexion. Votre texte n'est jamais envoyé ailleurs.

Ce qu'il fait bien et ce qu'il fait mal, avec des exemples : [docs/capabilities.md](docs/capabilities.md).

### Installer

Il vous faut **quatre fichiers**, tous sur la page des versions (*Releases*) du projet :

1. le programme pour votre système :
   - Windows : `vaugelas-v0.2.1-windows-x86_64.zip`
   - Mac avec puce Apple (M1, M2, M3…) : `vaugelas-v0.2.1-macos-arm64.tar.gz` ; Mac Intel : `…-macos-x86_64.tar.gz`
   - Linux : `vaugelas-v0.2.1-linux-x86_64.tar.gz`
2. les trois fichiers de données : `model.frcm.gz` (70 Mo), `lexicon.tsv.gz` (3 Mo) et `resources.tar.gz` (9 Mo).

Décompressez le programme (double-clic sur le `.zip` ou le `.tar.gz`), puis **posez les trois fichiers de données
dans le même dossier que le programme**, sans les décompresser. C'est tout.

Au premier lancement, Vaugelas trouve ces fichiers à côté de lui, vérifie leur empreinte (sha256) et les installe
dans votre dossier utilisateur (une dizaine de secondes). Ensuite vous pouvez effacer les trois fichiers téléchargés.
Si les fichiers ne sont pas à côté du programme, Vaugelas essaie de les télécharger lui-même ; tant que le projet est
privé ce téléchargement échoue, et le programme vous dit quoi faire.

Les données occupent environ 200 Mo sur le disque, dans :
Windows `%LOCALAPPDATA%\vaugelas`, macOS `~/Library/Application Support/vaugelas`, Linux `~/.local/share/vaugelas`.
Pour un autre dossier : option `--data DOSSIER` ou variable d'environnement `VAUGELAS_DATA`.
`vaugelas info` montre où sont les données et quel modèle est installé.

**Mac :** le programme n'est pas signé par Apple. Si macOS refuse de l'ouvrir, tapez une fois dans le Terminal,
dans le dossier du programme : `xattr -d com.apple.quarantine vaugelas`.
**Windows :** si Windows affiche « Windows a protégé votre ordinateur », cliquez sur *Informations complémentaires*
puis *Exécuter quand même*.

### Utiliser

Vaugelas s'utilise dans un terminal (Windows : ouvrez le dossier du programme, tapez `cmd` dans la barre d'adresse et
Entrée ; Mac : app *Terminal* ; puis `cd` vers le dossier). Sous Windows tapez `vaugelas`, sous Mac et Linux
`./vaugelas`.

```
vaugelas check "Les élections ont été annulés par le juge."
```
```
ligne 1, colonne 23 : « annulés »  →  annulées
    L’attribut doit s’accorder avec le sujet.  [FR_ATTR_AGREE, rules+model, score 1.000]
```

- Vérifier un fichier texte : `vaugelas check -f mon_texte.txt` (texte brut en UTF-8).
- Résultat en JSON (pour d'autres programmes) : `vaugelas check --json "…"`.
- **Plus ou moins d'alertes.** Le modèle signale un mot quand son score dépasse un seuil, 0.9935 par défaut (choisi
  pour que le modèle signale environ 3,5 phrases sur 1 000 phrases correctes de livres). `--threshold 0.9` : il signale davantage
  de fautes, mais aussi davantage de phrases correctes ; `--threshold 0.999` : moins d'alertes, plus sûres.
- Règles seulement : `--rules-only` ; modèle seulement : `--model-only`.
- Conventions de la typographie soignée (espaces insécables, `…`, `’`) : `--strict-typo`.
- Aide : `vaugelas --help`.

### Réglages

Tous les réglages peuvent être écrits une fois pour toutes dans un fichier, `settings.toml`. Pour le créer :

```
vaugelas settings --init
```

Le programme indique où il l'a créé (Windows `%APPDATA%\vaugelas`, macOS `~/Library/Application Support/vaugelas`,
Linux `~/.config/vaugelas`). Ouvrez-le avec un éditeur de texte : chaque réglage y est expliqué, en français et en
anglais. On peut aussi le poser à côté du programme. Ce qu'on peut y régler :

- **le seuil du modèle** (`threshold`) : plus bas, Vaugelas trouve plus de fautes mais signale aussi plus de phrases
  correctes. Par exemple, « Il est partit à la gare ce matin. » passe au seuil par défaut (0.9935, score 0.984) et est
  signalée à 0.98 ;
- les règles et le modèle (chacun peut être coupé), les suggestions d'orthographe, la typographie soignée, le modèle
  en int8 (moins de mémoire) ;
- les règles ou catégories à ne jamais signaler (`disabled_rules`, `disabled_categories` ; liste des règles :
  `vaugelas rules`) ;
- le dossier des données, et l'adresse, le port et le nombre de fils du serveur.

`vaugelas settings` montre le fichier utilisé et les réglages en vigueur. Les options de la ligne de commande passent
avant le fichier. Une erreur dans le fichier (nom de réglage mal écrit, valeur hors limites, règle inconnue) arrête le
programme avec un message qui dit laquelle.

### Dans votre éditeur (serveur compatible LanguageTool)

Vaugelas peut tourner en arrière-plan comme un serveur **LanguageTool** local :

```
vaugelas serve --port 8081
```

Laissez cette fenêtre ouverte. Les extensions qui savent parler à un serveur LanguageTool peuvent alors l'utiliser :
dans leurs réglages, choisissez un serveur « local » ou « personnalisé » et donnez l'adresse `http://localhost:8081`
(certaines veulent `http://localhost:8081/v2`). Exemple pour l'extension LTeX de VS Code :
`"ltex.languageToolHttpServerUri": "http://localhost:8081"` et `"ltex.language": "fr"`.
Même principe pour l'extension LanguageTool des navigateurs (réglages avancés, serveur local), le module LanguageTool
de LibreOffice (*Outils › Options › Paramètres linguistiques*, serveur distant), Obsidian ou Emacs.
Le serveur suit l'API publique de LanguageTool (`/v2`) ; il a été testé avec des requêtes directes, pas avec chacune
de ces extensions : si l'une ne marche pas, signalez-le (voir plus bas).

Le serveur charge le modèle une seule fois (environ 1 seconde), puis répond en une quinzaine de millisecondes par
phrase. Il n'écoute que votre ordinateur (`127.0.0.1`), sauf si vous demandez `--host 0.0.0.0`.

Ce qui est pris en charge : `POST /v2/check` (champs `language`, `text` ou `data` annoté, `disabledRules`,
`enabledRules` + `enabledOnly`, `disabledCategories`) et `GET /v2/languages`. Positions en unités UTF-16 comme
LanguageTool. Seul le français est accepté (`fr`, `fr-FR`, `fr-BE`, `fr-CA`, `fr-CH`, `auto`).

### Vie privée

Vaugelas ne se connecte à Internet que pour télécharger ses données, et seulement si elles manquent. Il ne
collecte rien et n'envoie jamais votre texte. Le serveur n'accepte que les connexions venant de votre ordinateur.

### Ce qu'il sait faire, et ses limites

Résumé (détails, chiffres et exemples : [docs/capabilities.md](docs/capabilities.md)) :

- **Bon** sur les accords (sujet-verbe, participe passé, adjectif, attribut), sur l'infinitif / participe
  (*a manger*, *vais acheté*), sur le subjonctif et le conditionnel quand une règle connaît le déclencheur, sur les
  majuscules des jours et des mois, et sur une partie des homophones (*à / a*, *son / sont*, *peu / peut*,
  *vert / verre / vers*).
- **Moyen** sur les homophones : sur un test d'homophones fabriqué de la même façon que ses exemples
  d'entraînement, le modèle en trouve 55 %, mais ce test le favorise (voir la page détaillée) ; il en rate encore
  beaucoup (*ou / où*, *ces / ses*, *mer / mère*).
- **Faible** sur le choix du mot quand il faut comprendre le sens (*conjecture / conjoncture*, *voie / voix*), sur
  les fautes qui demandent d'ajouter ou de retirer un mot, sur la ponctuation et le style.
- Le modèle ne fait que remplacer un mot par une autre forme ; il n'ajoute, ne retire et ne déplace jamais de mot.

Sur le jeu de test difficile du projet (final-2, phrases jamais vues à l'entraînement), le modèle seul trouve
51,4 % des fautes au seuil par défaut, et les règles seules 47,0 % (1,6 % des phrases correctes signalées). Les
deux réunis en trouvent davantage : les règles et le modèle attrapent en partie des fautes différentes. Sur 18 703
phrases correctes de livres, le modèle seul signale environ 3,5 phrases sur 1 000.

### Vitesse et mémoire

Mesuré sur un portable de 2013 (Intel i5-4300M, 2 cœurs, 7 Go de mémoire) ; une machine récente fera mieux.

| | par phrase | mémoire |
|---|---:|---:|
| règles + modèle | environ 12 ms | environ 650 Mo |
| règles seules (`--rules-only`) | moins de 0,1 ms | idem (le modèle est chargé quand même) |

Démarrage : environ 1 seconde ; la toute première fois environ 5 à 10 secondes (installation des données et
préparation des tables). Le serveur occupe environ 620 Mo. Le programme lui-même fait environ 6 Mo.

### Signaler un problème

Ouvrez une *issue* sur la page GitHub du projet (onglet *Issues*, bouton *New issue*) avec :
la phrase exacte, ce que Vaugelas a répondu, ce que vous attendiez, et le résultat de `vaugelas --version` et de
`vaugelas info`. Une fausse alerte sur une phrase correcte est aussi utile qu'une faute manquée.

### Licence

Le programme (le code) est sous licence **MIT** ([LICENSE](LICENSE)) : vous pouvez l'utiliser, le modifier, le
redistribuer et l'inclure dans d'autres logiciels, même commerciaux, à condition de garder la mention de copyright et
le texte de la licence.
Les fichiers de données ont leurs propres licences : le modèle et l'archive de ressources sont sous **CC BY-SA 4.0**
(utilisation libre, même commerciale, en citant les sources et en partageant les versions modifiées sous la même
licence) ; le lexique, issu du dictionnaire de Grammalecte (Olivier R.), reste sous **MPL-2.0**. Détails et sources à
citer : [DATA-LICENSES.md](DATA-LICENSES.md), [NOTICE.md](NOTICE.md).

---

## English

### What it is

Vaugelas proofreads French text and points out mistakes: agreement (subject-verb, past participles, adjectives),
homophones (*a / à*, *son / sont*, *ces / ses*, *vert / verre*…), moods (subjunctive, conditional), infinitive vs
participle (*manger / mangé*), capitals, unknown words and typography, with a correction when it has one.

It combines two checkers into **one report, one entry per word**:
- **hand-written rules**, very fast and precise;
- **a learned model** (a paradigm-energy neural network, version N1, trained without any data written by Claude) that compares each word with the other forms it
  could take and flags the one that does not fit, together with the most plausible form. It also reads what the
  rules found.

Everything runs **on your computer**, offline. Your text is never sent anywhere.
Strengths and weaknesses, with examples: [docs/capabilities.en.md](docs/capabilities.en.md).

### Install

You need **four files**, all on the project's *Releases* page:
1. the program for your system: `vaugelas-v0.2.1-windows-x86_64.zip`, `…-macos-arm64.tar.gz` (Apple silicon),
   `…-macos-x86_64.tar.gz` (Intel Mac) or `…-linux-x86_64.tar.gz`;
2. the three data files: `model.frcm.gz` (70 MB), `lexicon.tsv.gz` (3 MB), `resources.tar.gz` (9 MB).

Unpack the program, then **put the three data files in the same folder as the program**, still compressed. On first
run Vaugelas finds them there, checks their sha256 and installs them in your user data folder (about ten seconds);
you can then delete the downloaded files. Without them it tries to download them itself, which fails while the
repository is private, and it tells you what to do.

The data uses about 200 MB on disk: Windows `%LOCALAPPDATA%\vaugelas`, macOS `~/Library/Application
Support/vaugelas`, Linux `~/.local/share/vaugelas`; `--data DIR` or `VAUGELAS_DATA` to put it elsewhere.
`vaugelas info` shows where the data is and which model is installed.

**macOS:** the program is not signed by Apple; if macOS refuses to open it, run once in Terminal, in the program's
folder: `xattr -d com.apple.quarantine vaugelas`. **Windows:** on "Windows protected your PC", click *More info*,
then *Run anyway*.

### Run

```
vaugelas check "Les élections ont été annulés par le juge."
vaugelas check -f my_text.txt
vaugelas check --json "…"              # machine-readable output
vaugelas check --threshold 0.9 "…"     # more model flags (and more false alarms); default 0.9935
vaugelas check --rules-only "…"        # rules only; --model-only for the model only
vaugelas --help
```

The default threshold 0.9935 makes the model flag about 3.5 of 1,000 correct sentences of published books.

### Settings

Every setting can be written once in a file, `settings.toml`. To create it:

```
vaugelas settings --init
```

The program says where it wrote it (Windows `%APPDATA%\vaugelas`, macOS `~/Library/Application Support/vaugelas`,
Linux `~/.config/vaugelas`); a `settings.toml` next to the program is read too. Open it in a text editor: every
setting is explained in it, in French and English. It sets:

- **the model threshold** (`threshold`): lower finds more errors but also flags more correct sentences. For example,
  "Il est partit à la gare ce matin." passes at the default threshold (0.9935, score 0.984) and is flagged at 0.98;
- rules and model (each can be switched off), spelling suggestions, strict typography, the int8 model (less memory);
- rules or categories never to report (`disabled_rules`, `disabled_categories`; list of rules: `vaugelas rules`);
- the data folder, and the server's host, port and threads.

`vaugelas settings` shows the file in use and the settings in force. Command-line options take precedence over the
file. A mistake in the file (misspelled setting, value out of range, unknown rule) stops the program with a message
naming it.

### Use it from an editor (LanguageTool-compatible server)

```
vaugelas serve --port 8081
```
Point a LanguageTool client at `http://localhost:8081` (some want `http://localhost:8081/v2`): LTeX in VS Code
(`"ltex.languageToolHttpServerUri": "http://localhost:8081"`, `"ltex.language": "fr"`), the LanguageTool browser
extension (advanced settings, local server), LibreOffice's LanguageTool remote checker, Obsidian or Emacs plug-ins.
The server follows LanguageTool's public `/v2` API; it was tested with direct requests, not with each of these
clients, so please report one that does not work.
Supported: `POST /v2/check` (`language`, `text` or annotated `data`, `disabledRules`, `enabledRules` +
`enabledOnly`, `disabledCategories`) and `GET /v2/languages`. Offsets are in UTF-16 code units, as in LanguageTool.
French only. Each match also carries `"vaugelas": {"source": "rules" | "model" | "rules+model", "score": …}`.
The model is loaded once at start (about 1 s); a one-sentence request then takes about 15 ms. The server binds to
`127.0.0.1` unless `--host` is given. It uses [tiny_http](https://crates.io/crates/tiny_http), a small synchronous
HTTP server: enough for a local service answering one editor.

### Privacy

Vaugelas goes online only to download its data files, and only when they are missing. It collects nothing and
never sends your text anywhere. The server accepts connections from your own computer only.

### Strengths and limits

- **Good** at agreement (subject-verb, past participle, adjective, attribute), infinitive vs participle, subjunctive
  and conditional when a rule knows the trigger, capitals of days and months, and part of the homophones.
- **Fair** on homophones: 55% on a homophone test made the same way as its training examples (which favours it), and
  it still misses many (*ou / où*, *ces / ses*, *mer / mère*).
- **Weak** on word choice that needs meaning (*conjecture / conjoncture*, *voie / voix*), on errors that need a word
  added or removed, on punctuation and style. The model only replaces a word with another form of it.

On the project's hard test set (final-2, never seen in training) the model alone finds 51.4% of the errors at the
default threshold, and the rules alone 47.0% (with 1.6% of correct sentences flagged); together they find more,
since they partly catch different errors. On 18,703 correct sentences of books the model alone flags about 3.5 per
1,000. Details: [docs/capabilities.en.md](docs/capabilities.en.md).

### Speed and memory

2013 laptop (Intel i5-4300M, 2 cores, 7 GB): rules + model about 12 ms per sentence and about 650 MB of memory; rules
only under 0.1 ms per sentence (the model is still loaded). Startup about 1 s, the very first one 5-10 s while the
data is installed and the tables are built. The server uses about 620 MB. The program itself is about 6 MB.

### Reporting a problem

Open an issue on the project's GitHub page with the exact sentence, Vaugelas's answer, what you expected, and the
output of `vaugelas --version` and `vaugelas info`. A false alarm on a correct sentence is as useful as a missed error.

### For developers

```
cargo build --release            # binary: target/release/vaugelas
cargo test --release             # data-dependent tests skip when the data is not installed
cargo clippy --all-targets -- -D warnings
vaugelas download-model --from DIR   # install from local files (model.frcm.gz, lexicon.tsv.gz, resources.tar.gz)
vaugelas score -f sentences.txt      # per-word model scores, one sentence per line (JSON lines)
```
- `crates/vaugelas`: the library. `rules`: the fr-check rule checker. `model`: energy-model inference and the v14
  record path (candidate lists, fr-check lints as model inputs, corpus atoms); `model::profile` holds the settings
  that belong to each model (fr-check lints it reads, default threshold). `text`: tokenizer, sentence filter,
  characterizer and lexicon tables. `report`: the merged report.
- `crates/vaugelas-cli`: the `vaugelas` program (CLI, data install, LanguageTool-compatible server).
- The model and text code is a port of the research implementation (fr-check-infer, commit cc7d94e). Parity:
  `tests/fixtures/sentences.tsv` holds 343 hand-written sentences and `sentences.frc-infer-v14.jsonl` the scores
  `frc-infer v14` gives them with the shipped model (N1); the `parity_model` test compares every per-word score
  (2026-11-10: max |diff| 0 on 3,256 words). `tools/parity.sh FRC_INFER DATA_DIR` regenerates the reference. The rule checker is checked
  against `frc` on `tests/fixtures/sample.txt` (1,006 hand-written sentences, byte-identical).
- **Data release.** The data is not in git. Data set `data-3` = release `data-3` with `model.frcm.gz`,
  `lexicon.tsv.gz`, `resources.tar.gz` and `SHA256SUMS`. The sha256 of the uncompressed files (and of every file
  inside the resources archive) are in `crates/vaugelas-cli/src/data.rs`. `VAUGELAS_DATA_URL` points the download
  at another location. While the repository is private, anonymous downloads fail: put the files next to the program
  or use `download-model --from DIR`.
- Binary releases: push a tag `vX.Y.Z`; `.github/workflows/release.yml` builds Linux x86_64, Windows x86_64, macOS
  x86_64 and arm64, adds the data files of the current data set, and makes a draft release. See [CHANGELOG.md](CHANGELOG.md).

### Licence

The program (the code) is licensed under the **MIT licence** ([LICENSE](LICENSE)): you may use, change and
redistribute it and include it in other software, commercial or not, as long as you keep the copyright notice and
the licence text.
The data files have their own licences: the model and the resources archive are under **CC BY-SA 4.0** (free use,
commercial too, with credit to the sources and changed versions shared under the same licence); the lexicon, from
the Grammalecte dictionary (Olivier R.), stays under **MPL-2.0**. Details and the credits to give:
[DATA-LICENSES.md](DATA-LICENSES.md), [NOTICE.md](NOTICE.md).
