//! Agent sources and their liveness: asks each source what it wants, reads and feeds it, and turns its sessions into
//! one `Event::Sources` per source, each session paired with its liveness (architecture section 7: the adapter checks
//! liveness, never the parser). No thread or folder watch of its own: the watch thread calls [`Agents::on_tick`] at
//! start and on each tick, [`Agents::on_change`] on a notice, [`Agents::retry`] [`RETRY_MS`] later,
//! [`Agents::run_due`] when [`Agents::next_due`] comes, and [`Agents::snapshots`] after each and after a process exit.
//!
//! A file is opened only as a source asked: a `Folder` or `Recent` file whose name the want's `accept` passes (checked
//! again at the open), a `Tail` path, and the file a `FindTail` or `FindNewest` found. `Names` are listed, never
//! opened. A `Recent` file last written more than [`RECENT_WINDOW`] ago, and before `Agents` was made, is not opened
//! until it changes.
//!
//! A `Query` runs only while the database's `-wal` exists, at most once per [`QUERY_EVERY_MS`]. OpenCode's commits
//! give no change notice, so while another process holds the database open it is polled every [`QUERY_POLL_MS`]. A
//! poll or a tick reads only when the `-shm`'s WAL-index header changed since the last read
//! ([`sqlite::wal_index_header`]), so files a stopped agent left are read once. A notice reads only when it names the
//! database or its `-wal`, never the `-shm`, which a read itself rewrites on such files.
//!
//! Restart Manager ([`AgentIo::owners`]) is asked rarely: one question costs as much CPU as minutes of rest. For a
//! database: at a want's first run, on a notice that names one of its files while nobody held it (an agent's first
//! open rebuilds the `-shm`), after a read that found a commit, after every holder ended, and [`HOSTS_RECHECK_MS`]
//! after the last question while held. For an `AnyProcess` lock file (Codex, Copilot): when it changes, when its
//! holders end, or, while nobody held it, when its session's facts change; never on a tick. Holders go to
//! [`Snapshots::live`], so their exit is waited on.
//!
//! An `AnyProcess` session lives while a process running its image holds its lock file open; that process is then
//! checked by pid and start like any other. A lock file whose name gives a pid (Copilot's `inuse.<pid>.hold`, seen
//! live 2026-11-06) also counts that holder whatever its image, since VS Code's Copilot harness holds it as
//! `copilot-runtime.exe`. If Restart Manager fails, the process the name gives counts while it runs; a name with no
//! pid (Codex) counts any live process running the image, which can outlive its session (architecture section 7). An
//! `ImageSince` session lives while a process running one of its images started early enough; each such process goes
//! to [`Snapshots::live`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wakeshot_core::event::Event;
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::sources::{
    self, Activity, AgentSource, Fed, Feed, FileId, FormatHealth, IMAGE_SINCE_SLACK_MS,
    SessionFacts, SessionKey, SourceId, WantedFile,
};
use wakeshot_core::time::Now;

use crate::lockowner::{LockOwners, RestartManager};
use crate::log::{Level, Log};
use crate::process::{Liveness, ProcessEntry};
use crate::sqlite;
use crate::watch::read::{
    Finder, RECENT_CAP, Stamp, Tail, names, read_whole, read_whole_stamped, recent_files,
};

/// How long after a call that returned [`Retry`]s the caller runs [`Agents::retry`]: Claude truncates its status file,
/// then writes it, so a read 200 ms later usually finds it whole (architecture section 7).
pub const RETRY_MS: u64 = 200;

/// At most one log line per source in this much run time (architecture section 8), so a format change that hits every
/// status write cannot flood the log.
const LOG_EVERY_MS: u64 = 60_000;

/// The most wants one call handles for one source. A source whose wants never settle would otherwise hold the watch
/// thread; what is left waits for the next call.
const UNITS_PER_RUN: usize = 1024;

const AREA: &str = "watch";

/// The least run time between two runs of one `Query` want: one database write gives several notices (the db and its
/// `-wal`), and an agent that streams may write many times a second.
pub const QUERY_EVERY_MS: u64 = 1_000;

/// How often a `Query` looks for a commit while a process holds its database open: OpenCode keeps its `-wal` open at
/// one size, so a commit gives no change notice (`docs/research/opencode-signals.md`, "Live test (2026-11-06)"). A
/// look reads 48 bytes of the `-shm` ([`sqlite::wal_index_header`]), and the query reads the database only when they
/// changed since its last read.
pub const QUERY_POLL_MS: u64 = 2_000;

/// How far back a `Recent` want reads at start (`docs/research/copilot-vscode-signals.md`, detection rule 3): up to
/// 400 old sessions per workspace stay unread.
pub const RECENT_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);

/// A folder to watch for change notices. `subtree`: its subfolders too, for a `Names` want of depth 1 or more and a
/// `Recent` want with folder levels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WatchFolder {
    pub dir: PathBuf,
    pub subtree: bool,
}

/// A file to read and feed again, once, [`RETRY_MS`] after the call that returned it: its source answered
/// `Fed::RetryOnce`, or the read failed in a way that can pass (a writer that shares no read). A `Folder` or a
/// `Recent` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retry {
    slot: usize,
    of: RetryOf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RetryOf {
    Folder { dir: PathBuf, name: String },
    Recent { key: RecentKey, path: PathBuf },
}

impl Retry {
    pub fn file(&self) -> FileId {
        match &self.of {
            RetryOf::Folder { dir, name } => FileId(dir.join(name)),
            RetryOf::Recent { path, .. } => FileId(path.clone()),
        }
    }
}

/// Runs one `Query` want: the database, the SQL.
pub type QueryFn = dyn Fn(&Path, &str) -> sqlite::Answer + Send;

/// What `Agents` calls besides file reads, so tests stand in for it. [`Default`] is the real one.
pub struct AgentIo {
    /// Who holds a lock file (`AnyProcess`) or a `Query` database open: Restart Manager.
    pub owners: Box<dyn LockOwners>,
    /// [`sqlite::query`].
    pub query: Box<QueryFn>,
    /// Run time for the `Query` limit: `Instant::now`.
    pub now: Box<dyn Fn() -> Instant + Send>,
    /// Wall time for the `Recent` window: `SystemTime::now`. Read once when `Agents` is made (the start) and at each
    /// call.
    pub wall: Box<dyn Fn() -> SystemTime + Send>,
}

impl Default for AgentIo {
    fn default() -> Self {
        Self {
            owners: Box::new(RestartManager),
            query: Box::new(sqlite::query),
            now: Box::new(Instant::now),
            wall: Box::new(SystemTime::now),
        }
    }
}

/// One call's view of [`AgentIo`].
struct Ctx<'a> {
    query: &'a QueryFn,
    owners: &'a dyn LockOwners,
    now: Instant,
    /// A `Recent` file last written before this (a FILETIME) is not read until it changes.
    recent_since: u64,
    /// The databases whose holders this call asked about: the queries on one database share the answer.
    asked: RefCell<BTreeSet<PathBuf>>,
}

impl<'a> Ctx<'a> {
    fn new(io: &'a AgentIo, started: SystemTime) -> Self {
        let wall = (io.wall)();
        let window = wall.checked_sub(RECENT_WINDOW).unwrap_or(UNIX_EPOCH);
        Self {
            query: &*io.query,
            owners: &*io.owners,
            now: (io.now)(),
            recent_since: filetime_of(window.min(started)),
            asked: RefCell::default(),
        }
    }
}

/// How long a database's holders stay known while it is polled. A process that held it only for a moment (a virus
/// scan when the agent exits, another reader) would otherwise keep the poll on for as long as that process lives.
pub const HOSTS_RECHECK_MS: u64 = 5 * 60_000;

/// Who holds a `Query` want's database open, so its poll runs only while a commit can come.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Hosts {
    /// Nobody but Wakeshot: no poll until a notice names one of its files or a read finds a commit.
    Nobody,
    /// These processes, asked about at that time, checked at each [`Agents::snapshots`] and waited on; when all
    /// ended, or [`HOSTS_RECHECK_MS`] after the question, asked again.
    Held(Vec<ProcessRef>, Instant),
    /// Restart Manager failed: the poll runs, and the next tick asks again.
    Unknown,
}

impl Hosts {
    fn polls(&self) -> bool {
        !matches!(self, Hosts::Nobody)
    }
}

/// A wall time as a FILETIME (100 ns units since 1601), the form of the listing's write times.
fn filetime_of(time: SystemTime) -> u64 {
    // The FILETIME of 1970-01-01.
    const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;
    let ticks = |d: Duration| u64::try_from(d.as_nanos() / 100).unwrap_or(u64::MAX);
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => UNIX_EPOCH_TICKS.saturating_add(ticks(after)),
        Err(before) => UNIX_EPOCH_TICKS.saturating_sub(ticks(before.duration())),
    }
}

/// Wall ms as a FILETIME, the form of a process start; 0 before 1601.
fn filetime_of_unix_ms(ms: i64) -> u64 {
    const UNIX_EPOCH_MS_SINCE_1601: i64 = 11_644_473_600_000;
    u64::try_from(ms.saturating_add(UNIX_EPOCH_MS_SINCE_1601))
        .unwrap_or(0)
        .saturating_mul(10_000)
}

