use std::cell::RefCell;
use std::collections::VecDeque;
use std::os::windows::fs::OpenOptionsExt;
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
use wakeshot_core::ownership::fake::{
    Call as PlanCall, FAKE_DEFAULT_LID, FAKE_DEFAULT_OTHER, FAKE_DEFAULT_SLEEP_DC, FakePlans, SetBy,
};
use wakeshot_core::ownership::journal::JournalParts;
use wakeshot_core::ownership::reset::KeyReset;
use wakeshot_core::process::FileTime;
use wakeshot_core::settings::{Guid, Journal, JournalEntry, PlanValues, SettingKey};
use windows::Win32::System::Registry::REG_EXPAND_SZ;

use super::*;
use crate::install::PATH_VALUE;
use crate::log::{LogWriter, MemoryLog};
use crate::process::FakeLiveness;
use crate::store::ScratchKey;

use SettingKey::{LidDc, SleepDc};

const FAILED: Failure = Failure {
    call: "FakePlans::active_plan",
    code: 31,
};
const OWNER: JournalOwner = JournalOwner {
    pid: 9120,
    proc_start: FileTime(5),
    session: 1,
};
const DENIED: WinError = WinError {
    call: "RegSetValueExW",
    code: 5,
};
/// The `StartupApproved` value forms the real Settings switch wrote on build 26300: state 1 and the FILETIME of the
/// switch-off, or 12 zero bytes for on (research "Start at login switch check", "The value's format").
const SWITCH_OFF: [u8; 12] = [1, 0, 0, 0, 0x9D, 0xAA, 0x39, 0x37, 0xA4, 0x6C, 0xDD, 0x01];
const SWITCH_ON: [u8; 12] = [0; 12];

/// One call to the fake, in order. `TryRestore` holds the fake clock when it started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    SetRun,
    RemoveRun,
    RemoveApproval,
    AddPath,
    RemovePath,
    RegisterNotices,
    RemoveNotices,
    RemoveAgentHooks,
    RemoveRunOnce,
    TryRestore(u64),
    ResetLost,
    Clean(DataFolder),
    Wait(u64),
}

/// A `HookEnv` with a fake clock that moves only when a step takes time or the hook waits.
struct FakeEnv {
    now: u64,
    calls: Vec<Call>,
    lines: RefCell<Vec<(Level, String)>>,
    /// The answers of `try_restore` in order, each with the time it takes; `then` once they are used up.
    attempts: VecDeque<(u64, Attempt)>,
    then: Attempt,
    /// The time each registry step takes (a Path broadcast waits up to 1 s per window).
    step_ms: u64,
    /// Every registry step fails with this.
    registry_error: Option<WinError>,
    broadcast_error: Option<WinError>,
    undeleted: Vec<Undeleted>,
    /// Whether the user ever used the Settings switch, so there is a `StartupApproved` value to remove.
    switch_used: bool,
    /// Whether the notice icon ships beside the exe.
    notice_icon: bool,
}

impl FakeEnv {
    fn new(attempts: &[Attempt]) -> Self {
        Self::timed(&attempts.iter().map(|&a| (0, a)).collect::<Vec<_>>())
    }

    fn timed(attempts: &[(u64, Attempt)]) -> Self {
        Self {
            // Not zero, so a limit counted from zero instead of the hook's start would show.
            now: 1_000_000,
            calls: Vec::new(),
            lines: RefCell::default(),
            attempts: attempts.iter().copied().collect(),
            then: Attempt::Failed(FAILED),
            step_ms: 0,
            registry_error: None,
            broadcast_error: None,
            undeleted: Vec::new(),
            switch_used: true,
            notice_icon: true,
        }
    }

    fn step<T>(&mut self, call: Call, ok: T) -> Result<T, WinError> {
        self.calls.push(call);
        self.now += self.step_ms;
        match self.registry_error {
            Some(e) => Err(e),
            None => Ok(ok),
        }
    }

    fn path_change(&self) -> PathChange {
        PathChange::Changed {
            broadcast: self.broadcast_error.map_or(Ok(()), Err),
        }
    }

    fn tries(&self) -> Vec<u64> {
        self.calls
            .iter()
            .filter_map(|c| match c {
                Call::TryRestore(at) => Some(at - 1_000_000),
                _ => None,
            })
            .collect()
    }

    fn lines(&self) -> Vec<(Level, String)> {
        self.lines.borrow().clone()
    }

    fn count(&self, part: &str) -> usize {
        self.lines
            .borrow()
            .iter()
            .filter(|(_, text)| text.contains(part))
            .count()
    }

    fn at_level(&self, level: Level) -> usize {
        self.lines
            .borrow()
            .iter()
            .filter(|(l, _)| *l == level)
            .count()
    }
}

impl HookEnv for FakeEnv {
    fn set_run(&mut self) -> Result<String, WinError> {
        self.step(
            Call::SetRun,
            r#""C:\x\current\wakeshot-tray.exe""#.to_owned(),
        )
    }

    fn remove_run(&mut self) -> Result<bool, WinError> {
        self.step(Call::RemoveRun, true)
    }

    fn remove_approval(&mut self) -> Result<bool, WinError> {
        let there = self.switch_used;
        self.step(Call::RemoveApproval, there)
    }

    fn add_path(&mut self) -> Result<PathChange, WinError> {
        let change = self.path_change();
        self.step(Call::AddPath, change)
    }

    fn remove_path(&mut self) -> Result<PathChange, WinError> {
        let change = self.path_change();
        self.step(Call::RemovePath, change)
    }

    fn register_notices(&mut self) -> Result<Registered, WinError> {
        let icon = PathBuf::from(r"C:\x\current\wakeshot-notice.png");
        let wrote = match self.notice_icon {
            true => Registered::WithIcon(icon),
            false => Registered::NoIcon(icon),
        };
        self.step(Call::RegisterNotices, wrote)
    }

