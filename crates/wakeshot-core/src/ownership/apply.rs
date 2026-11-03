//! The apply (architecture section 5, "Apply", steps 1 to 6).
//!
//! Owner: core track (`p2-ownership-apply`). The signature is contract: the power track's IO thread calls it through
//! `apply_locked`.

use crate::command::ApplyError;
use crate::ownership::io::{PlanIo, SafetyNet};
use crate::settings::{Guid, JournalEntry, JournalOwner, SettingKey};

/// How one apply went, for `Outcome::Applied`. `journal`: the known entries left after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    pub written: Vec<SettingKey>,
    pub result: Result<(), ApplyError>,
    pub journal: Vec<JournalEntry>,
}

/// One apply to `plan` with `writes`, owned by `me`. Entries on `plan` whose key is not in `writes` are restored in
/// the same run (`Command::Apply` doc).
pub fn apply(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    me: JournalOwner,
    plan: Guid,
    writes: &[(SettingKey, u32)],
) -> Applied {
    let _ = (plans, net, me, plan, writes);
    unimplemented!("p2-ownership-apply")
}
