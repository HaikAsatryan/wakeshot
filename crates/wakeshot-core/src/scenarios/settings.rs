//! Settings scenarios over `Core`: when the core applies and restores, the journal mirror, released keys, plan
//! switches, retries, the user's own actions, session end and fast user switching.
//!
//! Owner: `p2-core-settings`.
//!
//! The IO side is not here: each test feeds the `Done` outcomes and plan reads by hand, as the IO thread would post
//! them, and checks the commands the core gives back.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

use crate::command::{ApplyError, Command, Failure, RestoreWhy};
use crate::event::{Event, SessionChange};
use crate::holds::HoldId;
use crate::ownership::apply::JOURNAL_CORRUPT;
use crate::process::{Alive, FileTime};
use crate::settings::{Guid, Journal, JournalEntry, JournalOwner, OwnedKey, SettingKey};
use crate::sim::{
    self, BALANCED, KEEP_AWAKE, KEYS, OTHER_PLAN, SAFETY_TICK_MS, START, Sim, Start, applied,
    claude, entries, plan, restored,
};
use crate::sources::Activity;
use crate::view::{KeepAwake, MenuCommand, Notice};

const SECOND: u64 = 1_000;

/// Not a multiple of the 30 s safety tick, so a read checked to the millisecond is the core's own deadline at work.
const OFF_THE_TICK: u64 = 7 * SECOND;

const OFF: Command = Command::Requests {
    system: false,
    display: false,
};
const SYSTEM: Command = Command::Requests {
    system: true,
    display: false,
};
const SCREEN: Command = Command::Requests {
    system: true,
    display: true,
};

/// A guard start that failed: a safety net step, nothing written.
const GUARD_FAILED: Failure = Failure {
    call: "CreateProcessW (guard)",
    code: 2,
};
const WRITE_FAILED: Failure = Failure {
    call: "PowerWriteDCValueIndex",
    code: 31,
};
const ACCESS_DENIED: Failure = Failure {
    call: "PowerSettingAccessCheckEx",
    code: 5,
};
const POLICY_DENIED: Failure = Failure {
    call: "PowerWriteACValueIndex",
    code: 1260,
};
const RESTORE_FAILED: Failure = Failure {
    call: "PowerReadDCValueIndex",
    code: 2,
};
const LOAD_FAILED: Failure = Failure {
    call: "ReadFile (journal.json)",
    code: 33,
};

fn working(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Working), Alive::Yes)])
}

fn idle(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Idle), Alive::Yes)])
}

fn apply(plan: Guid, keys: &[SettingKey]) -> Command {
    Command::Apply {
        plan,
        writes: keys.iter().map(|&key| (key, 0)).collect(),
    }
}

fn restore(plan: Option<Guid>, why: RestoreWhy) -> Command {
    Command::Restore { plan, why }
}

fn read(id: Guid) -> Event {
    Event::Plan(Ok(plan(id)))
}

fn count(commands: &[Command], wanted: &Command) -> usize {
    commands.iter().filter(|c| *c == wanted).count()
}

/// The `Apply` and `Restore` commands, in order.
fn settings(commands: &[Command]) -> Vec<&Command> {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Apply { .. } | Command::Restore { .. }))
        .collect()
}

fn logs(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Log(_)))
        .count()
}

fn notified(commands: &[Command], notice: &Notice) -> usize {
    count(commands, &Command::Notify(notice.clone()))
}

fn position(commands: &[Command], wanted: &Command) -> usize {
    commands
        .iter()
        .position(|c| c == wanted)
        .unwrap_or_else(|| panic!("no {wanted:?} in {commands:?}"))
}

/// Started, with Balanced read, 7 s after the start: off the safety tick grid.
fn known_plan() -> Sim {
    let mut sim = Sim::new();
    sim.start();
    sim.run(OFF_THE_TICK);
    sim.send(read(BALANCED));
    sim
}

/// A Claude Code session holds, and the apply to Balanced worked.
fn holding() -> Sim {
    let mut sim = known_plan();
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim
}

/// The keys `state.json` lists as overridden, once its 1 s throttle has passed.
fn overridden(sim: &mut Sim) -> Vec<OwnedKey> {
    sim.run(SECOND);
    sim.state_file()
        .unwrap_or_else(|| panic!("no state.json yet\n{}", sim.trace()))
        .overridden
        .clone()
}

fn owned(plan: Guid, keys: &[SettingKey]) -> Vec<OwnedKey> {
    keys.iter().map(|&key| OwnedKey { plan, key }).collect()
}

/// When each `ReadPlan` went out, in run time since `START`.
fn reads_at(sim: &Sim) -> Vec<u64> {
    sim.sent_at(&Command::ReadPlan)
        .iter()
        .map(|t| t.mono_ms - START.mono_ms)
        .collect()
}

