use std::panic::{AssertUnwindSafe, catch_unwind};

use super::*;
use crate::files::{JournalFile, JournalFileEntry, JournalKey};
use crate::ownership::check::check;
use crate::ownership::fake::{
    Action, Call, FAKE_BLOCKED, FakeGuard, FakeJournal, FakePlans, SetBy,
};
use crate::ownership::journal::JournalParts;
use crate::ownership::restore::{Restored, restore};
use crate::process::FileTime;
use crate::settings::{Journal, OwnedKey, PlanValues};

use SettingKey::{LidAc, LidDc, SleepDc};

/// Balanced, the active plan at the start.
const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
/// High performance, inactive at the start.
const P2: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
const KEYS: [SettingKey; 3] = [LidAc, LidDc, SleepDc];
/// What the core sends at a hold start with nothing released.
const ALL: [(SettingKey, u32); 3] = [(LidAc, 0), (LidDc, 0), (SleepDc, 0)];
/// The same with sleep DC released.
const LIDS: [(SettingKey, u32); 2] = [(LidAc, 0), (LidDc, 0)];
const TRAY: JournalOwner = JournalOwner {
    pid: 9120,
    proc_start: FileTime(5),
    session: 1,
};
const DEAD_TRAY: JournalOwner = JournalOwner {
    pid: 7344,
    proc_start: FileTime(3),
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

/// The entry a fresh apply to P1 makes.
fn new(key: SettingKey) -> JournalEntry {
    entry(P1, key, false)
}

fn marked(e: JournalEntry) -> JournalEntry {
    JournalEntry {
        restored: true,
        ..e
    }
}

fn unmarked(e: JournalEntry) -> JournalEntry {
    JournalEntry {
        restored: false,
        ..e
    }
}

/// How an entry's value stands when the apply starts.
#[derive(Clone, Copy)]
enum Live {
    /// Our value, stored and in force: an apply finished, or a restore failed before or right after its mark.
    Ours,
    /// `original` stored again, ours still in force: a restore wrote it and its `set_active` failed.
    Unapplied,
    /// Ours stored again, `original` in force: an apply took over an applied restore's entry and stopped before
    /// its call.
    Rewritten,
    /// The user set this value since.
    User(u32),
}

/// What a failed restore leaves on P1 for an apply to take over, one entry per way to fail (E1 to E3).
fn takeover() -> [(JournalEntry, Live); 3] {
    [
        // E1: the restore wrote `original`, then its call failed.
        (entry(P1, LidAc, true), Live::Unapplied),
        // E2: the restore stopped after its mark.
        (entry(P1, LidDc, true), Live::Ours),
        // E3: the restore failed on a read or on its mark.
        (entry(P1, SleepDc, false), Live::Ours),
    ]
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

fn file(
    owner: JournalOwner,
    entries: Vec<JournalEntry>,
    other: Vec<JournalFileEntry>,
) -> JournalFile {
    JournalParts {
        known: Journal { owner, entries },
        other,
    }
    .merge()
}

/// P1 active and P2 inactive, both with the user's values; no journal, RunOnce value or guard.
fn fresh() -> FakePlans {
    let fake = FakePlans::new(user_values(P1));
    fake.add_plan(user_values(P2));
    fake
}

/// `fresh`, plus what earlier runs owned by `owner` left: the values in `rows`, a journal with their entries and
/// `other`, the RunOnce value, a running guard.
fn left(
    owner: JournalOwner,
    rows: &[(JournalEntry, Live)],
    other: Vec<JournalFileEntry>,
) -> FakePlans {
    let fake = fresh();
    let mut s = fake.state();
    for &(e, live) in rows {
        let ours = (e.written, SetBy::Override);
        let theirs = (e.original, SetBy::Restore);
        let (stored, in_force) = match live {
            Live::Ours => (ours, ours),
            Live::Unapplied => (theirs, ours),
            Live::Rewritten => (ours, theirs),
            Live::User(value) => {
                s.user_write(e.plan, e.key, value);
                continue;
            }
        };
        s.set_stored(e.plan, e.key, stored.0, stored.1);
        if e.plan == s.active {
            s.set_in_force(e.plan, e.key, in_force.0, in_force.1);
        }
    }
    let entries = rows.iter().map(|r| r.0).collect();
    s.journal = FakeJournal::File(file(owner, entries, other));
    s.run_once = true;
    s.guard = FakeGuard::Running;
    drop(s);
    fake
}

/// P1's sleep DC entry, left by a failed restore after the user released the key on P2, and an entry a newer
/// version wrote. The hold then applies `LIDS`.
fn released() -> FakePlans {
    left(
        TRAY,
        &[(entry(P1, SleepDc, false), Live::Ours)],
        vec![newer_entry()],
    )
}

fn run(fake: &FakePlans, writes: &[(SettingKey, u32)]) -> Applied {
    apply(&mut fake.clone(), &mut fake.clone(), TRAY, P1, writes)
}

/// Runs the apply with a stop at call `index`, as a crash would end it.
fn run_stopped(fake: &FakePlans, writes: &[(SettingKey, u32)], index: usize) {
    fake.script(index, Action::Stop);
    let run = catch_unwind(AssertUnwindSafe(|| run(fake, writes)));
    assert!(run.is_err(), "the apply did not reach call {index}");
}

fn restore_all(fake: &FakePlans) -> Restored {
    restore(&mut fake.clone(), &mut fake.clone(), None, None)
}

fn calls(fake: &FakePlans) -> Vec<Call> {
    fake.state().calls.clone()
}

fn count(calls: &[Call], call: &Call) -> usize {
    calls.iter().filter(|c| *c == call).count()
}

fn is_save(c: &Call) -> bool {
    matches!(c, Call::SaveJournal(_))
}

fn is_write(c: &Call) -> bool {
    matches!(c, Call::Write(..))
}

fn is_set_active(c: &Call) -> bool {
    matches!(c, Call::SetActive(_))
}

fn saves(calls: &[Call]) -> Vec<JournalFile> {
    calls
        .iter()
        .filter_map(|c| match c {
            Call::SaveJournal(f) => Some(f.clone()),
            _ => None,
        })
        .collect()
}

/// The index of the `n`th call (from 0) that `pick` matches.
fn nth(calls: &[Call], n: usize, pick: impl Fn(&Call) -> bool) -> usize {
    let found = calls.iter().enumerate().filter(|(_, c)| pick(c)).nth(n);
    found
        .unwrap_or_else(|| panic!("no call {n} matches in {calls:?}"))
        .0
}

/// The index of that call in a fault-free apply from `start` with `writes`.
fn index_in(
    start: impl Fn() -> FakePlans,
    writes: &[(SettingKey, u32)],
    n: usize,
    pick: impl Fn(&Call) -> bool,
) -> usize {
    let clean = start();
    run(&clean, writes);
    nth(&calls(&clean), n, pick)
}

/// Nothing of this apply is left: no journal, RunOnce value or running guard, no write, every value the user's.
fn assert_nothing_left(fake: &FakePlans) {
    let s = fake.state();
    assert_eq!(s.journal, FakeJournal::Missing);
    assert!(!s.run_once);
    assert_ne!(s.guard, FakeGuard::Running);
    assert_eq!(s.held(), Vec::<OwnedKey>::new());
    assert!(!s.calls.iter().any(is_write), "{:?}", s.calls);
}

/// What every stop and failure keeps: each value we still hold, stored or in force, has a known entry, and while
/// one exists the RunOnce value is set and the guard runs.
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

/// Each of `keys` holds our 0 on P1, stored and in force.
fn assert_overridden(fake: &FakePlans, keys: &[SettingKey]) {
    let s = fake.state();
    assert_eq!(s.active, P1);
    for &key in keys {
        let value = (
            s.stored(P1, key),
            s.in_force(P1, key),
            s.in_force_by(P1, key),
        );
        assert_eq!(value, (Some(0), Some(0), Some(SetBy::Override)), "{key:?}");
    }
}

fn safety_net(call: &'static str, code: u32) -> Result<(), ApplyError> {
    Err(ApplyError::SafetyNet(Failure { call, code }))
}

#[test]
fn step_1_read_fails_writes_nothing() {
    let other = entry(P2, SleepDc, false);
    let fake = left(TRAY, &[(other, Live::Ours)], vec![]);
    fake.script(1, Action::Fail(31));
    let r = run(&fake, &ALL);
    let failed = Applied {
        written: vec![],
        result: safety_net("FakePlans::active_plan", 31),
        journal: vec![other],
    };
    assert_eq!(r, failed);
    assert_eq!(calls(&fake), [Call::LoadJournal, Call::ActivePlan]);
}

#[test]
fn step_1_other_plan_writes_nothing() {
    let fake = fresh();
    fake.state().switch_plan(P2);
    let r = run(&fake, &ALL);
    let changed = Applied {
        written: vec![],
        result: Err(ApplyError::PlanChanged),
        journal: vec![],
    };
    assert_eq!(r, changed);
    assert_eq!(calls(&fake), [Call::LoadJournal, Call::ActivePlan]);
    assert_nothing_left(&fake);
}

#[test]
fn unreadable_journal_fails_and_touches_nothing() {
    let fake = left(TRAY, &[(entry(P2, SleepDc, false), Live::Ours)], vec![]);
    fake.state().journal = FakeJournal::Unreadable(32);
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::load_journal", 32));
    assert_eq!(r.journal, []);
    assert_eq!(calls(&fake), [Call::LoadJournal]);
    let s = fake.state();
    assert_eq!(s.journal, FakeJournal::Unreadable(32));
    assert!(s.run_once);
    assert_eq!(s.guard, FakeGuard::Running);
}

#[test]
fn corrupt_journal_fails_and_is_left_for_the_restore() {
    let fake = fresh();
    fake.state().journal = FakeJournal::Corrupt;
    let r = run(&fake, &ALL);
    assert_eq!(r.result, Err(ApplyError::SafetyNet(JOURNAL_CORRUPT)));
    assert_eq!(r.journal, []);
    assert_eq!(calls(&fake), [Call::LoadJournal]);
    assert_eq!(fake.state().journal, FakeJournal::Corrupt);
}

#[test]
fn can_write_fails_for_the_second_key_and_nothing_is_left() {
    let fake = fresh();
    fake.state().blocked.push(LidDc);
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::can_write", FAKE_BLOCKED));
    assert_eq!(r.written, []);
    assert_eq!(
        calls(&fake),
        [
            Call::LoadJournal,
            Call::ActivePlan,
            Call::CanWrite(LidAc),
            Call::CanWrite(LidDc)
        ]
    );
    assert_nothing_left(&fake);
}

#[test]
fn a_failed_live_read_leaves_nothing() {
    let read = index_in(fresh, &ALL, 0, |c| *c == Call::Read(P1, LidDc));
    let fake = fresh();
    fake.script(read, Action::Fail(31));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::read", 31));
    assert_eq!(calls(&fake).len(), read + 1, "no call after the read");
    assert_nothing_left(&fake);
}

#[test]
fn the_users_own_value_at_the_target_is_skipped() {
    let fake = fresh();
    fake.state().user_write(P1, LidDc, 0);
    let r = run(&fake, &ALL);
    let applied = Applied {
        written: vec![LidAc, SleepDc],
        result: Ok(()),
        journal: vec![new(LidAc), new(SleepDc)],
    };
    assert_eq!(r, applied);
    let c = calls(&fake);
    assert!(
        !c.iter().any(|c| matches!(c, Call::Write(_, LidDc, _))),
        "{c:?}"
    );
    let s = fake.state();
    assert_eq!(s.known_entries(), [new(LidAc), new(SleepDc)]);
    assert_eq!(s.in_force_by(P1, LidDc), Some(SetBy::User));
}

#[test]
fn nothing_to_override_makes_no_safety_net_call() {
    let fake = fresh();
    for key in KEYS {
        fake.state().user_write(P1, key, 0);
    }
    let r = run(&fake, &ALL);
    let nothing = Applied {
        written: vec![],
        result: Ok(()),
        journal: vec![],
    };
    assert_eq!(r, nothing);
    let c = calls(&fake);
    assert_eq!(c.len(), 8, "the journal, step 1 and step 2 only: {c:?}");
    let s = fake.state();
    assert_eq!(s.journal, FakeJournal::Missing);
    assert!(!s.run_once);
    assert_eq!(s.guard, FakeGuard::None);
}

#[test]
fn a_failed_run_once_write_leaves_nothing() {
    let at = index_in(fresh, &ALL, 0, |c| *c == Call::SetRunOnce);
    let fake = fresh();
    fake.script(at, Action::Fail(5));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::set_run_once", 5));
    assert_eq!(calls(&fake).len(), at + 1, "no guard, journal or write");
    assert_nothing_left(&fake);
}

#[test]
fn a_failed_guard_start_takes_back_the_run_once_value() {
    let at = index_in(fresh, &ALL, 0, |c| *c == Call::EnsureGuard);
    let fake = fresh();
    fake.script(at, Action::Fail(1450));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::ensure_guard", 1450));
    assert_eq!(calls(&fake)[at..], [Call::EnsureGuard, Call::ClearRunOnce]);
    assert_nothing_left(&fake);
}

#[test]
fn a_failed_journal_save_takes_back_what_this_apply_added() {
    let at = index_in(fresh, &ALL, 0, is_save);
    let fake = fresh();
    fake.script(at, Action::Fail(112));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, safety_net("FakePlans::save_journal", 112));
    assert_eq!(r.journal, []);
    assert_eq!(
        calls(&fake)[at + 1..],
        [Call::ClearRunOnce, Call::StopGuard]
    );
    assert_nothing_left(&fake);
}

/// A RunOnce value and a guard left with no journal are not this apply's to take back.
#[test]
fn a_failed_journal_save_keeps_what_was_there_before() {
    let at = index_in(fresh, &ALL, 0, is_save);
    let fake = fresh();
    {
        let mut s = fake.state();
        s.run_once = true;
        s.guard = FakeGuard::Running;
    }
    fake.script(at, Action::Fail(112));
    let r = run(&fake, &ALL);
    assert!(matches!(r.result, Err(ApplyError::SafetyNet(_))));
    assert_eq!(calls(&fake).len(), at + 1, "nothing taken back");
    let s = fake.state();
    assert!(s.run_once);
    assert_eq!((s.guard, s.guards_started), (FakeGuard::Running, 0));
    assert_eq!(s.journal, FakeJournal::Missing);
}

/// The entries already in the journal need the RunOnce value and a guard, so what this apply added for them
/// stays. Here Windows deleted the RunOnce value at sign-in and the guard had ended.
#[test]
fn with_a_journal_present_the_added_run_once_value_and_guard_stay() {
    let theirs = entry(P2, SleepDc, false);
    let start = || {
        let fake = left(TRAY, &[(theirs, Live::Ours)], vec![]);
        let mut s = fake.state();
        s.run_once = false;
        s.guard = FakeGuard::None;
        drop(s);
        fake
    };
    let guard = index_in(start, &ALL, 0, |c| *c == Call::EnsureGuard);
    let save = index_in(start, &ALL, 0, is_save);
    for (at, guard_added) in [(guard, false), (save, true)] {
        let fake = start();
        fake.script(at, Action::Fail(112));
        let r = run(&fake, &ALL);
        assert!(matches!(r.result, Err(ApplyError::SafetyNet(_))), "{at}");
        assert_eq!(r.journal, [theirs]);
        assert_eq!(calls(&fake).len(), at + 1, "{at}: nothing taken back");
        let s = fake.state();
        assert!(s.run_once, "{at}");
        assert_eq!(s.guard == FakeGuard::Running, guard_added, "{at}");
        assert_eq!(s.known_entries(), [theirs]);
        assert!(!s.calls.iter().any(is_write));
    }
}

/// A file with only a newer version's entries counts as empty (`JournalParts`): this version's RunOnce value and
/// guard do nothing for it, so they are taken back, and the file is left as it was.
#[test]
fn a_journal_with_only_unknown_entries_counts_as_absent() {
    let start = || {
        let fake = fresh();
        fake.state().journal = FakeJournal::File(file(DEAD_TRAY, vec![], vec![newer_entry()]));
        fake
    };
    let at = index_in(start, &ALL, 0, is_save);
    let fake = start();
    fake.script(at, Action::Fail(112));
    run(&fake, &ALL);
    assert_eq!(
        calls(&fake)[at + 1..],
        [Call::ClearRunOnce, Call::StopGuard]
    );
    let s = fake.state();
    let untouched = FakeJournal::File(file(DEAD_TRAY, vec![], vec![newer_entry()]));
    assert_eq!(s.journal, untouched);
    assert!(!s.run_once);
}

/// New entries are saved marked and the marks cleared once our value is in force.
#[test]
fn a_fresh_apply_journals_last_then_writes_and_activates() {
    let fake = fresh();
    let r = run(&fake, &ALL);
    let entries = KEYS.map(new).to_vec();
    let applied = Applied {
        written: KEYS.to_vec(),
        result: Ok(()),
        journal: entries.clone(),
    };
    assert_eq!(r, applied);
    assert_eq!(
        calls(&fake),
        [
            Call::LoadJournal,
            Call::ActivePlan,
            Call::CanWrite(LidAc),
            Call::CanWrite(LidDc),
            Call::CanWrite(SleepDc),
            Call::Read(P1, LidAc),
            Call::Read(P1, LidDc),
            Call::Read(P1, SleepDc),
            Call::SetRunOnce,
            Call::EnsureGuard,
            Call::SaveJournal(file(TRAY, KEYS.map(|k| marked(new(k))).to_vec(), vec![])),
            Call::Write(P1, LidAc, 0),
            Call::Read(P1, LidAc),
            Call::Write(P1, LidDc, 0),
            Call::Read(P1, LidDc),
            Call::Write(P1, SleepDc, 0),
            Call::Read(P1, SleepDc),
            Call::ActivePlan,
            Call::SetActive(P1),
            Call::SaveJournal(file(TRAY, entries.clone(), vec![])),
        ]
    );
    assert_overridden(&fake, &KEYS);
    let s = fake.state();
    assert_eq!(s.known_entries(), entries);
    assert!(s.run_once);
    assert_eq!((s.guard, s.guards_started), (FakeGuard::Running, 1));
}

/// An apply that saved its new entries and never wrote them (a stop, or a failed write whose own restore failed
/// too): the next restore puts every value back.
#[test]
fn a_new_entry_never_written_is_never_a_user_change() {
    let first_write = index_in(fresh, &ALL, 0, is_write);
    let fake = fresh();
    run_stopped(&fake, &ALL, first_write);
    let checked = check(&mut fake.clone(), &mut fake.clone());
    assert_eq!(checked.user_changed, [], "{checked:?}");
    assert_eq!(checked.journal, KEYS.map(|k| marked(new(k))));

    assert_eq!(restore_all(&fake).result, Ok(()));
    assert_values_back(&fake, "the restore after the stop");
}

/// The apply skips the key and keeps the entry as it is, so the next check reports the change and the core
/// releases the key (decision 26, the latest change wins at once).
#[test]
fn a_user_change_no_check_saw_yet_is_never_written_over() {
    let changed = entry(P1, LidAc, false);
    let marked_changed = entry(P1, LidDc, true);
    let rows = [
        (changed, Live::User(2)),
        (marked_changed, Live::User(3)),
        (entry(P1, SleepDc, false), Live::Ours),
    ];
    let fake = left(TRAY, &rows, vec![]);
    let r = run(&fake, &ALL);
    let applied = Applied {
        written: vec![],
        result: Ok(()),
        journal: rows.map(|r| r.0).to_vec(),
    };
    assert_eq!(r, applied);
    let c = calls(&fake);
    assert!(!c.iter().any(is_write), "{c:?}");
    {
        let s = fake.state();
        assert_eq!(s.stored(P1, LidAc), Some(2));
        assert_eq!(s.in_force(P1, LidAc), Some(2));
        assert_eq!(s.in_force(P1, LidDc), Some(3));
    }
    assert_overridden(&fake, &[SleepDc]);

    let checked = check(&mut fake.clone(), &mut fake.clone());
    assert_eq!(
        checked.user_changed,
        [
            OwnedKey {
                plan: P1,
                key: LidAc
            },
            OwnedKey {
                plan: P1,
                key: LidDc
            }
        ]
    );
    assert_eq!(restore_all(&fake).result, Ok(()));
    let s = fake.state();
    assert_eq!(KEYS.map(|k| s.in_force(P1, k).unwrap()), [2, 3, 600]);
}

#[test]
fn takeover_keeps_each_entrys_original_and_mark_until_the_call() {
    let rows = takeover();
    let fake = left(TRAY, &rows, vec![]);
    let r = run(&fake, &ALL);
    let taken = rows.map(|r| r.0).to_vec();
    let cleared: Vec<JournalEntry> = taken.iter().copied().map(unmarked).collect();
    let applied = Applied {
        written: vec![LidAc],
        result: Ok(()),
        journal: cleared.clone(),
    };
    assert_eq!(r, applied);
    let c = calls(&fake);
    assert_eq!(
        saves(&c),
        [
            file(TRAY, taken, vec![]),
            file(TRAY, cleared.clone(), vec![])
        ]
    );
    // Only E1 needs its write: E2 and E3 already hold the target.
    let writes: Vec<&Call> = c.iter().filter(|c| is_write(c)).collect();
    assert_eq!(writes, [&Call::Write(P1, LidAc, 0)]);
    let (save, write) = (nth(&c, 0, is_save), nth(&c, 0, is_write));
    let (call, clear) = (nth(&c, 0, is_set_active), nth(&c, 1, is_save));
    assert!(save < write && write < call && call < clear, "{c:?}");
    assert_overridden(&fake, &KEYS);
    assert_eq!(fake.state().known_entries(), cleared);
}

/// Our value is stored but `original` is in force: no write is needed, and only the call brings ours back.
#[test]
fn takeover_with_nothing_to_write_still_calls_set_active() {
    let e = entry(P1, LidDc, true);
    let fake = left(TRAY, &[(e, Live::Rewritten)], vec![]);
    let r = run(&fake, &[(LidDc, 0)]);
    let applied = Applied {
        written: vec![],
        result: Ok(()),
        journal: vec![unmarked(e)],
    };
    assert_eq!(r, applied);
    let c = calls(&fake);
    assert!(!c.iter().any(is_write), "{c:?}");
    assert_eq!(count(&c, &Call::SetActive(P1)), 1, "{c:?}");
    assert_overridden(&fake, &[LidDc]);
}

#[test]
fn other_plans_entries_and_unknown_keys_are_saved_back_untouched() {
    let theirs = entry(P2, SleepDc, true);
    let mine = entry(P1, LidDc, false);
    let rows = [(theirs, Live::Unapplied), (mine, Live::Ours)];
    let fake = left(DEAD_TRAY, &rows, vec![newer_entry()]);
    let r = run(&fake, &ALL);
    assert_eq!(r.result, Ok(()));
    let entries = vec![theirs, mine, new(LidAc), new(SleepDc)];
    assert_eq!(r.journal, entries);
    let c = calls(&fake);
    let saved = file(TRAY, entries, vec![newer_entry()]);
    assert_eq!(fake.state().journal, FakeJournal::File(saved.clone()));
    let first = vec![theirs, mine, marked(new(LidAc)), marked(new(SleepDc))];
    assert_eq!(
        saves(&c),
        [file(TRAY, first, vec![newer_entry()]), saved],
        "this tray owns the journal now"
    );
    assert!(!c.contains(&Call::Read(P2, SleepDc)), "{c:?}");
    assert!(!c.contains(&Call::SetActive(P2)), "{c:?}");
}

/// An entry is ours while its live value is `written`, or it is marked with `original` stored (restore step 3).
/// One not marked whose live value is `original` was never written (an older version's apply stopped before its
/// write, or the user set exactly that value back): the apply replaces it with a new one whose `original` is the
/// live value. Any other value is the user's change, which the apply leaves for the check. So a later restore puts
/// back the user's latest value, never an older one.
#[test]
fn an_entry_never_written_is_replaced_and_a_users_change_is_kept() {
    let rows = [
        // An earlier apply stopped before its write.
        (entry(P1, LidAc, false), Live::User(1)),
        // The user changed it while a restore waited for its retry.
        (entry(P1, LidDc, true), Live::User(3)),
        (entry(P1, SleepDc, false), Live::User(300)),
    ];
    let fake = left(TRAY, &rows, vec![]);
    let r = run(&fake, &ALL);
    let kept = [rows[1].0, rows[2].0];
    let entries = vec![new(LidAc), kept[0], kept[1]];
    let applied = Applied {
        written: vec![LidAc],
        result: Ok(()),
        journal: entries.clone(),
    };
    assert_eq!(r, applied);
    assert_eq!(
        saves(&calls(&fake)),
        [
            file(TRAY, vec![marked(new(LidAc)), kept[0], kept[1]], vec![]),
            file(TRAY, entries, vec![])
        ]
    );
    assert_overridden(&fake, &[LidAc]);

    assert_eq!(restore_all(&fake).result, Ok(()));
    let s = fake.state();
    let back = KEYS.map(|k| s.in_force(P1, k).unwrap());
    assert_eq!(back, [1, 3, 300]);
    assert!(s.held().is_empty());
}

/// A takeover run from `rows` with `action` at the call `pick` finds (the `n`th): the run reports `Write` from
/// `call`, the restore routine ran on P1's entries, and no value we hold lost its entry.
fn takeover_fails(
    rows: &[(JournalEntry, Live)],
    n: usize,
    pick: impl Fn(&Call) -> bool,
    action: Action,
    call: &str,
) -> FakePlans {
    let start = || left(TRAY, rows, vec![]);
    let at = index_in(start, &ALL, n, pick);
    let fake = start();
    fake.script(at, action);
    let r = run(&fake, &ALL);
    assert!(
        matches!(r.result, Err(ApplyError::Write(f)) if f.call == call),
        "{call}: {:?}",
        r.result
    );
    let c = calls(&fake);
    assert_eq!(count(&c, &Call::LoadJournal), 2, "{call}: the routine ran");
    assert_eq!(r.journal, fake.state().known_entries(), "{call}");
    assert_reachable(&fake, call);
    fake
}

/// Step 5: a write for an earlier key fails, so the second E1 is never written again. Then E1 alone, whose own
/// write fails: no other entry's call in the routine brings its `original` into force.
#[test]
fn takeover_then_a_failed_write_keeps_every_held_value() {
    let e1 = (entry(P1, LidAc, true), Live::Unapplied);
    let rows = [
        e1,
        (entry(P1, LidDc, true), Live::Unapplied),
        (entry(P1, SleepDc, false), Live::Ours),
    ];
    let pick = |c: &Call| *c == Call::Write(P1, LidAc, 0);
    let fake = takeover_fails(&rows, 0, pick, Action::Fail(87), "FakePlans::write");
    assert!(!calls(&fake).contains(&Call::Write(P1, LidDc, 0)));
    assert_values_back(&fake, "the routine restored them");

    let fake = takeover_fails(&[e1], 0, pick, Action::Fail(87), "FakePlans::write");
    assert_values_back(&fake, "E1 alone");
}

#[test]
fn takeover_then_a_failed_read_back_keeps_every_held_value() {
    let read_back = |c: &Call| *c == Call::Read(P1, LidAc);
    let fake = takeover_fails(
        &takeover(),
        1,
        read_back,
        Action::Fail(31),
        "FakePlans::read",
    );
    assert_values_back(&fake, "failed read-back");

    // A read-back that returns another value: here the user's own change, which the routine then leaves alone.
    let user = Action::Run(Box::new(|s| s.user_write(P1, LidAc, 3)));
    let fake = takeover_fails(&takeover(), 1, read_back, user, READ_BACK_DIFFERS.call);
    let s = fake.state();
    assert_eq!(s.in_force(P1, LidAc), Some(3));
    assert_eq!(s.in_force(P1, LidDc), Some(1));
    assert!(s.held().is_empty() && s.known_entries().is_empty());
}

#[test]
fn takeover_then_a_failed_second_plan_read_keeps_every_held_value() {
    let second_read = |c: &Call| *c == Call::ActivePlan;
    let fail = Action::Fail(31);
    let fake = takeover_fails(&takeover(), 1, second_read, fail, "FakePlans::active_plan");
    assert!(
        !calls(&fake)[..nth(&calls(&fake), 1, |c| *c == Call::LoadJournal)]
            .iter()
            .any(is_set_active)
    );
    assert_values_back(&fake, "the routine restored them");
}

/// Step 6: the call fails, and likely fails again in the routine, since it just failed in a restore. The entries
/// stay, marked, and a later restore finishes them.
#[test]
fn takeover_then_a_failed_set_active_keeps_every_held_value() {
    let fake = takeover_fails(
        &takeover(),
        0,
        is_set_active,
        Action::Fail(5),
        "FakePlans::set_active",
    );
    assert_values_back(&fake, "the routine restored them");

    let start = || left(TRAY, &takeover(), vec![]);
    let first = index_in(start, &ALL, 0, is_set_active);
    let probe = start();
    probe.script(first, Action::Fail(5));
    run(&probe, &ALL);
    let second = nth(&calls(&probe), 1, is_set_active);
    let fake = start();
    fake.script(first, Action::Fail(5));
    fake.script(second, Action::Fail(5));
    let r = run(&fake, &ALL);
    assert!(matches!(r.result, Err(ApplyError::Write(_))));
    let left = fake.state().known_entries();
    assert_eq!(left, takeover().map(|r| marked(r.0)), "kept, marked");
    assert_eq!(r.journal, left);
    assert_reachable(&fake, "both calls failed");
    assert_eq!(restore_all(&fake).result, Ok(()));
    assert_values_back(&fake, "the retry finished them");
}

/// A starting state and the writes the core sends for it.
type Start = fn() -> FakePlans;
type Writes = &'static [(SettingKey, u32)];

/// Stops at every call of one apply from `start`, then runs a fault-free restore: at the stop no held value lacks
/// its entry, and the restore puts every value back.
fn walk_every_stop(start: impl Fn() -> FakePlans, writes: &[(SettingKey, u32)], name: &str) {
    let clean = start();
    assert_eq!(run(&clean, writes).result, Ok(()), "{name}");
    let clean_calls = calls(&clean);
    for (stop, call) in clean_calls.iter().enumerate() {
        let case = format!("{name}, stop at call {stop} ({call:?})");
        let fake = start();
        run_stopped(&fake, writes, stop);
        assert_reachable(&fake, &case);

        let next = restore_all(&fake);
        assert_eq!(next.result, Ok(()), "{case}");
        assert!(next.journal.is_empty(), "{case}: {:?}", next.journal);
        assert_values_back(&fake, &case);
    }
}

#[test]
fn a_stop_at_every_call_of_a_fresh_apply_then_a_restore_puts_every_value_back() {
    walk_every_stop(fresh, &ALL, "fresh");
}

/// Each taken-over entry alone too: beside the others, their `set_active` in the next restore would bring an E1's
/// stored `original` into force and hide an E1 the run dropped.
#[test]
fn takeover_then_a_stop_at_any_call_drops_no_held_value() {
    walk_every_stop(
        || left(TRAY, &takeover(), vec![newer_entry()]),
        &ALL,
        "takeover",
    );
    for (i, row) in takeover().into_iter().enumerate() {
        walk_every_stop(
            || left(TRAY, &[row], vec![]),
            &ALL,
            &format!("E{} alone", i + 1),
        );
    }
}

#[test]
fn a_released_entry_stays_reachable_at_every_stop() {
    walk_every_stop(released, &LIDS, "released");
}

#[test]
fn a_plan_switch_between_the_writes_and_set_active_makes_no_call() {
    let starts: [(&str, Start, Writes); 2] =
        [("fresh", fresh, &ALL), ("released", released, &LIDS)];
    for (name, start, writes) in starts {
        let second_read = index_in(start, writes, 1, |c| *c == Call::ActivePlan);
        let fake = start();
        fake.script(second_read, Action::Run(Box::new(|s| s.switch_plan(P2))));
        let r = run(&fake, writes);
        assert_eq!(r.result, Err(ApplyError::PlanChanged), "{name}");
        let written: Vec<SettingKey> = writes.iter().map(|w| w.0).collect();
        assert_eq!(r.written, written, "{name}");
        let c = calls(&fake);
        assert!(!c.iter().any(is_set_active), "{name}: {c:?}");
        assert_eq!(r.journal, fake.state().known_entries(), "{name}");
        assert_eq!(c.last(), Some(&Call::ActivePlan), "{name}: entries kept");
        assert_reachable(&fake, name);

        // The plan switch path: restore the old plan, which needs no call now that it is inactive.
        let switch = restore(&mut fake.clone(), &mut fake.clone(), Some(P1), None);
        assert_eq!(switch.result, Ok(()), "{name}");
        let c = calls(&fake);
        assert!(!c.iter().any(is_set_active), "{name}: {c:?}");
        assert_values_back(&fake, name);
        assert_eq!(fake.state().active, P2);
    }
}

/// The call takes the id the second read returned, right before it. The call runs only when that read agrees with
/// step 1, so the second read decides it, never the step 1 id alone.
#[test]
fn set_active_uses_the_id_read_in_step_6() {
    let first_write = index_in(fresh, &ALL, 0, is_write);
    let second_read = index_in(fresh, &ALL, 1, |c| *c == Call::ActivePlan);

    // Switched away during the writes and back before the second read: the call goes to the plan read then.
    let fake = fresh();
    fake.script(first_write, Action::Run(Box::new(|s| s.switch_plan(P2))));
    fake.script(second_read, Action::Run(Box::new(|s| s.switch_plan(P1))));
    assert_eq!(run(&fake, &ALL).result, Ok(()));
    let c = calls(&fake);
    let call = nth(&c, 0, is_set_active);
    assert_eq!(c[call - 1..=call], [Call::ActivePlan, Call::SetActive(P1)]);
    assert_overridden(&fake, &KEYS);

    // Switched away and not back: no call at all, so the user's switch stands.
    let fake = fresh();
    fake.script(first_write, Action::Run(Box::new(|s| s.switch_plan(P2))));
    assert_eq!(run(&fake, &ALL).result, Err(ApplyError::PlanChanged));
    assert!(!calls(&fake).iter().any(is_set_active));
    assert_eq!(fake.state().active, P2);
}

/// It is marked in step 3's save, gets its `original` in step 5 when ours is stored, and leaves after the one
/// successful call.
#[test]
fn a_released_entry_on_this_plan_is_restored_in_the_same_run() {
    let e = entry(P1, SleepDc, false);
    let restore_write = Call::Write(P1, SleepDc, 600);
    let cases = [
        ("ours", e, Live::Ours, true),
        ("marked, ours stored", marked(e), Live::Ours, true),
        ("marked, original stored", marked(e), Live::Unapplied, false),
    ];
    for (name, e, live, write) in cases {
        let fake = left(TRAY, &[(e, live)], vec![]);
        let r = run(&fake, &LIDS);
        let applied = Applied {
            written: vec![LidAc, LidDc],
            result: Ok(()),
            journal: vec![new(LidAc), new(LidDc)],
        };
        assert_eq!(r, applied, "{name}");
        let c = calls(&fake);
        assert_eq!(
            saves(&c),
            [
                file(
                    TRAY,
                    vec![marked(e), marked(new(LidAc)), marked(new(LidDc))],
                    vec![]
                ),
                file(TRAY, vec![new(LidAc), new(LidDc)], vec![])
            ],
            "{name}"
        );
        assert_eq!(count(&c, &restore_write), usize::from(write), "{name}");
        assert_eq!(count(&c, &Call::SetActive(P1)), 1, "{name}");
        if write {
            assert!(nth(&c, 0, |c| *c == restore_write) < nth(&c, 0, is_set_active));
        }
        assert_overridden(&fake, &[LidAc, LidDc]);
        let s = fake.state();
        let sleep = (s.stored(P1, SleepDc), s.in_force(P1, SleepDc));
        assert_eq!(sleep, (Some(600), Some(600)), "{name}");
        assert_eq!(s.known_entries(), [new(LidAc), new(LidDc)], "{name}");
    }
}

#[test]
fn a_released_entry_the_user_changed_is_dropped_without_a_write() {
    let fake = left(
        TRAY,
        &[(entry(P1, SleepDc, false), Live::User(300))],
        vec![],
    );
    let r = run(&fake, &LIDS);
    assert_eq!(r.result, Ok(()));
    let c = calls(&fake);
    assert_eq!(
        saves(&c),
        [
            file(TRAY, vec![marked(new(LidAc)), marked(new(LidDc))], vec![]),
            file(TRAY, vec![new(LidAc), new(LidDc)], vec![])
        ]
    );
    assert!(
        !c.iter().any(|c| matches!(c, Call::Write(_, SleepDc, _))),
        "{c:?}"
    );
    assert_eq!(fake.state().in_force(P1, SleepDc), Some(300));
}

/// All keys released: the run is only the restore of this plan's entries, and the last drop releases the net.
#[test]
fn only_released_entries_are_restored_and_the_safety_net_released() {
    let e = entry(P1, SleepDc, false);
    let fake = left(TRAY, &[(e, Live::Ours)], vec![]);
    let r = run(&fake, &[]);
    let applied = Applied {
        written: vec![],
        result: Ok(()),
        journal: vec![],
    };
    assert_eq!(r, applied);
    assert_eq!(
        calls(&fake),
        [
            Call::LoadJournal,
            Call::ActivePlan,
            Call::Read(P1, SleepDc),
            Call::SetRunOnce,
            Call::EnsureGuard,
            Call::SaveJournal(file(TRAY, vec![marked(e)], vec![])),
            Call::Write(P1, SleepDc, 600),
            Call::ActivePlan,
            Call::SetActive(P1),
            Call::DeleteJournal,
            Call::ClearRunOnce,
            Call::StopGuard,
        ]
    );
    assert_values_back(&fake, "released only");
    let s = fake.state();
    assert_eq!(s.journal, FakeJournal::Missing);
    assert!(!s.run_once);
    assert_eq!(s.guard, FakeGuard::Stopping);
}

#[test]
fn a_failed_read_of_a_released_entry_writes_nothing() {
    let read = index_in(released, &LIDS, 0, |c| *c == Call::Read(P1, SleepDc));
    let fake = released();
    let before = fake.state().journal.clone();
    fake.script(read, Action::Fail(31));
    let r = run(&fake, &LIDS);
    assert_eq!(r.result, safety_net("FakePlans::read", 31));
    assert_eq!(r.journal, [entry(P1, SleepDc, false)]);
    assert_eq!(calls(&fake).len(), read + 1);
    assert_eq!(fake.state().journal, before);
}

/// The design writes each key whose step 2 read was not the target and does not read it again before the write.
/// A user change in that window (a few calls) is overwritten, and the entry keeps the step 2 value as `original`,
/// so the restore puts back the value from before the user's change. The read-back sees ours, so the run succeeds.
#[test]
fn a_user_change_between_steps_2_and_5_is_overwritten() {
    let save = index_in(fresh, &ALL, 0, is_save);
    let fake = fresh();
    fake.script(save, Action::Run(Box::new(|s| s.user_write(P1, LidDc, 3))));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, Ok(()));
    assert_eq!(r.journal, KEYS.map(new));
    assert_overridden(&fake, &KEYS);

    assert_eq!(restore_all(&fake).result, Ok(()));
    assert_eq!(fake.state().in_force(P1, LidDc), Some(1));
}

