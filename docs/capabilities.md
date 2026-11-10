# Ce que Vaugelas fait bien, et ce qu'il fait mal

*[English version](capabilities.en.md)*

Vaugelas a deux parties qui travaillent ensemble :

- **les règles** (fr-check) : des règles de grammaire écrites à la main, rapides et prévisibles ;
- **le modèle** (H1) : un réseau de neurones qui regarde chaque mot, le compare aux autres formes qu'il pourrait
  prendre (*chante / chantent / chantait*, *vert / verre / vers*) et le signale quand une autre forme convient nettement
  mieux à la phrase. Il propose alors cette forme. Il lit aussi ce que les règles ont trouvé.

Le rapport réunit les deux : un mot signalé par les règles, par le modèle ou par les deux n'apparaît qu'une fois.
Cette page dit ce que chacun sait faire, avec des chiffres mesurés et des exemples. Elle concerne les versions 0.2.x
(modèle H1, seuil par défaut 0,9914).

## 1. Les chiffres

Deux sortes de chiffres, à ne pas mélanger :

- des **taux mesurés** sur les jeux de test du projet, qui n'ont jamais servi à entraîner le modèle (sources en fin de
  page) ;
- des **exemples** : 343 phrases écrites pour tester cette version (`tests/fixtures/sentences.tsv`), passées dans le
  programme distribué (`vaugelas check`, réglages par défaut). C'est trop peu pour un taux fiable ; elles montrent des
  cas concrets.

### Le modèle H1, seul

« Détecte » = part des phrases fautives où le mot fautif est signalé, au seuil par défaut 0,9914.

| jeu de test | ce qu'il contient | détecte |
|---|---|---:|
| final-2 | 1 835 phrases fautives et 3 270 correctes, 97 sortes de fautes difficiles, écrites par DeepSeek | **51,4 %** |
| homophones | 1 500 phrases de livres où un mot a été remplacé par un homophone (*cours* pour *court*) | 55,1 % |
| cw | fautes écrites par Claude : majuscules, modes, noms, paronymes, prépositions, relatifs | 39,3 % |
| cw2 | fautes écrites par Claude : coordinations, inversions, *lequel*, style littéraire, noms propres | 68,2 % |
| livres corrects | 18 703 phrases de livres publiés : phrases signalées par le modèle | 3,5 pour 1 000 |

Le seuil 0,9914 a été choisi pour ce dernier chiffre : le modèle signale à peu près autant de phrases correctes que le
modèle précédent. Avec `--threshold 0.9` il signale davantage de fautes et davantage de phrases correctes.

**Réserve sur le test d'homophones.** Il est fabriqué de la même façon que les exemples d'homophones de
l'entraînement de H1 (d'autres livres, d'autres phrases), et presque toutes ses paires de mots (1 481 sur 1 500) ont
été vues à l'entraînement : il mesure des contextes nouveaux pour des paires connues, pas des confusions nouvelles.
Sur final-2, qui est indépendant, la famille « homophones » passe de 39-40 % (modèles précédents) à 45 % (au seuil
de réglage de l'entraînement, voir plus bas). Un test d'homophones sur de vraies fautes est prévu.

### Les règles, seules, et les autres correcteurs

Mêmes phrases de final-2, règles de grammaire seulement (orthographe, typographie et majuscules mises de côté),
mesures du 7 novembre 2026 :

| | détecte | phrases correctes signalées |
|---|---:|---:|
| règles de Vaugelas (fr-check) | 47,0 % | 1,6 % |
| LanguageTool 6.8 | 36,9 % | 4,6 % |
| Grammalecte 2.3 | 27,2 % | 0,9 % |

**Règles + modèle ensemble** : le rapport de Vaugelas est la réunion des deux, il trouve donc au moins autant que le
meilleur des deux, et davantage parce qu'ils attrapent en partie des fautes différentes. Le chiffre exact de la
réunion avec H1 n'a pas encore été mesuré sur final-2.

### Par famille de fautes (modèle H1, final-2)

Mesuré au seuil de réglage de l'entraînement (0,9848, un peu plus permissif que le seuil distribué 0,9914 : les taux
au seuil distribué sont un peu plus bas).

| famille | détecte |
|---|---:|
| participe passé (*les fleurs que j'ai acheté*) | 65 % |
| relatifs (*dont*, *lequel*, *à qui*) | 62 % |
| modes (subjonctif, conditionnel) | 60 % |
| homophones (*a / à*, *ou / où*, *-er / -é*, *notre / nôtre*) | 45 % |
| pronoms (*tout*, *le* neutre, *y*) | 40 % |
| majuscules (*le Mardi*, *en Septembre*) | 38 % |
| prépositions et constructions (*permettre à*, *malgré que*) | 32 % |
| élision et trait d'union (*va-t'en*, *a-t-il*) | 30 % |
| temps des verbes (*depuis* + présent, futur dans le passé) | 25 % |

