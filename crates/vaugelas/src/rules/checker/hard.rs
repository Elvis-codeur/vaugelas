//! Rules for the "hard phenomena" catalog (hardloop): inversion, quantities, coordination of persons,
//! participles in special constructions, adjectives with special agreement, mood triggers.
use super::*;

#[allow(dead_code)]
const SPEECH: &[&str] = &[
    "dire", "répondre", "demander", "écrier", "ajouter", "expliquer", "murmurer", "crier", "répliquer", "déclarer",
    "affirmer", "souffler", "lancer", "conclure", "poursuivre", "reprendre", "interrompre", "observer", "rétorquer",
    "insister", "annoncer", "préciser", "soupirer", "s’exclamer", "exclamer", "objecter", "protester", "assurer",
];
const OPENER_ADV: &[&str] = &[
    "ici", "là", "là-bas", "ensuite", "puis", "alors", "bientôt", "soudain", "enfin", "aussitôt", "partout",
    "ailleurs", "dehors", "dedans", "ainsi", "devant", "derrière", "au-dessus", "plus loin", "après",
];
const OBJ_CLITICS: &[&str] = &["le", "la", "l’", "les", "me", "m’", "te", "t’", "nous", "vous", "se", "s’"];
const PRON_INDIRECT: &[&str] = &[
    "téléphoner", "parler", "succéder", "plaire", "déplaire", "complaire", "nuire", "mentir", "ressembler",
    "sourire", "rire", "suffire", "convenir", "répondre", "écrire", "dire", "demander", "souhaiter", "permettre",
    "promettre", "rendre", "succéder", "survivre", "en vouloir",
];
const SUBJ_VERBS: &[&str] = &[
    "vouloir", "souhaiter", "désirer", "exiger", "ordonner", "douter", "craindre", "regretter", "préférer",
    "attendre", "refuser", "interdire", "accepter", "aimer", "redouter", "déplorer", "falloir", "demander",
    "permettre", "empêcher", "consentir", "tolérer", "supporter", "apprécier", "défendre", "proposer", "suggérer",
];
const SUBJ_ADJ: &[&str] = &[
    "possible", "impossible", "nécessaire", "important", "essentiel", "indispensable", "normal", "naturel",
    "dommage", "étonnant", "urgent", "préférable", "juste", "rare", "inutile", "utile", "souhaitable",
    "regrettable", "honteux", "bon", "content", "heureux", "triste", "désolé", "surpris", "ravi", "fier",
    "étonné", "furieux", "fâché", "déçu", "temps", "peu probable", "douteux", "inadmissible", "inconcevable",
];
const OPINION: &[&str] = &["penser", "croire", "trouver", "estimer", "imaginer", "considérer", "affirmer", "dire", "espérer"];
const COLOUR_NOUNS: &[&str] = &[
    "marron", "orange", "cerise", "olive", "crème", "chocolat", "kaki", "turquoise", "saumon", "citron", "émeraude",
    "ivoire", "moutarde", "prune", "sable", "paille", "framboise", "caramel", "corail", "noisette", "abricot", "bordeaux",
];
const COLOUR_ADJ: &[&str] = &["bleu", "vert", "rouge", "jaune", "gris", "noir", "blanc", "brun", "violet", "beige"];
const COLOUR_QUAL: &[&str] = &[
    "clair", "claire", "clairs", "claires", "foncé", "foncée", "foncés", "foncées", "pâle", "pâles", "vif", "vive",
    "vifs", "vives", "sombre", "sombres", "marine", "ciel", "nuit", "pastel", "électrique", "canard", "roi",
];
const NUM_WORDS: &[&str] = &[
    "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze", "douze", "treize", "quatorze",
    "quinze", "seize", "vingt", "trente", "quarante", "cinquante", "soixante", "cent", "cents", "mille", "un", "une",
    "dix-sept", "dix-huit", "dix-neuf", "quatre-vingt", "quatre-vingts",
];

fn stressed(l: &str) -> Option<(u8, u8)> {
    let l = l.split('-').next().unwrap_or(l);
    Some(match l {
        "moi" => (1, 1), "toi" => (2, 1), "lui" | "elle" => (4, 1), "nous" => (1, 2), "vous" => (2, 2), "eux" | "elles" => (4, 2),
        _ => return None,
    })
}
fn ms_only(m: &Morph) -> bool { m.gender & G_M != 0 && m.number & N_S != 0 }
fn vowel(s: &str) -> bool {
    let c = s.chars().next().unwrap_or('x');
    "aeiouyéèêëàâîïôûœhAEIOUYÉÈÊÀÂÎÔÛH".contains(c)
}

impl Checker {
    pub(super) fn hard_rules(&self, t: &[Tok], o: &mut Vec<Lint>) {
        self.inv_compound(t, o);
        self.inv_opener_question(t, o);
        self.incise(t, o);
        self.qui_pronoun(t, o);
        self.quantity_subject(t, o);
        self.coord_persons(t, o);
        self.summary_subject(t, o);
        self.pp_interrogative(t, o);
        self.pp_en(t, o);
        self.pp_no_preceding_object(t, o);
        self.pp_pron_indirect(t, o);
        self.pp_detached(t, o);
        self.prep_participles(t, o);
        self.des_plus(t, o);
        self.colours(t, o);
        self.demi(t, o);
        self.tout_adverb(t, o);
        self.quel_que(t, o);
        self.numbers(t, o);
        self.mood_triggers(t, o);
        self.si_future_conditional(t, o);
        self.imperative_s(t, o);
        self.on_negation(t, o);
        self.pronoun_participle(t, o);
        self.floating_tous(t, o);
        self.small_fixed(t, o);
        self.compound_repair(t, o);
        self.lun_lautre(t, o);
        self.tel_que(t, o);
        self.present_participle(t, o);
        self.quelque_adverb(t, o);
        self.toute_before_vowel(t, o);
        self.capitals(t, o);
        self.dont_relative(t, o);
        self.lequel(t, o);
        self.ne_que(t, o);
        self.pallier(t, o);
        self.au_cas_ou(t, o);
        self.en_y(t, o);
        self.verb_constructions(t, o);
        self.ce_etre(t, o);
        self.restent(t, o);
        self.clause_subject(t, o);
        self.pp_clitic_object(t, o);
        self.pp_pron_relative(t, o);
        self.fait_infinitive(t, o);
        self.object_attribute(t, o);
        self.inverted_copula_attr(t, o);
        self.ne_que_any(t, o);
        self.dont_parler(t, o);
        self.adj_verbal(t, o);
        self.misc_lexical(t, o);
        self.hypothetical_future(t, o);
        self.pronoun_infinitive(t, o);
        self.impersonal_participle(t, o);
    }

    /// "Le musée ouvre que le mardi" -> n'ouvre ... que (only when a phrase, not a clause, follows "que")
    fn ne_que_any(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const QUE_VERBS: &[&str] = &["dire", "penser", "croire", "savoir", "vouloir", "falloir", "sembler", "paraître", "espérer", "craindre", "douter", "demander",
            "affirmer", "arriver", "importer", "suffire", "trouver", "estimer", "souhaiter", "prétendre", "annoncer", "expliquer", "répondre", "promettre", "voir",
            "comprendre", "apprendre", "constater", "remarquer", "rester", "convenir", "valoir", "attendre", "préférer", "exiger", "aimer", "admettre", "reconnaître",
            "considérer", "juger", "imaginer", "supposer", "ajouter", "écrire", "lire", "entendre", "oublier", "rappeler", "montrer", "prouver", "signifier",
            "objecter", "souvenir", "garder", "noter", "observer", "répliquer", "avouer", "soutenir", "déclarer", "nier", "conclure", "insinuer", "sentir",
            "deviner", "découvrir", "décider", "ordonner", "permettre", "défendre", "interdire", "accorder", "obtenir", "mériter", "assurer", "jurer",
            "raconter", "enseigner", "démontrer", "établir", "reconnaître", "concevoir", "comprendre", "rêver", "voir", "craindre", "redouter",
            "rapporter", "répéter", "soupçonner", "certifier", "préciser", "souligner", "signaler", "indiquer", "avertir", "prévenir", "écrier", "exprimer", "proclamer", "enseigner"];
        for v in 0..t.len().saturating_sub(2) {
            if !t[v].word || !has_fin(&t[v]) || !Self::word_at(t, v + 1) { continue; }
            let (head, q) = if t[v + 1].morph.iter().any(|m| m.moods & M_Q != 0) && (self.fin_lemma(&t[v], "avoir") || self.fin_lemma(&t[v], "être")) { (v + 1, v + 2) } else { (v, v + 1) };
            if !Self::word_at(t, q) || !matches!(t[q].lower.as_str(), "que" | "qu’") || !Self::word_at(t, q + 1) { continue; }
            if t[head].morph.iter().any(|m| QUE_VERBS.contains(&self.lemma_of(m))) || t[head].lower == "est" || t[head].inv.is_some() || t[head].compound { continue; }
            if has_pos(&t[head], Pos::Noun) && head > 0 && t[head - 1].word && det_info(&t[head - 1]).is_some() { continue; } // "ce point que", "de sorte que"
            if (v.saturating_sub(5)..q).any(|k| t[k].word && matches!(t[k].lower.as_str(), "plus" | "moins" | "autant" | "aussi" | "tel" | "telle" | "tels" | "telles" | "même" | "mêmes" | "autre" | "autres" | "ne" | "n’" | "si" | "tant" | "mieux" | "pire" | "rien" | "jamais" | "personne" | "c’" | "ce")) { continue; }
            let ss = (0..=v).rev().find(|&k| t[k].sent_start).unwrap_or(0);
            if (ss..v).any(|k| t[k].word && matches!(t[k].lower.as_str(), "ne" | "n’")) { continue; }
            let nx = &t[q + 1];
            let phrase_start = det_info(nx).is_some() || NUM_WORDS.contains(&nx.lower.as_str()) || nx.lower.chars().all(|c| c.is_ascii_digit()) || matches!(nx.lower.as_str(), "depuis" | "pour" | "par" | "dans" | "à" | "en");
            if !phrase_start { continue; }
            // the restricted phrase must close (punctuation or sentence end) within 7 words, with no verb in it
            let close = (q + 1..(q + 9).min(t.len())).find(|&k| !t[k].word || t[k].sid != t[q].sid);
            let Some(cl) = close else { continue };
            if (q + 1..cl).any(|k| has_fin(&t[k]) && !has_pos(&t[k], Pos::Noun) && !CLITICS.contains(&t[k].lower.as_str())) { continue; }
            let sug = if vowel(&t[v].lower) { format!("n’{}", t[v].text) } else { format!("ne {}", t[v].text) };
            o.push(self.mk(&t[v], "FR_NE_QUE", "Restriction « ne … que » : le « ne » est obligatoire à l’écrit.", vec![sug]));
        }
    }

    /// "Les difficultés que parlent les habitants" -> dont (parler de), not for languages
    fn dont_parler(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const LANG_NOUNS: &[&str] = &["langue", "langues", "dialecte", "dialectes", "patois", "langage", "jargon", "argot", "créole", "idiome"];
        for q in 1..t.len() {
            if !t[q].word || !matches!(t[q].lower.as_str(), "que" | "qu’") || !t[q - 1].word || !has_pos(&t[q - 1], Pos::Noun) || has_fin(&t[q - 1]) { continue; }
            if TIME_NOUNS.contains(&t[q - 1].lower.as_str()) || matches!(t[q - 1].lower.as_str(), "fois" | "jours" | "moments") { continue; }
            let ss = (0..q).rev().find(|&k| t[k].sent_start).unwrap_or(0);
            if (ss..q).any(|k| t[k].word && matches!(t[k].lower.as_str(), "c’" | "ce") && Self::word_at(t, k + 1) && self.fin_lemma(&t[k + 1], "être")) { continue; } // cleft "c'est à X que"
            if LANG_NOUNS.contains(&t[q - 1].lower.as_str()) || t[q - 1].morph.iter().any(|m| m.pos == Pos::Noun && ["français", "anglais", "italien", "espagnol", "allemand", "arabe", "chinois", "russe"].contains(&self.lemma_of(m))) { continue; }
            let end = (q + 1..(q + 14).min(t.len())).take_while(|&k| t[k].sid == t[q].sid).last().unwrap_or(q);
            let mut k = q + 1;
            while k <= end {
                if !t[k].word {
                    if matches!(t[k].text.as_str(), "," | "—") { if let Some(c) = (k + 1..=end).find(|&x| !t[x].word && t[x].text == t[k].text) { k = c + 1; continue; } }
                    break;
                }
                if t[k].word && matches!(t[k].lower.as_str(), "dont" | "que" | "qu’" | "qui") { break; }
                if t[k].word && t[k].morph.iter().any(|m| (is_fin(m) || m.moods & M_Q != 0) && self.lemma_of(m) == "parler") {
                    let det_after = Self::word_at(t, k + 1) && det_info(&t[k + 1]).is_some() && !matches!(t[k + 1].lower.as_str(), "de" | "d’" | "du" | "des");
                    // "que parlent les habitants": the noun phrase after the verb is its inverted subject, not an object
                    let inverted = det_after && t[k].morph.iter().any(is_fin) && !(q + 1..k).any(|x| t[x].word && (pron_pn(&t[x].lower).is_some() || det_info(&t[x]).is_some()));
                    if !det_after || inverted { o.push(self.mk(&t[q], "FR_DONT", "« parler de quelque chose » : le relatif est « dont ».", vec!["dont".into()])); }
                    break;
                }
                if t[k].word && self.verbish(&t[k]) && !(self.fin_lemma(&t[k], "avoir") && Self::word_at(t, k + 1) && t[k + 1].morph.iter().any(|m| m.moods & M_Q != 0)) { break; }
                k += 1;
            }
        }
    }

