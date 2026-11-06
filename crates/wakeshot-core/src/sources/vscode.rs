//! Not started by the v1 tray: a Local chat whose window is closed mid-turn writes nothing, so it stays Pending on disk
//! and its end is not seen (`docs/tasks/p4-live-vscode-stop.md`). Not in v1 per "No guessing" (`docs/product.md`),
//! Haik 2026-11-06 (`docs/tasks/p4-vscode-local-off.md`). Kept, with its tests, for a later version. VS Code's Copilot
//! harness chats hold through `sources::copilot`.
//!
//! VS Code chat sessions: Copilot and any other participant in the chat panel's default "Local" harness (architecture
//! section 7; `docs/research/copilot-vscode-signals.md`, "Recommended detection rules" 1 to 7).
//!
//! Owner: `p4-vscode-source`; fixes by the VS Code live card.
//!
//! Built from VS Code 1.145.1's own code, read as data (`docs/tasks/p4-vscode-source.md`, Log), then checked on real
//! chats (`docs/tasks/p4-live-vscode.md`, `docs/tasks/p4-live-vscode-stop.md`). Rules still marked "not verified"
//! were not seen live.
//!
//! - Each chat is one operation log: `<User>\workspaceStorage\<workspace>\chatSessions\<session id>.jsonl`, or
//!   `<User>\globalStorage\emptyWindowChatSessions\<session id>.jsonl` for a window with no folder (every profile uses
//!   the default profile's folder). VS Code compacts a log into one line in place, so it is read whole
//!   (`WantedFile::Recent`), never tailed. Old `.json` sessions (before 1.109) are not read.
//! - The log is replayed on `requests` only, with VS Code's own rules (`objectMutationLog.ts`): an Initial line
//!   (`kind` 0), then Set, Push and Delete (1 to 3). Per request only `timestamp` and `modelState` are read; every
//!   other value, the prompt and answer text among them, is skipped unread. A line on a path this source does not read
//!   is skipped whatever its kind, so a new kind of write in the answer text cannot make a session hold.
//! - Working: some request's `modelState.value` is 0 (Pending). Waiting: none is 0 and some is 4 (NeedsInput: a tool
//!   approval, a question); VS Code writes no time for it, and it is never remote. Else idle. An unknown value on a
//!   chat's newest request, or a line of unknown `kind` on a path this source reads, is `UnknownValue` (rule 5); an
//!   unknown value on an older request is history, since VS Code sends one request after the other.
//! - Liveness is `ImageSince` (rule 7): a session counts while a process of its build runs that started before the
//!   request that makes it work or wait. A crash leaves a request Pending in the file until that chat is opened again;
//!   once VS Code has started anew, the leftover no longer counts. A request that went back from done to Pending (a
//!   retry from an error card keeps the first send time) or that has no usable time counts while any process of the
//!   build runs.
//! - A line that does not read, or a file with no Initial line, asks for one retry, then is `Unreadable`.
//! - Not in the file: a terminal command still running after its tool returned, which the terminal tool does after 5
//!   minutes with no output or after about 2 minutes (seen live). Each window saves about once a minute, or when it
//!   loses focus, so work shows up to a minute late and ends up to a minute late: a Stop is saved as Cancelled at
//!   the next save (seen live, 40 to 52 s after it).

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

/// What the tooltip names a VS Code chat session. Nearly every user of the chat panel runs Copilot there; other chat
/// participants are read the same way.
pub const LABEL: &str = "Copilot in VS Code";

/// `FormatHealth::UnknownValue` field names: a line's `kind`, and a request's `modelState`. The value is a number or
/// a type word, never file text.
pub const KIND: &str = "kind";
pub const MODEL_STATE: &str = "modelState";

/// `FormatHealth::Unreadable` text.
pub const UNREADABLE: &str = "chat session file";

/// The chat folder of one workspace, under `workspaceStorage\<workspace>\`.
pub const CHAT_SESSIONS: &str = "chatSessions";

/// A VS Code build: the folder its data lives in (VS Code's `nameShort`) and the image its processes run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Product {
    pub folder: &'static str,
    pub images: &'static [&'static str],
}

/// VS Code Stable: `%APPDATA%\Code`, `Code.exe` (seen on the laptop).
pub const STABLE: Product = Product {
    folder: "Code",
    images: &["Code.exe"],
};

