//! Port of training-design/characterize.py: `bundle`, `key`, `GOV`/`LW`, `Lexicon` (readings + suffix statistics),
//! `tokenize`, `Characterizer.known/sentence/guess`. Output equal to the Python reference by construction and by the
//! golden tests in `prep/tests/golden/` (see BENCHMARKS-dataprep.md).
//!
//! Reading keys are interned: every key string the characterizer can emit is known after the lexicon is loaded, and
//! ids are assigned in string order, so sorting ids == Python's `sorted()` of the strings.
use crate::text::pystr::{has_w, is_decimal, is_upper, lower, str_isalpha, str_isdigit};
use crate::text::fxhash::FxMap;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub type Key = u32;

const MOODS: [&str; 11] = ["Ip", "Iq", "Is", "If", "K", "Sp", "Sq", "E", "Y", "P", "Q"];

/// One reading (Python dict `f`); `None`/empty fields are dropped by `key`.
#[derive(Clone, Default, Debug)]
pub struct Bundle {
    pub pos: String,
    pub sub: Option<&'static str>,
    pub g: Option<String>,
    pub n: Option<String>,
    pub p: Option<String>,
    pub mood: Option<String>,
    pub grp: Option<String>,
    pub tr: Option<String>,
    pub aux: Option<&'static str>,
    pub gov: Option<String>,
    pub lw: Option<String>,
    /// round 3 (characterize.py FRC_LEMMA=1): lemma of open-class readings (N/A/V/ADV, lemma without "|")
    pub lem: Option<String>,
}

fn split_semi_hash(x: &str) -> &str { x.split(|c| c == ';' || c == '#').next().unwrap_or("") }

/// `bundle(tag)`: Grammalecte tag -> one reading.
pub fn bundle(tag: &str) -> Bundle {
    let parts: Vec<&str> = tag.split('/').next().unwrap().split(':').filter(|x| !x.is_empty()).map(split_semi_hash).collect();
    let mut f = Bundle::default();
    if parts.is_empty() { f.pos = "OTHER".into(); return f; }
    let head = parts[0];
    let rest = &parts[1..];
    let hc: Vec<char> = head.chars().collect();
    if hc.first() == Some(&'V') && hc.len() > 1 && "0123".contains(hc[1]) {
        f.pos = "V".into();
        f.grp = Some(hc[1].to_string());
        let flags: Vec<char> = if hc.len() > 3 { hc[3..].to_vec() } else { Vec::new() };
        f.tr = Some(flags.iter().take(6).filter(|c| crate::text::pystr::is_alpha(**c) && "itnpqrue".contains(**c)).collect());
        let second_last = if flags.len() >= 2 { Some(flags[flags.len() - 2]) } else { None };
        f.aux = Some(if second_last == Some('e') { "e" } else if flags.last() == Some(&'a') { "a" } else { "?" });
        if head == "V0e" || head == "V0a" || head.starts_with("V0") { f.sub = Some("aux"); }
    } else if head == "N" { f.pos = "N".into(); }
    else if head == "A" { f.pos = "A".into(); }
    else if head == "W" || head == "ÉW" { f.pos = "ADV".into(); }
    else if head.starts_with('M') { f.pos = "PROPN".into(); f.sub = Some(if head == "MP" { "place" } else { "person" }); }
    else if head.starts_with('B') { f.pos = "NUM".into(); }
    else if head == "J" { f.pos = "INTJ".into(); }
    else if head == "G" {
        let r = rest.first().copied().unwrap_or("");
        if r.starts_with('D') {
            f.pos = "DET".into();
            f.sub = Some(match r { "Dd" => "dem", "Dp" => "poss", "Di" => "indef", "Dn" => "neg", _ => "def" });
        } else if r.starts_with('R') { f.pos = "PREP".into(); }
        else if r.starts_with('O') {
            f.pos = "PRON".into();
            f.sub = Some(match r { "Os" => "subj", "Oo" => "obj", "Or" => "rel", "Ot" => "int", "Od" => "dem", "Oi" => "indef", "On" => "neg", _ => "pron" });
        } else if r == "Cc" { f.pos = "CONJ".into(); f.sub = Some("coord"); }
        else if r == "Cs" { f.pos = "CONJ".into(); f.sub = Some("sub"); }
        else if r == "X" { f.pos = "NEG".into(); }
        else if r == "W" { f.pos = "ADV".into(); }
        else { f.pos = "OTHER".into(); }
    } else { f.pos = "OTHER".into(); }
    let mut moods: Vec<&str> = Vec::new();
    for &x in rest {
        let xc: Vec<char> = x.chars().collect();
        if x == "m" || x == "f" || x == "e" { f.g = Some(x.into()); }
        else if x == "s" || x == "p" || x == "i" { f.n = Some(x.into()); }
        else if MOODS.contains(&x) { moods.push(x); }
        else if xc.len() >= 2 && "123".contains(xc[0]) && "sp".contains(xc[1]) {
            let p = f.p.get_or_insert_with(String::new);
            if !p.contains(xc[0]) { p.push(xc[0]); }
            let x1 = xc[1].to_string();
            f.n = Some(if f.n.is_none() || f.n.as_deref() == Some(x1.as_str()) { x1 } else { "i".into() });
        } else if x == "O1" || x == "O2" || x == "O3" { f.p = Some(xc[1].to_string()); }
    }
    if !moods.is_empty() { f.mood = Some(moods.join("+")); }
    f
}

