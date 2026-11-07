//! The watch thread (architecture section 4, "Tray threads"): one thread that waits on folder notices, process exits,
//! the loop's safety tick and a stop event, reads what changed through [`data`] and [`agents`], and posts
//! `ConfigChanged`, `Holds`, `Sources` and `Menu(Restore)` to the loop through the `EventSink`.
//!
//! Start it after `Started` is posted: its first round posts at once. Call [`WatchHandle::tick`] on each safety tick:
//! the thread has no timer of its own beyond the 200 ms retries and the query runs a source's database changes held
//! back ([`agents::QUERY_EVERY_MS`]). It makes no power call and opens only the data folder's files and the files a
//! source asks for; exit waits open processes with wait and query rights only.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use wakeshot_core::event::Event;
use wakeshot_core::files::Config;
use wakeshot_core::holds::HoldFacts;
use wakeshot_core::process::ProcessRef;
use wakeshot_core::sources::AgentSource;
use wakeshot_core::view::MenuCommand;
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_WINDOW_HANDLE, ERROR_PATH_NOT_FOUND, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};

use crate::WinError;
use crate::clock;
use crate::log::{Level, Log};
use crate::process::{ExitWait, Liveness, ProcessHandle};
use crate::sink::EventSink;
use crate::store::{DataDir, DataFile};
use crate::watch::agents::{self, Agents, Retry};
use crate::watch::data::{self, DataFolder};
use crate::watch::dir::{Changed, FolderWatch, State, Woke};

const AREA: &str = "watch";

/// How long [`WatchHandle::stop`] waits for the thread. Stop wins every wait, so only a read in progress delays it; a
/// thread still busy after this is left to end on its own, so the tray's shutdown never hangs on it.
const STOP_WAIT_MS: u32 = 2_000;

/// After a failed wait the thread pauses this long (a stop still ends the pause), then reads everything as on a tick,
/// so a wait that keeps failing never spins.
const PAUSE_MS: u32 = 1_000;

/// How many times one call follows "a feed asked for a new folder, and its first read asked for another". Claude asks
/// for one folder; the bound only keeps a source whose wants never settle from holding the thread.
const FOLDER_ROUNDS: usize = 8;

/// Positions in the wait's extra handles. Stop comes first, so it wins over everything else.
const STOP: usize = 0;
const TICK: usize = 1;

/// Starts the watch thread.
pub struct WatchThread;

impl WatchThread {
    /// Starts the thread with every source; its first round reads everything and posts `ConfigChanged` (only when
    /// `config.json` differs from `config`), `Holds` and one `Sources` per source, then `Menu(Restore)` if a restore
    /// request is waiting.
    ///
    /// `config` is the value the core has: the one `Started` carried, so a file changed between that read and this
    /// one is still posted. `liveness` checks every session and hold; a live one also gets an exit wait, so its end is
    /// posted at once.
    pub fn start(
        dir: DataDir,
        config: Config,
        sources: Vec<Box<dyn AgentSource + Send>>,
        liveness: Arc<dyn Liveness>,
        sink: Arc<dyn EventSink>,
        log: Arc<dyn Log>,
    ) -> Result<WatchHandle, WinError> {
        let agents = Agents::new(sources, Arc::clone(&log));
        Self::spawn(dir, config, agents, liveness, sink, log)
    }

    /// [`start`](Self::start) with `agents` made by the caller, so a test gives its I/O.
    fn spawn(
        dir: DataDir,
        config: Config,
        agents: Agents,
        liveness: Arc<dyn Liveness>,
        sink: Arc<dyn EventSink>,
        log: Arc<dyn Log>,
    ) -> Result<WatchHandle, WinError> {
        let signals = Arc::new(Signals::new()?);
        let inner = Arc::clone(&signals);
        let thread_log = Arc::clone(&log);
        let thread = std::thread::Builder::new()
            .name("wakeshot-watch".to_owned())
            .spawn(move || {
                Watcher::new(dir, config, agents, liveness, sink, thread_log, inner).run();
            })
            .map_err(|e| WinError {
                call: "CreateThread",
                code: e.raw_os_error().map_or(u32::MAX, |c| c as u32),
            })?;
        Ok(WatchHandle {
            signals,
            thread: Some(thread),
            log,
        })
    }
}

