//! Copilot CLI sessions: in-use lock files and each session's event log (architecture section 7).
//!
//! Built from Copilot CLI 1.0.106's own schema (`copilot-sdk/generated/session-events.d.ts`) and strings in its
//! bundle, read as data, then checked on a real 1.0.106 install (2026-11-06). Rules still marked "not verified" were
//! not seen live.
//!
//! - A session is a folder `<home>\session-state\<id>\` with in-use files in it: the process that has the session
//!   open makes `inuse.<pid>.hold` and `inuse.<pid>.lock`, names both after its own pid, keeps the `.hold` open (the
//!   `.lock`, a few bytes, is open by nobody), and deletes both at a clean exit; a crash leaves both, until
//!   the next Copilot start deletes them (all seen live). So the session's liveness names the `.hold` file, the one
//!   Restart Manager ties to the holder; a `.lock` stands in only while no `.hold` is listed. Both are listed, never
//!   opened. The holder is the CLI's `copilot.exe`, or `copilot-runtime.exe` for VS Code's Copilot harness (seen
//!   live), or another host; the watch takes the holder whose pid the name gives, whatever its image. With no in-use
//!   file left, the session is gone.
//! - Activity is the last relevant event of `events.jsonl` (`docs/research/other-agent-signals.md`, "GitHub Copilot
//!   CLI"). Copilot persists no idle event (`session.idle` is ephemeral), so idle is read from the turn's end. Waiting
//!   for approval never holds, so it is reported as waiting with `remote: false`.
//! - Scheduled prompts (`/every`, `/after`, the `manage_schedule` tool) are timers. A fire is a `user.message` whose
//!   `source` is `schedule-<id>` (seen live: `/every 2m` made schedule 1, and its fire's `source` was `schedule-1`); a
//!   fire that names no known id moves every interval schedule that was due by then. Interval and cron
//!   times also move past the newest event time in the file: an event proves the process was alive then, so a fire
//!   this source failed to see cannot leave a due time in the past.
//! - The log rotates (1.0.106): the old file becomes `events.<n>.jsonl` and a new `events.jsonl` starts with a
//!   `session.journal_link` record, then a replay of earlier records marked `checkpoint: true`. The watch thread feeds
//!   that as `Gone`, then the new file from byte 0. What a rotation knew stays; a file that starts any other way (a
//!   rewind truncates it) is replayed from scratch. Checkpoint records only add schedules; whether they carry every
//!   live schedule is not verified.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Number;

use crate::holds::file::parse_expires;
use crate::sources::cron::{Cron, LocalOffset};
use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

/// What the tooltip names a Copilot session.
pub const LABEL: &str = "Copilot";

/// The process that runs the CLI's own sessions. VS Code's Copilot harness hosts its sessions in
/// `copilot-runtime.exe`, with the same in-use files named after that process (seen live); the watch counts such a
/// holder by the pid in the file name, not by this image.
pub const IMAGE: &str = "copilot.exe";

/// Each session's event log, in its folder.
const EVENTS_FILE: &str = "events.jsonl";

/// `FormatHealth` field names: an event that does not read (the value is its type, or `line` when it is not an event
/// at all), a cron text `Cron::parse` does not know, and an interval or a time that is not a number of ms.
pub const EVENT: &str = "event";
pub const CRON: &str = "cron";
const INTERVAL: &str = "intervalMs";
pub const TIMESTAMP: &str = "timestamp";

/// `FormatHealth::Unreadable` text, for a log of which no line reads.
pub const UNREADABLE: &str = "events file";

const STATE_DIR: &str = "session-state";
const LOCK_PREFIX: &str = "inuse.";
const LOCK_SUFFIX: &str = ".lock";
/// The in-use file the session's process keeps open.
const HOLD_SUFFIX: &str = ".hold";
const JOURNAL_LINK: &str = "session.journal_link";
/// Copilot's built-in tool that asks the user a question; its `user_input.requested` is ephemeral, so the tool's
/// start is the only trace of the wait.
const ASK_USER: &str = "ask_user";
const SCHEDULE_SOURCE: &str = "schedule-";

