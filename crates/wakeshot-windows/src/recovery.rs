//! Guard mode, restore mode and the tray's guard control (architecture section 5: "Restore routine" step 7, "Guard",
//! "Logon restore").
//!
//! Owner: `p3-recovery`. The tray exe's mode files (`--guard`, `--restore`) build the real parts and call
//! [`run_guard`] and [`run_restore_mode`]; the tray's `FileNet` takes a [`TrayGuard`]. Both modes restore only through
//! `power::journal::restore_locked`.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wakeshot_core::command::Failure;
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::restore::Found;
use wakeshot_core::process::{Alive, FileTime, ProcessRef};
use wakeshot_core::settings::JournalOwner;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HANDLE, SetLastError,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::Threading::{
    CREATE_EVENT_MANUAL_RESET, CreateEventExW, EVENT_ALL_ACCESS, GetExitCodeProcess, INFINITE,
    OpenEventW, SYNCHRONIZATION_SYNCHRONIZE, SetEvent, WaitForMultipleObjects, WaitForSingleObject,
};
use windows::core::HSTRING;

use crate::WinError;
use crate::log::{Level, Log, LogWriter, Proc};
use crate::power::journal::{GuardControl, restore_locked};
use crate::process::{self, Liveness, ProcessHandle};
use crate::store::{DataDir, Names};

/// A failed restore is tried again this often (architecture section 5, restore step 7).
pub const RETRY_EVERY_MS: u64 = 10_000;

/// Tries go on this long after the first; then the tray is started (Phase 1 Question 12, Haik 2026-11-03 18:09).
pub const RETRY_FOR_MS: u64 = 120_000;

/// Every guard's stop event name starts with this; `<tray pid>.<n>` follows.
pub const STOP_EVENT_PREFIX: &str = r"Local\Wakeshot.GuardStop.";

/// The exit code of a mode that could not restore: it gave up, or found no data folder.
pub const EXIT_FAILED: u8 = 1;

/// The exit code of a command line the mode does not take, the same as the tray's `main` gives one no mode takes.
pub const EXIT_BAD_ARGS: u8 = 2;

const AREA: &str = "recovery";

/// How often the guard looks at a tray whose handle it could not get.
const POLL_MS: u32 = 1_000;

/// Stop event names `TrayGuard` tries before it reports the start as failed.
const NAME_TRIES: u32 = 8;

/// The guard's command line after `--guard`: the tray's pid and start time, and the guard's own stop event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardArgs {
    pub tray: ProcessRef,
    pub stop_event: String,
}

impl GuardArgs {
    /// `<pid> <start> <stop event>`: all three, nothing more. `Err` says what is wrong, for the log.
    pub fn parse(args: &[OsString]) -> Result<Self, String> {
        let [pid, start, event] = args else {
            return Err(format!(
                "expected 3 arguments (pid, start time, stop event), got {}",
                args.len()
            ));
        };
        let pid = decimal(pid)
            .and_then(|pid| u32::try_from(pid).ok())
            .filter(|&pid| pid != 0)
            .ok_or("the pid is not a decimal number from 1 to 4294967295")?;
        let start = decimal(start).ok_or("the start time is not a decimal FILETIME")?;
        let stop_event = event
            .to_str()
            .filter(|name| name.len() > STOP_EVENT_PREFIX.len())
            .filter(|name| name.starts_with(STOP_EVENT_PREFIX))
            .ok_or_else(|| {
                format!("the stop event is not named {STOP_EVENT_PREFIX}<tray pid>.<n>")
            })?
            .to_owned();
        Ok(Self {
            tray: ProcessRef {
                pid,
                start: FileTime(start),
            },
            stop_event,
        })
    }

    /// The arguments that start a guard, `--guard` first; `parse` takes back what follows it.
    pub fn command(&self) -> Vec<OsString> {
        vec![
            "--guard".into(),
            self.tray.pid.to_string().into(),
            self.tray.start.0.to_string().into(),
            self.stop_event.clone().into(),
        ]
    }
}

/// Digits only: `u64::from_str` also takes a leading `+`, which no writer of these arguments emits.
fn decimal(arg: &OsStr) -> Option<u64> {
    let text = arg.to_str()?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Why the guard's wait ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    /// The stop event is set: the tray released everything, so nothing is left to guard.
    Stopped,
    /// The tray ended, was gone before the guard looked, or its pid names another process now.
    TrayGone,
}

/// A tray the give-up step started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrayStarted {
    pub pid: u32,
    /// `false`: started as this process's own child, because Explorer could not be its parent.
    pub outside_tree: bool,
}

/// The guard's wait, the retry clock and the tray start: the parts of both modes that block or start a process, so
/// the unit tests run the loops with no wait.
pub trait RecoveryOs {
    /// Returns once the tray in `args` is gone or its stop event is set.
    fn wait_for_tray(&self, args: &GuardArgs, log: &dyn Log) -> Wake;
    /// Milliseconds on a clock that never goes back; only differences count.
    fn now_ms(&self) -> u64;
    fn sleep_ms(&self, ms: u64);
    /// Starts `wakeshot-tray.exe` with no arguments, which is tray mode.
    fn start_tray(&self) -> Result<TrayStarted, WinError>;
}

/// The real waits and the real tray start.
#[derive(Debug)]
pub struct Win32Recovery {
    tray_exe: Result<PathBuf, WinError>,
    clock: Instant,
}

impl Win32Recovery {
    /// The tray is this exe: both modes run as `wakeshot-tray.exe --guard` and `--restore`. A path that cannot be
    /// read fails only the tray start, so the restore itself still runs.
    pub fn this_exe() -> Self {
        let exe = std::env::current_exe().map_err(|e| io_error("GetModuleFileNameW", &e));
        Self {
            tray_exe: exe,
            clock: Instant::now(),
        }
    }

    pub fn new(tray_exe: PathBuf) -> Self {
        Self {
            tray_exe: Ok(tray_exe),
            clock: Instant::now(),
        }
    }
}

impl RecoveryOs for Win32Recovery {
    fn wait_for_tray(&self, args: &GuardArgs, log: &dyn Log) -> Wake {
        wait_for_tray(args, log)
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn start_tray(&self) -> Result<TrayStarted, WinError> {
        let exe = self.tray_exe.as_ref().map_err(|e| *e)?;
        // Under Explorer, as a start from the Start menu would be, so the tray never lives in this process's tree.
        let spawned = process::spawn_outside_tree(exe, &[])?;
        Ok(TrayStarted {
            pid: spawned.handle.process().pid,
            outside_tree: spawned.outside_tree,
        })
    }
}

/// The real wait: on the tray's handle (opened with exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION`, its
/// start time checked) and on the stop event at once.
///
/// The tray comes first in the wait, so a tray that ends right after it set the event still has its journal checked.
/// A tray that cannot be opened is looked at once a second instead.
pub fn wait_for_tray(args: &GuardArgs, log: &dyn Log) -> Wake {
    let pid = args.tray.pid;
    let tray = match ProcessHandle::open(args.tray) {
        Ok(Some(handle)) => Some(handle),
        Ok(None) => {
            let text = format!(
                "tray pid {pid} is not running: it ended before the guard looked, or its pid names another process"
            );
            log.line(Level::Info, AREA, &text);
            return Wake::TrayGone;
        }
        Err(e) => {
            let text = format!(
                "tray pid {pid} cannot be opened ({}): looking at it once a second instead",
                error_text(e)
            );
            log.line(Level::Warn, AREA, &text);
            None
        }
    };
    let event = match StopEvent::open(&args.stop_event) {
        Ok(event) => Some(event),
        Err(e) => {
            // `TrayGuard` keeps the event open until this guard ends, so only a tray on its way out closes it first;
            // waiting on the tray alone still restores when it goes.
            let text = format!(
                "stop event {} cannot be opened ({}): waiting on the tray alone",
                args.stop_event,
                error_text(e)
            );
            log.line(Level::Warn, AREA, &text);
            None
        }
    };
    if let Some(tray) = &tray {
        let mut handles = vec![tray.raw()];
        handles.extend(event.as_ref().map(|event| event.0));
        // SAFETY: both handles stay open until the wait returns, and both have SYNCHRONIZE.
        let woke = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
        if woke == WAIT_OBJECT_0 {
            return Wake::TrayGone;
        }
        if woke.0 == WAIT_OBJECT_0.0 + 1 {
            return Wake::Stopped;
        }
        let e = thread_error("WaitForMultipleObjects");
        let text = format!(
            "wait on tray pid {pid} failed ({}): looking at it once a second instead",
            error_text(e)
        );
        log.line(Level::Warn, AREA, &text);
    }
    poll(args.tray, event.as_ref())
}

/// The fallback wait: the stop event (or a plain pause), then a liveness check, once a second. Unknown is never taken
/// for gone (architecture section 5, "Liveness").
fn poll(tray: ProcessRef, event: Option<&StopEvent>) -> Wake {
    loop {
        match event.map(|event| event.wait(POLL_MS)) {
            Some(Ok(true)) => return Wake::Stopped,
            Some(Ok(false)) => {}
            // No event, or its wait failed: a plain pause, so the loop never spins.
            None | Some(Err(_)) => std::thread::sleep(Duration::from_millis(POLL_MS.into())),
        }
        if process::liveness(tray) == Alive::Gone {
            return Wake::TrayGone;
        }
    }
}

/// A guard's stop event: manual reset, so a set made before the guard opens it still stops it. Closed on drop.
///
/// The default security is enough for the guard, which runs with Explorer's Medium token: a Medium guard opened an
/// event an elevated test made (`docs/research.md`, "Recovery checks"). Change it to a DACL for the user's SID when
/// a guard logs that it cannot open its stop event.
#[derive(Debug)]
pub struct StopEvent(HANDLE);

// SAFETY: an event handle is a kernel handle that any thread of this process may use or close.
unsafe impl Send for StopEvent {}
// SAFETY: every method takes `&self` and only passes the handle to thread-safe kernel calls.
unsafe impl Sync for StopEvent {}

impl StopEvent {
    /// Creates `name`, not set, with the default security. `None`: an event with that name exists already, so it is
    /// not this caller's.
    pub fn create(name: &str) -> Result<Option<Self>, WinError> {
        let name = HSTRING::from(name);
        // SAFETY: the last error is cleared first, so the check below reads what this call set; `name` lives for the
        // call.
        let handle = unsafe {
            SetLastError(ERROR_SUCCESS);
            CreateEventExW(None, &name, CREATE_EVENT_MANUAL_RESET, EVENT_ALL_ACCESS.0)
        }
        .map_err(|e| core_error("CreateEventExW", &e))?;
        let event = Self(handle);
        // SAFETY: plain call, right after the create.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Ok(None);
        }
        Ok(Some(event))
    }

