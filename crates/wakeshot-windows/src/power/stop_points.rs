//! The stop-point walk against the real parts: a plan copy and `FileNet` in a temp folder (test only).
//!
//! Owner: `p3-stop-points`.
//!
//! Track note 4 (`docs/phases/03-windows-power.md`): the core walks every stop point against its fake; this walk runs
//! the same routines through the real `PlanIo` and `SafetyNet`. `CopyAsActive` over a `TestPlan` stands in for the
//! user's plan and is never active, so no write reaches the user's own plan. The `real_` tests run alone on the
//! real laptop: `cargo test --locked -p wakeshot-windows real_stop_points -- --ignored --test-threads=1`.

use std::cell::RefCell;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Once};
use std::time::Instant;

use wakeshot_core::command::Failure;
use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
use wakeshot_core::ownership::apply::Applied;
use wakeshot_core::ownership::check::Checked;
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::journal::JournalParts;
use wakeshot_core::ownership::restore::{Found, Restored};
use wakeshot_core::process::FileTime;
use wakeshot_core::settings::{
    Guid, Journal, JournalEntry, JournalOwner, OwnedKey, PlanValues, SettingKey,
};

use super::journal::{
    FileNet, GuardCall, RUN_ONCE_NAME, RecordingGuard, apply_locked, check_locked, restore_locked,
};
use super::plan::{CopyAsActive, TestPlan, Win32Power};
use crate::log::MemoryLog;
use crate::store::{AutoStart, DataDir, Names, ScratchKey, StartKey};

use SettingKey::{LidAc, LidDc, SleepDc};

const KEYS: [SettingKey; 3] = [LidAc, LidDc, SleepDc];

/// Every key's target (architecture section 5, "Power settings"): Do nothing, and Never.
const TARGET: u32 = 0;

/// What a scripted failure returns. `ERROR_GEN_FAILURE`: the restore routine branches on no code.
const SCRIPTED: Failure = Failure {
    call: "scripted failure",
    code: 31,
};

/// The owner a start's journal names: a tray that is gone, as at the next start. The routines never check
/// liveness (their callers do), so any values serve.
const GONE_TRAY: JournalOwner = JournalOwner {
    pid: 9120,
    proc_start: FileTime(5),
    session: 1,
};

/// The tray that applies in the walk.
const THIS_TRAY: JournalOwner = JournalOwner {
    pid: 4242,
    proc_start: FileTime(7),
    session: 1,
};

/// No plan has this id, so an entry on it reads as "plan deleted"; `Rig::new` checks that it is not listed.
const DELETED_PLAN: Guid = Guid(0x5707_0000_0000_4000_8000_0000_0000_0d1e);

/// The RunOnce value names it, but under the test key, which Windows never runs.
const TRAY_EXE: &str = r"C:\Wakeshot stop points\wakeshot-tray.exe";

/// What the shared counter does at its scripted index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    /// Panics before the call reaches the real part, as a crash there would end the routine.
    Stop,
    /// Returns `SCRIPTED` without reaching the real part.
    Fail,
}

/// The panic payload of a scripted stop. The walk catches only this one, so any other panic still fails the test.
#[derive(Debug)]
struct StopMarker;

/// One call counter for both wrappers of a run, so index k counts plan calls and safety net calls in the order the
/// routine made them, as the core's fake does.
#[derive(Debug)]
struct Script {
    at: Option<(usize, Act)>,
    calls: RefCell<Vec<String>>,
}

impl Script {
    fn new(at: Option<(usize, Act)>) -> Rc<Self> {
        Rc::new(Self {
            at,
            calls: RefCell::new(Vec::new()),
        })
    }

    fn step(&self, call: String) -> Result<(), Failure> {
        let index = {
            let mut calls = self.calls.borrow_mut();
            calls.push(call);
            calls.len() - 1
        };
        match self.at {
            Some((at, Act::Stop)) if at == index => panic::panic_any(StopMarker),
            Some((at, Act::Fail)) if at == index => Err(SCRIPTED),
            _ => Ok(()),
        }
    }

    /// Every call so far, the scripted one included.
    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

/// A `PlanIo` or `SafetyNet` over the real part below it that counts each call on the shared script.
struct StopAt<'a, T> {
    inner: &'a mut T,
    script: Rc<Script>,
}

impl<'a, T> StopAt<'a, T> {
    fn new(inner: &'a mut T, script: &Rc<Script>) -> Self {
        Self {
            inner,
            script: script.clone(),
        }
    }
}

