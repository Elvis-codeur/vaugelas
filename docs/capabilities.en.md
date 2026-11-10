# What Vaugelas does well, and what it does badly

*[Version française](capabilities.md)*

Vaugelas has two parts working together:

- **the rules** (fr-check): hand-written grammar rules, fast and predictable;
- **the model** (H1): a neural network that looks at each word, compares it with the other forms it could take
  (*chante / chantent / chantait*, *vert / verre / vers*) and flags it when another form fits the sentence clearly
  better, proposing that form. It also reads what the rules found.

The report merges both: a word flagged by the rules, the model or both appears once. This page says what each one
can do, with measured numbers and examples. It is about versions 0.2.x (model H1, default threshold 0.9914).

## 1. Numbers

Two kinds of numbers, not to be mixed:

- **measured rates** on the project's test sets, never used to train the model (sources at the end);
- **examples**: 343 sentences written to test this version (`tests/fixtures/sentences.tsv`), run through the
  shipped program (`vaugelas check`, default settings). Too few for a reliable rate; they show concrete cases.

### The H1 model alone

"Detects" = share of the sentences with an error where the wrong word is flagged, at the default threshold 0.9914.

| test set | what it holds | detects |
|---|---|---:|
| final-2 | 1,835 sentences with an error and 3,270 correct ones, 97 kinds of hard errors, written by DeepSeek | **51.4%** |
| homophones | 1,500 book sentences where one word was replaced by a homophone (*cours* for *court*) | 55.1% |
| cw | errors written by Claude: capitals, moods, nouns, paronyms, prepositions, relatives | 39.3% |
| cw2 | errors written by Claude: coordination, inversion, *lequel*, literary style, proper names | 68.2% |
| correct books | 18,703 sentences of published books: sentences the model flags | 3.5 per 1,000 |

The threshold 0.9914 was chosen for that last number: the model flags about as many correct sentences as the previous
model. With `--threshold 0.9` it flags more errors and more correct sentences.

**Caveat on the homophone test.** It is made the same way as H1's homophone training examples (other books, other
sentences), and nearly all its word pairs (1,481 of 1,500) were seen in training: it measures new contexts for known
pairs, not new confusions. On final-2, which is independent, the homophone family goes from 39-40% (earlier models)
to 45% (at the training's tuning threshold, see below). A homophone test on real errors is planned.

### The rules alone, and other checkers

Same final-2 sentences, grammar rules only (spelling, typography and capitals set aside), measured on 2026-11-07:

| | detects | correct sentences flagged |
|---|---:|---:|
| Vaugelas rules (fr-check) | 47.0% | 1.6% |
| LanguageTool 6.8 | 36.9% | 4.6% |
| Grammalecte 2.3 | 27.2% | 0.9% |

**Rules + model together**: Vaugelas's report is the union of both, so it finds at least as much as the better of the
two, and more because they partly catch different errors. The exact union figure with H1 has not been measured on
final-2 yet.

### By error family (H1 model, final-2)

Measured at the training's tuning threshold (0.9848, a little more permissive than the shipped 0.9914: rates at the
shipped threshold are a little lower).

