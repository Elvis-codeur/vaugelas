# Ce que Vaugelas fait bien, et ce qu'il fait mal

*[English version](capabilities.en.md)*

Vaugelas a deux parties qui travaillent ensemble :

- **les règles** (fr-check) : des règles de grammaire écrites à la main, rapides et prévisibles ;
- **le modèle** (N1) : un réseau de neurones qui regarde chaque mot, le compare aux autres formes qu'il pourrait
  prendre (*chante / chantent / chantait*, *vert / verre / vers*) et le signale quand une autre forme convient nettement
  mieux à la phrase. Il propose alors cette forme. Il lit aussi ce que les règles ont trouvé. N1 a été entraîné sans
  aucune donnée écrite par Claude (Anthropic) : tous les exemples de ce type ont été remplacés par des exemples écrits
  avec DeepSeek.

Le rapport réunit les deux : un mot signalé par les règles, par le modèle ou par les deux n'apparaît qu'une fois.
Cette page dit ce que chacun sait faire, avec des chiffres mesurés et des exemples. Elle concerne les versions 0.3.x
(modèle N1, seuil par défaut 0,9935).

## 1. Les chiffres

Deux sortes de chiffres, à ne pas mélanger :

- des **taux mesurés** sur les jeux de test du projet, qui n'ont jamais servi à entraîner le modèle (sources en fin de
  page) ;
- des **exemples** : 343 phrases écrites pour tester le programme (`tests/fixtures/sentences.tsv`), passées dans le
  programme distribué (`vaugelas check`, réglages par défaut). C'est trop peu pour un taux fiable ; elles montrent des
  cas concrets.

### Le modèle N1, seul

« Détecte » = part des phrases fautives où le mot fautif est signalé, au seuil par défaut 0,9935.

| jeu de test | ce qu'il contient | détecte |
|---|---|---:|
| final-2 | 1 835 phrases fautives et 3 270 phrases correctes, 97 sortes de fautes difficiles, écrites par DeepSeek | **47,8 %** |
| homophones | 1 500 phrases de livres où un mot a été remplacé par un homophone (*cours* pour *court*) | 54,8 % |
| dw | fautes écrites par DeepSeek (partie test) : majuscules, modes, noms, paronymes, prépositions, relatifs | 48,4 % |
| dw2 | fautes écrites par DeepSeek (partie test) : coordination, inversion, *lequel*, style littéraire, noms propres | 72,7 % |
| livres corrects | 18 703 phrases de livres publiés : phrases que le modèle signale | 3,5 sur 1 000 |

Le seuil 0,9935 a été choisi pour ce dernier chiffre : le modèle signale autant de phrases correctes que le modèle
précédent, H1, à son propre seuil. Avec `--threshold 0.9` (ou `threshold` dans le fichier de réglages), il signale
plus de fautes, et plus de phrases correctes.

**Comparé à H1** (versions 0.2.x, qui avait appris en partie sur des exemples écrits par Claude) : à budget égal de
fausses alertes sur les livres, N1 trouve 47,8 % des fautes de final-2, H1 51,4 %. L'écart vient de deux familles :
les formes des noms (*des caillous*, *une auteuse*) et les majuscules (*en Septembre*), où N1 est nettement plus
faible. Dans le programme, les règles en rattrapent une partie : le correcteur d'orthographe signale les formes
inventées comme *caillous* ou *chevals*, et la règle des jours et des mois signale *Septembre*. N1 est meilleur que H1
sur les formes verbales et l'élision. À taux égal de fausses alertes sur final-2 lui-même, les deux modèles font
jeu égal (53,3 % et 53,8 % à 2 %).

**Réserve sur le test d'homophones.** Il est fabriqué de la même façon que les exemples d'homophones de
l'entraînement du modèle (d'autres livres, d'autres phrases), et presque toutes ses paires de mots ont été vues à
l'entraînement : il mesure de nouveaux contextes pour des paires connues, pas de nouvelles confusions. Un test
d'homophones sur de vraies fautes est prévu.

### Les règles seules, et les autres correcteurs

Mêmes phrases de final-2, règles de grammaire seules (orthographe, typographie et majuscules mises à part), mesuré le
7 novembre 2026 :

| | détecte | phrases correctes signalées |
|---|---:|---:|
| règles de Vaugelas (fr-check) | 47,0 % | 1,6 % |
| LanguageTool 6.8 | 36,9 % | 4,6 % |
| Grammalecte 2.3 | 27,2 % | 0,9 % |

**Règles et modèle ensemble** : le rapport de Vaugelas réunit les deux ; il trouve donc au moins autant que le
meilleur des deux, et davantage, parce qu'ils attrapent en partie des fautes différentes. Le chiffre exact de la
réunion avec N1 n'a pas encore été mesuré sur final-2.

### Par famille de fautes (modèle N1 seul, final-2, seuil par défaut 0,9935)

