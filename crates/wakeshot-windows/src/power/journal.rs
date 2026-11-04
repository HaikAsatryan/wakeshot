//! The journal file, the RunOnce value and the guard as the core's `SafetyNet` (`FileNet`, real and dry run), and the
//! locked entry points every caller of the ownership routines goes through.
//!
//! Owner: power track (`p3-journal`). The three signatures below are contract: the IO thread, the guard, restore mode,
//! the CLI (`wakeshot restore`) and the Velopack hooks call them, never `wakeshot_core::ownership` directly.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wakeshot_core::command::{ApplyError, Failure};
use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
use wakeshot_core::ownership::apply::{Applied, apply};
use wakeshot_core::ownership::check::{Checked, check};
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::journal::JournalParts;
use wakeshot_core::ownership::reset::{Found as ResetFound, KeyReset, Reset, reset_lost};
use wakeshot_core::ownership::restore::{Found, Restored, restore};
use wakeshot_core::settings::{Guid, JournalEntry, JournalOwner, OwnedKey, SettingKey};
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_DATA, ERROR_NO_UNICODE_TRANSLATION, ERROR_NOT_SUPPORTED,
    ERROR_PATH_NOT_FOUND, WAIT_TIMEOUT,
};

use crate::WinError;
use crate::log::{Level, Log};
use crate::power::plan::PlanSetting;
use crate::store::{self, AutoStart, DataDir, DataFile, NamedMutex, Names, StartKey};

/// The RunOnce value's name (architecture section 5, "Logon restore"); the uninstall hook removes it by this name.
pub const RUN_ONCE_NAME: &str = "WakeshotRestore";

/// How long a caller waits for `Local\Wakeshot.Journal` (track note 9, signed off by Haik 2026-11-03 18:09).
pub const JOURNAL_WAIT_MS: u32 = 10_000;

const AREA: &str = "journal";

/// A journal mutex not free within the wait. 258 is `WAIT_TIMEOUT`, which is also the Win32 error code of that name.
const LOCK_TIMEOUT: Failure = Failure {
    call: "WaitForSingleObject",
    code: WAIT_TIMEOUT.0,
};

/// `ownership::restore` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back as a failed
/// `result`, and nothing was done; `journal` then holds the file's known entries, read without the lock.
pub fn restore_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    plan: Option<Guid>,
    expect_owner: Option<JournalOwner>,
    names: &Names,
    log: &dyn Log,
) -> Restored {
    let lock = JournalLock {
        names,
        log,
        wait_ms: JOURNAL_WAIT_MS,
    };
    restore_within(plans, net, plan, expect_owner, lock)
}

/// `ownership::apply` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back as
/// `ApplyError::SafetyNet`, and nothing was written; `journal` then holds the file's known entries, read without the
/// lock.
pub fn apply_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    me: JournalOwner,
    plan: Guid,
    writes: &[(SettingKey, u32)],
    names: &Names,
    log: &dyn Log,
) -> Applied {
    let lock = JournalLock {
        names,
        log,
        wait_ms: JOURNAL_WAIT_MS,
    };
    apply_within(plans, net, me, plan, writes, lock)
}

/// `ownership::reset::reset_lost` under `Local\Wakeshot.Journal` (`p2-6-record-lost`), with one log line per key: its
/// old and new value, or why it was left. A lock not taken within the wait comes back as a failed `result`, and nothing
/// was done; `journal` then holds the file's known entries, read without the lock.
pub fn reset_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    damaged: bool,
    names: &Names,
    log: &dyn Log,
) -> Reset {
    let lock = JournalLock {
        names,
        log,
        wait_ms: JOURNAL_WAIT_MS,
    };
    reset_within(plans, net, damaged, lock)
}

/// `ownership::check` under `Local\Wakeshot.Journal`. A lock not taken within the wait comes back in `failures`, and
/// no entry was dropped; `journal` then holds the file's known entries, read without the lock.
pub fn check_locked(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    names: &Names,
    log: &dyn Log,
) -> Checked {
    let lock = JournalLock {
        names,
        log,
        wait_ms: JOURNAL_WAIT_MS,
    };
    check_within(plans, net, lock)
}

fn restore_within(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    plan: Option<Guid>,
    expect_owner: Option<JournalOwner>,
    lock: JournalLock<'_>,
) -> Restored {
    match lock.run("restore", || restore(plans, net, plan, expect_owner)) {
        Ok(done) => {
            log_restored(lock.log, plan, &done);
            done
        }
        Err(failure) => {
            let journal = entries_unlocked(net);
            Restored {
                found: match journal.is_empty() {
                    true => Found::Nothing,
                    false => Found::Entries,
                },
                result: Err(failure),
                journal,
            }
        }
    }
}

fn apply_within(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    me: JournalOwner,
    plan: Guid,
    writes: &[(SettingKey, u32)],
    lock: JournalLock<'_>,
) -> Applied {
    match lock.run("apply", || apply(plans, net, me, plan, writes)) {
        Ok(done) => {
            log_applied(lock.log, plan, &done);
            done
        }
        Err(failure) => Applied {
            written: Vec::new(),
            result: Err(ApplyError::SafetyNet(failure)),
            journal: entries_unlocked(net),
        },
    }
}

fn reset_within(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    damaged: bool,
    lock: JournalLock<'_>,
) -> Reset {
    match lock.run("lost journal reset", || reset_lost(plans, net, damaged)) {
        Ok(done) => {
            log_reset(lock.log, &done);
            done
        }
        Err(failure) => {
            let journal = entries_unlocked(net);
            Reset {
                found: match journal.is_empty() {
                    true => ResetFound::Lost,
                    false => ResetFound::Entries,
                },
                plan: None,
                keys: Vec::new(),
                set_aside: None,
                result: Err(failure),
                journal,
            }
        }
    }
}

fn check_within(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    lock: JournalLock<'_>,
) -> Checked {
    match lock.run("ownership check", || check(plans, net)) {
        Ok(done) => {
            log_checked(lock.log, &done);
            done
        }
        Err(failure) => Checked {
            user_changed: Vec::new(),
            failures: vec![failure],
            journal: entries_unlocked(net),
        },
    }
}

/// The journal mutex as one caller takes it.
#[derive(Clone, Copy)]
struct JournalLock<'a> {
    names: &'a Names,
    log: &'a dyn Log,
    wait_ms: u32,
}

impl JournalLock<'_> {
    /// Runs `routine` while this thread holds the journal mutex. `Err`: the mutex was not taken, and `routine` did
    /// not run. The mutex is taken and released on this thread, as Windows requires of a mutex.
    fn run<T>(self, what: &str, routine: impl FnOnce() -> T) -> Result<T, Failure> {
        let name = &self.names.journal;
        let held = match NamedMutex::open(name).and_then(|mutex| mutex.take(self.wait_ms)) {
            Ok(Some(held)) => held,
            Ok(None) => {
                let text = format!(
                    "{what} failed: {name} was not free within {} ms",
                    self.wait_ms
                );
                self.log.line(Level::Warn, AREA, &text);
                return Err(LOCK_TIMEOUT);
            }
            Err(e) => {
                let text = format!("{what} failed: {name}: {}", error_text(e.into()));
                self.log.line(Level::Warn, AREA, &text);
                return Err(e.into());
            }
        };
        if held.abandoned() {
            // Microsoft grants ownership with `WAIT_ABANDONED`; the journal itself is whole because every write of it
            // is atomic (architecture section 4).
            let text = format!("{name} taken after its holder ended without releasing it");
            self.log.line(Level::Warn, AREA, &text);
        }
        let done = routine();
        drop(held);
        Ok(done)
    }
}

/// The known entries as the file holds them now, for a run that did not get the lock, so the core's mirror keeps
/// what the file holds rather than nothing. A read without the lock sees a whole file: every write of it is atomic.
fn entries_unlocked(net: &mut impl SafetyNet) -> Vec<JournalEntry> {
    match net.load_journal() {
        JournalRead::Read(file) => JournalParts::split(file).known.entries,
        JournalRead::Missing | JournalRead::Corrupt | JournalRead::Failed(_) => Vec::new(),
    }
}