/// VS Code Insiders: `%APPDATA%\Code - Insiders` (VS Code docs, `configure/profiles.md`) and `Code - Insiders.exe`
/// (VS Code's own `test/sanity/src/context.ts`). Not seen on a PC.
pub const INSIDERS: Product = Product {
    folder: "Code - Insiders",
    images: &["Code - Insiders.exe"],
};

/// One build's `User` folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    pub user_dir: PathBuf,
    pub product: Product,
}

/// The `User` folders of Stable and Insiders, found the way VS Code finds its data folder (`userDataPath.ts`):
/// `vscode_appdata` (the value of `VSCODE_APPDATA`) when set, else `appdata` (`APPDATA`), else
/// `<profile>\AppData\Roaming`; then `<build folder>\User`. An empty value counts as unset, as in VS Code. The caller
/// reads the values, so the core stays free of environment reads; a value set in one shell only is not seen.
///
/// `VSCODE_PORTABLE` is not read: VS Code sets it inside its own process when a `data` folder sits next to the install
/// (`bootstrap-node.ts`), so the tray never has it, and that folder is next to an install the tray does not know. A
/// `--user-data-dir` on VS Code's command line is not seen either.
pub fn installs(
    profile: &Path,
    appdata: Option<&OsStr>,
    vscode_appdata: Option<&OsStr>,
) -> Vec<Install> {
    let set = |value: Option<&OsStr>| value.filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = set(vscode_appdata)
        .or_else(|| set(appdata))
        .unwrap_or_else(|| profile.join("AppData").join("Roaming"));
    [STABLE, INSIDERS]
        .into_iter()
        .map(|product| Install {
            user_dir: base.join(product.folder).join("User"),
            product,
        })
        .collect()
}

/// Whether a file name is a chat session log: `<session id>.jsonl`, case ignored as NTFS does. The name filter the
/// watch thread runs before any open.
pub fn is_session_file_name(name: &str) -> bool {
    session_id(name).is_some()
}

/// The session id of a log file name. VS Code's are UUIDs; a name that could hold other text is never a key.
fn session_id(name: &str) -> Option<&str> {
    let split = name.len().checked_sub(SUFFIX.len())?;
    let (id, suffix) = (name.get(..split)?, name.get(split..)?);
    let plain = !id.is_empty()
        && id.len() <= MAX_ID
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
    (suffix.eq_ignore_ascii_case(SUFFIX) && plain).then_some(id)
}

const SUFFIX: &str = ".jsonl";
const WORKSPACE_STORAGE: &str = "workspaceStorage";
const GLOBAL_STORAGE: &str = "globalStorage";
const EMPTY_WINDOW: &str = "emptyWindowChatSessions";
const REQUESTS: &str = "requests";
const TIMESTAMP: &str = "timestamp";
const VALUE: &str = "value";

const INITIAL: u64 = 0;
const SET: u64 = 1;
const PUSH: u64 = 2;
const DELETE: u64 = 3;

/// `ResponseModelState` (`chatService.ts`): 1 Complete, 2 Cancelled, 3 Failed are done.
const PENDING: u64 = 0;
const NEEDS_INPUT: u64 = 4;

/// The longest session id kept as a key.
const MAX_ID: usize = 128;
/// A request index this high is no real chat; a path to it is skipped, so a broken line cannot grow the list.
const MAX_INDEX: u64 = 1 << 16;
/// The longest value a format problem names.
const VALUE_CHARS: usize = 32;
/// 2020-01-01 UTC. A request time before it is no send time of this format (VS Code 1.109 and later), for example a
/// time in seconds; it is not used, so it cannot end a session too early.
const EARLIEST_MS: i64 = 1_577_836_800_000;

/// VS Code's chats in the chat panel, one per session log under each build's `User` folder.
#[derive(Debug)]
pub struct VsCodeSource {
    roots: Vec<Root>,
    files: BTreeMap<PathBuf, Slot>,
}

/// One build's two chat folders.
#[derive(Debug)]
struct Root {
    workspaces: PathBuf,
    empty: PathBuf,
    images: &'static [&'static str],
}

/// One session log, by its path.
#[derive(Debug)]
struct Slot {
    id: String,
    images: &'static [&'static str],
    /// The last read that parsed.
    read: Option<Read>,
    retry_pending: bool,
    unreadable: bool,
}

