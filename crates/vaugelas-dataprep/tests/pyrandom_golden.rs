//! CPython parity of the `random.Random` stream, against `tests/pyrandom.json`.
//!
//! The fixture was generated twice, by Python 3.12.3 (the ThinkPad's interpreter, the one that produced the
//! archived datasets) and by Python 3.14.4 (this laptop); the two files were byte-identical, which is why the
//! golden carries no Python version. Values: `random()`, `getrandbits(k)` for k in {5, 17, 33, 64},
//! `_randbelow(1000)`, `shuffle(range(20))` and `choice` over the generator's 16 training names.
use serde_json::Value;
use vaugelas_dataprep::pyrandom::PyRandom;

const NAMES: [&str; 16] = [
    "Jean", "Marie", "Pierre", "Paul", "Anne", "Luc", "Marthe", "Thomas", "Claire", "Étienne", "Catherine",
    "Michel", "Hélène", "Vincent", "Suzanne", "Antoine",
];

fn u64s(v: &Value) -> Vec<u64> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect()
}

#[test]
fn matches_cpython() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pyrandom.json")).unwrap();
    let all: Value = serde_json::from_str(&text).unwrap();
    for (seed_s, d) in all.as_object().unwrap() {
        let seed: u64 = seed_s.parse().unwrap();
        let mut r = PyRandom::new(seed);
        let f64s: Vec<f64> = d["random"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(f64s, (0..f64s.len()).map(|_| r.random()).collect::<Vec<_>>(), "seed {seed}: random()");
        for k in [5u32, 17, 33, 64] {
            let mut r = PyRandom::new(seed);
            let got: Vec<u64> = (0..5).map(|_| r.getrandbits(k)).collect();
            assert_eq!(u64s(&d[format!("getrandbits{k}")]), got, "seed {seed}: getrandbits({k})");
        }
        let mut r = PyRandom::new(seed);
        let got: Vec<u64> = (0..8).map(|_| r.randbelow(1000)).collect();
        assert_eq!(u64s(&d["randbelow1000"]), got, "seed {seed}: _randbelow(1000)");
        let mut r = PyRandom::new(seed);
        let mut xs: Vec<u64> = (0..20).collect();
        r.shuffle(&mut xs);
        assert_eq!(u64s(&d["shuffle20"]), xs, "seed {seed}: shuffle");
        let mut r = PyRandom::new(seed);
        let got: Vec<&str> = (0..10).map(|_| *r.choice(&NAMES)).collect();
        let want: Vec<&str> = d["choice16"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
        assert_eq!(want, got, "seed {seed}: choice");
    }
}
