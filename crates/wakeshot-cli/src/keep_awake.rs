//! `wakeshot keep-awake`: Keep awake from a terminal, through the tray's hold file.
//!
//! Owner: sources track (`p4-cli-commands`).

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;
use wakeshot_core::view::KeepAwake;

pub fn main(global: &Global, choice: KeepAwake, screen: bool) -> ExitCode {
    let _ = global.dry_run;
    let _ = (choice, screen);
    let _ = writeln!(
        std::io::stderr(),
        "wakeshot: 'keep-awake' is not built yet."
    );
    ExitCode::from(2)
}