Sur ces familles, les règles font mieux que le modèle pour les majuscules : la règle des jours et des mois en trouve
la plupart, et elle est dans le rapport de toute façon.

## 2. Ce qu'il fait bien (exemples)

Sur les 343 phrases d'essai : 126 contiennent une faute (129 fautes en tout) ; Vaugelas en signale **93**, dont 88 avec
exactement la correction attendue en première suggestion. Ci-dessous, la correction affichée et qui l'a trouvée
(règles, modèle, ou les deux).

**Accords** (sujet-verbe, participe, adjectif, attribut) : son point fort ; souvent règles et modèle ensemble.

- *Le chien des voisins **aboient** toute la nuit.* → **aboie** (les deux)
- *C'est moi qui **a** fait ce gâteau.* → **ai** (les deux)
- *Les fleurs du jardin **pousse** très vite au printemps.* → **poussent** (modèle)
- *Les clés que tu as **perdu** étaient dans ta poche.* → **perdues** (les deux)
- *La lettre qu'il a **écrit** était très longue.* → **écrite** (règles)
- *Ils ont **finis** leur travail avant midi.* → **fini** (modèle)
- *Une **grand** maison se dresse au bout du chemin.* → **grande** (modèle)
- *Les chercheurs qui ont participé à l'étude sont **convaincu** que leurs résultats seront confirmés.* →
  **convaincus** (les deux)

**Infinitif ou participe** :

- *Il a **manger** toute la tarte.* → **mangé** (les deux)
- *Il faut **changé** les draps.* → **changer** (modèle)

**Modes** (surtout les règles, quand elles connaissent le mot qui déclenche le mode) :

- *Bien qu'il **est** malade, il travaille.* → **soit** (les deux)
- *Je veux qu'il **fait** ses devoirs.* → **fasse** (modèle)
- *Si j'**aurais** su, je serais venu.* → **avais** (règles)
- *Après qu'il **soit** parti, nous avons dîné.* → **est** (les deux)

**Homophones** (surtout le modèle) :

- *Mon père **à** acheté une nouvelle voiture.* → **a** ; *Il **peu** venir demain.* → **peut** ;
  *Il a perdu **sont** portefeuille.* → **son** ; *Il se demande **se** qu'il va faire.* → **ce** ;
  *Quant il fait beau, nous sortons.* → **Quand** (modèle)
- *Il a bu un **vert** de vin.* → **verre** ; *Il marche **vert** la gare.* → **vers** ;
  *Il a acheté du **pin** à la boulangerie.* → **pain** ; *Il est parti **cent** dire au revoir.* → **sans** (modèle)

**Conjugaison, majuscules, orthographe** :

- *Il a **prit** le train de nuit.* → **pris** (modèle) ; *Tu **peut** venir avec nous.* → **peux** (les deux)
- *Nous partons **Lundi** matin.* → **lundi** (les deux) ; *J'apprends l'**Espagnol** depuis deux ans.* → **espagnol**
  (règles)
- *Elle a acheté un nouvel **ordinnateur**.* → **ordinateur** (règles)

**Peu de fausses alertes.** Sur les 217 phrases correctes de l'essai, le modèle n'en signale aucune ; les règles en
signalent une (voir plus bas). Sur des livres publiés, le modèle seul signale environ 3,5 phrases sur 1 000.

## 3. Ce qu'il fait mal (exemples)

**1. Le choix du mot quand il faut comprendre le sens.** Le modèle compare des formes qui s'écrivent ou se
prononcent presque pareil, mais il ne sait pas ce que les mots veulent dire. Manquées dans l'essai :
*Je suis allé à la **mère** pendant les vacances.* (*mer*) ; *La **mer** de cette fille est professeur.* (*mère*) ;
*Le **conte** en banque est vide.* (*compte*) ; *Elle a une belle **voie** pour chanter.* (*voix*) ;
*Le **chant** de blé est doré en été.* (*champ*) ; *Il a perdu beaucoup de **cent** dans l'accident.* (*sang*).
Sur 14 fautes de ce genre, il en trouve 5.

**2. Une partie des homophones grammaticaux.** Manquées : *Je ne sais pas **ou** il habite.* (*où*) ;
*Elle a rangé **ces** affaires dans sa chambre.* (*ses*) ; *Il a mal **a** la tête.* (*à*) ;
*Il **c'est** trompé de route.* (*s'est*) ; *Je **leurs** ai donné un livre.* (*leur*).
Pour *ou* et *ces*, le modèle ne voit presque rien d'anormal (score 0,03 et 0,48) ; pour *a* il hésite (0,93), sous le
seuil.

