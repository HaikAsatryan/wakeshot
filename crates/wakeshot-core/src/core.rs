//! The core's entry point: events in, commands out (architecture section 3).
//!
//! Owner: core track (`p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`) for the bodies; the
//! signatures are contract.
//!
//! Each `handle` first takes the event's facts into the state, then runs one settle pass over the whole state: the
//! holds that count now, the requests they need, the start and end of holding, the tray view with `state.json`, and
//! the next deadline. The settle pass reads only the state, never the event, so a `Tick`, a late event or a repeated
//! snapshot all give the same answer for the same facts and time.
//!
//! Power settings live in `settings`: the apply goes out from the settle pass after the requests, whenever the
//! episode wants one and the core applies now; restores go out at the edges and as retries on the tick.
//!
//! Seams for `p2-core-lid`: `Core::notify` (notices held while locked), `Core::want_apply` (apply as at a hold
//! start, for the end of a step back), `Core::restore_all` (for the step back), `Core::applies`, and the event and
//! outcome arms that name their card.

mod holds;
mod settings;
mod view;

use std::collections::BTreeSet;

use crate::command::{Command, FileKind, Outcome, RestoreWhy};
use crate::event::Event;
use crate::files::{Config, HoldFile};
use crate::holds::file::{format_expires, parse_expires};
use crate::holds::{HoldId, HoldKind};
use crate::policy::requests::{self, RequestFacts};
use crate::state::{ApplyStep, Episode, Held, KEEP_AWAKE_FILE, KeepAwakeMemo, State};
use crate::time::{Deadline, Now};
use crate::view::{KeepAwake, MenuCommand, Notice};

const MINUTE_MS: i64 = 60_000;

/// Wakeshot's decisions: events in, commands out, no I/O and no clock reads.
///
/// Owner: core track (Phase 2) builds the state and every rule behind these three calls; their signatures are the
/// contract. The loop may rely on: `handle` never blocks and never fails, and `next_deadline` is read after each
/// `handle` to set the one timer. The config in `Event::Started` wins over the one given to `new`.
#[derive(Debug)]
pub struct Core {
    state: State,
}

impl Core {
    pub fn new(config: Config) -> Self {
        Self {
            state: State::new(&config),
        }
    }

    pub fn handle(&mut self, now: Now, event: Event) -> Vec<Command> {
        let mut out = Vec::new();
        if self.state.quit || (!self.state.started && !matches!(event, Event::Started { .. })) {
            return out;
        }
        let resumed = event == Event::Resumed;
        self.take(now, event, &mut out);
        // Quit's commands are the last: a settle would take the requests again.
        if !self.state.quit {
            self.settle(now, resumed, &mut out);
        }
        out
    }

    pub fn next_deadline(&self) -> Option<Deadline> {
        self.state.next
    }

    /// The event's own facts and answers; everything that follows from them is the settle pass's.
    fn take(&mut self, now: Now, event: Event, out: &mut Vec<Command>) {
        match event {
            Event::Started {
                config,
                journal,
                owner,
                on_console,
                tray,
                first_run,
            } => {
                // `Started` comes once; a second one would reset a running episode.
                if self.state.started {
                    return;
                }
                let state = &mut self.state;
                state.started = true;
                state.automatic = config.automatic;
                state.owner = owner;
                state.on_console = on_console;
                state.tray = tray;
                state.journal = journal.map(|j| j.entries).unwrap_or_default();
                // Start-up already tried this restore once; the tick retries it until it works (track note 9).
                if !state.journal.is_empty() {
                    state.retry = Some(RestoreWhy::Leftover);
                }
                if first_run {
                    self.notify(Notice::FirstRun, out);
                    out.push(Command::WriteConfig(config));
                }
            }
            Event::ConfigChanged(config) => self.state.automatic = config.automatic,
            Event::Menu(command) => self.menu(now, command, out),
            Event::Sources { source, sessions } => {
                holds::take_sources(&mut self.state, now, source, sessions, out);
            }
            Event::Holds(files) => holds::take_files(&mut self.state, now, files, out),
            Event::Done(outcome) => self.done(now, outcome, out),
            Event::Tick => self.tick(now, out),
            // The settle pass does all of its work.
            Event::Resumed => {}
            Event::SettingsOwner => {
                self.state.owner = true;
                self.want_apply();
            }
            Event::Plan(read) => self.take_plan(read, out),
            Event::ActivePlan(id) => self.take_active_plan(id, now, out),
            Event::UserChanged(changed) => self.take_user_changed(changed, out),
            Event::JournalCorrupt => self.take_journal_corrupt(out),
            // `Locked` and `Unlocked` are p2-core-lid's.
            Event::Session(change) => self.take_session(change, out),
            // p2-core-lid.
            Event::Lid { .. }
            | Event::Power { .. }
            | Event::InputIdle { .. }
            | Event::Suspending
            | Event::UpdateReady { .. }
            | Event::Updated { .. } => {}
        }
    }