/// A longer line is skipped: no line this source reads comes near it, and the buffer must stay bounded.
const MAX_LINE: usize = 32 << 20;
/// Bounds for state a broken log could grow without end.
const MAX_SCHEDULES: usize = 256;
const MAX_SUBAGENTS: usize = 256;
/// The longest session folder name or tool call id kept. Copilot's are UUIDs and short ids.
const MAX_ID: usize = 128;
/// The longest value a format problem names.
const VALUE_CHARS: usize = 32;
/// A fire that names no schedule moves an interval schedule due up to this much after it: its own timestamp can come
/// a little before the time computed from the creation line's.
const FIRE_SLACK_MS: i64 = 60_000;
/// A one-shot this far behind the newest event time can never hold again (the core holds at most 2 minutes past a due
/// time), so it is dropped.
const SPENT_MS: i64 = 60 * 60 * 1000;

/// Whether a file name in a session folder is an in-use file: `inuse.<something>.lock` or `inuse.<something>.hold`,
/// case ignored as NTFS does. Only names pass through here; the file is never opened.
pub fn is_lock_file_name(name: &str) -> bool {
    let len = name.len();
    len > LOCK_PREFIX.len() + LOCK_SUFFIX.len()
        && name
            .get(..LOCK_PREFIX.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(LOCK_PREFIX))
        && [LOCK_SUFFIX, HOLD_SUFFIX].iter().any(|suffix| {
            name.get(len - suffix.len()..)
                .is_some_and(|s| s.eq_ignore_ascii_case(suffix))
        })
}

/// An in-use file that its process keeps open: the one Restart Manager ties to the session's process.
fn is_hold_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(&HOLD_SUFFIX[1..]))
}

/// A session folder name as Copilot makes them (UUIDs). Dot folders such as `.session-operation-locks` are not
/// sessions, and a name that could hold other text is never kept as a key.
fn is_session_id(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_ID
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Copilot CLI's sessions, one per folder in `<home>\session-state` that holds an in-use file, each with the
/// `events.jsonl` in that folder.
///
/// The watch thread is expected to read a `Tail` from its start when it enters `wanted()`, keep its place while it
/// stays, and forget it when it leaves; a session whose lock comes back is then replayed from byte 0.
pub struct CopilotSource {
    state_dir: PathBuf,
    /// The PC's zone, for cron times.
    zone: Box<dyn LocalOffset + Send>,
    sessions: BTreeMap<String, Session>,
}

#[derive(Debug, Default)]
struct Session {
    locks: BTreeSet<PathBuf>,
    log: Log,
}

impl CopilotSource {
    /// `home` is Copilot's own folder: `COPILOT_HOME` when set, else `%USERPROFILE%\.copilot`, both read by the caller
    /// so the core stays free of environment reads. `zone` gives the PC's local time.
    pub fn new(home: PathBuf, zone: Box<dyn LocalOffset + Send>) -> Self {
        Self {
            state_dir: home.join(STATE_DIR),
            zone,
            sessions: BTreeMap::new(),
        }
    }

    /// The session a lock file belongs to; `None` for any other file, which is then never taken as a lock.
    fn lock_session<'a>(&self, file: &'a FileId) -> Option<&'a str> {
        let folder = file.0.parent()?;
        if folder.parent()? != self.state_dir || !is_lock_file_name(file.0.file_name()?.to_str()?) {
            return None;
        }
        let id = folder.file_name()?.to_str()?;
        is_session_id(id).then_some(id)
    }

    /// The session whose `events.jsonl` this is.
    fn events_session<'a>(&self, file: &'a FileId) -> Option<&'a str> {
        let folder = file.0.parent()?;
        if folder.parent()? != self.state_dir || file.0.file_name()? != EVENTS_FILE {
            return None;
        }
        let id = folder.file_name()?.to_str()?;
        is_session_id(id).then_some(id)
    }
}

impl fmt::Debug for CopilotSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotSource")
            .field("state_dir", &self.state_dir)
            .field("sessions", &self.sessions)
            .finish_non_exhaustive()
    }
}