impl<P: PlanIo> PlanIo for StopAt<'_, P> {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        self.script.step("active_plan".into())?;
        self.inner.active_plan()
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        self.script.step("plans".into())?;
        self.inner.plans()
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        self.script.step(format!("can_write {key:?}"))?;
        self.inner.can_write(key)
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        self.script.step(format!("read {plan} {key:?}"))?;
        self.inner.read(plan, key)
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        self.script.step(format!("write {plan} {key:?} {value}"))?;
        self.inner.write(plan, key, value)
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        self.script.step(format!("set_active {plan}"))?;
        self.inner.set_active(plan)
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        self.script.step("values".into())?;
        self.inner.values()
    }

    fn read_default(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        self.script.step(format!("read_default {plan} {key:?}"))?;
        self.inner.read_default(plan, key)
    }
}

impl<N: SafetyNet> SafetyNet for StopAt<'_, N> {
    fn load_journal(&mut self) -> JournalRead {
        match self.script.step("load_journal".into()) {
            Ok(()) => self.inner.load_journal(),
            Err(failure) => JournalRead::Failed(failure),
        }
    }

    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure> {
        let entries = journal.entries.len();
        self.script
            .step(format!("save_journal ({entries} entries)"))?;
        self.inner.save_journal(journal)
    }

    fn delete_journal(&mut self) -> Result<(), Failure> {
        self.script.step("delete_journal".into())?;
        self.inner.delete_journal()
    }

    fn set_aside_journal(&mut self) -> Result<(), Failure> {
        self.script.step("set_aside_journal".into())?;
        self.inner.set_aside_journal()
    }

    fn set_run_once(&mut self) -> Result<bool, Failure> {
        self.script.step("set_run_once".into())?;
        self.inner.set_run_once()
    }

    fn clear_run_once(&mut self) -> Result<(), Failure> {
        self.script.step("clear_run_once".into())?;
        self.inner.clear_run_once()
    }

    fn ensure_guard(&mut self) -> Result<bool, Failure> {
        self.script.step("ensure_guard".into())?;
        self.inner.ensure_guard()
    }

    fn stop_guard(&mut self) -> Result<(), Failure> {
        self.script.step("stop_guard".into())?;
        self.inner.stop_guard()
    }
}

/// Runs `f`; `None` when it reached the scripted stop. Dev and test builds unwind (release aborts), and any other
/// panic is passed on.
fn until_stop<T>(f: impl FnOnce() -> T) -> Option<T> {
    quiet_stops();
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(done) => Some(done),
        Err(payload) if payload.is::<StopMarker>() => None,
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// A walk stops thousands of times on purpose; the default hook would print each one. Every other panic still goes
/// to the hook that was there before.
fn quiet_stops() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let before = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !info.payload().is::<StopMarker>() {
                before(info);
            }
        }));
    });
}

/// How one key stands when a case starts (walk notation of `p1-design-verify-fixes`, Log 16:35).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    /// No entry: the user's value stored and in force.
    Free,
    /// Unmarked, ours stored and in force: an apply finished, or E3 (a restore failed on a read or on its mark).
    Ours,
    /// E1: marked, the user's value stored again, ours still in force (a restore wrote it, the call failed).
    E1,
    /// E2: marked, ours stored and in force (a restore stopped after its mark).
    E2,
    /// Unmarked, but the user set this value in Settings since; a Settings change puts the whole plan in force.
    User(u32),
}

#[derive(Clone, Copy, Debug)]
enum Routine {
    /// Each key to the target. A journaled key not named is released (track note 6).
    Apply(&'static [SettingKey]),
    /// `one_plan`: `Restore { plan: Some(copy) }`, else `None`.
    Restore {
        one_plan: bool,
    },
    Check,
}

#[derive(Clone, Debug)]
struct Start {
    name: &'static str,
    keys: [Held; 3],
    /// An entry on a plan that is not listed, beside the copy's.
    deleted_plan: bool,
    /// An entry with a key this version does not know (correction 12).
    unknown: bool,
    routine: Routine,
}

impl Start {
    fn new(name: &'static str, keys: [Held; 3], routine: Routine) -> Self {
        Self {
            name,
            keys,
            deleted_plan: false,
            unknown: false,
            routine,
        }
    }

    fn with_deleted_plan(self) -> Self {
        Self {
            deleted_plan: true,
            ..self
        }
    }

    fn with_unknown(self, unknown: bool) -> Self {
        Self { unknown, ..self }
    }

    fn stored(&self, rig: &Rig, i: usize) -> u32 {
        match self.keys[i] {
            Held::Free | Held::E1 => rig.user[i],
            Held::Ours | Held::E2 => TARGET,
            Held::User(value) => value,
        }
    }

    fn in_force(&self, rig: &Rig, i: usize) -> u32 {
        if self.keys.iter().any(|h| matches!(h, Held::User(_))) {
            return self.stored(rig, i);
        }
        match self.keys[i] {
            Held::Free => rig.user[i],
            _ => TARGET,
        }
    }

