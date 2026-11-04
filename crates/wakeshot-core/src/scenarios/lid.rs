//! Lid, countdown and notice scenarios over `Core`: the lid-close lock and chime, the lock when a closed laptop
//! changes power source, the battery floor, the countdown to sleep with its short timeouts and fallback hibernate, the
//! Keep screen on idle lock, the closed-lid report, notices that wait while the user is away, and updates.
//!
//! Owner: `p2-core-lid`.
//!
//! The IO side is not here: each test feeds the `Done` outcomes, plan reads, lid, power, session and input by hand, as
//! the loop and the IO thread would post them. The plan read in most fixtures comes before the apply and is never
//! read again, so the lid rules must see what the apply put in force, not the last read.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

use crate::command::{
    ApplyError, Command, CountdownReason, Failure, HibernateEnd, HibernateSkip, Outcome, RestoreWhy,
};
use crate::event::{Event, SessionChange};
use crate::policy::ending::{FALLBACK_MS, IDLE_MS};
use crate::policy::lid::SOURCE_SETTLE_MS;
use crate::process::{Alive, FileTime};
use crate::settings::{Journal, JournalOwner, OwnedKey, PlanValues, SettingKey};
use crate::sim::{
    self, BALANCED, KEYS, SAFETY_TICK_MS, Sim, Start, applied, claude, entries, hold, plan,
    reset_done, restored, short_entries,
};
use crate::sources::Activity;
use crate::time::Now;
use crate::view::{Cue, KeepAwake, MenuCommand, Notice};

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

/// Not a multiple of the 30 s safety tick, so a time checked to the millisecond is the core's own deadline at work.
const OFF_THE_TICK: u64 = 7 * SECOND;

/// How long the session may take to report `Locked` after a closed-lid `Lock` (section 8, "Lock fails").
const LOCK_CHECK: u64 = 5 * SECOND;

const LOCK: Command = Command::Lock;
const CHIME: Command = Command::Play(Cue::LidClosedHolding);
const FLOOR_CHIME: Command = Command::Play(Cue::BatteryFloor);
const OFF: Command = Command::Requests {
    system: false,
    display: false,
};
const SYSTEM: Command = Command::Requests {
    system: true,
    display: false,
};

const RESTORE_FAILED: Failure = Failure {
    call: "PowerReadDCValueIndex",
    code: 2,
};
const LOCK_FAILED: Failure = Failure {
    call: "LockWorkStation",
    code: 5,
};
const SUSPEND_FAILED: Failure = Failure {
    call: "SetSuspendState",
    code: 1,
};
const UPDATE_FAILED: Failure = Failure {
    call: "UpdateManager::apply_updates_and_restart",
    code: 1,
};

fn lid(closed: bool) -> Event {
    Event::Lid { closed }
}

fn power(ac: bool, pct: u8) -> Event {
    Event::Power {
        ac,
        battery_pct: Some(pct),
    }
}

fn session(change: SessionChange) -> Event {
    Event::Session(change)
}

fn working(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Working), Alive::Yes)])
}

fn idle(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Idle), Alive::Yes)])
}

fn read(values: PlanValues) -> Event {
    Event::Plan(Ok(values))
}

fn hibernated(end: HibernateEnd) -> Event {
    Event::Done(Outcome::Hibernate(end))
}

/// Docked use: the user's own lid action on AC is Do nothing, on battery Sleep (other test values).
fn docked() -> PlanValues {
    PlanValues {
        lid_ac: 0,
        ..plan(BALANCED)
    }
}

/// What an apply leaves on a docked plan: the AC lid key is the user's own Do nothing, so the IO side skips it.
const DOCKED_KEYS: [SettingKey; 2] = [SettingKey::LidDc, SettingKey::SleepDc];

/// What the countdown sends when it is due: the last check and the going to sleep sound, then the short timeouts.
fn sleep_check(reason: CountdownReason, last_resume: Option<Now>) -> Command {
    Command::SleepCheck {
        reason,
        floor_pct: 20,
        last_resume_mono_ms: last_resume.map(|t| t.mono_ms),
    }
}

/// The fallback when the PC did not sleep within 5 minutes of the short timeouts.
fn fallback(reason: CountdownReason, last_resume: Option<Now>) -> Command {
    Command::Hibernate {
        reason,
        floor_pct: 20,
        last_resume_mono_ms: last_resume.map(|t| t.mono_ms),
    }
}

fn checked(result: Result<(), HibernateSkip>) -> Event {
    Event::Done(Outcome::SleepCheck(result))
}

/// The sleep route's `Apply` to Balanced with every short key at 1 minute.
fn short_apply() -> Command {
    Command::Apply {
        plan: BALANCED,
        writes: SettingKey::SHORT
            .iter()
            .map(|&k| (k, crate::settings::SHORT_TIMEOUT_S))
            .collect(),
    }
}

const SHORT_BACK: Command = Command::Restore {
    plan: None,
    why: RestoreWhy::ShortTimeouts,
};

/// The sleep check passed and the short timeouts landed on Balanced; gives the commands of both steps.
fn short_written(sim: &mut Sim) -> Vec<Command> {
    let mut out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
    out.extend(sim.send(applied(BALANCED, Ok(()), short_entries(BALANCED))));
    out
}

/// From a due countdown to the fallback: the check passes, the short timeouts land, and the PC does not sleep for
/// `FALLBACK_MS`. Gives the commands of the last run, which hold the `Hibernate`.
fn to_fallback(sim: &mut Sim) -> Vec<Command> {
    short_written(sim);
    sim.run(FALLBACK_MS)
}

fn fallbacks(commands: &[Command]) -> Vec<&Command> {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Hibernate { .. }))
        .collect()
}

fn count(commands: &[Command], wanted: &Command) -> usize {
    commands.iter().filter(|c| *c == wanted).count()
}

fn position(commands: &[Command], wanted: &Command) -> usize {
    commands
        .iter()
        .position(|c| c == wanted)
        .unwrap_or_else(|| panic!("no {wanted:?} in {commands:?}"))
}

fn logs(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Log(_)))
        .count()
}

fn notices(commands: &[Command]) -> Vec<&Notice> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::Notify(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

fn sleep_checks(commands: &[Command]) -> Vec<&Command> {
    commands
        .iter()
        .filter(|c| matches!(c, Command::SleepCheck { .. }))
        .collect()
}

fn plays(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Play(_)))
        .count()
}

fn reports(sim: &Sim) -> Vec<(u64, Vec<String>)> {
    sim.notices()
        .into_iter()
        .filter_map(|n| match n {
            Notice::ClosedLidReport { ran_ms, held_by } => Some((*ran_ms, held_by.clone())),
            _ => None,
        })
        .collect()
}

/// When each `SleepCheck` went out, in run time since `from`.
fn sleep_check_times(sim: &Sim, from: Now) -> Vec<i128> {
    sim.commands()
        .filter(|(_, c)| matches!(c, Command::SleepCheck { .. }))
        .map(|(at, _)| i128::from(at.mono_ms) - i128::from(from.mono_ms))
        .collect()
}

/// Every command so far.
fn all(sim: &Sim) -> Vec<Command> {
    sim.commands().map(|(_, c)| c.clone()).collect()
}

/// When each `Lock` went out, in run time since `from`.
fn lock_times(sim: &Sim, from: Now) -> Vec<i128> {
    sim.sent_at(&LOCK)
        .iter()
        .map(|t| i128::from(t.mono_ms) - i128::from(from.mono_ms))
        .collect()
}

fn never_going_to_sleep(sim: &Sim) {
    assert!(
        sim.sent_at(&Command::Play(Cue::GoingToSleep)).is_empty(),
        "the hibernate thread plays the going to sleep sound, never the core\n{}",
        sim.trace()
    );
}

/// Started with `values` read 7 s in, on AC or battery at `pct`, the lid open, the user at the keyboard just now.
fn started_with(values: PlanValues, ac: bool, pct: u8) -> Sim {
    let mut sim = Sim::new();
    sim.start();
    sim.send(power(ac, pct));
    sim.send(lid(false));
    sim.input();
    sim.run(OFF_THE_TICK);
    sim.send(read(values));
    sim
}

/// A Claude Code session holds with the test's user values, and the apply worked. The plan is not read again, so the
/// copy still shows Sleep on both lid keys.
fn holding(ac: bool) -> Sim {
    let mut sim = started_with(plan(BALANCED), ac, 80);
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim
}

/// The same with the user's own Do nothing on AC.
fn holding_docked(ac: bool) -> Sim {
    let mut sim = started_with(docked(), ac, 80);
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &DOCKED_KEYS)));
    sim
}

/// Holding, the lid closed (lock and chime), then the work ends: the countdown runs and the restore worked.
fn work_ended_closed(ac: bool) -> Sim {
    let mut sim = holding(ac);
    sim.send(lid(true));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim
}

// Item 1: the lid closes.

#[test]
fn lid_closed_on_ac_while_holding_locks_and_chimes() {
    let mut sim = holding(true);
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(
        position(&out, &LOCK) < position(&out, &CHIME),
        "the lock never waits for the sound\n{trace}"
    );
    assert!(logs(&out) >= 1, "the lock is logged\n{trace}");

    // Windows reports the lock; closed it stays: no second lock.
    sim.send(session(SessionChange::Locked));
    let out = sim.run(10 * MINUTE);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
}

#[test]
fn lid_closed_on_battery_while_holding_locks_and_chimes() {
    let mut sim = holding(false);
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(position(&out, &LOCK) < position(&out, &CHIME), "{trace}");
}

#[test]
fn docked_user_with_their_own_do_nothing_gets_no_lock_and_no_sound() {
    let mut sim = holding_docked(true);
    let mut out = sim.send(lid(true));
    out.extend(sim.run(10 * MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert_eq!(plays(&out), 0, "{trace}");
}

#[test]
fn lid_closed_in_the_first_seconds_of_a_hold_locks() {
    // The last plan read shows the user's Sleep; the apply's journal says Do nothing is in force since.
    let mut sim = holding(false);
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());

    // Once a read shows the written values, the same.
    let mut sim = holding(false);
    sim.send(read(PlanValues {
        lid_ac: 0,
        lid_dc: 0,
        sleep_dc: 0,
        ..plan(BALANCED)
    }));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
}

#[test]
fn lid_closed_while_the_apply_is_in_flight_locks() {
    let mut sim = started_with(plan(BALANCED), false, 80);
    sim.send(working(1));
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
}

#[test]
fn lid_closed_after_a_failed_restore_still_locks_and_chimes() {
    // Nothing holds, but the entries a failed restore left keep a closed laptop running.
    let mut sim = holding(true);
    sim.send(idle(1));
    sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
}

#[test]
fn lid_closed_on_a_failed_restore_after_work_ended_counts_down() {
    // Our Do nothing and battery sleep Never are left: nothing else would ever stop the closed laptop.
    let mut sim = holding(false);
    sim.send(idle(1));
    sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    sim.input();
    let at = sim.now();
    sim.send(lid(true));
    sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
}

#[test]
fn lid_closed_during_a_step_back_with_our_do_nothing_left_counts_down() {
    let mut sim = holding(false);
    sim.send(power(false, 19));
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim.send(session(SessionChange::Locked));
    sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::Floor, None)],
        "{trace}"
    );
}

#[test]
fn lid_closed_with_nothing_of_ours_gives_nothing() {
    let mut sim = started_with(plan(BALANCED), true, 80);
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert_eq!(plays(&out), 0, "{trace}");
}

#[test]
fn the_first_lid_reading_is_no_close() {
    let mut sim = Sim::new();
    sim.start();
    sim.send(power(true, 80));
    sim.run(OFF_THE_TICK);
    sim.send(read(plan(BALANCED)));
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    let out = sim.send(lid(true));
    assert_eq!(
        count(&out, &LOCK),
        0,
        "a reading, not a close\n{}",
        sim.trace()
    );

    sim.send(lid(false));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
}

/// `Started` with a crashed tray's journal whose start-up restore failed: its values for `keys` on Balanced.
fn crashed_tray(keys: &[SettingKey]) -> Event {
    Start {
        journal: Some(Journal {
            owner: JournalOwner {
                pid: 4100,
                proc_start: FileTime(7),
                session: 1,
            },
            entries: entries(BALANCED, keys),
        }),
        ..Start::default()
    }
    .event()
}

#[test]
fn a_lid_close_before_the_first_plan_read_locks_on_a_crashed_trays_do_nothing() {
    // `p2-core-props` finding 2: no plan read yet, but the journal says our Do nothing may be in force.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(false, 80));
    sim.send(lid(false));
    let out = sim.send(lid(true));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(position(&out, &LOCK) < position(&out, &CHIME), "{trace}");

    // Only the battery sleep key left: no Do nothing of ours, so nothing.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&[SettingKey::SleepDc]));
    sim.send(power(false, 80));
    sim.send(lid(false));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(plays(&out), 0, "{}", sim.trace());
}

#[test]
fn unplugged_before_the_first_plan_read_locks_on_a_crashed_trays_do_nothing_once_battery_lasted() {
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(true, 80));
    sim.send(lid(true));
    let unplugged = sim.now();
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert_eq!(
        lock_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{trace}"
    );
}

// A journal the copy does not know (`journal.json` did not read at start, or a damaged one went aside): a crashed
// tray's Do nothing may be in force, yet Wakeshot neither locks nor counts down until the window ends in a read or the
// reset (product "User settings safety" 7, Haik 2026-11-04; `p2-6-record-lost`).

fn unread_start() -> Event {
    Start {
        journal_unread: true,
        ..Start::default()
    }
    .event()
}

/// Battery, the lid open, then closed: the commands of the close.
fn close_on_battery(sim: &mut Sim) -> Vec<Command> {
    sim.send(lid(false));
    sim.send(lid(true))
}

#[test]
fn no_lock_at_a_lid_close_while_the_journal_is_unknown() {
    // Before any plan read.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(plays(&out), 0, "{}", sim.trace());

    // A plan read that shows Do nothing on battery, the crashed tray's or the user's own, and a retry whose load
    // fails again: still nothing known.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(RESTORE_FAILED),
        vec![],
    ));
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(plays(&out), 0, "{}", sim.trace());

    // Unplugged with the lid closed: no source lock either.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(true, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(true));
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());

    // A damaged file whose reset has not answered yet.
    let mut sim = Sim::new();
    sim.start();
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(Event::JournalCorrupt);
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
}

#[test]
fn once_the_journal_reads_its_entries_decide_the_lock() {
    // A retry that read the file but could not restore (its owner lives): the entries it names decide. The crashed
    // tray's battery lid key: a lock.
    let alive = Failure {
        call: "journal owner (alive)",
        code: 170,
    };
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(alive),
        entries(BALANCED, &[SettingKey::LidDc]),
    ));
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    assert_eq!(count(&out, &CHIME), 1, "{}", sim.trace());

    // Only the battery sleep key is the crashed tray's, so the Do nothing is the user's own (decision 18).
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(alive),
        entries(BALANCED, &[SettingKey::SleepDc]),
    ));
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(plays(&out), 0, "{}", sim.trace());

    // The lid closed in the window, before the read: once the entries name the crashed tray's Do nothing, it keeps
    // the closed laptop running, and the lock that close did not get comes then (`p2-6-sleep-route-fixes`).
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    let out = close_on_battery(&mut sim);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(alive),
        entries(BALANCED, &[SettingKey::LidDc]),
    ));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    assert_eq!(count(&out, &CHIME), 1, "{}", sim.trace());
}

