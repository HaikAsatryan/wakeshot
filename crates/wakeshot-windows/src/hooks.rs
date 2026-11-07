//! What each Velopack hook does, in order and within its time, over trait `HookEnv`.
//!
//! Registry first, restore last: Velopack kills a hook that runs past its limit, and a restore that hangs must not
//! leave a Run value or a Path entry that points into a deleted folder. Each failure is a log line and the hook goes
//! on: hooks never panic and never show UI (`research/distribution.md`, section 1).
//!
//! `velo.rs` calls each hook with [`RealHookEnv::installed`]; tests use a fake `HookEnv`, or a `RealHookEnv` over a
//! temp data folder, a scratch registry key and `FakePlans`.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wakeshot_core::command::Failure;
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::reset::{Found as ResetFound, Reset};
use wakeshot_core::ownership::restore::Found;
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::JournalOwner;
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, ERROR_NO_UNICODE_TRANSLATION};

use crate::WinError;
use crate::agent_hooks::{self, Agent, CopyRun, Folders};
use crate::install::{PathChange, UserPath};
use crate::log::{self, Level, Log, Proc};
use crate::notify::{NOTICE_APP_ID, NOTICE_ICON_FILE, NoticeAppId};
use crate::power::journal::{FileNet, NoGuard, RUN_ONCE_NAME, reset_locked, restore_locked};
use crate::power::plan::Win32Power;
use crate::process::{self, Liveness, SystemLiveness};
use crate::recovery::open_log;
use crate::store::{AutoStart, DataDir, DataFile, Names, StartKey};

/// The start-at-login value's name under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (architecture section
/// 6, "Registry values").
pub const RUN_NAME: &str = "Wakeshot";

/// Restore tries go on this long from the start of the install hook; Velopack kills it at 30 s.
pub const INSTALL_LIMIT_MS: u64 = 25_000;

/// The same for the updated hook, which Velopack kills at 15 s: the hook runs a just-extracted exe, which took up to
/// 1.7 s to start in a Velopack test, so 12 s would leave only about 1.3 s.
pub const UPDATED_LIMIT_MS: u64 = 10_000;

/// The same for the uninstall hook: Velopack's docs say 30 s (60 s measured), and the data folder step comes after.
pub const UNINSTALL_LIMIT_MS: u64 = 25_000;

/// A failed restore is tried again this often within a hook's limit.
pub const RETRY_EVERY_MS: u64 = 2_000;

const AREA: &str = "hook";

/// What a hook needs from the PC: the real one is [`RealHookEnv`].
pub trait HookEnv {
    /// Writes the Run value: the quoted tray exe. `Ok`: the command written.
    fn set_run(&mut self) -> Result<String, WinError>;
    /// `true`: there was one.
    fn remove_run(&mut self) -> Result<bool, WinError>;
    /// The Run value's `StartupApproved` value, which Windows writes once the user has used the Settings > Apps >
    /// Startup switch. Only the uninstall removes it: install and update leave it, so the user's off choice outlasts
    /// `winget upgrade`. `true`: there was one.
    fn remove_approval(&mut self) -> Result<bool, WinError>;
    /// The tray exe's folder in the user Path, where `wakeshot.exe` sits beside it.
    fn add_path(&mut self) -> Result<PathChange, WinError>;
    fn remove_path(&mut self) -> Result<PathChange, WinError>;
    /// The notice header's name and picture: the app id key (`notify::NoticeAppId`), with the PNG that ships beside
    /// the tray exe.
    fn register_notices(&mut self) -> Result<Registered, WinError>;
    /// `true`: there was one.
    fn remove_notices(&mut self) -> Result<bool, WinError>;
    /// The hook folders Wakeshot added to agents (`agent_hooks`; `docs/agent-hooks.md`, "Removing"), by their fixed
    /// names only. It never fails the hook: each agent's outcome is its own log line, written here.
    fn remove_agent_hooks(&mut self);
    /// `true`: there was one.
    fn remove_run_once(&mut self) -> Result<bool, WinError>;
    /// One restore attempt, if a journal exists and its owner is Gone.
    fn try_restore(&mut self) -> Attempt;
    /// The lost-journal reset of a journal that does not parse (`power::journal::reset_locked`, `damaged`).
    fn reset_lost(&mut self) -> Reset;
    /// The uninstall's data folder step. Empty: done.
    fn clean_data_folder(&mut self, step: DataFolder) -> Vec<Undeleted>;
    /// Monotonic milliseconds from any fixed start.
    fn now_ms(&self) -> u64;
    fn wait_ms(&mut self, ms: u64);
    fn log(&self, level: Level, text: &str);
}