impl AgentSource for CopilotSource {
    fn id(&self) -> SourceId {
        SourceId::Copilot
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let mut wanted = vec![WantedFile::Names {
            dir: self.state_dir.clone(),
            depth: 1,
            accept: is_lock_file_name,
        }];
        wanted.extend(
            self.sessions
                .keys()
                .map(|id| WantedFile::Tail(self.state_dir.join(id).join(EVENTS_FILE))),
        );
        wanted
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        if let Some(id) = self.lock_session(file) {
            match data {
                Feed::Present => {
                    self.sessions
                        .entry(id.to_owned())
                        .or_default()
                        .locks
                        .insert(file.0.clone());
                }
                Feed::Gone => {
                    if let Some(session) = self.sessions.get_mut(id) {
                        session.locks.remove(&file.0);
                        if session.locks.is_empty() {
                            self.sessions.remove(id);
                        }
                    }
                }
                // A lock is listed, never read, so bytes for one say nothing.
                Feed::Whole(_) | Feed::Appended(_) => {}
            }
            return Fed::Taken;
        }
        if let Some(id) = self.events_session(file)
            && let Some(session) = self.sessions.get_mut(id)
        {
            match data {
                Feed::Appended(bytes) => session.log.feed(&bytes),
                Feed::Gone => session.log.restart(),
                Feed::Whole(_) | Feed::Present => {}
            }
        }
        Fed::Taken
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.sessions
            .iter()
            .filter_map(|(id, session)| session.facts(id, &*self.zone))
            .collect()
    }
}

impl Session {
    fn facts(&self, id: &str, zone: &dyn LocalOffset) -> Option<SessionFacts> {
        // Several in-use files (a second client on one session) all name the same session; the first held one stands
        // for them, since a `.lock` has no holder for Restart Manager to name.
        let lock_file = self
            .locks
            .iter()
            .find(|path| is_hold_file(path))
            .or_else(|| self.locks.iter().next())?
            .clone();
        let replay = &self.log.replay;
        let activity = replay.activity();
        Some(SessionFacts {
            key: SessionKey {
                source: SourceId::Copilot,
                id: id.to_owned(),
            },
            liveness: Liveness::AnyProcess {
                image: IMAGE,
                lock_file,
            },
            activity,
            timers: replay.timers(zone),
            open_tasks: 0,
            label: LABEL,
            format: replay.format(activity),
        })
    }
}

/// One `events.jsonl` as it is appended. A line is read once its newline has come, so any split of the file gives the
/// same state.
#[derive(Default)]
struct Log {
    partial: Vec<u8>,
    skipping: bool,
    /// The file was fed `Gone`: its next line tells a rotation (`session.journal_link` first) from a new start.
    restarted: bool,
    replay: Replay,
}

impl fmt::Debug for Log {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A partial line can hold prompt text, so only its length is shown.
        f.debug_struct("Log")
            .field("partial_bytes", &self.partial.len())
            .field("skipping", &self.skipping)
            .field("restarted", &self.restarted)
            .field("replay", &self.replay)
            .finish()
    }
}

impl Log {
    fn feed(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while let Some(end) = rest.iter().position(|&b| b == b'\n') {
            let line = &rest[..end];
            rest = &rest[end + 1..];
            if std::mem::take(&mut self.skipping) {
                continue;
            }
            if self.partial.is_empty() {
                self.line(line);
            } else {
                let mut whole = std::mem::take(&mut self.partial);
                whole.extend_from_slice(line);
                self.line(&whole);
            }
        }
        if self.skipping {
            return;
        }
        if self.partial.len() + rest.len() > MAX_LINE {
            self.partial = Vec::new();
            self.skipping = true;
        } else {
            self.partial.extend_from_slice(rest);
        }
    }

    fn restart(&mut self) {
        self.partial = Vec::new();
        self.skipping = false;
        self.restarted = true;
    }

    fn line(&mut self, line: &[u8]) {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let envelope = serde_json::from_slice::<Envelope>(line).ok();
        if std::mem::take(&mut self.restarted)
            && envelope.as_ref().is_none_or(|e| e.kind != JOURNAL_LINK)
        {
            self.replay = Replay::default();
        }
        match envelope {
            Some(envelope) => self.replay.take(&envelope, line),
            None => self.replay.note(EVENT, "line"),
        }
    }
}

