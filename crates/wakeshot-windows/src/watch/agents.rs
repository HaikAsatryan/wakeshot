//! Agent sources and their liveness: asks each source what it wants, reads and feeds it, and turns its sessions into
//! one `Event::Sources` per source, each session paired with its liveness (architecture section 7: the adapter checks
//! liveness, never the parser).
//!
//! Driven by calls, with no thread or folder watch of its own: `p4-watch-thread` calls [`Agents::on_tick`] at start
//! and on the safety tick, [`Agents::on_change`] on a notice in a folder [`Agents::folders`] named, [`Agents::retry`]
//! [`RETRY_MS`] after a call that returned retries, and [`Agents::snapshots`] after each of them and after a process
//! exit.
//!
//! A file is opened only as a source asked: a `Folder` file whose name the want's `accept` passes (checked again at
//! the open by `read_whole`), a `Tail` path, and the file a `FindTail` found. `Names` are listed, never opened.
//!
//! An `AnyProcess` session (Codex, Copilot) lives while a process running its image holds its lock file open, as
//! Restart Manager tells ([`LockOwners`]); that process is then checked by pid and start like any other. If Restart
//! Manager fails, any live process running the image counts, which can outlive its session (architecture section 7).
//!
//! Owner: `p4-watch-agents`; the `AnyProcess` check `p4-codex-source`; fixes by `p4-live-claude`, `p4-live-codex`,
//! `p4-live-copilot`.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use wakeshot_core::event::Event;
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::sources::{
    self, AgentSource, Fed, Feed, FileId, FormatHealth, SessionFacts, SourceId, WantedFile,
};
use wakeshot_core::time::Now;

use crate::lockowner::{LockOwners, RestartManager};
use crate::log::{Level, Log};
use crate::process::{Liveness, ProcessEntry};
use crate::watch::read::{Finder, Tail, names, read_whole};

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

/// A folder to watch for change notices. `subtree`: its subfolders too, for a `Names` want of depth 1 or more.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WatchFolder {
    pub dir: PathBuf,
    pub subtree: bool,
}

/// A folder file to read and feed again, once, [`RETRY_MS`] after the call that returned it: its source answered
/// `Fed::RetryOnce`, or the read failed in a way that can pass (a writer that shares no read).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retry {
    slot: usize,
    dir: PathBuf,
    name: String,
}

impl Retry {
    pub fn file(&self) -> FileId {
        FileId(self.dir.join(&self.name))
    }
}

/// One [`Agents::snapshots`] call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshots {
    /// One `Event::Sources` per source whose sessions or their liveness changed since the last call, in source order;
    /// every source on the first call and after [`Agents::forget_posted`] (track note 14).
    pub events: Vec<Event>,
    /// The process of every live session of every source (for a lock file session, the holders that keep it alive),
    /// sorted, so the caller can wait on its exit; the session then shows as `Gone` at the next call.
    pub live: Vec<ProcessRef>,
}

/// Every agent source, with what was last read and fed to it and what was last returned for it.
pub struct Agents {
    slots: Vec<Slot>,
    log: Arc<dyn Log>,
    owners: Box<dyn LockOwners>,
}

impl Agents {
    pub fn new(sources: Vec<Box<dyn AgentSource + Send>>, log: Arc<dyn Log>) -> Self {
        Self::with_owners(sources, log, Box::new(RestartManager))
    }

    /// [`new`](Self::new) with lock file holders from `owners`, so tests give the answer.
    pub fn with_owners(
        sources: Vec<Box<dyn AgentSource + Send>>,
        log: Arc<dyn Log>,
        owners: Box<dyn LockOwners>,
    ) -> Self {
        Self {
            slots: sources.into_iter().map(Slot::new).collect(),
            log,
            owners,
        }
    }

    /// The folders to watch, sorted, each once. It changes only after a call that fed a source. Tails are not watched:
    /// each transcript line that matters comes with a status file write (architecture section 7).
    pub fn folders(&self) -> Vec<WatchFolder> {
        let mut subtree: BTreeMap<&Path, bool> = BTreeMap::new();
        for want in self.slots.iter().flat_map(|slot| &slot.wants) {
            let (dir, deep) = match want {
                WantedFile::Folder { dir, .. } => (dir, false),
                WantedFile::Names { dir, depth, .. } => (dir, *depth > 0),
                WantedFile::Tail(_) | WantedFile::FindTail { .. } => continue,
            };
            *subtree.entry(dir.as_path()).or_default() |= deep;
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
        let mut retries = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            // A lock file nobody held is asked about again, in case a notice of its change was lost.
            slot.owners.retain(|_, held| !held.is_empty());
            slot.run(&Scope::All, i, &mut retries);
        }
        retries
    }

