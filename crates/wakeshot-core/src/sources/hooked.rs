//! What the hooked sources share, Cursor's and Antigravity's (`docs/agent-hooks.md`): the state file format, the chat
//! id rule, the event rules and the walk up a process list to the agent's process, so neither source module owns the
//! other's code. The CLI's `hook` command uses it too: the input parse and the state file text live here, since the
//! core has `serde` and the CLI has no JSON crate of its own. "Step N" is step N of "The hook command" in
//! `docs/agent-hooks.md`, and a quoted section name is a section there.
//!
//! Privacy ("Privacy"): the parse keeps only the fields an agent's table names and skips every other value unread
//! into memory; no `serde_json` error text leaves this module, since it can quote an input value.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufReader, ErrorKind, Read};
use std::path::{Component, Path, PathBuf};

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess,
    Visitor,
};
use serde::{Deserialize, Serialize};

use crate::process::{FileTime, ProcessRef};
use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

/// The most input the hook reads (step 1); its caller drains the rest unread, so the agent's pipe never breaks.
pub const INPUT_LIMIT: u64 = 64 * 1024 * 1024;

/// The version of every file the hook writes.
const FILE_VERSION: u32 = 1;

/// How many parents the walk looks at, from the hook's own parent up (step 3).
pub const WALK_STEPS: usize = 10;

/// Says the agent's input did not read: the source shows "update Wakeshot" while the agent process lives.
pub const PROBLEM_FILE: &str = "problem.json";

/// Says the runtime check found the agent's input on a command line ("Privacy", "How the build checks it").
pub const BLOCKED_FILE: &str = "blocked.json";

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// What Wakeshot knows of one hooked agent: what its hook is handed, what each event does, which process runs the
/// hook, and how the source names its sessions. The CLI's `hook` command and the agent's source read the same table.
#[derive(Debug)]
pub struct Agent {
    pub source: SourceId,
    /// The word in `wakeshot hook <agent> <event>` and the folder `agents\<agent>`.
    pub name: &'static str,
    /// The agent's plain name, for a file whose `label` is none of `images`' labels and for its problems.
    pub label: &'static str,
    pub fields: InputFields,
    pub events: &'static [EventRule],
    /// Nearest first wins, so the order only matters for one process that two rules could match.
    pub images: &'static [ImageRule],
    /// A key of the agent's own input. Found in the hook's parent's command line, the input came that way, so it may
    /// be in process audit logs. Bare, no quotes: Node writes a quote inside an argument as `\"`.
    pub payload_key: &'static str,
    /// The `FormatHealth::Unreadable` texts of `problem.json` and `blocked.json`.
    pub input_problem: &'static str,
    pub blocked_problem: &'static str,
}

impl Agent {
    pub fn event(&self, name: &str) -> Option<&EventRule> {
        self.events.iter().find(|rule| rule.name == name)
    }

    fn image(&self, key: &str) -> Option<&'static ImageRule> {
        self.images.iter().find(|rule| rule.key == key)
    }
}

/// The names of the input fields the hook keeps. Every other field is skipped by the parser without being stored.
#[derive(Debug)]
pub struct InputFields {
    /// The chat id: the state file's name, so it must have the UUID shape.
    pub chat: &'static str,
    /// The turn id, which a `stop` must name to delete the chat's file.
    pub turn: Option<&'static str>,
    /// The event name; when present it must be the event of the command.
    pub event: Option<&'static str>,
    /// The agent's version, logged on a problem only.
    pub version: Option<&'static str>,
    /// A flag that an end event is a full stop (Antigravity's `fullyIdle`).
    pub idle: Option<&'static str>,
}

/// What one hooked event does to the chat's file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRule {
    pub name: &'static str,
    pub action: Action,
    /// The event also prunes the folder (step 5): the turn start, so a long-gone chat is cleared at the next prompt.
    pub prune: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Writes the chat's file with this state. Skipped when the file already holds exactly that, so an event that
    /// fires on every model call costs one read.
    Write(State),
    /// Deletes the chat's file if it names this event's turn, so a late end of turn N leaves turn N+1's file.
    DeleteIfTurn,
    /// Deletes the chat's file.
    Delete,
    /// The idle flag true: deletes the chat's file; false: writes it with `Background`.
    StopIfIdle,
    /// Deletes the chat's file, unless the input's [`Running`] field names a turn that still runs: then writes it
    /// `Working`, so that turn's own end deletes it. Cursor closes a running chat's tab with this event while the
    /// turn goes on and fires its own `stop` later (seen live). Any other value, another type or no field at all
    /// deletes, as a plain end does.
    EndUnlessRunning(Running),
}

/// The input field and the one value of it that says a turn still runs, for [`Action::EndUnlessRunning`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    pub field: &'static str,
    pub value: &'static str,
}

impl EventRule {
    fn running(&self) -> Option<Running> {
        match self.action {
            Action::EndUnlessRunning(running) => Some(running),
            _ => None,
        }
    }
}

/// A process that runs the agent's hooks: an exe name (ASCII case ignored), and where that exe must lie when the name
/// alone says nothing (a `node.exe`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageRule {
    pub image: &'static str,
    pub under: Option<Under>,
    /// The rule's word in the hand-off's checked values: a fixed set, so no file text reaches a command line.
    pub key: &'static str,
    /// What the tooltip names a session the rule found.
    pub label: &'static str,
}

/// A folder by a known base and the parts below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Under {
    pub base: Base,
    pub parts: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    LocalAppData,
    Profile,
}

/// The known folders, read by the caller from their folder ids, so the core reads no environment. `None`: not known,
/// so a rule under it matches nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bases {
    pub local_app_data: Option<PathBuf>,
    pub profile: Option<PathBuf>,
}

impl Bases {
    fn get(&self, base: Base) -> Option<&Path> {
        match base {
            Base::LocalAppData => self.local_app_data.as_deref(),
            Base::Profile => self.profile.as_deref(),
        }
    }
}

/// What a chat file says the agent does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Working,
    /// The agent's loop ended but its background tasks run (Antigravity's `fullyIdle` false).
    Background,
}

/// Why a problem file was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Problem {
    Input,
    CommandLine,
}

impl Problem {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Input => PROBLEM_FILE,
            Self::CommandLine => BLOCKED_FILE,
        }
    }
}

/// One event's work on the agent's folder, decided before any file is touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Write(State),
    DeleteIfTurn,
    Delete,
    Problem(Problem),
}

/// The kept input, checked: `chat` has the UUID shape and is lowercased, `turn` has the turn id shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookInput {
    pub chat: String,
    pub turn: String,
    /// Whether an end event says the agent is fully idle: Antigravity's `fullyIdle`, or for an
    /// [`Action::EndUnlessRunning`] event whether its field names no running turn. `None`: the input does not say.
    pub idle: Option<bool>,
}

/// Why the input counts as unreadable. Every text is fixed: a parse error keeps only its class and place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputProblem {
    Parse {
        class: &'static str,
        line: usize,
        column: usize,
    },
    NoChat,
    ChatShape,
    TurnShape,
    OtherEvent,
    NoIdle,
}

impl fmt::Display for InputProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                class,
                line,
                column,
            } => write!(
                f,
                "the input does not parse ({class} at line {line}, column {column})"
            ),
            Self::NoChat => f.write_str("the input has no chat id"),
            Self::ChatShape => f.write_str("the chat id is not a UUID"),
            Self::TurnShape => f.write_str("the turn id has another shape"),
            Self::OtherEvent => f.write_str("the input names another event"),
            Self::NoIdle => f.write_str("the input has no idle flag"),
        }
    }
}

/// The parse's answer: the checked input or why it is unreadable, and the agent's version for a problem's log line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub input: Result<HookInput, InputProblem>,
    pub version: Option<String>,
}

