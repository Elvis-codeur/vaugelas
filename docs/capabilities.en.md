# What Vaugelas does well, and what it does badly

*[Version française](capabilities.md)*

Vaugelas has two parts working together:

- **the rules** (fr-check): hand-written grammar rules, fast and predictable;
- **the model** (N1): a neural network that looks at each word, compares it with the other forms it could take
  (*chante / chantent / chantait*, *vert / verre / vers*) and flags it when another form fits the sentence clearly
  better, proposing that form. It also reads what the rules found. N1 was trained without any data written by
  Claude (Anthropic): every example of that kind was replaced by examples written with DeepSeek.

The report merges both: a word flagged by the rules, the model or both appears once. This page says what each one
can do, with measured numbers and examples. It is about versions 0.3.x (model N1, default threshold 0.9935).

## 1. Numbers

Two kinds of numbers, not to be mixed:

- **measured rates** on the project's test sets, never used to train the model (sources at the end);
- **examples**: 343 sentences written to test the program (`tests/fixtures/sentences.tsv`), run through the
  shipped program (`vaugelas check`, default settings). Too few for a reliable rate; they show concrete cases.

### The N1 model alone

"Detects" = share of the sentences with an error where the wrong word is flagged, at the default threshold 0.9935.

| test set | what it holds | detects |
|---|---|---:|
| final-2 | 1,835 sentences with an error and 3,270 correct ones, 97 kinds of hard errors, written by DeepSeek | **47.8%** |
| homophones | 1,500 book sentences where one word was replaced by a homophone (*cours* for *court*) | 54.8% |
| dw | errors written by DeepSeek (test part): capitals, moods, nouns, paronyms, prepositions, relatives | 48.4% |
| dw2 | errors written by DeepSeek (test part): coordination, inversion, *lequel*, literary style, proper names | 72.7% |
| correct books | 18,703 sentences of published books: sentences the model flags | 3.5 per 1,000 |

The threshold 0.9935 was chosen for that last number: the model flags as many correct sentences as the previous
model, H1, at its own threshold. With `--threshold 0.9` (or `threshold` in the settings file) it flags more errors
and more correct sentences.

**Compared with H1** (versions 0.2.x, which learnt partly from examples written by Claude): at the same budget of
false alarms on books, N1 finds 47.8% of the final-2 errors, H1 51.4%. The gap comes from two families: noun forms
(*des caillous*, *une auteuse*) and capitals (*en Septembre*), where N1 is clearly weaker. In the program the rules
make up for part of it: the spelling check flags invented forms such as *caillous* or *chevals*, and the
days-and-months rule flags *Septembre*. N1 is stronger than H1 on verb forms and elision. At an equal rate of false
alarms on final-2 itself the two models are level (53.3% and 53.8% at 2%).

**Caveat on the homophone test.** It is made the same way as the model's homophone training examples (other books,
other sentences), and nearly all its word pairs were seen in training: it measures new contexts for known pairs, not
new confusions. A homophone test on real errors is planned.

### The rules alone, and other checkers

Same final-2 sentences, grammar rules only (spelling, typography and capitals set aside), measured on 2026-11-07:

| | detects | correct sentences flagged |
|---|---:|---:|
| Vaugelas rules (fr-check) | 47.0% | 1.6% |
| LanguageTool 6.8 | 36.9% | 4.6% |
| Grammalecte 2.3 | 27.2% | 0.9% |

**Rules + model together**: Vaugelas's report is the union of both, so it finds at least as much as the better of the
two, and more because they partly catch different errors. The exact union figure with N1 has not been measured on
final-2 yet.

### By error family (N1 model alone, final-2, default threshold 0.9935)

