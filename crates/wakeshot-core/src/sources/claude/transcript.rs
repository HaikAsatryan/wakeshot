//! Claude Code transcripts: timers and background tasks.
//!
//! Owner: `p4-claude-transcript`; fixes by `p4-live-claude`.
//!
//! Rules 3, 5 and 6 of `docs/research/claude-code-signals.md`, with the Phase 4 live shapes, checked against Claude
//! Code 2.1.317's own code (`docs/research.md`, "Claude transcript checks"). A [`Replay`] reads one session process's
//! transcript lines as they are appended and keeps only ids, cron texts, times and flags: prompts and other text are
//! read through visitors that look at them in place and keep nothing (`notice_ids`), and every field of an unrelated
//! line kind is skipped without being kept.
//!
//! Times on lines older than the session process's start belong to an earlier process (`claude --resume` reuses the
//! transcript and writes no line for it, `p4-claude-live-shapes`). The replay keeps every line's time and applies the
//! resume rules only when it reports, so one replay serves any process that takes the transcript over.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::status::UNKNOWN_VALUE_CHARS;
use crate::holds::file::parse_expires;
use crate::sources::cron::{ClaudeJob, ClaudeScheduler, Cron, LocalOffset};

/// Claude Code's own limit on jobs per session; more than this is bad data, so later ones are not kept.
const MAX_JOBS: usize = 50;
/// Bounds for state a broken transcript could grow without end.
const MAX_WAKEUPS: usize = 50;
const MAX_TASKS: usize = 256;
const MAX_CALLS: usize = 256;
const MAX_LIST: usize = 256;
const MAX_NOTICE_IDS: usize = 64;
/// The longest id, cron or tool name kept; longer text is not one of them.
const MAX_TEXT: usize = 128;
/// A longer line is skipped: no line that matters comes near it, and the buffer must stay bounded.
const MAX_LINE: usize = 32 << 20;
/// A wakeup still listed this long after its time was cancelled with Esc (which writes nothing) or missed; it held
/// nothing for long before this, since the core's window ends 2 minutes after the time.
const WAKEUP_FORGET_MS: i64 = 60 * 60 * 1000;
/// The most steps when walking a carried job's fires up to the carry line: at most 30 minutes of fires, one a minute.
const CHAIN_STEPS: usize = 64;
/// The longest timeout a Monitor that is not persistent has in Claude Code 2.1.317 (`eEn`, its schema's bound). The
/// core holds a timer from an hour before its due time, so one due time at the timeout covers the whole run.
const MONITOR_MAX_MS: i64 = 60 * 60 * 1000;
/// A longer timeout than 2.1.317 allows still ends: due times one [`MONITOR_MAX_MS`] apart cover it, up to this many.
const MONITOR_STEPS: i64 = 24;
/// A `CronCreate` result's `nextFireAt` this far from the built-in scheduler's time means its remote flag changed.
const DRIFT_MS: i64 = 2_000;

/// What `FormatHealth::UnknownValue` names: a transcript line kind, or a cron text track note 5 does not know.
pub const TRANSCRIPT: &str = "transcript";
pub const CRON: &str = "cron";
const CARRY: &str = "session_cron_carry";

/// Every line that matters holds one of these, so the others are never parsed.
const MARKERS: [&[u8]; 4] = [
    b"tool_use",
    b"scheduled_task_fire",
    b"session_cron_carry",
    b"<task-notification>",
];

const NOTICE: &str = "<task-notification>";
const NOTICE_END: &str = "</task-notification>";
/// The first line of the header Claude Code 2.1.317 puts before task notifications on some paths (`Dwe`, `RJr`); the
/// header ends with a blank line. Its own reader also takes it inside a `<system-reminder>`.
const HEADER: &str = "[SYSTEM NOTIFICATION - NOT USER INPUT]";
const REMINDER: &str = "<system-reminder>\n";

/// One session process's transcript lines, replayed into its armed timers and open background tasks.
///
/// Fed appended bytes; a line is read once its newline has come, so any split of the file gives the same state.
#[derive(Clone, Default)]
pub struct Replay {
    jobs: BTreeMap<String, Job>,
    /// Recurring jobs dropped at the built-in 7 day age, kept in case Claude Code's flag lets them live longer.
    aged: BTreeMap<String, Job>,
    wakeups: Vec<Wakeup>,
    /// Background tasks by id that no notification or stop has closed.
    tasks: BTreeMap<String, Task>,
    /// `tool_use` ids waiting for their result, oldest first.
    calls: VecDeque<(String, Call)>,
    /// The first problem seen: a field name and a value, never line content.
    problem: Option<(&'static str, String)>,
    /// The first gap seen between a `nextFireAt` and the built-in scheduler's time for it.
    drift: Option<i64>,
    partial: Vec<u8>,
    skipping: bool,
}

/// One `CronCreate` job. `basis_ms` is the time of the line that set `basis`, so the resume rules can tell an earlier
/// process's view from this one's.
#[derive(Clone, Debug)]
struct Job {
    /// `None` when Claude Code's syntax changed (track note 5): only an exact time from the transcript then counts.
    cron: Option<Cron>,
    recurring: bool,
    created_ms: i64,
    basis: Basis,
    basis_ms: i64,
    /// Claude Code fired it after the built-in 7 day age, so its flag says otherwise and the age no longer ends it.
    outlived: bool,
}

/// One background task. `ends_ms` is set for a Monitor that is not persistent: Claude Code ends it at its timeout and
/// writes no notification with a `<status>` for that (2.1.317, `We`), so it holds as a timer up to then.
#[derive(Clone, Copy, Debug)]
struct Task {
    opened_ms: i64,
    ends_ms: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
enum Basis {
    /// `nextFireAt` as the transcript gave it, jitter included.
    Exact(i64),
    /// Computed from this time: the last fire, or the creation (`ClaudeJob::from_ms`).
    From(i64),
}

/// A `ScheduleWakeup` wakeup: no id, so it ends by time (rule 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wakeup {
    due_ms: i64,
    armed_ms: i64,
}

#[derive(Clone, Debug)]
enum Call {
    CronCreate {
        cron: String,
        recurring: Option<bool>,
    },
    CronDelete(String),
    Wakeup,
    Monitor,
    /// `TaskStop` (`KillShell`, `KillBash`): Claude Code writes no notification for the task it stops, so its result
    /// is the end (2.1.317, `ILe`; its own resume scanner `hrn` reads it the same way). The input's id, if plain.
    Stop(Option<String>),
    /// A shell run with `run_in_background: true`, whose result must name its task.
    Background,
}

impl fmt::Debug for Replay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A partial line can hold any text, so only its length is shown.
        f.debug_struct("Replay")
            .field("jobs", &self.jobs)
            .field("aged", &self.aged)
            .field("wakeups", &self.wakeups)
            .field("tasks", &self.tasks)
            .field("calls", &self.calls)
            .field("problem", &self.problem)
            .field("drift", &self.drift)
            .field("partial_bytes", &self.partial.len())
            .field("skipping", &self.skipping)
            .finish()
    }
}