/// One [`Agents::snapshots`] call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshots {
    /// One `Event::Sources` per source whose sessions or their liveness changed since the last call, in source order;
    /// every source on the first call and after [`Agents::forget_posted`].
    pub events: Vec<Event>,
    /// The process of every live session of every source (for a lock file session, the holders that keep it alive),
    /// and every process that holds a `Query` database open, sorted, so the caller can wait on its exit; the session
    /// then shows as `Gone` at the next call, and a database nobody holds any more stops its poll.
    pub live: Vec<ProcessRef>,
}

/// Every agent source, with what was last read and fed to it and what was last returned for it.
pub struct Agents {
    slots: Vec<Slot>,
    log: Arc<dyn Log>,
    io: AgentIo,
    started: SystemTime,
}

impl Agents {
    pub fn new(sources: Vec<Box<dyn AgentSource + Send>>, log: Arc<dyn Log>) -> Self {
        Self::with_io(sources, log, AgentIo::default())
    }

    /// [`new`](Self::new) with lock file holders from `owners`, so tests give the answer.
    pub fn with_owners(
        sources: Vec<Box<dyn AgentSource + Send>>,
        log: Arc<dyn Log>,
        owners: Box<dyn LockOwners>,
    ) -> Self {
        let io = AgentIo {
            owners,
            ..AgentIo::default()
        };
        Self::with_io(sources, log, io)
    }

    /// [`new`](Self::new) with every call besides file reads from `io`, so tests give the answers and the clocks.
    pub fn with_io(
        sources: Vec<Box<dyn AgentSource + Send>>,
        log: Arc<dyn Log>,
        io: AgentIo,
    ) -> Self {
        let started = (io.wall)();
        Self {
            slots: sources.into_iter().map(Slot::new).collect(),
            log,
            io,
            started,
        }
    }

    /// The folders to watch, sorted, each once. It changes only after a call that fed a source. Tails are not watched:
    /// each transcript line that matters comes with a status file write (architecture section 7). A `Query` watches
    /// its database's folder.
    pub fn folders(&self) -> Vec<WatchFolder> {
        let mut subtree: BTreeMap<&Path, bool> = BTreeMap::new();
        for want in self.slots.iter().flat_map(|slot| &slot.wants) {
            let (dir, deep) = match want {
                WantedFile::Folder { dir, .. } => (dir.as_path(), false),
                WantedFile::Names { dir, depth, .. } => (dir.as_path(), *depth > 0),
                WantedFile::Recent {
                    root, depth, sub, ..
                } => (root.as_path(), *depth > 0 || sub.is_some()),
                WantedFile::Query { db, .. } => match db.parent() {
                    Some(dir) => (dir, false),
                    None => continue,
                },
                WantedFile::Tail(_)
                | WantedFile::FindTail { .. }
                | WantedFile::FindNewest { .. } => {
                    continue;
                }
            };
            *subtree.entry(dir).or_default() |= deep;
        }
        subtree
            .into_iter()
            .map(|(dir, subtree)| WatchFolder {
                dir: dir.to_path_buf(),
                subtree,
            })
            .collect()
    }

    /// Reads every want of every source in full and polls every tail: the first round at start, and the safety tick
    /// that mends a lost notice. A folder file whose bytes the source already took is not fed again.
    pub fn on_tick(&mut self) -> Vec<Retry> {
        let ctx = Ctx::new(&self.io, self.started);
        let mut retries = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            // Restart Manager failed on a database last time; asking again on each 2 s poll would cost more.
            slot.hosts.retain(|_, hosts| *hosts != Hosts::Unknown);
            slot.run(&Scope::All, i, &mut retries, &ctx);
        }
        retries
    }

    /// A change notice in `folder`, one of [`folders`](Self::folders). `names` are the changed names relative to it;
    /// `None` when the notice lost them (an overflow, or the folder opened again), and then the folder is read in full.
    /// Runs each source that watches `folder`: its files there as `names` say, its listings of `folder`, its queries
    /// on a database there whose files `names` name, and every tail of that source.
    pub fn on_change(&mut self, folder: &Path, names: Option<&[PathBuf]>) -> Vec<Retry> {
        let ctx = Ctx::new(&self.io, self.started);
        let scope = Scope::Change(folder, names);
        let mut retries = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.watches(folder) {
                slot.forget_owners(folder, names);
                slot.forget_hosts(folder, names);
                slot.run(&scope, i, &mut retries, &ctx);
            }
        }
        retries
    }

    /// Reads and feeds `retries` again, even when their bytes did not change. A `RetryOnce` answer now is final: the
    /// retries this returns are only for other files the same call read.
    pub fn retry(&mut self, retries: &[Retry]) -> Vec<Retry> {
        let ctx = Ctx::new(&self.io, self.started);
        let mut again = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let mine: Vec<Retry> = retries.iter().filter(|r| r.slot == i).cloned().collect();
            if !mine.is_empty() {
                slot.run(&Scope::Retry(&mine), i, &mut again, &ctx);
            }
        }
        again
    }

    /// When the first `Query` run is due, its poll or a run held back by [`QUERY_EVERY_MS`], in [`AgentIo::now`] time:
    /// the caller then calls [`run_due`](Self::run_due).
    pub fn next_due(&self) -> Option<Instant> {
        self.slots
            .iter()
            .flat_map(|slot| slot.queries.values())
            .filter_map(|query| query.due)
            .min()
    }

    /// How many feeds and query reads ran so far. A caller that woke only for the `Query` poll publishes when this
    /// moved: a look at the `-shm` that found no commit changes nothing, and [`snapshots`](Self::snapshots) would list
    /// every process again for nothing.
    pub fn changes(&self) -> u64 {
        self.slots.iter().map(|slot| slot.changes).sum()
    }

    /// Runs the `Query` wants whose run is due, and every tail of their sources.
    pub fn run_due(&mut self) -> Vec<Retry> {
        let ctx = Ctx::new(&self.io, self.started);
        let mut retries = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let due = slot
                .queries
                .values()
                .any(|query| query.due.is_some_and(|due| due <= ctx.now));
            if due {
                slot.run(&Scope::Due, i, &mut retries, &ctx);
            }
        }
        retries
    }

    /// Every source's sessions, each checked once through `liveness`, as events for the sources whose snapshot
    /// changed. Logs format and read problems, and why a session whose files show work counts as ended, each once while
    /// it lasts, at most one line per source per minute of `now.mono_ms`.
    pub fn snapshots(&mut self, liveness: &dyn Liveness, now: Now) -> Snapshots {
        let mut out = Snapshots::default();
        let lock_owners = &*self.io.owners;
        // Asked at most once per call, for every `ImageSince` session of every source.
        let mut processes: Option<Option<Vec<ProcessEntry>>> = None;
        for slot in &mut self.slots {
            let mut problems = slot.io_problems();
            let mut known = std::mem::take(&mut slot.owners);
            let mut named = BTreeSet::new();
            let before: BTreeMap<&SessionKey, &SessionFacts> = slot
                .posted
                .iter()
                .flatten()
                .map(|(facts, _)| (&facts.key, facts))
                .collect();
            let sessions: Vec<(SessionFacts, Alive)> = slot
                .source
                .sessions()
                .into_iter()
                .map(|session| {
                    let alive = match &session.liveness {
                        sources::Liveness::Process { pid, start } => {
                            let process = ProcessRef {
                                pid: *pid,
                                start: *start,
                            };
                            let alive = check(liveness, process, &mut problems);
                            if alive == Alive::Yes {
                                out.live.push(process);
                            }
                            alive
                        }
                        sources::Liveness::AnyProcess { image, lock_file } => {
                            named.insert(lock_file.clone());
                            // A process can take up a lock file a crash left with no notice (Codex reopens it and
                            // writes nothing), but its turn writes the session's log, which changes these facts.
                            if before.get(&session.key) != Some(&&session)
                                && known
                                    .get(lock_file)
                                    .is_some_and(|held| held.counted.is_empty())
                            {
                                known.remove(lock_file);
                            }
                            let lock = LockFile {
                                image,
                                path: lock_file,
                            };
                            let alive = any_process(
                                liveness,
                                lock_owners,
                                &mut known,
                                &lock,
                                &mut problems,
                                &mut out.live,
                            );
                            // No answer kept means Restart Manager failed, which has its own line.
                            if alive == Alive::Gone
                                && shows_work(&session)
                                && let Some(held) = known.get(lock_file)
                            {
                                problems.insert(Problem::Ended {
                                    session: short_id(&session.key.id),
                                    why: Ended::NotHeld {
                                        image,
                                        named: named_pid(lock_file).is_some(),
                                        others: held.others.clone(),
                                    },
                                });
                            }
                            alive
                        }
                        sources::Liveness::ImageSince {
                            images,
                            since_wall_ms,
                        } => {
                            let list = processes.get_or_insert_with(|| liveness.list().ok());
                            let alive = image_since(
                                list.as_deref(),
                                images,
                                *since_wall_ms,
                                &mut problems,
                                &mut out.live,
                            );
                            if alive == Alive::Gone && shows_work(&session) {
                                problems.insert(Problem::Ended {
                                    session: short_id(&session.key.id),
                                    why: Ended::NoImage {
                                        images,
                                        since: since_wall_ms.is_some(),
                                    },
                                });
                            }
                            alive
                        }
                    };
                    match session.format {
                        FormatHealth::Ok => {}
                        FormatHealth::Unreadable(what) => {
                            problems.insert(Problem::Unreadable(what));
                        }
                        // The value is file content, so only the field is kept.
                        FormatHealth::UnknownValue(field, _) => {
                            problems.insert(Problem::UnknownValue(field));
                        }
                    }
                    (session, alive)
                })
                .collect();
            known.retain(|path, _| named.contains(path));
            slot.owners = known;
            slot.check_hosts(liveness, &mut out.live);
            slot.log_problems(&*self.log, now, problems);
            if slot.posted.as_ref() != Some(&sessions) {
                out.events.push(Event::Sources {
                    source: slot.source.id(),
                    sessions: sessions.clone(),
                });
                slot.posted = Some(sessions);
            }
        }
        out.live.sort();
        out.live.dedup();
        out
    }

    /// The next [`snapshots`](Self::snapshots) returns every source again, for a caller whose post failed.
    pub fn forget_posted(&mut self) {
        for slot in &mut self.slots {
            slot.posted = None;
        }
    }
}

