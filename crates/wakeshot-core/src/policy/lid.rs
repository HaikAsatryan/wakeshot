//! The lid-close lock and sound, and a power source change with the lid closed (decision 18).
//!
//! The core calls [`ours`] when the lid closes (`Lock` and `Play(LidClosedHolding)` side by side when true),
//! [`source_change`] at a power source change, [`settle`] at each later event while a [`SourceLock`] waits, and
//! [`user_action`] when holding ends (the countdown rule in `policy::ending`). Each rule reads the last plan read and
//! the core's journal mirror as they are at that event.
//!
//! Owner: `p2-policy-power`.

use crate::settings::{Guid, JournalEntry, PlanValues, SettingKey};
use crate::time::Deadline;

/// The lid action Do nothing, the only lid value Wakeshot writes. The others: 1 Sleep, 2 Hibernate, 3 Shut down
/// (research, "Power setting IDs").
pub const DO_NOTHING: u32 = 0;

/// How long a new power source must last before a closed laptop is locked for it, so a dock's short AC drop does not
/// lock someone who types on an external screen (product, "Never lower security"). The two drops seen on the test
/// laptop lasted 5.3 s and 6.3 s (research, "Power rules checks"), so 5 s was too short; Haik picked a longer wait
/// ("15 or even more ... can be even a minute") and the orchestrator set 30 s (2026-11-03 21:23). The cost: an
/// unplugged, closed laptop stays unlocked up to 30 s.
pub const SOURCE_SETTLE_MS: u64 = 30_000;

/// The lid key for a power source (`ac`: on AC).
pub fn key(ac: bool) -> SettingKey {
    if ac {
        SettingKey::LidAc
    } else {
        SettingKey::LidDc
    }
}

/// The lid action the plan read shows for a power source. A read gives the stored value, which is also the value in
/// force except between a write and its `PowerSetActiveScheme` (research, "Stored at once, applied only on
/// `PowerSetActiveScheme`").
pub fn action(plan: &PlanValues, ac: bool) -> u32 {
    if ac { plan.lid_ac } else { plan.lid_dc }
}

/// The journal entry that makes `live` ours: the same plan and key, and either `live` equals `written` (section 5,
/// "Ownership") or the entry is `restored`. A restored entry has its original stored but maybe not applied (only
/// `PowerSetActiveScheme` was left), so our value may still be in force; counting it errs toward a lock. Change it
/// when the plan read reports the value in force instead of the stored one.
pub fn owned_entry(
    journal: &[JournalEntry],
    plan: Guid,
    key: SettingKey,
    live: u32,
) -> Option<&JournalEntry> {
    journal
        .iter()
        .find(|e| e.plan == plan && e.key == key && (e.restored || e.written == live))
}

/// The user's own value of a key: the journal's `original` while we own it, else the live value. `policy::lock` uses
/// it for the battery sleep timeout.
pub fn user_value(journal: &[JournalEntry], plan: Guid, key: SettingKey, live: u32) -> u32 {
    owned_entry(journal, plan, key, live).map_or(live, |e| e.original)
}

/// The user's own lid action for a power source.
pub fn user_action(plan: &PlanValues, ac: bool, journal: &[JournalEntry]) -> u32 {
    user_value(journal, plan.active, key(ac), action(plan, ac))
}

/// The lid action in force for `ac` is our Do nothing, so a closed laptop runs only because of Wakeshot. At a lid
/// close the core locks and chimes exactly when this is true; a user whose own action is Do nothing gets neither
/// (decision 18). It does not ask whether a hold runs: an entry a failed restore left keeps the laptop running too.
pub fn ours(plan: &PlanValues, ac: bool, journal: &[JournalEntry]) -> bool {
    owned_entry(journal, plan.active, key(ac), action(plan, ac))
        .is_some_and(|e| e.written == DO_NOTHING && e.original != DO_NOTHING)
}

/// A lock and chime that wait for a new power source to last [`SOURCE_SETTLE_MS`].
///
/// Owner: core track (Phase 2) keeps at most one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLock {
    /// The new source: `true` for AC.
    pub ac: bool,
    pub due_mono_ms: u64,
}

impl SourceLock {
    pub fn deadline(self) -> Deadline {
        Deadline::Mono(self.due_mono_ms)
    }
}

/// At a power source change: the lock to wait for, or `None`. The core keeps the answer in place of any earlier one,
/// so a change back inside the settle drops the first. It applies when the lid is closed, the action in force for the
/// new source is ours, and either the user's own action for the old source was Do nothing (that lid close gave no
/// lock) or the change is AC to battery: a user who signed in on an external screen after the lid-close lock and then
/// unplugs the dock would otherwise bag a laptop that runs unlocked.
pub fn source_change(
    plan: &PlanValues,
    journal: &[JournalEntry],
    lid_closed: bool,
    from_ac: bool,
    to_ac: bool,
    now_mono_ms: u64,
) -> Option<SourceLock> {
    if !lid_closed || from_ac == to_ac || !ours(plan, to_ac, journal) {
        return None;
    }
    let users_own_do_nothing_before = user_action(plan, from_ac, journal) == DO_NOTHING;
    let ac_to_battery = from_ac && !to_ac;
    (users_own_do_nothing_before || ac_to_battery).then(|| SourceLock {
        ac: to_ac,
        due_mono_ms: now_mono_ms.saturating_add(SOURCE_SETTLE_MS),
    })
}