impl Replay {
    /// Appended transcript bytes. `zone` places cron times for a job known only from a carry line.
    pub fn feed(&mut self, bytes: &[u8], zone: &dyn LocalOffset) {
        let mut rest = bytes;
        while let Some(end) = rest.iter().position(|&b| b == b'\n') {
            let line = &rest[..end];
            rest = &rest[end + 1..];
            if std::mem::take(&mut self.skipping) {
                continue;
            }
            if self.partial.is_empty() {
                self.take_line(line, zone);
            } else {
                let mut whole = std::mem::take(&mut self.partial);
                whole.extend_from_slice(line);
                self.take_line(&whole, zone);
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

    /// The same process goes on in a new transcript (`/clear`): its jobs and tasks stay, a partial line does not.
    pub fn start_file(&mut self) {
        self.partial = Vec::new();
        self.skipping = false;
    }

    /// Due times in wall ms, jitter included and without the core's margin, of the timers armed in the process that
    /// started at `since_ms`; sorted, no repeats.
    ///
    /// Lines older than `since_ms` are an earlier process's. Claude Code 2.1.317 restores its `CronCreate` jobs at a
    /// resume (`zi`): a recurring job counts its next fire from the resume and is dropped at 7 days old, a one-shot
    /// whose time has passed is dropped. Its wakeups are not restored for a terminal or SDK session.
    pub fn timers(&self, since_ms: i64, zone: &dyn LocalOffset) -> Vec<i64> {
        let mut due = Vec::new();
        for (id, job) in &self.jobs {
            job.dues(id, since_ms, zone, &mut due);
        }
        due.extend(
            self.wakeups
                .iter()
                .filter(|wakeup| wakeup.armed_ms >= since_ms)
                .map(|wakeup| wakeup.due_ms),
        );
        due.sort_unstable();
        due.dedup();
        due
    }

    /// Due times, as for [`Replay::timers`], of the open background tasks that end on their own: a Monitor that is
    /// not persistent, at its timeout. Rule 3 counts them where it counts [`Replay::open_tasks`]; sorted, no repeats.
    pub fn task_timers(&self, since_ms: i64) -> Vec<i64> {
        let mut due = Vec::new();
        for task in self
            .tasks
            .values()
            .filter(|task| task.opened_ms >= since_ms)
        {
            if let Some(ends) = task.ends_ms {
                monitor_dues(task.opened_ms, ends, &mut due);
            }
        }
        due.sort_unstable();
        due.dedup();
        due
    }

    /// Background tasks with no known end, opened in the process that started at `since_ms` and not closed: a task
    /// does not outlive the process that ran it. A Monitor with a timeout is not counted here but in
    /// [`Replay::task_timers`], so it stops holding at its timeout even though that writes no `<status>`.
    pub fn open_tasks(&self, since_ms: i64) -> u32 {
        let open = self
            .tasks
            .values()
            .filter(|task| task.opened_ms >= since_ms && task.ends_ms.is_none())
            .count();
        u32::try_from(open).unwrap_or(u32::MAX)
    }

    /// The first gap, in ms, between a recurring `CronCreate` result's `nextFireAt` and the time Claude Code's built-in
    /// scheduler values give for it; `None` while they agree. A gap means Claude Code's remote flag
    /// `tengu_kairos_cron_config` changed, so computed due times may be off (they still cover any value its schema
    /// allows). For a live check or a log; it changes no fact.
    pub fn scheduler_drift_ms(&self) -> Option<i64> {
        self.drift
    }

    /// The first line problem seen, as a field name ([`TRANSCRIPT`] or [`CRON`]) and a value: a line kind, or a cron
    /// text cut to 32 ASCII characters. Once seen it stays, so "update Wakeshot" can still show at the session's next
    /// turn.
    pub fn problem(&self) -> Option<(&'static str, &str)> {
        self.problem
            .as_ref()
            .map(|(field, value)| (*field, value.as_str()))
    }

    fn flag(&mut self, field: &'static str, value: impl Into<String>) {
        if self.problem.is_none() {
            self.problem = Some((field, value.into()));
        }
    }

    fn take_line(&mut self, line: &[u8], zone: &dyn LocalOffset) {
        if !MARKERS.iter().any(|marker| contains(line, marker)) {
            return;
        }
        let line = line.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(line);
        let Ok(Maybe(Some(line))) = serde_json::from_slice::<Maybe<Line>>(line) else {
            return self.flag(TRANSCRIPT, "line");
        };
        if let Some(time) = line.time {
            self.wakeups
                .retain(|wakeup| wakeup.due_ms.saturating_add(WAKEUP_FORGET_MS) > time);
            self.tasks.retain(|_, task| {
                task.ends_ms
                    .is_none_or(|ends| ends.saturating_add(WAKEUP_FORGET_MS) > time)
            });
        }
        match (line.kind, line.subtype) {
            (Some(Word::Assistant), _) => self.calls(line),
            (Some(Word::User), _) => self.results(line, zone),
            (Some(Word::System), Some(Word::ScheduledTaskFire)) => self.fired(&line),
            (Some(Word::Attachment), _) => self.attachment(line, zone),
            (Some(Word::QueueOperation), _) => self.queued(&line),
            _ => {}
        }
    }

    /// A `queue-operation` line. Claude Code writes each notification it queues as an `enqueue` line with the text as
    /// `content` before it delivers it as a user line (live fixtures 01, 02, 08, 12). A stop from the UI queues its
    /// `killed` notice as passive, so its user line may wait for the next prompt (2.1.317, `tUr`); this line is on disk
    /// at once (review M1 part b; not checked live, `p4-live-claude` checks it). A notice with a `<status>` is queued
    /// only after its task ended, so this never ends a running task.
    fn queued(&mut self, line: &Line) {
        if let (Some(Word::Enqueue), Some(ids)) = (line.operation, &line.notice) {
            self.close(ids);
        }
    }

    /// `tool_use` items of the tools that matter, kept until their result comes (rule 5: results by `tool_use_id`).
    fn calls(&mut self, line: Line) {
        let Some(content) = line.message.and_then(|message| message.content) else {
            return;
        };
        for item in content.items {
            if item.kind != Some(Word::ToolUse) {
                continue;
            }
            let input = item.input.unwrap_or_default();
            let (name, call) = match item.name {
                Some(Word::CronCreate) => (
                    "CronCreate",
                    input.cron.map(|cron| Call::CronCreate {
                        cron,
                        recurring: input.recurring,
                    }),
                ),
                Some(Word::CronDelete) => (
                    "CronDelete",
                    input.id.filter(|id| is_plain_id(id)).map(Call::CronDelete),
                ),
                Some(Word::ScheduleWakeup) => ("ScheduleWakeup", Some(Call::Wakeup)),
                Some(Word::Monitor) => ("Monitor", Some(Call::Monitor)),
                // Claude Code takes `task_id`, else the older `shell_id`; its result names the task anyway.
                Some(Word::TaskStop) => (
                    "TaskStop",
                    Some(Call::Stop(
                        input
                            .task_id
                            .or(input.shell_id)
                            .filter(|id| is_plain_id(id)),
                    )),
                ),
                Some(Word::Shell) if input.run_in_background == Some(true) => {
                    ("Bash", Some(Call::Background))
                }
                _ => continue,
            };
            match (item.id.filter(|id| is_plain_id(id)), call) {
                (Some(id), Some(call)) => self.push_call(id, call),
                _ => self.flag(TRANSCRIPT, name),
            }
        }
    }

    fn push_call(&mut self, id: String, call: Call) {
        self.take_call(&id);
        if self.calls.len() >= MAX_CALLS {
            self.calls.pop_front();
        }
        self.calls.push_back((id, call));
    }

    fn take_call(&mut self, id: &str) -> Option<Call> {
        let at = self.calls.iter().position(|(call_id, _)| call_id == id)?;
        self.calls.remove(at).map(|(_, call)| call)
    }

    /// A user line: a tool result, or a task notification as its text.
    fn results(&mut self, line: Line, zone: &dyn LocalOffset) {
        let Some(content) = line.message.and_then(|message| message.content) else {
            return;
        };
        if let Some(ids) = &content.notice {
            self.close(ids);
        }
        for item in &content.items {
            if let Some(ids) = &item.notice {
                self.close(ids);
            }
        }
        // Claude Code writes one result per line, with its `toolUseResult` beside it.
        let Some(item) = content
            .items
            .iter()
            .find(|item| item.kind == Some(Word::ToolResult))
        else {
            return;
        };
        let call = item
            .tool_use_id
            .as_deref()
            .and_then(|id| self.take_call(id));
        if item.is_error == Some(true) {
            return;
        }
        let result = line.result;
        let time = line.time;
        // Rule 3: any tool can start one (a shell with `run_in_background`, or one moved to the background).
        let background = result
            .as_ref()
            .and_then(|result| result.background_task_id.clone());
        if let Some(task) = &background {
            self.open(task, time, None);
        }
        match call {
            None => {}
            Some(Call::CronCreate { cron, recurring }) => {
                self.created(&cron, recurring, result, time, zone);
            }
            Some(Call::CronDelete(id)) => {
                self.jobs.remove(&id);
                self.aged.remove(&id);
            }
            Some(Call::Wakeup) => self.wakeup_result(result, time),
            Some(Call::Monitor) => self.monitor_result(result, time),
            Some(Call::Stop(asked)) => {
                let stopped = result
                    .and_then(|result| result.stopped_task)
                    .filter(|id| is_plain_id(id))
                    .or(asked);
                match stopped {
                    Some(id) => self.close(&[id]),
                    None => self.flag(TRANSCRIPT, "TaskStop result"),
                }
            }
            // Without its id the shell would not hold at all, and nothing else would say so (review m1).
            Some(Call::Background) if background.is_none() => {
                self.flag(TRANSCRIPT, "background shell result");
            }
            Some(Call::Background) => {}
        }
    }

    /// A Monitor result `{taskId, timeoutMs, persistent}`: `timeoutMs` is 0 when it is persistent, which only its
    /// notification or a stop ends.
    fn monitor_result(&mut self, result: Option<ToolResult>, time: Option<i64>) {
        let Some(result) = result else {
            return self.flag(TRANSCRIPT, "Monitor result");
        };
        let Some(task) = result.task_id else {
            return self.flag(TRANSCRIPT, "Monitor result");
        };
        let runs = match (result.persistent, result.timeout_ms.and_then(ceil_ms)) {
            (Some(true), _) => None,
            (_, Some(ms)) if (1..=MONITOR_MAX_MS).contains(&ms) => Some(ms),
            (_, Some(ms)) if ms > MONITOR_MAX_MS => {
                self.flag(TRANSCRIPT, "Monitor result");
                Some(ms)
            }
            // Never held for the life of the process on a shape not known: it ends by the longest 2.1.317 allows.
            _ => {
                self.flag(TRANSCRIPT, "Monitor result");
                Some(MONITOR_MAX_MS)
            }
        };
        self.open(&task, time, runs);
    }

    fn created(
        &mut self,
        cron: &str,
        recurring: Option<bool>,
        result: Option<ToolResult>,
        time: Option<i64>,
        zone: &dyn LocalOffset,
    ) {
        let Some(result) = result else {
            return self.flag(TRANSCRIPT, "CronCreate result");
        };
        let Some(id) = result.id.filter(|id| is_plain_id(id)) else {
            return self.flag(TRANSCRIPT, "CronCreate result");
        };
        // Rule 7: a durable job lives in `scheduled_tasks.json`, which v1 does not read.
        if result.durable == Some(true) {
            return;
        }
        let Some(time) = time else {
            return self.flag(TRANSCRIPT, "timestamp");
        };
        let basis = match result.next_fire_at.and_then(ceil_ms) {
            Some(at) => Basis::Exact(at),
            None => {
                self.flag(TRANSCRIPT, "CronCreate result");
                Basis::From(time)
            }
        };
        let cron = self.cron(cron);
        // Claude Code's own readers take `recurring !== false`.
        let recurring = result.recurring.or(recurring).unwrap_or(true);
        if let (Some(parsed), Basis::Exact(at), true, None) = (&cron, basis, recurring, self.drift)
        {
            self.drift = drift(&id, parsed, at, time, zone);
        }
        self.add_job(
            id,
            Job {
                cron,
                recurring,
                created_ms: time,
                basis,
                basis_ms: time,
                outlived: false,
            },
        );
    }

    fn cron(&mut self, text: &str) -> Option<Cron> {
        let cron = Cron::parse(text);
        if cron.is_none() {
            self.flag(CRON, shown(text));
        }
        cron
    }

    fn add_job(&mut self, id: String, job: Job) {
        if self.jobs.len() < MAX_JOBS || self.jobs.contains_key(&id) {
            self.jobs.insert(id, job);
        }
    }

    fn wakeup_result(&mut self, result: Option<ToolResult>, time: Option<i64>) {
        let Some(result) = result else {
            return self.flag(TRANSCRIPT, "ScheduleWakeup result");
        };
        // `stop: true` ends the loop and cancels every pending wakeup (`cancelledWakeups` counts them).
        if result.stopped == Some(true) {
            self.wakeups.clear();
            return;
        }
        let Some(at) = result.scheduled_for else {
            return self.flag(TRANSCRIPT, "ScheduleWakeup result");
        };
        // 0 without `stopped`: nothing was armed, as when the loop is 7 days old.
        if at <= 0.0 {
            return;
        }
        let Some(due) = ceil_ms(at) else {
            return self.flag(TRANSCRIPT, "ScheduleWakeup result");
        };
        let Some(time) = time else {
            return self.flag(TRANSCRIPT, "timestamp");
        };
        self.arm(due, time);
    }

    fn arm(&mut self, due_ms: i64, armed_ms: i64) {
        if self.wakeups.len() < MAX_WAKEUPS && !self.wakeups.iter().any(|w| w.due_ms == due_ms) {
            self.wakeups.push(Wakeup { due_ms, armed_ms });
        }
    }

    /// A task opened at `time`; `runs_ms` is how long it runs at most, when that is known.
    fn open(&mut self, task: &str, time: Option<i64>, runs_ms: Option<i64>) {
        if !is_plain_id(task) {
            return self.flag(TRANSCRIPT, "task id");
        }
        let Some(time) = time else {
            return self.flag(TRANSCRIPT, "timestamp");
        };
        if self.tasks.len() < MAX_TASKS || self.tasks.contains_key(task) {
            self.tasks.entry(task.to_owned()).or_insert(Task {
                opened_ms: time,
                ends_ms: runs_ms.map(|ms| time.saturating_add(ms)),
            });
        }
    }

    fn close(&mut self, ids: &[String]) {
        for id in ids {
            self.tasks.remove(id);
        }
    }

    /// `scheduled_task_fire`: a job ran (rule 5).
    fn fired(&mut self, line: &Line) {
        let (Some(id), Some(time)) = (line.task_id.as_deref(), line.time) else {
            return self.flag(TRANSCRIPT, "scheduled_task_fire");
        };
        let scheduler = ClaudeScheduler::DEFAULT;
        if line.task_kind == Some(Word::Loop) {
            // The line names no wakeup. A wakeup on :00 or :30 fires up to `one_shot_max_ms` early, and any fires late
            // after a busy turn, so every one due by then is done.
            let by = time.saturating_add(scheduler.one_shot_max_ms);
            self.wakeups.retain(|wakeup| wakeup.due_ms > by);
            return;
        }
        // A fire of a job dropped at the built-in age shows the flag keeps it longer; the fire lines then decide.
        if let Some(mut job) = self.aged.remove(id) {
            job.outlived = true;
            self.add_job(id.to_owned(), job);
        }
        let Some(job) = self.jobs.get_mut(id) else {
            return;
        };
        // A recurring job 7 days old fires once more, then Claude Code deletes it (`Ze`).
        let aged = job.recurring
            && !job.outlived
            && scheduler.recurring_max_age_ms > 0
            && time.saturating_sub(job.created_ms) >= scheduler.recurring_max_age_ms;
        if job.recurring {
            job.basis = Basis::From(time);
            job.basis_ms = time;
        }
        if !job.recurring || aged {
            let job = self.jobs.remove(id);
            if let Some(job) = job.filter(|_| aged && self.aged.len() < MAX_JOBS) {
                self.aged.insert(id.to_owned(), job);
            }
        }
    }

    fn attachment(&mut self, line: Line, zone: &dyn LocalOffset) {
        let Some(attachment) = line.attachment else {
            return;
        };
        match attachment.kind {
            Some(Word::SessionCronCarry) => self.carried(attachment, line.time, zone),
            // A notification that came while the main thread was busy (`p4-claude-live-shapes`, fixture 07).
            Some(Word::QueuedCommand) => {
                if let Some(ids) = &attachment.notice {
                    self.close(ids);
                }
            }
            _ => {}
        }
    }

    /// `session_cron_carry`, the first line after `/clear`: the process's armed jobs (`tasks` and `otherArmed`), its
    /// pending wakeup (`wakeup`) and jobs that ended (`ended`), as Claude Code 2.1.317 writes it (`OZn`).
    fn carried(&mut self, attachment: Attachment, time: Option<i64>, zone: &dyn LocalOffset) {
        let Some(time) = time else {
            return self.flag(TRANSCRIPT, "timestamp");
        };
        let Some(Some(tasks)) = attachment.tasks else {
            return self.flag(TRANSCRIPT, CARRY);
        };
        let other = match attachment.other_armed {
            None => List::default(),
            Some(Some(list)) => list,
            Some(None) => {
                self.flag(TRANSCRIPT, CARRY);
                List::default()
            }
        };
        if tasks.bad + other.bad > 0 {
            self.flag(TRANSCRIPT, CARRY);
        }
        let mut armed = BTreeSet::new();
        for entry in tasks.items.into_iter().chain(other.items) {
            let created = entry.created_at.and_then(floor_ms);
            let (Some(id), Some(cron), Some(created), Some(recurring)) = (
                entry.id.filter(|id| is_plain_id(id)),
                entry.cron,
                created,
                entry.recurring,
            ) else {
                self.flag(TRANSCRIPT, CARRY);
                continue;
            };
            armed.insert(id.clone());
            // Still armed after the built-in age: Claude Code's flag keeps it longer.
            if let Some(mut job) = self.aged.remove(&id) {
                job.outlived = true;
                self.add_job(id.clone(), job);
            }
            // A job this replay already knows keeps its own fires; the carry line has none.
            if self.jobs.contains_key(&id) {
                continue;
            }
            let parsed = self.cron(&cron);
            let from = match &parsed {
                Some(parsed) if recurring => last_fire_by(&id, parsed, created, time, zone),
                _ => created,
            };
            self.add_job(
                id,
                Job {
                    cron: parsed,
                    recurring,
                    created_ms: created,
                    basis: Basis::From(from),
                    basis_ms: time,
                    outlived: false,
                },
            );
        }
        self.jobs.retain(|id, _| armed.contains(id));
        self.aged.clear();
        if let Some(ended) = attachment.ended.flatten() {
            for id in ended.items {
                self.jobs.remove(&id.0);
            }
        }
        match attachment.wakeup {
            // An older Claude Code writes no `wakeup`; a kept wakeup ends by its own time.
            None => {}
            Some(Some(Wake {
                scheduled_for: Some(at),
            })) => {
                self.wakeups.clear();
                if let Some(due) = ceil_ms(at).filter(|&due| due > 0) {
                    self.arm(due, time);
                }
            }
            Some(_) => self.flag(TRANSCRIPT, CARRY),
        }
    }
}

impl Job {
    /// The job's due times for the process that started at `since_ms`. A time computed for a recurring job comes
    /// twice: as the built-in scheduler values give it, and as the latest any value of Claude Code's remote flag allows
    /// (review m2). The real fire then always falls inside the core's window, and its fire line moves the job on, so
    /// with the built-in values the hold is the same as with one time.
    fn dues(&self, id: &str, since_ms: i64, zone: &dyn LocalOffset, out: &mut Vec<i64>) {
        let scheduler = ClaudeScheduler::DEFAULT;
        let earlier = self.basis_ms < since_ms;
        let basis = if self.recurring && earlier {
            let max_age = scheduler.recurring_max_age_ms;
            if !self.outlived && max_age > 0 && since_ms.saturating_sub(self.created_ms) >= max_age
            {
                return;
            }
            Basis::From(since_ms)
        } else {
            self.basis
        };
        let job = |from_ms| {
            Some(ClaudeJob {
                id,
                cron: self.cron.as_ref()?,
                recurring: self.recurring,
                from_ms,
            })
        };
        let due = match basis {
            Basis::Exact(at) => Some(at),
            Basis::From(from_ms) => job(from_ms).and_then(|job| scheduler.next_fire_ms(&job, zone)),
        };
        let Some(due) = due else {
            return;
        };
        // Lines of an earlier process: Claude Code drops a one-shot whose time passed while none ran.
        if !self.recurring && earlier && due < since_ms {
            return;
        }
        out.push(due);
        if let (true, Basis::From(from_ms)) = (self.recurring, basis) {
            out.extend(
                job(from_ms).and_then(|job| ClaudeScheduler::latest_recurring_ms(&job, zone)),
            );
        }
    }
}

/// A Monitor's due times: its timeout, and for one longer than 2.1.317 allows, one every [`MONITOR_MAX_MS`] from its
/// start, so the core's hour before each covers the run up to [`MONITOR_STEPS`] of them.
fn monitor_dues(opened_ms: i64, ends_ms: i64, out: &mut Vec<i64>) {
    out.push(ends_ms);
    let mut at = opened_ms;
    for _ in 1..MONITOR_STEPS {
        at = at.saturating_add(MONITOR_MAX_MS);
        if at >= ends_ms {
            break;
        }
        out.push(at);
    }
}

/// How far a recurring job's `nextFireAt` is from the built-in scheduler's time for a job made at `made_ms`, when
/// more than [`DRIFT_MS`]. The result line comes a few ms after Claude Code's own creation time, so a job made just
/// before a cron run is also tried from 1 s earlier.
fn drift(id: &str, cron: &Cron, next_ms: i64, made_ms: i64, zone: &dyn LocalOffset) -> Option<i64> {
    let gap = |from_ms: i64| {
        let job = ClaudeJob {
            id,
            cron,
            recurring: true,
            from_ms,
        };
        ClaudeScheduler::DEFAULT
            .next_fire_ms(&job, zone)
            .map(|due| next_ms.saturating_sub(due))
    };
    let gaps = [gap(made_ms), gap(made_ms.saturating_sub(1_000))];
    if gaps.iter().flatten().any(|gap| gap.abs() <= DRIFT_MS) {
        return None;
    }
    gaps.into_iter().flatten().next()
}

/// The time to count a carried recurring job's next fire from, so it is the first fire after the carry line at `at_ms`:
/// its chain of fires from its creation, started at most `recurring_cap_ms` before `at_ms`, since a fire is never
/// later than that after its cron time. Exact for every job but a `*/5` one, whose chain hangs on its real fires and
/// comes out within one 285 s step.
fn last_fire_by(id: &str, cron: &Cron, created_ms: i64, at_ms: i64, zone: &dyn LocalOffset) -> i64 {
    let scheduler = ClaudeScheduler::DEFAULT;
    let mut from_ms = created_ms.max(at_ms.saturating_sub(scheduler.recurring_cap_ms));
    for _ in 0..CHAIN_STEPS {
        let job = ClaudeJob {
            id,
            cron,
            recurring: true,
            from_ms,
        };
        match scheduler.next_fire_ms(&job, zone) {
            Some(next) if next <= at_ms && next > from_ms => from_ms = next,
            _ => break,
        }
    }
    from_ms
}

/// The task ids a task notification closes: every `<task-id>` before its `<summary>`, when it has a `<status>`. Seen:
/// `completed`, `failed`, `stopped`, and `killed` in the code; a Monitor event has no `<status>` and closes nothing. A
/// batch notification lists several ids (2.1.317, resume). Only the start of the text counts, since the `/loop`
/// instructions quote the tag; Claude Code's own header may come first, and after it several notifications may be
/// joined, each after a blank line (2.1.317, `qi`; review m4).
fn notice_ids(text: &str) -> Option<Vec<String>> {
    let (mut rest, joined) = match after_header(text) {
        Some(rest) => (rest, true),
        None => (text, false),
    };
    let mut ids = Vec::new();
    while let Some(block) = rest.strip_prefix(NOTICE) {
        let end = block.find(NOTICE_END);
        block_ids(&block[..end.unwrap_or(block.len())], &mut ids);
        match end {
            Some(end) if joined => {
                rest = block[end + NOTICE_END.len()..]
                    .strip_prefix("\n\n")
                    .unwrap_or_default();
            }
            _ => break,
        }
    }
    (!ids.is_empty()).then_some(ids)
}

/// The text after Claude Code's notification header, which ends with a blank line; `None` without the header.
fn after_header(text: &str) -> Option<&str> {
    let text = text.strip_prefix(REMINDER).unwrap_or(text);
    if !text.starts_with(HEADER) {
        return None;
    }
    text.split_once("\n\n").map(|(_, rest)| rest)
}

/// The ids one notification closes, added to `ids`: each `<task-id>` before its `<summary>`, when it has a
/// `<status>`.
fn block_ids(block: &str, ids: &mut Vec<String>) {
    let mut found = Vec::new();
    let mut closes = false;
    for line in block.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((tag, value)) = tag_line(line) else {
            break;
        };
        match tag {
            "task-id" if is_plain_id(value) && ids.len() + found.len() < MAX_NOTICE_IDS => {
                found.push(value.to_owned());
            }
            "status" => closes = true,
            "summary" | "note" | "result" | "event" | "usage" => break,
            _ => {}
        }
    }
    if closes {
        ids.extend(found);
    }
}

/// `<name>value</name>` on one line, as Claude Code's `Ai` writes each header field.
fn tag_line(line: &str) -> Option<(&str, &str)> {
    let (name, rest) = line.strip_prefix('<')?.split_once('>')?;
    let value = rest
        .strip_suffix('>')?
        .strip_suffix(name)?
        .strip_suffix("</")?;
    Some((name, value))
}

/// Ids are short and have no spaces, so a field that holds anything else is not an id and is never kept.
fn is_plain_id(text: &str) -> bool {
    (1..=64).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_graphic())
}

/// As `status` shows an unknown value: at most 32 printable ASCII characters.
fn shown(text: &str) -> String {
    text.chars()
        .take(UNKNOWN_VALUE_CHARS)
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect()
}

/// Claude Code writes `nextFireAt` and `scheduledFor` as float ms and fires once `Date.now()` reaches them, so the
/// due time is the first whole ms at or after the float.
fn ceil_ms(at: f64) -> Option<i64> {
    (at.is_finite() && at.abs() < 9.0e15).then(|| at.ceil() as i64)
}

fn floor_ms(at: f64) -> Option<i64> {
    (at.is_finite() && at.abs() < 9.0e15).then(|| at.floor() as i64)
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|window| window == needle)
}

