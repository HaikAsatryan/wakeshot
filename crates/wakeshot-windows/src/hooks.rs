//! What each Velopack hook does, in order and within its time, over trait `HookEnv`.
//!
//! Owner: `p6-hooks`.
//!
//! Registry first, restore last (`06-ship.md`, "Hooks"): Velopack kills a hook that runs past its limit, and a restore
//! that hangs must not leave a Run value or a Path entry that points into a deleted folder. Each failure is a log line
//! and the hook goes on: hooks never panic and never show UI (`research/distribution.md`, section 1).
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
use wakeshot_core::ownership::restore::Found;
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::JournalOwner;
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, ERROR_NO_UNICODE_TRANSLATION};

use crate::WinError;
use crate::install::{PathChange, UserPath};
use crate::log::{Level, Log, Proc};
use crate::power::journal::{FileNet, NoGuard, RUN_ONCE_NAME, restore_locked};
use crate::power::plan::Win32Power;
use crate::process::{Liveness, SystemLiveness};
use crate::recovery::open_log;
use crate::store::{AutoStart, DataDir, DataFile, Names, StartKey};

/// The start-at-login value's name under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (architecture section
/// 6, "Registry values").
pub const RUN_NAME: &str = "Wakeshot";

/// Restore tries go on this long from the start of the install hook; Velopack kills it at 30 s.
pub const INSTALL_LIMIT_MS: u64 = 25_000;

/// The same for the updated hook, which Velopack kills at 15 s: the hook runs a just-extracted exe, which took up to
/// 1.7 s to start, so 12 s would leave about 1.3 s (`p6-velopack-facts`, Result).
pub const UPDATED_LIMIT_MS: u64 = 10_000;

/// The same for the uninstall hook: Velopack's docs say 30 s (60 s measured), and the data folder step comes after.
pub const UNINSTALL_LIMIT_MS: u64 = 25_000;

/// A failed restore is tried again this often within a hook's limit.
pub const RETRY_EVERY_MS: u64 = 2_000;

const AREA: &str = "hook";

/// `log.rs` keeps the name private; `real_delete_all_takes_the_open_log_too` checks the two agree.
const LOG_FILE: &str = "wakeshot.log";

/// What a hook needs from the PC: the real one is [`RealHookEnv`].
pub trait HookEnv {
    /// Writes the Run value: the quoted tray exe. `Ok`: the command written.
    fn set_run(&mut self) -> Result<String, WinError>;
    /// `true`: there was one.
    fn remove_run(&mut self) -> Result<bool, WinError>;
    /// The tray exe's folder in the user Path, where `wakeshot.exe` sits beside it.
    fn add_path(&mut self) -> Result<PathChange, WinError>;
    fn remove_path(&mut self) -> Result<PathChange, WinError>;
    /// `true`: there was one.
    fn remove_run_once(&mut self) -> Result<bool, WinError>;
    /// One restore attempt, if a journal exists and its owner is Gone.
    fn try_restore(&mut self) -> Attempt;
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

/// The uninstall's data folder step (Question 2 of `p6-plan-ship`: the default, Haik 2026-11-03 18:09).
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

/// `--veloapp-install` (also Setup.exe over an install, which is what `winget upgrade` runs): the Run value and the
/// Path entry, then the restore.
pub fn on_install(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Install, version);
    set_run(env);
    add_path(env);
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

/// `--veloapp-updated`, on the new version: the Path entry (no change while `current` keeps its path), then the
/// restore. The Run value is left as it is, so a user who removed it keeps it removed.
pub fn on_updated(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Updated, version);
    add_path(env);
    restore(env, Hook::Updated, started);
    end(env, Hook::Updated, started);
}

