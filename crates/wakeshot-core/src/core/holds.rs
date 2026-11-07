//! Agent sessions and hold files to the holds that count now, through `policy::sessions` and `policy::hold_files`,
//! with the Keep awake length the menu checks.
//!
//! Owner: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`.

use std::collections::{BTreeMap, BTreeSet};

use super::nearest;
use crate::command::Command;
use crate::holds::{HoldFacts, HoldKind};
use crate::policy::sessions::{TIMER_LEAD_MS, TIMER_MARGIN_MS};
use crate::policy::{hold_files, sessions};
use crate::present::{self, Holder};
use crate::process::Alive;
use crate::sources::{Activity, FormatHealth, SessionFacts, SourceId};
use crate::state::{FileHold, KeepAwakeMemo, Session, State};
use crate::time::{Deadline, Now};
use crate::view::KeepAwake;

/// A format problem is logged at most once a minute per source (architecture section 8).
const FORMAT_LOG_EVERY_MS: u64 = 60_000;

/// How much of an unknown value the log keeps: enough to report, too little to carry text a file should not leak.
const VALUE_CHARS: usize = 40;

/// What holds at one `Now`.
#[derive(Debug)]
pub(super) struct Found {
    /// Every hold that counts, of any kind.
    pub holders: Vec<Holder>,
    /// Keep screen on is asked for, by a Keep awake hold that counts.
    pub screen: bool,
    /// The Keep awake row the menu checks.
    pub keep_awake: KeepAwake,
    /// Agents whose format wants "update Wakeshot".
    pub update: Vec<&'static str>,
    /// When the answer next changes on its own: a timer window, the waiting hour, an `expires`, the next minute of
    /// Keep awake's time left.
    pub next: Option<Deadline>,
    /// One log line per session or hold file that started or stopped holding at this `Now`, with the reason
    /// (architecture section 8).
    pub lines: Vec<String>,
}

/// A full snapshot of one source replaces the last one. A session keeps its memory while it stays in the snapshots
/// and loses it when it leaves.
pub(super) fn take_sources(
    state: &mut State,
    now: Now,
    source: SourceId,
    snapshot: Vec<(SessionFacts, Alive)>,
    out: &mut Vec<Command>,
) {
    let mut old = state.sessions.remove(&source).unwrap_or_default();
    let seen_before = old.len();
    let mut problems = BTreeSet::new();
    let mut label = None;
    let mut new = BTreeMap::new();
    for (facts, alive) in snapshot {
        if let Some(problem) = problem(&facts.format) {
            problems.insert(problem);
            label = Some(facts.label);
        }
        let (memory, held) = old
            .remove(&facts.key)
            .map(|s| (s.memory, s.held))
            .unwrap_or_default();
        new.insert(
            facts.key.clone(),
            Session {
                facts,
                alive,
                memory,
                held,
            },
        );
    }
    // How many sessions the agent's files show, at each change: a session that never holds gets no line of its own,
    // and "was it seen at all" is the first question about an agent that did not hold.
    if new.len() != seen_before {
        let agent = new
            .values()
            .chain(old.values())
            .next()
            .map(|s| s.facts.label);
        let count = match new.len() {
            1 => "1 session".to_owned(),
            n => format!("{n} sessions"),
        };
        out.push(Command::Log(format!(
            "{}: {count} in its files",
            agent.unwrap_or("An agent")
        )));
    }
    if !new.is_empty() {
        state.sessions.insert(source, new);
    }
    // A session that leaves the snapshot while it holds gets its end line here: `evaluate` no longer sees it.
    for session in old.values().filter(|s| s.held) {
        out.push(Command::Log(format!(
            "{} stops holding: it is gone from the agent's files",
            session_name(&session.facts)
        )));
    }
    let Some(label) = label else {
        return;
    };
    let due = state
        .format_logged_mono
        .get(&source)
        .is_none_or(|&at| now.mono_ms.saturating_sub(at) >= FORMAT_LOG_EVERY_MS);
    if due {
        state.format_logged_mono.insert(source, now.mono_ms);
        let problems: Vec<String> = problems.into_iter().collect();
        out.push(Command::Log(format!(
            "{label}: format problem: {}",
            problems.join("; ")
        )));
    }
}