/// `key(f)`
pub fn key(f: &Bundle) -> String {
    let mut o = String::with_capacity(48);
    let mut add = |k: &str, v: &str| {
        if v.is_empty() { return; }
        if !o.is_empty() { o.push('|'); }
        o.push_str(k); o.push('='); o.push_str(v);
    };
    add("pos", &f.pos);
    add("sub", f.sub.unwrap_or(""));
    add("g", f.g.as_deref().unwrap_or(""));
    add("n", f.n.as_deref().unwrap_or(""));
    add("p", f.p.as_deref().unwrap_or(""));
    add("mood", f.mood.as_deref().unwrap_or(""));
    add("grp", f.grp.as_deref().unwrap_or(""));
    add("tr", f.tr.as_deref().unwrap_or(""));
    add("aux", f.aux.unwrap_or(""));
    add("gov", f.gov.as_deref().unwrap_or(""));
    add("lw", f.lw.as_deref().unwrap_or(""));
    add("lem", f.lem.as_deref().unwrap_or(""));
    o
}

const GOV_SRC: [(&str, &str); 6] = [
    ("cod", "savoir aider pallier rappeler regarder attendre écouter chercher payer prier remercier féliciter aimer voir connaître lire écrire prendre mettre acheter vendre porter oublier perdre trouver garder préparer quitter rencontrer suivre"),
    ("a", "penser téléphoner plaire nuire ressembler succéder sourire mentir obéir renoncer participer assister songer réfléchir tenir appartenir convenir correspondre consentir remédier recourir s'intéresser habituer"),
    ("de", "souvenir douter servir occuper méfier profiter dépendre moquer rêver passer approcher emparer réjouir plaindre soucier contenter nourrir charger munir vanter repentir"),
    ("a+de", "parler jouer manquer"),
    ("a+cod", "permettre demander dire donner offrir promettre envoyer montrer apprendre enseigner conseiller interdire défendre proposer raconter expliquer reprocher prêter rendre confier"),
    ("prop", "savoir croire penser dire ignorer sentir deviner comprendre affirmer annoncer prétendre supposer"),
];
const LW_SRC: &str = "falloir faire laisser pouvoir vouloir devoir coûter durer peser valoir vivre mesurer courir sembler paraître devenir rester demeurer trouver considérer rendre aller venir gens plupart combien reste tout même tel quel quelque demi cent vingt nombre majorité moitié foule partie groupe ensemble";

/// `GOV`: verb lemma -> "+"-joined sorted government set.
pub fn gov_table() -> HashMap<String, String> {
    let mut sets: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (g, vs) in GOV_SRC {
        for v in vs.split_whitespace() {
            let s = sets.entry(v.to_string()).or_default();
            for x in g.split('+') { s.insert(x.to_string()); }
        }
    }
    sets.into_iter().map(|(k, s)| (k, s.into_iter().collect::<Vec<_>>().join("+"))).collect()
}