    /// A change notice in `folder`, one of [`folders`](Self::folders). `names` are the changed names relative to it;
    /// `None` when the notice lost them (an overflow, or the folder opened again), and then the folder is read in full.
    /// Runs each source that watches `folder`: its files there as `names` say, its listings of `folder`, and every tail
    /// of that source.
    pub fn on_change(&mut self, folder: &Path, names: Option<&[PathBuf]>) -> Vec<Retry> {
        let scope = Scope::Change(folder, names);
        let mut retries = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.watches(folder) {
                slot.forget_owners(folder, names);
                slot.run(&scope, i, &mut retries);
            }
        }
        retries
    }

    /// Reads and feeds `retries` again, even when their bytes did not change. A `RetryOnce` answer now is final: the
    /// retries this returns are only for other files the same call read.
    pub fn retry(&mut self, retries: &[Retry]) -> Vec<Retry> {
        let mut again = Vec::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let mine: Vec<Retry> = retries.iter().filter(|r| r.slot == i).cloned().collect();
            if !mine.is_empty() {
                slot.run(&Scope::Retry(&mine), i, &mut again);
            }
        }
        again
    }

    /// Every source's sessions, each checked once through `liveness`, as events for the sources whose snapshot
    /// changed. Logs format and read problems, at most one line per source per minute of `now.mono_ms`.
    pub fn snapshots(&mut self, liveness: &dyn Liveness, now: Now) -> Snapshots {
        let mut out = Snapshots::default();
        let lock_owners = &*self.owners;
        for slot in &mut self.slots {
            let mut problems = slot.io_problems();
            let mut known = std::mem::take(&mut slot.owners);
            let mut named = BTreeSet::new();
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
                            let lock = LockFile {
                                image,
                                path: lock_file,
                            };
                            any_process(
                                liveness,
                                lock_owners,
                                &mut known,
                                &lock,
                                &mut problems,
                                &mut out.live,
                            )
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

/// `Yes` while a process running the image holds the lock file open; its holders go to `live`. The holders found
/// are kept in `known` until the lock file changes ([`Slot::forget_owners`], a listing that feeds it) or all of them
/// are gone, so Restart Manager is asked once per lock file, not on every call. A holder of another image (a virus
/// scanner, say) does not count. If Restart Manager fails, any live process running the image counts while the
/// source lists the lock file, and nothing goes to `live`.
fn any_process(
    liveness: &dyn Liveness,
    owners: &dyn LockOwners,
    known: &mut BTreeMap<PathBuf, Vec<ProcessRef>>,
    lock: &LockFile<'_>,
    problems: &mut BTreeSet<Problem>,
    live: &mut Vec<ProcessRef>,
) -> Alive {
    if let Some(held) = known.get(lock.path) {
        if held.is_empty() {
            return Alive::Gone;
        }
        let answers: Vec<(ProcessRef, Alive)> =
            held.iter().map(|&p| (p, liveness.check(p))).collect();
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
    let of_image = |e: &ProcessEntry| e.image.eq_ignore_ascii_case(lock.image);
    match owners.holders(lock.path) {
        Ok(holders) => {
            let held: Vec<ProcessRef> = if holders.is_empty() {
                holders
            } else {
                let Ok(list) = liveness.list() else {
                    problems.insert(Problem::LivenessUnknown);
                    return Alive::Unknown;
                };
                holders
                    .into_iter()
                    .filter(|h| {
                        list.iter()
                            .any(|e| e.pid == h.pid && e.start == h.start && of_image(e))
                    })
                    .collect()
            };
            let alive = if held.is_empty() {
                Alive::Gone
            } else {
                live.extend(&held);
                Alive::Yes
            };
            known.insert(lock.path.to_path_buf(), held);
            alive
        }
        Err(e) => {
            problems.insert(Problem::NoLockOwner {
                image: lock.image,
                code: e.code,
            });
            match liveness.list() {
                Ok(list) if list.iter().any(of_image) => Alive::Yes,
                Ok(_) => Alive::Gone,
                Err(_) => {
                    problems.insert(Problem::LivenessUnknown);
                    Alive::Unknown
                }
            }
        }
    }
}

/// What one call reads.
enum Scope<'a> {
    /// Every want in full: the tick and the first round.
    All,
    /// A notice in one folder, with the names it gave.
    Change(&'a Path, Option<&'a [PathBuf]>),
    /// Folder files read once more, fed even when unchanged.
    Retry(&'a [Retry]),
}

impl Scope<'_> {
    fn covers_folder(&self, dir: &Path) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(folder, _) => *folder == dir,
            Scope::Retry(retries) => retries.iter().any(|r| r.dir == dir),
        }
    }

    fn covers_listing(&self, dir: &Path) -> bool {
        match self {
            Scope::All => true,
            Scope::Change(folder, _) => *folder == dir,
            Scope::Retry(_) => false,
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
                    .filter(|r| r.dir == dir)
                    .map(|r| r.name.clone())
                    .collect(),
                true,
            )),
        }
    }
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
        WantedFile::Folder { .. } | WantedFile::Names { .. } => None,
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
    tails: BTreeMap<TailKey, TailState>,
    finder: Finder,
    /// The holders found for each lock file an `AnyProcess` session names; an empty list: nobody held it.
    owners: BTreeMap<PathBuf, Vec<ProcessRef>>,
    posted: Option<Vec<(SessionFacts, Alive)>>,
    logged: BTreeSet<Problem>,
    last_line_ms: Option<u64>,
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

#[derive(Debug, Default)]
struct TailState {
    tail: Option<Tail>,
    /// The last poll found the file, so its going is fed once as `Gone`.
    present: bool,
    unread: Option<Option<i32>>,
}

impl Slot {
    fn new(source: Box<dyn AgentSource + Send>) -> Self {
        let wants = source.wanted();
        let mut slot = Self {
            source,
            wants,
            folders: BTreeMap::new(),
            listings: BTreeMap::new(),
            tails: BTreeMap::new(),
            finder: Finder::new(),
            owners: BTreeMap::new(),
            posted: None,
            logged: BTreeSet::new(),
            last_line_ms: None,
        };
        slot.sync();
        slot
    }