// Item 1: when the core applies, and what.

#[test]
fn hold_start_applies_the_three_keys_after_the_requests() {
    let mut sim = known_plan();
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{trace}");
    assert!(
        position(&out, &SYSTEM) < position(&out, &apply(BALANCED, &KEYS)),
        "requests first\n{trace}"
    );
    assert_eq!(count(&out, &Command::ReadPlan), 0, "{trace}");
}

#[test]
fn hold_start_before_the_first_plan_read_reads_first() {
    let mut sim = Sim::new();
    sim.start();
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(count(&out, &Command::ReadPlan), 1, "{trace}");
    assert!(settings(&out).is_empty(), "no plan values yet\n{trace}");

    let out = sim.send(read(BALANCED));
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());

    // Later reads of the same plan apply nothing more.
    let out = sim.send(read(BALANCED));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_failed_first_plan_read_waits_for_the_next_one() {
    let mut sim = Sim::new();
    sim.start();
    sim.run(OFF_THE_TICK);
    sim.send(working(1));
    let out = sim.send(Event::Plan(Err(RESTORE_FAILED)));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
    // Read again 10 s after the first ask, off the safety tick.
    sim.run(10 * SECOND);
    assert_eq!(
        reads_at(&sim),
        [OFF_THE_TICK, OFF_THE_TICK + 10 * SECOND],
        "{}",
        sim.trace()
    );
    let out = sim.send(read(BALANCED));
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());
}

// Item 14.

#[test]
fn without_the_settings_owner_requests_only_until_it_is_taken() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            owner: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(read(BALANCED));
    let out = sim.send(working(1));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert!(settings(&out).is_empty(), "{trace}");

    let out = sim.send(Event::SettingsOwner);
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());
}

#[test]
fn settings_owner_with_nothing_holding_applies_nothing() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            owner: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(read(BALANCED));
    let out = sim.send(Event::SettingsOwner);
    assert!(settings(&out).is_empty(), "{}", sim.trace());
    // The next hold applies.
    let out = sim.send(working(1));
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());
}

// Item 2.

#[test]
fn read_plan_on_every_tick_and_every_10_s_while_owned() {
    let mut sim = holding();
    sim.run(45 * SECOND);
    // Owned from 7 s: at once, then every 10 s; the 30 s safety tick comes 3 s after a read and adds none.
    assert_eq!(
        reads_at(&sim),
        [7, 17, 27, 37, 47].map(|s| s * SECOND),
        "{}",
        sim.trace()
    );

    // Nothing owned: the safety tick only.
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    let before = reads_at(&sim).len();
    sim.run(2 * SAFETY_TICK_MS);
    assert_eq!(
        reads_at(&sim)[before..],
        [60, 90].map(|s| s * SECOND),
        "{}",
        sim.trace()
    );
}

#[test]
fn active_plan_event_reads_plan() {
    let mut sim = holding();
    let out = sim.send(Event::ActivePlan(OTHER_PLAN));
    let trace = sim.trace();
    assert_eq!(count(&out, &Command::ReadPlan), 1, "{trace}");
    assert!(settings(&out).is_empty(), "never acts on it alone\n{trace}");

    // The plan the last read named: nothing.
    let out = sim.send(Event::ActivePlan(BALANCED));
    assert_eq!(count(&out, &Command::ReadPlan), 0, "{}", sim.trace());

    // Nothing owned and no hold: nothing either.
    let mut sim = known_plan();
    let out = sim.send(Event::ActivePlan(OTHER_PLAN));
    assert_eq!(count(&out, &Command::ReadPlan), 0, "{}", sim.trace());
}

// Item 3.

#[test]
fn plan_switch_restores_the_old_plan_then_applies_to_the_new() {
    let mut sim = holding();
    let out = sim.send(read(OTHER_PLAN));
    assert_eq!(
        settings(&out),
        [
            &restore(Some(BALANCED), RestoreWhy::PlanSwitch),
            &apply(OTHER_PLAN, &KEYS)
        ],
        "{}",
        sim.trace()
    );
}

#[test]
fn failed_plan_read_releases_nothing() {
    let mut sim = holding();
    let out = sim.send(Event::Plan(Err(RESTORE_FAILED)));
    let trace = sim.trace();
    assert!(settings(&out).is_empty(), "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Notify(_))),
        "{trace}"
    );
    assert_eq!(overridden(&mut sim), owned(BALANCED, &KEYS));
    // A later read of a new plan is still a switch from the plan last read.
    let out = sim.send(read(OTHER_PLAN));
    assert_eq!(
        settings(&out),
        [
            &restore(Some(BALANCED), RestoreWhy::PlanSwitch),
            &apply(OTHER_PLAN, &KEYS)
        ],
        "{}",
        sim.trace()
    );
}