pub const ELIDED: [&str; 14] = ["l’", "d’", "j’", "m’", "t’", "s’", "n’", "c’", "qu’", "jusqu’", "lorsqu’", "puisqu’", "quoiqu’", "presqu’"];
const ELIDED_MAP: [(&str, &[&str]); 14] = [
    ("l’", &["le", "la"]), ("d’", &["de"]), ("j’", &["je"]), ("m’", &["me"]), ("t’", &["te"]), ("s’", &["se", "si"]),
    ("n’", &["ne"]), ("c’", &["ce"]), ("qu’", &["que"]), ("jusqu’", &["jusque"]), ("lorsqu’", &["lorsque"]),
    ("puisqu’", &["puisque"]), ("quoiqu’", &["quoique"]), ("presqu’", &["presque"]),
];
const INV_ALTS: [&str; 24] = ["je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles", "le", "la", "les", "moi", "toi",
                              "lui", "leur", "leurs", "en", "y", "ce", "ci", "là", "me", "te"];

/// Per-key facts the characterizer tests (all derived from the key string, as the Python does with `startswith`).
#[derive(Clone, Default, Debug)]
pub struct KeyInfo {
    pub det: bool,        // startswith("pos=DET")
    pub pron: bool,       // startswith("pos=PRON")
    pub propn: bool,      // startswith("pos=PROPN")
    pub has_pron: bool,   // "pos=PRON" in k
    pub n_or_a: bool,     // startswith(("pos=N", "pos=A"))
    pub det_n: Vec<String>, // values of the "n=" atoms
    /// for open-class keys (pos N/A/V/ADV): the key without its gov=/lw= atoms
    pub open_stripped: Option<Key>,
}

pub struct Lexicon {
    /// id -> key string (ids are in string order)
    pub keys: Vec<String>,
    /// id -> key as a JSON string literal (quotes + escapes), for the writers
    pub keys_json: Vec<String>,
    pub info: Vec<KeyInfo>,
    /// lower-cased form -> sorted, unique key ids (Python `self.read`)
    pub read: FxMap<String, Box<[Key]>>,
    /// suffix -> reading keys, `Counter.most_common(6)` order, filtered at 8% (Python `self.suffix`)
    pub suffix: FxMap<String, Vec<Key>>,
    pub k_punct: Key,
    pub k_num: Key,
    pub k_deictic: Key,
    pub k_propn: Key,
    pub k_n: Key,
    pub k_a: Key,
    /// "pos=PROPN|sub=person|n=X" for every n value a determiner key carries
    pub k_propn_n: FxMap<String, Key>,
}

fn last_chars(s: &str, l: usize) -> &str {
    match s.char_indices().rev().nth(l - 1) { Some((b, _)) => &s[b..], None => s }
}

impl Lexicon {
    /// `Lexicon()` on `lexicon.tsv` (form, lemma, tag).
    pub fn load(path: &Path) -> std::io::Result<Lexicon> {
        let text = crate::text::pystr::universal_newlines(std::fs::read_to_string(path)?);
        Ok(Self::from_tsv(&text))
    }

    /// `Lexicon()`; the lemma atom follows the FRC_LEMMA=1 environment variable, as in characterize.py
    pub fn from_tsv(text: &str) -> Lexicon {
        Self::from_tsv_with(text, std::env::var("FRC_LEMMA").map_or(false, |v| v == "1"))
    }