| famille | détecte |
|---|---:|
| accord sujet-verbe (*les enfants joue*) | 69 % |
| participe passé (*les fleurs que j'ai acheté*) | 62 % |
| modes (subjonctif, conditionnel) | 58 % |
| accord de l'attribut et de l'adjectif | 54 % |
| temps et formes des verbes | 53 % |
| élision et trait d'union (*va-t'en*, *a-t-il*) | 50 % |
| relatifs (*dont*, *lequel*, *à qui*) | 49 % |
| homophones (*a / à*, *ou / où*, *-er / -é*, *notre / nôtre*) | 40 % |
| pronoms (*tout*, *le* neutre, *y*) | 28 % |
| prépositions et constructions (*permettre à*, *malgré que*) | 24 % |
| formes des noms (pluriels, féminins, mots composés) | 18 % |
| majuscules (*le Mardi*, *en Septembre*) | 17 % |
| paronymes (*éminent / imminent*) | 1 % |

Pour les majuscules et les formes de noms inventées, les règles font mieux que le modèle (règle des jours et des
mois, correcteur d'orthographe), et elles sont de toute façon dans le rapport.

## 2. Ce qu'il fait bien (exemples)

Sur les 343 phrases de test, 126 contiennent une faute (129 fautes en tout) ; Vaugelas en signale **88**, dont 80
avec exactement la correction attendue en première suggestion. Ci-dessous : la correction affichée, et qui l'a
trouvée (règles, modèle, ou les deux).

**Accords** (sujet-verbe, participe, adjectif, attribut) : son point fort, souvent règles et modèle ensemble.

- *Le chien des voisins **aboient** toute la nuit.* → **aboie** (les deux)
- *C'est moi qui **a** fait ce gâteau.* → **ai** (les deux)
- *Les fleurs du jardin **pousse** très vite au printemps.* → **poussent** (modèle)
- *Les enfants **joue** dans la cour de l'école.* → **jouent** (modèle)
- *Les clés que tu as **perdu** étaient dans ta poche.* → **perdues** (les deux)
- *Nous sommes **allé** au cinéma hier.* → **allés** (modèle)
- *Ils ont **finis** leur travail avant midi.* → **fini** (modèle)
- *Une **grand** maison se dresse au bout du chemin.* → **grande** (modèle)
- *Les chercheurs qui ont participé à l'étude sont **convaincu** que leurs résultats seront confirmés.* →
  **convaincus** (les deux)

**Infinitif ou participe** :

- *Il a **manger** toute la tarte.* → **mangé** (les deux)
- *Il faut **changé** les draps.* → **changer** (modèle)

**Modes** (surtout les règles, quand elles connaissent le mot qui appelle le mode) :

- *Bien qu'il **est** malade, il travaille.* → **soit** (les deux)
- *Je veux qu'il **fait** ses devoirs.* → **fasse** (modèle)
- *Si j'**aurais** su, je serais venu.* → **avais** (règles)
- *Après qu'il **soit** parti, nous avons dîné.* → **est** (les deux)

**Homophones** (surtout le modèle) :

- *Mon père **à** acheté une nouvelle voiture.* → **a** ; *Il **peu** venir demain.* → **peut** ;
  *Il a perdu **sont** portefeuille.* → **son** ; *Il est grand **est** fort.* → **et** (modèle)
- *Il a bu un **vert** de vin.* → **verre** ; *Il a acheté du **pin** à la boulangerie.* → **pain** ;
  *Il est parti **cent** dire au revoir.* → **sans** (modèle)

**Conjugaison, majuscules, orthographe** :

- *Tu **peut** venir avec nous.* → **peux** ; *Je **croyait** qu'il était parti.* → **croyais** (les deux)
- *Nous partons **Lundi** matin.* → **lundi** (les deux) ; *J'apprends l'**Espagnol** depuis deux ans.* →
  **espagnol** (règles)
- *Elle a acheté un nouvel **ordinnateur**.* → **ordinateur** (règles)

**Peu de fausses alertes.** Sur les 217 phrases de test correctes, le modèle n'en signale aucune ; les règles en
signalent une (voir plus bas). Sur des livres publiés, le modèle seul signale environ 3,5 phrases sur 1 000.

## 3. Ce qu'il fait mal (exemples)

**1. Le choix du mot quand il faut comprendre le sens.** Le modèle compare des formes qui s'écrivent ou se prononcent
presque pareil, mais il ne sait pas ce que les mots veulent dire. Manqués dans le test : *Je suis allé à la **mère**
pendant les vacances.* (*mer*) ; *La **mer** de cette fille est professeur.* (*mère*) ; *Le **conte** en banque est
vide.* (*compte*) ; *Elle a une belle **voie** pour chanter.* (*voix*) ; *Le **chant** de blé est doré en été.*
(*champ*) ; *Il a perdu beaucoup de **cent** dans l'accident.* (*sang*). Sur 14 fautes de ce genre, il en signale 4.

**2. Beaucoup d'homophones grammaticaux.** Manqués : *Il a mal **a** la tête.* (*à*) ; *Le ciel **et** bleu
aujourd'hui.* (*est*) ; *Je ne sais pas **ou** il habite.* (*où*) ; *Elle a rangé **ces** affaires dans sa chambre.*
(*ses*) ; *Il se demande **se** qu'il va faire.* (*ce*) ; ***Quant** il fait beau, nous sortons.* (*Quand*) ;
*Il **c'est** trompé de route.* (*s'est*). Le modèle hésite sur plusieurs d'entre eux (scores de 0,4 à 0,94, sous le
seuil) ; sur les 18 fautes de ce genre, il en signale 7.

