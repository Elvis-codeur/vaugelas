//! vaugelas: French grammar checker (rules + energy model). See `vaugelas --help`.
mod config;
mod data;
mod lt;

use serde_json::json;
use std::io::Read;
use config::{Config, Filter};
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
  vaugelas settings                       réglages en vigueur et fichier de réglages /
                                          settings in force and the settings file
  vaugelas settings --init                crée le fichier de réglages, tout expliqué dedans /
                                          creates the settings file, everything explained in it
  vaugelas rules                          liste des règles (pour disabled_rules) / list of rules

OPTIONS
  --json               sortie JSON / JSON output
  --threshold T        seuil du modèle, 0 à 1 (défaut : celui du modèle, 0.9935 pour N1) ;
                       plus haut = moins d’alertes / model threshold (default: the model's own,
                       0.9935 for N1); higher = fewer flags
  --rules-only         règles seulement / rules only
  --model-only         modèle seulement / model only
  --strict-typo        conventions typographiques (espaces insécables, …) / typographic conventions
  --no-spell-suggest   pas de suggestions orthographiques (plus rapide) / no spelling suggestions (faster)
  --int8               modèle en int8 (moins de mémoire) / int8 model weights (less memory)
  --data DIR           dossier des données / data directory (or VAUGELAS_DATA)
  --config FICHIER     fichier de réglages / settings file (or VAUGELAS_CONFIG)
  serve: --port N (8081), --host H (127.0.0.1), --threads N
  download-model: --from DIR (install from a folder holding model.frcm.gz, lexicon.tsv.gz and
                  resources.tar.gz instead of downloading), --force

Tous ces réglages peuvent aussi être écrits une fois pour toutes dans le fichier de réglages
(vaugelas settings --init) ; la ligne de commande passe avant le fichier. / Every setting can also be
written once in the settings file (vaugelas settings --init); the command line takes precedence.

Au premier lancement, les données sont prises dans le dossier du programme (ou le dossier courant)
si les trois fichiers y sont, sinon téléchargées. / On first run the data files are taken from the
program's folder (or the current folder) when the three files are there, else downloaded.
";

struct Args { rest: Vec<String>, flags: Vec<String>, opts: Vec<(String, String)> }

