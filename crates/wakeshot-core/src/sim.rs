//! The `Sim` harness: the core driven by a fake clock the way the tray loop drives it (test-support).
//!
//! Owner: `p2-core-holds`, then `p2-core-settings`, `p2-core-lid`, `p2-core-props`.
//!
//! A test sends events at the current time and reads the commands back. Time moves only through [`Sim::run`],
//! [`Sim::suspend`] and [`Sim::set_wall`]. `run` plays the loop's one timer: after each event the loop sets it to the
//! core's `next_deadline`, as a wait in run time from that moment, and fires the 30 s safety tick between. So a `Wall`
//! deadline comes early or late after a clock change, as on a real PC, and the core must re-check on `Tick`
//! (`time::Deadline`). After every `Tick` the harness checks that the next deadline lies ahead, so no scenario can
//! hide a core that would spin the loop. Every step is recorded, so a failed assert can print what happened
//! ([`Sim::trace`]).
//!
//! The hold folder is a small world: [`Sim::put_hold`] and [`Sim::remove_hold`] change it as a script would, and
//! [`Sim::watch`] runs the core's own `WriteHold` and `DeleteHold` on it, the way the IO and watch threads do.
//!
//! So is the user's input: after the first [`Sim::input`], each loop timer posts `InputIdle` right before its `Tick`,
//! as the loop does (`docs/phases/06-integration.md`, "one timer"). Its idle grows with run time and with suspended
//! time, as `GetLastInputInfo` does (research, "Clock"). Before it, no `InputIdle` is posted, so scenarios that do not
//! care about input see only ticks.
//!
//! [`World`] puts a `Sim` in a whole world for the property tests: Windows' power plans on a `FakePlans`, the IO
//! thread that answers `ReadPlan`, `Apply`, `Restore` and `ResetLost` through the real ownership routines, answers
//! that come late, and the user, each change turned into the event Windows or a watch thread posts. It logs what
//! Windows holds at each moment ([`Truth`]), so a property is checked against that, not only against the commands.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;

use crate::command::{
    ApplyError, Command, CountdownReason, Failure, FileKind, HibernateEnd, HibernateSkip, Outcome,
    RestoreWhy,
};
use crate::core::Core;
use crate::event::{Event, SessionChange};
use crate::files::{Config, HoldFile, JournalFile, JournalFileEntry, JournalKey, StateFile};
use crate::holds::file::{self, Parsed};
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::ownership::apply::apply;
use crate::ownership::check::check;
use crate::ownership::fake::{Action, FAKE_SCREEN_OFF, FakeJournal, FakePlans, SetBy};
use crate::ownership::io::PlanIo;
use crate::ownership::reset::{KeyReset, reset_lost};
use crate::ownership::restore::{Found, Restored, restore};
use crate::policy::ending::IDLE_MS;
use crate::policy::requests::Requests;
use crate::process::{Alive, FileTime, ProcessRef};
use crate::settings::{
    Guid, Journal, JournalEntry, JournalOwner, PlanValues, SHORT_TIMEOUT_S, SettingKey,
};
use crate::sources::{Activity, FormatHealth, Liveness, SessionFacts, SessionKey, SourceId};
use crate::testing::FakeClock;
use crate::time::{Clock, Deadline, Now};
use crate::view::{KeepAwake, MenuCommand, Notice, TrayView};

/// 2026-11-04 00:00:00 UTC, a whole second, so a Keep awake `expires` written at the start reads back exactly.
pub const START: Now = Now {
    wall_ms: 1_793_750_400_000,
    mono_ms: 5_000,
};

/// The tray the default `Started` names.
pub const TRAY: ProcessRef = ProcessRef {
    pid: 9120,
    start: FileTime(134_381_211_391_959_336),
};

/// The loop's safety tick (architecture section 3, "Clocks").
pub const SAFETY_TICK_MS: u64 = 30_000;

/// The Keep awake hold file the core writes.
pub const KEEP_AWAKE: &str = crate::state::KEEP_AWAKE_FILE;

/// The test laptop's Balanced plan.
pub const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

/// A second plan (High performance), for plan switches.
pub const OTHER_PLAN: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);

/// The three keys an apply writes, in the order the core sends them.
pub const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];

/// A plan read of `active` holding the test's user values: lid Sleep on AC and battery, sleep after 3 h on AC and
/// 10 min on battery, energy saver at 20 %. Test values only, not read from any laptop.
pub fn plan(active: Guid) -> PlanValues {
    PlanValues {
        active,
        lid_ac: 1,
        lid_dc: 1,
        sleep_ac: 10_800,
        sleep_dc: 600,
        saver_threshold_dc: 20,
    }
}

/// The journal entries a hold's apply leaves on `plan_id` for `keys`: each target 0 over the values of [`plan`].
pub fn entries(plan_id: Guid, keys: &[SettingKey]) -> Vec<JournalEntry> {
    let values = plan(plan_id);
    keys.iter()
        .map(|&key| JournalEntry {
            plan: plan_id,
            key,
            original: key_value(&values, key),
            written: 0,
            restored: false,
        })
        .collect()
}

/// The journal entries the sleep route's apply leaves on `plan_id`: the four short keys at 1 minute over the values
/// of [`plan`] and the fake's screen timeouts, all longer.
pub fn short_entries(plan_id: Guid) -> Vec<JournalEntry> {
    let values = plan(plan_id);
    SettingKey::SHORT
        .iter()
        .map(|&key| JournalEntry {
            plan: plan_id,
            key,
            original: key_value(&values, key),
            written: SHORT_TIMEOUT_S,
            restored: false,
        })
        .collect()
}

/// `Done(Applied)` for an apply to `plan` that left `journal`; `written` is every key of `journal` on `plan` when
/// it worked, none otherwise.
pub fn applied(plan: Guid, result: Result<(), ApplyError>, journal: Vec<JournalEntry>) -> Event {
    let written = match result {
        Ok(()) => journal
            .iter()
            .filter(|e| e.plan == plan)
            .map(|e| e.key)
            .collect(),
        Err(_) => Vec::new(),
    };
    Event::Done(Outcome::Applied {
        plan,
        written,
        result,
        journal,
    })
}

/// `Done(Restored)` for a restore that left `journal`.
pub fn restored(
    plan: Option<Guid>,
    why: RestoreWhy,
    result: Result<(), Failure>,
    journal: Vec<JournalEntry>,
) -> Event {
    Event::Done(Outcome::Restored {
        plan,
        why,
        result,
        journal,
    })
}

/// `Done(ResetLost)` for a reset that ran and set `reset` to Windows' defaults (the file did not read after all).
pub fn reset_done(reset: &[SettingKey], result: Result<(), Failure>) -> Event {
    Event::Done(Outcome::ResetLost {
        reset: reset.to_vec(),
        result,
        journal: Vec::new(),
    })
}

/// One event as the core got it, with the commands it gave.
#[derive(Clone, Debug)]
pub struct Step {
    pub at: Now,
    pub event: Event,
    pub commands: Vec<Command>,
}

/// `Event::Started` with the values a test usually wants: Automatic on, no journal and it read, settings owner, on the
/// console, this [`TRAY`], not the first run. Change a field with struct update syntax.
#[derive(Clone, Debug)]
pub struct Start {
    pub config: Config,
    pub journal: Option<Journal>,
    pub journal_unread: bool,
    pub owner: bool,
    pub on_console: bool,
    pub tray: ProcessRef,
    pub first_run: bool,
}

impl Default for Start {
    fn default() -> Self {
        Self {
            config: Config::default(),
            journal: None,
            journal_unread: false,
            owner: true,
            on_console: true,
            tray: TRAY,
            first_run: false,
        }
    }
}

impl Start {
    pub fn event(self) -> Event {
        Event::Started {
            config: self.config,
            journal: self.journal,
            journal_unread: self.journal_unread,
            owner: self.owner,
            on_console: self.on_console,
            tray: self.tray,
            first_run: self.first_run,
        }
    }
}

