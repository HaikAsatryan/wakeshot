//! The IO thread: its loop over `IoQueue`, the executor, file commands, `Log`, `Notify` and `OpenLog`.
//!
//! Owner: `p6-int-io-thread`; fixes by the e2e cards. It sets the `Ack` after the end-of-session restore
//! (`docs/phases/06-integration.md` note 5).

use std::ffi::OsStr;
use std::os::windows::io::AsRawHandle;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use wakeshot_core::command::{Command, Failure, FileKind, Outcome, RestoreWhy};
use wakeshot_core::event::Event;
use wakeshot_core::files::{self, Config, Read};
use wakeshot_core::holds::HoldId;
use wakeshot_core::ownership::io::{PlanIo, SafetyNet};
use wakeshot_core::view::Notice;
use windows::Win32::Foundation::{
    ERROR_INVALID_DATA, ERROR_INVALID_NAME, ERROR_INVALID_WINDOW_HANDLE, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::WaitForSingleObject;

use crate::WinError;
use crate::app::Ack;
use crate::io_queue::IoQueue;
use crate::log::{self, Level, Log};
use crate::notify::Notifier;
use crate::power::executor::PowerExecutor;
use crate::power::journal::{FileNet, NoGuard};
use crate::power::plan::{DryRunPower, Win32Power};
use crate::sink::EventSink;
use crate::store::{self, DataDir, DataFile};

const AREA: &str = "io";

/// The answer to a `WriteConfig` that found a newer `config.json` and kept it. 13 (`ERROR_INVALID_DATA`), as
/// `READ_BACK_DIFFERS` uses where Windows gives no code; the core ignores a `Config` answer.
const NEWER_CONFIG_KEPT: Failure = Failure {
    call: "config.json (newer version kept)",
    code: ERROR_INVALID_DATA.0,
};

/// What the tray gives `finish` (phase file note 5): the journal mutex wait is 10 s, so a restore that waits for the
/// mutex still fits. Dropping an `IoThread` waits as long.
pub const FINISH_MS: u32 = 10_000;

/// The longest wait `pop` takes: the thread wakes only for a push (`finish` pushes its stop marker), never on a
/// timer, since every timer wake costs battery.
const IDLE_WAIT_MS: u32 = u32::MAX;

/// What the IO thread runs `ReadPlan`, `Apply` and `Restore` on: `PowerExecutor`, so the real and the dry-run executor
/// (two types) start the thread the same way, and a test may block one.
pub trait PowerRunner: Send {
    /// The events for one command, in order (`PowerExecutor::run`).
    fn run(&mut self, command: &Command) -> Vec<Event>;
}

impl<P: PlanIo + Send, N: SafetyNet + Send> PowerRunner for PowerExecutor<P, N> {
    fn run(&mut self, command: &Command) -> Vec<Event> {
        PowerExecutor::run(self, command)
    }
}

// The executor forms the tray ships must fit, or the composition root could not start this thread.
const _: () = {
    const fn is_power_runner<T: PowerRunner>() {}
    is_power_runner::<PowerExecutor<Win32Power, FileNet<NoGuard>>>();
    is_power_runner::<PowerExecutor<DryRunPower<Win32Power>, FileNet<NoGuard>>>();
};

/// Opens a file with its default app: `shell::open_with_default_app` in the tray, a recorder in tests, so no test
/// opens a window.
pub type Opener = Box<dyn Fn(&OsStr) -> Result<(), WinError> + Send>;

/// What the IO thread works with. `power`, `notifier` and `open` move to the thread; the rest are shared.
pub struct IoParts {
    /// The loop pushes, this thread pops.
    pub queue: Arc<IoQueue>,
    /// The executor after its startup restore (tray startup step 6).
    pub power: Box<dyn PowerRunner>,
    /// The data folder, real or dry run: `state.json`, `config.json`, `holds\`, `restore-request.json` and the log.
    pub dir: DataDir,
    pub notifier: Box<dyn Notifier>,
    pub open: Opener,
    pub log: Arc<dyn Log>,
    pub sink: Arc<dyn EventSink>,
    /// Set after each `Restore { why: SessionEnd }` ran; the loop waits on it at the end of the session.
    pub ack: Ack,
}

/// The running IO thread (architecture section 4, "Tray threads"). Dropping it finishes it as `finish(FINISH_MS)`
/// does, so an early exit of the tray still runs what is queued.
pub struct IoThread {
    queue: Arc<IoQueue>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    log: Arc<dyn Log>,
}

impl IoThread {
    /// Starts the thread `wakeshot-io`. It runs each queued command in order (phase file note 4, the IO rows) and
    /// posts every answer through the sink.
    pub fn start(parts: IoParts) -> Result<IoThread, WinError> {
        let stopping = Arc::new(AtomicBool::new(false));
        let queue = Arc::clone(&parts.queue);
        let log = Arc::clone(&parts.log);
        let runner = Runner {
            queue: parts.queue,
            power: parts.power,
            dir: parts.dir,
            notifier: parts.notifier,
            open: parts.open,
            log: parts.log,
            sink: parts.sink,
            ack: parts.ack,
            stopping: Arc::clone(&stopping),
            loop_gone: false,
        };
        let thread = std::thread::Builder::new()
            .name("wakeshot-io".to_owned())
            .spawn(move || runner.run_queue())
            .map_err(|e| WinError {
                call: "CreateThread",
                code: e.raw_os_error().map_or(u32::MAX, |c| c as u32),
            })?;
        Ok(IoThread {
            queue,
            stopping,
            thread: Some(thread),
            log,
        })
    }

    /// Runs what is queued, then stops the thread. `true`: all of it ran and the thread ended within `timeout_ms`.
    /// `false`: the time ran out, and the thread is left to run on while the process exits (the guard covers a restore
    /// it did not finish), or the thread had ended with a panic. Both are logged.
    pub fn finish(mut self, timeout_ms: u32) -> bool {
        self.end(timeout_ms)
    }

    fn end(&mut self, timeout_ms: u32) -> bool {
        let Some(thread) = self.thread.take() else {
            return true;
        };
        self.stopping.store(true, Ordering::Release);
        // The queue has no wake of its own, so a marker goes in last. `Quit` never comes this way otherwise: the loop
        // runs it.
        self.queue.push(Command::Quit);
        // SAFETY: the join handle owns the thread's handle and lives through the call.
        let waited = unsafe { WaitForSingleObject(HANDLE(thread.as_raw_handle()), timeout_ms) };
        if waited != WAIT_OBJECT_0 {
            let text = format!(
                "the IO thread did not finish its queue within {timeout_ms} ms, so it is left to end on its own"
            );
            self.log.line(Level::Warn, AREA, &text);
            return false;
        }
        if thread.join().is_err() {
            self.log
                .line(Level::Error, AREA, "the IO thread ended with a panic");
            return false;
        }
        true
    }
}

impl Drop for IoThread {
    fn drop(&mut self) {
        self.end(FINISH_MS);
    }
}

/// The thread's side: everything it owns.
struct Runner {
    queue: Arc<IoQueue>,
    power: Box<dyn PowerRunner>,
    dir: DataDir,
    notifier: Box<dyn Notifier>,
    open: Opener,
    log: Arc<dyn Log>,
    sink: Arc<dyn EventSink>,
    ack: Ack,
    stopping: Arc<AtomicBool>,
    /// A post failed with 1400: the loop window is gone for good, so the answers after it are dropped without a line
    /// each.
    loop_gone: bool,
}

impl Runner {
    fn run_queue(mut self) {
        loop {
            let Some(command) = self.queue.pop(IDLE_WAIT_MS) else {
                continue;
            };
            if command == Command::Quit && self.stopping.load(Ordering::Acquire) {
                break;
            }
            self.run(&command);
        }
        // Nothing should come after the marker, since the loop has ended; whatever did still runs, as `finish`
        // promises.
        while let Some(command) = self.queue.pop(0) {
            self.run(&command);
        }
    }

    fn run(&mut self, command: &Command) {
        match command {
            Command::ReadPlan | Command::Apply { .. } | Command::Restore { .. } => {
                for event in self.power.run(command) {
                    self.post(event, command);
                }
                // After the answer is posted, so the loop finds it queued when its wait ends.
                if matches!(
                    command,
                    Command::Restore {
                        why: RestoreWhy::SessionEnd,
                        ..
                    }
                ) {
                    self.ack.set();
                }
            }
            Command::WriteState(state) => {
                let path = self.dir.file(DataFile::State);
                let result = write_json(&path, serde_json::to_vec_pretty(state));
                self.file_done(FileKind::State, result, "state.json not written", command);
            }
            Command::WriteConfig(config) => self.write_config(config, command),
            Command::WriteHold { id, hold } => {
                let result = hold_path(&self.dir, id)
                    .and_then(|path| write_json(&path, serde_json::to_vec_pretty(hold)));
                let failed = format!(r"holds\{:?} not written", id.0);
                self.file_done(FileKind::Hold(id.clone()), result, &failed, command);
            }
            Command::DeleteHold(id) => {
                let result = hold_path(&self.dir, id).and_then(|path| remove_file(&path));
                let failed = format!(r"holds\{:?} not deleted", id.0);
                self.file_done(FileKind::Hold(id.clone()), result, &failed, command);
            }
            Command::FinishRestoreRequest(state) => {
                let result = self.finish_request(serde_json::to_vec_pretty(state));
                let answer = Outcome::Written {
                    file: FileKind::RestoreRequest,
                    result,
                };
                self.post(Event::Done(answer), command);
            }
            Command::Notify(notice) => {
                let result = self.notifier.show(notice).map_err(Failure::from);
                let name = notice_name(notice);
                match result {
                    Ok(()) => self.note(Level::Info, &format!("notice shown: {name}")),
                    Err(f) => {
                        let text = format!("notice {name} not shown: {}", failure_text(f));
                        self.note(Level::Warn, &text);
                    }
                }
                self.post(Event::Done(Outcome::Notified(result)), command);
            }
            Command::Log(text) => self.log.line(Level::Info, "core", text),
            Command::OpenLog => {
                let path = log::log_file(&self.dir.logs());
                match (self.open)(path.as_os_str()) {
                    Ok(()) => self.note(Level::Info, "log opened with the default app"),
                    Err(e) => {
                        let text = format!("log not opened: {}", failure_text(e.into()));
                        self.note(Level::Warn, &text);
                    }
                }
            }
            // No wildcard: a new command must be placed here or on the loop, never dropped by default.
            Command::Requests { .. }
            | Command::Retake
            | Command::Lock
            | Command::Hibernate { .. }
            | Command::Tray(_)
            | Command::Play(_)
            | Command::ApplyUpdate
            | Command::Quit => {
                let text = format!(
                    "wiring bug: {} reached the IO thread, which does not run it; dropped",
                    label(command)
                );
                self.note(Level::Error, &text);
            }
        }
    }

    /// `state.json` first, so `wakeshot restore`, which waits for the request file to go, then reads the state after
    /// the restore. A failed state write keeps the request: the CLI times out and says so instead of printing the state
    /// from before the restore.
    fn finish_request(&self, state: serde_json::Result<Vec<u8>>) -> Result<(), Failure> {
        if let Err(f) = write_json(&self.dir.file(DataFile::State), state) {
            let text = format!(
                "restore request not answered: state.json not written: {}; restore-request.json kept",
                failure_text(f)
            );
            self.note(Level::Warn, &text);
            return Err(f);
        }
        if let Err(f) = remove_file(&self.dir.file(DataFile::RestoreRequest)) {
            let text = format!(
                "restore request: state.json written, but restore-request.json not removed: {}",
                failure_text(f)
            );
            self.note(Level::Warn, &text);
            return Err(f);
        }
        self.note(
            Level::Info,
            "restore request answered: state.json written, restore-request.json removed",
        );
        Ok(())
    }

    /// Never over a newer `config.json` (architecture section 6): it may hold fields this version does not know, and
    /// a write would drop them. The menu's Automatic then holds for this run only. A file that does not parse is
    /// replaced, and one that does not read is kept, as `wakeshot automatic` does.
    fn write_config(&mut self, config: &Config, command: &Command) {
        let path = self.dir.file(DataFile::Config);
        let result = match store::read_whole(&path) {
            Ok(Some(bytes)) => match files::read::<Config>(&bytes) {
                Read::Newer(version) => {
                    let word = if config.automatic { "on" } else { "off" };
                    let text = format!(
                        "config.json not written: version {version} is newer than {}; Automatic {word} for this run \
                         only",
                        Config::VERSION
                    );
                    self.note(Level::Warn, &text);
                    let answer = Outcome::Written {
                        file: FileKind::Config,
                        result: Err(NEWER_CONFIG_KEPT),
                    };
                    self.post(Event::Done(answer), command);
                    return;
                }
                Read::Value(_) | Read::Bad => write_json(&path, serde_json::to_vec_pretty(config)),
            },
            Ok(None) => write_json(&path, serde_json::to_vec_pretty(config)),
            Err(e) => Err(Failure::from(e)),
        };
        self.file_done(FileKind::Config, result, "config.json not written", command);
    }

    /// Answers a file command; `failed` names the file, never its content (architecture section 8).
    fn file_done(
        &mut self,
        file: FileKind,
        result: Result<(), Failure>,
        failed: &str,
        command: &Command,
    ) {
        if let Err(f) = result {
            self.note(Level::Warn, &format!("{failed}: {}", failure_text(f)));
        }
        self.post(Event::Done(Outcome::Written { file, result }), command);
    }

    /// One answer to the loop. A failed post is logged and the answer dropped: the next safety tick recovers
    /// (`EventSink`).
    fn post(&mut self, event: Event, answering: &Command) {
        let Err(e) = self.sink.post(event) else {
            return;
        };
        if e.code == ERROR_INVALID_WINDOW_HANDLE.0 {
            // The loop window is gone, as after every quit while `finish` runs the queue (`WindowSink`), and it never
            // comes back: one line says so for all the answers after it.
            if !std::mem::replace(&mut self.loop_gone, true) {
                let text = format!(
                    "the loop window is gone ({}), as after a quit: the answer to {} and every later answer are \
                     dropped",
                    failure_text(e.into()),
                    label(answering)
                );
                self.note(Level::Info, &text);
            }
            return;
        }
        let text = format!(
            "the answer to {} was not posted: {}; the next safety tick recovers",
            label(answering),
            failure_text(e.into())
        );
        self.note(Level::Warn, &text);
    }

    fn note(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }
}

/// `holds\<id>`. The core names a hold by its bare file name (`HoldId`'s contract); anything else is refused, so a bug
/// can never write or delete a file outside `holds\`.
fn hold_path(dir: &DataDir, id: &HoldId) -> Result<PathBuf, Failure> {
    let mut parts = Path::new(&id.0).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(name)), None) if name == OsStr::new(&id.0) => {
            Ok(dir.holds().join(name))
        }
        _ => Err(Failure {
            call: "hold file name check",
            code: ERROR_INVALID_NAME.0,
        }),
    }
}