// The watch thread owns `Agents`; a field that is not `Send` would break the thread's start, so it fails here instead.
const _: () = {
    const fn is_send<T: Send>() {}
    is_send::<Agents>();
};

fn check(liveness: &dyn Liveness, process: ProcessRef, problems: &mut BTreeSet<Problem>) -> Alive {
    let alive = liveness.check(process);
    if alive == Alive::Unknown {
        problems.insert(Problem::LivenessUnknown);
    }
    alive
}

/// An `AnyProcess` session's lock file and the image of the process that may hold it.
struct LockFile<'a> {
    image: &'static str,
    path: &'a Path,
}

/// What Restart Manager last said about one lock file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Holders {
    /// The holders that keep the session alive; empty when none did.
    counted: Vec<ProcessRef>,
    /// The images of the other holders, sorted, at most [`MAX_OTHERS`]: an agent whose exe got a new name shows here
    /// in the log instead of never holding without a word.
    others: Vec<String>,
}

/// The most other holders' images a log line names.
const MAX_OTHERS: usize = 4;

/// The session's files show work that holds while it lives (a turn, a wait, a timer, a background task), so a
/// liveness of `Gone` is what keeps it from holding. An idle session's liveness changes nothing, so it gets no line.
fn shows_work(facts: &SessionFacts) -> bool {
    facts.activity != Activity::Idle || facts.open_tasks > 0 || !facts.timers.is_empty()
}

/// A session id as the core's log lines show it: a pid as it is, any other id as 8 hex digits of its FNV-1a hash, so
/// a line here and the core's lines name one session the same way, and no id that is a file name reaches the log.
fn short_id(id: &str) -> String {
    if (1..=10).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()) {
        return id.to_owned();
    }
    let hash = id.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    format!("{hash:08x}")
}

/// `Yes` while a process running the image, or the process the lock file's name gives ([`named_pid`]), holds the lock
/// file open; its holders go to `live`. The holders found are kept in `known` until the lock file changes
/// ([`Slot::forget_owners`], a listing that feeds it) or all of them are gone, so Restart Manager is asked once per
/// lock file, not on every call. An answer of nobody is kept the same way, and also until the facts of the session
/// that names the file change ([`Agents::snapshots`]): a question costs 14 to 22 ms of CPU, and a crash leaves lock
/// files that nobody holds for days. Another holder (a virus scanner, say) does not count. If Restart Manager fails:
/// the process the name gives counts while it is listed, and goes to `live`; with no pid in the name, any live process
/// running the image counts while the source lists the lock file, and nothing goes to `live`.
fn any_process(
    liveness: &dyn Liveness,
    owners: &dyn LockOwners,
    known: &mut BTreeMap<PathBuf, Holders>,
    lock: &LockFile<'_>,
    problems: &mut BTreeSet<Problem>,
    live: &mut Vec<ProcessRef>,
) -> Alive {
    if let Some(held) = known.get(lock.path) {
        if held.counted.is_empty() {
            return Alive::Gone;
        }
        let answers: Vec<(ProcessRef, Alive)> = held
            .counted
            .iter()
            .map(|&p| (p, liveness.check(p)))
            .collect();
        let yes: Vec<ProcessRef> = answers
            .iter()
            .filter(|(_, alive)| *alive == Alive::Yes)
            .map(|(p, _)| *p)
            .collect();
        if !yes.is_empty() {
            live.extend(yes);
            return Alive::Yes;
        }
        if answers.iter().any(|(_, alive)| *alive == Alive::Unknown) {
            problems.insert(Problem::LivenessUnknown);
            return Alive::Unknown;
        }
        known.remove(lock.path);
    }
    let named = named_pid(lock.path);
    let of_image = |e: &ProcessEntry| e.image.eq_ignore_ascii_case(lock.image);
    let counts = |e: &ProcessEntry| of_image(e) || named == Some(e.pid);
    match owners.holders(lock.path) {
        Ok(holders) => {
            let mut held = Holders::default();
            if !holders.is_empty() {
                let Ok(list) = liveness.list() else {
                    problems.insert(Problem::LivenessUnknown);
                    return Alive::Unknown;
                };
                for h in holders {
                    match list.iter().find(|e| e.pid == h.pid && e.start == h.start) {
                        Some(e) if counts(e) => held.counted.push(h),
                        Some(e) => held.others.push(e.image.clone()),
                        None => {}
                    }
                }
                held.others.sort_unstable();
                held.others.dedup();
                held.others.truncate(MAX_OTHERS);
            }
            let alive = if held.counted.is_empty() {
                Alive::Gone
            } else {
                live.extend(&held.counted);
                Alive::Yes
            };
            known.insert(lock.path.to_path_buf(), held);
            alive
        }
        Err(e) => {
            problems.insert(Problem::NoLockOwner {
                image: lock.image,
                code: e.code,
                named: named.is_some(),
            });
            let Ok(list) = liveness.list() else {
                problems.insert(Problem::LivenessUnknown);
                return Alive::Unknown;
            };
            // A pid in the name is the holder's, so another process of the image (VS Code's `copilot` shim, a
            // second session) says nothing about this one.
            if let Some(pid) = named {
                return match list.iter().find(|e| e.pid == pid) {
                    Some(e) => {
                        live.push(ProcessRef {
                            pid: e.pid,
                            start: e.start,
                        });
                        Alive::Yes
                    }
                    None => Alive::Gone,
                };
            }
            if list.iter().any(of_image) {
                Alive::Yes
            } else {
                Alive::Gone
            }
        }
    }
}

/// The pid a lock file's name gives: its first dot-separated part that is a whole number, such as `52300` in
/// Copilot's `inuse.52300.hold`. A Codex lock (`<thread uuid>.lock`) gives none.
fn named_pid(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    name.split('.')
        .find(|part| {
            !part.is_empty() && part.len() <= 10 && part.bytes().all(|b| b.is_ascii_digit())
        })?
        .parse::<u32>()
        .ok()
        .filter(|&pid| pid != 0)
}

/// `Yes` while `list` has a process running one of `images` (case ignored) that started at or before `since_wall_ms`
/// plus the slack, or at any time for `None`; every such process goes to `live`, so the first exit among them is seen
/// at once. No list: `Unknown`.
fn image_since(
    list: Option<&[ProcessEntry]>,
    images: &[&str],
    since_wall_ms: Option<i64>,
    problems: &mut BTreeSet<Problem>,
    live: &mut Vec<ProcessRef>,
) -> Alive {
    let Some(list) = list else {
        problems.insert(Problem::LivenessUnknown);
        return Alive::Unknown;
    };
    let latest =
        since_wall_ms.map(|ms| filetime_of_unix_ms(ms.saturating_add(IMAGE_SINCE_SLACK_MS)));
    let matched: Vec<ProcessRef> = list
        .iter()
        .filter(|e| {
            images
                .iter()
                .any(|image| e.image.eq_ignore_ascii_case(image))
        })
        .filter(|e| latest.is_none_or(|latest| e.start.0 <= latest))
        .map(|e| ProcessRef {
            pid: e.pid,
            start: e.start,
        })
        .collect();
    if matched.is_empty() {
        return Alive::Gone;
    }
    live.extend(matched);
    Alive::Yes
}

/// What one call reads.
enum Scope<'a> {
    /// Every want in full: the tick and the first round.
    All,
    /// A notice in one folder, with the names it gave.
    Change(&'a Path, Option<&'a [PathBuf]>),
    /// Folder and recent files read once more, fed even when unchanged.
    Retry(&'a [Retry]),
    /// The queries whose held-back run is due.
    Due,
}