// Item 4.

#[test]
fn journal_mirror_is_a_copy_of_each_outcome_less_user_changes() {
    let mut sim = holding();
    assert_eq!(overridden(&mut sim), owned(BALANCED, &KEYS));

    sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    assert_eq!(
        overridden(&mut sim),
        owned(BALANCED, &[SettingKey::LidAc, SettingKey::SleepDc])
    );

    // A plan switch: the outcomes' journals, not a guess.
    sim.send(read(OTHER_PLAN));
    sim.send(restored(
        Some(BALANCED),
        RestoreWhy::PlanSwitch,
        Ok(()),
        vec![],
    ));
    assert_eq!(overridden(&mut sim), []);
    let keys = [SettingKey::LidAc, SettingKey::SleepDc];
    sim.send(applied(OTHER_PLAN, Ok(()), entries(OTHER_PLAN, &keys)));
    assert_eq!(overridden(&mut sim), owned(OTHER_PLAN, &keys));
}

// Items 5 and 15, and "UserChanged then a plan switch".

#[test]
fn user_change_releases_the_key_and_says_so() {
    let mut sim = holding();
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    let trace = sim.trace();
    assert_eq!(
        notified(&out, &Notice::UserChanged(SettingKey::LidDc)),
        1,
        "{trace}"
    );
    assert_eq!(logs(&out), 1, "the release is logged\n{trace}");
    assert!(settings(&out).is_empty(), "{trace}");

    // Settings writes the key on a second plan too: the same release, no second notice.
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: OTHER_PLAN,
        key: SettingKey::LidDc,
    }));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Notify(_))),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_released_key_stays_released_after_a_plan_switch() {
    let mut sim = holding();
    sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::LidDc,
    }));
    let out = sim.send(read(OTHER_PLAN));
    let kept = [SettingKey::LidAc, SettingKey::SleepDc];
    assert_eq!(
        settings(&out),
        [
            &restore(Some(BALANCED), RestoreWhy::PlanSwitch),
            &apply(OTHER_PLAN, &kept)
        ],
        "{}",
        sim.trace()
    );

    // Released only for the episode: the next hold writes all three.
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    let out = sim.send(working(1));
    assert_eq!(
        settings(&out),
        [&apply(OTHER_PLAN, &KEYS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_release_while_the_switchs_apply_is_on_its_way_applies_again_without_the_key() {
    // `p2-core-props` finding 1: the switch's read posts `Plan` before its check's `UserChanged`, so the switch's
    // apply goes out with all three keys and would write the key the user just took back.
    let mut sim = holding();
    let out = sim.send(read(OTHER_PLAN));
    assert_eq!(
        settings(&out),
        [
            &restore(Some(BALANCED), RestoreWhy::PlanSwitch),
            &apply(OTHER_PLAN, &KEYS)
        ],
        "{}",
        sim.trace()
    );
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: BALANCED,
        key: SettingKey::SleepDc,
    }));
    let trace = sim.trace();
    let kept = [SettingKey::LidAc, SettingKey::LidDc];
    assert_eq!(settings(&out), [&apply(OTHER_PLAN, &kept)], "{trace}");
    assert_eq!(
        notified(&out, &Notice::UserChanged(SettingKey::SleepDc)),
        1,
        "{trace}"
    );

    // Both answered in order: nothing more.
    let mut out = sim.send(applied(OTHER_PLAN, Ok(()), entries(OTHER_PLAN, &KEYS)));
    out.extend(sim.send(applied(OTHER_PLAN, Ok(()), entries(OTHER_PLAN, &kept))));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_release_named_on_another_plan_takes_the_active_plans_override_back() {
    // A failed restore left LidDc on the other plan; the user sets it there with `powercfg`. The key is released on
    // every plan, so Balanced's override of it must go too.
    let mut sim = known_plan();
    sim.send(working(1));
    let mut journal = entries(BALANCED, &KEYS);
    journal.extend(entries(OTHER_PLAN, &[SettingKey::LidDc]));
    sim.send(applied(BALANCED, Ok(()), journal));
    let out = sim.send(Event::UserChanged(OwnedKey {
        plan: OTHER_PLAN,
        key: SettingKey::LidDc,
    }));
    let kept = [SettingKey::LidAc, SettingKey::SleepDc];
    assert_eq!(settings(&out), [&apply(BALANCED, &kept)], "{}", sim.trace());
}

#[test]
fn every_key_released_while_an_apply_is_on_its_way_restores() {
    // With every key released no apply comes, so a restore takes back what the one on its way writes.
    let mut sim = known_plan();
    sim.send(working(1));
    let mut out = Vec::new();
    for key in KEYS {
        out = sim.send(Event::UserChanged(OwnedKey {
            plan: BALANCED,
            key,
        }));
    }
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{}",
        sim.trace()
    );
}

