//! A fake for both ownership seams, for the stop-point and property tests of every track.
//!
//! Owner: core track (`p2-ownership-restore`, then `p2-ownership-apply`); first version from the contracts card. Its
//! public API is contract: the power track tests `DryRunPower`, the executor, the guard and restore mode against it.
//! Later cards may add to it, never change what is here.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::command::Failure;
use crate::files::JournalFile;
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::settings::{Guid, PlanValues, SettingKey};

/// The code the fake returns for a plan it does not hold. The fake's own choice: what Windows returns for a deleted
/// plan is not verified.
pub const FAKE_MISSING: u32 = 2;

/// The code `can_write` returns for a key in `FakeState::blocked` (access denied, as policy gives it).
pub const FAKE_BLOCKED: u32 = 5;

/// One call to either seam, with its arguments, in the order the routine made them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    ActivePlan,
    Plans,
    CanWrite(SettingKey),
    Read(Guid, SettingKey),
    Write(Guid, SettingKey, u32),
    SetActive(Guid),
    Values,
    LoadJournal,
    SaveJournal(JournalFile),
    DeleteJournal,
    SetAsideJournal,
    SetRunOnce,
    ClearRunOnce,
    EnsureGuard,
    StopGuard,
}

/// What happens at one call index (0 is the first call). `Fail`: the call returns that code and changes nothing.
/// `Stop`: the call panics before it does anything, as a crash would; dev and test builds unwind, so the test catches
/// it and runs the next start against the same state. `Run`: the closure changes the state (a user change, a plan
/// switch), then the call runs.
pub enum Action {
    Fail(u32),
    Stop,
    Run(Box<dyn FnMut(&mut FakeState) + Send>),
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fail(code) => write!(f, "Fail({code})"),
            Self::Stop => f.write_str("Stop"),
            Self::Run(_) => f.write_str("Run(..)"),
        }
    }
}

/// `journal.json` as the fake holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FakeJournal {
    Missing,
    Corrupt,
    /// Every load fails with this code.
    Unreadable(u32),
    File(JournalFile),
}

/// The guard process as the fake holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeGuard {
    None,
    Running,
    Stopping,
}

/// One plan. `stored` is what a read returns; `in_force` is what Windows runs with, copied from `stored` at each
/// `set_active` of the plan, so a test sees a value we wrote that is stored back but still in force. Both carry the
/// plan's own id in `active`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FakePlan {
    pub stored: PlanValues,
    pub in_force: PlanValues,
}

/// Everything the fake holds. Tests read and change it through `FakePlans::state` or an `Action::Run` closure.
#[derive(Debug)]
pub struct FakeState {
    pub active: Guid,
    pub plans: BTreeMap<Guid, FakePlan>,
    /// Keys whose `can_write` fails with `FAKE_BLOCKED`.
    pub blocked: Vec<SettingKey>,
    pub journal: FakeJournal,
    /// How many journals were set aside as corrupt.
    pub set_aside: u32,
    pub run_once: bool,
    pub guard: FakeGuard,
    pub guards_started: u32,
    pub calls: Vec<Call>,
    pub script: BTreeMap<usize, Action>,
}

impl FakeState {
    /// The user changes a value in Settings: stored, and in force at once when the plan is active.
    pub fn user_write(&mut self, plan: Guid, key: SettingKey, value: u32) {
        let active = self.active;
        if let Some(p) = self.plans.get_mut(&plan) {
            *value_mut(&mut p.stored, key) = value;
            if plan == active {
                *value_mut(&mut p.in_force, key) = value;
            }
        }
    }

    /// The user or another app switches the active plan; its stored values come into force.
    pub fn switch_plan(&mut self, plan: Guid) {
        if let Some(p) = self.plans.get_mut(&plan) {
            p.in_force = p.stored;
            self.active = plan;
        }
    }

    pub fn delete_plan(&mut self, plan: Guid) {
        self.plans.remove(&plan);
    }

    pub fn stored(&self, plan: Guid, key: SettingKey) -> Option<u32> {
        self.plans.get(&plan).map(|p| value(&p.stored, key))
    }

    pub fn in_force(&self, plan: Guid, key: SettingKey) -> Option<u32> {
        self.plans.get(&plan).map(|p| value(&p.in_force, key))
    }
}

/// A handle to one shared `FakeState`. The routines take the two seams as two `&mut` arguments, so a test passes two
/// clones of one handle; both log into one call list with one call index.
#[derive(Clone, Debug)]
pub struct FakePlans(Arc<Mutex<FakeState>>);