/// A Claude Code session of process `pid`, format readable, no timers, no open tasks.
pub fn claude(pid: u32, activity: Activity) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::ClaudeCode,
            id: pid.to_string(),
        },
        liveness: Liveness::Process {
            pid,
            start: FileTime(134_381_211_391_959_336 + u64::from(pid)),
        },
        activity,
        timers: Vec::new(),
        open_tasks: 0,
        label: "Claude Code",
        format: FormatHealth::Ok,
    }
}

/// A Claude Code snapshot, as the watch thread posts it.
pub fn sources(sessions: Vec<(SessionFacts, Alive)>) -> Event {
    Event::Sources {
        source: SourceId::ClaudeCode,
        sessions,
    }
}

/// A hold file with no `expires`, no reason and no screen; its kind comes from the name, as the parser gives it.
pub fn hold(name: &str, alive: Alive) -> HoldFacts {
    HoldFacts {
        id: HoldId(name.to_owned()),
        kind: file::kind_of(name).unwrap_or(HoldKind::Script),
        alive,
        expires_wall_ms: None,
        reason: None,
        screen: false,
    }
}

/// The facts the watch thread would post for `hold` written as `id`: the file goes through JSON and the real parser,
/// so `expires` reads back the way it will on disk.
///
/// # Panics
///
/// When the parser refuses the file, which no hold the core writes may cause.
pub fn watched(id: &HoldId, hold: &HoldFile, alive: Alive) -> HoldFacts {
    let bytes = serde_json::to_vec(hold).expect("a hold file serializes");
    match file::parse(&id.0, &bytes) {
        Parsed::Ok(parsed) => file::facts(id.clone(), &parsed, alive),
        other => panic!("the parser refused {id:?}: {other:?}"),
    }
}

/// The core with a fake clock, a loop timer, a hold folder and a record of every step.
#[derive(Debug)]
pub struct Sim {
    core: Core,
    clock: FakeClock,
    origin: Now,
    steps: Vec<Step>,
    trace: Vec<String>,
    /// When the loop's one timer fires, in run time; set after each event from `next_deadline`.
    timer_mono: Option<u64>,
    next_safety_mono: u64,
    folder: BTreeMap<HoldId, HoldFacts>,
    /// Steps whose hold commands `watch` already ran on the folder.
    watched: usize,
    /// Input idle at a run time; `None` until the first [`Sim::input`].
    input: Option<InputIdle>,
}

/// Input idle as `GetLastInputInfo` gives it: `ms` at run time `at_mono`, growing from there.
#[derive(Clone, Copy, Debug)]
struct InputIdle {
    ms: u64,
    at_mono: u64,
}

impl Default for Sim {
    fn default() -> Self {
        Self::new()
    }
}

impl Sim {
    /// At [`START`], with `Core::new(Config::default())`. Nothing is sent yet: call [`Sim::start`].
    pub fn new() -> Self {
        Self::with(Config::default(), START)
    }

    pub fn with(config: Config, start: Now) -> Self {
        Self {
            core: Core::new(config),
            clock: FakeClock::new(start),
            origin: start,
            steps: Vec::new(),
            trace: Vec::new(),
            timer_mono: None,
            next_safety_mono: start.mono_ms + SAFETY_TICK_MS,
            folder: BTreeMap::new(),
            watched: 0,
            input: None,
        }
    }

    pub fn now(&self) -> Now {
        self.clock.now()
    }

    pub fn deadline(&self) -> Option<Deadline> {
        self.core.next_deadline()
    }

    /// Sends the default `Started` ([`Start::default`]).
    pub fn start(&mut self) -> Vec<Command> {
        self.send(Start::default().event())
    }

    /// One event at the current time; the loop's timer is set again from `next_deadline`.
    ///
    /// # Panics
    ///
    /// After a `Tick`, when the next deadline is not after now on its clock: the loop would spin.
    pub fn send(&mut self, event: Event) -> Vec<Command> {
        let now = self.clock.now();
        let tick = event == Event::Tick;
        let commands = self.core.handle(now, event.clone());
        let deadline = self.core.next_deadline();
        self.note(now, &event, &commands, deadline);
        self.steps.push(Step {
            at: now,
            event,
            commands: commands.clone(),
        });
        let wait = deadline.map(|d| ms_ahead(d, now));
        if tick && wait.is_some_and(|ms| ms <= 0) {
            panic!(
                "next_deadline {deadline:?} is not after now after a Tick\n{}",
                self.trace()
            );
        }
        self.timer_mono = wait.map(|ms| {
            now.mono_ms
                .saturating_add(u64::try_from(ms.max(0)).unwrap_or(u64::MAX))
        });
        commands
    }

    /// The PC runs for `ms`: a `Tick` at each timer the loop would fire (the core's deadline, the safety tick), each
    /// at its moment, and one at once for a timer already due. Gives every command of those ticks.
    pub fn run(&mut self, ms: u64) -> Vec<Command> {
        let end = self.clock.now().mono_ms.saturating_add(ms);
        let mut out = Vec::new();
        while let Some(commands) = self.fire_next(end) {
            out.extend(commands);
        }
        self.wait_until(end);
        if ms > 0 {
            self.line(format!("ran {}", seconds(ms)));
        }
        out
    }

    /// The loop's next timer, if it fires by run time `end_mono`: the clock moves to it, then `InputIdle` (after the
    /// first [`Sim::input`]) and `Tick` go in, as the loop posts them. `None` when no timer is due by then; the clock
    /// stays. [`Sim::run`] is these fires, then [`Sim::wait_until`]; a caller that must act between two timers (the
    /// IO thread of [`World`]) calls them itself.
    pub fn fire_next(&mut self, end_mono: u64) -> Option<Vec<Command>> {
        let mono = self.clock.now().mono_ms;
        let due = self
            .timer_mono
            .map_or(self.next_safety_mono, |t| t.min(self.next_safety_mono))
            .max(mono);
        if due > end_mono {
            return None;
        }
        self.clock.run(due - mono);
        // `send` refuses a deadline at or before now after a Tick, so a caller's loop always moves on.
        while self.next_safety_mono <= due {
            self.next_safety_mono += SAFETY_TICK_MS;
        }
        let mut out = Vec::new();
        if let Some(ms) = self.idle_ms() {
            out.extend(self.send(Event::InputIdle { ms }));
        }
        out.extend(self.send(Event::Tick));
        Some(out)
    }

    /// [`Sim::run`] in steps of at most 15 s, each `Restore` the core sends answered at the end of its step as one
    /// whose journal load failed with `failure`, as the executor posts it (no entries known). Gives every command.
    pub fn run_failing_restores(&mut self, ms: u64, failure: Failure) -> Vec<Command> {
        let end = self.clock.now().mono_ms.saturating_add(ms);
        let mut all = Vec::new();
        while self.clock.now().mono_ms < end {
            let step = (end - self.clock.now().mono_ms).min(15_000);
            let out = self.run(step);
            for command in &out {
                if let Command::Restore { plan, why } = *command {
                    all.extend(self.send(restored(plan, why, Err(failure), Vec::new())));
                }
            }
            all.extend(out);
        }
        all
    }

    /// The clock moves on to run time `end_mono` with no timer fired: call it only after [`Sim::fire_next`] said none
    /// is due by then.
    pub fn wait_until(&mut self, end_mono: u64) {
        let mono = self.clock.now().mono_ms;
        self.clock.run(end_mono.saturating_sub(mono));
    }

    /// The PC sleeps for `ms`: `Suspending`, only the wall clock moves, then `Resumed`. Gives both events' commands.
    pub fn suspend(&mut self, ms: u64) -> Vec<Command> {
        let mut out = self.send(Event::Suspending);
        self.clock.suspend(ms);
        // Input idle counts suspended time; run time does not.
        if let Some(input) = self.input.as_mut() {
            input.ms = input.ms.saturating_add(ms);
        }
        self.line(format!("slept {}", seconds(ms)));
        out.extend(self.send(Event::Resumed));
        out
    }

