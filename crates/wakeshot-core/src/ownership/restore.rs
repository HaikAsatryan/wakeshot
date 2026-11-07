//! The restore routine (architecture section 5, "Restore routine", steps 1 to 7). The tray's IO thread, the guard,
//! restore mode, `wakeshot restore` and the Velopack hooks call it through `restore_locked` in `wakeshot-windows`.

use crate::command::Failure;
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::settings::{Guid, Journal, JournalEntry, JournalOwner};

/// What the routine found in `journal.json`. The caller maps `SetAside` to `Event::JournalCorrupt` and `OtherOwner`
/// to "another tray took over": the routine wrote nothing then.
///
/// - `Nothing`: no file, or one with no known entry (the clean-up ran); also a load that failed, with `result: Err`.
/// - `Entries`: a file with at least one known entry, in the scope or not.
/// - `SetAside`: the file did not parse. `result: Ok`: it is now `journal.json.corrupt`; `Err`: the move failed and
///   the file is still in place, so post `Event::JournalCorrupt` only for `Ok`.
/// - `OtherOwner`: the file's owner is not `expect_owner`; `result` is `Ok`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    Nothing,
    Entries,
    SetAside,
    OtherOwner(JournalOwner),
}

/// How one run of the routine went. `journal`: the known entries left after it, for `Outcome::Restored`.
///
/// `journal` is what the file holds after the run, in every plan (empty when the file is missing, set aside or could
/// not be read). `result` is the first failure; a failed run kept every entry it could not finish.
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
    let empty = |found, result| Restored {
        found,
        result,
        journal: Vec::new(),
    };
    // Step 1.
    let parts = match net.load_journal() {
        JournalRead::Missing => return empty(Found::Nothing, Ok(())),
        JournalRead::Failed(failure) => return empty(Found::Nothing, Err(failure)),
        JournalRead::Corrupt => return empty(Found::SetAside, net.set_aside_journal()),
        JournalRead::Read(file) => JournalParts::split(file),
    };
    let owner = parts.known.owner;
    if expect_owner.is_some_and(|expected| expected != owner) {
        return Restored {
            found: Found::OtherOwner(owner),
            result: Ok(()),
            journal: parts.known.entries,
        };
    }
    if parts.known.entries.is_empty() {
        // A stop inside an earlier clean-up, or a file only a newer version wrote to. A file with unknown entries
        // already holds exactly them, so it is left as it is rather than rewritten.
        let result = match parts.other.is_empty() {
            true => save_or_clean_up(net, &parts),
            false => release(net),
        };
        return empty(Found::Nothing, result);
    }
    restore_entries(plans, net, parts, plan)
}

/// What a run does next with one known entry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Next {
    /// Out of scope, or a call for it failed: it stays as it is.
    Keep,
    /// Live equals `written`: mark it, then write `original` (step 4).
    Write,
    /// Marked with `original` stored: it waits for `set_active` (step 5).
    Apply,
    Drop,
}

struct Row {
    entry: JournalEntry,
    next: Next,
}