    /// The Keep awake files to delete: by the name the snapshot has, since the parser ignores case and a script may
    /// write any case; `keep-awake.json` when the snapshot has none.
    fn keep_awake_files(&self) -> BTreeSet<HoldId> {
        let mut ids: BTreeSet<HoldId> = self
            .state
            .files
            .values()
            .filter(|f| f.facts.kind == HoldKind::KeepAwake)
            .map(|f| f.facts.id.clone())
            .collect();
        if ids.is_empty() {
            ids.insert(HoldId(KEEP_AWAKE_FILE.to_owned()));
        }
        ids
    }

    fn menu(&mut self, now: Now, command: MenuCommand, out: &mut Vec<Command>) {
        match command {
            MenuCommand::Automatic(on) => {
                if on != self.state.automatic {
                    self.state.automatic = on;
                    out.push(Command::WriteConfig(Config::new(on)));
                }
            }
            MenuCommand::KeepAwake(KeepAwake::Off) => {
                out.extend(self.keep_awake_files().into_iter().map(Command::DeleteHold));
            }
            MenuCommand::KeepAwake(choice) => {
                let expires = match choice {
                    KeepAwake::Minutes(n) => {
                        Some(now.wall_ms.saturating_add(i64::from(n) * MINUTE_MS))
                    }
                    KeepAwake::Off | KeepAwake::UntilOff => None,
                };
                // A new length keeps Keep screen on as it is; Off cleared it with the file (track note 16).
                let screen = holds::keep_awake_on(&self.state, now).is_some_and(|f| f.facts.screen);
                self.write_keep_awake(expires, screen, choice, out);
            }
            MenuCommand::KeepScreenOn(on) => {
                // Only while Keep awake is on (track note 16); the menu greys the row otherwise.
                let Some((screen, expires)) = holds::keep_awake_on(&self.state, now)
                    .map(|f| (f.facts.screen, f.facts.expires_wall_ms))
                else {
                    return;
                };
                if screen != on {
                    let choice = holds::keep_awake_check(&self.state, expires, now);
                    self.write_keep_awake(expires, on, choice, out);
                }
            }
            MenuCommand::OpenLog => out.push(Command::OpenLog),
            MenuCommand::Restore => self.menu_restore(out),
            MenuCommand::Quit => self.menu_quit(out),
        }
    }

    /// `keep-awake.json` with this tray's pid and start, so Keep awake ends with the tray (decision 32).
    fn write_keep_awake(
        &mut self,
        expires_wall_ms: Option<i64>,
        screen: bool,
        choice: KeepAwake,
        out: &mut Vec<Command>,
    ) {
        let text = expires_wall_ms.map(format_expires);
        // The memo must match what the watch thread reads back, and the text drops the milliseconds.
        let read_back = text.as_deref().and_then(parse_expires).or(expires_wall_ms);
        self.state.keep_awake = Some(KeepAwakeMemo {
            expires_wall_ms: read_back,
            choice,
        });
        self.state.keep_awake_written = true;
        let tray = self.state.tray;
        out.push(Command::WriteHold {
            id: HoldId(KEEP_AWAKE_FILE.to_owned()),
            hold: HoldFile {
                expires: text,
                screen,
                ..HoldFile::new(tray.pid, Some(tray.start))
            },
        });
    }

    fn done(&mut self, now: Now, outcome: Outcome, out: &mut Vec<Command>) {
        match outcome {
            Outcome::Requests(Ok(())) => self.state.requests_failed = false,
            // A failed release keeps nothing awake that a notice could explain.
            Outcome::Requests(Err(_)) if self.state.requests.system => {
                self.state.requests_failed = true;
                let first = self
                    .state
                    .episode
                    .as_mut()
                    .is_some_and(|e| !std::mem::replace(&mut e.cannot_hold_shown, true));
                if first {
                    self.notify(Notice::CannotHold, out);
                }
            }
            Outcome::Requests(Err(_)) => {}
            // A failed `FinishRestoreRequest` may have failed at its `state.json` write: write it again.
            Outcome::Written {
                file: FileKind::State | FileKind::RestoreRequest,
                result: Err(_),
            } => self.state.state_file = None,
            Outcome::Written {
                file: FileKind::Hold(id),
                ..
            } => {
                // Answered either way: a delete that failed is asked again at the next snapshot that shows it.
                self.state.deleting.remove(&id);
            }
            // `Config`: its runner logs a failure.
            Outcome::Written { .. } => {}
            Outcome::Applied {
                plan,
                written: _,
                result,
                journal,
            } => self.done_applied(plan, result, journal, now, out),
            Outcome::Restored {
                plan,
                why,
                result,
                journal,
            } => self.done_restored(plan, why, result, journal, out),
            // p2-core-lid.
            Outcome::Locked(_) | Outcome::Hibernate(_) | Outcome::Update(_) => {}
            // Their runners log them.
            Outcome::Sound(_) | Outcome::Notified(_) => {}
        }
    }