    /// The user touches the keyboard or mouse now. From the first call on, every loop timer posts `InputIdle` before
    /// its `Tick`. No event now: the loop reads input only when a timer fires.
    pub fn input(&mut self) {
        self.input = Some(InputIdle {
            ms: 0,
            at_mono: self.clock.now().mono_ms,
        });
        self.line("input".to_owned());
    }

    /// Input idle now, as the loop would read it; `None` before the first [`Sim::input`].
    pub fn idle_ms(&self) -> Option<u64> {
        let now = self.clock.now().mono_ms;
        self.input
            .map(|i| i.ms.saturating_add(now.saturating_sub(i.at_mono)))
    }

    /// Someone sets the clock. No event: the loop learns of it only from the times of later events.
    pub fn set_wall(&mut self, wall_ms: i64) {
        let before = self.clock.now().wall_ms;
        self.clock.set_wall(wall_ms);
        self.line(format!(
            "wall clock set by {} ms",
            i128::from(wall_ms) - i128::from(before)
        ));
    }

    /// A script writes or replaces a hold file: the folder changes and the watch thread posts it.
    pub fn put_hold(&mut self, facts: HoldFacts) -> Vec<Command> {
        self.folder.insert(facts.id.clone(), facts);
        self.post_folder()
    }

    /// A hold file goes away by itself: the folder changes and the watch thread posts it.
    pub fn remove_hold(&mut self, name: &str) -> Vec<Command> {
        self.folder.remove(&HoldId(name.to_owned()));
        self.post_folder()
    }

    /// Runs every `WriteHold` and `DeleteHold` the core sent since the last `watch`, as the IO thread does (each
    /// answered with `Done(Written)`), then posts the folder as the watch thread does. A written file names this
    /// tray, so it is alive.
    pub fn watch(&mut self) -> Vec<Command> {
        let mut done = Vec::new();
        for step in &self.steps[self.watched..] {
            for command in &step.commands {
                match command {
                    Command::WriteHold { id, hold } => {
                        self.folder
                            .insert(id.clone(), watched(id, hold, Alive::Yes));
                        done.push(id.clone());
                    }
                    Command::DeleteHold(id) => {
                        self.folder.remove(id);
                        done.push(id.clone());
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for id in done {
            out.extend(self.send(Event::Done(Outcome::Written {
                file: FileKind::Hold(id),
                result: Ok(()),
            })));
        }
        self.watched = self.steps.len();
        out.extend(self.post_folder());
        out
    }

    fn post_folder(&mut self) -> Vec<Command> {
        let snapshot = self.folder.values().cloned().collect();
        self.send(Event::Holds(snapshot))
    }

    /// The hold folder as it is now.
    pub fn folder(&self) -> impl Iterator<Item = &HoldFacts> {
        self.folder.values()
    }

    /// A line in the trace for something outside the core: what the world or the IO thread did.
    pub fn remark(&mut self, text: String) {
        self.line(text);
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Every command so far with the time it was sent.
    pub fn commands(&self) -> impl Iterator<Item = (Now, &Command)> {
        self.steps
            .iter()
            .flat_map(|step| step.commands.iter().map(move |c| (step.at, c)))
    }

    /// When `command` was sent, each time.
    pub fn sent_at(&self, command: &Command) -> Vec<Now> {
        self.commands()
            .filter(|(_, c)| *c == command)
            .map(|(at, _)| at)
            .collect()
    }

    /// The tray as last drawn.
    pub fn view(&self) -> Option<&TrayView> {
        self.commands()
            .filter_map(|(_, c)| match c {
                Command::Tray(view) => Some(view),
                _ => None,
            })
            .last()
    }

    /// The requests as last set; none before the first `Requests`.
    pub fn requests(&self) -> Requests {
        self.commands()
            .filter_map(|(_, c)| match *c {
                Command::Requests { system, display } => Some(Requests { system, display }),
                _ => None,
            })
            .last()
            .unwrap_or(Requests::NONE)
    }

    /// Every notice so far, in order.
    pub fn notices(&self) -> Vec<&Notice> {
        self.commands()
            .filter_map(|(_, c)| match c {
                Command::Notify(notice) => Some(notice),
                _ => None,
            })
            .collect()
    }

    /// Every `state.json` write so far, with its time.
    pub fn state_writes(&self) -> Vec<(Now, &StateFile)> {
        self.commands()
            .filter_map(|(at, c)| match c {
                Command::WriteState(file) => Some((at, file)),
                _ => None,
            })
            .collect()
    }

    /// The last `state.json` written.
    pub fn state_file(&self) -> Option<&StateFile> {
        self.state_writes().last().map(|&(_, file)| file)
    }

    /// Every step and clock move as short lines, for a failed assert's message.
    pub fn trace(&self) -> String {
        self.trace.join("\n")
    }

    fn note(&mut self, now: Now, event: &Event, commands: &[Command], next: Option<Deadline>) {
        let mut text = event_text(event);
        text.push_str(" ->");
        if commands.is_empty() {
            text.push_str(" nothing");
        }
        for command in commands {
            let _ = write!(text, " {};", command_text(command));
        }
        if let Some(next) = next {
            let _ = write!(text, " next {}", deadline_text(next, now));
        }
        self.line(text);
    }

    fn line(&mut self, text: String) {
        let now = self.clock.now();
        self.trace.push(format!(
            "[run {:>10} wall {:>+12} ms] {text}",
            seconds(now.mono_ms.saturating_sub(self.origin.mono_ms)),
            i128::from(now.wall_ms) - i128::from(self.origin.wall_ms),
        ));
    }
}

/// How far ahead of `now` a deadline lies on its own clock; zero or less is due.
fn ms_ahead(deadline: Deadline, now: Now) -> i128 {
    match deadline {
        Deadline::Wall(wall_ms) => i128::from(wall_ms) - i128::from(now.wall_ms),
        Deadline::Mono(mono_ms) => i128::from(mono_ms) - i128::from(now.mono_ms),
    }
}

fn seconds(ms: u64) -> String {
    format!("{}.{:03} s", ms / 1000, ms % 1000)
}

fn deadline_text(deadline: Deadline, now: Now) -> String {
    let clock = match deadline {
        Deadline::Wall(_) => "wall",
        Deadline::Mono(_) => "run",
    };
    format!("{clock} +{} ms", ms_ahead(deadline, now))
}

fn alive_text(alive: Alive) -> &'static str {
    match alive {
        Alive::Yes => "alive",
        Alive::Gone => "gone",
        Alive::Unknown => "unknown",
    }
}

fn event_text(event: &Event) -> String {
    match event {
        Event::Sources { source, sessions } => {
            let list: Vec<String> = sessions
                .iter()
                .map(|(s, alive)| {
                    format!(
                        "{} {:?} timers {:?} tasks {} {:?} {}",
                        s.key.id,
                        s.activity,
                        s.timers,
                        s.open_tasks,
                        s.format,
                        alive_text(*alive)
                    )
                })
                .collect();
            format!("Sources {source:?} [{}]", list.join(", "))
        }
        Event::Holds(files) => {
            let list: Vec<String> = files
                .iter()
                .map(|f| {
                    format!(
                        "{} expires {:?} screen {} {}",
                        f.id.0,
                        f.expires_wall_ms,
                        f.screen,
                        alive_text(f.alive)
                    )
                })
                .collect();
            format!("Holds [{}]", list.join(", "))
        }
        other => format!("{other:?}"),
    }
}

fn command_text(command: &Command) -> String {
    match command {
        Command::Tray(view) => format!("Tray {:?} {:?}", view.icon, view.tooltip),
        Command::WriteState(file) => format!("WriteState holding {:?}", file.holding),
        other => format!("{other:?}"),
    }
}

// The world around the core (`p2-core-props`): Windows' power plans, the IO thread and the other runners, and the
// user. `World` plays the parts the tray's other threads play, so a property is checked against what Windows would
// hold, not only against the commands.

/// This tray as the journal's owner: every apply of the world writes it (`p3-power-io`'s `me`).
pub const ME: JournalOwner = JournalOwner {
    pid: TRAY.pid,
    proc_start: TRAY.start,
    session: 1,
};

/// A tray that crashed before this one started and left its journal behind.
pub const DEAD_TRAY: JournalOwner = JournalOwner {
    pid: 6012,
    proc_start: FileTime(134_381_100_000_000_000),
    session: 1,
};

/// A third plan, for docked use.
pub const DOCKED_PLAN: Guid = Guid(0x1f6c_2d53_8e2a_4b17_9c4e_5a0d_3b7e_9f21);

/// The world's plans; [`WorldStart::active`] picks the active one.
pub const PLANS: [Guid; 3] = [BALANCED, OTHER_PLAN, DOCKED_PLAN];

/// What the locker answers when a lock fails.
pub const LOCK_FAILED: Failure = Failure {
    call: "LockWorkStation",
    code: 5,
};

/// What the update thread answers when an update fails.
pub const UPDATE_FAILED: Failure = Failure {
    call: "UpdateManager::apply_updates_and_restart",
    code: 1,
};

/// Do nothing for the lid and Never for sleep (architecture section 5, "Power settings").
const TARGET: u32 = 0;

/// How long the world runs after the last hold, so the core's restore retries (at most one per 10 s) get their turn.
const RETRIES_MS: u64 = 2 * 60_000;

/// How long a hibernate the world lets happen lasts.
const HIBERNATED_MS: u64 = 60 * 60_000;

/// Each round of the world's IO thread is a command and its answers; more than this many in one flush means the core
/// and the IO side feed each other without end.
const FLUSH_LIMIT: usize = 10_000;

/// The user's own values of each world plan, which differ, so a value put back on the wrong plan shows:
///
/// - Balanced: the test's user values ([`plan`]).
/// - [`OTHER_PLAN`]: the lid on battery is the user's own Do nothing, on AC Hibernate; energy saver Never (floor 20).
/// - [`DOCKED_PLAN`]: the lid on AC is the user's own Do nothing (docked use), on battery Sleep; energy saver 30 %.
pub fn world_plan(id: Guid) -> PlanValues {
    match id {
        OTHER_PLAN => PlanValues {
            active: id,
            lid_ac: 2,
            lid_dc: 0,
            sleep_ac: 0,
            sleep_dc: 900,
            saver_threshold_dc: 0,
        },
        DOCKED_PLAN => PlanValues {
            active: id,
            lid_ac: 0,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 1_800,
            saver_threshold_dc: 30,
        },
        _ => plan(id),
    }
}

/// One key of a plan read; the screen timeouts, which a plan read does not carry, at a new fake plan's own
/// ([`FAKE_SCREEN_OFF`]).
pub fn key_value(values: &PlanValues, key: SettingKey) -> u32 {
    match key {
        SettingKey::LidAc => values.lid_ac,
        SettingKey::LidDc => values.lid_dc,
        SettingKey::SleepDc => values.sleep_dc,
        SettingKey::SleepAc => values.sleep_ac,
        SettingKey::VideoIdleAc => FAKE_SCREEN_OFF[0],
        SettingKey::VideoIdleDc => FAKE_SCREEN_OFF[1],
    }
}

fn plan_name(id: Guid) -> &'static str {
    match id {
        BALANCED => "Balanced",
        OTHER_PLAN => "other plan",
        DOCKED_PLAN => "docked plan",
        _ => "unknown plan",
    }
}

/// What a Claude Code session does at a [`WorldStep::Agent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentNow {
    Working,
    Idle,
    /// Its process ended; the status file is still there.
    Gone,
    /// It left the snapshot.
    Leaves,
}

/// Which IO command a [`WorldStep::Fault`] hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultOn {
    /// Whichever routine makes the call: counted from now.
    AnyCommand,
    ReadPlan,
    Apply,
    Restore,
}

/// What a `wakeshot run` hold file does at a [`WorldStep::RunFile`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileNow {
    Alive,
    /// Its process ended; the file is still there until the core deletes it.
    Gone,
    Removed,
}

