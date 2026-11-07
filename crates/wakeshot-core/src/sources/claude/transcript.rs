//! Claude Code transcripts: timers and background tasks.
//!
//! Rules 3, 5 and 6 of `docs/research/claude-code-signals.md`, with the shapes seen live, checked against Claude Code
//! 2.1.317's own code (`docs/research.md`, "Claude Code transcripts: cron carry, resume restore and notifications"). A
//! [`Replay`] reads one session process's transcript lines as they are appended and keeps only ids, cron texts, times
//! and flags: prompts and other text are read through visitors that look at them in place and keep nothing
//! (`notice_ids`), and every field of an unrelated line kind is skipped without being kept.
//!
//! Times on lines older than the session process's start belong to an earlier process (`claude --resume` reuses the
//! transcript and writes no line for it, seen live). The replay keeps every line's time and applies the resume rules
//! only when it reports, so one replay serves any process that takes the transcript over.

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

/// What `FormatHealth::UnknownValue` names: a transcript line kind, or a cron text [`Cron`] does not read.
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
    /// `None` when Claude Code's cron syntax changed: only an exact time from the transcript then counts.
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
    /// at once (not checked live). A notice with a `<status>` is queued only after its task ended, so this never ends a
    /// running task.
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
            // Without its id the shell would not hold at all, and nothing else would say so.
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
            // A notification that came while the main thread was busy (live fixture 07).
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
    /// twice: as the built-in scheduler values give it, and as the latest any value of Claude Code's remote flag
    /// allows. The real fire then always falls inside the core's window, and its fire line moves the job on, so with
    /// the built-in values the hold is the same as with one time.
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
/// joined, each after a blank line (2.1.317, `qi`).
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
mod tests;