impl VsCodeSource {
    /// `installs` from [`installs`]: each build's `User` folder and its images.
    pub fn new(installs: Vec<Install>) -> Self {
        let roots = installs
            .into_iter()
            .map(|install| Root {
                workspaces: install.user_dir.join(WORKSPACE_STORAGE),
                empty: install.user_dir.join(GLOBAL_STORAGE).join(EMPTY_WINDOW),
                images: install.product.images,
            })
            .collect();
        Self {
            roots,
            files: BTreeMap::new(),
        }
    }

    /// The session id and images of a log in one of the chat folders; `None` for any other file.
    fn locate<'a>(&self, file: &'a FileId) -> Option<(&'a str, &'static [&'static str])> {
        let id = session_id(file.0.file_name()?.to_str()?)?;
        let dir = file.0.parent()?;
        let workspaces = (dir.file_name() == Some(OsStr::new(CHAT_SESSIONS)))
            .then(|| dir.parent().and_then(Path::parent))
            .flatten();
        let root = self
            .roots
            .iter()
            .find(|root| dir == root.empty || workspaces == Some(root.workspaces.as_path()))?;
        Some((id, root.images))
    }
}

impl AgentSource for VsCodeSource {
    fn id(&self) -> SourceId {
        SourceId::VsCode
    }

    fn wanted(&self) -> Vec<WantedFile> {
        self.roots
            .iter()
            .flat_map(|root| {
                [
                    WantedFile::Recent {
                        root: root.workspaces.clone(),
                        depth: 1,
                        sub: Some(CHAT_SESSIONS),
                        accept: is_session_file_name,
                    },
                    // The folder itself, not `globalStorage`: that one is written all the time by others.
                    WantedFile::Recent {
                        root: root.empty.clone(),
                        depth: 0,
                        sub: None,
                        accept: is_session_file_name,
                    },
                ]
            })
            .collect()
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        let Some((id, images)) = self.locate(file) else {
            return Fed::Taken;
        };
        match data {
            Feed::Whole(bytes) => {
                let slot = self.files.entry(file.0.clone()).or_insert_with(|| Slot {
                    id: id.to_owned(),
                    images,
                    read: None,
                    retry_pending: false,
                    unreadable: false,
                });
                match read(&bytes) {
                    Some(read) => {
                        slot.read = Some(read);
                        slot.retry_pending = false;
                        slot.unreadable = false;
                    }
                    // VS Code appends whole lines and rewrites in place, so a read 200 ms later usually finds the
                    // write done.
                    None if !slot.retry_pending => {
                        slot.retry_pending = true;
                        return Fed::RetryOnce;
                    }
                    None => {
                        slot.retry_pending = false;
                        slot.unreadable = true;
                    }
                }
            }
            Feed::Gone => {
                self.files.remove(&file.0);
            }
            // A session log comes whole or gone; a tail or a listing of it says nothing.
            Feed::Appended(_) | Feed::Present => {}
        }
        Fed::Taken
    }

    /// One entry per session id. VS Code moves a chat to a new folder when an empty window opens a folder, so the
    /// same id can be in two files for a while: the one that says the most stands for both.
    fn sessions(&self) -> Vec<SessionFacts> {
        let mut by_id: BTreeMap<&str, (u8, SessionFacts)> = BTreeMap::new();
        for slot in self.files.values() {
            let Some(facts) = slot.facts() else {
                continue;
            };
            let rank = rank(&facts);
            if by_id
                .get(slot.id.as_str())
                .is_none_or(|(kept, _)| rank > *kept)
            {
                by_id.insert(&slot.id, (rank, facts));
            }
        }
        by_id.into_values().map(|(_, facts)| facts).collect()
    }
}

/// How much a session's facts say: holding first, then waiting, then idle, then unreadable.
fn rank(facts: &SessionFacts) -> u8 {
    match (&facts.format, facts.activity) {
        (FormatHealth::UnknownValue(..), _) | (_, Activity::Working) => 3,
        (FormatHealth::Unreadable(_), _) => 0,
        (_, Activity::Waiting { .. }) => 2,
        (_, Activity::Idle) => 1,
    }
}