/// Apply step 6: a failed clearing save is tolerated. A mark left hides from the check only a user change back to
/// exactly `original`, and the next restore handles each marked entry right.
#[test]
fn a_failed_mark_clearing_save_keeps_the_marks_for_the_next_restore() {
    let start = || left(TRAY, &takeover(), vec![]);
    let clear = index_in(start, &ALL, 1, is_save);
    let fake = start();
    fake.script(clear, Action::Fail(112));
    let r = run(&fake, &ALL);
    let taken = takeover().map(|r| r.0).to_vec();
    let applied = Applied {
        written: vec![LidAc],
        result: Ok(()),
        journal: taken.clone(),
    };
    assert_eq!(r, applied);
    assert_eq!(fake.state().known_entries(), taken);
    assert_overridden(&fake, &KEYS);

    // E1 and E2 are marked: LidAc set back to exactly its `original` is not seen; LidDc set to another value is.
    fake.state().user_write(P1, LidAc, 1);
    fake.state().user_write(P1, LidDc, 3);
    let checked = check(&mut fake.clone(), &mut fake.clone());
    let lid_dc = OwnedKey {
        plan: P1,
        key: LidDc,
    };
    assert_eq!(checked.user_changed, [lid_dc], "{checked:?}");

    assert_eq!(restore_all(&fake).result, Ok(()));
    let s = fake.state();
    let back = KEYS.map(|k| s.in_force(P1, k).unwrap());
    assert_eq!(back, [1, 3, 600]);
    assert!(s.held().is_empty() && s.known_entries().is_empty());
}

