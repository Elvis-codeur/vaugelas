//! Command-line flags that need no data files.
use std::process::Command;

fn run(args: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_vaugelas")).args(args).output().unwrap();
    assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn version_flag_prints_the_version() {
    let want = format!("vaugelas {}\n", env!("CARGO_PKG_VERSION"));
    for f in ["--version", "-V"] {
        assert_eq!(run(&[f]), want, "{f}");
    }
}

#[test]
fn no_command_prints_the_help() {
    assert!(run(&[]).contains("vaugelas check"));
    assert!(run(&["--help"]).contains("vaugelas check"));
}