impl Slot {
    fn facts(&self) -> Option<SessionFacts> {
        let (activity, since_wall_ms, format) = match (&self.read, self.unreadable) {
            (read, true) => (
                Activity::Idle,
                read.as_ref().and_then(|r| r.since_wall_ms),
                FormatHealth::Unreadable(UNREADABLE),
            ),
            (Some(read), false) => (
                read.activity,
                read.since_wall_ms,
                match &read.problem {
                    Some((field, value)) => FormatHealth::UnknownValue(field, value.clone()),
                    None => FormatHealth::Ok,
                },
            ),
            // The first read failed and its retry is still to come: nothing to say yet.
            (None, false) => return None,
        };
        Some(SessionFacts {
            key: SessionKey {
                source: SourceId::VsCode,
                id: self.id.clone(),
            },
            liveness: Liveness::ImageSince {
                images: self.images,
                since_wall_ms,
            },
            activity,
            timers: Vec::new(),
            open_tasks: 0,
            label: LABEL,
            format,
        })
    }
}

/// What one whole log says.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Read {
    activity: Activity,
    /// For `Liveness::ImageSince`: the newest send time of the requests that decide the activity.
    since_wall_ms: Option<i64>,
    /// The first format problem: a field name and a number or type word.
    problem: Option<(&'static str, String)>,
}

/// One whole log, replayed; `None` when a line does not read or no Initial line came.
fn read(bytes: &[u8]) -> Option<Read> {
    let mut replay = Replay::default();
    for line in bytes.split(|&b| b == b'\n') {
        let line = line.trim_ascii();
        if line.is_empty() {
            continue;
        }
        // VS Code writes every entry as an object; serde would also take a list as a struct, field by position.
        if line.first() != Some(&b'{') {
            return None;
        }
        replay.line(line).ok()?;
    }
    replay.finish()
}

/// A line that does not read: not JSON, or a value on a path this source reads that has another shape.
struct Broken;

/// The fields every line has; `v` is read again only for a path this source reads, so the text in other lines is
/// never kept.
#[derive(Deserialize)]
struct Head {
    #[serde(default)]
    kind: Option<Value>,
    #[serde(default)]
    k: Option<Value>,
}

#[derive(Deserialize)]
struct Initial {
    #[serde(default)]
    v: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    requests: Option<Vec<Option<Raw>>>,
}

/// A Set or Push of the whole list; `i` is where a Push cuts the list first.
#[derive(Deserialize)]
struct List {
    #[serde(default)]
    v: Option<Vec<Option<Raw>>>,
    #[serde(default)]
    i: Option<u64>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    v: Option<Raw>,
}

#[derive(Deserialize)]
struct Field {
    #[serde(default)]
    v: Option<Value>,
}

/// A request as written; `message`, `response` and the rest are skipped unread.
#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    timestamp: Option<Value>,
    #[serde(rename = "modelState", default)]
    model_state: Option<Value>,
}

/// Where a Set, Push or Delete writes, as far as this source reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Requests,
    Item(usize),
    State(usize),
    StateValue(usize),
    Time(usize),
    Elsewhere,
}

fn target(k: Option<&Value>) -> Target {
    let Some(Value::Array(path)) = k else {
        return Target::Elsewhere;
    };
    let Some((first, rest)) = path.split_first() else {
        return Target::Elsewhere;
    };
    if first.as_str() != Some(REQUESTS) {
        return Target::Elsewhere;
    }
    let Some((index, rest)) = rest.split_first() else {
        return Target::Requests;
    };
    let Some(n) = index
        .as_u64()
        .filter(|&n| n < MAX_INDEX)
        .and_then(|n| usize::try_from(n).ok())
    else {
        return Target::Elsewhere;
    };
    let names: Vec<Option<&str>> = rest.iter().map(Value::as_str).collect();
    match names.as_slice() {
        [] => Target::Item(n),
        [Some(TIMESTAMP)] => Target::Time(n),
        [Some(MODEL_STATE)] => Target::State(n),
        [Some(MODEL_STATE), Some(VALUE)] => Target::StateValue(n),
        _ => Target::Elsewhere,
    }
}

/// Whether a line of unknown kind at path `k` could change what this source reads: no path, the root, the list, one
/// request, or a request's `modelState` or `timestamp` and below.
fn touches_read(k: Option<&Value>) -> bool {
    let Some(Value::Array(path)) = k else {
        return true;
    };
    match path.as_slice() {
        [] => true,
        [first, rest @ ..] => {
            first.as_str() == Some(REQUESTS)
                && match rest {
                    [] | [_] => true,
                    [_, field, ..] => matches!(field.as_str(), Some(MODEL_STATE | TIMESTAMP)),
                }
        }
    }
}