    /// `Lexicon()` with the lemma atom on or off explicitly (round-3 models are trained with it on)
    pub fn from_tsv_with(text: &str, lemma: bool) -> Lexicon {
        let prof = std::env::var_os("FRC_PREP_PROFILE").is_some();
        let t0 = std::time::Instant::now();
        let ph = |name: &str| if prof { eprintln!("  lexicon {name}: {:.3}s", t0.elapsed().as_secs_f64()); };
        let gov: FxMap<String, String> = gov_table().into_iter().collect();
        let lw: crate::text::fxhash::FxSet<&str> = LW_SRC.split_whitespace().collect();
        // phase 1: key strings per form, in first-insertion order of the lower-cased form
        let mut tmp: FxMap<String, u32> = FxMap::default();
        let mut tmp_keys: Vec<String> = Vec::new();
        let mut intern = |s: String, tmp_keys: &mut Vec<String>| -> u32 {
            if let Some(&k) = tmp.get(&s) { return k; }
            let k = tmp_keys.len() as u32;
            tmp.insert(s.clone(), k);
            tmp_keys.push(s);
            k
        };
        let mut forms: Vec<(String, Vec<u32>)> = Vec::new();
        let mut index: FxMap<String, usize> = FxMap::default();
        // tag -> (bundle, its keys when the lemma adds no gov=/lw=): most lines hit this cache
        let mut tcache: FxMap<&str, (Bundle, u32, Option<u32>)> = FxMap::default();
        let lines: Vec<&str> = text.split('\n').collect();
        let n = if text.ends_with('\n') { lines.len() - 1 } else { lines.len() };
        // parallel pre-pass (pure per line): split the fields, lower-case form and lemma, look the lemma up in GOV/LW
        let rows: Vec<(String, &str, Option<&String>, Option<String>, Option<String>)> = lines[..n].par_iter().with_min_len(4096).map(|line| {
            let mut it = line.split('\t');
            let (form, lem, tag) = match (it.next(), it.next(), it.next(), it.next()) {
                (Some(a), Some(b), Some(c), None) => (a, b, c),
                _ => panic!("lexicon line does not have 3 fields: {line:?}"),
            };
            let lm = lower(lem);
            let lem = if lemma && !lm.contains('|') { Some(lm.clone()) } else { None };
            (lower(form), tag, gov.get(lm.as_str()), if lw.contains(lm.as_str()) { Some(lm) } else { None }, lem)
        }).collect();
        ph("lines (parallel part)");
        for (fl, tag, gv, lwm, lem) in rows {
            if !tcache.contains_key(tag) {
                let b = bundle(tag);
                let k1 = intern(key(&b), &mut tmp_keys);
                let k2 = if tag.starts_with(":N:A") { let mut b2 = b.clone(); b2.pos = "A".into(); Some(intern(key(&b2), &mut tmp_keys)) } else { None };
                tcache.insert(tag, (b, k1, k2));
            }
            let (b0, ck1, ck2) = &tcache[tag];
            let g = if b0.pos == "V" { gv } else { None };
            let l = lwm.is_some() && matches!(b0.pos.as_str(), "V" | "N" | "A" | "ADV" | "DET" | "PRON" | "OTHER");
            let le = lem.is_some() && matches!(b0.pos.as_str(), "N" | "A" | "V" | "ADV");
            let (k1, k2) = if g.is_none() && !l && !le { (*ck1, *ck2) } else {
                let mut b = b0.clone();
                if let Some(g) = g { b.gov = Some(g.clone()); }
                if l { b.lw = lwm.clone(); }
                if le { b.lem = lem.clone(); }
                let k1 = intern(key(&b), &mut tmp_keys);
                let k2 = if tag.starts_with(":N:A") { let mut b2 = b.clone(); b2.pos = "A".into(); Some(intern(key(&b2), &mut tmp_keys)) } else { None };
                (k1, k2)
            };
            let slot = match index.get(&fl) { Some(&i) => i, None => { index.insert(fl.clone(), forms.len()); forms.push((fl, Vec::new())); forms.len() - 1 } };
            let v = &mut forms[slot].1;
            if !v.contains(&k1) { v.push(k1); }
            if let Some(k2) = k2 { if !v.contains(&k2) { v.push(k2); } }
        }
        ph("lines");
        // stripped open-class keys and the keys the characterizer adds itself
        let n_lex_keys = tmp_keys.len();
        let mut stripped_of: Vec<Option<u32>> = vec![None; n_lex_keys];
        for k in 0..n_lex_keys {
            let s = tmp_keys[k].clone();
            let first = s.split('|').next().unwrap();
            if matches!(first, "pos=N" | "pos=A" | "pos=V" | "pos=ADV") {
                let st: Vec<&str> = s.split('|').filter(|a| !a.starts_with("gov=") && !a.starts_with("lw=") && !a.starts_with("lem=")).collect();
                stripped_of[k] = Some(intern(st.join("|"), &mut tmp_keys));
            }
        }
        let mut det_ns: BTreeSet<String> = BTreeSet::new();
        for k in 0..n_lex_keys {
            let s = &tmp_keys[k];
            if s.starts_with("pos=DET") {
                for a in s.split('|') { if a.starts_with("n=") { det_ns.insert(a.split('=').nth(1).unwrap().to_string()); } }
            }
        }
        let specials = ["pos=PUNCT", "pos=NUM", "pos=ADV|sub=deictic", "pos=PROPN|sub=person", "pos=N", "pos=A"];
        let sp_ids: Vec<u32> = specials.iter().map(|s| intern(s.to_string(), &mut tmp_keys)).collect();
        let propn_n_tmp: Vec<(String, u32)> = det_ns.iter().map(|v| (v.clone(), intern(format!("pos=PROPN|sub=person|n={v}"), &mut tmp_keys))).collect();
        // phase 2: rank ids by string
        let mut order: Vec<u32> = (0..tmp_keys.len() as u32).collect();
        order.sort_by(|a, b| tmp_keys[*a as usize].cmp(&tmp_keys[*b as usize]));
        let mut rank = vec![0u32; tmp_keys.len()];
        for (r, &t) in order.iter().enumerate() { rank[t as usize] = r as u32; }
        let keys: Vec<String> = order.iter().map(|&t| tmp_keys[t as usize].clone()).collect();
        let keys_json: Vec<String> = keys.iter().map(|k| { let mut o = String::new(); crate::text::pyjson::write_str(&mut o, k); o }).collect();
        let mut info: Vec<KeyInfo> = keys.iter().map(|s| {
            let det = s.starts_with("pos=DET");
            KeyInfo {
                det,
                pron: s.starts_with("pos=PRON"),
                propn: s.starts_with("pos=PROPN"),
                has_pron: s.contains("pos=PRON"),
                n_or_a: s.starts_with("pos=N") || s.starts_with("pos=A"),
                det_n: s.split('|').filter(|a| a.starts_with("n=")).map(|a| a.split('=').nth(1).unwrap().to_string()).collect(),
                open_stripped: None,
            }
        }).collect();
        for (t, st) in stripped_of.iter().enumerate() {
            if let Some(st) = st { info[rank[t] as usize].open_stripped = Some(rank[*st as usize]); }
        }
        ph("rank");
        // read: ranked, sorted
        let mut read: FxMap<String, Box<[Key]>> = FxMap::with_capacity_and_hasher(forms.len(), Default::default());
        let mut forms_ranked: Vec<(String, Box<[Key]>)> = Vec::with_capacity(forms.len());
        for (f, ks) in forms {
            let mut v: Vec<Key> = ks.iter().map(|k| rank[*k as usize]).collect();
            v.sort_unstable();
            forms_ranked.push((f, v.into_boxed_slice()));
        }
        ph("read");
        // suffix statistics, in dict order of `read`. Sharded by suffix hash: every shard walks all forms in order
        // and owns its suffixes, so each suffix's Counter sees exactly Python's sequence of `+=`.
        let shards = rayon::current_num_threads().clamp(1, 64) as u64;
        let shard_of = |s: &str| { use std::hash::{BuildHasher, BuildHasherDefault}; (BuildHasherDefault::<crate::text::fxhash::Fx>::default().hash_one(s) % shards) as u8 };
        type Open<'a> = Option<(Vec<Key>, f64, [(&'a str, u8); 5])>;
        let open_of: Vec<Open> = forms_ranked.par_iter().with_min_len(4096).map(|(form, ks)| {
            if !str_isalpha(form) || form.chars().count() < 5 { return None; }
            let mut open: Vec<Key> = ks.iter().filter_map(|k| info[*k as usize].open_stripped).collect();
            open.sort_unstable();
            open.dedup();
            if open.is_empty() { return None; }
            let inc = 1.0 / open.len() as f64;
            let sufs = [2, 3, 4, 5, 6].map(|l| { let s = last_chars(form, l); (s, shard_of(s)) });
            Some((open, inc, sufs))
        }).collect();
        let suffix: FxMap<String, Vec<Key>> = (0..shards as u8).into_par_iter().map(|sh| {
            let mut cnt: FxMap<&str, Vec<(Key, f64)>> = FxMap::default();
            for (open, inc, sufs) in open_of.iter().flatten() {
                for &(suf, s) in sufs {
                    if s != sh { continue; }
                    let c = cnt.entry(suf).or_default();
                    for &k in open {
                        match c.iter_mut().find(|e| e.0 == k) { Some(e) => e.1 += inc, None => c.push((k, *inc)) }
                    }
                }
            }
            let mut out: Vec<(String, Vec<Key>)> = Vec::new();
            for (suf, mut c) in cnt {
                let tot = neumaier_sum(c.iter().map(|e| e.1));
                if tot >= 20.0 {
                    c.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // stable: ties keep insertion order (most_common)
                    out.push((suf.to_string(), c.into_iter().take(6).filter(|e| e.1 / tot >= 0.08).map(|e| e.0).collect()));
                }
            }
            out
        }).flatten().collect();
        ph("suffix");
        for (f, ks) in forms_ranked { read.insert(f, ks); }
        ph("done");
        let r = |i: usize| rank[sp_ids[i] as usize];
        Lexicon {
            k_punct: r(0), k_num: r(1), k_deictic: r(2), k_propn: r(3), k_n: r(4), k_a: r(5),
            k_propn_n: propn_n_tmp.into_iter().map(|(v, t)| (v, rank[t as usize])).collect(),
            keys, keys_json, info, read, suffix,
        }
    }

    pub fn has(&self, low: &str) -> bool { self.read.contains_key(low) }
}

