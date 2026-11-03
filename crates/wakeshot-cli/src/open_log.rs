//! `wakeshot log`: open the log with the default app.
//!
//! Owner: sources track (`p4-cli-commands`).

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;

pub fn main(global: &Global) -> ExitCode {
    let _ = global.dry_run;
    let _ = writeln!(std::io::stderr(), "wakeshot: 'log' is not built yet.");
    ExitCode::from(2)
}
