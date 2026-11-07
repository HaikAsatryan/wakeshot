//! The tray exe: tray, guard and restore modes (architecture section 4).

// A console exe would flash a window at every login.
#![windows_subsystem = "windows"]

mod guard;
mod mode;
mod restore;
mod velo;

use std::process::ExitCode;

use mode::Mode;
use wakeshot_windows::app::tray_mode::{self, Updates};

fn main() -> ExitCode {
    // Tray startup step 1: Velopack runs a hook and exits before anything else starts.
    velo::startup();
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match mode::parse(&args) {
        Some(Mode::Guard) => guard::run(&args[1..]),
        Some(Mode::Restore) => restore::run(),
        // The wiring lives in the lib, where it is tested with fakes. Only tray mode updates.
        Some(Mode::Tray) => tray_mode::run(false, velo::updates()),
        // Run from an installed copy, an update would end that copy's real tray and run its hooks.
        Some(Mode::DryRun) => tray_mode::run(true, Updates::default()),
        // A GUI exe has no console to print to, so a bad command line shows only in the exit code.
        None => ExitCode::from(2),
    }
}