/// Starts and stops the guard process (architecture section 5, "Guard") for `FileNet`, so only the tray ever starts
/// one and tests record the calls instead.
///
/// Owner: power track (`p3-journal`) for the trait; `p3-recovery` implements it for the tray as `TrayGuard`.
pub trait GuardControl {
    /// Starts a guard if none runs or the current one's stop event is set (it is leaving). `true`: this call started
    /// one.
    fn ensure(&mut self) -> Result<bool, WinError>;
    /// Sets the current guard's stop event. No guard is not an error.
    fn stop(&mut self) -> Result<(), WinError>;
}

/// The guard control of every process but the tray: a guard, restore mode, a hook and the CLI have no tray for a
/// guard to wait on, and a dry run never starts one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoGuard;

impl GuardControl for NoGuard {
    fn ensure(&mut self) -> Result<bool, WinError> {
        Ok(false)
    }

    fn stop(&mut self) -> Result<(), WinError> {
        Ok(())
    }
}

/// So a caller that picks `TrayGuard` or `NoGuard` at run time (the tray in dry run) needs one `FileNet` type.
impl<G: GuardControl + ?Sized> GuardControl for Box<G> {
    fn ensure(&mut self) -> Result<bool, WinError> {
        (**self).ensure()
    }

    fn stop(&mut self) -> Result<(), WinError> {
        (**self).stop()
    }
}

/// One call to a `RecordingGuard`.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardCall {
    Ensure,
    Stop,
}

/// A `GuardControl` that starts no process: it records each call and answers like a guard that starts on `ensure`
/// and leaves on `stop`. For the stop-point and recovery tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct RecordingGuard {
    pub calls: Vec<GuardCall>,
    /// A guard runs and is not leaving.
    pub running: bool,
    pub started: u32,
    /// Returned by the next `ensure` instead of doing anything.
    pub fail_ensure: Option<WinError>,
    /// Returned by the next `stop` instead of doing anything.
    pub fail_stop: Option<WinError>,
}

#[cfg(any(test, feature = "test-support"))]
impl GuardControl for RecordingGuard {
    fn ensure(&mut self) -> Result<bool, WinError> {
        self.calls.push(GuardCall::Ensure);
        if let Some(e) = self.fail_ensure.take() {
            return Err(e);
        }
        if self.running {
            return Ok(false);
        }
        self.running = true;
        self.started += 1;
        Ok(true)
    }

    fn stop(&mut self) -> Result<(), WinError> {
        self.calls.push(GuardCall::Stop);
        if let Some(e) = self.fail_stop.take() {
            return Err(e);
        }
        self.running = false;
        Ok(())
    }
}

/// What a `FileNet` may do with the RunOnce value.
enum RunOnce {
    /// The tray: set before the journal, set again before each plan read, apply and restore while the tray owns a known
    /// entry (Windows deletes the value when it runs it; `PowerExecutor`'s `keep_guard`), removed in the clean-up.
    /// `None`: the exe path is not valid Unicode, so no value can name it.
    SetAndClear {
        auto: AutoStart,
        command: Option<String>,
    },
    /// Guard, restore mode, hooks, CLI: removed in the clean-up, never set. Windows deletes the value before restore
    /// mode runs, so restore mode must not write that key (architecture section 5, "Logon restore"); and an apply
    /// through this form fails at its safety net step, so it writes nothing.
    ClearOnly(AutoStart),
    /// Dry run: never touched (architecture section 4, "Tray startup" step 3).
    Off,
}

/// The core's `SafetyNet` over the data folder, the RunOnce value and a `GuardControl`, in the form each process
/// needs: [`FileNet::tray`], [`FileNet::dry_run`], [`FileNet::restore_only`].
///
/// Owner: power track (`p3-journal`). Callers may rely on: the journal is `journal.json` in `dir`, written only
/// through `store::write_atomic`; a dry run never touches the registry or a guard; `restore_only` never sets RunOnce.
/// It takes no lock itself: callers go through `restore_locked`, `apply_locked` and `check_locked`.
pub struct FileNet<G> {
    dir: DataDir,
    run_once: RunOnce,
    guard: G,
    log: Arc<dyn Log>,
    /// The unknown entries this net last logged: the ownership check loads the journal every 10 s during a hold, and
    /// one line per load would bury the rest of the log.
    unknown_logged: Vec<JournalFileEntry>,
}

impl<G: GuardControl> FileNet<G> {
    /// The tray's form: RunOnce `"<tray_exe>" --restore` under `auto` (`AutoStart::real()` in the tray, a test key in
    /// tests) and `guard` (`TrayGuard`).
    pub fn tray(
        dir: DataDir,
        auto: AutoStart,
        tray_exe: &Path,
        guard: G,
        log: Arc<dyn Log>,
    ) -> Self {
        // A Windows path cannot hold a quote, so quoting the whole path is enough for any folder name.
        let command = tray_exe.to_str().map(|exe| format!("\"{exe}\" --restore"));
        Self::with(dir, RunOnce::SetAndClear { auto, command }, guard, log)
    }

    pub fn dir(&self) -> &DataDir {
        &self.dir
    }

    pub fn guard(&self) -> &G {
        &self.guard
    }

    fn with(dir: DataDir, run_once: RunOnce, guard: G, log: Arc<dyn Log>) -> Self {
        Self {
            dir,
            run_once,
            guard,
            log,
            unknown_logged: Vec::new(),
        }
    }

    fn path(&self, file: DataFile) -> PathBuf {
        self.dir.file(file)
    }

    fn note(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }

    /// Correction 12: the entries stay in the file for the version that wrote them; the log is where a user on a
    /// downgraded PC finds which setting stays overridden.
    fn note_unknown(&mut self, file: &JournalFile) {
        let unknown: Vec<JournalFileEntry> = file
            .entries
            .iter()
            .filter(|e| matches!(e.key, JournalKey::Other(_)))
            .cloned()
            .collect();
        if unknown == self.unknown_logged {
            return;
        }
        if !unknown.is_empty() {
            let mut text = format!(
                "journal (version {}) holds {} entries with keys this version does not know, kept as they are:",
                file.version,
                unknown.len()
            );
            for entry in &unknown {
                push_entry(&mut text, entry);
            }
            self.note(Level::Info, &text);
        }
        self.unknown_logged = unknown;
    }
}

impl FileNet<NoGuard> {
    /// `--dry-run`, in any process: `dir` is the dry-run folder (`DataDir::dry_run()`), and the caller passes
    /// `Names::dry_run()` to the locked entry points. No RunOnce value, no guard.
    pub fn dry_run(dir: DataDir, log: Arc<dyn Log>) -> Self {
        Self::with(dir, RunOnce::Off, NoGuard, log)
    }

    /// The guard, restore mode, the Velopack hooks and `wakeshot restore`: RunOnce under `auto` is removed in the
    /// clean-up and never set; no guard.
    pub fn restore_only(dir: DataDir, auto: AutoStart, log: Arc<dyn Log>) -> Self {
        Self::with(dir, RunOnce::ClearOnly(auto), NoGuard, log)
    }
}