    fn remove_notices(&mut self) -> Result<bool, WinError> {
        self.step(Call::RemoveNotices, true)
    }

    fn remove_agent_hooks(&mut self) {
        self.calls.push(Call::RemoveAgentHooks);
        self.now += self.step_ms;
    }

    fn remove_run_once(&mut self) -> Result<bool, WinError> {
        self.step(Call::RemoveRunOnce, true)
    }

    fn try_restore(&mut self) -> Attempt {
        self.calls.push(Call::TryRestore(self.now));
        let (takes, attempt) = self.attempts.pop_front().unwrap_or((0, self.then));
        self.now += takes;
        attempt
    }

    /// The battery keys reset, as for a crashed tray's hold.
    fn reset_lost(&mut self) -> Reset {
        self.calls.push(Call::ResetLost);
        Reset {
            found: ResetFound::Lost,
            plan: Some(P1),
            keys: vec![
                (LidDc, KeyReset::Reset { old: 0, default: 1 }),
                (
                    SleepDc,
                    KeyReset::Reset {
                        old: 0,
                        default: 180,
                    },
                ),
            ],
            set_aside: Some(Ok(())),
            result: Ok(()),
            journal: Vec::new(),
        }
    }

    fn clean_data_folder(&mut self, step: DataFolder) -> Vec<Undeleted> {
        self.calls.push(Call::Clean(step));
        self.undeleted.clone()
    }

    fn now_ms(&self) -> u64 {
        self.now
    }

    fn wait_ms(&mut self, ms: u64) {
        self.calls.push(Call::Wait(ms));
        self.now += ms;
    }

    fn log(&self, level: Level, text: &str) {
        self.lines.borrow_mut().push((level, text.to_owned()));
    }
}

const FAIL: Attempt = Attempt::Failed(FAILED);
const WAIT: Call = Call::Wait(RETRY_EVERY_MS);

/// The calls but the restore tries' times, for order checks.
fn order(env: &FakeEnv) -> Vec<Call> {
    env.calls
        .iter()
        .map(|c| match c {
            Call::TryRestore(_) => Call::TryRestore(0),
            other => *other,
        })
        .collect()
}

const TRY: Call = Call::TryRestore(0);

#[test]
fn each_hook_calls_in_order() {
    let mut env = FakeEnv::new(&[Attempt::Restored]);
    on_install(&mut env, "0.1.0");
    assert_eq!(
        order(&env),
        [Call::SetRun, Call::AddPath, Call::RegisterNotices, TRY]
    );
    let lines = env.lines();
    assert_eq!(
        lines[0],
        (
            Level::Info,
            "install hook for version 0.1.0 started".to_owned()
        )
    );
    assert!(
        lines
            .last()
            .unwrap()
            .1
            .starts_with("install hook done after")
    );
    assert_eq!(env.at_level(Level::Info), lines.len(), "{lines:?}");

    let mut env = FakeEnv::new(&[Attempt::Restored]);
    on_updated(&mut env, "0.1.1");
    assert_eq!(order(&env), [Call::AddPath, Call::RegisterNotices, TRY]);
    assert_eq!(env.count("updated hook for version 0.1.1 started"), 1);

    let mut env = FakeEnv::new(&[Attempt::Restored]);
    on_uninstall(&mut env, "0.1.1");
    let want = [
        Call::RemoveRun,
        Call::RemoveApproval,
        Call::RemoveRunOnce,
        Call::RemovePath,
        Call::RemoveNotices,
        Call::RemoveAgentHooks,
        TRY,
        Call::Clean(DataFolder::DeleteAll),
    ];
    assert_eq!(order(&env), want);
    assert_eq!(env.count("data folder deleted"), 1);
}

#[test]
fn notice_app_id_lines() {
    let mut env = FakeEnv::new(&[Attempt::Nothing, Attempt::Nothing]);
    on_install(&mut env, "0.1.0");
    on_uninstall(&mut env, "0.1.0");
    assert_eq!(
        env.count(
            r"notice app id velopack.HaikAsatryan.Wakeshot registered with its icon C:\x\current\wakeshot-notice.png"
        ),
        1
    );
    assert_eq!(
        env.count("notice app id velopack.HaikAsatryan.Wakeshot removed"),
        1
    );
    assert_eq!(env.at_level(Level::Info), env.lines().len());

    // The icon file is missing: the name is still written, and the line says what the header shows.
    let mut env = FakeEnv::new(&[Attempt::Nothing]);
    env.notice_icon = false;
    on_updated(&mut env, "0.1.1");
    assert_eq!(env.at_level(Level::Warn), 1, "{:?}", env.lines());
    assert_eq!(
        env.count(
            r"registered without its icon: C:\x\current\wakeshot-notice.png not found; notices show a plain square"
        ),
        1
    );
}

#[test]
fn only_uninstall_removes_the_switch_value() {
    let answers = [
        Attempt::Restored,
        Attempt::Nothing,
        FAIL,
        Attempt::OwnerAlive(OWNER),
        Attempt::OwnerUnknown(OWNER),
        Attempt::Unrestorable(Unrestorable::DoesNotParse),
    ];
    for answer in answers {
        // Every hook Setup, an update and `winget upgrade` run, with the switch value there.
        let mut env = FakeEnv::new(&[answer, answer]);
        on_install(&mut env, "0.1.0");
        on_obsolete(&mut env, "0.1.0");
        on_updated(&mut env, "0.1.1");
        assert!(!env.calls.contains(&Call::RemoveApproval), "{answer:?}");
        assert_eq!(env.count("StartupApproved"), 0, "{answer:?}");

        on_uninstall(&mut env, "0.1.1");
        let removes = env.calls.iter().filter(|&&c| c == Call::RemoveApproval);
        assert_eq!(removes.count(), 1, "{answer:?}");
        assert_eq!(
            env.count("StartupApproved value Wakeshot removed"),
            1,
            "{answer:?}"
        );
    }

    // A user who never used the switch.
    let mut env = FakeEnv::new(&[Attempt::Restored]);
    env.switch_used = false;
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(order(&env)[..2], [Call::RemoveRun, Call::RemoveApproval]);
    assert_eq!(env.count("StartupApproved value Wakeshot was not there"), 1);
    assert_eq!(env.at_level(Level::Info), env.lines().len());
}

