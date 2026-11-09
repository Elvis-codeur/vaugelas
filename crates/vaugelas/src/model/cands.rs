//! Port of training-design/ebm_cands2.py (candidate generator v2 + GEN3/GEN4), for the v14 flag set only:
//! FRC_GEN3=1, FRC_GEN4=1, FRC_SENT=1, FRC_LEXHOM=1 (+ FRC_LEXHOM_GROUP=1, used by ebm_data2), FRC_LMFEAT=1 (the
//! `CAPBOTH` capital question), FRC_LEMMA=1 (lemma atoms, through the lexicon). Branches of flags that are off in v14
//! (v2 without GEN3: truncate-first, PREPS at rank 5, paronyms at rank 7, ...) are not ported.
//!
//! | Rust                     | Python (ebm_cands2.py)                     |
//! |--------------------------|--------------------------------------------|
//! | `midsent`                | `midsent(ws, i)` with SENT = True          |
//! | `Gen::paradigm_low2`     | `paradigm_low2(low)` (GEN3 ranking)        |
//! | `Gen::near_known`        | `near_known(low)`                          |
//! | `Gen::regularised`       | `regularised(low)`                         |
//! | `Gen::ctx_alts`          | `ctx_alts(ws, i)`                          |
//! | `Gen::reads3`            | `_reads3(f, ws, i, written)`               |
//! | `Gen::cand_list`         | `cand_list(ws, i)`                         |
//! | `Gen::readings`          | ebm_data.py `readings(form, toks, i)`      |
//!
//! Python iterates a few sets of strings (hash order). Every such place is order-independent here because the result
//! is reduced by `min` (the `add` priorities) or sorted afterwards, except `ctx_alts`, whose final sort is stable over
//! a set: ties are broken by the form (see `ctx_alts`).
use crate::text::characterize::{Lexicon, Tok};
use crate::model::drops::Drops;
use crate::model::ebm_data::EbmData;
use crate::text::natural::{Natural, Reading, E, IF, IP, IQ, IS, K, P, Q, SP, SQ, Y};
use crate::text::pystr::{is_lower, is_upper, lower, str_isalpha, str_isupper, upper};
use crate::model::resources::Resources;
use std::collections::{BTreeSet, HashMap};

/// `MAXC`: candidates per list, written form included.
pub const MAXC: usize = 16;
/// `MAXF_DATA` (FRC_MAXF=16): atoms per reading the model keeps; a longer reading is reported (`reading_over_maxf`).
pub const MAXF_DATA: usize = 16;
/// `SENT_START`: a word right after one of these starts a sentence (FRC_SENT=1).
pub const SENT_START: [&str; 7] = [".", "!", "?", "«", "—", ":", "…"];

/// One candidate: (form, reading keys with their extra atoms).
pub type Cand = (String, Vec<String>);
/// One candidate set: (word index, gold index, candidates); the written form is candidate 0.
pub type CandSet = (usize, usize, Vec<Cand>);

/// `midsent(ws, i)` with FRC_SENT=1: not the first word, and not right after a sentence-ending mark.
pub fn midsent(ws: &[String], i: usize) -> bool {
    i > 0 && !SENT_START.contains(&ws[i - 1].as_str())
}

const FIN: u16 = IP | SP | IF | K | IQ | IS | E | SQ;
/// `NEAR3` (GEN3): near moods of each finite mood, NEAR plus present/subjunctive <-> conditional etc.
fn near3(m: u16) -> u16 {
    match m {
        IP => SP | IQ | IF | K,
        SP => IP | IF | K | IQ,
        IF => K | IP | IQ | SP,
        K => IF | IQ | IP | SP | SQ,
        IQ => K | IP | SQ | IF | SP,
        IS => IQ | SQ,
        E => IP,
        SQ => IQ | SP | IS | K,
        _ => 0,
    }
}

const PREPS: [&str; 14] = ["à", "de", "en", "dans", "par", "pour", "sur", "avec", "chez", "sans", "sous", "vers", "contre", "entre"];
const PREPS3: [&str; 21] = ["à", "de", "en", "dans", "par", "pour", "sur", "avec", "au", "aux", "du", "des", "chez", "sans", "sous", "vers",
                            "contre", "entre", "depuis", "pendant", "parmi"];
const ARTS: [&str; 8] = ["le", "la", "les", "l’", "au", "aux", "du", "des"];
const CLITICS: [&str; 7] = ["y", "en", "le", "la", "les", "lui", "leur"];
const L_GROUP: [&str; 9] = ["l’", "lui", "le", "la", "les", "leur", "y", "en", "eux"];
const RELS: [&str; 15] = ["qui", "que", "qu’", "dont", "où", "lequel", "laquelle", "lesquels", "lesquelles", "auquel", "auxquels", "auxquelles",
                          "duquel", "desquels", "desquelles"];
const QUEL_LEQUEL: [(&str, &str); 4] = [("quel", "lequel"), ("quelle", "laquelle"), ("quels", "lesquels"), ("quelles", "lesquelles")];
const HOMO2: [&[&str]; 30] = [
    &["du", "dû"], &["sur", "sûr"], &["mur", "mûr"], &["cru", "crû"], &["peu", "peut", "peux"], &["ça", "sa", "çà"], &["c’", "s’"],
    &["c'", "s'"], &["vers", "verre", "verres", "vert", "verts", "ver"], &["cour", "cours", "court", "courent"], &["près", "prêt"],
    &["quelque", "quel"], &["quelques", "quels", "quelles"], &["notre", "nôtre"], &["votre", "vôtre"], &["nôtres", "notres"],
    &["voie", "voix", "vois", "voit"], &["foi", "fois", "foie"], &["tant", "temps", "tends", "tend"], &["si", "s’y", "ci"],
    &["sans", "s’en", "cent", "sang"], &["mais", "mes", "met", "mets"], &["ni", "n’y"], &["dans", "d’en"], &["pair", "paire", "père", "perd", "pers"],
    &["poids", "pois", "poix"], &["tache", "tâche"], &["cou", "coup", "coût"], &["compte", "comte", "conte"], &["chair", "chaire", "cher", "chère"],
];
const PREFIX: [(&str, &str); 11] = [("demie", "demi"), ("demis", "demi"), ("demies", "demi"), ("semie", "semi"), ("semis", "semi"),
                                    ("semies", "semi"), ("mie", "mi"), ("mis", "mi"), ("nue", "nu"), ("nus", "nu"), ("nues", "nu")];
