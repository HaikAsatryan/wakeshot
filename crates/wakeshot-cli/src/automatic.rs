//! `wakeshot automatic on|off`: write `config.json`.
//!
//! Owner: sources track (`p4-cli-commands`).

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;

pub fn main(global: &Global, on: bool) -> ExitCode {
    let _ = global.dry_run;
    let _ = on;
    let _ = writeln!(std::io::stderr(), "wakeshot: 'automatic' is not built yet.");
    ExitCode::from(2)
}