#[test]
fn obsolete_calls_nothing_but_the_log() {
    let mut env = FakeEnv::new(&[]);
    on_obsolete(&mut env, "0.1.0");
    assert_eq!(env.calls, []);
    assert_eq!(env.lines().len(), 1);
    assert_eq!(env.count("obsolete hook for version 0.1.0"), 1);
}

#[test]
fn restore_fails_twice_then_works() {
    let attempts = [FAIL, FAIL, Attempt::Restored];
    let tries = [TRY, WAIT, TRY, WAIT, TRY];

    let mut env = FakeEnv::new(&attempts);
    on_install(&mut env, "0.1.0");
    assert_eq!(order(&env)[3..], tries);
    assert_eq!(env.tries(), [0, 2_000, 4_000]);
    assert_eq!(env.count("next try in 2 s"), 2);
    assert_eq!(env.count("the journal's values are back"), 1);

    let mut env = FakeEnv::new(&attempts);
    on_updated(&mut env, "0.1.1");
    assert_eq!(order(&env)[2..], tries);

    let mut env = FakeEnv::new(&attempts);
    on_uninstall(&mut env, "0.1.1");
    let mut want = vec![
        Call::RemoveRun,
        Call::RemoveApproval,
        Call::RemoveRunOnce,
        Call::RemovePath,
        Call::RemoveNotices,
        Call::RemoveAgentHooks,
    ];
    want.extend(tries);
    want.push(Call::Clean(DataFolder::DeleteAll));
    assert_eq!(order(&env), want);
}

#[test]
fn restore_that_fails_until_the_limit() {
    // Install: a try at 0, 2, ... 24 s; at 24 s another 2 s would pass 25 s.
    let mut env = FakeEnv::new(&[]);
    on_install(&mut env, "0.1.0");
    let every_2s = |n: u64| (0..n).map(|i| i * RETRY_EVERY_MS).collect::<Vec<_>>();
    assert_eq!(env.tries(), every_2s(13));
    assert_eq!(
        order(&env)[..3],
        [Call::SetRun, Call::AddPath, Call::RegisterNotices]
    );
    assert_eq!(env.count("no time for another within 25 s"), 1);
    assert_eq!(
        env.count("the tray restores the values at its next start"),
        1
    );

    // Updated: 10 s, so a try at 0 ... 10 s.
    let mut env = FakeEnv::new(&[]);
    on_updated(&mut env, "0.1.1");
    assert_eq!(env.tries(), every_2s(6));
    assert_eq!(env.count("no time for another within 10 s"), 1);

    // Uninstall: Run, its switch value, RunOnce, Path, the notice app id and the agent hooks go first anyway; the
    // journal and the logs stay.
    let mut env = FakeEnv::new(&[]);
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(env.tries(), every_2s(13));
    let calls = order(&env);
    assert_eq!(
        calls[..6],
        [
            Call::RemoveRun,
            Call::RemoveApproval,
            Call::RemoveRunOnce,
            Call::RemovePath,
            Call::RemoveNotices,
            Call::RemoveAgentHooks
        ]
    );
    assert_eq!(
        calls.last(),
        Some(&Call::Clean(DataFolder::KeepJournalAndLogs))
    );
    assert_eq!(env.count("a reinstall restores the values"), 1);
    assert_eq!(env.count("emptied but for journal.json and the logs"), 1);
    assert_eq!(env.at_level(Level::Error), 1, "{:?}", env.lines());
}

#[test]
fn restore_that_hangs_comes_after_the_registry_steps() {
    let hang = [(40_000, FAIL)];

    let mut env = FakeEnv::timed(&hang);
    on_install(&mut env, "0.1.0");
    assert_eq!(
        order(&env),
        [Call::SetRun, Call::AddPath, Call::RegisterNotices, TRY]
    );

    let mut env = FakeEnv::timed(&hang);
    on_updated(&mut env, "0.1.1");
    assert_eq!(order(&env), [Call::AddPath, Call::RegisterNotices, TRY]);

    let mut env = FakeEnv::timed(&hang);
    on_uninstall(&mut env, "0.1.1");
    let want = [
        Call::RemoveRun,
        Call::RemoveApproval,
        Call::RemoveRunOnce,
        Call::RemovePath,
        Call::RemoveNotices,
        Call::RemoveAgentHooks,
        TRY,
        Call::Clean(DataFolder::KeepJournalAndLogs),
    ];
    assert_eq!(order(&env), want);
}

#[test]
fn owner_alive_or_unknown_no_restore_one_line() {
    for answer in [Attempt::OwnerAlive(OWNER), Attempt::OwnerUnknown(OWNER)] {
        let mut env = FakeEnv::new(&[answer]);
        on_install(&mut env, "0.1.0");
        assert_eq!(
            order(&env),
            [Call::SetRun, Call::AddPath, Call::RegisterNotices, TRY],
            "{answer:?}"
        );
        assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");

        let mut env = FakeEnv::new(&[answer]);
        on_updated(&mut env, "0.1.1");
        assert_eq!(
            order(&env),
            [Call::AddPath, Call::RegisterNotices, TRY],
            "{answer:?}"
        );
        assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");

        let mut env = FakeEnv::new(&[answer]);
        on_uninstall(&mut env, "0.1.1");
        assert_eq!(
            order(&env)[6..],
            [TRY, Call::Clean(DataFolder::KeepJournalAndLogs)],
            "{answer:?}"
        );
        assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");
    }
}

