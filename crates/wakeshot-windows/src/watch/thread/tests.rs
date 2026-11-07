use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use wakeshot_core::holds::{HoldId, HoldKind};
use wakeshot_core::process::{Alive, FileTime, Lookup};
use wakeshot_core::sources::{
    self, Activity, Fed, Feed, FileId, FormatHealth, SessionFacts, SessionKey, SourceId, WantedFile,
};
use wakeshot_core::testing::FakeSource;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

use super::*;
use crate::log::MemoryLog;
use crate::process::FakeLiveness;
use crate::sink::FakeEventSink;
use crate::watch::read::TempDir;

/// Only keeps a broken run from hanging; the 1 s limits are asserted with [`ONE_SECOND`] where they apply.
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

/// A data folder that cannot be watched is read only on the tick, so a hold file waits up to 30 s: said once. A
/// `holds\` not made yet is the normal state until the first hold file.
#[test]
fn a_data_folder_that_cannot_be_watched_is_logged_and_one_not_made_yet_is_not() {
    let mut missing = Rig::new("holds-missing");
    fs::remove_dir(missing.dir.holds()).unwrap();
    missing.started(Config::default(), 2);
    let mut file = Rig::new("holds-file");
    fs::remove_dir(file.dir.holds()).unwrap();
    fs::write(file.dir.holds(), b"").unwrap();
    file.started(Config::default(), 2);
    missing.settled();
    file.settled();

    assert!(missing.lines().is_empty(), "{:?}", missing.lines());
    let lines = file.lines();
    let watch: Vec<&String> = lines.iter().filter(|l| l.contains("not watched")).collect();
    assert_eq!(watch.len(), 1, "{lines:?}");
    assert!(
        watch[0].starts_with("the holds folder is not watched (ReadDirectoryChangesW error "),
        "{lines:?}"
    );
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

// Held-back query runs, `ImageSince` exits and the query poll.

fn opencode_want(db: PathBuf) -> WantedFile {
    WantedFile::Query {
        db,
        name: "tables",
        sql: "SELECT 1",
    }
}

impl Rig {
    /// [`Rig::start`] with `io` for the agents.
    fn start_with(&mut self, config: Config, io: crate::watch::agents::AgentIo) {
        let agents = Agents::with_io(vec![self.source.boxed()], self.log.clone(), io);
        let handle = WatchThread::spawn(
            self.dir.clone(),
            config,
            agents,
            self.liveness.clone(),
            self.sink.clone(),
            self.log.clone(),
        )
        .unwrap();
        self.handle = Some(handle);
    }
}

#[test]
fn a_query_held_back_by_the_limit_runs_without_a_tick() {
    let mut rig = Rig::new("query");
    let db = rig.agent.path().join("opencode.db");
    let wal = rig.agent.write("opencode.db-wal", b"");
    rig.agent.write("opencode.db", b"");
    let want = opencode_want(db.clone());
    rig.source.set(|f| f.wanted = vec![want]);
    let runs = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&runs);
    let io = crate::watch::agents::AgentIo {
        query: Box::new(move |_, _| {
            seen.lock().unwrap().push(Instant::now());
            crate::sqlite::Answer::Json(b"{\"rows\":[]}".to_vec())
        }),
        ..Default::default()
    };
    rig.start_with(Config::default(), io);
    let count = || runs.lock().unwrap().len();
    until(Instant::now(), || count() >= 1, || count().to_string());

    // Two writes right after the first run: one run, held back until a second after the first.
    fs::write(&wal, b"1").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    fs::write(&wal, b"12").unwrap();
    until(Instant::now(), || count() >= 2, || count().to_string());
    std::thread::sleep(QUIET);
    let times = runs.lock().unwrap().clone();
    assert_eq!(times.len(), 2, "{times:?}");
    let gap = times[1] - times[0];
    println!(
        "the held-back run came {} ms after the first",
        gap.as_millis()
    );
    assert!(
        gap >= Duration::from_millis(990) && gap < Duration::from_millis(1_900),
        "{gap:?}"
    );
    assert_eq!(
        rig.source.fed(),
        [(
            FileId::query(&db, "tables"),
            Feed::Whole(b"{\"rows\":[]}".to_vec())
        )],
        "the same answer is fed once"
    );
}