    /// Opens `name` with `SYNCHRONIZE` only: the guard waits on it and never sets it.
    pub fn open(name: &str) -> Result<Self, WinError> {
        let name = HSTRING::from(name);
        // SAFETY: `name` lives for the call; the handle is owned from here on.
        let handle = unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, false, &name) }
            .map_err(|e| core_error("OpenEventW", &e))?;
        Ok(Self(handle))
    }

    pub fn set(&self) -> Result<(), WinError> {
        // SAFETY: the handle is open; a handle from `open` lacks the right, which the call reports.
        unsafe { SetEvent(self.0) }.map_err(|e| core_error("SetEvent", &e))
    }

    pub fn is_set(&self) -> Result<bool, WinError> {
        self.wait(0)
    }

    /// `true`: set within `timeout_ms`.
    pub fn wait(&self, timeout_ms: u32) -> Result<bool, WinError> {
        // SAFETY: the handle is open and has SYNCHRONIZE.
        let waited = unsafe { WaitForSingleObject(self.0, timeout_ms) };
        if waited == WAIT_OBJECT_0 {
            Ok(true)
        } else if waited == WAIT_TIMEOUT {
            Ok(false)
        } else {
            Err(thread_error("WaitForSingleObject"))
        }
    }
}

impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: the handle was opened for this value and is closed once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// One guard process the tray started, with its stop event.
#[derive(Debug)]
struct Guard {
    process: ProcessHandle,
    event: StopEvent,
}

impl Guard {
    /// Ended, or its handle cannot tell. Either way a new guard is started: two guards only restore twice, one after
    /// the other under the journal mutex.
    fn ended(&self) -> bool {
        self.process.wait(0).unwrap_or(true)
    }

    /// Set, or its state cannot be read; as above.
    fn told_to_stop(&self) -> bool {
        self.event.is_set().unwrap_or(true)
    }

    fn pid(&self) -> u32 {
        self.process.process().pid
    }

    fn exit_code(&self) -> Option<u32> {
        let mut code = 0;
        // SAFETY: the handle from CreateProcessW has every right; `code` is a local out value.
        unsafe { GetExitCodeProcess(self.process.raw(), &mut code) }.ok()?;
        Some(code)
    }
}

/// The tray's `GuardControl` (architecture section 5, "Guard"): one guard process at a time, started outside the
/// tray's process tree, each with its own stop event `Local\Wakeshot.GuardStop.<tray pid>.<n>`.
///
/// Callers may rely on: `ensure` starts a guard when none runs or the current one's event is set (it is leaving);
/// `stop` sets the current guard's event and nothing else; a drop stops no guard, so a tray that ends with a journal
/// left is still guarded. `Send`, so the tray may build it on one thread and use it on the IO thread.
pub struct TrayGuard {
    exe: PathBuf,
    tray: ProcessRef,
    log: Arc<dyn Log>,
    /// The `<n>` of the last stop event made.
    made: u64,
    current: Option<Guard>,
    /// Guards told to stop that still run. Each keeps its event open, so a guard that has not opened it yet still
    /// can, and leaves rather than wait for the tray to end.
    leaving: Vec<Guard>,
    /// Arguments before `--guard`; only the tests' child role has any.
    lead: Vec<OsString>,
}

impl TrayGuard {
    /// `exe`: the tray exe (`std::env::current_exe()` in the tray; the harness passes the built one). `tray`: the
    /// process each guard waits on, this process in the tray.
    pub fn new(exe: PathBuf, tray: ProcessRef, log: Arc<dyn Log>) -> Self {
        Self {
            exe,
            tray,
            log,
            made: 0,
            current: None,
            leaving: Vec::new(),
            lead: Vec::new(),
        }
    }

    /// A guard runs and is not told to stop.
    pub fn running(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|guard| !guard.ended() && !guard.told_to_stop())
    }

    /// The pid of the guard started last, for the harness and the log.
    pub fn guard_pid(&self) -> Option<u32> {
        self.current.as_ref().map(Guard::pid)
    }

    fn note(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }

    fn start(&mut self) -> Result<Guard, WinError> {
        let (event, name) = self.new_event()?;
        let args = GuardArgs {
            tray: self.tray,
            stop_event: name,
        };
        let mut line = self.lead.clone();
        line.extend(args.command());
        let spawned = process::spawn_outside_tree(&self.exe, &line)?;
        let guard = Guard {
            process: spawned.handle,
            event,
        };
        let text = format!(
            "guard pid {} started for tray pid {}, stop event {}",
            guard.pid(),
            self.tray.pid,
            args.stop_event
        );
        self.note(Level::Info, &text);
        if !spawned.outside_tree {
            // Track note 9: the log says so, because End task on the tray's group then ends both.
            let text = format!(
                "guard pid {} runs as the tray's own child: Explorer is missing, or its open or the start under it \
                 failed",
                guard.pid()
            );
            self.note(Level::Warn, &text);
        }
        Ok(guard)
    }

    fn new_event(&mut self) -> Result<(StopEvent, String), WinError> {
        // A taken name means a reused pid whose older tray's guard still holds its event: the next `n` is free.
        for _ in 0..NAME_TRIES {
            self.made += 1;
            let name = format!("{STOP_EVENT_PREFIX}{}.{}", self.tray.pid, self.made);
            if let Some(event) = StopEvent::create(&name)? {
                return Ok((event, name));
            }
        }
        Err(WinError {
            call: "CreateEventExW",
            code: ERROR_ALREADY_EXISTS.0,
        })
    }
}

impl GuardControl for TrayGuard {
    fn ensure(&mut self) -> Result<bool, WinError> {
        self.leaving.retain(|guard| !guard.ended());
        if self.running() {
            return Ok(false);
        }
        let new = self.start()?;
        if let Some(old) = self.current.replace(new) {
            if !old.ended() {
                self.leaving.push(old);
            } else if !old.told_to_stop() {
                // Task Manager's End task, or a crash: the case manual check 4 looks for.
                let code = old
                    .exit_code()
                    .map_or_else(|| "unknown".to_owned(), |c| c.to_string());
                let text = format!(
                    "guard pid {} ended without being told to stop (exit code {code}); a new one took over",
                    old.pid()
                );
                self.note(Level::Warn, &text);
            }
        }
        Ok(true)
    }

    fn stop(&mut self) -> Result<(), WinError> {
        self.leaving.retain(|guard| !guard.ended());
        let Some(guard) = &self.current else {
            return Ok(());
        };
        if guard.event.is_set() == Ok(true) {
            return Ok(());
        }
        guard.event.set()?;
        let text = format!("guard pid {} told to stop", guard.pid());
        self.note(Level::Info, &text);
        Ok(())
    }
}

/// How a retry loop ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovered {
    /// No journal, or one with no known entry: nothing to restore.
    Nothing,
    /// The journal's entries are restored and the clean-up ran.
    Restored,
    /// The journal names another owner: not this guard's tray, or a tray that took it over during the tries.
    OtherOwner(JournalOwner),
    /// Restore mode: the journal's owner runs, so it is a tray of this sign-in and restores itself.
    OwnerAlive(JournalOwner),
    /// The journal does not parse. Restore mode moved it to `journal.json.corrupt` (`set_aside`). The guard leaves
    /// it: its owner cannot be read, and the RunOnce value still set brings restore mode at the next sign-in.
    Corrupt { set_aside: bool },
    /// Every try failed for 2 minutes; the tray was started (or its start failed) to keep trying and warn.
    GaveUp(Result<TrayStarted, WinError>),
}

impl Recovered {
    /// 0, or [`EXIT_FAILED`] when the mode gave up.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::GaveUp(_) => EXIT_FAILED,
            _ => 0,
        }
    }
}