#[test]
fn unrestorable_journal_is_tried_once_and_kept() {
    let mut env = FakeEnv::new(&[Attempt::Unrestorable(Unrestorable::NewerVersion)]);
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(
        order(&env)[6..],
        [TRY, Call::Clean(DataFolder::KeepJournalAndLogs)]
    );
    assert_eq!(env.count("no restore"), 1);

    // One that does not parse gets the lost-journal reset: no tray starts after an uninstall to do it. The logs
    // stay, since they hold the old values.
    let mut env = FakeEnv::new(&[Attempt::Unrestorable(Unrestorable::DoesNotParse)]);
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(
        order(&env)[6..],
        [
            TRY,
            Call::ResetLost,
            Call::Clean(DataFolder::KeepJournalAndLogs)
        ]
    );
    assert_eq!(env.count("no restore"), 1);
    assert_eq!(
        env.count("lost journal reset: [LidDc, SleepDc] set to Windows defaults"),
        1,
        "{:?}",
        env.lines()
    );

    // Install and update leave it to the tray's next start, which can warn.
    let mut env = FakeEnv::new(&[Attempt::Unrestorable(Unrestorable::DoesNotParse); 2]);
    on_install(&mut env, "0.1.0");
    on_updated(&mut env, "0.1.1");
    assert!(!env.calls.contains(&Call::ResetLost));
}

#[test]
fn data_folder_step_after_success_and_after_failure() {
    let cases = [
        (Attempt::Restored, DataFolder::DeleteAll),
        (Attempt::Nothing, DataFolder::DeleteAll),
        (FAIL, DataFolder::KeepJournalAndLogs),
        (Attempt::OwnerAlive(OWNER), DataFolder::KeepJournalAndLogs),
        (Attempt::OwnerUnknown(OWNER), DataFolder::KeepJournalAndLogs),
    ];
    for (end, step) in cases {
        // A failure every try, so the last answer is `end`.
        let mut env = FakeEnv::new(&[]);
        env.then = end;
        on_uninstall(&mut env, "0.1.1");
        assert_eq!(env.calls.last(), Some(&Call::Clean(step)), "{end:?}");
    }

    // A folder the step could not delete: a line for each thing left.
    let mut env = FakeEnv::new(&[Attempt::Restored]);
    env.undeleted = vec![
        Undeleted {
            path: PathBuf::from(r"C:\data\holds\x.json"),
            error: WinError {
                call: "DeleteFileW",
                code: 32,
            },
        },
        Undeleted {
            path: PathBuf::from(r"C:\data\state.json"),
            error: WinError {
                call: "DeleteFileW",
                code: 5,
            },
        },
    ];
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(env.at_level(Level::Error), 2, "{:?}", env.lines());
    assert_eq!(
        env.count(r"could not delete C:\data\holds\x.json: DeleteFileW failed with 32"),
        1
    );
    assert_eq!(env.count("data folder deleted"), 0);
}

#[test]
fn registry_failures_are_lines_and_the_hook_goes_on() {
    let mut env = FakeEnv::new(&[Attempt::Restored]);
    env.registry_error = Some(DENIED);
    on_install(&mut env, "0.1.0");
    assert_eq!(
        order(&env),
        [Call::SetRun, Call::AddPath, Call::RegisterNotices, TRY]
    );
    assert_eq!(env.at_level(Level::Error), 2, "{:?}", env.lines());
    // Only the notice header is at stake: a warning.
    assert_eq!(env.at_level(Level::Warn), 1, "{:?}", env.lines());
    assert_eq!(
        env.count("not registered: RegSetValueExW failed with 5; notices show a plain square"),
        1
    );

    let mut env = FakeEnv::new(&[Attempt::Restored]);
    env.registry_error = Some(DENIED);
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(order(&env).len(), 8);
    assert_eq!(env.at_level(Level::Error), 3, "{:?}", env.lines());
    assert_eq!(env.count("RegSetValueExW failed with 5"), 5);
    // The switch value names no exe and the app id only a picture, so a value left behind is a warning.
    assert_eq!(env.at_level(Level::Warn), 2, "{:?}", env.lines());
    assert_eq!(
        env.count("StartupApproved value Wakeshot not removed: RegSetValueExW failed with 5"),
        1
    );
    assert_eq!(
        env.count("velopack.HaikAsatryan.Wakeshot not removed: RegSetValueExW failed with 5"),
        1
    );

    // A failed broadcast keeps the change: a warning, not an error.
    let mut env = FakeEnv::new(&[Attempt::Restored]);
    env.broadcast_error = Some(WinError {
        call: "SendMessageTimeoutW",
        code: 1460,
    });
    on_updated(&mut env, "0.1.1");
    assert_eq!(env.at_level(Level::Warn), 1, "{:?}", env.lines());
    assert_eq!(env.at_level(Level::Error), 0);
}

#[test]
fn slow_registry_steps_come_out_of_the_restore_time() {
    // Three steps of 5 s: tries at 15, 17, ... 25 s.
    let mut env = FakeEnv::new(&[]);
    env.step_ms = 5_000;
    on_install(&mut env, "0.1.0");
    let want: Vec<u64> = (0..6).map(|i| 15_000 + i * RETRY_EVERY_MS).collect();
    assert_eq!(env.tries(), want);

    // Two steps of 4.5 s (a slow Path broadcast) leave the updated hook one try.
    let mut env = FakeEnv::new(&[]);
    env.step_ms = 4_500;
    on_updated(&mut env, "0.1.1");
    assert_eq!(env.tries(), [9_000]);

    // Past the limit already: still one try.
    let mut env = FakeEnv::new(&[]);
    env.step_ms = 6_000;
    on_updated(&mut env, "0.1.1");
    assert_eq!(env.tries(), [12_000]);
    assert!(!env.calls.contains(&WAIT));
}

// `RealHookEnv` against a temp data folder, a scratch registry key and `FakePlans`.

/// Balanced, the active plan.
const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