// `p6-int-wiring-fixes` Found outside scope 3: with nothing holding, the lid closes on a Do nothing that may be a
// crashed tray's. Once the retry puts the user's own Sleep back, the closed laptop must count down as with a journal
// that read; with their own Do nothing it must not. Since `p2-6-record-lost` the close in the window neither locks nor
// wants a countdown; the want comes when the window ends (`Core::unknown_journal_ended`).

/// Do nothing on battery: the crashed tray's or the user's own.
fn do_nothing_dc() -> PlanValues {
    PlanValues {
        lid_dc: 0,
        ..plan(BALANCED)
    }
}

/// An unread journal, a plan read with Do nothing on battery, then the lid closes with nothing holding: no lock in the
/// window. Gives the time of the last input.
fn closed_on_an_unread_do_nothing(sim: &mut Sim) -> Now {
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(false));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    at
}

/// The one `Hibernate` came at `IDLE_MS` after `at`, for `reason`.
fn hibernated_after_idle(sim: &Sim, at: Now, reason: CountdownReason) {
    let trace = sim.trace();
    assert_eq!(sleep_check_times(sim, at), [i128::from(IDLE_MS)], "{trace}");
    assert_eq!(
        sleep_checks(&all(sim)),
        [&sleep_check(reason, None)],
        "{trace}"
    );
}

#[test]
fn with_an_unread_journal_a_closed_lid_counts_down_once_the_retry_puts_the_users_sleep_back() {
    // The tick sends the retry, then a read; the plan copy shows the Do nothing until that read lands. A loop timer
    // and a battery reading come in between, and decide nothing.
    let mut sim = Sim::new();
    let at = closed_on_an_unread_do_nothing(&mut sim);
    let out = sim.run(SAFETY_TICK_MS);
    let retry = Command::Restore {
        plan: None,
        why: RestoreWhy::Leftover,
    };
    assert_eq!(count(&out, &retry), 1, "{}", sim.trace());
    assert!(
        position(&out, &retry) < position(&out, &Command::ReadPlan),
        "{}",
        sim.trace()
    );
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    sim.run(SECOND);
    sim.send(Event::Tick);
    sim.send(power(false, 79));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);

    // The lid closed before any plan read: that read, taken after the restore, decides as well.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS + MINUTE);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);
}

#[test]
fn with_an_unread_journal_the_countdown_waits_through_failed_retries() {
    // A retry whose load fails reads nothing, and the reads in between still show the Do nothing: still unknown.
    let mut sim = Sim::new();
    let at = closed_on_an_unread_do_nothing(&mut sim);
    sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(RESTORE_FAILED),
        vec![],
    ));
    sim.send(read(do_nothing_dc()));
    let out = sim.run(MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);
}

#[test]
fn with_an_unread_journal_the_users_own_do_nothing_gives_no_countdown() {
    // The read behind the retry shows the Do nothing again: the user's own, so Wakeshot only lets go (decision 18).
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    let mut out = sim.send(read(do_nothing_dc()));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    // The lid opens: the waiting want ends as a countdown would. Closed again, nothing of ours keeps it running, so a
    // Sleep the user sets later is Windows' to act on.
    sim.send(lid(false));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let mut out = sim.send(read(plan(BALANCED)));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

// `p2-6-unread-lid-countdown` Found outside scope 2: the same for a step back with the lid closed. Once a restore puts
// the user's own Sleep back, the closed laptop on battery under the floor must count down as with a journal that read;
// with their own battery Do nothing it must not (power safety review finding 7).

/// An unread journal, on battery under the floor, a plan read with Do nothing on battery, the tray's first lid reading
/// closed, then a hold counts: the step back, and its restore goes out. Gives the time of the last input.
fn stepped_back_closed_on_an_unread_do_nothing(sim: &mut Sim) -> Now {
    sim.send(unread_start());
    sim.send(power(false, 19));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(true));
    sim.input();
    let at = sim.now();
    let out = sim.send(working(1));
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &restore), 1, "{}", sim.trace());
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{}", sim.trace());
    at
}

#[test]
fn with_an_unread_journal_a_closed_step_back_counts_down_once_the_users_sleep_is_back() {
    // The step back's own restore reads the file; the plan copy shows the Do nothing until a later read. A loop timer
    // and a battery reading come in between, and decide nothing.
    let mut sim = Sim::new();
    let at = stepped_back_closed_on_an_unread_do_nothing(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(SECOND);
    sim.send(power(false, 18));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::Floor);

    // The PC did not sleep, so the fallback hibernate came and resumed with the lid still closed: the countdown is
    // done, and the want that waited does not start another.
    to_fallback(&mut sim);
    sim.send(hibernated(HibernateEnd::Resumed));
    sim.run(20 * MINUTE);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1, "{}", sim.trace());
}

#[test]
fn with_an_unread_journal_a_close_after_the_step_back_counts_down_once_a_retry_works() {
    // The step back's restore fails to load the file, so the lid close in the window neither locks nor wants a
    // countdown. A retry that fails again, and the reads in between, still tell nothing; the window's end does.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 19));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(false));
    sim.send(working(1));
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        vec![],
    ));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        vec![],
    ));
    sim.send(read(do_nothing_dc()));
    let out = sim.run(MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::Floor);
}

#[test]
fn with_an_unread_journal_the_users_own_battery_do_nothing_gives_no_floor_countdown() {
    // The read behind the restore shows the Do nothing again: the user's own, so the step back only lets go.
    let mut sim = Sim::new();
    stepped_back_closed_on_an_unread_do_nothing(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let mut out = sim.send(read(do_nothing_dc()));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    // The lid opens: the waiting want ends as a countdown would. Closed again, nothing of ours keeps it running, so a
    // Sleep the user sets later is Windows' to act on.
    sim.send(lid(false));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let mut out = sim.send(read(plan(BALANCED)));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn with_an_unread_journal_a_step_back_with_the_lid_open_wants_no_countdown() {
    // The step back's restore has not landed, so the journal is still unknown, but the lid is open: no want. The close
    // meets the user's own Sleep, which Windows acts on itself.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 19));
    sim.send(read(plan(BALANCED)));
    sim.send(lid(false));
    let out = sim.send(working(1));
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{}", sim.trace());
    sim.input();
    let mut out = sim.send(lid(true));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert!(sleep_checks(&out).is_empty(), "{trace}");
}

// The end of the window (`p2-6-record-lost` deliverable 4): known or lost, with the lid closed and nothing holding,
// the work-end countdown starts once a read shows the user's own lid action as not Do nothing.

const RESET: Command = Command::ResetLost { damaged: false };

#[test]
fn a_lost_journal_with_the_lid_closed_counts_down_after_the_reset() {
    let mut sim = Sim::new();
    let at = closed_on_an_unread_do_nothing(&mut sim);
    let out = sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let trace = sim.trace();
    assert!(
        sleep_checks(&out).is_empty(),
        "no countdown in the window\n{trace}"
    );
    assert_eq!(count(&out, &RESET), 1, "{trace}");

    // The reset put the battery keys back to the plan's defaults; the read behind it shows Sleep.
    let out = sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    assert_eq!(count(&out, &Command::ReadPlan), 1, "{}", sim.trace());
    let lost = Notice::JournalLost {
        reset: vec![SettingKey::LidDc, SettingKey::SleepDc],
        failed: false,
    };
    assert_eq!(notices(&out), [&lost], "{}", sim.trace());
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);

    // The notice went to a closed laptop's screen: it comes again at the open laptop.
    to_fallback(&mut sim);
    sim.send(hibernated(HibernateEnd::Resumed));
    let out = sim.send(lid(false));
    assert_eq!(notices(&out), [&lost], "{}", sim.trace());
}

#[test]
fn a_damaged_file_with_the_lid_closed_counts_down_after_its_reset() {
    // A crashed tray's battery Do nothing, the tray starting with the lid closed, and its journal damaged.
    let mut sim = Sim::new();
    sim.start();
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(true));
    sim.input();
    let at = sim.now();
    let out = sim.send(Event::JournalCorrupt);
    assert_eq!(
        count(&out, &Command::ResetLost { damaged: true }),
        1,
        "{}",
        sim.trace()
    );
    let out = sim.run(MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    sim.send(reset_done(&[SettingKey::LidDc], Ok(())));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);
}

#[test]
fn after_the_window_a_do_nothing_still_in_force_gives_no_countdown() {
    // The plan's own default is Do nothing, so the reset left it: as without Wakeshot, the closed laptop runs on.
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let mut out = sim.send(reset_done(&[], Ok(())));
    out.extend(sim.send(read(do_nothing_dc())));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(
        notices(&out).is_empty(),
        "nothing reset, no notice\n{trace}"
    );
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
}

const RESET_WRITE_FAILED: Failure = Failure {
    call: "PowerWriteDCValueIndex",
    code: 5,
};

#[test]
fn a_failed_reset_locks_a_laptop_whose_lid_closed_in_the_window_once() {
    // Review minor 5 of `p2-6-sleep-route`: the Do nothing the reset could not put back is most likely Wakeshot's, yet
    // the journal now counts as known and empty, so nothing else would ever lock or sleep the closed laptop.
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let out = sim.send(reset_done(&[], Err(RESET_WRITE_FAILED)));
    let trace = sim.trace();
    let failed = Notice::JournalLost {
        reset: vec![],
        failed: true,
    };
    assert_eq!(notices(&out), [&failed], "{trace}");
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(position(&out, &LOCK) < position(&out, &CHIME), "{trace}");

    // Once: no countdown on a Do nothing that may be the user's own, and no second lock, even unconfirmed.
    let mut out = sim.send(read(do_nothing_dc()));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &LOCK), 0, "{trace}");

    // A lid that closed before the tray started, or one that opened again, saw no close in the window: no lock.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(true));
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let out = sim.send(reset_done(&[], Err(RESET_WRITE_FAILED)));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.send(lid(false));
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let out = sim.send(reset_done(&[], Err(RESET_WRITE_FAILED)));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
}

#[test]
fn a_hold_in_the_window_runs_on_requests_and_counts_down_when_it_ends() {
    // The hold's apply cannot load the journal either: requests only for the episode. The lid close in the window
    // neither locks nor counts down, and the reset does not end the hold.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(false));
    let out = sim.send(working(1));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    let load_failed = Failure {
        call: "ReadFile (journal.json)",
        code: 33,
    };
    let out = sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(load_failed)),
        vec![],
    ));
    assert_eq!(notices(&out), [&Notice::RequestsOnly], "{}", sim.trace());
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    // The window ends: the copy still shows the Do nothing, so the lock the close did not get waits for a read
    // (`p2-6-sleep-route` deliverable 8).
    let out = sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.send(read(plan(BALANCED)));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    assert!(
        position(&out, &LOCK) < position(&out, &CHIME),
        "{}",
        sim.trace()
    );
    sim.send(session(SessionChange::Locked));
    let out = sim.run(10 * MINUTE);
    assert_eq!(count(&out, &LOCK), 0, "once {}", sim.trace());
    assert!(
        sleep_checks(&out).is_empty(),
        "the hold holds\n{}",
        sim.trace()
    );
    assert!(sim.requests().system, "{}", sim.trace());

    // The hold ends with the lid closed: the user's own Sleep is known now, so the countdown starts at once.
    sim.input();
    let at = sim.now();
    sim.send(idle(1));
    sim.run(IDLE_MS);
    hibernated_after_idle(&sim, at, CountdownReason::WorkEnded);
}

#[test]
fn a_hold_in_the_window_on_the_users_own_do_nothing_gets_no_lock_when_it_ends() {
    // As above, but the user's own battery lid action is Do nothing: the laptop would run closed without Wakeshot too.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 80));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(false));
    sim.send(working(1));
    let load_failed = Failure {
        call: "ReadFile (journal.json)",
        code: 33,
    };
    sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(load_failed)),
        vec![],
    ));
    sim.send(lid(true));
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let mut out = sim.send(reset_done(&[], Ok(())));
    out.extend(sim.send(read(do_nothing_dc())));
    out.extend(sim.run(10 * MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(count(&out, &CHIME), 0, "{}", sim.trace());
}

// Review major 1 and minor 7 of `p2-6-sleep-route` (`p2-6-sleep-route-fixes`): the lock a lid close in the window did
// not get waits until the lid opens or the session locks, so a hold that holds only after the window still gets it,
// once its power source lasted, and a lock the session never confirms gets the retry and the step back.

#[test]
fn a_hold_that_starts_after_the_window_locks_a_laptop_closed_in_it() {
    // The window ends with nothing holding: the countdown starts after the reset and the read.
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let mut out = sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    out.extend(sim.send(read(plan(BALANCED))));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "nothing holds\n{}", sim.trace());

    // An agent turn starts before the countdown is due: its apply writes Do nothing, and the closed laptop would run
    // on it unlocked.
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert!(
        out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(position(&out, &LOCK) < position(&out, &CHIME), "{trace}");
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim.send(session(SessionChange::Locked));
    let out = sim.run(10 * MINUTE);
    assert_eq!(count(&out, &LOCK), 0, "once\n{}", sim.trace());
    assert!(sim.requests().system, "{}", sim.trace());

    // The hold starts right after the reset, while the copy still shows the crashed tray's Do nothing: the lock waits
    // for the apply's journal, which names the user's own Sleep.
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    let out = sim.send(working(1));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());

    // The session locked after the close, or was locked at it: nothing is owed. Signed in again later (on an external
    // screen), the user is there, as after a lid-close lock.
    for locked_before in [false, true] {
        let mut sim = Sim::new();
        sim.send(unread_start());
        sim.send(power(false, 80));
        sim.send(read(do_nothing_dc()));
        sim.send(lid(false));
        if locked_before {
            sim.send(session(SessionChange::Locked));
        }
        sim.send(lid(true));
        if !locked_before {
            sim.send(session(SessionChange::Locked));
        }
        sim.send(session(SessionChange::Unlocked));
        sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
        sim.send(reset_done(
            &[SettingKey::LidDc, SettingKey::SleepDc],
            Ok(()),
        ));
        sim.send(read(plan(BALANCED)));
        let mut out = sim.send(working(1));
        out.extend(sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS))));
        out.extend(sim.run(10 * MINUTE));
        assert_eq!(count(&out, &LOCK), 0, "{locked_before}\n{}", sim.trace());
    }
}