    /// The value every key must have once the next restore succeeded: the user's, or the user's own later change.
    fn expected(&self, rig: &Rig, i: usize) -> u32 {
        match self.keys[i] {
            Held::User(value) => value,
            _ => rig.user[i],
        }
    }

    fn deleted_plan_entry() -> JournalEntry {
        JournalEntry {
            plan: DELETED_PLAN,
            key: LidDc,
            original: 1,
            written: TARGET,
            restored: false,
        }
    }

    fn entries(&self, rig: &Rig) -> Vec<JournalEntry> {
        let mut entries: Vec<JournalEntry> = KEYS
            .into_iter()
            .enumerate()
            .filter(|&(i, _)| self.keys[i] != Held::Free)
            .map(|(i, key)| JournalEntry {
                plan: rig.copy.guid(),
                key,
                original: rig.user[i],
                written: TARGET,
                restored: matches!(self.keys[i], Held::E1 | Held::E2),
            })
            .collect();
        if self.deleted_plan {
            entries.push(Self::deleted_plan_entry());
        }
        entries
    }

    fn file(&self, rig: &Rig) -> Option<JournalFile> {
        let entries = self.entries(rig);
        let other = match self.unknown {
            true => vec![rig.unknown_entry()],
            false => Vec::new(),
        };
        if entries.is_empty() && other.is_empty() {
            return None;
        }
        let parts = JournalParts {
            known: Journal {
                owner: GONE_TRAY,
                entries,
            },
            other,
        };
        Some(parts.merge())
    }

    /// The safety net stays while any known entry exists, so a start with one has RunOnce set and a guard running.
    fn has_known(&self) -> bool {
        self.deleted_plan || self.keys.iter().any(|&h| h != Held::Free)
    }
}

fn apply_starts(unknown: bool) -> Vec<Start> {
    use Held::*;
    [
        Start::new("fresh apply", [Free, Free, Free], Routine::Apply(&KEYS)),
        // `p2-ownership-apply` Found outside scope 8: with E2 or E3 beside it, their `set_active` would hide a
        // dropped E1.
        Start::new(
            "apply over E1 alone",
            [E1, Free, Free],
            Routine::Apply(&[LidAc]),
        ),
        Start::new(
            "apply over E1, E2 and E3",
            [E1, E2, Ours],
            Routine::Apply(&KEYS),
        ),
        Start::new(
            "apply with lid AC released",
            [Ours, Ours, Ours],
            Routine::Apply(&[LidDc, SleepDc]),
        ),
    ]
    .into_iter()
    .map(|s| s.with_unknown(unknown))
    .collect()
}

fn restore_starts(unknown: bool) -> Vec<Start> {
    use Held::*;
    let all = Routine::Restore { one_plan: false };
    let one = Routine::Restore { one_plan: true };
    [
        Start::new("restore all plans after an apply", [Ours, Ours, Ours], all),
        Start::new("restore all plans, E1 alone", [E1, Free, Free], all),
        Start::new("restore all plans, E1, E2, E3", [E1, E2, Ours], all).with_deleted_plan(),
        Start::new("restore one plan, E1, E2, E3", [E1, E2, Ours], one).with_deleted_plan(),
    ]
    .into_iter()
    .map(|s| s.with_unknown(unknown))
    .collect()
}

fn check_starts(unknown: bool) -> Vec<Start> {
    use Held::*;
    [
        Start::new(
            "check after the user changed lid DC",
            [E1, User(3), Ours],
            Routine::Check,
        ),
        Start::new(
            "check after the user changed the last key",
            [Free, Free, User(1800)],
            Routine::Check,
        ),
    ]
    .into_iter()
    .map(|s| s.with_unknown(unknown))
    .collect()
}

/// A fresh folder under `%TEMP%`, removed with everything in it when the case ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("wakeshot-stop-points-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn data(&self) -> DataDir {
        DataDir::at(self.0.clone())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One case: the copy set to the start, its "in force" values, a fresh data folder with the start's journal, and
/// the stopped run's `FileNet` with RunOnce and the guard as the start has them.
struct World {
    power: CopyAsActive,
    net: FileNet<RecordingGuard>,
    dir: TempDir,
    log: Arc<MemoryLog>,
}

/// What the routine under test returned, when it was not stopped.
#[derive(Debug)]
enum Done {
    Applied(Applied),
    Restored(Restored),
    Checked(Checked),
}

/// How a next start's restore went.
struct Next {
    restored: Restored,
    calls: Vec<String>,
    guard: Vec<GuardCall>,
    found_file: bool,
}

/// What one start's walk counted, for the card.
struct Tally {
    calls: usize,
    next_calls: usize,
    cases: usize,
}

/// What every case of one test shares: the plan copy, the user's values on it, the test RunOnce key.
struct Rig {
    copy: TestPlan,
    /// The user's value per `KEYS` index: the copy's own, read live.
    user: [u32; 3],
    /// The copy's five values at the start; sleep AC and the energy saver are never written.
    values: PlanValues,
    names: Names,
    auto: AutoStart,
    command: String,
    _key: ScratchKey,
}

impl Rig {
    fn new(test: &str) -> Self {
        TestPlan::sweep().expect("sweep of copies earlier runs left");
        let copy = TestPlan::new().expect("plan copy");
        let plan = copy.guid();
        let log = Arc::new(MemoryLog::default());
        let mut power = CopyAsActive::over(&copy, Win32Power::new(log)).expect("copy as active");
        assert!(!power.plans().expect("plan list").contains(&DELETED_PLAN));
        let mut user = [TARGET; 3];
        for (i, key) in KEYS.into_iter().enumerate() {
            let own = power.read(plan, key).expect("read the copy");
            // A user value already at the target is skipped by the apply, and the walk would show nothing for that
            // key, so the copy gets a stand-in: lid Sleep, battery sleep 10 minutes.
            user[i] = match (own, key) {
                (TARGET, SleepDc) => 600,
                (TARGET, _) => 1,
                (own, _) => own,
            };
            if user[i] != own {
                power.write(plan, key, user[i]).expect("stand-in value");
            }
        }
        let values = power.values().expect("the copy's values");
        let key = ScratchKey::new(&format!("power::stop_points::{test}"));
        Self {
            copy,
            user,
            values,
            names: Names::test(test),
            auto: AutoStart::at(key.path()),
            command: format!("\"{TRAY_EXE}\" --restore"),
            _key: key,
        }
    }