/// Streams `input` (at most what the caller lets through) through `serde_json` into the fields `agent` names, then
/// checks them for `event`. A UTF-8 byte order mark is skipped.
pub fn parse_input(agent: &Agent, event: &str, input: impl Read) -> Parsed {
    let mut reader = BufReader::new(input);
    let mut head = [0u8; 3];
    let got = read_up_to(&mut reader, &mut head);
    let head = if head[..got] == UTF8_BOM {
        &[][..]
    } else {
        &head[..got]
    };
    let mut de = serde_json::Deserializer::from_reader(head.chain(reader));
    let seed = InputSeed {
        fields: &agent.fields,
        running: agent.event(event).and_then(EventRule::running),
    };
    let raw = seed
        .deserialize(&mut de)
        .and_then(|raw| de.end().map(|()| raw));
    match raw {
        Ok(raw) => Parsed {
            version: raw.version.as_deref().and_then(log_version),
            input: check(event, raw),
        },
        Err(e) => Parsed {
            input: Err(InputProblem::Parse {
                class: class_word(e.classify()),
                line: e.line(),
                column: e.column(),
            }),
            version: None,
        },
    }
}

fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> usize {
    let mut got = 0;
    while got < buf.len() {
        match reader.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            // The parse reads on and reports the failure as its own.
            Err(_) => break,
        }
    }
    got
}

fn class_word(class: serde_json::error::Category) -> &'static str {
    match class {
        serde_json::error::Category::Io => "io",
        serde_json::error::Category::Syntax => "syntax",
        serde_json::error::Category::Data => "data",
        serde_json::error::Category::Eof => "eof",
    }
}

fn check(event: &str, raw: RawInput) -> Result<HookInput, InputProblem> {
    if raw.event.as_deref().is_some_and(|named| named != event) {
        return Err(InputProblem::OtherEvent);
    }
    let chat =
        chat_id(raw.chat.as_deref().ok_or(InputProblem::NoChat)?).ok_or(InputProblem::ChatShape)?;
    let turn = raw.turn.unwrap_or_default();
    if !is_turn_id(&turn) {
        return Err(InputProblem::TurnShape);
    }
    Ok(HookInput {
        chat,
        turn,
        // An agent's table has one or the other, so the two never meet.
        idle: raw.idle.or(raw.running.map(|running| !running)),
    })
}

/// The chat id as a file name: the UUID shape (8-4-4-4-12 hex digits), lowercased, so no device name (`CON`), no path
/// part and no case twin can occur. `None` for any other shape.
pub fn chat_id(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let shaped = bytes.len() == 36
        && bytes.iter().enumerate().all(|(at, &b)| {
            if matches!(at, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        });
    shaped.then(|| value.to_ascii_lowercase())
}

/// A turn id: at most 80 of `A-Z a-z 0-9 - _`, or empty (an agent with no turn id).
fn is_turn_id(value: &str) -> bool {
    value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The agent's version as a log line may hold it: cut to 32 characters, and only when those start with a digit and are
/// all of `0-9 A-Z a-z . -`. Anything else (a word, a path) gives `None`.
fn log_version(value: &str) -> Option<String> {
    let cut: String = value.chars().take(32).collect();
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
    (cut.starts_with(|c: char| c.is_ascii_digit()) && cut.chars().all(allowed)).then_some(cut)
}

/// What the event does with this input.
pub fn resolve(rule: &EventRule, input: &HookInput) -> Result<Op, InputProblem> {
    Ok(match rule.action {
        Action::Write(state) => Op::Write(state),
        Action::DeleteIfTurn => Op::DeleteIfTurn,
        Action::Delete => Op::Delete,
        Action::StopIfIdle => match input.idle {
            Some(true) => Op::Delete,
            Some(false) => Op::Write(State::Background),
            None => return Err(InputProblem::NoIdle),
        },
        Action::EndUnlessRunning(_) => match input.idle {
            Some(false) => Op::Write(State::Working),
            Some(true) | None => Op::Delete,
        },
    })
}

#[derive(Default)]
struct RawInput {
    chat: Option<String>,
    turn: Option<String>,
    event: Option<String>,
    version: Option<String>,
    idle: Option<bool>,
    /// Whether the event's [`Running`] field holds its running value; its text is never kept.
    running: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Chat,
    Turn,
    Event,
    Version,
    Idle,
    Running,
    Other,
}

/// Names a key by comparing it with the table, so a key is never kept.
struct KeySeed<'a> {
    fields: &'a InputFields,
    running: Option<Running>,
}

impl<'de> DeserializeSeed<'de> for KeySeed<'_> {
    type Value = Key;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Key, D::Error> {
        d.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for KeySeed<'_> {
    type Value = Key;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a field name")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<Key, E> {
        let fields = self.fields;
        let named = |name: Option<&str>| name == Some(key);
        Ok(if key == fields.chat {
            Key::Chat
        } else if named(fields.turn) {
            Key::Turn
        } else if named(fields.event) {
            Key::Event
        } else if named(fields.version) {
            Key::Version
        } else if named(fields.idle) {
            Key::Idle
        } else if named(self.running.map(|running| running.field)) {
            Key::Running
        } else {
            Key::Other
        })
    }
}

/// Whether a value is the given string. Any other value or type is no match, never a parse error, so a changed field
/// type only ends the chat as a plain end would.
struct IsText(&'static str);

impl<'de> DeserializeSeed<'de> for IsText {
    type Value = bool;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<bool, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for IsText {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any value")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<bool, E> {
        Ok(value == self.0)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_unit<E: de::Error>(self) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_none<E: de::Error>(self) -> Result<bool, E> {
        Ok(false)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<bool, D::Error> {
        d.deserialize_any(self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<bool, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(false)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<bool, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(false)
    }
}

struct InputSeed<'a> {
    fields: &'a InputFields,
    running: Option<Running>,
}

impl<'de> DeserializeSeed<'de> for InputSeed<'_> {
    type Value = RawInput;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<RawInput, D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for InputSeed<'_> {
    type Value = RawInput;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<RawInput, A::Error> {
        let mut raw = RawInput::default();
        let mut seen = Vec::with_capacity(6);
        let keys = || KeySeed {
            fields: self.fields,
            running: self.running,
        };
        while let Some(key) = map.next_key_seed(keys())? {
            if key == Key::Other {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            // Two values for one kept field: which one the agent meant is unknown, so the input is unreadable.
            if seen.contains(&key) {
                return Err(de::Error::custom("duplicate field"));
            }
            seen.push(key);
            match key {
                Key::Chat => raw.chat = map.next_value()?,
                Key::Turn => raw.turn = map.next_value()?,
                Key::Event => raw.event = map.next_value()?,
                Key::Version => raw.version = map.next_value()?,
                Key::Idle => raw.idle = map.next_value()?,
                Key::Running => {
                    let value = self.running.map_or("", |running| running.value);
                    raw.running = Some(map.next_value_seed(IsText(value))?);
                }
                Key::Other => {}
            }
        }
        Ok(raw)
    }
}

/// `agents\<agent>` in the data folder.
pub fn agent_folder(data_root: &Path, agent: &Agent) -> PathBuf {
    data_root.join("agents").join(agent.name)
}

/// The chat's file name in the agent's folder.
pub fn chat_file(chat: &str) -> String {
    format!("{chat}.json")
}

/// `agents\<agent>\<chat>.json`: one chat the agent works on. `pid` and `procStart` name the agent process the walk
/// found, never the hook's own, which is gone within a second.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateFile {
    pub version: u32,
    pub pid: u32,
    pub proc_start: FileTime,
    pub generation: String,
    pub label: String,
    pub state: State,
    /// The bare name of the agent process's own log, for an agent whose turn can end with no hook (Antigravity's Esc):
    /// its source reads that log for the end. Absent for every other agent, so their files keep their bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// With `log`: when the agent started the hook that wrote the file, so an end the log names at or after it ends
    /// this turn, and an older one does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_at: Option<FileTime>,
}

/// What a hook notes for an agent whose source reads the agent's own log ([`StateFile::log`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogMark {
    /// A bare file name of [`is_log_name`]'s shape, never a path: the source joins it to its own fixed folder.
    pub name: String,
    /// The start of the process the agent ran the hook in (its shell), the earliest moment the hook can name.
    pub turn_at: FileTime,
}

/// A log file name as a state file and the checked values may carry it: 5 to 64 of `A-Z a-z 0-9 - _ .`, starting
/// with a letter or digit and ending in `.log`, so it names no folder, no device and no other kind of file.
pub fn is_log_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (5..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && name.ends_with(".log")
        && bytes
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// `problem.json` and `blocked.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemFile {
    pub version: u32,
    pub pid: u32,
    pub proc_start: FileTime,
    pub problem: Problem,
}

/// A file as read: version 1, a newer version this build does not know, or not a file of this shape at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRead<T> {
    V1(T),
    Newer(u32),
    Bad,
}