// Items 6 and 15.

#[test]
fn failed_apply_holds_with_requests_only_for_the_episode() {
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(GUARD_FAILED)),
        vec![],
    ));
    let trace = sim.trace();
    assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{trace}");
    assert_eq!(logs(&out), 1, "the switch is logged\n{trace}");
    assert!(settings(&out).is_empty(), "nothing was written\n{trace}");
    assert!(!out.contains(&OFF), "the requests stay\n{trace}");

    // No apply for the rest of the episode, not even after a plan switch.
    let out = sim.send(read(OTHER_PLAN));
    assert!(
        !settings(&out)
            .iter()
            .any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );

    // The next episode applies again; the same failure again warns no more in this streak.
    sim.send(idle(1));
    let out = sim.send(working(1));
    assert_eq!(
        settings(&out),
        [&apply(OTHER_PLAN, &KEYS)],
        "{}",
        sim.trace()
    );
    let out = sim.send(applied(
        OTHER_PLAN,
        Err(ApplyError::SafetyNet(GUARD_FAILED)),
        vec![],
    ));
    assert_eq!(notified(&out, &Notice::RequestsOnly), 0, "{}", sim.trace());

    // An apply that works ends the streak.
    sim.send(idle(1));
    sim.send(working(1));
    sim.send(applied(OTHER_PLAN, Ok(()), entries(OTHER_PLAN, &KEYS)));
    sim.send(idle(1));
    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    sim.send(working(1));
    let out = sim.send(applied(
        OTHER_PLAN,
        Err(ApplyError::SafetyNet(GUARD_FAILED)),
        vec![],
    ));
    assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{}", sim.trace());
}

#[test]
fn a_failed_write_restores_what_the_apply_left_at_once() {
    let mut sim = known_plan();
    sim.send(working(1));
    // The restore inside the apply could not put LidDc back.
    let left = entries(BALANCED, &[SettingKey::LidDc]);
    let out = sim.send(applied(
        BALANCED,
        Err(ApplyError::Write(WRITE_FAILED)),
        left,
    ));
    let trace = sim.trace();
    assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{trace}");
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{trace}"
    );
}

#[test]
fn a_failed_write_whose_own_restore_cannot_load_the_journal_still_restores() {
    // Power safety review finding 2: the apply saved its entries and wrote one, then the restore inside it could not
    // load the journal, so the outcome names no entry. The copy was empty: the first hold of the run.
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(Event::Done(crate::command::Outcome::Applied {
        plan: BALANCED,
        written: vec![SettingKey::LidAc],
        result: Err(ApplyError::Write(WRITE_FAILED)),
        journal: vec![],
    }));
    let trace = sim.trace();
    assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{trace}");
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{trace}"
    );

    // A safety net failure writes nothing, so it still restores nothing.
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(applied(
        BALANCED,
        Err(ApplyError::SafetyNet(GUARD_FAILED)),
        vec![],
    ));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn access_denied_holds_with_requests_only_for_the_run() {
    for denied in [
        ApplyError::SafetyNet(ACCESS_DENIED),
        ApplyError::Write(POLICY_DENIED),
    ] {
        let mut sim = known_plan();
        sim.send(working(1));
        let out = sim.send(applied(BALANCED, Err(denied), vec![]));
        assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{}", sim.trace());
        sim.send(idle(1));
        let out = sim.send(working(1));
        let trace = sim.trace();
        assert!(settings(&out).is_empty(), "{denied:?}\n{trace}");
        assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    }
}

#[test]
fn plan_changed_waits_for_the_next_plan_read() {
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(applied(BALANCED, Err(ApplyError::PlanChanged), vec![]));
    let trace = sim.trace();
    assert!(settings(&out).is_empty(), "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Notify(_))),
        "{trace}"
    );
    assert_eq!(count(&out, &Command::ReadPlan), 1, "{trace}");

    // The plan switched and back before the read: the same plan, applied again.
    let out = sim.send(read(BALANCED));
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());
    let out = sim.send(applied(BALANCED, Err(ApplyError::PlanChanged), vec![]));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
    // A new plan: the switch path.
    let out = sim.send(read(OTHER_PLAN));
    assert_eq!(
        settings(&out),
        [&apply(OTHER_PLAN, &KEYS)],
        "{}",
        sim.trace()
    );
}

// Item 7.

#[test]
fn holding_ends_releases_and_restores_at_once() {
    let mut sim = holding();
    let out = sim.send(idle(1));
    let trace = sim.trace();
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{trace}"
    );
    assert!(
        position(&out, &OFF) < position(&out, &restore(None, RestoreWhy::WorkEnded)),
        "{trace}"
    );
}

