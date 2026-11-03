//! Agent sessions and hold files to the holds that count now, through `policy::sessions` and `policy::hold_files`,
//! with the Keep awake length the menu checks.
//!
//! Owner: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`.

use std::collections::{BTreeMap, BTreeSet};

use super::nearest;
use crate::command::Command;
use crate::holds::{HoldFacts, HoldKind};
use crate::policy::{hold_files, sessions};
use crate::present::{self, Holder};
use crate::process::Alive;
use crate::sources::{FormatHealth, SessionFacts, SourceId};
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
    let mut problems = BTreeSet::new();
    let mut label = None;
    let mut new = BTreeMap::new();
    for (facts, alive) in snapshot {
        if let Some(problem) = problem(&facts.format) {
            problems.insert(problem);
            label = Some(facts.label);
        }
        let memory = old.remove(&facts.key).map(|s| s.memory).unwrap_or_default();
        new.insert(
            facts.key.clone(),
            Session {
                facts,
                alive,
                memory,
            },
        );
    }
    if !new.is_empty() {
        state.sessions.insert(source, new);
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
        let last = state
            .files
            .get(&facts.id)
            .map_or(Alive::Unknown, |f| f.alive);
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
            },
        );
    }
    state.deleting.retain(|id| new.contains_key(id));
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
    };
    let agents = hold_files::kind_counts(HoldKind::Agent, automatic);
    for session in state.sessions.values_mut().flat_map(BTreeMap::values_mut) {
        let answer = sessions::decide(&session.facts, session.alive, &session.memory, now);
        session.memory = answer.memory;
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