**3. Des accords faciles manqués de peu.** Le modèle donne parfois un score élevé mais sous le seuil 0,9914 :
*Les enfants **joue** dans la cour de l'école.* (score 0,985, suggestion *jouent*) ;
*J'ai lu tous les **livre** de la bibliothèque.* (0,93, *livres*) ; *Hier, je suis **aller** au marché.* (0,978,
*allé*). Avec `--threshold 0.9`, les signalements de grammaire touchent 91 des 126 phrases fautives de l'essai au
lieu de 83, mais aussi 4 phrases correctes sur 217 au lieu d'une (par exemple *Le **pin** est un arbre qui pousse bien dans les Landes.*,
« corrigé » en *pain*). Certains accords lui échappent nettement : *Je pense qu'ils **viendra** demain.*
(0,71) ; *Nous sommes **allé** au cinéma hier.* (0,81) ; *Nous avons vu un film et nous avons **rit**.* (0,01).

**4. Les fautes qui demandent d'ajouter, d'enlever ou de déplacer un mot.** Le modèle ne fait que remplacer un mot
par une autre forme de ce mot. *Je vais **au** coiffeur demain.* (*chez le*) et *Il se rappelle **de** son enfance.*
ne sont pas signalées. Les règles trouvent le *ne* oublié après *on* (*On a pas* → *On n'a pas*).

**5. Mauvaises suggestions.** Rares dans l'essai, mais elles existent : *Les feuilles sont **verre** au printemps.*
est bien signalé, avec la suggestion *vers* (il faut *vertes*) ; pour *Il a eu une crise de **fois**.* (non signalé),
la meilleure forme du modèle serait *foi* (il faut *foie*). Quand une phrase contient deux fautes, une règle peut se
tromper à cause de l'autre : dans *Ils **on** mangé et il son parti.*, *on* → *ont* est juste, mais *mangé* est aussi
signalé, à tort.

**6. Fausses alertes des règles.** *Les voisins nous ont **invités** à leur mariage.* est correcte (*nous* est le
complément placé avant), mais une règle propose *invité*.

**7. Les phrases qui ne sont pas en français.** Le modèle ne les note pas (il reconnaît les phrases anglaises ou
étrangères et les laisse de côté), mais le correcteur d'orthographe signale chaque mot inconnu : *The weather is
nice…* reçoit neuf signalements d'orthographe : ignorez-les dans ces passages.

**8. Hors de son rôle :** le style, la ponctuation (sauf les espaces autour de la ponctuation), les virgules, l'ordre
des mots, le registre. Aucun des tests ci-dessus ne les mesure.

## 4. D'où viennent les chiffres

- **Modèle H1** : fiche du modèle (`MODEL_CARD.md` du dépôt de recherche, `models/h1/`) et journal du projet du
  9 novembre 2026 (entrées « C1 (control) scored » et « H1 is the base »). Taux mesurés par le chemin d'évaluation
  « produit » de la recherche (mêmes entrées que le programme : texte brut, règles relancées, sans indice de la bonne
  réponse). L'implémentation Rust de recherche reproduit les scores de PyTorch à 5 millionièmes près, et le programme
  reproduit exactement ceux de l'implémentation Rust (voir plus bas). Le tableau par famille vient de la même
  entrée du journal, au seuil de réglage de l'entraînement.
- **Règles, LanguageTool, Grammalecte** : rapport de capacités du 7 novembre 2026 (programme distribué de l'époque,
  mêmes règles qu'aujourd'hui) et ERRATA 13-14 du projet. Notation identique pour tous : une phrase fautive est
  détectée quand un signalement recouvre le mot fautif ; une phrase correcte est une fausse alerte dès qu'elle reçoit
  un signalement.
- **Réserve sur final-2** : ses phénomènes ont été choisis en partie là où les règles étaient faibles ; il avantage
  tout ce qui complète les règles. C'est un test de cas difficiles.
- **Petits nombres** : entre deux entraînements du même modèle, les taux varient d'environ 1 à 2 points ; final-2 a
  1 835 fautes (erreur type d'environ 1,2 point).
- **Exemples** : `tests/fixtures/sentences.tsv` (famille, phrase, correction attendue), passés dans
  `vaugelas check --json` (et `--rules-only`, `--model-only` pour savoir qui a trouvé quoi) le 9 novembre 2026.
  Les scores du modèle cités viennent de `vaugelas score`. Ces mêmes phrases servent au test de parité : le
  programme donne, mot par mot, exactement les scores de l'implémentation de recherche (écart maximal 0 sur 3 256 mots).