    /// Everything that follows from the facts at `now`, in the order the runners need: requests before the apply
    /// at a hold start, and before the restore at a hold end (architecture section 3, "Episode flow").
    fn settle(&mut self, now: Now, resumed: bool, out: &mut Vec<Command>) {
        let found = holds::evaluate(&mut self.state, now);
        for &agent in &found.update {
            if self.state.update_shown.insert(agent) {
                self.notify(Notice::UpdateWakeshot { agent }, out);
            }
        }

        let wanted = requests::wanted(RequestFacts {
            holding: !found.holders.is_empty(),
            keep_screen_on: found.screen,
            stepped_back: self.state.stepped_back,
            on_console: self.state.on_console,
        });
        if wanted != self.state.requests {
            out.push(wanted.command());
            self.state.requests = wanted;
            self.state.requests_failed = false;
        }
        // A sleep ended every request; Windows honors them again only once taken again (decision 16).
        if resumed && wanted.any() {
            out.push(Command::Retake);
            self.state.requests_failed = false;
        }

        let holding: BTreeSet<Held> = found.holders.iter().map(held).collect();
        match self.state.episode.as_mut() {
            None if !holding.is_empty() => self.holding_starts(holding, out),
            Some(_) if holding.is_empty() => self.holding_ends(out),
            Some(episode) if episode.holding != holding => {
                log_changes(&episode.holding, &holding, out);
                episode.held_by.extend(holding.iter().cloned());
                episode.holding = holding;
            }
            _ => {}
        }
        self.send_apply(now, out);

        let state_due = view::show(&mut self.state, &found, now, out);
        let plan_due = self.plan_read_due(now);
        self.state.next = nearest(nearest(found.next, state_due, now), plan_due, now);
    }

    /// The first hold counts; the settle pass sends its apply after the requests (section 3, "Holding starts").
    fn holding_starts(&mut self, holding: BTreeSet<Held>, out: &mut Vec<Command>) {
        out.push(Command::Log(format!(
            "Holding starts: {}",
            labels(&holding)
        )));
        self.state.episode = Some(Episode {
            held_by: holding.clone(),
            holding,
            apply: ApplyStep::Wanted,
            ..Episode::default()
        });
    }

    /// The last hold stopped counting: every entry back at once, behind the requests (section 3, "Holding ends").
    fn holding_ends(&mut self, out: &mut Vec<Command>) {
        if let Some(episode) = self.state.episode.take() {
            out.push(Command::Log(format!(
                "Holding ends; held by {}",
                labels(&episode.held_by)
            )));
        }
        self.restore_all(RestoreWhy::WorkEnded, out);
    }

    /// Every notice goes out here, so p2-core-lid can hold them while the session is locked (track note 18).
    fn notify(&mut self, notice: Notice, out: &mut Vec<Command>) {
        out.push(Command::Notify(notice));
    }
}

fn held(holder: &crate::present::Holder) -> Held {
    Held {
        kind: holder.kind,
        // Keep awake is named by its kind; a reason the CLI wrote into it shows nowhere.
        label: holder
            .label
            .clone()
            .filter(|_| holder.kind != HoldKind::KeepAwake),
    }
}

/// Each hold's start and end with its label (architecture section 8, "Logged").
fn log_changes(before: &BTreeSet<Held>, after: &BTreeSet<Held>, out: &mut Vec<Command>) {
    let started: BTreeSet<Held> = after.difference(before).cloned().collect();
    let ended: BTreeSet<Held> = before.difference(after).cloned().collect();
    if !started.is_empty() {
        out.push(Command::Log(format!("Hold starts: {}", labels(&started))));
    }
    if !ended.is_empty() {
        out.push(Command::Log(format!("Hold ends: {}", labels(&ended))));
    }
}

/// Log words, not tray words: the log is for whoever reads why the PC stayed awake.
fn labels(holds: &BTreeSet<Held>) -> String {
    let names: Vec<&str> = holds
        .iter()
        .map(|held| match (held.kind, held.label.as_deref()) {
            (HoldKind::KeepAwake, _) => "Keep awake",
            (_, Some(label)) => label,
            (HoldKind::Run, None) => "wakeshot run",
            (HoldKind::Agent | HoldKind::Script, None) => "a hold file",
        })
        .collect();
    names.join(", ")
}

/// The nearer of two deadlines as seen from `now`, each on its own clock; on a tie the wall one, which a sleep past
/// it makes due at resume.
fn nearest(a: Option<Deadline>, b: Option<Deadline>, now: Now) -> Option<Deadline> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if ms_ahead(b, now) < ms_ahead(a, now) {
            b
        } else if ms_ahead(a, now) < ms_ahead(b, now) {
            a
        } else if matches!(b, Deadline::Wall(_)) {
            b
        } else {
            a
        }),
        (a, None) => a,
        (None, b) => b,
    }
}

fn ms_ahead(deadline: Deadline, now: Now) -> i128 {
    match deadline {
        Deadline::Wall(wall_ms) => i128::from(wall_ms) - i128::from(now.wall_ms),
        Deadline::Mono(mono_ms) => i128::from(mono_ms) - i128::from(now.mono_ms),
    }
}