/// CPython 3.12 `sum()` over floats (Neumaier compensated summation, as in bltinmodule.c).
pub fn neumaier_sum(it: impl Iterator<Item = f64>) -> f64 {
    let (mut f, mut c) = (0.0f64, 0.0f64);
    for x in it {
        let t = f + x;
        if f.abs() >= x.abs() { c += (f - t) + x; } else { c += (x - t) + f; }
        f = t;
    }
    if c != 0.0 && c.is_finite() { f += c; }
    f
}

// ---------------------------------------------------------------- tokenizer

#[inline]
fn tok_word_char(c: char) -> bool { c == '’' || c == '\'' || c == '-' || crate::text::pystr::is_w(c) }

/// `INV.match(w)` -> (group 1, group 3), group 1 as a byte length of `w`.
fn inv_match(w: &str) -> Option<(usize, &str)> {
    let idx: Vec<(usize, char)> = w.char_indices().collect();
    let n = idx.len();
    let eq = |seg: &[(usize, char)], pat: &str| -> bool {
        let mut it = seg.iter();
        for p in pat.chars() {
            match it.next() { Some(&(_, t)) if crate::text::pystr::ci_eq(t, p) => {}, _ => return false }
        }
        it.next().is_none()
    };
    for p in 1..n {
        if idx[p - 1].1 == '\n' { return None; } // `.` does not cross a newline
        let rest = &idx[p..];
        for with_t in [true, false] {
            let r = if with_t {
                if rest.len() < 3 || rest[0].1 != '-' || !crate::text::pystr::ci_eq(rest[1].1, 't') { continue; }
                &rest[2..]
            } else { rest };
            if r.is_empty() || r[0].1 != '-' { continue; }
            let alt = &r[1..];
            if alt.is_empty() { continue; }
            for a in INV_ALTS {
                if eq(alt, a) { return Some((idx[p].0, &w[alt[0].0..])); }
            }
        }
    }
    None
}