#[test]
fn a_hold_held_back_at_the_windows_end_locks_once_it_holds_again() {
    // Under the floor on battery a hold steps back at once, and its restore cannot load the journal. The lid closes in
    // the window, which ends while the step back holds the episode: the floor countdown, no lock.
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(false, 19));
    sim.send(read(do_nothing_dc()));
    sim.send(lid(false));
    sim.send(working(1));
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        vec![],
    ));
    sim.input();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    let mut out = sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    out.extend(sim.send(read(plan(BALANCED))));
    out.extend(sim.run(MINUTE));
    assert_eq!(
        count(&out, &LOCK),
        0,
        "the step back holds nothing\n{}",
        sim.trace()
    );

    // AC comes back: the hold holds again on Wakeshot's Do nothing, and the closed laptop is locked once AC lasted.
    let back = sim.now();
    let mut out = sim.send(power(true, 19));
    assert!(
        out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
    out.extend(sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS))));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert_eq!(
        lock_times(&sim, back),
        [i128::from(SOURCE_SETTLE_MS)],
        "{trace}"
    );
}

/// A hold that runs on requests only through the window (its apply cannot load the journal either), on `values`, on
/// AC or battery, with the lid closed in the window, a minute ago; the window still runs.
fn closed_in_the_window_on_requests(values: PlanValues, ac: bool) -> Sim {
    let mut sim = Sim::new();
    sim.send(unread_start());
    sim.send(power(ac, 80));
    sim.send(read(values));
    sim.send(lid(false));
    sim.send(working(1));
    let load_failed = Failure {
        call: "ReadFile (journal.json)",
        code: 33,
    };
    sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(load_failed)),
        vec![],
    ));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.run(MINUTE);
    sim
}

/// The window ends in a retry that reads the file, with nothing in it.
fn read_after_all() -> Event {
    restored(None, RestoreWhy::Leftover, Ok(()), vec![])
}

#[test]
fn a_window_lock_waits_out_a_short_ac_drop_for_a_docked_user() {
    // Docked: the user's own Do nothing on AC, Sleep on battery. On AC their Do nothing keeps a closed laptop running
    // anyway: no lock.
    let mut sim = closed_in_the_window_on_requests(docked(), true);
    let mut out = sim.send(read_after_all());
    out.extend(sim.send(read(docked())));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());

    // The dock drops AC for 5 s: no lock (product: Haik was locked by exactly this drop).
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(5 * SECOND));
    out.extend(sim.send(power(true, 80)));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());

    // Unplugged for good: the lock once battery lasted.
    let unplugged = sim.now();
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(
        lock_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{trace}"
    );
}

#[test]
fn a_window_lock_the_session_never_confirms_is_sent_again_then_steps_back() {
    // The hold runs on requests only, so no Do nothing of Wakeshot's is in force, yet its requests keep the closed
    // laptop running: the lock is watched as at a lid close.
    let mut sim = closed_in_the_window_on_requests(plan(BALANCED), false);
    sim.input();
    let input = sim.now();
    let out = sim.send(read_after_all());
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    let out = sim.run(LOCK_CHECK + SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "the retry\n{trace}");
    assert_eq!(notices(&out), [&Notice::LockFailed], "{trace}");
    let out = sim.run(LOCK_CHECK + SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "the step back\n{trace}");
    assert_eq!(count(&out, &LOCK), 0, "no third lock\n{trace}");

    // Stepped back with the lid closed and the user's own Sleep: the countdown, as after work that ended.
    sim.run(IDLE_MS);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, input),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
}

#[test]
fn a_window_lock_on_wakeshots_do_nothing_that_fails_twice_steps_back_with_no_third_lock() {
    // A hold that starts after the window writes Do nothing over the user's Sleep, and the lock it gets fails twice.
    // Until the step back's restore lands, that Do nothing is still in force: the step back is the answer, not a
    // third lock.
    let mut sim = Sim::new();
    closed_on_an_unread_do_nothing(&mut sim);
    sim.run_failing_restores(2 * MINUTE, RESTORE_FAILED);
    sim.send(reset_done(
        &[SettingKey::LidDc, SettingKey::SleepDc],
        Ok(()),
    ));
    sim.send(read(plan(BALANCED)));
    let out = sim.send(working(1));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    let mut out = failed_twice(&mut sim);
    let trace = sim.trace();
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(count(&out, &restore), 1, "{trace}");
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "no third lock\n{}", sim.trace());
}

// Item 2: the power source changes with the lid closed.

#[test]
fn closed_then_unplugged_after_signing_in_on_an_external_screen_locks_once_battery_lasted() {
    // Power verify minor 11: docked with the test's user values, the lid-close lock, the user signs in on the external
    // screen.
    let mut sim = holding(true);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));

    let unplugged = sim.now();
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS - 1));
    assert_eq!(
        count(&out, &LOCK),
        0,
        "not before the settle\n{}",
        sim.trace()
    );

    let out = sim.run(SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert_eq!(
        lock_times(&sim, unplugged).last(),
        Some(&i128::from(SOURCE_SETTLE_MS)),
        "{trace}"
    );
    assert!(logs(&out) >= 1, "the lock is logged\n{trace}");
}

#[test]
fn a_short_ac_drop_with_the_lid_closed_gives_no_lock() {
    for drop_ms in [3 * SECOND, 6_300] {
        let mut sim = holding(true);
        sim.send(lid(true));
        sim.send(session(SessionChange::Locked));
        sim.send(session(SessionChange::Unlocked));
        let mut out = sim.send(power(false, 80));
        out.extend(sim.run(drop_ms));
        out.extend(sim.send(power(true, 80)));
        out.extend(sim.run(5 * MINUTE));
        let trace = sim.trace();
        assert_eq!(count(&out, &LOCK), 0, "a {drop_ms} ms drop\n{trace}");
        assert_eq!(plays(&out), 0, "a {drop_ms} ms drop\n{trace}");
    }
}

#[test]
fn docked_with_their_own_ac_do_nothing_then_unplugged_locks_once_battery_lasted() {
    // Review minor 17: no lock at the close (the user's own Do nothing), but on battery our Do nothing keeps it running.
    let mut sim = holding_docked(true);
    sim.send(lid(true));
    let unplugged = sim.now();
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert_eq!(
        lock_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{trace}"
    );

    // Docked again: the user's own Do nothing is in force, so nothing.
    let mut out = sim.send(power(true, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
}

#[test]
fn the_first_power_reading_is_no_source_change() {
    let mut sim = Sim::new();
    sim.start();
    sim.send(lid(false));
    sim.run(OFF_THE_TICK);
    sim.send(read(plan(BALANCED)));
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    // The source is not known yet: ours on either source locks.
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim.send(session(SessionChange::Locked));

    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(SOURCE_SETTLE_MS + SECOND));
    assert_eq!(
        count(&out, &LOCK),
        0,
        "a reading, not a change\n{}",
        sim.trace()
    );
}

// Item 3: the battery floor.

#[test]
fn battery_floor_with_the_lid_open_steps_back_then_ac_back_holds_again() {
    let mut sim = holding(false);
    let out = sim.send(power(false, 19));
    let trace = sim.trace();
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(count(&out, &restore), 1, "{trace}");
    assert!(position(&out, &OFF) < position(&out, &restore), "{trace}");
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{trace}"
    );
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{trace}");
    assert!(logs(&out) >= 1, "the step back is logged\n{trace}");
    sim.run(SECOND);
    assert!(
        sim.state_file().is_some_and(|f| f.stepped_back),
        "{}",
        sim.trace()
    );
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    // The lid is open: no countdown, and a lower percent changes nothing.
    let mut out = sim.run(10 * MINUTE);
    out.extend(sim.send(power(false, 15)));
    let trace = sim.trace();
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert!(notices(&out).is_empty(), "{trace}");
    assert_eq!(plays(&out), 0, "{trace}");
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");

    // AC back: the hold again, as at a hold start.
    let out = sim.send(power(true, 15));
    let trace = sim.trace();
    let apply = Command::Apply {
        plan: BALANCED,
        writes: KEYS.iter().map(|&k| (k, 0)).collect(),
    };
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert_eq!(count(&out, &apply), 1, "{trace}");
    assert!(position(&out, &SYSTEM) < position(&out, &apply), "{trace}");
}

#[test]
fn battery_floor_with_the_lid_closed_hibernates_after_5_minutes_idle() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.input();
    let at = sim.now();
    let out = sim.send(power(false, 19));
    let trace = sim.trace();
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "sounds never wait\n{trace}");
    assert!(
        notices(&out).is_empty(),
        "the notice waits for the unlock\n{trace}"
    );
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    let out = sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::Floor, None)],
        "{trace}"
    );
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    never_going_to_sleep(&sim);
}

#[test]
fn ac_back_cancels_the_floor_countdown_and_holds_again() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(2 * MINUTE);
    let out = sim.send(power(true, 19));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert!(logs(&out) >= 1, "the cancel is logged\n{trace}");

    let out = sim.run(10 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn no_hold_starts_while_stepped_back() {
    let mut sim = started_with(plan(BALANCED), false, 15);
    assert!(
        sim.notices().is_empty(),
        "nothing held, nothing to say\n{}",
        sim.trace()
    );

    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{trace}"
    );
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{trace}");

    // The next turn of the same work says nothing new.
    let mut out = sim.send(idle(1));
    out.extend(sim.send(working(1)));
    out.extend(sim.run(MINUTE));
    let trace = sim.trace();
    assert!(notices(&out).is_empty(), "{trace}");
    assert_eq!(plays(&out), 0, "{trace}");
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");

    let out = sim.send(power(true, 15));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert!(
        out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );
}

#[test]
fn the_floor_follows_the_energy_saver_threshold() {
    let values = PlanValues {
        saver_threshold_dc: 40,
        ..plan(BALANCED)
    };
    let mut sim = started_with(values, false, 39);
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 40 }],
        "{trace}"
    );
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");

    // At the floor itself it still holds.
    let mut sim = started_with(values, false, 40);
    let out = sim.send(working(1));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
}

/// The user's own lid action on battery is Do nothing, on AC Sleep.
fn own_battery_do_nothing() -> PlanValues {
    PlanValues {
        lid_dc: 0,
        ..plan(BALANCED)
    }
}

/// What an apply leaves on that plan: the battery lid key is the user's own Do nothing, so the IO side skips it.
const OWN_DC_KEYS: [SettingKey; 2] = [SettingKey::LidAc, SettingKey::SleepDc];

#[test]
fn battery_floor_with_the_lid_closed_and_the_users_own_battery_do_nothing_gives_no_countdown() {
    // Power safety review finding 7, Haik 2026-11-03 23:45: the same rule as at work end. Their own Do nothing keeps
    // the closed laptop running without Wakeshot too.
    let mut sim = started_with(own_battery_do_nothing(), false, 80);
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &OWN_DC_KEYS)));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.send(power(false, 19));
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "the step back itself\n{trace}");
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{trace}");
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let out = sim.run(20 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_lid_close_after_the_step_back_with_the_users_own_battery_do_nothing_gives_no_countdown() {
    // Finding 7's second path, with our battery sleep Never left by a failed restore.
    let mut sim = started_with(own_battery_do_nothing(), false, 80);
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &OWN_DC_KEYS)));
    sim.send(power(false, 19));
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        entries(BALANCED, &OWN_DC_KEYS),
    ));
    let mut out = sim.send(lid(true));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert!(sleep_checks(&out).is_empty(), "{trace}");
}

// Item 3b: the floor's grace (`p2-6-floor-grace`, Haik 2026-11-05): under the floor the step back comes only once the
// PC has been on battery for 30 s, so a dock's short AC drop changes nothing.

const FLOOR_RESTORE: Command = Command::Restore {
    plan: None,
    why: RestoreWhy::Floor,
};

/// A Claude Code session holds on AC at `pct` on `values`, the lid open, and the apply worked, writing `keys`.
fn holding_on_ac_at(values: PlanValues, keys: &[SettingKey], pct: u8) -> Sim {
    let mut sim = started_with(values, true, pct);
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, keys)));
    sim
}

/// No part of a step back or of the hold again after it: no request change, restore, apply, lock, notice or sound.
fn nothing_of_a_step_back(sim: &Sim, out: &[Command]) {
    let acted: Vec<&Command> = out
        .iter()
        .filter(|c| {
            matches!(
                c,
                Command::Requests { .. }
                    | Command::Restore { .. }
                    | Command::Apply { .. }
                    | Command::Lock
                    | Command::Notify(_)
                    | Command::Play(_)
            )
        })
        .collect();
    assert!(acted.is_empty(), "{acted:?}\n{}", sim.trace());
    assert!(
        sim.state_file().is_some_and(|f| !f.stepped_back),
        "{}",
        sim.trace()
    );
}

/// The step back's own commands, once each: the requests off before the restore, the notice and the floor chime.
fn steps_back(sim: &Sim, out: &[Command]) {
    let trace = sim.trace();
    assert_eq!(count(out, &OFF), 1, "{trace}");
    assert_eq!(count(out, &FLOOR_RESTORE), 1, "{trace}");
    assert!(
        position(out, &OFF) < position(out, &FLOOR_RESTORE),
        "{trace}"
    );
    assert_eq!(count(out, &FLOOR_CHIME), 1, "{trace}");
}

#[test]
fn a_5_s_or_a_29_s_ac_drop_under_the_floor_changes_nothing() {
    for drop_ms in [5 * SECOND, 29 * SECOND] {
        let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 19);
        let mut out = sim.send(power(false, 19));
        out.extend(sim.run(drop_ms));
        nothing_of_a_step_back(&sim, &out);
        let mut out = sim.send(power(true, 19));
        out.extend(sim.run(10 * MINUTE));
        nothing_of_a_step_back(&sim, &out);
    }
}

#[test]
fn a_30_s_ac_drop_under_the_floor_steps_back_at_30_s_with_holds_holding_until_then() {
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 19);
    // Off the 10 s plan reads that began with the hold, so only the grace's own deadline can make the step back come
    // at 30 s.
    sim.run(3 * SECOND);
    let at = sim.now();
    let mut out = sim.send(power(false, 19));
    out.extend(sim.run(SOURCE_SETTLE_MS - 1));
    nothing_of_a_step_back(&sim, &out);

    let out = sim.run(1);
    steps_back(&sim, &out);
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{}",
        sim.trace()
    );
    assert_eq!(
        sim.sent_at(&FLOOR_CHIME)
            .iter()
            .map(|t| t.mono_ms - at.mono_ms)
            .collect::<Vec<_>>(),
        [SOURCE_SETTLE_MS],
        "the grace's own deadline, off the safety tick and the plan reads\n{}",
        sim.trace()
    );
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    // AC back: the hold again, as at a hold start.
    let out = sim.send(power(true, 19));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    assert!(
        out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
}