impl<G: GuardControl> SafetyNet for FileNet<G> {
    fn load_journal(&mut self) -> JournalRead {
        let bytes = match store::read_whole(&self.path(DataFile::Journal)) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return JournalRead::Missing,
            Err(e) => return JournalRead::Failed(e.into()),
        };
        match parse(&bytes) {
            Ok(file) => {
                self.note_unknown(&file);
                JournalRead::Read(file)
            }
            Err(why) => {
                let text = format!("journal.json does not parse ({} bytes): {why}", bytes.len());
                self.note(Level::Warn, &text);
                JournalRead::Corrupt
            }
        }
    }

    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure> {
        let mut file = journal.clone();
        if file.version != JournalFile::VERSION {
            // Review m2: a newer version reads an older file, never the reverse.
            let text = format!(
                "journal save got version {}; written as version {}",
                file.version,
                JournalFile::VERSION
            );
            self.note(Level::Warn, &text);
            file.version = JournalFile::VERSION;
        }
        // Pretty: the file is small, and a person may have to read it by hand.
        let bytes = serde_json::to_vec_pretty(&file).map_err(|_| Failure {
            call: "serde_json::to_vec_pretty",
            code: ERROR_INVALID_DATA.0,
        })?;
        store::write_atomic(&self.path(DataFile::Journal), &bytes)?;
        let mut text = format!(
            "journal saved: owner pid {} start {} session {};",
            file.owner.pid, file.owner.proc_start.0, file.owner.session
        );
        if file.entries.is_empty() {
            text.push_str(" no entries");
        }
        for entry in &file.entries {
            push_entry(&mut text, entry);
        }
        self.note(Level::Info, &text);
        Ok(())
    }

    fn delete_journal(&mut self) -> Result<(), Failure> {
        if store::remove(&self.path(DataFile::Journal))? {
            self.note(Level::Info, "journal deleted");
        }
        Ok(())
    }

    fn set_aside_journal(&mut self) -> Result<(), Failure> {
        // `std::fs::rename` replaces an older `journal.json.corrupt`, also one a reader has open with delete sharing
        // (research, "Journal seam checks"). A rename in one folder is never a copy, so write-through adds nothing.
        std::fs::rename(
            self.path(DataFile::Journal),
            self.path(DataFile::JournalCorrupt),
        )
        .map_err(|e| Failure::from(io_error("MoveFileExW", &e)))?;
        // The tray's reset puts what looks like Wakeshot's back to Windows' defaults (`p2-6-record-lost`); a guard,
        // restore mode or a hook leaves that to the tray's next start.
        self.note(
            Level::Warn,
            "journal.json moved to journal.json.corrupt; the journal saves earlier in this log name the values it \
             held",
        );
        Ok(())
    }

    fn set_run_once(&mut self) -> Result<bool, Failure> {
        let (auto, command) = match &self.run_once {
            RunOnce::SetAndClear { auto, command } => (auto, command),
            RunOnce::ClearOnly(_) => {
                return Err(Failure {
                    call: "FileNet::set_run_once",
                    code: ERROR_NOT_SUPPORTED.0,
                });
            }
            RunOnce::Off => {
                self.note(Level::Info, "dry run: RunOnce value not set");
                return Ok(false);
            }
        };
        let Some(command) = command else {
            return Err(Failure {
                call: "RegSetValueExW",
                code: ERROR_NO_UNICODE_TRANSLATION.0,
            });
        };
        let before = auto.get(StartKey::RunOnce, RUN_ONCE_NAME);
        if matches!(&before, Ok(Some(value)) if value == command) {
            return Ok(false);
        }
        // A value that does not read (another type) is overwritten: refusing would fail every apply until someone
        // removed it by hand.
        auto.set(StartKey::RunOnce, RUN_ONCE_NAME, command)?;
        let text = format!("RunOnce value {RUN_ONCE_NAME} set to {command}");
        self.note(Level::Info, &text);
        // Only a value known to be missing counts as added, so a failed apply takes back only what it added.
        Ok(matches!(before, Ok(None)))
    }

    fn clear_run_once(&mut self) -> Result<(), Failure> {
        let auto = match &self.run_once {
            RunOnce::SetAndClear { auto, .. } | RunOnce::ClearOnly(auto) => auto,
            RunOnce::Off => return Ok(()),
        };
        if auto.remove(StartKey::RunOnce, RUN_ONCE_NAME)? {
            let text = format!("RunOnce value {RUN_ONCE_NAME} removed");
            self.note(Level::Info, &text);
        }
        Ok(())
    }

    fn ensure_guard(&mut self) -> Result<bool, Failure> {
        let started = self.guard.ensure()?;
        if started {
            self.note(Level::Info, "guard started");
        }
        Ok(started)
    }

    fn stop_guard(&mut self) -> Result<(), Failure> {
        Ok(self.guard.stop()?)
    }
}

/// Renames `journal.json.corrupt` to `journal.json.corrupt.seen`, replacing an older one, once the tray has shown the
/// notice for it (track note 7). `true`: there was one.
pub fn mark_corrupt_seen(dir: &DataDir) -> Result<bool, WinError> {
    let renamed = std::fs::rename(
        dir.file(DataFile::JournalCorrupt),
        dir.file(DataFile::JournalCorruptSeen),
    );
    match renamed.map_err(|e| io_error("MoveFileExW", &e)) {
        Ok(()) => Ok(true),
        Err(e) if e.code == ERROR_FILE_NOT_FOUND.0 || e.code == ERROR_PATH_NOT_FOUND.0 => Ok(false),
        Err(e) => Err(e),
    }
}

/// The first JSON value, after a UTF-8 byte order mark if there is one, the way `files::read` reads the other files:
/// a hand edit that adds either is no reason to set the journal aside, which leaves the user's values overridden.
fn parse(bytes: &[u8]) -> Result<JournalFile, String> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut values = serde_json::Deserializer::from_slice(bytes).into_iter::<JournalFile>();
    match values.next() {
        Some(Ok(file)) => Ok(file),
        Some(Err(e)) => Err(e.to_string()),
        None => Err("no JSON value".to_owned()),
    }
}

/// The journal file's own name for a key (`lidAc`), the one the plan lines use too, so a log line, a plan line and the
/// file can be matched by eye.
fn key_name(key: SettingKey) -> &'static str {
    PlanSetting::from(key).name()
}

