//! The command line: parsing, help and errors.
//!
//! Owner: sources track (`p4-cli-args`); fixes by `p6-int-dry-run-e2e`.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use crate::command::{Command, Global};

const USAGE: &str = "\
Wakeshot keeps this PC awake while something needs it.

Usage: wakeshot <command>

Commands (not built yet):
  run -- <command>
  keep-awake 30m|1h|2h|4h|on|off [--screen]
  automatic on|off
  status
  restore
  log
";

pub fn parse(args: &[OsString]) -> (Global, Command) {
    let global = Global { dry_run: false };
    let command = match args.first() {
        None => Command::Help,
        Some(a) if a == "--help" => Command::Help,
        Some(a) => Command::Bad(format!("unknown command '{}'", a.to_string_lossy())),
    };
    (global, command)
}

// Write errors are ignored here and below: `println!` would panic on a closed pipe, and the release build aborts on
// panic.
pub fn help() -> ExitCode {
    let _ = std::io::stdout().write_all(USAGE.as_bytes());
    ExitCode::SUCCESS
}

pub fn bad(why: &str) -> ExitCode {
    let _ = writeln!(std::io::stderr(), "wakeshot: {why}. Run 'wakeshot --help'.");
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn help_for_no_arguments_or_help() {
        let plain = Global { dry_run: false };
        assert_eq!(parse(&args(&[])), (plain, Command::Help));
        assert_eq!(parse(&args(&["--help"])), (plain, Command::Help));
        assert_eq!(
            parse(&args(&["status"])),
            (plain, Command::Bad("unknown command 'status'".into()))
        );
    }
}