impl Scope<'_> {
    fn covers_folder(&self, dir: &Path) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(folder, _) => *folder == dir,
            Scope::Retry(retries) => retries
                .iter()
                .any(|r| matches!(&r.of, RetryOf::Folder { dir: d, .. } if d == dir)),
            Scope::Due => false,
        }
    }

    fn covers_listing(&self, dir: &Path) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(folder, _) => *folder == dir,
            Scope::Retry(_) | Scope::Due => false,
        }
    }

    fn covers_recent(&self, key: &RecentKey) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(folder, _) => *folder == key.root,
            Scope::Retry(retries) => retries
                .iter()
                .any(|r| matches!(&r.of, RetryOf::Recent { key: k, .. } if k == key)),
            Scope::Due => false,
        }
    }

    /// A run of a query not held back: `due` is when its held-back run is due.
    fn covers_query(&self, key: &QueryKey, due: Option<Instant>, now: Instant) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(..) => self.names_in_db(key, names_db_file),
            Scope::Retry(_) => false,
            Scope::Due => due.is_some_and(|due| due <= now),
        }
    }

    /// A notice in the database's folder with a name `names` passes, or one that lost its names.
    fn names_in_db(&self, key: &QueryKey, names: fn(&Path, &Path) -> bool) -> bool {
        match self {
            Scope::Change(folder, changed) => {
                key.db.parent() == Some(*folder)
                    && changed.is_none_or(|changed| changed.iter().any(|n| names(&key.db, n)))
            }
            Scope::All | Scope::Retry(_) | Scope::Due => false,
        }
    }

    /// The file names to read in `dir`, and whether unchanged bytes are fed too; `None` reads the folder in full.
    fn names_in(&self, dir: &Path) -> Option<(Vec<String>, bool)> {
        match self {
            Scope::All | Scope::Change(_, None) => None,
            Scope::Change(_, Some(changed)) => {
                let plain: BTreeSet<&str> = changed.iter().filter_map(|n| plain_name(n)).collect();
                Some((plain.into_iter().map(str::to_owned).collect(), false))
            }
            Scope::Retry(retries) => Some((
                retries
                    .iter()
                    .filter_map(|r| match &r.of {
                        RetryOf::Folder { dir: d, name } if d == dir => Some(name.clone()),
                        _ => None,
                    })
                    .collect(),
                true,
            )),
            Scope::Due => Some((Vec::new(), false)),
        }
    }

    /// The `Recent` files of `key` to read, and whether unchanged bytes are fed too; `None` lists them in full.
    fn recent_paths(&self, key: &RecentKey, accept: Accept) -> Option<(Vec<PathBuf>, bool)> {
        match self {
            Scope::All | Scope::Change(_, None) => None,
            Scope::Change(_, Some(changed)) => {
                let paths: BTreeSet<PathBuf> = changed
                    .iter()
                    .filter_map(|n| recent_name(key, accept, n))
                    .collect();
                Some((paths.into_iter().collect(), false))
            }
            Scope::Retry(retries) => Some((
                retries
                    .iter()
                    .filter_map(|r| match &r.of {
                        RetryOf::Recent { key: k, path } if k == key => Some(path.clone()),
                        _ => None,
                    })
                    .collect(),
                true,
            )),
            Scope::Due => Some((Vec::new(), false)),
        }
    }
}

/// A notice name that names the database or its `-wal`, right in its folder; case ignored. An agent's commit always
/// writes the `-wal`, and a checkpoint the database. Never the `-shm`: after a crash each read rebuilds the `-shm` the
/// agent left, so its notice would run the next read, three reads a second without end.
fn names_db_file(db: &Path, name: &Path) -> bool {
    db_file_rest(db, name).is_some_and(|rest| rest.is_empty() || rest.eq_ignore_ascii_case("-wal"))
}

/// A notice name that names any file of the database, its `-shm` included: the first process to open a database
/// truncates and rebuilds the `-shm` it finds, so an agent's start gives that notice while its commits give none.
fn names_any_db_file(db: &Path, name: &Path) -> bool {
    names_db_file(db, name)
        || db_file_rest(db, name).is_some_and(|rest| rest.eq_ignore_ascii_case("-shm"))
}

/// What follows the database's name in a notice name right in its folder, case ignored.
fn db_file_rest<'a>(db: &Path, name: &'a Path) -> Option<&'a str> {
    let (name, db_name) = (plain_name(name)?, file_name(db)?);
    name.get(..db_name.len())
        .filter(|head| head.eq_ignore_ascii_case(db_name))
        .map(|_| &name[db_name.len()..])
}

/// The full path of a notice name (relative to the want's root) that names a `Recent` file of `key`:
/// `<depth folders>\<sub>\<name>` with `accept(name)`. `sub` is compared without case and spelled as the want spells
/// it, as `recent_files` does, so one file always has one id.
fn recent_name(key: &RecentKey, accept: Accept, rel: &Path) -> Option<PathBuf> {
    let parts: Vec<&str> = rel
        .components()
        .map(|part| match part {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let (name, folders) = parts.split_last()?;
    if folders.len() != usize::from(key.depth) + usize::from(key.sub.is_some()) || !accept(name) {
        return None;
    }
    let mut path = key.root.clone();
    match key.sub {
        Some(sub) => {
            let (last, wild) = folders.split_last()?;
            if !last.eq_ignore_ascii_case(sub) {
                return None;
            }
            path.extend(wild);
            path.push(sub);
        }
        None => path.extend(folders),
    }
    path.push(name);
    Some(path)
}

/// A notice name that names a file right in the folder; anything with a separator or `..` is not a `Folder` file.
fn plain_name(name: &Path) -> Option<&str> {
    let mut parts = name.components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(name)), None) => name.to_str(),
        _ => None,
    }
}

/// One want's place in a call, so each is handled at most once per call.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Unit {
    Folder(PathBuf),
    Names(PathBuf, u8),
    Recent(RecentKey),
    Query(QueryKey),
    Tail(TailKey),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TailKey {
    Path(PathBuf),
    Find {
        root: PathBuf,
        depth: u8,
        suffix: String,
    },
    Newest {
        root: PathBuf,
        depth: u8,
        key: String,
    },
}

/// A `Recent` want, by what it reads; its `accept` lives in its state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RecentKey {
    root: PathBuf,
    depth: u8,
    sub: Option<&'static str>,
}

/// A `Query` want, by the id its answers come under; its SQL lives in its state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct QueryKey {
    db: PathBuf,
    name: &'static str,
}

fn tail_key(want: &WantedFile) -> Option<TailKey> {
    match want {
        WantedFile::Tail(path) => Some(TailKey::Path(path.clone())),
        WantedFile::FindTail {
            root,
            depth,
            name_suffix,
        } => Some(TailKey::Find {
            root: root.clone(),
            depth: *depth,
            suffix: name_suffix.clone(),
        }),
        WantedFile::FindNewest {
            root, depth, key, ..
        } => Some(TailKey::Newest {
            root: root.clone(),
            depth: *depth,
            key: key.clone(),
        }),
        WantedFile::Folder { .. }
        | WantedFile::Names { .. }
        | WantedFile::Recent { .. }
        | WantedFile::Query { .. } => None,
    }
}

fn recent_key(want: &WantedFile) -> Option<RecentKey> {
    match want {
        WantedFile::Recent {
            root, depth, sub, ..
        } => Some(RecentKey {
            root: root.clone(),
            depth: *depth,
            sub: *sub,
        }),
        _ => None,
    }
}

fn query_key(want: &WantedFile) -> Option<QueryKey> {
    match want {
        WantedFile::Query { db, name, .. } => Some(QueryKey {
            db: db.clone(),
            name,
        }),
        _ => None,
    }
}

fn wants_folder(wants: &[WantedFile], folder: &Path) -> bool {
    wants
        .iter()
        .any(|w| matches!(w, WantedFile::Folder { dir, .. } if dir == folder))
}

fn wants_listing(wants: &[WantedFile], folder: &Path, levels: u8) -> bool {
    wants.iter().any(
        |w| matches!(w, WantedFile::Names { dir, depth, .. } if dir == folder && *depth == levels),
    )
}

fn wants_tail(wants: &[WantedFile], key: &TailKey) -> bool {
    wants.iter().any(|w| tail_key(w).as_ref() == Some(key))
}

fn wants_recent(wants: &[WantedFile], key: &RecentKey) -> bool {
    wants.iter().any(|w| recent_key(w).as_ref() == Some(key))
}

fn wants_query(wants: &[WantedFile], key: &QueryKey) -> bool {
    wants.iter().any(|w| query_key(w).as_ref() == Some(key))
}

