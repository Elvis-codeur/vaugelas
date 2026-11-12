#!/usr/bin/env python3
"""Write `tests/fixtures/sample.txt`: 1,006 French sentences for the rule-parity fixture (issue #3).

The sentences are generated from templates and word lists written here, with a fixed seed; no sentence is taken
from any corpus, workspace file or third-party text. The mix follows the old fixture's brief: agreement,
participles, homophones, mood, numbers, spelling, spacing, duplicated words, apostrophes, guillemets, hyphenated
words, long sentences, sentences with no error, and a few edge cases (quotes, backslash, tab, emoji, non-breaking
space).

  python3 tools/make_sample.py            # rewrites tests/fixtures/sample.txt (deterministic: same file every run)

The expected output (`sample.frc.jsonl`) is regenerated from the binary, not from fr-check:
  cargo run --release --example gen_parity
"""
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests/fixtures/sample.txt"
N = 1006
R = random.Random(20261112)

SUBJ = [("Le chat", "m"), ("La voiture", "f"), ("Les enfants", "mp"), ("Une femme", "f"), ("Mon frère", "m"),
        ("Ces maisons", "fp"), ("Le professeur", "m"), ("Les étudiantes", "fp"), ("Ton ami", "m"), ("La rivière", "f"),
        ("Le voisin", "m"), ("Ma sœur", "f"), ("Les voyageurs", "mp"), ("Cette ville", "f"), ("Le médecin", "m"),
        ("Les musiciens", "mp"), ("Une élève", "f"), ("Le facteur", "m"), ("La boulangère", "f"), ("Les oiseaux", "mp"),
        ("Le jardinier", "m"), ("Ma cousine", "f"), ("Les invités", "mp"), ("La cheminée", "f"), ("Le capitaine", "m"),
        ("Les lecteurs", "mp"), ("Une infirmière", "f"), ("Le peintre", "m"), ("La montagne", "f"), ("Les chevaux", "mp")]
AGREE_ERR = [("Le chat noir", "noire"), ("La voiture rouge", "rouges"), ("Les enfants sage", "sages"),
             ("Une femme grande", "grands"), ("Ces maisons ancien", "anciennes")]
VERB_INF = [("mange", "manger"), ("écrit", "écrire"), ("part", "partir"), ("vient", "venir"), ("chante", "chanter")]
HOMO = [("a", "à"), ("son", "sont"), ("ce", "se"), ("ou", "où"), ("la", "là"), ("mes", "mais")]
ADJ = ["content", "fatigué", "pressé", "surpris", "heureux", "calme", "attentif", "malade", "prêt", "sérieux",
       "curieux", "silencieux", "rapide", "patient", "inquiet", "satisfait", "déçu", "courageux", "timide", "prudent"]
NOUN = ["livre", "jardin", "projet", "voyage", "repas", "discours", "rapport", "concert", "tableau", "souvenir",
        "roman", "dossier", "chantier", "vélo", "manteau", "billet", "cadeau", "plan", "dessin", "poème",
        "roman", "rapport", "atelier", "bateau", "château", "chemin", "drapeau", "fromage", "gâteau", "journal"]
TIME = ["ce matin", "hier soir", "la semaine dernière", "tout à l'heure", "dans une heure", "aujourd'hui",
        "lundi", "au printemps", "pendant l'été", "vers midi", "après le repas", "avant la nuit",
        "ce soir", "la nuit dernière", "depuis peu", "dès demain"]
LONG = [
    "Quand la pluie cesse enfin, les habitants du village sortent dans les rues étroites pour vérifier les toits, "
    "les jardins et les chemins qui descendent vers la rivière",
    "Après avoir relu la lettre trois fois, elle comprit que son frère, parti depuis des années, ne reviendrait pas "
    "avant l'automne, et elle rangea le papier dans le tiroir",
    "Le comité, réuni depuis le matin dans la salle du fond, discutait encore des dépenses, des horaires et des "
    "moyens de transport quand la nouvelle arriva",
]


def agree():
    s, g = R.choice(SUBJ)
    adj = R.choice(ADJ)
    if R.random() < 0.5:
        return f"{s} est {adj}.", None
    wrong = R.choice(AGREE_ERR)[1]
    return f"{s} est {wrong}.", None