/// How the guard ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardEnd {
    /// The stop event was set.
    Stopped,
    /// The tray is gone; what the retry loop did then.
    TrayGone(Recovered),
}

impl GuardEnd {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Stopped => 0,
            Self::TrayGone(recovered) => recovered.exit_code(),
        }
    }
}

/// How restore mode ended: the ends of the same retry loop the guard runs.
pub type RestoreModeEnd = Recovered;

/// What both modes run on: the real parts in the tray exe's mode files, fakes in the tests.
pub struct RecoveryDeps<'a, P, N> {
    pub plans: P,
    /// `FileNet::restore_only` in both modes: its clean-up clears the RunOnce value and never sets it.
    pub net: N,
    pub names: &'a Names,
    pub log: &'a dyn Log,
    /// Restore mode's owner check. The guard needs none: its wait already says the tray is gone.
    pub liveness: &'a dyn Liveness,
    pub os: &'a dyn RecoveryOs,
}

impl<P, N> RecoveryDeps<'_, P, N> {
    fn note(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }
}

/// Guard mode (architecture section 5, "Guard"): waits on the tray and its stop event. When the tray is gone and the
/// journal names it as owner, restores, with the retries of restore step 7 and then the give-up step.
pub fn run_guard<P: PlanIo, N: SafetyNet>(
    args: &GuardArgs,
    deps: &mut RecoveryDeps<'_, P, N>,
) -> GuardEnd {
    let tray = args.tray;
    let text = format!(
        "guard started for tray pid {} (start {}), stop event {}",
        tray.pid, tray.start, args.stop_event
    );
    deps.note(Level::Info, &text);
    let end = match deps.os.wait_for_tray(args, deps.log) {
        Wake::Stopped => GuardEnd::Stopped,
        Wake::TrayGone => {
            let text = format!("tray pid {} is gone; looking at the journal", tray.pid);
            deps.note(Level::Info, &text);
            GuardEnd::TrayGone(retry(deps, Wanted::Tray(tray)))
        }
    };
    let (level, what) = match end {
        GuardEnd::Stopped => (Level::Info, "stopped".to_owned()),
        GuardEnd::TrayGone(recovered) => (level_of(recovered), describe(recovered)),
    };
    let text = format!("guard for tray pid {}: {what}", tray.pid);
    deps.note(level, &text);
    end
}

/// Restore mode (architecture section 5, "Logon restore"): at sign-in, restores a journal whose owner is gone, with
/// the retries of restore step 7 and then the give-up step. It never sets the RunOnce value: Windows deleted it
/// before it ran this command, and only the clean-up of a restore touches the key.
pub fn run_restore_mode<P: PlanIo, N: SafetyNet>(
    deps: &mut RecoveryDeps<'_, P, N>,
) -> RestoreModeEnd {
    deps.note(Level::Info, "restore mode started");
    let end = retry(deps, Wanted::DeadOwner);
    let text = format!("restore mode: {}", describe(end));
    deps.note(level_of(end), &text);
    end
}

/// The give-up step, the one place both modes end a restore that keeps failing (Phase 1 Question 12: the default,
/// Haik 2026-11-03 18:09): start the tray, which retries on each safety tick and shows the warning.
pub fn give_up(os: &dyn RecoveryOs, log: &dyn Log) -> Result<TrayStarted, WinError> {
    let started = os.start_tray();
    match started {
        Ok(tray) => {
            let how = match tray.outside_tree {
                true => "",
                false => " as this process's own child",
            };
            let text = format!(
                "restore still fails after {} s: started the tray (pid {}){how} to keep trying and warn",
                RETRY_FOR_MS / 1000,
                tray.pid
            );
            log.line(Level::Warn, AREA, &text);
        }
        Err(e) => {
            let text = format!(
                "restore still fails after {} s, and the tray did not start ({}): the values stay as they are until \
                 Wakeshot next starts",
                RETRY_FOR_MS / 1000,
                error_text(e)
            );
            log.line(Level::Error, AREA, &text);
        }
    }
    started
}

/// The log of both modes: `logs\wakeshot.log` in the data folder, or none when it cannot be opened, since a mode
/// that cannot log must still restore.
pub fn open_log(dir: &DataDir, proc: Proc) -> Arc<dyn Log> {
    match LogWriter::open(&dir.logs(), proc) {
        Ok(writer) => Arc::new(writer),
        Err(_) => Arc::new(NoLog),
    }
}

struct NoLog;