/// `lidAc, lidDc`, or `nothing`.
fn key_list(keys: &[SettingKey]) -> String {
    match keys.is_empty() {
        true => "nothing".to_owned(),
        false => keys
            .iter()
            .map(|&key| key_name(key))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn push_entry(text: &mut String, entry: &JournalFileEntry) {
    let _ = match &entry.key {
        JournalKey::Known(key) => write!(text, " {}", key_name(*key)),
        // Quoted and escaped: the name comes from the file, so a stray quote or line break cannot break the line.
        JournalKey::Other(name) => write!(text, " {name:?}"),
    };
    // The `restored` mark is saved before an apply's first write and before a restore writes the original back, so it
    // tells of a change under way, not one done.
    let _ = write!(
        text,
        " on {}: original {}, written {}{};",
        entry.plan,
        entry.original,
        entry.written,
        if entry.restored {
            ", being changed"
        } else {
            ""
        }
    );
}

fn log_restored(log: &dyn Log, plan: Option<Guid>, done: &Restored) {
    let scope = match plan {
        None => "every plan".to_owned(),
        Some(plan) => format!("plan {plan}"),
    };
    let found = match done.found {
        Found::Nothing => "no journal entry".to_owned(),
        Found::Entries => "journal entries".to_owned(),
        Found::SetAside => "a journal that does not parse".to_owned(),
        Found::OtherOwner(owner) => format!(
            "a journal of another owner (pid {} start {}), left alone",
            owner.pid, owner.proc_start.0
        ),
    };
    let left = done.journal.len();
    match done.result {
        Ok(()) => {
            let text = format!("restore of {scope}: found {found}; {left} entries left");
            log.line(Level::Info, AREA, &text);
        }
        Err(f) => {
            let text = format!(
                "restore of {scope} failed: {}; found {found}; {left} entries kept",
                error_text(f)
            );
            log.line(Level::Warn, AREA, &text);
        }
    }
}

fn log_applied(log: &dyn Log, plan: Guid, done: &Applied) {
    let (written, left) = (&done.written, done.journal.len());
    let (level, how) = match done.result {
        Ok(()) => (Level::Info, "done".to_owned()),
        Err(ApplyError::PlanChanged) => {
            (Level::Warn, "stopped: the active plan changed".to_owned())
        }
        Err(ApplyError::SafetyNet(f)) => (
            Level::Warn,
            format!("failed at the safety net: {}", error_text(f)),
        ),
        Err(ApplyError::Write(f)) => (Level::Warn, format!("failed at a write: {}", error_text(f))),
    };
    let text = format!(
        "apply to plan {plan} {how}; wrote {}; {left} journal entries",
        key_list(written)
    );
    log.line(level, AREA, &text);
}

/// Each key with its old and new value, or why it was left: the notice sends the user here for the values they had.
fn log_reset(log: &dyn Log, done: &Reset) {
    match done.found {
        ResetFound::Entries => {
            let text = format!(
                "lost journal reset not run: journal.json reads now, with {} known entries",
                done.journal.len()
            );
            log.line(Level::Info, AREA, &text);
            return;
        }
        ResetFound::Nothing => {
            log.line(
                Level::Info,
                AREA,
                "lost journal reset not run: journal.json is gone or names nothing, so it was restored elsewhere",
            );
            return;
        }
        ResetFound::Lost => {}
    }
    let plan = done
        .plan
        .map_or_else(|| "the plan in use".to_owned(), |p| format!("plan {p}"));
    for &(key, outcome) in &done.keys {
        let name = key_name(key);
        let (level, text) = match outcome {
            KeyReset::Reset { old, default } => (
                Level::Warn,
                format!(
                    "lost journal: {name} on {plan} was {old}, Wakeshot's value: set to the Windows default {default}"
                ),
            ),
            KeyReset::Theirs(value) => (
                Level::Info,
                format!("lost journal: {name} on {plan} is {value}, not Wakeshot's value: kept"),
            ),
            KeyReset::AlreadyDefault(value) => (
                Level::Info,
                format!(
                    "lost journal: {name} on {plan} is {value}, which is also the Windows default: kept"
                ),
            ),
            KeyReset::Failed(f) => (
                Level::Warn,
                format!(
                    "lost journal: {name} on {plan} not reset: {}",
                    error_text(f)
                ),
            ),
        };
        log.line(level, AREA, &text);
    }
    if let Some(Err(f)) = done.set_aside {
        let text = format!(
            "lost journal: journal.json not set aside: {}; the next journal save replaces it",
            error_text(f)
        );
        log.line(Level::Warn, AREA, &text);
    }
    let set = key_list(&done.reset_keys());
    let text = match done.result {
        Ok(()) => format!("lost journal reset on {plan} done; keys set to Windows defaults: {set}"),
        Err(f) => format!(
            "lost journal reset on {plan} failed: {}; keys set to Windows defaults: {set}",
            error_text(f)
        ),
    };
    let level = match done.result {
        Ok(()) => Level::Info,
        Err(_) => Level::Warn,
    };
    log.line(level, AREA, &text);
}

/// Logged only when something happened: the check runs every 10 s while anything is owned.
fn log_checked(log: &dyn Log, done: &Checked) {
    if done.user_changed.is_empty() && done.failures.is_empty() {
        return;
    }
    let mut text = String::from("ownership check:");
    if !done.user_changed.is_empty() {
        text.push_str(" the user changed");
        for OwnedKey { plan, key } in &done.user_changed {
            let _ = write!(text, " {} on {plan};", key_name(*key));
        }
    }
    for f in &done.failures {
        let _ = write!(text, " {};", error_text(*f));
    }
    let _ = write!(text, " {} journal entries", done.journal.len());
    let level = match done.failures.is_empty() {
        true => Level::Info,
        false => Level::Warn,
    };
    log.line(level, AREA, &text);
}

fn error_text(f: Failure) -> String {
    format!("{} failed with {}", f.call, f.code)
}

/// A std error without an OS code (a NUL in the path) gets `u32::MAX`, which no Win32 error uses.
fn io_error(call: &'static str, e: &std::io::Error) -> WinError {
    WinError {
        call,
        code: e.raw_os_error().map_or(u32::MAX, |code| code as u32),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use wakeshot_core::ownership::fake::{Action, Call, FakeJournal, FakePlans};
    use wakeshot_core::process::FileTime;
    use wakeshot_core::settings::PlanValues;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Storage::FileSystem::{
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE,
        RegCreateKeyExW, RegSetValueExW,
    };
    use windows::core::{Owned, PCWSTR};

    use super::*;
    use crate::log::MemoryLog;
    use crate::store::ScratchKey;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OWNER: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(134_381_211_391_959_336),
        session: 1,
    };
    const VALUES: PlanValues = PlanValues {
        active: BALANCED,
        lid_ac: 1,
        lid_dc: 1,
        sleep_ac: 10_800,
        sleep_dc: 600,
        saver_threshold_dc: 20,
    };
    const TRAY_EXE: &str =
        r"C:\Users\Test User\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe";

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("wakeshot-journal-{tag}-{}-{n}", std::process::id()));
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

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn lines(log: &MemoryLog) -> Vec<(Level, String)> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(level, area, text)| {
                assert_eq!(*area, AREA);
                (*level, text.clone())
            })
            .collect()
    }

    fn count_containing(log: &MemoryLog, part: &str) -> usize {
        lines(log)
            .iter()
            .filter(|(_, text)| text.contains(part))
            .count()
    }

    fn known(key: SettingKey, restored: bool) -> JournalFileEntry {
        JournalFileEntry {
            plan: BALANCED,
            key: JournalKey::Known(key),
            original: 1,
            written: 0,
            restored,
        }
    }

    fn unknown(name: &str) -> JournalFileEntry {
        JournalFileEntry {
            plan: BALANCED,
            key: JournalKey::Other(name.into()),
            original: 10_800,
            written: 0,
            restored: false,
        }
    }

    fn journal(entries: Vec<JournalFileEntry>) -> JournalFile {
        JournalFile {
            version: JournalFile::VERSION,
            owner: OWNER,
            entries,
        }
    }

    fn dry(dir: &TempDir, log: &Arc<MemoryLog>) -> FileNet<NoGuard> {
        FileNet::dry_run(dir.data(), log.clone())
    }

    fn lock<'a>(names: &'a Names, log: &'a dyn Log, wait_ms: u32) -> JournalLock<'a> {
        JournalLock {
            names,
            log,
            wait_ms,
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        OsStr::new(text).encode_wide().chain([0]).collect()
    }

    /// A `REG_DWORD` under `<base>\RunOnce`, which `AutoStart` cannot write.
    fn set_dword(base: &str, name: &str, value: u32) {
        let path = wide(&format!(r"{base}\RunOnce"));
        let mut key = HKEY::default();
        // SAFETY: `path` is NUL-terminated and lives for the call; the key is written to a live local.
        let created = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &raw mut key,
                None,
            )
        };
        assert_eq!(created, ERROR_SUCCESS);
        // SAFETY: the key is new and owned only here.
        let key = unsafe { Owned::new(key) };
        let name = wide(name);
        // SAFETY: an open key with `KEY_SET_VALUE`; the data is a live 4-byte DWORD.
        let set = unsafe {
            RegSetValueExW(
                *key,
                PCWSTR(name.as_ptr()),
                None,
                REG_DWORD,
                Some(&value.to_le_bytes()[..]),
            )
        };
        assert_eq!(set, ERROR_SUCCESS);
    }

    /// Another thread takes the journal mutex and holds it until the sender is dropped or sends.
    fn hold(names: &Names) -> (mpsc::Sender<()>, thread::JoinHandle<()>) {
        let name = names.journal.clone();
        let (taken_tx, taken_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder = thread::spawn(move || {
            let held = NamedMutex::open(&name).unwrap().take(0).unwrap().unwrap();
            taken_tx.send(()).unwrap();
            let _ = release_rx.recv();
            drop(held);
        });
        taken_rx.recv().unwrap();
        (release_tx, holder)
    }

    #[test]
    fn load_tells_missing_corrupt_failed() {
        let dir = TempDir::new("load");
        let log = Arc::new(MemoryLog::default());
        let mut net = dry(&dir, &log);
        let path = dir.data().file(DataFile::Journal);
        assert_eq!(net.load_journal(), JournalRead::Missing);
        // A data folder that does not exist yet is missing too, not an error.
        let mut fresh = FileNet::dry_run(DataDir::at(dir.0.join("none")), log.clone());
        assert_eq!(fresh.load_journal(), JournalRead::Missing);

        let file = journal(vec![known(SettingKey::LidDc, false)]);
        let text = serde_json::to_string(&file).unwrap();
        for bad in [
            "",
            "   \r\n",
            "not json",
            "{ \"version\": 1 }",
            "{ \"version\": 1, \"owner\": 5, \"entries\": [] }",
            &text[..text.len() - 1],
        ] {
            fs::write(&path, bad).unwrap();
            assert_eq!(net.load_journal(), JournalRead::Corrupt, "{bad:?}");
        }
        assert_eq!(count_containing(&log, "journal.json does not parse"), 6);
        // Invalid UTF-8 is read in full and does not parse.
        fs::write(&path, b"\xff\xfe{").unwrap();
        assert_eq!(net.load_journal(), JournalRead::Corrupt);

        // A byte order mark or bytes after the first value do not make a whole journal corrupt.
        for good in [
            format!("\u{feff}{text}"),
            format!("{text}\r\n"),
            format!("{text} trailing"),
        ] {
            fs::write(&path, good.as_bytes()).unwrap();
            assert_eq!(
                net.load_journal(),
                JournalRead::Read(file.clone()),
                "{good:?}"
            );
        }

        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let JournalRead::Failed(failure) = net.load_journal() else {
            panic!("a folder in place of the journal must be a failed read");
        };
        println!("a folder in place of journal.json: {failure:?}");
        assert_eq!(failure.call, "CreateFileW");
    }

    #[test]
    fn save_load_round_trip() {
        let dir = TempDir::new("save");
        let log = Arc::new(MemoryLog::default());
        // The data folder does not exist yet: the first save makes it.
        let data = dir.0.join("data");
        let mut net = FileNet::dry_run(DataDir::at(data.clone()), log.clone());
        let file = journal(vec![
            known(SettingKey::LidAc, false),
            unknown("hibernateDc"),
            known(SettingKey::SleepDc, true),
        ]);
        net.save_journal(&file).unwrap();
        assert_eq!(net.load_journal(), JournalRead::Read(file.clone()));
        assert_eq!(names_in(&data), ["journal.json"]);
        let saved = lines(&log);
        assert_eq!(saved.len(), 2, "{saved:?}");
        assert_eq!(saved[0].0, Level::Info);
        assert_eq!(
            saved[0].1,
            format!(
                "journal saved: owner pid 9120 start 134381211391959336 session 1; lidAc on {BALANCED}: original 1, \
                 written 0; \"hibernateDc\" on {BALANCED}: original 10800, written 0; sleepDc on {BALANCED}: \
                 original 1, written 0, being changed;"
            )
        );

        // Whatever version the caller passes, the file gets this version (review m2).
        let newer = JournalFile {
            version: 2,
            ..file.clone()
        };
        net.save_journal(&newer).unwrap();
        assert_eq!(net.load_journal(), JournalRead::Read(file.clone()));
        let text = fs::read_to_string(data.join("journal.json")).unwrap();
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(
            text.contains("\"procStart\": \"134381211391959336\""),
            "{text}"
        );
        assert_eq!(
            count_containing(&log, "journal save got version 2; written as version 1"),
            1
        );

        // An empty entry list saves and loads too: the clean-up keeps such a file only with unknown entries.
        let empty = journal(Vec::new());
        net.save_journal(&empty).unwrap();
        assert_eq!(net.load_journal(), JournalRead::Read(empty));
        assert_eq!(names_in(&data), ["journal.json"]);
    }

    #[test]
    fn unknown_keys_are_logged_once() {
        let dir = TempDir::new("unknown");
        let log = Arc::new(MemoryLog::default());
        let mut net = dry(&dir, &log);
        let path = dir.data().file(DataFile::Journal);
        // A newer version's file, as that version would write it.
        fs::write(
            &path,
            r#"{ "version": 2, "owner": { "pid": 9120, "procStart": "134381211391959336", "session": 1 },
                 "entries": [
                   { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "lidDc", "original": 1, "written": 0,
                     "restored": false },
                   { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "hibernateAc", "original": 10800,
                     "written": 0, "restored": false, "since": 5 } ] }"#,
        )
        .unwrap();
        for _ in 0..3 {
            let JournalRead::Read(file) = net.load_journal() else {
                panic!("a newer journal reads");
            };
            assert_eq!(file.version, 2);
            assert_eq!(file.entries[1], unknown("hibernateAc"));
        }
        let unknown_lines =
            |log: &MemoryLog| count_containing(log, "keys this version does not know");
        assert_eq!(unknown_lines(&log), 1);
        let line = &lines(&log)[0].1;
        assert_eq!(
            line,
            &format!(
                "journal (version 2) holds 1 entries with keys this version does not know, kept as they are: \
                 \"hibernateAc\" on {BALANCED}: original 10800, written 0;"
            )
        );

        // Another unknown entry is news; a journal with none says nothing and resets, so they show again later.
        net.save_journal(&journal(vec![
            unknown("hibernateAc"),
            unknown("hibernateDc"),
        ]))
        .unwrap();
        net.load_journal();
        net.load_journal();
        assert_eq!(unknown_lines(&log), 2);
        net.save_journal(&journal(vec![known(SettingKey::LidAc, false)]))
            .unwrap();
        net.load_journal();
        assert_eq!(unknown_lines(&log), 2);
        net.save_journal(&journal(vec![unknown("hibernateAc")]))
            .unwrap();
        net.load_journal();
        assert_eq!(unknown_lines(&log), 3);

        // Each process logs them at its first load.
        let mut other = dry(&dir, &log);
        other.load_journal();
        assert_eq!(unknown_lines(&log), 4);
    }

    #[test]
    fn delete_missing_is_done() {
        let dir = TempDir::new("delete");
        let log = Arc::new(MemoryLog::default());
        let mut net = dry(&dir, &log);
        assert_eq!(net.delete_journal(), Ok(()));
        let mut fresh = FileNet::dry_run(DataDir::at(dir.0.join("none")), log.clone());
        assert_eq!(fresh.delete_journal(), Ok(()));
        assert_eq!(count_containing(&log, "journal deleted"), 0);

        net.save_journal(&journal(vec![known(SettingKey::LidAc, false)]))
            .unwrap();
        assert_eq!(net.delete_journal(), Ok(()));
        assert_eq!(net.load_journal(), JournalRead::Missing);
        assert!(names_in(&dir.0).is_empty());
        assert_eq!(count_containing(&log, "journal deleted"), 1);
        assert_eq!(net.delete_journal(), Ok(()));
        assert_eq!(count_containing(&log, "journal deleted"), 1);
    }

    #[test]
    fn set_aside_keeps_bytes() {
        let dir = TempDir::new("aside");
        let log = Arc::new(MemoryLog::default());
        let mut net = dry(&dir, &log);
        let data = dir.data();
        let (path, aside) = (
            data.file(DataFile::Journal),
            data.file(DataFile::JournalCorrupt),
        );
        let first: &[u8] = b"{ \"version\": 1, \"owner\": \xff not json";
        fs::write(&path, first).unwrap();
        assert_eq!(net.load_journal(), JournalRead::Corrupt);
        net.set_aside_journal().unwrap();
        assert_eq!(fs::read(&aside).unwrap(), first);
        assert_eq!(names_in(&dir.0), ["journal.json.corrupt"]);
        assert_eq!(net.load_journal(), JournalRead::Missing);
        assert_eq!(count_containing(&log, "moved to journal.json.corrupt"), 1);

        // A second corrupt file replaces the older one, also while a reader that shares delete has it open.
        let second: &[u8] = b"second";
        fs::write(&path, second).unwrap();
        let reader = fs::OpenOptions::new()
            .read(true)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .open(&aside)
            .unwrap();
        let moved = net.set_aside_journal();
        println!("set aside over a corrupt file a reader has open: {moved:?}");
        assert_eq!(moved, Ok(()));
        drop(reader);
        assert_eq!(fs::read(&aside).unwrap(), second);
        assert_eq!(names_in(&dir.0), ["journal.json.corrupt"]);

        // Nothing to move is a failure: the routine asks only after a load said corrupt.
        let none = net.set_aside_journal().unwrap_err();
        assert_eq!(
            (none.call, none.code),
            ("MoveFileExW", ERROR_FILE_NOT_FOUND.0)
        );
        assert_eq!(fs::read(&aside).unwrap(), second);
    }

    #[test]
    fn corrupt_seen_rename_once() {
        let dir = TempDir::new("seen");
        let data = dir.data();
        assert_eq!(mark_corrupt_seen(&data), Ok(false));
        assert_eq!(
            mark_corrupt_seen(&DataDir::at(dir.0.join("none"))),
            Ok(false)
        );

        fs::write(data.file(DataFile::JournalCorrupt), b"one").unwrap();
        assert_eq!(mark_corrupt_seen(&data), Ok(true));
        assert_eq!(names_in(&dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(
            fs::read(data.file(DataFile::JournalCorruptSeen)).unwrap(),
            b"one"
        );
        assert_eq!(mark_corrupt_seen(&data), Ok(false));

        // The next corrupt file replaces the older seen one.
        fs::write(data.file(DataFile::JournalCorrupt), b"two").unwrap();
        assert_eq!(mark_corrupt_seen(&data), Ok(true));
        assert_eq!(names_in(&dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(
            fs::read(data.file(DataFile::JournalCorruptSeen)).unwrap(),
            b"two"
        );
    }

    #[test]
    fn run_once_set_reports_added() {
        let key = ScratchKey::new("power::journal::run_once_set_reports_added");
        let dir = TempDir::new("run-once-set");
        let log = Arc::new(MemoryLog::default());
        let auto = AutoStart::at(key.path());
        let exe = PathBuf::from(TRAY_EXE);
        let mut net = FileNet::tray(dir.data(), auto.clone(), &exe, NoGuard, log.clone());
        let command = format!("\"{TRAY_EXE}\" --restore");

        assert_eq!(net.set_run_once(), Ok(true));
        assert_eq!(
            auto.get(StartKey::RunOnce, RUN_ONCE_NAME),
            Ok(Some(command.clone()))
        );
        assert_eq!(net.set_run_once(), Ok(false));
        assert_eq!(
            count_containing(&log, "RunOnce value WakeshotRestore set to"),
            1
        );

        // A value that names something else is ours by name: rewritten, but not added by this call.
        auto.set(
            StartKey::RunOnce,
            RUN_ONCE_NAME,
            "\"C:\\old\\wakeshot-tray.exe\" --restore",
        )
        .unwrap();
        assert_eq!(net.set_run_once(), Ok(false));
        assert_eq!(
            auto.get(StartKey::RunOnce, RUN_ONCE_NAME),
            Ok(Some(command.clone()))
        );

        // So is a value of another type, which does not even read.
        set_dword(key.path(), RUN_ONCE_NAME, 7);
        assert!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME).is_err());
        assert_eq!(net.set_run_once(), Ok(false));
        assert_eq!(
            auto.get(StartKey::RunOnce, RUN_ONCE_NAME),
            Ok(Some(command))
        );

        // An exe path that is not valid Unicode cannot be named in the value: the call fails and writes nothing.
        auto.remove(StartKey::RunOnce, RUN_ONCE_NAME).unwrap();
        let odd = PathBuf::from(OsString::from_wide(&[
            u16::from(b'C'),
            u16::from(b':'),
            u16::from(b'\\'),
            0xD800,
            u16::from(b'x'),
        ]));
        let mut odd_net = FileNet::tray(dir.data(), auto.clone(), &odd, NoGuard, log.clone());
        let err = odd_net.set_run_once().unwrap_err();
        assert_eq!(err.code, ERROR_NO_UNICODE_TRANSLATION.0);
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
    }

    #[test]
    fn run_once_clear_missing_is_done() {
        let key = ScratchKey::new("power::journal::run_once_clear_missing_is_done");
        let dir = TempDir::new("run-once-clear");
        let log = Arc::new(MemoryLog::default());
        let auto = AutoStart::at(key.path());
        let mut net = FileNet::tray(
            dir.data(),
            auto.clone(),
            Path::new(TRAY_EXE),
            NoGuard,
            log.clone(),
        );

        // No RunOnce key at all yet, then a key with no such value.
        assert_eq!(net.clear_run_once(), Ok(()));
        auto.set(StartKey::RunOnce, "Other", "x").unwrap();
        assert_eq!(net.clear_run_once(), Ok(()));
        assert_eq!(count_containing(&log, "removed"), 0);

        assert_eq!(net.set_run_once(), Ok(true));
        assert_eq!(net.clear_run_once(), Ok(()));
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_eq!(
            count_containing(&log, "RunOnce value WakeshotRestore removed"),
            1
        );
        // Only our value goes.
        assert_eq!(
            auto.get(StartKey::RunOnce, "Other"),
            Ok(Some("x".to_owned()))
        );
        assert_eq!(net.clear_run_once(), Ok(()));
        // After a clear, the next set adds it again.
        assert_eq!(net.set_run_once(), Ok(true));
    }

    #[test]
    fn dry_run_never_touches_run_once() {
        let dir = TempDir::new("dry");
        let log = Arc::new(MemoryLog::default());
        let mut net = dry(&dir, &log);
        // No registry key is reachable from the dry-run form at all.
        assert!(matches!(net.run_once, RunOnce::Off));
        for _ in 0..2 {
            assert_eq!(net.set_run_once(), Ok(false));
            assert_eq!(net.clear_run_once(), Ok(()));
            assert_eq!(net.ensure_guard(), Ok(false));
            assert_eq!(net.stop_guard(), Ok(()));
        }
        assert_eq!(count_containing(&log, "dry run: RunOnce value not set"), 2);
        assert_eq!(count_containing(&log, "guard started"), 0);
        // The journal itself works as in the tray, in the folder the dry run was given.
        let file = journal(vec![known(SettingKey::LidDc, false)]);
        net.save_journal(&file).unwrap();
        assert_eq!(names_in(&dir.0), ["journal.json"]);
        assert_eq!(net.load_journal(), JournalRead::Read(file));
    }

    #[test]
    fn guard_forms_never_set_run_once() {
        let key = ScratchKey::new("power::journal::guard_forms_never_set_run_once");
        let dir = TempDir::new("guard-forms");
        let log = Arc::new(MemoryLog::default());
        let auto = AutoStart::at(key.path());
        let mut net = FileNet::restore_only(dir.data(), auto.clone(), log.clone());

        let refused = net.set_run_once().unwrap_err();
        assert_eq!(
            refused,
            Failure {
                call: "FileNet::set_run_once",
                code: ERROR_NOT_SUPPORTED.0
            }
        );
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_eq!(net.ensure_guard(), Ok(false));

        // A value the tray left is removed in the clean-up; a missing one counts as removed.
        auto.set(StartKey::RunOnce, RUN_ONCE_NAME, "left by the tray")
            .unwrap();
        assert_eq!(net.clear_run_once(), Ok(()));
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
        assert_eq!(net.clear_run_once(), Ok(()));
        assert!(net.set_run_once().is_err());
        assert_eq!(auto.get(StartKey::RunOnce, RUN_ONCE_NAME), Ok(None));
    }

    #[test]
    fn ensure_guard_passes_through() {
        let key = ScratchKey::new("power::journal::ensure_guard_passes_through");
        let dir = TempDir::new("guard");
        let log = Arc::new(MemoryLog::default());
        let auto = AutoStart::at(key.path());
        let mut net = FileNet::tray(
            dir.data(),
            auto,
            Path::new(TRAY_EXE),
            RecordingGuard::default(),
            log.clone(),
        );
        assert_eq!(net.ensure_guard(), Ok(true));
        assert_eq!(net.ensure_guard(), Ok(false));
        assert_eq!(net.stop_guard(), Ok(()));
        // A guard that was told to stop is leaving: the next ensure starts a new one.
        assert_eq!(net.ensure_guard(), Ok(true));
        assert_eq!(count_containing(&log, "guard started"), 2);

        let start_failed = WinError {
            call: "CreateProcessW",
            code: 2,
        };
        let stop_failed = WinError {
            call: "SetEvent",
            code: 6,
        };
        net.guard.running = false;
        net.guard.fail_ensure = Some(start_failed);
        net.guard.fail_stop = Some(stop_failed);
        assert_eq!(net.ensure_guard(), Err(Failure::from(start_failed)));
        assert_eq!(net.stop_guard(), Err(Failure::from(stop_failed)));
        assert_eq!(count_containing(&log, "guard started"), 2);
        let guard = net.guard();
        assert_eq!(guard.started, 2);
        assert_eq!(
            guard.calls,
            [
                GuardCall::Ensure,
                GuardCall::Ensure,
                GuardCall::Stop,
                GuardCall::Ensure,
                GuardCall::Ensure,
                GuardCall::Stop,
            ]
        );
    }

    #[test]
    fn no_guard_starts_nothing() {
        let key = ScratchKey::new("power::journal::no_guard_starts_nothing");
        let dir = TempDir::new("no-guard");
        let log = Arc::new(MemoryLog::default());
        let auto = AutoStart::at(key.path());
        let mut restore_only = FileNet::restore_only(dir.data(), auto.clone(), log.clone());
        let mut dry_run = dry(&dir, &log);
        // The tray in dry run picks its guard at run time, behind a box.
        let picked: Box<dyn GuardControl> = Box::new(NoGuard);
        let mut boxed = FileNet::tray(dir.data(), auto, Path::new(TRAY_EXE), picked, log.clone());
        for _ in 0..2 {
            assert_eq!(restore_only.ensure_guard(), Ok(false));
            assert_eq!(restore_only.stop_guard(), Ok(()));
            assert_eq!(dry_run.ensure_guard(), Ok(false));
            assert_eq!(dry_run.stop_guard(), Ok(()));
            assert_eq!(boxed.ensure_guard(), Ok(false));
            assert_eq!(boxed.stop_guard(), Ok(()));
        }
        assert_eq!(count_containing(&log, "guard"), 0);

        // The same box passes a real guard's answers through.
        let recording: Box<dyn GuardControl> = Box::new(RecordingGuard::default());
        let mut boxed = FileNet::tray(
            dir.data(),
            AutoStart::at(key.path()),
            Path::new(TRAY_EXE),
            recording,
            log.clone(),
        );
        assert_eq!(boxed.ensure_guard(), Ok(true));
        assert_eq!(boxed.ensure_guard(), Ok(false));
    }

    #[test]
    fn lock_excludes_second_caller() {
        let names = Names::test("journal::excludes");
        let (inside_tx, inside_rx) = mpsc::channel();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let first = FakePlans::new(VALUES);
        // The first caller's journal load waits until the test lets it go, so the lock is held meanwhile.
        first.script(
            0,
            Action::Run(Box::new(move |_| {
                inside_tx.send(()).unwrap();
                go_rx.recv().unwrap();
            })),
        );
        let first_names = names.clone();
        let holder = thread::spawn(move || {
            let log = MemoryLog::default();
            let done = restore_within(
                &mut first.clone(),
                &mut first.clone(),
                None,
                None,
                lock(&first_names, &log, 5_000),
            );
            let calls = first.state().calls.clone();
            (done, calls)
        });
        inside_rx.recv().unwrap();

        let second = FakePlans::new(VALUES);
        second.state().journal = FakeJournal::File(journal(vec![known(SettingKey::LidDc, false)]));
        let log = MemoryLog::default();
        let started = Instant::now();
        let blocked = restore_within(
            &mut second.clone(),
            &mut second.clone(),
            None,
            None,
            lock(&names, &log, 100),
        );
        let waited = started.elapsed();
        println!("second caller gave up after {waited:?}");
        assert!(waited >= Duration::from_millis(80), "{waited:?}");
        assert_eq!(
            blocked,
            Restored {
                found: Found::Entries,
                result: Err(LOCK_TIMEOUT),
                journal: JournalParts::split(journal(vec![known(SettingKey::LidDc, false)]))
                    .known
                    .entries,
            }
        );
        // The routine never ran: only the read without the lock.
        assert_eq!(second.state().calls, [Call::LoadJournal]);
        assert_eq!(
            lines(&log),
            [(
                Level::Warn,
                format!(
                    "restore failed: {} was not free within 100 ms",
                    names.journal
                )
            )]
        );

        go_tx.send(()).unwrap();
        let (done, calls) = holder.join().unwrap();
        assert_eq!(
            done,
            Restored {
                found: Found::Nothing,
                result: Ok(()),
                journal: Vec::new(),
            }
        );
        assert_eq!(calls, [Call::LoadJournal]);

        // Released: the next caller takes it at once.
        let third = FakePlans::new(VALUES);
        let log = MemoryLog::default();
        let done = restore_within(
            &mut third.clone(),
            &mut third.clone(),
            None,
            None,
            lock(&names, &log, 0),
        );
        assert_eq!(done.result, Ok(()));
        assert_eq!(
            lines(&log),
            [(
                Level::Info,
                "restore of every plan: found no journal entry; 0 entries left".to_owned()
            )]
        );
    }

    #[test]
    fn lock_abandoned_counts_as_taken() {
        let names = Names::test("journal::abandoned");
        let name = names.journal.clone();
        thread::spawn(move || {
            let held = NamedMutex::open(&name).unwrap().take(0).unwrap().unwrap();
            // The thread ends while it holds the mutex, as a killed process would.
            std::mem::forget(held);
        })
        .join()
        .unwrap();

        let fake = FakePlans::new(VALUES);
        let log = MemoryLog::default();
        let done = restore_within(
            &mut fake.clone(),
            &mut fake.clone(),
            None,
            None,
            lock(&names, &log, 1_000),
        );
        assert_eq!(done.result, Ok(()));
        assert_eq!(fake.state().calls, [Call::LoadJournal]);
        assert_eq!(
            lines(&log),
            [
                (
                    Level::Warn,
                    format!(
                        "{} taken after its holder ended without releasing it",
                        names.journal
                    )
                ),
                (
                    Level::Info,
                    "restore of every plan: found no journal entry; 0 entries left".to_owned()
                ),
            ]
        );

        // Released on the way out: the next run takes it plainly.
        let log = MemoryLog::default();
        let done = restore_within(
            &mut fake.clone(),
            &mut fake.clone(),
            None,
            None,
            lock(&names, &log, 0),
        );
        assert_eq!(done.result, Ok(()));
        assert_eq!(count_containing(&log, "without releasing"), 0);
    }

    #[test]
    fn lock_timeout_fails_the_run() {
        assert_eq!(JOURNAL_WAIT_MS, 10_000);
        let names = Names::test("journal::timeout");
        let (release, holder) = hold(&names);
        let fake = FakePlans::new(VALUES);
        let file = journal(vec![known(SettingKey::LidAc, true), unknown("hibernateAc")]);
        let left = JournalParts::split(file.clone()).known.entries;
        fake.state().journal = FakeJournal::File(file);
        let log = MemoryLog::default();

        let restored = restore_within(
            &mut fake.clone(),
            &mut fake.clone(),
            Some(BALANCED),
            Some(OWNER),
            lock(&names, &log, 30),
        );
        assert_eq!(
            restored,
            Restored {
                found: Found::Entries,
                result: Err(LOCK_TIMEOUT),
                journal: left.clone(),
            }
        );
        let applied = apply_within(
            &mut fake.clone(),
            &mut fake.clone(),
            OWNER,
            BALANCED,
            &[(SettingKey::LidAc, 0), (SettingKey::SleepDc, 0)],
            lock(&names, &log, 30),
        );
        assert_eq!(
            applied,
            Applied {
                written: Vec::new(),
                result: Err(ApplyError::SafetyNet(LOCK_TIMEOUT)),
                journal: left.clone(),
            }
        );
        let checked = check_within(&mut fake.clone(), &mut fake.clone(), lock(&names, &log, 30));
        assert_eq!(
            checked,
            Checked {
                user_changed: Vec::new(),
                failures: vec![LOCK_TIMEOUT],
                journal: left,
            }
        );
        assert_eq!(LOCK_TIMEOUT.code, 258);
        // Nothing ran but three reads of the file: no plan call, no write, no RunOnce, no guard.
        assert_eq!(
            fake.state().calls,
            [Call::LoadJournal, Call::LoadJournal, Call::LoadJournal]
        );
        let wait_lines: Vec<String> = lines(&log).into_iter().map(|(_, text)| text).collect();
        assert_eq!(
            wait_lines,
            ["restore", "apply", "ownership check"]
                .map(|what| format!("{what} failed: {} was not free within 30 ms", names.journal))
        );

        // A journal that is missing or unreadable gives an empty list, never a guess.
        fake.state().journal = FakeJournal::Unreadable(5);
        let restored = restore_within(
            &mut fake.clone(),
            &mut fake.clone(),
            None,
            None,
            lock(&names, &log, 0),
        );
        assert_eq!(
            restored,
            Restored {
                found: Found::Nothing,
                result: Err(LOCK_TIMEOUT),
                journal: Vec::new(),
            }
        );

        release.send(()).unwrap();
        holder.join().unwrap();
    }

    #[test]
    fn a_lost_journal_reset_logs_each_key_and_a_busy_lock_does_nothing() {
        use wakeshot_core::ownership::fake::{
            FAKE_DEFAULT_LID, FAKE_DEFAULT_SLEEP_DC, FAKE_SCREEN_OFF,
        };

        let names = Names::test("journal::reset");
        let fake = FakePlans::new(PlanValues {
            lid_ac: 0,
            sleep_dc: 0,
            ..VALUES
        });
        fake.state().journal = FakeJournal::Unreadable(32);
        let log = MemoryLog::default();
        let done = reset_within(
            &mut fake.clone(),
            &mut fake.clone(),
            false,
            lock(&names, &log, 1_000),
        );
        assert_eq!(done.reset_keys(), [SettingKey::LidAc, SettingKey::SleepDc]);
        assert_eq!(done.result, Ok(()));
        assert_eq!(
            lines(&log),
            [
                (
                    Level::Warn,
                    format!(
                        "lost journal: lidAc on plan {BALANCED} was 0, Wakeshot's value: set to the Windows default \
                         {FAKE_DEFAULT_LID}"
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "lost journal: lidDc on plan {BALANCED} is 1, not Wakeshot's value: kept"
                    )
                ),
                (
                    Level::Warn,
                    format!(
                        "lost journal: sleepDc on plan {BALANCED} was 0, Wakeshot's value: set to the Windows default \
                         {FAKE_DEFAULT_SLEEP_DC}"
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "lost journal: sleepAc on plan {BALANCED} is 10800, not Wakeshot's value: kept"
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "lost journal: videoIdleAc on plan {BALANCED} is {}, not Wakeshot's value: kept",
                        FAKE_SCREEN_OFF[0]
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "lost journal: videoIdleDc on plan {BALANCED} is {}, not Wakeshot's value: kept",
                        FAKE_SCREEN_OFF[1]
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "lost journal reset on plan {BALANCED} done; keys set to Windows defaults: lidAc, sleepDc"
                    )
                ),
            ]
        );

        // A set-aside that fails is a warning, and the reset still counts.
        let fake = FakePlans::new(PlanValues {
            lid_dc: 0,
            ..VALUES
        });
        fake.state().journal = FakeJournal::Corrupt;
        // Load, active plan, six reads, one default read, one write, `set_active`, then the set-aside.
        let set_aside_at = 11;
        fake.script(set_aside_at, Action::Fail(32));
        let log = MemoryLog::default();
        let done = reset_within(
            &mut fake.clone(),
            &mut fake.clone(),
            false,
            lock(&names, &log, 1_000),
        );
        assert_eq!(fake.state().calls[set_aside_at], Call::SetAsideJournal);
        assert_eq!(
            (done.reset_keys(), done.result),
            (vec![SettingKey::LidDc], Ok(()))
        );
        assert_eq!(
            count_containing(
                &log,
                "journal.json not set aside: FakePlans::set_aside_journal failed with 32; the next journal save \
                 replaces it"
            ),
            1
        );

        // Gone since the core's last try: one line, nothing else.
        let log = MemoryLog::default();
        fake.state().journal = FakeJournal::Missing;
        let done = reset_within(
            &mut fake.clone(),
            &mut fake.clone(),
            false,
            lock(&names, &log, 1_000),
        );
        assert_eq!(done.found, ResetFound::Nothing);
        assert_eq!(lines(&log).len(), 1, "{:?}", lines(&log));

        // The mutex is not free: nothing is done, and the failure says why.
        let (release, holder) = hold(&names);
        fake.state().journal = FakeJournal::Unreadable(32);
        let before = fake.state().calls.len();
        let log = MemoryLog::default();
        let done = reset_within(
            &mut fake.clone(),
            &mut fake.clone(),
            false,
            lock(&names, &log, 30),
        );
        assert_eq!(
            (done.result, done.keys.len(), done.journal.len()),
            (Err(LOCK_TIMEOUT), 0, 0)
        );
        assert_eq!(fake.state().calls[before..], [Call::LoadJournal]);
        assert_eq!(
            count_containing(&log, "lost journal reset failed: "),
            1,
            "{:?}",
            lines(&log)
        );
        release.send(()).unwrap();
        holder.join().unwrap();
    }

    /// The outcome lines of the routines that do not run here yet (`apply`, `check`) and the quiet check.
    #[test]
    fn outcome_lines_name_the_result() {
        let log = MemoryLog::default();
        let entry = JournalEntry {
            plan: BALANCED,
            key: SettingKey::LidDc,
            original: 1,
            written: 0,
            restored: false,
        };
        log_applied(
            &log,
            BALANCED,
            &Applied {
                written: vec![SettingKey::LidAc, SettingKey::LidDc],
                result: Ok(()),
                journal: vec![entry, entry],
            },
        );
        log_applied(
            &log,
            BALANCED,
            &Applied {
                written: Vec::new(),
                result: Err(ApplyError::Write(Failure {
                    call: "PowerWriteDCValueIndex",
                    code: 5,
                })),
                journal: Vec::new(),
            },
        );
        log_checked(
            &log,
            &Checked {
                user_changed: Vec::new(),
                failures: Vec::new(),
                journal: vec![entry],
            },
        );
        log_checked(
            &log,
            &Checked {
                user_changed: vec![OwnedKey {
                    plan: BALANCED,
                    key: SettingKey::SleepDc,
                }],
                failures: vec![Failure {
                    call: "PowerReadDCValueIndex",
                    code: 2,
                }],
                journal: vec![entry],
            },
        );
        log_restored(
            &log,
            Some(BALANCED),
            &Restored {
                found: Found::OtherOwner(OWNER),
                result: Ok(()),
                journal: vec![entry],
            },
        );
        log_restored(
            &log,
            None,
            &Restored {
                found: Found::Entries,
                result: Err(Failure {
                    call: "PowerSetActiveScheme",
                    code: 5,
                }),
                journal: vec![entry],
            },
        );
        assert_eq!(
            lines(&log),
            [
                (
                    Level::Info,
                    format!("apply to plan {BALANCED} done; wrote lidAc, lidDc; 2 journal entries")
                ),
                (
                    Level::Warn,
                    format!(
                        "apply to plan {BALANCED} failed at a write: PowerWriteDCValueIndex failed with 5; wrote \
                         nothing; 0 journal entries"
                    )
                ),
                (
                    Level::Warn,
                    format!(
                        "ownership check: the user changed sleepDc on {BALANCED}; PowerReadDCValueIndex failed with \
                         2; 1 journal entries"
                    )
                ),
                (
                    Level::Info,
                    format!(
                        "restore of plan {BALANCED}: found a journal of another owner (pid 9120 start \
                         134381211391959336), left alone; 1 entries left"
                    )
                ),
                (
                    Level::Warn,
                    "restore of every plan failed: PowerSetActiveScheme failed with 5; found journal entries; 1 \
                     entries kept"
                        .to_owned()
                ),
            ]
        );
    }
}