/// A request's `modelState`, as far as it says working, waiting or done.
#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    /// No `modelState`: old data, or a request with no response.
    Unset,
    Pending,
    NeedsInput,
    Done,
    Unknown(String),
}

fn state_of(value: Option<&Value>) -> State {
    match value {
        None | Some(Value::Null) => State::Unset,
        Some(Value::Object(fields)) => value_state(fields.get(VALUE)),
        Some(other) => State::Unknown(word(Some(other))),
    }
}

fn value_state(value: Option<&Value>) -> State {
    match value.and_then(Value::as_u64) {
        Some(PENDING) => State::Pending,
        Some(NEEDS_INPUT) => State::NeedsInput,
        Some(1..=3) => State::Done,
        _ => State::Unknown(word(value)),
    }
}

/// A value for a format problem: a number as written, else only its type, so no file text is kept.
fn word(value: Option<&Value>) -> String {
    match value {
        None => "missing".to_owned(),
        Some(Value::Null) => "null".to_owned(),
        Some(Value::Bool(_)) => "bool".to_owned(),
        Some(Value::Number(n)) => n.to_string().chars().take(VALUE_CHARS).collect(),
        Some(Value::String(_)) => "text".to_owned(),
        Some(Value::Array(_)) => "list".to_owned(),
        Some(Value::Object(_)) => "object".to_owned(),
    }
}

/// A send time in epoch ms; a whole float is taken too.
fn sent_ms(value: Option<&Value>) -> Option<i64> {
    let Some(Value::Number(n)) = value else {
        return None;
    };
    let ms = n.as_i64().or_else(|| {
        n.as_f64()
            .filter(|ms| ms.is_finite() && ms.abs() < 9.0e15)
            .map(|ms| ms.floor() as i64)
    })?;
    (ms >= EARLIEST_MS).then_some(ms)
}

#[derive(Clone, Debug)]
struct Request {
    sent_ms: Option<i64>,
    state: State,
    /// A done state was seen for this request in the log.
    done_seen: bool,
    /// It went back from done to Pending or NeedsInput: its time is the first send's, not the new run's.
    reopened: bool,
}

impl Request {
    fn of(raw: Raw) -> Self {
        let state = state_of(raw.model_state.as_ref());
        Self {
            sent_ms: sent_ms(raw.timestamp.as_ref()),
            done_seen: state == State::Done,
            state,
            reopened: false,
        }
    }

    fn set_state(&mut self, state: State) {
        if matches!(state, State::Pending | State::NeedsInput) && self.done_seen {
            self.reopened = true;
        }
        if state == State::Done {
            self.done_seen = true;
        }
        self.state = state;
    }

    /// The latest time a process may have started and still have seen this request sent; `None`: any start.
    fn since(&self) -> Option<i64> {
        if self.reopened { None } else { self.sent_ms }
    }

    fn open(&self) -> bool {
        matches!(self.state, State::Pending | State::NeedsInput)
    }
}

fn requests_of(raw: Vec<Option<Raw>>) -> Vec<Option<Request>> {
    raw.into_iter().map(|r| r.map(Request::of)).collect()
}

/// The log's state as far as this source reads it. `requests` is `None` while VS Code's would be undefined.
#[derive(Default)]
struct Replay {
    started: bool,
    requests: Option<Vec<Option<Request>>>,
    /// The first unknown kind on a path this source reads.
    problem: Option<(&'static str, String)>,
}

impl Replay {
    fn line(&mut self, line: &[u8]) -> Result<(), Broken> {
        let head: Head = parse(line)?;
        match head.kind.as_ref().and_then(Value::as_u64) {
            Some(INITIAL) => {
                let initial: Initial = parse(line)?;
                self.started = true;
                self.requests = initial.v.and_then(|d| d.requests).map(requests_of);
            }
            // Before the Initial line there is nothing to apply it to; a file with none is unreadable at the end.
            Some(SET | PUSH | DELETE) if !self.started => {}
            Some(kind @ (SET | PUSH | DELETE)) => {
                self.apply(kind, target(head.k.as_ref()), line)?
            }
            _ => {
                if self.problem.is_none() && touches_read(head.k.as_ref()) {
                    self.problem = Some((KIND, word(head.kind.as_ref())));
                }
            }
        }
        Ok(())
    }