/// The bytes' hash, so a big `Recent` file read again unchanged is not fed again without keeping its bytes.
fn hash_of(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// A read error worth one more try: not a refused name or an oversized file, which a second read gives again.
fn may_pass(e: &io::Error) -> bool {
    !matches!(
        e.kind(),
        io::ErrorKind::InvalidInput | io::ErrorKind::FileTooLarge
    )
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

/// A want's name filter, run before any open.
type Accept = fn(&str) -> bool;

/// One source and the state of each of its wants. A want's state is dropped when the source stops wanting it, so a
/// want that comes back starts over (a tail from byte 0).
struct Slot {
    source: Box<dyn AgentSource + Send>,
    wants: Vec<WantedFile>,
    folders: BTreeMap<PathBuf, FolderState>,
    listings: BTreeMap<(PathBuf, u8), ListingState>,
    recents: BTreeMap<RecentKey, RecentState>,
    queries: BTreeMap<QueryKey, QueryState>,
    tails: BTreeMap<TailKey, TailState>,
    finder: Finder,
    /// The holders found for each lock file an `AnyProcess` session names.
    owners: BTreeMap<PathBuf, Holders>,
    /// Who holds each `Query` database open; none for a database not asked about yet.
    hosts: BTreeMap<PathBuf, Hosts>,
    posted: Option<Vec<(SessionFacts, Alive)>>,
    logged: BTreeSet<Problem>,
    last_line_ms: Option<u64>,
    /// Feeds and query reads so far ([`Agents::changes`]).
    changes: u64,
}

struct FolderState {
    accept: Accept,
    /// Read in full at the next call, whatever its scope: the want is new.
    fresh: bool,
    /// Each file the source was fed and not told is gone, with the bytes it took; `None` while a `RetryOnce` waits for
    /// its retry, so a file whose retry never came is fed again.
    known: BTreeMap<String, Option<Vec<u8>>>,
    unread: BTreeMap<String, Option<i32>>,
    unlisted: Option<Option<i32>>,
}

struct ListingState {
    accept: Accept,
    fresh: bool,
    listed: BTreeSet<PathBuf>,
    unlisted: Option<Option<i32>>,
}

struct RecentState {
    accept: Accept,
    /// Listed in full at the next call: the want is new.
    fresh: bool,
    /// Each file the source was fed and not told is gone: the stamp it was read at and the hash of the bytes it took;
    /// no hash while a `RetryOnce` waits for its retry, so a file whose retry never came is fed again.
    known: BTreeMap<PathBuf, (Stamp, Option<u64>)>,
    unread: BTreeMap<PathBuf, Option<i32>>,
    unlisted: Option<Option<i32>>,
}

struct QueryState {
    sql: &'static str,
    /// Run at the next call, whatever its scope: the want is new.
    fresh: bool,
    last_run: Option<Instant>,
    /// The next run without a notice: the poll, or a run held back by the limit.
    due: Option<Instant>,
    /// A notice named the database since the last read, so the next run reads whatever the header says.
    noticed: bool,
    /// The WAL-index header after the last read that answered; `None` until one did. A run reads again only when
    /// the header differs, as only a commit changes it.
    header: Option<Option<[u8; sqlite::HEADER_LEN]>>,
    fed: QueryFed,
    /// The fixed word of the last answer when it was an error, until rows come or the database closes.
    failed: Option<&'static str>,
    /// At the last run the database was there without its `-wal`, so it was not read.
    no_wal: bool,
}

/// The last feed of one query, so an unchanged answer is not fed again and `Gone` is fed once.
#[derive(Debug, PartialEq, Eq)]
enum QueryFed {
    Nothing,
    Answer(Vec<u8>),
    Gone,
}

#[derive(Debug, Default)]
struct TailState {
    tail: Option<Tail>,
    /// The last poll found the file, so its going is fed once as `Gone`.
    present: bool,
    unread: Option<Option<i32>>,
    /// A `FindTail` or `FindNewest` found no file and a folder on its way could not be listed, so the file may be
    /// there unseen.
    unlisted: Option<Option<i32>>,
}

impl Slot {
    fn new(source: Box<dyn AgentSource + Send>) -> Self {
        let wants = source.wanted();
        let mut slot = Self {
            source,
            wants,
            folders: BTreeMap::new(),
            listings: BTreeMap::new(),
            recents: BTreeMap::new(),
            queries: BTreeMap::new(),
            tails: BTreeMap::new(),
            finder: Finder::new(),
            owners: BTreeMap::new(),
            hosts: BTreeMap::new(),
            posted: None,
            logged: BTreeSet::new(),
            last_line_ms: None,
            changes: 0,
        };
        slot.sync();
        slot
    }

    fn watches(&self, folder: &Path) -> bool {
        self.wants.iter().any(|w| match w {
            WantedFile::Folder { dir, .. } | WantedFile::Names { dir, .. } => dir == folder,
            WantedFile::Recent { root, .. } => root == folder,
            WantedFile::Query { db, .. } => db.parent() == Some(folder),
            WantedFile::Tail(_) | WantedFile::FindTail { .. } | WantedFile::FindNewest { .. } => {
                false
            }
        })
    }

    /// A change notice in `folder`: the holders of the lock files it names are asked again, every one under `folder`
    /// when the notice lost its names. A lock file deleted and made again between two listings lists the same, so
    /// the notice is what tells.
    fn forget_owners(&mut self, folder: &Path, names: Option<&[PathBuf]>) {
        match names {
            Some(names) => {
                for name in names {
                    self.owners.remove(&folder.join(name));
                }
            }
            None => self.owners.retain(|path, _| !path.starts_with(folder)),
        }
    }

    /// The processes that keep a `Query` poll on go to `live`, so their exit wakes the caller; a database whose holders
    /// all ended is asked about again at its next run (the poll, still due). A check that fails keeps the process and
    /// logs nothing: the poll then only runs a little longer.
    fn check_hosts(&mut self, liveness: &dyn Liveness, live: &mut Vec<ProcessRef>) {
        self.hosts.retain(|_, hosts| {
            let Hosts::Held(held, _) = hosts else {
                return true;
            };
            held.retain(|&p| match liveness.check(p) {
                Alive::Yes => {
                    live.push(p);
                    true
                }
                Alive::Unknown => true,
                Alive::Gone => false,
            });
            !held.is_empty()
        });
    }

    /// A notice that names a file of a database nobody held: someone may hold it now, so it is asked about again. A
    /// database already held stays so until its holders end, so the notices of a working agent cost no question.
    fn forget_hosts(&mut self, folder: &Path, names: Option<&[PathBuf]>) {
        self.hosts.retain(|db, hosts| {
            let named = db.parent() == Some(folder)
                && names.is_none_or(|names| names.iter().any(|n| names_any_db_file(db, n)));
            !(named && *hosts == Hosts::Nobody)
        });
    }

    /// Feeds one file and asks the source what it wants now (wants change only after a feed: the `AgentSource`
    /// promise). The state of every want that left goes at once, so a want that leaves and comes back within one call
    /// starts over: a resumed session whose leftover status file was fed `Gone` first gets its transcript from byte 0.
    fn feed(&mut self, path: PathBuf, data: Feed) -> Fed {
        self.changes += 1;
        let fed = self.source.feed(&FileId(path), data);
        self.wants = self.source.wanted();
        self.prune();
        fed
    }

    fn prune(&mut self) {
        let wants = &self.wants;
        self.folders.retain(|dir, _| wants_folder(wants, dir));
        self.listings
            .retain(|(dir, depth), _| wants_listing(wants, dir, *depth));
        self.recents.retain(|key, _| wants_recent(wants, key));
        self.queries.retain(|key, _| wants_query(wants, key));
        self.tails.retain(|key, _| wants_tail(wants, key));
        self.hosts.retain(|db, _| {
            wants
                .iter()
                .any(|w| matches!(w, WantedFile::Query { db: d, .. } if d == db))
        });
    }

    /// Drops the state of wants that left and adds fresh state for new folders, listings, recent files and queries.
    /// Tail state is made when the tail is first polled.
    fn sync(&mut self) {
        self.prune();
        let mut folders: BTreeMap<&Path, Accept> = BTreeMap::new();
        let mut listings: BTreeMap<(&Path, u8), Accept> = BTreeMap::new();
        let mut recents: BTreeMap<RecentKey, Accept> = BTreeMap::new();
        let mut queries: BTreeMap<QueryKey, &'static str> = BTreeMap::new();
        for want in &self.wants {
            match want {
                WantedFile::Folder { dir, accept } => {
                    folders.entry(dir.as_path()).or_insert(*accept);
                }
                WantedFile::Names { dir, depth, accept } => {
                    listings.entry((dir.as_path(), *depth)).or_insert(*accept);
                }
                WantedFile::Recent { accept, .. } => {
                    if let Some(key) = recent_key(want) {
                        recents.entry(key).or_insert(*accept);
                    }
                }
                WantedFile::Query { sql, .. } => {
                    if let Some(key) = query_key(want) {
                        queries.entry(key).or_insert(*sql);
                    }
                }
                WantedFile::Tail(_)
                | WantedFile::FindTail { .. }
                | WantedFile::FindNewest { .. } => {}
            }
        }
        for (dir, accept) in folders {
            self.folders
                .entry(dir.to_path_buf())
                .or_insert_with(|| FolderState {
                    accept,
                    fresh: true,
                    known: BTreeMap::new(),
                    unread: BTreeMap::new(),
                    unlisted: None,
                })
                .accept = accept;
        }
        for ((dir, depth), accept) in listings {
            self.listings
                .entry((dir.to_path_buf(), depth))
                .or_insert_with(|| ListingState {
                    accept,
                    fresh: true,
                    listed: BTreeSet::new(),
                    unlisted: None,
                })
                .accept = accept;
        }
        for (key, accept) in recents {
            self.recents
                .entry(key)
                .or_insert_with(|| RecentState {
                    accept,
                    fresh: true,
                    known: BTreeMap::new(),
                    unread: BTreeMap::new(),
                    unlisted: None,
                })
                .accept = accept;
        }
        for (key, sql) in queries {
            self.queries
                .entry(key)
                .or_insert_with(|| QueryState {
                    sql,
                    fresh: true,
                    last_run: None,
                    due: None,
                    noticed: false,
                    header: None,
                    fed: QueryFed::Nothing,
                    failed: None,
                    no_wal: false,
                })
                .sql = sql;
        }
    }

    /// Folders, listings, recent files and queries first, so a tail that a status file just named is polled in the
    /// same call.
    fn run(&mut self, scope: &Scope<'_>, slot: usize, retries: &mut Vec<Retry>, ctx: &Ctx<'_>) {
        // The trait promises wants change only after a feed; asking costs little and keeps a source that breaks the
        // promise from being read with stale wants.
        self.wants = self.source.wanted();
        // `FindNewest` must see a file made since the last call.
        self.finder.forget_walks();
        let mut done = BTreeSet::new();
        for _ in 0..UNITS_PER_RUN {
            self.sync();
            let Some(unit) = self.next_unit(scope, &done, ctx.now) else {
                break;
            };
            match &unit {
                Unit::Folder(dir) => self.read_folder(dir, scope, slot, retries),
                Unit::Names(dir, depth) => self.list(dir, *depth),
                Unit::Recent(key) => self.read_recent(key, scope, slot, retries, ctx),
                Unit::Query(key) => {
                    self.run_query(key, ctx, scope.names_in_db(key, names_db_file));
                }
                Unit::Tail(key) => self.poll_tail(key),
            }
            done.insert(unit);
        }
        self.sync();
        self.finder.forget_walks();
    }

    fn next_unit(&self, scope: &Scope<'_>, done: &BTreeSet<Unit>, now: Instant) -> Option<Unit> {
        let reads = self.wants.iter().filter_map(|want| match want {
            WantedFile::Folder { dir, .. } => {
                let fresh = self.folders.get(dir).is_some_and(|f| f.fresh);
                (fresh || scope.covers_folder(dir)).then(|| Unit::Folder(dir.clone()))
            }
            WantedFile::Names { dir, depth, .. } => {
                let key = (dir.clone(), *depth);
                let fresh = self.listings.get(&key).is_some_and(|l| l.fresh);
                (fresh || scope.covers_listing(dir)).then_some(Unit::Names(key.0, key.1))
            }
            WantedFile::Recent { .. } => {
                let key = recent_key(want)?;
                let fresh = self.recents.get(&key).is_some_and(|r| r.fresh);
                (fresh || scope.covers_recent(&key)).then_some(Unit::Recent(key))
            }
            WantedFile::Query { .. } => {
                let key = query_key(want)?;
                let state = self.queries.get(&key);
                let fresh = state.is_some_and(|q| q.fresh);
                let due = state.and_then(|q| q.due);
                // A `-shm` notice runs a query only to ask who holds the database (`forget_hosts`), never to read.
                let ask =
                    !self.hosts.contains_key(&key.db) && scope.names_in_db(&key, names_any_db_file);
                (fresh || ask || scope.covers_query(&key, due, now)).then_some(Unit::Query(key))
            }
            WantedFile::Tail(_) | WantedFile::FindTail { .. } | WantedFile::FindNewest { .. } => {
                None
            }
        });
        let tails = self.wants.iter().filter_map(tail_key).map(Unit::Tail);
        reads.chain(tails).find(|unit| !done.contains(unit))
    }

    fn read_recent(
        &mut self,
        key: &RecentKey,
        scope: &Scope<'_>,
        slot: usize,
        retries: &mut Vec<Retry>,
        ctx: &Ctx<'_>,
    ) {
        let Some(mut state) = self.recents.remove(key) else {
            return;
        };
        if self.fill_recent(&mut state, key, scope, slot, retries, ctx) {
            self.recents.insert(key.clone(), state);
        }
    }

    /// `false`: the source stopped wanting `key`, so its state is dropped.
    fn fill_recent(
        &mut self,
        state: &mut RecentState,
        key: &RecentKey,
        scope: &Scope<'_>,
        slot: usize,
        retries: &mut Vec<Retry>,
        ctx: &Ctx<'_>,
    ) -> bool {
        let accept = state.accept;
        let planned = if state.fresh {
            None
        } else {
            scope.recent_paths(key, accept)
        };
        // Each file to read, with its stamp when the listing chose it.
        let (targets, forced): (Vec<(PathBuf, Option<Stamp>)>, bool) = match planned {
            Some((paths, forced)) => (paths.into_iter().map(|p| (p, None)).collect(), forced),
            None => {
                state.fresh = false;
                let found = match recent_files(&key.root, key.depth, key.sub, accept) {
                    Ok(found) => found,
                    // The known files stay as they were, so a listing error never reads as every session gone.
                    Err(e) => {
                        state.unlisted = Some(e.raw_os_error());
                        return true;
                    }
                };
                state.unlisted = found.failed.first().map(|(_, code)| *code);
                let listed: BTreeSet<&PathBuf> = found.files.iter().map(|(path, _)| path).collect();
                state.unread.retain(|path, _| listed.contains(path));
                let gone: Vec<PathBuf> = state
                    .known
                    .keys()
                    .filter(|path| !listed.contains(path))
                    .filter(|path| !found.failed.iter().any(|(dir, _)| path.starts_with(dir)))
                    .cloned()
                    .collect();
                for path in gone {
                    state.known.remove(&path);
                    self.feed(path, Feed::Gone);
                    if !wants_recent(&self.wants, key) {
                        return false;
                    }
                }
                let targets = found
                    .files
                    .into_iter()
                    .filter(|(path, stamp)| match state.known.get(path) {
                        Some((known, Some(_))) => known != stamp,
                        // A retry that never came.
                        Some((_, None)) => true,
                        None => stamp.written >= ctx.recent_since,
                    })
                    .map(|(path, stamp)| (path, Some(stamp)))
                    .collect();
                (targets, false)
            }
        };
        for (path, listed) in targets {
            if !file_name(&path).is_some_and(accept) {
                continue;
            }
            match read_whole_stamped(&path, accept, RECENT_CAP) {
                Ok((bytes, read)) => {
                    state.unread.remove(&path);
                    // The listing's stamp when it chose the read, so the file lists the same at the next call.
                    let stamp = listed.unwrap_or(read);
                    let hash = hash_of(&bytes);
                    let taken = state.known.get(&path).and_then(|(_, hash)| *hash);
                    if !forced && taken == Some(hash) {
                        state.known.insert(path, (stamp, Some(hash)));
                        continue;
                    }
                    let fed = self.feed(path.clone(), Feed::Whole(bytes));
                    // After its one retry the source has had its answer, so the same bytes are not fed again.
                    let settled = fed == Fed::Taken || forced;
                    state
                        .known
                        .insert(path.clone(), (stamp, settled.then_some(hash)));
                    if fed == Fed::RetryOnce && !forced {
                        retries.push(Retry {
                            slot,
                            of: RetryOf::Recent {
                                key: key.clone(),
                                path,
                            },
                        });
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    state.unread.remove(&path);
                    if state.known.remove(&path).is_none() {
                        continue;
                    }
                    self.feed(path, Feed::Gone);
                }
                // The source keeps what it had: a file that cannot be read now is not gone.
                Err(e) => {
                    state.unread.insert(path.clone(), e.raw_os_error());
                    if !forced && may_pass(&e) {
                        retries.push(Retry {
                            slot,
                            of: RetryOf::Recent {
                                key: key.clone(),
                                path,
                            },
                        });
                    }
                    continue;
                }
            }
            if !wants_recent(&self.wants, key) {
                return false;
            }
        }
        true
    }

    /// Runs one query, unless its last read was less than [`QUERY_EVERY_MS`] ago: then a run is due at that time. While
    /// a process other than Wakeshot holds the database open, the next run is due [`QUERY_POLL_MS`] later. `noticed`:
    /// a change notice named the database or its `-wal`; any other run reads only when the WAL-index header changed
    /// since the last read.
    fn run_query(&mut self, key: &QueryKey, ctx: &Ctx<'_>, noticed: bool) {
        let Some(mut state) = self.queries.remove(key) else {
            return;
        };
        state.fresh = false;
        state.noticed |= noticed;
        let every = Duration::from_millis(QUERY_EVERY_MS);
        if let Some(next) = state.last_run.map(|last| last + every)
            && ctx.now < next
        {
            state.due = Some(state.due.map_or(next, |due| due.min(next)));
            self.queries.insert(key.clone(), state);
            return;
        }
        let there = key.db.is_file();
        if !there {
            self.hosts.remove(&key.db);
        }
        let mut wal = key.db.clone().into_os_string();
        wal.push("-wal");
        // The `-wal` file lives while the agent has the database open, and a read without it could make one.
        let open = there && Path::new(&wal).is_file();
        state.no_wal = there && !open;
        let feed = if !open {
            state.header = None;
            state.noticed = false;
            state.failed = None;
            matches!(state.fed, QueryFed::Answer(_)).then_some(Feed::Gone)
        } else if !state.noticed && state.header == Some(sqlite::wal_index_header(&key.db)) {
            // No commit since the last read: the same rows. Files a stopped agent left are so read once, not on each
            // tick.
            None
        } else {
            state.last_run = Some(ctx.now);
            state.noticed = false;
            self.changes += 1;
            // Something wrote the database since the last read, so someone may hold it now.
            if self.hosts.get(&key.db) == Some(&Hosts::Nobody)
                && !ctx.asked.borrow().contains(&key.db)
            {
                self.hosts.remove(&key.db);
            }
            match (ctx.query)(&key.db, state.sql) {
                // The header is kept from before, so the next look reads again.
                sqlite::Answer::Busy => None,
                sqlite::Answer::Json(bytes) => {
                    state.failed = sqlite::error_kind(&bytes);
                    // After the read: on files a stopped agent left, the read itself rebuilt the header.
                    state.header = Some(sqlite::wal_index_header(&key.db));
                    let same = matches!(&state.fed, QueryFed::Answer(last) if *last == bytes);
                    (!same).then_some(Feed::Whole(bytes))
                }
            }
        };
        if let Some(feed) = feed {
            state.fed = match &feed {
                Feed::Whole(bytes) => QueryFed::Answer(bytes.clone()),
                _ => QueryFed::Gone,
            };
            self.feed(FileId::query(&key.db, key.name).0, feed);
        }
        // A database not made yet needs no poll: the notice of its making names it.
        let polls = there && self.hosts_poll(&key.db, ctx);
        state.due = polls.then(|| ctx.now + Duration::from_millis(QUERY_POLL_MS));
        if wants_query(&self.wants, key) {
            self.queries.insert(key.clone(), state);
        }
    }

    /// Whether `db`'s poll runs, asking Restart Manager first when no answer is kept: only a process that holds the
    /// database open can commit, and a poll wakes the thread every 2 s for about 1 ms of CPU each time. Wakeshot's own
    /// reads do not count.
    fn hosts_poll(&mut self, db: &Path, ctx: &Ctx<'_>) -> bool {
        let recheck = Duration::from_millis(HOSTS_RECHECK_MS);
        if let Some(Hosts::Held(_, asked)) = self.hosts.get(db)
            && ctx.now >= *asked + recheck
            && !ctx.asked.borrow().contains(db)
        {
            self.hosts.remove(db);
        }
        self.hosts
            .entry(db.to_path_buf())
            .or_insert_with(|| {
                ctx.asked.borrow_mut().insert(db.to_path_buf());
                match ctx.owners.holders(db) {
                    Ok(holders) => {
                        let me = std::process::id();
                        let held: Vec<ProcessRef> =
                            holders.into_iter().filter(|h| h.pid != me).collect();
                        if held.is_empty() {
                            Hosts::Nobody
                        } else {
                            Hosts::Held(held, ctx.now)
                        }
                    }
                    Err(_) => Hosts::Unknown,
                }
            })
            .polls()
    }

    fn read_folder(
        &mut self,
        dir: &Path,
        scope: &Scope<'_>,
        slot: usize,
        retries: &mut Vec<Retry>,
    ) {
        let Some(mut state) = self.folders.remove(dir) else {
            return;
        };
        if self.fill_folder(&mut state, dir, scope, slot, retries) {
            self.folders.insert(dir.to_path_buf(), state);
        }
    }

    /// `false`: the source stopped wanting `dir`, so its state is dropped.
    fn fill_folder(
        &mut self,
        state: &mut FolderState,
        dir: &Path,
        scope: &Scope<'_>,
        slot: usize,
        retries: &mut Vec<Retry>,
    ) -> bool {
        let accept = state.accept;
        let planned = if state.fresh {
            None
        } else {
            scope.names_in(dir)
        };
        let (targets, forced) = match planned {
            Some(planned) => planned,
            None => {
                state.fresh = false;
                match names(dir, 0, accept) {
                    Ok(paths) => {
                        let listed: BTreeSet<String> = paths
                            .iter()
                            .filter_map(|p| file_name(p))
                            .map(str::to_owned)
                            .collect();
                        state.unlisted = None;
                        state.unread.retain(|name, _| listed.contains(name));
                        let gone: Vec<String> = state
                            .known
                            .keys()
                            .filter(|name| !listed.contains(*name))
                            .cloned()
                            .collect();
                        for name in gone {
                            state.known.remove(&name);
                            self.feed(dir.join(&name), Feed::Gone);
                            if !wants_folder(&self.wants, dir) {
                                return false;
                            }
                        }
                        (listed.into_iter().collect(), false)
                    }
                    // The known files are read one by one instead: a listing error never reads as every session
                    // gone, and a status file that changes or goes still reaches its source. A new file waits until
                    // the folder lists again.
                    Err(e) => {
                        state.unlisted = Some(e.raw_os_error());
                        (state.known.keys().cloned().collect(), false)
                    }
                }
            }
        };
        for name in targets {
            if !accept(&name) {
                continue;
            }
            let path = dir.join(&name);
            match read_whole(&path, accept) {
                Ok(bytes) => {
                    state.unread.remove(&name);
                    let taken = state.known.get(&name).and_then(Option::as_deref);
                    if !forced && taken == Some(bytes.as_slice()) {
                        continue;
                    }
                    let fed = self.feed(path, Feed::Whole(bytes.clone()));
                    // After its one retry the source has had its answer, so the same bytes are not fed again.
                    let settled = fed == Fed::Taken || forced;
                    state.known.insert(name.clone(), settled.then_some(bytes));
                    if fed == Fed::RetryOnce && !forced {
                        retries.push(Retry {
                            slot,
                            of: RetryOf::Folder {
                                dir: dir.to_path_buf(),
                                name,
                            },
                        });
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    state.unread.remove(&name);
                    if state.known.remove(&name).is_none() {
                        continue;
                    }
                    self.feed(path, Feed::Gone);
                }
                // The source keeps what it had: a file that cannot be read now is not gone.
                Err(e) => {
                    state.unread.insert(name.clone(), e.raw_os_error());
                    if !forced && may_pass(&e) {
                        retries.push(Retry {
                            slot,
                            of: RetryOf::Folder {
                                dir: dir.to_path_buf(),
                                name,
                            },
                        });
                    }
                    continue;
                }
            }
            if !wants_folder(&self.wants, dir) {
                return false;
            }
        }
        true
    }

    fn list(&mut self, dir: &Path, depth: u8) {
        let key = (dir.to_path_buf(), depth);
        let Some(mut state) = self.listings.remove(&key) else {
            return;
        };
        if self.fill_listing(&mut state, dir, depth) {
            self.listings.insert(key, state);
        }
    }

    /// `false`: the source stopped wanting this listing, so its state is dropped.
    fn fill_listing(&mut self, state: &mut ListingState, dir: &Path, depth: u8) -> bool {
        state.fresh = false;
        let listed: BTreeSet<PathBuf> = match names(dir, depth, state.accept) {
            Ok(paths) => paths.into_iter().collect(),
            // The last list stays, so a listing error never reads as every name gone.
            Err(e) => {
                state.unlisted = Some(e.raw_os_error());
                return true;
            }
        };
        state.unlisted = None;
        let gone: Vec<PathBuf> = state.listed.difference(&listed).cloned().collect();
        let new: Vec<PathBuf> = listed.difference(&state.listed).cloned().collect();
        for path in gone {
            state.listed.remove(&path);
            self.owners.remove(&path);
            self.feed(path, Feed::Gone);
            if !wants_listing(&self.wants, dir, depth) {
                return false;
            }
        }
        for path in new {
            state.listed.insert(path.clone());
            self.owners.remove(&path);
            self.feed(path, Feed::Present);
            if !wants_listing(&self.wants, dir, depth) {
                return false;
            }
        }
        true
    }

    /// Feeds what the tail's file gained. A restart (the file got shorter or was replaced) is fed as `Gone`, then the
    /// lines from the start, since `Feed` has no restart of its own. A tail feed's
    /// `RetryOnce` is not retried: complete lines read again would be the same lines.
    fn poll_tail(&mut self, key: &TailKey) {
        let mut state = self.tails.remove(key).unwrap_or_default();
        match key {
            TailKey::Path(path) => {
                if state.tail.is_none() {
                    state.tail = Some(Tail::new(path.clone()));
                }
            }
            TailKey::Find {
                root,
                depth,
                suffix,
            } => {
                let walked = self.finder.find(root, *depth, suffix);
                // A folder that could not be listed matters only while it may hide the one file wanted.
                state.unlisted = walked.found.is_empty().then_some(walked.unlisted).flatten();
                let found = walked.found;
                // The file found first stays the one tailed while it is still found, so a second match never splits
                // a source's lines across two files.
                let kept = state
                    .tail
                    .as_ref()
                    .is_some_and(|tail| found.iter().any(|p| p == tail.path()));
                if !kept {
                    state.unread = None;
                    if let Some(old) = state.tail.take()
                        && std::mem::take(&mut state.present)
                        && !self.feed_tail(key, old.path(), Feed::Gone)
                    {
                        return;
                    }
                    state.tail = found.into_iter().next().map(Tail::new);
                }
            }
            TailKey::Newest {
                root,
                depth,
                key: of,
            } => {
                let matches = self.wants.iter().find_map(|want| match want {
                    WantedFile::FindNewest {
                        root: r,
                        depth: d,
                        key: k,
                        matches,
                    } if r == root && d == depth && k == of => Some(*matches),
                    _ => None,
                });
                let Some(matches) = matches else {
                    return;
                };
                let walked = self.finder.newest(root, *depth, |name| matches(name, of));
                state.unlisted = walked.found.is_none().then_some(walked.unlisted).flatten();
                let newest = walked.found;
                let kept = state
                    .tail
                    .as_ref()
                    .is_some_and(|tail| newest.as_deref() == Some(tail.path()));
                if !kept {
                    state.unread = None;
                    // The source drops what the old file said, then reads the new one from its start.
                    if let Some(old) = state.tail.take()
                        && std::mem::take(&mut state.present)
                        && !self.feed_tail(key, old.path(), Feed::Gone)
                    {
                        return;
                    }
                    state.tail = newest.map(Tail::new);
                }
            }
        }
        let Some(mut tail) = state.tail.take() else {
            self.tails.insert(key.clone(), state);
            return;
        };
        loop {
            match tail.poll() {
                Ok(read) => {
                    state.unread = None;
                    // A file that went away was already fed as gone.
                    if read.restarted
                        && state.present
                        && !self.feed_tail(key, tail.path(), Feed::Gone)
                    {
                        return;
                    }
                    state.present = true;
                    if !read.lines.is_empty()
                        && !self.feed_tail(key, tail.path(), Feed::Appended(read.lines))
                    {
                        return;
                    }
                    if !read.more {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    state.unread = None;
                    if std::mem::take(&mut state.present)
                        && !self.feed_tail(key, tail.path(), Feed::Gone)
                    {
                        return;
                    }
                    break;
                }
                Err(e) => {
                    state.unread = Some(e.raw_os_error());
                    break;
                }
            }
        }
        state.tail = Some(tail);
        self.tails.insert(key.clone(), state);
    }

    /// `false`: the source stopped wanting this tail, so its place is forgotten.
    fn feed_tail(&mut self, key: &TailKey, path: &Path, data: Feed) -> bool {
        self.feed(path.to_path_buf(), data);
        wants_tail(&self.wants, key)
    }

    fn io_problems(&self) -> BTreeSet<Problem> {
        let folders = self.folders.values().flat_map(|f| {
            let unread = f.unread.values().map(|code| Problem::Unread(*code));
            unread.chain(f.unlisted.map(Problem::Unlisted))
        });
        let listings = self
            .listings
            .values()
            .filter_map(|l| l.unlisted.map(Problem::Unlisted));
        let recents = self.recents.values().flat_map(|r| {
            let unread = r.unread.values().map(|code| Problem::Unread(*code));
            unread.chain(r.unlisted.map(Problem::Unlisted))
        });
        let tails = self.tails.values().flat_map(|t| {
            let unread = t.unread.map(Problem::Unread);
            unread.into_iter().chain(t.unlisted.map(Problem::Unlisted))
        });
        let queries = self.queries.iter().flat_map(|(key, q)| {
            // Only while another process has the database open: with nobody there a missing `-wal` is the normal
            // state of a closed database.
            let held = matches!(self.hosts.get(&key.db), Some(Hosts::Held(..)));
            let no_wal = (q.no_wal && held).then_some(Problem::NoWal);
            q.failed.map(Problem::QueryFailed).into_iter().chain(no_wal)
        });
        folders
            .chain(listings)
            .chain(recents)
            .chain(tails)
            .chain(queries)
            .collect()
    }

    /// Logs the problems not logged yet in one line, at most once per [`LOG_EVERY_MS`]; one held back by the limit is
    /// logged by a later call if it is still there. A problem that cleared is forgotten, so it is logged again if it
    /// comes back.
    fn log_problems(&mut self, log: &dyn Log, now: Now, problems: BTreeSet<Problem>) {
        self.logged.retain(|p| problems.contains(p));
        let new: Vec<Problem> = problems
            .into_iter()
            .filter(|p| !self.logged.contains(p))
            .collect();
        if new.is_empty()
            || self
                .last_line_ms
                .is_some_and(|last| now.mono_ms.saturating_sub(last) < LOG_EVERY_MS)
        {
            return;
        }
        let texts: Vec<String> = new.iter().map(Problem::text).collect();
        let name = source_name(self.source.id());
        log.line(Level::Warn, AREA, &format!("{name}: {}", texts.join("; ")));
        self.last_line_ms = Some(now.mono_ms);
        self.logged.extend(new);
    }
}

/// What one log line can name: field names, fixed texts, error codes, short session ids and exe names, never file
/// content or a path (a path holds the user's name; a `.key` name must never reach the log).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Problem {
    Unreadable(&'static str),
    UnknownValue(&'static str),
    /// `named`: the lock file's name gives the holder's pid, so that process counts, not every one of the image.
    NoLockOwner {
        image: &'static str,
        code: u32,
        named: bool,
    },
    LivenessUnknown,
    Unlisted(Option<i32>),
    Unread(Option<i32>),
    /// A session whose files show work counts as ended, so it does not hold. `session` is its short id.
    Ended {
        session: String,
        why: Ended,
    },
    /// A query answered with this error word ([`sqlite::error_kind`]).
    QueryFailed(&'static str),
    /// A database another process has open has no `-wal`, so it is not read.
    NoWal,
}

/// Why a session counts as ended.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Ended {
    /// `AnyProcess`: no holder that counts. `named`: the lock file's name gives a pid, which would count too. `others`:
    /// the images of the holders that do not count.
    NotHeld {
        image: &'static str,
        named: bool,
        others: Vec<String>,
    },
    /// `ImageSince`: no process of the images runs, or `since`, none that started early enough.
    NoImage {
        images: &'static [&'static str],
        since: bool,
    },
}

impl Ended {
    fn text(&self) -> String {
        match self {
            Ended::NotHeld {
                image,
                named,
                others,
            } => {
                let who = if *named {
                    format!("neither {image} nor the process its name gives holds its lock file")
                } else {
                    format!("no {image} holds its lock file")
                };
                if others.is_empty() {
                    who
                } else {
                    format!("{who}, only {}", others.join(", "))
                }
            }
            Ended::NoImage { images, since } => {
                let images = images.join(" or ");
                if *since {
                    format!("no {images} that started before it runs")
                } else {
                    format!("no {images} runs")
                }
            }
        }
    }
}

impl Problem {
    fn text(&self) -> String {
        match self {
            Problem::Unreadable(what) => format!("format problem in {what}"),
            Problem::UnknownValue(field) => format!("unknown value in {field}"),
            Problem::NoLockOwner {
                code, named: true, ..
            } => format!(
                "Restart Manager could not tell which process holds a lock file (error {code}), so the process its \
                 name gives counts"
            ),
            Problem::NoLockOwner {
                image,
                code,
                named: false,
            } => format!(
                "Restart Manager could not tell which process holds a lock file (error {code}), so any running \
                 {image} counts"
            ),
            Problem::LivenessUnknown => {
                "a session's process could not be checked, so its liveness is unknown".to_owned()
            }
            Problem::Unlisted(code) => {
                format!(
                    "a folder it reads could not be listed ({})",
                    code_text(*code)
                )
            }
            Problem::Unread(code) => {
                format!("a file it reads could not be read ({})", code_text(*code))
            }
            Problem::Ended { session, why } => {
                format!(
                    "session {session} shows work but counts as ended: {}",
                    why.text()
                )
            }
            Problem::QueryFailed(sqlite::NO_SQLITE) => {
                "Windows' own SQLite (winsqlite3.dll) is missing, older than 3.22.0 or not usable, so its database is \
                 not read"
                    .to_owned()
            }
            Problem::QueryFailed(kind) => format!("a query of its database failed ({kind})"),
            Problem::NoWal => {
                "its database is open but has no -wal file, so it is not read".to_owned()
            }
        }
    }
}

fn source_name(id: SourceId) -> &'static str {
    match id {
        SourceId::ClaudeCode => "Claude Code",
        SourceId::Codex => "Codex CLI",
        SourceId::Copilot => "Copilot CLI",
        SourceId::OpenCode => "OpenCode",
        SourceId::VsCode => "VS Code chat",
        SourceId::Cursor => "Cursor",
        SourceId::Antigravity => "Antigravity",
    }
}

fn code_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "no error code".to_owned(),
        |code| format!("error {code}"),
    )
}

#[cfg(test)]
mod tests;