/// `tokenize(text)`
pub fn tokenize(lex: &Lexicon, text: &str) -> Vec<String> {
    let text = if text.contains('\'') { std::borrow::Cow::Owned(text.replace('\'', "’")) } else { std::borrow::Cow::Borrowed(text) };
    let mut out: Vec<String> = Vec::new();
    let b = text.as_ref();
    let mut it = b.char_indices().peekable();
    while let Some((s, c)) = it.next() {
        let e;
        if tok_word_char(c) {
            let mut end = s + c.len_utf8();
            while let Some(&(j, d)) = it.peek() { if tok_word_char(d) { end = j + d.len_utf8(); it.next(); } else { break; } }
            e = end;
        } else if crate::text::pystr::is_space(c) { continue; } else { e = s + c.len_utf8(); }
        let mut w: &str = &b[s..e];
        let low = lower(w);
        let lowc = low.chars().count();
        if let Some(p) = ELIDED.iter().find(|p| low.starts_with(**p) && lowc > p.chars().count()) {
            let pl = p.chars().count();
            let cut = w.char_indices().nth(pl).map_or(w.len(), |x| x.0);
            out.push(w[..cut].to_string());
            w = &w[cut..];
        }
        let mut tail: Vec<String> = Vec::new();
        while w.contains('-') && !lex.has(&lower(w)) {
            match inv_match(w) {
                Some((g1, g3)) => { tail.insert(0, format!("-{g3}")); w = &w[..g1]; }
                None => break,
            }
        }
        out.push(w.to_string());
        out.extend(tail);
    }
    out
}

