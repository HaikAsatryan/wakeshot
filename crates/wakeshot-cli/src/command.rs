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
///
/// `Hook`: `wakeshot hook <agent> <event> [--checked <value>...]`, which an agent's hook runs (`docs/agent-hooks.md`,
/// "The hook command"); not in the help. Its form is frozen: every later version takes it, and any words after `hook`
/// give `Hook`, never `Bad` or `Help`, since an exit 2 would block the agent's prompt. `values`: the words after
/// `--checked`, empty without it. A missing word is empty; any other shape gives empty words. An agent no table knows
/// does nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Run,
    KeepAwake {
        choice: KeepAwake,
        screen: bool,
    },
    Automatic(bool),
    Status,
    Restore,
    Log,
    Hook {
        agent: String,
        event: String,
        values: Vec<String>,
    },
    Help,
    Bad(String),
}
