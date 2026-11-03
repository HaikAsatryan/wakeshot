//! The journal file, the RunOnce value and the guard as the core's `SafetyNet` (`FileNet`, real and dry run), and the
//! locked entry points every caller of the ownership routines goes through.
//!
//! Owner: power track (`p3-journal`). The three signatures below are contract: the IO thread, the guard, restore mode,
//! the CLI (`wakeshot restore`) and the Velopack hooks call them, never `wakeshot_core::ownership` directly.

use wakeshot_core::ownership::apply::Applied;
use wakeshot_core::ownership::check::Checked;
use wakeshot_core::ownership::io::{PlanIo, SafetyNet};
use wakeshot_core::ownership::restore::Restored;
use wakeshot_core::settings::{Guid, JournalOwner, SettingKey};

use crate::log::Log;
use crate::store::Names;

/// `ownership::restore` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back as a failed
/// `result`, and nothing was done.
pub fn restore_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    plan: Option<Guid>,
    expect_owner: Option<JournalOwner>,
    names: &Names,
    log: &dyn Log,
) -> Restored {
    let _ = (plans, net, plan, expect_owner, names, log);
    unimplemented!("p3-journal")
}

/// `ownership::apply` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back as
/// `ApplyError::SafetyNet`, and nothing was written.
pub fn apply_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    me: JournalOwner,
    plan: Guid,
    writes: &[(SettingKey, u32)],
    names: &Names,
    log: &dyn Log,
) -> Applied {
    let _ = (plans, net, me, plan, writes, names, log);
    unimplemented!("p3-journal")
}

/// `ownership::check` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back in `failures`, and
/// no entry was dropped.
pub fn check_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    names: &Names,
    log: &dyn Log,
) -> Checked {
    let _ = (plans, net, names, log);
    unimplemented!("p3-journal")
}