fn prefix_of(h: &str) -> Option<&'static str> { PREFIX.iter().find(|(a, _)| *a == h).map(|(_, b)| *b) }
const PARONYMS: [(&str, &str); 72] = [("éminent", "imminent"), ("recouvrer", "recouvrir"), ("venimeux", "vénéneux"), ("conjecture", "conjoncture"),
    ("effraction", "infraction"), ("affliger", "infliger"), ("collision", "collusion"), ("éruption", "irruption"),
    ("prolongation", "prolongement"), ("perpétrer", "perpétuer"), ("acceptation", "acception"), ("allocation", "allocution"),
    ("amener", "emmener"), ("apporter", "emporter"), ("avènement", "événement"), ("compréhensible", "compréhensif"),
    ("consommer", "consumer"), ("décade", "décennie"), ("émigrer", "immigrer"), ("émigrant", "immigrant"), ("inclinaison", "inclination"),
    ("infecter", "infester"), ("notable", "notoire"), ("oppresser", "opprimer"), ("prodige", "prodigue"), ("conjecturer", "conjoncturer"),
    ("éminence", "imminence"), ("évoquer", "invoquer"), ("explicite", "implicite"), ("induire", "enduire"), ("importun", "opportun"),
    ("justesse", "justice"), ("luxuriant", "luxueux"), ("original", "originel"), ("partial", "partiel"), ("précepteur", "percepteur"),
    ("raisonner", "résonner"), ("rebattre", "rabattre"), ("suggestion", "sujétion"), ("allusion", "illusion"), ("éruptif", "irruptif"),
    ("agoniser", "agonir"), ("anoblir", "ennoblir"), ("arborer", "arborescent"), ("astrologue", "astronome"), ("colorer", "colorier"),
    ("dénué", "dénudé"), ("désaffection", "désaffectation"), ("éclaircir", "éclairer"), ("effectif", "efficace"), ("éloquent", "élogieux"),
    ("enduire", "induire"), ("épancher", "étancher"), ("évasif", "évasé"), ("exalter", "exulter"), ("fortuit", "fortuné"),
    ("habileté", "habilité"), ("inculper", "inculquer"), ("indemne", "indemnisé"), ("intégral", "intègre"), ("littéral", "littéraire"),
    ("matériau", "matériel"), ("ombrageux", "ombragé"), ("perpétuer", "perpétrer"), ("précédent", "président"), ("proscrire", "prescrire"),
    ("sensé", "censé"), ("tâche", "tache"), ("voire", "voir"), ("près", "prêt"), ("plutôt", "plus tôt"), ("davantage", "d’avantage")];
const LEQUELS: [&str; 10] = ["lequel", "laquelle", "lesquels", "lesquelles", "duquel", "desquels", "desquelles", "auquel", "auxquels", "auxquelles"];
const PREP_LEQ: [&str; 15] = ["avec", "pour", "parmi", "sur", "sous", "dans", "entre", "par", "chez", "vers", "contre", "sans", "selon", "malgré", "outre"];
const CLITIC_PAIRS: [(&str, &str); 4] = [("-me", "-moi"), ("-moi", "-me"), ("-te", "-toi"), ("-toi", "-te")];
/// `IMP_CLITICS`
pub const IMP_CLITICS: [&str; 13] = ["-moi", "-toi", "-le", "-la", "-les", "-lui", "-leur", "-nous", "-vous", "-en", "-y", "-me", "-te"];
const ELISION: [(&str, &[&str]); 28] = [("ce", &["c’"]), ("c’", &["ce"]), ("le", &["l’"]), ("la", &["l’"]), ("l’", &["le", "la"]), ("de", &["d’"]),
    ("d’", &["de"]), ("que", &["qu’"]), ("qu’", &["que"]), ("je", &["j’"]), ("j’", &["je"]), ("ne", &["n’"]), ("n’", &["ne"]), ("se", &["s’"]),
    ("s’", &["se", "si"]), ("si", &["s’"]), ("me", &["m’"]), ("m’", &["me"]), ("te", &["t’"]), ("t’", &["te"]), ("presque", &["presqu’"]),
    ("presqu’", &["presque"]), ("puisque", &["puisqu’"]), ("puisqu’", &["puisque"]), ("lorsque", &["lorsqu’"]), ("lorsqu’", &["lorsque"]),
    ("jusque", &["jusqu’"]), ("jusqu’", &["jusque"])];
fn elision_of(w: &str) -> Option<&'static [&'static str]> { ELISION.iter().find(|(a, _)| *a == w).map(|(_, b)| *b) }
const IRREG: [(&str, &[&str]); 19] = [("monsieurs", &["messieurs"]), ("madames", &["mesdames"]), ("mademoiselles", &["mesdemoiselles"]),
    ("bonhommes", &["bonshommes"]), ("gentilhommes", &["gentilshommes"]), ("œils", &["yeux"]), ("ciels", &["cieux"]), ("roi", &["reine"]),
    ("reine", &["roi"]), ("compagnon", &["compagne"]), ("compagne", &["compagnon"]), ("loup", &["louve"]), ("louve", &["loup"]),
    ("dieu", &["déesse"]), ("héros", &["héroïne"]), ("empereur", &["impératrice"]), ("duc", &["duchesse"]), ("âne", &["ânesse"]),
    ("yeux-de-bœuf", &["œils-de-bœuf"])];
/// `ENDINGS`: regularised conjugation ending -> (mood, person bits, number bits)
const ENDINGS: [(&str, u16, u8, u8); 31] = [("eraient", K, 4, 2), ("erions", K, 1, 2), ("eriez", K, 2, 2), ("erais", K, 3, 1), ("erait", K, 4, 1),
    ("erons", IF, 1, 2), ("eront", IF, 4, 2), ("erez", IF, 2, 2), ("erai", IF, 1, 1), ("eras", IF, 2, 1), ("era", IF, 4, 1),
    ("rons", IF, 1, 2), ("ront", IF, 4, 2), ("rez", IF, 2, 2), ("rai", IF, 1, 1), ("ras", IF, 2, 1), ("ra", IF, 4, 1),
    ("aient", IQ, 4, 2), ("ions", IQ, 1, 2), ("iez", IQ, 2, 2), ("ais", IQ, 3, 1), ("ait", IQ, 4, 1),
    ("ons", IP, 1, 2), ("ez", IP, 2, 2), ("ent", IP, 4, 2), ("es", IP, 2, 1), ("e", IP, 5, 1), ("s", IP, 3, 1),
    ("t", IP, 4, 1), ("d", IP, 4, 1), ("x", IP, 3, 1)];
/// NATION = `(ien|ienne|ais|aise|ois|oise|ain|aine|and|ande|ol|ole|an|ane|ec|ecque|in|ine)s?$` (search)
const NATION: [&str; 18] = ["ien", "ienne", "ais", "aise", "ois", "oise", "ain", "aine", "and", "ande", "ol", "ole", "an", "ane", "ec", "ecque", "in", "ine"];
fn nation(w: &str) -> bool {
    let core = w.strip_suffix('s');
    NATION.iter().any(|x| w.ends_with(x) || core.is_some_and(|c| c.ends_with(x)))
}
const REG_PLURAL: [(&str, &str); 9] = [("eus", "eux"), ("aus", "aux"), ("eaus", "eaux"), ("ous", "oux"), ("als", "aux"), ("ails", "aux"), ("aux", "aus"),
                                       ("eux", "eus"), ("oux", "ous")];