def participle():
    v = R.choice(["préparé", "écrit", "lu", "ouvert", "fini", "rendu"])
    if R.random() < 0.5:
        return f"Elle a {v} la lettre {R.choice(TIME)}.", None
    return f"Elle a {v} la lettre {R.choice(TIME)}.", None


def homophone():
    a, b = R.choice(HOMO)
    return f"Il {a} oublié son parapluie {b} la maison.", None


def mood():
    if R.random() < 0.5:
        return "Il faut que tu viennes avant midi.", None
    return "Je veux que vous soyez prêts à partir.", None


def numbers():
    n = R.randint(2, 99)
    return f"Il reste {n} places et {R.randint(2, 20)} billets pour le concert.", None


def spacing():
    s = R.choice(["Bonjour , comment allez-vous ?", "C'est ici , mais pas là .", "Attention : le train part."])
    return s, None


def dup():
    w = R.choice(["très", "vraiment", "le", "elle"])
    return f"Il est {w} {w} content de te voir.", None


def apos():
    return R.choice(["L'enfant lit dans le jardin.", "J'ai vu l'oiseau sur la branche.",
                     "C'est aujourd'hui qu'il part.", "Qu'il vienne demain matin."]), None


def guillemets():
    return R.choice(['Il a dit « bonjour » en entrant.', 'Elle a répondu : « Je viendrai demain. »',
                     '« Attends-moi ! » cria-t-il.']), None


def hyphen():
    return R.choice(["Une pomme-de-terre est tombée du sac.", "Rendez-vous à dix-huit heures.",
                     "C'est un rendez-vous important.", "Il a un arc-en-ciel au-dessus de la tête."]), None


def edge():
    return R.choice([
        "Le chemin est long\tmais il mène à la mer.",
        "Elle a écrit « C:\\dossier\\fichier » sur le tableau.",
        "Le résultat est simple 😀 et tout le monde est content.",
        "Le texte utilise une espace\u00a0insécable avant le deux-points.",
        "Il a dit : « Ne t'en fais pas. » Puis il est parti.",
    ]), None


PLACES = ["à la gare", "au marché", "dans le parc", "près de la rivière", "devant la mairie", "chez le voisin",
          "sur la place", "au bord du lac", "dans la cour", "à la bibliothèque", "au bout de la rue", "sous le porche"]
VERB2 = [("range", "chante"), ("lit", "écrit"), ("cherche", "trouve"), ("prépare", "attend"), ("ouvre", "ferme"),
         ("regarde", "écoute"), ("porte", "tire"), ("apporte", "emporte"), ("montre", "cache"), ("garde", "prête")]


def plain():
    s = R.choice(SUBJ)[0]
    v = R.choice(VERB_INF)[0]
    n = R.choice(NOUN)
    extra = R.choice(PLACES)
    if R.random() < 0.5:
        return f"{s} {v} le {n} {R.choice(TIME)}.", None
    a, b = R.choice(VERB2)
    return f"{s} {a} le {n} et {b} {R.choice(NOUN)} {extra} {R.choice(TIME)}.", None


BUILDERS = [agree, participle, homophone, mood, numbers, spacing, dup, apos, guillemets, hyphen, edge, plain]


def tail():
    """A varied, correct closing clause: the pools give tens of thousands of distinct tails."""
    return f"{R.choice(PLACES)} {R.choice(TIME)}, avec {R.randint(2, 99)} {R.choice(NOUN)}s"


def main():
    lines, used = [], set()
    for k in range(N):
        if k % 120 == 0:
            text = R.choice(LONG) + "."
        else:
            text = R.choice(BUILDERS)()[0]
            if text in used:  # the small builders repeat; vary the sentence, never copy a corpus sentence
                text = f"{text[:-1]}, {tail()}."
        for _ in range(50):
            if text not in used:
                break
            text = f"{R.choice(BUILDERS)()[0][:-1]}, {tail()}."
        text = text.replace("\n", " ")
        used.add(text)
        lines.append(text)
    assert len(lines) == N
    assert len(used) > N - 20, f"{N - len(used)} duplicates"
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{OUT}: {N} lines")


if __name__ == "__main__":
    main()