    /// VS Code's own rules (`objectMutationLog.ts`, `_applySet` and `_applyPush`): Delete sets the value to undefined,
    /// a write past the end of a list leaves holes, Push cuts at `i` first. A write VS Code itself could not apply (into
    /// a request that is not there) is skipped.
    fn apply(&mut self, kind: u64, target: Target, line: &[u8]) -> Result<(), Broken> {
        match (kind, target) {
            (_, Target::Elsewhere) => {}
            (SET, Target::Requests) => {
                let list: List = parse(line)?;
                self.requests = list.v.map(requests_of);
            }
            (PUSH, Target::Requests) => {
                let list: List = parse(line)?;
                let requests = self.requests.get_or_insert_with(Vec::new);
                if let Some(cut) = list.i {
                    if cut >= MAX_INDEX {
                        return Ok(());
                    }
                    requests.resize_with(usize::try_from(cut).unwrap_or(0), || None);
                }
                requests.extend(list.v.into_iter().flatten().map(|r| r.map(Request::of)));
            }
            (DELETE, Target::Requests) => self.requests = None,
            (SET, Target::Item(n)) => {
                let item: Item = parse(line)?;
                self.put(n, item.v.map(Request::of));
            }
            (DELETE, Target::Item(n)) => self.put(n, None),
            (SET, Target::State(n)) => {
                let field: Field = parse(line)?;
                if let Some(request) = self.request(n) {
                    request.set_state(state_of(field.v.as_ref()));
                }
            }
            (DELETE, Target::State(n)) => {
                if let Some(request) = self.request(n) {
                    request.set_state(State::Unset);
                }
            }
            (SET | DELETE, Target::StateValue(n)) => {
                let value = if kind == SET {
                    parse::<Field>(line)?.v
                } else {
                    None
                };
                if let Some(request) = self.request(n)
                    && request.state != State::Unset
                {
                    request.set_state(value_state(value.as_ref()));
                }
            }
            (SET | DELETE, Target::Time(n)) => {
                let value = if kind == SET {
                    parse::<Field>(line)?.v
                } else {
                    None
                };
                if let Some(request) = self.request(n) {
                    request.sent_ms = sent_ms(value.as_ref());
                }
            }
            // A Push onto a request or one of its fields is nothing VS Code writes.
            _ => {}
        }
        Ok(())
    }

    fn put(&mut self, n: usize, item: Option<Request>) {
        let Some(requests) = self.requests.as_mut() else {
            return;
        };
        if n >= requests.len() {
            requests.resize_with(n + 1, || None);
        }
        requests[n] = item;
    }

    fn request(&mut self, n: usize) -> Option<&mut Request> {
        self.requests.as_mut()?.get_mut(n)?.as_mut()
    }

    fn finish(self) -> Option<Read> {
        if !self.started {
            return None;
        }
        let all: Vec<&Request> = self.requests.iter().flatten().flatten().collect();
        let newest = all.len().checked_sub(1);
        let unknown = newest.and_then(|i| match &all[i].state {
            State::Unknown(value) => Some((MODEL_STATE, value.clone())),
            _ => None,
        });
        let activity = if all.iter().any(|r| r.state == State::Pending) {
            Activity::Working
        } else if all.iter().any(|r| r.state == State::NeedsInput) {
            Activity::Waiting {
                since_wall_ms: None,
                remote: false,
            }
        } else {
            Activity::Idle
        };
        // After an unknown kind any request may be the one it changed.
        let deciding: Vec<&Request> = if self.problem.is_some() {
            all.clone()
        } else {
            all.iter()
                .enumerate()
                .filter(|&(i, r)| r.open() || (Some(i) == newest && unknown.is_some()))
                .map(|(_, r)| *r)
                .collect()
        };
        let since_wall_ms = if deciding.is_empty() {
            latest(&all)
        } else {
            latest(&deciding)
        };
        Some(Read {
            activity,
            since_wall_ms,
            problem: self.problem.or(unknown),
        })
    }
}

/// The newest `since` of `requests`; `None` when any has none (any start counts) or there are none.
fn latest(requests: &[&Request]) -> Option<i64> {
    requests
        .iter()
        .map(|r| r.since())
        .try_fold(None, |newest: Option<i64>, since| {
            since.map(|ms| Some(newest.map_or(ms, |n| n.max(ms))))
        })
        .flatten()
}

fn parse<'a, T: Deserialize<'a>>(line: &'a [u8]) -> Result<T, Broken> {
    serde_json::from_slice(line).map_err(|_| Broken)
}