impl Args {
    fn parse() -> Result<Args, String> {
        const WITH_VALUE: [&str; 8] = ["--threshold", "--data", "--config", "--port", "--host", "--threads", "--from", "-f"];
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

/// The check settings: command line first, then the settings file (`c`), then the defaults.
fn settings(a: &Args, c: &config::Check) -> Settings {
    let threshold = a.opt("--threshold").map(|t| match t.parse::<f64>() {
        Ok(x) if (0.0..=1.0).contains(&x) => x,
        _ => fail("--threshold needs a number between 0 and 1"),
    }).or(c.threshold);
    let s = Settings {
        threshold,
        rules: c.rules && !a.flag("--model-only"),
        model: c.model && !a.flag("--rules-only"),
        spell_suggestions: c.spell_suggestions && !a.flag("--no-spell-suggest"),
        strict_typography: c.strict_typography || a.flag("--strict-typo"),
    };
    if !s.rules && !s.model { fail("ni règles ni modèle : rien à vérifier / neither rules nor model: nothing to check"); }
    s
}

fn data_dir(a: &Args, cfg: &Config) -> std::path::PathBuf { data::data_dir(a.opt("--data"), cfg.data.dir.as_deref()) }

/// The model's own settings, read from the installed model's header.
fn model_profile(dir: &Path) -> Result<vaugelas::model::profile::Profile, String> {
    let f = data::Files::in_dir(dir);
    vaugelas::model::read_header(&f.model.to_string_lossy()).and_then(|h| vaugelas::model::profile::Profile::from_header(&h))
}

/// Load everything; installs the data first when it is missing (first run).
fn load(a: &Args, cfg: &Config) -> Data {
    let dir = data_dir(a, cfg);
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
    let d = Data::load(&paths, a.flag("--int8") || cfg.check.int8, &LexCache::NextToTsv).unwrap_or_else(|e| fail(&e));
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

fn cmd_check(a: &Args, cfg: &Config) {
    let s = settings(a, &cfg.check);
    let filter = Filter::from_check(&cfg.check);
    let texts = inputs(a);
    let d = load(a, cfg);
    let vg = Vaugelas::new(&d);
    let json = a.flag("--json");
    for (k, t) in texts.iter().enumerate() {
        let issues: Vec<Issue> = vg.check(t, &s).into_iter().filter(|i| filter.keeps(i)).collect();
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
fn cmd_score(a: &Args, cfg: &Config) {
    let texts = inputs(a);
    let d = load(a, cfg);
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

/// `vaugelas settings`: the file in use and every setting in force (command line + file + defaults).
fn cmd_settings(a: &Args, ld: &config::Loaded) {
    let cfg = &ld.cfg;
    match &ld.path {
        Some(p) => println!("Fichier de réglages / settings file: {}", p.display()),
        None => {
            println!("Aucun fichier de réglages : réglages par défaut / no settings file: defaults");
            if let Some(u) = config::user_file() { println!("  vaugelas settings --init  crée / creates {}", u.display()); }
        }
    }
    let s = settings(a, &cfg.check);
    let dir = data_dir(a, cfg);
    let threshold = match s.threshold {
        Some(t) => format!("{t}"),
        None => match model_profile(&dir) {
            Ok(p) => format!("{} (seuil du modèle {} / the model's own)", p.threshold, p.name),
            Err(_) => "seuil du modèle / the model's own (modèle non installé / model not installed)".into(),
        },
    };
    let list = |v: &[String]| if v.is_empty() { "aucune / none".to_string() } else { v.join(", ") };
    println!("\n[check]\n  threshold           = {threshold}\n  rules               = {}\n  model               = {}", s.rules, s.model);
    println!("  spell_suggestions   = {}\n  strict_typography   = {}\n  int8                = {}", s.spell_suggestions, s.strict_typography, a.flag("--int8") || cfg.check.int8);
    println!("  disabled_rules      = {}\n  disabled_categories = {}", list(&cfg.check.disabled_rules), list(&cfg.check.disabled_categories));
    println!("[data]\n  dir                 = {}", dir.display());
    let threads = cfg.server.threads.map_or("nombre de cœurs, au plus 4 / cores, at most 4".into(), |t| t.to_string());
    println!("[server]\n  host                = {}\n  port                = {}\n  threads             = {threads}", cfg.server.host, cfg.server.port);
}

/// `vaugelas settings --init`: write the explained default settings file (`--config FILE`, else the user's folder).
fn cmd_settings_init(a: &Args) {
    let path = a.opt("--config").map(std::path::PathBuf::from).or_else(config::user_file)
        .unwrap_or_else(|| fail("pas de dossier de réglages connu : utilisez --config FICHIER / no settings folder known: use --config FILE"));
    if path.exists() && !a.flag("--force") {
        fail(&format!("{} existe déjà (--force pour le remplacer) / already exists (--force to replace it)", path.display()));
    }
    if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).unwrap_or_else(|e| fail(&format!("{}: {e}", d.display())));
    }
    std::fs::write(&path, config::TEMPLATE).unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
    println!("Fichier de réglages créé / settings file created: {}", path.display());
    println!("Ouvrez-le avec un éditeur de texte ; chaque réglage y est expliqué. / Open it in a text editor; every setting is explained in it.");
}

fn main() {
    let a = Args::parse().unwrap_or_else(|e| fail(&e));
    if a.flag("--version") || a.flag("-V") { println!("vaugelas {}", env!("CARGO_PKG_VERSION")); return; }
    if a.flag("--help") || a.flag("-h") || a.rest.first().is_none_or(|c| c == "help") { print!("{HELP}"); return; }
    if a.rest[0] == "settings" && a.flag("--init") { return cmd_settings_init(&a); }
    let ld = config::load(a.opt("--config")).unwrap_or_else(|e| fail(&e));
    let cfg = &ld.cfg;
    match a.rest[0].as_str() {
        "check" => cmd_check(&a, cfg),
        "score" => cmd_score(&a, cfg),
        "serve" => {
            let port: u16 = a.opt("--port").map(|p| p.parse().unwrap_or_else(|_| fail("--port needs a number"))).unwrap_or(cfg.server.port);
            let host = a.opt("--host").unwrap_or(&cfg.server.host).to_string();
            let threads: usize = a.opt("--threads").map(|p| p.parse().unwrap_or_else(|_| fail("--threads needs a number"))).or(cfg.server.threads)
                .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get().min(4)));
            let s = settings(&a, &cfg.check);
            let d = load(&a, cfg);
            lt::serve(&Vaugelas::new(&d), &host, port, threads.max(1), s, Filter::from_check(&cfg.check)).unwrap_or_else(|e| fail(&e));
        }
        "download-model" => {
            let dir = data_dir(&a, cfg);
            data::install(&dir, a.opt("--from").map(Path::new), a.flag("--force")).unwrap_or_else(|e| fail(&e));
            eprintln!("Données installées / data installed: {}", dir.display());
        }
        "info" => {
            let dir = data_dir(&a, cfg);
            let f = data::Files::in_dir(&dir);
            println!("vaugelas {}\ndata directory: {}\ndata set: {}\ncomplete: {}", env!("CARGO_PKG_VERSION"), dir.display(), data::DATA_SET, f.complete());
            for p in [&f.model, &f.lexicon, &f.resources, &f.lex_dir] { println!("  {} {}", if p.exists() { "ok     " } else { "missing" }, p.display()); }
            if f.model.is_file() {
                match model_profile(&dir) {
                    Ok(p) => println!("model: {}\n  default threshold: {}\n  fr-check lints not read by the model: {}", p.name, p.threshold, p.frc_ignore.join(",")),
                    Err(e) => println!("model: {e}"),
                }
            }
            println!("settings file: {}", ld.path.as_ref().map_or("none (defaults)".into(), |p| p.display().to_string()));
        }
        "settings" => cmd_settings(&a, &ld),
        "rules" => {
            for (r, desc) in vaugelas::report::RULES {
                println!("{r:<22} {:<10} {desc}", vaugelas::report::Category::of_rule(r).id());
            }
        }
        c => fail(&format!("unknown command '{c}' (see vaugelas --help)")),
    }
}