// The sleep route's short timeouts: the four short keys at 1 minute, where the user's own is longer or Never.

use crate::ownership::fake::FAKE_SCREEN_OFF;
use crate::settings::SHORT_TIMEOUT_S;
use SettingKey::{SleepAc, VideoIdleAc, VideoIdleDc};

const SHORT: [(SettingKey, u32); 4] = [
    (SleepAc, SHORT_TIMEOUT_S),
    (SleepDc, SHORT_TIMEOUT_S),
    (VideoIdleAc, SHORT_TIMEOUT_S),
    (VideoIdleDc, SHORT_TIMEOUT_S),
];

/// A fresh plan's value of every key, as `fresh` and the fake set them.
fn fresh_value(key: SettingKey) -> u32 {
    match key {
        LidAc | LidDc => 1,
        SleepDc => 600,
        SleepAc => 10_800,
        VideoIdleAc => FAKE_SCREEN_OFF[0],
        VideoIdleDc => FAKE_SCREEN_OFF[1],
    }
}

#[test]
fn a_ceiling_is_met_at_or_under_it_and_not_by_never() {
    for key in [SleepAc, SleepDc, VideoIdleAc, VideoIdleDc] {
        assert!(
            !meets(key, 0, 60),
            "{key:?}: Never is longer than any timeout"
        );
        assert!(
            meets(key, 1, 60) && meets(key, 30, 60) && meets(key, 60, 60),
            "{key:?}"
        );
        assert!(!meets(key, 61, 60) && !meets(key, 18_000, 60), "{key:?}");
        assert!(
            meets(key, 0, 0) && !meets(key, 60, 0),
            "{key:?}: a target of Never is met only by Never"
        );
    }
    for key in [LidAc, LidDc] {
        assert!(meets(key, 0, 0) && !meets(key, 1, 0), "{key:?}");
        assert!(
            meets(key, 3, 3) && !meets(key, 2, 3),
            "{key:?}: an action is no ceiling"
        );
    }
}