/// One thing that happens in the world. Each prints as one short line, so a shrunk case reads as a story.
#[derive(Clone, PartialEq, Eq)]
pub enum WorldStep {
    /// A Claude Code session of process `pid`; the watch thread posts the whole snapshot.
    Agent {
        pid: u32,
        now: AgentNow,
    },
    /// `run-<4000 + n>.json`.
    RunFile {
        n: u8,
        now: FileNow,
    },
    Menu(MenuCommand),
    /// Only a change is posted, as the lid notification does.
    Lid {
        closed: bool,
    },
    /// Only a change is posted.
    Ac(bool),
    /// A new battery reading on the same power source.
    Battery(u8),
    /// The user touches the keyboard or mouse.
    Input,
    /// The PC runs for this many ms; the loop's timers fire on the way.
    Run(u64),
    /// The user sleeps the PC for this many ms.
    Suspend(u64),
    /// Someone sets the clock by this many ms.
    WallJump(i64),
    /// The user locks the session (Win+L).
    Lock,
    /// The user signs in again, on the laptop's screen or an external one.
    Unlock,
    /// Another user's session takes the PC's screen (fast user switching).
    ConsoleLeft,
    ConsoleBack,
    /// Windows ends the session (`WM_ENDSESSION`).
    Ending,
    EndCancelled,
    /// The settings mutex comes free and this tray takes it.
    TakeOwner,
    /// The user changes a value: on the active plan as Settings does (`None`), or on the plan `PLANS[i]` as
    /// `powercfg` can. No event: the next plan read finds it.
    UserSets {
        plan: Option<usize>,
        key: SettingKey,
        value: u32,
    },
    /// The user or an OEM tool switches the active plan (`plan` indexes [`PLANS`]); Windows posts `ActivePlan`.
    Switch(usize),
    /// An IO call fails with `code`: the call `at` calls into the next command of kind `on`.
    Fault {
        on: FaultOn,
        at: usize,
        code: u32,
    },
    /// The next `LockWorkStation` fails.
    LockFails,
    /// Whether the next hibernate is allowed (`IsPwrHibernateAllowed`).
    Hibernation {
        allowed: bool,
    },
    /// An update is downloaded; `ok`: applying it works.
    UpdateReady {
        ok: bool,
    },
    /// The IO thread runs its next command; its answers reach the core at once.
    Io,
    /// The IO thread runs its next command; its answers wait (late outcomes).
    IoLate,
    /// The answers that wait reach the core.
    Deliver,
    /// The IO thread runs everything and every answer reaches the core.
    Flush,
}

impl std::fmt::Debug for WorldStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agent { pid, now } => write!(f, "agent {pid} {now:?}"),
            Self::RunFile { n, now } => write!(f, "{} {now:?}", run_file(*n)),
            Self::Menu(command) => write!(f, "menu {command:?}"),
            Self::Lid { closed: true } => f.write_str("lid closes"),
            Self::Lid { closed: false } => f.write_str("lid opens"),
            Self::Ac(true) => f.write_str("AC in"),
            Self::Ac(false) => f.write_str("AC out"),
            Self::Battery(pct) => write!(f, "battery {pct} %"),
            Self::Input => f.write_str("input"),
            Self::Run(ms) => write!(f, "run {}", seconds(*ms)),
            Self::Suspend(ms) => write!(f, "user sleeps the PC for {}", seconds(*ms)),
            Self::WallJump(ms) => write!(f, "wall clock {ms:+} ms"),
            Self::Lock => f.write_str("user locks"),
            Self::Unlock => f.write_str("user unlocks"),
            Self::ConsoleLeft => f.write_str("another session takes the console"),
            Self::ConsoleBack => f.write_str("console back"),
            Self::Ending => f.write_str("session ending"),
            Self::EndCancelled => f.write_str("session end cancelled"),
            Self::TakeOwner => f.write_str("settings mutex taken"),
            Self::UserSets {
                plan: None,
                key,
                value,
            } => write!(f, "user sets the active plan's {key:?} = {value}"),
            Self::UserSets {
                plan: Some(plan),
                key,
                value,
            } => {
                let id = PLANS[plan % PLANS.len()];
                write!(f, "user sets {} {key:?} = {value}", plan_name(id))
            }
            Self::Switch(plan) => {
                let id = PLANS[plan % PLANS.len()];
                write!(f, "user switches to {}", plan_name(id))
            }
            Self::Fault { on, at, code } => {
                write!(f, "IO call +{at} of the next {on:?} fails (code {code})")
            }
            Self::LockFails => f.write_str("next lock fails"),
            Self::Hibernation { allowed } => write!(f, "hibernate allowed: {allowed}"),
            Self::UpdateReady { ok } => write!(f, "update ready (applies: {ok})"),
            Self::Io => f.write_str("IO runs one"),
            Self::IoLate => f.write_str("IO runs one, answer waits"),
            Self::Deliver => f.write_str("answers delivered"),
            Self::Flush => f.write_str("IO flush"),
        }
    }
}