    fn unknown_entry(&self) -> JournalFileEntry {
        JournalFileEntry {
            plan: self.copy.guid(),
            key: JournalKey::Other("sleepAc".into()),
            original: self.values.sleep_ac,
            written: TARGET,
            restored: false,
        }
    }

    fn run_once(&self) -> Option<String> {
        self.auto
            .get(StartKey::RunOnce, RUN_ONCE_NAME)
            .expect("RunOnce read")
    }

    fn tray_net(
        &self,
        dir: &TempDir,
        guard: RecordingGuard,
        log: &Arc<MemoryLog>,
    ) -> FileNet<RecordingGuard> {
        FileNet::tray(
            dir.data(),
            self.auto.clone(),
            Path::new(TRAY_EXE),
            guard,
            log.clone(),
        )
    }

    fn world(&self, start: &Start) -> World {
        let plan = self.copy.guid();
        let log = Arc::new(MemoryLog::default());
        // `CopyAsActive` takes the stored values at creation as in force, so those go first.
        let mut first = CopyAsActive::over(&self.copy, Win32Power::new(log.clone())).unwrap();
        for (i, key) in KEYS.into_iter().enumerate() {
            first.write(plan, key, start.in_force(self, i)).unwrap();
        }
        let mut power = CopyAsActive::over(&self.copy, Win32Power::new(log.clone())).unwrap();
        for (i, key) in KEYS.into_iter().enumerate() {
            let stored = start.stored(self, i);
            if stored != start.in_force(self, i) {
                power.write(plan, key, stored).unwrap();
            }
        }
        let dir = TempDir::new();
        if let Some(file) = start.file(self) {
            FileNet::dry_run(dir.data(), log.clone())
                .save_journal(&file)
                .unwrap();
        }
        match start.has_known() {
            true => self
                .auto
                .set(StartKey::RunOnce, RUN_ONCE_NAME, &self.command)
                .unwrap(),
            false => {
                self.auto.remove(StartKey::RunOnce, RUN_ONCE_NAME).unwrap();
            }
        }
        let guard = RecordingGuard {
            running: start.has_known(),
            ..RecordingGuard::default()
        };
        let net = self.tray_net(&dir, guard, &log);
        World {
            power,
            net,
            dir,
            log,
        }
    }

    /// The routine under test through the locked entry point, as the tray's IO thread calls it; `None`: stopped.
    fn run(&self, start: &Start, w: &mut World, script: &Rc<Script>) -> Option<Done> {
        let plan = self.copy.guid();
        let log = w.log.clone();
        let mut plans = StopAt::new(&mut w.power, script);
        let mut net = StopAt::new(&mut w.net, script);
        until_stop(|| match start.routine {
            Routine::Apply(keys) => {
                let writes: Vec<(SettingKey, u32)> = keys.iter().map(|&k| (k, TARGET)).collect();
                let names = &self.names;
                Done::Applied(apply_locked(
                    &mut plans, &mut net, THIS_TRAY, plan, &writes, names, &*log,
                ))
            }
            Routine::Restore { one_plan } => Done::Restored(restore_locked(
                &mut plans,
                &mut net,
                one_plan.then_some(plan),
                None,
                &self.names,
                &*log,
            )),
            Routine::Check => Done::Checked(check_locked(&mut plans, &mut net, &self.names, &*log)),
        })
    }