impl FakePlans {
    /// One active plan, `values.active`, with these values stored and in force; no journal, RunOnce or guard.
    pub fn new(values: PlanValues) -> Self {
        let state = FakeState {
            active: values.active,
            plans: BTreeMap::from([(
                values.active,
                FakePlan {
                    stored: values,
                    in_force: values,
                },
            )]),
            blocked: Vec::new(),
            journal: FakeJournal::Missing,
            set_aside: 0,
            run_once: false,
            guard: FakeGuard::None,
            guards_started: 0,
            calls: Vec::new(),
            script: BTreeMap::new(),
        };
        Self(Arc::new(Mutex::new(state)))
    }

    /// Adds an inactive plan, `values.active`.
    pub fn add_plan(&self, values: PlanValues) {
        let plan = FakePlan {
            stored: values,
            in_force: values,
        };
        self.state().plans.insert(values.active, plan);
    }

    pub fn script(&self, index: usize, action: Action) {
        self.state().script.insert(index, action);
    }

    /// The state, also after a `Stop` panicked while the fake was in use. Drop the guard before the next call to the
    /// fake: a call while it is held (even a temporary in the same statement) blocks forever.
    pub fn state(&self) -> MutexGuard<'_, FakeState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn begin(&self, call: Call, name: &'static str) -> Result<MutexGuard<'_, FakeState>, Failure> {
        let mut state = self.state();
        let index = state.calls.len();
        state.calls.push(call);
        match state.script.remove(&index) {
            None => Ok(state),
            Some(Action::Fail(code)) => Err(Failure { call: name, code }),
            Some(Action::Stop) => {
                drop(state);
                panic!("FakePlans: stopped at call {index} ({name})");
            }
            Some(Action::Run(mut change)) => {
                change(&mut state);
                Ok(state)
            }
        }
    }
}

fn value(values: &PlanValues, key: SettingKey) -> u32 {
    match key {
        SettingKey::LidAc => values.lid_ac,
        SettingKey::LidDc => values.lid_dc,
        SettingKey::SleepDc => values.sleep_dc,
    }
}

fn value_mut(values: &mut PlanValues, key: SettingKey) -> &mut u32 {
    match key {
        SettingKey::LidAc => &mut values.lid_ac,
        SettingKey::LidDc => &mut values.lid_dc,
        SettingKey::SleepDc => &mut values.sleep_dc,
    }
}

fn missing(call: &'static str) -> Failure {
    Failure {
        call,
        code: FAKE_MISSING,
    }
}

impl PlanIo for FakePlans {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        Ok(self
            .begin(Call::ActivePlan, "FakePlans::active_plan")?
            .active)
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        let state = self.begin(Call::Plans, "FakePlans::plans")?;
        Ok(state.plans.keys().copied().collect())
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        const NAME: &str = "FakePlans::can_write";
        let state = self.begin(Call::CanWrite(key), NAME)?;
        match state.blocked.contains(&key) {
            true => Err(Failure {
                call: NAME,
                code: FAKE_BLOCKED,
            }),
            false => Ok(()),
        }
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        const NAME: &str = "FakePlans::read";
        let state = self.begin(Call::Read(plan, key), NAME)?;
        state.stored(plan, key).ok_or(missing(NAME))
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        const NAME: &str = "FakePlans::write";
        let mut state = self.begin(Call::Write(plan, key, value), NAME)?;
        let p = state.plans.get_mut(&plan).ok_or(missing(NAME))?;
        *value_mut(&mut p.stored, key) = value;
        Ok(())
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        const NAME: &str = "FakePlans::set_active";
        let mut state = self.begin(Call::SetActive(plan), NAME)?;
        let p = state.plans.get_mut(&plan).ok_or(missing(NAME))?;
        p.in_force = p.stored;
        state.active = plan;
        Ok(())
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        const NAME: &str = "FakePlans::values";
        let state = self.begin(Call::Values, NAME)?;
        let p = state.plans.get(&state.active).ok_or(missing(NAME))?;
        Ok(p.stored)
    }
}

