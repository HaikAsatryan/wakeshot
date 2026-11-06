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
//! The closed-lid and battery side lives in `lid`: the battery floor goes into the settle pass before the requests
//! (stepped back takes none; its grace gives a deadline), the source lock, the step back's actions and the countdown
//! after the episode edges.
//! Every notice goes through `Core::notify`, which keeps it while the user is away. Updates live in `update`.

mod holds;
mod lid;
mod settings;
mod update;
mod view;

use std::collections::BTreeSet;

use crate::command::{Command, FileKind, HibernateEnd, Outcome, RestoreWhy};
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
        // A hibernate that resumed is a resume too: the requests need taking again (decision 16).
        let resumed = matches!(
            event,
            Event::Resumed | Event::Done(Outcome::Hibernate(HibernateEnd::Resumed))
        );
        let tick = event == Event::Tick;
        self.take(now, event, &mut out);
        // Quit's commands are the last: a settle would take the requests again.
        if !self.state.quit {
            self.settle(now, resumed, tick, &mut out);
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
                journal_unread,
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
                // Start-up already tried this restore once; the tick retries it until it works (track note 9). A
                // journal that did not read may hold a crashed tray's values, so the copy stays empty and the retry
                // reads it again (power safety review finding 3).
                if !state.journal.is_empty() || journal_unread {
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
            Event::Tick => {
                self.tick(now, out);
                self.check_lock(now, out);
                self.check_slept(now, out);
            }
            // The idle clamp counts from here; the settle pass takes the requests again.
            Event::Resumed => self.take_resumed(now, out),
            // The idle clamp needs only the resume; the sleep route's fallback needs to know the PC slept.
            Event::Suspending => self.take_suspending(now),
            Event::SettingsOwner => {
                self.state.owner = true;
                self.want_apply();
            }
            Event::Plan(read) => self.take_plan(read, out),
            Event::ActivePlan(id) => self.take_active_plan(id, now, out),
            Event::UserChanged(changed) => self.take_user_changed(changed, out),
            Event::JournalCorrupt => self.take_journal_corrupt(now, out),
            Event::Session(change) => self.take_session(change, now, out),
            Event::Lid { closed } => self.take_lid(closed, now, out),
            Event::Power { ac, battery_pct } => self.take_power(ac, battery_pct, now, out),
            Event::InputIdle { ms } => self.take_input_idle(ms, now, out),
            Event::UpdateReady { version } => self.take_update_ready(version, out),
            Event::Updated { version } => self.take_updated(version, out),
            // Each event is one folder added, so each gets its notice; it waits while the user is away.
            Event::HooksAdded { agent } => self.notify(Notice::HooksAdded { agent }, out),
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
            Outcome::ResetLost {
                reset,
                result,
                journal,
            } => self.done_reset(reset, result, journal, now, out),
            Outcome::Locked(result) => self.done_locked(result, now, out),
            Outcome::SleepCheck(result) => self.done_sleep_check(result, now, out),
            Outcome::Hibernate(end) => self.done_hibernate(end, now, out),
            Outcome::Update(result) => self.done_update(result, out),
            // Their runners log them.
            Outcome::Sound(_) | Outcome::Notified(_) => {}
        }
    }

    /// Everything that follows from the facts at `now`, in the order the runners need: requests before the apply
    /// at a hold start, and before the restore at a hold end (architecture section 3, "Episode flow").
    fn settle(&mut self, now: Now, resumed: bool, tick: bool, out: &mut Vec<Command>) {
        let found = holds::evaluate(&mut self.state, now);
        for &agent in &found.update {
            if self.state.update_shown.insert(agent) {
                self.notify(Notice::UpdateWakeshot { agent }, out);
            }
        }
        let floor_due = self.settle_floor(now, out);

        let wanted = requests::wanted(RequestFacts {
            holding: !found.holders.is_empty(),
            keep_screen_on: found.screen,
            stepped_back: self.holds_back(),
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
        // Before the lid side, so a window that ends here sets its countdown want in this pass.
        let journal_due = self.settle_journal(now);
        let lid_due = self.settle_lid(now, tick, out);
        self.send_apply(now, out);
        self.send_update(out);

        let state_due = view::show(&mut self.state, &found, now, out);
        let plan_due = self.plan_read_due(now);
        let next = nearest(nearest(found.next, state_due, now), plan_due, now);
        let next = nearest(next, floor_due, now);
        self.state.next = nearest(nearest(next, lid_due, now), journal_due, now);
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
    /// With the lid closed the countdown starts, read before this restore lands.
    fn holding_ends(&mut self, out: &mut Vec<Command>) {
        if let Some(episode) = self.state.episode.take() {
            out.push(Command::Log(format!(
                "Holding ends; held by {}",
                labels(&episode.held_by)
            )));
        }
        self.work_ended();
        self.restore_all(RestoreWhy::WorkEnded, out);
    }

    /// Every notice goes out here. One raised while the session is locked or another session has the PC's screen
    /// waits for the user, in order (track note 18): a balloon shown to nobody is lost. The same notice twice in one
    /// absence says nothing new, so it waits once. Sounds never wait.
    fn notify(&mut self, notice: Notice, out: &mut Vec<Command>) {
        if self.state.session_locked || !self.state.on_console {
            if !self.state.held_notices.contains(&notice) {
                self.state.held_notices.push(notice);
            }
        } else {
            out.push(Command::Notify(notice));
        }
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

fn labels(holds: &BTreeSet<Held>) -> String {
    let names: Vec<&str> = holds.iter().map(label).collect();
    names.join(", ")
}

/// What held, for the log and the closed-lid report: the agent's label or the file's reason, else a name for the
/// kind. Log words, not tray words: `text` words the notice around these names.
fn label(held: &Held) -> &str {
    match (held.kind, held.label.as_deref()) {
        (HoldKind::KeepAwake, _) => "Keep awake",
        (_, Some(label)) => label,
        (HoldKind::Run, None) => "wakeshot run",
        (HoldKind::Agent | HoldKind::Script, None) => "a hold file",
    }
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