/// Pretty, as the journal is: the files are small, and a person may read them by hand. A serde error never reaches a
/// log line, since it may quote the content.
fn write_json(path: &Path, bytes: serde_json::Result<Vec<u8>>) -> Result<(), Failure> {
    let bytes = bytes.map_err(|_| Failure {
        call: "serde_json::to_vec_pretty",
        code: ERROR_INVALID_DATA.0,
    })?;
    store::write_atomic(path, &bytes).map_err(Failure::from)
}

/// A file that is already gone counts as removed.
fn remove_file(path: &Path) -> Result<(), Failure> {
    store::remove(path).map(drop).map_err(Failure::from)
}

fn failure_text(f: Failure) -> String {
    format!("{} failed with {}", f.call, f.code)
}

/// A command as a log line names it: never its content. Hold names are quoted, so an odd name cannot break a line.
fn label(command: &Command) -> String {
    match command {
        Command::Requests { .. } => "Requests".to_owned(),
        Command::Retake => "Retake".to_owned(),
        Command::ReadPlan => "ReadPlan".to_owned(),
        Command::Apply { .. } => "Apply".to_owned(),
        Command::Restore { why, .. } => format!("Restore ({why:?})"),
        Command::Lock => "Lock".to_owned(),
        Command::Hibernate { .. } => "Hibernate".to_owned(),
        Command::Tray(_) => "Tray".to_owned(),
        Command::Notify(notice) => format!("Notify ({})", notice_name(notice)),
        Command::Play(_) => "Play".to_owned(),
        Command::WriteState(_) => "WriteState".to_owned(),
        Command::WriteConfig(_) => "WriteConfig".to_owned(),
        Command::WriteHold { id, .. } => format!("WriteHold ({:?})", id.0),
        Command::DeleteHold(id) => format!("DeleteHold ({:?})", id.0),
        Command::FinishRestoreRequest(_) => "FinishRestoreRequest".to_owned(),
        Command::Log(_) => "Log".to_owned(),
        Command::OpenLog => "OpenLog".to_owned(),
        Command::ApplyUpdate => "ApplyUpdate".to_owned(),
        Command::Quit => "Quit".to_owned(),
    }
}