/// Steps 2 to 6 for a journal with at least one known entry.
fn restore_entries(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    parts: JournalParts,
    scope: Option<Guid>,
) -> Restored {
    let JournalParts {
        known: Journal { owner, entries },
        other,
    } = parts;
    let parts_of = |rows: &[Row]| JournalParts {
        known: Journal {
            owner,
            entries: kept(rows),
        },
        other: other.clone(),
    };
    let done = |result, journal| Restored {
        found: Found::Entries,
        result,
        journal,
    };
    let in_scope = |e: &JournalEntry| scope.is_none_or(|p| e.plan == p);
    let mut rows: Vec<Row> = entries
        .into_iter()
        .map(|entry| Row {
            entry,
            next: Next::Keep,
        })
        .collect();
    let mut on_disk = kept(&rows);
    if !rows.iter().any(|r| in_scope(&r.entry)) {
        return done(Ok(()), on_disk);
    }

    // Step 2. Both reads must work before anything is dropped or written, or a failed list reads as "every plan
    // deleted". The id itself is read again in step 5, right before the call.
    let listed = match plans.active_plan().and_then(|_| plans.plans()) {
        Ok(listed) => listed,
        Err(failure) => return done(Err(failure), on_disk),
    };

    // Step 3.
    let mut failure = None;
    for row in rows.iter_mut().filter(|r| in_scope(&r.entry)) {
        let e = row.entry;
        row.next = match listed.contains(&e.plan) {
            false => Next::Drop,
            true => match plans.read(e.plan, e.key) {
                Err(f) => {
                    failure.get_or_insert(f);
                    Next::Keep
                }
                Ok(live) if live == e.written => Next::Write,
                Ok(live) if e.restored && live == e.original => Next::Apply,
                Ok(_) => Next::Drop,
            },
        };
    }

    // Step 4: the marks reach the disk before any `original` is written, so a stop between the two is seen as
    // "marked, live equals `written`" and written again.
    if rows.iter().any(|r| r.next == Next::Write) {
        for row in rows.iter_mut().filter(|r| r.next == Next::Write) {
            row.entry.restored = true;
        }
        if let Err(f) = net.save_journal(&parts_of(&rows).merge()) {
            return done(Err(f), on_disk);
        }
        on_disk = kept(&rows);
        for row in rows.iter_mut().filter(|r| r.next == Next::Write) {
            let e = row.entry;
            row.next = match plans.write(e.plan, e.key, e.original) {
                Ok(()) => Next::Apply,
                Err(f) => {
                    failure.get_or_insert(f);
                    Next::Keep
                }
            };
        }
    }

    // Step 5. "Inactive" is judged by this second read, not step 2's: a switch to another plan before its `original`
    // was written brought our value into force there, and only `set_active` replaces it.
    if rows.iter().any(|r| r.next == Next::Apply) {
        let applied = plans.active_plan().and_then(|active| {
            let inactive = |r: &&mut Row| r.next == Next::Apply && r.entry.plan != active;
            for row in rows.iter_mut().filter(inactive) {
                row.next = Next::Drop;
            }
            match rows.iter().any(|r| r.next == Next::Apply) {
                true => plans.set_active(active),
                false => Ok(()),
            }
        });
        let next = match applied {
            Ok(()) => Next::Drop,
            Err(f) => {
                failure.get_or_insert(f);
                Next::Keep
            }
        };
        for row in rows.iter_mut().filter(|r| r.next == Next::Apply) {
            row.next = next;
        }
    }

    // Step 6. Drops are saved even in a failed run: each came from a successful read or call.
    let mut result = failure.map_or(Ok(()), Err);
    let left = kept(&rows);
    if left != on_disk {
        match write_file(net, &parts_of(&rows)) {
            Err(f) => result = result.and(Err(f)),
            Ok(()) => {
                on_disk = left;
                if on_disk.is_empty() {
                    result = result.and(release(net));
                }
            }
        }
    }
    done(result, on_disk)
}

fn kept(rows: &[Row]) -> Vec<JournalEntry> {
    rows.iter()
        .filter(|r| r.next != Next::Drop)
        .map(|r| r.entry)
        .collect()
}

/// The file step of a save: deleted when it would hold no entry at all, else saved with the unknown entries.
fn write_file(net: &mut impl SafetyNet, parts: &JournalParts) -> Result<(), Failure> {
    match parts.known.entries.is_empty() && parts.other.is_empty() {
        true => net.delete_journal(),
        false => net.save_journal(&parts.merge()),
    }
}

/// The rest of the clean-up, after the file: a stop between the two then leaves at most a RunOnce value or a guard
/// with no journal, and both find nothing to do.
fn release(net: &mut impl SafetyNet) -> Result<(), Failure> {
    net.clear_run_once()?;
    net.stop_guard()
}

