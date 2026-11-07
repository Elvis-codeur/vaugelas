//! Memory-mapped French lexicon: surface form -> readings (lemma, morphology).
//! Files (produced by `frc-build`): words.fst, recs.bin, lemmas.bin, tags.txt, gen.fst.
use fst::automaton::Levenshtein;
use fst::{IntoStreamer, Map, Set, Streamer};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub const M_IP: u16 = 1;
pub const M_IQ: u16 = 2;
pub const M_IS: u16 = 4;
pub const M_IF: u16 = 8;
pub const M_K: u16 = 16;
pub const M_SP: u16 = 32;
pub const M_SQ: u16 = 64;
pub const M_E: u16 = 128;
pub const M_Y: u16 = 256;
pub const M_P: u16 = 512;
pub const M_Q: u16 = 1024;
/// Finite moods (carry a person).
pub const M_FIN: u16 = M_IP | M_IQ | M_IS | M_IF | M_K | M_SP | M_SQ;

pub const G_M: u8 = 1;
pub const G_F: u8 = 2;
pub const N_S: u8 = 1;
pub const N_P: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pos {
    Noun, Adj, Verb, Adv, Prep, Det, DetPoss, DetDem, DetInd, DetNum,
    ConjC, ConjS, PronSubj, PronObj, PronDem, PronOther, Neg, Proper, Interj, Other,
}

impl Pos {
    pub fn is_det(self) -> bool {
        matches!(self, Pos::Det | Pos::DetPoss | Pos::DetDem | Pos::DetInd | Pos::DetNum)
    }
}

/// One morphological reading of a word. gender/number are bit sets (unknown = all bits).
#[derive(Clone, Copy, Debug)]
pub struct Morph {
    pub pos: Pos,
    pub gender: u8,
    pub number: u8,
    pub person: u8, // bit 1,2,4 = 1st,2nd,3rd
    pub moods: u16,
    pub lemma: u32,
    pub tag: u16,
}

#[derive(Clone, Copy)]
struct TagInfo { pos: Pos, gender: u8, number: u8, person: u8, moods: u16, dual: bool }

pub fn parse_tag(tag: &str) -> (Pos, u8, u8, u8, u16) {
    let mut it = tag.split(':').filter(|s| !s.is_empty());
    let first = it.next().unwrap_or("");
    let rest: Vec<&str> = it.collect();
    let c = first.chars().next().unwrap_or(' ');
    let mut pos = match c {
        'N' => Pos::Noun, 'A' => Pos::Adj, 'V' => Pos::Verb, 'M' => Pos::Proper,
        'W' => Pos::Adv, 'J' => Pos::Interj, _ => Pos::Other,
    };
    if c == 'G' {
        let k = rest.first().copied().unwrap_or("");
        pos = match k {
            "D" => Pos::Det, "Dp" => Pos::DetPoss, "Dd" => Pos::DetDem, "Di" => Pos::DetInd,
            "Dn" => Pos::DetNum, "R" => Pos::Prep, "Cs" => Pos::ConjS, "Cc" => Pos::ConjC,
            "W" => Pos::Adv, "X" => Pos::Neg,
            _ => {
                if rest.iter().any(|r| *r == "Os") { Pos::PronSubj }
                else if rest.iter().any(|r| r.starts_with("Oo")) { Pos::PronObj }
                else if rest.iter().any(|r| *r == "Od") { Pos::PronDem }
                else if k.starts_with('O') { Pos::PronOther }
                else { Pos::Other }
            }
        };
    }
    let (mut g, mut n, mut p, mut mo) = (0u8, 0u8, 0u8, 0u16);
    for r in &rest {
        // tags can carry suffixes: "s;C", "i#G#L"
        let r = &r.split(|c| c == ';' || c == '#').next().unwrap_or("");
        match *r {
            "m" => g |= 1, "f" => g |= 2, "e" => g |= 3,
            "s" => n |= 1, "p" => n |= 2, "i" => n |= 3,
            "Ip" => mo |= M_IP, "Iq" => mo |= M_IQ, "Is" => mo |= M_IS, "If" => mo |= M_IF,
            "K" => mo |= M_K, "Sp" => mo |= M_SP, "Sq" => mo |= M_SQ, "E" => mo |= M_E,
            "Y" => mo |= M_Y, "P" => mo |= M_P, "Q" => mo |= M_Q,
            "O1" => p |= 1, "O2" => p |= 2, "O3" => p |= 4,
            s => {
                let b = s.as_bytes();
                if b.len() >= 2 && (b'1'..=b'3').contains(&b[0]) {
                    p |= 1 << (b[0] - b'1');
                    if b[1] == b's' { n |= 1 } else if b[1] == b'p' { n |= 2 }
                }
            }
        }
    }
    if g == 0 { g = 3 }
    if n == 0 { n = 3 }
    (pos, g, n, p, mo)
}

