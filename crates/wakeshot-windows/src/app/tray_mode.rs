//! The composition root: the real objects, thread starts, startup and shutdown order, `RealRunners`.
//!
//! Owner: `p6-int-tray-mode`, then `p6-int-update-wiring`; fixes by the e2e cards.

use std::process::ExitCode;

/// Runs tray mode, or dry-run mode when `dry_run`, until Quit: tray startup steps 3 to 9
/// (`docs/phases/06-integration.md` note 6). Steps 1 and 2, Velopack and the mode, are the tray `main`'s.
pub fn run(dry_run: bool) -> ExitCode {
    let _ = dry_run;
    // Until `p6-int-tray-mode` fills this in, the tray exits at once, as it did before the integration step.
    ExitCode::SUCCESS
}