/// What one restore attempt found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    /// The journal's values are back and the journal is gone.
    Restored,
    /// No journal is left: none was there, or it held nothing to restore.
    Nothing,
    /// The journal's owner runs (a dev build started from elsewhere: Velopack kills an installed tray before each
    /// hook), or another tray took the journal over. It restores the values itself.
    OwnerAlive(JournalOwner),
    /// The owner's state could not be read. Unknown never starts a restore (architecture section 5, "Liveness").
    OwnerUnknown(JournalOwner),
    /// A journal this version leaves as it is; trying again would not change it.
    Unrestorable(Unrestorable),
    /// Tried again while the hook's limit allows.
    Failed(Failure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unrestorable {
    /// It does not parse. The tray, which can warn, sets it aside at its next start (restore step 1).
    DoesNotParse,
    /// It holds only entries of a newer version, which keep the user's values for that version (architecture section
    /// 6, "Journal format").
    NewerVersion,
}

/// What `register_notices` wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Registered {
    /// The name, and the picture in this file.
    WithIcon(PathBuf),
    /// The name only: this file was not there, so the header shows Windows' default picture.
    NoIcon(PathBuf),
}

/// The uninstall's data folder step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFolder {
    /// After a restore that left no journal: `%LocalAppData%\Wakeshot` goes whole.
    DeleteAll,
    /// Otherwise: `journal.json` and `logs\` stay, so a reinstall's install hook restores and the log holds the old
    /// values; the rest goes.
    KeepJournalAndLogs,
}

/// One thing in the data folder the step could not delete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undeleted {
    pub path: PathBuf,
    pub error: WinError,
}

/// The hooks that restore, for their limits and log lines.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hook {
    Install,
    Updated,
    Uninstall,
}

impl Hook {
    fn name(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Updated => "updated",
            Self::Uninstall => "uninstall",
        }
    }

    fn limit_ms(self) -> u64 {
        match self {
            Self::Install => INSTALL_LIMIT_MS,
            Self::Updated => UPDATED_LIMIT_MS,
            Self::Uninstall => UNINSTALL_LIMIT_MS,
        }
    }

    /// What keeps a journal this hook could not restore reachable.
    fn kept(self) -> &'static str {
        match self {
            Self::Uninstall => {
                "journal.json and the logs stay in the data folder, so a reinstall restores the values"
            }
            Self::Install | Self::Updated => {
                "the journal stays, so the tray restores the values at its next start"
            }
        }
    }
}

/// `--veloapp-install` (also Setup.exe over an install, which is what `winget upgrade` runs): the Run value, the
/// Path entry and the notice app id, then the restore.
pub fn on_install(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Install, version);
    set_run(env);
    add_path(env);
    register_notices(env);
    restore(env, Hook::Install, started);
    end(env, Hook::Install, started);
}

/// `--veloapp-obsolete`, on the old version before an update: a log line only. Updates apply only when nothing is
/// overridden, and the updated hook restores anyway.
pub fn on_obsolete(env: &mut impl HookEnv, version: &str) {
    let text = format!(
        "obsolete hook for version {version}: nothing to do; the updated hook adds the Path entry and restores"
    );
    env.log(Level::Info, &text);
}

/// `--veloapp-updated`, on the new version: the Path entry (no change while `current` keeps its path) and the notice
/// app id (so an update from a version without it gets it), then the restore. The Run value is left as it is, so a
/// user who removed it keeps it removed.
pub fn on_updated(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Updated, version);
    add_path(env);
    register_notices(env);
    restore(env, Hook::Updated, started);
    end(env, Hook::Updated, started);
}