#[test]
fn haiks_night_five_dock_drops_under_the_floor_give_no_chime_and_no_notice() {
    // His checks, 2026-11-05: drops of 4.3, 4.4, 4.5, 4.7 and 9.65 s; each gave a chime and a notice before the grace.
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 19);
    let mut out = Vec::new();
    for drop_ms in [4_300, 4_400, 4_500, 4_700, 9_650] {
        out.extend(sim.send(power(false, 19)));
        out.extend(sim.run(drop_ms));
        out.extend(sim.send(power(true, 19)));
        out.extend(sim.run(3 * MINUTE));
    }
    let trace = sim.trace();
    assert_eq!(plays(&out), 0, "{trace}");
    assert!(notices(&out).is_empty(), "{trace}");
    nothing_of_a_step_back(&sim, &out);
}

#[test]
fn ac_that_flaps_inside_the_grace_starts_it_again_at_the_last_loss() {
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 19);
    let mut out = sim.send(power(false, 19));
    out.extend(sim.run(5 * SECOND));
    out.extend(sim.send(power(true, 19)));
    out.extend(sim.run(5 * SECOND));
    let last_loss = sim.now();
    out.extend(sim.send(power(false, 19)));
    // 30 s after the first loss, 20 s after the last.
    out.extend(sim.run(SOURCE_SETTLE_MS - 1));
    nothing_of_a_step_back(&sim, &out);

    let out = sim.run(1);
    steps_back(&sim, &out);
    assert_eq!(
        sim.sent_at(&FLOOR_CHIME)
            .iter()
            .map(|t| t.mono_ms - last_loss.mono_ms)
            .collect::<Vec<_>>(),
        [SOURCE_SETTLE_MS],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_percent_that_falls_under_the_floor_after_30_s_on_battery_steps_back_at_once() {
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 25);
    sim.send(power(false, 25));
    let out = sim.run(2 * MINUTE);
    nothing_of_a_step_back(&sim, &out);
    let out = sim.send(power(false, 19));
    steps_back(&sim, &out);
}

#[test]
fn a_percent_that_falls_under_the_floor_inside_the_grace_waits_for_its_end() {
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 25);
    let at = sim.now();
    let mut out = sim.send(power(false, 25));
    out.extend(sim.run(10 * SECOND));
    out.extend(sim.send(power(false, 19)));
    out.extend(sim.run(SOURCE_SETTLE_MS - 10 * SECOND - 1));
    nothing_of_a_step_back(&sim, &out);
    let out = sim.run(1);
    steps_back(&sim, &out);
    assert_eq!(
        sim.sent_at(&FLOOR_CHIME)[0].mono_ms - at.mono_ms,
        SOURCE_SETTLE_MS,
        "{}",
        sim.trace()
    );
}

#[test]
fn a_tray_that_starts_on_battery_under_the_floor_steps_back_at_once() {
    // No AC loss was seen, so there is no drop to wait out.
    let mut sim = started_with(plan(BALANCED), false, 19);
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{trace}"
    );
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{trace}");
}

/// The lid closes while holding on AC under the floor, and the PC is unplugged: the source lock and the step back
/// are both due 30 s after the unplug, the lock first and with the step back's sound, not the lid chime. Gives the
/// commands at that moment.
fn closed_unplug_at_30_s(sim: &mut Sim) -> Vec<Command> {
    let at = sim.now();
    let mut out = sim.send(power(false, 19));
    out.extend(sim.run(SOURCE_SETTLE_MS - 1));
    nothing_of_a_step_back(sim, &out);

    let out = sim.run(1);
    let trace = sim.trace();
    steps_back(sim, &out);
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(
        count(&out, &CHIME),
        0,
        "the lid chime says the laptop keeps running\n{trace}"
    );
    assert!(
        position(&out, &LOCK) < position(&out, &FLOOR_RESTORE),
        "the lock never waits for the restore\n{trace}"
    );
    assert_eq!(
        lock_times(sim, at).last(),
        Some(&i128::from(SOURCE_SETTLE_MS)),
        "{trace}"
    );
    out
}

#[test]
fn an_unplug_with_the_lid_closed_under_the_floor_locks_and_steps_back_at_30_s() {
    // Haik's block 4 (2026-11-05): his own Sleep on AC and battery, the lid-close lock confirmed, then the unplug.
    let mut sim = holding_on_ac_at(plan(BALANCED), &KEYS, 19);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    let out = closed_unplug_at_30_s(&mut sim);
    assert!(
        notices(&out).is_empty(),
        "the notice waits for the unlock\n{}",
        sim.trace()
    );
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    // The floor countdown, as without the grace.
    let out = sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::Floor, None)],
        "{}",
        sim.trace()
    );
    let out = sim.send(session(SessionChange::Unlocked));
    assert!(
        notices(&out).contains(&&Notice::StepBack { floor_pct: 20 }),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_docked_unplug_with_the_lid_closed_under_the_floor_locks_and_steps_back_at_30_s() {
    // The user's own Do nothing on AC, signed in on the external screen: no lid-close lock, so the unplug's lock is
    // the only one, and it is watched.
    let mut sim = holding_on_ac_at(docked(), &DOCKED_KEYS, 19);
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = closed_unplug_at_30_s(&mut sim);
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{}",
        sim.trace()
    );
    // Not confirmed in 5 s: the one retry.
    let out = sim.run(LOCK_CHECK);
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
}

// Item 4: the countdown.

#[test]
fn work_ends_with_the_lid_closed_hibernates_after_5_minutes_idle() {
    let mut sim = holding(true);
    sim.send(lid(true));
    sim.input();
    let at = sim.now();
    let out = sim.send(idle(1));
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(
        count(
            &out,
            &Command::Restore {
                plan: None,
                why: RestoreWhy::WorkEnded
            }
        ),
        1,
        "settings come back first\n{trace}"
    );
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert!(logs(&out) >= 2, "hold end and countdown start\n{trace}");
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));

    let out = sim.run(IDLE_MS - SECOND);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(plays(&out), 0, "no sound from the core\n{trace}");
    assert!(logs(&out) >= 1, "the due countdown is logged\n{trace}");
    never_going_to_sleep(&sim);
}

#[test]
fn work_ended_with_the_lid_closed_does_not_hibernate_while_typing() {
    let mut sim = work_ended_closed(true);
    let mut out = Vec::new();
    for _ in 0..10 {
        sim.input();
        out.extend(sim.run(MINUTE));
    }
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    sim.input();
    let stopped = sim.now();
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_check_times(&sim, stopped),
        [i128::from(IDLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn work_ended_after_a_long_idle_hibernates_at_once() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.run(20 * MINUTE);
    let ended = sim.now();
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.run(SECOND);
    assert_eq!(sleep_check_times(&sim, ended), [0], "{}", sim.trace());
}

#[test]
fn a_resume_during_the_countdown_restarts_the_idle_count() {
    let mut sim = work_ended_closed(true);
    sim.input();
    sim.run(2 * MINUTE);
    sim.suspend(HOUR);
    let resumed = sim.now();
    let out = sim.run(IDLE_MS - SECOND);
    assert!(
        sleep_checks(&out).is_empty(),
        "a resume counts as input\n{}",
        sim.trace()
    );

    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, Some(resumed))],
        "{trace}"
    );
    assert_eq!(
        sleep_check_times(&sim, resumed),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
}

#[test]
fn work_ends_with_the_users_own_do_nothing_gives_no_countdown() {
    let mut sim = holding_docked(true);
    sim.send(lid(true));
    let mut out = sim.send(idle(1));
    out.extend(sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![])));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &OFF), 1, "only the release\n{trace}");
}

#[test]
fn a_new_hold_cancels_the_countdown() {
    let mut sim = work_ended_closed(true);
    sim.run(MINUTE);
    let out = sim.send(working(1));
    assert!(logs(&out) >= 1, "the cancel is logged\n{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn opening_the_lid_cancels_the_countdown() {
    let mut sim = work_ended_closed(false);
    sim.run(MINUTE);
    let out = sim.send(lid(false));
    assert!(logs(&out) >= 1, "the cancel is logged\n{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn another_session_on_the_console_holds_the_countdown_back() {
    let mut sim = work_ended_closed(true);
    sim.run(MINUTE);
    let mut out = sim.send(session(SessionChange::ConsoleLeft));
    out.extend(sim.run(20 * MINUTE));
    assert!(
        sleep_checks(&out).is_empty(),
        "never hibernate the other user's PC\n{}",
        sim.trace()
    );
    let back = sim.now();
    sim.send(session(SessionChange::ConsoleBack));
    sim.run(SECOND);
    assert_eq!(sleep_check_times(&sim, back), [0], "{}", sim.trace());
}

#[test]
fn a_power_source_change_onto_the_users_own_do_nothing_cancels_the_countdown_and_back_starts_it() {
    // Orchestrator decision 2026-11-03 21:22, Haik's yes 21:23. Work ends on battery with the lid closed; the user's
    // own battery action is Sleep, their own AC action Do nothing.
    let mut sim = holding_docked(false);
    sim.send(lid(true));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.run(MINUTE);

    // Docked: their own Do nothing keeps it running, so only the release.
    let out = sim.send(power(true, 80));
    assert!(logs(&out) >= 1, "the cancel is logged\n{}", sim.trace());
    let out = sim.run(10 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    // A short AC drop does not start it again.
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(3 * SECOND));
    out.extend(sim.send(power(true, 80)));
    out.extend(sim.run(5 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    // Unplugged for good: the countdown again once the source lasted; idle is long past 5 minutes.
    let unplugged = sim.now();
    let out = sim.send(power(false, 80));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    sim.run(SOURCE_SETTLE_MS + SECOND);
    assert_eq!(
        sleep_check_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_power_source_change_onto_the_users_own_sleep_keeps_the_countdown() {
    // The test's user values: Sleep on AC too, so docking changes nothing.
    let mut sim = work_ended_closed(false);
    sim.input();
    let at = sim.now();
    sim.send(power(true, 80));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn work_ended_on_the_users_own_ac_do_nothing_then_unplugged_gives_no_countdown() {
    // Power safety review finding 4, kept as it is: their own AC Do nothing kept the laptop running at the close, so
    // Wakeshot changed nothing and only releases (product, work end). After an unplug Windows' own rules apply, as if
    // Wakeshot were not there (Haik, 2026-11-03 23:45).
    let mut sim = holding_docked(true);
    sim.input();
    sim.send(lid(true));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.run(10 * MINUTE);
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(
        count(&out, &LOCK),
        0,
        "nothing of ours keeps it running\n{trace}"
    );
}

// Item 5: the sleep check, the short timeouts and the fallback hibernate (`p2-6-sleep-route`).

/// When each fallback `Hibernate` went out, in run time since `from`.
fn fallback_times(sim: &Sim, from: Now) -> Vec<i128> {
    sim.commands()
        .filter(|(_, c)| matches!(c, Command::Hibernate { .. }))
        .map(|(at, _)| i128::from(at.mono_ms) - i128::from(from.mono_ms))
        .collect()
}

/// The sleep route's apply to Balanced without the keys the user took back.
fn short_apply_without(released: &[SettingKey]) -> Command {
    Command::Apply {
        plan: BALANCED,
        writes: SettingKey::SHORT
            .iter()
            .filter(|k| !released.contains(k))
            .map(|&k| (k, crate::settings::SHORT_TIMEOUT_S))
            .collect(),
    }
}

#[test]
fn a_passed_sleep_check_writes_the_short_timeouts_and_hibernates_5_minutes_after_them() {
    let mut sim = work_ended_closed(false);
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1, "{}", sim.trace());
    let written = sim.now();
    let out = short_written(&mut sim);
    let trace = sim.trace();
    assert!(fallbacks(&out).is_empty(), "{trace}");
    assert_eq!(
        plays(&out),
        0,
        "the sound is the hibernate thread's\n{trace}"
    );
    assert!(
        logs(&out) >= 2,
        "the check and the write are logged\n{trace}"
    );

    let out = sim.run(FALLBACK_MS - SECOND);
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
    assert_eq!(count(&out, &SHORT_BACK), 0, "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(
        fallbacks(&out),
        [&fallback(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
    assert_eq!(
        fallback_times(&sim, written),
        [i128::from(FALLBACK_MS)],
        "{trace}"
    );
    assert_eq!(
        count(&out, &SHORT_BACK),
        0,
        "they stay for Windows\n{trace}"
    );
    never_going_to_sleep(&sim);
}

#[test]
fn a_sleep_check_skipped_for_input_goes_on() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Err(HibernateSkip::InputSeen)));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    assert_eq!(count(&out, &short_apply()), 0, "{}", sim.trace());
    sim.input();
    let typed = sim.now();
    let out = sim.run(IDLE_MS + MINUTE);
    assert_eq!(sleep_checks(&out).len(), 1, "{}", sim.trace());
    assert_eq!(
        sleep_check_times(&sim, typed).last(),
        Some(&i128::from(IDLE_MS)),
        "{}",
        sim.trace()
    );
}

/// `p2-6-props-rare`: the user comes back while the sleep check is out, before its answer, which the hibernate thread
/// took before that: input whose reading the core has, or an unlock (a sign-in by face gives no input Windows counts).
/// That pass is old: no 1-minute timeouts right after it, no fallback, and the countdown waits 5 minutes from it. An
/// unknown lid passes as `Ok` does, so it is old the same way.
#[test]
fn a_sleep_check_that_passed_before_the_user_came_back_counts_as_a_skip_for_input() {
    for (unlock, result) in [
        (false, Ok(())),
        (true, Ok(())),
        (false, Err(HibernateSkip::LidUnknown)),
    ] {
        let case = format!("unlock {unlock}, {result:?}");
        let mut sim = work_ended_closed(true);
        if unlock {
            sim.send(session(SessionChange::Locked));
        }
        let out = sim.run(IDLE_MS);
        assert_eq!(sleep_checks(&out).len(), 1, "{case}\n{}", sim.trace());
        let back = sim.now();
        let mut out = if unlock {
            sim.send(session(SessionChange::Unlocked))
        } else {
            sim.input();
            sim.run(SAFETY_TICK_MS)
        };
        out.extend(sim.send(checked(result)));
        out.extend(sim.run(IDLE_MS - SAFETY_TICK_MS - SECOND));
        let trace = sim.trace();
        assert_eq!(count(&out, &short_apply()), 0, "{case}\n{trace}");
        assert!(fallbacks(&out).is_empty(), "{case}\n{trace}");
        assert!(sleep_checks(&out).is_empty(), "{case}\n{trace}");
        sim.run(MINUTE);
        assert_eq!(
            sleep_check_times(&sim, back).last(),
            Some(&i128::from(IDLE_MS)),
            "{case}\n{}",
            sim.trace()
        );
    }
}

#[test]
fn a_sleep_check_skipped_with_the_lid_open_ends_the_countdown() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let mut out = sim.send(checked(Err(HibernateSkip::LidOpen)));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(notices(&out).is_empty(), "{trace}");
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &short_apply()), 0, "{trace}");
}

#[test]
fn a_sleep_check_with_the_lid_unknown_still_writes_the_short_timeouts() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Err(HibernateSkip::LidUnknown)));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
}