impl<T> FileRead<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> FileRead<U> {
        match self {
            Self::V1(file) => FileRead::V1(f(file)),
            Self::Newer(version) => FileRead::Newer(version),
            Self::Bad => FileRead::Bad,
        }
    }
}

#[derive(Deserialize)]
struct Versioned {
    version: u32,
}

fn read_file<T: DeserializeOwned>(bytes: &[u8]) -> FileRead<T> {
    // serde also fills a struct from a JSON array, so the text must open with a brace to be an object.
    if bytes.trim_ascii_start().first() != Some(&b'{') {
        return FileRead::Bad;
    }
    match serde_json::from_slice::<Versioned>(bytes).map(|v| v.version) {
        Ok(FILE_VERSION) => serde_json::from_slice(bytes).map_or(FileRead::Bad, FileRead::V1),
        Ok(version) if version > FILE_VERSION => FileRead::Newer(version),
        _ => FileRead::Bad,
    }
}

pub fn read_state(bytes: &[u8]) -> FileRead<StateFile> {
    read_file(bytes)
}

pub fn read_problem(bytes: &[u8]) -> FileRead<ProblemFile> {
    read_file(bytes)
}

/// The chat file's text.
pub fn state_text(process: ProcessRef, turn: &str, label: &str, state: State) -> Vec<u8> {
    state_text_marked(process, turn, label, state, None)
}

/// The chat file's text, with the agent's log noted when the hook found it.
pub fn state_text_marked(
    process: ProcessRef,
    turn: &str,
    label: &str,
    state: State,
    log: Option<&LogMark>,
) -> Vec<u8> {
    let file = StateFile {
        version: FILE_VERSION,
        pid: process.pid,
        proc_start: process.start,
        generation: turn.to_owned(),
        label: label.to_owned(),
        state,
        log: log.map(|mark| mark.name.clone()),
        turn_at: log.map(|mark| mark.turn_at),
    };
    // Plain fields only, so serialization cannot fail.
    serde_json::to_vec(&file).unwrap_or_default()
}

/// A problem file's text.
pub fn problem_text(process: ProcessRef, problem: Problem) -> Vec<u8> {
    let file = ProblemFile {
        version: FILE_VERSION,
        pid: process.pid,
        proc_start: process.start,
        problem,
    };
    serde_json::to_vec(&file).unwrap_or_default()
}

/// Whether a chat file is version 1 and names this turn, the one case a `stop` deletes it.
pub fn names_turn(bytes: &[u8], turn: &str) -> bool {
    matches!(read_state(bytes), FileRead::V1(file) if file.generation == turn)
}

/// The process a version 1 file of either shape names, for the prune: it deletes such a file once that process is
/// gone, and nothing else.
pub fn named_process(bytes: &[u8]) -> Option<ProcessRef> {
    if let FileRead::V1(file) = read_state(bytes) {
        return Some(ProcessRef {
            pid: file.pid,
            start: file.proc_start,
        });
    }
    match read_problem(bytes) {
        FileRead::V1(file) => Some(ProcessRef {
            pid: file.pid,
            start: file.proc_start,
        }),
        _ => None,
    }
}

/// One process of the system list, as the walk needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed<'a> {
    pub pid: u32,
    pub parent: u32,
    pub start: FileTime,
    pub image: &'a str,
}

/// The agent process the walk found and the rule that matched it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub process: ProcessRef,
    pub image: &'static ImageRule,
}

/// The walk's answer. `parent`: the hook's own parent (the agent's shell), for the runtime transport check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    pub parent: Option<ProcessRef>,
    pub agent: Option<Found>,
}

/// Walks up from `me` (the hook) at most [`WALK_STEPS`] parents and takes the nearest one a rule of `agent` matches:
/// the process that ran the hook, never a longer-lived one above it. A parent counts only when it is listed and
/// started at or before its child, so a reused pid ends the walk. `path_of` gives a process's full exe path; it is
/// asked only for a rule with `under`.
pub fn walk(
    agent: &'static Agent,
    list: &[Listed<'_>],
    me: ProcessRef,
    bases: &Bases,
    mut path_of: impl FnMut(ProcessRef) -> Option<PathBuf>,
) -> Walk {
    let mut answer = Walk::default();
    let Some(mut child) = list
        .iter()
        .find(|e| e.pid == me.pid && e.start == me.start)
        .copied()
    else {
        return answer;
    };
    for _ in 0..WALK_STEPS {
        let Some(up) = list
            .iter()
            .find(|e| e.pid == child.parent && e.pid != child.pid && e.start <= child.start)
            .copied()
        else {
            break;
        };
        let process = ProcessRef {
            pid: up.pid,
            start: up.start,
        };
        answer.parent.get_or_insert(process);
        let found = agent.images.iter().find(|rule| {
            up.image.eq_ignore_ascii_case(rule.image)
                && rule.under.is_none_or(|under| {
                    let Some(base) = bases.get(under.base) else {
                        return false;
                    };
                    let root = under
                        .parts
                        .iter()
                        .fold(base.to_path_buf(), |p, part| p.join(part));
                    path_of(process).is_some_and(|path| is_under(&path, &root))
                })
        });
        if let Some(image) = found {
            answer.agent = Some(Found { process, image });
            return answer;
        }
        child = up;
    }
    answer
}

/// Whether `path` lies below `root`, each part compared with case ignored (Windows paths), and nothing in the rest
/// climbs back out.
fn is_under(path: &Path, root: &Path) -> bool {
    let fold = |part: Component<'_>| part.as_os_str().to_string_lossy().to_lowercase();
    let mut parts = path.components();
    for want in root.components() {
        match parts.next() {
            Some(part) if fold(part) == fold(want) => {}
            _ => return false,
        }
    }
    let mut rest = parts.peekable();
    rest.peek().is_some() && rest.all(|part| matches!(part, Component::Normal(_)))
}

/// The checked values an elevated hook hands to its Medium copy (step 4): `hook <agent> <event> --checked <chat>
/// <turn> <pid> <procStart> <image key> <op> <deadline> [<log> <turnAt>]`. Each is from a fixed set, and the copy
/// checks each again, since any process of the user can run it with any values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    /// Empty for a problem, else a lowercase chat id.
    pub chat: String,
    pub turn: String,
    pub process: ProcessRef,
    pub image: &'static ImageRule,
    pub op: Op,
    /// Wall ms (since 1970) after which a copy drops a `Write`, so a slow copy never lands after the turn's end.
    pub deadline_wall_ms: i64,
    /// Only with a `Write`: the elevated hook found it, since the copy starts later than the moment it names.
    pub log: Option<LogMark>,
}

