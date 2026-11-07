//! The reset of a lost journal (product "User settings safety" 7; architecture section 3, "A journal that cannot be
//! read"): on the plan in use, each key Wakeshot writes that holds exactly a value Wakeshot writes goes to the plan's
//! Windows default, and the bad file is set aside. Those values are a hold's (lid Do nothing, battery sleep Never)
//! and the sleep route's (sleep and screen off after 1 minute, AC and battery): a journal lost while the 1-minute
//! timeouts were in force would leave them for good. The accepted cost: a user's own value equal to one of these goes
//! to the default, with the notice.
//!
//! `wakeshot-windows` calls it through `power::journal::reset_locked`, under the journal mutex, from the tray's
//! executor and the uninstall hook; `sim::World` calls it as that executor does.
//!
//! The file is loaded once more because the core decides "lost" from its own copy, which cannot tell a load that
//! failed from a failed clean-up, and the file may have come back since the core's last try: only this run, under the
//! mutex, sees the truth. The writes come before the set-aside: a stop between the two leaves the bad file for the
//! next start, whose reset finds the defaults already in place; the other order would leave Wakeshot's values with
//! nothing that ever looks at them again.

use crate::command::Failure;
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::settings::{Guid, JournalEntry, SHORT_TIMEOUT_S, SettingKey};

/// The keys Wakeshot writes, in the order the reset takes them.
const KEYS: [SettingKey; 6] = SettingKey::ALL;

/// Do nothing for the lid and Never for sleep are both 0: a hold's value (architecture section 5).
const TARGET: u32 = 0;

/// A value Wakeshot writes to `key`: a hold's 0 for the hold keys, the sleep route's 1 minute for the short ones.
/// Battery sleep is both.
fn is_wakeshots_value(key: SettingKey, value: u32) -> bool {
    (SettingKey::HOLD.contains(&key) && value == TARGET)
        || (SettingKey::SHORT.contains(&key) && value == SHORT_TIMEOUT_S)
}

/// What the reset found in `journal.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    /// A file with known entries: it read after all, so nothing was reset; `Reset::journal` holds them.
    Entries,
    /// No file, or one with no known entry, for a journal that did not read: someone else restored it and cleaned up
    /// (the guard, `wakeshot restore`), so nothing was reset.
    Nothing,
    /// The file still did not read or did not parse, or a damaged one was set aside right before: the reset ran.
    Lost,
}

/// One key on the plan in use, as the reset found and left it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyReset {
    /// It held a value Wakeshot writes; the plan's default is written over `old`.
    Reset { old: u32, default: u32 },
    /// Another value: the user's own, left as it is.
    Theirs(u32),
    /// A value Wakeshot writes, which is also the plan's default (High performance sleeps Never): nothing to write.
    AlreadyDefault(u32),
    /// A read or the write failed: left as it is.
    Failed(Failure),
}

/// How one run of the reset went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reset {
    pub found: Found,
    /// The plan in use, once its id read.
    pub plan: Option<Guid>,
    /// Each key of the plan in use, in [`SettingKey::ALL`] order; empty when the reset did not reach the keys.
    pub keys: Vec<(SettingKey, KeyReset)>,
    /// The bad file moved to `journal.json.corrupt`; `None` when there was no bad file to move.
    pub set_aside: Option<Result<(), Failure>>,
    /// The first failed call of the reset itself. A failed set-aside is not one: the values are what matter, and the
    /// next save replaces the file.
    pub result: Result<(), Failure>,
    /// The known entries, when the file read after all.
    pub journal: Vec<JournalEntry>,
}

impl Reset {
    fn found(found: Found, journal: Vec<JournalEntry>) -> Self {
        Self {
            found,
            plan: None,
            keys: Vec::new(),
            set_aside: None,
            result: Ok(()),
            journal,
        }
    }

    /// The keys set to the plan's default, for `Outcome::ResetLost`.
    pub fn reset_keys(&self) -> Vec<SettingKey> {
        self.keys
            .iter()
            .filter(|(_, k)| matches!(k, KeyReset::Reset { .. }))
            .map(|&(key, _)| key)
            .collect()
    }

    fn fail(&mut self, failure: Failure) {
        if self.result.is_ok() {
            self.result = Err(failure);
        }
    }
}

