//! Lid, countdown and notice scenarios over `Core`: the lid-close lock and chime, the lock when a closed laptop
//! changes power source, the battery floor, the countdown to hibernate, the Keep screen on idle lock, the closed-lid
//! report, notices that wait while the user is away, and updates.
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
    Command, CountdownReason, Failure, HibernateEnd, HibernateSkip, Outcome, RestoreWhy,
};
use crate::event::{Event, SessionChange};
use crate::policy::ending::IDLE_MS;
use crate::policy::lid::SOURCE_SETTLE_MS;
use crate::process::{Alive, FileTime};
use crate::settings::{Journal, JournalOwner, OwnedKey, PlanValues, SettingKey};
use crate::sim::{
    self, BALANCED, KEYS, SAFETY_TICK_MS, Sim, Start, applied, claude, entries, hold, plan,
    restored,
};
use crate::sources::Activity;
use crate::time::Now;
use crate::view::{Cue, KeepAwake, MenuCommand, Notice};

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

/// Not a multiple of the 30 s safety tick, so a time checked to the millisecond is the core's own deadline at work.
const OFF_THE_TICK: u64 = 7 * SECOND;

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

/// Docked use: the user's own lid action on AC is Do nothing, on battery Sleep (Haik's other values).
fn docked() -> PlanValues {
    PlanValues {
        lid_ac: 0,
        ..plan(BALANCED)
    }
}

/// What an apply leaves on a docked plan: the AC lid key is the user's own Do nothing, so the IO side skips it.
const DOCKED_KEYS: [SettingKey; 2] = [SettingKey::LidDc, SettingKey::SleepDc];

fn hibernate(reason: CountdownReason, last_resume: Option<Now>) -> Command {
    Command::Hibernate {
        reason,
        floor_pct: 20,
        last_resume_mono_ms: last_resume.map(|t| t.mono_ms),
    }
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

fn hibernates(commands: &[Command]) -> Vec<&Command> {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Hibernate { .. }))
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