/// The number of checked values, and the two more a [`LogMark`] adds.
const CHECKED_VALUES: usize = 7;
const CHECKED_LOG_VALUES: usize = 2;

impl Checked {
    pub fn values(&self) -> Vec<String> {
        let mut values = vec![
            self.chat.clone(),
            self.turn.clone(),
            self.process.pid.to_string(),
            self.process.start.to_string(),
            self.image.key.to_owned(),
            op_word(self.op).to_owned(),
            self.deadline_wall_ms.to_string(),
        ];
        if let Some(mark) = &self.log {
            values.extend([mark.name.clone(), mark.turn_at.to_string()]);
        }
        values
    }

    /// The values back, each checked as the hook checks its input; `None` when one fails.
    pub fn parse(agent: &'static Agent, rule: &EventRule, values: &[String]) -> Option<Self> {
        let (values, log) = if values.len() == CHECKED_VALUES + CHECKED_LOG_VALUES {
            let (head, tail) = values.split_at(CHECKED_VALUES);
            let [name, turn_at] = tail else {
                return None;
            };
            if !is_log_name(name) {
                return None;
            }
            let mark = LogMark {
                name: name.clone(),
                turn_at: FileTime(digits(turn_at)?),
            };
            (head, Some(mark))
        } else {
            (values, None)
        };
        let [chat, turn, pid, start, key, op, deadline] = values else {
            return None;
        };
        let op = op_for(rule, op)?;
        let ids_fit = match op {
            Op::Problem(_) => chat.is_empty() && turn.is_empty(),
            _ => chat_id(chat).as_deref() == Some(chat.as_str()) && is_turn_id(turn),
        };
        // A log matters only to a write: a delete or a problem ends or replaces the file that would name it.
        if !ids_fit || (log.is_some() && !matches!(op, Op::Write(_))) {
            return None;
        }
        Some(Self {
            chat: chat.clone(),
            turn: turn.clone(),
            process: ProcessRef {
                pid: digits(pid)?,
                start: FileTime(digits(start)?),
            },
            image: agent.image(key)?,
            op,
            deadline_wall_ms: digits(deadline)?,
            log,
        })
    }
}

fn digits<T: std::str::FromStr>(text: &str) -> Option<T> {
    let plain = (1..=20).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_digit());
    plain.then(|| text.parse().ok()).flatten()
}

fn op_word(op: Op) -> &'static str {
    match op {
        Op::Write(State::Working) => "working",
        Op::Write(State::Background) => "background",
        Op::DeleteIfTurn | Op::Delete => "none",
        Op::Problem(Problem::Input) => "input",
        Op::Problem(Problem::CommandLine) => "command-line",
    }
}

/// The op a checked word gives for this event; only what the event itself could do, or a problem.
fn op_for(rule: &EventRule, word: &str) -> Option<Op> {
    let op = match (word, rule.action) {
        ("input", _) => Op::Problem(Problem::Input),
        ("command-line", _) => Op::Problem(Problem::CommandLine),
        ("working", Action::Write(State::Working)) => Op::Write(State::Working),
        ("background", Action::Write(State::Background) | Action::StopIfIdle) => {
            Op::Write(State::Background)
        }
        ("working", Action::EndUnlessRunning(_)) => Op::Write(State::Working),
        ("none", Action::DeleteIfTurn) => Op::DeleteIfTurn,
        ("none", Action::Delete | Action::StopIfIdle | Action::EndUnlessRunning(_)) => Op::Delete,
        _ => return None,
    };
    Some(op)
}

/// An agent's sessions from the files its hook writes in `agents\<agent>\` (`docs/agent-hooks.md`, "How the source
/// reads them"): one working session per chat file while its process lives, and one "update Wakeshot" entry per
/// problem file. The tray never writes or deletes these files.
#[derive(Debug)]
pub struct HookedSource {
    agent: &'static Agent,
    dir: PathBuf,
    files: BTreeMap<String, Slot>,
}

/// One file. `entry` survives a bad read until its retry, so a retry keeps the session it named.
#[derive(Debug, Default)]
struct Slot {
    entry: Option<Entry>,
    retry_pending: bool,
}

#[derive(Debug)]
enum Entry {
    Chat(StateFile),
    Problem(ProblemFile),
}

#[derive(Clone, Copy)]
enum Kind {
    Chat,
    Problem,
}

/// `write_atomic`'s temp names end in `.tmp`, so they are never read.
fn is_state_file_name(name: &str) -> bool {
    name.ends_with(".json")
}

impl HookedSource {
    /// `data_root` is Wakeshot's data folder, the one that holds `agents`.
    pub fn new(agent: &'static Agent, data_root: &Path) -> Self {
        Self {
            agent,
            dir: agent_folder(data_root, agent),
            files: BTreeMap::new(),
        }
    }

    /// The chat file read last under this chat id, for a source that adds its own rules to the shared ones.
    pub fn chat(&self, id: &str) -> Option<&StateFile> {
        match self.files.get(id)?.entry.as_ref()? {
            Entry::Chat(file) => Some(file),
            Entry::Problem(_) => None,
        }
    }

    /// Every chat file read, by chat id.
    pub fn chats(&self) -> impl Iterator<Item = (&str, &StateFile)> {
        self.files
            .iter()
            .filter_map(|(id, slot)| match slot.entry.as_ref()? {
                Entry::Chat(file) => Some((id.as_str(), file)),
                Entry::Problem(_) => None,
            })
    }

    /// The session id and shape of a file in the agent's folder: a lowercase chat id, or a problem file; any other
    /// name is never parsed.
    fn kind(&self, file: &FileId) -> Option<(String, Kind)> {
        if file.0.parent() != Some(self.dir.as_path()) {
            return None;
        }
        let name = file.0.file_name()?.to_str()?;
        let stem = name.strip_suffix(".json")?;
        if name == PROBLEM_FILE || name == BLOCKED_FILE {
            return Some((stem.to_owned(), Kind::Problem));
        }
        (chat_id(stem).as_deref() == Some(stem)).then(|| (stem.to_owned(), Kind::Chat))
    }

    /// One of the source's fixed labels; file text never reaches the tooltip.
    fn label(&self, value: &str) -> &'static str {
        self.agent
            .images
            .iter()
            .map(|rule| rule.label)
            .find(|label| *label == value)
            .unwrap_or(self.agent.label)
    }

    fn facts(&self, id: &str, entry: &Entry) -> SessionFacts {
        let key = SessionKey {
            source: self.agent.source,
            id: id.to_owned(),
        };
        match entry {
            Entry::Chat(file) => {
                let (activity, open_tasks) = match file.state {
                    State::Working => (Activity::Working, 0),
                    State::Background => (Activity::Idle, 1),
                };
                SessionFacts {
                    key,
                    liveness: Liveness::Process {
                        pid: file.pid,
                        start: file.proc_start,
                    },
                    activity,
                    timers: Vec::new(),
                    open_tasks,
                    label: self.label(&file.label),
                    format: FormatHealth::Ok,
                }
            }
            Entry::Problem(file) => SessionFacts {
                key,
                liveness: Liveness::Process {
                    pid: file.pid,
                    start: file.proc_start,
                },
                activity: Activity::Idle,
                timers: Vec::new(),
                open_tasks: 0,
                label: self.agent.label,
                format: FormatHealth::Unreadable(match file.problem {
                    Problem::Input => self.agent.input_problem,
                    Problem::CommandLine => self.agent.blocked_problem,
                }),
            },
        }
    }
}

impl AgentSource for HookedSource {
    fn id(&self) -> SourceId {
        self.agent.source
    }

