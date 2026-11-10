//! The settings file (`settings.toml`): every user setting of `check` and `serve`, read at each start.
//!
//! Where it is looked for, first found wins: `--config FILE`, the `VAUGELAS_CONFIG` variable (both must name an
//! existing file), `settings.toml` next to the program, then the user's settings folder (`user_file`). No file means
//! the defaults. Command-line options take precedence over the file; the file takes precedence over the defaults.
//! Unknown keys, values out of range and unknown rule or category names are errors, never silently ignored.
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vaugelas::report::{Category, Issue, RULES};

pub const FILE_NAME: &str = "settings.toml";

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub check: Check,
    pub data: DataCfg,
    pub server: Server,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Check {
    /// None = the model's own threshold
    pub threshold: Option<f64>,
    pub rules: bool,
    pub model: bool,
    pub spell_suggestions: bool,
    pub strict_typography: bool,
    pub int8: bool,
    pub disabled_rules: Vec<String>,
    pub disabled_categories: Vec<String>,
}

impl Default for Check {
    fn default() -> Self {
        Check { threshold: None, rules: true, model: true, spell_suggestions: true, strict_typography: false, int8: false,
                disabled_rules: Vec::new(), disabled_categories: Vec::new() }
    }
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataCfg {
    /// data directory; a relative path is relative to the settings file's folder
    pub dir: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    pub host: String,
    pub port: u16,
    /// None = the number of cores, at most 4
    pub threads: Option<usize>,
}

impl Default for Server {
    fn default() -> Self { Server { host: "127.0.0.1".into(), port: 8081, threads: None } }
}

/// The settings in force and the file they came from (None: no file, defaults).
pub struct Loaded { pub path: Option<PathBuf>, pub cfg: Config }

/// Rules and categories never reported.
#[derive(Clone, Debug, Default)]
pub struct Filter { rules: HashSet<String>, categories: HashSet<String> }

impl Filter {
    pub fn from_check(c: &Check) -> Filter {
        Filter { rules: c.disabled_rules.iter().cloned().collect(), categories: c.disabled_categories.iter().cloned().collect() }
    }
    pub fn keeps(&self, i: &Issue) -> bool { !self.rules.contains(&i.rule) && !self.categories.contains(i.category.id()) }
}

fn env_path(k: &str) -> Option<PathBuf> { std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from) }

/// The settings file in the user's settings folder: Windows `%APPDATA%\vaugelas`, macOS
/// `~/Library/Application Support/vaugelas`, Linux `$XDG_CONFIG_HOME/vaugelas` or `~/.config/vaugelas`.
pub fn user_file() -> Option<PathBuf> {
    let dir = if cfg!(windows) {
        env_path("APPDATA").map(|d| d.join("vaugelas"))
    } else if cfg!(target_os = "macos") {
        env_path("HOME").map(|h| h.join("Library/Application Support/vaugelas"))
    } else {
        env_path("XDG_CONFIG_HOME").map(|d| d.join("vaugelas")).or_else(|| env_path("HOME").map(|h| h.join(".config/vaugelas")))
    };
    dir.map(|d| d.join(FILE_NAME))
}

fn beside_program() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(FILE_NAME)))
}

/// The settings file to read, if any (see the module comment for the order).
pub fn find(explicit: Option<&str>) -> Result<Option<PathBuf>, String> {
    let named = explicit.map(PathBuf::from).map(|p| (p, "--config"))
        .or_else(|| env_path("VAUGELAS_CONFIG").map(|p| (p, "VAUGELAS_CONFIG")));
    if let Some((p, by)) = named {
        return if p.is_file() { Ok(Some(p)) } else {
            Err(format!("{by}: fichier introuvable / file not found: {}", p.display()))
        };
    }
    Ok([beside_program(), user_file()].into_iter().flatten().find(|p| p.is_file()))
}

pub fn load(explicit: Option<&str>) -> Result<Loaded, String> {
    let Some(path) = find(explicit)? else { return Ok(Loaded { path: None, cfg: Config::default() }) };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let cfg = parse(&text, &path)?;
    Ok(Loaded { path: Some(path), cfg })
}