/// Field names and values only, never file content (`FormatHealth`).
fn problem(format: &FormatHealth) -> Option<String> {
    match format {
        FormatHealth::Ok => None,
        FormatHealth::Unreadable(what) => Some(format!("cannot read {what}")),
        // Debug quoting escapes control characters, so a value cannot split the log line.
        FormatHealth::UnknownValue(field, value) => {
            let value: String = value.chars().take(VALUE_CHARS).collect();
            Some(format!("unknown {field} {value:?}"))
        }
    }
}

/// A full `Holds` snapshot replaces the last one; a Gone file gets one `DeleteHold` until it is answered.
pub(super) fn take_files(
    state: &mut State,
    now: Now,
    snapshot: Vec<HoldFacts>,
    out: &mut Vec<Command>,
) {
    let mut new = BTreeMap::new();
    for facts in snapshot {
        let (last, counted) = state
            .files
            .get(&facts.id)
            .map_or((Alive::Unknown, false), |f| (f.alive, f.counted));
        let answer = hold_files::decide(&facts, last, state.automatic, now);
        let ours = facts.kind == HoldKind::KeepAwake && state.keep_awake_written;
        if answer.delete && !ours && state.deleting.insert(facts.id.clone()) {
            out.push(Command::DeleteHold(facts.id.clone()));
        }
        new.insert(
            facts.id.clone(),
            FileHold {
                facts,
                alive: answer.alive,
                counted,
            },
        );
    }
    state.deleting.retain(|id| new.contains_key(id));
    for (id, file) in &state.files {
        if file.counted && !new.contains_key(id) {
            out.push(Command::Log(format!(
                "Hold file {} stops counting: the file is gone",
                id.0
            )));
        }
    }
    state.files = new;
}

/// Every session and hold file decided at `now`, their memories kept for the next call. Automatic off counts no
/// agent, so their deadlines and "update Wakeshot" wait until it is on again.
pub(super) fn evaluate(state: &mut State, now: Now) -> Found {
    let automatic = state.automatic;
    let mut found = Found {
        holders: Vec::new(),
        screen: false,
        keep_awake: KeepAwake::Off,
        update: Vec::new(),
        next: None,
        lines: Vec::new(),
    };
    let agents = hold_files::kind_counts(HoldKind::Agent, automatic);
    for session in state.sessions.values_mut().flat_map(BTreeMap::values_mut) {
        let answer = sessions::decide(&session.facts, session.alive, &session.memory, now);
        session.memory = answer.memory;
        let holds = agents && answer.holds;
        if holds != session.held {
            session.held = holds;
            let why = session_why(&session.facts, session.memory.alive, holds, agents, now);
            let verb = if holds { "holds" } else { "stops holding" };
            found
                .lines
                .push(format!("{} {verb}: {why}", session_name(&session.facts)));
        }
        if !agents {
            continue;
        }
        if answer.update_notice {
            found.update.push(answer.label);
        }
        if answer.holds {
            found.holders.push(Holder {
                kind: HoldKind::Agent,
                label: Some(answer.label.to_owned()),
                expires_wall_ms: None,
            });
        }
        found.next = nearest(found.next, answer.next, now);
    }
    for file in state.files.values_mut() {
        let answer = hold_files::decide(&file.facts, file.alive, automatic, now);
        file.alive = answer.alive;
        if answer.counts != file.counted {
            file.counted = answer.counts;
            found.lines.push(file_line(
                &file.facts,
                answer.alive,
                answer.counts,
                automatic,
                now,
            ));
        }
        if answer.counts {
            found.holders.push(Holder {
                kind: answer.kind,
                label: answer.label,
                expires_wall_ms: file.facts.expires_wall_ms,
            });
        }
        found.screen |= answer.screen;
        found.next = nearest(found.next, answer.next, now);
    }
    if let Some(file) = keep_awake_on(state, now) {
        let expires = file.facts.expires_wall_ms;
        // The tooltip's minutes left change at each minute boundary.
        let minute = hold_files::keep_awake(Some(&file.facts), file.alive, now).next;
        found.next = nearest(found.next, minute, now);
        let choice = keep_awake_check(state, expires, now);
        state.keep_awake = Some(KeepAwakeMemo {
            expires_wall_ms: expires,
            choice,
        });
        found.keep_awake = choice;
    }
    found
}