#[test]
fn holding_ends_with_nothing_owned_restores_nothing() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            owner: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(read(BALANCED));
    sim.send(working(1));
    let out = sim.send(idle(1));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

// Item 8, and the orchestrator's addition.

#[test]
fn a_restore_that_fails_twice_then_works_warns_once() {
    let mut sim = holding();
    sim.send(idle(1));
    let kept = entries(BALANCED, &KEYS);
    let mut out = sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        kept.clone(),
    ));
    out.extend(sim.run(SAFETY_TICK_MS));
    let retries = count(&out, &restore(None, RestoreWhy::WorkEnded));
    assert!(retries >= 1, "{}", sim.trace());
    out.extend(sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        kept,
    )));
    out.extend(sim.run(SAFETY_TICK_MS));
    let trace = sim.trace();
    assert_eq!(notified(&out, &Notice::RestoreFailed), 1, "{trace}");
    assert!(
        count(&out, &restore(None, RestoreWhy::WorkEnded)) > retries,
        "{trace}"
    );

    sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![]));
    let out = sim.run(2 * SAFETY_TICK_MS);
    assert!(settings(&out).is_empty(), "{}", sim.trace());

    // A new streak warns again.
    sim.send(working(1));
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    sim.send(idle(1));
    let out = sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        entries(BALANCED, &KEYS),
    ));
    assert_eq!(notified(&out, &Notice::RestoreFailed), 1, "{}", sim.trace());
}

#[test]
fn a_hold_that_starts_while_a_restore_fails_keeps_its_entries() {
    let mut sim = holding();
    sim.send(idle(1));
    let mut kept = entries(BALANCED, &KEYS);
    kept.extend(entries(OTHER_PLAN, &[SettingKey::LidDc]));
    sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(RESTORE_FAILED),
        kept.clone(),
    ));

    // A new hold takes over the entries on the active plan.
    let out = sim.send(working(1));
    assert_eq!(settings(&out), [&apply(BALANCED, &KEYS)], "{}", sim.trace());
    sim.send(applied(BALANCED, Ok(()), kept));

    // The retry takes only the other plan's entries.
    let out = sim.run(SAFETY_TICK_MS);
    let trace = sim.trace();
    let retries = settings(&out);
    assert!(!retries.is_empty(), "{trace}");
    assert!(
        retries
            .iter()
            .all(|c| **c == restore(Some(OTHER_PLAN), RestoreWhy::WorkEnded)),
        "{trace}"
    );

    // Done: no retry left while the hold runs.
    sim.send(restored(
        Some(OTHER_PLAN),
        RestoreWhy::WorkEnded,
        Ok(()),
        entries(BALANCED, &KEYS),
    ));
    let out = sim.run(SAFETY_TICK_MS);
    assert!(settings(&out).is_empty(), "{}", sim.trace());
    let out = sim.send(idle(1));
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{}",
        sim.trace()
    );
}

#[test]
fn a_restore_whose_journal_load_fails_is_retried_whatever_journal_holds() {
    let mut sim = holding();
    sim.send(idle(1));
    // Nothing is known, so `journal` is empty: never read as "nothing owned".
    let out = sim.send(restored(
        None,
        RestoreWhy::WorkEnded,
        Err(LOAD_FAILED),
        vec![],
    ));
    assert_eq!(notified(&out, &Notice::RestoreFailed), 1, "{}", sim.trace());
    let out = sim.run(SAFETY_TICK_MS);
    assert!(
        count(&out, &restore(None, RestoreWhy::WorkEnded)) >= 1,
        "{}",
        sim.trace()
    );
    assert_eq!(overridden(&mut sim), owned(BALANCED, &KEYS));
}