impl SafetyNet for FakePlans {
    fn load_journal(&mut self) -> JournalRead {
        match self.begin(Call::LoadJournal, "FakePlans::load_journal") {
            Err(failure) => JournalRead::Failed(failure),
            Ok(state) => match &state.journal {
                FakeJournal::Missing => JournalRead::Missing,
                FakeJournal::Corrupt => JournalRead::Corrupt,
                FakeJournal::Unreadable(code) => JournalRead::Failed(Failure {
                    call: "FakePlans::load_journal",
                    code: *code,
                }),
                FakeJournal::File(file) => JournalRead::Read(file.clone()),
            },
        }
    }

    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure> {
        let call = Call::SaveJournal(journal.clone());
        self.begin(call, "FakePlans::save_journal")?.journal = FakeJournal::File(journal.clone());
        Ok(())
    }

    fn delete_journal(&mut self) -> Result<(), Failure> {
        self.begin(Call::DeleteJournal, "FakePlans::delete_journal")?
            .journal = FakeJournal::Missing;
        Ok(())
    }

    fn set_aside_journal(&mut self) -> Result<(), Failure> {
        let mut state = self.begin(Call::SetAsideJournal, "FakePlans::set_aside_journal")?;
        state.journal = FakeJournal::Missing;
        state.set_aside += 1;
        Ok(())
    }

    fn set_run_once(&mut self) -> Result<bool, Failure> {
        let mut state = self.begin(Call::SetRunOnce, "FakePlans::set_run_once")?;
        let added = !state.run_once;
        state.run_once = true;
        Ok(added)
    }

    fn clear_run_once(&mut self) -> Result<(), Failure> {
        self.begin(Call::ClearRunOnce, "FakePlans::clear_run_once")?
            .run_once = false;
        Ok(())
    }

    fn ensure_guard(&mut self) -> Result<bool, Failure> {
        let mut state = self.begin(Call::EnsureGuard, "FakePlans::ensure_guard")?;
        if state.guard == FakeGuard::Running {
            return Ok(false);
        }
        state.guard = FakeGuard::Running;
        state.guards_started += 1;
        Ok(true)
    }

