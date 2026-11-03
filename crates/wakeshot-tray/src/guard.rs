//! Guard mode: wait on the tray and restore if it dies (architecture section 5, "Guard").
//!
//! Owner: power track (`p3-recovery`).

use std::ffi::OsString;
use std::process::ExitCode;

/// `args`: what follows `--guard` (the tray's pid, start time and the stop event).
pub fn run(args: &[OsString]) -> ExitCode {
    let _ = args;
    ExitCode::SUCCESS
}