#[test]
fn a_short_apply_writes_only_what_is_longer_or_never() {
    let fake = fresh();
    {
        let mut s = fake.state();
        s.user_write(P1, SleepAc, 0);
        s.user_write(P1, VideoIdleAc, 30);
        s.user_write(P1, VideoIdleDc, 60);
    }
    let r = run(&fake, &SHORT);
    assert_eq!(r.result, Ok(()));
    assert_eq!(r.written, [SleepAc, SleepDc], "{r:?}");
    let new = |key, original| JournalEntry {
        plan: P1,
        key,
        original,
        written: SHORT_TIMEOUT_S,
        restored: false,
    };
    assert_eq!(r.journal, [new(SleepAc, 0), new(SleepDc, 600)]);
    let s = fake.state();
    for (key, value) in [
        (SleepAc, 60),
        (SleepDc, 60),
        (VideoIdleAc, 30),
        (VideoIdleDc, 60),
    ] {
        assert_eq!(s.in_force(P1, key), Some(value), "{key:?}");
    }
    assert!(s.run_once && s.guard == FakeGuard::Running);
}

#[test]
fn a_hold_takes_the_short_battery_sleep_over_and_puts_the_other_short_keys_back() {
    let fake = fresh();
    assert_eq!(run(&fake, &SHORT).result, Ok(()));
    let r = run(&fake, &ALL);
    assert_eq!(r.result, Ok(()));
    // Battery sleep keeps the user's own 600 as `original`; the three others are the user's again.
    let mut journal = r.journal.clone();
    journal.sort_by_key(|e| e.key);
    assert_eq!(
        journal,
        [LidAc, LidDc, SleepDc].map(|k| JournalEntry {
            plan: P1,
            key: k,
            original: fresh_value(k),
            written: 0,
            restored: false,
        })
    );
    {
        let s = fake.state();
        for key in [SleepAc, VideoIdleAc, VideoIdleDc] {
            assert_eq!(s.in_force(P1, key), Some(fresh_value(key)), "{key:?}");
        }
    }
    assert_eq!(restore_all(&fake).result, Ok(()));
    let s = fake.state();
    for key in SettingKey::ALL {
        assert_eq!(s.in_force(P1, key), Some(fresh_value(key)), "{key:?}");
    }
    assert!(s.held().is_empty() && s.known_entries().is_empty());
}