**3. Des accords faciles manqués de peu.** Le modèle donne parfois un score élevé juste sous 0,9935 :
*Tu as oublié tes clés et tu **cherche** partout.* (0,987, *cherches*) ; *Je **leurs** ai donné un livre.* (0,976,
*leur*) ; *Nous avons mangé et nous **somme** partis.* (0,969, *sommes*). Avec `--threshold 0.9`, les alertes de
grammaire touchent 91 des 126 phrases fautives du test au lieu de 85, mais aussi 4 des 217 phrases correctes au lieu
d'une. Certains sont nettement manqués : *Je pense qu'ils **viendra** demain.* (0,79) ; *J'ai lu tous les **livre**
de la bibliothèque.* (0,73) ; *Hier, je suis **aller** au marché.* (0,15) ; *Il a **prit** le train de nuit.*
(0,66) ; *Nous avons vu un film et nous avons **rit**.* (0,02).

**4. Les fautes qui demandent d'ajouter, de retirer ou de déplacer un mot.** Le modèle ne fait que remplacer un mot
par une autre de ses formes. *Je vais **au** coiffeur demain.* (*chez le*) et *Il se rappelle **de** son enfance.* ne
sont pas signalées. Les règles trouvent en revanche le *ne* manquant après *on* (*On a pas* → *On n'a pas*).

**5. Des suggestions fausses.** Rares dans le test, mais elles existent : *Les feuilles sont **verre** au printemps.*
est signalée, avec la suggestion *verts* (il faudrait *vertes*). Quand une phrase contient deux fautes, l'une peut
cacher l'autre, ou une règle peut se tromper à cause d'elle : dans *Ils **on** mangé et il son parti.*, *on* → *ont*
est juste, *son* est manqué et *mangé* est signalé lui aussi, à tort ; dans *Les enfant joue dans le jardin.*,
*enfant* est signalé mais pas *joue*.

**6. Fausses alertes des règles.** *Les voisins nous ont **invités** à leur mariage.* est correct (*nous* est le
complément placé avant), mais une règle propose *invité*.

**7. Le texte qui n'est pas en français.** Le modèle ne le note pas (il reconnaît les phrases anglaises ou étrangères
et les laisse de côté), mais le correcteur d'orthographe signale les mots inconnus : *The weather is nice…* reçoit
plusieurs alertes d'orthographe ; ignorez-les dans ces passages.

**8. Hors de son rôle :** le style, la ponctuation (sauf les espaces autour de la ponctuation), les virgules, l'ordre
des mots, le registre. Aucun des tests ci-dessus ne les mesure.

## 4. D'où viennent les chiffres

- **Modèle N1** : fiche du modèle (`MODEL_CARD.md` du dépôt de recherche, `models/n1/`) et journal du projet du
  10 novembre 2026. Taux mesurés par le chemin d'évaluation « produit » de la recherche (mêmes entrées que le
  programme : texte brut, règles relancées, aucune indication de la bonne réponse). L'implémentation Rust de la
  recherche reproduit les scores de PyTorch à 5 millionièmes près (4 838 phrases de final-2), et le programme
  reproduit exactement ceux de l'implémentation Rust de la recherche (voir plus bas). Le tableau par famille est
  mesuré au seuil distribué, 0,9935.
- **Règles, LanguageTool, Grammalecte** : rapport de capacités du 7 novembre 2026 (le programme distribué ce jour-là,
  mêmes règles qu'aujourd'hui) et ERRATA 13-14 du projet. Même notation pour tous : une phrase fautive est détectée
  quand une alerte couvre le mot fautif ; une phrase correcte est une fausse alerte dès qu'elle reçoit une alerte.
- **Réserve sur final-2** : ses phénomènes ont été choisis en partie là où les règles étaient faibles, ce qui avantage
  tout ce qui les complète. C'est un test de cas difficiles.
- **Petits nombres** : entre deux entraînements du même modèle, les taux bougent d'environ 1 à 2 points ; final-2
  contient 1 835 fautes (erreur type d'environ 1,2 point).
- **Exemples** : `tests/fixtures/sentences.tsv` (famille, phrase, correction attendue), passé dans
  `vaugelas check --json` (et `--rules-only`, `--model-only` pour savoir qui a trouvé quoi) le 10 novembre 2026. Les
  scores du modèle cités viennent de `vaugelas score`. Les mêmes phrases servent au test de parité : le programme
  donne, mot par mot, exactement les scores de l'implémentation de la recherche (écart maximal 0 sur 3 256 mots).