/// `--veloapp-uninstall`: the Run value, the RunOnce value and the Path entry go first, since the exe they name is
/// being removed, then the restore, then the data folder step. The tray icons' `NotifyIconSettings` entries stay:
/// Windows wrote them, not Wakeshot (`05-tray-ux.md`; change it when Haik wants a fully clean uninstall: remove the
/// three entries by GUID here, after the restore).
pub fn on_uninstall(env: &mut impl HookEnv, version: &str) {
    let started = begin(env, Hook::Uninstall, version);
    remove_run(env);
    remove_run_once(env);
    remove_path(env);
    let step = match restore(env, Hook::Uninstall, started) {
        Attempt::Restored | Attempt::Nothing => DataFolder::DeleteAll,
        _ => DataFolder::KeepJournalAndLogs,
    };
    clean(env, step);
    end(env, Hook::Uninstall, started);
}

/// The start line, which also proves `VelopackApp` ran the hook (`p6-velopack-facts`: `vpk` does not check it).
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

fn remove_run_once(env: &mut impl HookEnv) {
    let removed = env.remove_run_once();
    removed_line(env, removed, &format!("RunOnce value {RUN_ONCE_NAME}"));
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
            let next = match hook {
                Hook::Uninstall => "after a reinstall, the tray sets it aside and warns",
                Hook::Install | Hook::Updated => "the tray sets it aside and warns at its next start",
            };
            let text = format!("no restore: journal.json does not parse and stays as it is; {next}");
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
    pub names: Names,
    /// The tray exe the Run value starts. Its folder is the Path entry: `wakeshot.exe` sits beside it.
    pub exe: PathBuf,
}