| family | detects |
|---|---:|
| subject-verb agreement (*les enfants joue*) | 69% |
| past participle (*les fleurs que j'ai acheté*) | 62% |
| moods (subjunctive, conditional) | 58% |
| attribute and adjective agreement | 54% |
| verb tenses and forms | 53% |
| elision and hyphen (*va-t'en*, *a-t-il*) | 50% |
| relatives (*dont*, *lequel*, *à qui*) | 49% |
| homophones (*a / à*, *ou / où*, *-er / -é*, *notre / nôtre*) | 40% |
| pronouns (*tout*, neuter *le*, *y*) | 28% |
| prepositions and constructions (*permettre à*, *malgré que*) | 24% |
| noun forms (plurals, feminines, compounds) | 18% |
| capitals (*le Mardi*, *en Septembre*) | 17% |
| paronyms (*éminent / imminent*) | 1% |

For capitals and invented noun forms the rules do better than the model (days and months rule, spelling check), and
they are in the report anyway.

## 2. What it does well (examples)

Of the 343 test sentences, 126 hold an error (129 errors in all); Vaugelas flags **88**, 80 of them with exactly the
expected correction as first suggestion. Below: the correction shown, and who found it (rules, model, or both).

**Agreement** (subject-verb, participle, adjective, attribute): its strong point, often rules and model together.

- *Le chien des voisins **aboient** toute la nuit.* → **aboie** (both)
- *C'est moi qui **a** fait ce gâteau.* → **ai** (both)
- *Les fleurs du jardin **pousse** très vite au printemps.* → **poussent** (model)
- *Les enfants **joue** dans la cour de l'école.* → **jouent** (model)
- *Les clés que tu as **perdu** étaient dans ta poche.* → **perdues** (both)
- *Nous sommes **allé** au cinéma hier.* → **allés** (model)
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
  *Il a perdu **sont** portefeuille.* → **son**; *Il est grand **est** fort.* → **et** (model)
- *Il a bu un **vert** de vin.* → **verre**; *Il a acheté du **pin** à la boulangerie.* → **pain**;
  *Il est parti **cent** dire au revoir.* → **sans** (model)

**Conjugation, capitals, spelling**:

- *Tu **peut** venir avec nous.* → **peux**; *Je **croyait** qu'il était parti.* → **croyais** (both)
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
*Il a perdu beaucoup de **cent** dans l'accident.* (*sang*). Of 14 errors of this kind it flags 4.

**2. Many grammatical homophones.** Missed: *Il a mal **a** la tête.* (*à*); *Le ciel **et** bleu aujourd'hui.*
(*est*); *Je ne sais pas **ou** il habite.* (*où*); *Elle a rangé **ces** affaires dans sa chambre.* (*ses*);
*Il se demande **se** qu'il va faire.* (*ce*); ***Quant** il fait beau, nous sortons.* (*Quand*);
*Il **c'est** trompé de route.* (*s'est*). The model hesitates on several of them (scores 0.4 to 0.94, below the
threshold); of the 18 errors of this kind it flags 7.

**3. Easy agreements missed narrowly.** The model sometimes gives a high score just under 0.9935:
*Tu as oublié tes clés et tu **cherche** partout.* (0.987, *cherches*); *Je **leurs** ai donné un livre.* (0.976,
*leur*); *Nous avons mangé et nous **somme** partis.* (0.969, *sommes*). With `--threshold 0.9`, grammar flags
reach 91 of the 126 test sentences with an error instead of 85, but also 4 of the 217 correct sentences instead of
one. Some it clearly misses: *Je pense qu'ils **viendra** demain.* (0.79); *J'ai lu tous les **livre** de la
bibliothèque.* (0.73); *Hier, je suis **aller** au marché.* (0.15); *Il a **prit** le train de nuit.* (0.66);
*Nous avons vu un film et nous avons **rit**.* (0.02).

**4. Errors that need a word added, removed or moved.** The model only replaces a word with another form of it.
*Je vais **au** coiffeur demain.* (*chez le*) and *Il se rappelle **de** son enfance.* are not flagged. The rules do
find the *ne* missing after *on* (*On a pas* → *On n'a pas*).

**5. Wrong suggestions.** Rare in the test, but they exist: *Les feuilles sont **verre** au printemps.* is flagged,
with the suggestion *verts* (it should be *vertes*). When a sentence holds two errors, one can hide the other, or a
rule can go wrong because of it: in *Ils **on** mangé et il son parti.*, *on* → *ont* is right, *son* is missed and
*mangé* is flagged as well, wrongly; in *Les enfant joue dans le jardin.*, *enfant* is flagged but *joue* is not.

**6. Rule false alarms.** *Les voisins nous ont **invités** à leur mariage.* is correct (*nous* is the object placed
before), but a rule proposes *invité*.

**7. Text that is not French.** The model does not score it (it recognises English or foreign sentences and leaves
them out), but the spelling checker flags unknown words: *The weather is nice…* gets several spelling flags; ignore
them in such passages.

**8. Outside its job:** style, punctuation (except spaces around punctuation), commas, word order, register. None of
the tests above measures them.

## 4. Where the numbers come from

- **N1 model**: the model card (`MODEL_CARD.md` in the research repository, `models/n1/`) and the project log of
  2026-11-10. Rates measured on the research "product" evaluation path (same inputs as the program: raw text, rules
  re-run, no hint of the right answer). The research Rust implementation reproduces PyTorch's scores within 5
  millionths (4,838 final-2 sentences), and the program reproduces the research Rust implementation's scores exactly
  (see below). The family table is measured at the shipped threshold 0.9935.
- **Rules, LanguageTool, Grammalecte**: capabilities report of 2026-11-07 (the shipped program of that day, same rules
  as now) and the project's ERRATA 13-14. Same scoring for all: a sentence with an error is detected when a flag
  covers the wrong word; a correct sentence is a false alarm as soon as it gets a flag.
- **Caveat on final-2**: its phenomena were chosen partly where the rules were weak, which favours anything that
  complements the rules. It is a test of hard cases.
- **Small numbers**: between two trainings of the same model, rates move by about 1 to 2 points; final-2 has 1,835
  errors (standard error about 1.2 points).
- **Examples**: `tests/fixtures/sentences.tsv` (family, sentence, expected correction), run through
  `vaugelas check --json` (and `--rules-only`, `--model-only` to see who found what) on 2026-11-10. The model scores
  quoted come from `vaugelas score`. The same sentences are the parity test: the program gives, word by word, exactly
  the scores of the research implementation (maximum difference 0 on 3,256 words).