    fn wanted(&self) -> Vec<WantedFile> {
        vec![WantedFile::Folder {
            dir: self.dir.clone(),
            accept: is_state_file_name,
        }]
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        let Some((id, kind)) = self.kind(file) else {
            return Fed::Taken;
        };
        match data {
            Feed::Whole(bytes) => {
                let read = match kind {
                    Kind::Chat => read_state(&bytes).map(Entry::Chat),
                    Kind::Problem => read_problem(&bytes).map(Entry::Problem),
                };
                match read {
                    FileRead::V1(entry) => {
                        self.files.insert(
                            id,
                            Slot {
                                entry: Some(entry),
                                retry_pending: false,
                            },
                        );
                    }
                    // A newer Wakeshot wrote it (an update between a hook run and the tray's restart): the tray that
                    // reads it next knows it.
                    FileRead::Newer(_) => {
                        self.files.remove(&id);
                    }
                    FileRead::Bad => {
                        let slot = self.files.entry(id.clone()).or_default();
                        if !slot.retry_pending {
                            slot.retry_pending = true;
                            return Fed::RetryOnce;
                        }
                        // The hook writes whole files only, so a second bad read is not a write in progress: no signal.
                        self.files.remove(&id);
                    }
                }
            }
            Feed::Gone => {
                self.files.remove(&id);
            }
            // A folder file comes whole or gone.
            Feed::Appended(_) | Feed::Present => {}
        }
        Fed::Taken
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.files
            .iter()
            .filter_map(|(id, slot)| slot.entry.as_ref().map(|entry| self.facts(id, entry)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table with every action, so the shared rules are tested apart from any one agent.
    static TEST_AGENT: Agent = Agent {
        source: SourceId::Antigravity,
        name: "test",
        label: "Test",
        fields: InputFields {
            chat: "chat",
            turn: Some("turn"),
            event: Some("event"),
            version: Some("version"),
            idle: Some("idle"),
        },
        events: &[
            EventRule {
                name: "start",
                action: Action::Write(State::Working),
                prune: true,
            },
            EventRule {
                name: "end",
                action: Action::DeleteIfTurn,
                prune: false,
            },
            EventRule {
                name: "gone",
                action: Action::Delete,
                prune: false,
            },
            EventRule {
                name: "stop",
                action: Action::StopIfIdle,
                prune: false,
            },
            EventRule {
                name: "close",
                action: Action::EndUnlessRunning(Running {
                    field: "status",
                    value: "running",
                }),
                prune: false,
            },
        ],
        images: &[
            ImageRule {
                image: "Agent.exe",
                under: None,
                key: "app",
                label: "Test",
            },
            ImageRule {
                image: "node.exe",
                under: Some(Under {
                    base: Base::LocalAppData,
                    parts: &["test-agent"],
                }),
                key: "cli",
                label: "Test CLI",
            },
        ],
        payload_key: "chat",
        input_problem: "Test hook input",
        blocked_problem: "Test hook command line",
    };

    const CHAT: &str = "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3";

    fn rule(name: &str) -> &'static EventRule {
        TEST_AGENT.event(name).unwrap()
    }

    fn parse(event: &str, text: &str) -> Parsed {
        parse_input(&TEST_AGENT, event, text.as_bytes())
    }

    fn input(event: &str, text: &str) -> Result<HookInput, InputProblem> {
        parse(event, text).input
    }

    fn problem_class(event: &str, text: &str) -> &'static str {
        match input(event, text) {
            Err(InputProblem::Parse { class, .. }) => class,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_kept_fields_come_back_checked_and_the_rest_is_skipped() {
        let text = format!(
            r#"{{"prompt":"hello","chat":"{}","turn":"t-1_x","event":"start","version":"3.27.12",
            "attachments":[{{"path":"C:\\x"}}],"user_email":"a@b.c","n":1.5,"z":null,"deep":{{"a":[1,{{"b":2}}]}}}}"#,
            CHAT.to_uppercase()
        );
        let parsed = parse("start", &text);
        assert_eq!(
            parsed,
            Parsed {
                input: Ok(HookInput {
                    chat: CHAT.to_owned(),
                    turn: "t-1_x".to_owned(),
                    idle: None,
                }),
                version: Some("3.27.12".to_owned()),
            }
        );
    }

    #[test]
    fn a_byte_order_mark_is_skipped_and_short_inputs_parse() {
        let mut text = UTF8_BOM.to_vec();
        text.extend_from_slice(format!(r#"{{"chat":"{CHAT}"}}"#).as_bytes());
        let parsed = parse_input(&TEST_AGENT, "start", &text[..]);
        assert_eq!(parsed.input.unwrap().chat, CHAT);
        assert_eq!(problem_class("start", ""), "eof");
        assert_eq!(problem_class("start", "{"), "eof");
        assert_eq!(problem_class("start", "{}x"), "syntax");
        // A mark in the middle is not skipped.
        let mut late = b" ".to_vec();
        late.extend_from_slice(&UTF8_BOM);
        late.extend_from_slice(b"{}");
        assert!(parse_input(&TEST_AGENT, "start", &late[..]).input.is_err());
    }

    #[test]
    fn the_input_must_be_one_object_with_string_fields() {
        for text in [
            "[]",
            "\"chat\"",
            "12",
            "null",
            r#"{"chat":12}"#,
            r#"{"chat":{"a":1}}"#,
            &format!(r#"{{"chat":["{CHAT}"]}}"#),
            &format!(r#"{{"chat":"{CHAT}","turn":5}}"#),
            &format!(r#"{{"chat":"{CHAT}","idle":"yes"}}"#),
            &format!(r#"{{"chat":"{CHAT}","chat":"{CHAT}"}}"#),
        ] {
            assert_eq!(problem_class("start", text), "data", "{text}");
        }
        // Null counts as missing.
        assert_eq!(
            input("start", r#"{"chat":null}"#),
            Err(InputProblem::NoChat)
        );
        assert_eq!(input("start", r#"{"turn":"t"}"#), Err(InputProblem::NoChat));
    }

    #[test]
    fn a_parse_problem_keeps_its_place_and_no_text() {
        let Err(InputProblem::Parse {
            class,
            line,
            column,
        }) = input("start", "{\n  \"chat\": 7\n}")
        else {
            panic!("a number is no chat id");
        };
        // serde_json names the place it stopped at, past the value.
        assert_eq!(class, "data");
        assert!(line >= 2, "{line}");
        let text = InputProblem::Parse {
            class,
            line,
            column,
        }
        .to_string();
        assert_eq!(
            text,
            format!("the input does not parse (data at line {line}, column {column})")
        );
    }

    #[test]
    fn the_chat_id_has_the_uuid_shape_and_is_lowercased() {
        assert_eq!(chat_id(CHAT).as_deref(), Some(CHAT));
        assert_eq!(chat_id(&CHAT.to_uppercase()).as_deref(), Some(CHAT));
        for bad in [
            "",
            "CON",
            "COM1",
            "nul.txt",
            r"..\..\x",
            "../../x",
            r"0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b\",
            "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3.json",
            "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b",
            "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b33",
            "0b9e6b0e3c1a-4f7e-9a51-2d64c1f0a7b3-",
            "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7g3",
            "0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b ",
            "\u{0660}b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b",
            &"a".repeat(300),
            &"0b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3".repeat(10),
        ] {
            assert_eq!(chat_id(bad), None, "{bad}");
            let text = serde_json::json!({ "chat": bad }).to_string();
            assert_eq!(input("start", &text), Err(InputProblem::ChatShape), "{bad}");
        }
    }

    #[test]
    fn the_turn_id_is_short_and_plain_or_empty() {
        assert!(is_turn_id(""));
        assert!(is_turn_id("8f0c-AZ_09"));
        assert!(is_turn_id(&"a".repeat(80)));
        for bad in [&"a".repeat(81), "a b", "a.b", "a/b", r"a\b", "ё", "a\0"] {
            assert!(!is_turn_id(bad), "{bad}");
            let text = serde_json::json!({ "chat": CHAT, "turn": bad }).to_string();
            assert_eq!(input("start", &text), Err(InputProblem::TurnShape), "{bad}");
        }
        let text = serde_json::json!({ "chat": CHAT }).to_string();
        assert_eq!(input("start", &text).unwrap().turn, "");
    }

    #[test]
    fn the_named_event_must_be_the_commands() {
        let text = serde_json::json!({ "chat": CHAT, "event": "end" }).to_string();
        assert_eq!(input("start", &text), Err(InputProblem::OtherEvent));
        assert!(input("end", &text).is_ok());
        let text = serde_json::json!({ "chat": CHAT, "event": "Start" }).to_string();
        assert_eq!(input("start", &text), Err(InputProblem::OtherEvent));
    }

    #[test]
    fn only_a_version_shaped_value_reaches_a_log_line() {
        assert_eq!(log_version("3.27.12").as_deref(), Some("3.27.12"));
        assert_eq!(
            log_version("2026.11.05-86047df").as_deref(),
            Some("2026.11.05-86047df")
        );
        assert_eq!(log_version(&"1".repeat(40)).unwrap().len(), 32);
        for bad in [
            "",
            "v3",
            "beta",
            "3.27 12",
            "3/27",
            "QXV7 marker",
            "3.27.12\n",
        ] {
            assert_eq!(log_version(bad), None, "{bad:?}");
        }
        // Cut first: a long value that only goes bad after 32 characters still logs its head.
        assert_eq!(
            log_version(&format!("{}!", "9".repeat(32))).as_deref(),
            Some("9".repeat(32).as_str())
        );
    }

    #[test]
    fn each_event_resolves_to_its_op() {
        let mut input = HookInput {
            chat: CHAT.to_owned(),
            turn: String::new(),
            idle: None,
        };
        assert_eq!(
            resolve(rule("start"), &input),
            Ok(Op::Write(State::Working))
        );
        assert_eq!(resolve(rule("end"), &input), Ok(Op::DeleteIfTurn));
        assert_eq!(resolve(rule("gone"), &input), Ok(Op::Delete));
        assert_eq!(resolve(rule("stop"), &input), Err(InputProblem::NoIdle));
        input.idle = Some(true);
        assert_eq!(resolve(rule("stop"), &input), Ok(Op::Delete));
        input.idle = Some(false);
        assert_eq!(
            resolve(rule("stop"), &input),
            Ok(Op::Write(State::Background))
        );
    }

    #[test]
    fn an_end_that_names_a_running_turn_keeps_the_chat_working() {
        let op = |text: String| {
            let input = input("close", &text).expect("readable");
            resolve(rule("close"), &input)
        };
        let with = |status: &str| format!(r#"{{"chat":"{CHAT}","turn":"t1","status":{status}}}"#);
        assert_eq!(op(with(r#""running""#)), Ok(Op::Write(State::Working)));
        // Any other value or type, or no field, ends the chat as a plain end does; none is a parse problem.
        for other in [
            r#""done""#,
            r#""Running""#,
            r#""running ""#,
            r#""""#,
            "null",
            "true",
            "7",
            "1.5",
            r#"["running"]"#,
            r#"{"status":"running"}"#,
        ] {
            assert_eq!(op(with(other)), Ok(Op::Delete), "{other}");
        }
        assert_eq!(
            op(format!(r#"{{"chat":"{CHAT}","turn":"t1"}}"#)),
            Ok(Op::Delete)
        );
        // Only the event with the rule reads the field: for another event it is one more skipped value.
        assert_eq!(input("gone", &with(r#""running""#)).unwrap().idle, None);
        // Two values for it: which one the agent meant is unknown.
        let twice = format!(r#"{{"chat":"{CHAT}","status":"done","status":"running"}}"#);
        assert_eq!(problem_class("close", &twice), "data");
        // Its Medium copy takes only what the event itself could do.
        let mut values = checked(Op::Write(State::Working)).values();
        values[5] = "background".to_owned();
        assert_eq!(Checked::parse(&TEST_AGENT, rule("close"), &values), None);
    }

    fn me() -> ProcessRef {
        ProcessRef {
            pid: 100,
            start: FileTime(5_000),
        }
    }

    fn listed(pid: u32, parent: u32, start: u64, image: &'static str) -> Listed<'static> {
        Listed {
            pid,
            parent,
            start: FileTime(start),
            image,
        }
    }

    fn local() -> PathBuf {
        PathBuf::from("Users")
            .join("u")
            .join("AppData")
            .join("Local")
    }

    fn bases() -> Bases {
        Bases {
            local_app_data: Some(local()),
            profile: None,
        }
    }

    #[test]
    fn the_walk_takes_the_nearest_agent_process_above_the_hook() {
        let list = [
            listed(100, 90, 5_000, "wakeshot.exe"),
            listed(90, 80, 4_000, "powershell.exe"),
            listed(80, 70, 3_000, "agent.EXE"),
            listed(70, 1, 2_000, "Agent.exe"),
        ];
        let walk = walk(&TEST_AGENT, &list, me(), &bases(), |_| None);
        let parent = ProcessRef {
            pid: 90,
            start: FileTime(4_000),
        };
        assert_eq!(walk.parent, Some(parent));
        let found = walk.agent.unwrap();
        assert_eq!(found.process.pid, 80, "the nearest, not the one above it");
        assert_eq!(found.image.key, "app");
    }

    #[test]
    fn a_reused_or_missing_parent_ends_the_walk() {
        // The shell's parent pid now runs a process that started after the shell: not its parent.
        let list = [
            listed(100, 90, 5_000, "wakeshot.exe"),
            listed(90, 80, 4_000, "powershell.exe"),
            listed(80, 70, 4_500, "Agent.exe"),
        ];
        let answer = walk(&TEST_AGENT, &list, me(), &bases(), |_| None);
        assert!(answer.agent.is_none());
        assert_eq!(answer.parent.map(|p| p.pid), Some(90));
        // The hook itself is not listed with its own start: no walk at all.
        let list = [
            listed(100, 80, 9_999, "wakeshot.exe"),
            listed(80, 1, 1, "Agent.exe"),
        ];
        assert_eq!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| None),
            Walk::default()
        );
        // A loop of equal starts never runs past the step limit.
        let list = [
            listed(100, 90, 5_000, "wakeshot.exe"),
            listed(90, 91, 5_000, "a.exe"),
            listed(91, 90, 5_000, "b.exe"),
        ];
        assert!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| None)
                .agent
                .is_none()
        );
    }

    #[test]
    fn the_walk_looks_at_most_ten_parents() {
        let mut list = vec![listed(100, 99, 5_000, "wakeshot.exe")];
        for pid in (89..=99).rev() {
            let image = if pid == 89 { "Agent.exe" } else { "cmd.exe" };
            list.push(listed(pid, pid - 1, u64::from(pid), image));
        }
        // Parents 99 down to 90 are ten; the agent is the eleventh.
        assert!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| None)
                .agent
                .is_none()
        );
        list.retain(|e| e.pid != 90);
        list.iter_mut().find(|e| e.pid == 91).unwrap().parent = 89;
        assert_eq!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| None)
                .agent
                .map(|f| f.process.pid),
            Some(89)
        );
    }

    #[test]
    fn a_node_counts_only_from_the_agents_own_folder() {
        let list = [
            listed(100, 90, 5_000, "wakeshot.exe"),
            listed(90, 80, 4_000, "powershell.exe"),
            listed(80, 70, 3_000, "NODE.exe"),
            listed(70, 1, 2_000, "node.exe"),
        ];
        let agent_node = local()
            .join("Test-Agent")
            .join("versions")
            .join("1")
            .join("node.exe");
        let other_node = PathBuf::from("Program Files")
            .join("nodejs")
            .join("node.exe");
        let mut asked = Vec::new();
        let answer = walk(&TEST_AGENT, &list, me(), &bases(), |p| {
            asked.push(p.pid);
            Some(if p.pid == 80 {
                other_node.clone()
            } else {
                agent_node.clone()
            })
        });
        assert_eq!(asked, [80, 70], "only a rule with a folder asks for a path");
        let found = answer.agent.unwrap();
        assert_eq!((found.process.pid, found.image.label), (70, "Test CLI"));
        // No known base, no path, or a path that climbs out: no match.
        let none = Bases::default();
        assert!(
            walk(&TEST_AGENT, &list, me(), &none, |_| Some(
                agent_node.clone()
            ))
            .agent
            .is_none()
        );
        assert!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| None)
                .agent
                .is_none()
        );
        let out = local()
            .join("test-agent")
            .join("..")
            .join("evil")
            .join("node.exe");
        assert!(
            walk(&TEST_AGENT, &list, me(), &bases(), |_| Some(out.clone()))
                .agent
                .is_none()
        );
        let folder_itself = local().join("test-agent");
        assert!(!is_under(&folder_itself, &folder_itself));
        let twin = local().join("test-agent2").join("node.exe");
        assert!(!is_under(&twin, &local().join("test-agent")));
    }

    #[test]
    fn the_files_read_back_and_a_newer_or_other_shape_is_told_apart() {
        let process = ProcessRef {
            pid: 23_816,
            start: FileTime(134_381_211_391_959_336),
        };
        let text = state_text(process, "8f0c", "Test", State::Working);
        assert_eq!(
            String::from_utf8(text.clone()).unwrap(),
            r#"{"version":1,"pid":23816,"procStart":"134381211391959336","generation":"8f0c","label":"Test","state":"working"}"#
        );
        assert!(names_turn(&text, "8f0c"));
        assert!(!names_turn(&text, "8f0d"));
        assert!(!names_turn(&text, ""));
        assert_eq!(named_process(&text), Some(process));
        let problem = problem_text(process, Problem::CommandLine);
        assert_eq!(
            String::from_utf8(problem.clone()).unwrap(),
            r#"{"version":1,"pid":23816,"procStart":"134381211391959336","problem":"command-line"}"#
        );
        assert_eq!(named_process(&problem), Some(process));
        assert!(!names_turn(&problem, ""));

        let newer = r#"{"version":2,"pid":1,"what":"else"}"#;
        assert_eq!(read_state(newer.as_bytes()), FileRead::Newer(2));
        assert_eq!(named_process(newer.as_bytes()), None);
        for bad in [
            "",
            "[1,1,\"1\",\"g\",\"Test\",\"working\"]",
            r#"{"version":0,"pid":1,"procStart":"1","generation":"","label":"","state":"working"}"#,
            r#"{"version":1,"pid":1,"procStart":1,"generation":"","label":"","state":"working"}"#,
            r#"{"version":1,"pid":1,"procStart":"1","generation":"","label":"","state":"waiting"}"#,
            r#"{"version":"1"}"#,
            "{",
        ] {
            assert_eq!(read_state(bad.as_bytes()), FileRead::Bad, "{bad}");
            assert_eq!(named_process(bad.as_bytes()), None, "{bad}");
        }
    }

    fn checked(op: Op) -> Checked {
        let (chat, turn) = match op {
            Op::Problem(_) => (String::new(), String::new()),
            _ => (CHAT.to_owned(), "t1".to_owned()),
        };
        Checked {
            chat,
            turn,
            process: ProcessRef {
                pid: 4_242,
                start: FileTime(134_000_000_000_000_001),
            },
            image: &TEST_AGENT.images[1],
            op,
            deadline_wall_ms: 1_762_400_000_000,
            log: None,
        }
    }

    fn mark() -> LogMark {
        LogMark {
            name: "cli-20261106_185448.log".to_owned(),
            turn_at: FileTime(134_000_000_000_000_777),
        }
    }

    #[test]
    fn a_log_mark_goes_with_a_write_only_and_its_name_is_checked() {
        let mut sent = checked(Op::Write(State::Working));
        sent.log = Some(mark());
        let values = sent.values();
        assert_eq!(values.len(), CHECKED_VALUES + CHECKED_LOG_VALUES);
        assert_eq!(
            values[CHECKED_VALUES..],
            ["cli-20261106_185448.log", "134000000000000777"]
        );
        assert_eq!(
            Checked::parse(&TEST_AGENT, rule("start"), &values),
            Some(sent)
        );
        let long = format!("{}.log", "a".repeat(61));
        for (at, value) in [
            (7, ""),
            (7, r"..\x.log"),
            (7, "x/y.log"),
            (7, r"C:\x.log"),
            (7, ".log"),
            (7, ".x.log"),
            (7, "a b.log"),
            (7, "cli.txt"),
            (7, "CON"),
            (7, long.as_str()),
            (8, ""),
            (8, "-1"),
            (8, "0x10"),
        ] {
            let mut bad = values.clone();
            bad[at] = value.to_owned();
            assert_eq!(
                Checked::parse(&TEST_AGENT, rule("start"), &bad),
                None,
                "value {at} = {value:?}"
            );
        }
        // A delete or a problem carries no log; nor does a list one value short of it.
        let mut delete = values.clone();
        delete[5] = "none".to_owned();
        assert_eq!(Checked::parse(&TEST_AGENT, rule("gone"), &delete), None);
        let mut problem = values.clone();
        problem[5] = "input".to_owned();
        problem[0].clear();
        problem[1].clear();
        assert_eq!(Checked::parse(&TEST_AGENT, rule("start"), &problem), None);
        assert_eq!(
            Checked::parse(&TEST_AGENT, rule("start"), &values[..8]),
            None
        );
    }

    #[test]
    fn a_marked_file_names_its_log_and_an_unmarked_one_keeps_its_bytes() {
        let process = ProcessRef {
            pid: 51_384,
            start: FileTime(134_000_000_000_000_001),
        };
        let text = state_text_marked(process, "", "Test CLI", State::Working, Some(&mark()));
        assert_eq!(
            String::from_utf8(text.clone()).unwrap(),
            r#"{"version":1,"pid":51384,"procStart":"134000000000000001","generation":"","label":"Test CLI","state":"working","log":"cli-20261106_185448.log","turnAt":"134000000000000777"}"#
        );
        let FileRead::V1(file) = read_state(&text) else {
            panic!("reads back");
        };
        assert_eq!(
            (file.log.as_deref(), file.turn_at),
            (
                Some("cli-20261106_185448.log"),
                Some(FileTime(134_000_000_000_000_777))
            )
        );
        assert_eq!(
            state_text_marked(process, "t", "Test", State::Working, None),
            state_text(process, "t", "Test", State::Working)
        );
        let FileRead::V1(plain) = read_state(&state_text(process, "t", "Test", State::Working))
        else {
            panic!("reads back");
        };
        assert_eq!((plain.log, plain.turn_at), (None, None));
    }

    #[test]
    fn checked_values_go_and_come_back_whole() {
        for (event, op) in [
            ("start", Op::Write(State::Working)),
            ("end", Op::DeleteIfTurn),
            ("gone", Op::Delete),
            ("stop", Op::Delete),
            ("stop", Op::Write(State::Background)),
            ("close", Op::Delete),
            ("close", Op::Write(State::Working)),
            ("start", Op::Problem(Problem::Input)),
            ("end", Op::Problem(Problem::CommandLine)),
        ] {
            let sent = checked(op);
            let values = sent.values();
            assert_eq!(values.len(), CHECKED_VALUES);
            assert_eq!(
                Checked::parse(&TEST_AGENT, rule(event), &values),
                Some(sent),
                "{event}"
            );
        }
        assert_eq!(
            checked(Op::Write(State::Working)).values(),
            [
                CHAT,
                "t1",
                "4242",
                "134000000000000001",
                "cli",
                "working",
                "1762400000000"
            ]
        );
    }

    #[test]
    fn a_checked_value_that_does_not_fit_gives_nothing() {
        let good = checked(Op::Write(State::Working)).values();
        let start = rule("start");
        let bad = |at: usize, value: &str| {
            let mut values = good.clone();
            values[at] = value.to_owned();
            Checked::parse(&TEST_AGENT, start, &values)
        };
        for (at, value) in [
            (0, r"..\..\x"),
            (0, "CON"),
            (0, &CHAT.to_uppercase()),
            (0, ""),
            (1, "a b"),
            (1, &"a".repeat(81)),
            (2, "-1"),
            (2, "+5"),
            (2, "4294967296"),
            (2, ""),
            (3, "0x10"),
            (3, "1 2"),
            (4, "Agent.exe"),
            (4, "CLI"),
            (5, "background"),
            (5, "none"),
            (5, "Working"),
            (6, "-5"),
            (6, "99999999999999999999999"),
        ] {
            assert_eq!(bad(at, value), None, "value {at} = {value:?}");
        }
        assert_eq!(Checked::parse(&TEST_AGENT, start, &good[..6]), None);
        let mut longer = good.clone();
        longer.push("x".to_owned());
        assert_eq!(Checked::parse(&TEST_AGENT, start, &longer), None);
        // An end event never writes "working"; a problem carries no ids.
        let mut values = good.clone();
        assert_eq!(Checked::parse(&TEST_AGENT, rule("end"), &values), None);
        values[5] = "input".to_owned();
        assert_eq!(Checked::parse(&TEST_AGENT, start, &values), None);
        values[0].clear();
        values[1].clear();
        assert!(Checked::parse(&TEST_AGENT, start, &values).is_some());
    }

    fn data_root() -> PathBuf {
        PathBuf::from("data")
    }

    fn file_id(name: &str) -> FileId {
        FileId(agent_folder(&data_root(), &TEST_AGENT).join(name))
    }

    fn source() -> HookedSource {
        HookedSource::new(&TEST_AGENT, &data_root())
    }

    fn whole(text: &[u8]) -> Feed {
        Feed::Whole(text.to_vec())
    }

    #[test]
    fn the_source_watches_its_agents_folder_for_json_names_only() {
        let source = source();
        assert_eq!(source.id(), SourceId::Antigravity);
        let wanted = source.wanted();
        let [WantedFile::Folder { dir, accept }] = &wanted[..] else {
            panic!("one folder");
        };
        assert_eq!(dir, &data_root().join("agents").join("test"));
        assert!(accept(&format!("{CHAT}.json")));
        assert!(accept(PROBLEM_FILE));
        assert!(!accept(&format!("{CHAT}.json.4242.0.0123456789abcdef.tmp")));
        assert!(!accept("x.txt"));
    }

    #[test]
    fn each_chat_file_is_one_session_of_its_process() {
        let mut source = source();
        let process = ProcessRef {
            pid: 7,
            start: FileTime(77),
        };
        let other = "1b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3";
        assert_eq!(
            source.feed(
                &file_id(&chat_file(CHAT)),
                whole(&state_text(process, "t", "Test CLI", State::Working))
            ),
            Fed::Taken
        );
        source.feed(
            &file_id(&chat_file(other)),
            whole(&state_text(
                process,
                "",
                "Forged <b>label</b>",
                State::Background,
            )),
        );
        let sessions = source.sessions();
        assert_eq!(sessions.len(), 2);
        assert_eq!(
            sessions[0],
            SessionFacts {
                key: SessionKey {
                    source: SourceId::Antigravity,
                    id: CHAT.to_owned(),
                },
                liveness: Liveness::Process {
                    pid: 7,
                    start: FileTime(77)
                },
                activity: Activity::Working,
                timers: Vec::new(),
                open_tasks: 0,
                label: "Test CLI",
                format: FormatHealth::Ok,
            }
        );
        assert_eq!(sessions[1].activity, Activity::Idle);
        assert_eq!(sessions[1].open_tasks, 1);
        assert_eq!(
            sessions[1].label, "Test",
            "file text never reaches the tooltip"
        );
        source.feed(&file_id(&chat_file(CHAT)), Feed::Gone);
        assert_eq!(source.sessions().len(), 1);
    }

    #[test]
    fn problem_files_show_update_wakeshot_while_their_process_lives() {
        let mut source = source();
        let process = ProcessRef {
            pid: 9,
            start: FileTime(99),
        };
        source.feed(
            &file_id(PROBLEM_FILE),
            whole(&problem_text(process, Problem::Input)),
        );
        source.feed(
            &file_id(BLOCKED_FILE),
            whole(&problem_text(process, Problem::CommandLine)),
        );
        let sessions = source.sessions();
        let formats: Vec<_> = sessions
            .iter()
            .map(|s| (s.key.id.as_str(), s.format.clone()))
            .collect();
        assert_eq!(
            formats,
            [
                (
                    "blocked",
                    FormatHealth::Unreadable("Test hook command line")
                ),
                ("problem", FormatHealth::Unreadable("Test hook input")),
            ]
        );
        assert!(sessions.iter().all(|s| s.activity == Activity::Idle
            && s.liveness
                == Liveness::Process {
                    pid: 9,
                    start: FileTime(99)
                }));
    }

    #[test]
    fn a_bad_file_is_retried_once_then_gives_no_signal_and_a_newer_one_is_ignored() {
        let mut source = source();
        let id = file_id(&chat_file(CHAT));
        let good = state_text(
            ProcessRef {
                pid: 1,
                start: FileTime(1),
            },
            "",
            "Test",
            State::Working,
        );
        source.feed(&id, whole(&good));
        assert_eq!(source.feed(&id, whole(b"{")), Fed::RetryOnce);
        assert_eq!(source.sessions().len(), 1, "kept until the retry");
        assert_eq!(source.feed(&id, whole(b"{")), Fed::Taken);
        assert!(source.sessions().is_empty());
        // A good read after a retry clears the retry.
        assert_eq!(source.feed(&id, whole(b"{")), Fed::RetryOnce);
        source.feed(&id, whole(&good));
        assert_eq!(source.feed(&id, whole(b"{")), Fed::RetryOnce);
        source.feed(&id, whole(br#"{"version":2}"#));
        assert!(source.sessions().is_empty());
    }

    #[test]
    fn files_of_other_names_or_folders_are_never_parsed() {
        let mut source = source();
        let good = state_text(
            ProcessRef {
                pid: 1,
                start: FileTime(1),
            },
            "",
            "Test",
            State::Working,
        );
        for id in [
            file_id(&chat_file(&CHAT.to_uppercase())),
            file_id("con.json"),
            file_id("notes.json"),
            file_id(CHAT),
            FileId(
                data_root()
                    .join("agents")
                    .join("cursor")
                    .join(chat_file(CHAT)),
            ),
            FileId(data_root().join(chat_file(CHAT))),
        ] {
            assert_eq!(source.feed(&id, whole(b"{")), Fed::Taken, "{id:?}");
            source.feed(&id, whole(&good));
        }
        assert!(source.sessions().is_empty());
    }
}
