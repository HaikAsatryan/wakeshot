//! `wakeshot run -- <command>`: hold the PC awake while a command runs.
//!
//! Owner: sources track (`p4-cli-run`); fixes by `p6-int-dry-run-e2e`.

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;

pub fn main(global: &Global) -> ExitCode {
    let _ = global.dry_run;
    let _ = writeln!(std::io::stderr(), "wakeshot: 'run' is not built yet.");
    ExitCode::from(2)
}