    /// The next start's restore: a fresh `FileNet` and guard over the same folder, RunOnce key and copy, with the
    /// owner on disk passed as restore mode and the guard pass the one they found gone. The copy's `CopyAsActive`
    /// carries over: its "in force" values are Windows state, not the stopped process's, and a fresh one would take
    /// the stored values as in force, which would hide a dropped E1.
    fn next_restore(&self, w: &mut World, at: Option<(usize, Act)>) -> Next {
        let file = on_disk(w);
        let owner = file.as_ref().map(|f| f.owner);
        let mut net = self.tray_net(&w.dir, RecordingGuard::default(), &w.log);
        let script = Script::new(at);
        let restored = {
            let mut plans = StopAt::new(&mut w.power, &script);
            let mut safety = StopAt::new(&mut net, &script);
            restore_locked(&mut plans, &mut safety, None, owner, &self.names, &*w.log)
        };
        Next {
            restored,
            calls: script.calls(),
            guard: net.guard().calls.clone(),
            found_file: file.is_some(),
        }
    }

    /// The clean run of one start: counts its calls, then each stop with the next start's restore, then each stop
    /// with a next restore that fails at each of its calls in turn and a retry.
    fn walk(&self, start: &Start) -> Tally {
        let mut w = self.world(start);
        let script = Script::new(None);
        let done = self.run(start, &mut w, &script).expect("no stop scripted");
        let clean = script.calls();
        self.assert_clean(start, &mut w, &done, &format!("{}, clean run", start.name));
        let mut tally = Tally {
            calls: clean.len(),
            next_calls: 0,
            cases: 1,
        };
        for stop in (0..clean.len()).map(Some).chain([None]) {
            let at = stop.map(|k| (k, Act::Stop));
            let case = match stop {
                Some(k) => format!(
                    "{}, stop at call {k} ({}) of {clean:?}",
                    start.name, clean[k]
                ),
                None => format!("{}, no stop", start.name),
            };
            let mut w = self.world(start);
            let done = self.run(start, &mut w, &Script::new(at));
            assert_eq!(
                done.is_none(),
                stop.is_some(),
                "{case}: the stop was not reached"
            );
            self.assert_reachable(start, &mut w, &[], &case);
            let next = self.next_restore(&mut w, None);
            self.assert_back(start, &mut w, &next, &case);
            self.assert_released(start, &w, &next, stop, &clean, &case);
            tally.next_calls = tally.next_calls.max(next.calls.len());
            tally.cases += 1;

            for fail in 0..next.calls.len() {
                let case = format!(
                    "{case}; next restore fails at {fail} ({})",
                    next.calls[fail]
                );
                let mut w = self.world(start);
                let _ = self.run(start, &mut w, &Script::new(at));
                let failed = self.next_restore(&mut w, Some((fail, Act::Fail)));
                assert!(
                    failed.restored.result.is_err(),
                    "{case}: {:?}",
                    failed.restored
                );
                self.assert_reachable(start, &mut w, &failed.guard, &case);
                // The tray retries on its next safety tick, restore mode and the guard every 10 s.
                let retry = self.next_restore(&mut w, None);
                self.assert_back(start, &mut w, &retry, &format!("{case}; retry"));
                self.assert_retry_released(&retry, &format!("{case}; retry"));
                tally.cases += 1;
            }
        }
        tally
    }