    /// "Son explication était convainquant" -> convaincante (verbal adjective spelling)
    fn adj_verbal(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const PAIRS: &[(&str, &str)] = &[("convainquant", "convaincant"), ("fatiguant", "fatigant"), ("provoquant", "provocant"), ("communiquant", "communicant"),
            ("suffoquant", "suffocant"), ("intriguant", "intrigant"), ("naviguant", "navigant"), ("extravaguant", "extravagant"), ("zigzaguant", "zigzagant"),
            ("différant", "différent"), ("négligeant", "négligent"), ("précédant", "précédent"), ("excellant", "excellent"), ("divergeant", "divergent"),
            ("influant", "influent"), ("équivalant", "équivalent"), ("adhérant", "adhérent"), ("affluant", "affluent"), ("émergeant", "émergent"),
            ("convergeant", "convergent"), ("somnolant", "somnolent"), ("résidant", "résident"), ("vaquant", "vacant")];
        for i in 1..t.len() {
            if !t[i].word { continue; }
            let Some((_, adj)) = PAIRS.iter().find(|(p, _)| t[i].lower == *p || t[i].lower.strip_suffix('s') == Some(*p)) else { continue };
            let copula = ["être", "sembler", "paraître", "devenir", "rester"].iter().any(|l| self.fin_lemma(&t[i - 1], l) || t[i - 1].morph.iter().any(|m| m.moods & M_Q != 0 && self.lemma_of(m) == *l))
                || (i >= 2 && ADVERBS.contains(&t[i - 1].lower.as_str()) && ["être", "sembler", "paraître", "devenir", "rester"].iter().any(|l| self.fin_lemma(&t[i - 2], l)));
            let after_noun = t[i - 1].word && has_pos(&t[i - 1], Pos::Noun) && !has_fin(&t[i - 1]);
            let followed_by_object = Self::word_at(t, i + 1) && (det_info(&t[i + 1]).is_some() || CLITICS.contains(&t[i + 1].lower.as_str()));
            if !(copula || after_noun) || followed_by_object { continue; }
            let (g, n) = if after_noun {
                let g = t[i - 1].morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender);
                let n = t[i - 1].morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number);
                (g, n)
            } else {
                let ss = (0..i).rev().find(|&k| t[k].sent_start).unwrap_or(0);
                self.np_at(t, ss).map(|np| (np.gender, np.number)).unwrap_or((G_M, N_S))
            };
            let mut sug = self.inflect_adj(adj, false, if single(g) { g } else { G_M }, if single(n) { n } else { N_S });
            if sug.is_empty() { sug = vec![adj.to_string()]; }
            o.push(self.mk(&t[i], "FR_ATTR_AGREE", "Adjectif verbal : orthographe différente du participe présent (convaincant, fatigant…).", sug));
        }
    }

    /// Small lexical rules: quelque/quelques, même adverb, rose, nôtre, gens, toute + mute h
    fn misc_lexical(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word { continue; }
            let l = t[i].lower.as_str();
            let nx = if Self::word_at(t, i + 1) { Some(&t[i + 1]) } else { None };
            if l == "quelque" {
                if let Some(n) = nx {
                    if all_pos(n, Pos::Noun) && n.morph.iter().all(|m| m.number == N_P) && n.lower != "chose" {
                        o.push(self.mk(&t[i], "FR_DET_NOUN_NUM", "« quelques » devant un nom pluriel.", vec!["quelques".into()]));
                    }
                }
            }
            if l == "mêmes" && t[i].sent_start {
                if let Some(n) = nx { if det_info(n).is_some() { o.push(self.mk(&t[i], "FR_ATTR_AGREE", "« même » adverbe (= aussi) est invariable.", vec!["même".into()])); } }
            }
            if matches!(l, "nôtre" | "vôtre") {
                if let Some(n) = nx {
                    if has_pos(n, Pos::Noun) && !(i > 0 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "le" | "la" | "les")) {
                        o.push(self.mk(&t[i], "FR_HOMOPHONE", "Déterminant possessif sans accent : notre, votre (« le nôtre » est un pronom).", vec![if l == "nôtre" { "notre" } else { "votre" }.to_string()]));
                    }
                }
            }
            if matches!(l, "rose" | "mauve" | "fauve" | "pourpre" | "écarlate") && i > 0 && t[i - 1].word
                && t[i - 1].morph.iter().any(|m| m.pos == Pos::Noun && m.number == N_P) && !t[i - 1].morph.iter().any(|m| m.number & N_S != 0)
                && !nx.map_or(false, |n| COLOUR_QUAL.contains(&n.lower.as_str())) {
                o.push(self.mk(&t[i], "FR_COLOUR", "« rose, mauve, fauve, pourpre, écarlate » s’accordent.", vec![format!("{l}s")]));
            }
            if nx.map_or(false, |n| n.lower == "gens") {
                let fem = match l { "vieux" => Some("vieilles"), "bons" => Some("bonnes"), "petits" => Some("petites"), "certains" => Some("certaines"), "méchants" => Some("méchantes"), "sots" => Some("sottes"), _ => None };
                if let Some(f) = fem { o.push(self.mk(&t[i], "FR_ATTR_AGREE", "Un adjectif placé juste avant « gens » se met au féminin.", vec![f.to_string()])); }
            }
            if l == "toute" && i > 0 && t[i - 1].word && self.fin_lemma(&t[i - 1], "être") {
                if let Some(n) = nx {
                    if n.lower.starts_with('h') && n.morph.iter().any(adjish) && !has_pos(n, Pos::Noun)
                        && !["honteuse", "hardie", "haute", "hâtive", "hautaine", "hargneuse", "hagarde", "haïe"].contains(&n.lower.as_str()) {
                        o.push(self.mk(&t[i], "FR_TOUT", "« tout » adverbe reste invariable devant un h muet.", vec!["tout".into()]));
                    }
                }
            }
        }
    }

    /// "Si j'avais le choix ... je choisirai" anywhere in the sentence, "À ta place, je demanderai" -> conditional
    fn hypothetical_future(&self, t: &[Tok], o: &mut Vec<Lint>) {
        let mut s = 0;
        while s < t.len() {
            let e = (s..t.len()).take_while(|&k| t[k].sid == t[s].sid).last().unwrap_or(s);
            let si_imparfait = (s..=e).any(|k| t[k].word && matches!(t[k].lower.as_str(), "si" | "s’")
                && (k + 1..(k + 5).min(e + 1)).any(|j| t[j].word && t[j].morph.iter().any(|m| m.moods & M_IQ != 0) && !t[j].morph.iter().any(|m| m.moods & M_IP != 0)));
            let a_ta_place = (s..e.saturating_sub(1)).any(|k| t[k].word && t[k].lower == "à" && matches!(t[k + 1].lower.as_str(), "ta" | "votre" | "sa" | "leur" | "ma" | "notre") && t.get(k + 2).map_or(false, |x| x.lower == "place"));
            if si_imparfait || a_ta_place {
                for k in s..=e {
                    let x = &t[k];
                    if !x.word || x.lower.ends_with("rais") || !x.lower.ends_with("rai") { continue; }
                    if !(k > 0 && t[k - 1].word && matches!(t[k - 1].lower.as_str(), "je" | "j’")) { continue; }
                    let Some(r) = x.morph.iter().find(|m| m.moods & M_IF != 0 && m.person & 1 != 0) else { continue };
                    let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_K != 0 && f.person & 1 != 0 && f.number & N_S != 0);
                    o.push(self.mk(x, "FR_SI_COND", "Hypothèse (si + imparfait, à ta place…) : conditionnel en -rais, pas futur.", sug));
                }
            }
            s = e + 1;
        }
    }

    /// "Vous recueillir les témoignages" -> recueillez (subject pronoun + infinitive at a clause start)
    fn pronoun_infinitive(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !(t[i].sent_start || (i > 0 && !t[i - 1].word)) { continue; }
            let Some((p, n)) = pron_pn(&t[i].lower) else { continue };
            if matches!(t[i].lower.as_str(), "nous" | "vous") { continue; }
            let mut v = i + 1;
            while Self::word_at(t, v) && CLITICS.contains(&t[v].lower.as_str()) && v <= i + 2 { v += 1; }
            if !Self::word_at(t, v) { continue; }
            let x = &t[v];
            if x.morph.is_empty() || !x.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_Y != 0) { continue; }
            let lemma = self.lemma_of(&x.morph[0]).to_string();
            let sug = self.forms_where(&lemma, |f| f.pos == Pos::Verb && f.moods & M_IP != 0 && f.person & p != 0 && f.number & n != 0);
            o.push(self.mk(x, "FR_ER_E", "Après un pronom sujet, il faut un verbe conjugué, pas un infinitif.", sug));
        }
    }

    /// "Il s'est glissées des erreurs" -> glissé ; "les accidents qu'il est arrivés" -> arrivé (impersonal il)
    fn impersonal_participle(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || t[i].lower != "il" { continue; }
            let mut k = i + 1;
            if Self::word_at(t, k) && matches!(t[k].lower.as_str(), "s’" | "se" | "y") { k += 1; }
            if !Self::word_at(t, k) || !(self.fin_lemma(&t[k], "être") || self.fin_lemma(&t[k], "avoir")) { continue; }
            let p = k + 1;
            if !Self::word_at(t, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| ms_only(m)) { continue; }
            let real_subject_after = Self::word_at(t, p + 1) && det_info(&t[p + 1]).is_some();
            let que_before = i >= 1 && matches!(t[i - 1].lower.as_str(), "que" | "qu’");
            if !(real_subject_after || que_before) { continue; }
            if !qr.iter().any(|m| matches!(self.lemma_of(m), "arriver" | "glisser" | "produire" | "passer" | "falloir" | "manquer" | "rester" | "venir" | "agir" | "écouler" | "trouver" | "présenter")) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, G_M, N_S);
            o.push(self.mk(&t[p], "FR_PP_INVARIABLE", "Construction impersonnelle (il …) : participe invariable.", sug));
        }
    }


    /// "C'était les vacances" -> C'étaient ; "C'étaient une soirée" -> C'était
    fn ce_etre(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "c’" | "ce") || !self.fin_lemma(&t[i + 1], "être") { continue; }
            let k = i + 2;
            let Some((_, dn)) = det_info(&t[k]) else { continue };
            if !single(dn) || matches!(t[k].lower.as_str(), "de" | "d’" | "du") { continue; }
            let Some(np) = self.np_core(t, k) else { continue };
            if !single(np.number) || np.number != dn { continue; }
            let v = &t[i + 1];
            let plural = v.morph.iter().any(|m| is_fin(m) && m.number == N_P);
            let singular = v.morph.iter().any(|m| is_fin(m) && m.number == N_S);
            if (np.number == N_P && singular && !plural) || (np.number == N_S && plural && !singular) {
                let sug = self.verb_forms_for(v, 4, np.number);
                o.push(self.mk(v, "FR_SUBJ_VERB", "« c’est » devant un nom singulier, « ce sont » devant un nom pluriel.", sug));
            }
        }
    }

    /// "Restent une énigme" -> Reste (the plural needs a plural subject; "Reste les questions" is tolerated)
    fn restent(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !t[s].sent_start || !t[s].morph.iter().any(|m| is_fin(m) && matches!(self.lemma_of(m), "rester" | "suivre" | "venir" | "figurer" | "manquer")) { continue; }
            if !t[s].morph.iter().any(|m| is_fin(m) && m.number == N_P) || t[s].morph.iter().any(|m| is_fin(m) && m.number == N_S) { continue; }
            let mut k = s + 1;
            while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || matches!(t[k].lower.as_str(), "ensuite" | "enfin" | "encore")) { k += 1; }
            let Some(np) = self.np_at(t, k) else { continue };
            if np.number != N_S { continue; }
            let sug = self.verb_forms_for(&t[s], 4, N_S);
            o.push(self.mk(&t[s], "FR_SUBJ_VERB", "Le verbe s’accorde avec le sujet placé après lui.", sug));
        }
    }

    /// "Que tu viennes me font plaisir" -> fait ; "Voyager me plaisent" -> plaît
    fn clause_subject(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !t[s].sent_start { continue; }
            let start_que = matches!(t[s].lower.as_str(), "que" | "qu’");
            let start_inf = t[s].morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_Y != 0) && !t[s].morph.is_empty();
            if !start_que && !start_inf { continue; }
            // first finite verb (the clause's own verb for "que", none for an infinitive), then the main verb
            let mut k = s + 1;
            let mut seen_clause_verb = !start_que;
            let mut main = None;
            while k < t.len() && k <= s + 14 && t[k].sid == t[s].sid {
                if !t[k].word { break; }
                if has_fin(&t[k]) && t[k].morph.iter().all(|m| m.pos == Pos::Verb) && !CLITICS.contains(&t[k].lower.as_str()) {
                    if seen_clause_verb { main = Some(k); break; }
                    seen_clause_verb = true;
                }
                k += 1;
            }
            let Some(v) = main else { continue };
            if v > 0 && !(CLITICS.contains(&t[v - 1].lower.as_str()) || matches!(t[v - 1].lower.as_str(), "lui" | "leur") || has_pos(&t[v - 1], Pos::Noun) || t[v - 1].morph.iter().any(adjish) || has_pos(&t[v - 1], Pos::Adv)) { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & N_S != 0) || !t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], 4, N_S);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Un infinitif ou une proposition sujet commande le singulier.", sug));
        }
    }

    /// "Ces fleurs, je les ai cueilli" -> cueillies ; "Ta lettre, je l'ai lu" -> lue
    fn pp_clitic_object(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for c in 1..t.len().saturating_sub(2) {
            let l = t[c].lower.as_str();
            if !t[c].word || !matches!(l, "la" | "les" | "l’") { continue; }
            if !(has_pos(&t[c - 1], Pos::PronSubj) || matches!(t[c - 1].lower.as_str(), "ne" | "n’") || (t[c - 1].word && has_pos(&t[c - 1], Pos::Noun))) { continue; }
            let mut a = c + 1;
            if Self::word_at(t, a) && matches!(t[a].lower.as_str(), "lui" | "leur" | "y") { a += 1; }
            if !Self::word_at(t, a) || !self.fin_lemma(&t[a], "avoir") { continue; }
            let mut p = a + 1;
            while Self::word_at(t, p) && (matches!(t[p].lower.as_str(), "pas" | "plus" | "jamais" | "déjà" | "bien" | "toujours") || all_pos(&t[p], Pos::Adv)) { p += 1; }
            if !Self::word_at(t, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| PP_INVARIABLE.contains(&self.lemma_of(m)) || matches!(self.lemma_of(m), "faire" | "laisser")) { continue; }
            if Self::word_at(t, p + 1) && t[p + 1].morph.iter().any(|m| m.moods & M_Y != 0) { continue; }
            // referent: a dislocated noun phrase at the start of the sentence, else the clitic itself
            let ss = (0..=c).rev().find(|&k| t[k].sent_start).unwrap_or(0);
            let disloc = self.np_at(t, ss).filter(|np| t.get(np.end + 1).map_or(false, |x| x.text == ","));
            let (g, n) = match (l, disloc) {
                (_, Some(np)) if single(np.number) => (np.gender, np.number),
                ("la", _) => (G_F, N_S), ("les", _) => (3, N_P), _ => continue,
            };
            if l == "les" && n != N_P || l == "la" && n != N_S { continue; }
            if qr.iter().any(|m| m.number & n != 0 && (m.gender & g != 0 || !single(g)) && !(m.gender == 3 && m.number == 3)) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, if single(g) { g } else { G_M }, n);
            o.push(self.mk(&t[p], "FR_PP_COD", "Le participe s’accorde avec le pronom complément placé avant (le, la, les).", sug));
        }
    }

    /// "Les mains qu'elle s'est lavé" -> lavées (the object "que" precedes)
    fn pp_pron_relative(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            let Some(np) = self.np_core(t, i) else { continue };
            let q = np.end + 1;
            if !Self::word_at(t, q) || !matches!(t[q].lower.as_str(), "que" | "qu’") || !single(np.number) || !single(np.gender) { continue; }
            if np.gender == G_M && np.number == N_S { continue; }
            let mut j = q + 1;
            if Self::word_at(t, j) && pron_pn(&t[j].lower).is_some() { j += 1; } else if let Some(n2) = self.np_core(t, j) { j = n2.end + 1; } else { continue; }
            while Self::word_at(t, j) && matches!(t[j].lower.as_str(), "ne" | "n’") { j += 1; }
            if !Self::word_at(t, j) || !matches!(t[j].lower.as_str(), "se" | "s’" | "me" | "m’" | "te" | "t’" | "nous" | "vous") { continue; }
            let e = j + 1;
            if !Self::word_at(t, e) || !self.fin_lemma(&t[e], "être") { continue; }
            let p = e + 1;
            if !Self::word_at(t, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() { continue; }
            if qr.iter().any(|m| m.gender & np.gender != 0 && m.number & np.number != 0 && !(m.gender == 3 && m.number == 3)) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, np.gender, np.number);
            o.push(self.mk(&t[p], "FR_PP_PRONOMINAL", "Le complément direct (« que ») est placé avant : le participe s’accorde avec lui.", sug));
        }
    }

    /// "la maison qu'il a faite construire" -> fait
    fn fait_infinitive(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "faite" | "faits" | "faites") { continue; }
            if !(self.fin_lemma(&t[i - 1], "avoir") || self.fin_lemma(&t[i - 1], "être")) { continue; }
            if !t[i + 1].morph.iter().any(|m| m.moods & M_Y != 0) || has_pos(&t[i + 1], Pos::Noun) && !t[i + 1].lower.ends_with('r') { continue; }
            o.push(self.mk(&t[i], "FR_PP_INVARIABLE", "« fait » suivi d’un infinitif est toujours invariable.", vec!["fait".into()]));
        }
    }

    /// "Je les trouve remarquable" -> remarquables
    fn object_attribute(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const JUDGE: &[&str] = &["trouver", "juger", "considérer", "estimer", "croire", "rendre", "imaginer", "déclarer", "supposer", "savoir"];
        for c in 0..t.len().saturating_sub(2) {
            let l = t[c].lower.as_str();
            if !t[c].word || !matches!(l, "les" | "la") { continue; }
            let v = c + 1;
            if !Self::word_at(t, v) || !t[v].morph.iter().any(|m| (is_fin(m) || m.moods & M_Q != 0) && JUDGE.contains(&self.lemma_of(m))) { continue; }
            if !(c > 0 && t[c - 1].word && (has_pos(&t[c - 1], Pos::PronSubj) || matches!(t[c - 1].lower.as_str(), "ne" | "n’"))) { continue; }
            let mut k = v + 1;
            if k < t.len() && !t[k].word && t[k].text == "," { if let Some(cl) = (k + 1..(k + 15).min(t.len())).find(|&x| !t[x].word && t[x].text == ",") { k = cl + 1; } }
            while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv) || t[k].lower == "comme") { k += 1; }
            if !Self::word_at(t, k) { continue; }
            let a = &t[k];
            if !has_pos(a, Pos::Adj) || has_pos(a, Pos::Noun) || has_fin(a) || a.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv) || m.pos.is_det()) { continue; }
            let n = if l == "les" { N_P } else { N_S };
            let g = if l == "la" { G_F } else { 3 };
            if a.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0)) { continue; }
            let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { continue };
            let sug = self.inflect_adj(self.lemma_of(m0), false, if single(g) { g } else { m0.gender }, n);
            o.push(self.mk(a, "FR_ATTR_AGREE", "L’attribut du complément s’accorde avec lui.", sug));
        }
    }

    /// "Ces mesures te paraissent-elles nécessaire ?" -> nécessaires
    fn inverted_copula_attr(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for v in 0..t.len().saturating_sub(1) {
            let Some((_, n)) = t[v].inv else { continue };
            if !["être", "sembler", "paraître", "devenir", "rester"].iter().any(|l| self.fin_lemma(&t[v], l)) { continue; }
            let last = t[v].lower.rsplit('-').next().unwrap_or("");
            let g = match last { "il" | "ils" => G_M, "elle" | "elles" => G_F, _ => 3 };
            let mut k = v + 1;
            while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) { k += 1; }
            if !Self::word_at(t, k) || !single(n) { continue; }
            let a = &t[k];
            if !a.morph.iter().any(adjish) || has_pos(a, Pos::Noun) || a.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv) || m.pos.is_det()) { continue; }
            if a.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0 || !single(g))) { continue; }
            let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { continue };
            let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, if single(g) { g } else { m0.gender }, n);
            o.push(self.mk(a, "FR_ATTR_AGREE", "L’attribut s’accorde avec le sujet inversé.", sug));
        }
    }

    /// Verbs with a fixed construction: se rappeler qqch, se souvenir de qqch, téléphoner à qqn, aider qqn,
    /// permettre à qqn de, jouer à (sport) / de (instrument), penser de (opinion), bien que (not malgré que).
    fn verb_constructions(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const SPORTS: &[&str] = &["football", "foot", "tennis", "rugby", "basket", "basket-ball", "volley", "volley-ball", "handball", "golf", "hockey", "ping-pong", "badminton", "échecs", "cartes", "billard", "baseball", "polo", "squash", "bowling", "dames", "pétanque", "boules"];
        const INSTR: &[&str] = &["piano", "violon", "guitare", "flûte", "batterie", "trompette", "violoncelle", "saxophone", "harpe", "accordéon", "clarinette", "orgue", "tambour", "contrebasse", "hautbois", "trombone", "cor", "basson", "ukulélé", "banjo", "synthétiseur", "xylophone"];
        let lemma_is = |x: &Tok, l: &str| x.morph.iter().any(|m| self.lemma_of(m) == l);
        for i in 0..t.len() {
            if !t[i].word { continue; }
            let nx = |k: usize| if Self::word_at(t, i + k) && t[i + k].sid == t[i].sid { Some(&t[i + k]) } else { None };
            // malgré que -> bien que
            if t[i].lower == "malgré" && nx(1).map_or(false, |x| matches!(x.lower.as_str(), "que" | "qu’")) {
                o.push(self.mk(&t[i], "FR_PREP", "« malgré que » est fautif : « bien que » (+ subjonctif).", vec!["bien".into()]));
            }
            let refl = i > 0 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "me" | "m’" | "te" | "t’" | "se" | "s’" | "nous" | "vous");
            // se rappeler du nom -> le nom
            if refl && lemma_is(&t[i], "rappeler") && has_fin(&t[i]) {
                if let Some(p) = nx(1) {
                    let after = nx(2);
                    let inf = after.map_or(false, |a| a.morph.iter().any(|m| m.moods & M_Y != 0));
                    let sug = match p.lower.as_str() { "du" => Some("le"), "de" if after.map_or(false, |a| matches!(a.lower.as_str(), "la" | "l’")) => Some(""), "d’" if !inf => Some(""), _ => None };
                    if let Some(s) = sug { o.push(self.mk(p, "FR_PREP", "« se rappeler quelque chose » se construit sans « de ».", if s.is_empty() { vec![] } else { vec![s.to_string()] })); }
                }
            }
            let noun_use = i > 0 && t[i - 1].word && det_info(&t[i - 1]).is_some() && has_pos(&t[i], Pos::Noun);
            let dont_before = (i.saturating_sub(5)..i).any(|k| t[k].word && t[k].lower == "dont");
            // se souvenir le / un ... -> du / d'un
            if refl && lemma_is(&t[i], "souvenir") && has_fin(&t[i]) && !dont_before && !(Self::word_at(t, i + 2) && matches!(t[i + 2].lower.as_str(), "mieux" | "plus" | "moins")) {
                if let Some(p) = nx(1) {
                    let sug = match p.lower.as_str() { "le" => Some("du"), "la" => Some("de la"), "les" => Some("des"), "l’" => Some("de l’"), "un" => Some("d’un"), "une" => Some("d’une"),
                        "avoir" | "être" => Some(if p.lower == "avoir" { "d’avoir" } else { "d’être" }), _ => None };
                    if let Some(s) = sug { o.push(self.mk(p, "FR_PREP", "« se souvenir de quelque chose ».", vec![s.to_string()])); }
                }
            }
            // téléphoner le plombier -> au plombier ; le téléphone -> lui téléphone
            if lemma_is(&t[i], "téléphoner") && !noun_use && t[i].morph.iter().any(|m| is_fin(m) || m.moods & (M_Q | M_Y) != 0) {
                if let Some(p) = nx(1) {
                    if let Some(s) = match p.lower.as_str() { "le" => Some("au"), "la" => Some("à la"), "les" => Some("aux"), "l’" => Some("à l’"), _ => None } {
                        if nx(2).map_or(false, |n| has_pos(n, Pos::Noun)) { o.push(self.mk(p, "FR_PREP", "On téléphone « à » quelqu’un.", vec![s.to_string()])); }
                    }
                }
                if i > 1 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "le" | "la" | "les" | "l’") && t[i - 2].word && has_pos(&t[i - 2], Pos::PronSubj) {
                    let s = if t[i - 1].lower == "les" { "leur" } else { "lui" };
                    o.push(self.mk(&t[i - 1], "FR_PREP", "On téléphone « à » quelqu’un : pronom complément indirect.", vec![s.to_string()]));
                }
            }
            // aider aux familles -> les familles
            if lemma_is(&t[i], "aider") && has_fin(&t[i]) && !noun_use {
                if let Some(p) = nx(1) {
                    if let Some(s) = match p.lower.as_str() { "aux" => Some("les"), "au" => Some("le"), _ => None } {
                        o.push(self.mk(p, "FR_PREP", "« aider quelqu’un » : complément direct.", vec![s.to_string()]));
                    }
                }
            }
            // permettre les visiteurs de -> aux visiteurs
            if lemma_is(&t[i], "permettre") && (has_fin(&t[i]) || t[i].morph.iter().any(|m| m.moods & M_Y != 0)) {
                if let (Some(p), Some(n)) = (nx(1), nx(2)) {
                    let de_follows = (i + 3..(i + 7).min(t.len())).any(|k| t[k].word && matches!(t[k].lower.as_str(), "de" | "d’") && Self::word_at(t, k + 1) && t[k + 1].morph.iter().any(|m| m.moods & M_Y != 0));
                    if de_follows && has_pos(n, Pos::Noun) && n.morph.iter().any(|m| m.pos == Pos::Noun) {
                        if let Some(s) = match p.lower.as_str() { "les" => Some("aux"), "le" => Some("au"), "la" => Some("à la"), "l’" => Some("à l’"), _ => None } {
                            o.push(self.mk(p, "FR_PREP", "« permettre à quelqu’un de faire ».", vec![s.to_string()]));
                        }
                    }
                }
            }
            // jouer du tennis -> au tennis ; jouer au piano -> du piano
            if lemma_is(&t[i], "jouer") && t[i].morph.iter().any(|m| is_fin(m) || m.moods & (M_Q | M_Y) != 0) {
                if let (Some(p), Some(n)) = (nx(1), nx(2)) {
                    let (pl, nl) = (p.lower.as_str(), n.lower.as_str());
                    if matches!(pl, "du" | "de") && SPORTS.contains(&nl) { o.push(self.mk(p, "FR_PREP", "On joue « à » un sport ou un jeu.", vec![if pl == "du" { "au" } else { "à" }.to_string()])); }
                    if matches!(pl, "au" | "à") && INSTR.contains(&nl) { o.push(self.mk(p, "FR_PREP", "On joue « de » un instrument.", vec![if pl == "au" { "du" } else { "de" }.to_string()])); }
                    if pl == "à" && Self::word_at(t, i + 3) && t[i + 2].lower == "la" && INSTR.contains(&t[i + 3].lower.as_str()) { o.push(self.mk(p, "FR_PREP", "On joue « de » un instrument.", vec!["de".into()])); }
                }
            }
            // se souvenir à/au -> de/du (adverbs allowed between)
            if refl && lemma_is(&t[i], "souvenir") && has_fin(&t[i]) {
                let mut k = i + 1;
                while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) { k += 1; }
                if k < t.len() && !t[k].word && t[k].text == "," { if let Some(c) = (k + 1..(k + 15).min(t.len())).find(|&x| !t[x].word && t[x].text == ",") { k = c + 1; } }
                if Self::word_at(t, k) {
                    if let Some(s) = match t[k].lower.as_str() { "à" => Some("de"), "au" => Some("du"), "aux" => Some("des"), _ => None } {
                        o.push(self.mk(&t[k], "FR_PREP", "« se souvenir de quelque chose ».", vec![s.to_string()]));
                    }
                }
            }
            // se rappeler (adverbs) du -> le ; "dont je me rappelle" -> que
            if refl && lemma_is(&t[i], "rappeler") && has_fin(&t[i]) {
                let mut k = i + 1;
                while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) && k <= i + 3 { k += 1; }
                if k > i + 1 && Self::word_at(t, k) && t[k].lower == "du" { o.push(self.mk(&t[k], "FR_PREP", "« se rappeler quelque chose » se construit sans « de ».", vec!["le".into()])); }
                if let Some(d) = (i.saturating_sub(5)..i).find(|&x| t[x].word && t[x].lower == "dont") {
                    if !(d + 1..i).any(|x| t[x].word && has_fin(&t[x]) && t[x].morph.iter().all(|m| m.pos == Pos::Verb)) {
                        o.push(self.mk(&t[d], "FR_DONT", "« se rappeler quelque chose » : le relatif est « que ».", vec!["que".into()]));
                    }
                }
            }
            // aider: "leur aider" -> les ; "aide (souvent) au voisin" -> le
            if lemma_is(&t[i], "aider") && has_fin(&t[i]) && !noun_use {
                if i > 0 && t[i - 1].word && t[i - 1].lower == "leur" && !(i > 1 && det_info(&t[i - 2]).is_some()) {
                    o.push(self.mk(&t[i - 1], "FR_PREP", "« aider quelqu’un » : pronom complément direct.", vec!["les".into()]));
                }
                let mut k = i + 1;
                while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv)) && k <= i + 3 { k += 1; }
                if k > i + 1 && Self::word_at(t, k) {
                    if let Some(s) = match t[k].lower.as_str() { "aux" => Some("les"), "au" => Some("le"), _ => None } { o.push(self.mk(&t[k], "FR_PREP", "« aider quelqu’un » : complément direct.", vec![s.to_string()])); }
                }
            }
            // jouer le tennis -> au tennis ; "... et au piano" after jouer -> du
            if lemma_is(&t[i], "jouer") && t[i].morph.iter().any(|m| is_fin(m) || m.moods & (M_Q | M_Y) != 0) {
                let mut k = i + 1;
                if k < t.len() && !t[k].word && t[k].text == "," { if let Some(c) = (k + 1..(k + 20).min(t.len())).find(|&x| !t[x].word && t[x].text == ",") { k = c + 1; } }
                while Self::word_at(t, k) && (ADVERBS.contains(&t[k].lower.as_str()) || all_pos(&t[k], Pos::Adv) || matches!(t[k].lower.as_str(), "chaque" | "semaine" | "dimanche" | "soir" | "jour")) && k <= i + 6 { k += 1; }
                if Self::word_at(t, k + 1) {
                    let (pl, nl) = (t[k].lower.as_str(), t[k + 1].lower.as_str());
                    if matches!(pl, "le" | "la" | "les") && SPORTS.contains(&nl) { o.push(self.mk(&t[k], "FR_PREP", "On joue « à » un sport ou un jeu.", vec![match pl { "le" => "au", "la" => "à la", _ => "aux" }.to_string()])); }
                    if matches!(pl, "le" | "la" | "les") && INSTR.contains(&nl) { o.push(self.mk(&t[k], "FR_PREP", "On joue « de » un instrument.", vec![match pl { "le" => "du", "la" => "de la", _ => "des" }.to_string()])); }
                }
                for k in i + 2..(i + 14).min(t.len()) {
                    if !t[k].word || t[k].sid != t[i].sid || !Self::word_at(t, k + 1) || t[k - 1].lower != "et" { continue; }
                    let (pl, nl) = (t[k].lower.as_str(), t[k + 1].lower.as_str());
                    if pl == "au" && INSTR.contains(&nl) { o.push(self.mk(&t[k], "FR_PREP", "On joue « de » un instrument.", vec!["du".into()])); }
                    if pl == "du" && SPORTS.contains(&nl) { o.push(self.mk(&t[k], "FR_PREP", "On joue « à » un sport ou un jeu.", vec!["au".into()])); }
                }
            }
            // penser du bien / du mal à X -> de X ; "ce que je pense à" -> de
            if t[i].morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == "penser") {
                let mut k = i + 1;
                if Self::word_at(t, k + 1) && t[k].lower == "du" && matches!(t[k + 1].lower.as_str(), "bien" | "mal") { k += 2; }
                let ce_que = (i.saturating_sub(4)..i).any(|x| t[x].lower == "ce") && (i.saturating_sub(3)..i).any(|x| matches!(t[x].lower.as_str(), "que" | "qu’"));
                if Self::word_at(t, k) && matches!(t[k].lower.as_str(), "à" | "au" | "aux") && (k > i + 1 || ce_que) {
                    let s = match t[k].lower.as_str() { "au" => "du", "aux" => "des", _ => "de" };
                    o.push(self.mk(&t[k], "FR_PREP", "Pour une opinion : « penser (du bien / du mal) de ».", vec![s.to_string()]));
                }
            }
            // Que penses-tu à ce film ? -> de
            if t[i].morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == "penser") {
                let ss = (0..=i).rev().find(|&k| t[k].sent_start).unwrap_or(0);
                let q_open = t[ss].word && matches!(t[ss].lower.as_str(), "que" | "qu’") && i - ss <= 5;
                let k = if t[i].inv.is_some() { i + 1 } else { (i + 1..(i + 4).min(t.len())).find(|&k| t[k].word && matches!(t[k].lower.as_str(), "à" | "au" | "aux")).unwrap_or(i + 1) };
                if q_open && Self::word_at(t, k) && matches!(t[k].lower.as_str(), "à" | "au" | "aux") {
                    let s = match t[k].lower.as_str() { "au" => "du", "aux" => "des", _ => "de" };
                    o.push(self.mk(&t[k], "FR_PREP", "Pour demander une opinion : « penser de » (que penses-tu de …).", vec![s.to_string()]));
                }
            }
        }
    }

    /// Days, months, languages and nationality adjectives take a lowercase initial.
    fn capitals(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const DAYS: &[&str] = &["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche", "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
        const NAT: &[&str] = &["français", "française", "françaises", "anglais", "anglaise", "anglaises", "allemand", "allemande", "allemands", "allemandes",
            "italien", "italienne", "italiens", "italiennes", "espagnol", "espagnole", "espagnols", "espagnoles", "portugais", "portugaise", "portugaises",
            "chinois", "chinoise", "chinoises", "japonais", "japonaise", "japonaises", "russe", "russes", "arabe", "arabes", "américain", "américaine",
            "américains", "américaines", "canadien", "canadienne", "canadiens", "canadiennes", "belge", "belges", "suisse", "suisses", "grec", "grecque",
            "grecs", "grecques", "polonais", "polonaise", "polonaises", "néerlandais", "néerlandaise", "européen", "européenne", "européens", "européennes",
            "africain", "africaine", "africains", "africaines", "latin", "latine", "breton", "bretonne", "occitan", "basque", "corse"];
        let stems = ["brésilien", "irlandais", "mexicain", "argentin", "chilien", "péruvien", "marocain", "algérien", "tunisien", "sénégalais", "ivoirien",
            "camerounais", "congolais", "togolais", "béninois", "malien", "égyptien", "iranien", "indien", "pakistanais", "vietnamien", "coréen",
            "thaïlandais", "australien", "suédois", "norvégien", "danois", "finlandais", "hongrois", "roumain", "ukrainien", "autrichien", "écossais",
            "gallois", "québécois", "haïtien", "cubain", "colombien", "vénézuélien", "israélien", "libanais", "syrien", "irakien", "turc", "néo-zélandais", "sud-africain"];
        let extra: Vec<String> = stems.iter().flat_map(|s| {
            let f = if s.ends_with("ien") { format!("{s}ne") } else if *s == "turc" { "turque".to_string() } else { format!("{s}e") };
            let mp = if s.ends_with('s') { s.to_string() } else { format!("{s}s") };
            vec![s.to_string(), f.clone(), mp, format!("{f}s")]
        }).collect();
        let nat = |w: &str| NAT.contains(&w) || extra.iter().any(|e| e == w);
        const LANG_VERBS: &[&str] = &["parler", "apprendre", "étudier", "enseigner", "traduire", "comprendre", "lire", "écrire", "maîtriser", "pratiquer"];
        for i in 1..t.len() {
            let x = &t[i];
            if !x.word || x.sent_start || !x.text.chars().next().map_or(false, |c| c.is_uppercase()) || x.text.chars().skip(1).any(|c| c.is_uppercase()) { continue; }
            if !t[i - 1].word && matches!(t[i - 1].text.as_str(), "." | "!" | "?" | "«" | "“" | "\"" | ":" | "—") { continue; }
            let l = x.lower.as_str();
            let lower_sug = vec![x.lower.clone()];
            if matches!(l, "mars" | "mai") { continue; } // the god / planet, or a name
            let in_names = (i >= 2 && t[i - 1].text == "," && t[i - 2].text.chars().next().map_or(false, |c| c.is_uppercase()))
                || (i >= 2 && t[i - 1].lower == "de" && t[i - 2].text.chars().next().map_or(false, |c| c.is_uppercase()));
            if in_names { continue; }
            let feast = Self::word_at(t, i + 1) && (matches!(t[i + 1].lower.as_str(), "saint" | "sainte" | "gras")
                || (matches!(t[i + 1].lower.as_str(), "des" | "de") && Self::word_at(t, i + 2) && t[i + 2].text.chars().next().map_or(false, |c| c.is_uppercase())));
            if DAYS.contains(&l) && !feast { o.push(Lint { start: x.start, end: x.end, rule: "FR_CAPITAL", message: "Les jours et les mois s’écrivent en minuscules.".into(), suggestions: lower_sug }); continue; }
            if !nat(l) { continue; }
            let mut pi = i - 1;
            while pi > 0 && t[pi].word && (all_pos(&t[pi], Pos::Adv) || ADVERBS.contains(&t[pi].lower.as_str())) && t[pi - 1].word { pi -= 1; }
            let p = &t[pi];
            let lang_ctx = p.word && (p.lower == "en" || ((p.lower == "le" || p.lower == "l’") && i >= 2 && t[i - 2].word && t[i - 2].morph.iter().any(|m| LANG_VERBS.contains(&self.lemma_of(m))))
                || p.morph.iter().any(|m| is_fin(m) && LANG_VERBS.contains(&self.lemma_of(m))));
            if matches!(l, "suisse" | "suisses") { continue; } // also the country
            const PERSON_ADJ: &[&str] = &["jeune", "jeunes", "vieux", "vieil", "vieille", "vieilles", "bon", "bons", "bonne", "bonnes", "grand", "grands", "petit", "petits", "brave", "braves", "pauvre", "pauvres", "riche", "riches", "célèbre", "illustre", "simple", "vrai", "vrais"];
            let head_noun = p.word && has_pos(p, Pos::Noun) && !PERSON_ADJ.contains(&p.lower.as_str()) && pi >= 1 && t[pi - 1].word && det_info(&t[pi - 1]).is_some();
            let adj_after_noun = p.word && has_pos(p, Pos::Noun) && !p.text.chars().next().map_or(false, |c| c.is_uppercase()) && det_info(p).is_none()
                && (!p.morph.iter().any(adjish) || head_noun);
            if lang_ctx || adj_after_noun {
                o.push(Lint { start: x.start, end: x.end, rule: "FR_CAPITAL", message: "Langue ou adjectif de nationalité : minuscule (majuscule seulement pour les habitants).".into(), suggestions: lower_sug });
            }
        }
    }

    /// "La chanson que je me souviens" -> dont
    fn dont_relative(&self, t: &[Tok], o: &mut Vec<Lint>) {
        const DE_VERBS: &[&str] = &["souvenir", "servir", "occuper", "moquer", "méfier", "plaindre", "passer", "souvenir", "rappeler"];
        for q in 1..t.len() {
            if !t[q].word || !matches!(t[q].lower.as_str(), "que" | "qu’") || !t[q - 1].word || !has_pos(&t[q - 1], Pos::Noun) { continue; }
            if t[q - 1].morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::ConjS | Pos::Adv) || adjish(m) || is_fin(m) || m.moods & M_Y != 0) { continue; }
            let end = (q + 1..(q + 9).min(t.len())).take_while(|&k| t[k].word && t[k].sid == t[q].sid).last().unwrap_or(q);
            let mut hit = false;
            for k in q + 1..=end {
                let x = &t[k];
                if matches!(x.lower.as_str(), "dont" | "que" | "qu’" | "qui") { break; }
                let refl = k > q + 1 && matches!(t[k - 1].lower.as_str(), "se" | "s’" | "me" | "m’" | "te" | "t’" | "nous" | "vous");
                if refl && x.morph.iter().any(|m| (is_fin(m) || m.moods & M_Q != 0) && matches!(self.lemma_of(m), "souvenir" | "servir" | "occuper" | "moquer" | "méfier" | "plaindre")) { hit = true; break; }
                if x.morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == "avoir") && Self::word_at(t, k + 1) && matches!(t[k + 1].lower.as_str(), "besoin" | "envie" | "peur") && !(Self::word_at(t, k + 2) && matches!(t[k + 2].lower.as_str(), "de" | "d’")) { hit = true; break; }
                let ind = k > q + 1 && matches!(t[k - 1].lower.as_str(), "t’" | "te" | "lui" | "leur" | "vous" | "nous" | "m’" | "me");
                let ind_aux = k > q + 2 && self.fin_lemma(&t[k - 1], "avoir") && matches!(t[k - 2].lower.as_str(), "t’" | "te" | "lui" | "leur" | "vous" | "nous" | "m’" | "me");
                if (ind || ind_aux) && x.morph.iter().any(|m| (is_fin(m) || m.moods & M_Q != 0) && self.lemma_of(m) == "parler") { hit = true; break; }
                let _ = DE_VERBS;
                if has_fin(x) && !refl && !self.fin_lemma(x, "avoir") && k > q + 2 { break; }
            }
            if hit { o.push(self.mk(&t[q], "FR_DONT", "Ce verbe se construit avec « de » : le relatif est « dont ».", vec!["dont".into()])); }
        }
    }

    /// "avec quelle je collabore" -> laquelle
    fn lequel(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "quel" | "quelle" | "quels" | "quelles") { continue; }
            let p = t[i - 1].lower.as_str();
            if !t[i - 1].word || !(PREPS.contains(&p) || matches!(p, "parmi" | "grâce")) || !Self::word_at(t, i + 1) { continue; }
            let nx = &t[i + 1];
            if has_pos(nx, Pos::Noun) && !has_pos(nx, Pos::PronSubj) || nx.morph.iter().any(adjish) && !has_pos(nx, Pos::PronSubj) { continue; }
            if !(has_pos(nx, Pos::PronSubj) || det_info(nx).is_some() || has_fin(nx)) { continue; }
            let base = match t[i].lower.as_str() { "quel" => "lequel", "quelle" => "laquelle", "quels" => "lesquels", _ => "lesquelles" };
            let sug = match (p, base) { ("à", "lequel") => "auquel".to_string(), ("à", "lesquels") => "auxquels".to_string(), ("à", "lesquelles") => "auxquelles".to_string(), (_, b) => b.to_string() };
            o.push(self.mk(&t[i], "FR_LEQUEL", "Après une préposition, le pronom relatif est « lequel » (laquelle, lesquels, lesquelles).", vec![sug]));
        }
    }

    /// "Nous avons que quinze minutes" -> n'avons ... que
    fn ne_que(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for v in 1..t.len().saturating_sub(2) {
            if !t[v].word || !matches!(t[v + 1].lower.as_str(), "que" | "qu’") || !Self::word_at(t, v + 2) { continue; }
            let restr_verb = t[v].morph.iter().any(|m| is_fin(m) && matches!(self.lemma_of(m), "avoir" | "rester" | "coûter" | "durer" | "mesurer" | "peser" | "valoir" | "compter"));
            let nx = &t[v + 2];
            let quantity = nx.lower.chars().all(|c| c.is_ascii_digit()) || matches!(nx.lower.as_str(), "deux" | "trois" | "quatre" | "cinq" | "six" | "sept" | "huit" | "neuf" | "dix" | "quinze" | "vingt" | "trente" | "cent" | "mille" | "quelques" | "un" | "une");
            if !restr_verb || !quantity { continue; }
            let ss = (0..=v).rev().find(|&k| t[k].sent_start).unwrap_or(0);
            if (ss..v).any(|k| t[k].word && matches!(t[k].lower.as_str(), "ne" | "n’")) { continue; }
            let sug = if vowel(&t[v].lower) { format!("n’{}", t[v].text) } else { format!("ne {}", t[v].text) };
            o.push(self.mk(&t[v], "FR_NE_QUE", "Restriction « ne … que » : le « ne » est obligatoire à l’écrit.", vec![sug]));
        }
    }

    /// "pallier au manque" -> pallier le manque
    fn pallier(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !t[i].morph.iter().any(|m| self.lemma_of(m) == "pallier") || !Self::word_at(t, i + 1) { continue; }
            let p = &t[i + 1];
            let sug = match p.lower.as_str() { "au" => "le", "aux" => "les", "à" if Self::word_at(t, i + 2) && matches!(t[i + 2].lower.as_str(), "la" | "l’") => "", _ => continue };
            if sug.is_empty() { o.push(Lint { start: p.start, end: t[i + 2].end, rule: "FR_PREP", message: "« pallier » est transitif direct (pallier quelque chose).".into(), suggestions: vec![t[i + 2].text.clone()] }); }
            else { o.push(self.mk(p, "FR_PREP", "« pallier » est transitif direct (pallier quelque chose).", vec![sug.to_string()])); }
        }
    }

    /// "au cas où il pleuvra" -> pleuvrait
    fn au_cas_ou(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(3) {
            if !(t[i].word && t[i].lower == "au" && t[i + 1].lower == "cas" && t[i + 2].lower == "où") { continue; }
            let s = i + 3;
            let (p, n, v) = if let Some((p, n)) = pron_pn(&t[s].lower) { (p, n, self.skip_clitics(t, s + 1, i)) }
                else if let Some(np) = self.np_at(t, s) { (4u8, np.number, self.skip_clitics(t, np.end + 1, i)) } else { continue };
            if !Self::word_at(t, v) || !single(n) { continue; }
            let x = &t[v];
            let fut = x.morph.iter().any(|m| m.moods & M_IF != 0 && m.person & p != 0 && m.number & n != 0);
            let cond = x.morph.iter().any(|m| m.moods & M_K != 0 && m.person & p != 0 && m.number & n != 0);
            if !fut || cond { continue; }
            let Some(r) = x.morph.iter().find(|m| is_fin(m)) else { continue };
            let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_K != 0 && f.person & p != 0 && f.number & n != 0);
            o.push(self.mk(x, "FR_SI_COND", "Après « au cas où », le verbe est au conditionnel.", sug));
        }
    }

    /// "J'en vais aussi" -> J'y vais (but "je m'en vais" is correct)
    fn en_y(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || t[i].lower != "en" || !Self::word_at(t, i + 1) || !self.fin_lemma(&t[i + 1], "aller") { continue; }
            if i > 0 && t[i - 1].word && matches!(t[i - 1].lower.as_str(), "m’" | "t’" | "s’" | "nous" | "vous" | "me" | "te" | "se") { continue; }
            o.push(self.mk(&t[i], "FR_PRONOM", "Avec « aller », le lieu se reprend par « y » (« j’y vais »).", vec!["y".into()]));
        }
    }

    /// Hyphenated words not in the lexicon whose known variant fixes a classic error.
    fn compound_repair(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for tk in t {
            if !tk.word || !tk.text.contains('-') && !tk.text.contains('’') && !tk.text.contains('\'') { continue; }
            let low = tk.lower.clone();
            if (self.lex.known(&low) && !low.starts_with("vas-")) || low == "va-t’en" { continue; }
            let parts: Vec<&str> = low.split('-').collect();
            let mut cands: Vec<String> = Vec::new();
            if parts.len() >= 2 {
                let last = parts[parts.len() - 1];
                let prep_compound = parts.len() >= 3 && matches!(parts[1], "au" | "aux" | "à" | "de" | "d’" | "en" | "la");
                if let Some(f) = parts[0].strip_suffix('s') {
                    let verb_like = self.lex.analyze(f).1.iter().any(|m| m.moods & M_IP != 0 && m.person & 4 != 0);
                    if verb_like && !prep_compound { cands.push(std::iter::once(f).chain(parts[1..].iter().copied()).collect::<Vec<_>>().join("-")); }
                }
                if let Some(l) = last.strip_suffix('s') { cands.push(parts[..parts.len() - 1].iter().copied().chain(std::iter::once(l)).collect::<Vec<_>>().join("-")); }
                if parts[0] == "presque" { cands.push(format!("presqu’{}", parts[1..].join("-"))); }
                if parts[0].ends_with('s') && parts.len() >= 2 {
                    if let Some(l) = parts[parts.len() - 1].strip_suffix('s') { let f = parts[0].trim_end_matches('s'); cands.push(std::iter::once(f).chain(parts[1..parts.len() - 1].iter().copied()).chain(std::iter::once(l)).collect::<Vec<_>>().join("-")); }
                }
            }
            let mut sug: Option<String> = cands.into_iter().find(|c| self.lex.known(c));
            // grammar-level compounds
            if sug.is_none() {
                if low == "vas-t’en" || low == "vas-t-en" || low == "va-t-en" { sug = Some("va-t’en".into()); }
                else if parts.len() >= 2 && matches!(parts[parts.len() - 1], "me" | "te") { let last = if parts[parts.len() - 1] == "me" { "moi" } else { "toi" }; sug = Some(format!("{}-{}", parts[..parts.len() - 1].join("-"), last)); }
                else if parts.len() == 2 && matches!(parts[1], "en" | "y") && (parts[0] == "va" || self.lex.analyze(parts[0]).1.iter().any(|m| m.moods & M_E != 0 && m.person & 2 != 0 && m.number & N_S != 0 && self.lemma_of(m).ends_with("er"))) && !parts[0].ends_with('s') {
                    sug = Some(format!("{}s-{}", parts[0], parts[1]));
                }
                else if ["-t’il", "-t’elle", "-t’on", "-t’ils", "-t’elles"].iter().any(|e| low.ends_with(e)) { sug = Some(low.replace("-t’", "-t-")); }
                else if parts.len() >= 2 && parts[0].ends_with("es") && matches!(parts[1], "moi" | "toi" | "le" | "la" | "les" | "lui" | "leur") {
                    let (_, m) = self.lex.analyze(parts[0]);
                    if m.iter().any(|x| x.moods & M_IP != 0 && x.person & 2 != 0 && self.lemma_of(x).ends_with("er")) && (tk.sent_start || true) {
                        sug = Some(format!("{}-{}", &parts[0][..parts[0].len() - 1], parts[1..].join("-")));
                    }
                }
            }
            if let Some(s) = sug {
                if s != low { o.push(self.mk(tk, "FR_COMPOUND", "Forme correcte de ce mot composé.", vec![s])); }
            }
        }
    }

    /// "L'un et l'autre a présenté" -> ont
    fn lun_lautre(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(4) {
            if !(t[i].word && t[i].lower == "l’" && matches!(t[i + 1].lower.as_str(), "un" | "une") && t[i + 2].lower == "et" && t[i + 3].lower == "l’" && t[i + 4].lower == "autre") { continue; }
            if !self.subj_ctx(t, i) || (i > 0 && t[i - 1].word && (PREPS.contains(&t[i - 1].lower.as_str()) || has_pos(&t[i - 1], Pos::Prep))) { continue; }
            let mut k = i + 5;
            if Self::word_at(t, k) && has_pos(&t[k], Pos::Noun) && !has_fin(&t[k]) { k += 1; }
            let Some(v) = self.main_verb_after(t, i, k) else { continue };
            if !Self::word_at(t, v) || !has_fin(&t[v]) || t[v].morph.iter().any(|m| is_fin(m) && m.number & N_P != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], 4, N_P);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "« L’un et l’autre » sujet : verbe au pluriel.", sug));
        }
    }

    /// "des difficultés tel que ..." -> telles
    fn tel_que(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "tel" | "telle" | "tels" | "telles") || !matches!(t[i + 1].lower.as_str(), "que" | "qu’") { continue; }
            let n = &t[i - 1];
            if !n.word || !has_pos(n, Pos::Noun) || has_fin(n) { continue; }
            if (i.saturating_sub(4)..i).any(|k| t[k].word && matches!(t[k].lower.as_str(), "et" | "ou")) { continue; }
            let g = n.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender);
            let nn = n.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number);
            if !single(g) || !single(nn) { continue; }
            let want = match (g == G_F, nn == N_P) { (false, false) => "tel", (true, false) => "telle", (false, true) => "tels", (true, true) => "telles" };
            if t[i].lower != want { o.push(self.mk(&t[i], "FR_ATTR_AGREE", "« tel » s’accorde avec le nom auquel il se rapporte.", vec![want.to_string()])); }
        }
    }

    /// "Ayants terminé", "chantants une comptine" -> present participle is invariable
    fn present_participle(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            let x = &t[i];
            if !x.word || !(x.lower.ends_with("ants") || x.lower.ends_with("ante") || x.lower.ends_with("antes")) { continue; }
            let base = format!("{}ant", &x.lower[..x.lower.rfind("ant").unwrap()]);
            let (k, m) = self.lex.analyze(&base);
            if !k || !m.iter().any(|r| r.moods & M_P != 0) || has_pos(x, Pos::Noun) { continue; }
            let nx = &t[i + 1];
            let adj = has_pos(x, Pos::Adj);
            let verbal = nx.word && (det_info(nx).is_some() || nx.morph.iter().any(|r| r.moods & M_Q != 0 && r.pos == Pos::Verb) || matches!(nx.lower.as_str(), "que" | "qu’") || CLITICS.contains(&nx.lower.as_str())
                || (!adj && matches!(nx.lower.as_str(), "à" | "de" | "d’" | "sur" | "dans" | "avec")));
            let start = x.sent_start || (i > 0 && !t[i - 1].word && t[i - 1].text == ",");
            if verbal && start { o.push(self.mk(x, "FR_PPRESENT", "Le participe présent est invariable.", vec![base])); }
        }
    }

    /// "Quelques méritants que soient ...", "quelques deux cents amateurs" -> quelque (adverb)
    fn quelque_adverb(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || t[i].lower != "quelques" { continue; }
            let nx = &t[i + 1];
            let concessive = nx.word && nx.morph.iter().any(adjish) && !has_pos(nx, Pos::Noun) && matches!(t[i + 2].lower.as_str(), "que" | "qu’");
            let approx = nx.word && matches!(nx.lower.as_str(), "deux" | "trois" | "quatre" | "cinq" | "six" | "dix" | "vingt" | "trente" | "cinquante" | "cent" | "mille");
            if concessive || approx { o.push(self.mk(&t[i], "FR_QUEL_QUE", "« quelque » adverbe (devant un adjectif + que, ou un nombre) est invariable.", vec!["quelque".into()])); }
        }
    }

    /// "Elle est toute étonnée" -> tout (before a vowel or mute h, the adverb is invariable)
    fn toute_before_vowel(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(1) {
            if !t[i].word || t[i].lower != "toute" { continue; }
            let a = &t[i + 1];
            if !a.word || !vowel(&a.lower) || a.lower.starts_with('h') || !a.morph.iter().any(adjish) || has_pos(a, Pos::Noun) { continue; }
            if !(t[i - 1].word && (self.fin_lemma(&t[i - 1], "être") || ["sembler", "paraître", "rester", "devenir"].iter().any(|l| self.fin_lemma(&t[i - 1], l)))) { continue; }
            o.push(self.mk(&t[i], "FR_TOUT", "« tout » adverbe reste invariable devant un adjectif féminin commençant par une voyelle.", vec!["tout".into()]));
        }
    }

    /// "bleu clair", "vert foncé", noun-colours ("marron"): invariable, skipped by the noun/adjective rule
    pub(super) fn colour_compound(&self, t: &[Tok], a: usize) -> bool {
        let l = t[a].lower.as_str();
        let base_colour = |w: &str| COLOUR_ADJ.iter().any(|c| w.starts_with(c) || (*c == "blanc" && w.starts_with("blanch")));
        COLOUR_NOUNS.contains(&l) || COLOUR_NOUNS.contains(&l.trim_end_matches('s'))
            || (COLOUR_QUAL.contains(&l) && a > 0 && base_colour(&t[a - 1].lower))
            || (base_colour(l) && Self::word_at(t, a + 1) && (COLOUR_QUAL.contains(&t[a + 1].lower.as_str()) || matches!(t[a + 1].lower.as_str(), "cassé" | "cassés" | "cassée" | "cassées" | "bouteille" | "pomme" | "amande" | "sapin" | "olive" | "d’" | "de" | "émeraude" | "anglais" | "tendre" | "pétrole")))
            || (matches!(l, "cassé" | "cassée" | "cassés" | "cassées") && a > 0 && base_colour(&t[a - 1].lower))
    }

    /// "ils sont tout partis" -> tous; "je les ai tout lus" -> tous
    fn floating_tous(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len().saturating_sub(1) {
            if !t[i].word || t[i].lower != "tout" { continue; }
            let aux = t[i - 1].word && (self.fin_lemma(&t[i - 1], "être") || self.fin_lemma(&t[i - 1], "avoir"));
            let p = &t[i + 1];
            if !aux || !p.word || has_pos(p, Pos::Adj) { continue; }
            let q: Vec<&Morph> = p.morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if q.is_empty() || !q.iter().all(|m| m.number == N_P) { continue; }
            // "ils sont tout étonnés" is correct (adverb): only non-gradable motion participles force "tous"
            const MOTION: &[&str] = &["partir", "venir", "arriver", "sortir", "entrer", "monter", "descendre", "rentrer", "revenir", "tomber", "aller", "retourner", "mourir", "naître", "parvenir", "repartir", "ressortir"];
            if !q.iter().any(|m| MOTION.contains(&self.lemma_of(m))) { continue; }
            let fem = q.iter().all(|m| m.gender == G_F);
            if fem && vowel(&p.lower) { continue; } // "elles sont tout émues": adverb, invariable
            o.push(self.mk(&t[i], "FR_TOUT", "« tous / toutes » (pronom) devant ce participe pluriel.", vec![if fem { "toutes" } else { "tous" }.to_string()]));
        }
    }

    /// Small fixed forms: "qu'il y a eus" -> eu, "a pus" -> pu, "demie-heure" -> demi-heure, "nus-pieds" -> nu-pieds
    fn small_fixed(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word { continue; }
            let l = t[i].lower.as_str();
            if matches!(l, "eus" | "eue" | "eues") && i >= 2 && t[i - 2].lower == "y" && self.fin_lemma(&t[i - 1], "avoir") {
                o.push(self.mk(&t[i], "FR_PP_INVARIABLE", "« il y a eu » : participe invariable (impersonnel).", vec!["eu".into()]));
            }
            if matches!(l, "pus" | "pue" | "pues") && i >= 1 && self.fin_lemma(&t[i - 1], "avoir") {
                o.push(self.mk(&t[i], "FR_PP_INVARIABLE", "Le participe passé de « pouvoir » est invariable.", vec!["pu".into()]));
            }
            for (bad, good) in [("demie-", "demi-"), ("demies-", "demi-"), ("demis-", "demi-"), ("nus-", "nu-"), ("nue-", "nu-"), ("nues-", "nu-"), ("semis-", "semi-"), ("mie-", "mi-")] {
                if let Some(rest) = l.strip_prefix(bad) {
                    if !rest.is_empty() { o.push(self.mk(&t[i], "FR_ATTR_AGREE", "Devant le nom, « demi », « nu », « semi » sont invariables.", vec![format!("{good}{rest}")])); }
                }
            }
        }
    }

    /// "auraient-il" -> "aurait-il": a verb agrees with its attached inverted pronoun.
    fn inv_compound(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for tk in t {
            let Some((p, n)) = tk.inv else { continue };
            if tk.lower.ends_with("-ce") { continue; } // "sont-ce", "furent-ce": ce + être agrees with the attribute
            if tk.morph.iter().any(|m| is_fin(m) && m.person & p != 0 && m.number & n != 0) { continue; }
            let cut = tk.text.find('-').unwrap_or(tk.text.len());
            let rest = &tk.text[cut..];
            let sug = self.verb_forms_for(tk, p, n).into_iter().map(|f| format!("{f}{rest}")).collect();
            o.push(self.mk(tk, "FR_SUBJ_VERB", "Le verbe s’accorde avec le pronom inversé.", sug));
        }
    }

    /// Verb at `v` followed (after adverbs/participles/bare nouns) by a noun phrase that closes the clause:
    /// returns that NP's number when the verb disagrees with it.
    pub(super) fn inverted_np(&self, t: &[Tok], v: usize, allow_imperative: bool) -> Option<u8> {
        if !Self::word_at(t, v) || !has_fin(&t[v]) || has_pos(&t[v], Pos::PronSubj) || has_pos(&t[v], Pos::Prep) || t[v].inv.is_some() { return None; }
        if v > 0 && t[v - 1].word && det_info(&t[v - 1]).is_some() && !CLITICS.contains(&t[v - 1].lower.as_str()) { return None; }
        if !t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0) { return None; }
        if !allow_imperative && t[v].morph.iter().any(|m| m.moods & M_E != 0) { return None; }
        let mut k = v + 1;
        while Self::word_at(t, k) && Self::same(t, v, k) && k <= v + 4 {
            let x = &t[k];
            if det_info(x).is_some() { break; }
            if (PREPS.contains(&x.lower.as_str()) || all_pos(x, Pos::Prep)) && !matches!(x.lower.as_str(), "des" | "du" | "aux" | "au") {
                // "que prend depuis plusieurs semaines les responsables": skip a prepositional phrase
                if let Some(pp) = self.np_at(t, k + 1) { k = pp.end + 1; continue; }
                if Self::word_at(t, k + 1) && has_pos(&t[k + 1], Pos::Noun) { k += 2; continue; }
                return None;
            }
            let filler = x.morph.iter().any(|m| m.moods & (M_Q | M_Y) != 0) || ADVERBS.contains(&x.lower.as_str()) || all_pos(x, Pos::Adv)
                || (all_pos(x, Pos::Noun) && !x.text.chars().next().map_or(false, |c| c.is_uppercase()));
            if !filler { return None; }
            k += 1;
        }
        let np = self.np_at(t, k)?;
        let e = np.end + 1;
        let closes = e >= t.len() || !t[e].word || t[e].sid != t[v].sid || matches!(t[e].lower.as_str(), "et" | "ou" | "mais" | "qui" | "que" | "qu’" | "dont" | "où")
            || t[e].morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::ConjS | Pos::Adv)) || self.verbish(&t[e]) || CLITICS.contains(&t[e].lower.as_str());
        if !closes || !single(np.number) || np.alt != 0 { return None; }
        if t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & np.number != 0) { return None; }
        Some(np.number)
    }

    /// "Dans la vallée vivait des loups", "Ici commencent ...", "Où va ces trains ?"
    fn inv_opener_question(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !t[s].sent_start { continue; }
            let end = (s..t.len()).take_while(|&k| t[k].sid == t[s].sid).last().unwrap_or(s);
            let question = !t[end].word && t[end].text == "?";
            let l = t[s].lower.as_str();
            let mut comma = false;
            let v = if question && matches!(l, "où" | "que" | "qu’" | "comment" | "quand" | "combien" | "pourquoi") {
                Some(s + 1)
            } else if !matches!(l, "des" | "du" | "de" | "d’" | "vu" | "excepté" | "sauf" | "selon" | "malgré") && (OPENER_ADV.contains(&l) || t[s].morph.iter().any(|m| m.pos == Pos::Prep) || PREPS.contains(&l)) {
                // genuine opener: place/time adverb, or preposition + noun phrase (or + adverb: "Au loin"),
                // directly followed (optionally after a comma) by the verb
                let opener_end = if OPENER_ADV.contains(&l) { Some(s) }
                    else {
                        let start = if matches!(l, "au" | "aux") { s } else { s + 1 };
                        if let Some(np) = self.np_at(t, start) { Some(np.end) }
                        else if Self::word_at(t, s + 1) && (has_pos(&t[s + 1], Pos::Adv) || OPENER_ADV.contains(&t[s + 1].lower.as_str())) { Some(s + 1) }
                        else { None }
                    };
                opener_end.and_then(|e| {
                    let mut k = e + 1;
                    if Self::word_at(t, k) && matches!(t[k].lower.as_str(), "que" | "qu’" | "où" | "dont" | "qui") {
                        return self.main_verb_after(t, s, k);
                    }
                    if k < t.len() && !t[k].word && t[k].text == "," { comma = true; k += 1; }
                    Some(k)
                })
            } else { None };
            let Some(mut v) = v else { continue };
            while Self::word_at(t, v) && matches!(t[v].lower.as_str(), "se" | "s’" | "y" | "en") { v += 1; }
            if let Some(n) = self.inverted_np(t, v, question || !comma) {
                let sug = self.verb_forms_for(&t[v], 4, n);
                o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Le verbe s’accorde avec son sujet placé après lui.", sug));
            }
        }
    }

    /// "« Nous partons », répondit les élèves." -> répondirent
    fn incise(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for v in 1..t.len() {
            let pre = if v >= 2 && t[v - 1].word && matches!(t[v - 1].lower.as_str(), "s’" | "se") { v - 2 } else { v - 1 };
            if t[pre].word || !matches!(t[pre].text.as_str(), "," | "»" | "—" | "”") { continue; }
            let q = |k: usize| !t[k].word && matches!(t[k].text.as_str(), "»" | "”" | "\"");
            let after_quote = (t[pre].text == "," && pre >= 1 && q(pre - 1)) || (q(pre) && pre >= 1 && !t[pre - 1].word && matches!(t[pre - 1].text.as_str(), "!" | "?" | "…" | ","));
            if !t[v].word || !after_quote || !has_fin(&t[v]) || t[v].text.chars().next().map_or(false, |c| c.is_uppercase()) { continue; }
            if let Some(n) = self.inverted_np(t, v, true) {
                let sug = self.verb_forms_for(&t[v], 4, n);
                o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Dans l’incise, le verbe s’accorde avec le sujet inversé.", sug));
            }
        }
    }

    /// "C'est moi qui a raison" -> ai
    fn qui_pronoun(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for q in 1..t.len() {
            if !t[q].word || t[q].lower != "qui" || !t[q - 1].word { continue; }
            let Some((p, n)) = stressed(&t[q - 1].lower) else { continue };
            let ok_ctx = q < 2 || !t[q - 2].word || t[q - 2].sent_start || matches!(t[q - 2].lower.as_str(), "et" | "ou" | "mais") || self.fin_lemma(&t[q - 2], "être");
            if !ok_ctx || (q >= 2 && t[q - 2].lower.ends_with('-')) { continue; }
            let v = self.skip_clitics(t, q + 1, q);
            if !Self::word_at(t, v) || !self.verbish(&t[v]) { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & p != 0 && m.number & n != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], p, n);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Après « moi/toi/nous/vous qui », le verbe prend la personne du pronom.", sug));
        }
    }

    /// "Plus d'un élève ont échoué" -> a; "Beaucoup de gens pense" -> pensent; "La plupart sont ..."
    fn quantity_subject(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !self.subj_ctx(t, i) { continue; }
            let l = t[i].lower.as_str();
            let (number, noun_at) = if l == "plus" && Self::word_at(t, i + 2) && t[i + 1].lower == "d’" && matches!(t[i + 2].lower.as_str(), "un" | "une") {
                (N_S, i + 3)
            } else if l == "moins" && Self::word_at(t, i + 2) && t[i + 1].lower == "de" && t[i + 2].lower == "deux" {
                (N_P, i + 3)
            } else if matches!(l, "beaucoup" | "peu" | "trop" | "assez" | "tant" | "combien" | "énormément" | "davantage") && Self::word_at(t, i + 1) && matches!(t[i + 1].lower.as_str(), "de" | "d’") {
                let j = i + 2;
                if let Some(np) = self.np_at(t, j) { if single(np.number) { (np.number, j) } else { continue } }
                else {
                if !Self::word_at(t, j) || !has_pos(&t[j], Pos::Noun) { continue; }
                let nn = t[j].morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number);
                if !single(nn) { continue; }
                (nn, j)
                }
            } else if l == "la" && Self::word_at(t, i + 1) && t[i + 1].lower == "plupart" {
                let sing = Self::word_at(t, i + 2) && (t[i + 2].lower == "du" || (t[i + 2].lower == "de" && Self::word_at(t, i + 3) && matches!(t[i + 3].lower.as_str(), "la" | "l’")));
                (if sing { N_S } else { N_P }, i + 2)
            } else { continue };
            // extent of the quantified noun phrase: noun + adjectives + prepositional chain
            let mut end = noun_at.min(t.len().saturating_sub(1));
            if noun_at < t.len() && t[noun_at].word && det_info(&t[noun_at]).is_some() && !matches!(t[noun_at].lower.as_str(), "des" | "de" | "d’") {
                if let Some(np) = self.np_at(t, noun_at) { end = np.end; }
            } else if noun_at < t.len() && t[noun_at].word && has_pos(&t[noun_at], Pos::Noun) {
                end = noun_at;
                while Self::word_at(t, end + 1) && t[end + 1].morph.iter().any(adjish) && !has_fin(&t[end + 1]) { end += 1; }
            } else if noun_at < t.len() && t[noun_at].word && matches!(t[noun_at].lower.as_str(), "des" | "de" | "d’") {
                if let Some(np) = self.np_at(t, if t[noun_at].lower == "des" { noun_at } else { noun_at + 1 }) { end = np.end; } else { end = noun_at - 1; }
            } else { end = noun_at - 1; }
            let Some(v) = self.main_verb_after(t, i, end + 1) else { continue };
            if !Self::word_at(t, v) || !has_fin(&t[v]) || t[v].inv.is_some() || !t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0) { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & number != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], 4, number);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Accord du verbe avec un sujet de quantité.", sug));
        }
    }

    /// "Toi et moi partiront" -> partirons; "Mon frère et moi est" -> sommes
    fn coord_persons(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !self.subj_ctx(t, i) { continue; }
            // left conjunct: stressed pronoun or short NP; then "et"; right conjunct: stressed pronoun or NP
            let conj = |k: usize| -> Option<(u8, usize)> {
                if !Self::word_at(t, k) { return None; }
                if let Some((p, _)) = stressed(&t[k].lower) { return Some((p, k)); }
                if let Some(np) = self.np_core(t, k) { return Some((4, np.end)); }
                if has_pos(&t[k], Pos::Proper) { return Some((4, k)); }
                None
            };
            let Some((p1, e1)) = conj(i) else { continue };
            if !Self::word_at(t, e1 + 1) || !matches!(t[e1 + 1].lower.as_str(), "et" | "ni") { continue; }
            let Some((p2, last)) = conj(e1 + 2) else { continue };
            let persons = p1 | p2;
            if persons & 3 == 0 { continue; }
            let p = if persons & 1 != 0 { 1 } else { 2 };
            let mut v = last + 1;
            if Self::word_at(t, v) && matches!(t[v].lower.as_str(), "nous" | "vous") { v += 1; }
            if v < t.len() && !t[v].word && t[v].text == "," {
                if Self::word_at(t, v + 1) && matches!(t[v + 1].lower.as_str(), "nous" | "vous") && Self::word_at(t, v + 2) && has_fin(&t[v + 2]) { v += 2; }
                else if let Some(m) = self.main_verb_after(t, i, v) { v = m; } else { continue; }
            }
            let v = self.skip_clitics(t, v, i);
            // "Mon frère et toi êtes arrivé" -> arrivés
            if Self::word_at(t, v + 1) && self.fin_lemma(&t[v], "être") && t[v + 1].morph.iter().any(adjish) && !t[v + 1].morph.iter().any(|m| adjish(m) && m.number & N_P != 0) && !has_pos(&t[v + 1], Pos::Noun) {
                if let Some(m0) = t[v + 1].morph.iter().find(|m| adjish(m)) {
                    let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, G_M, N_P);
                    o.push(self.mk(&t[v + 1], "FR_ATTR_AGREE", "Sujets coordonnés : participe ou attribut au pluriel.", sug));
                }
            }
            if !Self::word_at(t, v) || !self.verbish(&t[v]) || t[v].inv.is_some() { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & p != 0 && m.number & N_P != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], p, N_P);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Sujets coordonnés : la 1re personne l’emporte sur la 2e, la 2e sur la 3e.", sug));
        }
    }

    /// "..., tout le monde étaient là" -> était; "Chacun ont" -> a; "Personne ne savent" -> sait
    fn summary_subject(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word { continue; }
            let l = t[i].lower.as_str();
            let after = if l == "tout" && Self::word_at(t, i + 2) && t[i + 1].lower == "le" && t[i + 2].lower == "monde" { i + 3 }
                else if matches!(l, "chacun" | "chacune" | "quelqu’un") { i + 1 }
                else if l == "tout" && i > 0 && !t[i - 1].word && t[i - 1].text == "," && Self::word_at(t, i + 1) && (has_fin(&t[i + 1]) || CLITICS.contains(&t[i + 1].lower.as_str())) { i + 1 }
                else if matches!(l, "personne" | "rien" | "nul" | "nulle") && Self::word_at(t, i + 1) && matches!(t[i + 1].lower.as_str(), "ne" | "n’") { i + 1 }
                else { continue };
            if i > 0 && t[i - 1].word && (det_info(&t[i - 1]).is_some() || PREPS.contains(&t[i - 1].lower.as_str())) { continue; }
            if !(i == 0 || t[i].sent_start || !t[i - 1].word || t[i - 1].morph.iter().any(|m| matches!(m.pos, Pos::ConjC | Pos::ConjS))) { continue; }
            let v = self.skip_clitics(t, after, i);
            if !Self::word_at(t, v) || !self.verbish(&t[v]) { continue; }
            if t[v].morph.iter().any(|m| is_fin(m) && m.person & 4 != 0 && m.number & N_S != 0) { continue; }
            let sug = self.verb_forms_for(&t[v], 4, N_S);
            o.push(self.mk(&t[v], "FR_SUBJ_VERB", "Après « tout le monde, chacun, personne, rien », le verbe est au singulier.", sug));
        }
    }

    /// "Quelles villes avez-vous visité ?" -> visitées
    fn pp_interrogative(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !(t[s].sent_start || (s > 0 && !t[s - 1].word)) { continue; }
            let (g, n, noun) = match t[s].lower.as_str() {
                "quel" => (G_M, N_S, s + 1), "quelle" => (G_F, N_S, s + 1), "quels" => (G_M, N_P, s + 1), "quelles" => (G_F, N_P, s + 1),
                "combien" if Self::word_at(t, s + 2) && matches!(t[s + 1].lower.as_str(), "de" | "d’") => {
                    let nn = t[s + 2].morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.number);
                    let gg = t[s + 2].morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender);
                    if !single(nn) || !single(gg) { continue; }
                    (gg, nn, s + 2)
                }
                _ => continue,
            };
            if !Self::word_at(t, noun) || !has_pos(&t[noun], Pos::Noun) { continue; }
            let Some(a) = (noun + 1..(noun + 7).min(t.len())).find(|&k| t[k].word && t[k].sid == t[s].sid && self.fin_lemma(&t[k], "avoir")) else { continue };
            // the fronted noun is the object only if the auxiliary has its own subject (avez-vous, ... il a ...)
            let own_subject = t[a].inv.is_some() || (noun + 1..a).any(|k| t[k].word && (has_pos(&t[k], Pos::PronSubj) || det_info(&t[k]).is_some() || has_pos(&t[k], Pos::Proper)));
            let incise = (noun + 1..a).any(|k| !t[k].word && t[k].text == ",");
            let measure = TIME_NOUNS.iter().any(|w| t[noun].lower.trim_end_matches('s') == *w || t[noun].lower == *w) || matches!(t[noun].lower.as_str(), "ans" | "saisons" | "kilomètres" | "euros" | "mètres" | "minutes" | "heures" | "jours" | "semaines" | "mois" | "années" | "fois");
            if !own_subject || incise || measure || (a > 0 && t[a - 1].lower == "y") { continue; }
            let p = a + 1;
            if !Self::word_at(t, p) || !Self::same(t, a, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| PP_INVARIABLE.contains(&self.lemma_of(m))) { continue; }
            if qr.iter().any(|m| m.gender & g != 0 && m.number & n != 0 && (m.gender != 3 || m.number != 3)) { continue; }
            if Self::word_at(t, p + 1) && t[p + 1].morph.iter().any(|m| m.moods & M_Y != 0) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, g, n);
            o.push(self.mk(&t[p], "FR_PP_COD", "Le participe s’accorde avec le complément placé avant (interrogation).", sug));
        }
    }

    /// "Des pommes, j'en ai mangées" -> mangé
    fn pp_en(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || t[i].lower != "en" || !self.fin_lemma(&t[i + 1], "avoir") { continue; }
            let p = &t[i + 2];
            if !p.word || p.sid != t[i].sid { continue; }
            let qr: Vec<&Morph> = p.morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| ms_only(m)) { continue; }
            if (i.saturating_sub(8)..i).any(|k| t[k].sid == t[i].sid && matches!(t[k].lower.as_str(), "que" | "qu’" | "les" | "la" | "l’")) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, G_M, N_S);
            o.push(self.mk(p, "FR_PP_COD", "Avec « en », le participe passé reste invariable.", sug));
        }
    }

    /// "Elle a achetées des fleurs" -> acheté (no direct object before the participle)
    fn pp_no_preceding_object(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for a in 0..t.len().saturating_sub(1) {
            if !t[a].word || !self.fin_lemma(&t[a], "avoir") || t[a].morph.iter().any(|m| m.pos == Pos::Noun && t[a].lower != "a") { continue; }
            let mut p = a + 1;
            if p < t.len() && !t[p].word && matches!(t[p].text.as_str(), "," | "—" | "(") {
                let close = match t[p].text.as_str() { "(" => ")", x => x };
                if let Some(c) = (p + 1..(p + 18).min(t.len())).find(|&x| !t[x].word && t[x].text == close) { p = c + 1; }
            }
            while Self::word_at(t, p) && (ADVERBS.contains(&t[p].lower.as_str()) || all_pos(&t[p], Pos::Adv)) && p <= a + 3 { p += 1; }
            if !Self::word_at(t, p) || !Self::same(t, a, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| ms_only(m)) || has_fin(&t[p]) { continue; }
            if qr.iter().any(|m| PP_INVARIABLE.contains(&self.lemma_of(m))) { continue; }
            // any possible preceding object: clitic before the auxiliary, que/qu', interrogative, en
            // a direct-object clitic right before the auxiliary (possibly behind lui/leur/y/ne): "les lui avait ôtées"
            let mut b = a;
            let mut obj_clitic = false;
            while b > 0 && t[b - 1].word && t[b - 1].sid == t[a].sid && (CLITICS.contains(&t[b - 1].lower.as_str()) || matches!(t[b - 1].lower.as_str(), "lui" | "leur" | "y" | "en" | "ne" | "n’")) {
                let l = t[b - 1].lower.as_str();
                let subj_pron = matches!(l, "nous" | "vous") && !(b >= 2 && t[b - 2].word && (has_pos(&t[b - 2], Pos::PronSubj) || CLITICS.contains(&t[b - 2].lower.as_str())));
                if (OBJ_CLITICS.contains(&l) && !subj_pron) || l == "en" { obj_clitic = true; }
                b -= 1;
            }
            if obj_clitic { continue; }
            if t[a].inv.is_some() && a > 0 && t[a - 1].word && OBJ_CLITICS.contains(&t[a - 1].lower.as_str()) { continue; }
            let ss = (0..=a).rev().find(|&k| t[k].sent_start).unwrap_or(0);
            // a relative "que" (after a noun or "ce") or an interrogative before the auxiliary may be the object
            const CONJ_BEFORE_QUE: &[&str] = &["bien", "pour", "afin", "avant", "sans", "après", "parce", "alors", "dès", "depuis", "pendant", "tandis", "lors", "ainsi", "sorte", "moins", "malgré", "plus", "autant", "aussi", "si", "tant", "pourvu", "condition", "peur", "crainte", "faut", "veux", "veut", "pense", "crois", "dit", "sait", "sûr", "certain", "possible", "probable", "est"];
            // a "que" in the 6 words before the auxiliary that is not a conjunction is a relative pronoun: possible preceding object
            let near_rel = (a.saturating_sub(6)..a).any(|k| t[k].word && t[k].sid == t[a].sid && matches!(t[k].lower.as_str(), "que" | "qu’")
                && !(k > 0 && t[k - 1].word && CONJ_BEFORE_QUE.contains(&t[k - 1].lower.as_str())));
            if near_rel { continue; }
            let relative = |k: usize| matches!(t[k].lower.as_str(), "que" | "qu’") && k > 1 && !t[k - 1].word && t[k - 1].text == "," && t[k - 2].word && (has_pos(&t[k - 2], Pos::Noun) || has_pos(&t[k - 2], Pos::Proper) || t[k - 2].morph.iter().any(adjish) || t[k - 2].morph.is_empty())
                || matches!(t[k].lower.as_str(), "que" | "qu’") && k > 0 && t[k - 1].word
                && !ADVERBS.contains(&t[k - 1].lower.as_str()) && (has_pos(&t[k - 1], Pos::Noun) || has_pos(&t[k - 1], Pos::Proper) || t[k - 1].morph.iter().any(adjish) || matches!(t[k - 1].lower.as_str(), "ce" | "c’" | "tout" | "celui" | "celle" | "ceux" | "celles"));
            if (ss..a).any(|k| t[k].word && (relative(k) || matches!(t[k].lower.as_str(), "quel" | "quelle" | "quels" | "quelles" | "combien" | "lequel" | "laquelle" | "lesquels" | "lesquelles" | "que" ) && k == ss)) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, G_M, N_S);
            o.push(self.mk(&t[p], "FR_PP_AVOIR", "Avec « avoir », le participe ne s’accorde pas avec un complément placé après lui.", sug));
        }
    }

    /// "Elles se sont téléphonées" -> téléphoné ; "Elle s'est lavée les mains" -> lavé
    fn pp_pron_indirect(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "se" | "s’" | "me" | "m’" | "te" | "t’" | "nous" | "vous") { continue; }
            let prev = if i > 0 && t[i - 1].word { t[i - 1].lower.as_str() } else { "" };
            let reflexive = match t[i].lower.as_str() { "se" | "s’" => true, "me" | "m’" => matches!(prev, "je" | "j’"), "te" | "t’" => prev == "tu", l => prev == l };
            if !reflexive { continue; }
            let e = i + 1;
            if !Self::word_at(t, e) || !self.fin_lemma(&t[e], "être") { continue; }
            let p = e + 1;
            if !Self::word_at(t, p) || !Self::same(t, i, p) { continue; }
            let qr: Vec<&Morph> = t[p].morph.iter().filter(|m| m.pos == Pos::Verb && m.moods & M_Q != 0).collect();
            if qr.is_empty() || qr.iter().any(|m| ms_only(m)) { continue; }
            if (i.saturating_sub(8)..i).any(|k| t[k].sid == t[i].sid && t[k].word && matches!(t[k].lower.as_str(), "que" | "qu’" | "quel" | "quelle" | "quels" | "quelles" | "combien")) { continue; }
            let indirect = qr.iter().any(|m| PRON_INDIRECT.contains(&self.lemma_of(m)))
                && !(self.lemma_of(qr[0]) == "rendre" && !Self::word_at(t, p + 1));
            let object_after = Self::word_at(t, p + 1) && Self::same(t, p, p + 1)
                && (det_info(&t[p + 1]).is_some() || t[p + 1].lower == "compte") && !t[p + 1].morph.iter().any(|m| m.pos == Pos::Prep)
                && !matches!(t[p + 1].lower.as_str(), "du" | "de" | "d’" | "des")
                && !self.np_at(t, p + 1).map_or(false, |np| qr.iter().any(|m| m.gender & np.gender != 0 && m.number & np.number != 0) && (np.end + 1 >= t.len() || !t[np.end + 1].word))
                && !t.get(p + 2).map_or(false, |h| TIME_NOUNS.contains(&h.lower.as_str()) || matches!(h.lower.as_str(), "long" | "côté" | "cours" | "bord" | "part")
                    || h.lower.chars().all(|c| c.is_ascii_digit()) || NUM_WORDS.contains(&h.lower.as_str()));
            let rendre_compte = self.lemma_of(qr[0]) == "rendre" && Self::word_at(t, p + 1) && t[p + 1].lower == "compte";
            if !(object_after || rendre_compte || (indirect && self.lemma_of(qr[0]) != "rendre")) { continue; }
            let sug = self.inflect_adj(self.lemma_of(qr[0]), true, G_M, N_S);
            o.push(self.mk(&t[p], "FR_PP_PRONOMINAL", "Pronom réfléchi complément indirect (ou complément direct placé après) : participe invariable.", sug));
        }
    }

    /// "Fatigué par le voyage, les filles ..." -> Fatiguées
    fn pp_detached(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !t[s].sent_start { continue; }
            let a = &t[s];
            if !a.morph.iter().any(adjish) || has_fin(a) || a.morph.iter().any(|m| m.pos == Pos::Noun && !adjish(m) && a.morph.iter().all(|x| x.pos == Pos::Noun)) { continue; }
            if a.morph.iter().any(|m| m.moods & M_P != 0) || a.lower.ends_with("ant") || a.lower == "eu" { continue; } // present participle: invariable; "Eu égard"
            if a.morph.iter().any(|m| matches!(m.pos, Pos::Prep | Pos::Adv | Pos::ConjC | Pos::ConjS) || m.pos.is_det()) { continue; }
            let Some(c) = (s + 1..(s + 12).min(t.len())).find(|&k| !t[k].word && t[k].text == ",") else { continue };
            if t[c].sid != t[s].sid { continue; }
            if (s + 1..c).any(|k| t[k].word && (has_fin(&t[k]) && t[k].morph.iter().all(|m| m.pos == Pos::Verb) || matches!(t[k].lower.as_str(), "soit" | "soient" | "sois")))
                || has_pos(a, Pos::PronSubj) { continue; }
            let k = c + 1;
            if !Self::word_at(t, k) { continue; }
            let (g, n) = if let Some(gn) = pron_gn(&t[k].lower) { gn }
                else if let Some(np) = self.np_at(t, k) { (np.gender, np.number) } else { continue };
            if !single(n) { continue; }
            if a.morph.iter().any(|m| adjish(m) && m.number & n != 0 && (m.gender & g != 0 || !single(g))) { continue; }
            let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { continue };
            let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, g, n);
            o.push(self.mk(a, "FR_ATTR_AGREE", "Le participe ou l’adjectif détaché s’accorde avec le sujet de la phrase.", sug));
        }
    }

    /// "Étant données les circonstances" -> Étant donné ; "Vues les conditions" -> Vu
    fn prep_participles(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word { continue; }
            let l = t[i].lower.as_str();
            let (target, base) = if l == "étant" && Self::word_at(t, i + 1) && matches!(t[i + 1].lower.as_str(), "donnée" | "données" | "donnés") { (i + 1, "donné") }
                else if (t[i].sent_start || (i > 0 && !t[i - 1].word)) && matches!(l, "vus" | "vue" | "vues") { (i, "vu") }
                else if (t[i].sent_start || (i > 0 && !t[i - 1].word)) && matches!(l, "exceptée" | "exceptés" | "exceptées") { (i, "excepté") }
                else if matches!(l, "ci-jointe" | "ci-joints" | "ci-jointes") && (t[i].sent_start || (i > 0 && !t[i - 1].word)) { (i, "ci-joint") }
                else { continue };
            let nx = target + 1;
            if !Self::word_at(t, nx) || det_info(&t[nx]).is_none() || matches!(t[nx].lower.as_str(), "de" | "d’" | "du" | "des") { continue; }
            o.push(self.mk(&t[target], "FR_PP_INVARIABLE", "Placé devant le nom, ce participe est invariable.", vec![base.to_string()]));
        }
    }

    /// "une affaire des plus délicate" -> délicates
    fn des_plus(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !(t[i].word && t[i].lower == "des" && Self::word_at(t, i + 1) && matches!(t[i + 1].lower.as_str(), "plus" | "moins" | "mieux")) { continue; }
            let a = &t[i + 2];
            if !a.word || !a.morph.iter().any(adjish) || a.morph.iter().any(|m| adjish(m) && m.number & N_P != 0) || has_fin(a) { continue; }
            let Some(m0) = a.morph.iter().find(|m| adjish(m)) else { continue };
            let sug = self.inflect_adj(self.lemma_of(m0), m0.pos == Pos::Verb, m0.gender, N_P);
            if sug.is_empty() || sug.iter().all(|s| *s == a.lower) { continue; } // invariable adjective ("chic")
            o.push(self.mk(a, "FR_ATTR_AGREE", "Après « des plus », l’adjectif se met au pluriel.", sug));
        }
    }

    /// "des yeux marrons" -> marron ; "des robes bleues claires" -> bleu clair
    fn colours(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 1..t.len() {
            if !t[i].word || !t[i - 1].word || !has_pos(&t[i - 1], Pos::Noun) { continue; }
            let l = t[i].lower.as_str();
            if let Some(base) = l.strip_suffix('s') {
                if COLOUR_NOUNS.contains(&base) && t[i - 1].morph.iter().any(|m| m.pos == Pos::Noun && m.number & N_P != 0) && !t[i - 1].morph.iter().any(|m| m.pos.is_det()) {
                    o.push(self.mk(&t[i], "FR_COLOUR", "Les noms employés comme adjectifs de couleur sont invariables.", vec![base.to_string()]));
                    continue;
                }
            }
            if Self::word_at(t, i + 1) && COLOUR_QUAL.contains(&t[i + 1].lower.as_str()) {
                let base = COLOUR_ADJ.iter().find(|c| l != **c && (l.starts_with(*c) || (**c == "blanc" && l.starts_with("blanch")) || (**c == "violet" && l.starts_with("violett"))));
                if let Some(b) = base {
                    o.push(self.mk(&t[i], "FR_COLOUR", "Adjectif de couleur composé : invariable.", vec![b.to_string()]));
                }
            }
        }
    }

    /// "une heure et demi" -> demie ; "deux kilos et demie" -> demi
    fn demi(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 2..t.len() {
            if !t[i].word || !matches!(t[i].lower.as_str(), "demi" | "demie" | "demis" | "demies") || t[i - 1].lower != "et" { continue; }
            let n = &t[i - 2];
            if !n.word { continue; }
            let g = n.morph.iter().filter(|m| m.pos == Pos::Noun).fold(0u8, |a, m| a | m.gender);
            if !single(g) { continue; }
            let want = if g == G_F { "demie" } else { "demi" };
            if t[i].lower != want { o.push(self.mk(&t[i], "FR_ATTR_AGREE", "« et demi » s’accorde en genre (jamais en nombre) avec le nom qui précède.", vec![want.to_string()])); }
        }
    }

    /// "Elle est tout contente" -> toute ; "elles sont tous contentes" -> toutes
    fn tout_adverb(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "tout" | "tous") { continue; }
            let a = &t[i + 1];
            if !a.word || vowel(&a.lower) || !a.morph.iter().any(|m| m.pos == Pos::Adj || (m.pos == Pos::Verb && m.moods & M_Q != 0)) { continue; }
            if !has_pos(a, Pos::Adj) && (has_pos(a, Pos::Noun) || has_fin(a)) { continue; } // noun/verb homographs only count when they are also adjectives
            if !a.morph.iter().filter(|m| adjish(m)).all(|m| m.gender == G_F) { continue; }
            if i > 0 && t[i - 1].word && (det_info(&t[i - 1]).is_some() || PREPS.contains(&t[i - 1].lower.as_str()) || has_pos(&t[i - 1], Pos::Prep)) { continue; }
            let n = a.morph.iter().filter(|m| adjish(m)).fold(0u8, |x, m| x | m.number);
            let want = if n == N_P { "toutes" } else { "toute" };
            o.push(self.mk(&t[i], "FR_TOUT", "« tout » adverbe varie devant un adjectif féminin commençant par une consonne.", vec![want.to_string()]));
        }
    }

    /// "Quelque soit la raison" -> Quelle que soit
    fn quel_que(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            // "Quel que soit la raison" -> Quelle que
            if t[i].word && matches!(t[i].lower.as_str(), "quel" | "quelle" | "quels" | "quelles") && t[i + 1].lower == "que"
                && Self::word_at(t, i + 2) && matches!(t[i + 2].lower.as_str(), "soit" | "soient" | "fût" | "fussent" | "fut") {
                if let Some(np) = self.np_at(t, i + 3) {
                    if single(np.gender) && single(np.number) {
                        let q = match (np.gender == G_F, np.number == N_P) { (false, false) => "quel", (true, false) => "quelle", (false, true) => "quels", (true, true) => "quelles" };
                        if t[i].lower != q { o.push(self.mk(&t[i], "FR_QUEL_QUE", "« quel que » s’accorde avec le sujet de « être ».", vec![q.to_string()])); }
                        let verb = if np.number == N_P { "soient" } else { "soit" };
                        if matches!(t[i + 2].lower.as_str(), "soit" | "soient") && t[i + 2].lower != verb { o.push(self.mk(&t[i + 2], "FR_SUBJ_VERB", "Le verbe s’accorde avec le sujet qui suit.", vec![verb.to_string()])); }
                    }
                }
            }
        }
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word || !matches!(t[i].lower.as_str(), "quelque" | "quelques") { continue; }
            if !Self::word_at(t, i + 1) || !matches!(t[i + 1].lower.as_str(), "soit" | "soient" | "fût" | "fussent" | "puisse" | "puissent") { continue; }
            let k = i + 2;
            let (g, n) = self.np_at(t, k).map(|np| (np.gender, np.number)).unwrap_or((G_M, if t[i + 1].lower.ends_with("nt") { N_P } else { N_S }));
            let q = match (g == G_F, n == N_P) { (false, false) => "quel", (true, false) => "quelle", (false, true) => "quels", (true, true) => "quelles" };
            o.push(Lint { start: t[i].start, end: t[i].end, rule: "FR_QUEL_QUE", message: "Devant le verbe être : « quel(le)(s) que » en deux mots, accordé avec le sujet.".into(),
                          suggestions: vec![match_case(&t[i].text, &format!("{q} que"))] });
        }
    }

    /// "quatre-vingt personnes" -> quatre-vingts ; "deux cent chaises" -> deux cents ; "deux milles" -> deux mille
    fn numbers(&self, t: &[Tok], o: &mut Vec<Lint>) {
        let is_num = |x: &Tok| x.word && (NUM_WORDS.contains(&x.lower.as_str()) || x.lower.chars().all(|c| c.is_ascii_digit()) || x.lower.contains("-vingt") || x.lower.starts_with("vingt-") || x.lower.contains("-dix"));
        for i in 0..t.len() {
            if !t[i].word { continue; }
            let l = t[i].lower.as_str();
            let next_num = i + 1 < t.len() && is_num(&t[i + 1]);
            let next_noun = Self::word_at(t, i + 1) && has_pos(&t[i + 1], Pos::Noun) && !next_num;
            let prev_mult = i > 0 && t[i - 1].word && NUM_WORDS.contains(&t[i - 1].lower.as_str()) && !matches!(t[i - 1].lower.as_str(), "un" | "une" | "cent" | "mille");
            if l == "quatre-vingt" && next_noun { o.push(self.mk(&t[i], "FR_NUMBER", "« quatre-vingts » prend un s quand il termine le nombre.", vec!["quatre-vingts".into()])); }
            if l == "cent" && prev_mult && next_noun { o.push(self.mk(&t[i], "FR_NUMBER", "« cent » multiplié prend un s quand il termine le nombre.", vec!["cents".into()])); }
            if l == "cents" && next_num { o.push(self.mk(&t[i], "FR_NUMBER", "« cent » suivi d’un autre nombre ne prend pas de s.", vec!["cent".into()])); }
            if l == "milles" && prev_mult && next_noun && t[i + 1].lower != "marins" {
                o.push(self.mk(&t[i], "FR_NUMBER", "« mille » (nombre) est invariable.", vec!["mille".into()]));
            }
        }
    }

    /// Subjunctive / indicative after a much wider trigger set.
    fn mood_triggers(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for q in 1..t.len() {
            if !t[q].word || !matches!(t[q].lower.as_str(), "que" | "qu’") { continue; }
            let p1 = &t[q - 1];
            if !p1.word || p1.sid != t[q].sid { continue; }
            let lem = |x: &Tok| -> Vec<&str> { x.morph.iter().map(|m| self.lemma_of(m)).collect() };
            let before = |k: usize| if q >= k && t[q - k].word && t[q - k].sid == t[q].sid { t[q - k].lower.as_str() } else { "" };
            let negated = (q.saturating_sub(6)..q).any(|k| t[k].word && t[k].sid == t[q].sid && matches!(t[k].lower.as_str(), "ne" | "n’"));
            // "Penses-tu que ...", "Crois-tu que ...": both moods are correct
            if (q.saturating_sub(3)..q).any(|k| t[k].inv.is_some() && t[k].morph.iter().any(|m| OPINION.contains(&self.lemma_of(m)))) { continue; }
            let opinion_verb = |x: &Tok| x.morph.iter().any(|m| is_fin(m) && OPINION.contains(&self.lemma_of(m)));
            let mut want: Option<bool> = None; // Some(true) = subjunctive required, Some(false) = indicative required
            let noun_use = q >= 2 && t[q - 2].word && det_info(&t[q - 2]).is_some() && has_pos(p1, Pos::Noun);
            let comparative = (q.saturating_sub(7)..q).any(|k| t[k].word && t[k].sid == t[q].sid && matches!(t[k].lower.as_str(), "plus" | "moins" | "autant" | "mieux" | "davantage" | "autre" | "même" | "aussi" | "si" | "tant" | "tel" | "telle" | "pire"));
            if comparative || noun_use { continue; }
            let se_douter = q >= 2 && matches!(t[q - 2].lower.as_str(), "se" | "s’" | "me" | "m’" | "te" | "t’" | "nous" | "vous") && p1.morph.iter().any(|m| self.lemma_of(m) == "douter");
            if p1.morph.iter().any(|m| is_fin(m) && SUBJ_VERBS.contains(&self.lemma_of(m))) && !se_douter { want = Some(true); }
            let etre_before = (q >= 2 && t[q - 2].word && self.fin_lemma(&t[q - 2], "être"))
                || (q >= 3 && t[q - 3].word && self.fin_lemma(&t[q - 3], "être") && ADVERBS.contains(&t[q - 2].lower.as_str()));
            const EMOTION: &[&str] = &["ravir", "surprendre", "étonner", "soulager", "désoler", "réjouir", "attrister", "choquer", "décevoir", "fâcher", "enchanter", "vexer", "contrarier", "inquiéter", "navrer", "peiner", "satisfaire", "rassurer", "consterner", "indigner", "scandaliser", "émouvoir", "flatter", "honorer", "toucher"];
            if (p1.morph.iter().any(|m| m.moods & M_Q != 0 && EMOTION.contains(&self.lemma_of(m))) && etre_before) || (p1.morph.iter().any(|m| adjish(m) && SUBJ_ADJ.contains(&self.lemma_of(m))) && etre_before) || (p1.lower == "temps" && before(2) == "est") || p1.lower == "dommage" { want = Some(true); }
            if matches!(p1.lower.as_str(), "pourvu" | "peur" | "crainte" | "condition" | "moins" | "afin" | "quoique") { want = Some(true); }
            if p1.lower == "ce" && before(2) == "à" && before(3) == "jusqu’" { want = Some(true); }
            if p1.lower == "attendant" && before(2) == "en" { want = Some(true); }
            if p1.lower == "mieux" && lem(&t[q.saturating_sub(2)]).contains(&"valoir") { want = Some(true); }
            if p1.lower == "semble" && before(2) == "il" { want = Some(true); }
            if p1.lower == "peut" && before(2) == "se" { want = Some(true); }
            let neg_word = (q.saturating_sub(6)..q).any(|k| t[k].word && t[k].sid == t[q].sid && matches!(t[k].lower.as_str(), "pas" | "plus" | "jamais" | "guère" | "point" | "nullement"));
            let opinion_negated = negated && neg_word && (q.saturating_sub(4)..q).any(|k| opinion_verb(&t[k]) && !t[k].morph.iter().any(|m| self.lemma_of(m) == "dire" || self.lemma_of(m) == "affirmer"));
            if opinion_negated { want = Some(true); }
            // indicative contexts (normative)
            if p1.lower == "après" || (p1.lower == "semble" && matches!(before(2), "me" | "m’" | "te" | "t’" | "lui" | "nous" | "vous" | "leur")) { want = Some(false); }
            if p1.morph.iter().any(|m| is_fin(m) && self.lemma_of(m) == "espérer") && !negated { want = Some(false); }
            if !negated && p1.morph.iter().any(|m| is_fin(m) && matches!(self.lemma_of(m), "penser" | "croire" | "affirmer" | "savoir")) && !(q >= 2 && t[q - 1].inv.is_some()) { want = Some(false); }
            if p1.morph.iter().any(|m| adjish(m) && matches!(self.lemma_of(m), "probable" | "certain" | "sûr" | "évident" | "clair" | "vrai" | "exact")) && !negated && before(2) != "peu" { want = Some(false); }
            let Some(subj_needed) = want else { continue };
            let skip_aside = |mut k: usize| -> usize {
                if k < t.len() && !t[k].word && matches!(t[k].text.as_str(), "," | "—" | "(") {
                    let close = if t[k].text == "(" { ")".to_string() } else { t[k].text.clone() };
                    if let Some(c) = (k + 1..(k + 20).min(t.len())).find(|&x| !t[x].word && t[x].text == close) { k = c + 1; }
                }
                k
            };
            let end = (q..t.len()).take_while(|&k| t[k].sid == t[q].sid).last().unwrap_or(q);
            let question = !t[end].word && t[end].text == "?";
            let after_si = (q.saturating_sub(8)..q).any(|k| t[k].word && t[k].sid == t[q].sid && matches!(t[k].lower.as_str(), "si" | "s’"));
            if !subj_needed && (question || after_si) { continue; }
            // subject of the subordinate, then its verb
            let s = skip_aside(q + 1);
            if !Self::word_at(t, s) { continue; }
            let (p, n, v0) = if let Some((p, n)) = pron_pn(&t[s].lower) { (p, n, s + 1) }
                else if let Some(np) = self.np_at(t, s) { (4u8, np.number, np.end + 1) }
                else if has_pos(&t[s], Pos::Proper) { (4, N_S, s + 1) } else { continue };
            let mut v = skip_aside(v0);
            if !single(n) { continue; }
            while Self::word_at(t, v) && (matches!(t[v].lower.as_str(), "ne" | "n’") || CLITICS.contains(&t[v].lower.as_str())) { v += 1; }
            if !Self::word_at(t, v) || !Self::same(t, q, v) || !self.verbish(&t[v]) { continue; }
            let x = &t[v];
            let has_subj = x.morph.iter().any(|m| m.moods & (M_SP | M_SQ) != 0 && m.person & p != 0 && m.number & n != 0);
            let has_ind = x.morph.iter().any(|m| m.moods & (M_IP | M_IQ | M_IF | M_IS | M_K) != 0 && m.person & p != 0 && m.number & n != 0);
            let Some(r) = x.morph.iter().find(|m| is_fin(m)) else { continue };
            let cond_only = x.morph.iter().any(|m| m.moods & M_K != 0) && !x.morph.iter().any(|m| m.moods & (M_IP | M_IQ | M_IF | M_IS) != 0);
            if subj_needed && !has_subj && has_ind && !(opinion_negated && cond_only) {
                let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_SP != 0 && f.person & p != 0 && f.number & n != 0);
                o.push(self.mk(x, "FR_SUBJONCTIF", "Le subjonctif est requis après cette expression.", sug));
            } else if !subj_needed && has_subj && !has_ind && !matches!(x.lower.as_str(), "eût" | "fût" | "eussent" | "fussent") {
                let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_IP != 0 && f.person & p != 0 && f.number & n != 0);
                o.push(self.mk(x, "FR_INDICATIF", "L’indicatif est requis après cette expression.", sug));
            }
        }
    }

    /// "Si j'avais le temps, je viendrai" -> viendrais ; "Si tu veux, je viendrais" -> viendrai
    fn si_future_conditional(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len() {
            if !t[i].word || !(t[i].lower == "si" || t[i].lower == "s’") || !(t[i].sent_start || (i > 0 && !t[i - 1].word)) { continue; }
            let Some(sv) = (i + 1..(i + 6).min(t.len())).find(|&k| t[k].word && self.verbish(&t[k])) else { continue };
            let imparfait = t[sv].morph.iter().any(|m| m.moods & M_IQ != 0) && !t[sv].morph.iter().any(|m| m.moods & M_IP != 0);
            let present = t[sv].morph.iter().any(|m| m.moods & M_IP != 0) && !t[sv].morph.iter().any(|m| m.moods & (M_IQ | M_K) != 0);
            if !imparfait && !present { continue; }
            let Some(c) = (sv..(sv + 15).min(t.len())).find(|&k| !t[k].word && t[k].text == ",") else { continue };
            if t[c].sid != t[i].sid { continue; }
            let k = c + 1;
            let found: Option<(u8, u8, usize)> = if Self::word_at(t, k) && pron_pn(&t[k].lower).is_some() {
                let (p, n) = pron_pn(&t[k].lower).unwrap(); Some((p, n, self.skip_clitics(t, k + 1, i)))
            } else {
                (k..(k + 16).min(t.len())).take_while(|&j| t[j].sid == t[i].sid).find(|&j| t[j].word && (t[j].inv.is_some() || (self.verbish(&t[j]) && !CLITICS.contains(&t[j].lower.as_str()) && !has_pos(&t[j], Pos::Noun))))
                    .and_then(|j| if let Some((p, n)) = t[j].inv { Some((p, n, j)) } else { t[j].morph.iter().find(|m| is_fin(m)).map(|m| (m.person, m.number, j)) })
            };
            let Some((p, n, k)) = found else { continue };
            if !Self::word_at(t, k) { continue; }
            let v = &t[k];
            let fut = v.morph.iter().any(|m| m.moods & M_IF != 0 && m.person & p != 0 && m.number & n != 0);
            let cond = v.morph.iter().any(|m| m.moods & M_K != 0 && m.person & p != 0 && m.number & n != 0);
            let Some(r) = v.morph.iter().find(|m| is_fin(m)) else { continue };
            if imparfait && fut && !cond {
                let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_K != 0 && f.person & p != 0 && f.number & n != 0);
                o.push(self.mk(v, "FR_SI_COND", "Si + imparfait : conditionnel dans la principale.", sug));
            } else if present && cond && !fut {
                let rest = v.text.find('-').map(|c| v.text[c..].to_string()).unwrap_or_default();
                let sug = self.forms_where(self.lemma_of(r), |f| f.pos == Pos::Verb && f.moods & M_IF != 0 && f.person & p != 0 && f.number & n != 0).into_iter().map(|f| format!("{f}{rest}")).collect();
                o.push(self.mk(v, "FR_SI_COND", "Si + présent : futur (ou présent) dans la principale, pas le conditionnel.", sug));
            }
        }
    }

    /// "Manges ta soupe !" -> Mange ; "Vas à l'école !" -> Va
    fn imperative_s(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for s in 0..t.len() {
            if !t[s].word || !t[s].sent_start || t[s].inv.is_some() || t[s].compound { continue; }
            let x = &t[s];
            let end = (s..t.len()).take_while(|&k| t[k].sid == x.sid).last().unwrap_or(s);
            if !t[end].word && t[end].text == "?" { continue; }
            if !(x.lower.ends_with("es") || x.lower == "vas") { continue; }
            if x.morph.iter().any(|m| m.moods & M_E != 0) || !x.morph.iter().any(|m| m.pos == Pos::Verb) { continue; }
            if !(Self::word_at(t, s + 1) || (s + 1 < t.len() && t[s + 1].text == ",")) || (t[s + 1].word && has_pos(&t[s + 1], Pos::PronSubj) && !has_pos(&t[s + 1], Pos::PronObj)) { continue; }
            let nl = t[s + 1].lower.as_str();
            let comma_next = s + 1 < t.len() && !t[s + 1].word && t[s + 1].text == ",";
            if has_pos(x, Pos::Noun) && !comma_next && (matches!(nl, "de" | "des" | "du" | "d’" | "que" | "qu’" | "qui") || has_pos(&t[s + 1], Pos::Prep) || !(det_info(&t[s + 1]).is_some() || has_pos(&t[s + 1], Pos::Adv))) { continue; }
            let Some(m0) = x.morph.iter().find(|m| m.moods & M_IP != 0 && m.person & 2 != 0 && m.number & N_S != 0) else { continue };
            let lemma = self.lemma_of(m0);
            if !(lemma.ends_with("er") || lemma == "aller") { continue; }
            let sug = self.forms_where(lemma, |f| f.pos == Pos::Verb && f.moods & M_E != 0 && f.person & 2 != 0 && f.number & N_S != 0);
            o.push(self.mk(x, "FR_IMPERATIF", "Impératif des verbes en -er : pas de s à la 2e personne du singulier (sauf devant en/y).", sug));
        }
    }

    /// "On a rien vu" -> "On n’a rien vu"
    fn on_negation(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(2) {
            if !t[i].word || t[i].lower != "on" || !Self::word_at(t, i + 1) || !Self::same(t, i, i + 1) { continue; }
            let v = &t[i + 1];
            if !vowel(&v.lower) || !self.verbish(v) { continue; }
            let mut window = Vec::new();
            let mut k = i + 2;
            while k < t.len() && t[k].sid == t[i].sid && window.len() < 6 {
                if !t[k].word {
                    if matches!(t[k].text.as_str(), "," | "—" | "(") {
                        let close = if t[k].text == "(" { ")".to_string() } else { t[k].text.clone() };
                        match (k + 1..(k + 15).min(t.len())).find(|&x| !t[x].word && t[x].text == close) { Some(c) => { k = c + 1; continue; } None => break }
                    }
                    break;
                }
                if matches!(t[k].lower.as_str(), "que" | "qu’" | "qui" | "si" | "quand" | "mais" | "et") { break; }
                window.push(k); k += 1;
            }
            let neg = window.iter().copied().any(|k| t[k].word && t[k].sid == t[i].sid && (matches!(t[k].lower.as_str(), "rien" | "personne" | "jamais" | "pas" | "aucun" | "aucune" | "guère" | "nullement")
                || (t[k].lower == "plus" && Self::word_at(t, k + 1) && matches!(t[k + 1].lower.as_str(), "rien" | "jamais" | "personne" | "aucun" | "aucune"))));
            if !neg { continue; }
            if t[i + 2].lower == "plus" && (i + 3 >= t.len() || !t[i + 3].word || t[i + 3].lower == "de") { continue; } // "on a plus de temps"
            o.push(Lint { start: t[i].start, end: v.end, rule: "FR_ON_NEG", message: "Négation : « on n’ » devant une voyelle.".into(),
                          suggestions: vec![format!("{} n’{}", t[i].text, v.text)] });
        }
    }

    /// "vous mangé" -> mangez ; "il mangé" -> mange
    fn pronoun_participle(&self, t: &[Tok], o: &mut Vec<Lint>) {
        for i in 0..t.len().saturating_sub(1) {
            if !t[i].word { continue; }
            let Some((p, n)) = pron_pn(&t[i].lower) else { continue };
            if i > 0 && (!t[i - 1].word && t[i - 1].text == "-" || t[i - 1].lower.ends_with('-') || (t[i - 1].word && (self.fin_lemma(&t[i - 1], "être") || self.fin_lemma(&t[i - 1], "avoir")))) { continue; }
            if matches!(t[i].lower.as_str(), "nous" | "vous") && i > 0 && t[i - 1].word && (has_pos(&t[i - 1], Pos::PronSubj) || has_fin(&t[i - 1]) || PREPS.contains(&t[i - 1].lower.as_str())) { continue; }
            let v = self.skip_clitics(t, i + 1, i);
            if !Self::word_at(t, v) || !Self::same(t, i, v) { continue; }
            let x = &t[v];
            if x.morph.is_empty() || !x.morph.iter().all(|m| m.pos == Pos::Verb && m.moods & M_Q != 0) || !x.lower.ends_with('é') { continue; }
            let Some(m0) = x.morph.first() else { continue };
            let lemma = self.lemma_of(m0);
            if !lemma.ends_with("er") { continue; }
            let sug = self.forms_where(lemma, |f| f.pos == Pos::Verb && f.moods & M_IP != 0 && f.person & p != 0 && f.number & n != 0);
            o.push(self.mk(x, "FR_ER_E", "Après un pronom sujet, il faut un verbe conjugué, pas un participe passé.", sug));
        }
    }
}