/// The fields every event has, as far as this source reads them; `data` is read again only for the kinds that need
/// it, so the text in other lines is never kept.
#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(rename = "type", borrow)]
    kind: Cow<'a, str>,
    #[serde(borrow)]
    timestamp: Cow<'a, str>,
    /// Present on a sub-agent's own events, absent on the main agent's and the session's.
    #[serde(rename = "agentId", default)]
    agent_id: Option<IgnoredAny>,
    #[serde(default)]
    checkpoint: Option<bool>,
    #[serde(default)]
    ephemeral: Option<bool>,
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

#[derive(Deserialize)]
struct UserMessage<'a> {
    #[serde(borrow, default)]
    source: Option<Cow<'a, str>>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    #[serde(rename = "toolRequests", default)]
    tool_requests: Option<Vec<IgnoredAny>>,
}

#[derive(Deserialize)]
struct ToolStart<'a> {
    #[serde(rename = "toolName", borrow)]
    tool_name: Cow<'a, str>,
}

#[derive(Deserialize)]
struct Subagent<'a> {
    #[serde(rename = "toolCallId", borrow)]
    tool_call_id: Cow<'a, str>,
}

#[derive(Deserialize)]
struct ScheduleCreated<'a> {
    id: u64,
    #[serde(default)]
    at: Option<Number>,
    #[serde(rename = "intervalMs", default)]
    interval_ms: Option<Number>,
    #[serde(borrow, default)]
    cron: Option<Cow<'a, str>>,
    #[serde(default)]
    recurring: Option<bool>,
    #[serde(rename = "selfPaced", default)]
    self_paced: Option<bool>,
}

#[derive(Deserialize)]
struct ScheduleCancelled {
    id: u64,
}

#[derive(Deserialize)]
struct ScheduleRearmed {
    id: u64,
    #[serde(rename = "nextRunAt")]
    next_run_at: Number,
}

/// The event kinds this source reads; every other kind is skipped without a word, since Copilot has well over 100 and
/// adds more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    UserMessage,
    TurnStart,
    TurnEnd,
    AssistantMessage,
    ToolStart,
    ToolComplete,
    PermissionRequested,
    PermissionAnswered,
    SubagentStarted,
    SubagentEnded,
    Abort,
    Error,
    Shutdown,
    Start,
    ScheduleCreated,
    ScheduleCancelled,
    ScheduleRearmed,
    Other,
}

impl Kind {
    fn of(kind: &str) -> Kind {
        match kind {
            "user.message" => Kind::UserMessage,
            "assistant.turn_start" => Kind::TurnStart,
            "assistant.turn_end" => Kind::TurnEnd,
            "assistant.message" => Kind::AssistantMessage,
            "tool.execution_start" => Kind::ToolStart,
            "tool.execution_complete" => Kind::ToolComplete,
            "permission.requested" => Kind::PermissionRequested,
            // An earlier approval carried forward answers the request as a reply would (1.0.106).
            "permission.completed" | "permission.carriedForward" => Kind::PermissionAnswered,
            "subagent.started" => Kind::SubagentStarted,
            // A cancelled sub-agent ends with `subagent.completed` and `cancelled: true`.
            "subagent.completed" | "subagent.failed" => Kind::SubagentEnded,
            "abort" => Kind::Abort,
            "session.error" => Kind::Error,
            "session.shutdown" => Kind::Shutdown,
            "session.start" | "session.resume" => Kind::Start,
            "session.schedule_created" => Kind::ScheduleCreated,
            "session.schedule_cancelled" => Kind::ScheduleCancelled,
            "session.schedule_rearmed" => Kind::ScheduleRearmed,
            _ => Kind::Other,
        }
    }
}

/// What the last relevant event says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Last {
    #[default]
    Idle,
    Working,
    /// Since the event's time, wall ms.
    Waiting(i64),
}

/// When a schedule fires next, as its creation line and later lines say.
#[derive(Clone, Debug)]
enum Timing {
    /// One fixed time; Copilot refuses a recurring one ("A schedule for a fixed time cannot be recurring.").
    At(i64),
    /// Every `every_ms` from `anchor_ms`: the creation, then each fire seen.
    Interval {
        every_ms: i64,
        anchor_ms: i64,
        recurring: bool,
    },
    /// Cron runs after `from_ms`, in the PC's zone whatever `tz` says.
    Cron {
        cron: Cron,
        from_ms: i64,
        recurring: bool,
    },
    /// No cadence: the model arms each run (`session.schedule_rearmed`).
    SelfPaced(Option<i64>),
}