/// `SENT.split(text)` with SENT = `(?<=[.!?…»])\s+(?=[«A-ZÉÈÀÂÎÔÇ—-])`.
pub fn sent_split(text: &str) -> Vec<&str> {
    let closer = |c: char| matches!(c, '.' | '!' | '?' | '…' | '»');
    let opener = |c: char| matches!(c, '«' | 'A'..='Z' | 'É' | 'È' | 'À' | 'Â' | 'Î' | 'Ô' | 'Ç' | '—' | '-');
    let mut out = Vec::new();
    let mut last = 0usize;
    let mut prev: Option<char> = None;
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if crate::text::pystr::is_space(c) {
            let start = i;
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = it.peek() { if crate::text::pystr::is_space(d) { end = j + d.len_utf8(); it.next(); } else { break; } }
            let next = text[end..].chars().next();
            if prev.map_or(false, closer) && next.map_or(false, opener) {
                out.push(&text[last..start]);
                last = end;
            }
            prev = Some(text[start..end].chars().last().unwrap());
            continue;
        }
        prev = Some(c);
    }
    out.push(&text[last..]);
    out
}

// ---------------------------------------------------------------- characterizer

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src { Punct, Lex, Shape, ShapeInitial, Unknown, Suffix, SuffixContext }

#[derive(Clone, Debug)]
pub struct Tok {
    pub r: Vec<Key>,
    pub src: Src,
    pub guessed: bool,
}

impl Lexicon {
    /// `Characterizer.known(w)` (sorted, unique).
    pub fn known(&self, w: &str) -> Vec<Key> {
        let low = lower(w);
        if let Some((_, xs)) = ELIDED_MAP.iter().find(|(p, _)| *p == low) {
            let mut v: Vec<Key> = xs.iter().flat_map(|x| self.read.get(*x).map(|b| b.to_vec()).unwrap_or_default()).collect();
            v.sort_unstable(); v.dedup();
            return v;
        }
        if low == "-là" || low == "-ci" { return vec![self.k_deictic]; }
        if let Some(rest) = low.strip_prefix('-') {
            if let Some(ks) = self.read.get(rest) { return ks.iter().copied().filter(|k| self.info[*k as usize].has_pron).collect(); }
        }
        self.read.get(&low).map(|b| b.to_vec()).unwrap_or_default()
    }

    /// `Characterizer.sentence(toks)`
    pub fn sentence(&self, toks: &[String]) -> Vec<Tok> {
        let mut out: Vec<Tok> = Vec::with_capacity(toks.len());
        for (i, w) in toks.iter().enumerate() {
            if !has_w(w) { out.push(Tok { r: vec![self.k_punct], src: Src::Punct, guessed: false }); continue; }
            let mut ks = self.known(w);
            let c0 = w.chars().next().unwrap();
            let mid_cap = (i > 0 && is_upper(c0) && has_w(&toks[i - 1])) || (i > 0 && matches!(toks[i - 1].as_str(), "," | ";" | ":"));
            if !ks.is_empty() {
                if mid_cap && is_upper(c0) && !ks.iter().any(|k| self.info[*k as usize].propn) {
                    if let Err(p) = ks.binary_search(&self.k_propn) { ks.insert(p, self.k_propn); }
                }
                out.push(Tok { r: ks, src: Src::Lex, guessed: false });
                continue;
            }
            let g = self.guess(w, i, &out);
            out.push(g);
        }
        out
    }