pub struct Lexicon {
    words: Map<Mmap>,
    recs: Mmap,
    lemmas: Mmap,
    gen: Set<Mmap>,
    tags: Vec<TagInfo>,
}

fn map_file(p: &Path) -> std::io::Result<Mmap> {
    let f = File::open(p)?;
    unsafe { Mmap::map(&f) }
}

impl Lexicon {
    pub fn open(dir: &Path) -> Result<Lexicon, Box<dyn std::error::Error>> {
        let words = Map::new(map_file(&dir.join("words.fst"))?)?;
        let gen = Set::new(map_file(&dir.join("gen.fst"))?)?;
        let recs = map_file(&dir.join("recs.bin"))?;
        let lemmas = map_file(&dir.join("lemmas.bin"))?;
        let tags_txt = std::fs::read_to_string(dir.join("tags.txt"))?;
        let tags = tags_txt
            .lines()
            .map(|l| {
                let (mut pos, gender, number, person, moods) = parse_tag(l);
                // ":N:A:..." = common word usable as noun and as adjective (petit, grand, content)
                let dual = l.starts_with(":N:A");
                if dual { pos = Pos::Adj; }
                TagInfo { pos, gender, number, person, moods, dual }
            })
            .collect();
        Ok(Lexicon { words, recs, lemmas, gen, tags })
    }

    pub fn lemma(&self, id: u32) -> &str {
        let b = &self.lemmas[..];
        let count = u32::from_le_bytes(b[0..4].try_into().unwrap()) as usize;
        let id = id as usize;
        if id >= count { return ""; }
        let off = |k: usize| u32::from_le_bytes(b[4 + 4 * k..8 + 4 * k].try_into().unwrap()) as usize;
        let base = 4 + 4 * (count + 1);
        std::str::from_utf8(&b[base + off(id)..base + off(id + 1)]).unwrap_or("")
    }

    fn lookup_exact(&self, w: &str, out: &mut Vec<Morph>) -> bool {
        let Some(off) = self.words.get(w) else { return false };
        let b = &self.recs[off as usize..];
        let n = b[0] as usize;
        let mut p = 1;
        for _ in 0..n {
            let (mut lemma, mut shift) = (0u32, 0);
            loop {
                let x = b[p]; p += 1;
                lemma |= ((x & 0x7f) as u32) << shift;
                if x & 0x80 == 0 { break }
                shift += 7;
            }
            let tag = u16::from_le_bytes([b[p], b[p + 1]]); p += 2;
            let t = self.tags[tag as usize];
            out.push(Morph { pos: t.pos, gender: t.gender, number: t.number, person: t.person, moods: t.moods, lemma, tag });
            if t.dual {
                out.push(Morph { pos: Pos::Noun, gender: t.gender, number: t.number, person: t.person, moods: t.moods, lemma, tag });
            }
        }
        true
    }

    /// Readings of a word. Tries the exact form, then the lowercase form.
    /// Returns (known, readings).
    pub fn analyze(&self, word: &str) -> (bool, Vec<Morph>) {
        let norm: String = word.replace('\'', "’");
        let mut out = Vec::new();
        let mut known = self.lookup_exact(&norm, &mut out);
        let lower = norm.to_lowercase();
        if lower != norm && self.lookup_exact(&lower, &mut out) { known = true; }
        (known, out)
    }

    pub fn known(&self, word: &str) -> bool { self.analyze(word).0 }

    /// All noun/adjective/verb forms of a lemma.
    pub fn forms(&self, lemma: &str) -> Vec<(String, Morph)> {
        let mut prefix = lemma.as_bytes().to_vec();
        prefix.push(0);
        let mut end = prefix.clone();
        *end.last_mut().unwrap() = 1;
        let mut st = self.gen.range().ge(&prefix).lt(&end).into_stream();
        let mut res = Vec::new();
        while let Some(k) = st.next() {
            let s = &k[prefix.len()..];
            let mut it = s.split(|b| *b == 0);
            let form = std::str::from_utf8(it.next().unwrap_or(b"")).unwrap_or("").to_string();
            let tag: usize = std::str::from_utf8(it.next().unwrap_or(b"0")).ok().and_then(|x| x.parse().ok()).unwrap_or(0);
            let t = self.tags[tag];
            res.push((form, Morph { pos: t.pos, gender: t.gender, number: t.number, person: t.person, moods: t.moods, lemma: u32::MAX, tag: tag as u16 }));
        }
        res
    }