// The shapes read from a line. Each reads one JSON value of any type and keeps it only in its expected shape, so a
// field that an unrelated line kind uses for something else skips without spoiling the line, and text is looked at
// in place, never copied unless it is an id, a cron text or a time.

/// One JSON value, kept only in the shape `T` reads.
struct Maybe<T>(Option<T>);

trait Shape<'de>: Sized {
    fn text(_text: &str) -> Option<Self> {
        None
    }

    fn number(_value: f64) -> Option<Self> {
        None
    }

    fn flag(_value: bool) -> Option<Self> {
        None
    }

    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(None)
    }

    fn array<A: SeqAccess<'de>>(mut seq: A) -> Result<Option<Self>, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(None)
    }
}

impl<'de, T: Shape<'de>> Deserialize<'de> for Maybe<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(MaybeVisitor(PhantomData))
    }
}

struct MaybeVisitor<T>(PhantomData<T>);

impl<'de, T: Shape<'de>> Visitor<'de> for MaybeVisitor<T> {
    type Value = Maybe<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Maybe(T::flag(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Maybe(T::number(value as f64)))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Maybe(T::number(value as f64)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Maybe(T::number(value)))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Maybe(T::text(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Maybe(None))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        T::object(map).map(Maybe)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        T::array(seq).map(Maybe)
    }
}

fn read<'de, A: MapAccess<'de>, T: Shape<'de>>(map: &mut A) -> Result<Option<T>, A::Error> {
    Ok(map.next_value::<Maybe<T>>()?.0)
}

/// `Some(None)`: the key is there with another shape, which a line that must have it reports.
fn read_present<'de, A: MapAccess<'de>, T: Shape<'de>>(
    map: &mut A,
) -> Result<Option<Option<T>>, A::Error> {
    Ok(Some(read(map)?))
}

fn skip<'de, A: MapAccess<'de>>(map: &mut A) -> Result<(), A::Error> {
    map.next_value::<IgnoredAny>().map(drop)
}

fn read_text<'de, A: MapAccess<'de>>(map: &mut A) -> Result<Option<String>, A::Error> {
    Ok(read::<A, Text>(map)?.map(|text| text.0))
}

fn read_number<'de, A: MapAccess<'de>>(map: &mut A) -> Result<Option<f64>, A::Error> {
    Ok(read::<A, Num>(map)?.map(|number| number.0))
}

fn read_flag<'de, A: MapAccess<'de>>(map: &mut A) -> Result<Option<bool>, A::Error> {
    Ok(read::<A, Flag>(map)?.map(|flag| flag.0))
}

fn read_notice<'de, A: MapAccess<'de>>(map: &mut A) -> Result<Option<Vec<String>>, A::Error> {
    Ok(read::<A, Notice>(map)?.map(|notice| notice.0))
}

/// The keys any shape reads; every other key is skipped unread.
#[derive(Clone, Copy)]
enum Key {
    Type,
    Subtype,
    Timestamp,
    Message,
    ToolUseResult,
    TaskId,
    TaskKind,
    Attachment,
    Content,
    Id,
    Name,
    Input,
    ToolUseId,
    IsError,
    Text,
    Cron,
    Recurring,
    NextFireAt,
    Durable,
    ScheduledFor,
    Stopped,
    BackgroundTaskId,
    Tasks,
    OtherArmed,
    Wakeup,
    Ended,
    Prompt,
    CreatedAt,
    StopTaskId,
    ShellId,
    RunInBackground,
    TimeoutMs,
    Persistent,
    Operation,
    Other,
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;

        impl Visitor<'_> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object key")
            }

            fn visit_str<E: de::Error>(self, key: &str) -> Result<Key, E> {
                Ok(match key {
                    "type" => Key::Type,
                    "subtype" => Key::Subtype,
                    "timestamp" => Key::Timestamp,
                    "message" => Key::Message,
                    "toolUseResult" => Key::ToolUseResult,
                    "taskId" => Key::TaskId,
                    "taskKind" => Key::TaskKind,
                    "attachment" => Key::Attachment,
                    "content" => Key::Content,
                    "id" => Key::Id,
                    "name" => Key::Name,
                    "input" => Key::Input,
                    "tool_use_id" => Key::ToolUseId,
                    "is_error" => Key::IsError,
                    "text" => Key::Text,
                    "cron" => Key::Cron,
                    "recurring" => Key::Recurring,
                    "nextFireAt" => Key::NextFireAt,
                    "durable" => Key::Durable,
                    "scheduledFor" => Key::ScheduledFor,
                    "stopped" => Key::Stopped,
                    "backgroundTaskId" => Key::BackgroundTaskId,
                    "tasks" => Key::Tasks,
                    "otherArmed" => Key::OtherArmed,
                    "wakeup" => Key::Wakeup,
                    "ended" => Key::Ended,
                    "prompt" => Key::Prompt,
                    "createdAt" => Key::CreatedAt,
                    "task_id" => Key::StopTaskId,
                    "shell_id" => Key::ShellId,
                    "run_in_background" => Key::RunInBackground,
                    "timeoutMs" => Key::TimeoutMs,
                    "persistent" => Key::Persistent,
                    "operation" => Key::Operation,
                    _ => Key::Other,
                })
            }
        }

        deserializer.deserialize_identifier(KeyVisitor)
    }
}

/// The values of `type`, `subtype`, `name`, `taskKind` and `operation` that matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Word {
    Assistant,
    User,
    System,
    Attachment,
    QueueOperation,
    Enqueue,
    ScheduledTaskFire,
    ToolUse,
    ToolResult,
    CronCreate,
    CronDelete,
    ScheduleWakeup,
    Monitor,
    TaskStop,
    Shell,
    SessionCronCarry,
    QueuedCommand,
    Loop,
    Other,
}