#[test]
fn leftover_journal_is_restored_on_each_tick_until_it_succeeds() {
    let leftover = Journal {
        owner: JournalOwner {
            pid: 4100,
            proc_start: FileTime(7),
            session: 1,
        },
        entries: entries(BALANCED, &[SettingKey::LidDc]),
    };
    let mut sim = Sim::new();
    let out = sim.send(
        Start {
            journal: Some(leftover),
            ..Start::default()
        }
        .event(),
    );
    assert!(
        settings(&out).is_empty(),
        "start-up already tried\n{}",
        sim.trace()
    );
    let leftover = restore(None, RestoreWhy::Leftover);
    let out = sim.run(SAFETY_TICK_MS);
    assert!(count(&out, &leftover) >= 1, "{}", sim.trace());
    sim.send(restored(
        None,
        RestoreWhy::Leftover,
        Err(RESTORE_FAILED),
        entries(BALANCED, &[SettingKey::LidDc]),
    ));
    let out = sim.run(SAFETY_TICK_MS);
    assert!(count(&out, &leftover) >= 1, "{}", sim.trace());
    sim.send(restored(None, RestoreWhy::Leftover, Ok(()), vec![]));
    let out = sim.run(2 * SAFETY_TICK_MS);
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

// Item 9, and the orchestrator's decision on `JOURNAL_CORRUPT`.

#[test]
fn journal_corrupt_shows_a_notice() {
    let mut sim = Sim::new();
    sim.start();
    let out = sim.send(Event::JournalCorrupt);
    assert_eq!(
        notified(&out, &Notice::JournalCorrupt),
        1,
        "{}",
        sim.trace()
    );
}

#[test]
fn a_corrupt_journal_at_apply_sends_one_restore_per_file() {
    let corrupt = || {
        applied(
            BALANCED,
            Err(ApplyError::SafetyNet(JOURNAL_CORRUPT)),
            vec![],
        )
    };
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(corrupt());
    let trace = sim.trace();
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{trace}"
    );
    assert_eq!(notified(&out, &Notice::RequestsOnly), 1, "{trace}");

    // The same file at the next episode's apply: no second restore.
    sim.send(idle(1));
    sim.send(working(1));
    let out = sim.send(corrupt());
    assert!(settings(&out).is_empty(), "{}", sim.trace());

    // Set aside: a new corrupt file gets its own restore.
    sim.send(Event::JournalCorrupt);
    sim.send(idle(1));
    sim.send(working(1));
    let out = sim.send(corrupt());
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "{}",
        sim.trace()
    );
}

// Items 10 and 15, and "Restore my power settings, then a plan switch".

#[test]
fn restore_my_power_settings_releases_every_key_and_keeps_the_requests() {
    let mut sim = holding();
    let out = sim.send(Event::Menu(MenuCommand::Restore));
    let trace = sim.trace();
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::UserAsked)],
        "{trace}"
    );
    assert_eq!(notified(&out, &Notice::RestoredByUser), 1, "{trace}");
    assert_eq!(logs(&out), 1, "the release is logged\n{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::Requests { .. })),
        "{trace}"
    );

    // Its answer: `state.json` at once, without the 1 s wait, and then the request file goes.
    let out = sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    let trace = sim.trace();
    let finished: Vec<_> = out
        .iter()
        .filter_map(|c| match c {
            Command::FinishRestoreRequest(file) => Some(file),
            _ => None,
        })
        .collect();
    assert_eq!(finished.len(), 1, "{trace}");
    assert!(finished[0].overridden.is_empty(), "{trace}");
    assert!(!finished[0].holding.is_empty(), "still holding\n{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::WriteState(_))),
        "{trace}"
    );
    // A later retry with the same reason finishes nothing.
    let out = sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    assert!(
        !out.iter()
            .any(|c| matches!(c, Command::FinishRestoreRequest(_))),
        "{}",
        sim.trace()
    );
}

#[test]
fn restore_my_power_settings_then_a_plan_switch_writes_nothing_until_work_ends() {
    let mut sim = holding();
    sim.send(Event::Menu(MenuCommand::Restore));
    sim.send(restored(None, RestoreWhy::UserAsked, Ok(()), vec![]));
    let mut out = sim.send(read(OTHER_PLAN));
    out.extend(sim.send(Event::SettingsOwner));
    out.extend(sim.send(Event::Session(SessionChange::ConsoleLeft)));
    out.extend(sim.send(Event::Session(SessionChange::ConsoleBack)));
    out.extend(sim.run(SAFETY_TICK_MS));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
    sim.send(idle(1));
    let out = sim.send(working(1));
    assert_eq!(
        settings(&out),
        [&apply(OTHER_PLAN, &KEYS)],
        "{}",
        sim.trace()
    );
}

// Item 11.

#[test]
fn quit_with_keep_awake_on() {
    let mut sim = known_plan();
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(60))));
    sim.watch();
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    let out = sim.send(Event::Menu(MenuCommand::Quit));
    assert_eq!(
        out,
        [
            OFF,
            restore(None, RestoreWhy::Quit),
            Command::DeleteHold(HoldId(KEEP_AWAKE.to_owned())),
            Command::Quit
        ],
        "{}",
        sim.trace()
    );
    // Nothing more after Quit.
    assert!(sim.send(Event::Tick).is_empty(), "{}", sim.trace());
    assert!(sim.send(read(OTHER_PLAN)).is_empty(), "{}", sim.trace());
}

// Item 12.