/// The notice's name only: `ClosedLidReport` carries hold labels, which may come from a script's hold file.
fn notice_name(notice: &Notice) -> &'static str {
    match notice {
        Notice::FirstRun => "FirstRun",
        Notice::UserChanged(_) => "UserChanged",
        Notice::RestoredByUser => "RestoredByUser",
        Notice::StepBack { .. } => "StepBack",
        Notice::RequestsOnly => "RequestsOnly",
        Notice::CannotHold => "CannotHold",
        Notice::RestoreFailed => "RestoreFailed",
        Notice::HibernateFailed => "HibernateFailed",
        Notice::LockFailed => "LockFailed",
        Notice::ClosedLidReport { .. } => "ClosedLidReport",
        Notice::UpdateWakeshot { .. } => "UpdateWakeshot",
        Notice::JournalCorrupt => "JournalCorrupt",
        Notice::IdleLocked => "IdleLocked",
        Notice::Updated { .. } => "Updated",
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Mutex, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};

    use wakeshot_core::command::CountdownReason;
    use wakeshot_core::files::{Config, HoldFile, StateFile};
    use wakeshot_core::ownership::fake::{Action, Call, FakeJournal, FakePlans};
    use wakeshot_core::process::FileTime;
    use wakeshot_core::settings::{Guid, JournalOwner, OwnedKey, PlanValues, SettingKey};
    use wakeshot_core::view::{Cue, IconState, MenuModel, TrayView};
    use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_ENOUGH_QUOTA};
    use windows::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

    use super::*;
    use crate::log::MemoryLog;
    use crate::notify::FakeNotifier;
    use crate::process::FakeLiveness;
    use crate::sink::FakeEventSink;
    use crate::store::Names;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const ME: JournalOwner = JournalOwner {
        pid: 4242,
        proc_start: FileTime(134_381_300_000_000_000),
        session: 1,
    };
    /// `finish`'s time in tests: far above what a test needs, so only a blocked thread reaches it.
    const FINISH_WAIT: u32 = 10_000;

    /// The user's own values before any hold. Test values only, not read from any laptop.
    fn user_values() -> PlanValues {
        PlanValues {
            active: BALANCED,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 14_400,
            saver_threshold_dc: 30,
        }
    }

    fn state(holding: &[&str]) -> StateFile {
        StateFile {
            version: StateFile::VERSION,
            pid: ME.pid,
            proc_start: ME.proc_start,
            automatic: true,
            holding: holding.iter().map(|&h| h.to_owned()).collect(),
            overridden: vec![OwnedKey {
                plan: BALANCED,
                key: SettingKey::LidDc,
            }],
            stepped_back: false,
        }
    }

    fn hold(reason: &str) -> HoldFile {
        HoldFile {
            reason: Some(reason.to_owned()),
            ..HoldFile::new(ME.pid, Some(ME.proc_start))
        }
    }

    fn id(name: &str) -> HoldId {
        HoldId(name.to_owned())
    }

    fn apply() -> Command {
        Command::Apply {
            plan: BALANCED,
            writes: vec![(SettingKey::LidDc, 0)],
        }
    }

    fn restore(why: RestoreWhy) -> Command {
        Command::Restore { plan: None, why }
    }

    fn written(file: FileKind, result: Result<(), Failure>) -> Event {
        Event::Done(Outcome::Written { file, result })
    }

    /// Opened without delete sharing, so no write can replace the file while the handle lives.
    fn hold_open(path: &Path) -> fs::File {
        fs::OpenOptions::new()
            .read(true)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
            .open(path)
            .unwrap()
    }

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "wakeshot-io-thread-{tag}-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Lets the test read the `FakeNotifier` the thread owns.
    struct SharedNotifier(Arc<Mutex<FakeNotifier>>);

    impl Notifier for SharedNotifier {
        fn show(&self, notice: &Notice) -> Result<(), WinError> {
            self.0.lock().unwrap().show(notice)
        }
    }

    /// One IO thread's surroundings: an executor over `FakePlans`, a temp data folder, and recorders.
    struct Rig {
        dir: TempDir,
        fake: FakePlans,
        names: Names,
        liveness: Arc<FakeLiveness>,
        queue: Arc<IoQueue>,
        sink: Arc<FakeEventSink>,
        notifier: Arc<Mutex<FakeNotifier>>,
        opened: Arc<Mutex<Vec<OsString>>>,
        open_fails: Option<WinError>,
        log: Arc<MemoryLog>,
        ack: Ack,
    }

    impl Rig {
        fn new(tag: &str) -> Self {
            Self::with_sink(tag, FakeEventSink::default())
        }

        fn with_sink(tag: &str, sink: FakeEventSink) -> Self {
            Self {
                dir: TempDir::new(tag),
                fake: FakePlans::new(user_values()),
                names: Names::test(&format!("io-thread-{tag}")),
                liveness: Arc::default(),
                queue: Arc::default(),
                sink: Arc::new(sink),
                notifier: Arc::default(),
                opened: Arc::default(),
                open_fails: None,
                log: Arc::default(),
                ack: Ack::new(),
            }
        }

        fn data(&self) -> DataDir {
            DataDir::at(self.dir.0.clone())
        }

        /// As the tray builds it, over the fake for both seams.
        fn executor(&self) -> PowerExecutor<FakePlans, FakePlans> {
            PowerExecutor::new(
                self.fake.clone(),
                self.fake.clone(),
                ME,
                self.names.clone(),
                self.data(),
                self.liveness.clone(),
                self.log.clone(),
            )
        }

        fn start(&self) -> IoThread {
            let opened = Arc::clone(&self.opened);
            let fails = self.open_fails;
            let open: Opener = Box::new(move |target: &OsStr| {
                opened.lock().unwrap().push(target.to_owned());
                fails.map_or(Ok(()), Err)
            });
            IoThread::start(IoParts {
                queue: Arc::clone(&self.queue),
                power: Box::new(self.executor()),
                dir: self.data(),
                notifier: Box::new(SharedNotifier(Arc::clone(&self.notifier))),
                open,
                log: self.log.clone(),
                sink: self.sink.clone(),
                ack: self.ack.clone(),
            })
            .expect("the IO thread starts")
        }

        /// Starts a thread on what is queued, lets `finish` run all of it, and returns what this thread posted.
        fn finish_now(&self) -> Vec<Event> {
            let before = self.sink.events().len();
            assert!(self.start().finish(FINISH_WAIT));
            self.sink.events().split_off(before)
        }

        fn run(&self, commands: Vec<Command>) -> Vec<Event> {
            for command in commands {
                self.queue.push(command);
            }
            self.finish_now()
        }

        fn lines(&self) -> Vec<(Level, &'static str, String)> {
            self.log.lines.lock().unwrap().clone()
        }

        /// This module's own lines at `level`.
        fn io_lines(&self, level: Level) -> Vec<String> {
            self.lines()
                .into_iter()
                .filter(|(l, area, _)| *l == level && *area == AREA)
                .map(|(_, _, text)| text)
                .collect()
        }

        fn has_core_line(&self, text: &str) -> bool {
            self.lines()
                .contains(&(Level::Info, "core", text.to_owned()))
        }

        fn read(&self, file: DataFile) -> Option<Vec<u8>> {
            store::read_whole(&self.data().file(file)).unwrap()
        }

        fn shown(&self) -> Vec<Notice> {
            self.notifier.lock().unwrap().shown.borrow().clone()
        }
    }

    /// What a fresh executor over the same start answers to `commands`, run directly on this thread.
    fn direct(tag: &str, journal: FakeJournal, commands: &[Command]) -> Vec<Event> {
        let rig = Rig::new(tag);
        rig.fake.state().journal = journal;
        let mut executor = rig.executor();
        commands.iter().flat_map(|c| executor.run(c)).collect()
    }

    #[test]
    fn every_command_answers_in_queue_order() {
        let rig = Rig::new("every");
        let data = rig.data();
        store::write_atomic(&data.file(DataFile::RestoreRequest), br#"{"version":1}"#).unwrap();
        store::write_atomic(&data.holds().join("run-42.json"), b"{}").unwrap();
        let power = [Command::ReadPlan, apply(), restore(RestoreWhy::WorkEnded)];
        let mut commands = power.to_vec();
        commands.extend([
            Command::WriteState(state(&["first"])),
            Command::WriteConfig(Config::new(false)),
            Command::WriteHold {
                id: id("keep-awake.json"),
                hold: hold("Keep awake"),
            },
            Command::DeleteHold(id("run-42.json")),
            Command::FinishRestoreRequest(state(&["second"])),
            Command::Notify(Notice::RestoreFailed),
            Command::Log("hold ended: build".to_owned()),
            Command::OpenLog,
        ]);

        let events = rig.run(commands);

        let mut expected = direct("every-direct", FakeJournal::Missing, &power);
        assert!(
            matches!(
                expected.as_slice(),
                [
                    Event::Plan(Ok(_)),
                    Event::Done(Outcome::Applied { result: Ok(()), .. }),
                    Event::Done(Outcome::Restored { result: Ok(()), .. }),
                ]
            ),
            "{expected:?}"
        );
        expected.extend([
            written(FileKind::State, Ok(())),
            written(FileKind::Config, Ok(())),
            written(FileKind::Hold(id("keep-awake.json")), Ok(())),
            written(FileKind::Hold(id("run-42.json")), Ok(())),
            written(FileKind::RestoreRequest, Ok(())),
            Event::Done(Outcome::Notified(Ok(()))),
        ]);
        assert_eq!(events, expected);

        // The files as the commands left them.
        let now: StateFile = serde_json::from_slice(&rig.read(DataFile::State).unwrap()).unwrap();
        assert_eq!(now, state(&["second"]));
        let config: Config = serde_json::from_slice(&rig.read(DataFile::Config).unwrap()).unwrap();
        assert_eq!(config, Config::new(false));
        let keep: HoldFile =
            serde_json::from_slice(&rig.read(DataFile::KeepAwake).unwrap()).unwrap();
        assert_eq!(keep, hold("Keep awake"));
        assert!(!data.holds().join("run-42.json").exists());
        assert_eq!(rig.read(DataFile::RestoreRequest), None);

        assert_eq!(rig.shown(), [Notice::RestoreFailed]);
        assert_eq!(
            *rig.opened.lock().unwrap(),
            [data.logs().join("wakeshot.log").into_os_string()]
        );
        assert!(rig.has_core_line("hold ended: build"));
        let lines = rig.lines();
        assert!(lines.iter().all(|(l, ..)| *l == Level::Info), "{lines:?}");
        assert!(!rig.ack.wait(0));
    }

    #[test]
    fn power_events_post_in_the_order_the_executor_gives_them() {
        let rig = Rig::new("corrupt");
        rig.fake.state().journal = FakeJournal::Corrupt;
        let commands = [restore(RestoreWhy::UserAsked), Command::ReadPlan];

        let events = rig.run(commands.to_vec());

        let expected = direct("corrupt-direct", FakeJournal::Corrupt, &commands);
        assert!(
            matches!(
                expected.as_slice(),
                [
                    Event::Done(Outcome::Restored { .. }),
                    Event::JournalCorrupt,
                    Event::Plan(Ok(_)),
                ]
            ),
            "{expected:?}"
        );
        assert_eq!(events, expected);
    }

    #[test]
    fn finish_restore_request_writes_the_state_then_removes_the_request() {
        let rig = Rig::new("finish-ok");
        let data = rig.data();
        store::write_atomic(&data.file(DataFile::RestoreRequest), br#"{"version":1}"#).unwrap();

        let events = rig.run(vec![Command::FinishRestoreRequest(state(&["a"]))]);

        assert_eq!(events, [written(FileKind::RestoreRequest, Ok(()))]);
        assert_eq!(rig.read(DataFile::RestoreRequest), None);
        let now: StateFile = serde_json::from_slice(&rig.read(DataFile::State).unwrap()).unwrap();
        assert_eq!(now, state(&["a"]));

        // The request is gone already (the CLI gave up and removed it): still answered `Ok`, the state still written.
        let events = rig.run(vec![Command::FinishRestoreRequest(state(&["b"]))]);
        assert_eq!(events, [written(FileKind::RestoreRequest, Ok(()))]);
        let now: StateFile = serde_json::from_slice(&rig.read(DataFile::State).unwrap()).unwrap();
        assert_eq!(now, state(&["b"]));
        assert!(rig.io_lines(Level::Warn).is_empty());
    }

    #[test]
    fn failed_state_write_keeps_the_restore_request() {
        let rig = Rig::new("finish-fail");
        let data = rig.data();
        let request = br#"{"version":1}"#;
        store::write_atomic(&data.file(DataFile::RestoreRequest), request).unwrap();
        store::write_atomic(&data.file(DataFile::State), b"old").unwrap();
        let blocker = hold_open(&data.file(DataFile::State));

        let events = rig.run(vec![Command::FinishRestoreRequest(state(&["a"]))]);
        drop(blocker);

        let failure = Failure {
            call: "MoveFileExW",
            code: ERROR_ACCESS_DENIED.0,
        };
        assert_eq!(events, [written(FileKind::RestoreRequest, Err(failure))]);
        assert_eq!(
            rig.read(DataFile::RestoreRequest).as_deref(),
            Some(&request[..])
        );
        assert_eq!(rig.read(DataFile::State).as_deref(), Some(&b"old"[..]));
        let warned = rig.io_lines(Level::Warn);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(
            warned[0].contains("restore-request.json kept"),
            "{warned:?}"
        );
    }

    /// Core round B, Needs another card 1: a newer version's fields survive the menu's Automatic toggle.
    #[test]
    fn write_config_never_writes_over_a_newer_file() {
        let rig = Rig::new("config-newer");
        let newer = br#"{ "version": 2, "automatic": true, "theme": "dark" }"#;
        store::write_atomic(&rig.data().file(DataFile::Config), newer).unwrap();

        let events = rig.run(vec![Command::WriteConfig(Config::new(false))]);

        assert_eq!(events, [written(FileKind::Config, Err(NEWER_CONFIG_KEPT))]);
        assert_eq!(rig.read(DataFile::Config).unwrap(), newer);
        assert_eq!(
            rig.io_lines(Level::Warn),
            ["config.json not written: version 2 is newer than 1; Automatic off for this run only"]
        );
    }

    #[test]
    fn write_config_replaces_a_bad_or_older_file_and_keeps_one_that_does_not_read() {
        for old in [
            &b"{ not json"[..],
            br#"{ "version": 0, "automatic": true }"#,
        ] {
            let rig = Rig::new("config-bad");
            store::write_atomic(&rig.data().file(DataFile::Config), old).unwrap();
            let events = rig.run(vec![Command::WriteConfig(Config::new(false))]);
            assert_eq!(events, [written(FileKind::Config, Ok(()))]);
            let config: Config =
                serde_json::from_slice(&rig.read(DataFile::Config).unwrap()).unwrap();
            assert_eq!(config, Config::new(false));
        }

        // A folder in the file's place makes the read fail: whether it is newer cannot be known, so it stays.
        let rig = Rig::new("config-unread");
        let path = rig.data().file(DataFile::Config);
        fs::create_dir(&path).unwrap();
        let events = rig.run(vec![Command::WriteConfig(Config::new(true))]);
        assert!(
            matches!(
                events.as_slice(),
                [Event::Done(Outcome::Written {
                    file: FileKind::Config,
                    result: Err(_),
                })]
            ),
            "{events:?}"
        );
        assert!(path.is_dir());
        let warned = rig.io_lines(Level::Warn);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(
            warned[0].starts_with("config.json not written: "),
            "{warned:?}"
        );
    }

    #[test]
    fn delete_hold_of_a_missing_file_answers_ok() {
        let rig = Rig::new("delete-missing");
        // No `holds` folder at all, then the folder without the file.
        let mut events = rig.run(vec![Command::DeleteHold(id("run-7.json"))]);
        fs::create_dir_all(rig.data().holds()).unwrap();
        events.extend(rig.run(vec![Command::DeleteHold(id("run-7.json"))]));

        assert_eq!(
            events,
            vec![written(FileKind::Hold(id("run-7.json")), Ok(())); 2]
        );
        assert!(rig.io_lines(Level::Warn).is_empty());
    }

    #[test]
    fn hold_ids_that_are_not_bare_file_names_are_refused() {
        let rig = Rig::new("bare");
        let data = rig.data();
        store::write_atomic(&data.file(DataFile::Config), b"kept").unwrap();
        store::write_atomic(&data.holds().join("x.json"), b"kept").unwrap();
        let inside_temp = data.root().join("abs.json").to_string_lossy().into_owned();
        let bad = [
            r"..\config.json".to_owned(),
            r"sub\x.json".to_owned(),
            r"x.json\".to_owned(),
            String::new(),
            inside_temp,
        ];
        let mut commands = Vec::new();
        for name in &bad {
            commands.push(Command::DeleteHold(id(name)));
            commands.push(Command::WriteHold {
                id: id(name),
                hold: hold("x"),
            });
        }

        let events = rig.run(commands);

        let refused = Failure {
            call: "hold file name check",
            code: ERROR_INVALID_NAME.0,
        };
        let expected: Vec<Event> = bad
            .iter()
            .flat_map(|name| vec![written(FileKind::Hold(id(name)), Err(refused)); 2])
            .collect();
        assert_eq!(events, expected);
        assert_eq!(rig.read(DataFile::Config).as_deref(), Some(&b"kept"[..]));
        assert_eq!(fs::read(data.holds().join("x.json")).unwrap(), b"kept");
        assert!(!data.holds().join("sub").exists());
        assert!(!data.root().join("abs.json").exists());
        assert_eq!(rig.io_lines(Level::Warn).len(), bad.len() * 2);
    }

    #[test]
    fn a_failed_post_is_logged_once_and_the_next_command_runs() {
        let full = WinError {
            call: "PostMessageW",
            code: ERROR_NOT_ENOUGH_QUOTA.0,
        };
        let sink = FakeEventSink {
            fail_with: Some(full),
            ..FakeEventSink::default()
        };
        let rig = Rig::with_sink("post-fails", sink);

        let events = rig.run(vec![
            Command::WriteConfig(Config::new(false)),
            Command::Log("next".to_owned()),
        ]);

        assert!(events.is_empty());
        let warned = rig.io_lines(Level::Warn);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(
            warned[0].contains("WriteConfig") && warned[0].contains("1816"),
            "{warned:?}"
        );
        assert!(rig.has_core_line("next"));
        assert!(rig.read(DataFile::Config).is_some());
    }

    #[test]
    fn posts_after_the_loop_window_went_log_one_line() {
        let gone = WinError {
            call: "PostMessageW",
            code: ERROR_INVALID_WINDOW_HANDLE.0,
        };
        let sink = FakeEventSink {
            fail_with: Some(gone),
            ..FakeEventSink::default()
        };
        let rig = Rig::with_sink("loop-gone", sink);

        rig.run(vec![
            Command::WriteState(state(&[])),
            restore(RestoreWhy::Quit),
            Command::DeleteHold(id("keep-awake.json")),
        ]);

        let about_posts: Vec<_> = rig
            .lines()
            .into_iter()
            .filter(|(_, area, text)| *area == AREA && text.contains("1400"))
            .collect();
        assert_eq!(about_posts.len(), 1, "{about_posts:?}");
        assert_eq!(about_posts[0].0, Level::Info);
        assert!(rig.io_lines(Level::Warn).is_empty());
    }

    #[test]
    fn only_a_session_end_restore_sets_the_ack() {
        let rig = Rig::new("ack");
        let mut others: Vec<Command> = [
            RestoreWhy::WorkEnded,
            RestoreWhy::UserAsked,
            RestoreWhy::PlanSwitch,
            RestoreWhy::Floor,
            RestoreWhy::Quit,
            RestoreWhy::ConsoleLeft,
            RestoreWhy::Leftover,
        ]
        .map(restore)
        .to_vec();
        others.extend([
            Command::ReadPlan,
            apply(),
            Command::WriteState(state(&[])),
            Command::WriteConfig(Config::new(true)),
            Command::WriteHold {
                id: id("keep-awake.json"),
                hold: hold("x"),
            },
            Command::DeleteHold(id("keep-awake.json")),
            Command::FinishRestoreRequest(state(&[])),
            Command::Notify(Notice::CannotHold),
            Command::Log("a".to_owned()),
            Command::OpenLog,
        ]);

        rig.run(others);
        assert!(!rig.ack.wait(0));

        rig.run(vec![restore(RestoreWhy::SessionEnd)]);
        assert!(rig.ack.wait(0));
    }

    #[test]
    fn session_end_runs_the_restore_first_and_drops_notices_and_applies() {
        let rig = Rig::new("session-end");
        for command in [
            Command::Notify(Notice::RestoreFailed),
            apply(),
            Command::WriteConfig(Config::new(true)),
            Command::Notify(Notice::CannotHold),
            Command::Log("kept".to_owned()),
        ] {
            rig.queue.push(command);
        }
        rig.queue.session_ending();

        let events = rig.finish_now();

        let restored = Event::Done(Outcome::Restored {
            plan: None,
            why: RestoreWhy::SessionEnd,
            result: Ok(()),
            journal: Vec::new(),
        });
        assert_eq!(events, [restored, written(FileKind::Config, Ok(()))]);
        assert!(rig.shown().is_empty());
        let calls = rig.fake.state().calls.clone();
        assert!(
            !calls.iter().any(|c| matches!(c, Call::Write(..))),
            "{calls:?}"
        );
        assert!(rig.has_core_line("kept"));
        assert!(rig.ack.wait(0));
    }

    #[test]
    fn finish_with_an_empty_queue_returns_true_at_once() {
        let rig = Rig::new("finish-empty");
        let io = rig.start();
        // The thread waits in `pop` by then, so `finish` must wake it.
        thread::sleep(Duration::from_millis(50));

        let start = Instant::now();
        assert!(io.finish(FINISH_WAIT));
        let took = start.elapsed();

        assert!(took < Duration::from_secs(2), "{took:?}");
        assert!(rig.sink.events().is_empty());
        assert!(rig.lines().is_empty(), "{:?}", rig.lines());
    }

    #[test]
    fn finish_gives_up_when_the_executor_blocks() {
        let rig = Rig::new("finish-blocked");
        let (release, blocked) = mpsc::channel::<()>();
        // The fake's first call (the journal read before the plan read) waits for the test.
        let hang = move |_: &mut _| {
            let _ = blocked.recv_timeout(Duration::from_secs(30));
        };
        rig.fake.script(0, Action::Run(Box::new(hang)));
        rig.queue.push(Command::ReadPlan);
        let io = rig.start();

        let start = Instant::now();
        assert!(!io.finish(200));
        let took = start.elapsed();

        // A Windows wait can end a few ms before `Instant` says its time is up (196.5 ms seen for 200), so the bound
        // only shows that `finish` waited instead of giving up at once.
        assert!(took >= Duration::from_millis(150), "{took:?}");
        assert!(took < Duration::from_secs(5), "{took:?}");
        let warned = rig.io_lines(Level::Warn);
        assert!(warned.iter().any(|t| t.contains("200 ms")), "{warned:?}");

        // Left running, the thread goes on once the call returns.
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while rig.sink.events().is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let events = rig.sink.events();
        assert!(
            matches!(events.as_slice(), [Event::Plan(Ok(_))]),
            "{events:?}"
        );
    }

    #[test]
    fn a_log_line_never_holds_file_content() {
        let rig = Rig::new("no-content");
        let data = rig.data();
        let commands = || {
            vec![
                Command::WriteConfig(Config::new(false)),
                Command::WriteHold {
                    id: id("keep-awake.json"),
                    hold: hold("secret reason"),
                },
                Command::WriteState(state(&["secret label"])),
            ]
        };
        rig.run(commands());
        // The second round cannot replace the files, so each write fails and is logged.
        let blockers = [DataFile::Config, DataFile::KeepAwake, DataFile::State]
            .map(|file| hold_open(&data.file(file)));

        let events = rig.run(commands());
        drop(blockers);

        assert!(
            events
                .iter()
                .all(|e| matches!(e, Event::Done(Outcome::Written { result: Err(_), .. }))),
            "{events:?}"
        );
        let lines = rig.lines();
        assert_eq!(rig.io_lines(Level::Warn).len(), 3, "{lines:?}");
        for (_, _, text) in &lines {
            let content = ["secret", "automatic", "version", "{"];
            assert!(!content.iter().any(|c| text.contains(c)), "{text}");
        }
    }

    #[test]
    fn loop_commands_are_logged_as_wiring_bugs_and_dropped() {
        let rig = Rig::new("wiring");
        let tray = TrayView {
            icon: IconState::Idle,
            tooltip: "Wakeshot".to_owned(),
            menu: MenuModel::default(),
        };
        let strays = vec![
            Command::Requests {
                system: true,
                display: false,
            },
            Command::Retake,
            Command::Lock,
            Command::Hibernate {
                reason: CountdownReason::WorkEnded,
                floor_pct: 10,
                last_resume_mono_ms: None,
            },
            Command::Tray(tray),
            Command::Play(Cue::GoingToSleep),
            Command::ApplyUpdate,
            Command::Quit,
        ];
        let count = strays.len();
        for command in strays {
            rig.queue.push(command);
        }
        rig.queue.push(Command::Log("after".to_owned()));
        let io = rig.start();
        // A stray `Quit` that is still queued when `finish` starts would count as its marker, so the test waits.
        let deadline = Instant::now() + Duration::from_secs(10);
        while !rig.has_core_line("after") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(io.finish(FINISH_WAIT));

        assert!(rig.sink.events().is_empty());
        let errors = rig.io_lines(Level::Error);
        assert_eq!(errors.len(), count, "{errors:?}");
        assert!(errors.iter().all(|t| t.starts_with("wiring bug: ")));
        assert!(rig.has_core_line("after"));
        assert!(rig.fake.state().calls.is_empty());
    }

    #[test]
    fn open_log_failure_is_one_log_line() {
        let mut rig = Rig::new("open-fails");
        rig.open_fails = Some(WinError {
            call: "ShellExecuteExW",
            code: 1155,
        });

        let events = rig.run(vec![Command::OpenLog]);

        assert!(events.is_empty());
        let warned = rig.io_lines(Level::Warn);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(
            warned[0].contains("ShellExecuteExW failed with 1155"),
            "{warned:?}"
        );
        assert_eq!(rig.opened.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_notice_that_fails_answers_the_failure() {
        let rig = Rig::new("notify-fails");
        let refused = WinError {
            call: "Shell_NotifyIconW",
            code: ERROR_ACCESS_DENIED.0,
        };
        rig.notifier.lock().unwrap().fail_with = Some(refused);

        let events = rig.run(vec![Command::Notify(Notice::LockFailed)]);

        assert_eq!(
            events,
            [Event::Done(Outcome::Notified(Err(refused.into())))]
        );
        let warned = rig.io_lines(Level::Warn);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(warned[0].contains("LockFailed"), "{warned:?}");
    }

    #[test]
    fn the_real_opener_fits() {
        // A type check only: nothing is called, so no window opens.
        let _open: Opener = Box::new(crate::shell::open_with_default_app);
    }

    #[test]
    fn dropping_the_thread_runs_its_queue() {
        let rig = Rig::new("drop");
        let io = rig.start();
        rig.queue.push(Command::WriteConfig(Config::new(true)));

        drop(io);

        assert_eq!(rig.sink.events(), [written(FileKind::Config, Ok(()))]);
    }
}