impl Shape<'_> for Word {
    fn text(text: &str) -> Option<Self> {
        Some(match text {
            "assistant" => Word::Assistant,
            "user" => Word::User,
            "system" => Word::System,
            "attachment" => Word::Attachment,
            "queue-operation" => Word::QueueOperation,
            "enqueue" => Word::Enqueue,
            "scheduled_task_fire" => Word::ScheduledTaskFire,
            "tool_use" => Word::ToolUse,
            "tool_result" => Word::ToolResult,
            "CronCreate" => Word::CronCreate,
            "CronDelete" => Word::CronDelete,
            "ScheduleWakeup" => Word::ScheduleWakeup,
            "Monitor" => Word::Monitor,
            "TaskStop" | "KillShell" | "KillBash" => Word::TaskStop,
            "Bash" | "PowerShell" => Word::Shell,
            "session_cron_carry" => Word::SessionCronCarry,
            "queued_command" => Word::QueuedCommand,
            "loop" => Word::Loop,
            _ => Word::Other,
        })
    }
}

/// An id, a cron text or a time: short text only.
struct Text(String);

impl Shape<'_> for Text {
    fn text(text: &str) -> Option<Self> {
        (text.len() <= MAX_TEXT).then(|| Text(text.to_owned()))
    }
}

struct Num(f64);

impl Shape<'_> for Num {
    fn number(value: f64) -> Option<Self> {
        value.is_finite().then_some(Num(value))
    }
}

struct Flag(bool);

impl Shape<'_> for Flag {
    fn flag(value: bool) -> Option<Self> {
        Some(Flag(value))
    }
}

/// `timestamp`: the ISO text `Date.toISOString` writes.
struct Time(i64);

impl Shape<'_> for Time {
    fn text(text: &str) -> Option<Self> {
        parse_expires(text).map(Time)
    }
}

/// Text that may be a task notification: only the ids it closes are kept.
struct Notice(Vec<String>);

impl Shape<'_> for Notice {
    fn text(text: &str) -> Option<Self> {
        notice_ids(text).map(Notice)
    }
}

#[derive(Default)]
struct Line {
    kind: Option<Word>,
    subtype: Option<Word>,
    time: Option<i64>,
    message: Option<Message>,
    result: Option<ToolResult>,
    task_id: Option<String>,
    task_kind: Option<Word>,
    attachment: Option<Attachment>,
    operation: Option<Word>,
    /// A `queue-operation` line's `content`, read only as a notification.
    notice: Option<Vec<String>>,
}

impl<'de> Shape<'de> for Line {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut line = Line::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Type => line.kind = read(&mut map)?,
                Key::Subtype => line.subtype = read(&mut map)?,
                Key::Timestamp => line.time = read::<A, Time>(&mut map)?.map(|time| time.0),
                Key::Message => line.message = read(&mut map)?,
                Key::ToolUseResult => line.result = read(&mut map)?,
                Key::TaskId => line.task_id = read_text(&mut map)?,
                Key::TaskKind => line.task_kind = read(&mut map)?,
                Key::Attachment => line.attachment = read(&mut map)?,
                Key::Operation => line.operation = read(&mut map)?,
                Key::Content => line.notice = read_notice(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(line))
    }
}

#[derive(Default)]
struct Message {
    content: Option<Content>,
}

impl<'de> Shape<'de> for Message {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut message = Message::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Content => message.content = read(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(message))
    }
}

/// `message.content`: a user's text (only a task notification is kept), or a list of items.
#[derive(Default)]
struct Content {
    notice: Option<Vec<String>>,
    items: Vec<Item>,
}

impl<'de> Shape<'de> for Content {
    fn text(text: &str) -> Option<Self> {
        Some(Content {
            notice: notice_ids(text),
            items: Vec::new(),
        })
    }

    fn array<A: SeqAccess<'de>>(mut seq: A) -> Result<Option<Self>, A::Error> {
        let mut items = Vec::new();
        while let Some(Maybe(item)) = seq.next_element::<Maybe<Item>>()? {
            items.extend(item);
        }
        Ok(Some(Content {
            notice: None,
            items,
        }))
    }
}

/// A `content` item: `tool_use`, `tool_result` or text.
#[derive(Default)]
struct Item {
    kind: Option<Word>,
    id: Option<String>,
    name: Option<Word>,
    input: Option<Input>,
    tool_use_id: Option<String>,
    is_error: Option<bool>,
    notice: Option<Vec<String>>,
}

impl<'de> Shape<'de> for Item {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut item = Item::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Type => item.kind = read(&mut map)?,
                Key::Id => item.id = read_text(&mut map)?,
                Key::Name => item.name = read(&mut map)?,
                Key::Input => item.input = read(&mut map)?,
                Key::ToolUseId => item.tool_use_id = read_text(&mut map)?,
                Key::IsError => item.is_error = read_flag(&mut map)?,
                Key::Text => item.notice = read_notice(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(item))
    }
}

/// A `tool_use` input: `CronCreate` `{cron, recurring}`, `CronDelete` `{id}`, `TaskStop` `{task_id}` or `{shell_id}`,
/// a shell's `run_in_background`. `prompt` and `command` are never read.
#[derive(Default)]
struct Input {
    cron: Option<String>,
    recurring: Option<bool>,
    id: Option<String>,
    task_id: Option<String>,
    shell_id: Option<String>,
    run_in_background: Option<bool>,
}

impl<'de> Shape<'de> for Input {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut input = Input::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Cron => input.cron = read_text(&mut map)?,
                Key::Recurring => input.recurring = read_flag(&mut map)?,
                Key::Id => input.id = read_text(&mut map)?,
                Key::StopTaskId => input.task_id = read_text(&mut map)?,
                Key::ShellId => input.shell_id = read_text(&mut map)?,
                Key::RunInBackground => input.run_in_background = read_flag(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(input))
    }
}

/// `toolUseResult`: an object for the tools that matter; a tool's output text, a `TaskStop`'s `message` and `command`
/// are never read.
#[derive(Default)]
struct ToolResult {
    id: Option<String>,
    next_fire_at: Option<f64>,
    recurring: Option<bool>,
    durable: Option<bool>,
    scheduled_for: Option<f64>,
    stopped: Option<bool>,
    task_id: Option<String>,
    background_task_id: Option<String>,
    timeout_ms: Option<f64>,
    persistent: Option<bool>,
    /// `TaskStop`'s `task_id`: the task it stopped.
    stopped_task: Option<String>,
}

impl<'de> Shape<'de> for ToolResult {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut result = ToolResult::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Id => result.id = read_text(&mut map)?,
                Key::NextFireAt => result.next_fire_at = read_number(&mut map)?,
                Key::Recurring => result.recurring = read_flag(&mut map)?,
                Key::Durable => result.durable = read_flag(&mut map)?,
                Key::ScheduledFor => result.scheduled_for = read_number(&mut map)?,
                Key::Stopped => result.stopped = read_flag(&mut map)?,
                Key::TaskId => result.task_id = read_text(&mut map)?,
                Key::BackgroundTaskId => result.background_task_id = read_text(&mut map)?,
                Key::TimeoutMs => result.timeout_ms = read_number(&mut map)?,
                Key::Persistent => result.persistent = read_flag(&mut map)?,
                Key::StopTaskId => result.stopped_task = read_text(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(result))
    }
}

/// An `attachment`: `session_cron_carry` or `queued_command` (whose `prompt` is read only as a notification).
#[derive(Default)]
struct Attachment {
    kind: Option<Word>,
    tasks: Option<Option<List<CarryJob>>>,
    other_armed: Option<Option<List<CarryJob>>>,
    wakeup: Option<Option<Wake>>,
    ended: Option<Option<List<Text>>>,
    notice: Option<Vec<String>>,
}

impl<'de> Shape<'de> for Attachment {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut attachment = Attachment::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Type => attachment.kind = read(&mut map)?,
                Key::Tasks => attachment.tasks = read_present(&mut map)?,
                Key::OtherArmed => attachment.other_armed = read_present(&mut map)?,
                Key::Wakeup => attachment.wakeup = read_present(&mut map)?,
                Key::Ended => attachment.ended = read_present(&mut map)?,
                Key::Prompt => attachment.notice = read_notice(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(attachment))
    }
}

/// An array; `bad` counts the entries of another shape.
struct List<T> {
    items: Vec<T>,
    bad: usize,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            bad: 0,
        }
    }
}

impl<'de, T: Shape<'de>> Shape<'de> for List<T> {
    fn array<A: SeqAccess<'de>>(mut seq: A) -> Result<Option<Self>, A::Error> {
        let mut list = List::default();
        while let Some(Maybe(entry)) = seq.next_element::<Maybe<T>>()? {
            match entry {
                Some(entry) if list.items.len() < MAX_LIST => list.items.push(entry),
                Some(_) => {}
                None => list.bad += 1,
            }
        }
        Ok(Some(list))
    }
}

/// A carried job: `{id, cron, prompt, createdAt, recurring}`; `prompt` is never read.
#[derive(Default)]
struct CarryJob {
    id: Option<String>,
    cron: Option<String>,
    created_at: Option<f64>,
    recurring: Option<bool>,
}

impl<'de> Shape<'de> for CarryJob {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut job = CarryJob::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::Id => job.id = read_text(&mut map)?,
                Key::Cron => job.cron = read_text(&mut map)?,
                Key::CreatedAt => job.created_at = read_number(&mut map)?,
                Key::Recurring => job.recurring = read_flag(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(job))
    }
}

/// A carried wakeup: `{prompt, scheduledFor, reason?, keepalive?}`; only the time is read.
#[derive(Default)]
struct Wake {
    scheduled_for: Option<f64>,
}

impl<'de> Shape<'de> for Wake {
    fn object<A: MapAccess<'de>>(mut map: A) -> Result<Option<Self>, A::Error> {
        let mut wake = Wake::default();
        while let Some(key) = map.next_key::<Key>()? {
            match key {
                Key::ScheduledFor => wake.scheduled_for = read_number(&mut map)?,
                _ => skip(&mut map)?,
            }
        }
        Ok(Some(wake))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holds::file::format_expires;
    use crate::sources::cron::FixedOffset;
    use proptest::prelude::*;
    use serde_json::{Value, json};

    // A Monitor's one due time covers its run only while the core holds a timer at least this long before it.
    const _: () = assert!(crate::policy::sessions::TIMER_LEAD_MS >= MONITOR_MAX_MS);

    /// The test laptop's zone at the live tests: UTC+4, no daylight saving.
    const ZONE: FixedOffset = FixedOffset(240);
    /// 2026-11-03T14:19:23.530Z, when the live `/loop 1m` job was made.
    const T: i64 = 1_793_715_563_530;
    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;
    const SID: &str = "5e550004-0000-4000-8000-000000000004";

    fn iso(ms: i64) -> String {
        format_expires(ms).replace('Z', &format!(".{:03}Z", ms.rem_euclid(1000)))
    }

    fn line(mut fields: Value, at: i64) -> String {
        fields["timestamp"] = json!(iso(at));
        fields["sessionId"] = json!(SID);
        fields["entrypoint"] = json!("cli");
        fields["version"] = json!("2.1.317");
        fields.to_string()
    }

    fn call(id: &str, name: &str, input: Value, at: i64) -> String {
        line(
            json!({"isSidechain": false, "type": "assistant", "message": {"model": "claude-haiku-5-5",
                "role": "assistant", "content": [{"type": "tool_use", "id": id, "name": name, "input": input,
                "caller": {"type": "direct"}}], "stop_reason": "tool_use"}}),
            at,
        )
    }

    fn result(id: &str, result: Value, at: i64) -> String {
        line(
            json!({"type": "user", "message": {"role": "user", "content": [{"tool_use_id": id,
                "type": "tool_result", "content": "<str 192>"}]}, "toolUseResult": result}),
            at,
        )
    }