fn run_file(n: u8) -> String {
    format!("run-{}.json", 4000 + u32::from(n))
}

/// How a case ends, before the restore property looks at the plans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Every hold goes, then the PC runs long enough for the restore retries; a closed lid then opens, since the sleep
    /// route's short timeouts stay while it is closed.
    LastHold,
    Quit,
    /// `WM_ENDSESSION`, then the session ends.
    SessionEnd,
}

/// The world when the tray starts.
#[derive(Clone, PartialEq, Eq)]
pub struct WorldStart {
    /// Indexes [`PLANS`].
    pub active: usize,
    pub automatic: bool,
    /// This tray holds the settings mutex.
    pub owner: bool,
    pub on_console: bool,
    pub locked: bool,
    pub lid_closed: bool,
    pub ac: bool,
    pub battery_pct: u8,
    /// A tray that crashed while holding left its values on the active plan, with its journal and RunOnce value.
    pub leftover: bool,
    /// The start-up restore of that journal fails, so `Started` carries it and the core retries.
    pub startup_fails: bool,
    /// The start-up read of `journal.json` fails, so nothing is restored at start and `Started` says it did not read
    /// (power safety review finding 3); with `leftover`, the crashed tray's values stay until the core's retry.
    pub startup_unread: bool,
    /// The IO thread and the loop keep up: after every step each command runs and each answer is handled.
    pub keep_up: bool,
}

impl Default for WorldStart {
    fn default() -> Self {
        Self {
            active: 0,
            automatic: true,
            owner: true,
            on_console: true,
            locked: false,
            lid_closed: false,
            ac: true,
            battery_pct: 80,
            leftover: false,
            startup_fails: false,
            startup_unread: false,
            keep_up: true,
        }
    }
}

impl std::fmt::Debug for WorldStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flag = |on: bool, name: &str| {
            if on {
                format!(" {name}")
            } else {
                String::new()
            }
        };
        write!(
            f,
            "start on {}, lid {}, on {} at {} %{}{}{}{}{}{}{}{}{}",
            plan_name(PLANS[self.active % PLANS.len()]),
            if self.lid_closed { "closed" } else { "open" },
            if self.ac { "AC" } else { "battery" },
            self.battery_pct,
            flag(!self.automatic, "automatic-off"),
            flag(!self.owner, "not-owner"),
            flag(!self.on_console, "off-console"),
            flag(self.locked, "locked"),
            flag(self.leftover, "leftover"),
            flag(self.startup_fails, "startup-restore-fails"),
            flag(self.startup_unread, "startup-read-fails"),
            flag(self.keep_up, "io-keeps-up"),
            flag(!self.keep_up, "io-lags"),
        )
    }
}

/// What Windows and the user's side hold at one moment, for the properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truth {
    pub at: Now,
    pub lid_closed: bool,
    pub ac: bool,
    pub battery_pct: u8,
    /// The session is locked (by the user or by a `Lock` the locker ran).
    pub locked: bool,
    pub on_console: bool,
    pub active: Guid,
    /// The values in force on the active plan, each with who set it.
    pub lid_ac: (u32, SetBy),
    pub lid_dc: (u32, SetBy),
    pub sleep_dc: (u32, SetBy),
    pub sleep_ac: (u32, SetBy),
    pub screen_ac: (u32, SetBy),
    pub screen_dc: (u32, SetBy),
    /// No command waits for the IO thread and no answer waits for the loop: the core has heard all the IO did.
    pub in_sync: bool,
    /// Windows holds a value of ours (stored on any plan, or in force on the active one), or an apply is not answered
    /// yet. A journal entry whose value was never written does not count: nothing of ours is in force for it.
    pub owned: bool,
    /// A hold file other than Keep awake names a process that is not gone.
    pub live_file: bool,
    pub last_input_mono: u64,
    pub last_resume_mono: Option<u64>,
}

impl Truth {
    /// The lid action in force for the current power source.
    pub fn lid(&self) -> (u32, SetBy) {
        if self.ac { self.lid_ac } else { self.lid_dc }
    }

    pub fn in_force(&self, key: SettingKey) -> (u32, SetBy) {
        match key {
            SettingKey::LidAc => self.lid_ac,
            SettingKey::LidDc => self.lid_dc,
            SettingKey::SleepDc => self.sleep_dc,
            SettingKey::SleepAc => self.sleep_ac,
            SettingKey::VideoIdleAc => self.screen_ac,
            SettingKey::VideoIdleDc => self.screen_dc,
        }
    }
}

/// One thing that happened, in order, with the world as it was.
#[derive(Clone, Debug)]
pub enum Entry {
    /// The core handled `sim.steps()[step]`; `truth` is the world when the event came.
    Core { step: usize, truth: Truth },
    /// The IO thread ran `command`; `truth` is the world right after.
    Io { command: Command, truth: Truth },
    /// A world step ended, or a flush did; `truth` is the world then, also after a change no event tells (a value set
    /// in Settings). Unlike a core entry's, it comes after the commands of every step before it were taken.
    World { truth: Truth },
}

impl Entry {
    pub fn truth(&self) -> &Truth {
        match self {
            Self::Core { truth, .. } | Self::Io { truth, .. } | Self::World { truth } => truth,
        }
    }
}

/// The core in its world: the power plans on a [`FakePlans`], the IO thread that runs `ReadPlan`, `Apply`, `Restore`
/// and `ResetLost` through the real ownership routines as `p3-power-io`'s executor does, the loop's other runners
/// (lock, hibernate, update, hold files), and the user, each change turned into the event Windows or a watch thread
/// posts.
///
/// The IO thread runs its commands in order. With `keep_up` every command runs and every answer is handled after each
/// step; without it, commands wait for [`WorldStep::Io`] and [`WorldStep::IoLate`], whose answers may wait for
/// [`WorldStep::Deliver`]. Answers always reach the core in the order they were posted.
///
/// Requests and the cheap file commands are taken at once and answer nothing the core needs: requests always work,
/// and `WriteHold` and `DeleteHold` go to the hold folder at once ([`Sim::watch`]). A Keep awake file always names
/// this tray, which lives.
#[derive(Debug)]
pub struct World {
    sim: Sim,
    plans: FakePlans,
    io: VecDeque<Command>,
    posted: VecDeque<Event>,
    /// Sim steps whose commands the world has taken.
    taken: usize,
    log: Vec<Entry>,
    keep_up: bool,
    /// The user's own value of every key on every plan: the start's, then each value set in Settings.
    user: BTreeMap<(Guid, SettingKey), u32>,
    lid_closed: bool,
    ac: bool,
    battery_pct: u8,
    locked: bool,
    on_console: bool,
    owner: bool,
    /// `Session(Ending)` went in and no `EndCancelled` yet.
    ending: bool,
    /// The IO queue puts the next end-of-session restore first (`IoQueue::session_ending`).
    ending_front: bool,
    /// The tray quit, or its session ended: nothing reaches the core any more.
    quit: bool,
    agents: BTreeMap<u32, (Activity, Alive)>,
    last_input_mono: u64,
    last_resume_mono: Option<u64>,
    lock_fails: bool,
    hibernate_allowed: bool,
    update_ok: bool,
    /// Faults waiting for the next command of their kind: call offset and code.
    armed: Vec<(FaultOn, usize, u32)>,
}

