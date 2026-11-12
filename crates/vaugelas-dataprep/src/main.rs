//! `vaugelas-dataprep`: the Rust entry point of the data-preparation stack (see the crate documentation).
use std::path::{Path, PathBuf};
use std::time::Instant;
use vaugelas_dataprep::{lexpairs, product, records, DEFAULT_SEED};

const HELP: &str = "\
vaugelas-dataprep — préparation des données / data preparation, in Rust

USAGE
  vaugelas-dataprep lexpairs --out DIR [--per-subtype N] [--seed S] [--contexts N]
  vaugelas-dataprep product-pairs --out DIR --src BOOKS.jsonl [--n N] [--seed S]
  vaugelas-dataprep pairs --items ITEMS.jsonl --out RECORDS.jsonl[.gz] --data DIR
  vaugelas-dataprep cands -f TEXTS --out RECORDS.jsonl[.gz] --data DIR
  vaugelas-dataprep lexconf --lexpairs LEXPAIRS.jsonl --out CONF.json [--reverse]

COMMANDS
  lexpairs       Synthetic lexical minimal pairs (replaces lexpairs.py): DIR/lexpairs.jsonl + DIR/CARD.md.
                 Default 40 per sub-type (1,080 records), seed 20261112. --contexts 8 reproduces the earlier
                 revision (git 3693633) that produced the old data/lexpairs.jsonl.
  product-pairs  Minimal pairs inside real book sentences (replaces product_pairs.py): DIR/items.jsonl + CARD.md.
                 Reads the clean-text carrier JSONL ({\"text\": ...} per line); the corpus is not in this repo.
  pairs          Candidate records of a pair file (ebm_data2.py pairs): good record + bad record with gold.
  cands          Evaluation records (record_pf, no gold) for each line of TEXTS (JSON with text/sentence, or text).
  lexconf        Build the optional inference resource [[wrong, right, n], ...] from produced lexpairs records; `--reverse`
                 adds both directions. Drop it at <data>/resources/interp/lexpairs_conf.json to enable it in inference.

OPTIONS
  --data DIR     the model data: model.frcm, lexicon.tsv, resources/, lex/ (default: $VAUGELAS_DATA)
  --lexconf F    extra candidate confusions for pairs/cands, [[wrong, right, n], ...] (FRC_LEXPAIRS_CONF); the
                 archived product batch used one with 143 token pairs, which the v14 generator does not carry
  --jobs N       rayon threads (default: all cores)
  --int8         unused by the records (kept for symmetry with the model CLI)
  --version      version and random-stream contract (pyrandom-mt19937-v1)
  -v             timing and counters on standard error

The output files are byte-identical to the Python generators for the same inputs and seed; the record path is
parity-checked against ebm_data2.py on the same flag set (see tools/parity.py).
";

struct Args {
    rest: Vec<String>,
    opts: Vec<(String, String)>,
    flags: Vec<String>,
}

impl Args {
    fn parse() -> Result<Args, String> {
        const WITH_VALUE: [&str; 11] = ["--out", "--per-subtype", "--seed", "--contexts", "--src", "--n", "--items", "--data", "--jobs", "--lexconf", "--lexpairs"];
        let mut a = Args { rest: Vec::new(), opts: Vec::new(), flags: Vec::new() };
        let mut it = std::env::args().skip(1);
        while let Some(x) = it.next() {
            if WITH_VALUE.contains(&x.as_str()) || x == "-f" {
                let v = it.next().ok_or(format!("{x} needs a value"))?;
                a.opts.push((x, v));
            } else if x.starts_with('-') && x.len() > 1 {
                a.flags.push(x);
            } else {
                a.rest.push(x);
            }
        }
        Ok(a)
    }
    fn opt(&self, o: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(k, _)| k == o).map(|(_, v)| v.as_str())
    }
    fn num<T: std::str::FromStr>(&self, o: &str, default: T) -> Result<T, String> {
        match self.opt(o) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| format!("{o}: not a number: {v}")),
        }
    }
    fn flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("vaugelas-dataprep: {msg}");
    std::process::exit(1)
}

fn out_dir(a: &Args) -> PathBuf {
    match a.opt("--out") {
        Some(d) => PathBuf::from(d),
        None => fail("--out DIR is required"),
    }
}

fn data_dir(a: &Args) -> PathBuf {
    if let Some(d) = a.opt("--data") {
        return PathBuf::from(d);
    }
    if let Some(d) = std::env::var_os("VAUGELAS_DATA") {
        return PathBuf::from(d);
    }
    PathBuf::from(".local/share/vaugelas")
}