/// Parse and check a settings file; `origin` is its path (for messages and relative data folders).
pub fn parse(text: &str, origin: &Path) -> Result<Config, String> {
    let at = |m: String| format!("{} : {m}", origin.display());
    let mut c: Config = toml::from_str(text).map_err(|e| at(e.to_string()))?;
    let k = &c.check;
    if let Some(t) = k.threshold {
        if !(0.0..=1.0).contains(&t) { return Err(at(format!("threshold = {t} : il faut un nombre entre 0 et 1 / must be between 0 and 1"))); }
    }
    if !k.rules && !k.model { return Err(at("rules et model sont tous deux à false : rien à vérifier / rules and model are both false: nothing to check".into())); }
    for r in &k.disabled_rules {
        if !RULES.iter().any(|(x, _)| x == r) { return Err(at(format!("disabled_rules : règle inconnue / unknown rule \"{r}\" (liste / list: vaugelas rules)"))); }
    }
    for g in &k.disabled_categories {
        if !Category::ALL.iter().any(|x| x.id() == g) {
            let all: Vec<_> = Category::ALL.iter().map(|x| x.id()).collect();
            return Err(at(format!("disabled_categories : catégorie inconnue / unknown category \"{g}\" (possibles / known: {})", all.join(", "))));
        }
    }
    if let Some(d) = &c.data.dir {
        if d.as_os_str().is_empty() { return Err(at("[data] dir est vide / is empty".into())); }
        if d.is_relative() {
            c.data.dir = Some(origin.parent().unwrap_or(Path::new(".")).join(d));
        }
    }
    if c.server.host.trim().is_empty() { return Err(at("[server] host est vide / is empty".into())); }
    if c.server.threads == Some(0) { return Err(at("[server] threads = 0 : il faut au moins 1 / must be at least 1".into())); }
    Ok(c)
}

/// What `vaugelas settings --init` writes: every setting with its default and an explanation (French, English).
pub const TEMPLATE: &str = r#"# Réglages de Vaugelas / Vaugelas settings
#
# Vaugelas lit ce fichier à chaque lancement. Une ligne qui commence par # est un commentaire : enlevez le # pour
# activer le réglage. Les options de la ligne de commande passent avant ce fichier.
# « vaugelas settings » montre le fichier utilisé et les réglages en vigueur.
#
# Vaugelas reads this file at every start. A line starting with # is a comment: remove the # to use the setting.
# Command-line options take precedence over this file. "vaugelas settings" shows the file in use and the settings
# in force.

[check]

# Seuil du modèle, entre 0 et 1 : le modèle signale un mot quand son score dépasse ce seuil.
#   Plus bas  : Vaugelas trouve plus d'erreurs, mais signale aussi plus de mots corrects.
#   Plus haut : moins d'alertes, presque toutes justes, mais plus d'erreurs passent.
# Par défaut, le seuil du modèle (0.9914 pour le modèle H1), choisi pour très peu de fausses alertes : le modèle
# seul signale à tort environ 3,5 phrases correctes sur 1 000. Exemple : « Les enfants joue dans le jardin. » passe au
# seuil par défaut (score 0.981) et est signalée avec threshold = 0.98.
#
# Model threshold, between 0 and 1: the model flags a word when its score is above it.
#   Lower : Vaugelas finds more errors, but also flags more correct words.
#   Higher: fewer flags, nearly all right, but more errors get through.
# Default: the model's own threshold (0.9914 for model H1), chosen for very few false alarms: the model alone
# flags about 3.5 correct sentences in 1,000. Example: "Les enfants joue dans le jardin." passes at the default threshold
# (score 0.981) and is flagged with threshold = 0.98.
#threshold = 0.98

# Vérifier avec les règles de grammaire / check with the grammar rules
rules = true
# Vérifier avec le modèle / check with the model
model = true
# Proposer des corrections pour les mots inconnus (false = plus rapide) / suggest spellings for unknown words (false = faster)
spell_suggestions = true
# Signaler les conventions typographiques : espaces insécables, apostrophe ’, points de suspension …
# Report typographic conventions: non-breaking spaces, ’ apostrophe, … ellipsis
strict_typography = false
# Modèle en int8 : moins de mémoire, scores très légèrement différents / int8 model: less memory, very slightly different scores
int8 = false