    fn watches(&self, folder: &Path) -> bool {
        self.wants.iter().any(|w| match w {
            WantedFile::Folder { dir, .. } | WantedFile::Names { dir, .. } => dir == folder,
            WantedFile::Tail(_) | WantedFile::FindTail { .. } => false,
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

    /// Feeds one file and asks the source what it wants now (wants change only after a feed: the `AgentSource`
    /// promise). The state of every want that left goes at once, so a want that leaves and comes back within one call
    /// starts over: a resumed session whose leftover status file was fed `Gone` first gets its transcript from byte 0.
    fn feed(&mut self, path: PathBuf, data: Feed) -> Fed {
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
        self.tails.retain(|key, _| wants_tail(wants, key));
    }

    /// Drops the state of wants that left and adds fresh state for new folders and listings. Tail state is made when
    /// the tail is first polled.
    fn sync(&mut self) {
        self.prune();
        let mut folders: BTreeMap<&Path, Accept> = BTreeMap::new();
        let mut listings: BTreeMap<(&Path, u8), Accept> = BTreeMap::new();
        for want in &self.wants {
            match want {
                WantedFile::Folder { dir, accept } => {
                    folders.entry(dir.as_path()).or_insert(*accept);
                }
                WantedFile::Names { dir, depth, accept } => {
                    listings.entry((dir.as_path(), *depth)).or_insert(*accept);
                }
                WantedFile::Tail(_) | WantedFile::FindTail { .. } => {}
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
    }

    /// Folders and listings first, so a tail that a status file just named is polled in the same call.
    fn run(&mut self, scope: &Scope<'_>, slot: usize, retries: &mut Vec<Retry>) {
        // The trait promises wants change only after a feed; asking costs little and keeps a source that breaks the
        // promise from being read with stale wants.
        self.wants = self.source.wanted();
        let mut done = BTreeSet::new();
        for _ in 0..UNITS_PER_RUN {
            self.sync();
            let Some(unit) = self.next_unit(scope, &done) else {
                break;
            };
            match &unit {
                Unit::Folder(dir) => self.read_folder(dir, scope, slot, retries),
                Unit::Names(dir, depth) => self.list(dir, *depth),
                Unit::Tail(key) => self.poll_tail(key),
            }
            done.insert(unit);
        }
        self.sync();
    }

    fn next_unit(&self, scope: &Scope<'_>, done: &BTreeSet<Unit>) -> Option<Unit> {
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
            WantedFile::Tail(_) | WantedFile::FindTail { .. } => None,
        });
        let tails = self.wants.iter().filter_map(tail_key).map(Unit::Tail);
        reads.chain(tails).find(|unit| !done.contains(unit))
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
                    // The known files are read one by one instead (Claude parsing review m3): a listing error never
                    // reads as every session gone, and a status file that changes or goes still reaches its source.
                    // A new file waits until the folder lists again.
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
                            dir: dir.to_path_buf(),
                            name,
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
                            dir: dir.to_path_buf(),
                            name,
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
    /// lines from the start (orchestrator, 2026-11-03), since `Feed` has no restart of its own. A tail feed's
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
                let found = self.finder.find(root, *depth, suffix);
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
        let tails = self
            .tails
            .values()
            .filter_map(|t| t.unread.map(Problem::Unread));
        folders.chain(listings).chain(tails).collect()
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

/// What one log line can name: field names, fixed texts and error codes, never file content or a path (a path holds
/// the user's name; a `.key` name must never reach the log).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Problem {
    Unreadable(&'static str),
    UnknownValue(&'static str),
    NoLockOwner { image: &'static str, code: u32 },
    LivenessUnknown,
    Unlisted(Option<i32>),
    Unread(Option<i32>),
}

impl Problem {
    fn text(&self) -> String {
        match self {
            Problem::Unreadable(what) => format!("format problem in {what}"),
            Problem::UnknownValue(field) => format!("unknown value in {field}"),
            Problem::NoLockOwner { image, code } => format!(
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
        }
    }
}

fn source_name(id: SourceId) -> &'static str {
    match id {
        SourceId::ClaudeCode => "Claude Code",
        SourceId::Codex => "Codex CLI",
        SourceId::Copilot => "Copilot CLI",
    }
}

fn code_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "no error code".to_owned(),
        |code| format!("error {code}"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use wakeshot_core::process::FileTime;
    use wakeshot_core::sources::{Activity, SessionKey};
    use wakeshot_core::testing::{FakeClock, FakeSource};
    use wakeshot_core::time::Clock;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LUID, LocalFree};
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{
        AdjustTokenPrivileges, DACL_SECURITY_INFORMATION, ImpersonateSelf, LUID_AND_ATTRIBUTES,
        LookupPrivilegeValueW, PSECURITY_DESCRIPTOR, RevertToSelf, SE_BACKUP_NAME,
        SecurityImpersonation, SetFileSecurityW, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        TOKEN_PRIVILEGES_ATTRIBUTES,
    };
    use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
    use windows::core::{HSTRING, PCWSTR, w};

    use super::*;
    use crate::log::MemoryLog;
    use crate::process::FakeLiveness;
    use crate::watch::read::TempDir;

    const START: Now = Now {
        wall_ms: 1_793_750_400_000,
        mono_ms: 5_000,
    };

    type Hook = Box<dyn FnMut(&mut FakeSource, &FileId) + Send>;

    struct Inner {
        fake: FakeSource,
        after_feed: Option<Hook>,
        asked: usize,
    }

    /// A `FakeSource` the test still reaches after `Agents` took it, with a hook that runs after each feed, the way a
    /// parser's wants change once a status file names its transcript.
    #[derive(Clone)]
    struct Shared(Arc<Mutex<Inner>>);

    impl Shared {
        fn new(id: SourceId) -> Self {
            Self(Arc::new(Mutex::new(Inner {
                fake: FakeSource::new(id),
                after_feed: None,
                asked: 0,
            })))
        }

        fn inner(&self) -> MutexGuard<'_, Inner> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn set(&self, f: impl FnOnce(&mut FakeSource)) {
            f(&mut self.inner().fake);
        }

        /// The feeds since the last call.
        fn fed(&self) -> Vec<(FileId, Feed)> {
            std::mem::take(&mut self.inner().fake.fed)
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
            let mut inner = self.inner();
            inner.asked += 1;
            inner.fake.wanted()
        }

        fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
            let mut inner = self.inner();
            let Inner {
                fake, after_feed, ..
            } = &mut *inner;
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

    fn agents(sources: &[&Shared]) -> (Agents, Arc<MemoryLog>) {
        let log = Arc::new(MemoryLog::default());
        let boxed = sources.iter().map(|s| s.boxed()).collect();
        (Agents::new(boxed, log.clone()), log)
    }

    fn lines(log: &MemoryLog) -> Vec<String> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(level, area, text)| {
                assert_eq!((*level, *area), (Level::Warn, AREA));
                text.clone()
            })
            .collect()
    }

    /// The Claude status file rule: `<digits>.json` and nothing else.
    fn status_name(name: &str) -> bool {
        name.strip_suffix(".json")
            .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
    }

    fn lock_name(name: &str) -> bool {
        name.ends_with(".lock")
    }

    fn folder(dir: &Path) -> WantedFile {
        WantedFile::Folder {
            dir: dir.to_path_buf(),
            accept: status_name,
        }
    }

    fn find_tail(root: &Path, suffix: &str) -> WantedFile {
        WantedFile::FindTail {
            root: root.to_path_buf(),
            depth: 1,
            name_suffix: suffix.to_owned(),
        }
    }

    fn whole(path: &Path, bytes: &[u8]) -> (FileId, Feed) {
        (FileId(path.to_path_buf()), Feed::Whole(bytes.to_vec()))
    }

    fn appended(path: &Path, bytes: &[u8]) -> (FileId, Feed) {
        (FileId(path.to_path_buf()), Feed::Appended(bytes.to_vec()))
    }

    fn gone(path: &Path) -> (FileId, Feed) {
        (FileId(path.to_path_buf()), Feed::Gone)
    }

    fn present(path: &Path) -> (FileId, Feed) {
        (FileId(path.to_path_buf()), Feed::Present)
    }

    fn process(pid: u32) -> ProcessRef {
        ProcessRef {
            pid,
            start: FileTime(134_381_211_391_959_336 + u64::from(pid)),
        }
    }

    fn session(pid: u32) -> SessionFacts {
        let p = process(pid);
        SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: pid.to_string(),
            },
            liveness: sources::Liveness::Process {
                pid: p.pid,
                start: p.start,
            },
            activity: Activity::Working,
            timers: vec![],
            open_tasks: 0,
            label: "Claude Code",
            format: FormatHealth::Ok,
        }
    }

    fn append(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    #[test]
    fn folders_lists_each_watched_folder_once_with_subtree_for_depth() {
        let root = TempDir::new("agents-folders");
        let [d1, d2, d3] = ["d1", "d2", "d3"].map(|d| root.path().join(d));
        let a = Shared::new(SourceId::ClaudeCode);
        a.set(|f| {
            f.wanted = vec![
                folder(&d1),
                WantedFile::Names {
                    dir: d2.clone(),
                    depth: 1,
                    accept: lock_name,
                },
                WantedFile::Tail(root.path().join("t.jsonl")),
                find_tail(&root.path().join("projects"), "s.jsonl"),
            ];
        });
        let b = Shared::new(SourceId::Codex);
        b.set(|f| {
            f.wanted = vec![
                folder(&d1),
                WantedFile::Names {
                    dir: d1.clone(),
                    depth: 0,
                    accept: lock_name,
                },
                WantedFile::Names {
                    dir: d3.clone(),
                    depth: 0,
                    accept: lock_name,
                },
            ];
        });
        let (agents, _) = agents(&[&a, &b]);
        let watch = |dir: &Path, subtree| WatchFolder {
            dir: dir.to_path_buf(),
            subtree,
        };
        assert_eq!(
            agents.folders(),
            [watch(&d1, false), watch(&d2, true), watch(&d3, false)]
        );
    }

    #[test]
    fn tick_feeds_each_want_kind_and_never_a_refused_name() {
        let root = TempDir::new("agents-kinds");
        let sessions = root.path().join("sessions");
        let status = root.write("sessions/20636.json", b"{\"pid\":20636}");
        root.write("sessions/20636.685bf0d758162e18.key", b"secret");
        root.write("sessions/notes.txt", b"other");
        let locks = root.path().join("locks");
        let lock = root.write("locks/t1/a.lock", b"");
        let tail = root.write("t.jsonl", b"{\"a\":1}\n{\"b\"");
        let found = root.write("projects/C--work/s1.jsonl", b"{\"s\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| {
            f.wanted = vec![
                folder(&sessions),
                WantedFile::Names {
                    dir: locks.clone(),
                    depth: 1,
                    accept: lock_name,
                },
                WantedFile::Tail(tail.clone()),
                find_tail(&root.path().join("projects"), "s1.jsonl"),
            ];
        });
        let (mut agents, _) = agents(&[&source]);

        assert!(agents.on_tick().is_empty());
        assert_eq!(
            source.fed(),
            [
                whole(&status, b"{\"pid\":20636}"),
                present(&lock),
                appended(&tail, b"{\"a\":1}\n"),
                appended(&found, b"{\"s\":1}\n"),
            ]
        );
    }

    #[test]
    fn folder_feeds_a_file_again_only_when_it_changed_and_gone_when_deleted() {
        let root = TempDir::new("agents-folder");
        let one = root.write("1.json", b"{\"v\":1}");
        let two = root.write("2.json", b"{\"v\":2}");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(root.path())]);
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(
            source.fed(),
            [whole(&one, b"{\"v\":1}"), whole(&two, b"{\"v\":2}")]
        );

        agents.on_tick();
        assert!(source.fed().is_empty(), "unchanged files are not fed again");

        fs::write(&one, b"{\"v\":11}").unwrap();
        fs::remove_file(&two).unwrap();
        agents.on_tick();
        assert_eq!(source.fed(), [gone(&two), whole(&one, b"{\"v\":11}")]);

        fs::remove_file(&one).unwrap();
        agents.on_change(root.path(), Some(&[PathBuf::from("1.json")]));
        assert_eq!(source.fed(), [gone(&one)]);
        agents.on_change(root.path(), Some(&[PathBuf::from("1.json")]));
        assert!(source.fed().is_empty(), "a file is gone once");
    }

