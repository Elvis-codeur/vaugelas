//! Command-line flags and the settings file, without the data files.
use std::path::PathBuf;
use std::process::{Command, Output};

/// A fresh folder per test; the user's own settings folder is replaced by it, so no real settings file is read.
fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vaugelas-cli-test-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vaugelas(home: &PathBuf, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vaugelas")).args(args)
        .env("XDG_CONFIG_HOME", home).env("APPDATA", home).env("HOME", home)
        .env_remove("VAUGELAS_CONFIG").env("VAUGELAS_DATA", home.join("no-data"))
        .output().unwrap()
}

fn ok(o: &Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn err(o: &Output) -> String {
    assert!(!o.status.success(), "should fail: {}", String::from_utf8_lossy(&o.stdout));
    String::from_utf8(o.stderr.clone()).unwrap()
}

#[test]
fn version_flag_prints_the_version() {
    let h = tmp("version");
    let want = format!("vaugelas {}\n", env!("CARGO_PKG_VERSION"));
    for f in ["--version", "-V"] {
        assert_eq!(ok(&vaugelas(&h, &[f])), want, "{f}");
    }
}

#[test]
fn no_command_prints_the_help() {
    let h = tmp("help");
    assert!(ok(&vaugelas(&h, &[])).contains("vaugelas check"));
    assert!(ok(&vaugelas(&h, &["--help"])).contains("vaugelas settings --init"));
}

#[test]
fn settings_init_writes_an_explained_file_once() {
    let h = tmp("init");
    let f = h.join("sub/settings.toml");
    let fs = f.to_str().unwrap();
    assert!(ok(&vaugelas(&h, &["settings", "--init", "--config", fs])).contains(fs));
    let text = std::fs::read_to_string(&f).unwrap();
    assert!(text.contains("threshold") && text.contains("disabled_rules"));
    assert!(err(&vaugelas(&h, &["settings", "--init", "--config", fs])).contains("--force"));
    ok(&vaugelas(&h, &["settings", "--init", "--config", fs, "--force"]));
    // without --config: the user's settings folder, then found there automatically
    ok(&vaugelas(&h, &["settings", "--init"]));
    let shown = ok(&vaugelas(&h, &["settings"]));
    assert!(shown.contains("settings.toml") && shown.contains("rules               = true"), "{shown}");
}

#[test]
fn settings_shows_file_values_and_command_line_wins() {
    let h = tmp("show");
    let f = h.join("s.toml");
    std::fs::write(&f, "[check]\nthreshold = 0.95\nstrict_typography = true\ndisabled_rules = [\"FR_SPELL\"]\n[server]\nport = 9000\n").unwrap();
    let fs = f.to_str().unwrap();
    let s = ok(&vaugelas(&h, &["settings", "--config", fs]));
    for want in ["threshold           = 0.95", "strict_typography   = true", "disabled_rules      = FR_SPELL", "port                = 9000"] {
        assert!(s.contains(want), "{want}\n{s}");
    }
    let s = ok(&vaugelas(&h, &["settings", "--config", fs, "--threshold", "0.98", "--rules-only"]));
    assert!(s.contains("threshold           = 0.98") && s.contains("model               = false"), "{s}");
}

#[test]
fn a_wrong_settings_file_is_an_error() {
    let h = tmp("wrong");
    let f = h.join("s.toml");
    for (text, says) in [("[check]\nthresold = 0.9\n", "thresold"), ("[check]\ndisabled_rules = [\"FR_NOPE\"]\n", "FR_NOPE"),
                         ("[check]\nthreshold = 2\n", "threshold")] {
        std::fs::write(&f, text).unwrap();
        let e = err(&vaugelas(&h, &["check", "--config", f.to_str().unwrap(), "Bonjour."]));
        assert!(e.contains(says), "{e}");
    }
    assert!(err(&vaugelas(&h, &["settings", "--config", h.join("missing.toml").to_str().unwrap()])).contains("missing.toml"));
    assert!(err(&vaugelas(&h, &["settings", "--rules-only", "--model-only"])).contains("nothing to check"));
}

#[test]
fn rules_lists_every_rule() {
    let h = tmp("rules");
    let s = ok(&vaugelas(&h, &["rules"]));
    assert!(s.contains("VAUGELAS_MODEL") && s.contains("FR_SPELL") && s.contains("FR_SUBJ_VERB"), "{s}");
}
