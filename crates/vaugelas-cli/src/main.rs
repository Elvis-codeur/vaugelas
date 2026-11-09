//! vaugelas: French grammar checker (rules + energy model). See `vaugelas --help`.
mod data;
mod lt;

use serde_json::json;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use vaugelas::model::pipeline::LexCache;
use vaugelas::report::{Data, DataPaths, Issue, Settings, Vaugelas};

const HELP: &str = "\
Vaugelas : correcteur de grammaire française / French grammar checker

UTILISATION / USAGE
  vaugelas check \"Texte à vérifier.\"     vérifie un texte / check a text
  vaugelas check -f FICHIER.txt           vérifie un fichier / check a file
  vaugelas check                          lit l’entrée standard / read standard input
  vaugelas serve [--port 8081]            serveur compatible LanguageTool / LanguageTool-compatible server
  vaugelas download-model                 installe les données / install the data files
  vaugelas info                           où sont les données / where the data files are
  vaugelas score -f PHRASES.txt           scores du modèle, une phrase par ligne (JSON) /
                                          model scores, one sentence per line (JSON)

OPTIONS
  --json               sortie JSON / JSON output
  --threshold T        seuil du modèle, 0 à 1 (défaut : celui du modèle, 0.9914 pour H1) ;
                       plus haut = moins d’alertes / model threshold (default: the model's own,
                       0.9914 for H1); higher = fewer flags
  --rules-only         règles seulement / rules only
  --model-only         modèle seulement / model only
  --strict-typo        conventions typographiques (espaces insécables, …) / typographic conventions
  --no-spell-suggest   pas de suggestions orthographiques (plus rapide) / no spelling suggestions (faster)
  --int8               modèle en int8 (moins de mémoire) / int8 model weights (less memory)
  --data DIR           dossier des données / data directory (or VAUGELAS_DATA)
  serve: --port N (8081), --host H (127.0.0.1), --threads N
  download-model: --from DIR (install from a folder holding model.frcm.gz, lexicon.tsv.gz and
                  resources.tar.gz instead of downloading), --force

Au premier lancement, les données sont prises dans le dossier du programme (ou le dossier courant)
si les trois fichiers y sont, sinon téléchargées. / On first run the data files are taken from the
program's folder (or the current folder) when the three files are there, else downloaded.
";

struct Args { rest: Vec<String>, flags: Vec<String>, opts: Vec<(String, String)> }

impl Args {
    fn parse() -> Result<Args, String> {
        const WITH_VALUE: [&str; 7] = ["--threshold", "--data", "--port", "--host", "--threads", "--from", "-f"];
        let mut a = Args { rest: Vec::new(), flags: Vec::new(), opts: Vec::new() };
        let mut it = std::env::args().skip(1);
        while let Some(x) = it.next() {
            if WITH_VALUE.contains(&x.as_str()) {
                let v = it.next().ok_or(format!("{x} needs a value"))?;
                a.opts.push((x, v));
            } else if x.starts_with('-') && x.len() > 1 && x != "--" {
                a.flags.push(x);
            } else { a.rest.push(x); }
        }
        Ok(a)
    }
    fn flag(&self, f: &str) -> bool { self.flags.iter().any(|x| x == f) }
    fn opt(&self, o: &str) -> Option<&str> { self.opts.iter().rev().find(|(k, _)| k == o).map(|(_, v)| v.as_str()) }
}

fn fail(msg: &str) -> ! { eprintln!("vaugelas: {msg}"); std::process::exit(1) }

fn settings(a: &Args) -> Settings {
    let threshold = a.opt("--threshold").map(|t| match t.parse::<f64>() {
        Ok(x) if (0.0..=1.0).contains(&x) => x,
        _ => fail("--threshold needs a number between 0 and 1"),
    });
    Settings {
        threshold,
        rules: !a.flag("--model-only"),
        model: !a.flag("--rules-only"),
        spell_suggestions: !a.flag("--no-spell-suggest"),
        strict_typography: a.flag("--strict-typo"),
    }
}

/// Load everything; installs the data first when it is missing (first run).
fn load(a: &Args) -> Data {
    let dir = data::data_dir(a.opt("--data"));
    let files = data::Files::in_dir(&dir);
    if !files.complete() {
        eprintln!("Première utilisation : installation des données / first run: installing the data files");
        let local = data::local_source();
        if let Err(e) = data::install(&dir, local.as_deref(), false) {
            let names = data::download_names().join(", ");
            fail(&format!("{e}\n\nTéléchargez ces trois fichiers depuis la page des versions (Releases) et posez-les à côté du programme,\n\
                           puis relancez-le / download these three files from the Releases page, put them next to the program and run it again:\n  {names}\n\
                           (ou / or: vaugelas download-model --from DOSSIER)"));
        }
    }
    let t0 = Instant::now();
    let paths = DataPaths { lex_dir: &files.lex_dir, lexicon_tsv: &files.lexicon, resources: &files.resources, model: &files.model };
    let d = Data::load(&paths, a.flag("--int8"), &LexCache::NextToTsv).unwrap_or_else(|e| fail(&e));
    eprintln!("chargé en / loaded in {:.1} s (modèle / model {})", t0.elapsed().as_secs_f64(), d.model.profile.name);
    d
}

fn line_col(text: &str, b: usize) -> (usize, usize) {
    let before = &text[..b];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}

fn issue_json(text: &str, i: &Issue) -> serde_json::Value {
    let (line, column) = line_col(text, i.start);
    json!({
        "offset": text[..i.start].chars().count(), "length": text[i.start..i.end].chars().count(),
        "line": line, "column": column, "text": &text[i.start..i.end],
        "rule": i.rule, "category": i.category.id(), "message": i.message, "replacements": i.replacements,
        "source": i.source.as_str(), "score": i.score,
    })
}