/// When each `Hibernate` went out, in run time since `from`.
fn hibernate_times(sim: &Sim, from: Now) -> Vec<i128> {
    sim.commands()
        .filter(|(_, c)| matches!(c, Command::Hibernate { .. }))
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

/// A Claude Code session holds with Haik's own values, and the apply worked. The plan is not read again, so the copy
/// still shows Sleep on both lid keys.
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

    // Closed it stays: no second lock.
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
    assert_eq!(hibernate_times(&sim, at), [i128::from(IDLE_MS)], "{trace}");
    assert_eq!(
        hibernates(&all(&sim)),
        [&hibernate(CountdownReason::WorkEnded, None)],
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
    sim.run(IDLE_MS + MINUTE);
    let trace = sim.trace();
    assert_eq!(hibernate_times(&sim, at), [i128::from(IDLE_MS)], "{trace}");
    assert_eq!(
        hibernates(&all(&sim)),
        [&hibernate(CountdownReason::Floor, None)],
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

// Item 2: the power source changes with the lid closed.

#[test]
fn closed_then_unplugged_after_signing_in_on_an_external_screen_locks_once_battery_lasted() {
    // Power verify minor 11: docked with Haik's own values, the lid-close lock, he signs in on the external screen.
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
    assert!(hibernates(&out).is_empty(), "{trace}");
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
        hibernates(&out),
        [&hibernate(CountdownReason::Floor, None)],
        "{trace}"
    );
    assert_eq!(hibernate_times(&sim, at), [i128::from(IDLE_MS)], "{trace}");
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
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
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
    assert!(hibernates(&out).is_empty(), "{trace}");
    assert!(logs(&out) >= 2, "hold end and countdown start\n{trace}");
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));

    let out = sim.run(IDLE_MS - SECOND);
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(
        hibernates(&out),
        [&hibernate(CountdownReason::WorkEnded, None)],
        "{trace}"
    );
    assert_eq!(hibernate_times(&sim, at), [i128::from(IDLE_MS)], "{trace}");
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
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());

    sim.input();
    let stopped = sim.now();
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        hibernate_times(&sim, stopped),
        [i128::from(IDLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn work_ended_after_a_long_idle_hibernates_at_once() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.run(20 * MINUTE);
    let ended = sim.now();
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.run(SECOND);
    assert_eq!(hibernate_times(&sim, ended), [0], "{}", sim.trace());
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
        hibernates(&out).is_empty(),
        "a resume counts as input\n{}",
        sim.trace()
    );

    let out = sim.run(2 * SECOND);
    let trace = sim.trace();
    assert_eq!(
        hibernates(&out),
        [&hibernate(CountdownReason::WorkEnded, Some(resumed))],
        "{trace}"
    );
    assert_eq!(
        hibernate_times(&sim, resumed),
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
    assert!(hibernates(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &OFF), 1, "only the release\n{trace}");
}

#[test]
fn a_new_hold_cancels_the_countdown() {
    let mut sim = work_ended_closed(true);
    sim.run(MINUTE);
    let out = sim.send(working(1));
    assert!(logs(&out) >= 1, "the cancel is logged\n{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn opening_the_lid_cancels_the_countdown() {
    let mut sim = work_ended_closed(false);
    sim.run(MINUTE);
    let out = sim.send(lid(false));
    assert!(logs(&out) >= 1, "the cancel is logged\n{}", sim.trace());
    let out = sim.run(20 * MINUTE);
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn another_session_on_the_console_holds_the_countdown_back() {
    let mut sim = work_ended_closed(true);
    sim.run(MINUTE);
    let mut out = sim.send(session(SessionChange::ConsoleLeft));
    out.extend(sim.run(20 * MINUTE));
    assert!(
        hibernates(&out).is_empty(),
        "never hibernate the other user's PC\n{}",
        sim.trace()
    );
    let back = sim.now();
    sim.send(session(SessionChange::ConsoleBack));
    sim.run(SECOND);
    assert_eq!(hibernate_times(&sim, back), [0], "{}", sim.trace());
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
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());

    // A short AC drop does not start it again.
    let mut out = sim.send(power(false, 80));
    out.extend(sim.run(3 * SECOND));
    out.extend(sim.send(power(true, 80)));
    out.extend(sim.run(5 * MINUTE));
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());

    // Unplugged for good: the countdown again once the source lasted; idle is long past 5 minutes.
    let unplugged = sim.now();
    let out = sim.send(power(false, 80));
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
    sim.run(SOURCE_SETTLE_MS + SECOND);
    assert_eq!(
        hibernate_times(&sim, unplugged),
        [i128::from(SOURCE_SETTLE_MS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_power_source_change_onto_the_users_own_sleep_keeps_the_countdown() {
    // Haik's own values: Sleep on AC too, so docking changes nothing.
    let mut sim = work_ended_closed(false);
    sim.input();
    let at = sim.now();
    sim.send(power(true, 80));
    sim.run(IDLE_MS + MINUTE);
    assert_eq!(
        hibernate_times(&sim, at),
        [i128::from(IDLE_MS)],
        "{}",
        sim.trace()
    );
}

// Item 5: how the hibernate ended.

#[test]
fn hibernate_not_allowed_shows_a_notice_and_ends_the_countdown() {
    for end in [
        HibernateEnd::NotAllowed,
        HibernateEnd::Failed(SUSPEND_FAILED),
        HibernateEnd::Skipped(HibernateSkip::LidUnknown),
    ] {
        let mut sim = work_ended_closed(true);
        sim.run(IDLE_MS + MINUTE);
        assert_eq!(hibernates(&all(&sim)).len(), 1);
        let out = sim.send(hibernated(end));
        let trace = sim.trace();
        assert_eq!(
            notices(&out),
            [&Notice::HibernateFailed],
            "{end:?}\n{trace}"
        );
        assert!(logs(&out) >= 1, "{end:?}\n{trace}");
        let out = sim.run(20 * MINUTE);
        assert!(hibernates(&out).is_empty(), "{end:?}\n{}", sim.trace());
    }
}

#[test]
fn hibernate_skipped_for_input_goes_on() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::InputSeen)));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    sim.input();
    let typed = sim.now();
    let out = sim.run(IDLE_MS + MINUTE);
    assert_eq!(hibernates(&out).len(), 1, "{}", sim.trace());
    assert_eq!(
        hibernate_times(&sim, typed).last(),
        Some(&i128::from(IDLE_MS)),
        "{}",
        sim.trace()
    );
}

#[test]
fn hibernate_skipped_with_the_lid_open_ends_the_countdown() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let mut out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::LidOpen)));
    out.extend(sim.run(20 * MINUTE));
    let trace = sim.trace();
    assert!(notices(&out).is_empty(), "{trace}");
    assert!(hibernates(&out).is_empty(), "{trace}");
}