#[test]
fn a_sleep_check_skipped_on_ac_ends_only_the_floor_countdown() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    let mut out = sim.send(checked(Err(HibernateSkip::OnAc)));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    assert_eq!(count(&out, &short_apply()), 0, "{}", sim.trace());
}

#[test]
fn a_sleep_check_skipped_above_the_floor_waits_for_a_new_battery_reading() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Err(HibernateSkip::AboveFloor)));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.run(10 * MINUTE);
    assert!(
        sleep_checks(&out).is_empty(),
        "the two readings disagree\n{}",
        sim.trace()
    );

    // A new reading under the floor: the countdown is still due.
    sim.send(power(false, 18));
    let out = sim.run(SAFETY_TICK_MS);
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::Floor, None)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_fallback_that_does_not_happen_shows_a_notice_and_keeps_the_short_timeouts() {
    for end in [
        HibernateEnd::NotAllowed,
        HibernateEnd::Failed(SUSPEND_FAILED),
        HibernateEnd::Skipped(HibernateSkip::LidUnknown),
    ] {
        let mut sim = work_ended_closed(true);
        sim.run(IDLE_MS + MINUTE);
        let out = to_fallback(&mut sim);
        assert_eq!(fallbacks(&out).len(), 1, "{end:?}\n{}", sim.trace());
        let out = sim.send(hibernated(end));
        let trace = sim.trace();
        assert_eq!(
            notices(&out),
            [&Notice::HibernateFailed],
            "{end:?}\n{trace}"
        );
        assert_eq!(count(&out, &SHORT_BACK), 0, "{end:?}\n{trace}");
        assert!(logs(&out) >= 1, "{end:?}\n{trace}");
        let out = sim.run(20 * MINUTE);
        let trace = sim.trace();
        assert!(
            fallbacks(&out).is_empty(),
            "one per countdown\n{end:?}\n{trace}"
        );
        assert!(sleep_checks(&out).is_empty(), "{end:?}\n{trace}");
        assert_eq!(count(&out, &SHORT_BACK), 0, "{end:?}\n{trace}");

        // Someone types on an external screen: their own timeouts back; once they leave, the short ones again,
        // with no second hibernate and no second notice.
        sim.input();
        let out = sim.run(SAFETY_TICK_MS);
        assert_eq!(count(&out, &SHORT_BACK), 1, "{end:?}\n{}", sim.trace());
        let out = sim.run(IDLE_MS);
        assert_eq!(sleep_checks(&out).len(), 1, "{end:?}\n{}", sim.trace());
        let mut out = short_written(&mut sim);
        out.extend(sim.run(20 * MINUTE));
        let trace = sim.trace();
        assert!(fallbacks(&out).is_empty(), "{end:?}\n{trace}");
        assert!(notices(&out).is_empty(), "{end:?}\n{trace}");
    }
}

#[test]
fn a_fallback_skipped_for_input_puts_the_short_timeouts_back_and_counts_again() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    to_fallback(&mut sim);
    let out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::InputSeen)));
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    sim.input();
    let typed = sim.now();
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_check_times(&sim, typed).last(),
        Some(&i128::from(IDLE_MS)),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_fallback_skipped_with_the_lid_open_puts_the_short_timeouts_back() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    to_fallback(&mut sim);
    let mut out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::LidOpen)));
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_hibernate_that_resumed_counts_as_a_resume() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    to_fallback(&mut sim);
    let back = sim.now();
    let out = sim.send(hibernated(HibernateEnd::Resumed));
    assert_eq!(
        count(&out, &SHORT_BACK),
        1,
        "the countdown is done\n{}",
        sim.trace()
    );
    let out = sim.run(20 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "it ended\n{}", sim.trace());
    assert!(fallbacks(&out).is_empty(), "it ended\n{}", sim.trace());

    // The lid stayed closed; new work ends again: idle counts from that resume.
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim.send(idle(1));
    let out = sim.run(SECOND);
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, Some(back))],
        "{}",
        sim.trace()
    );
}

#[test]
fn the_short_timeouts_go_back_when_the_lid_opens() {
    let mut sim = work_ended_closed(false);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.run(MINUTE);
    let out = sim.send(lid(false));
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
    assert_eq!(count(&out, &SHORT_BACK), 0, "once\n{}", sim.trace());
}

#[test]
fn an_unlock_puts_the_short_timeouts_back_and_the_countdown_counts_from_it() {
    let mut sim = work_ended_closed(true);
    sim.send(session(SessionChange::Locked));
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.run(MINUTE);
    // A sign-in by face gives no input Windows counts.
    let unlocked = sim.now();
    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    let out = sim.run(IDLE_MS - SECOND);
    assert!(
        sleep_checks(&out).is_empty(),
        "the user is there\n{}",
        sim.trace()
    );
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, Some(unlocked))],
        "{}",
        sim.trace()
    );
}

#[test]
fn input_after_the_write_puts_them_back_and_waits_5_minutes_again() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.run(MINUTE);
    sim.input();
    let typed = sim.now();
    let out = sim.run(SAFETY_TICK_MS);
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    let out = sim.run(IDLE_MS);
    let trace = sim.trace();
    assert!(fallbacks(&out).is_empty(), "{trace}");
    assert_eq!(
        sleep_check_times(&sim, typed).last(),
        Some(&i128::from(IDLE_MS)),
        "{trace}"
    );
}

#[test]
fn a_hold_that_starts_puts_them_back_before_its_apply() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    let out = sim.send(working(2));
    let hold_apply = Command::Apply {
        plan: BALANCED,
        writes: KEYS.iter().map(|&k| (k, 0)).collect(),
    };
    let trace = sim.trace();
    assert_eq!(count(&out, &SHORT_BACK), 1, "{trace}");
    assert!(
        position(&out, &SHORT_BACK) < position(&out, &hold_apply),
        "{trace}"
    );
    let out = sim.run(20 * MINUTE);
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_wake_with_the_lid_closed_keeps_them_and_the_fallback_counts_from_the_resume() {
    let mut sim = work_ended_closed(false);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.run(MINUTE);
    let out = sim.suspend(30 * MINUTE);
    assert_eq!(count(&out, &SHORT_BACK), 0, "a wake alone\n{}", sim.trace());
    let woke = sim.now();
    let out = sim.run(FALLBACK_MS - SECOND);
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
    assert_eq!(count(&out, &SHORT_BACK), 0, "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    assert_eq!(
        fallbacks(&out),
        [&fallback(CountdownReason::WorkEnded, Some(woke))],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_sleep_seen_only_by_the_clock_gap_starts_the_fallback_again() {
    // The `Resumed` post was lost: the tick after the suspend sees the wall clock 10 minutes past run time.
    let mut sim = work_ended_closed(false);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.run(MINUTE);
    sim.send(Event::Suspending);
    let woke = sim.now();
    sim.set_wall(woke.wall_ms + 10 * 60_000);
    sim.run(FALLBACK_MS + SAFETY_TICK_MS);
    let times = fallback_times(&sim, woke);
    let trace = sim.trace();
    assert_eq!(times.len(), 1, "{trace}");
    assert!(
        (i128::from(FALLBACK_MS)..=i128::from(FALLBACK_MS + SAFETY_TICK_MS)).contains(&times[0]),
        "{trace}"
    );

    // A `Suspending` the clocks do not confirm leaves the fallback where it was.
    let mut sim = work_ended_closed(false);
    sim.run(IDLE_MS + MINUTE);
    let written_too = sim.now();
    short_written(&mut sim);
    sim.run(MINUTE);
    sim.send(Event::Suspending);
    sim.run(FALLBACK_MS);
    assert_eq!(
        fallback_times(&sim, written_too),
        [i128::from(FALLBACK_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_users_change_to_a_short_timeout_is_not_written_again_in_this_countdown() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    let changed = OwnedKey {
        plan: BALANCED,
        key: SettingKey::VideoIdleDc,
    };
    let out = sim.send(Event::UserChanged(changed));
    assert_eq!(
        notices(&out),
        [&Notice::UserChanged(SettingKey::VideoIdleDc)],
        "{}",
        sim.trace()
    );
    sim.input();
    sim.run(SAFETY_TICK_MS);
    let out = sim.run(IDLE_MS);
    assert_eq!(sleep_checks(&out).len(), 1, "{}", sim.trace());
    let out = sim.send(checked(Ok(())));
    assert_eq!(
        count(&out, &short_apply_without(&[SettingKey::VideoIdleDc])),
        1,
        "{}",
        sim.trace()
    );
}

#[test]
fn the_floor_countdown_writes_them_too_and_ac_back_puts_them_back() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::Floor, None)],
        "{}",
        sim.trace()
    );
    short_written(&mut sim);
    let out = sim.send(power(true, 19));
    assert_eq!(count(&out, &SHORT_BACK), 1, "{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(fallbacks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn short_keys_the_user_changes_in_a_step_back_never_stop_the_hold() {
    // Review minor 4 of `p2-6-sleep-route`: three short keys the user changes while the floor countdown's 1-minute
    // timeouts are in force are released for the episode, but none of them is a hold key, so the hold applies again
    // when AC comes back.
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    let mut out = Vec::new();
    for key in [
        SettingKey::SleepAc,
        SettingKey::VideoIdleAc,
        SettingKey::VideoIdleDc,
    ] {
        out.extend(sim.send(Event::UserChanged(OwnedKey {
            plan: BALANCED,
            key,
        })));
    }
    out.extend(sim.send(power(true, 19)));
    let hold_apply = Command::Apply {
        plan: BALANCED,
        writes: KEYS.iter().map(|&k| (k, 0)).collect(),
    };
    let trace = sim.trace();
    assert_eq!(count(&out, &hold_apply), 1, "{trace}");
    assert!(
        !out.iter()
            .any(|c| matches!(c, Command::Log(l) if l.starts_with("Every key is the user's"))),
        "{trace}"
    );
}

#[test]
fn short_timeouts_that_cannot_be_written_hibernate_at_once() {
    // Another session's tray owns the settings: no write, so the plain hibernate as before the sleep route.
    let mut sim = Sim::new();
    sim.send(
        Start {
            owner: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    sim.run(OFF_THE_TICK);
    sim.send(read(plan(BALANCED)));
    sim.send(working(1));
    sim.send(lid(true));
    sim.send(idle(1));
    sim.run(IDLE_MS + MINUTE);
    let commands = all(&sim);
    let trace = sim.trace();
    assert!(sleep_checks(&commands).is_empty(), "{trace}");
    assert_eq!(
        fallbacks(&commands),
        [&fallback(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
}

#[test]
fn a_failed_short_write_makes_the_fallback_due_at_once() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
    let failed = sim.now();
    sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(RESTORE_FAILED)),
        vec![],
    ));
    sim.run(SECOND);
    assert_eq!(fallback_times(&sim, failed), [0], "{}", sim.trace());
}

#[test]
fn a_plan_switch_moves_the_short_timeouts_to_the_new_plan() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    let out = sim.send(read(plan(sim::OTHER_PLAN)));
    let old_back = Command::Restore {
        plan: Some(BALANCED),
        why: RestoreWhy::PlanSwitch,
    };
    let new_apply = Command::Apply {
        plan: sim::OTHER_PLAN,
        writes: SettingKey::SHORT
            .iter()
            .map(|&k| (k, crate::settings::SHORT_TIMEOUT_S))
            .collect(),
    };
    let trace = sim.trace();
    assert_eq!(count(&out, &old_back), 1, "{trace}");
    assert_eq!(count(&out, &new_apply), 1, "{trace}");
    assert!(
        position(&out, &old_back) < position(&out, &new_apply),
        "{trace}"
    );
}

#[test]
fn a_short_key_released_while_the_switchs_short_apply_is_on_its_way_is_written_again_without_it() {
    // `p2-6-props-flake`: the switch's read posts `Plan` before its check's `UserChanged`, so the switch's short apply
    // goes out with the key the user just took back on the old plan. As for a hold, the release sends the short
    // timeouts again without it, which restores the new plan's entry for it.
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    short_written(&mut sim);
    sim.send(read(plan(sim::OTHER_PLAN)));
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::SleepDc,
    }));
    let again = Command::Apply {
        plan: sim::OTHER_PLAN,
        writes: SettingKey::SHORT
            .iter()
            .filter(|&&k| k != SettingKey::SleepDc)
            .map(|&k| (k, crate::settings::SHORT_TIMEOUT_S))
            .collect(),
    };
    let trace = sim.trace();
    assert_eq!(count(&out, &again), 1, "{trace}");
    assert_eq!(
        notices(&out),
        [&Notice::UserChanged(SettingKey::SleepDc)],
        "{trace}"
    );

    // Both answered in order: nothing more to write or restore.
    let mut out = sim.send(applied(
        sim::OTHER_PLAN,
        Ok(()),
        short_entries(sim::OTHER_PLAN),
    ));
    let mut left = short_entries(sim::OTHER_PLAN);
    left.retain(|e| e.key != SettingKey::SleepDc);
    out.extend(sim.send(applied(sim::OTHER_PLAN, Ok(()), left)));
    assert!(
        !out.iter()
            .any(|c| matches!(c, Command::Apply { .. } | Command::Restore { .. })),
        "{}",
        sim.trace()
    );
}