impl Timing {
    /// The next due time in wall ms, without the core's margin. `clock` is the newest event time in the file.
    fn due(&self, clock: Option<i64>, zone: &dyn LocalOffset) -> Option<i64> {
        match *self {
            Timing::At(at) => Some(at),
            Timing::Interval {
                every_ms,
                anchor_ms,
                recurring,
            } => interval_due(every_ms, anchor_ms, recurring, clock),
            Timing::Cron {
                ref cron,
                from_ms,
                recurring,
            } => {
                let basis = if recurring {
                    clock.map_or(from_ms, |c| c.max(from_ms))
                } else {
                    from_ms
                };
                cron.next_after(basis, zone)
            }
            Timing::SelfPaced(next) => next,
        }
    }

    /// The one due time of a one-shot that needs no zone; a one-shot cron is left to [`MAX_SCHEDULES`], since its
    /// time needs the zone and such schedules are rare.
    fn one_shot_due(&self) -> Option<i64> {
        match *self {
            Timing::At(at) => Some(at),
            Timing::Interval {
                every_ms,
                anchor_ms,
                recurring: false,
            } => anchor_ms.checked_add(every_ms),
            _ => None,
        }
    }
}

/// The first run after both `anchor_ms` and `clock` for a recurring interval; the one run for a one-shot.
fn interval_due(every_ms: i64, anchor_ms: i64, recurring: bool, clock: Option<i64>) -> Option<i64> {
    if !recurring {
        return anchor_ms.checked_add(every_ms);
    }
    let basis = clock.map_or(anchor_ms, |c| c.max(anchor_ms));
    let steps = basis.checked_sub(anchor_ms)? / every_ms + 1;
    anchor_ms.checked_add(steps.checked_mul(every_ms)?)
}

/// One session's log, replayed into its activity and its schedules. Keeps ids, times, flags and cron texts only.
#[derive(Debug, Default)]
struct Replay {
    last: Last,
    /// The main agent's last message asked for tools, so its turn's end is not the end of the work.
    tools_follow: bool,
    /// `toolCallId`s of sub-agents started and not yet ended.
    subagents: BTreeSet<String>,
    schedules: BTreeMap<u64, Timing>,
    /// After `session.shutdown`, until the session starts again: its schedules fire only in a running session.
    closed: bool,
    /// The newest event time seen, wall ms: the process was alive then.
    clock_ms: Option<i64>,
    /// Some line read as an event, so a problem is not the whole file.
    read: bool,
    /// The first problem seen: a field name and a value, never line content.
    problem: Option<(&'static str, String)>,
}

impl Replay {
    fn take(&mut self, envelope: &Envelope, line: &[u8]) {
        if envelope.ephemeral == Some(true) {
            return;
        }
        let Some(at) = parse_expires(&envelope.timestamp) else {
            return self.note(TIMESTAMP, &cut(&envelope.timestamp));
        };
        self.read = true;
        let kind = Kind::of(&envelope.kind);
        if envelope.checkpoint == Some(true) {
            // A replay of earlier records at the head of a rotated log: its times and turns are history.
            if matches!(
                kind,
                Kind::ScheduleCreated | Kind::ScheduleCancelled | Kind::ScheduleRearmed
            ) {
                self.schedule(kind, line, at, &envelope.kind);
            }
            return;
        }
        let clock = self.clock_ms;
        self.clock_ms = Some(clock.map_or(at, |c| c.max(at)));
        let main = envelope.agent_id.is_none();
        match kind {
            Kind::UserMessage => {
                let Some(data) = data::<UserMessage>(line) else {
                    return self.note(EVENT, &envelope.kind);
                };
                if let Some(rest) = data
                    .source
                    .as_deref()
                    .and_then(|s| s.strip_prefix(SCHEDULE_SOURCE))
                {
                    self.fired(rest, at, clock);
                }
                self.work(main);
            }
            Kind::TurnStart => self.work(main),
            Kind::AssistantMessage => {
                let Some(data) = data::<AssistantMessage>(line) else {
                    return self.note(EVENT, &envelope.kind);
                };
                let tools = data.tool_requests.is_some_and(|t| !t.is_empty());
                if tools {
                    self.last = Last::Working;
                }
                if main {
                    self.tools_follow = tools;
                }
            }
            // A sub-agent's turn or abort ends only its own work; its `subagent.*` line tells the session's.
            Kind::TurnEnd => {
                if main && !self.tools_follow {
                    self.last = Last::Idle;
                }
            }
            Kind::Abort | Kind::Error => {
                if main {
                    self.last = Last::Idle;
                    self.tools_follow = false;
                }
            }
            Kind::ToolStart => {
                let Some(data) = data::<ToolStart>(line) else {
                    return self.note(EVENT, &envelope.kind);
                };
                self.last = if data.tool_name == ASK_USER {
                    Last::Waiting(at)
                } else {
                    Last::Working
                };
            }
            Kind::ToolComplete | Kind::PermissionAnswered => self.last = Last::Working,
            Kind::PermissionRequested => self.last = Last::Waiting(at),
            Kind::SubagentStarted | Kind::SubagentEnded => {
                let Some(data) = data::<Subagent>(line) else {
                    return self.note(EVENT, &envelope.kind);
                };
                let id = data.tool_call_id;
                if kind == Kind::SubagentEnded {
                    self.subagents.remove(id.as_ref());
                } else {
                    if id.len() <= MAX_ID && self.subagents.len() < MAX_SUBAGENTS {
                        self.subagents.insert(id.into_owned());
                    }
                    self.last = Last::Working;
                }
            }
            Kind::Shutdown => {
                self.rest();
                self.closed = true;
            }
            // A resume may follow a crash that left a turn or a sub-agent open; the new process starts at the prompt.
            Kind::Start => {
                self.rest();
                self.closed = false;
            }
            Kind::ScheduleCreated | Kind::ScheduleCancelled | Kind::ScheduleRearmed => {
                self.schedule(kind, line, at, &envelope.kind);
            }
            Kind::Other => {}
        }
    }