    /// Typical French slips fixed by rule (accents, g+a/o/u -> ge, doubled consonants, plural -als -> -aux).
    fn repairs(&self, lower: &str) -> Vec<String> {
        let chars: Vec<char> = lower.chars().collect();
        let mut out: Vec<String> = Vec::new();
        let try_add = |v: Vec<char>, out: &mut Vec<String>| {
            let s: String = v.into_iter().collect();
            if s != lower && self.words.contains_key(&s) && !out.contains(&s) { out.push(s); }
        };
        let table: [(char, &str); 12] = [('e', "éèêë"), ('a', "àâä"), ('u', "ùûü"), ('i', "îï"), ('o', "ôö"), ('c', "ç"),
            ('é', "eèêë"), ('è', "eéêë"), ('ê', "eéèë"), ('à', "aâ"), ('ç', "c"), ('ô', "o")];
        for (k, c) in chars.iter().enumerate() {
            for (src, alts) in table.iter() {
                if c == src { for a in alts.chars() { let mut v = chars.clone(); v[k] = a; try_add(v, &mut out); } }
            }
        }
        for k in 0..chars.len().saturating_sub(1) {
            if chars[k] == 'g' && "aou".contains(chars[k + 1]) { let mut v = chars.clone(); v.insert(k + 1, 'e'); try_add(v, &mut out); }
            if chars[k] == 'c' && "aou".contains(chars[k + 1]) { let mut v = chars.clone(); v[k] = 'ç'; try_add(v, &mut out); }
            if chars[k] == chars[k + 1] { let mut v = chars.clone(); v.remove(k); try_add(v, &mut out); }
        }
        for k in 1..chars.len() {
            if !"aeiouyéèêàùîôâ’-".contains(chars[k]) { let mut v = chars.clone(); v.insert(k, chars[k]); try_add(v, &mut out); }
        }
        for (a, b) in [("als", "aux"), ("ails", "aux"), ("eaus", "eaux"), ("s", "x"), ("x", "s"), ("aus", "aux")] {
            if let Some(stem) = lower.strip_suffix(a) { let s = format!("{stem}{b}"); if self.words.contains_key(&s) && !out.contains(&s) { out.push(s); } }
        }
        // generic adjacent swaps last: French-specific repairs above are more likely ("mangons" -> "mangeons", not "magnons")
        for k in 0..chars.len().saturating_sub(1) {
            if chars[k] != chars[k + 1] { let mut v = chars.clone(); v.swap(k, k + 1); try_add(v, &mut out); }
        }
        out
    }

    /// Spelling suggestions: rule-based repairs first, then edit distance 1, then 2.
    pub fn suggest(&self, word: &str, max: usize) -> Vec<String> {
        let norm = word.replace('\'', "’");
        let lower = norm.to_lowercase();
        let cap = norm.chars().next().map_or(false, |c| c.is_uppercase());
        let rep = self.repairs(&lower);
        if !rep.is_empty() {
            let mut r = rep;
            for s in self.suggest_lev(&lower, cap, max) { if !r.contains(&s) { r.push(s); } }
            r.truncate(max);
            return r;
        }
        self.suggest_lev(&lower, cap, max)
    }

    fn suggest_lev(&self, lower: &str, cap: bool, max: usize) -> Vec<String> {
        let lower = lower.to_string();
        // adjacent swaps are already covered by `repairs`, so distance 2 is only a fallback
        for d in [1u32, 2] {
            let Ok(lev) = Levenshtein::new(&lower, d) else { continue };
            let mut st = self.words.search(lev).into_stream();
            let mut cands: Vec<(u32, bool, String)> = Vec::new();
            while let Some((k, _)) = st.next() {
                let Ok(s) = std::str::from_utf8(k) else { continue };
                if s.chars().next().map_or(false, |c| c.is_uppercase()) && !cap { continue; }
                let dist = damerau(&lower, s);
                let first_same = s.chars().next() == lower.chars().next();
                cands.push((dist, !first_same, s.to_string()));
                if cands.len() > 400 { break; }
            }
            if !cands.is_empty() {
                cands.sort();
                return cands.into_iter().take(max).map(|(_, _, s)| s).collect();
            }
        }
        Vec::new()
    }

    pub fn word_count(&self) -> usize { self.words.len() }
}

/// Optimal-string-alignment (Damerau) distance.
pub fn damerau(a: &str, b: &str) -> u32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0u32; m + 1]; n + 1];
    for i in 0..=n { d[i][0] = i as u32; }
    for j in 0..=m { d[0][j] = j as u32; }
    for i in 1..=n {
        for j in 1..=m {
            let cost = (a[i - 1] != b[j - 1]) as u32;
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}