#[test]
fn every_short_key_released_while_a_short_apply_is_on_its_way_restores() {
    // With no short key left to write no apply comes, so a restore takes back what the one on its way writes.
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
    let mut out = Vec::new();
    for key in SettingKey::SHORT {
        out = sim.send(Event::UserChanged(OwnedKey {
            plan: BALANCED,
            key,
        }));
    }
    let trace = sim.trace();
    assert_eq!(count(&out, &SHORT_BACK), 1, "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );

    // All answered in order (the first apply, then one per key until the last): nothing is left to write again.
    let mut out = Vec::new();
    for _ in 0..SettingKey::SHORT.len() {
        out.extend(sim.send(applied(BALANCED, Ok(()), vec![])));
    }
    out.extend(sim.send(restored(None, RestoreWhy::ShortTimeouts, Ok(()), vec![])));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_hold_key_released_while_the_floor_short_apply_is_out_keeps_the_short_timeouts() {
    // `p2-6-reset-gaps`, gap B: the floor step back's restore failed and its retries keep failing, so the hold's
    // entries stay. The user takes back the AC lid action and battery sleep, then, while the countdown's short apply is
    // out, the battery lid action. Every hold key is the user's now, so a restore takes back what Wakeshot may still
    // hold, which takes the short timeouts too. The countdown still wants them: without them Windows' idle sleep never
    // comes, and the fallback hibernates.
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(power(false, 19));
    sim.send(restored(
        None,
        RestoreWhy::Floor,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    for key in [SettingKey::LidAc, SettingKey::SleepDc] {
        sim.send(Event::UserChanged(OwnedKey {
            plan: BALANCED,
            key,
        }));
    }
    sim.run_failing_restores(IDLE_MS + MINUTE, RESTORE_FAILED);
    let short = short_apply_without(&[SettingKey::SleepDc]);
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short), 1, "{}", sim.trace());
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    let every = Command::Restore {
        plan: None,
        why: RestoreWhy::WorkEnded,
    };
    assert_eq!(count(&out, &every), 1, "{}", sim.trace());

    // In order: the short apply, then the restore, which takes them back.
    let mut left = short_entries(BALANCED);
    left.retain(|e| e.key != SettingKey::SleepDc);
    sim.send(applied(BALANCED, Ok(()), left));
    let out = sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    let trace = sim.trace();
    assert_eq!(count(&out, &short), 1, "{trace}");
    assert_eq!(count(&out, &SHORT_BACK), 0, "{trace}");
}

#[test]
fn a_restore_retry_that_works_while_the_short_timeouts_are_on_writes_them_again() {
    // `p2-6-reset-gaps`, the same shape as gap B: the work end's restore failed and its retries failed until the
    // countdown wrote the short timeouts. The next retry takes every entry, theirs too.
    let failed_work_end = || {
        let mut sim = holding(false);
        sim.send(lid(true));
        sim.send(session(SessionChange::Locked));
        sim.send(idle(1));
        sim.send(restored(
            None,
            RestoreWhy::WorkEnded,
            Err(RESTORE_FAILED),
            entries(BALANCED, &KEYS),
        ));
        sim
    };
    let mut sim = failed_work_end();
    sim.run_failing_restores(IDLE_MS + MINUTE, RESTORE_FAILED);
    short_written(&mut sim);
    let retry = Command::Restore {
        plan: None,
        why: RestoreWhy::WorkEnded,
    };
    let sent = sim.run(SAFETY_TICK_MS);
    assert!(count(&sent, &retry) >= 2, "{}", sim.trace());
    // One that fails took nothing, or not all: the next decides.
    let out = sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        vec![],
    ));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
    let mut out = Vec::new();
    for _ in 1..count(&sent, &retry) {
        out.extend(sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![])));
    }
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());

    // The user's own restore wins: nothing goes out again after it.
    sim.send(applied(BALANCED, Ok(()), short_entries(BALANCED)));
    sim.send(Event::Menu(MenuCommand::Restore));
    let out = sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    let trace = sim.trace();
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );

    // A retry sent before the short apply runs before it: the apply writes them, and no second one goes out.
    let mut sim = failed_work_end();
    sim.run_failing_restores(IDLE_MS - SAFETY_TICK_MS, RESTORE_FAILED);
    let sent = sim.run(MINUTE);
    assert_eq!(sleep_checks(&sent).len(), 1, "{}", sim.trace());
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
    let mut out = Vec::new();
    for _ in 0..count(&sent, &retry) {
        out.extend(sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![])));
    }
    out.extend(sim.send(applied(BALANCED, Ok(()), short_entries(BALANCED))));
    let trace = sim.trace();
    assert!(count(&sent, &retry) >= 1, "{trace}");
    assert!(
        !out.iter()
            .any(|c| matches!(c, Command::Apply { .. } | Command::Restore { .. })),
        "{trace}"
    );
}

#[test]
fn a_lost_journals_reset_over_the_short_timeouts_writes_them_again() {
    // `p2-6-reset-gaps`, gap A for the sleep route: the short timeouts move to a new plan, and the old plan's restore
    // fails to load the journal before any of their applies answered, so the journal is unknown. The reset goes out 2
    // minutes later, behind the new plan's apply, and its own load fails, so it puts them to the defaults too.
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
    sim.send(read(plan(sim::OTHER_PLAN)));
    sim.send(applied(BALANCED, Err(ApplyError::PlanChanged), vec![]));
    sim.send(restored(
        Some(BALANCED),
        RestoreWhy::PlanSwitch,
        Err(RESTORE_FAILED),
        vec![],
    ));
    let out = sim.run_failing_restores(2 * MINUTE + 5 * SECOND, RESTORE_FAILED);
    let reset = Command::ResetLost { damaged: false };
    assert_eq!(count(&out, &reset), 1, "{}", sim.trace());
    sim.send(applied(
        sim::OTHER_PLAN,
        Ok(()),
        short_entries(sim::OTHER_PLAN),
    ));
    let out = sim.send(reset_done(&SettingKey::SHORT, Ok(())));
    let again = Command::Apply {
        plan: sim::OTHER_PLAN,
        writes: SettingKey::SHORT
            .iter()
            .map(|&k| (k, crate::settings::SHORT_TIMEOUT_S))
            .collect(),
    };
    assert_eq!(count(&out, &again), 1, "{}", sim.trace());
}

#[test]
fn a_failed_lock_step_back_ends_in_the_short_timeouts_too() {
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{}",
        sim.trace()
    );
    let out = sim.send(checked(Ok(())));
    assert_eq!(count(&out, &short_apply()), 1, "{}", sim.trace());
}

// Item 6: Keep screen on.

/// Keep awake with Keep screen on, applied, on battery: the test's battery sleep is 10 minutes.
fn screen_on() -> Sim {
    let mut sim = started_with(plan(BALANCED), false, 80);
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    sim.watch();
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim
}

#[test]
fn keep_screen_on_locks_once_per_idle_period() {
    let mut sim = screen_on();
    assert!(sim.requests().display, "{}", sim.trace());
    sim.input();
    let first = sim.now();
    let out = sim.run(10 * MINUTE - SECOND);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(notices(&out), [&Notice::IdleLocked], "{trace}");
    assert_eq!(
        lock_times(&sim, first),
        [i128::from(10 * MINUTE)],
        "{trace}"
    );
    assert!(logs(&out) >= 1, "the lock is logged\n{trace}");

    let out = sim.run(30 * MINUTE);
    assert_eq!(
        count(&out, &LOCK),
        0,
        "once per idle period\n{}",
        sim.trace()
    );

    sim.input();
    let second = sim.now();
    sim.run(10 * MINUTE + SECOND);
    assert_eq!(
        lock_times(&sim, second).last(),
        Some(&i128::from(10 * MINUTE)),
        "{}",
        sim.trace()
    );
}

#[test]
fn keep_screen_on_with_sleep_timeout_never_never_locks() {
    let mut sim = started_with(
        PlanValues {
            sleep_ac: 0,
            ..plan(BALANCED)
        },
        true,
        80,
    );
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    sim.watch();
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    let out = sim.run(4 * HOUR);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
}

#[test]
fn a_failed_lock_shows_a_notice() {
    let mut sim = holding(true);
    sim.send(lid(true));
    let out = sim.send(Event::Done(Outcome::Locked(Err(LOCK_FAILED))));
    let trace = sim.trace();
    assert_eq!(notices(&out), [&Notice::LockFailed], "{trace}");
    assert!(logs(&out) >= 1, "{trace}");
    let out = sim.send(Event::Done(Outcome::Locked(Ok(()))));
    assert!(out.is_empty(), "{}", sim.trace());

    // A lock sent into a session already locked that fails changes nothing: logged, no notice now or at the unlock.
    sim.send(session(SessionChange::Locked));
    let mut out = sim.send(Event::Done(Outcome::Locked(Err(LOCK_FAILED))));
    assert!(logs(&out) >= 1, "{}", sim.trace());
    out.extend(sim.send(session(SessionChange::Unlocked)));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
}

// Power safety review finding 1: a closed laptop never runs unlocked in a bag (section 8, "Lock fails").

fn lock_answer(result: Result<(), Failure>) -> Event {
    Event::Done(Outcome::Locked(result))
}

/// Holding with the test's user values, the lid closed: lock and chime, and the session has not reported `Locked` yet.
fn closed_unconfirmed(ac: bool) -> Sim {
    let mut sim = holding(ac);
    sim.input();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim
}

/// `Lock` failed twice: the second answer, then the tick that steps back. Gives both steps' commands.
fn failed_twice(sim: &mut Sim) -> Vec<Command> {
    sim.send(lock_answer(Err(LOCK_FAILED)));
    let out = sim.run(SECOND);
    assert_eq!(count(&out, &LOCK), 1, "the retry\n{}", sim.trace());
    let mut out = sim.send(lock_answer(Err(LOCK_FAILED)));
    out.extend(sim.run(SECOND));
    out
}