fn print_text(text: &str, issues: &[Issue]) {
    if issues.is_empty() { println!("Aucune erreur trouvée. / No error found."); return; }
    for i in issues {
        let (l, c) = line_col(text, i.start);
        let sugg = if i.replacements.is_empty() { String::new() } else { format!("  →  {}", i.replacements.join(" | ")) };
        let score = i.score.map(|s| format!(", score {s:.3}")).unwrap_or_default();
        println!("ligne {l}, colonne {c} : « {} »{sugg}", &text[i.start..i.end]);
        println!("    {}  [{}, {}{}]", i.message, i.rule, i.source.as_str(), score);
    }
}

/// The texts to check: arguments after the command, the file of -f, else standard input.
fn inputs(a: &Args) -> Vec<String> {
    let mut texts: Vec<String> = a.rest[1..].to_vec();
    if let Some(f) = a.opt("-f") {
        let bytes = std::fs::read(f).unwrap_or_else(|e| fail(&format!("{f}: {e}")));
        texts.push(String::from_utf8(bytes).unwrap_or_else(|_| fail(&format!("{f}: not UTF-8 text"))));
    }
    if texts.is_empty() {
        let mut t = String::new();
        std::io::stdin().read_to_string(&mut t).unwrap_or_else(|e| fail(&format!("stdin: {e}")));
        texts.push(t);
    }
    texts
}

fn cmd_check(a: &Args) {
    let s = settings(a);
    let texts = inputs(a);
    let d = load(a);
    let vg = Vaugelas::new(&d);
    let json = a.flag("--json");
    for (k, t) in texts.iter().enumerate() {
        let issues = vg.check(t, &s);
        if json {
            println!("{}", json!({"matches": issues.iter().map(|i| issue_json(t, i)).collect::<Vec<_>>()}));
        } else {
            if texts.len() > 1 { println!("— texte {} —", k + 1); }
            print_text(t, &issues);
        }
    }
}

/// Every non-blank line is one sentence, scored as a whole (no sentence splitting), exactly like
/// `frc-infer v14`: one JSON line per sentence {"text", "skip", "words", "scores", "suggestions"}.
fn cmd_score(a: &Args) {
    let texts = inputs(a);
    let d = load(a);
    let vg = Vaugelas::new(&d);
    let out = std::io::stdout();
    let mut out = std::io::BufWriter::new(out.lock());
    use std::io::Write;
    for line in texts.iter().flat_map(|t| t.lines()).filter(|l| !l.trim().is_empty()) {
        let v = match vg.scorer.score_sentence(line) {
            Ok(sc) => json!({
                "text": line, "skip": sc.skip.map(|k| k.name()), "words": sc.words,
                "scores": sc.scores.iter().map(|x| x.0).collect::<Vec<_>>(),
                "suggestions": sc.scores.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
            }),
            Err(e) => json!({"text": line, "error": e}),
        };
        writeln!(out, "{v}").unwrap_or_else(|e| fail(&format!("stdout: {e}")));
    }
    out.flush().unwrap_or_else(|e| fail(&format!("stdout: {e}")));
}

fn main() {
    let a = Args::parse().unwrap_or_else(|e| fail(&e));
    if a.flag("--help") || a.flag("-h") || a.rest.first().is_none_or(|c| c == "help") { print!("{HELP}"); return; }
    if a.flag("--version") || a.flag("-V") { println!("vaugelas {}", env!("CARGO_PKG_VERSION")); return; }
    match a.rest[0].as_str() {
        "check" => cmd_check(&a),
        "score" => cmd_score(&a),
        "serve" => {
            let port: u16 = a.opt("--port").map(|p| p.parse().unwrap_or_else(|_| fail("--port needs a number"))).unwrap_or(8081);
            let host = a.opt("--host").unwrap_or("127.0.0.1").to_string();
            let threads: usize = a.opt("--threads").map(|p| p.parse().unwrap_or_else(|_| fail("--threads needs a number")))
                .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get().min(4)));
            let s = settings(&a);
            let d = load(&a);
            lt::serve(&Vaugelas::new(&d), &host, port, threads.max(1), s).unwrap_or_else(|e| fail(&e));
        }
        "download-model" => {
            let dir = data::data_dir(a.opt("--data"));
            data::install(&dir, a.opt("--from").map(Path::new), a.flag("--force")).unwrap_or_else(|e| fail(&e));
            eprintln!("Données installées / data installed: {}", dir.display());
        }
        "info" => {
            let dir = data::data_dir(a.opt("--data"));
            let f = data::Files::in_dir(&dir);
            println!("vaugelas {}\ndata directory: {}\ndata set: {}\ncomplete: {}", env!("CARGO_PKG_VERSION"), dir.display(), data::DATA_SET, f.complete());
            for p in [&f.model, &f.lexicon, &f.resources, &f.lex_dir] { println!("  {} {}", if p.exists() { "ok     " } else { "missing" }, p.display()); }
            if f.model.is_file() {
                let p = vaugelas::model::read_header(&f.model.to_string_lossy()).and_then(|h| vaugelas::model::profile::Profile::from_header(&h));
                match p {
                    Ok(p) => println!("model: {}\n  default threshold: {}\n  fr-check lints not read by the model: {}", p.name, p.threshold, p.frc_ignore.join(",")),
                    Err(e) => println!("model: {e}"),
                }
            }
        }
        c => fail(&format!("unknown command '{c}' (see vaugelas --help)")),
    }
}