# Règles à ne jamais signaler ; la liste des règles : « vaugelas rules »
# Rules never to report; the list of rules: "vaugelas rules"
# Exemple / example: disabled_rules = ["FR_SPELL", "FR_CAPITAL"]
disabled_rules = []
# Catégories à ne jamais signaler / categories never to report:
# GRAMMAR (grammaire), TYPOS (orthographe / spelling), TYPOGRAPHY (typographie), CASING (majuscules / capitals)
disabled_categories = []

[data]
# Dossier des données (modèle, lexique). Par défaut : Windows %LOCALAPPDATA%\vaugelas,
# macOS ~/Library/Application Support/vaugelas, Linux ~/.local/share/vaugelas. Un chemin relatif part du dossier de
# ce fichier. La variable VAUGELAS_DATA et l'option --data passent avant.
# Data folder (model, lexicon); defaults as above. A relative path starts from this file's folder.
# The VAUGELAS_DATA variable and the --data option take precedence.
#dir = "vaugelas-data"

[server]
# Pour « vaugelas serve » (serveur compatible LanguageTool) / for "vaugelas serve" (LanguageTool-compatible server)
# 127.0.0.1 = cet ordinateur seulement / this computer only
host = "127.0.0.1"
port = 8081
# Nombre de fils (défaut : nombre de cœurs, au plus 4) / worker threads (default: number of cores, at most 4)
#threads = 4
"#;

#[cfg(test)]
mod tests {
    use super::*;
    fn p(s: &str) -> Result<Config, String> { parse(s, Path::new("/x/settings.toml")) }

    #[test]
    fn template_is_the_defaults() {
        assert_eq!(p(TEMPLATE).unwrap(), Config::default());
        assert_eq!(p("").unwrap(), Config::default());
    }

    #[test]
    fn every_template_example_is_valid() {
        // uncomment each "#key = value" line of the template: the file must still parse
        let on = TEMPLATE.lines().map(|l| match l.strip_prefix('#') {
            Some(r) if r.split_once(" = ").is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_lowercase() || c == '_')) => r,
            _ => l,
        }).collect::<Vec<_>>().join("\n");
        let c = p(&on).unwrap();
        assert_eq!(c.check.threshold, Some(0.98));
        assert_eq!(c.server.threads, Some(4));
        assert_eq!(c.data.dir, Some(PathBuf::from("/x/vaugelas-data")));
    }

    #[test]
    fn values_are_read() {
        let c = p("[check]\nthreshold = 0.95\nmodel = false\ndisabled_rules = [\"FR_SPELL\"]\ndisabled_categories = [\"TYPOGRAPHY\"]\n\
                   [server]\nport = 9000\n[data]\ndir = \"/d\"").unwrap();
        assert_eq!(c.check.threshold, Some(0.95));
        assert!(!c.check.model && c.check.rules);
        assert_eq!(c.server.port, 9000);
        assert_eq!(c.data.dir, Some(PathBuf::from("/d")));
        let f = Filter::from_check(&c.check);
        assert!(f.rules.contains("FR_SPELL") && f.categories.contains("TYPOGRAPHY"));
    }

    #[test]
    fn mistakes_are_errors() {
        for bad in ["[check]\nthreshold = 1.5", "[check]\nthreshold = -0.1", "[check]\nrules = false\nmodel = false",
                    "[check]\ndisabled_rules = [\"FR_NOPE\"]", "[check]\ndisabled_categories = [\"grammar\"]",
                    "[check]\nthresold = 0.9", "[checks]\nrules = true", "[server]\nport = 70000", "[server]\nthreads = 0",
                    "[server]\nhost = \"\"", "[data]\ndir = \"\"", "[check]\nrules = \"yes\"", "[check]\nthreshold = 0.9\nthreshold = 0.8"] {
            assert!(p(bad).is_err(), "accepted: {bad}");
        }
        assert!(p("[check]\nthresold = 0.9").unwrap_err().contains("thresold"));
    }
}