fn user_values() -> PlanValues {
    PlanValues {
        active: P1,
        lid_ac: 1,
        lid_dc: 1,
        sleep_ac: 10_800,
        sleep_dc: 14_400,
        saver_threshold_dc: 20,
    }
}

fn entries() -> Vec<JournalEntry> {
    vec![
        JournalEntry {
            plan: P1,
            key: LidDc,
            original: 1,
            written: 0,
            restored: false,
        },
        JournalEntry {
            plan: P1,
            key: SleepDc,
            original: 14_400,
            written: 0,
            restored: false,
        },
    ]
}

/// The user's values with our writes for `entries()` stored and in force, as a hold leaves them.
fn applied() -> FakePlans {
    let fake = FakePlans::new(user_values());
    let mut s = fake.state();
    for e in entries() {
        s.set_stored(e.plan, e.key, e.written, SetBy::Override);
        s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
    }
    drop(s);
    fake
}

fn assert_originals(fake: &FakePlans) {
    let s = fake.state();
    for e in entries() {
        assert_eq!(s.stored(P1, e.key), Some(e.original), "{:?}", e.key);
        assert_eq!(s.in_force(P1, e.key), Some(e.original), "{:?}", e.key);
    }
}

fn journal(owner: JournalOwner, other: Vec<JournalFileEntry>) -> JournalFile {
    JournalParts {
        known: Journal {
            owner,
            entries: entries(),
        },
        other,
    }
    .merge()
}

/// A process this test started and that has ended, so its pid and start time name no running process.
fn dead_owner() -> JournalOwner {
    let mut child = Command::new("cmd.exe")
        .args(["/c", "exit", "0"])
        .spawn()
        .unwrap();
    let start = crate::process::start_of(&child).unwrap();
    child.wait().unwrap();
    JournalOwner {
        pid: child.id(),
        proc_start: start,
        session: 1,
    }
}

fn this_process() -> JournalOwner {
    let me = crate::process::current().unwrap();
    JournalOwner {
        pid: me.pid,
        proc_start: me.start,
        session: 1,
    }
}

/// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("wakeshot-hooks-{tag}-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Lines to the real log file and to memory, so a test both holds the file open and reads what was logged.
struct Tee {
    file: Option<LogWriter>,
    memory: MemoryLog,
}

impl Log for Tee {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        if let Some(file) = &self.file {
            file.line(level, area, text);
        }
        self.memory.line(level, area, text);
    }
}

impl Tee {
    fn has(&self, part: &str) -> bool {
        let lines = self.memory.lines.lock().unwrap();
        lines.iter().any(|(_, _, text)| text.contains(part))
    }
}

/// One `RealHookEnv` world: a data folder under `temp`, the Run and RunOnce keys and the Path value under one
/// scratch key, test mutex names, an install folder that does not need to exist.
struct Rig {
    _temp: TempDir,
    scratch: ScratchKey,
    dir: DataDir,
    exe: PathBuf,
    auto: AutoStart,
    path: UserPath,
    notices: NoticeAppId,
    log: Arc<Tee>,
    /// A profile folder and `%LocalAppData%` under `temp`, so no test ever reads the real `.cursor`.
    agents: Folders,
}

impl Rig {
    fn new(test: &str) -> Self {
        let temp = TempDir::new(test.rsplit("::").next().unwrap_or(test));
        let dir = DataDir::at(temp.0.join("Wakeshot"));
        fs::create_dir_all(dir.root()).unwrap();
        let exe = temp
            .0
            .join(r"HaikAsatryan.Wakeshot\current\wakeshot-tray.exe");
        let scratch = ScratchKey::new(test);
        let auto = AutoStart::at(scratch.path());
        let path = UserPath::at(scratch.path(), PATH_VALUE, || Ok(()));
        let notices = NoticeAppId::at(scratch.path());
        let log = Arc::new(Tee {
            file: Some(LogWriter::open(&dir.logs(), Proc::Hook).unwrap()),
            memory: MemoryLog::default(),
        });
        let agents = Folders {
            profile: temp.0.join("profile"),
            local_app_data: temp.0.clone(),
        };
        Self {
            _temp: temp,
            scratch,
            dir,
            exe,
            auto,
            path,
            notices,
            log,
            agents,
        }
    }

    fn env(&self, plans: FakePlans, liveness: Box<dyn Liveness>) -> RealHookEnv<FakePlans> {
        let places = Places {
            dir: self.dir.clone(),
            auto: self.auto.clone(),
            path: self.path.clone(),
            notices: self.notices.clone(),
            names: Names::test(self.scratch.path().rsplit('\\').next().unwrap_or("hooks")),
            exe: self.exe.clone(),
            agents: self.agents.clone(),
        };
        RealHookEnv::new(places, plans, liveness, self.log.clone())
    }

    fn folder(&self) -> &Path {
        self.exe.parent().unwrap()
    }

    /// The notice icon beside the exe, as the package ships it.
    fn ship_notice_icon(&self) -> PathBuf {
        let icon = self.folder().join(NOTICE_ICON_FILE);
        fs::create_dir_all(self.folder()).unwrap();
        fs::write(&icon, b"\x89PNG").unwrap();
        icon
    }

    fn save_journal(&self, file: &JournalFile) {
        let mut net = FileNet::restore_only(self.dir.clone(), self.auto.clone(), self.log.clone());
        net.save_journal(file).unwrap();
    }

    fn journal_file(&self) -> PathBuf {
        self.dir.file(DataFile::Journal)
    }