impl Log for NoLog {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

/// Whose journal a retry loop restores.
#[derive(Clone, Copy)]
enum Wanted {
    /// The guard's: the journal must name this tray, which is known to be gone.
    Tray(ProcessRef),
    /// Restore mode's: the owner the first read names, while it is gone.
    DeadOwner,
}

/// Why one try failed; the loop tries again.
#[derive(Clone, Copy)]
enum TryFailed {
    Restore(Failure),
    /// Unknown never starts a restore (architecture section 5, "Liveness").
    OwnerUnknown(JournalOwner),
}

/// Restore step 7: a try at once, then one every 10 s while 2 minutes are not over, then the give-up step. Each try
/// reloads the journal and goes on only while its owner is the one the loop started for.
fn retry<P: PlanIo, N: SafetyNet>(deps: &mut RecoveryDeps<'_, P, N>, wanted: Wanted) -> Recovered {
    let deadline = deps.os.now_ms().saturating_add(RETRY_FOR_MS);
    let mut owner = None;
    let mut tries = 0u32;
    loop {
        tries = tries.saturating_add(1);
        let failed = match try_once(deps, wanted, &mut owner) {
            Ok(end) => return end,
            Err(failed) => failed,
        };
        let why = match failed {
            TryFailed::Restore(f) => {
                format!("the restore failed: {} failed with {}", f.call, f.code)
            }
            TryFailed::OwnerUnknown(o) => format!(
                "the journal owner's state (pid {} start {}) is unknown",
                o.pid, o.proc_start.0
            ),
        };
        if deps.os.now_ms().saturating_add(RETRY_EVERY_MS) > deadline {
            let text = format!("try {tries} failed, the last: {why}");
            deps.note(Level::Warn, &text);
            return Recovered::GaveUp(give_up(deps.os, deps.log));
        }
        let text = format!(
            "try {tries} failed: {why}; next try in {} s",
            RETRY_EVERY_MS / 1000
        );
        deps.note(Level::Warn, &text);
        deps.os.sleep_ms(RETRY_EVERY_MS);
    }
}

/// One try. `owner`: the owner the loop started for, set by the first read that names one.
fn try_once<P: PlanIo, N: SafetyNet>(
    deps: &mut RecoveryDeps<'_, P, N>,
    wanted: Wanted,
    owner: &mut Option<JournalOwner>,
) -> Result<Recovered, TryFailed> {
    // A look without the lock, to learn the owner: every write of the journal is atomic, so the file read is whole.
    // The routine reads it again under the lock and checks the owner once more.
    let file = match deps.net.load_journal() {
        JournalRead::Missing => return Ok(Recovered::Nothing),
        JournalRead::Failed(f) => return Err(TryFailed::Restore(f)),
        JournalRead::Corrupt => {
            return match wanted {
                Wanted::Tray(_) => Ok(Recovered::Corrupt { set_aside: false }),
                // Windows already deleted the RunOnce value, so no later sign-in would set it aside.
                Wanted::DeadOwner => restore(deps, None),
            };
        }
        JournalRead::Read(file) => file,
    };
    let found = file.owner;
    if let Wanted::Tray(tray) = wanted
        && (found.pid, found.proc_start) != (tray.pid, tray.start)
    {
        return Ok(Recovered::OtherOwner(found));
    }
    if owner.is_some_and(|first| first != found) {
        return Ok(Recovered::OtherOwner(found));
    }
    *owner = Some(found);
    if let Wanted::DeadOwner = wanted {
        let process = ProcessRef {
            pid: found.pid,
            start: found.proc_start,
        };
        match deps.liveness.check(process) {
            Alive::Yes => return Ok(Recovered::OwnerAlive(found)),
            Alive::Unknown => return Err(TryFailed::OwnerUnknown(found)),
            Alive::Gone => {}
        }
    }
    restore(deps, Some(found))
}

fn restore<P: PlanIo, N: SafetyNet>(
    deps: &mut RecoveryDeps<'_, P, N>,
    expect_owner: Option<JournalOwner>,
) -> Result<Recovered, TryFailed> {
    let done = restore_locked(
        &mut deps.plans,
        &mut deps.net,
        None,
        expect_owner,
        deps.names,
        deps.log,
    );
    match (done.found, done.result) {
        (Found::OtherOwner(other), _) => Ok(Recovered::OtherOwner(other)),
        (_, Err(f)) => Err(TryFailed::Restore(f)),
        (Found::Nothing, Ok(())) => Ok(Recovered::Nothing),
        (Found::Entries, Ok(())) => Ok(Recovered::Restored),
        (Found::SetAside, Ok(())) => Ok(Recovered::Corrupt { set_aside: true }),
    }
}

fn describe(recovered: Recovered) -> String {
    match recovered {
        Recovered::Nothing => "no journal, nothing to restore".to_owned(),
        Recovered::Restored => "restored the journal's entries".to_owned(),
        Recovered::OtherOwner(o) => format!(
            "the journal belongs to another tray (pid {} start {}), left alone",
            o.pid, o.proc_start.0
        ),
        Recovered::OwnerAlive(o) => format!(
            "the journal's owner (pid {} start {}) runs and restores it itself",
            o.pid, o.proc_start.0
        ),
        Recovered::Corrupt { set_aside: true } => {
            "the journal did not parse and was moved to journal.json.corrupt".to_owned()
        }
        Recovered::Corrupt { set_aside: false } => {
            "the journal does not parse; left for the next sign-in or tray start".to_owned()
        }
        Recovered::GaveUp(Ok(tray)) => format!(
            "gave up after {} s and started the tray (pid {})",
            RETRY_FOR_MS / 1000,
            tray.pid
        ),
        Recovered::GaveUp(Err(e)) => format!(
            "gave up after {} s; the tray did not start ({})",
            RETRY_FOR_MS / 1000,
            error_text(e)
        ),
    }
}

fn level_of(recovered: Recovered) -> Level {
    match recovered {
        Recovered::GaveUp(Ok(_)) | Recovered::Corrupt { .. } => Level::Warn,
        Recovered::GaveUp(Err(_)) => Level::Error,
        _ => Level::Info,
    }
}

fn error_text(e: WinError) -> String {
    format!("{} failed with {}", e.call, e.code)
}

/// The Win32 code when the HRESULT wraps one, as `store` and `log` report codes.
fn core_error(call: &'static str, e: &windows::core::Error) -> WinError {
    let hresult = e.code().0 as u32;
    let code = match hresult & 0xFFFF_0000 == 0x8007_0000 {
        true => hresult & 0xFFFF,
        false => hresult,
    };
    WinError { call, code }
}

fn thread_error(call: &'static str) -> WinError {
    core_error(call, &windows::core::Error::from_thread())
}

/// A std error without an OS code gets `u32::MAX`, which no Win32 error uses.
fn io_error(call: &'static str, e: &std::io::Error) -> WinError {
    WinError {
        call,
        code: e.raw_os_error().map_or(u32::MAX, |code| code as u32),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::windows::ffi::OsStringExt;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    use std::thread;

    use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
    use wakeshot_core::ownership::fake::{Action, Call, FakeGuard, FakeJournal, FakePlans, SetBy};
    use wakeshot_core::ownership::journal::JournalParts;
    use wakeshot_core::ownership::restore::Restored;
    use wakeshot_core::settings::{Guid, Journal, JournalEntry, OwnedKey, PlanValues, SettingKey};
    use windows::Win32::System::Threading::TerminateProcess;

    use super::*;
    use crate::log::MemoryLog;
    use crate::power::journal::{FileNet, GuardCall, RUN_ONCE_NAME, RecordingGuard};
    use crate::process::FakeLiveness;
    use crate::store::{AutoStart, ScratchKey, StartKey};

    use SettingKey::{LidAc, LidDc, SleepDc};

    /// Balanced, the active plan at the start.
    const P1: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    /// High performance, inactive at the start.
    const P2: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
    const KEYS: [SettingKey; 3] = [LidAc, LidDc, SleepDc];
    const TRAY: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(5),
        session: 1,
    };
    const NEW_TRAY: JournalOwner = JournalOwner {
        pid: 4242,
        proc_start: FileTime(7),
        session: 1,
    };
    /// The code a scripted failure returns.
    const FAILED: u32 = 31;
    const STARTED: TrayStarted = TrayStarted {
        pid: 4321,
        outside_tree: true,
    };

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

    fn original(key: SettingKey) -> u32 {
        if key == SleepDc { 600 } else { 1 }
    }

    fn entry(plan: Guid, key: SettingKey, restored: bool) -> JournalEntry {
        JournalEntry {
            plan,
            key,
            original: original(key),
            written: 0,
            restored,
        }
    }

    /// How an entry's value stands when the run starts.
    #[derive(Clone, Copy)]
    enum Live {
        /// Our value, stored and (on the active plan) in force: an apply finished.
        Ours,
        /// `original` stored again, ours still in force: a restore stopped or failed before `set_active`.
        Unapplied,
    }

    type Rows = Vec<(JournalEntry, Live)>;

    fn all_ours() -> Rows {
        [
            entry(P1, LidAc, false),
            entry(P1, LidDc, false),
            entry(P1, SleepDc, false),
            entry(P2, LidDc, false),
            entry(P2, SleepDc, false),
        ]
        .map(|e| (e, Live::Ours))
        .into()
    }

    /// As earlier failed runs leave them.
    fn mixed() -> Rows {
        vec![
            (entry(P1, LidAc, true), Live::Unapplied),
            (entry(P1, LidDc, true), Live::Ours),
            (entry(P1, SleepDc, false), Live::Ours),
            (entry(P2, LidDc, false), Live::Ours),
            (entry(P2, SleepDc, true), Live::Unapplied),
        ]
    }

    /// An entry a newer version wrote (correction 12): it stays in the file untouched.
    fn sleep_ac() -> JournalFileEntry {
        JournalFileEntry {
            plan: P1,
            key: JournalKey::Other("sleepAc".into()),
            original: 10_800,
            written: 0,
            restored: false,
        }
    }

    fn file(
        owner: JournalOwner,
        entries: Vec<JournalEntry>,
        other: Vec<JournalFileEntry>,
    ) -> JournalFile {
        JournalParts {
            known: Journal { owner, entries },
            other,
        }
        .merge()
    }

    /// Two plans with user values, P1 active, and what an apply by `owner` left: the values in `rows` and a journal
    /// with their entries and `other`; the RunOnce value and the guard as given.
    fn world(
        owner: JournalOwner,
        rows: &[(JournalEntry, Live)],
        other: Vec<JournalFileEntry>,
        run_once: bool,
        guard: FakeGuard,
    ) -> FakePlans {
        let fake = FakePlans::new(user_values(P1));
        fake.add_plan(user_values(P2));
        let mut s = fake.state();
        for &(e, live) in rows {
            let active = e.plan == s.active;
            match live {
                Live::Ours => {
                    s.set_stored(e.plan, e.key, e.written, SetBy::Override);
                    if active {
                        s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
                    }
                }
                Live::Unapplied => {
                    s.set_stored(e.plan, e.key, e.original, SetBy::Restore);
                    if active {
                        s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
                    }
                }
            }
        }
        s.journal = FakeJournal::File(file(owner, rows.iter().map(|r| r.0).collect(), other));
        s.run_once = run_once;
        s.guard = guard;
        drop(s);
        fake
    }

    /// What a tray that died during a hold leaves for its guard: its RunOnce value, and this guard running.
    fn guard_world(
        owner: JournalOwner,
        rows: &[(JournalEntry, Live)],
        other: Vec<JournalFileEntry>,
    ) -> FakePlans {
        world(owner, rows, other, true, FakeGuard::Running)
    }

    /// What restore mode finds at sign-in: Windows deleted the RunOnce value before it ran, and no guard runs.
    fn sign_in_world(
        owner: JournalOwner,
        rows: &[(JournalEntry, Live)],
        other: Vec<JournalFileEntry>,
    ) -> FakePlans {
        world(owner, rows, other, false, FakeGuard::None)
    }

    fn tray_of(owner: JournalOwner) -> ProcessRef {
        ProcessRef {
            pid: owner.pid,
            start: owner.proc_start,
        }
    }

    fn guard_args(tray: ProcessRef) -> GuardArgs {
        GuardArgs {
            tray,
            stop_event: format!("{STOP_EVENT_PREFIX}{}.1", tray.pid),
        }
    }

    /// A stop event name no `TrayGuard` of this test exe makes.
    fn test_event_name() -> String {
        static N: AtomicU64 = AtomicU64::new(1_000_000);
        let n = N.fetch_add(1, Ordering::Relaxed);
        format!("{STOP_EVENT_PREFIX}{}.{n}", std::process::id())
    }

    fn calls(fake: &FakePlans) -> Vec<Call> {
        fake.state().calls.clone()
    }

    fn count(calls: &[Call], call: &Call) -> usize {
        calls.iter().filter(|c| *c == call).count()
    }

    /// What every stop and failure keeps: each value we still hold has a known entry, and while one exists the
    /// RunOnce value is set and the guard runs.
    fn assert_reachable(fake: &FakePlans, case: &str) {
        let s = fake.state();
        assert_eq!(s.held_without_entry(), Vec::<OwnedKey>::new(), "{case}");
        if !s.known_entries().is_empty() {
            assert!(s.run_once, "{case}: a known entry without RunOnce");
            assert_eq!(s.guard, FakeGuard::Running, "{case}");
        }
    }

    /// Restore mode's form of the same: Windows deleted the RunOnce value before it ran, so only the journal keeps the
    /// values reachable, and restore mode never sets the value again.
    fn assert_reachable_at_sign_in(fake: &FakePlans, case: &str) {
        let s = fake.state();
        assert_eq!(s.held_without_entry(), Vec::<OwnedKey>::new(), "{case}");
        assert!(!s.run_once, "{case}: restore mode set RunOnce");
        assert!(!s.calls.contains(&Call::SetRunOnce), "{case}");
    }

    /// The user's values are back on every plan, stored and in force, and no known entry is left.
    fn assert_values_back(fake: &FakePlans, case: &str) {
        let s = fake.state();
        for &plan in s.plans.keys() {
            for key in KEYS {
                assert_eq!(
                    s.stored(plan, key),
                    Some(original(key)),
                    "{case}: {plan:?} {key:?}"
                );
                if plan == s.active {
                    let in_force = s.in_force(plan, key);
                    assert_eq!(in_force, Some(original(key)), "{case}: {key:?} in force");
                }
            }
        }
        assert_eq!(s.held(), Vec::<OwnedKey>::new(), "{case}");
        let left = s.known_entries();
        assert!(left.is_empty(), "{case}: {left:?}");
    }

    /// As `assert_values_back`, and the safety net is released: no file, no RunOnce value, the guard told to stop.
    fn assert_restored(fake: &FakePlans) {
        assert_values_back(fake, "restored");
        let s = fake.state();
        assert_eq!(s.journal, FakeJournal::Missing);
        assert!(!s.run_once);
        assert_eq!(s.guard, FakeGuard::Stopping);
    }

    /// The tests' `RecoveryOs`: a scripted or the real wait, a clock that moves only when slept on, a tray start that
    /// only counts.
    #[derive(Default)]
    struct TestOs {
        /// `None`: the real wait.
        wake: Option<Wake>,
        now: AtomicU64,
        sleeps: AtomicU32,
        starts: AtomicU32,
        start_error: Option<WinError>,
        /// Runs after each sleep with the number of sleeps so far: a change between two tries.
        on_sleep: Option<Box<dyn Fn(u32) + Send + Sync>>,
    }

    impl TestOs {
        fn waking(wake: Wake) -> Self {
            Self {
                wake: Some(wake),
                ..Self::default()
            }
        }

        fn sleeps(&self) -> u32 {
            self.sleeps.load(Ordering::Relaxed)
        }

        fn starts(&self) -> u32 {
            self.starts.load(Ordering::Relaxed)
        }
    }

    impl RecoveryOs for TestOs {
        fn wait_for_tray(&self, args: &GuardArgs, log: &dyn Log) -> Wake {
            self.wake.unwrap_or_else(|| wait_for_tray(args, log))
        }

        fn now_ms(&self) -> u64 {
            self.now.load(Ordering::Relaxed)
        }

        fn sleep_ms(&self, ms: u64) {
            self.now.fetch_add(ms, Ordering::Relaxed);
            let n = self.sleeps.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(change) = &self.on_sleep {
                change(n);
            }
        }

        fn start_tray(&self) -> Result<TrayStarted, WinError> {
            self.starts.fetch_add(1, Ordering::Relaxed);
            match self.start_error {
                Some(e) => Err(e),
                None => Ok(STARTED),
            }
        }
    }

    /// `FakePlans` whose `active_plan` fails, logging no call, while `fails` is above zero. Every try of the routine
    /// on a journal with known entries reads the active plan first, so `fails` = n fails the first n tries.
    struct Flaky {
        inner: FakePlans,
        fails: u32,
    }

    impl PlanIo for Flaky {
        fn active_plan(&mut self) -> Result<Guid, Failure> {
            if self.fails > 0 {
                self.fails -= 1;
                return Err(Failure {
                    call: "Flaky::active_plan",
                    code: FAILED,
                });
            }
            self.inner.active_plan()
        }

        fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
            self.inner.plans()
        }

        fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
            self.inner.can_write(key)
        }

        fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
            self.inner.read(plan, key)
        }

        fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
            self.inner.write(plan, key, value)
        }

        fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
            self.inner.set_active(plan)
        }

        fn values(&mut self) -> Result<PlanValues, Failure> {
            self.inner.values()
        }
    }

    /// One run of either mode, with its log, liveness answers and `TestOs`.
    struct Run {
        names: Names,
        log: MemoryLog,
        liveness: Arc<FakeLiveness>,
        os: TestOs,
    }

    impl Run {
        fn new(os: TestOs) -> Self {
            Self::with(os, Arc::default())
        }

        fn with(os: TestOs, liveness: Arc<FakeLiveness>) -> Self {
            Self {
                names: Names::test("recovery"),
                log: MemoryLog::default(),
                liveness,
                os,
            }
        }

        fn deps<P, N>(&self, plans: P, net: N) -> RecoveryDeps<'_, P, N> {
            RecoveryDeps {
                plans,
                net,
                names: &self.names,
                log: &self.log,
                liveness: &*self.liveness,
                os: &self.os,
            }
        }

        fn guard(&self, fake: &FakePlans, args: &GuardArgs) -> GuardEnd {
            run_guard(args, &mut self.deps(fake.clone(), fake.clone()))
        }

        fn guard_flaky(&self, fake: &FakePlans, fails: u32, args: &GuardArgs) -> GuardEnd {
            let plans = Flaky {
                inner: fake.clone(),
                fails,
            };
            run_guard(args, &mut self.deps(plans, fake.clone()))
        }

        fn restore_mode(&self, fake: &FakePlans) -> Recovered {
            run_restore_mode(&mut self.deps(fake.clone(), fake.clone()))
        }

        fn restore_mode_flaky(&self, fake: &FakePlans, fails: u32) -> Recovered {
            let plans = Flaky {
                inner: fake.clone(),
                fails,
            };
            run_restore_mode(&mut self.deps(plans, fake.clone()))
        }

        fn has_line(&self, level: Level, part: &str) -> bool {
            let lines = self.log.lines.lock().unwrap();
            lines
                .iter()
                .any(|(l, _, text)| *l == level && text.contains(part))
        }
    }

    /// The next start's restore: the tray's, at startup, for a journal whose owner is gone.
    fn next_start(fake: &FakePlans) -> Restored {
        let names = Names::test("next-start");
        restore_locked(
            &mut fake.clone(),
            &mut fake.clone(),
            None,
            None,
            &names,
            &MemoryLog::default(),
        )
    }

    #[test]
    fn guard_args_parse() {
        let os = |list: &[&str]| -> Vec<OsString> { list.iter().map(OsString::from).collect() };
        let event = r"Local\Wakeshot.GuardStop.9120.3";
        let good = GuardArgs::parse(&os(&["9120", "134381211391959336", event])).unwrap();
        let want = GuardArgs {
            tray: ProcessRef {
                pid: 9120,
                start: FileTime(134_381_211_391_959_336),
            },
            stop_event: event.to_owned(),
        };
        assert_eq!(good, want);
        let command = good.command();
        assert_eq!(command[0], "--guard");
        assert_eq!(GuardArgs::parse(&command[1..]), Ok(want));

        let bad: [&[&str]; 15] = [
            &[],
            &["1", "2"],
            &["1", "2", event, "x"],
            &["", "2", event],
            &["+1", "2", event],
            &["-1", "2", event],
            &["0", "2", event],
            &["4294967296", "2", event],
            &["1 ", "2", event],
            &["1", "", event],
            &["1", "2.5", event],
            &["1", "18446744073709551616", event],
            &["1", "2", r"Local\Other.1.1"],
            &["1", "2", STOP_EVENT_PREFIX],
            &["1", "2", r"Global\Wakeshot.GuardStop.1.1"],
        ];
        for args in bad {
            assert!(GuardArgs::parse(&os(args)).is_err(), "{args:?}");
        }
        // Not Unicode: a lone surrogate.
        let odd = OsString::from_wide(&[0x4C, 0xD800]);
        assert!(GuardArgs::parse(&["1".into(), "2".into(), odd]).is_err());
    }

    #[test]
    fn guard_exits_on_stop_event() {
        let me = process::current().unwrap();
        let owner = JournalOwner {
            pid: me.pid,
            proc_start: me.start,
            session: 1,
        };
        // A journal this "tray" owns: a guard that took the stop for a tray end would restore it.
        let fake = guard_world(owner, &all_ours(), vec![]);
        let name = test_event_name();
        let event = StopEvent::create(&name).unwrap().expect("a new name");
        let args = GuardArgs {
            tray: me,
            stop_event: name,
        };
        let run = Run::new(TestOs::default());
        let setter = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            event.set().unwrap();
            event
        });
        let started = Instant::now();
        assert_eq!(run.guard(&fake, &args), GuardEnd::Stopped);
        let waited = started.elapsed();
        let event = setter.join().unwrap();
        assert!(waited >= Duration::from_millis(150), "{waited:?}");
        assert!(waited < Duration::from_secs(5), "{waited:?}");
        assert!(calls(&fake).is_empty());
        assert!(run.has_line(Level::Info, "stopped"));

        // Manual reset: an event set before the guard opens it stops it too.
        assert_eq!(event.is_set(), Ok(true));
        let run = Run::new(TestOs::default());
        assert_eq!(run.guard(&fake, &args), GuardEnd::Stopped);
        assert!(calls(&fake).is_empty());
        assert_eq!(fake.state().known_entries().len(), 5);
    }

    #[test]
    fn guard_restores_when_tray_dies() {
        let mut kid = Kid::sleeping();
        let tray = kid.process();
        let owner = JournalOwner {
            pid: tray.pid,
            proc_start: tray.start,
            session: 1,
        };
        let fake = guard_world(owner, &all_ours(), vec![]);
        let name = test_event_name();
        let _event = StopEvent::create(&name).unwrap().expect("a new name");
        let args = GuardArgs {
            tray,
            stop_event: name,
        };
        let run = Run::new(TestOs::default());
        let ender = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            kid.end();
        });
        let end = run.guard(&fake, &args);
        ender.join().unwrap();
        assert_eq!(end, GuardEnd::TrayGone(Recovered::Restored));
        assert_eq!(end.exit_code(), 0);
        assert_restored(&fake);
        assert_eq!((run.os.sleeps(), run.os.starts()), (0, 0));

        // A tray already gone when the guard looks: the same restore, at once.
        let fake = guard_world(owner, &mixed(), vec![sleep_ac()]);
        let run = Run::new(TestOs::default());
        assert_eq!(
            run.guard(&fake, &args),
            GuardEnd::TrayGone(Recovered::Restored)
        );
        assert_values_back(&fake, "already gone");
        assert!(run.has_line(Level::Info, "is not running"));
    }

    #[test]
    fn guard_leaves_journal_of_another_owner() {
        let fake = guard_world(NEW_TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::waking(Wake::TrayGone));
        let end = run.guard(&fake, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::OtherOwner(NEW_TRAY)));
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        let s = fake.state();
        assert_eq!(s.known_entries().len(), 5);
        assert_eq!(s.held().len(), 5);
        assert!(s.run_once);
        assert_eq!(s.guard, FakeGuard::Running);
        drop(s);
        assert!(run.has_line(Level::Info, "another tray"));
    }

    #[test]
    fn guard_stops_retrying_when_another_tray_takes_over() {
        let fake = guard_world(TRAY, &all_ours(), vec![]);
        let journal = fake.clone();
        let os = TestOs {
            wake: Some(Wake::TrayGone),
            on_sleep: Some(Box::new(move |_| {
                if let FakeJournal::File(file) = &mut journal.state().journal {
                    file.owner = NEW_TRAY;
                }
            })),
            ..TestOs::default()
        };
        let run = Run::new(os);
        let end = run.guard_flaky(&fake, 1, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::OtherOwner(NEW_TRAY)));
        assert_eq!((run.os.sleeps(), run.os.starts()), (1, 0));
        assert_eq!(fake.state().known_entries().len(), 5);
    }

    #[test]
    fn guard_retries_then_gives_up_and_starts_tray() {
        let fake = guard_world(TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::waking(Wake::TrayGone));
        let end = run.guard_flaky(&fake, u32::MAX, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::GaveUp(Ok(STARTED))));
        assert_eq!(end.exit_code(), EXIT_FAILED);
        // A try at 0 s and one every 10 s up to 120 s: 13 tries, 12 pauses, then the tray start.
        assert_eq!(run.os.sleeps(), 12);
        assert_eq!(run.os.now_ms(), RETRY_FOR_MS);
        assert_eq!(run.os.starts(), 1);
        let c = calls(&fake);
        assert_eq!((c.len(), count(&c, &Call::LoadJournal)), (26, 26));
        assert_reachable(&fake, "gave up");
        assert_eq!(fake.state().known_entries().len(), 5);
        assert!(run.has_line(Level::Warn, "try 12 failed"));
        assert!(run.has_line(Level::Warn, "try 13 failed, the last"));
        assert!(run.has_line(Level::Warn, "started the tray (pid 4321)"));

        // The tray start fails too: the end says so, at Error.
        let fake = guard_world(TRAY, &all_ours(), vec![]);
        let error = WinError {
            call: "CreateProcessW",
            code: 2,
        };
        let os = TestOs {
            wake: Some(Wake::TrayGone),
            start_error: Some(error),
            ..TestOs::default()
        };
        let run = Run::new(os);
        let end = run.guard_flaky(&fake, u32::MAX, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::GaveUp(Err(error))));
        assert_eq!(end.exit_code(), EXIT_FAILED);
        assert!(run.has_line(Level::Error, "the tray did not start"));
    }

    #[test]
    fn guard_retry_finishes_a_failed_try() {
        let fake = guard_world(TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::waking(Wake::TrayGone));
        let end = run.guard_flaky(&fake, 2, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::Restored));
        assert_eq!((run.os.sleeps(), run.os.starts()), (2, 0));
        assert_restored(&fake);
    }

    #[test]
    fn guard_leaves_a_corrupt_journal() {
        let fake = guard_world(TRAY, &all_ours(), vec![]);
        fake.state().journal = FakeJournal::Corrupt;
        let run = Run::new(TestOs::waking(Wake::TrayGone));
        let end = run.guard(&fake, &guard_args(tray_of(TRAY)));
        assert_eq!(
            end,
            GuardEnd::TrayGone(Recovered::Corrupt { set_aside: false })
        );
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        assert_eq!(fake.state().set_aside, 0);
    }

    #[test]
    fn guard_with_no_journal_does_nothing() {
        let fake = FakePlans::new(user_values(P1));
        let run = Run::new(TestOs::waking(Wake::TrayGone));
        let end = run.guard(&fake, &guard_args(tray_of(TRAY)));
        assert_eq!(end, GuardEnd::TrayGone(Recovered::Nothing));
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        assert!(run.has_line(Level::Info, "no journal"));
    }

    #[test]
    fn restore_mode_missing_journal() {
        let fake = FakePlans::new(user_values(P1));
        let run = Run::new(TestOs::default());
        let end = run.restore_mode(&fake);
        assert_eq!(end, Recovered::Nothing);
        assert_eq!(end.exit_code(), 0);
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        assert!(run.liveness.checked.lock().unwrap().is_empty());
        assert_eq!((run.os.sleeps(), run.os.starts()), (0, 0));
        assert!(run.has_line(Level::Info, "restore mode: no journal"));
    }

    #[test]
    fn restore_mode_live_owner_exits() {
        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::default());
        run.liveness
            .alive
            .lock()
            .unwrap()
            .push((tray_of(TRAY), Alive::Yes));
        assert_eq!(run.restore_mode(&fake), Recovered::OwnerAlive(TRAY));
        assert_eq!(calls(&fake), [Call::LoadJournal]);
        assert_eq!(fake.state().held().len(), 5);
        assert_eq!((run.os.sleeps(), run.os.starts()), (0, 0));
    }

    #[test]
    fn restore_mode_unknown_owner_retries() {
        // Unknown at the first try, Gone from the second.
        let liveness = Arc::new(FakeLiveness::default());
        liveness
            .alive
            .lock()
            .unwrap()
            .push((tray_of(TRAY), Alive::Unknown));
        let changed = liveness.clone();
        let os = TestOs {
            on_sleep: Some(Box::new(move |_| changed.alive.lock().unwrap().clear())),
            ..TestOs::default()
        };
        let run = Run::with(os, liveness);
        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        assert_eq!(run.restore_mode(&fake), Recovered::Restored);
        assert_eq!(run.os.sleeps(), 1);
        // The first try only looked: Unknown starts no restore.
        let c = calls(&fake);
        assert_eq!(
            c[..3],
            [Call::LoadJournal, Call::LoadJournal, Call::LoadJournal]
        );
        assert_eq!(run.liveness.checked.lock().unwrap().len(), 2);
        assert_values_back(&fake, "after Unknown");
        assert!(run.has_line(Level::Warn, "is unknown"));

        // Unknown for the whole 2 minutes: no plan call at all, then the tray starts and keeps trying.
        let liveness = Arc::new(FakeLiveness::default());
        liveness
            .alive
            .lock()
            .unwrap()
            .push((tray_of(TRAY), Alive::Unknown));
        let run = Run::with(TestOs::default(), liveness);
        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        assert_eq!(run.restore_mode(&fake), Recovered::GaveUp(Ok(STARTED)));
        let c = calls(&fake);
        assert_eq!((c.len(), count(&c, &Call::LoadJournal)), (13, 13));
        assert_eq!(fake.state().held().len(), 5);
        assert_eq!(run.os.starts(), 1);
    }

    #[test]
    fn restore_mode_never_writes_run_once() {
        let check = |fake: &FakePlans, case: &str| {
            let s = fake.state();
            assert!(
                !s.calls.contains(&Call::SetRunOnce),
                "{case}: {:?}",
                s.calls
            );
            assert!(!s.run_once, "{case}");
        };

        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        assert_eq!(
            Run::new(TestOs::default()).restore_mode(&fake),
            Recovered::Restored
        );
        check(&fake, "restored");

        let fake = sign_in_world(TRAY, &mixed(), vec![sleep_ac()]);
        assert_eq!(
            Run::new(TestOs::default()).restore_mode(&fake),
            Recovered::Restored
        );
        check(&fake, "restored, an unknown entry kept");
        assert!(fake.state().known_entries().is_empty());

        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::default());
        assert_eq!(run.restore_mode_flaky(&fake, 3), Recovered::Restored);
        check(&fake, "restored after retries");

        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        let run = Run::new(TestOs::default());
        assert_eq!(
            run.restore_mode_flaky(&fake, u32::MAX),
            Recovered::GaveUp(Ok(STARTED))
        );
        check(&fake, "gave up");

        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        let journal = fake.clone();
        let os = TestOs {
            on_sleep: Some(Box::new(move |_| {
                if let FakeJournal::File(file) = &mut journal.state().journal {
                    file.owner = NEW_TRAY;
                }
            })),
            ..TestOs::default()
        };
        let run = Run::new(os);
        assert_eq!(
            run.restore_mode_flaky(&fake, 1),
            Recovered::OtherOwner(NEW_TRAY)
        );
        check(&fake, "taken over");

        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        fake.state().journal = FakeJournal::Corrupt;
        assert_eq!(
            Run::new(TestOs::default()).restore_mode(&fake),
            Recovered::Corrupt { set_aside: true }
        );
        check(&fake, "set aside");
        assert_eq!(fake.state().set_aside, 1);
    }

    #[test]
    fn restore_mode_sets_a_corrupt_journal_aside() {
        let fake = sign_in_world(TRAY, &all_ours(), vec![]);
        fake.state().journal = FakeJournal::Corrupt;
        let run = Run::new(TestOs::default());
        assert_eq!(
            run.restore_mode(&fake),
            Recovered::Corrupt { set_aside: true }
        );
        assert_eq!(
            calls(&fake),
            [Call::LoadJournal, Call::LoadJournal, Call::SetAsideJournal]
        );
        assert!(run.liveness.checked.lock().unwrap().is_empty());
        assert!(run.has_line(Level::Warn, "journal.json.corrupt"));
    }

    /// Track note 4 for one run: a stop at each of its calls, then the next start's restore, first with one of that
    /// restore's calls failing (each in turn), then whole. `fresh` builds the world with the run's own scripted
    /// failures; `after_stop` is the first point of the note.
    fn walk_stops(
        case: &str,
        fresh: &dyn Fn() -> FakePlans,
        run: &dyn Fn(&FakePlans),
        after_stop: &dyn Fn(&FakePlans, &str),
    ) {
        let clean = fresh();
        run(&clean);
        let left = clean.state().known_entries();
        assert!(left.is_empty(), "{case}: the run did not restore: {left:?}");
        let clean_calls = calls(&clean);
        let deleted_at = clean_calls.iter().position(|c| *c == Call::DeleteJournal);
        for (stop, call) in clean_calls.iter().enumerate() {
            let case = format!("{case}, stop at call {stop} ({call:?})");
            let stopped = || {
                let fake = fresh();
                fake.script(stop, Action::Stop);
                let ran = catch_unwind(AssertUnwindSafe(|| run(&fake)));
                assert!(ran.is_err(), "{case}: the run did not reach the call");
                // The run's own scripted failures died with it; the next start meets only its own.
                fake.state().script.clear();
                fake
            };
            let fake = stopped();
            after_stop(&fake, &case);
            let base = calls(&fake).len();

            let next = next_start(&fake);
            assert_eq!(next.result, Ok(()), "{case}");
            assert!(next.journal.is_empty(), "{case}: {:?}", next.journal);
            assert_values_back(&fake, &case);
            let s = fake.state();
            if deleted_at.is_none_or(|d| stop <= d) {
                assert!(!s.run_once, "{case}: RunOnce left");
                assert_ne!(
                    s.guard,
                    FakeGuard::Running,
                    "{case}: guard not told to stop"
                );
            } else {
                // A stop after the file is gone (walk R6): RunOnce or the guard may be left, and both find no
                // journal, so they do nothing.
                assert_eq!(s.journal, FakeJournal::Missing, "{case}");
            }
            let next_calls = s.calls.len() - base;
            drop(s);

            for fail in 0..next_calls {
                let fake = stopped();
                fake.script(base + fail, Action::Fail(FAILED));
                next_start(&fake);
                after_stop(
                    &fake,
                    &format!("{case}, the next restore failing at its call {fail}"),
                );
            }
        }
    }

    /// The index of the first plan write in a clean run of `run` on `fresh`'s world, for a try that fails halfway.
    fn first_write(fresh: &dyn Fn() -> FakePlans, run: &dyn Fn(&FakePlans)) -> usize {
        let clean = fresh();
        run(&clean);
        calls(&clean)
            .iter()
            .position(|c| matches!(c, Call::Write(..)))
            .expect("a write")
    }

    #[test]
    fn guard_stop_points() {
        let run = |fake: &FakePlans| {
            let run = Run::new(TestOs::waking(Wake::TrayGone));
            run.guard(fake, &guard_args(tray_of(TRAY)));
        };
        let starts: [(&str, Rows, Vec<JournalFileEntry>); 2] = [
            ("all ours", all_ours(), vec![]),
            ("mixed, an unknown entry", mixed(), vec![sleep_ac()]),
        ];
        for (name, rows, other) in starts {
            let fresh = || guard_world(TRAY, &rows, other.clone());
            walk_stops(name, &fresh, &run, &assert_reachable);

            // The first try fails at its first write, so the stops also fall in the retry.
            let fail_at = first_write(&fresh, &run);
            let failing = || {
                let fake = fresh();
                fake.script(fail_at, Action::Fail(FAILED));
                fake
            };
            walk_stops(
                &format!("{name}, a retry"),
                &failing,
                &run,
                &assert_reachable,
            );
        }
    }

    #[test]
    fn restore_mode_stop_points() {
        let run = |fake: &FakePlans| {
            Run::new(TestOs::default()).restore_mode(fake);
        };
        let starts: [(&str, Rows, Vec<JournalFileEntry>); 2] = [
            ("all ours", all_ours(), vec![]),
            ("mixed, an unknown entry", mixed(), vec![sleep_ac()]),
        ];
        for (name, rows, other) in starts {
            let fresh = || sign_in_world(TRAY, &rows, other.clone());
            walk_stops(name, &fresh, &run, &assert_reachable_at_sign_in);

            let fail_at = first_write(&fresh, &run);
            let failing = || {
                let fake = fresh();
                fake.script(fail_at, Action::Fail(FAILED));
                fake
            };
            let case = format!("{name}, a retry");
            walk_stops(&case, &failing, &run, &assert_reachable_at_sign_in);
        }
    }

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "wakeshot-recovery-{tag}-{}-{n}",
                std::process::id()
            ));
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

    /// One active plan with our values for `entries` stored and in force. The fake's own journal stays missing: the
    /// journal is `FileNet`'s here.
    fn applied(entries: &[JournalEntry]) -> FakePlans {
        let fake = FakePlans::new(user_values(P1));
        let mut s = fake.state();
        for e in entries {
            s.set_stored(e.plan, e.key, e.written, SetBy::Override);
            s.set_in_force(e.plan, e.key, e.written, SetBy::Override);
        }
        drop(s);
        fake
    }

    /// Stored and in force equal the originals; `held()` cannot tell here, since the fake has no journal to label the
    /// writes by.
    fn assert_originals(fake: &FakePlans, case: &str) {
        let s = fake.state();
        for key in KEYS {
            assert_eq!(s.stored(P1, key), Some(original(key)), "{case}: {key:?}");
            assert_eq!(
                s.in_force(P1, key),
                Some(original(key)),
                "{case}: {key:?} in force"
            );
        }
    }

    #[test]
    fn restore_locked_round_trip() {
        let dir = TempDir::new("round-trip");
        let journal = dir.data().file(crate::store::DataFile::Journal);
        let key = ScratchKey::new("recovery::tests::restore_locked_round_trip");
        let auto = AutoStart::at(key.path());
        let log = Arc::new(MemoryLog::default());
        let names = Names::test("round-trip");
        let entries = vec![entry(P1, LidDc, false), entry(P1, SleepDc, false)];

        // The tray's form: the RunOnce value and the journal as an apply leaves them.
        let fake = applied(&entries);
        let exe = Path::new(
            r"C:\Users\Test User\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe",
        );
        let mut net = FileNet::tray(
            dir.data(),
            auto.clone(),
            exe,
            RecordingGuard::default(),
            log.clone(),
        );
        assert_eq!(net.set_run_once(), Ok(true));
        net.save_journal(&file(TRAY, entries.clone(), vec![]))
            .unwrap();
        assert!(journal.exists());
        let done = restore_locked(&mut fake.clone(), &mut net, None, None, &names, &*log);
        let restored = Restored {
            found: Found::Entries,
            result: Ok(()),
            journal: vec![],
        };
        assert_eq!(done, restored);
        assert!(!journal.exists());
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_eq!(net.guard().calls, [GuardCall::Stop]);
        assert_originals(&fake, "tray form");

        // The guard's and restore mode's form: the value the tray set is cleared, and never set by this form.
        let fake = applied(&entries);
        auto.set(StartKey::RunOnce, RUN_ONCE_NAME, r#""x.exe" --restore"#)
            .unwrap();
        let mut net = FileNet::restore_only(dir.data(), auto.clone(), log.clone());
        net.save_journal(&file(TRAY, entries, vec![])).unwrap();
        let done = restore_locked(&mut fake.clone(), &mut net, None, Some(TRAY), &names, &*log);
        assert_eq!(done, restored);
        assert!(!journal.exists());
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_originals(&fake, "restore-only form");
    }

    #[test]
    fn open_log_without_a_folder_still_logs_nowhere() {
        let dir = TempDir::new("no-log");
        // A file where the logs folder should be: the writer cannot open, and the mode must still run.
        fs::write(dir.0.join("logs"), b"not a folder").unwrap();
        let log = open_log(&dir.data(), Proc::Guard);
        log.line(Level::Info, AREA, "dropped");
        assert!(dir.0.join("logs").is_file());
    }

    #[test]
    fn stop_event_create_open_set() {
        let name = test_event_name();
        let made = StopEvent::create(&name).unwrap().expect("a new name");
        assert_eq!(StopEvent::create(&name).unwrap().map(|_| ()), None);
        let opened = StopEvent::open(&name).unwrap();
        assert_eq!(opened.is_set(), Ok(false));
        // The guard's handle has SYNCHRONIZE only: it cannot set its own stop.
        assert_eq!(opened.set().map_err(|e| e.code), Err(5));
        made.set().unwrap();
        assert_eq!(opened.wait(1_000), Ok(true));
        drop((made, opened));
        // Gone with its last handle.
        assert_eq!(
            StopEvent::open(&name).map(|_| ()).map_err(|e| e.code),
            Err(2)
        );
    }

    #[test]
    fn tray_guard_is_send() {
        fn send<T: Send>() {}
        send::<TrayGuard>();
        send::<FileNet<TrayGuard>>();
    }

    const ROLE: &str = "recovery::tests::child_role";
    const SLEEP_ARG: &str = "wakeshot-recovery-sleep-ms=";
    /// A sleeping child that outlives its test (a crashed run) ends on its own after this.
    const SLEEP_MAX_MS: u64 = 120_000;
    const ROLE_STOPPED: u32 = 10;
    const ROLE_TRAY_GONE: u32 = 11;
    const ROLE_BAD_ARGS: u32 = 12;
    const ROLE_NO_EVENT: u32 = 13;
    /// How long the guard role lingers after its stop, so a test sees it leaving.
    const ROLE_LINGER_MS: u64 = 500;

    /// The child role. Started with `role_lead()` and then `wakeshot-recovery-sleep-ms=<ms>`, it sleeps; with
    /// `--guard <pid> <start> <event>`, it is a guard that waits as the real one does and exits with its wake as the
    /// code. Started any other way it ends at once.
    #[test]
    #[ignore = "a child role for the other tests of this module"]
    fn child_role() {
        let args: Vec<OsString> = std::env::args_os().collect();
        let sleep = args
            .iter()
            .find_map(|a| a.to_str()?.strip_prefix(SLEEP_ARG)?.parse::<u64>().ok());
        if let Some(ms) = sleep {
            thread::sleep(Duration::from_millis(ms.min(SLEEP_MAX_MS)));
            return;
        }
        let Some(at) = args.iter().position(|a| a == "--guard") else {
            return;
        };
        let Ok(guard) = GuardArgs::parse(&args[at + 1..]) else {
            std::process::exit(ROLE_BAD_ARGS as i32);
        };
        if StopEvent::open(&guard.stop_event).is_err() {
            std::process::exit(ROLE_NO_EVENT as i32);
        }
        let code = match wait_for_tray(&guard, &NoLog) {
            Wake::Stopped => {
                thread::sleep(Duration::from_millis(ROLE_LINGER_MS));
                ROLE_STOPPED
            }
            Wake::TrayGone => ROLE_TRAY_GONE,
        };
        std::process::exit(code as i32);
    }

    /// libtest options, then `--`: every argument after it is a filter, so the role's own arguments reach it whatever
    /// they look like, and only `ROLE` names a test.
    fn role_lead() -> Vec<OsString> {
        [
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--quiet",
            "--",
            ROLE,
        ]
        .map(OsString::from)
        .into()
    }

    fn this_exe() -> PathBuf {
        std::env::current_exe().expect("test exe path")
    }

    /// A sleeping child of this test exe, ended and reaped on drop, so a failed assert leaves no process.
    struct Kid(Child);

    impl Kid {
        fn sleeping() -> Self {
            let mut args = role_lead();
            args.push(format!("{SLEEP_ARG}{SLEEP_MAX_MS}").into());
            let child = Command::new(this_exe())
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start child");
            println!("sleeping child pid {}", child.id());
            Self(child)
        }

        fn process(&self) -> ProcessRef {
            ProcessRef {
                pid: self.0.id(),
                start: process::start_of(&self.0).expect("child start"),
            }
        }

        fn end(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    impl Drop for Kid {
        fn drop(&mut self) {
            self.end();
        }
    }

    /// A `TrayGuard` that starts this test exe's guard role for this process. The drop tells the current guard to
    /// stop; each guard also ends with this process, since it waits on it.
    struct TestGuards(TrayGuard);

    impl TestGuards {
        fn new(log: Arc<dyn Log>) -> Self {
            let mut guard = TrayGuard::new(this_exe(), process::current().unwrap(), log);
            guard.lead = role_lead();
            Self(guard)
        }

        fn current(&self) -> &Guard {
            self.0.current.as_ref().expect("a guard")
        }
    }

    impl Drop for TestGuards {
        fn drop(&mut self) {
            let _ = self.0.stop();
        }
    }

    fn lines_with(log: &MemoryLog, part: &str) -> usize {
        let lines = log.lines.lock().unwrap();
        lines
            .iter()
            .filter(|(_, _, text)| text.contains(part))
            .count()
    }

    #[test]
    fn tray_guard_stop_sets_event() {
        let log = Arc::new(MemoryLog::default());
        let mut guards = TestGuards::new(log.clone());
        assert_eq!(guards.0.stop(), Ok(()), "no guard yet: nothing to stop");
        assert_eq!(guards.0.ensure(), Ok(true));
        let guard = guards.current();
        let pid = guard.pid();
        println!("guard pid {pid}");
        assert_eq!(guards.0.guard_pid(), Some(pid));
        assert!(guards.0.running());
        // The child opened the tray and its event and waits: still running a moment later.
        assert_eq!(
            guard.process.wait(300),
            Ok(false),
            "exit {:?}",
            guard.exit_code()
        );

        guards.0.stop().unwrap();
        let guard = guards.current();
        assert_eq!(guard.event.is_set(), Ok(true));
        assert!(!guards.0.running());
        assert_eq!(
            guard.process.wait(5_000),
            Ok(true),
            "the guard did not leave"
        );
        assert_eq!(guard.exit_code(), Some(ROLE_STOPPED));
        assert_eq!(guards.0.stop(), Ok(()));
        assert_eq!(lines_with(&log, "told to stop"), 1);
        let outside = lines_with(&log, "runs as the tray's own child") == 0;
        println!("guard started outside the tree: {outside}");
    }

    #[test]
    fn tray_guard_starts_new_when_old_is_leaving() {
        let log = Arc::new(MemoryLog::default());
        let mut guards = TestGuards::new(log.clone());
        assert_eq!(guards.0.ensure(), Ok(true));
        let first = guards.current().pid();
        assert_eq!(guards.0.ensure(), Ok(false), "one runs: no second guard");
        assert_eq!(guards.0.guard_pid(), Some(first));

        guards.0.stop().unwrap();
        assert!(!guards.0.running());
        assert_eq!(
            guards.0.ensure(),
            Ok(true),
            "the old one is leaving: a new one"
        );
        let second = guards.current().pid();
        println!("guard pids {first} then {second}");
        assert_ne!(first, second);
        assert!(guards.0.running());
        assert_eq!(guards.current().event.is_set(), Ok(false));
        // The old one lingers after its stop, so it is still on the leaving list, with its event open.
        assert_eq!(guards.0.leaving.len(), 1);
        let old = &guards.0.leaving[0];
        assert_eq!(old.pid(), first);
        assert_eq!(old.event.is_set(), Ok(true));
        assert_eq!(old.process.wait(5_000), Ok(true));
        assert_eq!(old.exit_code(), Some(ROLE_STOPPED));
        assert_eq!(guards.0.ensure(), Ok(false));
        assert!(
            guards.0.leaving.is_empty(),
            "an ended guard leaves the list"
        );

        guards.0.stop().unwrap();
        let new = guards.current();
        assert_eq!(new.process.wait(5_000), Ok(true));
        assert_eq!(new.exit_code(), Some(ROLE_STOPPED));
    }

    #[test]
    fn tray_guard_replaces_a_guard_that_ended() {
        let log = Arc::new(MemoryLog::default());
        let mut guards = TestGuards::new(log.clone());
        assert_eq!(guards.0.ensure(), Ok(true));
        let first = guards.current();
        let pid = first.pid();
        // As Task Manager's End task would: the guard ends without its stop.
        // SAFETY: the handle from CreateProcessW has every right; the process is this test's own child.
        unsafe { TerminateProcess(first.process.raw(), 1) }.unwrap();
        assert_eq!(first.process.wait(5_000), Ok(true));
        assert!(!guards.0.running());
        assert_eq!(guards.0.ensure(), Ok(true));
        assert_ne!(guards.current().pid(), pid);
        assert_eq!(
            lines_with(
                &log,
                &format!("guard pid {pid} ended without being told to stop (exit code 1)")
            ),
            1
        );
        assert!(guards.0.leaving.is_empty());
    }
}