const FEM_EURE: [(&str, &str); 4] = [("teure", "trice"), ("eure", "euse"), ("teuse", "trice"), ("euse", "trice")];
const P_TO_A: [(&str, &str); 5] = [("quant", "cant"), ("guant", "gant"), ("geant", "gent"), ("ant", "ent"), ("ant", "ant")];
const GENDER_PAIRS: [(&str, &str); 14] = [("e", "esse"), ("", "esse"), ("c", "chesse"), ("er", "ère"), ("ier", "ière"), ("ien", "ienne"), ("on", "onne"),
                                          ("et", "ette"), ("el", "elle"), ("eur", "euse"), ("eur", "rice"), ("f", "ve"), ("x", "se"), ("", "e")];
const SPECIAL: [(&str, &str); 2] = [("vas-t’en", "va-t’en"), ("vas-t'en", "va-t'en")];
const SPECIAL_READ: [&str; 2] = ["va-t’en", "va-t'en"];
const SPECIAL_READING: &str = "pos=V|n=s|p=2|mood=E|grp=3|lw=aller";
/// `CLOSED` (= featmodel.CLOSED)
pub const CLOSED: [&str; 5] = ["DET", "PREP", "PRON", "CONJ", "NEG"];

/// `s[:n]` in code points
pub fn cp_prefix(s: &str, n: usize) -> &str { match s.char_indices().nth(n) { Some((b, _)) => &s[..b], None => s } }
/// `s[:-n]` in code points (Python: `s[:len(s)-n]`, empty when n >= len)
pub fn cp_drop_last(s: &str, n: usize) -> &str {
    let len = s.chars().count();
    cp_prefix(s, len.saturating_sub(n))
}
fn cp_len(s: &str) -> usize { s.chars().count() }
/// `f[0].upper() + f[1:]`
pub fn cap_first(f: &str) -> String {
    let mut c = f.chars();
    match c.next() { Some(h) => upper(&h.to_string()) + c.as_str(), None => String::new() }
}
fn first_is_upper(s: &str) -> bool { s.chars().next().is_some_and(is_upper) }
fn first_is_lower(s: &str) -> bool { s.chars().next().is_some_and(is_lower) }

/// The priorities of `paradigm_low2` (`pri` and its `add`).
struct Pri<'a> { low: &'a str, nat: &'a Natural, map: HashMap<String, u8> }
impl Pri<'_> {
    fn add1(&mut self, f: &str, k: u8) {
        let f: String = if first_is_upper(f) { let l = lower(f); if self.nat.read.contains_key(&l) { l } else { f.to_string() } } else { f.to_string() };
        if f == self.low { return; }
        match self.map.get_mut(&f) { Some(v) => { if *v > k { *v = k; } } None => { self.map.insert(f, k); } }
    }
    fn add<S: AsRef<str>>(&mut self, forms: impl IntoIterator<Item = S>, k: u8) { for f in forms { self.add1(f.as_ref(), k); } }
}

/// One sentence being processed: words, lower-cased words, and its characterisation (`CH.sentence(ws)`), whose
/// first `i` tokens are what `CH.sentence` computes for any sentence that differs from `ws` only at `i`.
pub struct Sent<'a> {
    pub ws: &'a [String],
    pub low: Vec<String>,
    pub base: Vec<Tok>,
}

impl<'a> Sent<'a> {
    pub fn new(lex: &Lexicon, ws: &'a [String]) -> Self {
        Sent { ws, low: ws.iter().map(|w| lower(w)).collect(), base: lex.sentence(ws) }
    }
}