    /// The clean run did what its start says, so the walk's starts are the states they claim to be.
    fn assert_clean(&self, start: &Start, w: &mut World, done: &Done, case: &str) {
        let plan = self.copy.guid();
        match (start.routine, done) {
            (Routine::Apply(keys), Done::Applied(a)) => {
                assert_eq!(a.result, Ok(()), "{case}");
                for (i, key) in KEYS.into_iter().enumerate() {
                    let want = match keys.contains(&key) {
                        true => TARGET,
                        false => start.expected(self, i),
                    };
                    assert_eq!(w.power.read(plan, key), Ok(want), "{case}: {key:?} stored");
                    assert_eq!(w.power.in_force(key), want, "{case}: {key:?} in force");
                }
                let owned: Vec<SettingKey> = a
                    .journal
                    .iter()
                    .filter(|e| e.plan == plan)
                    .map(|e| e.key)
                    .collect();
                assert_eq!(owned, keys, "{case}: journal {:?}", a.journal);
                assert!(
                    a.journal.iter().all(|e| !e.restored),
                    "{case}: {:?}",
                    a.journal
                );
            }
            (Routine::Restore { one_plan }, Done::Restored(r)) => {
                assert_eq!(r.result, Ok(()), "{case}");
                for (i, key) in KEYS.into_iter().enumerate() {
                    let want = start.expected(self, i);
                    assert_eq!(w.power.read(plan, key), Ok(want), "{case}: {key:?} stored");
                    assert_eq!(w.power.in_force(key), want, "{case}: {key:?} in force");
                }
                let left = match one_plan && start.deleted_plan {
                    true => vec![Start::deleted_plan_entry()],
                    false => Vec::new(),
                };
                assert_eq!(r.journal, left, "{case}");
            }
            (Routine::Check, Done::Checked(c)) => {
                assert_eq!(c.failures, [], "{case}");
                let changed: Vec<OwnedKey> = KEYS
                    .into_iter()
                    .enumerate()
                    .filter(|&(i, _)| matches!(start.keys[i], Held::User(_)))
                    .map(|(_, key)| OwnedKey { plan, key })
                    .collect();
                assert_eq!(c.user_changed, changed, "{case}");
            }
            (routine, done) => panic!("{case}: {routine:?} returned {done:?}"),
        }
    }

    /// Track note 4, first point: every value Wakeshot still holds, stored or in force, has a known entry with the
    /// user's original; while a known entry exists, RunOnce is set, the guard runs, and the next start's restore
    /// did not tell it to stop. An unknown key's entry is never touched.
    fn assert_reachable(&self, start: &Start, w: &mut World, next_guard: &[GuardCall], case: &str) {
        let plan = self.copy.guid();
        let parts = on_disk(w).map(JournalParts::split);
        let known = parts
            .as_ref()
            .map_or(Vec::new(), |p| p.known.entries.clone());
        for (i, key) in KEYS.into_iter().enumerate() {
            let stored = w.power.read(plan, key).expect("read the copy");
            let in_force = w.power.in_force(key);
            if stored != TARGET && in_force != TARGET {
                continue;
            }
            let entry = known.iter().find(|e| e.plan == plan && e.key == key);
            assert!(
                entry.is_some_and(|e| e.original == self.user[i] && e.written == TARGET),
                "{case}: {key:?} held (stored {stored}, in force {in_force}) without its entry: {known:?}"
            );
        }
        if !known.is_empty() {
            let run_once = self.run_once();
            assert_eq!(
                run_once.as_ref(),
                Some(&self.command),
                "{case}: a known entry without RunOnce"
            );
            assert!(
                w.net.guard().running,
                "{case}: a known entry, and the guard {:?}",
                w.net.guard()
            );
            assert!(
                !next_guard.contains(&GuardCall::Stop),
                "{case}: guard told to stop with {known:?} left"
            );
        }
        if start.unknown {
            let other = parts.map(|p| p.other);
            assert_eq!(
                other,
                Some(vec![self.unknown_entry()]),
                "{case}: the unknown key's entry changed"
            );
        }
    }

    /// Track note 4, second point: after the next restore succeeded, every key, stored and in force, is the user's
    /// value (or the user's later change) and no known entry is left; the keys Wakeshot never writes are as they
    /// were; an unknown key's entry stays in the file as it was.
    fn assert_back(&self, start: &Start, w: &mut World, next: &Next, case: &str) {
        let r = &next.restored;
        assert_eq!(
            r.result,
            Ok(()),
            "{case}: next restore {r:?}, calls {:?}",
            next.calls
        );
        assert!(!matches!(r.found, Found::OtherOwner(_)), "{case}: {r:?}");
        assert_eq!(r.journal, [], "{case}");
        let values = w.power.values().expect("the copy's values");
        for (i, key) in KEYS.into_iter().enumerate() {
            let want = start.expected(self, i);
            assert_eq!(value_of(&values, key), want, "{case}: {key:?} stored");
            assert_eq!(w.power.in_force(key), want, "{case}: {key:?} in force");
        }
        assert_eq!(values.sleep_ac, self.values.sleep_ac, "{case}: sleep AC");
        assert_eq!(
            values.saver_threshold_dc, self.values.saver_threshold_dc,
            "{case}: energy saver"
        );
        let file = on_disk(w);
        match start.unknown {
            true => {
                let file =
                    file.unwrap_or_else(|| panic!("{case}: the unknown key's journal is gone"));
                let parts = JournalParts::split(file);
                assert_eq!(parts.known.entries, [], "{case}");
                assert_eq!(parts.other, [self.unknown_entry()], "{case}");
            }
            false => assert_eq!(file, None, "{case}: journal left"),
        }
    }