    #[test]
    fn change_reads_only_the_named_files_and_runs_only_that_folders_sources() {
        let root = TempDir::new("agents-change");
        let da = root.path().join("a");
        let one = root.write("a/1.json", b"1");
        let two = root.write("a/2.json", b"2");
        let tail_a = root.write("a.jsonl", b"");
        let db = root.path().join("b");
        let tail_b = root.write("b.jsonl", b"");
        let a = Shared::new(SourceId::ClaudeCode);
        a.set(|f| f.wanted = vec![folder(&da), WantedFile::Tail(tail_a.clone())]);
        let b = Shared::new(SourceId::Codex);
        b.set(|f| f.wanted = vec![folder(&db), WantedFile::Tail(tail_b.clone())]);
        let (mut agents, _) = agents(&[&a, &b]);
        agents.on_tick();
        a.fed();

        fs::write(&one, b"11").unwrap();
        fs::write(&two, b"22").unwrap();
        append(&tail_a, b"x\n");
        append(&tail_b, b"y\n");
        let names = [
            PathBuf::from("1.json"),
            PathBuf::from("1.json"),
            PathBuf::from(r"sub\2.json"),
            PathBuf::from(r"..\a\2.json"),
        ];
        agents.on_change(&da, Some(&names));
        assert_eq!(
            a.fed(),
            [whole(&one, b"11"), appended(&tail_a, b"x\n")],
            "only the plain name is read, once; the source's tail is polled"
        );
        assert!(
            b.fed().is_empty(),
            "a source that does not watch the folder is not run"
        );

        agents.on_change(&da, None);
        assert_eq!(
            a.fed(),
            [whole(&two, b"22")],
            "no names: the folder is read in full"
        );
    }

    #[test]
    fn retry_reads_again_once_and_feeds_even_unchanged_bytes() {
        let root = TempDir::new("agents-retry");
        let file = root.write("1.json", b"{\"half\":");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| {
            f.wanted = vec![folder(root.path())];
            f.retry = vec![FileId(file.clone())];
        });
        let (mut agents, _) = agents(&[&source]);

        let retries = agents.on_tick();
        assert_eq!(
            retries.iter().map(Retry::file).collect::<Vec<_>>(),
            [FileId(file.clone())]
        );
        assert_eq!(source.fed(), [whole(&file, b"{\"half\":")]);

