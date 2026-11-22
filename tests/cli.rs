//! The binary's command line, run as a user would.

use std::process::{Command, Output};

fn macabout(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_macabout"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("UTF-8 output")
}

#[test]
fn text_prints_the_summary() {
    let output = macabout(&["--text", "--mock"]);
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "Zorin OS
Version 17.1
MacBook Air (13-inch, Mid 2013)

Processor  2.9 GHz Dual-Core Intel Core i7
   Memory  8 GB 1600 MHz DDR3
 Graphics  Intel HD Graphics 5000 1536 MB
"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn help_lists_the_options() {
    let output = macabout(&["--help"]);
    assert!(output.status.success());
    let help = stdout(&output);
    for line in ["Usage: macabout [OPTIONS]", "--mock", "--text", "--version"] {
        assert!(help.contains(line), "{line} missing from:\n{help}");
    }
}

#[test]
fn version() {
    let output = macabout(&["--version"]);
    assert_eq!(
        stdout(&output),
        format!("macabout {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_arguments_are_an_error() {
    for args in [&["--bogus"][..], &["extra"], &["--text=1"]] {
        let output = macabout(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("error: "));
    }
}