#[test]
fn a_failed_lid_close_lock_is_sent_once_more_at_the_next_tick() {
    let mut sim = closed_unconfirmed(false);
    let out = sim.send(lock_answer(Err(LOCK_FAILED)));
    let trace = sim.trace();
    assert_eq!(notices(&out), [&Notice::LockFailed], "{trace}");
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    let failed = sim.now();
    let out = sim.run(SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(
        plays(&out),
        0,
        "the chime went with the first lock\n{trace}"
    );
    assert_eq!(lock_times(&sim, failed).last(), Some(&0), "{trace}");
    assert!(logs(&out) >= 1, "the retry is logged\n{trace}");

    // It worked: the hold goes on, with no second notice.
    let mut out = sim.send(lock_answer(Ok(())));
    out.extend(sim.send(session(SessionChange::Locked)));
    out.extend(sim.run(10 * MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert_eq!(count(&out, &OFF), 0, "{trace}");
    assert!(sim.requests().system, "{trace}");
    assert_eq!(
        sim.notices()
            .into_iter()
            .filter(|n| **n == Notice::LockFailed)
            .count(),
        1,
        "{trace}"
    );
}

#[test]
fn a_lid_close_lock_that_fails_twice_steps_back_and_hibernates_after_5_minutes_idle() {
    let mut sim = closed_unconfirmed(false);
    let input = sim.now();
    let out = failed_twice(&mut sim);
    let trace = sim.trace();
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(count(&out, &restore), 1, "{trace}");
    assert!(position(&out, &OFF) < position(&out, &restore), "{trace}");
    assert!(
        notices(&out).is_empty(),
        "one LockFailed is enough\n{trace}"
    );
    assert_eq!(count(&out, &LOCK), 0, "no third lock\n{trace}");
    assert!(logs(&out) >= 1, "the step back is logged\n{trace}");
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    // No hold holds while stepped back, and a new one does not end the countdown.
    let both = sim::sources(vec![
        (claude(1, Activity::Working), Alive::Yes),
        (claude(2, Activity::Working), Alive::Yes),
    ]);
    let mut out = sim.send(both);
    out.extend(sim.run(IDLE_MS + MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
    assert_eq!(
        sleep_check_times(&sim, input),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    never_going_to_sleep(&sim);
}

#[test]
fn a_lock_the_session_does_not_confirm_within_5_s_is_sent_once_more_then_steps_back() {
    // `LockWorkStation` only starts the lock: `Ok` with no `Session(Locked)` counts as a failure.
    let mut sim = closed_unconfirmed(true);
    let locked = sim.now();
    sim.send(lock_answer(Ok(())));
    let out = sim.run(LOCK_CHECK - 1);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let out = sim.run(SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(notices(&out), [&Notice::LockFailed], "{trace}");
    assert_eq!(
        lock_times(&sim, locked).last(),
        Some(&i128::from(LOCK_CHECK)),
        "{trace}"
    );

    sim.send(lock_answer(Ok(())));
    let out = sim.run(LOCK_CHECK - SECOND);
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
    let out = sim.run(SECOND);
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(
        count(
            &out,
            &Command::Restore {
                plan: None,
                why: RestoreWhy::Floor
            }
        ),
        1,
        "{trace}"
    );
    assert!(notices(&out).is_empty(), "{trace}");
}

#[test]
fn nothing_is_applied_while_stepped_back_after_a_failed_lock() {
    // An apply would put our Do nothing back on a closed laptop nobody could lock.
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let mut out = sim.send(idle(1));
    out.extend(sim.send(working(1)));
    out.extend(sim.send(read(plan(sim::OTHER_PLAN))));
    out.extend(sim.run(MINUTE));
    let trace = sim.trace();
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{trace}"
    );
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");
}

#[test]
fn the_session_locking_or_the_lid_opening_ends_the_step_back() {
    let apply = Command::Apply {
        plan: BALANCED,
        writes: KEYS.iter().map(|&k| (k, 0)).collect(),
    };
    // Windows locked it after all (a late `Locked`, or its own lock at screen off): hold again as at a hold start.
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let mut out = sim.send(session(SessionChange::Locked));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert_eq!(count(&out, &apply), 1, "{trace}");
    assert!(position(&out, &SYSTEM) < position(&out, &apply), "{trace}");
    out.extend(sim.run(20 * MINUTE));
    assert!(
        sleep_checks(&out).is_empty(),
        "holding again ends the countdown\n{}",
        sim.trace()
    );

    // The lid opens: the user is at the laptop.
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let out = sim.send(lid(false));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert_eq!(count(&out, &apply), 1, "{trace}");
}

#[test]
fn a_failed_lock_needs_no_retry_once_the_lid_opens_or_the_session_locks() {
    let mut sim = closed_unconfirmed(false);
    sim.send(lock_answer(Err(LOCK_FAILED)));
    let mut out = sim.send(lid(false));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert!(sim.requests().system, "{}", sim.trace());

    let mut sim = closed_unconfirmed(false);
    sim.send(lock_answer(Err(LOCK_FAILED)));
    let mut out = sim.send(session(SessionChange::Locked));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert!(sim.requests().system, "{}", sim.trace());

    // The user took the battery lid action back (their own value is in force now): no retry, and the hold goes on.
    let mut sim = closed_unconfirmed(false);
    sim.send(lock_answer(Err(LOCK_FAILED)));
    let mut out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
}

#[test]
fn a_failed_source_lock_is_sent_once_more_too() {
    let mut sim = holding(true);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.send(power(false, 80));
    let out = sim.run(SOURCE_SETTLE_MS);
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    let out = sim.send(lock_answer(Err(LOCK_FAILED)));
    assert_eq!(notices(&out), [&Notice::LockFailed], "{}", sim.trace());
    let out = sim.run(SECOND);
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
}

// Power safety verify minor 4: the lock step back across a power source change.

#[test]
fn a_source_lock_that_fails_twice_steps_back_and_a_redock_onto_the_users_own_do_nothing_pauses_it()
{
    // Own lid action Do nothing on AC, Sleep on battery. The closed laptop is unplugged: the source lock fails twice.
    let mut sim = holding_docked(true);
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.send(power(false, 80));
    let out = sim.run(SOURCE_SETTLE_MS);
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    assert_eq!(count(&out, &CHIME), 1, "{}", sim.trace());
    let out = failed_twice(&mut sim);
    let trace = sim.trace();
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(count(&out, &restore), 1, "{trace}");
    assert!(position(&out, &OFF) < position(&out, &restore), "{trace}");
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));

    // Docked again: their own Do nothing keeps it running, as it would without Wakeshot.
    let mut out = sim.send(power(true, 80));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(sleep_checks(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &LOCK), 0, "{trace}");
    assert_eq!(count(&out, &SYSTEM), 0, "still stepped back\n{trace}");

    // Unplugged for good: the countdown again once battery lasted; idle is long past 5 minutes.
    let unplugged = sim.now();
    sim.send(power(false, 80));
    sim.run(SOURCE_SETTLE_MS + SECOND);
    assert_eq!(
        sleep_check_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn ac_back_after_a_lock_step_back_keeps_the_work_end_countdown() {
    // The test's user values: Sleep on AC too. A lock step back is no floor step back, so AC does not end it.
    let mut sim = closed_unconfirmed(false);
    let input = sim.now();
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let mut out = sim.send(power(true, 80));
    out.extend(sim.run(IDLE_MS + MINUTE));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 0, "{trace}");
    assert_eq!(
        sleep_checks(&out),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
    assert_eq!(
        sleep_check_times(&sim, input),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
}

// Power safety review finding 5: the plan copy after a restore.

/// A plan read while our values are in force, as the 10 s reads show it while Wakeshot owns them.
fn ours_read() -> PlanValues {
    PlanValues {
        lid_ac: 0,
        lid_dc: 0,
        sleep_dc: 0,
        ..plan(BALANCED)
    }
}

#[test]
fn a_work_end_right_after_restore_my_power_settings_reads_the_users_own_lid_action() {
    // The test's user values. Holding, lid closed and locked, signed in on the external screen: "Restore my power
    // settings". The plan copy still shows our Do nothing until the next read; the work ends before it.
    let mut sim = holding(true);
    sim.send(read(ours_read()));
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.send(Event::Menu(MenuCommand::Restore));
    sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    sim.input();
    let at = sim.now();
    sim.send(idle(1));
    sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    // The unlock was at the same moment: idle counts from it (`p2-6-sleep-route`).
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::WorkEnded, Some(at))],
        "{trace}"
    );

    // Docked: only the keys whose entries left get their old value back; the AC lid key was their own Do nothing.
    let mut sim = holding_docked(true);
    sim.send(read(ours_read()));
    sim.send(lid(true));
    sim.send(Event::Menu(MenuCommand::Restore));
    sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    let mut out = sim.send(idle(1));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_plan_read_after_the_restore_wins_over_the_values_put_back() {
    // After the restore the user sets the battery lid action to Do nothing; the next read shows it.
    let mut sim = holding(false);
    sim.send(read(ours_read()));
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.send(Event::Menu(MenuCommand::Restore));
    sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    sim.send(read(own_battery_do_nothing()));
    let mut out = sim.send(idle(1));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_value_a_read_already_shows_stays_when_its_entry_leaves() {
    // Keep screen on, on battery. The user set battery sleep to 5 minutes: the read shows it, then the check drops
    // the entry. The idle lock follows their 5 minutes, not the 10 the entry held.
    let mut sim = screen_on();
    sim.send(read(PlanValues {
        sleep_dc: 300,
        ..ours_read()
    }));
    sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::SleepDc,
    }));
    sim.input();
    let at = sim.now();
    sim.run(5 * MINUTE + SECOND);
    assert_eq!(
        lock_times(&sim, at),
        [i128::from(5 * MINUTE)],
        "{}",
        sim.trace()
    );
}

// Power safety verify minor 3: a countdown rule before the first plan read.

#[test]
fn a_lock_step_back_before_the_first_plan_read_counts_down_once_the_plan_is_read() {
    // A crashed tray's Do nothing is in force; the lid closes before the first plan answer and the lock fails twice.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    let out = failed_twice(&mut sim);
    let restore = Command::Restore {
        plan: None,
        why: RestoreWhy::Floor,
    };
    assert_eq!(count(&out, &restore), 1, "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    let out = sim.run(MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());

    // The first read answers with the test's user values.
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
}

#[test]
fn a_lid_close_with_nothing_holding_before_the_first_plan_read_counts_down_once_the_plan_is_read() {
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    let at = sim.now();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim.send(session(SessionChange::Locked));
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{}",
        sim.trace()
    );

    // Docked: the lid closed on battery, then the dock. The read, not a guess, gives their own AC Do nothing, so the
    // countdown waits on AC and starts once battery lasted.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&DOCKED_KEYS));
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 1, "{}", sim.trace());
    sim.send(session(SessionChange::Locked));
    sim.send(power(true, 80));
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    sim.send(read(docked()));
    let out = sim.run(20 * MINUTE);
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
    let unplugged = sim.now();
    sim.send(power(false, 80));
    sim.run(SOURCE_SETTLE_MS + SECOND);
    assert_eq!(
        sleep_check_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{}",
        sim.trace()
    );

    // The lid opens before the read: the waiting want ends as a countdown would. Closed again once the restore
    // landed, nothing of ours keeps it running, and the user's own Sleep is Windows' to act on.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(false, 80));
    sim.send(lid(false));
    sim.input();
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(lid(false));
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    let out = sim.send(lid(true));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let mut out = sim.send(read(plan(BALANCED)));
    out.extend(sim.run(20 * MINUTE));
    assert!(sleep_checks(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_floor_step_back_before_the_first_plan_read_counts_down_once_the_plan_is_read() {
    // The tray starts with the lid closed, on battery under the floor, on a crashed tray's values; a hold counts.
    let mut sim = Sim::new();
    sim.send(crashed_tray(&KEYS));
    sim.send(power(false, 19));
    sim.send(lid(true));
    sim.input();
    let at = sim.now();
    let out = sim.send(working(1));
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.send(read(plan(BALANCED)));
    sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(
        sleep_check_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{trace}"
    );
    assert_eq!(
        sleep_checks(&all(&sim)),
        [&sleep_check(CountdownReason::Floor, None)],
        "{trace}"
    );
}

// Power safety verify minor 1: a hibernate that did not happen leaves the closed laptop running.

/// When each `Lock` from `from` on went out, in run time since `from`.
fn locks_after(sim: &Sim, from: Now) -> Vec<i128> {
    lock_times(sim, from)
        .into_iter()
        .filter(|&t| t >= 0)
        .collect()
}

/// Every `Lock` from now on comes with the chime in its step.
fn locks_chime(sim: &Sim, from: usize) {
    for step in &sim.steps()[from..] {
        if step.commands.contains(&LOCK) {
            assert!(
                position(&step.commands, &LOCK) < position(&step.commands, &CHIME),
                "{}",
                sim.trace()
            );
        }
    }
}

#[test]
fn a_hibernate_not_allowed_after_a_lock_step_back_locks_again_with_the_chime_every_5_minutes() {
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1, "{}", sim.trace());
    to_fallback(&mut sim);
    let from = sim.steps().len();
    let out = sim.send(hibernated(HibernateEnd::NotAllowed));
    assert_eq!(notices(&out), [&Notice::HibernateFailed], "{}", sim.trace());
    let failed = sim.now();

    // Idle is past 5 minutes: at the next input reading.
    let out = sim.run(SAFETY_TICK_MS);
    let trace = sim.trace();
    assert_eq!(count(&out, &LOCK), 1, "{trace}");
    assert_eq!(count(&out, &CHIME), 1, "{trace}");
    assert!(notices(&out).is_empty(), "{trace}");
    assert!(logs(&out) >= 1, "the lock is logged\n{trace}");

    // It failed; once more 5 minutes later, not before.
    let out = sim.send(lock_answer(Err(LOCK_FAILED)));
    assert!(
        notices(&out).is_empty(),
        "no notice per try\n{}",
        sim.trace()
    );
    let first = locks_after(&sim, failed)[0];
    sim.run(5 * MINUTE + SAFETY_TICK_MS);
    let times = locks_after(&sim, failed);
    let trace = sim.trace();
    assert_eq!(times.len(), 2, "{trace}");
    assert!(
        (i128::from(5 * MINUTE)..=i128::from(5 * MINUTE + SAFETY_TICK_MS))
            .contains(&(times[1] - first)),
        "{trace}"
    );
    locks_chime(&sim, from);

    // That one worked: no more locks while the session is locked, and the hold holds again.
    let out = sim.send(session(SessionChange::Locked));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    let out = sim.run(30 * MINUTE);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());

    // The lid opens: never again.
    sim.send(lid(false));
    sim.send(session(SessionChange::Unlocked));
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    let before = locks_after(&sim, failed).len();
    sim.send(idle(1));
    sim.run(30 * MINUTE);
    assert_eq!(locks_after(&sim, failed).len(), before, "{}", sim.trace());
}

#[test]
fn a_failed_hibernate_waits_for_5_minutes_without_input_before_it_locks() {
    // The lid-close lock worked; the user signed in on the external screen with the lid closed, then the work ended.
    let mut sim = holding(true);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.input();
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1, "{}", sim.trace());
    to_fallback(&mut sim);
    let from = sim.steps().len();
    sim.send(hibernated(HibernateEnd::Failed(SUSPEND_FAILED)));

    // Back at the external screen: no lock while typing.
    sim.input();
    let typed = sim.now();
    let out = sim.run(IDLE_MS - SECOND);
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    sim.run(SAFETY_TICK_MS + SECOND);
    let times = locks_after(&sim, typed);
    let trace = sim.trace();
    assert_eq!(times.len(), 1, "{trace}");
    assert!(
        (i128::from(IDLE_MS)..=i128::from(IDLE_MS + SAFETY_TICK_MS)).contains(&times[0]),
        "{trace}"
    );
    locks_chime(&sim, from);
}

#[test]
fn a_failed_hibernate_gives_no_lock_on_the_users_own_do_nothing_or_off_the_console() {
    // Own lid action Do nothing on AC, Sleep on battery. Work ends on battery with the lid closed, the hibernate is not
    // allowed, then the dock: their own Do nothing keeps it running, as it would without Wakeshot.
    let mut sim = holding_docked(false);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.input();
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1, "{}", sim.trace());
    to_fallback(&mut sim);
    sim.send(hibernated(HibernateEnd::NotAllowed));
    let mut out = sim.send(power(true, 80));
    out.extend(sim.run(20 * MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());

    // Unplugged for good: once battery lasted.
    let unplugged = sim.now();
    sim.send(power(false, 80));
    sim.run(SOURCE_SETTLE_MS + SAFETY_TICK_MS);
    let times = locks_after(&sim, unplugged);
    let trace = sim.trace();
    assert_eq!(times.len(), 1, "{trace}");
    assert!(
        (i128::from(SOURCE_SETTLE_MS)..=i128::from(SOURCE_SETTLE_MS + SAFETY_TICK_MS))
            .contains(&times[0]),
        "{trace}"
    );

    // Another session has the PC's screen: this one's lock would not guard it. Back: at the next reading.
    let mut out = sim.send(session(SessionChange::ConsoleLeft));
    out.extend(sim.run(20 * MINUTE));
    assert_eq!(count(&out, &LOCK), 0, "{}", sim.trace());
    let back = sim.now();
    sim.send(session(SessionChange::ConsoleBack));
    sim.run(SAFETY_TICK_MS);
    assert_eq!(locks_after(&sim, back).len(), 1, "{}", sim.trace());
}

// Power safety verify minor 2: notices of a closed laptop nobody saw wait for the user's return.

#[test]
fn a_lock_step_back_keeps_its_notices_for_the_users_return() {
    // The lid opens with the session unlocked: the user is back.
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    to_fallback(&mut sim);
    sim.send(hibernated(HibernateEnd::NotAllowed));
    sim.run(MINUTE);
    let out = sim.send(lid(false));
    let trace = sim.trace();
    let shown = notices(&out);
    assert_eq!(shown.len(), 3, "{trace}");
    assert_eq!(
        shown[..2],
        [&Notice::LockFailed, &Notice::HibernateFailed],
        "{trace}"
    );
    assert!(
        matches!(shown[2], Notice::ClosedLidReport { .. }),
        "{trace}"
    );
    let mut out = sim.send(session(SessionChange::Locked));
    out.extend(sim.send(session(SessionChange::Unlocked)));
    assert!(notices(&out).is_empty(), "once\n{}", sim.trace());

    // Windows locked it after all; the user opens the lid and signs in.
    let mut sim = closed_unconfirmed(false);
    failed_twice(&mut sim);
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.send(session(SessionChange::Locked));
    let out = sim.send(lid(false));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.send(session(SessionChange::Unlocked));
    let trace = sim.trace();
    let shown = notices(&out);
    assert_eq!(shown.len(), 2, "{trace}");
    assert_eq!(shown[0], &Notice::LockFailed, "{trace}");
    assert!(
        matches!(shown[1], Notice::ClosedLidReport { .. }),
        "{trace}"
    );

    // A notice raised while the session is locked waits for the unlock anyway: once.
    let mut sim = work_ended_closed(true);
    sim.send(session(SessionChange::Locked));
    sim.run(IDLE_MS + MINUTE);
    to_fallback(&mut sim);
    let out = sim.send(hibernated(HibernateEnd::NotAllowed));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    sim.send(lid(false));
    let out = sim.send(session(SessionChange::Unlocked));
    let trace = sim.trace();
    let shown = notices(&out);
    assert_eq!(
        shown
            .iter()
            .filter(|n| ***n == Notice::HibernateFailed)
            .count(),
        1,
        "{trace}"
    );
}

// Item 7: the closed-lid report.

#[test]
fn closed_lid_report_once_with_the_right_time_across_a_suspend() {
    let mut sim = holding(false);
    let closed = sim.now();
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.run(HOUR);
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.run(SECOND);
    assert_eq!(sleep_checks(&all(&sim)).len(), 1);

    // Hibernated overnight after the fallback: run time stops, the wall clock does not.
    to_fallback(&mut sim);
    sim.suspend(8 * HOUR);
    sim.send(hibernated(HibernateEnd::Resumed));
    sim.run(2 * SECOND);
    let opened = sim.now();
    sim.send(lid(false));
    sim.run(10 * SECOND);
    assert!(
        reports(&sim).is_empty(),
        "not before the sign-in\n{}",
        sim.trace()
    );

    sim.send(session(SessionChange::Unlocked));
    let trace = sim.trace();
    assert_eq!(
        reports(&sim),
        [(
            opened.mono_ms - closed.mono_ms,
            vec!["Claude Code".to_owned()]
        )],
        "{trace}"
    );
    assert!(opened.mono_ms - closed.mono_ms < 2 * HOUR, "{trace}");

    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    assert_eq!(reports(&sim).len(), 1, "once\n{}", sim.trace());
}

#[test]
fn no_closed_lid_report_while_the_lid_stays_closed_then_one_when_it_opens() {
    let mut sim = holding(true);
    let closed = sim.now();
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    sim.run(HOUR);
    assert!(
        reports(&sim).is_empty(),
        "signed in on the external screen\n{}",
        sim.trace()
    );

    // Signed in already when the lid opens: the report then.
    let opened = sim.now();
    sim.send(lid(false));
    assert_eq!(
        reports(&sim),
        [(
            opened.mono_ms - closed.mono_ms,
            vec!["Claude Code".to_owned()]
        )],
        "{}",
        sim.trace()
    );
}

#[test]
fn no_closed_lid_report_for_the_users_own_do_nothing() {
    let mut sim = holding_docked(true);
    sim.send(lid(true));
    sim.run(HOUR);
    sim.send(lid(false));
    sim.send(session(SessionChange::Locked));
    sim.send(session(SessionChange::Unlocked));
    assert!(reports(&sim).is_empty(), "{}", sim.trace());
}

#[test]
fn the_closed_lid_report_names_everything_that_held() {
    let mut sim = holding(true);
    sim.send(lid(true));
    sim.send(session(SessionChange::Locked));
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.send(idle(1));
    sim.run(MINUTE);
    sim.send(lid(false));
    sim.send(session(SessionChange::Unlocked));
    let held: Vec<Vec<String>> = reports(&sim).into_iter().map(|(_, h)| h).collect();
    assert_eq!(held.len(), 1, "{}", sim.trace());
    let mut names = held[0].clone();
    names.sort();
    assert_eq!(names, ["Claude Code", "Keep awake"], "{}", sim.trace());
}

// Item 8a: notices wait while the user is away; sounds never wait.

#[test]
fn notices_raised_while_locked_go_out_at_unlock_in_order() {
    let mut sim = holding(true);
    sim.send(session(SessionChange::Locked));
    let mut out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    let lost = || reset_done(&[SettingKey::SleepDc], Ok(()));
    out.extend(sim.send(Event::JournalCorrupt));
    out.extend(sim.send(lost()));
    // A second corrupt file in the same absence: the same notice waits once.
    out.extend(sim.send(Event::JournalCorrupt));
    out.extend(sim.send(lost()));
    assert!(notices(&out).is_empty(), "{}", sim.trace());

    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(
        notices(&out),
        [
            &Notice::UserChanged(SettingKey::LidDc),
            &Notice::JournalLost {
                reset: vec![SettingKey::SleepDc],
                failed: false
            }
        ],
        "{}",
        sim.trace()
    );
}

#[test]
fn notices_raised_off_the_console_go_out_when_it_comes_back() {
    let mut sim = holding(true);
    sim.send(session(SessionChange::ConsoleLeft));
    let out = sim.send(restored(
        None,
        RestoreWhy::ConsoleLeft,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    assert!(notices(&out).is_empty(), "{}", sim.trace());

    // Locked when it comes back: still waiting.
    sim.send(session(SessionChange::Locked));
    let out = sim.send(session(SessionChange::ConsoleBack));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(notices(&out), [&Notice::RestoreFailed], "{}", sim.trace());
}

#[test]
fn a_tray_started_while_locked_holds_the_updated_notice_until_unlock() {
    let mut sim = Sim::new();
    sim.start();
    sim.send(session(SessionChange::Locked));
    let mut out = sim.send(Event::Updated {
        version: "1.1.0".to_owned(),
    });
    out.extend(sim.run(MINUTE));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(
        notices(&out),
        [&Notice::Updated {
            version: "1.1.0".to_owned()
        }],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_start_time_restore_failure_waits_for_the_unlock() {
    let journal = Journal {
        owner: JournalOwner {
            pid: 4100,
            proc_start: FileTime(7),
            session: 1,
        },
        entries: entries(BALANCED, &KEYS),
    };
    let mut sim = Sim::new();
    sim.send(
        Start {
            journal: Some(journal),
            ..Start::default()
        }
        .event(),
    );
    sim.send(session(SessionChange::Locked));
    let out = sim.run(SECOND);
    let retry = Command::Restore {
        plan: None,
        why: RestoreWhy::Leftover,
    };
    assert_eq!(count(&out, &retry), 1, "{}", sim.trace());
    let out = sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(notices(&out), [&Notice::RestoreFailed], "{}", sim.trace());
}

#[test]
fn sounds_never_wait() {
    let mut sim = holding(true);
    sim.send(session(SessionChange::Locked));
    sim.send(lid(true));
    sim.send(power(false, 19));
    let out = sim.run(SOURCE_SETTLE_MS);
    let trace = sim.trace();
    assert_eq!(count(&out, &FLOOR_CHIME), 1, "{trace}");
    assert!(notices(&out).is_empty(), "{trace}");
    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(
        notices(&out),
        [&Notice::StepBack { floor_pct: 20 }],
        "{}",
        sim.trace()
    );
}

// Item 9a: updates.

#[test]
fn update_ready_applies_when_nothing_holds_and_ok_quits() {
    let mut sim = started_with(plan(BALANCED), true, 80);
    let out = sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
    let out = sim.run(MINUTE);
    assert_eq!(
        count(&out, &Command::ApplyUpdate),
        0,
        "one at a time\n{}",
        sim.trace()
    );

    let out = sim.send(Event::Done(Outcome::Update(Ok(()))));
    assert_eq!(out.last(), Some(&Command::Quit), "{}", sim.trace());
    assert!(sim.send(Event::Tick).is_empty(), "{}", sim.trace());
}

#[test]
fn update_ready_waits_for_the_hold_to_end_and_the_settings_to_come_back() {
    let mut sim = holding(true);
    let mut out = sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    out.extend(sim.run(MINUTE));
    out.extend(sim.send(idle(1)));
    assert_eq!(
        count(&out, &Command::ApplyUpdate),
        0,
        "a hold or an owned key\n{}",
        sim.trace()
    );
    let out = sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
}

#[test]
fn update_waits_for_a_live_hold_file_even_with_automatic_off() {
    for alive in [Alive::Yes, Alive::Unknown] {
        let mut sim = started_with(plan(BALANCED), true, 80);
        sim.send(Event::Menu(MenuCommand::Automatic(false)));
        sim.put_hold(hold("backup.json", alive));
        let mut out = sim.send(Event::UpdateReady {
            version: "1.1.0".to_owned(),
        });
        out.extend(sim.run(MINUTE));
        assert_eq!(
            count(&out, &Command::ApplyUpdate),
            0,
            "{alive:?}\n{}",
            sim.trace()
        );
        let out = sim.remove_hold("backup.json");
        assert_eq!(
            count(&out, &Command::ApplyUpdate),
            1,
            "{alive:?}\n{}",
            sim.trace()
        );
    }
}

#[test]
fn update_waits_while_a_key_is_owned() {
    let journal = Journal {
        owner: JournalOwner {
            pid: 4100,
            proc_start: FileTime(7),
            session: 1,
        },
        entries: entries(BALANCED, &KEYS),
    };
    let mut sim = Sim::new();
    sim.send(
        Start {
            journal: Some(journal),
            ..Start::default()
        }
        .event(),
    );
    let mut out = sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    out.extend(sim.run(SECOND));
    assert_eq!(count(&out, &Command::ApplyUpdate), 0, "{}", sim.trace());
    let out = sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
}

#[test]
fn update_waits_for_the_countdown() {
    let mut sim = work_ended_closed(true);
    sim.input();
    let mut out = sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    out.extend(sim.run(MINUTE));
    assert_eq!(count(&out, &Command::ApplyUpdate), 0, "{}", sim.trace());
    let out = sim.send(lid(false));
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
}

#[test]
fn an_expired_keep_awake_file_does_not_hold_the_update_back() {
    let mut sim = started_with(plan(BALANCED), true, 80);
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))));
    sim.watch();
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim.run(31 * MINUTE);
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    let out = sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
}

#[test]
fn a_failed_update_waits_for_the_next_update_ready() {
    let mut sim = started_with(plan(BALANCED), true, 80);
    sim.send(Event::UpdateReady {
        version: "1.1.0".to_owned(),
    });
    let mut out = sim.send(Event::Done(Outcome::Update(Err(UPDATE_FAILED))));
    assert!(logs(&out) >= 1, "{}", sim.trace());
    out.extend(sim.run(10 * MINUTE));
    assert_eq!(count(&out, &Command::ApplyUpdate), 0, "{}", sim.trace());
    assert!(!out.contains(&Command::Quit), "{}", sim.trace());
    let out = sim.send(Event::UpdateReady {
        version: "1.1.1".to_owned(),
    });
    assert_eq!(count(&out, &Command::ApplyUpdate), 1, "{}", sim.trace());
}

#[test]
fn updated_gives_notice() {
    let mut sim = started_with(plan(BALANCED), true, 80);
    sim.send(working(1));
    let out = sim.send(Event::Updated {
        version: "1.1.0".to_owned(),
    });
    let trace = sim.trace();
    assert_eq!(
        out,
        [Command::Notify(Notice::Updated {
            version: "1.1.0".to_owned()
        })],
        "{trace}"
    );
    let out = sim.send(Event::Updated {
        version: "1.1.0".to_owned(),
    });
    assert!(out.is_empty(), "once\n{}", sim.trace());
}

#[test]
fn updated_before_any_sources_gives_the_notice() {
    let mut sim = Sim::new();
    sim.start();
    let out = sim.send(Event::Updated {
        version: "1.1.0".to_owned(),
    });
    assert_eq!(
        notices(&out),
        [&Notice::Updated {
            version: "1.1.0".to_owned()
        }],
        "{}",
        sim.trace()
    );
}

// Any order.

#[derive(Clone, Debug)]
enum Op {
    Send(Event),
    Run(u64),
    Suspend(u64),
    Input,
}

fn values() -> impl Strategy<Value = PlanValues> {
    prop_oneof![
        Just(plan(BALANCED)),
        Just(docked()),
        Just(PlanValues {
            sleep_ac: 0,
            ..plan(BALANCED)
        }),
    ]
}

fn hibernate_end() -> impl Strategy<Value = HibernateEnd> {
    prop_oneof![
        Just(HibernateEnd::Resumed),
        Just(HibernateEnd::NotAllowed),
        Just(HibernateEnd::Failed(SUSPEND_FAILED)),
        Just(HibernateEnd::Skipped(HibernateSkip::LidOpen)),
        Just(HibernateEnd::Skipped(HibernateSkip::LidUnknown)),
        Just(HibernateEnd::Skipped(HibernateSkip::InputSeen)),
        Just(HibernateEnd::Skipped(HibernateSkip::OnAc)),
        Just(HibernateEnd::Skipped(HibernateSkip::AboveFloor)),
    ]
}

fn session_change() -> impl Strategy<Value = SessionChange> {
    prop_oneof![
        Just(SessionChange::Locked),
        Just(SessionChange::Unlocked),
        Just(SessionChange::ConsoleLeft),
        Just(SessionChange::ConsoleBack),
    ]
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        3 => any::<bool>().prop_map(lid),
        3 => (any::<bool>(), prop_oneof![Just(10u8), Just(19), Just(20), Just(80)])
            .prop_map(|(ac, pct)| power(ac, pct)),
        3 => prop_oneof![Just(working(1)), Just(idle(1))],
        2 => values().prop_map(read),
        2 => prop_oneof![
            Just(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS))),
            Just(applied(BALANCED, Ok(()), entries(BALANCED, &DOCKED_KEYS))),
        ],
        2 => prop_oneof![
            Just(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![])),
            Just(restored(None, RestoreWhy::Floor, Err(RESTORE_FAILED), entries(BALANCED, &KEYS))),
        ],
        2 => session_change().prop_map(Event::Session),
        1 => any::<bool>().prop_map(|on| Event::Menu(MenuCommand::KeepScreenOn(on))),
        1 => prop_oneof![Just(KeepAwake::UntilOff), Just(KeepAwake::Off)]
            .prop_map(|c| Event::Menu(MenuCommand::KeepAwake(c))),
        1 => prop_oneof![Just(Ok(())), Just(Err(LOCK_FAILED))]
            .prop_map(|r| Event::Done(Outcome::Locked(r))),
        2 => hibernate_end().prop_map(hibernated),
        1 => Just(Event::UpdateReady { version: "1.1.0".to_owned() }),
        1 => prop_oneof![Just(Ok(())), Just(Err(UPDATE_FAILED))]
            .prop_map(|r| Event::Done(Outcome::Update(r))),
        1 => Just(Event::Updated { version: "1.1.0".to_owned() }),
        2 => Just(Event::Tick),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => event().prop_map(Op::Send),
        3 => (0..12 * MINUTE).prop_map(Op::Run),
        1 => (0..3 * HOUR).prop_map(Op::Suspend),
        2 => Just(Op::Input),
    ]
}