/// With real notices: on a database a killed agent left, each read rebuilds the `-shm` (SQLite's first-opener
/// rule), and that write must not run the next read.
#[test]
fn a_query_that_rewrites_the_shm_runs_once_not_in_a_loop() {
    let mut rig = Rig::new("query-shm");
    let db = rig.agent.path().join("opencode.db");
    rig.agent.write("opencode.db-wal", b"");
    rig.agent.write("opencode.db", b"");
    let shm = rig.agent.write("opencode.db-shm", b"");
    rig.source.set(|f| f.wanted = vec![opencode_want(db)]);
    let runs = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&runs);
    let io = crate::watch::agents::AgentIo {
        query: Box::new(move |_, _| {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            fs::write(&shm, vec![n as u8; 32_768]).unwrap();
            crate::sqlite::Answer::Json(b"{\"rows\":[]}".to_vec())
        }),
        ..Default::default()
    };
    rig.start_with(Config::default(), io);
    let count = || runs.load(Ordering::SeqCst);
    until(Instant::now(), || count() >= 1, || count().to_string());
    // Three times the limit: the loop ran a read each second. The poll finds the header the read wrote.
    std::thread::sleep(Duration::from_millis(
        3 * crate::watch::agents::QUERY_EVERY_MS,
    ));
    assert_eq!(count(), 1);
}

/// Lock file and database holders the test sets, the same for every path, and how often it was asked.
#[derive(Clone, Default)]
struct HeldBy(Arc<Mutex<Vec<ProcessRef>>>, Arc<AtomicUsize>);

impl HeldBy {
    fn new(holders: Vec<ProcessRef>) -> Self {
        Self(Arc::new(Mutex::new(holders)), Arc::default())
    }
}

impl crate::lockowner::LockOwners for HeldBy {
    fn holders(&self, _: &Path) -> Result<Vec<ProcessRef>, WinError> {
        self.1.fetch_add(1, Ordering::SeqCst);
        Ok(self.0.lock().unwrap().clone())
    }
}

/// A poll wakes the thread every 2 s, so it stops once the agent that held the database exits: the exit wait
/// wakes the thread, and the next poll finds nobody.
#[test]
fn the_poll_stops_when_the_agent_that_held_the_database_exits() {
    let mut rig = Rig::new("query-hosts");
    let db = rig.agent.path().join("opencode.db");
    rig.agent.write("opencode.db-wal", b"");
    rig.agent.write("opencode.db", b"");
    rig.agent.write("opencode.db-shm", &[1; 32_768]);
    rig.source.set(|f| f.wanted = vec![opencode_want(db)]);
    let mut kid = Kid::start();
    let agent = kid.process();
    rig.liveness.alive.lock().unwrap().push((agent, Alive::Yes));
    let owners = HeldBy::new(vec![agent]);
    let io = crate::watch::agents::AgentIo {
        owners: Box::new(owners.clone()),
        query: Box::new(|_, _| crate::sqlite::Answer::Json(b"{\"rows\":[]}".to_vec())),
        ..Default::default()
    };
    rig.start_with(Config::default(), io);
    let asked = || owners.1.load(Ordering::SeqCst);
    until(Instant::now(), || asked() >= 1, || "never asked".to_owned());
    let poll = Duration::from_millis(crate::watch::agents::QUERY_POLL_MS);
    std::thread::sleep(2 * poll);
    assert_eq!(asked(), 1, "a live holder is asked about once");

    owners.0.lock().unwrap().clear();
    rig.liveness.alive.lock().unwrap().clear();
    kid.end();
    let took = until(
        Instant::now(),
        || asked() >= 2,
        || "not asked again".to_owned(),
    );
    assert!(took <= poll + QUIET, "{took:?}");
    std::thread::sleep(3 * poll);
    assert_eq!(asked(), 2, "no poll, so no question");
}

/// [`FakeLiveness`] that counts the process lists asked of it.
#[derive(Default)]
struct CountingLiveness {
    inner: FakeLiveness,
    lists: AtomicUsize,
}

impl Liveness for CountingLiveness {
    fn check(&self, process: ProcessRef) -> Alive {
        self.inner.check(process)
    }

    fn start_of_pid(&self, pid: u32) -> Lookup {
        self.inner.start_of_pid(pid)
    }

    fn list(&self) -> Result<Vec<crate::process::ProcessEntry>, WinError> {
        self.lists.fetch_add(1, Ordering::SeqCst);
        self.inner.list()
    }
}