impl World {
    /// The tray starts: the start-up restore of a crashed tray's journal, `Started`, a lock that began before the
    /// tray, then the first lid and power readings (states, not changes) and the user's input.
    pub fn new(start: &WorldStart) -> Self {
        let active = PLANS[start.active % PLANS.len()];
        let plans = FakePlans::new(world_plan(active));
        for id in PLANS.into_iter().filter(|&id| id != active) {
            plans.add_plan(world_plan(id));
        }
        let mut user = BTreeMap::new();
        for id in PLANS {
            let values = world_plan(id);
            for key in SettingKey::ALL {
                user.insert((id, key), key_value(&values, key));
            }
        }
        let mut world = Self {
            sim: Sim::new(),
            plans,
            io: VecDeque::new(),
            posted: VecDeque::new(),
            taken: 0,
            log: Vec::new(),
            keep_up: start.keep_up,
            user,
            lid_closed: start.lid_closed,
            ac: start.ac,
            battery_pct: start.battery_pct,
            locked: start.locked,
            on_console: start.on_console,
            owner: start.owner,
            ending: false,
            ending_front: false,
            quit: false,
            agents: BTreeMap::new(),
            last_input_mono: START.mono_ms,
            last_resume_mono: None,
            lock_fails: false,
            hibernate_allowed: true,
            update_ok: false,
            armed: Vec::new(),
        };
        world.sim.remark(format!("world: {start:?}"));
        if start.leftover {
            world.crashed_tray(active);
        }
        let journal = if start.startup_unread {
            world
                .sim
                .remark("IO: the start-up read of journal.json fails: nothing restored".to_owned());
            None
        } else if start.leftover {
            world.leftover(start.startup_fails)
        } else {
            None
        };
        world.send(
            Start {
                config: Config::new(start.automatic),
                journal,
                journal_unread: start.startup_unread,
                owner: start.owner,
                on_console: start.on_console,
                ..Start::default()
            }
            .event(),
        );
        if start.locked {
            world.send(Event::Session(SessionChange::Locked));
        }
        world.send(Event::Lid {
            closed: start.lid_closed,
        });
        world.send_power();
        world.sim.input();
        world.checkpoint();
        if world.keep_up {
            world.flush();
        }
        world
    }

    /// A crashed tray's values on `plan_id`, its journal and RunOnce value.
    fn crashed_tray(&mut self, plan_id: Guid) {
        let values = world_plan(plan_id);
        let entries: Vec<JournalFileEntry> = KEYS
            .into_iter()
            .filter(|&key| key_value(&values, key) != TARGET)
            .map(|key| JournalFileEntry {
                plan: plan_id,
                key: JournalKey::Known(key),
                original: key_value(&values, key),
                written: TARGET,
                restored: false,
            })
            .collect();
        {
            let mut state = self.plans.state();
            for key in KEYS
                .into_iter()
                .filter(|&k| key_value(&values, k) != TARGET)
            {
                state.set_stored(plan_id, key, TARGET, SetBy::Override);
                state.set_in_force(plan_id, key, TARGET, SetBy::Override);
            }
            state.journal = FakeJournal::File(JournalFile {
                version: JournalFile::VERSION,
                owner: DEAD_TRAY,
                entries,
            });
            state.run_once = true;
        }
    }

    /// The start-up restore runs on the crashed tray's journal at once, its owner being Gone (`p3-power-io`,
    /// `startup`), and may fail. Gives the journal `Started` carries.
    fn leftover(&mut self, fails: bool) -> Option<Journal> {
        if fails {
            let mut state = self.plans.state();
            // The restore's second call reads the active plan; failing it fails the run before anything changes.
            let at = state.calls.len() + 1;
            state.script.insert(at, Action::Fail(2));
        }
        let (mut io, mut net) = (self.plans.clone(), self.plans.clone());
        let restored = restore(&mut io, &mut net, None, Some(DEAD_TRAY));
        self.sim.remark(format!(
            "IO: start-up restore of a crashed tray's journal -> {:?}, {} entries left",
            restored.result,
            restored.journal.len()
        ));
        (!restored.journal.is_empty()).then_some(Journal {
            owner: DEAD_TRAY,
            entries: restored.journal,
        })
    }

    /// One world step, then, with `keep_up`, every command it led to run and answered.
    pub fn play(&mut self, step: &WorldStep) {
        if self.quit {
            return;
        }
        self.sim.remark(format!("world: {step:?}"));
        match *step {
            WorldStep::Agent { pid, now } => {
                match now {
                    AgentNow::Working => {
                        self.agents.insert(pid, (Activity::Working, Alive::Yes));
                    }
                    AgentNow::Idle => {
                        self.agents.insert(pid, (Activity::Idle, Alive::Yes));
                    }
                    AgentNow::Gone => {
                        if let Some(agent) = self.agents.get_mut(&pid) {
                            agent.1 = Alive::Gone;
                        }
                    }
                    AgentNow::Leaves => {
                        self.agents.remove(&pid);
                    }
                }
                self.post_agents();
            }
            WorldStep::RunFile { n, now } => {
                let name = run_file(n);
                let present = self.sim.folder().any(|f| f.id.0 == name);
                match now {
                    FileNow::Alive => {
                        self.sim.put_hold(hold(&name, Alive::Yes));
                    }
                    FileNow::Gone if present => {
                        self.sim.put_hold(hold(&name, Alive::Gone));
                    }
                    FileNow::Removed if present => {
                        self.sim.remove_hold(&name);
                    }
                    FileNow::Gone | FileNow::Removed => {}
                }
                self.take();
            }
            WorldStep::Menu(command) => self.send(Event::Menu(command)),
            WorldStep::Lid { closed } => {
                if closed != self.lid_closed {
                    self.lid_closed = closed;
                    self.send(Event::Lid { closed });
                }
            }
            WorldStep::Ac(ac) => {
                if ac != self.ac {
                    self.ac = ac;
                    self.send_power();
                }
            }
            WorldStep::Battery(pct) => {
                self.battery_pct = pct;
                self.send_power();
            }
            WorldStep::Input => {
                self.sim.input();
                self.last_input_mono = self.sim.now().mono_ms;
            }
            WorldStep::Run(ms) => self.run(ms),
            WorldStep::Suspend(ms) => {
                self.suspend(ms);
                self.take();
            }
            WorldStep::WallJump(ms) => {
                let wall = self.sim.now().wall_ms.saturating_add(ms);
                self.sim.set_wall(wall);
            }
            WorldStep::Lock => {
                if !self.locked {
                    self.locked = true;
                    self.send(Event::Session(SessionChange::Locked));
                }
            }
            WorldStep::Unlock => {
                if self.locked {
                    self.locked = false;
                    self.send(Event::Session(SessionChange::Unlocked));
                }
            }
            WorldStep::ConsoleLeft => {
                if self.on_console {
                    self.on_console = false;
                    self.send(Event::Session(SessionChange::ConsoleLeft));
                }
            }
            WorldStep::ConsoleBack => {
                if !self.on_console {
                    self.on_console = true;
                    self.send(Event::Session(SessionChange::ConsoleBack));
                }
            }
            WorldStep::Ending => self.session_ending(),
            WorldStep::EndCancelled => {
                if self.ending {
                    self.ending = false;
                    self.ending_front = false;
                    self.send(Event::Session(SessionChange::EndCancelled));
                }
            }
            WorldStep::TakeOwner => {
                if !self.owner {
                    self.owner = true;
                    self.send(Event::SettingsOwner);
                }
            }
            WorldStep::UserSets { plan, key, value } => {
                let active = self.plans.state().active;
                let id = plan.map_or(active, |i| PLANS[i % PLANS.len()]);
                self.plans.state().user_write(id, key, value);
                self.user.insert((id, key), value);
            }
            WorldStep::Switch(plan) => {
                let id = PLANS[plan % PLANS.len()];
                let active = self.plans.state().active;
                if id != active {
                    self.plans.state().switch_plan(id);
                    self.send(Event::ActivePlan(id));
                }
            }
            WorldStep::Fault {
                on: FaultOn::AnyCommand,
                at,
                code,
            } => self.script_fail(at, code),
            WorldStep::Fault { on, at, code } => {
                self.armed.push((on, at, code));
            }
            WorldStep::LockFails => self.lock_fails = true,
            WorldStep::Hibernation { allowed } => self.hibernate_allowed = allowed,
            WorldStep::UpdateReady { ok } => {
                self.update_ok = ok;
                self.send(Event::UpdateReady {
                    version: "1.1.0".to_owned(),
                });
            }
            WorldStep::Io => {
                self.io_one();
                self.deliver();
            }
            WorldStep::IoLate => self.io_one(),
            WorldStep::Deliver => self.deliver(),
            WorldStep::Flush => self.flush(),
        }
        self.checkpoint();
        if self.keep_up {
            self.flush();
        }
    }

