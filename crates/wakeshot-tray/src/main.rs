//! The tray exe: tray, guard and restore modes (architecture section 4). This dispatch is the contracts card's and
//! stays as it is; each mode's file has its own owner.

// A console exe would flash a window at every login.
#![windows_subsystem = "windows"]

mod guard;
mod mode;
mod restore;
mod velo;

use std::process::ExitCode;

use mode::Mode;

fn main() -> ExitCode {
    // Tray startup step 1: Velopack runs a hook and exits before anything else starts.
    velo::startup();
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match mode::parse(&args) {
        Some(Mode::Guard) => guard::run(&args[1..]),
        Some(Mode::Restore) => restore::run(&args[1..]),
        // The tray loop is the integration step's.
        Some(Mode::Tray | Mode::DryRun) => ExitCode::SUCCESS,
        // A GUI exe has no console to print to, so a bad command line shows only in the exit code.
        None => ExitCode::from(2),
    }
}
