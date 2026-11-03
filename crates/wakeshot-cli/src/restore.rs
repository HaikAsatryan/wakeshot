//! `wakeshot restore`: ask the tray to restore, or restore here when its owner is dead.
//!
//! Owner: sources track (`p4-cli-restore`).

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;

pub fn main(global: &Global) -> ExitCode {
    let _ = global.dry_run;
    let _ = writeln!(std::io::stderr(), "wakeshot: 'restore' is not built yet.");
    ExitCode::from(2)
}
