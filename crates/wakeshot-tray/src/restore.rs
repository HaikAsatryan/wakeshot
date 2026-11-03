//! Restore mode: restore at sign-in if a journal exists and its owner is dead (architecture section 4, "Processes").
//!
//! Owner: power track (`p3-recovery`).

use std::ffi::OsString;
use std::process::ExitCode;

/// `args`: what follows `--restore` (nothing today).
pub fn run(args: &[OsString]) -> ExitCode {
    let _ = args;
    ExitCode::SUCCESS
}
