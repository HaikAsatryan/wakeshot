//! The watch thread (architecture section 4, "Tray threads"): one thread that waits on folder notices, process exits,
//! the loop's safety tick and a stop event, reads what changed through [`data`](super::data) and
//! [`agents`](super::agents), and posts `ConfigChanged`, `Holds`, `Sources` and `Menu(Restore)` to the loop through
//! the `EventSink`.
//!
//! Start it after `Started` is posted: its first round posts at once. Call [`WatchHandle::tick`] on each safety tick:
//! the thread has no timer of its own beyond the 200 ms retries. It makes no power call and opens only the data
//! folder's files and the files a source asks for; exit waits open processes with wait and query rights only.
//!
//! Owner: `p4-watch-thread`; fixes by `p4-live-claude`.

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
use windows::Win32::Foundation::{ERROR_INVALID_WINDOW_HANDLE, HANDLE, WAIT_OBJECT_0};
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
        let signals = Arc::new(Signals::new()?);
        let inner = Arc::clone(&signals);
        let thread_log = Arc::clone(&log);
        let thread = std::thread::Builder::new()
            .name("wakeshot-watch".to_owned())
            .spawn(move || {
                Watcher::new(dir, config, sources, liveness, sink, thread_log, inner).run();
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
        sources: Vec<Box<dyn AgentSource + Send>>,
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
            agents: Agents::new(sources, Arc::clone(&log)),
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
            self.publish();
        }
    }

    fn first_round(&mut self) {
        // Each folder is watched before its first full read, so a change in between is not lost. Two adds to an
        // empty watch cannot pass its limit, and a missing folder waits on its retry list.
        for dir in [self.root.clone(), self.holds_dir.clone()] {
            let _ = self.watch.add(&dir, false);
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
        let agents = self.retries.front().map(|(due, _)| *due);
        match (agents, self.holds_retry) {
            (Some(a), Some(h)) => Some(a.min(h)),
            (a, h) => a.or(h),
        }
    }

    fn run_due(&mut self) {
        let now = Instant::now();
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
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use wakeshot_core::holds::{HoldId, HoldKind};
    use wakeshot_core::process::{Alive, FileTime, Lookup};
    use wakeshot_core::sources::{
        self, Activity, Fed, Feed, FileId, FormatHealth, SessionFacts, SessionKey, SourceId,
        WantedFile,
    };
    use wakeshot_core::testing::FakeSource;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

    use super::*;
    use crate::log::MemoryLog;
    use crate::process::FakeLiveness;
    use crate::sink::FakeEventSink;
    use crate::watch::read::TempDir;

    /// Only keeps a broken run from hanging; the card's 1 s limits are asserted where they apply.
    const WITHIN: Duration = Duration::from_secs(10);
    const ONE_SECOND: Duration = Duration::from_secs(1);
    /// Long enough for a notice, a read and a post, many times over.
    const QUIET: Duration = Duration::from_millis(300);

    /// Above any pid Windows hands out in practice, so an exit wait for a made-up session opens nothing real.
    const FAKE_PID: u32 = 3_000_000_000;
    const START: FileTime = FileTime(134_381_211_391_959_336);

    type Hook = Box<dyn FnMut(&mut FakeSource, &FileId) + Send>;

    struct Inner {
        fake: FakeSource,
        after_feed: Option<Hook>,
    }

    /// A `FakeSource` the test still reaches after the thread took it, with a hook that runs after each feed.
    #[derive(Clone)]
    struct Shared(Arc<Mutex<Inner>>);

    impl Shared {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(Inner {
                fake: FakeSource::new(SourceId::ClaudeCode),
                after_feed: None,
            })))
        }

        fn inner(&self) -> MutexGuard<'_, Inner> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn set(&self, f: impl FnOnce(&mut FakeSource)) {
            f(&mut self.inner().fake);
        }

        fn fed(&self) -> Vec<(FileId, Feed)> {
            self.inner().fake.fed.clone()
        }

        fn after_feed(&self, hook: impl FnMut(&mut FakeSource, &FileId) + Send + 'static) {
            self.inner().after_feed = Some(Box::new(hook));
        }

        fn boxed(&self) -> Box<dyn AgentSource + Send> {
            Box::new(self.clone())
        }
    }

    impl AgentSource for Shared {
        fn id(&self) -> SourceId {
            self.inner().fake.id
        }

        fn wanted(&self) -> Vec<WantedFile> {
            self.inner().fake.wanted()
        }

        fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
            let mut inner = self.inner();
            let Inner { fake, after_feed } = &mut *inner;
            let fed = fake.feed(file, data);
            if let Some(hook) = after_feed {
                hook(fake, file);
            }
            fed
        }

        fn sessions(&self) -> Vec<SessionFacts> {
            self.inner().fake.sessions()
        }
    }

    /// A `FakeEventSink` that can fail and recover while the thread runs, and counts every try.
    #[derive(Default)]
    struct Sink {
        fake: FakeEventSink,
        /// The `PostMessageW` error each post fails with; 0 posts.
        failing: AtomicU32,
        tries: AtomicUsize,
    }

    impl EventSink for Sink {
        fn post(&self, event: Event) -> Result<(), WinError> {
            self.tries.fetch_add(1, Ordering::SeqCst);
            let code = self.failing.load(Ordering::SeqCst);
            if code != 0 {
                return Err(WinError {
                    call: "PostMessageW",
                    code,
                });
            }
            self.fake.post(event)
        }
    }

    /// A data folder with `holds\`, an agent folder the source reads, the fakes, and the thread once started.
    struct Rig {
        handle: Option<WatchHandle>,
        data: TempDir,
        agent: TempDir,
        dir: DataDir,
        source: Shared,
        liveness: Arc<FakeLiveness>,
        sink: Arc<Sink>,
        log: Arc<MemoryLog>,
    }

    impl Rig {
        fn new(tag: &str) -> Self {
            let data = TempDir::new(&format!("thread-{tag}"));
            fs::create_dir(data.path().join("holds")).unwrap();
            // Apart from the data folder, so a write to one never gives a notice in the other.
            let agent = TempDir::new(&format!("thread-{tag}-agent"));
            let source = Shared::new();
            let folder = agent.path().to_path_buf();
            source.set(|f| f.wanted = vec![status_folder(&folder)]);
            Self {
                handle: None,
                dir: DataDir::at(data.path().to_path_buf()),
                data,
                agent,
                source,
                liveness: Arc::default(),
                sink: Arc::default(),
                log: Arc::default(),
            }
        }

        fn start(&mut self, config: Config) {
            let handle = WatchThread::start(
                self.dir.clone(),
                config,
                vec![self.source.boxed()],
                self.liveness.clone(),
                self.sink.clone(),
                self.log.clone(),
            )
            .unwrap();
            self.handle = Some(handle);
        }

        /// Starts the thread and waits for its first round of `count` events.
        fn started(&mut self, config: Config, count: usize) {
            self.start(config);
            self.wait_for(Instant::now(), |e| e.len() >= count);
        }

        fn tick(&self) {
            self.handle.as_ref().unwrap().tick();
        }

        /// Session liveness checks so far: a tick checks each live session once, so this shows that a tick ran.
        fn checks(&self) -> usize {
            self.liveness.checked.lock().unwrap().len()
        }

        /// Ticks, then waits until the checks reach `want`, so the tick ran before the test looks at what it posted.
        fn tick_and_wait(&self, want: usize) {
            self.tick();
            until(
                Instant::now(),
                || self.checks() >= want,
                || format!("{} checks, want {want}", self.checks()),
            );
        }

        fn events(&self) -> Vec<Event> {
            self.sink.fake.events()
        }

        /// Waits until `done` holds for the events posted so far; returns the time from `since`.
        fn wait_for(&self, since: Instant, done: impl Fn(&[Event]) -> bool) -> Duration {
            until(
                since,
                || done(&self.events()),
                || format!("{:#?}", self.events()),
            )
        }

        /// What was posted, after a quiet spell.
        fn settled(&self) -> Vec<Event> {
            std::thread::sleep(QUIET);
            self.events()
        }

        /// Writes a data folder file as Wakeshot's own writers do, in one move. A plain write lets a busy thread read
        /// the empty file first and then the full one with a new write time: a restore request counted twice (seen
        /// under 8 test copies at once, 2026-11-04).
        fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
            let path = self.data.path().join(rel);
            crate::store::write_atomic(&path, bytes).unwrap();
            path
        }

        fn hold(&self, name: &str, process: ProcessRef) -> PathBuf {
            self.write(&format!("holds/{name}"), hold_json(process).as_bytes())
        }

        fn live_hold(&self, process: ProcessRef) {
            let lookup = (process.pid, Lookup::Running(process.start));
            self.liveness.starts.lock().unwrap().push(lookup);
        }

        fn live_session(&self, process: ProcessRef) {
            self.source.set(|f| f.sessions.push(session(process)));
            let alive = (process, Alive::Yes);
            self.liveness.alive.lock().unwrap().push(alive);
        }

        fn config(&self, automatic: bool) {
            let json = format!("{{\"version\":1,\"automatic\":{automatic}}}");
            self.write("config.json", json.as_bytes());
        }

        fn lines(&self) -> Vec<String> {
            let lines = self.log.lines.lock().unwrap();
            lines.iter().map(|(_, _, text)| text.clone()).collect()
        }

        fn levels(&self) -> Vec<Level> {
            let lines = self.log.lines.lock().unwrap();
            lines.iter().map(|(level, _, _)| *level).collect()
        }
    }

    impl Drop for Rig {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.stop();
            }
        }
    }

    /// Waits until `done`, then returns the time from `since`; panics with `show()` if it never comes.
    fn until(since: Instant, done: impl Fn() -> bool, show: impl Fn() -> String) -> Duration {
        loop {
            if done() {
                return since.elapsed();
            }
            assert!(since.elapsed() < WITHIN, "never came; so far: {}", show());
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn status_name(name: &str) -> bool {
        name.ends_with(".json")
    }

    fn status_folder(dir: &Path) -> WantedFile {
        WantedFile::Folder {
            dir: dir.to_path_buf(),
            accept: status_name,
        }
    }

    fn fake(n: u32) -> ProcessRef {
        ProcessRef {
            pid: FAKE_PID + 4 * n,
            start: START,
        }
    }

    fn hold_json(process: ProcessRef) -> String {
        format!(
            "{{\"version\":1,\"pid\":{},\"procStart\":\"{}\"}}",
            process.pid, process.start.0
        )
    }

    fn hold_facts(name: &str, alive: Alive) -> HoldFacts {
        HoldFacts {
            id: HoldId(name.to_owned()),
            kind: HoldKind::Script,
            alive,
            expires_wall_ms: None,
            reason: None,
            screen: false,
        }
    }

    fn session(process: ProcessRef) -> SessionFacts {
        SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: process.pid.to_string(),
            },
            liveness: sources::Liveness::Process {
                pid: process.pid,
                start: process.start,
            },
            activity: Activity::Working,
            timers: vec![],
            open_tasks: 0,
            label: "Claude Code",
            format: FormatHealth::Ok,
        }
    }

    fn claude(sessions: Vec<(SessionFacts, Alive)>) -> Event {
        Event::Sources {
            source: SourceId::ClaudeCode,
            sessions,
        }
    }

    fn whole(path: &Path, bytes: &[u8]) -> (FileId, Feed) {
        (FileId(path.to_path_buf()), Feed::Whole(bytes.to_vec()))
    }

    fn count(events: &[Event], want: impl Fn(&Event) -> bool) -> usize {
        events.iter().filter(|e| want(e)).count()
    }

    fn is_holds(event: &Event) -> bool {
        matches!(event, Event::Holds(_))
    }

    const SLEEP_ENV: &str = "WAKESHOT_TEST_WATCH_THREAD_SLEEP";

    /// The child role of the exit wait test: started with `SLEEP_ENV` it sleeps a minute at most, so a child that a
    /// crashed run leaves ends on its own; run any other way it ends at once.
    #[test]
    #[ignore = "a child role for the exit wait test"]
    fn sleep_role() {
        if std::env::var_os(SLEEP_ENV).is_some() {
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    /// A sleeping child, ended and reaped on drop, so a failed assert leaves no process.
    struct Kid(Child);

    impl Kid {
        fn start() -> Self {
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "watch::thread::tests::sleep_role",
                    "--exact",
                    "--ignored",
                    "--test-threads=1",
                    "--quiet",
                ])
                .env(SLEEP_ENV, "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            Self(child)
        }

        fn process(&self) -> ProcessRef {
            ProcessRef {
                pid: self.0.id(),
                start: crate::process::start_of(&self.0).unwrap(),
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

    #[test]
    fn start_posts_the_first_round() {
        let mut rig = Rig::new("first");
        rig.config(false);
        rig.hold("job.json", fake(1));
        rig.live_hold(fake(1));
        rig.live_session(fake(2));
        rig.write("restore-request.json", b"{\"version\":1}");

        let at = Instant::now();
        rig.start(Config::default());
        let took = rig.wait_for(at, |e| e.len() >= 4);
        println!("first round posted after {} ms", took.as_millis());
        assert_eq!(
            rig.settled(),
            [
                Event::ConfigChanged(Config::new(false)),
                Event::Holds(vec![hold_facts("job.json", Alive::Yes)]),
                claude(vec![(session(fake(2)), Alive::Yes)]),
                Event::Menu(MenuCommand::Restore),
            ]
        );
        assert!(rig.lines().is_empty(), "{:?}", rig.lines());
    }

    #[test]
    fn the_first_round_posts_no_config_the_core_already_has() {
        let mut missing = Rig::new("first-default");
        missing.started(Config::default(), 2);
        let mut same = Rig::new("first-same");
        same.config(false);
        same.started(Config::new(false), 2);
        for rig in [&missing, &same] {
            assert_eq!(rig.settled(), [Event::Holds(vec![]), claude(vec![])]);
        }
    }

    #[test]
    fn a_hold_file_posts_holds_within_a_second_and_its_delete_posts_holds_without_it() {
        let mut rig = Rig::new("hold");
        rig.live_hold(fake(1));
        rig.started(Config::default(), 2);

        let at = Instant::now();
        let path = rig.hold("job.json", fake(1));
        let with = Event::Holds(vec![hold_facts("job.json", Alive::Yes)]);
        let took = rig.wait_for(at, |e| e.contains(&with));
        println!("hold file written: Holds after {} ms", took.as_millis());
        assert!(took < ONE_SECOND, "{took:?}");

        let at = Instant::now();
        fs::remove_file(&path).unwrap();
        let took = rig.wait_for(at, |e| count(e, is_holds) == 3);
        println!("hold file deleted: Holds after {} ms", took.as_millis());
        assert!(took < ONE_SECOND, "{took:?}");
        let events = rig.settled();
        assert_eq!(events.last(), Some(&Event::Holds(vec![])), "{events:#?}");
        assert_eq!(count(&events, is_holds), 3, "{events:#?}");
    }

    #[test]
    fn a_holds_folder_made_after_start_is_watched_without_a_tick() {
        let mut rig = Rig::new("late-holds");
        fs::remove_dir(rig.dir.holds()).unwrap();
        rig.live_hold(fake(1));
        rig.started(Config::default(), 2);

        let at = Instant::now();
        let path = rig.hold("job.json", fake(1));
        let with = Event::Holds(vec![hold_facts("job.json", Alive::Yes)]);
        let took = rig.wait_for(at, |e| e.contains(&with));
        println!("holds folder made: Holds after {} ms", took.as_millis());
        assert!(took < ONE_SECOND, "{took:?}");

        let at = Instant::now();
        fs::remove_file(&path).unwrap();
        let took = rig.wait_for(at, |e| count(e, is_holds) == 3);
        assert!(took < ONE_SECOND, "{took:?}");
    }

    #[test]
    fn a_restore_request_posts_one_menu_restore() {
        let mut rig = Rig::new("restore");
        rig.started(Config::default(), 2);
        let restore = Event::Menu(MenuCommand::Restore);

        let at = Instant::now();
        rig.write("restore-request.json", b"{\"version\":1}");
        let took = rig.wait_for(at, |e| e.contains(&restore));
        println!(
            "restore request: Menu(Restore) after {} ms",
            took.as_millis()
        );
        assert!(took < ONE_SECOND, "{took:?}");
        rig.tick();
        let events = rig.settled();
        assert_eq!(count(&events, |e| *e == restore), 1, "{events:#?}");
    }

    #[test]
    fn a_config_change_posts_config_changed() {
        let mut rig = Rig::new("config");
        rig.started(Config::default(), 2);
        for automatic in [false, true] {
            let at = Instant::now();
            rig.config(automatic);
            let want = Event::ConfigChanged(Config::new(automatic));
            let took = rig.wait_for(at, |e| e.last() == Some(&want));
            println!(
                "config written: ConfigChanged after {} ms",
                took.as_millis()
            );
            assert!(took < ONE_SECOND, "{took:?}");
        }
        let events = rig.settled();
        let changes = count(&events, |e| matches!(e, Event::ConfigChanged(_)));
        assert_eq!(changes, 2, "{events:#?}");
    }

    #[test]
    fn a_process_exit_posts_the_session_and_the_hold_as_gone_at_once() {
        let (mut agent_kid, mut hold_kid) = (Kid::start(), Kid::start());
        let (agent, holder) = (agent_kid.process(), hold_kid.process());
        let mut rig = Rig::new("exit");
        rig.hold("job.json", holder);
        rig.live_hold(holder);
        rig.live_session(agent);
        rig.started(Config::default(), 2);
        // Settled first: right after the first round, a notice in the data folder names `holds` (every run on the test
        // laptop, 2026-11-04). The read it starts must find the fake still saying alive, or it posts the session gone.
        assert_eq!(
            rig.settled(),
            [
                Event::Holds(vec![hold_facts("job.json", Alive::Yes)]),
                claude(vec![(session(agent), Alive::Yes)]),
            ]
        );

        // The fake now says the session is gone, but nothing asks it: no notice, no tick.
        rig.liveness.alive.lock().unwrap().clear();
        let events = rig.settled();
        assert_eq!(events.len(), 2, "{events:#?}");
        let at = Instant::now();
        agent_kid.end();
        let gone = claude(vec![(session(agent), Alive::Gone)]);
        let took = rig.wait_for(at, |e| e.contains(&gone));
        println!(
            "session process ended: Sources after {} ms",
            took.as_millis()
        );
        assert!(took < ONE_SECOND, "{took:?}");
        assert_eq!(rig.settled().len(), 3, "the hold did not change");

        rig.liveness.starts.lock().unwrap().clear();
        let events = rig.settled();
        assert_eq!(events.len(), 3, "{events:#?}");
        let at = Instant::now();
        hold_kid.end();
        let gone = Event::Holds(vec![hold_facts("job.json", Alive::Gone)]);
        let took = rig.wait_for(at, |e| e.contains(&gone));
        println!("hold process ended: Holds after {} ms", took.as_millis());
        assert!(took < ONE_SECOND, "{took:?}");
        assert_eq!(rig.settled().len(), 4);
    }

    #[test]
    fn a_tick_posts_nothing_when_nothing_changed() {
        let mut rig = Rig::new("tick");
        rig.config(false);
        rig.hold("job.json", fake(1));
        rig.live_hold(fake(1));
        rig.live_session(fake(2));
        let file = rig.agent.write("1.json", b"{}");
        rig.started(Config::default(), 3);
        let first = rig.settled();
        let checks = rig.checks();

        // One tick at a time: ticks that come while one runs count once, so three at once could be one.
        for n in 1..=3 {
            rig.tick_and_wait(checks + n);
        }
        assert_eq!(rig.settled(), first);
        assert_eq!(
            rig.source.fed(),
            [whole(&file, b"{}")],
            "unchanged bytes are not fed again"
        );
    }

    #[test]
    fn a_failed_post_is_logged_once_and_the_next_tick_posts_everything_again() {
        let mut rig = Rig::new("failed");
        rig.config(false);
        rig.hold("job.json", fake(1));
        rig.live_hold(fake(1));
        rig.live_session(fake(2));
        rig.write("restore-request.json", b"{\"version\":1}");
        rig.sink.failing.store(1816, Ordering::SeqCst);
        rig.start(Config::default());

        let tries = || rig.sink.tries.load(Ordering::SeqCst);
        until(Instant::now(), || tries() >= 4, || tries().to_string());
        std::thread::sleep(QUIET);
        assert_eq!(tries(), 4, "nothing is tried again before the tick");
        assert_eq!(
            rig.lines(),
            [
                "an event could not be posted to the loop (PostMessageW error 1816); the next tick posts everything again"
            ]
        );
        assert_eq!(rig.levels(), [Level::Warn]);

        rig.sink.failing.store(0, Ordering::SeqCst);
        assert!(
            rig.settled().is_empty(),
            "nothing is posted again before the tick"
        );
        rig.tick();
        rig.wait_for(Instant::now(), |e| e.len() >= 4);
        let full = [
            Event::ConfigChanged(Config::new(false)),
            Event::Holds(vec![hold_facts("job.json", Alive::Yes)]),
            claude(vec![(session(fake(2)), Alive::Yes)]),
            Event::Menu(MenuCommand::Restore),
        ];
        assert_eq!(rig.settled(), full);
        rig.tick_and_wait(rig.checks() + 1);
        assert_eq!(rig.settled(), full, "the next tick posts nothing again");
        assert_eq!(rig.lines().len(), 1);
    }

    /// 1400 is the loop window gone, as after a quit while a hold file changes (the quit deletes Keep awake's file):
    /// one Info line, no warning, and no promise of a tick.
    #[test]
    fn a_post_after_the_loop_window_is_gone_is_logged_once_as_info() {
        let mut rig = Rig::new("gone");
        let path = rig.hold("job.json", fake(1));
        rig.live_hold(fake(1));
        rig.sink.failing.store(1400, Ordering::SeqCst);
        rig.start(Config::default());

        let tries = || rig.sink.tries.load(Ordering::SeqCst);
        until(Instant::now(), || tries() >= 2, || tries().to_string());
        fs::remove_file(&path).unwrap();
        until(Instant::now(), || tries() >= 3, || tries().to_string());
        std::thread::sleep(QUIET);
        assert_eq!(
            rig.lines(),
            [
                "the loop window is gone (PostMessageW error 1400), as after a quit: this event and every later one \
                 are dropped"
            ]
        );
        assert_eq!(rig.levels(), [Level::Info]);
        assert!(rig.events().is_empty());
    }

    #[test]
    fn a_retry_runs_200_ms_later_without_a_tick() {
        let mut rig = Rig::new("retry");
        let bytes = b"{\"half\":";
        let file = rig.agent.write("1.json", bytes);
        rig.source.set(|f| f.retry = vec![FileId(file.clone())]);
        let times = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&times);
        rig.source
            .after_feed(move |_, _| seen.lock().unwrap().push(Instant::now()));

        rig.start(Config::default());
        let fed = || times.lock().unwrap().len();
        until(Instant::now(), || fed() >= 2, || fed().to_string());
        let times = times.lock().unwrap().clone();
        let gap = times[1] - times[0];
        println!(
            "the retry was fed {} ms after the first feed",
            gap.as_millis()
        );
        assert!(
            gap >= Duration::from_millis(150) && gap < ONE_SECOND,
            "{gap:?}"
        );
        std::thread::sleep(QUIET);
        assert_eq!(rig.source.fed(), [whole(&file, bytes), whole(&file, bytes)]);
    }

    #[test]
    fn a_folder_a_source_asks_for_later_is_watched_and_read_in_full() {
        let mut rig = Rig::new("later");
        let later = TempDir::new("thread-later-folder");
        let early = later.write("early.json", b"{}");
        let dir = later.path().to_path_buf();
        rig.source.after_feed(move |fake, file| {
            if file.0.ends_with("1.json") && fake.wanted.len() == 1 {
                fake.wanted.push(status_folder(&dir));
            }
        });
        rig.started(Config::default(), 2);

        rig.agent.write("1.json", b"{}");
        let has = |path: &Path| rig.source.fed().contains(&whole(path, b"{}"));
        let show = || format!("{:#?}", rig.source.fed());
        until(Instant::now(), || has(&early), show);

        let at = Instant::now();
        let file = later.write("2.json", b"{}");
        let took = until(at, || has(&file), show);
        println!(
            "a file in the later folder was fed after {} ms",
            took.as_millis()
        );
        assert!(took < ONE_SECOND, "{took:?}");
    }

    #[test]
    fn an_agent_folder_made_after_start_is_read_at_the_next_tick() {
        let mut rig = Rig::new("late-agent");
        let late = rig.agent.path().join("late");
        rig.source.set(|f| f.wanted = vec![status_folder(&late)]);
        rig.started(Config::default(), 2);

        let file = rig.agent.write("late/1.json", b"{}");
        std::thread::sleep(QUIET);
        assert!(rig.source.fed().is_empty(), "the folder was not watched");
        rig.tick();
        let has = |path: &Path| rig.source.fed().contains(&whole(path, b"{}"));
        let show = || format!("{:#?}", rig.source.fed());
        until(Instant::now(), || has(&file), show);

        let at = Instant::now();
        let second = rig.agent.write("late/2.json", b"{}");
        let took = until(at, || has(&second), show);
        assert!(took < ONE_SECOND, "{took:?}");
    }

    #[test]
    fn stop_ends_the_thread_within_a_second() {
        let mut rig = Rig::new("stop");
        // This test's own process: a live session whose exit wait never fires.
        rig.live_session(crate::process::current().unwrap());
        rig.started(Config::default(), 2);

        let handle = rig.handle.take().unwrap();
        let at = Instant::now();
        assert!(handle.stop());
        let took = at.elapsed();
        println!("stop took {} ms", took.as_millis());
        assert!(took < ONE_SECOND, "{took:?}");
        rig.agent.write("1.json", b"{}");
        rig.hold("job.json", fake(1));
        assert_eq!(
            rig.settled().len(),
            2,
            "nothing is read or posted after the stop"
        );
        assert!(rig.source.fed().is_empty());
        assert!(rig.lines().is_empty(), "{:?}", rig.lines());
    }

    const LEAK_CHILD: &str = "WAKESHOT_TEST_WATCH_THREAD_LEAK";
    const CYCLES: usize = 50;

    #[test]
    fn start_and_stop_leave_no_handle() {
        if let Ok(dir) = std::env::var(LEAK_CHILD) {
            leak_child(Path::new(&dir));
            return;
        }
        // In a child process of its own, so the other tests' handles do not move the count.
        let dir = TempDir::new("thread-leak");
        let out = Command::new(std::env::current_exe().unwrap())
            .args([
                "watch::thread::tests::start_and_stop_leave_no_handle",
                "--exact",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(LEAK_CHILD, dir.path())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        println!("{text}");
        assert!(
            out.status.success(),
            "child failed: {text}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            text.contains("handles before"),
            "the child never ran the cycles"
        );
    }

    /// Each cycle has an open folder, a missing one, a hold file, and a live session with its exit wait.
    fn leak_child(root: &Path) {
        let me = crate::process::current().unwrap();
        let (data, agent) = (root.join("data"), root.join("agent"));
        fs::create_dir_all(data.join("holds")).unwrap();
        fs::create_dir_all(&agent).unwrap();
        fs::write(data.join("holds").join("job.json"), hold_json(me)).unwrap();
        let missing = root.join("missing");
        let cycle = || {
            let source = Shared::new();
            source.set(|f| {
                f.wanted = vec![status_folder(&agent), status_folder(&missing)];
                f.sessions = vec![session(me)];
            });
            let liveness = Arc::new(FakeLiveness::default());
            liveness.alive.lock().unwrap().push((me, Alive::Yes));
            liveness
                .starts
                .lock()
                .unwrap()
                .push((me.pid, Lookup::Running(me.start)));
            let sink = Arc::new(FakeEventSink::default());
            let handle = WatchThread::start(
                DataDir::at(data.clone()),
                Config::default(),
                vec![source.boxed()],
                liveness,
                sink.clone(),
                Arc::new(MemoryLog::default()),
            )
            .unwrap();
            until(
                Instant::now(),
                || sink.events().len() >= 2,
                || format!("{:?}", sink.events()),
            );
            assert!(handle.stop());
        };
        for _ in 0..3 {
            cycle();
        }
        let before = handle_count();
        for _ in 0..CYCLES {
            cycle();
        }
        let after = handle_count();
        println!("handles before {before}, after {after} ({CYCLES} start and stop cycles)");
        assert_eq!(before, after);
    }

    fn handle_count() -> u32 {
        let mut count = 0u32;
        // SAFETY: the pseudo handle of this process needs no close, and `count` outlives the call.
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.unwrap();
        count
    }
}
