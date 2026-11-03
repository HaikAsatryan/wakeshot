//! The ownership check on each plan read (architecture section 5, "Ownership").
//!
//! Owner: core track (`p2-ownership-apply`). The signature is contract: the power track's IO thread calls it through
//! `check_locked` on each `ReadPlan`.

use crate::command::Failure;
use crate::ownership::io::{PlanIo, SafetyNet};
use crate::settings::{JournalEntry, OwnedKey};

/// How one check went. The caller posts one `UserChanged` per key in `user_changed` and logs `failures`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub user_changed: Vec<OwnedKey>,
    pub failures: Vec<Failure>,
    pub journal: Vec<JournalEntry>,
}

/// One ownership check: a value we wrote that changed is the user's, and its entry is dropped.
pub fn check(plans: &mut impl PlanIo, net: &mut impl SafetyNet) -> Checked {
    let _ = (plans, net);
    unimplemented!("p2-ownership-apply")
}
