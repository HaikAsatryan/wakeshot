//! A fake for both ownership seams, for the stop-point and property tests of every track.
//!
//! Owner: core track (`p2-ownership-restore`, then `p2-ownership-apply`); first version from the contracts card. Its
//! public API is contract: the power track tests `DryRunPower`, the executor, the guard and restore mode against it.
//! Later cards may add to it, never change what is here.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::command::Failure;
use crate::files::{JournalFile, JournalKey};
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::settings::{Guid, JournalEntry, OwnedKey, PlanValues, SettingKey};

const KEYS: [SettingKey; 6] = SettingKey::ALL;

/// The code the fake returns for a plan it does not hold. The fake's own choice: what Windows returns for a deleted
/// plan is not verified.
pub const FAKE_MISSING: u32 = 2;

/// The code `can_write` returns for a key in `FakeState::blocked` (access denied, as policy gives it).
pub const FAKE_BLOCKED: u32 = 5;

/// `read_default` of a lid key on a plan with no entry in `FakeState::defaults`: Sleep, as Windows ships each plan
/// on the test laptop (research, "Windows defaults of the three keys").
pub const FAKE_DEFAULT_LID: u32 = 1;

/// `read_default` of battery sleep on a plan with no entry in `FakeState::defaults`: 3 minutes, Balanced's on the
/// Modern Standby test laptop (research, "Windows defaults of the three keys").
pub const FAKE_DEFAULT_SLEEP_DC: u32 = 180;

/// `read_default` of the other timeouts (AC sleep, screen off) on a plan with no entry in `FakeState::defaults`: the
/// fake's own choice, 5 minutes. The reset of a lost journal reads them for a key at the sleep route's 1 minute, since
/// it covers all six keys (`p2-6-sleep-route-fixes`).
pub const FAKE_DEFAULT_OTHER: u32 = 300;

/// A new fake plan's screen-off timeouts, AC then DC, which `PlanValues` does not carry: the fake's own choice, longer
/// than the sleep route's 1 minute, as most users' are.
pub const FAKE_SCREEN_OFF: [u32; 2] = [600, 300];

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
    ReadDefault(Guid, SettingKey),
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

/// Who set a stored or in-force value, so a property or stop-point test can tell a value Wakeshot still holds from
/// the user's own without knowing the routine's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetBy {
    /// The plan's values at the start, and every `FakeState::user_write`.
    User,
    /// A `write` of anything but the `original` of the fake's journal entry for that plan and key: a value Wakeshot
    /// holds. A write with no such entry is one too, so a routine that writes before it journals shows up.
    Override,
    /// A `write` of that entry's `original` (and not its `written`): Wakeshot putting the user's value back.
    Restore,
    /// A `write` of the plan's Windows default (`read_default`) with no known entry for that plan and key: the reset of
    /// a lost journal. Neither the user's own nor Wakeshot's to hold.
    Default,
}

/// [`SetBy`] for each key Wakeshot writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetByKeys {
    pub lid_ac: SetBy,
    pub lid_dc: SetBy,
    pub sleep_dc: SetBy,
    pub sleep_ac: SetBy,
    pub screen_ac: SetBy,
    pub screen_dc: SetBy,
}

impl SetByKeys {
    pub const USER: Self = Self {
        lid_ac: SetBy::User,
        lid_dc: SetBy::User,
        sleep_dc: SetBy::User,
        sleep_ac: SetBy::User,
        screen_ac: SetBy::User,
        screen_dc: SetBy::User,
    };

    pub fn get(&self, key: SettingKey) -> SetBy {
        match key {
            SettingKey::LidAc => self.lid_ac,
            SettingKey::LidDc => self.lid_dc,
            SettingKey::SleepDc => self.sleep_dc,
            SettingKey::SleepAc => self.sleep_ac,
            SettingKey::VideoIdleAc => self.screen_ac,
            SettingKey::VideoIdleDc => self.screen_dc,
        }
    }

    fn get_mut(&mut self, key: SettingKey) -> &mut SetBy {
        match key {
            SettingKey::LidAc => &mut self.lid_ac,
            SettingKey::LidDc => &mut self.lid_dc,
            SettingKey::SleepDc => &mut self.sleep_dc,
            SettingKey::SleepAc => &mut self.sleep_ac,
            SettingKey::VideoIdleAc => &mut self.screen_ac,
            SettingKey::VideoIdleDc => &mut self.screen_dc,
        }
    }
}