    fn work(&mut self, main: bool) {
        self.last = Last::Working;
        if main {
            self.tools_follow = false;
        }
    }

    fn rest(&mut self) {
        self.last = Last::Idle;
        self.tools_follow = false;
        self.subagents.clear();
    }

    fn schedule(&mut self, kind: Kind, line: &[u8], at: i64, name: &str) {
        match kind {
            Kind::ScheduleCreated => {
                let Some(data) = data::<ScheduleCreated>(line) else {
                    return self.note(EVENT, name);
                };
                if let Some(timing) = self.timing(&data, at) {
                    self.drop_spent();
                    if self.schedules.len() < MAX_SCHEDULES {
                        // Ids are never reused in a session, so a known one is a replay: what was learned since stays.
                        self.schedules.entry(data.id).or_insert(timing);
                    }
                }
            }
            Kind::ScheduleCancelled => {
                let Some(data) = data::<ScheduleCancelled>(line) else {
                    return self.note(EVENT, name);
                };
                self.schedules.remove(&data.id);
            }
            Kind::ScheduleRearmed => {
                let Some(data) = data::<ScheduleRearmed>(line) else {
                    return self.note(EVENT, name);
                };
                let Some(next) = ms(&data.next_run_at) else {
                    return self.note(EVENT, name);
                };
                // A rearm proves a self-paced schedule even when its creation line was lost (a rotation).
                if self.schedules.len() < MAX_SCHEDULES || self.schedules.contains_key(&data.id) {
                    self.schedules
                        .insert(data.id, Timing::SelfPaced(Some(next)));
                }
            }
            _ => {}
        }
    }