/// The `keep-awake.json` that counts now, if any (found by kind, since the parser ignores case).
pub(super) fn keep_awake_on(state: &State, now: Now) -> Option<&FileHold> {
    state
        .files
        .values()
        .filter(|f| f.facts.kind == HoldKind::KeepAwake)
        .find(|f| hold_files::keep_awake(Some(&f.facts), f.alive, now).choice != KeepAwake::Off)
}

/// The length a Keep awake file with this `expires` stands for: the one picked from the menu for it, else the
/// shortest menu length that covers its time left, worked out at its first sight (`present::keep_awake_choice`).
pub(super) fn keep_awake_check(state: &State, expires_wall_ms: Option<i64>, now: Now) -> KeepAwake {
    match state.keep_awake {
        Some(memo) if memo.expires_wall_ms == expires_wall_ms => memo.choice,
        _ => present::keep_awake_choice(expires_wall_ms, now),
    }
}

/// A session as its log lines name it: the agent's label and a short id (`short_id`). Never a prompt, a title or a path.
fn session_name(facts: &SessionFacts) -> String {
    format!("{} session {}", facts.label, short_id(&facts.key.id))
}

/// A session id as the log shows it: a pid (Claude Code's id) as it is, any other id as 8 hex digits of its FNV-1a
/// hash. Enough to follow one session from line to line; the hash keeps out an id that is a file name or a path
/// piece (OWASP Logging Cheat Sheet: hash session ids that are logged).
fn short_id(id: &str) -> String {
    if (1..=10).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()) {
        return id.to_owned();
    }
    let hash = id.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    format!("{hash:08x}")
}

/// Why a session holds or stopped holding, from the facts `policy::sessions::decide` used: field names and states only.
/// `alive` is the policy's memory, the last `Yes` or `Gone`.
fn session_why(facts: &SessionFacts, alive: Alive, holds: bool, agents: bool, now: Now) -> String {
    if holds {
        let timer = facts.timers.iter().any(|&due| {
            due.saturating_sub(TIMER_LEAD_MS) <= now.wall_ms
                && now.wall_ms < due.saturating_add(TIMER_MARGIN_MS)
        });
        return match (&facts.format, facts.activity) {
            (FormatHealth::UnknownValue(field, _), _) => {
                format!("an unknown {field}, counted as working")
            }
            (_, Activity::Working) => "working".to_owned(),
            _ if facts.open_tasks > 0 => format!("{} background tasks open", facts.open_tasks),
            _ if timer => "a timer is due within the hour".to_owned(),
            _ => "waiting with Remote Control, for up to an hour".to_owned(),
        };
    }
    if !agents {
        return "Automatic is off".to_owned();
    }
    match alive {
        Alive::Gone => return "its process ended".to_owned(),
        Alive::Unknown => return "its process is not checked yet".to_owned(),
        Alive::Yes => {}
    }
    match (&facts.format, facts.activity) {
        (FormatHealth::Unreadable(what), _) => format!("cannot read {what}"),
        (_, Activity::Waiting { remote: false, .. }) => "waiting for the user".to_owned(),
        (_, Activity::Waiting { remote: true, .. }) => "the waiting hour is over".to_owned(),
        _ => "idle".to_owned(),
    }
}

/// A hold file's start or end line: its bare file name, never a path, and why.
fn file_line(facts: &HoldFacts, alive: Alive, counts: bool, automatic: bool, now: Now) -> String {
    let name = &facts.id.0;
    if counts {
        let why = match facts.expires_wall_ms {
            Some(expires) => format!(
                "its process runs; it expires in {} min",
                u64::try_from(expires.saturating_sub(now.wall_ms))
                    .unwrap_or(0)
                    .div_ceil(60_000)
            ),
            None => "its process runs".to_owned(),
        };
        return format!("Hold file {name} counts: {why}");
    }
    let why = if !hold_files::kind_counts(facts.kind, automatic) {
        "Automatic is off"
    } else if alive == Alive::Gone {
        "its process ended"
    } else if alive == Alive::Unknown {
        "its process is not checked yet"
    } else {
        "it expired"
    };
    format!("Hold file {name} stops counting: {why}")
}