    fn failed(id: &str, at: i64) -> String {
        line(
            json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result",
                "content": "<str 40>", "is_error": true, "tool_use_id": id}]},
                "toolUseResult": "Error: <str 33>"}),
            at,
        )
    }

    fn fire(task: &str, cron: &str, kind: Option<&str>, at: i64) -> String {
        let mut fields = json!({"type": "system", "subtype": "scheduled_task_fire", "content": "<str 37>",
            "isMeta": false, "taskId": task, "cron": cron, "prompt": "<str 29>"});
        if let Some(kind) = kind {
            fields["taskKind"] = json!(kind);
            fields["cronKind"] = json!(kind);
        }
        line(fields, at)
    }

    fn user_text(text: &str, at: i64) -> String {
        line(
            json!({"type": "user", "message": {"role": "user", "content": text},
                "origin": {"kind": "task-notification"}}),
            at,
        )
    }

    fn attachment(attachment: Value, at: i64) -> String {
        line(json!({"type": "attachment", "attachment": attachment}), at)
    }

    fn notification(ids: &[&str], status: Option<&str>) -> String {
        let mut text = String::from("<task-notification>");
        for id in ids {
            text += &format!("\n<task-id>{id}</task-id>");
        }
        text += "\n<tool-use-id>toolu_01LuABWMxFXYBKsiunRt9TjE</tool-use-id>\n<output-file><str 117></output-file>";
        if let Some(status) = status {
            text += &format!("\n<status>{status}</status>");
        }
        text + "\n<summary><str 92></summary>\n</task-notification>"
    }

    fn create(tool: &str, cron: &str, recurring: bool, at: i64) -> String {
        call(
            tool,
            "CronCreate",
            json!({"cron": cron, "prompt": "<str 29>", "recurring": recurring}),
            at,
        )
    }

    fn created(tool: &str, id: &str, recurring: bool, next: f64, at: i64) -> String {
        result(
            tool,
            json!({"id": id, "humanSchedule": "Every minute", "recurring": recurring, "durable": false,
                "nextFireAt": next}),
            at,
        )
    }

    fn wakeup(tool: &str, delay: u32, at: i64) -> String {
        call(
            tool,
            "ScheduleWakeup",
            json!({"delaySeconds": delay, "noop": false, "reason": "<str 74>", "prompt": "<str 204>"}),
            at,
        )
    }

    fn woken(tool: &str, scheduled_for: i64, at: i64) -> String {
        result(
            tool,
            json!({"scheduledFor": scheduled_for, "clampedDelaySeconds": 60, "wasClamped": false}),
            at,
        )
    }

    fn shell(tool: &str, task: &str, at: i64) -> [String; 2] {
        [
            call(
                tool,
                "Bash",
                json!({"command": "<str 36>", "run_in_background": true}),
                at,
            ),
            result(
                tool,
                json!({"stdout": "", "backgroundTaskId": task}),
                at + 1,
            ),
        ]
    }

    fn carried(tasks: Value, other: Option<Value>, wakeup: Option<Value>, ended: &[&str]) -> Value {
        let mut carry = json!({"type": "session_cron_carry", "tasks": tasks});
        if let Some(other) = other {
            carry["otherArmed"] = other;
        }
        if let Some(wakeup) = wakeup {
            carry["wakeup"] = wakeup;
        }
        if !ended.is_empty() {
            carry["ended"] = json!(ended);
        }
        carry
    }

    fn job(id: &str, cron: &str, created: i64, recurring: bool) -> Value {
        json!({"id": id, "cron": cron, "prompt": "<str 29>", "createdAt": created, "recurring": recurring})
    }

    fn replay(lines: &[String]) -> Replay {
        let mut replay = Replay::default();
        for line in lines {
            replay.feed(format!("{line}\n").as_bytes(), &ZONE);
        }
        replay
    }

    /// Everything counts: no line is older than the process.
    fn timers(replay: &Replay) -> Vec<i64> {
        replay.timers(i64::MIN, &ZONE)
    }

    fn tasks(replay: &Replay) -> u32 {
        replay.open_tasks(i64::MIN)
    }

    fn ends(replay: &Replay) -> Vec<i64> {
        replay.task_timers(i64::MIN)
    }

    fn next_fire(id: &str, cron: &str, recurring: bool, from_ms: i64) -> i64 {
        let cron = Cron::parse(cron).unwrap();
        ClaudeScheduler::DEFAULT
            .next_fire_ms(
                &ClaudeJob {
                    id,
                    cron: &cron,
                    recurring,
                    from_ms,
                },
                &ZONE,
            )
            .unwrap()
    }

    /// The latest fire any value of Claude Code's remote flag allows, the second due time of a computed recurring time.
    fn latest(id: &str, cron: &str, from_ms: i64) -> i64 {
        let cron = Cron::parse(cron).unwrap();
        ClaudeScheduler::latest_recurring_ms(
            &ClaudeJob {
                id,
                cron: &cron,
                recurring: true,
                from_ms,
            },
            &ZONE,
        )
        .unwrap()
    }

    /// Live fixture 03, lines 5 and 6: the float `nextFireAt` is due at its ceiling.
    #[test]
    fn cron_create_arms_its_exact_next_fire() {
        let r = replay(&[
            create("toolu_1", "*/1 * * * *", true, T),
            created(
                "toolu_1",
                "bd9c33c4",
                true,
                1_793_715_622_219.941_2,
                T + 664,
            ),
        ]);
        assert_eq!(timers(&r), [1_793_715_622_220]);
        assert_eq!(r.problem(), None);
        assert_eq!(tasks(&r), 0);
    }

    #[test]
    fn a_failed_or_durable_cron_create_arms_nothing() {
        let r = replay(&[
            create("toolu_1", "*/1 * * * *", true, T),
            failed("toolu_1", T + 1),
            create("toolu_2", "*/1 * * * *", true, T + 2),
            result(
                "toolu_2",
                json!({"id": "aaaa0002", "humanSchedule": "Every minute", "recurring": true, "durable": true,
                    "nextFireAt": (T + MIN) as f64}),
                T + 3,
            ),
            // A result for a call never seen is not a job: it has no cron.
            created("toolu_3", "aaaa0003", true, (T + MIN) as f64, T + 4),
        ]);
        assert_eq!(timers(&r), Vec::<i64>::new());
        assert_eq!(r.problem(), None);
    }

    #[test]
    fn a_recurring_fire_moves_the_job_on_and_a_one_shot_fire_ends_it() {
        let fired_at = T + 58_744;
        let r = replay(&[
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, (T + 58_690) as f64, T + 664),
            fire("bd9c33c4", "*/1 * * * *", None, fired_at),
        ]);
        let next = next_fire("bd9c33c4", "*/1 * * * *", true, fired_at);
        // Live fixture 03: the next fire came 107 ms after this.
        assert_eq!(next, 1_793_715_682_220);
        assert_eq!(
            timers(&r),
            [next, latest("bd9c33c4", "*/1 * * * *", fired_at)]
        );

        let mut lines = vec![
            create("toolu_2", "23 18 3 11 *", false, T),
            created("toolu_2", "12d31851", false, 1_793_715_780_000.0, T + 1),
        ];
        assert_eq!(timers(&replay(&lines)), [1_793_715_780_000]);
        lines.push(fire("12d31851", "23 18 3 11 *", None, 1_793_715_780_849));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    }

    #[test]
    fn a_recurring_job_7_days_old_fires_once_more_then_is_gone() {
        let mut lines = vec![
            create("toolu_1", "0 * * * *", true, T),
            created("toolu_1", "00000000", true, (T + HOUR) as f64, T),
        ];
        let before = T + 7 * DAY - 10 * MIN;
        lines.push(fire("00000000", "0 * * * *", None, before));
        assert_eq!(
            timers(&replay(&lines))[0],
            next_fire("00000000", "0 * * * *", true, before)
        );
        lines.push(fire("00000000", "0 * * * *", None, T + 7 * DAY + 50 * MIN));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    }

    #[test]
    fn cron_delete_ends_the_job_unless_it_failed() {
        let mut lines = vec![
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
            call("toolu_2", "CronDelete", json!({"id": "bd9c33c4"}), T + 2),
            failed("toolu_2", T + 3),
            call("toolu_3", "CronDelete", json!({"id": "ffffffff"}), T + 4),
            result("toolu_3", json!({"id": "ffffffff"}), T + 5),
        ];
        assert_eq!(timers(&replay(&lines)), [T + MIN]);
        lines.push(call(
            "toolu_4",
            "CronDelete",
            json!({"id": "bd9c33c4"}),
            T + 6,
        ));
        lines.push(result("toolu_4", json!({"id": "bd9c33c4"}), T + 7));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    }

    /// Live fixture 04: armed at line 6, fired at line 10, stopped at line 16.
    #[test]
    fn a_wakeup_holds_its_time_until_a_loop_fire_or_stop() {
        let due = 1_793_715_660_000;
        let mut lines = vec![wakeup("toolu_1", 60, T), woken("toolu_1", due, T + 1)];
        assert_eq!(timers(&replay(&lines)), [due]);
        lines.push(fire("b19b19ca", "21 18 * * *", Some("loop"), due + 951));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());

        // A fire takes every wakeup due within 90 s of it, early ones included, and no later one.
        let lines = [
            wakeup("toolu_1", 60, T),
            woken("toolu_1", due, T + 1),
            wakeup("toolu_2", 600, T + 2),
            woken("toolu_2", due + 10 * MIN, T + 3),
            fire("c0ffee00", "0 18 * * *", Some("loop"), due - 89_000),
        ];
        assert_eq!(timers(&replay(&lines)), [due + 10 * MIN]);

        let stop = [
            wakeup("toolu_1", 60, T),
            woken("toolu_1", due, T + 1),
            call("toolu_3", "ScheduleWakeup", json!({"stop": true}), T + 2),
            result(
                "toolu_3",
                json!({"scheduledFor": 0, "clampedDelaySeconds": 0, "wasClamped": false, "stopped": true,
                    "cancelledWakeups": 1}),
                T + 3,
            ),
        ];
        assert_eq!(timers(&replay(&stop)), Vec::<i64>::new());

        // `scheduledFor: 0` without `stopped`: nothing was armed.
        let none = replay(&[wakeup("toolu_1", 60, T), woken("toolu_1", 0, T + 1)]);
        assert_eq!(timers(&none), Vec::<i64>::new());
        assert_eq!(none.problem(), None);
    }

    #[test]
    fn a_wakeup_is_forgotten_an_hour_after_its_time() {
        let due = T + 2 * MIN;
        let mut lines = vec![wakeup("toolu_1", 120, T), woken("toolu_1", due, T + 1)];
        lines.push(fire("aaaa0001", "*/1 * * * *", None, due + HOUR - 1));
        assert_eq!(timers(&replay(&lines)), [due]);
        lines.push(fire("aaaa0001", "*/1 * * * *", None, due + HOUR));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    }

    /// Live fixture 06: after `/clear`, a known job keeps its last fire.
    #[test]
    fn a_carry_line_is_the_armed_set() {
        let fired_at = T + 65_000;
        let lines = [
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "f7152559", true, (T + 64_000) as f64, T + 1),
            create("toolu_2", "*/5 * * * *", true, T + 2),
            created("toolu_2", "aaaa0002", true, (T + 285_000) as f64, T + 3),
            create("toolu_3", "0 9 * * *", true, T + 4),
            created("toolu_3", "aaaa0003", true, (T + 20 * HOUR) as f64, T + 5),
            fire("f7152559", "*/1 * * * *", None, fired_at),
            wakeup("toolu_4", 600, T + 6),
            woken("toolu_4", T + 10 * MIN, T + 7),
            attachment(
                carried(
                    json!([job("aaaa0003", "0 9 * * *", T + 5, true)]),
                    Some(json!([
                        job("f7152559", "*/1 * * * *", T + 1, true),
                        job("bbbb0001", "30 18 3 11 *", T - MIN, false),
                    ])),
                    None,
                    &["aaaa0003"],
                ),
                T + 2 * MIN,
            ),
        ];
        let r = replay(&lines);
        // `aaaa0002` is not listed and `aaaa0003` ended; the wakeup stays, since the line names none.
        let mut want = vec![
            next_fire("f7152559", "*/1 * * * *", true, fired_at),
            latest("f7152559", "*/1 * * * *", fired_at),
            next_fire("bbbb0001", "30 18 3 11 *", false, T - MIN),
            T + 10 * MIN,
        ];
        want.sort_unstable();
        assert_eq!(timers(&r), want);
        assert_eq!(r.problem(), None);

        // A carried wakeup replaces the ones before it.
        let mut with_wakeup = lines.to_vec();
        with_wakeup.push(attachment(
            carried(
                json!([]),
                None,
                Some(json!({"prompt": "<str 29>", "scheduledFor": T + 20 * MIN, "reason": "<str 9>"})),
                &[],
            ),
            T + 3 * MIN,
        ));
        assert_eq!(timers(&replay(&with_wakeup)), [T + 20 * MIN]);
    }

    /// A job known only from a carry line (Wakeshot started after the `/clear`) is due at its first fire after it.
    #[test]
    fn a_job_known_only_from_a_carry_is_due_at_its_first_fire_after_the_line() {
        // Live fixture 06, `.2` alone: the job fired 282 ms after this.
        let carry = attachment(
            carried(
                json!([]),
                Some(json!([job(
                    "f7152559",
                    "*/1 * * * *",
                    1_793_715_621_070,
                    true
                )])),
                None,
                &[],
            ),
            1_793_715_733_332,
        );
        assert_eq!(timers(&replay(&[carry]))[0], 1_793_715_748_955);

        // An hourly job made days ago whose run has passed but whose late fire has not: that fire.
        let id = "ffffffff";
        let run = next_fire(id, "0 * * * *", true, T);
        let pending = run - MIN;
        assert!(pending > T + MIN);
        let carry = attachment(
            carried(
                json!([job(id, "0 * * * *", T - 3 * DAY, true)]),
                None,
                None,
                &[],
            ),
            pending,
        );
        assert_eq!(timers(&replay(&[carry]))[0], run);
    }

    #[test]
    fn background_tasks_open_and_any_status_closes() {
        let opened = [
            call(
                "toolu_1",
                "PowerShell",
                json!({"command": "<str 33>", "run_in_background": true}),
                T,
            ),
            result(
                "toolu_1",
                json!({"stdout": "", "stderr": "", "interrupted": false, "backgroundTaskId": "b1irxlyqr"}),
                T + 1,
            ),
        ];
        assert_eq!(tasks(&replay(&opened)), 1);
        for status in [
            "completed",
            "failed",
            "killed",
            "stopped",
            "a status not seen yet",
        ] {
            let mut lines = opened.to_vec();
            lines.push(user_text(
                &notification(&["b1irxlyqr"], Some(status)),
                T + 2,
            ));
            assert_eq!(tasks(&replay(&lines)), 0, "{status}");
        }
        // Another task's notification closes nothing here, and a failed call opens nothing.
        let mut lines = opened.to_vec();
        lines.push(user_text(
            &notification(&["b0ther000"], Some("completed")),
            T + 2,
        ));
        assert_eq!(tasks(&replay(&lines)), 1);
        assert_eq!(tasks(&replay(&[failed("toolu_9", T)])), 0);
    }

    fn monitor(tool: &str, task: &str, timeout_ms: u32, persistent: bool, at: i64) -> [String; 2] {
        [
            call(
                tool,
                "Monitor",
                json!({"command": "<str 95>", "timeout_ms": timeout_ms, "persistent": persistent}),
                at,
            ),
            result(
                tool,
                json!({"taskId": task, "timeoutMs": if persistent { 0 } else { timeout_ms },
                    "persistent": persistent}),
                at + 1,
            ),
        ]
    }

    /// Live fixture 02: an event notification has no `<status>`. Review M1: Claude Code ends a Monitor that is not
    /// persistent at its timeout with one more event and no `<status>`, so it is a timer at its timeout, not an open
    /// task; a `<status>` before that ends it.
    #[test]
    fn a_monitor_holds_until_its_timeout_and_its_events_close_nothing() {
        let event = |at| {
            user_text(
                "<task-notification>\n<task-id>bu1gw9xxc</task-id>\n<summary><str 70></summary>\n<event><str 5></event><str 123></task-notification>",
                at,
            )
        };
        let mut lines = monitor("toolu_1", "bu1gw9xxc", 120_000, false, T).to_vec();
        lines.push(event(T + 2));
        let r = replay(&lines);
        assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + 120_000]));
        assert_eq!(r.problem(), None);
        // The timeout's own event changes nothing; the time passing ends it.
        lines.push(event(T + 1 + 120_000));
        assert_eq!(ends(&replay(&lines)), [T + 1 + 120_000]);
        let mut done = lines.clone();
        done.push(user_text(
            &notification(&["bu1gw9xxc"], Some("completed")),
            T + 3,
        ));
        assert_eq!(ends(&replay(&done)), Vec::<i64>::new());
        // Forgotten an hour after its time, like a wakeup.
        lines.push(fire(
            "aaaa0001",
            "*/1 * * * *",
            None,
            T + 1 + 120_000 + HOUR,
        ));
        assert_eq!(ends(&replay(&lines)), Vec::<i64>::new());

        // A persistent Monitor has no end of its own: an open task until its notification or a stop.
        let mut lines = monitor("toolu_2", "bpersist1", 0, true, T).to_vec();
        lines.push(event(T + 2));
        let r = replay(&lines);
        assert_eq!((tasks(&r), ends(&r)), (1, vec![]));
        lines.push(user_text(
            &notification(&["bpersist1"], Some("killed")),
            T + 3,
        ));
        assert_eq!(tasks(&replay(&lines)), 0);

        // A `taskId` from another tool opens nothing.
        let other = [
            call("toolu_2", "TaskCreate", json!({"subject": "<str 9>"}), T),
            result("toolu_2", json!({"taskId": "7"}), T + 1),
        ];
        assert_eq!(tasks(&replay(&other)), 0);
        assert_eq!(ends(&replay(&other)), Vec::<i64>::new());
    }

    /// Review M1: a Monitor result of a shape 2.1.317 does not write never holds for the life of the process.
    #[test]
    fn a_monitor_of_another_shape_still_ends() {
        let r = replay(&[
            call("toolu_1", "Monitor", json!({}), T),
            result("toolu_1", json!({"taskId": "bmon00001"}), T + 1),
        ]);
        assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + HOUR]));
        assert_eq!(r.problem(), Some((TRANSCRIPT, "Monitor result")));
        let r = replay(&[
            call("toolu_1", "Monitor", json!({}), T),
            result(
                "toolu_1",
                json!({"taskId": "bmon00001", "timeoutMs": 0, "persistent": false}),
                T + 1,
            ),
        ]);
        assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + HOUR]));
        // Longer than 2.1.317 allows: a due time every hour from its start, so the core's hour before each covers it.
        let r = replay(&[
            call("toolu_1", "Monitor", json!({}), T),
            result(
                "toolu_1",
                json!({"taskId": "bmon00001", "timeoutMs": 150 * MIN, "persistent": false}),
                T + 1,
            ),
        ]);
        assert_eq!(
            ends(&r),
            [T + 1 + HOUR, T + 1 + 2 * HOUR, T + 1 + 150 * MIN]
        );
        assert_eq!(r.problem(), Some((TRANSCRIPT, "Monitor result")));
        // At most a day of them.
        let r = replay(&[
            call("toolu_1", "Monitor", json!({}), T),
            result(
                "toolu_1",
                json!({"taskId": "bmon00001", "timeoutMs": 100 * DAY, "persistent": false}),
                T + 1,
            ),
        ]);
        assert_eq!(ends(&r).len(), MONITOR_STEPS as usize);
        // A Monitor of an earlier process is gone with it.
        let r = replay(&monitor("toolu_1", "bu1gw9xxc", 120_000, false, T));
        assert_eq!(r.task_timers(T + 2), Vec::<i64>::new());
    }

    fn stop(tool: &str, name: &str, input: Value, stopped: &str, at: i64) -> [String; 2] {
        [
            call(tool, name, input, at),
            result(
                tool,
                json!({"message": "<str 57>", "task_id": stopped, "task_type": "local_bash",
                    "command": "<str 36>"}),
                at + 1,
            ),
        ]
    }

    /// Review B1: Claude Code writes no notification for a task `TaskStop` stops; its result is the end.
    #[test]
    fn task_stop_closes_the_task_it_names() {
        let mut opened = shell("toolu_1", "b1irxlyqr", T).to_vec();
        opened.extend(shell("toolu_2", "b0000000b", T));
        opened.extend(monitor("toolu_3", "bu1gw9xxc", 120_000, false, T));
        opened.extend(monitor("toolu_4", "bpersist1", 0, true, T));
        let r = replay(&opened);
        assert_eq!((tasks(&r), ends(&r).len()), (3, 1));
        for name in ["TaskStop", "KillShell", "KillBash"] {
            let mut lines = opened.clone();
            lines.extend(stop(
                "toolu_9",
                name,
                json!({"task_id": "b1irxlyqr"}),
                "b1irxlyqr",
                T + 5,
            ));
            assert_eq!(tasks(&replay(&lines)), 2, "{name}");
        }
        // The result names the task; the input's `task_id` or older `shell_id` stands in when it does not.
        let mut lines = opened.clone();
        lines.extend(stop(
            "toolu_9",
            "TaskStop",
            json!({"task_id": "bu1gw9xxc"}),
            "bu1gw9xxc",
            T + 5,
        ));
        lines.extend(stop(
            "toolu_10",
            "TaskStop",
            json!({"task_id": "bpersist1"}),
            "bpersist1",
            T + 6,
        ));
        lines.push(call(
            "toolu_11",
            "KillShell",
            json!({"shell_id": "b0000000b"}),
            T + 7,
        ));
        lines.push(result("toolu_11", json!("Shell stopped"), T + 8));
        let r = replay(&lines);
        assert_eq!((tasks(&r), ends(&r).len()), (1, 0));
        assert_eq!(r.problem(), None);
        // A failed stop closes nothing; a stop of a task not known changes nothing.
        let mut lines = opened.clone();
        lines.push(call(
            "toolu_9",
            "TaskStop",
            json!({"task_id": "b1irxlyqr"}),
            T + 5,
        ));
        lines.push(failed("toolu_9", T + 6));
        lines.extend(stop(
            "toolu_10",
            "TaskStop",
            json!({"task_id": "a0ther000"}),
            "a0ther000",
            T + 7,
        ));
        assert_eq!(tasks(&replay(&lines)), 3);
        // Neither the result nor the input names a task: named, nothing closed.
        let mut lines = opened;
        lines.push(call("toolu_9", "TaskStop", json!({}), T + 5));
        lines.push(result("toolu_9", json!({"message": "<str 9>"}), T + 6));
        let r = replay(&lines);
        assert_eq!(tasks(&r), 3);
        assert_eq!(r.problem(), Some((TRANSCRIPT, "TaskStop result")));
    }

    /// Review m1: a background shell whose result has no `backgroundTaskId` would stop holding with no sign, so it is
    /// named. A foreground shell or a background subagent is not.
    #[test]
    fn a_background_shell_result_without_its_task_id_is_named() {
        let named = replay(&[
            call(
                "toolu_1",
                "PowerShell",
                json!({"command": "<str 33>", "run_in_background": true}),
                T,
            ),
            result(
                "toolu_1",
                json!({"stdout": "", "bgTaskId": "b1irxlyqr"}),
                T + 1,
            ),
        ]);
        assert_eq!(
            named.problem(),
            Some((TRANSCRIPT, "background shell result"))
        );
        assert_eq!(tasks(&named), 0);
        let quiet = replay(&[
            shell("toolu_1", "b1irxlyqr", T)[0].clone(),
            failed("toolu_1", T + 1),
            call("toolu_2", "Bash", json!({"command": "<str 9>"}), T + 2),
            result("toolu_2", json!({"stdout": "<str 3>"}), T + 3),
            call(
                "toolu_3",
                "Agent",
                json!({"prompt": "<str 9>", "run_in_background": true}),
                T + 4,
            ),
            result(
                "toolu_3",
                json!({"isAsync": true, "status": "async_launched", "agentId": "a1b2c3"}),
                T + 5,
            ),
        ]);
        assert_eq!(quiet.problem(), None);
        assert_eq!(tasks(&replay(&shell("toolu_1", "b1irxlyqr", T))), 1);
    }

    /// Review m4: Claude Code's own reader takes a notification after its header, and several joined after it.
    #[test]
    fn a_notification_after_the_header_and_joined_ones_close_their_tasks() {
        const DWE: &str = "[SYSTEM NOTIFICATION - NOT USER INPUT]\nThis is an automated background-task event, NOT a message from the user.\nDo NOT interpret this as user acknowledgement.\nNo human input has been received since the last genuine user message.\n\n";
        let mut opened = Vec::new();
        for (n, task) in ["b0000000a", "b0000000b", "b0000000c"].iter().enumerate() {
            opened.extend(shell(&format!("toolu_{n}"), task, T));
        }
        let one = notification(&["b0000000a"], Some("completed"));
        let two = notification(&["b0000000b"], Some("killed"));
        let event = "<task-notification>\n<task-id>b0000000c</task-id>\n<summary><str 9></summary>\n<event><str 5></event>\n</task-notification>";
        for (text, open) in [
            (format!("{DWE}{one}"), 2),
            (format!("{DWE}{one}\n\n{two}"), 1),
            (format!("{DWE}{event}\n\n{one}\n\n{two}"), 1),
            (
                format!("<system-reminder>\n{DWE}{one}\n</system-reminder>"),
                2,
            ),
            // Without the header only the first one counts, as Claude Code reads it.
            (format!("{one}\n\n{two}"), 2),
            // Text that only quotes the header or a notification closes nothing.
            (format!("<str 9> {DWE}{one}"), 3),
            (format!("{DWE}<str 9>\n\n{one}"), 3),
        ] {
            let mut lines = opened.clone();
            lines.push(user_text(&text, T + 5));
            assert_eq!(tasks(&replay(&lines)), open, "{text}");
            let mut lines = opened.clone();
            lines.push(attachment(
                json!({"type": "queued_command", "prompt": text}),
                T + 5,
            ));
            assert_eq!(tasks(&replay(&lines)), open, "queued: {text}");
        }
    }

    /// Review m2: a computed recurring time also comes as the latest any flag value allows, and the real fire, early or
    /// late, falls inside the core's window of one of them.
    #[test]
    fn a_computed_recurring_time_covers_every_flag_value() {
        let lead = crate::policy::sessions::TIMER_LEAD_MS;
        let margin = crate::policy::sessions::TIMER_MARGIN_MS;
        for (id, cron) in [
            ("bd9c33c4", "*/1 * * * *"),
            ("ffffffff", "*/5 * * * *"),
            ("80000000", "0 * * * *"),
            ("ffffffff", "0 9 * * *"),
            ("00000001", "*/15 * * * *"),
        ] {
            let lines = [
                create("toolu_1", cron, true, T),
                created("toolu_1", id, true, (T + MIN) as f64, T + 1),
                fire(id, cron, None, T + MIN),
            ];
            let due = timers(&replay(&lines));
            let parsed = Cron::parse(cron).unwrap();
            let job = ClaudeJob {
                id,
                cron: &parsed,
                recurring: true,
                from_ms: T + MIN,
            };
            for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for cap in [0, MIN, 30 * MIN] {
                    for cache_lead in [0, 1, 15_000, MIN] {
                        let flag = ClaudeScheduler {
                            recurring_frac: frac,
                            recurring_cap_ms: cap,
                            cache_lead_ms: cache_lead,
                            ..ClaudeScheduler::DEFAULT
                        };
                        let real = flag.next_fire_ms(&job, &ZONE).unwrap();
                        assert!(
                            due.iter().any(|&d| d - lead <= real && real < d + margin),
                            "{cron} {frac} {cap} {cache_lead}: {real} {due:?}"
                        );
                    }
                }
            }
        }
    }

    /// Review m2: a `nextFireAt` the built-in values do not give is kept as a drift, for a live check.
    #[test]
    fn a_next_fire_the_built_in_scheduler_does_not_give_is_a_drift() {
        let right = next_fire("bd9c33c4", "*/1 * * * *", true, T + 664);
        let r = replay(&[
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, right as f64, T + 664),
        ]);
        assert_eq!(r.scheduler_drift_ms(), None);
        let r = replay(&[
            create("toolu_1", "*/1 * * * *", true, T),
            created(
                "toolu_1",
                "bd9c33c4",
                true,
                (right + 20_000) as f64,
                T + 664,
            ),
        ]);
        assert_eq!(r.scheduler_drift_ms(), Some(20_000));
        assert_eq!(timers(&r), [right + 20_000]);
        assert_eq!(r.problem(), None);
    }

    /// Review m2: a job dropped at the built-in 7 day age comes back when Claude Code fires it again, and then only its
    /// fire lines end it.
    #[test]
    fn a_job_fired_after_its_age_comes_back() {
        let mut lines = vec![
            create("toolu_1", "0 * * * *", true, T),
            created("toolu_1", "00000000", true, (T + HOUR) as f64, T),
            fire("00000000", "0 * * * *", None, T + 7 * DAY + 50 * MIN),
        ];
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
        let again = T + 7 * DAY + 110 * MIN;
        lines.push(fire("00000000", "0 * * * *", None, again));
        let r = replay(&lines);
        assert_eq!(
            timers(&r)[0],
            next_fire("00000000", "0 * * * *", true, again)
        );
        // It no longer ends at its age, nor at a resume.
        lines.push(fire("00000000", "0 * * * *", None, again + HOUR));
        assert_eq!(
            timers(&replay(&lines)),
            [next_fire("00000000", "0 * * * *", true, again + HOUR)]
        );
        let resumed = again + 2 * HOUR;
        assert_eq!(
            replay(&lines).timers(resumed, &ZONE),
            [next_fire("00000000", "0 * * * *", true, resumed)]
        );
        // A delete still ends it.
        lines.push(call(
            "toolu_2",
            "CronDelete",
            json!({"id": "00000000"}),
            again + 2 * HOUR,
        ));
        lines.push(result(
            "toolu_2",
            json!({"id": "00000000"}),
            again + 2 * HOUR + 1,
        ));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    }

    /// Claude Code 2.1.317 marks the shells a dead process left in one notification with one `<task-id>` each.
    #[test]
    fn a_batch_notification_closes_every_listed_task() {
        let mut lines = Vec::new();
        for (n, task) in ["b0000000a", "b0000000b", "b0000000c"].iter().enumerate() {
            lines.extend(shell(&format!("toolu_{n}"), task, T));
        }
        lines.push(user_text(
            "<task-notification>\n<task-id>b0000000a</task-id>\n<task-id>b0000000b</task-id>\n<status>stopped</status>\n<summary><str 40></summary>\n<note><str 300></note>\n</task-notification>",
            T + 5,
        ));
        assert_eq!(tasks(&replay(&lines)), 1);
    }

    /// Live fixture 07: a notification that came during a busy turn is only an attachment.
    #[test]
    fn a_queued_command_notification_closes_its_task() {
        let mut lines = shell("toolu_1", "ad8c97644", T).to_vec();
        lines.push(attachment(
            json!({"type": "queued_command", "prompt": notification(&["ad8c97644"], Some("completed")),
                "source_uuid": "5e550029-0000-4000-8000-000000000029", "commandMode": "<str 17>"}),
            T + 2,
        ));
        assert_eq!(tasks(&replay(&lines)), 0);
    }

    fn queued(operation: &str, content: Option<&str>, at: i64) -> String {
        let mut fields = json!({"type": "queue-operation", "operation": operation});
        if let Some(content) = content {
            fields["content"] = json!(content);
        }
        line(fields, at)
    }

    /// Review M1 part b: Claude Code writes a notification as a `queue-operation` `enqueue` line when it queues it. A
    /// stop from the UI queues its `killed` notice as passive (2.1.317, `tUr`), so the user line may come only at the
    /// next prompt; the queued line ends the task at once. Not checked live for a UI stop (`p4-live-claude`).
    #[test]
    fn a_queued_notice_ends_its_task_at_once() {
        let killed = notification(&["b1irxlyqr"], Some("killed"));
        let opened = shell("toolu_1", "b1irxlyqr", T);
        let mut lines = opened.to_vec();
        lines.push(queued("enqueue", Some(&killed), T + 2));
        let r = replay(&lines);
        assert_eq!(tasks(&r), 0);
        assert_eq!(r.problem(), None);
        // Its later delivery changes nothing.
        lines.push(queued("dequeue", None, T + 3));
        lines.push(user_text(&killed, T + 4));
        assert_eq!(tasks(&replay(&lines)), 0);

        // A Monitor's timer ends too, persistent or not, and so does a notice after Claude Code's header.
        let header = format!("[SYSTEM NOTIFICATION - NOT USER INPUT]\n<str 64>\n\n{killed}");
        for persistent in [false, true] {
            let mut lines = monitor("toolu_2", "b1irxlyqr", 120_000, persistent, T).to_vec();
            lines.push(queued("enqueue", Some(&header), T + 2));
            let r = replay(&lines);
            assert_eq!((tasks(&r), ends(&r)), (0, vec![]), "{persistent}");
        }

        // Other operations, a notice with no `<status>`, a notice not at the start, a typed prompt and no content end
        // nothing and name nothing.
        let event = "<task-notification>\n<task-id>b1irxlyqr</task-id>\n<summary><str 70></summary>\n<event><str 5></event><str 123></task-notification>";
        let quoted = format!("<str 29> {killed}");
        for (operation, content) in [
            ("remove", Some(killed.as_str())),
            ("dequeue", None),
            ("enqueue", Some(event)),
            ("enqueue", Some(quoted.as_str())),
            ("enqueue", Some("<str 29>")),
            ("enqueue", None),
        ] {
            let mut lines = opened.to_vec();
            lines.push(queued(operation, content, T + 2));
            let r = replay(&lines);
            assert_eq!(tasks(&r), 1, "{operation} {content:?}");
            assert_eq!(r.problem(), None);
        }
        let mut lines = opened.to_vec();
        lines.push(line(
            json!({"type": "queue-operation", "operation": 5, "content": killed}),
            T + 2,
        ));
        lines.push(line(
            json!({"type": "queue-operation", "operation": "enqueue", "content": [killed]}),
            T + 2,
        ));
        assert_eq!(tasks(&replay(&lines)), 1);
    }

    #[test]
    fn only_a_notification_at_the_start_of_the_text_counts() {
        let closing = notification(&["b1irxlyqr"], Some("completed"));
        let mut lines = shell("toolu_1", "b1irxlyqr", T).to_vec();
        lines.push(user_text(&format!("<str 8206> {closing}"), T + 2));
        lines.push(user_text(&format!(" {closing}"), T + 3));
        // A `<status>` after the `<summary>` is the summary's text.
        lines.push(user_text(
            "<task-notification>\n<task-id>b1irxlyqr</task-id>\n<summary><str 8></summary>\n<status>completed</status>\n</task-notification>",
            T + 4,
        ));
        assert_eq!(tasks(&replay(&lines)), 1);
        // The same text as a text item of a list closes it.
        lines.push(line(
            json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": closing}]}}),
            T + 5,
        ));
        assert_eq!(tasks(&replay(&lines)), 0);
    }

    /// Live fixtures 09 to 11 and 13: lines older than the process are an earlier process's (resume).
    #[test]
    fn lines_of_an_earlier_process_follow_the_resume_rules() {
        let resumed = T + HOUR;
        let mut lines = vec![
            create("toolu_1", "*/5 * * * *", true, T),
            created("toolu_1", "f9471e8f", true, (T + 285_000) as f64, T + 1),
            create("toolu_2", "0 9 * * *", true, T - 8 * DAY),
            created(
                "toolu_2",
                "aaaa0002",
                true,
                (T - 7 * DAY) as f64,
                T - 8 * DAY,
            ),
            create("toolu_3", "50 18 3 11 *", false, T + 2),
            created("toolu_3", "aaaa0003", false, (T + 30 * MIN) as f64, T + 3),
            create("toolu_4", "40 19 3 11 *", false, T + 4),
            created("toolu_4", "aaaa0004", false, (T + 80 * MIN) as f64, T + 5),
            wakeup("toolu_5", 3_600, T + 6),
            woken("toolu_5", T + 2 * HOUR, T + 7),
        ];
        lines.extend(shell("toolu_6", "b01yw8588", T + 8));
        let r = replay(&lines);
        assert_eq!(r.open_tasks(T), 1);
        assert_eq!(r.open_tasks(resumed), 0);
        let mut old = vec![
            T + 285_000,
            T - 7 * DAY,
            T + 30 * MIN,
            T + 80 * MIN,
            T + 2 * HOUR,
        ];
        old.sort_unstable();
        assert_eq!(r.timers(T - 8 * DAY, &ZONE), old);
        // At the resume: the `*/5` job counts from it; the 8-day-old job, the missed one-shot and the wakeup are gone.
        assert_eq!(
            r.timers(resumed, &ZONE),
            [
                resumed + 285_000,
                latest("f9471e8f", "*/5 * * * *", resumed),
                T + 80 * MIN
            ]
        );
        // A fire of this process counts from the fire again.
        let fired = resumed + 285_500;
        lines.push(fire("f9471e8f", "*/5 * * * *", None, fired));
        assert_eq!(
            replay(&lines).timers(resumed, &ZONE),
            [
                fired + 285_000,
                latest("f9471e8f", "*/5 * * * *", fired),
                T + 80 * MIN
            ]
        );
    }

    /// Track note 5: an unknown cron syntax holds only for the exact time it came with.
    #[test]
    fn a_cron_that_does_not_parse_holds_only_for_its_exact_time() {
        let mut lines = vec![
            create("toolu_1", "@hourly", true, T),
            created("toolu_1", "aaaa0001", true, (T + HOUR) as f64, T + 1),
        ];
        let r = replay(&lines);
        assert_eq!(timers(&r), [T + HOUR]);
        assert_eq!(r.problem(), Some((CRON, "@hourly")));
        // Nor does it count at a resume, or after its fire.
        assert_eq!(r.timers(T + 2, &ZONE), Vec::<i64>::new());
        lines.push(fire("aaaa0001", "@hourly", None, T + HOUR + 500));
        assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());

        let long = "0 0 1 1 * ".repeat(10) + "\u{0531}";
        let r = replay(&[
            create("toolu_2", &long, true, T),
            created("toolu_2", "aaaa0002", true, (T + HOUR) as f64, T + 1),
        ]);
        assert_eq!(
            r.problem(),
            Some((CRON, "0 0 1 1 * 0 0 1 1 * 0 0 1 1 * 0 "))
        );
    }

    /// Track note 6: a line of a known kind without a field it needs is skipped and named, never guessed.
    #[test]
    fn a_known_kind_with_a_missing_field_is_skipped_and_named() {
        let made = || create("toolu_1", "*/1 * * * *", true, T);
        let cases: Vec<(Vec<String>, &str)> = vec![
            (
                vec![
                    made(),
                    result("toolu_1", json!({"humanSchedule": "Every minute", "nextFireAt": 1.0}), T),
                ],
                "CronCreate result",
            ),
            (vec![made(), result("toolu_1", json!("Created"), T)], "CronCreate result"),
            (
                vec![call("toolu_1", "CronCreate", json!({"prompt": "<str 29>"}), T)],
                "CronCreate",
            ),
            (vec![call("toolu_1", "CronDelete", json!({}), T)], "CronDelete"),
            (vec![call("toolu_1", "CronDelete", json!({"id": "a b"}), T)], "CronDelete"),
            (
                vec![line(
                    json!({"type": "assistant", "message": {"content": [{"type": "tool_use",
                        "name": "ScheduleWakeup", "input": {}}]}}),
                    T,
                )],
                "ScheduleWakeup",
            ),
            (
                vec![
                    wakeup("toolu_1", 60, T),
                    result("toolu_1", json!({"clampedDelaySeconds": 60}), T),
                ],
                "ScheduleWakeup result",
            ),
            (
                vec![
                    call("toolu_1", "Monitor", json!({}), T),
                    result("toolu_1", json!({"timeoutMs": 1}), T),
                ],
                "Monitor result",
            ),
            (
                vec![line(
                    json!({"type": "system", "subtype": "scheduled_task_fire", "cron": "*/1 * * * *"}),
                    T,
                )],
                "scheduled_task_fire",
            ),
            (
                vec![
                    json!({"type": "system", "subtype": "scheduled_task_fire", "taskId": "bd9c33c4"})
                        .to_string(),
                ],
                "scheduled_task_fire",
            ),
            (
                vec![attachment(
                    json!({"type": "session_cron_carry", "otherArmed": []}),
                    T,
                )],
                "session_cron_carry",
            ),
            (
                vec![attachment(
                    carried(json!([{"id": "aaaa0001", "cron": "*/1 * * * *"}]), None, None, &[]),
                    T,
                )],
                "session_cron_carry",
            ),
            (
                vec![attachment(carried(json!(["aaaa0001"]), None, None, &[]), T)],
                "session_cron_carry",
            ),
            (
                vec![attachment(
                    carried(json!([]), None, Some(json!({"prompt": "<str 1>"})), &[]),
                    T,
                )],
                "session_cron_carry",
            ),
            (
                vec![
                    made(),
                    json!({"type": "user", "message": {"content": [{"type": "tool_result",
                        "tool_use_id": "toolu_1"}]}, "toolUseResult": {"id": "aaaa0001", "nextFireAt": 1.0}})
                    .to_string(),
                ],
                "timestamp",
            ),
            (
                vec![
                    shell("toolu_1", "b1", T)[0].clone(),
                    result("toolu_1", json!({"backgroundTaskId": "b 1"}), T),
                ],
                "task id",
            ),
        ];
        for (lines, kind) in cases {
            let r = replay(&lines);
            assert_eq!(r.problem(), Some((TRANSCRIPT, kind)), "{lines:?}");
            assert_eq!(timers(&r), Vec::<i64>::new(), "{lines:?}");
            assert_eq!(tasks(&r), 0, "{lines:?}");
        }
        // A missing `nextFireAt` is named too, but the job still counts from its creation.
        let r = replay(&[
            made(),
            result(
                "toolu_1",
                json!({"id": "bd9c33c4", "recurring": true}),
                T + 1,
            ),
        ]);
        assert_eq!(r.problem(), Some((TRANSCRIPT, "CronCreate result")));
        assert_eq!(
            timers(&r),
            [
                next_fire("bd9c33c4", "*/1 * * * *", true, T + 1),
                latest("bd9c33c4", "*/1 * * * *", T + 1)
            ]
        );
    }

    /// Track note 6: transcripts have many line kinds, and the names read here mean other things in some of them.
    #[test]
    fn other_kinds_and_shapes_are_ignored_silently() {
        let lines = [
            line(
                json!({"type": "tool_use_summary", "subtype": "scheduled_task_fire", "taskId": 5}),
                T,
            ),
            attachment(
                json!({"type": "task_status", "taskType": "local_bash", "status": "running",
                    "tasks": {"b1": 1}, "prompt": 7}),
                T,
            ),
            call(
                "toolu_1",
                "TaskUpdate",
                json!({"id": 5, "cron": ["x"], "recurring": "yes"}),
                T,
            ),
            result("toolu_1", json!(["tool_use", 1, null, {"id": {}}]), T),
            json!({"type": "user", "message": {"role": "user", "content": {"type": "tool_use"}},
                "toolUseResult": null, "timestamp": 5})
            .to_string(),
            line(json!({"type": "assistant", "message": "tool_use"}), T),
            line(
                json!({"type": "queue-operation", "operation": "remove",
                    "content": notification(&["b1irxlyqr"], Some("completed"))}),
                T,
            ),
            json!({"type": "progress", "data": {"tool_use": [1, 2, {"nested": [[[]]]}]}})
                .to_string(),
            "{\"type\":\"user\"}".to_owned(),
            String::new(),
            "not json, no marker".to_owned(),
        ];
        let r = replay(&lines);
        assert_eq!(r.problem(), None);
        assert_eq!(timers(&r), Vec::<i64>::new());
        assert_eq!(tasks(&r), 0);
    }

    #[test]
    fn a_broken_line_that_may_matter_is_named() {
        let whole = fire("bd9c33c4", "*/1 * * * *", None, T);
        let cut = whole.find("scheduled_task_fire").unwrap() + 24;
        let r = replay(&[whole[..cut].to_owned(), whole.clone()]);
        assert_eq!(r.problem(), Some((TRANSCRIPT, "line")));
        // A broken line that names nothing that matters says nothing.
        let r = replay(&["{\"type\":\"last-prompt\",\"lastPrompt\":\"<str".to_owned()]);
        assert_eq!(r.problem(), None);
        // Not an object.
        let r = replay(&["\"scheduled_task_fire\"".to_owned()]);
        assert_eq!(r.problem(), Some((TRANSCRIPT, "line")));
    }

    #[test]
    fn the_first_problem_is_kept() {
        let r = replay(&[
            call("toolu_1", "CronDelete", json!({}), T),
            call("toolu_2", "CronCreate", json!({}), T),
        ]);
        assert_eq!(r.problem(), Some((TRANSCRIPT, "CronDelete")));
    }

    #[test]
    fn at_most_50_jobs() {
        let mut lines = Vec::new();
        for n in 0..60 {
            let tool = format!("toolu_{n}");
            lines.push(create(&tool, "0 9 * * *", true, T + n));
            lines.push(created(
                &tool,
                &format!("{n:08x}"),
                true,
                (T + DAY + n) as f64,
                T + n,
            ));
        }
        assert_eq!(timers(&replay(&lines)).len(), MAX_JOBS);
    }

    #[test]
    fn a_partial_line_waits_for_its_newline() {
        let made = format!(
            "{}\n{}",
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1)
        );
        let mut r = Replay::default();
        r.feed(made.as_bytes(), &ZONE);
        assert_eq!(timers(&r), Vec::<i64>::new());
        r.feed(b"\n", &ZONE);
        assert_eq!(timers(&r), [T + MIN]);

        // After `/clear` the new file starts clean.
        let mut r = Replay::default();
        r.feed(
            b"{\"type\":\"system\",\"subtype\":\"scheduled_task_f",
            &ZONE,
        );
        r.start_file();
        r.feed(format!("{made}\n").as_bytes(), &ZONE);
        assert_eq!(timers(&r), [T + MIN]);
        assert_eq!(r.problem(), None);
    }

    #[test]
    fn a_line_too_long_is_skipped() {
        let mut r = Replay::default();
        let mut huge = b"{\"type\":\"user\",\"tool_use\":\"".to_vec();
        huge.resize(MAX_LINE + 10, b'a');
        r.feed(&huge, &ZONE);
        r.feed(b"aaaa\"}\n", &ZONE);
        let made = format!(
            "{}\n{}\n",
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1)
        );
        r.feed(made.as_bytes(), &ZONE);
        assert_eq!(timers(&r), [T + MIN]);
        assert_eq!(r.problem(), None);
        assert!(r.partial.capacity() < 1 << 20);
    }

    /// Acceptance check: no prompt, text, output or summary is kept. Each one here is the same marked text, and the
    /// kept state shows none of it.
    #[test]
    fn the_replay_keeps_no_text() {
        let secret = "SECRET";
        let lines = [
            call(
                "toolu_1",
                "CronCreate",
                json!({"cron": "*/1 * * * *", "prompt": secret, "recurring": true}),
                T,
            ),
            created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
            call(
                "toolu_2",
                "Bash",
                json!({"command": secret, "run_in_background": true}),
                T,
            ),
            result(
                "toolu_2",
                json!({"stdout": secret, "stderr": secret, "backgroundTaskId": "b1irxlyqr"}),
                T,
            ),
            user_text(
                &format!(
                    "<task-notification>\n<task-id>b0ther000</task-id>\n<output-file>{secret}</output-file>\n<status>completed</status>\n<summary>{secret}</summary>\n</task-notification>"
                ),
                T,
            ),
            user_text(secret, T),
            line(
                json!({"type": "assistant", "message": {"content": [{"type": "text", "text": secret},
                    {"type": "thinking", "thinking": secret}]}}),
                T,
            ),
            attachment(
                carried(
                    json!([{"id": "aaaa0001", "cron": "0 9 * * *", "prompt": secret, "createdAt": T,
                        "recurring": true}]),
                    Some(json!([job("bd9c33c4", "*/1 * * * *", T, true)])),
                    Some(json!({"prompt": secret, "scheduledFor": T + HOUR, "reason": secret})),
                    &[],
                ),
                T + 2,
            ),
            attachment(json!({"type": "queued_command", "prompt": secret}), T + 3),
            fire("aaaa0001", "0 9 * * *", None, T + 4).replace("<str 29>", secret),
            queued(
                "enqueue",
                Some(&format!(
                    "<task-notification>\n<task-id>b0ther001</task-id>\n<output-file>{secret}</output-file>\n<status>killed</status>\n<summary>{secret}</summary>\n</task-notification>"
                )),
                T + 4,
            ),
            queued(
                "enqueue",
                Some(&format!("<task-notification>{secret}")),
                T + 4,
            ),
        ];
        let mut r = replay(&lines);
        assert_eq!(r.problem(), None);
        assert_eq!(timers(&r).len(), 3);
        assert_eq!(tasks(&r), 1);
        // A partial line waiting for its newline is held, but not shown.
        let held = user_text(secret, T + 5);
        let end = held.find(secret).unwrap() + secret.len();
        r.feed(&held.as_bytes()[..end], &ZONE);
        assert!(contains(&r.partial, secret.as_bytes()));
        let kept = format!("{r:?}");
        assert!(!kept.contains(secret), "{kept}");
        for id in ["bd9c33c4", "aaaa0001", "b1irxlyqr"] {
            assert!(kept.contains(id), "{kept}");
        }
    }

    fn sample() -> Vec<String> {
        let mut lines = vec![
            create("toolu_1", "*/1 * * * *", true, T),
            created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
            fire("bd9c33c4", "*/1 * * * *", None, T + MIN),
            wakeup("toolu_2", 60, T + MIN),
            woken("toolu_2", T + 3 * MIN, T + MIN + 1),
            call("toolu_3", "CronDelete", json!({}), T + 2 * MIN),
            attachment(
                carried(
                    json!([job("bd9c33c4", "*/1 * * * *", T + 1, true)]),
                    None,
                    None,
                    &[],
                ),
                T + 2 * MIN,
            ),
            "{\"type\":\"user\",\"tool_use\":broken".to_owned(),
        ];
        lines.extend(shell("toolu_4", "b1irxlyqr", T));
        lines.extend(shell("toolu_5", "b0000000b", T + 3));
        lines.extend(shell("toolu_6", "b0000000c", T + 4));
        lines.push(user_text(
            &notification(&["b1irxlyqr"], Some("completed")),
            T + 2 * MIN,
        ));
        lines.push(queued(
            "enqueue",
            Some(&notification(&["b0000000c"], Some("killed"))),
            T + 2 * MIN,
        ));
        lines
    }

    proptest! {
        /// Deliverable 6: the bytes may come in any pieces; the state is that of the whole file.
        #[test]
        fn any_split_gives_the_same_state(cuts in proptest::collection::vec(any::<prop::sample::Index>(), 0..12)) {
            let text: String = sample().iter().map(|line| format!("{line}\r\n")).collect();
            let bytes = text.as_bytes();
            let whole = replay(&sample());
            let mut at: Vec<usize> = cuts.iter().map(|cut| cut.index(bytes.len() + 1)).collect();
            at.sort_unstable();
            let mut pieces = Replay::default();
            let mut start = 0;
            for end in at.into_iter().chain([bytes.len()]) {
                pieces.feed(&bytes[start..end], &ZONE);
                start = end;
            }
            prop_assert_eq!(timers(&pieces), timers(&whole));
            prop_assert_eq!(tasks(&pieces), tasks(&whole));
            prop_assert_eq!(pieces.problem(), whole.problem());
            prop_assert_eq!(format!("{:?}", pieces.jobs), format!("{:?}", whole.jobs));
        }

        #[test]
        fn any_bytes_feed_without_a_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let mut r = Replay::default();
            r.feed(&bytes, &ZONE);
            r.feed(b"\n", &ZONE);
            let _ = (r.timers(i64::MIN, &ZONE), r.open_tasks(0), r.problem());
        }

        /// A line that matters, with any one value changed, reads without a panic and keeps only plain ids.
        #[test]
        fn changed_values_never_panic(value in prop_oneof![
            Just(json!(null)), any::<i64>().prop_map(|n| json!(n)), any::<f64>().prop_map(|n| json!(n)),
            "\\PC{0,80}".prop_map(|s| json!(s)), Just(json!([])), Just(json!({})), Just(json!(true)),
        ], which in 0usize..7) {
            let mut made = json!({"id": "bd9c33c4", "humanSchedule": "Every minute", "recurring": true,
                "durable": false, "nextFireAt": 1.0});
            let mut input = json!({"cron": "*/1 * * * *", "prompt": "<str 29>", "recurring": true});
            match which {
                0..=3 => made[["id", "recurring", "durable", "nextFireAt"][which]] = value.clone(),
                _ => input[["cron", "recurring", "prompt"][which - 4]] = value.clone(),
            }
            let entry = json!({"id": value.clone(), "cron": value.clone(), "createdAt": value.clone(),
                "recurring": value.clone()});
            let r = replay(&[
                call("toolu_1", "CronCreate", input, T),
                result("toolu_1", made, T + 1),
                attachment(carried(json!([value]), Some(json!([entry])), None, &[]), T + 2),
            ]);
            for id in r.jobs.keys() {
                prop_assert!(is_plain_id(id));
            }
            let _ = r.timers(i64::MIN, &ZONE);
        }
    }
}