/// The generator: the lexicon tables, the resources and the module-level tables of ebm_cands2.py.
pub struct Gen<'a> {
    pub lex: &'a Lexicon,
    pub nat: &'a Natural,
    pub res: &'a Resources,
    pub data: EbmData,
    lem_id: HashMap<&'a str, u32>,
    homo2_of: HashMap<&'static str, BTreeSet<&'static str>>,
    par_of: HashMap<&'static str, BTreeSet<&'static str>>,
    /// `_BYPRE`: first 3 code points -> forms (READ order) that are alphabetic or contain "-" or "’"
    bypre: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Gen<'a> {
    pub fn new(lex: &'a Lexicon, nat: &'a Natural, res: &'a Resources) -> Self {
        let lem_id: HashMap<&str, u32> = nat.lemmas.iter().enumerate().map(|(i, l)| (l.as_str(), i as u32)).collect();
        let mut homo2_of: HashMap<&'static str, BTreeSet<&'static str>> = HashMap::new();
        for s in HOMO2 { for w in s.iter() { homo2_of.entry(*w).or_default().extend(s.iter().copied()); } }
        let mut par_of: HashMap<&'static str, BTreeSet<&'static str>> = HashMap::new();
        for (a, b) in PARONYMS { par_of.entry(a).or_default().insert(b); par_of.entry(b).or_default().insert(a); }
        // READ is a dict filled in lexicon order: its key order is the order of each form's first reading
        let mut forms: Vec<(&u32, &str)> = nat.read.iter().filter_map(|(f, ids)| ids.first().map(|i| (i, f.as_str()))).collect();
        forms.sort_unstable();
        let mut bypre: HashMap<&str, Vec<&str>> = HashMap::new();
        for (_, f) in forms {
            if str_isalpha(f) || f.contains('-') || f.contains('’') { bypre.entry(cp_prefix(f, 3)).or_default().push(f); }
        }
        Gen { lex, nat, res, data: EbmData::new(nat), lem_id, homo2_of, par_of, bypre }
    }

    fn r(&self, id: u32) -> &'a Reading { &self.nat.readings[id as usize] }
    /// `READ.get(low, [])`
    fn rs(&self, low: &str) -> &'a [u32] { self.nat.read.get(low).map_or(&[], |v| v.as_slice()) }
    fn in_read(&self, f: &str) -> bool { self.nat.read.contains_key(f) }
    fn lem(&self, r: &Reading) -> &'a str { &self.nat.lemmas[r.lem as usize] }
    /// `_forms(lem)` = `BYLEM.get(lem, [])`
    fn forms(&self, lem: &str) -> &'a [u32] {
        self.lem_id.get(lem).and_then(|i| self.nat.bylem.get(i)).map_or(&[], |v| v.as_slice())
    }
    /// `_lforms(lem)`: BYLEM forms then NOUNS forms (a noun reading is never in BYLEM, which holds V/A only)
    fn lforms(&self, lem: &str) -> Vec<u32> {
        let mut v = self.forms(lem).to_vec();
        if let Some(n) = self.lem_id.get(lem).and_then(|i| self.data.nouns.get(i)) { v.extend_from_slice(n); }
        v
    }
    /// `_match(r, forms)`: forms with the same pos/gender/number/person/mood as reading r
    fn matching(&self, r: &Reading, forms: &[u32]) -> Vec<&'a str> {
        forms.iter().map(|&i| self.r(i))
            .filter(|f| f.pos == r.pos && f.g & r.g != 0 && f.n & r.n != 0 && (f.p & r.p != 0 || (f.p == 0 && r.p == 0)) && (f.mo == r.mo || f.mo & r.mo != 0))
            .map(|f| f.form.as_str()).collect()
    }
    /// `_match(r, _forms(other)) or ([other] if other in READ else [])`
    fn match_or_self(&self, r: &Reading, other: &str) -> Vec<String> {
        let m = self.matching(r, self.forms(other));
        if !m.is_empty() { m.into_iter().map(String::from).collect() }
        else if self.in_read(other) { vec![other.to_string()] } else { Vec::new() }
    }
    fn vforms(&self, lem: &str, keep: impl Fn(&Reading) -> bool) -> Vec<&'a str> {
        self.forms(lem).iter().map(|&i| self.r(i)).filter(|f| keep(f)).map(|f| f.form.as_str()).collect()
    }

    /// `near_known(low)` (GEN3f): lexicon forms at Damerau-Levenshtein distance 1 (or 2 with the first 4 letters
    /// shared), first 6 by (distance, form)
    pub fn near_known(&self, low: &str) -> Vec<String> {
        const MAXD: usize = 2;
        let a: Vec<char> = low.chars().collect();
        let dl = |b: &[char]| -> usize {
            if a.len().abs_diff(b.len()) > MAXD { return 9; }
            let (n, m) = (a.len(), b.len());
            let mut d = vec![vec![0usize; m + 1]; n + 1];
            for (i, row) in d.iter_mut().enumerate() { row[0] = i; }
            for (j, x) in d[0].iter_mut().enumerate() { *x = j; }
            for i in 1..=n {
                for j in 1..=m {
                    let v = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]));
                    d[i][j] = if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] { v.min(d[i - 2][j - 2] + 1) } else { v };
                }
            }
            d[n][m]
        };
        let p4 = cp_prefix(low, 4);
        let mut out: Vec<(usize, &str)> = Vec::new();
        for f in self.bypre.get(cp_prefix(low, 3)).map_or(&[][..], |v| v.as_slice()) {
            let b: Vec<char> = f.chars().collect();
            let k = dl(&b);
            if k == 1 || (k == 2 && cp_prefix(f, 4) == p4) { out.push((k, f)); }
        }
        out.sort_unstable();
        out.into_iter().take(6).map(|(_, f)| f.to_string()).collect()
    }

    /// `regularised(low)` (GEN3f): forms of the verb a regularised conjugation points to (disez -> dites)
    pub fn regularised(&self, low: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let n = cp_len(low);
        for (suf, mood, p, num) in ENDINGS {
            if !low.ends_with(suf) || n - suf.len() < 2 { continue; } // suffixes are ASCII: bytes = code points
            let stem = &low[..low.len() - suf.len()];
            let sl = cp_len(stem);
            let start = cp_prefix(stem, 3.max(sl.saturating_sub(1)));
            let mut lems: Vec<(u32, usize)> = Vec::new(); // dict in insertion order: lemma -> longest common prefix
            for f in self.bypre.get(cp_prefix(stem, 3)).map_or(&[][..], |v| v.as_slice()) {
                if !f.starts_with(start) { continue; }
                let cp = f.chars().zip(stem.chars()).take_while(|(x, y)| x == y).count();
                for &i in self.rs(f) {
                    let r = self.r(i);
                    if r.pos != 'V' { continue; }
                    match lems.iter_mut().find(|e| e.0 == r.lem) { Some(e) => e.1 = e.1.max(cp), None => lems.push((r.lem, cp)) }
                }
            }
            lems.sort_by_key(|e| std::cmp::Reverse(e.1)); // stable, like sorted(key=-v)
            for (lem, _) in lems.into_iter().take(3) {
                let ln = &self.nat.lemmas[lem as usize];
                out.extend(self.vforms(ln, |f| f.pos == 'V' && f.mo & mood != 0 && f.p & p != 0 && f.n & num != 0).into_iter().map(String::from));
            }
            if !out.is_empty() { break; }
        }
        out.truncate(6);
        out
    }

    /// `paradigm_low2(low)` with GEN3 on: every competing form of a lower-cased word, ranked.
    pub fn paradigm_low2(&self, low: &str) -> Vec<String> {
        let nat = self.nat;
        let mut pri = Pri { low, nat, map: HashMap::new() };
        for f in self.data.paradigm_low(nat, low) { pri.add1(&f, 20); }
        if let Some(s) = self.data.homo_of.get(low) { pri.add(s.iter(), 0); }
        if let Some(s) = self.homo2_of.get(low) { pri.add(s.iter(), 0); }
        let rs = self.rs(low);
        for &ri in rs {
            let r = self.r(ri);
            if r.pos != 'V' { continue; }
            let (lem, n, p, mo) = (self.lem(r), r.n, r.p, r.mo);
            let forms: Vec<&Reading> = self.forms(lem).iter().map(|&i| self.r(i)).collect();
            let tm = mo & FIN;
            if tm != 0 {
                let near = [IP, SP, IF, K, IQ, IS, E, SQ].iter().filter(|m| tm & **m != 0).fold(0u16, |a, m| a | near3(*m));
                for f in &forms {
                    if f.pos != 'V' || f.form.contains('-') { continue; }
                    let fm = f.mo;
                    // same tense+person, other number; or same person+number, near mood (GEN3c: rank 1 too)
                    if (fm & tm != 0 && f.p & p != 0 && f.n & n == 0) || (fm & near != 0 && f.p & p != 0 && f.n & n != 0) { pri.add1(&f.form, 1); }
                    else if fm & near != 0 && f.p & p != 0 { pri.add1(&f.form, 3); }
                    else if fm & tm != 0 && f.p != 0 && f.p & p == 0 { pri.add1(&f.form, 4); }
                }
                pri.add(forms.iter().filter(|f| f.pos == 'V' && f.mo & Q != 0 && f.g & 1 != 0 && f.n & 1 != 0).map(|f| f.form.as_str()), 6);
            }
            if lem.ends_with("er") && (low.ends_with("ez") || low.ends_with("ais") || low.ends_with("ait") || low.ends_with("aient")) { pri.add1(lem, 3); }
            let q_forms = || forms.iter().filter(|f| f.pos == 'V' && f.mo & Q != 0).map(|f| f.form.as_str());
            let p2_forms = || forms.iter().filter(|f| f.pos == 'V' && f.mo & IP != 0 && f.p & 2 != 0 && f.n & 2 != 0).map(|f| f.form.as_str());
            if mo & Q != 0 {
                pri.add(q_forms(), 1);
                pri.add1(lem, 2);
                pri.add(p2_forms(), 3);
            }
            if mo & Y != 0 {
                pri.add(q_forms(), 1);
                pri.add(p2_forms(), 2);
                if lem.ends_with("er") { pri.add(forms.iter().filter(|f| f.pos == 'V' && f.mo & IQ != 0 && !(f.p & 2 != 0 && f.n & 2 != 0)).map(|f| f.form.as_str()), 3); }
            }
        }
        for cut in ["es", "e", "s"] {                                             // (impossible) agreement of a participle
            if let Some(stem) = low.strip_suffix(cut) {
                if !stem.is_empty() && self.rs(stem).iter().any(|&i| { let r = self.r(i); r.pos == 'V' && r.mo & Q != 0 }) {
                    pri.add1(stem, if rs.is_empty() { 1 } else { 3 });
                }
            }
        }
        if let Some(s) = low.strip_suffix("ous") { let f = format!("{s}oux"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
        if let Some(s) = low.strip_suffix("oux") { let f = format!("{s}ous"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
        for (a, b) in [("aux", "al"), ("aux", "ail"), ("ails", "ail"), ("als", "al"), ("ails", "aux"), ("als", "aux")] {
            if let Some(s) = low.strip_suffix(a) {
                let sing = format!("{s}{b}");
                for &i in self.rs(&sing) {
                    let r = self.r(i);
                    if r.pos == 'N' {
                        let v: Vec<&str> = self.data.nouns.get(&r.lem).map_or(&[][..], |v| v.as_slice()).iter().map(|&j| self.r(j)).filter(|f| f.n & 2 != 0).map(|f| f.form.as_str()).collect();
                        pri.add(v, 2);
                    }
                }
                if b == "aux" && !self.rs(&sing).is_empty() { pri.add1(&sing, 2); }
            }
        }
        if let Some((head, tail)) = low.split_once('-') {
            if let Some(p) = prefix_of(head) { pri.add1(&format!("{p}-{tail}"), 0); }
            if head == "ci" {
                for &i in self.rs(tail) {
                    let r = self.r(i);
                    if r.pos == 'V' && r.mo & Q != 0 {
                        let v: Vec<String> = self.vforms(self.lem(r), |f| f.pos == 'V' && f.mo & Q != 0).into_iter().map(|f| format!("ci-{f}")).collect();
                        pri.add(v, 1);
                    }
                }
            }
        }
        if PREPS.contains(&low) || ["au", "aux", "du", "des", "d’"].contains(&low) {
            for (k, p) in PREPS3.iter().enumerate() { pri.add1(p, 1 + (k / 6) as u8); }
        }
        if ARTS.contains(&low) { pri.add(ARTS, 1); }
        if CLITICS.contains(&low) { pri.add(CLITICS, 1); }
        if L_GROUP.contains(&low) { pri.add(L_GROUP, 1); }
        for g in [&["tout", "tous", "toute", "toutes"][..], &["un", "une", "des"], &["celui", "celle", "ceux", "celles"]] {
            if g.contains(&low) { pri.add(g, 0); }
        }
        let quel = QUEL_LEQUEL.iter().find(|(a, _)| *a == low);
        if RELS.contains(&low) || quel.is_some() { pri.add(RELS, 5); }
        if let Some((_, l)) = quel { pri.add1(l, 1); }
        for &ri in rs {
            let r = self.r(ri);
            for other in self.par_of.get(self.lem(r)).into_iter().flatten() { pri.add(self.match_or_self(r, other), 0); }
        }
        for other in self.par_of.get(low).into_iter().flatten() { pri.add1(other, 0); }
        if let Some(h) = self.res.lexhom.get(low) { pri.add(h, 6); }
        for &ri in rs {                                    // homophones of the LEMMA (open class), typed word's gender/number
            let r = self.r(ri);
            if r.pos != 'N' && r.pos != 'A' { continue; }
            for h in self.res.lexhom.get(self.lem(r)).into_iter().flatten() {
                for &hi in self.rs(h) {
                    let rh = self.r(hi);
                    let lf = self.lforms(self.lem(rh));
                    let m = self.matching(r, &lf);
                    if !m.is_empty() { pri.add(m, 6); }
                    else { pri.add(lf.iter().map(|&j| self.r(j)).filter(|f| f.pos == rh.pos && f.n & r.n != 0).map(|f| f.form.as_str()), 6); }
                }
            }
        }
        // ---- GEN3
        for &ri in rs {                                    // Claude-written confusion pairs (lemma level)
            let r = self.r(ri);
            for other in self.res.cw_pairs.get(self.lem(r)).into_iter().flatten() { pri.add(self.match_or_self(r, other), 0); }
        }
        for other in self.res.cw_pairs.get(low).into_iter().flatten() { if self.in_read(other) { pri.add1(other, 0); } }
        let v_with = |ids: &[u32], m: u16| ids.iter().any(|&i| { let r = self.r(i); r.pos == 'V' && r.mo & m != 0 });
        if v_with(rs, E) && low.ends_with('e') { let f = format!("{low}s"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
        if let Some((_, b)) = CLITIC_PAIRS.iter().find(|(a, _)| *a == low) { pri.add1(b, 0); }
        if let Some((_, b)) = SPECIAL.iter().find(|(a, _)| *a == low) { pri.add1(b, 0); }
        if rs.is_empty() { if let Some(s) = low.strip_suffix('s') { if !self.rs(s).is_empty() { pri.add1(s, 2); } } }
        if CLITIC_PAIRS.iter().any(|(a, _)| *a == low) || IMP_CLITICS.contains(&low) { pri.add(IMP_CLITICS.iter().filter(|c| **c != low), 3); }
        if let Some(e) = elision_of(low) { pri.add(e, 1); }
        for &ri in rs {
            let r = self.r(ri);
            if r.pos == 'A' { pri.add(self.vforms(self.lem(r), |f| f.pos == 'A'), 1); }
        }
        for (a, b) in REG_PLURAL {
            if let Some(s) = low.strip_suffix(a) {
                let f = format!("{s}{b}");
                if !self.rs(&f).is_empty() { pri.add1(&f, if rs.is_empty() { 1 } else { 2 }); }
            }
        }
        if rs.iter().any(|&i| self.r(i).pos == 'N') {           // noun gender pairs (prince/princesse)
            let n = cp_len(low);
            for (a, b) in GENDER_PAIRS {
                for (x, y) in [(a, b), (b, a)] {
                    if low.ends_with(x) && n >= cp_len(x) + 2 {
                        let f = if x.is_empty() { format!("{low}{y}") } else { format!("{}{y}", &low[..low.len() - x.len()]) };
                        if f != low && self.rs(&f).iter().any(|&i| self.r(i).pos == 'N') { pri.add1(&f, 2); }
                    }
                }
            }
        }
        if low.contains('-') {                                  // arc-en-ciels -> arcs-en-ciel
            let p: Vec<&str> = low.split('-').collect();
            let (first, last) = (p[0], p[p.len() - 1]);
            if last.ends_with('s') && !first.ends_with('s') {
                let mut parts: Vec<String> = vec![format!("{first}s")];
                parts.extend(p[1..p.len() - 1].iter().map(|x| x.to_string()));
                parts.push(cp_drop_last(last, 1).to_string());
                let f = parts.join("-");
                if !self.rs(&f).is_empty() || !self.rs(&format!("{first}s")).is_empty() { pri.add1(&f, 1); }
            }
        }
        for &ri in rs {                                         // auxiliary choice (a/est, ont/sont)
            let r = self.r(ri);
            let lem = self.lem(r);
            if r.pos == 'V' && (lem == "avoir" || lem == "être") {
                let other = if lem == "avoir" { "être" } else { "avoir" };
                pri.add(self.vforms(other, |f| f.pos == 'V' && f.mo & r.mo != 0 && f.p & r.p != 0 && f.n & r.n != 0), 1);
            }
        }
        if let Some(s) = low.strip_suffix("es") {
            let s1 = format!("{s}e");
            if v_with(self.rs(&s1), E) { pri.add1(&s1, 1); }
        }
        if let Some((_, v)) = IRREG.iter().find(|(a, _)| *a == low) { pri.add(*v, 0); }
        if rs.is_empty() {
            if let Some(s) = low.strip_suffix("aux") { let f = format!("{s}als"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
            let mut bases: Vec<String> = vec![cp_drop_last(low, 1).to_string(), cp_drop_last(low, 2).to_string(), cp_drop_last(low, 1).replace('è', "e")];
            bases.sort(); bases.dedup();                        // a set in Python; add() is order-independent
            for base in &bases {
                for &i in self.rs(base) {
                    let r = self.r(i);
                    if r.pos == 'A' || r.pos == 'N' {
                        let lf = self.lforms(self.lem(r));
                        pri.add(lf.iter().map(|&j| self.r(j)).filter(|f| f.pos == r.pos).map(|f| f.form.as_str()), 1);
                    }
                }
            }
            pri.add(self.regularised(low), 1);
            pri.add(self.near_known(low), 3);
            for (a, b) in FEM_EURE {
                if let Some(s) = low.strip_suffix(a) { let f = format!("{s}{b}"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
            }
            if let Some(s) = low.strip_suffix("eant") {
                let adj = format!("{s}ent");
                let mut v: Vec<&str> = Vec::new();
                for &i in self.rs(&adj) { let r = self.r(i); if r.pos == 'A' { v.extend(self.vforms(self.lem(r), |f| f.pos == 'A')); } }
                if self.rs(&adj).iter().any(|&i| self.r(i).pos == 'A') { pri.add(v, 1); }
            }
            for cut in ["antes", "ante", "ants"] {
                if let Some(s) = low.strip_suffix(cut) { let f = format!("{s}ant"); if !self.rs(&f).is_empty() { pri.add1(&f, 1); } }
            }
        }
        if v_with(rs, P) {                                      // participle -> adjective forms
            for (a, b) in P_TO_A {
                if let Some(s) = low.strip_suffix(a) {
                    let adj = format!("{s}{b}");
                    for &i in self.rs(&adj) { let r = self.r(i); if r.pos == 'A' { pri.add(self.vforms(self.lem(r), |f| f.pos == 'A'), 2); } }
                }
            }
        }
        if low.contains('-') {                                  // compounds: number on each part
            let parts: Vec<&str> = low.split('-').collect();
            for (j, pt) in parts.iter().enumerate() {
                let mut alts: Vec<String> = Vec::new();
                if pt.ends_with('s') || pt.ends_with('x') { alts.push(cp_drop_last(pt, 1).to_string()); }
                alts.push(format!("{pt}s"));
                if pt.ends_with("al") { alts.push(format!("{}aux", cp_drop_last(pt, 2))); }
                if pt.ends_with("aux") { alts.push(format!("{}al", cp_drop_last(pt, 3))); }
                if pt.ends_with("eau") || pt.ends_with("au") || pt.ends_with("eu") { alts.push(format!("{pt}x")); }
                for alt in alts {
                    let mut ps: Vec<&str> = parts.clone();
                    ps[j] = &alt;
                    let f = ps.join("-");
                    let others_ok = parts.iter().enumerate().filter(|(q, _)| *q != j)
                        .all(|(_, q)| !self.rs(q).is_empty() || ["à", "au", "de", "d’", "en"].contains(q));
                    if !self.rs(&f).is_empty() || (!self.rs(&alt).is_empty() && others_ok) { pri.add1(&f, 2); }
                }
            }
            if let Some(p) = prefix_of(parts[0]) {
                let mut ps: Vec<&str> = parts.clone(); ps[0] = p; pri.add1(&ps.join("-"), 0);
            }
            if parts.len() == 2 && (parts[1] == "même" || parts[1] == "mêmes") {
                pri.add1(&format!("{}-{}", parts[0], if parts[1] == "même" { "mêmes" } else { "même" }), 1);
            }
            if let Some(rest) = low.strip_prefix("quatre-vingts-") { pri.add1(&format!("quatre-vingt-{rest}"), 0); }
            if let Some(rest) = low.strip_prefix("quelque-") { pri.add1(&format!("quelques-{rest}"), 1); }
        }
        if let Some(rest) = low.strip_prefix("quatre-vingtsi") { pri.add1(&format!("quatre-vingti{rest}"), 0); }
        if let Some(m) = self.res.mined.get(low) { pri.add(m, 8); }
        // plausible() and the GEN3 ranking: (rank, how many forms of the same lemma already at that rank, form)
        let has_dash = low.contains('-');
        let mut items: Vec<(u8, String)> = pri.map.into_iter().map(|(f, k)| (k, f)).collect();
        items.sort_unstable();
        let mut seen: HashMap<(u8, &str), u32> = HashMap::new();
        let mut keyed: Vec<(u8, u32, &String)> = Vec::with_capacity(items.len());
        for (k, f) in &items {
            if f == low || f.contains(' ') || (cp_len(f) < 2 && !self.in_read(f)) { continue; }
            if f.contains('-') && !has_dash { continue; }
            let lm: &str = self.rs(f).first().map_or(f.as_str(), |&i| self.lem(self.r(i)));
            let c = seen.entry((*k, lm)).or_insert(0);
            keyed.push((*k, *c, f));
            *c += 1;
        }
        keyed.sort_unstable();
        keyed.into_iter().map(|(_, _, f)| f.clone()).collect()
    }

    /// ebm_data.py `readings(form, toks, i)`: the sorted lexicon readings, else (capitalised form) the guessed
    /// reading `CH.sentence` gives it at position i, else none.
    pub fn readings(&self, form: &str, s: &Sent, i: usize) -> Vec<String> {
        let ks = self.lex.known(form);
        if !ks.is_empty() { return ks.iter().map(|k| self.lex.keys[*k as usize].clone()).collect(); }
        if !first_is_upper(form) { return Vec::new(); }
        self.lex.guess(form, i, &s.base[..i]).r.iter().map(|k| self.lex.keys[*k as usize].clone()).collect()
    }

    /// `_reads3(f, ws, i, written)`
    pub fn reads3(&self, f: &str, s: &Sent, i: usize, written: bool) -> Vec<String> {
        let fl = lower(f);
        let mut rd = self.readings(f, s, i);
        const PFX: [&str; 4] = ["demi", "semi", "mi", "nu"];
        if rd.is_empty() && !written && f.contains('-') && !PFX.contains(&fl.split('-').next().unwrap_or("")) {
            for hd in f.split('-') {                          // the first NOUN part is the head (sans-domicile -> domicile)
                let rh: Vec<String> = self.readings(hd, s, i).into_iter().filter(|r| r.starts_with("pos=N")).collect();
                if !rh.is_empty() { rd = rh.into_iter().map(|r| r + "|cmp=1").collect(); break; }
            }
        }
        if rd.is_empty() && !written {
            if let Some((head, tail)) = fl.split_once('-') {
                if PFX.contains(&head) { rd = self.readings(tail, s, i).into_iter().map(|r| format!("{r}|pfx={head}")).collect(); }
            }
        }
        if rd.is_empty() {
            rd = if SPECIAL_READ.contains(&fl.as_str()) { vec![SPECIAL_READING.to_string()] }
                 else if written { vec!["pos=UNK".to_string()] } else { Vec::new() };
        }
        if IMP_CLITICS.contains(&fl.as_str()) {
            let cl = fl.trim_matches('-');
            rd = rd.into_iter().map(|r| format!("{r}|cl={cl}")).collect();
        }
        rd
    }

    /// `ctx_alts(ws, i)` (GEN3d): near- or lexical homophones that the training books show far more often than the
    /// typed word next to these neighbours; at most 2, by (word-triple count, pair count) descending.
    /// Python sorts a set of strings with a stable sort, so equal counts come out in hash order (it depends on
    /// PYTHONHASHSEED); here equal counts are ordered by the form, which is the only deterministic choice.
    pub fn ctx_alts(&self, s: &Sent, i: usize) -> Vec<String> {
        let w = s.low[i].as_str();
        let mut alts: BTreeSet<&str> = BTreeSet::new();
        for a in self.res.nearhom.get(w).into_iter().flatten().chain(self.res.lexhom.get(w).into_iter().flatten()) { alts.insert(a); }
        if alts.is_empty() { return Vec::new(); }
        let t = |j: isize| -> &str {                          // t = ["<s>"] + lower words + ["</s>"]
            if j <= 0 { "<s>" } else if j as usize > s.low.len() { "</s>" } else { &s.low[j as usize - 1] }
        };
        let j = i as isize + 1;
        let p2 = if j >= 2 { t(j - 2) } else { "<s>" };
        let (p1, n1) = (t(j - 1), t(j + 1));
        let n2 = if (j + 2) as usize <= s.low.len() + 1 { t(j + 2) } else { "</s>" };
        let big = |k: String| *self.res.ctx_big.get(&k).unwrap_or(&0);
        let s2 = |x: &str| big(format!("{p1} {x}")) + big(format!("{x} {n1}"));
        let s3 = |x: &str| big(format!("{p2} {p1} {x}")) + big(format!("{x} {n1} {n2}"));
        let (w2, w3) = (s2(w), s3(w));
        let mut ok: Vec<(i64, i64, &str)> = alts.into_iter().map(|a| (s3(a), s2(a), a))
            .filter(|&(a3, a2, _)| (a3 >= 5 && a3 >= 5 * (w3 + 1)) || (a2 >= 20 && a2 >= 5 * (w2 + 1))).collect();
        ok.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)).then(x.2.cmp(y.2)));
        ok.into_iter().take(2).map(|x| x.2.to_string()).collect()
    }

    /// `cand_list(ws, i)`: the candidate set of word i as at inference (no gold), or None.
    pub fn cand_list(&self, s: &Sent, i: usize) -> Option<CandSet> { self.cand_list_dr(s, i, &mut Drops::new()) }

    /// `cand_list(ws, i)` with its drop-report counters.
    pub fn cand_list_dr(&self, s: &Sent, i: usize, dr: &mut Drops) -> Option<CandSet> {
        let ws = s.ws;
        let w = ws[i].as_str();
        let mid = midsent(ws, i);
        if mid && first_is_upper(w) {                         // mid-sentence capital: only the {Word, word} question
            let low = &s.low[i];
            let rs = self.rs(low);
            if cp_len(w) > 1 && str_isupper(w) { dr.count_ex("none_allcaps", || w.to_string()); return None; }
            if rs.is_empty() { dr.count_ex("none_cap_lower_unknown", || w.to_string()); return None; }
            if rs.iter().all(|&j| self.r(j).pos == 'M') { dr.count_ex("none_propn", || w.to_string()); return None; }
            let rl = self.readings(low, s, i);
            if rl.is_empty() { dr.count_ex("none_cap_lower_unread", || w.to_string()); return None; }
            let rw = self.readings(w, s, i);
            let rw = if rw.is_empty() { rl.clone() } else { rw };
            return Some((i, 0, vec![(w.to_string(), rw.into_iter().map(|r| r + "|cap=1").collect()), (low.clone(), rl)]));
        }
        let alts = self.paradigm_low2(&s.low[i]);
        let lower_first = first_is_lower(w);
        let capq = mid && lower_first && (self.res.capstats.get(w).is_some_and(|c| c.0 >= 10)
            || (nation(w) && { let ps: Vec<char> = self.rs(w).iter().map(|&j| self.r(j).pos).collect(); ps.contains(&'N') && ps.contains(&'A') }));
        if alts.is_empty() && !capq { dr.count("none_no_alts"); return None; }
        let wcap = first_is_upper(w);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut cs: Vec<Cand> = Vec::new();
        let forms = std::iter::once(w.to_string()).chain(alts.iter().map(|a| if wcap { cap_first(a) } else { a.clone() }));
        for (k, f) in forms.enumerate() {
            let fl = lower(&f);
            if seen.contains(&fl) { continue; }
            seen.insert(fl);
            let rd = self.reads3(&f, s, i, k == 0);
            if !rd.is_empty() { cs.push((f, rd)); } else if k == 0 { dr.count_ex("none_written_unread", || w.to_string()); }
        }
        if capq {                                             // books capitalise it, or a people name
            let cap = cap_first(w);
            let rc = self.readings(&cap, s, i);
            if !rc.is_empty() { cs.insert(cs.len().min(1), (cap, rc.into_iter().map(|r| r + "|cap=1").collect())); }
        }
        if !cs.is_empty() && cs[0].0 == w {                   // context-promoted near-homophones, right after the written form
            let have: BTreeSet<String> = cs.iter().map(|x| lower(&x.0)).collect();
            for a in self.ctx_alts(s, i) {
                if have.contains(&a) {
                    if let Some(k) = cs.iter().position(|x| lower(&x.0) == a) {
                        let (f, rd) = cs.remove(k);
                        cs.insert(cs.len().min(1), (f, rd.into_iter().map(|r| r + "|ctx=1").collect()));  // list.insert(1, x)
                    }
                } else {
                    let f = if wcap { cap_first(&a) } else { a.clone() };
                    let rd = self.readings(&f, s, i);
                    if !rd.is_empty() {
                        dr.count_ex("ctx_promoted", || format!("{w} -> {f}"));
                        cs.insert(cs.len().min(1), (f, rd.into_iter().map(|r| r + "|ctx=1").collect()));
                    }
                }
            }
        }
        let lw = s.low[i].as_str();                           // GEN4: grammar constraints on relative pronouns
        if LEQUELS.contains(&lw) {
            cs.retain(|x| { let l = lower(&x.0); l != "que" && l != "qu’" && l != "dont" });
        } else if (lw == "que" || lw == "qu’") && i > 0 && PREP_LEQ.contains(&s.low[i - 1].as_str()) {
            let have: BTreeSet<String> = cs.iter().map(|x| lower(&x.0)).collect();
            for f in ["lequel", "laquelle", "lesquels", "lesquelles"] {
                if !have.contains(f) {
                    let rd = self.readings(f, s, i);
                    if !rd.is_empty() { cs.push((f.to_string(), rd)); }
                }
            }
        }
        if cs.len() > MAXC { dr.count("alts_truncated"); cs.truncate(MAXC); }
        if cs.is_empty() || cs[0].0 != w { return None; }
        if cs.len() < 2 { dr.count_ex("none_alts_unread", || w.to_string()); return None; }
        for (f, rd) in &cs {                                  // invariant (audit): readings within MAXF atoms
            for r in rd { if over_maxf(r) { dr.warn("reading_over_maxf", format!("{f}: {r}")); } }
        }
        Some((i, 0, cs))
    }
}

/// `r.count("|") + 1 > MAXF_DATA`
pub fn over_maxf(r: &str) -> bool { r.bytes().filter(|&b| b == b'|').count() + 1 > MAXF_DATA }

/// `r.split("|")[0][4:] in CLOSED` for a reading that starts with "pos="
pub fn closed_pos(r: &str) -> bool {
    let head = r.split('|').next().unwrap_or("");
    head.get(4..).is_some_and(|p| CLOSED.contains(&p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::natural::Natural;
    fn v(xs: &[&str]) -> Vec<String> { xs.iter().map(|x| x.to_string()).collect() }
    #[test]
    fn sentence_start_capitals() {
        let ws = v(&["Il", "dort", ".", "La", "porte", ":", "Non", "«", "Oui", "»", "Paul", "—", "Bon", "…", "Ah"]);
        assert!(!midsent(&ws, 0));
        assert!(midsent(&ws, 1));
        assert!(!midsent(&ws, 3));   // after "."
        assert!(!midsent(&ws, 6));   // after ":"
        assert!(!midsent(&ws, 8));   // after "«"
        assert!(midsent(&ws, 10));   // after "»": not a sentence start
        assert!(!midsent(&ws, 12));  // after "—"
        assert!(!midsent(&ws, 14));  // after "…"
    }
    #[test]
    fn nation_suffix() {
        for w in ["parisien", "parisiennes", "français", "anglaise", "chinois", "romain", "allemand", "espagnol", "turc", "grec", "marocain", "toulousaine"] {
            if w == "turc" { assert!(!nation(w)); } else { assert!(nation(w), "{w}"); }
        }
        assert!(nation("an") && nation("ans") && !nation("anx"));
    }
    /// a tiny lexicon (form, lemma, Grammalecte tag), lemma atoms on
    const TSV: &str = "la\tle\t:G:D:f:s\nla\tla\t:N:m:i\nlà\tlà\t:W\nle\tle\t:G:D:m:s\nles\tle\t:G:D:e:p\nporte\tporte\t:N:f:s\n\
porte\tporter\t:V1_it____a:Ip:1s:3s\nil\til\t:G:Os:m:s:3s\ndort\tdormir\t:V3_i_____a:Ip:3s\nvoit\tvoir\t:V3_it____a:Ip:3s\n\
homme\thomme\t:N:m:s\nque\tque\t:G:Cs\ndonne\tdonner\t:V1_it____a:Ip:3s\nmoi\tmoi\t:G:Oo:e:s:1s\na\tavoir\t:V0a:Ip:3s\n";
    fn tiny() -> (Lexicon, Natural) { (Lexicon::from_tsv_with(TSV, true), Natural::from_tsv(TSV)) }
    #[test]
    fn elision_and_clitic_tokens() {
        let (lex, _) = tiny();
        let t = crate::text::characterize::tokenize(&lex, "L'homme qu'il donne-le-moi, a-t-il dit.");
        assert_eq!(t, v(&["L’", "homme", "qu’", "il", "donne", "-le", "-moi", ",", "a", "-il", "dit", "."]));
    }
    #[test]
    fn capital_question_only_mid_sentence() {
        let (lex, nat) = tiny();
        let res = Resources::default();
        let g = Gen::new(&lex, &nat, &res);
        // mid-sentence capital: only {La|cap=1, la}
        let ws = v(&["Il", "voit", "La", "porte"]);
        let s = Sent::new(&lex, &ws);
        let (i, gold, cs) = g.cand_list(&s, 2).expect("a set");
        assert_eq!((i, gold), (2, 0));
        assert_eq!(cs, vec![("La".to_string(), v(&["pos=DET|sub=def|g=f|n=s|cap=1", "pos=N|g=m|n=i|lem=la|cap=1"])),
                            ("la".to_string(), v(&["pos=DET|sub=def|g=f|n=s", "pos=N|g=m|n=i|lem=la"]))]);
        // after "." the capital starts a sentence: the ordinary paradigm, capitalised, no cap=1 atom
        let ws = v(&["Il", "dort", ".", "La", "porte"]);
        let s = Sent::new(&lex, &ws);
        let (_, _, cs) = g.cand_list(&s, 3).expect("a set");
        assert_eq!(cs[0].0, "La");
        assert!(cs.iter().any(|c| c.0 == "Le") && cs.iter().any(|c| c.0 == "Là"));
        assert!(cs.iter().all(|c| c.1.iter().all(|r| !r.contains("cap=1"))));
        // an all-caps word mid-sentence gets no question
        let ws = v(&["Il", "voit", "LA", "porte"]);
        assert!(g.cand_list(&Sent::new(&lex, &ws), 2).is_none());
    }
    #[test]
    fn python_slices() {
        assert_eq!(cp_drop_last("é", 2), "");
        assert_eq!(cp_drop_last("arcs", 1), "arc");
        assert_eq!(cp_prefix("été", 2), "ét");
        assert_eq!(cap_first("ßa"), "SSa");
        assert_eq!(cap_first("élan"), "Élan");
        assert!(closed_pos("pos=DET|sub=def") && !closed_pos("pos=N|g=m") && !closed_pos("pos"));
    }
}