fn mkdir(p: &Path) -> Result<(), String> {
    std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn cmd_lexpairs(a: &Args) -> Result<(), String> {
    let out = out_dir(a);
    mkdir(&out)?;
    let per: usize = a.num("--per-subtype", 40)?;
    let seed: u64 = a.num("--seed", DEFAULT_SEED)?;
    let contexts: usize = a.num("--contexts", lexpairs::MAX_CONTEXTS)?;
    let t0 = Instant::now();
    let recs = lexpairs::records(per, seed, contexts);
    let body = lexpairs::body(&recs);
    let sha = lexpairs::sha256(&body);
    std::fs::write(out.join("lexpairs.jsonl"), &body).map_err(|e| format!("{}: {e}", out.display()))?;
    std::fs::write(out.join("CARD.md"), lexpairs::card(&recs, &body, per, seed, &sha)).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut splits: Vec<(&str, usize)> = Vec::new();
    for r in &recs {
        let s = r["split"].as_str().unwrap();
        match splits.iter_mut().find(|x| x.0 == s) {
            Some(x) => x.1 += 1,
            None => splits.push((s, 1)),
        }
    }
    eprintln!(
        "{}: {} records, sha256 {}…; splits {:?}; {:.2} s",
        out.join("lexpairs.jsonl").display(),
        recs.len(),
        &sha[..16],
        splits,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn cmd_product(a: &Args) -> Result<(), String> {
    let out = out_dir(a);
    mkdir(&out)?;
    let src = a.opt("--src").ok_or("--src BOOKS.jsonl is required (the clean-text carrier corpus)")?;
    let n: usize = a.num("--n", 2000)?;
    let seed: u64 = a.num("--seed", DEFAULT_SEED)?;
    let text = std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let t0 = Instant::now();
    let p = product::generate(&lines, n, seed);
    let sha = product::sha256(&p.body);
    std::fs::write(out.join("items.jsonl"), &p.body).map_err(|e| format!("{}: {e}", out.display()))?;
    std::fs::write(out.join("CARD.md"), product::card(&p, &sha, t0.elapsed().as_secs_f64())).map_err(|e| format!("{}: {e}", out.display()))?;
    let probe = p.items.iter().filter(|x| x["split"] == "probe").count();
    eprintln!(
        "{}: {} items ({probe} probe), sha256 {}…; {} carriers; {:.2} s",
        out.join("items.jsonl").display(),
        p.items.len(),
        &sha[..16],
        p.carriers,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn cmd_pairs(a: &Args) -> Result<(), String> {
    let items = a.opt("--items").ok_or("--items ITEMS.jsonl is required")?;
    let out = out_dir(a);
    let lexconf = a.opt("--lexconf").map(PathBuf::from);
    let st = records::pairs(Path::new(items), &out, &data_dir(a), lexconf.as_deref())?;
    eprintln!("{}: {} records in {:.2} s", out.display(), st.records, st.secs);
    Ok(())
}

fn cmd_lexconf(a: &Args) -> Result<(), String> {
    let lexpairs = a.opt("--lexpairs").ok_or("--lexpairs LEXPAIRS.jsonl is required")?;
    let out = out_dir(a);
    let (rows, n) = lexpairs::confusions(Path::new(lexpairs), a.flag("--reverse"))?;
    let body = vaugelas::text::pyjson::dumps(&serde_json::Value::Array(rows.clone())) + "\n";
    std::fs::write(&out, &body).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!("{}: {} distinct pairs, {} rows (reverse: {})", out.display(), n, rows.len(), a.flag("--reverse"));
    Ok(())
}

fn cmd_cands(a: &Args) -> Result<(), String> {
    let texts = a.opt("-f").ok_or("-f TEXTS is required")?;
    let out = out_dir(a);
    let lexconf = a.opt("--lexconf").map(PathBuf::from);
    let st = records::cands(Path::new(texts), &out, &data_dir(a), lexconf.as_deref())?;
    eprintln!("{}: {} records in {:.2} s", out.display(), st.records, st.secs);
    Ok(())
}

fn main() {
    let a = match Args::parse() {
        Ok(a) => a,
        Err(e) => fail(&e),
    };
    if a.flag("--version") {
        println!("vaugelas-dataprep {} ({}; seed {})", env!("CARGO_PKG_VERSION"), vaugelas_dataprep::ALGORITHM_VERSION, vaugelas_dataprep::DEFAULT_SEED);
        return;
    }
    if a.rest.is_empty() || a.flag("-h") || a.flag("--help") {
        print!("{HELP}");
        return;
    }
    if let Ok(n) = a.num::<usize>("--jobs", 0) {
        if n > 0 {
            let _ = rayon::ThreadPoolBuilder::new().num_threads(n).build_global();
        }
    }
    let r = match a.rest[0].as_str() {
        "lexpairs" => cmd_lexpairs(&a),
        "product-pairs" => cmd_product(&a),
        "pairs" => cmd_pairs(&a),
        "cands" => cmd_cands(&a),
        "lexconf" => cmd_lexconf(&a),
        other => fail(&format!("unknown command {other:?} (try --help)")),
    };
    if let Err(e) = r {
        fail(&e);
    }
}
