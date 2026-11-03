//! The tray exe: tray, guard and restore modes (architecture section 4). Each mode's file has its own owner.
//!
//! Owner: the integration cards from now on (`p6-int-skeleton`, then `p6-int-update-wiring`); first version from the
//! contracts card.

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
        // The wiring lives in the lib, where it tests with the fakes; the integration cards own this arm from now on
        // (`p6-int-skeleton`, then `p6-int-update-wiring`).
        Some(m @ (Mode::Tray | Mode::DryRun)) => {
            wakeshot_windows::app::tray_mode::run(matches!(m, Mode::DryRun))
        }
        // A GUI exe has no console to print to, so a bad command line shows only in the exit code.
        None => ExitCode::from(2),
    }
}