    fn timing(&mut self, data: &ScheduleCreated, at: i64) -> Option<Timing> {
        if data.self_paced == Some(true) {
            return Some(Timing::SelfPaced(data.at.as_ref().and_then(ms)));
        }
        if let Some(fixed) = &data.at {
            let Some(fixed) = ms(fixed) else {
                self.note(EVENT, "session.schedule_created");
                return None;
            };
            return Some(Timing::At(fixed));
        }
        if let Some(every) = &data.interval_ms {
            let Some(every_ms) = ms(every).filter(|&ms| ms > 0) else {
                self.note(INTERVAL, &cut(&every.to_string()));
                return None;
            };
            return Some(Timing::Interval {
                every_ms,
                anchor_ms: at,
                recurring: data.recurring.unwrap_or(true),
            });
        }
        if let Some(text) = &data.cron {
            let Some(cron) = Cron::parse(text) else {
                self.note(CRON, &cut(text));
                return None;
            };
            return Some(Timing::Cron {
                cron,
                from_ms: at,
                recurring: data.recurring.unwrap_or(true),
            });
        }
        self.note(EVENT, "session.schedule_created");
        None
    }

    /// A scheduled prompt fired at `at`; `rest` is its `source` after `schedule-`, and `clock` the newest event time
    /// before this one.
    fn fired(&mut self, rest: &str, at: i64, clock: Option<i64>) {
        if let Ok(id) = rest.parse::<u64>()
            && let Some(timing) = self.schedules.get_mut(&id)
        {
            match timing {
                Timing::Interval {
                    anchor_ms,
                    recurring: true,
                    ..
                } => *anchor_ms = at,
                Timing::Cron {
                    from_ms,
                    recurring: true,
                    ..
                } => *from_ms = at,
                Timing::SelfPaced(_) => {}
                _ => {
                    self.schedules.remove(&id);
                }
            }
            return;
        }
        for timing in self.schedules.values_mut() {
            if let Timing::Interval {
                every_ms,
                anchor_ms,
                recurring: true,
            } = timing
                && interval_due(*every_ms, *anchor_ms, true, clock)
                    .is_some_and(|due| due <= at.saturating_add(FIRE_SLACK_MS))
            {
                *anchor_ms = at;
            }
        }
    }

    /// Drops one-shots that can no longer hold, so a long session's `/after` prompts do not fill the bound.
    fn drop_spent(&mut self) {
        let Some(clock) = self.clock_ms else {
            return;
        };
        let limit = clock.saturating_sub(SPENT_MS);
        self.schedules
            .retain(|_, timing| timing.one_shot_due().is_none_or(|due| due >= limit));
    }

    fn note(&mut self, field: &'static str, value: &str) {
        if self.problem.is_none() {
            self.problem = Some((field, value.to_owned()));
        }
    }

    fn activity(&self) -> Activity {
        match self.last {
            Last::Waiting(since) => Activity::Waiting {
                since_wall_ms: Some(since),
                remote: false,
            },
            Last::Working => Activity::Working,
            // A sub-agent started and not ended keeps the session working after the main turn ends (background).
            Last::Idle if !self.subagents.is_empty() => Activity::Working,
            Last::Idle => Activity::Idle,
        }
    }

    fn timers(&self, zone: &dyn LocalOffset) -> Vec<i64> {
        if self.closed {
            return Vec::new();
        }
        let mut due: Vec<i64> = self
            .schedules
            .values()
            .filter_map(|timing| timing.due(self.clock_ms, zone))
            .collect();
        due.sort_unstable();
        due.dedup();
        due
    }

    /// The core counts any `UnknownValue` as working while the process lives, so a problem may not make a session
    /// hold (as for Claude): it is told only while the session works anyway. A log of which no line reads gives no
    /// signal at all, and says so.
    fn format(&self, activity: Activity) -> FormatHealth {
        match &self.problem {
            None => FormatHealth::Ok,
            Some(_) if !self.read => FormatHealth::Unreadable(UNREADABLE),
            Some((field, value)) if activity == Activity::Working => {
                FormatHealth::UnknownValue(field, value.clone())
            }
            Some(_) => FormatHealth::Ok,
        }
    }
}

fn data<'a, T: Deserialize<'a>>(line: &'a [u8]) -> Option<T> {
    serde_json::from_slice::<Data<T>>(line).ok().map(|d| d.data)
}

/// Epoch ms as Copilot writes them; a whole float is taken too, since the schema says only "number" for some.
fn ms(value: &Number) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_f64()
            .filter(|ms| ms.is_finite() && ms.abs() < 9.0e15)
            .map(|ms| ms.floor() as i64)
    })
}

fn cut(value: &str) -> String {
    value
        .chars()
        .take(VALUE_CHARS)
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect()
}