    fn stop_guard(&mut self) -> Result<(), Failure> {
        let mut state = self.begin(Call::StopGuard, "FakePlans::stop_guard")?;
        if state.guard == FakeGuard::Running {
            state.guard = FakeGuard::Stopping;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::files::JournalFileEntry;
    use crate::files::JournalKey;
    use crate::process::FileTime;
    use crate::settings::JournalOwner;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OTHER: Guid = Guid(0xa000_0000_0000_0000_0000_0000_0000_0001);

    fn haiks(plan: Guid) -> PlanValues {
        PlanValues {
            active: plan,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 600,
            saver_threshold_dc: 20,
        }
    }

    fn journal() -> JournalFile {
        JournalFile {
            version: JournalFile::VERSION,
            owner: JournalOwner {
                pid: 9120,
                proc_start: FileTime(5),
                session: 1,
            },
            entries: vec![JournalFileEntry {
                plan: BALANCED,
                key: JournalKey::Known(SettingKey::LidDc),
                original: 1,
                written: 0,
                restored: false,
            }],
        }
    }

    /// The shape of an apply's plan calls, as any routine drives the two seams.
    fn write_then_activate(
        plans: &mut impl PlanIo,
        net: &mut impl SafetyNet,
    ) -> Result<(), Failure> {
        net.save_journal(&journal())?;
        net.set_run_once()?;
        net.ensure_guard()?;
        let plan = plans.active_plan()?;
        plans.can_write(SettingKey::LidDc)?;
        plans.write(plan, SettingKey::LidDc, 0)?;
        plans.set_active(plan)
    }

    #[test]
    fn write_is_stored_and_in_force_only_after_set_active() {
        let fake = FakePlans::new(haiks(BALANCED));
        let mut plans = fake.clone();
        plans.write(BALANCED, SettingKey::LidDc, 0).unwrap();
        assert_eq!(fake.state().stored(BALANCED, SettingKey::LidDc), Some(0));
        assert_eq!(fake.state().in_force(BALANCED, SettingKey::LidDc), Some(1));
        assert_eq!(plans.values().unwrap().lid_dc, 0);

        plans.set_active(BALANCED).unwrap();
        assert_eq!(fake.state().in_force(BALANCED, SettingKey::LidDc), Some(0));
        assert_eq!(
            fake.state().calls[0],
            Call::Write(BALANCED, SettingKey::LidDc, 0)
        );
    }

    #[test]
    fn both_seams_log_into_one_call_list() {
        let fake = FakePlans::new(haiks(BALANCED));
        write_then_activate(&mut fake.clone(), &mut fake.clone()).unwrap();
        let state = fake.state();
        assert_eq!(
            state.calls,
            [
                Call::SaveJournal(journal()),
                Call::SetRunOnce,
                Call::EnsureGuard,
                Call::ActivePlan,
                Call::CanWrite(SettingKey::LidDc),
                Call::Write(BALANCED, SettingKey::LidDc, 0),
                Call::SetActive(BALANCED),
            ]
        );
        assert_eq!(state.journal, FakeJournal::File(journal()));
        assert!(state.run_once);
        assert_eq!((state.guard, state.guards_started), (FakeGuard::Running, 1));
    }

    #[test]
    fn fail_returns_the_code_and_changes_nothing() {
        let fake = FakePlans::new(haiks(BALANCED));
        fake.script(5, Action::Fail(87));
        let err = write_then_activate(&mut fake.clone(), &mut fake.clone()).unwrap_err();
        assert_eq!(
            err,
            Failure {
                call: "FakePlans::write",
                code: 87
            }
        );
        assert_eq!(fake.state().stored(BALANCED, SettingKey::LidDc), Some(1));

        fake.state().blocked.push(SettingKey::LidAc);
        let mut plans = fake.clone();
        assert_eq!(
            plans.can_write(SettingKey::LidAc).unwrap_err().code,
            FAKE_BLOCKED
        );
        assert_eq!(
            plans.read(OTHER, SettingKey::LidAc).unwrap_err().code,
            FAKE_MISSING
        );
    }

    #[test]
    fn stop_ends_the_run_and_the_next_start_sees_the_state() {
        let fake = FakePlans::new(haiks(BALANCED));
        fake.script(6, Action::Stop);
        let run = catch_unwind(AssertUnwindSafe(|| {
            write_then_activate(&mut fake.clone(), &mut fake.clone())
        }));
        assert!(run.is_err());
        {
            let state = fake.state();
            assert_eq!(state.stored(BALANCED, SettingKey::LidDc), Some(0));
            assert_eq!(state.in_force(BALANCED, SettingKey::LidDc), Some(1));
            assert!(state.run_once);
        }
        let mut net = fake.clone();
        assert_eq!(net.load_journal(), JournalRead::Read(journal()));
        assert_eq!(net.ensure_guard(), Ok(false));
    }

    #[test]
    fn run_changes_the_state_between_two_calls() {
        let fake = FakePlans::new(haiks(BALANCED));
        fake.add_plan(haiks(OTHER));
        fake.script(4, Action::Run(Box::new(|s| s.switch_plan(OTHER))));
        fake.script(
            5,
            Action::Run(Box::new(|s| s.user_write(OTHER, SettingKey::LidDc, 3))),
        );
        write_then_activate(&mut fake.clone(), &mut fake.clone()).unwrap();
        let state = fake.state();
        assert_eq!(state.active, BALANCED);
        assert_eq!(state.stored(OTHER, SettingKey::LidDc), Some(3));
        assert_eq!(state.in_force(OTHER, SettingKey::LidDc), Some(3));
    }

    #[test]
    fn safety_net_states() {
        let fake = FakePlans::new(haiks(BALANCED));
        let mut net = fake.clone();
        assert_eq!(net.load_journal(), JournalRead::Missing);
        fake.state().journal = FakeJournal::Corrupt;
        assert_eq!(net.load_journal(), JournalRead::Corrupt);
        net.set_aside_journal().unwrap();
        assert_eq!(net.load_journal(), JournalRead::Missing);
        assert_eq!(fake.state().set_aside, 1);
        fake.state().journal = FakeJournal::Unreadable(32);
        assert!(matches!(net.load_journal(), JournalRead::Failed(f) if f.code == 32));
        let next = fake.state().calls.len();
        fake.script(next, Action::Fail(5));
        assert!(matches!(net.load_journal(), JournalRead::Failed(f) if f.code == 5));

        assert_eq!(net.set_run_once(), Ok(true));
        assert_eq!(net.set_run_once(), Ok(false));
        net.clear_run_once().unwrap();
        assert!(!fake.state().run_once);

        assert_eq!(net.ensure_guard(), Ok(true));
        net.stop_guard().unwrap();
        assert_eq!(fake.state().guard, FakeGuard::Stopping);
        assert_eq!(net.ensure_guard(), Ok(true));
        assert_eq!(fake.state().guards_started, 2);
        net.delete_journal().unwrap();
        assert_eq!(net.load_journal(), JournalRead::Missing);
    }
}