        assert!(agents.retry(&retries).is_empty());
        assert_eq!(source.fed(), [whole(&file, b"{\"half\":")]);
        agents.on_tick();
        assert!(source.fed().is_empty(), "taken bytes are not fed again");
    }

    #[test]
    fn a_retry_answered_with_retry_once_again_is_not_retried_again() {
        let root = TempDir::new("agents-retry-twice");
        let file = root.write("1.json", b"{");
        let source = Shared::new(SourceId::ClaudeCode);
        let id = FileId(file.clone());
        source.set(|f| {
            f.wanted = vec![folder(root.path())];
            f.retry = vec![id.clone(), id.clone()];
        });
        let (mut agents, _) = agents(&[&source]);
        let retries = agents.on_tick();
        assert_eq!(retries.len(), 1);
        assert!(agents.retry(&retries).is_empty(), "retried once only");
        assert_eq!(source.fed().len(), 2);
        agents.on_tick();
        assert!(source.fed().is_empty(), "the source had its retry");

        // A retry that never came: the bytes were not taken, so the next read feeds them again.
        fs::write(&file, b"{{").unwrap();
        source.set(|f| f.retry = vec![id.clone()]);
        assert_eq!(agents.on_tick().len(), 1);
        assert_eq!(source.fed(), [whole(&file, b"{{")]);
        assert!(agents.on_tick().is_empty());
        assert_eq!(source.fed(), [whole(&file, b"{{")]);
        agents.on_tick();
        assert!(source.fed().is_empty());
    }

    #[test]
    fn a_read_that_fails_is_retried_logged_and_never_fed_as_gone() {
        let root = TempDir::new("agents-unread");
        let file = root.write("1.json", b"{}");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(root.path())]);
        let (mut agents, log) = agents(&[&source]);
        let live = FakeLiveness::default();

        let writer = OpenOptions::new()
            .write(true)
            .share_mode(0)
            .open(&file)
            .unwrap();
        let retries = agents.on_tick();
        assert_eq!(retries.len(), 1);
        assert!(source.fed().is_empty());
        agents.snapshots(&live, START);
        assert_eq!(
            lines(&log),
            ["Claude Code: a file it reads could not be read (error 32)"]
        );

        drop(writer);
        assert!(agents.retry(&retries).is_empty());
        assert_eq!(source.fed(), [whole(&file, b"{}")]);

        let _writer = OpenOptions::new()
            .write(true)
            .share_mode(0)
            .open(&file)
            .unwrap();
        agents.on_tick();
        assert!(
            source.fed().is_empty(),
            "a file that cannot be read is not gone"
        );
    }

    #[test]
    fn names_feeds_a_file_appearing_and_going() {
        let root = TempDir::new("agents-names");
        let a = root.write("a.lock", b"");
        root.write("x.json", b"{}");
        let source = Shared::new(SourceId::Codex);
        source.set(|f| {
            f.wanted = vec![WantedFile::Names {
                dir: root.path().to_path_buf(),
                depth: 0,
                accept: lock_name,
            }];
        });
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed(), [present(&a)]);
        agents.on_tick();
        assert!(
            source.fed().is_empty(),
            "a name still there is not fed again"
        );

        let b = root.write("b.lock", b"");
        fs::remove_file(&a).unwrap();
        agents.on_change(root.path(), Some(&[PathBuf::from("a.lock")]));
        assert_eq!(source.fed(), [gone(&a), present(&b)]);
    }

    /// Denies listing a folder to everyone, on the folder only, so the files in it stay readable. Dropped, the folder
    /// lists again.
    struct NoListing(PathBuf);

    impl NoListing {
        fn on(dir: &Path) -> Self {
            set_dacl(dir, w!("D:(D;;0x1;;;WD)(A;OICI;FA;;;WD)")).unwrap();
            Self(dir.to_path_buf())
        }
    }

    impl Drop for NoListing {
        fn drop(&mut self) {
            // No assert: a panic here, during a failed test's unwind, would abort the run.
            let _ = set_dacl(&self.0, w!("D:(A;OICI;FA;;;WD)"));
        }
    }

    /// This test thread runs without `SeBackupPrivilege` until dropped. An elevated Git Bash (the MSYS2 runtime) starts
    /// every child with that privilege enabled, and a folder opened for listing with backup semantics, as `read_dir`
    /// opens it, then lists past a deny ACE; the tray never runs so. So the thread takes a copy of the process token
    /// with the privilege off, and the deny holds in any shell (research, "A deny ACE and the backup privilege").
    struct NoBackup;

    impl NoBackup {
        fn on() -> Self {
            // SAFETY: gives this thread its own copy of the process token; `RevertToSelf` in the drop ends it.
            unsafe { ImpersonateSelf(SecurityImpersonation) }.unwrap();
            let guard = Self;
            let mut token = HANDLE::default();
            // SAFETY: the current-thread pseudo handle needs no close, and `token` is a live out pointer.
            unsafe {
                OpenThreadToken(
                    GetCurrentThread(),
                    TOKEN_ADJUST_PRIVILEGES,
                    true,
                    &mut token,
                )
            }
            .unwrap();
            let mut luid = LUID::default();
            // SAFETY: a null system name means this PC; the name is a static string and `luid` a live out pointer.
            unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_BACKUP_NAME, &mut luid) }.unwrap();
            let off = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: TOKEN_PRIVILEGES_ATTRIBUTES(0),
                }],
            };
            // SAFETY: the token is open with TOKEN_ADJUST_PRIVILEGES and `off` outlives the call; no previous state is
            // asked for. A token without the privilege (an unelevated shell) answers success too.
            let adjusted =
                unsafe { AdjustTokenPrivileges(token, false, Some(&off), 0, None, None) };
            // SAFETY: a handle the call above opened, closed once.
            unsafe { CloseHandle(token) }.unwrap();
            adjusted.unwrap();
            guard
        }
    }

    impl Drop for NoBackup {
        fn drop(&mut self) {
            // No assert: a panic here, during a failed test's unwind, would abort the run.
            // SAFETY: ends this thread's impersonation; nothing else on the thread relies on it.
            let _ = unsafe { RevertToSelf() };
        }
    }

    /// `SetFileSecurityW` changes the folder only: the files in it keep the access they had.
    fn set_dacl(path: &Path, sddl: PCWSTR) -> windows::core::Result<()> {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        // SAFETY: a constant string; `sd` is freed below with LocalFree, as the call documents.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl,
                SDDL_REVISION_1,
                &mut sd,
                None,
            )
        }?;
        let name = HSTRING::from(path);
        // SAFETY: the name and the descriptor live through the call.
        let set = unsafe { SetFileSecurityW(&name, DACL_SECURITY_INFORMATION, sd) };
        // SAFETY: allocated by the conversion above, freed once.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sd.0)));
        }
        set.ok()
    }

    /// Claude parsing review m3: with its listing denied, a folder's known files are still read, so a status file that
    /// changes or goes reaches its source; a new one waits until the folder lists again.
    #[test]
    fn a_folder_that_cannot_be_listed_reads_its_known_files() {
        let root = TempDir::new("agents-no-list");
        let dir = root.path().join("sessions");
        let one = root.write("sessions/1.json", b"{\"v\":1}");
        let two = root.write("sessions/2.json", b"{\"v\":2}");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(&dir)]);
        let (mut agents, log) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed().len(), 2);

        let _no_backup = NoBackup::on();
        let denied = NoListing::on(&dir);
        let listing = names(&dir, 0, |_| true).map_err(|e| e.raw_os_error());
        assert_eq!(listing, Err(Some(5)), "the listing is denied");
        assert_eq!(
            fs::read(&one).unwrap(),
            b"{\"v\":1}",
            "the files stay readable"
        );
        fs::write(&one, b"{\"v\":11}").unwrap();
        fs::remove_file(&two).unwrap();
        let three = root.write("sessions/3.json", b"{\"v\":3}");
        agents.on_tick();
        assert_eq!(source.fed(), [whole(&one, b"{\"v\":11}"), gone(&two)]);
        agents.snapshots(&FakeLiveness::default(), START);
        assert_eq!(
            lines(&log),
            ["Claude Code: a folder it reads could not be listed (error 5)"]
        );

        drop(denied);
        agents.on_tick();
        assert_eq!(source.fed(), [whole(&three, b"{\"v\":3}")]);
    }

    #[test]
    fn a_folder_replaced_by_a_file_feeds_its_known_files_gone() {
        let root = TempDir::new("agents-unlisted");
        let dir = root.path().join("sessions");
        let file = root.write("sessions/1.json", b"{}");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(&dir)]);
        let (mut agents, log) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed(), [whole(&file, b"{}")]);

        // A file where the folder was: the listing fails, and the known file is not found.
        fs::remove_dir_all(&dir).unwrap();
        fs::write(&dir, b"").unwrap();
        agents.on_tick();
        assert_eq!(source.fed(), [gone(&file)]);
        agents.snapshots(&FakeLiveness::default(), START);
        let logged = lines(&log);
        assert_eq!(logged.len(), 1);
        assert!(
            logged[0].starts_with("Claude Code: a folder it reads could not be listed (error "),
            "{logged:?}"
        );

        fs::remove_file(&dir).unwrap();
        agents.on_tick();
        assert!(source.fed().is_empty(), "a file is gone once");
    }

    #[test]
    fn tail_feeds_only_new_complete_lines_on_ticks_and_changes() {
        let root = TempDir::new("agents-tail");
        let dir = root.path().join("sessions");
        fs::create_dir_all(&dir).unwrap();
        let tail = root.write("t.jsonl", b"{\"a\":1}\n{\"b\":2}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(&dir), WantedFile::Tail(tail.clone())]);
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n{\"b\":2}\n")]);

        append(&tail, b"{\"c\":3}\n{\"d\"");
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&tail, b"{\"c\":3}\n")]);
        agents.on_tick();
        assert!(source.fed().is_empty());

        append(&tail, b":4}\n");
        agents.on_change(&dir, Some(&[]));
        assert_eq!(source.fed(), [appended(&tail, b"{\"d\":4}\n")]);
    }

    #[test]
    fn tail_restart_feeds_gone_then_the_lines_from_the_start() {
        let root = TempDir::new("agents-restart");
        let tail = root.write("t.jsonl", b"{\"a\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n")]);

        let next = root.write("t.jsonl.tmp", b"{\"n\":1}\n{\"n\":2}\n");
        fs::rename(&next, &tail).unwrap();
        agents.on_tick();
        assert_eq!(
            source.fed(),
            [gone(&tail), appended(&tail, b"{\"n\":1}\n{\"n\":2}\n")]
        );

        // Gone, then back: fed as gone once, then from the start without a second `Gone`.
        fs::remove_file(&tail).unwrap();
        agents.on_tick();
        assert_eq!(source.fed(), [gone(&tail)]);
        agents.on_tick();
        assert!(source.fed().is_empty());
        fs::write(&tail, b"{\"z\":1}\n").unwrap();
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&tail, b"{\"z\":1}\n")]);
    }

    #[test]
    fn a_tail_that_leaves_the_wants_starts_from_the_start_when_it_comes_back() {
        let root = TempDir::new("agents-leave");
        let tail = root.write("t.jsonl", b"{\"a\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed().len(), 1);

        source.set(|f| f.wanted.clear());
        agents.on_tick();
        assert!(source.fed().is_empty());
        source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n")]);
    }

    #[test]
    fn find_tail_keeps_its_file_and_feeds_gone_when_it_goes() {
        let root = TempDir::new("agents-find");
        let projects = root.path().join("projects");
        let first = root.write("projects/p/s.jsonl", b"{\"a\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![find_tail(&projects, "s.jsonl")]);
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&first, b"{\"a\":1}\n")]);

        // A second match does not take over while the first is still there.
        let second = root.write("projects/q/s.jsonl", b"{\"z\":1}\n");
        append(&first, b"{\"b\":2}\n");
        agents.on_tick();
        assert_eq!(source.fed(), [appended(&first, b"{\"b\":2}\n")]);

        fs::remove_file(&first).unwrap();
        agents.on_tick();
        assert_eq!(
            source.fed(),
            [gone(&first), appended(&second, b"{\"z\":1}\n")]
        );
    }

    #[test]
    fn wanted_is_asked_again_after_each_feed() {
        let root = TempDir::new("agents-asked");
        let sessions = root.path().join("sessions");
        let status = root.write("sessions/1.json", b"{\"sessionId\":\"s1\"}");
        let projects = root.path().join("projects");
        let transcript = root.write("projects/p/s1.jsonl", b"{\"t\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(&sessions)]);
        let hook_projects = projects.clone();
        source.inner().after_feed = Some(Box::new(move |fake, file| {
            if file.0.extension().is_some_and(|e| e == "json") {
                fake.wanted.push(find_tail(&hook_projects, "s1.jsonl"));
            }
        }));
        let (mut agents, _) = agents(&[&source]);
        let asked = source.inner().asked;

        agents.on_change(&sessions, Some(&[PathBuf::from("1.json")]));
        assert_eq!(
            source.fed(),
            [
                whole(&status, b"{\"sessionId\":\"s1\"}"),
                appended(&transcript, b"{\"t\":1}\n"),
            ],
            "the tail the status file named is polled in the same call"
        );
        // Once at the start of the call, then once after each of the two feeds.
        assert_eq!(source.inner().asked - asked, 3);
    }

    /// A resume: the crashed process's leftover status file is fed `Gone` before the new process's file names the same
    /// transcript. The source dropped the transcript in between, so it must get it again from byte 0.
    #[test]
    fn a_tail_that_leaves_and_comes_back_within_one_call_starts_from_the_start() {
        let root = TempDir::new("agents-resume");
        let sessions = root.path().join("sessions");
        let old = root.write("sessions/100.json", b"{\"sessionId\":\"s1\"}");
        let projects = root.path().join("projects");
        let transcript = root.write("projects/p/s1.jsonl", b"{\"t\":1}\n");
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.wanted = vec![folder(&sessions)]);
        let hook_projects = projects.clone();
        source.inner().after_feed = Some(Box::new(move |fake, file| {
            let is_status = file.0.extension().is_some_and(|e| e == "json");
            let gone = matches!(fake.fed.last(), Some((_, Feed::Gone)));
            if is_status && gone {
                fake.wanted
                    .retain(|w| !matches!(w, WantedFile::FindTail { .. }));
            } else if is_status && fake.wanted.len() == 1 {
                fake.wanted.push(find_tail(&hook_projects, "s1.jsonl"));
            }
        }));
        let (mut agents, _) = agents(&[&source]);
        agents.on_tick();
        assert_eq!(
            source.fed(),
            [
                whole(&old, b"{\"sessionId\":\"s1\"}"),
                appended(&transcript, b"{\"t\":1}\n"),
            ]
        );

        append(&transcript, b"{\"t\":2}\n");
        fs::remove_file(&old).unwrap();
        let new = root.write("sessions/200.json", b"{\"sessionId\":\"s1\"}");
        agents.on_change(&sessions, None);
        assert_eq!(
            source.fed(),
            [
                gone(&old),
                whole(&new, b"{\"sessionId\":\"s1\"}"),
                appended(&transcript, b"{\"t\":1}\n{\"t\":2}\n"),
            ]
        );
    }

    #[test]
    fn process_liveness_is_yes_gone_or_unknown_and_live_ones_are_returned() {
        let source = Shared::new(SourceId::ClaudeCode);
        source.set(|f| f.sessions = vec![session(1), session(2), session(3)]);
        let (mut agents, log) = agents(&[&source]);
        let live = FakeLiveness::default();
        live.alive
            .lock()
            .unwrap()
            .extend([(process(1), Alive::Yes), (process(2), Alive::Unknown)]);

        let snapshots = agents.snapshots(&live, START);
        assert_eq!(
            snapshots,
            Snapshots {
                events: vec![Event::Sources {
                    source: SourceId::ClaudeCode,
                    sessions: vec![
                        (session(1), Alive::Yes),
                        (session(2), Alive::Unknown),
                        (session(3), Alive::Gone),
                    ],
                }],
                live: vec![process(1)],
            }
        );
        assert_eq!(
            *live.checked.lock().unwrap(),
            [process(1), process(2), process(3)]
        );
        assert_eq!(
            lines(&log),
            ["Claude Code: a session's process could not be checked, so its liveness is unknown"]
        );
    }

    /// Lock file holders the test sets, one answer for every path, and every path asked about.
    #[derive(Clone, Default)]
    struct Owners(Arc<Mutex<OwnersInner>>);

    #[derive(Default)]
    struct OwnersInner {
        answer: Option<Result<Vec<ProcessRef>, crate::WinError>>,
        asked: Vec<PathBuf>,
    }

    impl Owners {
        fn inner(&self) -> MutexGuard<'_, OwnersInner> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn answer(&self, answer: Result<Vec<ProcessRef>, crate::WinError>) {
            self.inner().answer = Some(answer);
        }

        /// How many times it was asked since the last call.
        fn asked(&self) -> usize {
            std::mem::take(&mut self.inner().asked).len()
        }
    }

    impl LockOwners for Owners {
        fn holders(&self, path: &Path) -> Result<Vec<ProcessRef>, crate::WinError> {
            let mut inner = self.inner();
            inner.asked.push(path.to_path_buf());
            inner.answer.clone().unwrap_or(Ok(Vec::new()))
        }
    }

    const RM_FAILED: crate::WinError = crate::WinError {
        call: "RmGetList",
        code: 5,
    };

    fn codex_session(lock_file: &Path) -> SessionFacts {
        SessionFacts {
            key: SessionKey {
                source: SourceId::Codex,
                id: "0199aaaa-0000-7000-8000-000000000001".to_owned(),
            },
            liveness: sources::Liveness::AnyProcess {
                image: "codex.exe",
                lock_file: lock_file.to_path_buf(),
            },
            activity: Activity::Working,
            timers: vec![],
            open_tasks: 0,
            label: "Codex",
            format: FormatHealth::Ok,
        }
    }

    fn entry(process: ProcessRef, image: &str) -> ProcessEntry {
        ProcessEntry {
            pid: process.pid,
            start: process.start,
            image: image.to_owned(),
        }
    }

    /// `Agents` over one Codex source whose one session names `lock_file`, with fake holders.
    fn lock_agents(lock_file: &Path) -> (Agents, Arc<MemoryLog>, Shared, Owners) {
        let source = Shared::new(SourceId::Codex);
        let session = codex_session(lock_file);
        source.set(|f| f.sessions = vec![session]);
        let owners = Owners::default();
        let log = Arc::new(MemoryLog::default());
        let agents =
            Agents::with_owners(vec![source.boxed()], log.clone(), Box::new(owners.clone()));
        (agents, log, source, owners)
    }

    fn only_alive(snapshots: &Snapshots) -> Alive {
        match snapshots.events.as_slice() {
            [Event::Sources { sessions, .. }] => {
                assert_eq!(sessions.len(), 1);
                sessions[0].1
            }
            other => panic!("one event with one session: {other:?}"),
        }
    }

    #[test]
    fn a_lock_file_session_lives_while_a_holder_running_its_image_lives() {
        let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
        let (mut agents, log, _source, owners) = lock_agents(&lock);
        let live = FakeLiveness::default();
        owners.answer(Ok(vec![process(7)]));
        live.processes
            .lock()
            .unwrap()
            .push(entry(process(7), "Codex.EXE"));
        live.alive.lock().unwrap().push((process(7), Alive::Yes));

        let first = agents.snapshots(&live, START);
        assert_eq!(
            first,
            Snapshots {
                events: vec![Event::Sources {
                    source: SourceId::Codex,
                    sessions: vec![(codex_session(&lock), Alive::Yes)],
                }],
                live: vec![process(7)],
            },
            "the holder is waited on like any session process"
        );
        assert_eq!(owners.asked(), 1);
        assert!(lines(&log).is_empty());

        // Known now: only its pid and start are checked.
        let again = agents.snapshots(&live, START);
        assert_eq!((again.events.len(), again.live), (0, vec![process(7)]));
        assert_eq!(owners.asked(), 0, "Restart Manager is not asked again");
        assert_eq!(*live.checked.lock().unwrap(), [process(7)]);

        // The holder ended: asked again; nobody holds it now.
        live.alive.lock().unwrap().clear();
        owners.answer(Ok(vec![]));
        let ended = agents.snapshots(&live, START);
        assert_eq!(only_alive(&ended), Alive::Gone);
        assert!(ended.live.is_empty());
        assert_eq!(owners.asked(), 1);

        // "Nobody" is kept too, until the tick asks again.
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 0);
        agents.on_tick();
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 1);
    }

    #[test]
    fn a_holder_of_another_image_or_start_does_not_count() {
        let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
        let (mut agents, _, _source, owners) = lock_agents(&lock);
        let live = FakeLiveness::default();
        let scanner = process(8);
        let reused = ProcessRef {
            pid: 9,
            start: FileTime(1),
        };
        owners.answer(Ok(vec![scanner, process(9), process(7)]));
        live.processes.lock().unwrap().extend([
            entry(scanner, "MsMpEng.exe"),
            entry(reused, "codex.exe"),
            entry(process(7), "codex.exe"),
        ]);
        live.alive.lock().unwrap().push((process(7), Alive::Yes));

        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Yes);
        assert_eq!(snapshots.live, [process(7)]);

        // Only the scanner holds it after the codex process ended: gone.
        live.alive.lock().unwrap().clear();
        owners.answer(Ok(vec![scanner]));
        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Gone);
        assert!(snapshots.live.is_empty());
    }

    #[test]
    fn a_change_notice_or_a_listing_of_the_lock_file_asks_again() {
        let dir = TempDir::new("agents-lock-owners");
        let lock = dir.write("t1.lock", b"");
        let (mut agents, _, source, owners) = lock_agents(&lock);
        source.set(|f| {
            f.wanted = vec![WantedFile::Names {
                dir: dir.path().to_path_buf(),
                depth: 0,
                accept: lock_name,
            }];
        });
        let live = FakeLiveness::default();
        owners.answer(Ok(vec![process(7)]));
        live.processes
            .lock()
            .unwrap()
            .push(entry(process(7), "codex.exe"));
        live.alive.lock().unwrap().push((process(7), Alive::Yes));
        agents.on_tick();
        assert_eq!(source.fed(), [present(&lock)]);
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 1);

        // A tick that lists the same names keeps what is known; so does a notice about another file.
        agents.on_tick();
        agents.on_change(dir.path(), Some(&[PathBuf::from("t2.lock")]));
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 0);

        // A notice that names it, or one that lost its names: the file may have been made again.
        agents.on_change(dir.path(), Some(&[PathBuf::from("t1.lock")]));
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 1);
        agents.on_change(dir.path(), None);
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 1);

        // A listing that feeds it gone (the fake source still lists the session).
        fs::remove_file(&lock).unwrap();
        agents.on_tick();
        assert_eq!(source.fed(), [gone(&lock)]);
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 1);
    }

    #[test]
    fn a_restart_manager_error_counts_any_live_process_running_the_image() {
        let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
        let (mut agents, log, _source, owners) = lock_agents(&lock);
        let live = FakeLiveness::default();
        owners.answer(Err(RM_FAILED));
        live.processes
            .lock()
            .unwrap()
            .push(entry(process(9), "codex.exe"));

        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Yes);
        assert!(
            snapshots.live.is_empty(),
            "no one process is this session's"
        );
        assert_eq!(
            lines(&log),
            [
                "Codex CLI: Restart Manager could not tell which process holds a lock file (error 5), so any running \
                 codex.exe counts"
            ]
        );

        // Nothing is kept from a failed answer.
        live.processes
            .lock()
            .unwrap()
            .push(entry(process(10), "other.exe"));
        live.processes.lock().unwrap().remove(0);
        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Gone);
        assert_eq!(owners.asked(), 2);

        *live.list_error.lock().unwrap() = Some(RM_FAILED);
        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Unknown);
    }

    #[test]
    fn a_failed_process_list_or_holder_check_is_unknown() {
        let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
        let (mut agents, log, _source, owners) = lock_agents(&lock);
        let live = FakeLiveness::default();
        owners.answer(Ok(vec![process(7)]));
        *live.list_error.lock().unwrap() = Some(RM_FAILED);

        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Unknown);
        assert!(snapshots.live.is_empty());
        assert_eq!(
            lines(&log),
            ["Codex CLI: a session's process could not be checked, so its liveness is unknown"]
        );
        agents.snapshots(&live, START);
        assert_eq!(owners.asked(), 2, "an unknown answer is not kept");

        // Known, then its check fails: unknown, and the holder stays known.
        *live.list_error.lock().unwrap() = None;
        live.processes
            .lock()
            .unwrap()
            .push(entry(process(7), "codex.exe"));
        live.alive.lock().unwrap().push((process(7), Alive::Yes));
        assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Yes);
        live.alive.lock().unwrap()[0].1 = Alive::Unknown;
        let snapshots = agents.snapshots(&live, START);
        assert_eq!(only_alive(&snapshots), Alive::Unknown);
        assert!(snapshots.live.is_empty());
        live.alive.lock().unwrap()[0].1 = Alive::Yes;
        assert_eq!(agents.snapshots(&live, START).events.len(), 1);
        assert_eq!(owners.asked(), 1, "asked once in the three calls");
    }

    /// The Codex source on a made-up Codex home, read through the real listing, finder, tail and Restart Manager; this
    /// process holds the lock file the way Codex does and passes for `codex.exe` in the fake process list.
    #[test]
    fn codex_from_a_held_lock_file_and_its_rollout() {
        use wakeshot_core::sources::codex::CodexSource;

        const ID: &str = "0199aaaa-0000-7000-8000-000000000001";
        let home = TempDir::new("agents-codex");
        let lock = home.write(&format!(r"thread-writer-locks\{ID}.lock"), b"");
        home.write(r"thread-writer-locks\.coordination.lock", b"");
        let held = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        held.try_lock().unwrap();
        let start = concat!(
            r#"{"timestamp":"2026-11-06T10:00:00.000Z","ordinal":0,"type":"session_meta","payload":{"id":"0199aaaa-0000-7000-8000-000000000001"}}"#,
            "\n",
            r#"{"timestamp":"2026-11-06T10:00:05.000Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#,
            "\n",
        );
        let rollout = home.write(
            &format!(r"sessions\2026\11\06\rollout-2026-11-06T12-00-00-{ID}.jsonl"),
            start.as_bytes(),
        );
        let log = Arc::new(MemoryLog::default());
        let mut agents = Agents::new(
            vec![Box::new(CodexSource::new(home.path().to_path_buf()))],
            log.clone(),
        );
        let me = crate::process::current().unwrap();
        let live = FakeLiveness::default();
        live.processes.lock().unwrap().push(entry(me, "codex.exe"));
        live.alive.lock().unwrap().push((me, Alive::Yes));
        let locks_dir = home.path().join("thread-writer-locks");
        assert_eq!(
            agents.folders(),
            [WatchFolder {
                dir: locks_dir.clone(),
                subtree: false,
            }]
        );

        let facts = |activity| SessionFacts {
            activity,
            ..codex_session(&lock)
        };
        agents.on_tick();
        let first = agents.snapshots(&live, START);
        assert_eq!(
            first,
            Snapshots {
                events: vec![Event::Sources {
                    source: SourceId::Codex,
                    sessions: vec![(facts(Activity::Working), Alive::Yes)],
                }],
                live: vec![me],
            }
        );

        append(
            &rollout,
            concat!(
                r#"{"timestamp":"2026-11-06T10:01:00.000Z","ordinal":2,"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#,
                "\n"
            )
            .as_bytes(),
        );
        agents.on_tick();
        assert_eq!(
            agents.snapshots(&live, START).events,
            [Event::Sources {
                source: SourceId::Codex,
                sessions: vec![(facts(Activity::Idle), Alive::Yes)],
            }]
        );

        // Listing, finding and asking left the lock as Codex took it: a second taker is still refused.
        let second = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        assert!(matches!(
            second.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        drop(second);

        // Codex closes the lock file, then deletes it.
        drop(held);
        fs::remove_file(&lock).unwrap();
        agents.on_change(&locks_dir, Some(&[PathBuf::from(format!("{ID}.lock"))]));
        assert_eq!(
            agents.snapshots(&live, START).events,
            [Event::Sources {
                source: SourceId::Codex,
                sessions: vec![],
            }]
        );
        assert!(lines(&log).is_empty(), "{:?}", lines(&log));
    }

    #[test]
    fn no_event_when_nothing_changed() {
        let a = Shared::new(SourceId::ClaudeCode);
        a.set(|f| f.sessions = vec![session(1)]);
        let b = Shared::new(SourceId::Codex);
        let (mut agents, _) = agents(&[&a, &b]);
        let live = FakeLiveness::default();
        live.alive.lock().unwrap().push((process(1), Alive::Yes));

        let first = agents.snapshots(&live, START);
        assert_eq!(
            first.events,
            [
                Event::Sources {
                    source: SourceId::ClaudeCode,
                    sessions: vec![(session(1), Alive::Yes)],
                },
                Event::Sources {
                    source: SourceId::Codex,
                    sessions: vec![],
                },
            ],
            "every source on the first call, even one without sessions"
        );
        let again = agents.snapshots(&live, START);
        assert_eq!(again.events, []);
        assert_eq!(again.live, [process(1)], "live processes come every time");

        // The process ended: only that source's snapshot changed.
        live.alive.lock().unwrap().clear();
        let ended = agents.snapshots(&live, START);
        assert_eq!(
            ended.events,
            [Event::Sources {
                source: SourceId::ClaudeCode,
                sessions: vec![(session(1), Alive::Gone)],
            }]
        );
        assert!(ended.live.is_empty());

        agents.forget_posted();
        assert_eq!(agents.snapshots(&live, START).events.len(), 2);
    }

    #[test]
    fn format_problems_log_at_most_once_a_minute_per_source() {
        let clock = FakeClock::new(START);
        let claude = Shared::new(SourceId::ClaudeCode);
        let mut unreadable = session(1);
        unreadable.format = FormatHealth::Unreadable("status file");
        let mut unknown = session(2);
        unknown.format = FormatHealth::UnknownValue("status", "thinking".into());
        claude.set(|f| f.sessions = vec![unreadable.clone()]);
        let codex = Shared::new(SourceId::Codex);
        let (mut agents, log) = agents(&[&claude, &codex]);
        let live = FakeLiveness::default();
        let take = |log: &MemoryLog| std::mem::take(&mut *log.lines.lock().unwrap()).len();

        agents.snapshots(&live, clock.now());
        assert_eq!(lines(&log), ["Claude Code: format problem in status file"]);
        take(&log);

        // A new problem 10 s later waits for the minute; another source logs at once.
        clock.run(10_000);
        claude.set(|f| f.sessions.push(unknown.clone()));
        let mut codex_unknown = session(3);
        codex_unknown.format = FormatHealth::UnknownValue("type", "x".into());
        codex.set(|f| f.sessions = vec![codex_unknown]);
        agents.snapshots(&live, clock.now());
        assert_eq!(lines(&log), ["Codex CLI: unknown value in type"]);
        take(&log);
        clock.run(49_000);
        agents.snapshots(&live, clock.now());
        assert_eq!(take(&log), 0, "59 s after the last line");

        clock.run(1_000);
        agents.snapshots(&live, clock.now());
        assert_eq!(lines(&log), ["Claude Code: unknown value in status"]);
        take(&log);

        // Still there an hour later: already logged.
        clock.run(3_600_000);
        agents.snapshots(&live, clock.now());
        assert_eq!(take(&log), 0);

        // Gone, then back: logged again.
        claude.set(|f| f.sessions = vec![session(1)]);
        agents.snapshots(&live, clock.now());
        claude.set(|f| f.sessions = vec![unreadable]);
        clock.run(60_000);
        agents.snapshots(&live, clock.now());
        assert_eq!(lines(&log), ["Claude Code: format problem in status file"]);
        assert!(
            !log.lines
                .lock()
                .unwrap()
                .iter()
                .any(|(_, _, text)| text.contains("thinking")),
            "a file's value never reaches the log"
        );
    }
}