#[test]
fn session_end_during_a_hold_then_cancelled() {
    let mut sim = holding();
    let out = sim.send(Event::Session(SessionChange::Ending));
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::SessionEnd)],
        "{}",
        sim.trace()
    );
    sim.send(restored(None, RestoreWhy::SessionEnd, Ok(()), vec![]));

    // No apply until the end is cancelled: not at a plan switch, a new hold, the owner or the console.
    let mut out = sim.send(read(OTHER_PLAN));
    out.extend(sim.send(idle(1)));
    out.extend(sim.send(working(1)));
    out.extend(sim.send(Event::SettingsOwner));
    out.extend(sim.send(Event::Session(SessionChange::ConsoleBack)));
    out.extend(sim.run(SAFETY_TICK_MS));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );

    let out = sim.send(Event::Session(SessionChange::EndCancelled));
    assert_eq!(
        settings(&out),
        [&apply(OTHER_PLAN, &KEYS)],
        "{}",
        sim.trace()
    );
}

#[test]
fn session_ended_does_nothing_more() {
    let mut sim = holding();
    sim.send(Event::Session(SessionChange::Ending));
    let out = sim.send(Event::Session(SessionChange::Ended));
    assert!(out.is_empty(), "{}", sim.trace());
    let out = sim.send(read(OTHER_PLAN));
    assert!(
        !out.iter().any(|c| matches!(c, Command::Apply { .. })),
        "{}",
        sim.trace()
    );
}

// Item 13.

#[test]
fn console_left_and_back_during_a_hold() {
    let mut sim = known_plan();
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    sim.watch();
    sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    assert!(sim.requests().display, "{}", sim.trace());

    let out = sim.send(Event::Session(SessionChange::ConsoleLeft));
    let trace = sim.trace();
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::ConsoleLeft)],
        "{trace}"
    );
    assert_eq!(count(&out, &SYSTEM), 1, "the system request only\n{trace}");
    assert_eq!(logs(&out), 1, "the requests-only switch is logged\n{trace}");
    sim.send(restored(None, RestoreWhy::ConsoleLeft, Ok(()), vec![]));

    // Away: no apply at a plan switch.
    let out = sim.send(read(OTHER_PLAN));
    assert!(settings(&out).is_empty(), "{}", sim.trace());

    let out = sim.send(Event::Session(SessionChange::ConsoleBack));
    let trace = sim.trace();
    assert_eq!(count(&out, &SCREEN), 1, "{trace}");
    assert_eq!(settings(&out), [&apply(OTHER_PLAN, &KEYS)], "{trace}");
    assert!(
        position(&out, &SCREEN) < position(&out, &apply(OTHER_PLAN, &KEYS)),
        "requests first\n{trace}"
    );
}

// The card's other cases.