/// `--veloapp-uninstall`: the Run value, the RunOnce value, the Path entry, the notice app id and the agents' hook
/// folders go first, since the files they name are being removed, then the restore, then the data folder step. The
/// Run value's `StartupApproved` value goes with it, though Windows wrote it, so a later install starts as on a new
/// PC. The tray icons' `NotifyIconSettings` entries and the app id's `Notifications\Settings` key stay: Windows wrote
/// them, not Wakeshot (change it when a fully clean uninstall is wanted: remove the three entries by GUID and that key
/// here, after the restore).
pub fn on_uninstall(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Uninstall, version);
    remove_run(env);
    remove_approval(env);
    remove_run_once(env);
    remove_path(env);
    remove_notices(env);
    env.remove_agent_hooks();
    let step = match restore(env, Hook::Uninstall, started) {
        Attempt::Restored | Attempt::Nothing => DataFolder::DeleteAll,
        Attempt::Unrestorable(Unrestorable::DoesNotParse) => {
            reset_lost(env);
            DataFolder::KeepJournalAndLogs
        }
        _ => DataFolder::KeepJournalAndLogs,
    };
    clean(env, step);
    end(env, Hook::Uninstall, started);
}

/// No tray starts after an uninstall, so a journal that does not parse would leave Wakeshot's values with nothing that
/// ever resets them (product "User settings safety" 5 and 7). So the uninstall runs the tray's lost-journal reset
/// itself: each lid, sleep and screen value at exactly Wakeshot's goes to the plan's Windows default, and the file is
/// set aside. `damaged`: the file was there and did not parse a moment ago, so one gone since is that file, set aside
/// by the restore routine, and its values are still in force. A hook shows no notice; the log keeps each old value
/// (the reset's own lines) and this summary, and the data folder step keeps the logs.
fn reset_lost(env: &mut impl HookEnv) {
    let done = env.reset_lost();
    let keys = done.reset_keys();
    let (level, text) = match done.result {
        Ok(()) if keys.is_empty() => (
            Level::Info,
            "lost journal reset: no value looked like Wakeshot's, so nothing was set".to_owned(),
        ),
        Ok(()) => (
            Level::Warn,
            format!(
                "lost journal reset: {keys:?} set to Windows defaults; the old values are above"
            ),
        ),
        Err(f) => (
            Level::Error,
            format!(
                "lost journal reset failed: {}; {keys:?} set to Windows defaults; the lid, sleep and screen \
                 settings may need setting by hand",
                error_text(f)
            ),
        ),
    };
    env.log(level, &text);
}

/// The start line, which also proves `VelopackApp` ran the hook: `vpk` does not check that it does.
fn begin(env: &impl HookEnv, hook: Hook, version: &str) -> u64 {
    let started = env.now_ms();
    let text = format!("{} hook for version {version} started", hook.name());
    env.log(Level::Info, &text);
    started
}

fn end(env: &impl HookEnv, hook: Hook, started: u64) {
    let ms = env.now_ms().saturating_sub(started);
    let text = format!(
        "{} hook done after {}.{} s",
        hook.name(),
        ms / 1000,
        ms % 1000 / 100
    );
    env.log(Level::Info, &text);
}

fn set_run(env: &mut impl HookEnv) {
    match env.set_run() {
        Ok(command) => env.log(
            Level::Info,
            &format!("Run value {RUN_NAME} set to {command}"),
        ),
        Err(e) => env.log(
            Level::Error,
            &format!(
                "Run value {RUN_NAME} not set: {}; Wakeshot does not start at sign-in",
                error_text(e)
            ),
        ),
    }
}

fn remove_run(env: &mut impl HookEnv) {
    let removed = env.remove_run();
    removed_line(env, removed, &format!("Run value {RUN_NAME}"));
}