/// A snapshot of an `ImageSince` session lists every process (about 14 ms on the test laptop), so a poll that
/// found no commit posts nothing and lists nothing.
#[test]
fn a_poll_that_finds_no_commit_lists_no_process() {
    let rig = Rig::new("query-quiet");
    let db = rig.agent.path().join("opencode.db");
    rig.agent.write("opencode.db-wal", b"");
    rig.agent.write("opencode.db", b"");
    rig.agent.write("opencode.db-shm", &[1; 32_768]);
    let mut facts = session(ProcessRef {
        pid: 4242,
        start: FileTime(1),
    });
    facts.liveness = sources::Liveness::ImageSince {
        images: &["opencode.exe"],
        since_wall_ms: None,
    };
    rig.source.set(|f| {
        f.wanted = vec![opencode_want(db.clone())];
        f.sessions = vec![facts];
    });
    // The agent that holds the database open, so the poll runs.
    let kid = Kid::start();
    let agent = kid.process();
    let io = crate::watch::agents::AgentIo {
        owners: Box::new(HeldBy::new(vec![agent])),
        query: Box::new(|_, _| crate::sqlite::Answer::Json(b"{\"rows\":[]}".to_vec())),
        ..Default::default()
    };
    let liveness = Arc::new(CountingLiveness::default());
    liveness
        .inner
        .alive
        .lock()
        .unwrap()
        .push((agent, Alive::Yes));
    let agents = Agents::with_io(vec![rig.source.boxed()], rig.log.clone(), io);
    let handle = WatchThread::spawn(
        rig.dir.clone(),
        Config::default(),
        agents,
        liveness.clone(),
        rig.sink.clone(),
        rig.log.clone(),
    )
    .unwrap();
    let lists = || liveness.lists.load(Ordering::SeqCst);
    until(
        Instant::now(),
        || lists() >= 1,
        || "no first snapshot".to_owned(),
    );
    let before = lists();
    let poll = Duration::from_millis(crate::watch::agents::QUERY_POLL_MS);
    std::thread::sleep(2 * poll + QUIET);
    assert_eq!(lists(), before, "two polls with no commit");

    rig.agent.write("opencode.db-shm", &[2; 32_768]);
    let took = until(Instant::now(), || lists() > before, || "no read".to_owned());
    assert!(took <= poll + QUIET, "{took:?}");
    assert!(handle.stop());
}

/// OpenCode's commits give no change notice (seen live, OpenCode 2.2.5): a new answer reaches the source by the
/// poll, with no notice and no tick.
#[test]
fn a_commit_with_no_notice_is_read_by_the_poll() {
    let mut rig = Rig::new("query-poll");
    let db = rig.agent.path().join("opencode.db");
    rig.agent.write("opencode.db-wal", b"");
    rig.agent.write("opencode.db", b"");
    rig.agent.write("opencode.db-shm", &[1; 32_768]);
    rig.source
        .set(|f| f.wanted = vec![opencode_want(db.clone())]);
    // The agent that holds the database open, so the poll runs.
    let kid = Kid::start();
    let agent = kid.process();
    rig.liveness.alive.lock().unwrap().push((agent, Alive::Yes));
    let committed = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&committed);
    let io = crate::watch::agents::AgentIo {
        owners: Box::new(HeldBy::new(vec![agent])),
        query: Box::new(move |_, _| {
            let rows = if seen.load(Ordering::SeqCst) {
                &b"{\"rows\":[[\"ses_1\",1]]}"[..]
            } else {
                &b"{\"rows\":[]}"[..]
            };
            crate::sqlite::Answer::Json(rows.to_vec())
        }),
        ..Default::default()
    };
    rig.start_with(Config::default(), io);
    let id = FileId::query(&db, "tables");
    until(
        Instant::now(),
        || !rig.source.fed().is_empty(),
        || "no first run".to_owned(),
    );
    // The commit: the rows and the `-shm`'s header change; its notice runs no query.
    let at = Instant::now();
    committed.store(true, Ordering::SeqCst);
    rig.agent.write("opencode.db-shm", &[2; 32_768]);
    let new = (id, Feed::Whole(b"{\"rows\":[[\"ses_1\",1]]}".to_vec()));
    let took = until(
        at,
        || rig.source.fed().contains(&new),
        || "no poll".to_owned(),
    );
    println!(
        "a commit with no notice was read {} ms later",
        took.as_millis()
    );
    assert!(
        took <= Duration::from_millis(crate::watch::agents::QUERY_POLL_MS + 500),
        "{took:?}"
    );
}

#[test]
fn an_image_since_session_is_posted_gone_at_once_when_its_process_exits() {
    let mut kid = Kid::start();
    let process = kid.process();
    let mut rig = Rig::new("image-since");
    rig.liveness
        .processes
        .lock()
        .unwrap()
        .push(crate::process::ProcessEntry {
            pid: process.pid,
            parent: 0,
            start: process.start,
            image: "kid.exe".to_owned(),
        });
    let mut facts = session(process);
    facts.liveness = sources::Liveness::ImageSince {
        images: &["KID.EXE"],
        since_wall_ms: None,
    };
    rig.source.set(|f| f.sessions = vec![facts.clone()]);
    rig.started(Config::default(), 2);
    assert_eq!(
        rig.settled(),
        [
            Event::Holds(vec![]),
            claude(vec![(facts.clone(), Alive::Yes)])
        ]
    );

    // The list now has no match, but nothing asks it: no notice, no tick.
    rig.liveness.processes.lock().unwrap().clear();
    assert_eq!(rig.settled().len(), 2);
    let at = Instant::now();
    kid.end();
    let gone = claude(vec![(facts, Alive::Gone)]);
    let took = rig.wait_for(at, |e| e.contains(&gone));
    println!("image process ended: Sources after {} ms", took.as_millis());
    assert!(took < ONE_SECOND, "{took:?}");
}