    /// Track note 4's release: a next restore that finds a journal clears RunOnce and tells the guard to stop. It
    /// finds none only after a stop between the clean-up's journal delete and its last call (walk R6,
    /// `p2-ownership-restore` Found outside scope 2), after a clean run that cleaned up itself, or after a fresh
    /// apply stopped before its first journal save; it then does nothing, and what the stopped run had added or not
    /// yet cleared stays with no journal, which is harmless: RunOnce and the guard find nothing to do.
    fn assert_released(
        &self,
        start: &Start,
        w: &World,
        next: &Next,
        stop: Option<usize>,
        clean: &[String],
        case: &str,
    ) {
        let index = |name: &str| clean.iter().position(|c| c.split(' ').next() == Some(name));
        let run_once = self.run_once();
        let guard = w.net.guard();
        if next.found_file {
            assert_eq!(run_once, None, "{case}: RunOnce left");
            assert!(
                next.guard.contains(&GuardCall::Stop),
                "{case}: {:?}",
                next.guard
            );
            return;
        }
        assert_eq!(next.calls, ["load_journal"], "{case}");
        match (stop, index("delete_journal")) {
            (None, Some(_)) => {
                assert_eq!(run_once, None, "{case}");
                assert_eq!(guard.calls.last(), Some(&GuardCall::Stop), "{case}");
            }
            (Some(k), Some(d)) if k > d => {
                // The clean-up's order is delete, RunOnce, guard: only a stop at the RunOnce clear leaves the value.
                assert_eq!(
                    run_once.is_some(),
                    k == d + 1,
                    "{case}: RunOnce {run_once:?}"
                );
                assert!(guard.running, "{case}: {guard:?}");
            }
            (Some(k), _) => {
                let first_save = index("save_journal").expect("a run that owns something saves");
                assert!(
                    start.file(self).is_none() && k <= first_save,
                    "{case}: journal gone"
                );
                let past = |name: &str| index(name).is_some_and(|i| k > i);
                assert_eq!(
                    run_once.is_some(),
                    past("set_run_once"),
                    "{case}: RunOnce {run_once:?}"
                );
                assert_eq!(guard.running, past("ensure_guard"), "{case}: {guard:?}");
            }
            (None, None) => panic!("{case}: no journal after a run that did not clean up"),
        }
    }

    /// After a failed next restore and its retry: the retry released the safety net whenever it found a journal.
    /// With none left (the stop or the failed run got past the delete: walk R6) it made no other call.
    fn assert_retry_released(&self, retry: &Next, case: &str) {
        match retry.found_file {
            true => {
                assert_eq!(self.run_once(), None, "{case}: RunOnce left");
                assert!(
                    retry.guard.contains(&GuardCall::Stop),
                    "{case}: {:?}",
                    retry.guard
                );
            }
            false => assert_eq!(retry.calls, ["load_journal"], "{case}"),
        }
    }
}

/// The journal as a reader finds it: `None` when missing.
fn on_disk(w: &World) -> Option<JournalFile> {
    match FileNet::dry_run(w.dir.data(), w.log.clone()).load_journal() {
        JournalRead::Missing => None,
        JournalRead::Read(file) => Some(file),
        other => panic!("journal.json does not load: {other:?}"),
    }
}

fn value_of(values: &PlanValues, key: SettingKey) -> u32 {
    match key {
        LidAc => values.lid_ac,
        LidDc => values.lid_dc,
        SleepDc => values.sleep_dc,
    }
}

/// The user's active plan and its five values; read only.
fn users_values() -> PlanValues {
    Win32Power::new(Arc::new(MemoryLog::default()))
        .values()
        .expect("the active plan's values")
}

/// Runs each start's walk on one plan copy, then checks that the user's plan and values are as before and the copy
/// is gone. Prints N and the time of each start, for the card.
fn real_walk(test: &str, starts: &[Start]) {
    let before = users_values();
    let began = Instant::now();
    let copy = {
        let rig = Rig::new(test);
        eprintln!("{test}: user values on the copy {:?}", rig.user);
        for start in starts {
            let t = Instant::now();
            let tally = rig.walk(start);
            eprintln!(
                "{test}: {}{}: N = {}, next restore up to {} calls, {} cases, {:.1} s",
                start.name,
                if start.unknown { " (unknown key)" } else { "" },
                tally.calls,
                tally.next_calls,
                tally.cases,
                t.elapsed().as_secs_f64()
            );
        }
        rig.copy.guid()
    };
    eprintln!("{test}: {:.1} s", began.elapsed().as_secs_f64());
    assert_eq!(
        users_values(),
        before,
        "the user's active plan or a value changed"
    );
    let mut power = Win32Power::new(Arc::new(MemoryLog::default()));
    assert!(
        !power.plans().unwrap().contains(&copy),
        "the plan copy is left"
    );
}

#[test]
#[ignore = "real laptop: walks a plan copy that is never active"]
fn real_stop_points_apply() {
    real_walk("real_stop_points_apply", &apply_starts(false));
}

#[test]
#[ignore = "real laptop: walks a plan copy that is never active"]
fn real_stop_points_restore() {
    real_walk("real_stop_points_restore", &restore_starts(false));
}

#[test]
#[ignore = "real laptop: walks a plan copy that is never active"]
fn real_stop_points_check() {
    real_walk("real_stop_points_check", &check_starts(false));
}

/// Correction 12: every start of the three walks again, each with an unknown key's entry, which no stop and no
/// failure touches and which never blocks the clean-up.
#[test]
#[ignore = "real laptop: walks a plan copy that is never active"]
fn real_stop_points_unknown_key() {
    let starts: Vec<Start> =
        [apply_starts(true), restore_starts(true), check_starts(true)].concat();
    real_walk("real_stop_points_unknown_key", &starts);
}

mod tests {
    use wakeshot_core::ownership::fake::{Call, FakeJournal, FakePlans, SetBy};
    use wakeshot_core::ownership::restore::restore;