/// One run of the reset. `damaged`: the core heard that a corrupt file was set aside (`Event::JournalCorrupt`), so a
/// missing file is that one, and the values it named are still in force.
pub fn reset_lost(plans: &mut impl PlanIo, net: &mut impl SafetyNet, damaged: bool) -> Reset {
    let bad = match net.load_journal() {
        JournalRead::Read(file) => {
            let known = JournalParts::split(file).known.entries;
            if !known.is_empty() {
                return Reset::found(Found::Entries, known);
            }
            false
        }
        JournalRead::Missing => false,
        JournalRead::Corrupt | JournalRead::Failed(_) => true,
    };
    if !bad && !damaged {
        return Reset::found(Found::Nothing, Vec::new());
    }
    let mut done = Reset::found(Found::Lost, Vec::new());

    match plans.active_plan() {
        Ok(plan) => {
            done.plan = Some(plan);
            reset_plan(plans, plan, &mut done);
        }
        Err(failure) => done.fail(failure),
    }

    if bad {
        done.set_aside = Some(net.set_aside_journal());
    }
    done
}

/// Each key of `plan`, then `set_active` once a value was written, since a stored value is in force only after it. A
/// stop between a write and `set_active` leaves the default stored but not in force, and a second run finds nothing at
/// Wakeshot's value to write; Windows' next plan change brings it into force. Only a stop in that moment does this.
fn reset_plan(plans: &mut impl PlanIo, plan: Guid, done: &mut Reset) {
    for key in KEYS {
        let outcome = match plans.read(plan, key) {
            Err(failure) => KeyReset::Failed(failure),
            Ok(live) if !is_wakeshots_value(key, live) => KeyReset::Theirs(live),
            Ok(live) => match plans.read_default(plan, key) {
                Err(failure) => KeyReset::Failed(failure),
                Ok(default) if default == live => KeyReset::AlreadyDefault(live),
                Ok(default) => match plans.write(plan, key, default) {
                    Ok(()) => KeyReset::Reset { old: live, default },
                    Err(failure) => KeyReset::Failed(failure),
                },
            },
        };
        if let KeyReset::Failed(failure) = outcome {
            done.fail(failure);
        }
        done.keys.push((key, outcome));
    }
    if done
        .keys
        .iter()
        .any(|(_, k)| matches!(k, KeyReset::Reset { .. }))
        && let Err(failure) = plans.set_active(plan)
    {
        done.fail(failure);
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::files::{JournalFile, JournalFileEntry, JournalKey};
    use crate::ownership::fake::{
        Action, Call, FAKE_DEFAULT_LID, FAKE_DEFAULT_OTHER, FAKE_DEFAULT_SLEEP_DC, FAKE_SCREEN_OFF,
        FakeJournal, FakePlans, SetBy,
    };
    use crate::process::FileTime;
    use crate::settings::{JournalOwner, PlanValues};

    use SettingKey::{LidAc, LidDc, SleepAc, SleepDc, VideoIdleAc, VideoIdleDc};

    /// Balanced, the active plan.
    const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const DEAD: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(5),
        session: 1,
    };
    const READ_FAILED: u32 = 32;

    /// A crashed tray's lid on AC and battery sleep (both at Wakeshot's value), and the user's own Sleep on battery.
    fn crashed() -> PlanValues {
        PlanValues {
            active: P1,
            lid_ac: 0,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 0,
            saver_threshold_dc: 20,
        }
    }

    fn rig(journal: FakeJournal) -> FakePlans {
        let fake = FakePlans::new(crashed());
        {
            let mut state = fake.state();
            for key in [LidAc, SleepDc] {
                state.set_stored(P1, key, 0, SetBy::Override);
                state.set_in_force(P1, key, 0, SetBy::Override);
            }
            state.journal = journal;
        }
        fake
    }

    fn run(fake: &FakePlans, damaged: bool) -> Reset {
        let (mut plans, mut net) = (fake.clone(), fake.clone());
        reset_lost(&mut plans, &mut net, damaged)
    }

    fn file(entries: Vec<JournalFileEntry>) -> JournalFile {
        JournalFile {
            version: JournalFile::VERSION,
            owner: DEAD,
            entries,
        }
    }

    fn known(key: SettingKey) -> JournalFileEntry {
        JournalFileEntry {
            plan: P1,
            key: JournalKey::Known(key),
            original: 1,
            written: 0,
            restored: false,
        }
    }

    fn in_force(fake: &FakePlans, key: SettingKey) -> (u32, SetBy) {
        let state = fake.state();
        (
            state.in_force(P1, key).unwrap(),
            state.in_force_by(P1, key).unwrap(),
        )
    }

    #[test]
    fn an_unreadable_journal_resets_only_wakeshots_values_then_sets_the_file_aside() {
        let fake = rig(FakeJournal::Unreadable(READ_FAILED));
        let done = run(&fake, false);
        assert_eq!(done.found, Found::Lost);
        assert_eq!(done.plan, Some(P1));
        assert_eq!(
            done.keys,
            [
                (
                    LidAc,
                    KeyReset::Reset {
                        old: 0,
                        default: FAKE_DEFAULT_LID
                    }
                ),
                (LidDc, KeyReset::Theirs(1)),
                (
                    SleepDc,
                    KeyReset::Reset {
                        old: 0,
                        default: FAKE_DEFAULT_SLEEP_DC
                    }
                ),
                (SleepAc, KeyReset::Theirs(10_800)),
                (VideoIdleAc, KeyReset::Theirs(FAKE_SCREEN_OFF[0])),
                (VideoIdleDc, KeyReset::Theirs(FAKE_SCREEN_OFF[1])),
            ]
        );
        assert_eq!(done.reset_keys(), [LidAc, SleepDc]);
        assert_eq!((done.set_aside, done.result), (Some(Ok(())), Ok(())));
        // In force at once, labelled as neither the user's nor ours.
        assert_eq!(in_force(&fake, LidAc), (FAKE_DEFAULT_LID, SetBy::Default));
        assert_eq!(in_force(&fake, LidDc), (1, SetBy::User));
        assert_eq!(
            in_force(&fake, SleepDc),
            (FAKE_DEFAULT_SLEEP_DC, SetBy::Default)
        );
        let state = fake.state();
        assert!(state.held().is_empty(), "{:?}", state.held());
        assert_eq!(
            (state.set_aside, &state.journal),
            (1, &FakeJournal::Missing)
        );
        // The file goes aside only after the values are back.
        assert_eq!(
            state.calls,
            [
                Call::LoadJournal,
                Call::ActivePlan,
                Call::Read(P1, LidAc),
                Call::ReadDefault(P1, LidAc),
                Call::Write(P1, LidAc, FAKE_DEFAULT_LID),
                Call::Read(P1, LidDc),
                Call::Read(P1, SleepDc),
                Call::ReadDefault(P1, SleepDc),
                Call::Write(P1, SleepDc, FAKE_DEFAULT_SLEEP_DC),
                Call::Read(P1, SleepAc),
                Call::Read(P1, VideoIdleAc),
                Call::Read(P1, VideoIdleDc),
                Call::SetActive(P1),
                Call::SetAsideJournal,
            ]
        );
    }

    #[test]
    fn the_sleep_routes_one_minute_is_reset_on_each_short_key_and_no_other_value() {
        // A crash while the 1-minute timeouts were in force: every short key at exactly 60 s goes to the default,
        // battery sleep too, where a hold's Never would also go.
        let fake = FakePlans::new(PlanValues {
            lid_ac: 1,
            sleep_ac: SHORT_TIMEOUT_S,
            sleep_dc: SHORT_TIMEOUT_S,
            ..crashed()
        });
        {
            let mut state = fake.state();
            for key in SettingKey::SHORT {
                state.set_stored(P1, key, SHORT_TIMEOUT_S, SetBy::Override);
                state.set_in_force(P1, key, SHORT_TIMEOUT_S, SetBy::Override);
            }
            state.journal = FakeJournal::Unreadable(READ_FAILED);
        }
        let done = run(&fake, false);
        let reset = |old, default| KeyReset::Reset { old, default };
        assert_eq!(
            done.keys,
            [
                (LidAc, KeyReset::Theirs(1)),
                (LidDc, KeyReset::Theirs(1)),
                (SleepDc, reset(SHORT_TIMEOUT_S, FAKE_DEFAULT_SLEEP_DC)),
                (SleepAc, reset(SHORT_TIMEOUT_S, FAKE_DEFAULT_OTHER)),
                (VideoIdleAc, reset(SHORT_TIMEOUT_S, FAKE_DEFAULT_OTHER)),
                (VideoIdleDc, reset(SHORT_TIMEOUT_S, FAKE_DEFAULT_OTHER)),
            ]
        );
        assert_eq!(done.result, Ok(()));
        for key in SettingKey::SHORT {
            let want = match key {
                SleepDc => FAKE_DEFAULT_SLEEP_DC,
                _ => FAKE_DEFAULT_OTHER,
            };
            assert_eq!(in_force(&fake, key), (want, SetBy::Default), "{key:?}");
        }

        // Only exact values are Wakeshot's: a minute and a second, 59 s, and a user's own AC sleep or screen Never
        // stay; the lid keys never take the short value as Wakeshot's.
        let fake = FakePlans::new(PlanValues {
            lid_ac: SHORT_TIMEOUT_S,
            lid_dc: 1,
            sleep_ac: 0,
            sleep_dc: SHORT_TIMEOUT_S + 1,
            ..crashed()
        });
        {
            let mut state = fake.state();
            for (key, value) in [(VideoIdleAc, SHORT_TIMEOUT_S - 1), (VideoIdleDc, 0)] {
                state.set_stored(P1, key, value, SetBy::User);
                state.set_in_force(P1, key, value, SetBy::User);
            }
            state.journal = FakeJournal::Corrupt;
        }
        let done = run(&fake, false);
        assert_eq!(done.reset_keys(), Vec::<SettingKey>::new(), "{done:?}");
        assert!(!fake.state().calls.contains(&Call::SetActive(P1)));
    }

    #[test]
    fn a_corrupt_file_is_reset_and_set_aside_the_same_way() {
        let fake = rig(FakeJournal::Corrupt);
        let done = run(&fake, false);
        assert_eq!(done.found, Found::Lost);
        assert_eq!(done.reset_keys(), [LidAc, SleepDc]);
        assert_eq!(done.set_aside, Some(Ok(())));
        assert_eq!(fake.state().set_aside, 1);
    }

    #[test]
    fn a_missing_file_is_reset_only_when_a_damaged_one_was_set_aside() {
        // Someone restored and deleted the file after the core's last try: nothing to reset.
        let fake = rig(FakeJournal::Missing);
        let done = run(&fake, false);
        assert_eq!(done, Reset::found(Found::Nothing, Vec::new()));
        assert_eq!(fake.state().calls, [Call::LoadJournal]);

        // The core heard the damaged file go aside: its values are still in force.
        let fake = rig(FakeJournal::Missing);
        let done = run(&fake, true);
        assert_eq!(done.found, Found::Lost);
        assert_eq!(done.reset_keys(), [LidAc, SleepDc]);
        assert_eq!(done.set_aside, None, "nothing left to move");
        assert_eq!(fake.state().set_aside, 0);
    }

    #[test]
    fn a_file_that_reads_after_all_resets_nothing() {
        for damaged in [false, true] {
            // Known entries: the restore routine's to take, with its owner check.
            let fake = rig(FakeJournal::File(file(vec![known(LidAc)])));
            let done = run(&fake, damaged);
            let entries = vec![JournalEntry {
                plan: P1,
                key: LidAc,
                original: 1,
                written: 0,
                restored: false,
            }];
            assert_eq!(done, Reset::found(Found::Entries, entries), "{damaged}");
            assert_eq!(fake.state().calls, [Call::LoadJournal], "{damaged}");
        }

        // Only entries a newer version wrote: nothing of this version's is known, as with no file.
        let other = JournalFileEntry {
            key: JournalKey::Other("lidAcNew".to_owned()),
            ..known(LidAc)
        };
        let fake = rig(FakeJournal::File(file(vec![other.clone()])));
        assert_eq!(run(&fake, false).found, Found::Nothing);
        let fake = rig(FakeJournal::File(file(vec![other])));
        let done = run(&fake, true);
        assert_eq!(done.found, Found::Lost);
        assert_eq!(done.set_aside, None, "a file that reads is never set aside");
    }

    #[test]
    fn nothing_at_wakeshots_value_writes_nothing() {
        let fake = FakePlans::new(PlanValues {
            lid_ac: 1,
            sleep_dc: 600,
            ..crashed()
        });
        fake.state().journal = FakeJournal::Unreadable(READ_FAILED);
        let done = run(&fake, false);
        assert_eq!(done.reset_keys(), Vec::<SettingKey>::new());
        assert!(!fake.state().calls.contains(&Call::SetActive(P1)));
        assert_eq!(done.set_aside, Some(Ok(())));
    }

    #[test]
    fn a_default_that_is_wakeshots_value_is_left() {
        // High performance ships battery sleep Never.
        let fake = rig(FakeJournal::Unreadable(READ_FAILED));
        fake.state().defaults.insert((P1, SleepDc), 0);
        let done = run(&fake, false);
        assert_eq!(done.keys[2], (SleepDc, KeyReset::AlreadyDefault(0)));
        assert_eq!(done.reset_keys(), [LidAc]);
        assert!(
            !fake
                .state()
                .calls
                .iter()
                .any(|c| matches!(c, Call::Write(_, SleepDc, _)))
        );
    }

    #[test]
    fn a_failed_call_leaves_its_key_and_the_rest_go_on() {
        let failure = |code| Failure {
            call: match code {
                1 => "FakePlans::active_plan",
                2 => "FakePlans::read",
                3 => "FakePlans::read_default",
                4 => "FakePlans::write",
                _ => "FakePlans::set_active",
            },
            code,
        };
        // The active plan does not read: nothing reset, the file still goes aside.
        let fake = rig(FakeJournal::Unreadable(READ_FAILED));
        fake.script(1, Action::Fail(1));
        let done = run(&fake, false);
        assert_eq!(
            (done.plan, done.keys.len(), done.result),
            (None, 0, Err(failure(1)))
        );
        assert_eq!(done.set_aside, Some(Ok(())));

        // The lid key's read, then its default read, then its write fail: battery sleep is still reset.
        for (at, code) in [(2, 2), (3, 3), (4, 4)] {
            let fake = rig(FakeJournal::Unreadable(READ_FAILED));
            fake.script(at, Action::Fail(code));
            let done = run(&fake, false);
            assert_eq!(done.keys[0], (LidAc, KeyReset::Failed(failure(code))));
            assert_eq!(done.reset_keys(), [SleepDc], "{code}");
            assert_eq!(done.result, Err(failure(code)));
            assert_eq!(in_force(&fake, LidAc), (0, SetBy::Override), "{code}");
        }

        // `set_active` fails: written, but not in force.
        let fake = rig(FakeJournal::Unreadable(READ_FAILED));
        fake.script(12, Action::Fail(5));
        let done = run(&fake, false);
        assert_eq!(done.reset_keys(), [LidAc, SleepDc]);
        assert_eq!(done.result, Err(failure(5)));
        assert_eq!(in_force(&fake, LidAc), (0, SetBy::Override));
    }

    #[test]
    fn a_failed_set_aside_is_logged_and_the_reset_still_counts() {
        let fake = rig(FakeJournal::Unreadable(READ_FAILED));
        fake.script(13, Action::Fail(5));
        let done = run(&fake, false);
        let failed = Failure {
            call: "FakePlans::set_aside_journal",
            code: 5,
        };
        assert_eq!((done.set_aside, done.result), (Some(Err(failed)), Ok(())));
        assert_eq!(done.reset_keys(), [LidAc, SleepDc]);
        assert_eq!(fake.state().journal, FakeJournal::Unreadable(READ_FAILED));
    }

    #[test]
    fn a_stop_at_any_call_leaves_nothing_a_second_run_cannot_finish() {
        // Fourteen calls in a full run (see the first test).
        for at in 0..14 {
            let fake = rig(FakeJournal::Unreadable(READ_FAILED));
            fake.script(at, Action::Stop);
            let stopped = catch_unwind(AssertUnwindSafe(|| run(&fake, false)));
            assert!(stopped.is_err(), "the stop at call {at} ran");
            // The next start: the file still does not read, so the core's window ends in a second reset.
            let done = run(&fake, false);
            assert!(done.result.is_ok(), "{at}: {done:?}");
            let state = fake.state();
            for key in [LidAc, SleepDc] {
                assert_ne!(state.stored(P1, key), Some(0), "{at}: {key:?}");
            }
            assert_eq!(state.journal, FakeJournal::Missing, "{at}");
        }
    }
}