fn remove_approval(env: &mut impl HookEnv) {
    let value = format!("StartupApproved value {RUN_NAME}");
    let (level, text) = match env.remove_approval() {
        Ok(true) => (Level::Info, format!("{value} removed")),
        Ok(false) => (Level::Info, format!("{value} was not there")),
        // A warning, not an error like the others: it names no exe, so a value left behind breaks nothing.
        Err(e) => (
            Level::Warn,
            format!(
                "{value} not removed: {}; a later install keeps its Settings > Apps > Startup switch as it was",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

fn remove_run_once(env: &mut impl HookEnv) {
    let removed = env.remove_run_once();
    removed_line(env, removed, &format!("RunOnce value {RUN_ONCE_NAME}"));
}

/// Warnings, not errors: only the picture and name in the notice header are at stake.
fn register_notices(env: &mut impl HookEnv) {
    let id = format!("notice app id {NOTICE_APP_ID}");
    let (level, text) = match env.register_notices() {
        Ok(Registered::WithIcon(icon)) => (
            Level::Info,
            format!("{id} registered with its icon {}", icon.display()),
        ),
        Ok(Registered::NoIcon(icon)) => (
            Level::Warn,
            format!(
                "{id} registered without its icon: {} not found; notices show a plain square in their header",
                icon.display()
            ),
        ),
        Err(e) => (
            Level::Warn,
            format!(
                "{id} not registered: {}; notices show a plain square in their header",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

fn remove_notices(env: &mut impl HookEnv) {
    let id = format!("notice app id {NOTICE_APP_ID}");
    let (level, text) = match env.remove_notices() {
        Ok(true) => (Level::Info, format!("{id} removed")),
        Ok(false) => (Level::Info, format!("{id} was not there")),
        // A warning: the key names only a picture, and nothing starts from it.
        Err(e) => (
            Level::Warn,
            format!(
                "{id} not removed: {}; it names an icon the uninstall deletes",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

fn removed_line(env: &impl HookEnv, removed: Result<bool, WinError>, value: &str) {
    let (level, text) = match removed {
        Ok(true) => (Level::Info, format!("{value} removed")),
        Ok(false) => (Level::Info, format!("{value} was not there")),
        Err(e) => (
            Level::Error,
            format!(
                "{value} not removed: {}; it names an exe the uninstall deletes",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

fn add_path(env: &mut impl HookEnv) {
    let (level, text) = match env.add_path() {
        Ok(PathChange::Changed { broadcast: Ok(()) }) => {
            (Level::Info, "CLI folder added to the user Path".to_owned())
        }
        Ok(PathChange::Changed { broadcast: Err(e) }) => (
            Level::Warn,
            format!(
                "CLI folder added to the user Path, but telling running programs failed ({}): new terminals see it \
                 after the next sign-in",
                error_text(e)
            ),
        ),
        Ok(PathChange::Unchanged) => (
            Level::Info,
            "CLI folder already in the user Path".to_owned(),
        ),
        Err(e) => (
            Level::Error,
            format!(
                "CLI folder not added to the user Path: {}; `wakeshot` works only by its full path",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

fn remove_path(env: &mut impl HookEnv) {
    let (level, text) = match env.remove_path() {
        Ok(PathChange::Changed { broadcast: Ok(()) }) => (
            Level::Info,
            "CLI folder removed from the user Path".to_owned(),
        ),
        Ok(PathChange::Changed { broadcast: Err(e) }) => (
            Level::Warn,
            format!(
                "CLI folder removed from the user Path, but telling running programs failed ({}): new terminals see \
                 it after the next sign-in",
                error_text(e)
            ),
        ),
        Ok(PathChange::Unchanged) => (
            Level::Info,
            "CLI folder was not in the user Path".to_owned(),
        ),
        Err(e) => (
            Level::Error,
            format!(
                "CLI folder not removed from the user Path: {}; the entry names a folder the uninstall deletes",
                error_text(e)
            ),
        ),
    };
    env.log(level, &text);
}

/// Tries at once, then every 2 s while the hook's limit, counted from the hook's start, allows another try. The time
/// the registry steps took (a Path broadcast waits up to 1 s per window) comes out of it. One try always runs, even
/// past the limit: Velopack's kill comes later, and a restore stopped at any point keeps the values reachable
/// (architecture section 5, "Every stop point").
fn restore(env: &mut impl HookEnv, hook: Hook, started: u64) -> Attempt {
    let deadline = started.saturating_add(hook.limit_ms());
    let mut tries = 0u32;
    loop {
        tries = tries.saturating_add(1);
        let attempt = env.try_restore();
        let Attempt::Failed(failure) = attempt else {
            let (level, text) = outcome(hook, attempt);
            env.log(level, &text);
            return attempt;
        };
        let why = error_text(failure);
        if env.now_ms().saturating_add(RETRY_EVERY_MS) > deadline {
            let level = match hook {
                Hook::Uninstall => Level::Error,
                Hook::Install | Hook::Updated => Level::Warn,
            };
            let text = format!(
                "restore try {tries} failed: {why}; no time for another within {} s: {}",
                hook.limit_ms() / 1000,
                hook.kept()
            );
            env.log(level, &text);
            return attempt;
        }
        let text = format!(
            "restore try {tries} failed: {why}; next try in {} s",
            RETRY_EVERY_MS / 1000
        );
        env.log(Level::Warn, &text);
        env.wait_ms(RETRY_EVERY_MS);
    }
}

/// The one line for an attempt that ends the tries.
fn outcome(hook: Hook, attempt: Attempt) -> (Level, String) {
    let owner = |o: JournalOwner| format!("pid {}, start {}", o.pid, o.proc_start.0);
    match attempt {
        Attempt::Restored => (Level::Info, "restore: the journal's values are back".to_owned()),
        Attempt::Nothing => (Level::Info, "restore: no journal, nothing to restore".to_owned()),
        Attempt::OwnerAlive(o) => {
            let kept = match hook {
                Hook::Uninstall => "; journal.json and the logs stay in the data folder",
                Hook::Install | Hook::Updated => "",
            };
            let text = format!(
                "no restore: the journal's owner ({}) runs and restores the values itself{kept}",
                owner(o)
            );
            (Level::Info, text)
        }
        Attempt::OwnerUnknown(o) => {
            let text = format!(
                "no restore: the state of the journal's owner ({}) is unknown; {}",
                owner(o),
                hook.kept()
            );
            (Level::Warn, text)
        }
        Attempt::Unrestorable(Unrestorable::DoesNotParse) => {
            let text = match hook {
                Hook::Uninstall => {
                    "no restore: journal.json does not parse; the lost-journal reset follows".to_owned()
                }
                Hook::Install | Hook::Updated => "no restore: journal.json does not parse and stays as it is; the \
                                                  tray sets it aside and warns at its next start"
                    .to_owned(),
            };
            (Level::Warn, text)
        }
        Attempt::Unrestorable(Unrestorable::NewerVersion) => (
            Level::Warn,
            "no restore: the journal holds only settings a newer Wakeshot version wrote, so it stays for that version"
                .to_owned(),
        ),
        Attempt::Failed(f) => {
            let text = format!("restore failed: {}", error_text(f));
            (Level::Warn, text)
        }
    }
}

fn clean(env: &mut impl HookEnv, step: DataFolder) {
    let undeleted = env.clean_data_folder(step);
    if undeleted.is_empty() {
        let text = match step {
            DataFolder::DeleteAll => "data folder deleted",
            DataFolder::KeepJournalAndLogs => {
                "data folder emptied but for journal.json and the logs"
            }
        };
        env.log(Level::Info, text);
    }
    for item in undeleted {
        let text = format!(
            "data folder: could not delete {}: {}",
            item.path.display(),
            error_text(item.error)
        );
        env.log(Level::Error, &text);
    }
}

fn error_text(e: impl Into<Failure>) -> String {
    let e = e.into();
    format!("{} failed with {}", e.call, e.code)
}

/// Where a [`RealHookEnv`] reads and writes. Tests point each one at a temp folder or a scratch key.
#[derive(Clone, Debug)]
pub struct Places {
    pub dir: DataDir,
    pub auto: AutoStart,
    pub path: UserPath,
    pub notices: NoticeAppId,
    pub names: Names,
    /// The tray exe the Run value starts. Its folder is the Path entry (`wakeshot.exe` sits beside it) and holds the
    /// notice icon.
    pub exe: PathBuf,
    /// Where the agents' hook folders are (`agent_hooks`).
    pub agents: Folders,
}

/// The real `HookEnv`: `store::AutoStart` for the Run and RunOnce values and the Run value's `StartupApproved` value
/// (their one writer), `install` for the Path entry, `notify::NoticeAppId` for the notice app id,
/// `power::journal::restore_locked` with `FileNet::restore_only` for one restore attempt, the same single call the
/// guard and restore mode make.
pub struct RealHookEnv<P> {
    /// An `Err` fails only the steps that need it, so the registry steps still run.
    dir: Result<DataDir, WinError>,
    exe: Result<PathBuf, WinError>,
    auto: AutoStart,
    path: UserPath,
    notices: NoticeAppId,
    names: Names,
    plans: P,
    net: Result<FileNet<NoGuard>, WinError>,
    liveness: Box<dyn Liveness>,
    log: Arc<dyn Log>,
    epoch: Instant,
    agents: Result<Folders, WinError>,
    /// `process::current_above_medium`; tests pick the level.
    above_medium: fn() -> Result<bool, WinError>,
    /// `agent_hooks::medium_copies`; tests record the hand-off instead.
    medium_copy: MediumCopy,
}

type MediumCopy = Box<dyn FnMut(&Path, &[&'static Agent]) -> Vec<CopyRun>>;

impl RealHookEnv<Win32Power> {
    /// The installed app's: `%LocalAppData%\Wakeshot`, the real Run, RunOnce and Path values, the real mutex names,
    /// this exe, the real power calls and the log with process `hook`. Never fails: a part that cannot be found fails
    /// only the steps that need it, each with its own log line.
    pub fn installed() -> Self {
        let dir = DataDir::real();
        let log: Arc<dyn Log> = match &dir {
            Ok(dir) => open_log(dir, Proc::Hook),
            Err(_) => Arc::new(NoLog),
        };
        let auto = AutoStart::real();
        let net = dir
            .clone()
            .map(|dir| FileNet::restore_only(dir, auto.clone(), log.clone()));
        Self {
            dir,
            exe: std::env::current_exe().map_err(|e| io_error("GetModuleFileNameW", &e)),
            auto,
            path: UserPath::real(),
            notices: NoticeAppId::real(),
            names: Names::real(),
            plans: Win32Power::new(log.clone()),
            net,
            liveness: Box::new(SystemLiveness),
            log,
            epoch: Instant::now(),
            agents: Folders::real(),
            above_medium: process::current_above_medium,
            medium_copy: Box::new(agent_hooks::medium_copies),
        }
    }
}

impl<P: PlanIo> RealHookEnv<P> {
    pub fn new(places: Places, plans: P, liveness: Box<dyn Liveness>, log: Arc<dyn Log>) -> Self {
        let net = FileNet::restore_only(places.dir.clone(), places.auto.clone(), log.clone());
        Self {
            dir: Ok(places.dir),
            exe: Ok(places.exe),
            auto: places.auto,
            path: places.path,
            notices: places.notices,
            names: places.names,
            plans,
            net: Ok(net),
            liveness,
            log,
            epoch: Instant::now(),
            agents: Ok(places.agents),
            above_medium: process::current_above_medium,
            medium_copy: Box::new(agent_hooks::medium_copies),
        }
    }

    fn cli_folder(&self) -> Result<&Path, WinError> {
        let exe = self.exe.as_ref().map_err(|e| *e)?;
        // An empty parent is refused by `UserPath` itself.
        exe.parent().ok_or(WinError {
            call: "RegSetValueExW",
            code: ERROR_INVALID_PARAMETER.0,
        })
    }
}

impl<P: PlanIo> HookEnv for RealHookEnv<P> {
    fn set_run(&mut self) -> Result<String, WinError> {
        let exe = self.exe.as_ref().map_err(|e| *e)?;
        let exe = exe.to_str().ok_or(WinError {
            call: "RegSetValueExW",
            code: ERROR_NO_UNICODE_TRANSLATION.0,
        })?;
        // A Windows path cannot hold a quote, so quoting the whole path is enough for any folder name.
        let command = format!("\"{exe}\"");
        self.auto.set(StartKey::Run, RUN_NAME, &command)?;
        Ok(command)
    }

    fn remove_run(&mut self) -> Result<bool, WinError> {
        self.auto.remove(StartKey::Run, RUN_NAME)
    }

    fn remove_approval(&mut self) -> Result<bool, WinError> {
        self.auto.remove_approval(RUN_NAME)
    }

    fn add_path(&mut self) -> Result<PathChange, WinError> {
        self.path.add(self.cli_folder()?)
    }

    fn remove_path(&mut self) -> Result<PathChange, WinError> {
        self.path.remove(self.cli_folder()?)
    }

    fn register_notices(&mut self) -> Result<Registered, WinError> {
        let icon = self.cli_folder()?.join(NOTICE_ICON_FILE);
        // A file that cannot be read counts as missing: a key that names it would show nothing either.
        if icon.is_file() {
            self.notices.register(Some(&icon))?;
            Ok(Registered::WithIcon(icon))
        } else {
            self.notices.register(None)?;
            Ok(Registered::NoIcon(icon))
        }
    }

    fn remove_notices(&mut self) -> Result<bool, WinError> {
        self.notices.remove()
    }

    /// `agent_hooks::on_uninstall`, with the `wakeshot.exe` beside the tray exe: the one the hook files name, and the
    /// one an elevated uninstall starts at Medium. Its lines go to this hook's log, area `agent-hooks`.
    fn remove_agent_hooks(&mut self) {
        let folders = match &self.agents {
            Ok(folders) => folders,
            Err(e) => {
                let text = format!(
                    "agent hook folders not removed: the profile folder does not read: {}",
                    error_text(*e)
                );
                self.log(Level::Warn, &text);
                return;
            }
        };
        let hook_exe = self
            .cli_folder()
            .ok()
            .map(|folder| folder.join(agent_hooks::HOOK_EXE));
        agent_hooks::on_uninstall(
            folders,
            hook_exe.as_deref(),
            (self.above_medium)(),
            &mut *self.medium_copy,
            &*self.log,
        );
    }

    fn remove_run_once(&mut self) -> Result<bool, WinError> {
        self.auto.remove(StartKey::RunOnce, RUN_ONCE_NAME)
    }

    fn try_restore(&mut self) -> Attempt {
        let net = match &mut self.net {
            Ok(net) => net,
            Err(e) => return Attempt::Failed((*e).into()),
        };
        // A look without the lock, to learn the owner: every write of the journal is atomic, so the read is whole.
        // The routine reads it again under the lock and leaves it if the owner changed in between.
        let owner = match net.load_journal() {
            JournalRead::Missing => return Attempt::Nothing,
            JournalRead::Failed(f) => return Attempt::Failed(f),
            JournalRead::Corrupt => return Attempt::Unrestorable(Unrestorable::DoesNotParse),
            JournalRead::Read(file) => file.owner,
        };
        let process = ProcessRef {
            pid: owner.pid,
            start: owner.proc_start,
        };
        match self.liveness.check(process) {
            Alive::Yes => return Attempt::OwnerAlive(owner),
            Alive::Unknown => return Attempt::OwnerUnknown(owner),
            Alive::Gone => {}
        }
        let done = restore_locked(
            &mut self.plans,
            net,
            None,
            Some(owner),
            &self.names,
            &*self.log,
        );
        let restored = match (done.found, done.result) {
            (Found::OtherOwner(other), _) => return Attempt::OwnerAlive(other),
            (_, Err(f)) => return Attempt::Failed(f),
            (Found::SetAside, Ok(())) => {
                return Attempt::Unrestorable(Unrestorable::DoesNotParse);
            }
            (Found::Entries, Ok(())) => Attempt::Restored,
            (Found::Nothing, Ok(())) => Attempt::Nothing,
        };
        // The routine keeps a file that holds only a newer version's entries; the data folder step must keep it too.
        match net.load_journal() {
            JournalRead::Missing => restored,
            JournalRead::Read(_) => Attempt::Unrestorable(Unrestorable::NewerVersion),
            JournalRead::Corrupt => Attempt::Unrestorable(Unrestorable::DoesNotParse),
            JournalRead::Failed(f) => Attempt::Failed(f),
        }
    }

    fn reset_lost(&mut self) -> Reset {
        match &mut self.net {
            Ok(net) => reset_locked(&mut self.plans, net, true, &self.names, &*self.log),
            Err(e) => Reset {
                found: ResetFound::Lost,
                plan: None,
                keys: Vec::new(),
                set_aside: None,
                result: Err((*e).into()),
                journal: Vec::new(),
            },
        }
    }

    fn clean_data_folder(&mut self, step: DataFolder) -> Vec<Undeleted> {
        match &self.dir {
            Ok(dir) => clean_on_disk(dir, step),
            Err(e) => vec![Undeleted {
                path: PathBuf::new(),
                error: *e,
            }],
        }
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn wait_ms(&mut self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn log(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }
}

/// The log of a hook that found no data folder: it still does its registry steps.
struct NoLog;

impl Log for NoLog {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

/// The data folder step on disk. `DeleteAll` takes the log last, and only once everything else is gone, so a folder
/// that cannot be deleted whole keeps the log that says why.
fn clean_on_disk(dir: &DataDir, step: DataFolder) -> Vec<Undeleted> {
    let root = dir.root();
    let journal = dir.file(DataFile::Journal);
    let logs = dir.logs();
    let logs_name = logs.file_name().unwrap_or_default();
    let mut undeleted = Vec::new();
    let keep = match step {
        DataFolder::DeleteAll => vec![logs_name],
        DataFolder::KeepJournalAndLogs => vec![logs_name, journal.file_name().unwrap_or_default()],
    };
    let exists = remove_children(root, &keep, &mut undeleted);
    if !exists || step == DataFolder::KeepJournalAndLogs || !undeleted.is_empty() {
        return undeleted;
    }
    let log_file = log::log_file(&logs);
    remove_children(
        &logs,
        &[log_file.file_name().unwrap_or_default()],
        &mut undeleted,
    );
    if !undeleted.is_empty() {
        return undeleted;
    }
    // Every process opens the log with delete sharing, so this hook's own open handle does not stop the delete
    // (tested, `real_delete_all_takes_the_open_log_too`).
    let last = [
        (log_file.as_path(), false, "DeleteFileW"),
        (logs.as_path(), true, "RemoveDirectoryW"),
        (root, true, "RemoveDirectoryW"),
    ];
    for (path, folder, call) in last {
        let removed = match folder {
            true => fs::remove_dir(path),
            false => fs::remove_file(path),
        };
        match removed {
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                undeleted.push(Undeleted {
                    path: path.to_path_buf(),
                    error: io_error(call, &e),
                });
                break;
            }
            _ => {}
        }
    }
    undeleted
}

/// Removes everything in `folder` but the names in `keep` (any case, as Windows compares names). `false`: the folder
/// does not exist.
fn remove_children(folder: &Path, keep: &[&OsStr], undeleted: &mut Vec<Undeleted>) -> bool {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return false,
        Err(e) => {
            undeleted.push(Undeleted {
                path: folder.to_path_buf(),
                error: io_error("FindFirstFileExW", &e),
            });
            return true;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                // The listing can fail the same way again, so stop rather than loop.
                undeleted.push(Undeleted {
                    path: folder.to_path_buf(),
                    error: io_error("FindNextFileW", &e),
                });
                break;
            }
        };
        let name = entry.file_name();
        if keep.iter().any(|k| name.eq_ignore_ascii_case(k)) {
            continue;
        }
        let path = entry.path();
        if let Err(error) = remove_entry(&entry, &path) {
            undeleted.push(Undeleted { path, error });
        }
    }
    true
}

fn remove_entry(entry: &fs::DirEntry, path: &Path) -> Result<(), WinError> {
    // The type comes from the listing itself.
    let kind = entry
        .file_type()
        .map_err(|e| io_error("FindNextFileW", &e))?;
    let (call, removed) = if kind.is_dir() {
        ("remove_dir_all", fs::remove_dir_all(path))
    } else if kind.is_symlink() {
        // A link to a folder, a junction too, goes with `RemoveDirectoryW`, which removes the link and never what it
        // points to; a link to a file needs `DeleteFileW`.
        let removed = fs::remove_dir(path).or_else(|_| fs::remove_file(path));
        ("DeleteFileW", removed)
    } else {
        ("DeleteFileW", fs::remove_file(path))
    };
    match removed {
        // Something else removed it first, which is what the step wants.
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_error(call, &e)),
        _ => Ok(()),
    }
}

/// A std error without an OS code gets `u32::MAX`, which no Win32 error uses.
fn io_error(call: &'static str, e: &io::Error) -> WinError {
    WinError {
        call,
        code: e.raw_os_error().map_or(u32::MAX, |code| code as u32),
    }
}

#[cfg(test)]
mod tests;