    /// The file and folder names left in the data folder, sorted.
    fn left(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dir.root())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

fn plan_writes(fake: &FakePlans) -> usize {
    let s = fake.state();
    s.calls
        .iter()
        .filter(|c| matches!(c, PlanCall::Write(..) | PlanCall::SetActive(_)))
        .count()
}

#[test]
fn real_uninstall_restores_removes_and_deletes() {
    let rig = Rig::new("hooks::tests::real_uninstall_restores_removes_and_deletes");
    // What an install and a hold left: the Run and RunOnce values, the Path entry beside another one, the journal
    // of a tray that died, and other data files.
    rig.auto
        .set(StartKey::Run, RUN_NAME, r#""C:\x\wakeshot-tray.exe""#)
        .unwrap();
    rig.auto
        .set(
            StartKey::RunOnce,
            RUN_ONCE_NAME,
            r#""C:\x\wakeshot-tray.exe" --restore"#,
        )
        .unwrap();
    rig.auto.set(StartKey::Run, "Other", "other.exe").unwrap();
    // The user switched start at login off in Settings, and another app on.
    rig.auto.set_approval(RUN_NAME, &SWITCH_OFF).unwrap();
    rig.auto.set_approval("Other", &SWITCH_ON).unwrap();
    let other = Path::new(r"C:\Other\bin");
    rig.path.add(other).unwrap();
    rig.path.add(rig.folder()).unwrap();
    rig.notices
        .register(Some(&rig.folder().join(NOTICE_ICON_FILE)))
        .unwrap();
    rig.save_journal(&journal(dead_owner(), vec![]));
    fs::write(rig.dir.file(DataFile::Config), b"{\"version\":1}").unwrap();
    fs::create_dir_all(rig.dir.holds()).unwrap();
    fs::write(rig.dir.file(DataFile::KeepAwake), b"{}").unwrap();
    let fake = applied();

    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    on_uninstall(&mut env, "0.1.1");

    assert_originals(&fake);
    assert_eq!(rig.auto.get(StartKey::Run, RUN_NAME), Ok(None));
    assert_eq!(rig.auto.approval(RUN_NAME), Ok(None));
    assert_eq!(rig.auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
    assert_eq!(
        rig.auto.get(StartKey::Run, "Other"),
        Ok(Some("other.exe".to_owned()))
    );
    assert_eq!(rig.auto.approval("Other"), Ok(Some(SWITCH_ON.to_vec())));
    assert!(rig.log.has("StartupApproved value Wakeshot removed"));
    assert!(!rig.notices.exists(), "the notice app id key is gone");
    assert!(
        rig.log
            .has("notice app id velopack.HaikAsatryan.Wakeshot removed")
    );
    assert_eq!(rig.path.remove(rig.folder()), Ok(PathChange::Unchanged));
    assert_eq!(rig.path.add(other), Ok(PathChange::Unchanged));
    assert!(!rig.dir.root().exists(), "the data folder is gone");
    assert!(rig.log.has("restore: the journal's values are back"));
    assert!(rig.log.has("data folder deleted"));
}

#[test]
fn real_uninstall_removes_agent_hooks_at_medium_and_hands_them_off_above() {
    let rig = Rig::new(
        "hooks::tests::real_uninstall_removes_agent_hooks_at_medium_and_hands_them_off_above",
    );
    // What a start of an earlier build of the installed tray added at Medium.
    let cursor = rig.agents.profile.join(".cursor");
    fs::create_dir_all(&cursor).unwrap();
    fs::create_dir_all(rig.folder()).unwrap();
    let hook_exe = rig.folder().join(agent_hooks::HOOK_EXE);
    fs::write(&hook_exe, b"MZ").unwrap();
    let record = rig.dir.root().join(agent_hooks::RECORD);
    let added = agent_hooks::add_as_an_earlier_build(&rig.agents, &record, &hook_exe, &*rig.log);
    assert_eq!(added, ["Cursor"]);
    let plugin = cursor.join(r"plugins\local\wakeshot");

    // Above Medium the step hands each agent to a Medium copy of `wakeshot.exe` and deletes nothing itself.
    let asked = Rc::new(RefCell::new(Vec::new()));
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    env.above_medium = || Ok(true);
    let seen = Rc::clone(&asked);
    env.medium_copy = Box::new(move |exe, agents| {
        let keys = agents.iter().map(|agent| (exe.to_path_buf(), agent.key));
        seen.borrow_mut().extend(keys);
        vec![CopyRun::Ended { outside_tree: true }; agents.len()]
    });
    env.remove_agent_hooks();
    assert_eq!(*asked.borrow(), [(hook_exe.clone(), "cursor")]);
    assert!(plugin.join(agent_hooks::MARKER).is_file());
    assert!(
        rig.log
            .has("Cursor: hook folder removal handed to a Medium copy, which ended")
    );

    // At Medium it removes them in this process, by the fixed names, and keeps the folders it did not make.
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    env.above_medium = || Ok(false);
    env.medium_copy = Box::new(|_, _| panic!("no copy at Medium"));
    env.remove_agent_hooks();
    assert!(!plugin.exists());
    assert!(cursor.join(r"plugins\local").is_dir());
    assert!(
        rig.log
            .has(r"Cursor: .cursor\plugins\local\wakeshot removed")
    );
}

#[test]
fn real_install_writes_run_and_path_then_restores() {
    let rig = Rig::new("hooks::tests::real_install_writes_run_and_path_then_restores");
    rig.save_journal(&journal(dead_owner(), vec![]));
    let fake = applied();

    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    on_install(&mut env, "0.1.0");

    let quoted = format!("\"{}\"", rig.exe.display());
    assert_eq!(rig.auto.get(StartKey::Run, RUN_NAME), Ok(Some(quoted)));
    assert_eq!(rig.path.add(rig.folder()), Ok(PathChange::Unchanged));
    assert_originals(&fake);
    assert!(!rig.journal_file().exists());
    assert_eq!(rig.left(), ["logs"], "install keeps the data folder");

    // The updated hook: the Path entry is there already; no journal, nothing to restore.
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_updated(&mut env, "0.1.1");
    assert!(rig.log.has("CLI folder already in the user Path"));
    assert!(rig.log.has("restore: no journal, nothing to restore"));
}

#[test]
fn real_install_and_updated_register_the_notice_app_id() {
    let rig = Rig::new("hooks::tests::real_install_and_updated_register_the_notice_app_id");
    let icon = rig.ship_notice_icon();
    let icon_text = icon.display().to_string();
    let expand = |text: &str| Ok(Some((REG_EXPAND_SZ, text.to_owned())));

    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_install(&mut env, "0.1.0");
    assert_eq!(rig.notices.value("DisplayName"), expand("Wakeshot"));
    assert_eq!(rig.notices.value("IconUri"), expand(&icon_text));
    assert!(rig.log.has(&format!(
        "notice app id velopack.HaikAsatryan.Wakeshot registered with its icon {icon_text}"
    )));

    // An update whose package lacks the icon: the name stays, and the key names no file that is gone.
    fs::remove_file(&icon).unwrap();
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_updated(&mut env, "0.1.1");
    assert_eq!(rig.notices.value("DisplayName"), expand("Wakeshot"));
    assert_eq!(rig.notices.value("IconUri"), Ok(None));
    assert!(
        rig.log
            .has(&format!("without its icon: {icon_text} not found"))
    );

    // The icon is back with the next update.
    rig.ship_notice_icon();
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_updated(&mut env, "0.1.2");
    assert_eq!(rig.notices.value("IconUri"), expand(&icon_text));

    // The uninstall takes the key; a second one finds nothing.
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_uninstall(&mut env, "0.1.2");
    assert!(!rig.notices.exists());
    on_uninstall(&mut env, "0.1.2");
    assert!(
        rig.log
            .has("notice app id velopack.HaikAsatryan.Wakeshot was not there")
    );
}

#[test]
fn real_install_and_updated_keep_the_switch_value() {
    let rig = Rig::new("hooks::tests::real_install_and_updated_keep_the_switch_value");
    // A fresh install writes no switch value: Windows writes it once the user uses the switch.
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_install(&mut env, "0.1.0");
    assert_eq!(rig.auto.approval(RUN_NAME), Ok(None));

    // The user switched it off; then Setup over the install (`winget upgrade`) and an update.
    rig.auto.set_approval(RUN_NAME, &SWITCH_OFF).unwrap();
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_install(&mut env, "0.1.0");
    on_obsolete(&mut env, "0.1.0");
    on_updated(&mut env, "0.1.1");
    assert_eq!(rig.auto.approval(RUN_NAME), Ok(Some(SWITCH_OFF.to_vec())));
    assert!(!rig.log.has("StartupApproved"));

    // The uninstall removes it; a second one finds nothing left.
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    on_uninstall(&mut env, "0.1.1");
    assert_eq!(rig.auto.approval(RUN_NAME), Ok(None));
    on_uninstall(&mut env, "0.1.1");
    assert!(rig.log.has("StartupApproved value Wakeshot removed"));
    assert!(rig.log.has("StartupApproved value Wakeshot was not there"));
}

#[test]
fn real_try_restore_answers() {
    let rig = Rig::new("hooks::tests::real_try_restore_answers");

    // No journal.
    let fake = applied();
    assert_eq!(
        rig.env(fake.clone(), Box::new(SystemLiveness))
            .try_restore(),
        Attempt::Nothing
    );

    // The owner runs: this test process.
    let me = this_process();
    rig.save_journal(&journal(me, vec![]));
    let before = fs::read(rig.journal_file()).unwrap();
    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    assert_eq!(env.try_restore(), Attempt::OwnerAlive(me));

    // Unknown.
    let unknown = FakeLiveness::default();
    let me_ref = ProcessRef {
        pid: me.pid,
        start: me.proc_start,
    };
    unknown.alive.lock().unwrap().push((me_ref, Alive::Unknown));
    let mut env = rig.env(fake.clone(), Box::new(unknown));
    assert_eq!(env.try_restore(), Attempt::OwnerUnknown(me));
    assert_eq!(plan_writes(&fake), 0, "no restore");
    assert_eq!(fs::read(rig.journal_file()).unwrap(), before);

    // Does not parse: left in place, not set aside.
    fs::write(rig.journal_file(), b"{ not json").unwrap();
    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    assert_eq!(
        env.try_restore(),
        Attempt::Unrestorable(Unrestorable::DoesNotParse)
    );
    assert_eq!(fs::read(rig.journal_file()).unwrap(), b"{ not json");
    assert!(!rig.dir.file(DataFile::JournalCorrupt).exists());
    assert_eq!(plan_writes(&fake), 0);

    // Known entries restored, a newer version's entry left: the file stays for it.
    let newer = JournalFileEntry {
        plan: P1,
        key: JournalKey::Other("hibernateAc".into()),
        original: 10_800,
        written: 0,
        restored: false,
    };
    rig.save_journal(&journal(dead_owner(), vec![newer]));
    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    assert_eq!(
        env.try_restore(),
        Attempt::Unrestorable(Unrestorable::NewerVersion)
    );
    assert_originals(&fake);
    assert!(rig.journal_file().exists());

    // A dead owner and only our entries: restored, the journal gone.
    let fake = applied();
    rig.save_journal(&journal(dead_owner(), vec![]));
    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    assert_eq!(env.try_restore(), Attempt::Restored);
    assert_originals(&fake);
    assert!(!rig.journal_file().exists());
}

#[test]
fn real_uninstall_resets_a_journal_that_does_not_parse() {
    // A tray died while its hold's battery lid Do nothing and sleep Never were in force, and the 1-minute AC sleep
    // of an earlier sleep route; its journal is damaged. No tray starts after an uninstall, so the hook puts each
    // to the plan's Windows default and sets the file aside.
    let rig = Rig::new("hooks::tests::real_uninstall_resets_a_journal_that_does_not_parse");
    fs::write(rig.journal_file(), b"{ not json").unwrap();
    let fake = applied();
    {
        let mut s = fake.state();
        let short = wakeshot_core::settings::SHORT_TIMEOUT_S;
        s.set_stored(P1, SettingKey::SleepAc, short, SetBy::Override);
        s.set_in_force(P1, SettingKey::SleepAc, short, SetBy::Override);
    }
    let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
    on_uninstall(&mut env, "0.1.1");

    let s = fake.state();
    for (key, default) in [
        (LidDc, FAKE_DEFAULT_LID),
        (SleepDc, FAKE_DEFAULT_SLEEP_DC),
        (SettingKey::SleepAc, FAKE_DEFAULT_OTHER),
    ] {
        assert_eq!(s.stored(P1, key), Some(default), "{key:?}");
        assert_eq!(s.in_force(P1, key), Some(default), "{key:?}");
    }
    assert_eq!(s.in_force(P1, SettingKey::LidAc), Some(1), "the user's own");
    drop(s);
    assert!(!rig.journal_file().exists());
    assert_eq!(rig.left(), ["logs"], "the logs keep the old values");
    assert!(
        rig.log
            .has("no restore: journal.json does not parse; the lost-journal reset follows")
    );
    assert!(rig.log.has("lost journal: lidDc on plan"));
    assert!(rig.log.has(
        "lost journal reset: [LidDc, SleepDc, SleepAc] set to Windows defaults; the old values are above"
    ));
}

/// Every kind of thing the data folder holds.
fn fill(dir: &DataDir) {
    let root = dir.root();
    for file in [
        DataFile::Config,
        DataFile::State,
        DataFile::Journal,
        DataFile::JournalCorrupt,
        DataFile::RestoreRequest,
        DataFile::KeepAwake,
    ] {
        fs::create_dir_all(dir.file(file).parent().unwrap()).unwrap();
        fs::write(dir.file(file), b"{}").unwrap();
    }
    fs::write(dir.holds().join("run-42.json"), b"{}").unwrap();
    fs::create_dir_all(root.join(r"dry-run\logs")).unwrap();
    fs::write(root.join(r"dry-run\logs\wakeshot.log"), b"x").unwrap();
    fs::write(root.join("journal.json.1.0.tmp"), b"x").unwrap();
    fs::write(dir.logs().join("wakeshot.1.log"), b"x").unwrap();
}

#[test]
fn real_keep_journal_and_logs() {
    let rig = Rig::new("hooks::tests::real_keep_journal_and_logs");
    fill(&rig.dir);
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    assert_eq!(env.clean_data_folder(DataFolder::KeepJournalAndLogs), []);
    assert_eq!(rig.left(), ["journal.json", "logs"]);
    let mut logs: Vec<String> = fs::read_dir(rig.dir.logs())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    logs.sort();
    assert_eq!(logs, ["wakeshot.1.log", "wakeshot.log"]);
}

#[test]
fn real_delete_all_takes_the_open_log_too() {
    let rig = Rig::new("hooks::tests::real_delete_all_takes_the_open_log_too");
    fill(&rig.dir);
    rig.log.line(Level::Info, AREA, "a line in the open log");

    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    assert_eq!(env.clean_data_folder(DataFolder::DeleteAll), []);
    assert!(!rig.dir.root().exists());
    // A line after the delete goes nowhere and brings nothing back.
    rig.log.line(Level::Info, AREA, "after");
    assert!(!rig.dir.root().exists());
    // A second run finds nothing to do.
    assert_eq!(env.clean_data_folder(DataFolder::DeleteAll), []);
}

#[test]
fn real_folder_that_cannot_go_whole_keeps_its_log() {
    let rig = Rig::new("hooks::tests::real_folder_that_cannot_go_whole_keeps_its_log");
    fill(&rig.dir);
    // Open without delete sharing, as another program might hold a file.
    let held = rig.dir.holds().join("held.json");
    fs::write(&held, b"{}").unwrap();
    let open = fs::OpenOptions::new()
        .read(true)
        .share_mode(1) // FILE_SHARE_READ only
        .open(&held)
        .unwrap();

    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    let left = env.clean_data_folder(DataFolder::DeleteAll);
    println!("a file open without delete sharing: {left:?}");
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!(left[0].path, rig.dir.holds());
    assert_eq!(left[0].error.call, "remove_dir_all");
    assert_eq!(rig.left(), ["holds", "logs"]);
    assert!(log::log_file(&rig.dir.logs()).is_file(), "the log stays");
    assert!(rig.dir.logs().join("wakeshot.1.log").is_file());
    drop(open);
}

#[test]
fn real_steps_without_a_data_folder_or_exe() {
    let rig = Rig::new("hooks::tests::real_steps_without_a_data_folder_or_exe");
    let missing = WinError {
        call: "SHGetKnownFolderPath",
        code: 2,
    };
    let mut env = rig.env(applied(), Box::new(SystemLiveness));
    env.dir = Err(missing);
    env.net = Err(missing);
    env.exe = Err(WinError {
        call: "GetModuleFileNameW",
        code: 122,
    });
    rig.auto.set(StartKey::RunOnce, RUN_ONCE_NAME, "x").unwrap();
    assert_eq!(env.try_restore(), Attempt::Failed(missing.into()));
    assert_eq!(
        env.clean_data_folder(DataFolder::DeleteAll),
        [Undeleted {
            path: PathBuf::new(),
            error: missing
        }]
    );
    assert_eq!(env.set_run().unwrap_err().call, "GetModuleFileNameW");
    assert_eq!(env.add_path().unwrap_err().call, "GetModuleFileNameW");
    assert_eq!(
        env.register_notices().unwrap_err().call,
        "GetModuleFileNameW"
    );
    assert!(!rig.notices.exists(), "no key without the exe's folder");
    // The steps that need neither still run.
    assert_eq!(env.remove_run_once(), Ok(true));
    assert_eq!(env.remove_run(), Ok(false));
    assert_eq!(env.remove_approval(), Ok(false));
    assert_eq!(env.remove_notices(), Ok(false));
}