/// One plan. `stored` is what a read returns; `in_force` is what Windows runs with, copied from `stored` at each
/// `set_active` of the plan, so a test sees a value we wrote that is stored back but still in force. Both carry the
/// plan's own id in `active`. `stored_by` and `in_force_by` say who set each value, and move with them. The screen-off
/// timeouts, AC then DC, are apart because `PlanValues` does not carry them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FakePlan {
    pub stored: PlanValues,
    pub in_force: PlanValues,
    pub stored_by: SetByKeys,
    pub in_force_by: SetByKeys,
    pub stored_screen: [u32; 2],
    pub in_force_screen: [u32; 2],
}

impl FakePlan {
    fn new(values: PlanValues) -> Self {
        Self {
            stored: values,
            in_force: values,
            stored_by: SetByKeys::USER,
            in_force_by: SetByKeys::USER,
            stored_screen: FAKE_SCREEN_OFF,
            in_force_screen: FAKE_SCREEN_OFF,
        }
    }

    fn come_into_force(&mut self) {
        self.in_force = self.stored;
        self.in_force_by = self.stored_by;
        self.in_force_screen = self.stored_screen;
    }

    fn stored_mut(&mut self, key: SettingKey) -> &mut u32 {
        value_mut(&mut self.stored, &mut self.stored_screen, key)
    }

    fn in_force_mut(&mut self, key: SettingKey) -> &mut u32 {
        value_mut(&mut self.in_force, &mut self.in_force_screen, key)
    }
}