/// The running thread. Dropping it stops the thread like [`stop`](Self::stop).
pub struct WatchHandle {
    signals: Arc<Signals>,
    thread: Option<JoinHandle<()>>,
    log: Arc<dyn Log>,
}

impl WatchHandle {
    /// The loop's safety tick: the thread reads every source and hold in full, checks their liveness, retries missing
    /// folders, and posts again what a failed post lost. Never blocks; ticks that come while one runs count once.
    pub fn tick(&self) {
        Signals::set(&self.signals.tick);
    }

    /// Ends the thread and waits for it, at most 2 s. The thread drops every exit wait, then every folder handle
    /// (each read cancelled and waited for). `false`: it did not end in time (it is left to end on its own) or it
    /// panicked; both are logged.
    pub fn stop(mut self) -> bool {
        self.end()
    }

    fn end(&mut self) -> bool {
        let Some(thread) = self.thread.take() else {
            return true;
        };
        Signals::set(&self.signals.stop);
        // SAFETY: the join handle owns the thread's handle and lives through the call.
        let waited = unsafe { WaitForSingleObject(HANDLE(thread.as_raw_handle()), STOP_WAIT_MS) };
        if waited != WAIT_OBJECT_0 {
            self.log.line(
                Level::Warn,
                AREA,
                "the watch thread did not stop within 2 s, so it is left to end on its own",
            );
            return false;
        }
        if thread.join().is_err() {
            self.log
                .line(Level::Error, AREA, "the watch thread ended with a panic");
            return false;
        }
        true
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.end();
    }
}

// The loop owns the handle and integration may share it; a field that is not `Send` and `Sync` fails here instead.
const _: () = {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<WatchHandle>();
};

/// The thread's events, shared with the handle and with each exit wait's callback.
struct Signals {
    /// Manual reset: once set it stays set, so no later wait can miss it.
    stop: OwnedHandle,
    /// Auto reset, as the folder watch wants its extra handles: one that stayed set would win every wait.
    tick: OwnedHandle,
    /// Auto reset. Set by an exit wait's callback, which runs on a pool thread and must do nothing else.
    wake: OwnedHandle,
}

impl Signals {
    fn new() -> Result<Self, WinError> {
        Ok(Self {
            stop: event(true)?,
            tick: event(false)?,
            wake: event(false)?,
        })
    }

    fn set(event: &OwnedHandle) {
        // SAFETY: the event is open for as long as the `Signals` that holds it. A set can fail only for a bad handle,
        // and a pool callback must not panic, so the result is dropped.
        let _ = unsafe { SetEvent(raw(event)) };
    }
}

fn event(manual_reset: bool) -> Result<OwnedHandle, WinError> {
    // SAFETY: no security attributes and no name are passed.
    let raw = unsafe { CreateEventW(None, manual_reset, false, None) }.map_err(|e| WinError {
        call: "CreateEventW",
        code: e.code().0 as u32,
    })?;
    // SAFETY: CreateEventW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw.0) })
}