#[test]
fn hibernate_skipped_on_ac_ends_only_the_floor_countdown() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    let mut out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::OnAc)));
    out.extend(sim.run(20 * MINUTE));
    assert!(hibernates(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn hibernate_skipped_above_the_floor_waits_for_a_new_battery_reading() {
    let mut sim = holding(false);
    sim.send(lid(true));
    sim.send(power(false, 19));
    sim.send(restored(None, RestoreWhy::Floor, Ok(()), vec![]));
    sim.run(IDLE_MS + MINUTE);
    let out = sim.send(hibernated(HibernateEnd::Skipped(HibernateSkip::AboveFloor)));
    assert!(notices(&out).is_empty(), "{}", sim.trace());
    let out = sim.run(10 * MINUTE);
    assert!(
        hibernates(&out).is_empty(),
        "the two readings disagree
{}",
        sim.trace()
    );

    // A new reading under the floor: the countdown is still due.
    sim.send(power(false, 18));
    let out = sim.run(SAFETY_TICK_MS);
    assert_eq!(
        hibernates(&out),
        [&hibernate(CountdownReason::Floor, None)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_hibernate_that_resumed_counts_as_a_resume() {
    let mut sim = work_ended_closed(true);
    sim.run(IDLE_MS + MINUTE);
    let back = sim.now();
    sim.send(hibernated(HibernateEnd::Resumed));
    let out = sim.run(20 * MINUTE);
    assert!(hibernates(&out).is_empty(), "it ended\n{}", sim.trace());

    // The lid stayed closed; new work ends again: idle counts from that resume.
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim.send(idle(1));
    let out = sim.run(SECOND);
    assert_eq!(
        hibernates(&out),
        [&hibernate(CountdownReason::WorkEnded, Some(back))],
        "{}",
        sim.trace()
    );
}

// Item 6: Keep screen on.

/// Keep awake with Keep screen on, applied, on battery: Haik's own battery sleep is 10 minutes.
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
    assert_eq!(hibernates(&all(&sim)).len(), 1);

    // Hibernated overnight: run time stops, the wall clock does not.
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
    out.extend(sim.send(Event::JournalCorrupt));
    // A second corrupt file in the same absence: the same notice waits once.
    out.extend(sim.send(Event::JournalCorrupt));
    assert!(notices(&out).is_empty(), "{}", sim.trace());

    let out = sim.send(session(SessionChange::Unlocked));
    assert_eq!(
        notices(&out),
        [
            &Notice::UserChanged(SettingKey::LidDc),
            &Notice::JournalCorrupt
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
    let out = sim.send(power(false, 19));
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
    /// comes with a sound or a notice in its step; no `Hibernate` within 5 minutes of input or a resume; the core
    /// never plays the going to sleep sound; nothing follows `Quit`.
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
        for step in sim.steps() {
            prop_assert!(!quit || step.commands.is_empty(), "{}", sim.trace());
            let explained = step.commands.iter().any(|c| {
                matches!(c, Command::Play(Cue::LidClosedHolding) | Command::Notify(Notice::IdleLocked))
            });
            prop_assert!(!step.commands.contains(&LOCK) || explained, "{}", sim.trace());
            prop_assert!(!step.commands.contains(&Command::Play(Cue::GoingToSleep)), "{}", sim.trace());
            quit |= step.commands.contains(&Command::Quit);
        }
    }
}