| family | detects |
|---|---:|
| past participle (*les fleurs que j'ai acheté*) | 65% |
| relatives (*dont*, *lequel*, *à qui*) | 62% |
| moods (subjunctive, conditional) | 60% |
| homophones (*a / à*, *ou / où*, *-er / -é*, *notre / nôtre*) | 45% |
| pronouns (*tout*, neuter *le*, *y*) | 40% |
| capitals (*le Mardi*, *en Septembre*) | 38% |
| prepositions and constructions (*permettre à*, *malgré que*) | 32% |
| elision and hyphen (*va-t'en*, *a-t-il*) | 30% |
| verb tenses (*depuis* + present, future in a past narrative) | 25% |

For capitals the rules do better than the model: the days-and-months rule finds most of them, and it is in the
report anyway.

## 2. What it does well (examples)

Of the 343 test sentences, 126 hold an error (129 errors in all); Vaugelas flags **93**, 88 of them with exactly the
expected correction as first suggestion. Below: the correction shown, and who found it (rules, model, or both).

**Agreement** (subject-verb, participle, adjective, attribute): its strong point, often rules and model together.

- *Le chien des voisins **aboient** toute la nuit.* → **aboie** (both)
- *C'est moi qui **a** fait ce gâteau.* → **ai** (both)
- *Les fleurs du jardin **pousse** très vite au printemps.* → **poussent** (model)
- *Les clés que tu as **perdu** étaient dans ta poche.* → **perdues** (both)
- *La lettre qu'il a **écrit** était très longue.* → **écrite** (rules)
- *Ils ont **finis** leur travail avant midi.* → **fini** (model)
- *Une **grand** maison se dresse au bout du chemin.* → **grande** (model)
- *Les chercheurs qui ont participé à l'étude sont **convaincu** que leurs résultats seront confirmés.* →
  **convaincus** (both)

**Infinitive vs participle**:

- *Il a **manger** toute la tarte.* → **mangé** (both)
- *Il faut **changé** les draps.* → **changer** (model)

**Moods** (mostly the rules, when they know the word that triggers the mood):

- *Bien qu'il **est** malade, il travaille.* → **soit** (both)
- *Je veux qu'il **fait** ses devoirs.* → **fasse** (model)
- *Si j'**aurais** su, je serais venu.* → **avais** (rules)
- *Après qu'il **soit** parti, nous avons dîné.* → **est** (both)

**Homophones** (mostly the model):

- *Mon père **à** acheté une nouvelle voiture.* → **a**; *Il **peu** venir demain.* → **peut**;
  *Il a perdu **sont** portefeuille.* → **son**; *Il se demande **se** qu'il va faire.* → **ce**;
  *Quant il fait beau, nous sortons.* → **Quand** (model)
- *Il a bu un **vert** de vin.* → **verre**; *Il marche **vert** la gare.* → **vers**;
  *Il a acheté du **pin** à la boulangerie.* → **pain**; *Il est parti **cent** dire au revoir.* → **sans** (model)

**Conjugation, capitals, spelling**:

- *Il a **prit** le train de nuit.* → **pris** (model); *Tu **peut** venir avec nous.* → **peux** (both)
- *Nous partons **Lundi** matin.* → **lundi** (both); *J'apprends l'**Espagnol** depuis deux ans.* → **espagnol**
  (rules)
- *Elle a acheté un nouvel **ordinnateur**.* → **ordinateur** (rules)

**Few false alarms.** Of the 217 correct test sentences, the model flags none; the rules flag one (see below). On
published books the model alone flags about 3.5 sentences per 1,000.

## 3. What it does badly (examples)

**1. Word choice that needs meaning.** The model compares forms that are spelt or pronounced almost alike, but it
does not know what words mean. Missed in the test: *Je suis allé à la **mère** pendant les vacances.* (*mer*);
*La **mer** de cette fille est professeur.* (*mère*); *Le **conte** en banque est vide.* (*compte*);
*Elle a une belle **voie** pour chanter.* (*voix*); *Le **chant** de blé est doré en été.* (*champ*);
*Il a perdu beaucoup de **cent** dans l'accident.* (*sang*). Of 14 errors of this kind it finds 5.

**2. Part of the grammatical homophones.** Missed: *Je ne sais pas **ou** il habite.* (*où*);
*Elle a rangé **ces** affaires dans sa chambre.* (*ses*); *Il a mal **a** la tête.* (*à*);
*Il **c'est** trompé de route.* (*s'est*); *Je **leurs** ai donné un livre.* (*leur*).
For *ou* and *ces* the model sees almost nothing wrong (scores 0.03 and 0.48); for *a* it hesitates (0.93), below the
threshold.

**3. Easy agreements missed narrowly.** The model sometimes gives a high score just under 0.9914:
*Les enfants **joue** dans la cour de l'école.* (score 0.985, suggestion *jouent*);
*J'ai lu tous les **livre** de la bibliothèque.* (0.93, *livres*); *Hier, je suis **aller** au marché.* (0.978,
*allé*). With `--threshold 0.9`, grammar flags reach 91 of the 126 test sentences with an error instead of 83, but also
4 of the 217 correct sentences instead of one (e.g. *Le **pin** est un arbre qui pousse bien dans les Landes.*,
"corrected" to *pain*). Some agreements it clearly misses: *Je pense qu'ils **viendra** demain.* (0.71);
*Nous sommes **allé** au cinéma hier.* (0.81); *Nous avons vu un film et nous avons **rit**.* (0.01).

**4. Errors that need a word added, removed or moved.** The model only replaces a word with another form of it.
*Je vais **au** coiffeur demain.* (*chez le*) and *Il se rappelle **de** son enfance.* are not flagged. The rules do
find the *ne* missing after *on* (*On a pas* → *On n'a pas*).

**5. Wrong suggestions.** Rare in the test, but they exist: *Les feuilles sont **verre** au printemps.* is flagged,
with the suggestion *vers* (it should be *vertes*); for *Il a eu une crise de **fois**.* (not flagged), the model's best
form would be *foi* (it should be *foie*). When a sentence holds two errors, a rule can go wrong because of the other
one: in *Ils **on** mangé et il son parti.*, *on* → *ont* is right, but *mangé* is flagged as well, wrongly.

**6. Rule false alarms.** *Les voisins nous ont **invités** à leur mariage.* is correct (*nous* is the object placed
before), but a rule proposes *invité*.

**7. Text that is not French.** The model does not score it (it recognises English or foreign sentences and leaves
them out), but the spelling checker flags every unknown word: *The weather is nice…* gets nine spelling flags; ignore
them in such passages.

**8. Outside its job:** style, punctuation (except spaces around punctuation), commas, word order, register. None of
the tests above measures them.

## 4. Where the numbers come from

- **H1 model**: the model card (`MODEL_CARD.md` in the research repository, `models/h1/`) and the project log of
  2026-11-09 (entries "C1 (control) scored" and "H1 is the base"). Rates measured on the research "product" evaluation
  path (same inputs as the program: raw text, rules re-run, no hint of the right answer). The research Rust
  implementation reproduces PyTorch's scores within 5 millionths, and the program reproduces the research Rust
  implementation's scores exactly (see below). The family table comes from the same log entry, at the training's
  tuning threshold.
- **Rules, LanguageTool, Grammalecte**: capabilities report of 2026-11-07 (the shipped program of that day, same rules
  as now) and the project's ERRATA 13-14. Same scoring for all: a sentence with an error is detected when a flag
  covers the wrong word; a correct sentence is a false alarm as soon as it gets a flag.
- **Caveat on final-2**: its phenomena were chosen partly where the rules were weak, which favours anything that
  complements the rules. It is a test of hard cases.
- **Small numbers**: between two trainings of the same model, rates move by about 1 to 2 points; final-2 has 1,835
  errors (standard error about 1.2 points).
- **Examples**: `tests/fixtures/sentences.tsv` (family, sentence, expected correction), run through
  `vaugelas check --json` (and `--rules-only`, `--model-only` to see who found what) on 2026-11-09. The model scores
  quoted come from `vaugelas score`. The same sentences are the parity test: the program gives, word by word, exactly
  the scores of the research implementation (maximum difference 0 on 3,256 words).