proptest! {
    // A regression file would be a file this card does not own; a shrunk case becomes a named test instead.
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    /// `handle` never panics on any order of these events and `Sim` finds no deadline behind a `Tick`; every `Lock`
    /// comes with a sound or a notice in its step, but for the one retry of a lid lock, which follows a failure or
    /// [`LOCK_CHECK`] with no `Session(Locked)`; no `Hibernate` within 5 minutes of input or a resume; the core never
    /// plays the going to sleep sound; nothing follows `Quit`.
    #[test]
    fn lid_events_in_any_order(ops in vec(op(), 0..40)) {
        let mut sim = Sim::new();
        sim.start();
        sim.input();
        // The latest input or resume, in run time.
        let mut quiet_since = sim.now().mono_ms;
        for op in ops {
            let before = sim.steps().len();
            match op {
                Op::Send(event) => {
                    let resumed = event == hibernated(HibernateEnd::Resumed);
                    sim.send(event);
                    if resumed {
                        quiet_since = sim.now().mono_ms;
                    }
                }
                Op::Run(ms) => {
                    sim.run(ms);
                }
                Op::Suspend(ms) => {
                    sim.suspend(ms);
                    quiet_since = sim.now().mono_ms;
                }
                Op::Input => {
                    sim.input();
                    quiet_since = sim.now().mono_ms;
                }
            }
            for step in &sim.steps()[before..] {
                if step.commands.iter().any(|c| matches!(c, Command::Hibernate { .. })) {
                    prop_assert!(
                        step.at.mono_ms.saturating_sub(quiet_since) >= IDLE_MS,
                        "{}", sim.trace()
                    );
                }
            }
        }
        let mut quit = false;
        // The last lid lock not retried yet, its time, and whether a failure answered it since.
        let mut retry_for: Option<(u64, bool)> = None;
        for step in sim.steps() {
            prop_assert!(!quit || step.commands.is_empty(), "{}", sim.trace());
            // A source lock that goes with a step back's first sound has no chime of its own (`p2-6-floor-grace`).
            let chime = step.commands.contains(&CHIME) || step.commands.contains(&FLOOR_CHIME);
            let explained = chime || step.commands.contains(&Command::Notify(Notice::IdleLocked));
            if step.commands.contains(&LOCK) && !explained {
                let due = retry_for.is_some_and(|(at, failed)| {
                    failed || step.at.mono_ms >= at.saturating_add(LOCK_CHECK)
                });
                prop_assert!(due && step.event == Event::Tick, "{}", sim.trace());
                retry_for = None;
            }
            if matches!(step.event, Event::Done(Outcome::Locked(Err(_))))
                && let Some((_, failed)) = retry_for.as_mut()
            {
                *failed = true;
            }
            if chime && step.commands.contains(&LOCK) {
                retry_for = Some((step.at.mono_ms, false));
            }
            prop_assert!(!step.commands.contains(&Command::Play(Cue::GoingToSleep)), "{}", sim.trace());
            quit |= step.commands.contains(&Command::Quit);
        }
    }
}
