//! What the command line asks for, as `args::parse` gives it.
//!
//! Owner: contracts card. Frozen: the sources track's CLI cards build against it.

use wakeshot_core::view::KeepAwake;

/// Flags that come before any command. `dry_run`: use the dry-run data folder, as `wakeshot-tray --dry-run` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Global {
    pub dry_run: bool,
}

/// One command. `Bad` carries the reason, worded for the user.
#[expect(
    dead_code,
    reason = "args::parse builds only Help and Bad until p4-cli-args; that card removes this attribute"
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Run,
    KeepAwake { choice: KeepAwake, screen: bool },
    Automatic(bool),
    Status,
    Restore,
    Log,
    Help,
    Bad(String),
}