    /// The case ends: from here the IO side keeps up and has no faults left, then `end` happens.
    pub fn end(&mut self, end: End) {
        self.sim.remark(format!("ending: {end:?}"));
        self.plans.state().script.clear();
        self.armed.clear();
        self.lock_fails = false;
        self.keep_up = true;
        self.flush();
        if !self.quit {
            match end {
                End::LastHold => {
                    self.agents.clear();
                    self.post_agents();
                    let files: Vec<String> = self
                        .sim
                        .folder()
                        .filter(|f| f.kind != HoldKind::KeepAwake)
                        .map(|f| f.id.0.clone())
                        .collect();
                    for name in files {
                        self.sim.remove_hold(&name);
                        self.take();
                    }
                    self.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Off)));
                    self.flush();
                    self.run(RETRIES_MS);
                    // With the lid closed the sleep route's short timeouts stay by design until the user is back
                    // (`p2-6-sleep-route`): the lid opens, and they go back with the retries.
                    if self.lid_closed {
                        self.lid_closed = false;
                        self.send(Event::Lid { closed: false });
                        self.flush();
                        self.run(RETRIES_MS);
                    }
                }
                End::Quit => self.send(Event::Menu(MenuCommand::Quit)),
                End::SessionEnd => {
                    // A session end from the steps may have met a fault: a failed end-of-session restore is the gap
                    // RunOnce covers (architecture section 5, "Gap left"). This one gets its fault-free chance.
                    if self.ending {
                        self.ending = false;
                        self.ending_front = false;
                        self.send(Event::Session(SessionChange::EndCancelled));
                        self.flush();
                    }
                    self.session_ending();
                    self.flush();
                    self.send(Event::Session(SessionChange::Ended));
                    self.flush();
                    // Windows ends the process once the end-of-session wait is over.
                    self.quit = true;
                    self.finish();
                }
            }
        }
        self.flush();
        self.checkpoint();
    }

    pub fn sim(&self) -> &Sim {
        &self.sim
    }

    pub fn plans(&self) -> &FakePlans {
        &self.plans
    }

    pub fn log(&self) -> &[Entry] {
        &self.log
    }

    pub fn now(&self) -> Now {
        self.sim.now()
    }

    /// The tray quit or its session ended.
    pub fn quit(&self) -> bool {
        self.quit
    }

    /// The user's own value of `key` on `plan`: the start's, or the last one set in Settings.
    pub fn user_value(&self, plan: Guid, key: SettingKey) -> u32 {
        self.user.get(&(plan, key)).copied().unwrap_or(TARGET)
    }

    /// The core's trace with every world step and IO run between its steps.
    pub fn trace(&self) -> String {
        self.sim.trace()
    }

    fn checkpoint(&mut self) {
        let truth = self.truth();
        self.log.push(Entry::World { truth });
    }

    fn truth(&self) -> Truth {
        let state = self.plans.state();
        let active = state.active;
        let force = |key| {
            (
                state.in_force(active, key).unwrap_or(TARGET),
                state.in_force_by(active, key).unwrap_or(SetBy::User),
            )
        };
        let apply_pending = self.io.iter().any(|c| matches!(c, Command::Apply { .. }))
            || self
                .posted
                .iter()
                .any(|e| matches!(e, Event::Done(Outcome::Applied { .. })));
        Truth {
            at: self.sim.now(),
            lid_closed: self.lid_closed,
            ac: self.ac,
            battery_pct: self.battery_pct,
            locked: self.locked,
            on_console: self.on_console,
            active,
            lid_ac: force(SettingKey::LidAc),
            lid_dc: force(SettingKey::LidDc),
            sleep_dc: force(SettingKey::SleepDc),
            sleep_ac: force(SettingKey::SleepAc),
            screen_ac: force(SettingKey::VideoIdleAc),
            screen_dc: force(SettingKey::VideoIdleDc),
            in_sync: self.io.is_empty() && self.posted.is_empty(),
            owned: !state.held().is_empty() || apply_pending,
            live_file: self
                .sim
                .folder()
                .any(|f| f.kind != HoldKind::KeepAwake && f.alive != Alive::Gone),
            last_input_mono: self.last_input_mono,
            last_resume_mono: self.last_resume_mono,
        }
    }

    /// One event into the core now, and the commands it leads to taken.
    fn send(&mut self, event: Event) {
        if self.quit {
            return;
        }
        self.sim.send(event);
        self.take();
    }

    fn send_power(&mut self) {
        self.send(Event::Power {
            ac: self.ac,
            battery_pct: Some(self.battery_pct),
        });
    }

    fn post_agents(&mut self) {
        let snapshot = self
            .agents
            .iter()
            .map(|(&pid, &(activity, alive))| (claude(pid, activity), alive))
            .collect();
        self.send(sources(snapshot));
    }

    /// The commands of every core step not taken yet, in order, as the dispatcher hands them out: the IO ones to the
    /// IO queue, the loop's runners at once, their answers back into the core after the batch.
    fn take(&mut self) {
        while self.taken < self.sim.steps().len() {
            let step = self.taken;
            self.taken += 1;
            let truth = self.truth();
            self.log.push(Entry::Core { step, truth });
            let commands = self.sim.steps()[step].commands.clone();
            let mut answers = Vec::new();
            let mut holds = false;
            for command in commands {
                match command {
                    Command::Restore {
                        why: RestoreWhy::SessionEnd,
                        ..
                    } if self.ending_front => {
                        self.ending_front = false;
                        self.io.push_front(command);
                    }
                    Command::ReadPlan
                    | Command::Apply { .. }
                    | Command::Restore { .. }
                    | Command::ResetLost { .. } => {
                        self.io.push_back(command);
                    }
                    Command::Lock => self.lock(&mut answers),
                    Command::SleepCheck {
                        reason, floor_pct, ..
                    } => {
                        let result = self.last_check(reason, floor_pct);
                        self.posted
                            .push_back(Event::Done(Outcome::SleepCheck(result)));
                    }
                    Command::Hibernate {
                        reason, floor_pct, ..
                    } => self.hibernate(reason, floor_pct),
                    Command::ApplyUpdate => {
                        let result = if self.update_ok {
                            Ok(())
                        } else {
                            Err(UPDATE_FAILED)
                        };
                        self.posted.push_back(Event::Done(Outcome::Update(result)));
                    }
                    Command::WriteHold { .. } | Command::DeleteHold(_) => holds = true,
                    Command::Quit => self.quit = true,
                    _ => {}
                }
            }
            if self.quit {
                self.finish();
                return;
            }
            for event in answers {
                self.sim.send(event);
            }
            if holds {
                self.sim.watch();
            }
        }
    }

    /// `LockWorkStation`, answered at once; Windows then tells the session is locked.
    fn lock(&mut self, answers: &mut Vec<Event>) {
        if std::mem::take(&mut self.lock_fails) {
            answers.push(Event::Done(Outcome::Locked(Err(LOCK_FAILED))));
            return;
        }
        answers.push(Event::Done(Outcome::Locked(Ok(()))));
        if !self.locked {
            self.locked = true;
            answers.push(Event::Session(SessionChange::Locked));
        }
    }

    /// The hibernate thread's last check (architecture section 5, "Lock and hibernate"), as `SleepCheck` and
    /// `Hibernate` both run it. The sleep check's answer is posted like an IO outcome, so it may come late. The world
    /// does not model Windows' own idle sleep under the short timeouts, so its fallback hibernate always comes.
    fn last_check(&self, reason: CountdownReason, floor_pct: u8) -> Result<(), HibernateSkip> {
        let now = self.sim.now();
        let since_resume = self
            .last_resume_mono
            .map_or(u64::MAX, |at| now.mono_ms.saturating_sub(at));
        let idle = self.sim.idle_ms().unwrap_or(u64::MAX).min(since_resume);
        let floor = reason == CountdownReason::Floor;
        if !self.lid_closed {
            Err(HibernateSkip::LidOpen)
        } else if idle < IDLE_MS {
            Err(HibernateSkip::InputSeen)
        } else if floor && self.ac {
            Err(HibernateSkip::OnAc)
        } else if floor && self.battery_pct >= floor_pct {
            Err(HibernateSkip::AboveFloor)
        } else {
            Ok(())
        }
    }

    /// The last check, then the PC hibernates and resumes, or the answer says why not. The answer is posted like an IO
    /// outcome, so it may come late.
    fn hibernate(&mut self, reason: CountdownReason, floor_pct: u8) {
        let end = match self.last_check(reason, floor_pct) {
            Err(skip) => HibernateEnd::Skipped(skip),
            Ok(()) if !self.hibernate_allowed => HibernateEnd::NotAllowed,
            Ok(()) => HibernateEnd::Resumed,
        };
        if end == HibernateEnd::Resumed {
            self.suspend(HIBERNATED_MS);
        }
        self.posted.push_back(Event::Done(Outcome::Hibernate(end)));
    }

    /// `Suspending`, only the wall clock moves, `Resumed`.
    fn suspend(&mut self, ms: u64) {
        self.sim.suspend(ms);
        self.last_resume_mono = Some(self.sim.now().mono_ms);
    }

    /// `IoQueue::session_ending`: the queue drops the applies it holds and puts the end-of-session restore first
    /// (`docs/phases/06-integration.md` section 4); then the core hears of it.
    fn session_ending(&mut self) {
        if self.ending {
            return;
        }
        self.ending = true;
        let before = self.io.len();
        self.io.retain(|c| !matches!(c, Command::Apply { .. }));
        if self.io.len() < before {
            self.sim.remark(format!(
                "IO: the session ends: {} queued applies dropped",
                before - self.io.len()
            ));
        }
        self.ending_front = true;
        self.send(Event::Session(SessionChange::Ending));
    }

    fn run(&mut self, ms: u64) {
        let end = self.sim.now().mono_ms.saturating_add(ms);
        while self.sim.fire_next(end).is_some() {
            self.take();
            if self.keep_up {
                self.flush();
            }
            if self.quit {
                break;
            }
        }
        self.sim.wait_until(end);
        self.sim.remark(format!("ran {}", seconds(ms)));
    }

    fn io_one(&mut self) {
        if let Some(command) = self.io.pop_front() {
            self.run_io(command);
        }
    }

    fn deliver(&mut self) {
        while let Some(event) = self.posted.pop_front() {
            self.send(event);
        }
    }

    /// Ends with a checkpoint: the world once the IO side caught up.
    ///
    /// # Panics
    ///
    /// When the core and the IO side never quiet down.
    fn flush(&mut self) {
        for _ in 0..FLUSH_LIMIT {
            if let Some(command) = self.io.pop_front() {
                self.run_io(command);
                self.deliver();
            } else if !self.posted.is_empty() {
                self.deliver();
            } else {
                self.checkpoint();
                return;
            }
        }
        panic!(
            "the core and the IO side never quiet down\n{}",
            self.trace()
        );
    }

    /// The tray is gone: the IO thread runs what is queued (`IoThread::finish`), with no faults left, since a failed
    /// restore there is the guard's to retry, not the core's; nothing reaches the core any more.
    fn finish(&mut self) {
        self.plans.state().script.clear();
        self.armed.clear();
        while let Some(command) = self.io.pop_front() {
            self.run_io(command);
        }
        self.posted.clear();
    }

    /// The fake's call `at` calls from now fails with `code`.
    fn script_fail(&mut self, at: usize, code: u32) {
        let mut state = self.plans.state();
        let index = state.calls.len() + at;
        state.script.insert(index, Action::Fail(code));
    }

    /// One IO command through the routine `p3-power-io`'s executor calls for it; its events are posted in its order.
    fn run_io(&mut self, command: Command) {
        let kind = match command {
            Command::ReadPlan => FaultOn::ReadPlan,
            Command::Apply { .. } => FaultOn::Apply,
            _ => FaultOn::Restore,
        };
        if let Some(i) = self.armed.iter().position(|&(on, ..)| on == kind) {
            let (_, at, code) = self.armed.remove(i);
            self.script_fail(at, code);
        }
        let (mut io, mut net) = (self.plans.clone(), self.plans.clone());
        let events = match &command {
            // Values first, then the check, which runs also when the values fail (`p3-power-io`, design choice 4).
            Command::ReadPlan => {
                let mut events = vec![Event::Plan(io.values())];
                let checked = check(&mut io, &mut net);
                events.extend(checked.user_changed.into_iter().map(Event::UserChanged));
                events
            }
            Command::Apply { plan, writes } => {
                let applied = apply(&mut io, &mut net, ME, *plan, writes);
                vec![Event::Done(Outcome::Applied {
                    plan: *plan,
                    written: applied.written,
                    result: applied.result,
                    journal: applied.journal,
                })]
            }
            Command::Restore { plan, why } => {
                let journal = self.plans.state().journal.clone();
                // A leftover runs only for a Gone owner or this tray, with that owner checked; in this world every
                // journal is this tray's or the crashed one's. With no file, or one that does not read, it answers
                // without the routine; a corrupt one goes to the routine, which sets it aside (`p3-power-io`).
                let restored = match (*why, journal) {
                    (RestoreWhy::Leftover, FakeJournal::Missing) => Restored {
                        found: Found::Nothing,
                        result: Ok(()),
                        journal: Vec::new(),
                    },
                    (RestoreWhy::Leftover, FakeJournal::Unreadable(code)) => Restored {
                        found: Found::Nothing,
                        result: Err(Failure {
                            call: "FakePlans::load_journal",
                            code,
                        }),
                        journal: Vec::new(),
                    },
                    (RestoreWhy::Leftover, FakeJournal::File(file)) => {
                        restore(&mut io, &mut net, *plan, Some(file.owner))
                    }
                    _ => restore(&mut io, &mut net, *plan, None),
                };
                let mut events = vec![Event::Done(Outcome::Restored {
                    plan: *plan,
                    why: *why,
                    result: restored.result,
                    journal: restored.journal,
                })];
                if restored.found == Found::SetAside && restored.result.is_ok() {
                    events.push(Event::JournalCorrupt);
                }
                events
            }
            Command::ResetLost { damaged } => {
                let reset = reset_lost(&mut io, &mut net, *damaged);
                // What the reset put in force is now the user's own as far as the world goes: rule 7 gives the
                // default in place of a value no record names. So it is not Wakeshot's either, also when the reset's
                // own load failed on a file that still named the key (`p2-6-props-flake`).
                if let Some(plan) = reset.plan {
                    for (key, outcome) in &reset.keys {
                        if let KeyReset::Reset { default, .. } = outcome {
                            self.user.insert((plan, *key), *default);
                            self.plans.state().mark_reset(plan, *key, *default);
                        }
                    }
                }
                vec![Event::Done(Outcome::ResetLost {
                    reset: reset.reset_keys(),
                    result: reset.result,
                    journal: reset.journal,
                })]
            }
            _ => Vec::new(),
        };
        let answers: Vec<String> = events.iter().map(event_text).collect();
        self.sim.remark(format!(
            "IO: {} -> {}",
            command_text(&command),
            answers.join("; ")
        ));
        self.posted.extend(events);
        let truth = self.truth();
        self.log.push(Entry::Io { command, truth });
    }
}