/// A waiting [`SourceLock`] at a later event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settle {
    /// Not due yet: check again then.
    Wait(Deadline),
    /// Send `Lock` and `Play(LidClosedHolding)` now, and forget it.
    Lock,
    /// Forget it.
    Drop,
}

/// Lock once due. Drop it when the source changed again, the lid opened, or the action in force is no longer ours (a
/// release or a plan switch during the wait), because then this lock no longer guards a laptop we keep running.
pub fn settle(
    pending: SourceLock,
    plan: &PlanValues,
    journal: &[JournalEntry],
    lid_closed: bool,
    ac: bool,
    now_mono_ms: u64,
) -> Settle {
    if !lid_closed || ac != pending.ac || !ours(plan, ac, journal) {
        Settle::Drop
    } else if now_mono_ms < pending.due_mono_ms {
        Settle::Wait(pending.deadline())
    } else {
        Settle::Lock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingKey::{LidAc, LidDc, SleepDc};

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OTHER: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
    const SLEEP: u32 = 1;
    const ACTIONS: [u32; 4] = [DO_NOTHING, SLEEP, 2, 3];
    const T0: u64 = 7_200_000;

    fn plan(lid_ac: u32, lid_dc: u32) -> PlanValues {
        PlanValues {
            active: BALANCED,
            lid_ac,
            lid_dc,
            sleep_ac: 10_800,
            sleep_dc: 0,
            saver_threshold_dc: 20,
        }
    }

    fn entry(key: SettingKey, original: u32) -> JournalEntry {
        JournalEntry {
            plan: BALANCED,
            key,
            original,
            written: DO_NOTHING,
            restored: false,
        }
    }

    /// Haik's values (Sleep on AC and on battery) after a hold's apply: both lid keys are ours.
    fn held() -> (PlanValues, Vec<JournalEntry>) {
        (
            plan(DO_NOTHING, DO_NOTHING),
            vec![
                entry(LidAc, SLEEP),
                entry(LidDc, SLEEP),
                entry(SleepDc, 600),
            ],
        )
    }

    /// Docked use after a hold's apply: the user's own AC action is Do nothing (no entry), the battery one is ours.
    fn docked() -> (PlanValues, Vec<JournalEntry>) {
        (
            plan(DO_NOTHING, DO_NOTHING),
            vec![entry(LidDc, SLEEP), entry(SleepDc, 600)],
        )
    }

    #[test]
    fn keys_and_actions_follow_the_power_source() {
        assert_eq!(key(true), LidAc);
        assert_eq!(key(false), LidDc);
        assert_eq!(action(&plan(2, 3), true), 2);
        assert_eq!(action(&plan(2, 3), false), 3);
    }

    #[test]
    fn lid_close_on_ac_locks_only_for_our_do_nothing() {
        for own in ACTIONS {
            assert!(!ours(&plan(own, SLEEP), true, &[]), "user's own {own}");
            let journal = [entry(LidAc, own)];
            let locks = ours(&plan(DO_NOTHING, SLEEP), true, &journal);
            assert_eq!(locks, own != DO_NOTHING, "ours over the user's {own}");
        }
    }

    #[test]
    fn lid_close_on_battery_locks_only_for_our_do_nothing() {
        for own in ACTIONS {
            assert!(!ours(&plan(SLEEP, own), false, &[]), "user's own {own}");
            let journal = [entry(LidDc, own)];
            let locks = ours(&plan(SLEEP, DO_NOTHING), false, &journal);
            assert_eq!(locks, own != DO_NOTHING, "ours over the user's {own}");
        }
    }

    #[test]
    fn users_own_do_nothing_gets_no_lock_when_docked() {
        let (p, j) = docked();
        assert!(!ours(&p, true, &j));
        assert_eq!(user_action(&p, true, &j), DO_NOTHING);
        assert!(ours(&p, false, &j));
    }

    #[test]
    fn a_value_the_user_changed_is_not_ours() {
        let journal = [entry(LidAc, SLEEP)];
        let p = plan(3, SLEEP);
        assert!(!ours(&p, true, &journal));
        assert_eq!(owned_entry(&journal, BALANCED, LidAc, 3), None);
        assert_eq!(user_action(&p, true, &journal), 3);
    }

    #[test]
    fn an_entry_on_another_plan_is_not_ours() {
        let journal = [JournalEntry {
            plan: OTHER,
            ..entry(LidAc, SLEEP)
        }];
        assert!(!ours(&plan(DO_NOTHING, SLEEP), true, &journal));
        assert_eq!(
            user_action(&plan(DO_NOTHING, SLEEP), true, &journal),
            DO_NOTHING
        );
    }

    #[test]
    fn a_restored_entry_still_counts_as_ours() {
        let journal = [JournalEntry {
            restored: true,
            ..entry(LidDc, SLEEP)
        }];
        let p = plan(SLEEP, SLEEP);
        assert!(ours(&p, false, &journal));
        assert_eq!(user_action(&p, false, &journal), SLEEP);
    }

    #[test]
    fn user_value_is_the_original_while_ours_else_the_live_value() {
        let journal = [entry(SleepDc, 600)];
        assert_eq!(user_value(&journal, BALANCED, SleepDc, 0), 600);
        assert_eq!(user_value(&journal, BALANCED, SleepDc, 900), 900);
        assert_eq!(user_value(&journal, OTHER, SleepDc, 0), 0);
        assert_eq!(user_value(&[], BALANCED, SleepDc, 0), 0);
    }

    #[test]
    fn ac_to_battery_locks_once_battery_lasted_the_settle() {
        let (p, j) = held();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        let due = T0 + SOURCE_SETTLE_MS;
        assert_eq!(
            pending,
            SourceLock {
                ac: false,
                due_mono_ms: due
            }
        );
        assert_eq!(pending.deadline(), Deadline::Mono(due));
        assert_eq!(
            settle(pending, &p, &j, true, false, due - 1),
            Settle::Wait(Deadline::Mono(due))
        );
        assert_eq!(settle(pending, &p, &j, true, false, due), Settle::Lock);
    }

    #[test]
    fn ac_to_battery_and_back_inside_the_settle_gives_nothing() {
        let (p, j) = held();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        let back = T0 + SOURCE_SETTLE_MS - 1;
        assert_eq!(source_change(&p, &j, true, false, true, back), None);
        assert_eq!(
            settle(pending, &p, &j, true, true, T0 + SOURCE_SETTLE_MS),
            Settle::Drop
        );
    }

    #[test]
    fn ac_to_battery_locks_after_the_users_own_do_nothing_on_ac() {
        let (p, j) = docked();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        assert_eq!(
            settle(pending, &p, &j, true, false, T0 + SOURCE_SETTLE_MS),
            Settle::Lock
        );
    }

    #[test]
    fn battery_to_ac_locks_when_the_users_own_battery_action_was_do_nothing() {
        let p = plan(DO_NOTHING, DO_NOTHING);
        let j = [entry(LidAc, SLEEP)];
        let pending = source_change(&p, &j, true, false, true, T0).expect("a lock waits");
        assert!(pending.ac);
        assert_eq!(
            settle(pending, &p, &j, true, true, T0 + SOURCE_SETTLE_MS),
            Settle::Lock
        );
    }

    #[test]
    fn battery_to_ac_gives_nothing_after_our_lid_close_lock() {
        let (p, j) = held();
        assert_eq!(source_change(&p, &j, true, false, true, T0), None);
    }

    #[test]
    fn battery_to_ac_onto_the_users_own_do_nothing_gives_nothing() {
        let (p, j) = docked();
        assert_eq!(source_change(&p, &j, true, false, true, T0), None);
    }

    #[test]
    fn ac_to_battery_onto_a_released_key_gives_nothing() {
        let p = plan(DO_NOTHING, SLEEP);
        let j = [entry(LidAc, SLEEP)];
        assert_eq!(source_change(&p, &j, true, true, false, T0), None);
    }

    #[test]
    fn a_source_change_with_the_lid_open_gives_nothing() {
        let (p, j) = held();
        assert_eq!(source_change(&p, &j, false, true, false, T0), None);
        assert_eq!(source_change(&p, &j, false, false, true, T0), None);
    }

    #[test]
    fn the_same_source_twice_is_no_change() {
        let (p, j) = docked();
        assert_eq!(source_change(&p, &j, true, false, false, T0), None);
    }

    #[test]
    fn the_lid_opening_in_the_settle_drops_the_lock() {
        let (p, j) = held();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        assert_eq!(
            settle(pending, &p, &j, false, false, T0 + SOURCE_SETTLE_MS),
            Settle::Drop
        );
    }

    #[test]
    fn a_release_in_the_settle_drops_the_lock() {
        let (p, j) = held();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        let released = plan(DO_NOTHING, SLEEP);
        let left = [entry(LidAc, SLEEP), entry(SleepDc, 600)];
        let due = T0 + SOURCE_SETTLE_MS;
        assert_eq!(
            settle(pending, &released, &left, true, false, due),
            Settle::Drop
        );
    }

    #[test]
    fn a_plan_switch_in_the_settle_drops_the_lock() {
        let (p, j) = held();
        let pending = source_change(&p, &j, true, true, false, T0).expect("a lock waits");
        let switched = PlanValues {
            active: OTHER,
            ..plan(SLEEP, SLEEP)
        };
        let due = T0 + SOURCE_SETTLE_MS;
        assert_eq!(
            settle(pending, &switched, &j, true, false, due),
            Settle::Drop
        );
    }
}
