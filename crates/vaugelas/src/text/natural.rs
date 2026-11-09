//! The part of training-design/natural.py featdata depends on: `parse_tag`, `READ` (form -> readings) and `BYLEM`
//! (lemma -> V/A readings).
use crate::text::pystr::lower;
use crate::text::fxhash::FxMap;

pub const IP: u16 = 1;
pub const IQ: u16 = 2;
pub const IS: u16 = 4;
pub const IF: u16 = 8;
pub const K: u16 = 16;
pub const SP: u16 = 32;
pub const SQ: u16 = 64;
pub const E: u16 = 128;
pub const Y: u16 = 256;
pub const P: u16 = 512;
pub const Q: u16 = 1024;
const MOOD_NAMES: [(&str, u16); 11] = [("Ip", IP), ("Iq", IQ), ("Is", IS), ("If", IF), ("K", K), ("Sp", SP), ("Sq", SQ), ("E", E), ("Y", Y), ("P", P), ("Q", Q)];

/// One reading `(f, lem, pos, g, n, p, mo)`.
#[derive(Clone, Debug)]
pub struct Reading {
    pub form: String,
    pub lem: u32,
    pub pos: char,
    pub g: u8,
    pub n: u8,
    pub p: u8,
    pub mo: u16,
}

/// `parse_tag(tag)` -> (pos, g, n, p, moods)
pub fn parse_tag(tag: &str) -> (char, u8, u8, u8, u16) {
    let t: Vec<&str> = tag.split('/').next().unwrap().split(':').filter(|x| !x.is_empty()).collect();
    let pos = t.first().map_or('?', |x| x.chars().next().unwrap());
    let (mut g, mut n, mut p, mut mo) = (0u8, 0u8, 0u8, 0u16);
    for r in t.iter().skip(1) {
        let r = r.split([';', '#']).next().unwrap_or("");
        match r {
            "m" => g |= 1,
            "f" => g |= 2,
            "e" => g |= 3,
            "s" => n |= 1,
            "p" => n |= 2,
            "i" => n |= 3,
            _ => {
                if let Some((_, b)) = MOOD_NAMES.iter().find(|(m, _)| *m == r) { mo |= b; continue; }
                let rc: Vec<char> = r.chars().collect();
                if rc.len() >= 2 && "123".contains(rc[0]) && "sp".contains(rc[1]) {
                    p |= 1 << (rc[0] as u8 - b'1');
                    n |= if rc[1] == 's' { 1 } else { 2 };
                }
            }
        }
    }
    (pos, if g == 0 { 3 } else { g }, if n == 0 { 3 } else { n }, p, mo)
}

pub struct Natural {
    pub readings: Vec<Reading>,
    pub lemmas: Vec<String>,
    /// lower-cased form -> readings (file order)
    pub read: FxMap<String, Vec<u32>>,
    /// lemma -> V/A readings (file order)
    pub bylem: FxMap<u32, Vec<u32>>,
}

impl Natural {
    pub fn from_tsv(text: &str) -> Natural {
        let mut nat = Natural { readings: Vec::new(), lemmas: Vec::new(), read: FxMap::default(), bylem: FxMap::default() };
        let mut lem_ids: FxMap<&str, u32> = FxMap::default();
        for line in text.split('\n') {
            if line.is_empty() { continue; }
            let mut it = line.split('\t');
            let (f, lem, tag) = (it.next().unwrap(), it.next().expect("lemma"), it.next().expect("tag"));
            let (pos, g, n, p, mo) = parse_tag(tag);
            let li = *lem_ids.entry(lem).or_insert_with(|| { nat.lemmas.push(lem.to_string()); (nat.lemmas.len() - 1) as u32 });
            let both = pos == 'N' && tag.starts_with(":N:A");
            let fl = lower(f);
            for ps in if both { vec![pos, 'A'] } else { vec![pos] } {
                let id = nat.readings.len() as u32;
                nat.readings.push(Reading { form: f.to_string(), lem: li, pos: ps, g, n, p, mo });
                nat.read.entry(fl.clone()).or_default().push(id);
                if ps == 'V' || ps == 'A' { nat.bylem.entry(li).or_default().push(id); }
            }
        }
        nat
    }
}