/// Saves the journal after an entry was dropped; when no known entry is left, runs the clean-up of step 6 instead:
/// the file first (deleted, or kept with only the entries a newer version wrote), then the RunOnce value, then the
/// guard's stop event, so a stop never leaves a known entry without its RunOnce value. The ownership check uses it
/// too (architecture section 5, "Ownership").
pub(crate) fn save_or_clean_up(
    net: &mut impl SafetyNet,
    parts: &JournalParts,
) -> Result<(), Failure> {
    write_file(net, parts)?;
    match parts.known.entries.is_empty() {
        true => release(net),
        false => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::files::{JournalFile, JournalFileEntry, JournalKey};
    use crate::ownership::fake::{Action, Call, FakeGuard, FakeJournal, FakePlans, SetBy};
    use crate::ownership::journal::JournalParts;
    use crate::process::FileTime;
    use crate::settings::{Journal, OwnedKey, PlanValues, SettingKey};

    use SettingKey::{LidAc, LidDc, SleepDc};

    /// Balanced, the active plan at the start.
    const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    /// High performance, inactive at the start.
    const P2: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
    const KEYS: [SettingKey; 3] = [LidAc, LidDc, SleepDc];
    const TRAY: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(5),
        session: 1,
    };
    const NEW_TRAY: JournalOwner = JournalOwner {
        pid: 4242,
        proc_start: FileTime(7),
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

    fn original(key: SettingKey) -> u32 {
        if key == SleepDc { 600 } else { 1 }
    }

    fn entry(plan: Guid, key: SettingKey, restored: bool) -> JournalEntry {
        JournalEntry {
            plan,
            key,
            original: original(key),
            written: 0,
            restored,
        }
    }

    fn marked(e: JournalEntry) -> JournalEntry {
        JournalEntry {
            restored: true,
            ..e
        }
    }

    /// How an entry's value stands when the routine starts.
    #[derive(Clone, Copy)]
    enum Live {
        /// Our value, stored and (on the active plan) in force: an apply finished.
        Ours,
        /// `original` stored again, ours still in force: a restore stopped or failed before `set_active`.
        Unapplied,
        /// The user's value never left: an apply stopped before its write.
        Never,
        /// The user set this value since.
        User(u32),
    }

    /// An entry a newer version wrote.
    fn newer_entry() -> JournalFileEntry {
        JournalFileEntry {
            plan: P1,
            key: JournalKey::Other("hibernateAc".into()),
            original: 10_800,
            written: 0,
            restored: false,
        }
    }

    fn file(entries: Vec<JournalEntry>, other: Vec<JournalFileEntry>) -> JournalFile {
        JournalParts {
            known: Journal {
                owner: TRAY,
                entries,
            },
            other,
        }
        .merge()
    }

    /// Two plans with the user's values, P1 active, and what an apply owned by `TRAY` left: the values in `rows`, a
    /// journal with their entries and `other`, the RunOnce value, a running guard.
    fn world(rows: &[(JournalEntry, Live)], other: Vec<JournalFileEntry>) -> FakePlans {
        let fake = FakePlans::new(user_values(P1));
        fake.add_plan(user_values(P2));
        let mut s = fake.state();
        for &(e, live) in rows {
            let active = e.plan == s.active;
            match live {
                Live::Ours => {
                    s.set_stored(e.plan, e.key, e.written, SetBy::Override);
                    if active {
                        s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
                    }
                }
                Live::Unapplied => {
                    s.set_stored(e.plan, e.key, e.original, SetBy::Restore);
                    if active {
                        s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
                    }
                }
                Live::Never => {}
                Live::User(value) => s.user_write(e.plan, e.key, value),
            }
        }
        s.journal = FakeJournal::File(file(rows.iter().map(|r| r.0).collect(), other));
        s.run_once = true;
        s.guard = FakeGuard::Running;
        drop(s);
        fake
    }

    fn ours(entries: &[JournalEntry]) -> Vec<(JournalEntry, Live)> {
        entries.iter().map(|&e| (e, Live::Ours)).collect()
    }

    fn run(fake: &FakePlans, plan: Option<Guid>) -> Restored {
        restore(&mut fake.clone(), &mut fake.clone(), plan, None)
    }

    /// Runs the routine with a stop at call `index`, as a crash would end it.
    fn run_stopped(fake: &FakePlans, index: usize) {
        fake.script(index, Action::Stop);
        let run = catch_unwind(AssertUnwindSafe(|| run(fake, None)));
        assert!(run.is_err(), "the run did not reach call {index}");
    }

    fn calls(fake: &FakePlans) -> Vec<Call> {
        fake.state().calls.clone()
    }

    fn ok(journal: Vec<JournalEntry>) -> Restored {
        Restored {
            found: Found::Entries,
            result: Ok(()),
            journal,
        }
    }

    /// What every stop and failure keeps: each value we still hold has a known entry, and while one exists the
    /// RunOnce value is set and the guard runs.
    fn assert_reachable(fake: &FakePlans, case: &str) {
        let s = fake.state();
        assert_eq!(s.held_without_entry(), Vec::<OwnedKey>::new(), "{case}");
        if !s.known_entries().is_empty() {
            assert!(s.run_once, "{case}: a known entry without RunOnce");
            assert_eq!(s.guard, FakeGuard::Running, "{case}");
        }
    }

    /// The user's values are back on every plan, stored and in force, and no known entry is left.
    fn assert_values_back(fake: &FakePlans, case: &str) {
        let s = fake.state();
        for &plan in s.plans.keys() {
            for key in KEYS {
                let stored = s.stored(plan, key);
                assert_eq!(stored, Some(original(key)), "{case}: {plan:?} {key:?}");
                if plan == s.active {
                    let in_force = s.in_force(plan, key);
                    assert_eq!(in_force, Some(original(key)), "{case}: {key:?} in force");
                }
            }
        }
        assert_eq!(s.held(), Vec::<OwnedKey>::new(), "{case}");
        let left = s.known_entries();
        assert!(left.is_empty(), "{case}: {left:?}");
    }

    /// As `assert_values_back`, and the safety net is released: no file, no RunOnce value, the guard told to stop.
    fn assert_restored(fake: &FakePlans) {
        assert_values_back(fake, "restored");
        let s = fake.state();
        assert_eq!(s.journal, FakeJournal::Missing);
        assert!(!s.run_once);
        assert_eq!(s.guard, FakeGuard::Stopping);
    }

    fn count(calls: &[Call], call: &Call) -> usize {
        calls.iter().filter(|c| *c == call).count()
    }

    #[test]
    fn missing_journal_does_nothing() {
        let fake = FakePlans::new(user_values(P1));
        let r = run(&fake, None);
        let nothing = Restored {
            found: Found::Nothing,
            result: Ok(()),
            journal: vec![],
        };
        assert_eq!(r, nothing);
        assert_eq!(calls(&fake), [Call::LoadJournal]);
    }

    #[test]
    fn unreadable_journal_fails_and_touches_nothing() {
        let fake = world(&ours(&[entry(P1, LidDc, false)]), vec![]);
        fake.state().journal = FakeJournal::Unreadable(32);
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.code), Err(32));
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        let s = fake.state();
        assert_eq!(s.journal, FakeJournal::Unreadable(32));
        assert_eq!(s.stored(P1, LidDc), Some(0));
        assert!(s.run_once);
    }

    #[test]
    fn no_known_entries_runs_the_clean_up() {
        let fake = world(&[], vec![]);
        let r = run(&fake, None);
        assert_eq!(r.found, Found::Nothing);
        assert_eq!(r.result, Ok(()));
        assert_eq!(
            calls(&fake),
            [
                Call::LoadJournal,
                Call::DeleteJournal,
                Call::ClearRunOnce,
                Call::StopGuard
            ]
        );
        assert_restored(&fake);
    }

    #[test]
    fn corrupt_journal_is_set_aside_and_values_stay() {
        let fake = world(&ours(&[entry(P1, LidDc, false)]), vec![]);
        fake.state().journal = FakeJournal::Corrupt;
        let r = run(&fake, None);
        let set_aside = Restored {
            found: Found::SetAside,
            result: Ok(()),
            journal: vec![],
        };
        assert_eq!(r, set_aside);
        assert_eq!(calls(&fake), [Call::LoadJournal, Call::SetAsideJournal]);
        let s = fake.state();
        assert_eq!(s.set_aside, 1);
        // The accepted gap (section 5, "Gap left"): the override stays until the user sets the values by hand.
        assert_eq!(
            (s.stored(P1, LidDc), s.in_force(P1, LidDc)),
            (Some(0), Some(0))
        );
    }

    /// A failed read at step 2 must not look like "every plan deleted".
    fn assert_fails_before_any_change(index: usize, call: &'static str) {
        let rows = ours(&[entry(P1, LidDc, false), entry(P2, SleepDc, false)]);
        let fake = world(&rows, vec![]);
        let before = fake.state().journal.clone();
        fake.script(index, Action::Fail(31));
        let r = run(&fake, None);
        assert_eq!(r.found, Found::Entries);
        assert_eq!(r.result, Err(Failure { call, code: 31 }));
        assert_eq!(
            r.journal,
            [entry(P1, LidDc, false), entry(P2, SleepDc, false)]
        );
        assert_eq!(
            calls(&fake).len(),
            index + 1,
            "no call after the failed one"
        );
        assert_eq!(fake.state().journal, before);
        assert_reachable(&fake, call);
    }

    #[test]
    fn failed_active_plan_read_fails_before_any_change() {
        assert_fails_before_any_change(1, "FakePlans::active_plan");
    }

    #[test]
    fn failed_plan_list_fails_before_any_change() {
        assert_fails_before_any_change(2, "FakePlans::plans");
    }

    #[test]
    fn deleted_plans_entries_are_dropped_from_a_complete_list() {
        let rows = ours(&[entry(P1, LidDc, false), entry(P2, SleepDc, false)]);
        let fake = world(&rows, vec![]);
        fake.state().delete_plan(P2);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert!(!c.contains(&Call::Read(P2, SleepDc)), "{c:?}");
        assert!(!c.contains(&Call::Write(P2, SleepDc, 600)), "{c:?}");
        assert_restored(&fake);
    }

    #[test]
    fn failed_entry_read_keeps_the_entry_and_fails() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        fake.script(3, Action::Fail(5));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.call), Err("FakePlans::read"));
        assert_eq!(r.journal, [e]);
        assert_eq!(
            calls(&fake),
            [
                Call::LoadJournal,
                Call::ActivePlan,
                Call::Plans,
                Call::Read(P1, LidDc)
            ]
        );
        assert_eq!(fake.state().known_entries(), [e]);
        assert_reachable(&fake, "failed read");
    }

    #[test]
    fn failed_entry_read_keeps_only_that_entry() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, false), entry(P1, LidDc, false));
        let fake = world(&ours(&[lid_ac, lid_dc]), vec![]);
        fake.script(3, Action::Fail(5));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.call), Err("FakePlans::read"));
        assert_eq!(r.journal, [lid_ac]);
        let c = calls(&fake);
        assert_eq!(count(&c, &Call::Write(P1, LidDc, 1)), 1);
        assert_eq!(
            c.last(),
            Some(&Call::SaveJournal(file(vec![lid_ac], vec![])))
        );
        for clean_up in [Call::DeleteJournal, Call::ClearRunOnce, Call::StopGuard] {
            assert_eq!(count(&c, &clean_up), 0, "{clean_up:?}");
        }
        assert_reachable(&fake, "failed read of one entry");
    }

    #[test]
    fn unmarked_live_written_is_restored() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        assert_eq!(
            calls(&fake),
            [
                Call::LoadJournal,
                Call::ActivePlan,
                Call::Plans,
                Call::Read(P1, LidDc),
                Call::SaveJournal(file(vec![marked(e)], vec![])),
                Call::Write(P1, LidDc, 1),
                Call::ActivePlan,
                Call::SetActive(P1),
                Call::DeleteJournal,
                Call::ClearRunOnce,
                Call::StopGuard,
            ]
        );
        assert_restored(&fake);
    }

    #[test]
    fn marked_live_written_is_restored_again() {
        let e = entry(P1, LidDc, true);
        let fake = world(&ours(&[e]), vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        assert_eq!(count(&calls(&fake), &Call::Write(P1, LidDc, 1)), 1);
        assert_restored(&fake);
    }

    #[test]
    fn marked_live_original_only_needs_set_active() {
        let e = entry(P1, LidDc, true);
        let fake = world(&[(e, Live::Unapplied)], vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        assert_eq!(
            calls(&fake),
            [
                Call::LoadJournal,
                Call::ActivePlan,
                Call::Plans,
                Call::Read(P1, LidDc),
                Call::ActivePlan,
                Call::SetActive(P1),
                Call::DeleteJournal,
                Call::ClearRunOnce,
                Call::StopGuard,
            ]
        );
        assert_restored(&fake);
    }

    #[test]
    fn any_other_live_value_is_dropped() {
        let rows = [
            (entry(P1, LidAc, false), Live::Never),
            (entry(P1, LidDc, false), Live::User(3)),
            (entry(P1, SleepDc, true), Live::User(300)),
        ];
        let fake = world(&rows, vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert_eq!(c.len(), 9, "{c:?}");
        assert_eq!(
            c[6..],
            [Call::DeleteJournal, Call::ClearRunOnce, Call::StopGuard]
        );
        let s = fake.state();
        assert_eq!(s.stored(P1, LidAc), Some(1));
        assert_eq!(s.stored(P1, LidDc), Some(3));
        assert_eq!(s.in_force(P1, SleepDc), Some(300));
        assert!(s.held().is_empty());
    }

    #[test]
    fn marks_are_saved_before_any_original_is_written() {
        let entries = [
            entry(P1, LidAc, false),
            entry(P1, LidDc, false),
            entry(P2, SleepDc, false),
        ];
        let fake = world(&ours(&entries), vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        let save = c.iter().position(|c| matches!(c, Call::SaveJournal(_)));
        let write = c.iter().position(|c| matches!(c, Call::Write(..)));
        assert!(save.unwrap() < write.unwrap(), "{c:?}");
        let all_marked = file(entries.map(marked).to_vec(), vec![]);
        assert_eq!(c[save.unwrap()], Call::SaveJournal(all_marked));
        assert_restored(&fake);
    }

    #[test]
    fn failed_mark_save_writes_nothing() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        fake.script(4, Action::Fail(112));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.code), Err(112));
        assert_eq!(r.journal, [e]);
        let c = calls(&fake);
        assert_eq!(c.len(), 5, "{c:?}");
        assert!(matches!(c[4], Call::SaveJournal(_)));
        let s = fake.state();
        assert_eq!(s.known_entries(), [e]);
        assert_eq!(
            (s.stored(P1, LidDc), s.in_force(P1, LidDc)),
            (Some(0), Some(0))
        );
    }

    #[test]
    fn stop_after_the_mark_before_the_write_restores_again() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        run_stopped(&fake, 5);
        assert_eq!(fake.state().known_entries(), [marked(e)]);
        assert_reachable(&fake, "stopped before the write");

        fake.state().calls.clear();
        assert_eq!(run(&fake, None), ok(vec![]));
        assert_eq!(count(&calls(&fake), &Call::Write(P1, LidDc, 1)), 1);
        assert_restored(&fake);
    }

    #[test]
    fn failed_write_keeps_the_entry_marked_and_fails() {
        let (lid_ac, lid_dc) = (entry(P1, LidAc, false), entry(P1, LidDc, false));
        let fake = world(&ours(&[lid_ac, lid_dc]), vec![]);
        fake.script(6, Action::Fail(5));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.call), Err("FakePlans::write"));
        assert_eq!(r.journal, [marked(lid_ac)]);
        assert_eq!(fake.state().known_entries(), [marked(lid_ac)]);
        assert_reachable(&fake, "failed write");

        assert_eq!(run(&fake, None), ok(vec![]));
        assert_restored(&fake);
    }

    #[test]
    fn failed_set_active_keeps_the_marked_entries_and_a_retry_finishes() {
        let (active, inactive) = (entry(P1, LidDc, false), entry(P2, SleepDc, false));
        let fake = world(&ours(&[active, inactive]), vec![]);
        fake.script(9, Action::Fail(5));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.call), Err("FakePlans::set_active"));
        assert_eq!(r.journal, [marked(active)]);
        {
            let s = fake.state();
            assert_eq!(s.known_entries(), [marked(active)]);
            assert_eq!(
                (s.stored(P1, LidDc), s.in_force(P1, LidDc)),
                (Some(1), Some(0))
            );
        }
        assert_reachable(&fake, "failed set_active");

        fake.state().calls.clear();
        assert_eq!(run(&fake, None), ok(vec![]));
        assert_eq!(count(&calls(&fake), &Call::SetActive(P1)), 1);
        assert_restored(&fake);
    }

    #[test]
    fn failed_second_active_plan_read_keeps_the_entries() {
        let (active, inactive) = (entry(P1, LidDc, false), entry(P2, SleepDc, false));
        let fake = world(&ours(&[active, inactive]), vec![]);
        fake.script(8, Action::Fail(5));
        let r = run(&fake, None);
        assert_eq!(r.result.map_err(|f| f.call), Err("FakePlans::active_plan"));
        assert_eq!(r.journal, [marked(active), marked(inactive)]);
        assert!(!calls(&fake).contains(&Call::SetActive(P1)));
        assert_reachable(&fake, "failed second plan read");

        assert_eq!(run(&fake, None), ok(vec![]));
        assert_restored(&fake);
    }

    #[test]
    fn plan_switch_before_set_active_drops_the_now_inactive_plans_entries() {
        let entries = [entry(P1, LidDc, false), entry(P1, SleepDc, false)];
        let fake = world(&ours(&entries), vec![]);
        fake.script(8, Action::Run(Box::new(|s| s.switch_plan(P2))));
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert!(!c.iter().any(|c| matches!(c, Call::SetActive(_))), "{c:?}");
        assert_eq!(fake.state().active, P2);
        assert_restored(&fake);
    }

    /// Step 5 judges "inactive" by the second read: a switch to P2 before P2's `original` is written brought our
    /// value into force there, so P2 needs the call.
    #[test]
    fn plan_switch_before_the_write_applies_the_plan_now_active() {
        let (lid_dc, sleep_dc) = (entry(P1, LidDc, false), entry(P2, SleepDc, false));
        let fake = world(&ours(&[lid_dc, sleep_dc]), vec![]);
        fake.script(7, Action::Run(Box::new(|s| s.switch_plan(P2))));
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert_eq!(count(&c, &Call::SetActive(P2)), 1, "{c:?}");
        assert_eq!(count(&c, &Call::SetActive(P1)), 0, "{c:?}");
        assert_eq!(fake.state().in_force(P2, SleepDc), Some(600));
        assert_restored(&fake);
    }

    #[test]
    fn marked_entries_on_an_inactive_plan_need_no_set_active() {
        let rows = [
            (entry(P2, LidDc, true), Live::Unapplied),
            (entry(P2, SleepDc, false), Live::Ours),
        ];
        let fake = world(&rows, vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert!(!c.iter().any(|c| matches!(c, Call::SetActive(_))), "{c:?}");
        assert_eq!(fake.state().active, P1);
        assert_restored(&fake);
    }

    #[test]
    fn a_plan_scope_takes_only_that_plans_entries() {
        let kept = entry(P1, LidDc, false);
        let entries = [kept, entry(P2, LidAc, false), entry(P2, SleepDc, false)];
        let fake = world(&ours(&entries), vec![]);
        assert_eq!(run(&fake, Some(P2)), ok(vec![kept]));
        let c = calls(&fake);
        assert!(!c.contains(&Call::Read(P1, LidDc)), "{c:?}");
        assert_eq!(c.last(), Some(&Call::SaveJournal(file(vec![kept], vec![]))));
        let s = fake.state();
        assert_eq!(
            (s.stored(P1, LidDc), s.in_force(P1, LidDc)),
            (Some(0), Some(0))
        );
        assert_eq!(s.stored(P2, SleepDc), Some(600));
        assert!(s.run_once);
        assert_eq!(s.guard, FakeGuard::Running);
    }

    #[test]
    fn nothing_in_scope_makes_no_plan_call() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        assert_eq!(run(&fake, Some(P2)), ok(vec![e]));
        assert_eq!(calls(&fake), [Call::LoadJournal]);
    }

    #[test]
    fn two_plans_in_one_journal_are_both_restored() {
        let entries = [
            entry(P1, LidAc, false),
            entry(P1, LidDc, false),
            entry(P2, LidDc, false),
            entry(P2, SleepDc, false),
        ];
        let fake = world(&ours(&entries), vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        assert_eq!(count(&c, &Call::SetActive(P1)), 1, "{c:?}");
        assert_eq!(count(&c, &Call::SetActive(P2)), 0, "{c:?}");
        assert_restored(&fake);
    }

    #[test]
    fn another_owner_on_a_retry_is_left_alone() {
        let e = entry(P1, LidDc, false);
        let fake = world(&ours(&[e]), vec![]);
        if let FakeJournal::File(f) = &mut fake.state().journal {
            f.owner = NEW_TRAY;
        }
        let r = restore(&mut fake.clone(), &mut fake.clone(), None, Some(TRAY));
        let left = Restored {
            found: Found::OtherOwner(NEW_TRAY),
            result: Ok(()),
            journal: vec![e],
        };
        assert_eq!(r, left);
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        assert_eq!(fake.state().stored(P1, LidDc), Some(0));

        let same = world(&ours(&[e]), vec![]);
        let r = restore(&mut same.clone(), &mut same.clone(), None, Some(TRAY));
        assert_eq!(r, ok(vec![]));
        assert_restored(&same);
    }

    #[test]
    fn unknown_keys_are_kept_and_saved_back_untouched() {
        let (lid_dc, sleep_dc) = (entry(P1, LidDc, false), entry(P2, SleepDc, false));
        let fake = world(&ours(&[lid_dc, sleep_dc]), vec![newer_entry()]);
        assert_eq!(run(&fake, Some(P1)), ok(vec![sleep_dc]));
        let c = calls(&fake);
        let mark = file(vec![marked(lid_dc), sleep_dc], vec![newer_entry()]);
        assert!(c.contains(&Call::SaveJournal(mark)), "{c:?}");
        let last = file(vec![sleep_dc], vec![newer_entry()]);
        assert_eq!(fake.state().journal, FakeJournal::File(last));
    }

    #[test]
    fn only_unknown_entries_left_keeps_the_file_and_releases() {
        let fake = world(&ours(&[entry(P1, LidDc, false)]), vec![newer_entry()]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        let only_unknown = file(vec![], vec![newer_entry()]);
        assert_eq!(
            c[c.len() - 3..],
            [
                Call::SaveJournal(only_unknown.clone()),
                Call::ClearRunOnce,
                Call::StopGuard
            ]
        );
        assert!(!c.contains(&Call::DeleteJournal));
        assert_values_back(&fake, "only unknown left");
        let s = fake.state();
        assert_eq!(s.journal, FakeJournal::File(only_unknown));
        assert!(!s.run_once);
        assert_eq!(s.guard, FakeGuard::Stopping);
    }

    #[test]
    fn a_file_with_only_unknown_entries_is_not_rewritten() {
        let fake = world(&[], vec![newer_entry()]);
        let before = fake.state().journal.clone();
        let r = run(&fake, None);
        assert_eq!((r.found, r.result), (Found::Nothing, Ok(())));
        assert_eq!(
            calls(&fake),
            [Call::LoadJournal, Call::ClearRunOnce, Call::StopGuard]
        );
        assert_eq!(fake.state().journal, before);
    }

    #[test]
    fn a_save_writes_the_current_version() {
        let (lid_dc, sleep_dc) = (entry(P1, LidDc, false), entry(P2, SleepDc, false));
        let fake = world(&ours(&[lid_dc, sleep_dc]), vec![newer_entry()]);
        if let FakeJournal::File(f) = &mut fake.state().journal {
            f.version = 2;
        }
        run(&fake, Some(P1));
        let c = calls(&fake);
        let saves: Vec<u32> = c
            .iter()
            .filter_map(|c| match c {
                Call::SaveJournal(f) => Some(f.version),
                _ => None,
            })
            .collect();
        assert_eq!(saves, [JournalFile::VERSION, JournalFile::VERSION]);
    }

    #[test]
    fn the_last_drop_runs_the_clean_up_once_in_its_order() {
        let entries = [
            entry(P1, LidAc, false),
            entry(P1, LidDc, false),
            entry(P2, SleepDc, false),
        ];
        let fake = world(&ours(&entries), vec![]);
        assert_eq!(run(&fake, None), ok(vec![]));
        let c = calls(&fake);
        let clean_up = [Call::DeleteJournal, Call::ClearRunOnce, Call::StopGuard];
        assert_eq!(c[c.len() - 3..], clean_up);
        for call in &clean_up {
            assert_eq!(count(&c, call), 1, "{call:?}");
        }
        assert_restored(&fake);
    }

    /// The rows and unknown entries `world` starts from.
    type Start = (Vec<(JournalEntry, Live)>, Vec<JournalFileEntry>);

    /// Two starting states, each over two plans and the three keys: all ours; and mixed, as earlier failed runs and
    /// a newer version leave them.
    fn starts() -> [Start; 2] {
        let all_ours = ours(&[
            entry(P1, LidAc, false),
            entry(P1, LidDc, false),
            entry(P1, SleepDc, false),
            entry(P2, LidDc, false),
            entry(P2, SleepDc, false),
        ]);
        let mixed = vec![
            (entry(P1, LidAc, true), Live::Unapplied),
            (entry(P1, LidDc, true), Live::Ours),
            (entry(P1, SleepDc, false), Live::Ours),
            (entry(P2, LidDc, false), Live::Ours),
            (entry(P2, SleepDc, true), Live::Unapplied),
        ];
        [(all_ours, vec![]), (mixed, vec![newer_entry()])]
    }

    /// Architecture section 9: a stop at any call keeps every held value reachable, and the next start's restore puts
    /// every value back.
    #[test]
    fn a_stop_at_any_call_keeps_every_held_value_journaled() {
        for (start, (rows, other)) in starts().into_iter().enumerate() {
            let clean = world(&rows, other.clone());
            assert_eq!(run(&clean, None), ok(vec![]));
            let clean_calls = calls(&clean);
            let deleted_at = clean_calls.iter().position(|c| *c == Call::DeleteJournal);
            for (stop, call) in clean_calls.iter().enumerate() {
                let case = format!("start {start}, stop at call {stop} ({call:?})");
                let fake = world(&rows, other.clone());
                run_stopped(&fake, stop);
                assert_reachable(&fake, &case);

                let next = run(&fake, None);
                assert_eq!(next.result, Ok(()), "{case}");
                assert!(next.journal.is_empty(), "{case}: {:?}", next.journal);
                assert_values_back(&fake, &case);
                let s = fake.state();
                if deleted_at.is_none_or(|d| stop <= d) {
                    assert!(!s.run_once, "{case}");
                    assert_eq!(s.guard, FakeGuard::Stopping, "{case}");
                } else {
                    // A stop after the file is gone: RunOnce or the guard may be left, and both find no journal, so
                    // they do nothing.
                    assert_eq!(s.journal, FakeJournal::Missing, "{case}");
                }
            }
        }
    }

    #[test]
    fn a_stop_inside_the_clean_up_is_finished_by_the_next_run() {
        for (rows, other) in starts() {
            let clean = world(&rows, other.clone());
            run(&clean, None);
            let n = calls(&clean).len();
            for stop in n - 3..n {
                let fake = world(&rows, other.clone());
                run_stopped(&fake, stop);
                assert_reachable(&fake, "stop in the clean-up");
                assert_eq!(run(&fake, None).result, Ok(()));
                assert_values_back(&fake, "after a stop in the clean-up");
                // Only a stop between the delete and the RunOnce clear leaves the value, which then finds no journal.
                assert_eq!(fake.state().run_once, stop == n - 2 && other.is_empty());
            }
        }
    }
}
