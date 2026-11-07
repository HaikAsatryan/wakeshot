//! The built `wakeshot.exe`: help and errors as a user sees them.

use std::process::{Command, Output};

fn wakeshot(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wakeshot"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_goes_to_stdout_with_exit_0() {
    for args in [&["--help"][..], &[], &["help"], &["--dry-run", "-h"]] {
        let out = wakeshot(args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.starts_with("Wakeshot keeps this PC awake"), "{text}");
        assert!(text.contains("run -- <command>"), "{text}");
        assert!(out.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn an_unknown_command_is_one_line_on_stderr_with_exit_2() {
    let out = wakeshot(&["nonsense"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "wakeshot: unknown command 'nonsense'. Run 'wakeshot --help'.\n"
    );
}

#[test]
fn a_late_dry_run_is_an_error_before_anything_runs() {
    let out = wakeshot(&["status", "--dry-run"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "wakeshot: put '--dry-run' before the command, such as 'wakeshot --dry-run status'. Run \
         'wakeshot --help'.\n"
    );
}