#[test]
fn a_short_apply_puts_back_a_never_left_over_the_users_own_shorter_sleep() {
    // A hold's battery sleep Never, left by a failed restore, over the user's own 30 s.
    let left_never = JournalEntry {
        plan: P1,
        key: SleepDc,
        original: 30,
        written: 0,
        restored: false,
    };
    let fake = left(TRAY, &[(left_never, Live::Ours)], vec![]);
    let r = run(&fake, &SHORT);
    assert_eq!(r.result, Ok(()));
    assert!(!r.written.contains(&SleepDc), "{r:?}");
    assert!(r.journal.iter().all(|e| e.key != SleepDc), "{r:?}");
    let s = fake.state();
    assert_eq!(s.stored(P1, SleepDc), Some(30));
    assert_eq!(s.in_force(P1, SleepDc), Some(30));
    assert_eq!(s.in_force(P1, SleepAc), Some(60));
}

#[test]
fn a_short_apply_stopped_at_any_call_keeps_every_value_reachable_and_the_next_restore_puts_all_back()
 {
    let clean = fresh();
    run(&clean, &SHORT);
    let total = calls(&clean).len();
    for index in 0..total {
        let fake = fresh();
        run_stopped(&fake, &SHORT, index);
        let case = format!("stop at call {index}");
        assert_reachable(&fake, &case);
        assert_eq!(restore_all(&fake).result, Ok(()), "{case}");
        let s = fake.state();
        for key in SettingKey::ALL {
            assert_eq!(s.stored(P1, key), Some(fresh_value(key)), "{case}: {key:?}");
            assert_eq!(
                s.in_force(P1, key),
                Some(fresh_value(key)),
                "{case}: {key:?} in force"
            );
        }
        assert!(
            s.held().is_empty() && s.known_entries().is_empty(),
            "{case}"
        );
    }
}