    fn guess(&self, w: &str, i: usize, prev: &[Tok]) -> Tok {
        let tok = |r: Vec<Key>, src: Src| Tok { r, src, guessed: true };
        if str_isdigit(w) || num_fullmatch(w) { return tok(vec![self.k_num], Src::Shape); }
        if !w.is_empty() && w.len() <= 6 && w.bytes().all(|b| b"IVXLCDM".contains(&b)) { return tok(vec![self.k_num], Src::Shape); }
        let c0 = w.chars().next().unwrap();
        if is_upper(c0) {
            let mut r = self.k_propn;
            if let Some(p) = prev.last() {
                if p.src == Src::Lex && p.r.iter().all(|k| { let f = &self.info[*k as usize]; f.det || f.pron }) {
                    let mut ns: Vec<&str> = Vec::new();
                    for k in &p.r {
                        let f = &self.info[*k as usize];
                        if f.det { for v in &f.det_n { if !ns.contains(&v.as_str()) { ns.push(v); } } }
                    }
                    if ns.len() == 1 { r = self.k_propn_n[ns[0]]; }
                }
            }
            return tok(vec![r], if i > 0 { Src::Shape } else { Src::ShapeInitial });
        }
        let low = lower(w);
        let mut cands: Option<&Vec<Key>> = None;
        for l in [6, 5, 4, 3, 2] {
            if let Some(c) = self.suffix.get(last_chars(&low, l)) { cands = Some(c); break; }
        }
        let cands = match cands { Some(c) if !c.is_empty() => c, _ => return tok(vec![self.k_n, self.k_a], Src::Unknown) };
        if let Some(p) = prev.last() {
            if p.src == Src::Lex {
                let any_det = p.r.iter().any(|k| self.info[*k as usize].det);
                if any_det && p.r.iter().all(|k| self.info[*k as usize].det) {
                    let na: Vec<Key> = cands.iter().copied().filter(|k| self.info[*k as usize].n_or_a).collect();
                    if !na.is_empty() { return tok(na, Src::SuffixContext); }
                }
            }
        }
        tok(cands.clone(), Src::Suffix)
    }

    /// featdata.feats(words) as one JSON array: `[["k", ...] + ["guessed"], ...]`.
    pub fn feats_json(&self, toks: &[Tok], o: &mut String) {
        o.push('[');
        for (i, t) in toks.iter().enumerate() {
            if i > 0 { o.push_str(", "); }
            o.push('[');
            for (j, k) in t.r.iter().enumerate() {
                if j > 0 { o.push_str(", "); }
                o.push_str(&self.keys_json[*k as usize]);
            }
            if t.guessed { if !t.r.is_empty() { o.push_str(", "); } o.push_str("\"guessed\""); }
            o.push(']');
        }
        o.push(']');
    }
}

/// `re.fullmatch(r"\d+([.,]\d+)?", w)`
fn num_fullmatch(w: &str) -> bool {
    let mut it = w.chars().peekable();
    let mut n = 0;
    while let Some(&c) = it.peek() { if is_decimal(c) { n += 1; it.next(); } else { break; } }
    if n == 0 { return false; }
    match it.next() {
        None => true,
        Some('.') | Some(',') => { let rest: Vec<char> = it.collect(); !rest.is_empty() && rest.iter().all(|c| is_decimal(*c)) }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_examples() {
        assert_eq!(key(&bundle(":V1_it_q_zz:Ip:3s")), "pos=V|n=s|p=3|mood=Ip|grp=1|tr=itq|aux=?");
        assert_eq!(key(&bundle(":G:Dd:m:s")), "pos=DET|sub=dem|g=m|n=s");
        assert_eq!(key(&bundle("")), "pos=OTHER");
    }
    #[test]
    fn neumaier_matches_python_sum() {
        // python3.12 -c "print(repr(sum([0.1]*10)))" -> 1.0 (plain accumulation gives 0.9999999999999999)
        assert_eq!(neumaier_sum([0.1f64; 10].into_iter()), 1.0);
    }
    #[test]
    fn sentence_split() {
        assert_eq!(sent_split("Il vient. Elle part !  « Non » dit-il. ok"), vec!["Il vient.", "Elle part !", "« Non » dit-il. ok"]);
    }
}
