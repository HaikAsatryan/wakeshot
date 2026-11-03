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

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::command::{ApplyError, Command, Failure, FileKind, Outcome, RestoreWhy};
use crate::core::Core;
use crate::event::Event;
use crate::files::{Config, HoldFile, StateFile};
use crate::holds::file::{self, Parsed};
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::policy::requests::Requests;
use crate::process::{Alive, FileTime, ProcessRef};
use crate::settings::{Guid, Journal, JournalEntry, PlanValues, SettingKey};
use crate::sources::{Activity, FormatHealth, Liveness, SessionFacts, SessionKey, SourceId};
use crate::testing::FakeClock;
use crate::time::{Clock, Deadline, Now};
use crate::view::{Notice, TrayView};

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

/// A plan read of `active` holding Haik's own values: lid Sleep on AC and battery, sleep after 3 h on AC and 10 min
/// on battery, energy saver at 20 %.
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

/// The journal entries an apply leaves on `plan_id` for `keys`: each target 0 over the values of [`plan`].
pub fn entries(plan_id: Guid, keys: &[SettingKey]) -> Vec<JournalEntry> {
    let values = plan(plan_id);
    keys.iter()
        .map(|&key| JournalEntry {
            plan: plan_id,
            key,
            original: match key {
                SettingKey::LidAc => values.lid_ac,
                SettingKey::LidDc => values.lid_dc,
                SettingKey::SleepDc => values.sleep_dc,
            },
            written: 0,
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

/// One event as the core got it, with the commands it gave.
#[derive(Clone, Debug)]
pub struct Step {
    pub at: Now,
    pub event: Event,
    pub commands: Vec<Command>,
}

/// `Event::Started` with the values a test usually wants: Automatic on, no journal, settings owner, on the console,
/// this [`TRAY`], not the first run. Change a field with struct update syntax.
#[derive(Clone, Debug)]
pub struct Start {
    pub config: Config,
    pub journal: Option<Journal>,
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
        loop {
            let mono = self.clock.now().mono_ms;
            let due = self
                .timer_mono
                .map_or(self.next_safety_mono, |t| t.min(self.next_safety_mono))
                .max(mono);
            if due > end {
                self.clock.run(end - mono);
                break;
            }
            self.clock.run(due - mono);
            // `send` refuses a deadline at or before now after a Tick, so this loop always moves on.
            while self.next_safety_mono <= due {
                self.next_safety_mono += SAFETY_TICK_MS;
            }
            if let Some(ms) = self.idle_ms() {
                out.extend(self.send(Event::InputIdle { ms }));
            }
            out.extend(self.send(Event::Tick));
        }
        if ms > 0 {
            self.line(format!("ran {}", seconds(ms)));
        }
        out
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