/// Everything the fake holds. Tests read and change it through `FakePlans::state` or an `Action::Run` closure.
#[derive(Debug)]
pub struct FakeState {
    pub active: Guid,
    pub plans: BTreeMap<Guid, FakePlan>,
    /// Keys whose `can_write` fails with `FAKE_BLOCKED`.
    pub blocked: Vec<SettingKey>,
    /// `read_default` of a plan and key; one not here reads [`FAKE_DEFAULT_LID`] or [`FAKE_DEFAULT_SLEEP_DC`].
    pub defaults: BTreeMap<(Guid, SettingKey), u32>,
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
            *p.stored_mut(key) = value;
            *p.stored_by.get_mut(key) = SetBy::User;
            if plan == active {
                *p.in_force_mut(key) = value;
                *p.in_force_by.get_mut(key) = SetBy::User;
            }
        }
    }

    /// The user or another app switches the active plan; its stored values come into force.
    pub fn switch_plan(&mut self, plan: Guid) {
        if let Some(p) = self.plans.get_mut(&plan) {
            p.come_into_force();
            self.active = plan;
        }
    }

    pub fn delete_plan(&mut self, plan: Guid) {
        self.plans.remove(&plan);
    }

    pub fn stored(&self, plan: Guid, key: SettingKey) -> Option<u32> {
        self.plans
            .get(&plan)
            .map(|p| value(&p.stored, &p.stored_screen, key))
    }

    pub fn in_force(&self, plan: Guid, key: SettingKey) -> Option<u32> {
        self.plans
            .get(&plan)
            .map(|p| value(&p.in_force, &p.in_force_screen, key))
    }

    pub fn stored_by(&self, plan: Guid, key: SettingKey) -> Option<SetBy> {
        self.plans.get(&plan).map(|p| p.stored_by.get(key))
    }

    pub fn in_force_by(&self, plan: Guid, key: SettingKey) -> Option<SetBy> {
        self.plans.get(&plan).map(|p| p.in_force_by.get(key))
    }

    /// Sets a stored value as a crash or an earlier run left it, with no call logged. A starting state only: a test
    /// step goes through the seams or `user_write`.
    pub fn set_stored(&mut self, plan: Guid, key: SettingKey, value: u32, by: SetBy) {
        if let Some(p) = self.plans.get_mut(&plan) {
            *p.stored_mut(key) = value;
            *p.stored_by.get_mut(key) = by;
        }
    }

    /// Sets an in-force value as a crash or an earlier run left it, with no call logged (see `set_stored`).
    pub fn set_in_force(&mut self, plan: Guid, key: SettingKey, value: u32, by: SetBy) {
        if let Some(p) = self.plans.get_mut(&plan) {
            *p.in_force_mut(key) = value;
            *p.in_force_by.get_mut(key) = by;
        }
    }

    /// The reset of a lost journal reports `default` written to `key` on `plan`: where it still holds, it is
    /// [`SetBy::Default`]. The reset writes before it sets the file aside, so when its own load failed on a file that
    /// names the key, the write looked like an override (`p2-6-props-flake`).
    pub fn mark_reset(&mut self, plan: Guid, key: SettingKey, default: u32) {
        let active = self.active;
        if let Some(p) = self.plans.get_mut(&plan) {
            if value(&p.stored, &p.stored_screen, key) == default {
                *p.stored_by.get_mut(key) = SetBy::Default;
            }
            if plan == active && value(&p.in_force, &p.in_force_screen, key) == default {
                *p.in_force_by.get_mut(key) = SetBy::Default;
            }
        }
    }

    /// The known entries of the journal file; none when it is missing, corrupt or unreadable.
    pub fn known_entries(&self) -> Vec<JournalEntry> {
        match &self.journal {
            FakeJournal::File(file) => JournalParts::split(file.clone()).known.entries,
            _ => Vec::new(),
        }
    }

    /// Every key whose value Wakeshot still holds: stored on any plan, or in force on the active plan, as an override.
    /// An inactive plan's in-force values do not count: activating it brings its stored values into force.
    pub fn held(&self) -> Vec<OwnedKey> {
        let mut held = Vec::new();
        for (&plan, p) in &self.plans {
            for key in KEYS {
                let in_force = plan == self.active && p.in_force_by.get(key) == SetBy::Override;
                if p.stored_by.get(key) == SetBy::Override || in_force {
                    held.push(OwnedKey { plan, key });
                }
            }
        }
        held
    }

    /// The held keys with no known entry in the journal file: the values no restore can bring back. The stop-point
    /// and property tests of every track assert this is empty.
    pub fn held_without_entry(&self) -> Vec<OwnedKey> {
        let entries = self.known_entries();
        let mut held = self.held();
        held.retain(|k| !entries.iter().any(|e| e.plan == k.plan && e.key == k.key));
        held
    }

    /// Windows' default of `key` on `plan`, as `read_default` gives it.
    pub fn default_of(&self, plan: Guid, key: SettingKey) -> u32 {
        self.defaults
            .get(&(plan, key))
            .copied()
            .unwrap_or(match key {
                SettingKey::LidAc | SettingKey::LidDc => FAKE_DEFAULT_LID,
                SettingKey::SleepDc => FAKE_DEFAULT_SLEEP_DC,
                SettingKey::SleepAc | SettingKey::VideoIdleAc | SettingKey::VideoIdleDc => {
                    FAKE_DEFAULT_OTHER
                }
            })
    }

    fn write_label(&self, plan: Guid, key: SettingKey, value: u32) -> SetBy {
        let entry = match &self.journal {
            FakeJournal::File(file) => file
                .entries
                .iter()
                .find(|e| e.plan == plan && e.key == JournalKey::Known(key))
                .map(|e| (e.original, e.written)),
            _ => None,
        };
        match entry {
            Some((original, written)) if value == original && value != written => SetBy::Restore,
            Some(_) => SetBy::Override,
            // Only the reset writes with no entry on purpose; any other value then is ours, so a routine that writes
            // before it journals still shows.
            None if value == self.default_of(plan, key) => SetBy::Default,
            None => SetBy::Override,
        }
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
            plans: BTreeMap::from([(values.active, FakePlan::new(values))]),
            blocked: Vec::new(),
            defaults: BTreeMap::new(),
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
        self.state()
            .plans
            .insert(values.active, FakePlan::new(values));
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

fn value(values: &PlanValues, screen: &[u32; 2], key: SettingKey) -> u32 {
    match key {
        SettingKey::LidAc => values.lid_ac,
        SettingKey::LidDc => values.lid_dc,
        SettingKey::SleepDc => values.sleep_dc,
        SettingKey::SleepAc => values.sleep_ac,
        SettingKey::VideoIdleAc => screen[0],
        SettingKey::VideoIdleDc => screen[1],
    }
}

fn value_mut<'a>(
    values: &'a mut PlanValues,
    screen: &'a mut [u32; 2],
    key: SettingKey,
) -> &'a mut u32 {
    match key {
        SettingKey::LidAc => &mut values.lid_ac,
        SettingKey::LidDc => &mut values.lid_dc,
        SettingKey::SleepDc => &mut values.sleep_dc,
        SettingKey::SleepAc => &mut values.sleep_ac,
        SettingKey::VideoIdleAc => &mut screen[0],
        SettingKey::VideoIdleDc => &mut screen[1],
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
        let by = state.write_label(plan, key, value);
        let p = state.plans.get_mut(&plan).ok_or(missing(NAME))?;
        *p.stored_mut(key) = value;
        *p.stored_by.get_mut(key) = by;
        Ok(())
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        const NAME: &str = "FakePlans::set_active";
        let mut state = self.begin(Call::SetActive(plan), NAME)?;
        let p = state.plans.get_mut(&plan).ok_or(missing(NAME))?;
        p.come_into_force();
        state.active = plan;
        Ok(())
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        const NAME: &str = "FakePlans::values";
        let state = self.begin(Call::Values, NAME)?;
        let p = state.plans.get(&state.active).ok_or(missing(NAME))?;
        Ok(p.stored)
    }

    fn read_default(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        const NAME: &str = "FakePlans::read_default";
        let state = self.begin(Call::ReadDefault(plan, key), NAME)?;
        match state.plans.contains_key(&plan) {
            true => Ok(state.default_of(plan, key)),
            false => Err(missing(NAME)),
        }
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
        let fake = FakePlans::new(user_values(BALANCED));
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
        let fake = FakePlans::new(user_values(BALANCED));
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
        let fake = FakePlans::new(user_values(BALANCED));
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
        let fake = FakePlans::new(user_values(BALANCED));
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
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(OTHER));
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
        let fake = FakePlans::new(user_values(BALANCED));
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

    #[test]
    fn writes_take_their_label_from_the_journal_entry() {
        let fake = FakePlans::new(user_values(BALANCED));
        let mut plans = fake.clone();
        plans.write(BALANCED, SettingKey::LidAc, 0).unwrap();
        assert_eq!(
            fake.state().stored_by(BALANCED, SettingKey::LidAc),
            Some(SetBy::Override),
            "no journal: a write is an override"
        );

        fake.state().journal = FakeJournal::File(journal());
        plans.write(BALANCED, SettingKey::LidDc, 0).unwrap();
        assert_eq!(
            fake.state().stored_by(BALANCED, SettingKey::LidDc),
            Some(SetBy::Override)
        );
        plans.write(BALANCED, SettingKey::LidDc, 1).unwrap();
        let state = fake.state();
        assert_eq!(
            state.stored_by(BALANCED, SettingKey::LidDc),
            Some(SetBy::Restore)
        );
        assert_eq!(
            state.in_force_by(BALANCED, SettingKey::LidDc),
            Some(SetBy::User)
        );
        assert_eq!(
            state.stored_by(BALANCED, SettingKey::SleepDc),
            Some(SetBy::User)
        );
    }

    #[test]
    fn labels_move_with_the_values() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(OTHER));
        fake.state().journal = FakeJournal::File(journal());
        let mut plans = fake.clone();
        plans.write(BALANCED, SettingKey::LidDc, 0).unwrap();
        plans.set_active(BALANCED).unwrap();
        assert_eq!(
            fake.state().in_force_by(BALANCED, SettingKey::LidDc),
            Some(SetBy::Override)
        );

        let mut state = fake.state();
        state.user_write(BALANCED, SettingKey::LidDc, 3);
        assert_eq!(
            state.in_force_by(BALANCED, SettingKey::LidDc),
            Some(SetBy::User)
        );
        state.set_stored(OTHER, SettingKey::LidAc, 0, SetBy::Override);
        state.switch_plan(OTHER);
        assert_eq!(
            (
                state.in_force(OTHER, SettingKey::LidAc),
                state.in_force_by(OTHER, SettingKey::LidAc)
            ),
            (Some(0), Some(SetBy::Override))
        );
        state.set_in_force(OTHER, SettingKey::SleepDc, 0, SetBy::Override);
        assert_eq!(
            state.in_force_by(OTHER, SettingKey::SleepDc),
            Some(SetBy::Override)
        );
        drop(state);
        assert!(fake.state().calls[2..].is_empty(), "setters log no call");
    }

    #[test]
    fn held_counts_stored_values_and_the_active_plans_in_force_values() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(OTHER));
        let mut state = fake.state();
        assert!(state.held().is_empty());
        state.set_stored(OTHER, SettingKey::SleepDc, 0, SetBy::Override);
        state.set_in_force(OTHER, SettingKey::LidAc, 0, SetBy::Override);
        state.set_in_force(BALANCED, SettingKey::LidDc, 0, SetBy::Override);
        state.set_stored(BALANCED, SettingKey::LidAc, 1, SetBy::Restore);
        let lid_dc = OwnedKey {
            plan: BALANCED,
            key: SettingKey::LidDc,
        };
        let sleep_dc = OwnedKey {
            plan: OTHER,
            key: SettingKey::SleepDc,
        };
        assert_eq!(state.held(), [lid_dc, sleep_dc]);
        assert_eq!(state.held_without_entry(), [lid_dc, sleep_dc]);
        assert!(state.known_entries().is_empty());

        state.journal = FakeJournal::File(journal());
        assert_eq!(state.known_entries().len(), 1);
        assert_eq!(state.held_without_entry(), [sleep_dc]);
        state.journal = FakeJournal::Corrupt;
        assert_eq!(state.held_without_entry(), [lid_dc, sleep_dc]);
    }
}