    use super::*;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

    fn fake() -> FakePlans {
        FakePlans::new(PlanValues {
            active: BALANCED,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 14_400,
            saver_threshold_dc: 30,
        })
    }

    #[test]
    fn both_wrappers_share_one_counter_and_stop_before_the_call() {
        let fake = fake();
        let (mut a, mut b) = (fake.clone(), fake.clone());
        let script = Script::new(Some((2, Act::Stop)));
        let mut plans = StopAt::new(&mut a, &script);
        let mut net = StopAt::new(&mut b, &script);
        let ran = until_stop(|| {
            plans.active_plan().unwrap();
            net.load_journal();
            plans.read(BALANCED, LidDc).unwrap();
        });
        assert_eq!(ran, None);
        let read = format!("read {BALANCED} LidDc");
        assert_eq!(
            script.calls(),
            ["active_plan", "load_journal", read.as_str()]
        );
        assert_eq!(fake.state().calls, [Call::ActivePlan, Call::LoadJournal]);
    }

    #[test]
    fn a_scripted_failure_skips_only_that_call() {
        let fake = fake();
        let (mut a, mut b) = (fake.clone(), fake.clone());
        {
            let script = Script::new(Some((0, Act::Fail)));
            let mut net = StopAt::new(&mut b, &script);
            assert_eq!(net.load_journal(), JournalRead::Failed(SCRIPTED));
            let mut plans = StopAt::new(&mut a, &script);
            assert_eq!(plans.write(BALANCED, LidAc, TARGET), Ok(()));
        }
        assert_eq!(fake.state().calls, [Call::Write(BALANCED, LidAc, TARGET)]);

        let script = Script::new(Some((0, Act::Fail)));
        let mut plans = StopAt::new(&mut a, &script);
        assert_eq!(plans.set_active(BALANCED), Err(SCRIPTED));
        assert_eq!(fake.state().calls.len(), 1);
    }

    #[test]
    fn only_the_stop_marker_is_caught() {
        let passed = panic::catch_unwind(|| until_stop(|| panic!("not a scripted stop")));
        assert!(passed.is_err());
        assert_eq!(until_stop(|| 7), Some(7));
    }

    #[test]
    fn the_count_matches_the_routines_own_calls() {
        let fake = fake();
        {
            let mut s = fake.state();
            s.journal = FakeJournal::File(JournalFile {
                version: JournalFile::VERSION,
                owner: GONE_TRAY,
                entries: vec![JournalFileEntry {
                    plan: BALANCED,
                    key: JournalKey::Known(LidDc),
                    original: 1,
                    written: TARGET,
                    restored: false,
                }],
            });
            s.set_stored(BALANCED, LidDc, TARGET, SetBy::Override);
            s.set_in_force(BALANCED, LidDc, TARGET, SetBy::Override);
        }
        let (mut a, mut b) = (fake.clone(), fake.clone());
        let script = Script::new(None);
        let done = {
            let mut plans = StopAt::new(&mut a, &script);
            let mut net = StopAt::new(&mut b, &script);
            restore(&mut plans, &mut net, None, None)
        };
        assert_eq!(done.result, Ok(()));
        let calls = fake.state().calls.clone();
        assert_eq!(script.calls().len(), calls.len());
        let deleted = script.calls().iter().position(|c| c == "delete_journal");
        assert_eq!(
            deleted,
            calls.iter().position(|c| *c == Call::DeleteJournal)
        );
        assert!(deleted.is_some());
    }
}