/// The real `HookEnv`: `store::AutoStart` for the Run and RunOnce values (their one writer, review m12), `install`
/// for the Path entry, `power::journal::restore_locked` with `FileNet::restore_only` for one restore attempt, the
/// same single call the guard and restore mode make.
pub struct RealHookEnv<P> {
    /// An `Err` fails only the steps that need it, so the registry steps still run.
    dir: Result<DataDir, WinError>,
    exe: Result<PathBuf, WinError>,
    auto: AutoStart,
    path: UserPath,
    names: Names,
    plans: P,
    net: Result<FileNet<NoGuard>, WinError>,
    liveness: Box<dyn Liveness>,
    log: Arc<dyn Log>,
    epoch: Instant,
}

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
            names: Names::real(),
            plans: Win32Power::new(log.clone()),
            net,
            liveness: Box::new(SystemLiveness),
            log,
            epoch: Instant::now(),
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
            names: places.names,
            plans,
            net: Ok(net),
            liveness,
            log,
            epoch: Instant::now(),
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

    fn add_path(&mut self) -> Result<PathChange, WinError> {
        self.path.add(self.cli_folder()?)
    }

    fn remove_path(&mut self) -> Result<PathChange, WinError> {
        self.path.remove(self.cli_folder()?)
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
    let log_file = logs.join(LOG_FILE);
    remove_children(&logs, &[OsStr::new(LOG_FILE)], &mut undeleted);
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
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::os::windows::fs::OpenOptionsExt;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
    use wakeshot_core::ownership::fake::{Call as PlanCall, FakePlans, SetBy};
    use wakeshot_core::ownership::journal::JournalParts;
    use wakeshot_core::process::FileTime;
    use wakeshot_core::settings::{Guid, Journal, JournalEntry, PlanValues, SettingKey};

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

    /// One call to the fake, in order. `TryRestore` holds the fake clock when it started.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        SetRun,
        RemoveRun,
        AddPath,
        RemovePath,
        RemoveRunOnce,
        TryRestore(u64),
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

        fn add_path(&mut self) -> Result<PathChange, WinError> {
            let change = self.path_change();
            self.step(Call::AddPath, change)
        }

        fn remove_path(&mut self) -> Result<PathChange, WinError> {
            let change = self.path_change();
            self.step(Call::RemovePath, change)
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
        assert_eq!(order(&env), [Call::SetRun, Call::AddPath, TRY]);
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
        assert_eq!(order(&env), [Call::AddPath, TRY]);
        assert_eq!(env.count("updated hook for version 0.1.1 started"), 1);

        let mut env = FakeEnv::new(&[Attempt::Restored]);
        on_uninstall(&mut env, "0.1.1");
        let want = [
            Call::RemoveRun,
            Call::RemoveRunOnce,
            Call::RemovePath,
            TRY,
            Call::Clean(DataFolder::DeleteAll),
        ];
        assert_eq!(order(&env), want);
        assert_eq!(env.count("data folder deleted"), 1);
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
        assert_eq!(order(&env)[2..], tries);
        assert_eq!(env.tries(), [0, 2_000, 4_000]);
        assert_eq!(env.count("next try in 2 s"), 2);
        assert_eq!(env.count("the journal's values are back"), 1);

        let mut env = FakeEnv::new(&attempts);
        on_updated(&mut env, "0.1.1");
        assert_eq!(order(&env)[1..], tries);

        let mut env = FakeEnv::new(&attempts);
        on_uninstall(&mut env, "0.1.1");
        let mut want = vec![Call::RemoveRun, Call::RemoveRunOnce, Call::RemovePath];
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
        assert_eq!(order(&env)[..2], [Call::SetRun, Call::AddPath]);
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

        // Uninstall: Run, RunOnce and Path go first anyway; the journal and the logs stay.
        let mut env = FakeEnv::new(&[]);
        on_uninstall(&mut env, "0.1.1");
        assert_eq!(env.tries(), every_2s(13));
        let calls = order(&env);
        assert_eq!(
            calls[..3],
            [Call::RemoveRun, Call::RemoveRunOnce, Call::RemovePath]
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
        assert_eq!(order(&env), [Call::SetRun, Call::AddPath, TRY]);

        let mut env = FakeEnv::timed(&hang);
        on_updated(&mut env, "0.1.1");
        assert_eq!(order(&env), [Call::AddPath, TRY]);

        let mut env = FakeEnv::timed(&hang);
        on_uninstall(&mut env, "0.1.1");
        let want = [
            Call::RemoveRun,
            Call::RemoveRunOnce,
            Call::RemovePath,
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
                [Call::SetRun, Call::AddPath, TRY],
                "{answer:?}"
            );
            assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");

            let mut env = FakeEnv::new(&[answer]);
            on_updated(&mut env, "0.1.1");
            assert_eq!(order(&env), [Call::AddPath, TRY], "{answer:?}");
            assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");

            let mut env = FakeEnv::new(&[answer]);
            on_uninstall(&mut env, "0.1.1");
            assert_eq!(
                order(&env)[3..],
                [TRY, Call::Clean(DataFolder::KeepJournalAndLogs)],
                "{answer:?}"
            );
            assert_eq!(env.count("pid 9120, start 5"), 1, "{answer:?}");
        }
    }

    #[test]
    fn unrestorable_journal_is_tried_once_and_kept() {
        for why in [Unrestorable::DoesNotParse, Unrestorable::NewerVersion] {
            let mut env = FakeEnv::new(&[Attempt::Unrestorable(why)]);
            on_uninstall(&mut env, "0.1.1");
            assert_eq!(
                order(&env)[3..],
                [TRY, Call::Clean(DataFolder::KeepJournalAndLogs)],
                "{why:?}"
            );
            assert_eq!(env.count("no restore"), 1, "{why:?}");
        }
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
        assert_eq!(order(&env), [Call::SetRun, Call::AddPath, TRY]);
        assert_eq!(env.at_level(Level::Error), 2, "{:?}", env.lines());

        let mut env = FakeEnv::new(&[Attempt::Restored]);
        env.registry_error = Some(DENIED);
        on_uninstall(&mut env, "0.1.1");
        assert_eq!(order(&env).len(), 5);
        assert_eq!(env.at_level(Level::Error), 3, "{:?}", env.lines());
        assert_eq!(env.count("RegSetValueExW failed with 5"), 3);

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
        // Two steps of 5 s: tries at 10, 12, ... 24 s.
        let mut env = FakeEnv::new(&[]);
        env.step_ms = 5_000;
        on_install(&mut env, "0.1.0");
        let want: Vec<u64> = (5..13).map(|i| i * RETRY_EVERY_MS).collect();
        assert_eq!(env.tries(), want);

        // A 9 s broadcast leaves the updated hook one try.
        let mut env = FakeEnv::new(&[]);
        env.step_ms = 9_000;
        on_updated(&mut env, "0.1.1");
        assert_eq!(env.tries(), [9_000]);

        // Past the limit already: still one try.
        let mut env = FakeEnv::new(&[]);
        env.step_ms = 12_000;
        on_updated(&mut env, "0.1.1");
        assert_eq!(env.tries(), [12_000]);
        assert!(!env.calls.contains(&WAIT));
    }

    // `RealHookEnv` against a temp data folder, a scratch registry key and `FakePlans`.

    /// Balanced, the active plan.
    const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

    fn haiks() -> PlanValues {
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

    /// Haik's values with our writes for `entries()` stored and in force, as a hold leaves them.
    fn applied() -> FakePlans {
        let fake = FakePlans::new(haiks());
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
            let dir = std::env::temp_dir()
                .join(format!("wakeshot-hooks-{tag}-{}-{n}", std::process::id()));
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
        log: Arc<Tee>,
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
            let log = Arc::new(Tee {
                file: Some(LogWriter::open(&dir.logs(), Proc::Hook).unwrap()),
                memory: MemoryLog::default(),
            });
            Self {
                _temp: temp,
                scratch,
                dir,
                exe,
                auto,
                path,
                log,
            }
        }

        fn env(&self, plans: FakePlans, liveness: Box<dyn Liveness>) -> RealHookEnv<FakePlans> {
            let places = Places {
                dir: self.dir.clone(),
                auto: self.auto.clone(),
                path: self.path.clone(),
                names: Names::test(self.scratch.path().rsplit('\\').next().unwrap_or("hooks")),
                exe: self.exe.clone(),
            };
            RealHookEnv::new(places, plans, liveness, self.log.clone())
        }

        fn folder(&self) -> &Path {
            self.exe.parent().unwrap()
        }

        fn save_journal(&self, file: &JournalFile) {
            let mut net =
                FileNet::restore_only(self.dir.clone(), self.auto.clone(), self.log.clone());
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
        let other = Path::new(r"C:\Other\bin");
        rig.path.add(other).unwrap();
        rig.path.add(rig.folder()).unwrap();
        rig.save_journal(&journal(dead_owner(), vec![]));
        fs::write(rig.dir.file(DataFile::Config), b"{\"version\":1}").unwrap();
        fs::create_dir_all(rig.dir.holds()).unwrap();
        fs::write(rig.dir.file(DataFile::KeepAwake), b"{}").unwrap();
        let fake = applied();

        let mut env = rig.env(fake.clone(), Box::new(SystemLiveness));
        on_uninstall(&mut env, "0.1.1");

        assert_originals(&fake);
        assert_eq!(rig.auto.get(StartKey::Run, RUN_NAME), Ok(None));
        assert_eq!(rig.auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_eq!(
            rig.auto.get(StartKey::Run, "Other"),
            Ok(Some("other.exe".to_owned()))
        );
        assert_eq!(rig.path.remove(rig.folder()), Ok(PathChange::Unchanged));
        assert_eq!(rig.path.add(other), Ok(PathChange::Unchanged));
        assert!(!rig.dir.root().exists(), "the data folder is gone");
        assert!(rig.log.has("restore: the journal's values are back"));
        assert!(rig.log.has("data folder deleted"));
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
            key: JournalKey::Other("sleepAc".into()),
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
        // The name this module deletes last is the one `LogWriter` writes.
        assert!(rig.dir.logs().join(LOG_FILE).is_file());

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
        assert!(rig.dir.logs().join(LOG_FILE).is_file(), "the log stays");
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
        // The steps that need neither still run.
        assert_eq!(env.remove_run_once(), Ok(true));
        assert_eq!(env.remove_run(), Ok(false));
    }
}
