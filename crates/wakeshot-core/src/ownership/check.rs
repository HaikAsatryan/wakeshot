//! The ownership check on each plan read (architecture section 5, "Ownership").
//!
//! Owner: core track (`p2-ownership-apply`). The signature is contract: the power track's IO thread calls it through
//! `check_locked` on each `ReadPlan`.

use crate::command::Failure;
use crate::ownership::apply::JOURNAL_CORRUPT;
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::ownership::restore::save_or_clean_up;
use crate::settings::{Journal, JournalEntry, OwnedKey};

/// How one check went. The caller posts one `UserChanged` per key in `user_changed` and logs `failures`.
///
/// - `user_changed`: each entry whose live value, read from its own plan, was not `written` (nor, for a marked entry,
///   `original`). It is reported even when the save after the drop failed (then in `failures` too): its live value is
///   the user's, so every later routine drops the entry left in the file again, and the cost is at most a repeated
///   notice, while not reporting it would let the next apply override the user's change.
/// - `journal`: the known entries the check kept, in every plan; empty when the file is missing, corrupt or
///   unreadable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub user_changed: Vec<OwnedKey>,
    pub failures: Vec<Failure>,
    pub journal: Vec<JournalEntry>,
}

/// One ownership check: a value we wrote that changed is the user's, and its entry is dropped.
///
/// Call order: `load_journal`; with a known entry, `active_plan`, then `read` per known entry from its own plan; after
/// a drop, one save, or the clean-up of restore step 6 when no known entry is left. A corrupt file is reported as
/// `JOURNAL_CORRUPT` and left for the restore routine to set aside.
pub fn check(plans: &mut impl PlanIo, net: &mut impl SafetyNet) -> Checked {
    let mut checked = Checked {
        user_changed: Vec::new(),
        failures: Vec::new(),
        journal: Vec::new(),
    };
    let JournalParts {
        known: Journal { owner, entries },
        other,
    } = match net.load_journal() {
        JournalRead::Missing => return checked,
        JournalRead::Corrupt => {
            checked.failures.push(JOURNAL_CORRUPT);
            return checked;
        }
        JournalRead::Failed(failure) => {
            checked.failures.push(failure);
            return checked;
        }
        JournalRead::Read(file) => JournalParts::split(file),
    };
    if entries.is_empty() {
        return checked;
    }
    // The design reads the active plan id first; when even that fails, nothing is read or dropped.
    if let Err(failure) = plans.active_plan() {
        checked.failures.push(failure);
        checked.journal = entries;
        return checked;
    }
    for e in entries {
        // A marked entry with `original` stored waits for a restore's call, ours maybe still in force, or for an
        // apply's write (power safety review finding 9): ours too. Any other value is the user's, as restore step 3
        // reads it, so a mark no longer hides it until a restore drops it with no notice (finding 10).
        let ours = match plans.read(e.plan, e.key) {
            Ok(live) => live == e.written || (e.restored && live == e.original),
            // Not the user's change (power review 2): kept, and read again at the next check.
            Err(failure) => {
                checked.failures.push(failure);
                true
            }
        };
        match ours {
            true => checked.journal.push(e),
            false => checked.user_changed.push(OwnedKey {
                plan: e.plan,
                key: e.key,
            }),
        }
    }
    if !checked.user_changed.is_empty() {
        let parts = JournalParts {
            known: Journal {
                owner,
                entries: checked.journal.clone(),
            },
            other,
        };
        if let Err(failure) = save_or_clean_up(net, &parts) {
            checked.failures.push(failure);
        }
    }
    checked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{JournalFileEntry, JournalKey};
    use crate::ownership::apply::{JOURNAL_CORRUPT, apply};
    use crate::ownership::fake::{
        Action, Call, FAKE_MISSING, FakeGuard, FakeJournal, FakePlans, SetBy,
    };
    use crate::ownership::journal::JournalParts;
    use crate::ownership::restore::restore;
    use crate::process::FileTime;
    use crate::settings::{Guid, Journal, JournalOwner, PlanValues, SettingKey};

    use SettingKey::{LidAc, LidDc, SleepDc};

    /// Balanced, the active plan at the start.
    const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    /// High performance, inactive at the start.
    const P2: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
    const TRAY: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(5),
        session: 1,
    };

    fn user_values(plan: Guid) -> PlanValues {
        PlanValues {
            active: plan,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 600,
            saver_threshold_dc: 20,
        }
    }

    fn entry(plan: Guid, key: SettingKey, restored: bool) -> JournalEntry {
        JournalEntry {
            plan,
            key,
            original: if key == SleepDc { 600 } else { 1 },
            written: 0,
            restored,
        }
    }

    fn owned(plan: Guid, key: SettingKey) -> OwnedKey {
        OwnedKey { plan, key }
    }

    /// An entry a newer version wrote.
    fn sleep_ac() -> JournalFileEntry {
        JournalFileEntry {
            plan: P1,
            key: JournalKey::Other("sleepAc".into()),
            original: 10_800,
            written: 0,
            restored: false,
        }
    }

    fn file(entries: Vec<JournalEntry>, other: Vec<JournalFileEntry>) -> FakeJournal {
        let parts = JournalParts {
            known: Journal {
                owner: TRAY,
                entries,
            },
            other,
        };
        FakeJournal::File(parts.merge())
    }

    /// P1 active and P2, both with user values, then what an apply left for each entry: an unmarked one holds our 0,
    /// stored and in force; a marked one holds `original` stored and our 0 in force (a restore whose call failed).
    /// Also the journal with `other`, the RunOnce value and a running guard.
    fn world(entries: &[JournalEntry], other: Vec<JournalFileEntry>) -> FakePlans {
        let fake = FakePlans::new(user_values(P1));
        fake.add_plan(user_values(P2));
        let mut s = fake.state();
        for e in entries {
            let stored = match e.restored {
                true => (e.original, SetBy::Restore),
                false => (e.written, SetBy::Override),
            };
            s.set_stored(e.plan, e.key, stored.0, stored.1);
            if e.plan == s.active {
                s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
            }
        }
        s.journal = file(entries.to_vec(), other);
        s.run_once = true;
        s.guard = FakeGuard::Running;
        drop(s);
        fake
    }

    fn run(fake: &FakePlans) -> Checked {
        check(&mut fake.clone(), &mut fake.clone())
    }

    fn calls(fake: &FakePlans) -> Vec<Call> {
        fake.state().calls.clone()
    }

    fn quiet(journal: Vec<JournalEntry>) -> Checked {
        Checked {
            user_changed: vec![],
            failures: vec![],
            journal,
        }
    }

    #[test]
    fn a_changed_value_is_dropped_and_reported() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, false), entry(P1, LidDc, false));
        let fake = world(&[lid_ac, lid_dc], vec![]);
        fake.state().user_write(P1, LidDc, 3);
        let r = run(&fake);
        let changed = Checked {
            user_changed: vec![owned(P1, LidDc)],
            failures: vec![],
            journal: vec![lid_ac],
        };
        assert_eq!(r, changed);
        let FakeJournal::File(saved) = file(vec![lid_ac], vec![]) else {
            unreachable!()
        };
        assert_eq!(
            calls(&fake),
            [
                Call::LoadJournal,
                Call::ActivePlan,
                Call::Read(P1, LidAc),
                Call::Read(P1, LidDc),
                Call::SaveJournal(saved),
            ]
        );
        let s = fake.state();
        assert!(s.run_once);
        assert_eq!(s.guard, FakeGuard::Running);
    }

    /// Power review 2: a failed read is not the user's change. The entry stays, so nothing runs the clean-up.
    #[test]
    fn a_read_error_keeps_the_entry_and_reports_the_failure() {
        let e = entry(P1, LidDc, false);
        let fake = world(&[e], vec![]);
        fake.script(2, Action::Fail(31));
        let r = run(&fake);
        let failure = Failure {
            call: "FakePlans::read",
            code: 31,
        };
        let kept = Checked {
            user_changed: vec![],
            failures: vec![failure],
            journal: vec![e],
        };
        assert_eq!(r, kept);
        assert_eq!(
            calls(&fake),
            [Call::LoadJournal, Call::ActivePlan, Call::Read(P1, LidDc)]
        );
        let s = fake.state();
        assert_eq!(s.journal, file(vec![e], vec![]));
        assert!(s.run_once);
    }

    #[test]
    fn a_deleted_plans_read_fails_and_keeps_the_entry() {
        let e = entry(P2, SleepDc, false);
        let fake = world(&[e], vec![]);
        fake.state().delete_plan(P2);
        let r = run(&fake);
        assert_eq!(r.user_changed, []);
        assert_eq!(
            r.failures.iter().map(|f| f.code).collect::<Vec<_>>(),
            [FAKE_MISSING]
        );
        assert_eq!(r.journal, [e]);
        assert_eq!(fake.state().journal, file(vec![e], vec![]));
    }

    /// A marked entry waits for a restore retry (its stored value is `original` while ours may be in force) or for an
    /// apply's write (a new entry, power safety review finding 9): `original` and `written` are both ours.
    #[test]
    fn a_marked_entry_at_original_or_written_is_kept() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, true), entry(P1, LidDc, false));
        let fake = world(&[lid_ac, lid_dc], vec![]);
        assert_eq!(run(&fake), quiet(vec![lid_ac, lid_dc]));

        let fake = world(&[lid_ac], vec![]);
        fake.state().set_stored(P1, LidAc, 0, SetBy::Override);
        assert_eq!(run(&fake), quiet(vec![lid_ac]));
        assert!(
            !calls(&fake)
                .iter()
                .any(|c| matches!(c, Call::SaveJournal(_)))
        );
    }

    /// Power safety review finding 10: a marked entry whose live value is neither `written` nor `original` is the
    /// user's change, as restore step 3 reads it, so the check reports it and the core releases the key; before, the
    /// mark hid it until a restore dropped it with no notice, and an apply in between took the user's value for a new
    /// `original` and wrote over it.
    #[test]
    fn a_marked_entry_the_user_changed_is_reported() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, true), entry(P1, LidDc, true));
        let fake = world(&[lid_ac, lid_dc], vec![]);
        fake.state().user_write(P1, LidAc, 3);
        let changed = Checked {
            user_changed: vec![owned(P1, LidAc)],
            failures: vec![],
            journal: vec![lid_dc],
        };
        assert_eq!(run(&fake), changed);
        assert_eq!(fake.state().journal, file(vec![lid_dc], vec![]));
    }

    /// The mark is what makes `original` ours: an entry not marked whose value the user set back to exactly its
    /// `original` (Sleep again, say) is the user's change.
    #[test]
    fn a_value_set_back_to_original_is_the_users_change() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, false), entry(P1, LidDc, false));
        let fake = world(&[lid_ac, lid_dc], vec![]);
        fake.state().user_write(P1, LidAc, 1);
        let r = run(&fake);
        assert_eq!(r.user_changed, [owned(P1, LidAc)]);
        assert_eq!(r.journal, [lid_dc]);
    }

    #[test]
    fn the_last_drop_runs_the_clean_up_in_its_order() {
        let e = entry(P1, LidDc, false);
        let fake = world(&[e], vec![]);
        fake.state().user_write(P1, LidDc, 3);
        let r = run(&fake);
        assert_eq!(r.user_changed, [owned(P1, LidDc)]);
        assert_eq!(r.journal, []);
        let c = calls(&fake);
        let clean_up = [Call::DeleteJournal, Call::ClearRunOnce, Call::StopGuard];
        assert_eq!(c[c.len() - 3..], clean_up);
        let s = fake.state();
        assert_eq!(s.journal, FakeJournal::Missing);
        assert!(!s.run_once);
        assert_eq!(s.guard, FakeGuard::Stopping);
    }

    /// Track note 2: a newer version's entry is never read, and with no known entry left the file keeps only it.
    #[test]
    fn unknown_keys_are_kept_and_never_read() {
        let e = entry(P1, LidDc, false);
        let fake = world(&[e], vec![sleep_ac()]);
        fake.state().user_write(P1, LidDc, 3);
        let r = run(&fake);
        assert_eq!(r.user_changed, [owned(P1, LidDc)]);
        let c = calls(&fake);
        let FakeJournal::File(only_unknown) = file(vec![], vec![sleep_ac()]) else {
            unreachable!()
        };
        assert_eq!(
            c[c.len() - 3..],
            [
                Call::SaveJournal(only_unknown),
                Call::ClearRunOnce,
                Call::StopGuard
            ]
        );
        assert!(!c.contains(&Call::DeleteJournal));
        assert_eq!(fake.state().journal, file(vec![], vec![sleep_ac()]));
    }

    /// Section 5, accepted: our value set again by the user reads as ours, so a restore puts back the old one.
    #[test]
    fn the_user_setting_exactly_our_value_is_not_seen() {
        let e = entry(P1, LidDc, false);
        let fake = world(&[e], vec![]);
        fake.state().user_write(P1, LidDc, 0);
        assert_eq!(run(&fake), quiet(vec![e]));

        let restored = restore(&mut fake.clone(), &mut fake.clone(), None, None);
        assert_eq!(restored.result, Ok(()));
        assert_eq!(fake.state().in_force(P1, LidDc), Some(1));
    }

    /// Power review 1: the check runs on the IO thread in command order, so one taken before an apply finds none of
    /// that apply's entries, though the live values differ from what the apply is about to write.
    #[test]
    fn a_check_before_an_apply_never_reports_that_applys_entries() {
        let fake = FakePlans::new(user_values(P1));
        assert_eq!(run(&fake), quiet(vec![]));
        assert_eq!(calls(&fake), [Call::LoadJournal]);

        let writes = [(LidAc, 0), (LidDc, 0), (SleepDc, 0)];
        let applied = apply(&mut fake.clone(), &mut fake.clone(), TRAY, P1, &writes);
        assert_eq!(applied.result, Ok(()));
        let r = run(&fake);
        assert_eq!(r, quiet(applied.journal));
    }

    /// Each entry is read from its own plan, so a switch to a plan with other values is no user change.
    #[test]
    fn a_plan_switch_never_looks_like_a_user_change() {
        let entries = [entry(P1, LidAc, false), entry(P1, SleepDc, false)];
        let fake = world(&entries, vec![]);
        fake.state().switch_plan(P2);
        assert_eq!(run(&fake), quiet(entries.to_vec()));
        let c = calls(&fake);
        assert!(c.contains(&Call::Read(P1, SleepDc)), "{c:?}");
        assert!(!c.iter().any(|c| matches!(c, Call::Read(p, _) if *p == P2)));
    }

    #[test]
    fn a_failed_active_plan_read_drops_nothing() {
        let e = entry(P1, LidDc, false);
        let fake = world(&[e], vec![]);
        fake.state().user_write(P1, LidDc, 3);
        fake.script(1, Action::Fail(31));
        let r = run(&fake);
        assert_eq!(r.user_changed, []);
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.journal, [e]);
        assert_eq!(calls(&fake), [Call::LoadJournal, Call::ActivePlan]);
    }

    #[test]
    fn an_unreadable_or_corrupt_journal_is_reported_and_left() {
        let fake = FakePlans::new(user_values(P1));
        fake.state().journal = FakeJournal::Unreadable(32);
        let r = run(&fake);
        assert_eq!(r.failures.iter().map(|f| f.code).collect::<Vec<_>>(), [32]);
        assert_eq!((r.user_changed, r.journal), (vec![], vec![]));

        fake.state().journal = FakeJournal::Corrupt;
        let r = run(&fake);
        assert_eq!(r.failures, [JOURNAL_CORRUPT]);
        assert_eq!(calls(&fake), [Call::LoadJournal, Call::LoadJournal]);
        let s = fake.state();
        assert_eq!((s.journal.clone(), s.set_aside), (FakeJournal::Corrupt, 0));
    }

    /// The user's change is reported even when the save fails: its live value is the user's, so the entry left in
    /// the file is dropped again by every later routine (a repeated notice at worst), and the core stops overriding.
    #[test]
    fn a_failed_save_still_reports_the_users_change() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, false), entry(P1, LidDc, false));
        let fake = world(&[lid_ac, lid_dc], vec![]);
        fake.state().user_write(P1, LidDc, 3);
        fake.script(4, Action::Fail(112));
        let r = run(&fake);
        assert_eq!(r.user_changed, [owned(P1, LidDc)]);
        assert_eq!(r.failures.iter().map(|f| f.code).collect::<Vec<_>>(), [112]);
        assert_eq!(r.journal, [lid_ac]);
        assert_eq!(fake.state().journal, file(vec![lid_ac, lid_dc], vec![]));

        assert_eq!(run(&fake).user_changed, [owned(P1, LidDc)], "seen again");
        let restored = restore(&mut fake.clone(), &mut fake.clone(), None, None);
        assert_eq!(restored.result, Ok(()));
        let s = fake.state();
        assert_eq!(
            (s.in_force(P1, LidAc), s.in_force(P1, LidDc)),
            (Some(1), Some(3))
        );
    }
}