#[test]
fn an_apply_in_flight_when_the_hold_ends() {
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(idle(1));
    assert_eq!(
        settings(&out),
        [&restore(None, RestoreWhy::WorkEnded)],
        "queued behind the apply\n{}",
        sim.trace()
    );
    let mut out = sim.send(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
    out.extend(sim.send(restored(None, RestoreWhy::WorkEnded, Ok(()), vec![])));
    out.extend(sim.run(2 * SAFETY_TICK_MS));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

#[test]
fn a_plan_switch_while_an_apply_is_in_flight() {
    let mut sim = known_plan();
    sim.send(working(1));
    let out = sim.send(read(OTHER_PLAN));
    assert_eq!(
        settings(&out),
        [
            &restore(Some(BALANCED), RestoreWhy::PlanSwitch),
            &apply(OTHER_PLAN, &KEYS)
        ],
        "{}",
        sim.trace()
    );
    // The first apply wrote to Balanced, now inactive: the restore queued behind it covers that.
    let out = sim.send(applied(
        BALANCED,
        Err(ApplyError::PlanChanged),
        entries(BALANCED, &KEYS),
    ));
    let trace = sim.trace();
    assert!(settings(&out).is_empty(), "{trace}");
    assert_eq!(count(&out, &Command::ReadPlan), 0, "{trace}");
    sim.send(restored(
        Some(BALANCED),
        RestoreWhy::PlanSwitch,
        Ok(()),
        vec![],
    ));
    sim.send(applied(OTHER_PLAN, Ok(()), entries(OTHER_PLAN, &KEYS)));
    assert_eq!(overridden(&mut sim), owned(OTHER_PLAN, &KEYS));
}

#[test]
fn a_plan_switch_with_nothing_owned_restores_nothing() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            owner: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(read(BALANCED));
    sim.send(working(1));
    let out = sim.send(read(OTHER_PLAN));
    assert!(settings(&out).is_empty(), "{}", sim.trace());
}

// Random settings events in any order, with time moving in between (the full properties are `p2-core-props`).

#[derive(Clone, Debug)]
enum Op {
    Send(Event),
    Run(u64),
}

fn plan_id() -> impl Strategy<Value = Guid> {
    prop_oneof![Just(BALANCED), Just(OTHER_PLAN)]
}

fn key() -> impl Strategy<Value = SettingKey> {
    prop_oneof![
        Just(SettingKey::LidAc),
        Just(SettingKey::LidDc),
        Just(SettingKey::SleepDc)
    ]
}

fn journal() -> impl Strategy<Value = Vec<JournalEntry>> {
    (plan_id(), vec(key(), 0..3)).prop_map(|(plan, keys)| entries(plan, &keys))
}

fn failure() -> impl Strategy<Value = Failure> {
    prop_oneof![
        Just(GUARD_FAILED),
        Just(ACCESS_DENIED),
        Just(JOURNAL_CORRUPT),
        Just(LOAD_FAILED)
    ]
}

fn why() -> impl Strategy<Value = RestoreWhy> {
    prop_oneof![
        Just(RestoreWhy::WorkEnded),
        Just(RestoreWhy::UserAsked),
        Just(RestoreWhy::PlanSwitch),
        Just(RestoreWhy::Leftover),
        Just(RestoreWhy::SessionEnd),
        Just(RestoreWhy::ConsoleLeft),
    ]
}

fn apply_result() -> impl Strategy<Value = Result<(), ApplyError>> {
    prop_oneof![
        Just(Ok(())),
        Just(Err(ApplyError::PlanChanged)),
        failure().prop_map(|f| Err(ApplyError::SafetyNet(f))),
        failure().prop_map(|f| Err(ApplyError::Write(f))),
    ]
}

fn session_change() -> impl Strategy<Value = SessionChange> {
    prop_oneof![
        Just(SessionChange::Ending),
        Just(SessionChange::EndCancelled),
        Just(SessionChange::Ended),
        Just(SessionChange::ConsoleLeft),
        Just(SessionChange::ConsoleBack),
    ]
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        3 => Just(Event::Tick),
        3 => prop_oneof![Just(working(1)), Just(idle(1))],
        3 => plan_id().prop_map(read),
        1 => Just(Event::Plan(Err(RESTORE_FAILED))),
        1 => plan_id().prop_map(Event::ActivePlan),
        1 => Just(Event::SettingsOwner),
        1 => (plan_id(), key()).prop_map(|(plan, key)| Event::UserChanged(OwnedKey { plan, key })),
        1 => Just(Event::JournalCorrupt),
        2 => session_change().prop_map(Event::Session),
        1 => Just(Event::Menu(MenuCommand::Restore)),
        1 => (plan_id(), apply_result(), journal()).prop_map(|(p, r, j)| applied(p, r, j)),
        2 => (
            proptest::option::of(plan_id()),
            why(),
            prop_oneof![Just(Ok(())), failure().prop_map(Err)],
            journal(),
        )
            .prop_map(|(p, w, r, j)| restored(p, w, r, j)),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => event().prop_map(Op::Send),
        2 => (0..2 * SAFETY_TICK_MS).prop_map(Op::Run),
        1 => Just(Op::Send(Event::Menu(MenuCommand::Quit))),
    ]
}

fn start() -> impl Strategy<Value = Event> {
    (
        any::<bool>(),
        any::<bool>(),
        proptest::option::of(journal()),
    )
        .prop_map(|(owner, on_console, entries)| {
            Start {
                owner,
                on_console,
                journal: entries.filter(|e| !e.is_empty()).map(|entries| Journal {
                    owner: JournalOwner {
                        pid: 4100,
                        proc_start: FileTime(7),
                        session: 1,
                    },
                    entries,
                }),
                ..Start::default()
            }
            .event()
        })
}

proptest! {
    // A regression file would be a file this card does not own; a shrunk case becomes a named test instead.
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    /// `handle` never panics on any order of these events and `Sim` finds no deadline behind a `Tick`; no `Apply`
    /// goes out between a session end and its cancel, every `Apply` writes only the three keys to their target, and
    /// nothing follows `Quit`.
    #[test]
    fn settings_events_in_any_order(start in start(), ops in vec(op(), 0..40)) {
        let mut sim = Sim::new();
        sim.send(start);
        for op in ops {
            match op {
                Op::Send(event) => {
                    sim.send(event);
                }
                Op::Run(ms) => {
                    sim.run(ms);
                }
            }
        }
        let mut ending = false;
        let mut quit = false;
        for step in sim.steps() {
            prop_assert!(!quit || step.commands.is_empty(), "{}", sim.trace());
            match step.event {
                Event::Session(SessionChange::Ending | SessionChange::Ended) => ending = true,
                Event::Session(SessionChange::EndCancelled) => ending = false,
                _ => {}
            }
            for command in &step.commands {
                if let Command::Apply { writes, .. } = command {
                    prop_assert!(!ending, "{}", sim.trace());
                    prop_assert!(!writes.is_empty(), "{}", sim.trace());
                    prop_assert!(writes.iter().all(|(k, v)| KEYS.contains(k) && *v == 0));
                }
                quit |= *command == Command::Quit;
            }
        }
    }
}
