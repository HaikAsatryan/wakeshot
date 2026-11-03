//! The restore routine (architecture section 5, "Restore routine", steps 1 to 7).
//!
//! Owner: core track (`p2-ownership-restore`). The signature is contract: the power track's IO thread, guard and
//! restore mode, the CLI's `wakeshot restore` and the Velopack hooks call it through `restore_locked`.

use crate::command::Failure;
use crate::ownership::io::{PlanIo, SafetyNet};
use crate::settings::{Guid, JournalEntry, JournalOwner};

/// What the routine found in `journal.json`. The caller maps `SetAside` to `Event::JournalCorrupt` and `OtherOwner`
/// to "another tray took over": the routine wrote nothing then.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    Nothing,
    Entries,
    SetAside,
    OtherOwner(JournalOwner),
}

/// How one run of the routine went. `journal`: the known entries left after it, for `Outcome::Restored`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restored {
    pub found: Found,
    pub result: Result<(), Failure>,
    pub journal: Vec<JournalEntry>,
}

/// One run of the restore routine. `plan`: `None` takes every entry, `Some` only that plan's. `expect_owner`: the
/// guard and restore mode pass the owner they checked, so a journal another tray took over is left alone.
pub fn restore(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    plan: Option<Guid>,
    expect_owner: Option<JournalOwner>,
) -> Restored {
    let _ = (plans, net, plan, expect_owner);
    unimplemented!("p2-ownership-restore")
}