fn raw(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

/// A folder that is not there (yet), as `FolderWatch` gives it: an HRESULT.
fn not_made_yet(e: &WinError) -> bool {
    [ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND]
        .iter()
        .any(|code| e.code == code.to_hresult().0 as u32)
}

/// Everything the thread owns. Fields drop in order: the exit waits before the folder watch, so no callback can wake
/// a thread whose watch is being taken down.
struct Watcher {
    waits: BTreeMap<ProcessRef, Option<ExitWait>>,
    /// Made here, on the thread that waits on it: a read ends when the thread that queued it ends.
    watch: FolderWatch,
    root: PathBuf,
    holds_dir: PathBuf,
    restore_file: PathBuf,
    data: DataFolder,
    agents: Agents,
    /// The agent folders in the watch, with their subtree flag.
    agent_folders: BTreeMap<PathBuf, bool>,
    retries: VecDeque<(Instant, Vec<Retry>)>,
    holds_retry: Option<Instant>,
    holds: Vec<HoldFacts>,
    holds_live: Vec<ProcessRef>,
    holds_posted: Option<Vec<HoldFacts>>,
    /// The last value posted, or meant to be: what the core should have.
    config: Config,
    config_due: bool,
    restore_due: bool,
    restore_unsent: bool,
    /// A post failed, so the next tick posts everything again.
    resync: bool,
    post_failing: bool,
    wait_failing: bool,
    full_logged: bool,
    liveness: Arc<dyn Liveness>,
    sink: Arc<dyn EventSink>,
    log: Arc<dyn Log>,
    signals: Arc<Signals>,
}

impl Watcher {
    fn new(
        dir: DataDir,
        config: Config,
        agents: Agents,
        liveness: Arc<dyn Liveness>,
        sink: Arc<dyn EventSink>,
        log: Arc<dyn Log>,
        signals: Arc<Signals>,
    ) -> Self {
        Self {
            waits: BTreeMap::new(),
            watch: FolderWatch::new(),
            root: dir.root().to_path_buf(),
            holds_dir: dir.holds(),
            restore_file: dir.file(DataFile::RestoreRequest),
            data: DataFolder::new(dir, config.clone(), Arc::clone(&log)),
            agents,
            agent_folders: BTreeMap::new(),
            retries: VecDeque::new(),
            holds_retry: None,
            holds: Vec::new(),
            holds_live: Vec::new(),
            holds_posted: None,
            config,
            config_due: false,
            restore_due: false,
            restore_unsent: false,
            resync: false,
            post_failing: false,
            wait_failing: false,
            full_logged: false,
            liveness,
            sink,
            log,
            signals,
        }
    }

    fn run(mut self) {
        let signals = Arc::clone(&self.signals);
        let extra = [raw(&signals.stop), raw(&signals.tick), raw(&signals.wake)];
        self.first_round();
        loop {
            let timeout = self
                .next_due()
                .map(|due| due.saturating_duration_since(Instant::now()));
            let woke = self.watch.wait(timeout, &extra);
            if woke.is_ok() {
                self.wait_failing = false;
            }
            let poll_only = matches!(woke, Ok(Woke::Timeout)) && self.only_queries_due();
            let changes = self.agents.changes();
            match woke {
                Ok(Woke::Extra(STOP)) => break,
                Ok(Woke::Extra(TICK)) => self.tick(),
                // A process ended. Sessions are checked again in `publish`, holds here.
                Ok(Woke::Extra(_)) => self.read_holds(),
                Ok(Woke::Changed(changes)) => self.changed(changes),
                Ok(Woke::Timeout) => {}
                Err(e) => {
                    if self.pause(e) {
                        break;
                    }
                    self.tick();
                }
            }
            self.run_due();
            // A `Query` poll that found no commit read nothing, so there is nothing to post; the process list a
            // snapshot asks for costs about 14 ms here, each 2 s.
            if poll_only && self.agents.changes() == changes {
                continue;
            }
            self.publish();
        }
    }

    /// No retry and no holds read is due: a timeout now is a `Query` run only.
    fn only_queries_due(&self) -> bool {
        let now = Instant::now();
        let due = |at: Option<Instant>| at.is_some_and(|at| at <= now);
        !due(self.retries.front().map(|(at, _)| *at)) && !due(self.holds_retry)
    }

    fn first_round(&mut self) {
        // Each folder is watched before its first full read, so a change in between is not lost. A folder that cannot
        // be opened waits on the retry list; one not made yet (`holds\` before the first hold file) is the normal
        // case and says nothing.
        let own = [
            (self.root.clone(), "the data folder"),
            (self.holds_dir.clone(), "the holds folder"),
        ];
        for (dir, name) in own {
            let e = match self.watch.add(&dir, false) {
                Ok(State::Watching) => continue,
                Ok(State::Missing(e)) if not_made_yet(&e) => continue,
                Ok(State::Missing(e)) | Err(e) => e,
            };
            self.warn(&format!(
                "{name} is not watched ({} error {}), so its changes are read on each tick",
                e.call, e.code
            ));
        }
        self.sync_folders(false);
        let retries = self.agents.on_tick();
        self.schedule(retries);
        self.sync_folders(true);
        self.read_holds();
        self.read_config();
        self.read_restore();
        self.publish();
    }

    fn tick(&mut self) {
        if std::mem::take(&mut self.resync) {
            self.holds_posted = None;
            self.agents.forget_posted();
            self.config_due = true;
            // A request whose post was lost still waits on disk, unless someone took it away since.
            if std::mem::take(&mut self.restore_unsent) && self.restore_file.is_file() {
                self.restore_due = true;
            }
        }
        let back = self.watch.retry_missing();
        self.changed(back);
        let retries = self.agents.on_tick();
        self.schedule(retries);
        self.sync_folders(true);
        self.read_holds();
        self.read_config();
        self.read_restore();
    }

    fn changed(&mut self, changes: Vec<Changed>) {
        let mut queue = VecDeque::from(changes);
        while let Some(change) = queue.pop_front() {
            let folder = change.folder();
            if folder == self.root {
                // Any name: a short 8.3 name or a lost notice must not hide a change, and both files are tiny.
                self.read_config();
                self.read_restore();
                // A folder made since (`holds\` at the first `wakeshot run`) is watched now, not at the next tick.
                queue.extend(self.watch.retry_missing());
            }
            if folder == self.holds_dir {
                self.read_holds();
            }
            if self.agent_folders.contains_key(folder) {
                let names = match &change {
                    Changed::Names { names, .. } => Some(names.as_slice()),
                    Changed::Rescan(_) => None,
                };
                let retries = self.agents.on_change(folder, names);
                self.schedule(retries);
                self.sync_folders(true);
            }
        }
    }

    /// Makes the watch match `Agents::folders`. `read_new`: read each folder that just opened in full, since the call
    /// that asked for it read it before its watch existed.
    fn sync_folders(&mut self, read_new: bool) {
        for _ in 0..FOLDER_ROUNDS {
            let wanted: BTreeMap<PathBuf, bool> = self
                .agents
                .folders()
                .into_iter()
                .map(|f| (f.dir, f.subtree))
                .collect();
            let left: Vec<PathBuf> = self
                .agent_folders
                .keys()
                .filter(|dir| !wanted.contains_key(*dir))
                .cloned()
                .collect();
            for dir in left {
                self.agent_folders.remove(&dir);
                if dir != self.root && dir != self.holds_dir {
                    self.watch.remove(&dir);
                }
            }
            let mut opened = Vec::new();
            for (dir, subtree) in wanted {
                if self.agent_folders.get(&dir) == Some(&subtree) {
                    continue;
                }
                match self.watch.add(&dir, subtree) {
                    Ok(state) => {
                        if state == State::Watching {
                            opened.push(dir.clone());
                        }
                        self.agent_folders.insert(dir, subtree);
                    }
                    // Still read on every tick, only without notices.
                    Err(e) => {
                        if !std::mem::replace(&mut self.full_logged, true) {
                            self.warn(&format!(
                                "a folder a source reads is not watched, the watch is full ({} error {}); it is read on each tick",
                                e.call, e.code
                            ));
                        }
                    }
                }
            }
            if !read_new || opened.is_empty() {
                return;
            }
            for dir in opened {
                let retries = self.agents.on_change(&dir, None);
                self.schedule(retries);
            }
        }
    }

    fn schedule(&mut self, retries: Vec<Retry>) {
        if !retries.is_empty() {
            let due = Instant::now() + Duration::from_millis(agents::RETRY_MS);
            self.retries.push_back((due, retries));
        }
    }

    fn next_due(&self) -> Option<Instant> {
        [
            self.retries.front().map(|(due, _)| *due),
            self.holds_retry,
            self.agents.next_due(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    fn run_due(&mut self) {
        let now = Instant::now();
        if self.agents.next_due().is_some_and(|due| due <= now) {
            let retries = self.agents.run_due();
            self.schedule(retries);
            self.sync_folders(true);
        }
        while self.retries.front().is_some_and(|(due, _)| *due <= now) {
            let Some((_, batch)) = self.retries.pop_front() else {
                break;
            };
            let again = self.agents.retry(&batch);
            self.schedule(again);
            self.sync_folders(true);
        }
        if self.holds_retry.is_some_and(|due| due <= now) {
            self.read_holds();
        }
    }

    fn read_holds(&mut self) {
        let read = self.data.holds(&*self.liveness);
        // Any read counts as the retry (`DataFolder::holds`), so only this read's list sets the next one.
        self.holds_retry = (!read.retry.is_empty())
            .then(|| Instant::now() + Duration::from_millis(data::RETRY_MS));
        self.holds = read.holds;
        self.holds_live = read.live;
    }

    fn read_config(&mut self) {
        if let Some(Event::ConfigChanged(config)) = self.data.config() {
            self.config = config;
            self.config_due = true;
        }
    }

    fn read_restore(&mut self) {
        if self.data.restore_request().is_some() {
            self.restore_due = true;
        }
    }

    /// Posts what changed. The config goes first, so the core never acts on a hold that a new Automatic setting turns
    /// off; the restore goes last, after the full picture.
    fn publish(&mut self) {
        if std::mem::take(&mut self.config_due) {
            self.post(Event::ConfigChanged(self.config.clone()));
        }
        if self.holds_posted.as_ref() != Some(&self.holds) {
            self.post(Event::Holds(self.holds.clone()));
            self.holds_posted = Some(self.holds.clone());
        }
        let snapshots = self.agents.snapshots(&*self.liveness, clock::now());
        for event in snapshots.events {
            self.post(event);
        }
        if std::mem::take(&mut self.restore_due) && !self.post(Event::Menu(MenuCommand::Restore)) {
            self.restore_unsent = true;
        }
        self.sync_waits(&snapshots.live);
    }

    /// `false`: the event is lost. One line per run of failures; the next tick posts everything again, so a full
    /// queue is not fed more posts at once.
    fn post(&mut self, event: Event) -> bool {
        match self.sink.post(event) {
            Ok(()) => {
                self.post_failing = false;
                true
            }
            Err(e) => {
                self.resync = true;
                if std::mem::replace(&mut self.post_failing, true) {
                    return false;
                }
                if e.code == ERROR_INVALID_WINDOW_HANDLE.0 {
                    // The loop window is gone for good, as after a quit while a hold file changes (`WindowSink`):
                    // nothing is wrong and no tick follows, so it is no warning, as on the IO thread.
                    let text = format!(
                        "the loop window is gone ({} error {}), as after a quit: this event and every later one are \
                         dropped",
                        e.call, e.code
                    );
                    self.log.line(Level::Info, AREA, &text);
                } else {
                    self.warn(&format!(
                        "an event could not be posted to the loop ({} error {}); the next tick posts everything again",
                        e.call, e.code
                    ));
                }
                false
            }
        }
    }

    /// One exit wait per live session and hold process; a wait whose process is no longer live is dropped.
    fn sync_waits(&mut self, sessions: &[ProcessRef]) {
        let live: BTreeSet<ProcessRef> = sessions.iter().chain(&self.holds_live).copied().collect();
        self.waits.retain(|process, _| live.contains(process));
        for process in live {
            if !self.waits.contains_key(&process) {
                let wait = self.exit_wait(process);
                self.waits.insert(process, wait);
            }
        }
    }

    /// `None` when the process cannot be waited on: its end is then seen at the next tick. It is not tried again
    /// while it stays live, so a liveness answer that disagrees with the open never spins the thread.
    fn exit_wait(&mut self, process: ProcessRef) -> Option<ExitWait> {
        let handle = match ProcessHandle::open(process) {
            Ok(Some(handle)) => handle,
            // It ended after its check said it was alive: check again at once.
            Ok(None) => {
                Signals::set(&self.signals.wake);
                return None;
            }
            Err(e) => {
                self.no_wait(e);
                return None;
            }
        };
        let signals = Arc::clone(&self.signals);
        match ExitWait::register(&handle, Box::new(move || Signals::set(&signals.wake))) {
            Ok(wait) => Some(wait),
            Err(e) => {
                self.no_wait(e);
                None
            }
        }
    }

    fn no_wait(&self, e: WinError) {
        self.warn(&format!(
            "a process exit cannot be waited on ({} error {}), so its end is seen at the next tick",
            e.call, e.code
        ));
    }

    /// `true`: stop was asked during the pause.
    fn pause(&mut self, e: WinError) -> bool {
        if !std::mem::replace(&mut self.wait_failing, true) {
            self.warn(&format!(
                "the wait failed ({} error {}); everything is read again each second",
                e.call, e.code
            ));
        }
        // SAFETY: the event is open for as long as `self.signals`.
        unsafe { WaitForSingleObject(raw(&self.signals.stop), PAUSE_MS) == WAIT_OBJECT_0 }
    }

    fn warn(&self, text: &str) {
        self.log.line(Level::Warn, AREA, text);
    }
}

#[cfg(test)]
mod tests;
