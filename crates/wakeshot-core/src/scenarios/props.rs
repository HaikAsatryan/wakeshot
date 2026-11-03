//! Property tests over `Core` in its world (architecture section 9, the list in `p2-core-props`): random sequences of
//! world steps (agent sessions and hold files coming and going, menu choices, lid, power, session and console changes,
//! the user's own changes in Settings, plan switches, IO faults, answers that come late, and time passing), each case
//! checked against what Windows would hold, not only against the commands.
//!
//! Owner: `p2-core-props`.
//!
//! `sim::World` plays Windows, the IO thread and the user; its log says what Windows held at each moment. `Model` below
//! follows the design rules a property needs (which holds count, the episode and its released keys, the step back,
//! whether the core applies now, whether the user is away), from the events the core got, written from sections 3 and
//! 5, not from the core's code. A shrunk case prints one line per world step; the failure message carries the trace,
//! with the IO thread's runs between the core's steps.
//!
//! Each property draws from the general mix of steps, and the lid and restore properties also from a focused mix of
//! the same steps (`lid_case`, `late_case`), since the general mix seldom builds their longer chains
//! (`coverage_tally` counts them). The two strict forms `p2-core-props` gave up for its findings 1 and 2 are tests
//! again since the power review fixes (`p2-core-lid`, "Power review fixes").

use std::collections::BTreeSet;
use std::fmt;

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError};

use crate::command::{ApplyError, Command, Outcome};
use crate::event::{Event, SessionChange};
use crate::holds::HoldFacts;
use crate::ownership::fake::SetBy;
use crate::policy::ending::IDLE_MS;
use crate::policy::lid::{DO_NOTHING, SOURCE_SETTLE_MS};
use crate::policy::{floor, hold_files};
use crate::process::Alive;
use crate::settings::{PlanValues, SettingKey};
use crate::sim::{
    self, AgentNow, End, Entry, FaultOn, FileNow, KEYS, PLANS, Truth, World, WorldStart, WorldStep,
};
use crate::sources::Activity;
use crate::time::Now;
use crate::view::{Cue, KeepAwake, MenuCommand, Notice};

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

/// The access denied codes the design names (section 8): requests only for the whole run.
const ACCESS_DENIED: [u32; 2] = [5, 1260];

// The case.

/// One random case: the world at the start, its steps, and how it ends.
#[derive(Clone)]
struct Case {
    start: WorldStart,
    steps: Vec<WorldStep>,
    end: End,
}

impl fmt::Debug for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self.start)?;
        for (i, step) in self.steps.iter().enumerate() {
            writeln!(f, "{:>3}. {step:?}", i + 1)?;
        }
        write!(f, "end: {:?}", self.end)
    }
}

fn play(case: &Case) -> World {
    let mut world = World::new(&case.start);
    for step in &case.steps {
        world.play(step);
    }
    world.end(case.end);
    world
}

/// A broken property as a test failure, with the whole story.
fn verdict(world: &World, result: Result<(), String>) -> Result<(), TestCaseError> {
    result.map_err(|why| TestCaseError::fail(format!("{why}\n\ntrace:\n{}", world.trace())))
}

// The model of the design rules.

/// The design's state as the events the core got make it (architecture section 3), at one moment.
#[derive(Clone, Debug, Default)]
struct Model {
    started: bool,
    quit: bool,
    automatic: bool,
    owner: bool,
    on_console: bool,
    locked: bool,
    session_ending: bool,
    /// Access denied at an apply: requests only for the run.
    requests_only_run: bool,
    agents: Vec<(Activity, Alive)>,
    files: Vec<HoldFacts>,
    /// The last plan read that worked: the floor comes from it, and the active plan the core knows.
    plan: Option<PlanValues>,
    ac: Option<bool>,
    battery_pct: Option<u8>,
    stepped_back: bool,
    step_back_shown: bool,
    episode: Option<Episode>,
    lid_closed: bool,
    /// A lid lock (with the chime) into an unlocked session that no `Session(Locked)` confirmed yet (section 8, "Lock
    /// fails").
    lock_watch: Option<LockWatch>,
    /// The lid lock and its retry both failed: no hold holds until the session locks or the lid opens. Set whenever
    /// the core may have stepped back (it also needs its own view that our Do nothing is in force), so the model may
    /// say "does not apply" when the core does; never the other way.
    lock_stepped_back: bool,
}

#[derive(Clone, Copy, Debug)]
struct LockWatch {
    /// The one retry went out.
    retry: bool,
    /// When a tick decides: 5 s after the lock, or the moment it failed.
    check_mono: u64,
}

/// How long the session may take to report `Locked` after a lid lock (section 8, "Lock fails").
const LOCK_CHECK_MS: u64 = 5 * SECOND;

/// From the first hold that counts to the last.
#[derive(Clone, Debug, Default)]
struct Episode {
    /// Keys the user took back, on every plan (decision 26).
    released: BTreeSet<SettingKey>,
    /// An apply failed (not access denied): requests only for the episode.
    requests_only: bool,
}

/// What one core step changed in the model.
#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    /// The first moment a hold counts in this step back: its notice or sound is due.
    step_back: bool,
}

impl Model {
    /// A hold counts: a working agent while its process lives, or a hold file by `policy::hold_files` (that card's
    /// own rules), with Automatic for all but Keep awake.
    fn holding(&self, now: Now) -> bool {
        let agents = self.automatic
            && self
                .agents
                .iter()
                .any(|&(activity, alive)| activity == Activity::Working && alive == Alive::Yes);
        agents
            || self
                .files
                .iter()
                .any(|f| hold_files::decide(f, Alive::Unknown, self.automatic, now).counts)
    }

    /// The core writes settings now (section 3, "Holding starts" and "A failed `Restore`").
    fn applies(&self) -> bool {
        self.episode
            .as_ref()
            .is_some_and(|e| !e.requests_only && e.released.len() < KEYS.len())
            && self.owner
            && self.on_console
            && !self.stepped_back
            && !self.lock_stepped_back
            && !self.session_ending
            && !self.requests_only_run
    }

    /// This step's `Lock` without the chime is the one retry of a lid lock: at a tick, once the lock failed or went
    /// [`LOCK_CHECK_MS`] without `Session(Locked)`.
    fn lock_retry_due(&self, step: &sim::Step) -> bool {
        step.event == Event::Tick
            && self
                .lock_watch
                .is_some_and(|w| !w.retry && step.at.mono_ms >= w.check_mono)
    }

    /// The lid lock watch through one step, from the section 8 row: the lock with the chime starts it, `Locked` or the
    /// lid opening ends it (and the step back), a failure makes the next tick decide, and a due tick gives the retry
    /// (its `Lock` in the step) or, after the retry, the step back.
    fn take_lock(&mut self, step: &sim::Step) {
        match step.event {
            Event::Session(SessionChange::Locked) | Event::Lid { closed: false } => {
                self.lock_watch = None;
                self.lock_stepped_back = false;
            }
            Event::Done(Outcome::Locked(Err(_))) => {
                if let Some(watch) = self.lock_watch.as_mut() {
                    watch.check_mono = step.at.mono_ms;
                }
            }
            Event::Tick => {
                if let Some(watch) = self.lock_watch.filter(|w| step.at.mono_ms >= w.check_mono) {
                    self.lock_watch = None;
                    if !watch.retry && step.commands.contains(&Command::Lock) {
                        self.lock_watch = Some(LockWatch {
                            retry: true,
                            check_mono: step.at.mono_ms + LOCK_CHECK_MS,
                        });
                    } else if watch.retry && self.lid_closed {
                        self.lock_stepped_back = true;
                    }
                }
            }
            _ => {}
        }
        let lid_lock = step.commands.contains(&Command::Lock)
            && step
                .commands
                .contains(&Command::Play(Cue::LidClosedHolding));
        if lid_lock && !self.locked {
            self.lock_watch = Some(LockWatch {
                retry: false,
                check_mono: step.at.mono_ms + LOCK_CHECK_MS,
            });
        }
    }

    /// The user cannot see a notice now, so it waits for them (track note 18).
    fn away(&self) -> bool {
        self.locked || !self.on_console
    }

    fn floor_pct(&self) -> u8 {
        self.plan.map_or(floor::DEFAULT_PCT, |p| {
            floor::floor_pct(p.saver_threshold_dc)
        })
    }

    fn released(&self, key: SettingKey) -> bool {
        self.episode
            .as_ref()
            .is_some_and(|e| e.released.contains(&key))
    }

    /// One core step: the event's facts, then what follows at its time, in the design's order (the floor, then the
    /// start and end of holding).
    fn take(&mut self, step: &sim::Step) -> Edges {
        let started = matches!(step.event, Event::Started { .. });
        if self.quit || (!self.started && !started) {
            return Edges::default();
        }
        match &step.event {
            Event::Started {
                config,
                owner,
                on_console,
                ..
            } => {
                if self.started {
                    return Edges::default();
                }
                self.started = true;
                self.automatic = config.automatic;
                self.owner = *owner;
                self.on_console = *on_console;
            }
            Event::ConfigChanged(config) => self.automatic = config.automatic,
            Event::Menu(MenuCommand::Automatic(on)) => self.automatic = *on,
            Event::Menu(MenuCommand::Restore) => {
                if let Some(episode) = self.episode.as_mut() {
                    episode.released.extend(KEYS);
                }
            }
            Event::Menu(MenuCommand::Quit) | Event::Done(Outcome::Update(Ok(()))) => {
                self.quit = true;
                return Edges::default();
            }
            Event::Sources { sessions, .. } => {
                self.agents = sessions
                    .iter()
                    .map(|(s, alive)| (s.activity, *alive))
                    .collect();
            }
            Event::Holds(files) => self.files.clone_from(files),
            Event::Plan(Ok(values)) => self.plan = Some(*values),
            Event::UserChanged(changed) => {
                if let Some(episode) = self.episode.as_mut() {
                    episode.released.insert(changed.key);
                }
            }
            Event::Done(Outcome::Applied {
                result: Err(ApplyError::SafetyNet(failure) | ApplyError::Write(failure)),
                ..
            }) => {
                if ACCESS_DENIED.contains(&failure.code) {
                    self.requests_only_run = true;
                } else if let Some(episode) = self.episode.as_mut() {
                    episode.requests_only = true;
                }
            }
            Event::Session(change) => match change {
                SessionChange::Locked => self.locked = true,
                SessionChange::Unlocked => self.locked = false,
                SessionChange::Ending | SessionChange::Ended => self.session_ending = true,
                SessionChange::EndCancelled => self.session_ending = false,
                SessionChange::ConsoleLeft => self.on_console = false,
                SessionChange::ConsoleBack => self.on_console = true,
            },
            Event::SettingsOwner => self.owner = true,
            Event::Power { ac, battery_pct } => {
                self.ac = Some(*ac);
                self.battery_pct = *battery_pct;
            }
            Event::Lid { closed } => self.lid_closed = *closed,
            _ => {}
        }
        self.take_lock(step);
        let was = self.stepped_back;
        if let Some(ac) = self.ac {
            self.stepped_back = floor::stepped_back(was, ac, self.battery_pct, self.floor_pct());
        }
        if !self.stepped_back {
            self.step_back_shown = false;
        }
        match (self.episode.is_some(), self.holding(step.at)) {
            (false, true) => self.episode = Some(Episode::default()),
            (true, false) => self.episode = None,
            _ => {}
        }
        let mut edges = Edges::default();
        if self.stepped_back && self.episode.is_some() && !self.step_back_shown {
            self.step_back_shown = true;
            edges.step_back = true;
        }
        edges
    }
}

/// One log entry with the model just before and just after it.
struct Seen<'a> {
    entry: &'a Entry,
    /// The core's step, for a core entry.
    step: Option<&'a sim::Step>,
    before: Model,
    after: Model,
    edges: Edges,
}

impl Seen<'_> {
    fn truth(&self) -> &Truth {
        self.entry.truth()
    }

    fn commands(&self) -> &[Command] {
        self.step.map_or(&[], |s| &s.commands)
    }

    fn sent(&self, command: &Command) -> bool {
        self.commands().contains(command)
    }

    fn where_(&self) -> String {
        match self.step {
            Some(step) => format!(
                "at run {} ms, event {:?}",
                step.at.mono_ms - sim::START.mono_ms,
                step.event
            ),
            None => format!("at run {} ms", self.truth().at.mono_ms - sim::START.mono_ms),
        }
    }
}

fn replay(world: &World) -> Vec<Seen<'_>> {
    let steps = world.sim().steps();
    let mut model = Model::default();
    world
        .log()
        .iter()
        .map(|entry| {
            let before = model.clone();
            let (step, edges) = match entry {
                Entry::Core { step, .. } => {
                    let step = &steps[*step];
                    (Some(step), model.take(step))
                }
                Entry::Io { .. } | Entry::World { .. } => (None, Edges::default()),
            };
            Seen {
                entry,
                step,
                before,
                after: model.clone(),
                edges,
            }
        })
        .collect()
}

// The properties.

/// Owned keys come back after the last hold, Quit and the end of the session: once the retries settle, nothing of
/// ours is stored on any plan or in force on the active one, every value is the user's own, and no entry is left.
fn restored_at_the_end(world: &World) -> Result<(), String> {
    let state = world.plans().state();
    let held = state.held();
    if !held.is_empty() {
        return Err(format!("still held at the end: {held:?}"));
    }
    let entries = state.known_entries();
    if !entries.is_empty() {
        return Err(format!("the journal still holds {entries:?}"));
    }
    for plan in PLANS {
        for key in KEYS {
            let want = world.user_value(plan, key);
            let stored = state.stored(plan, key);
            if stored != Some(want) {
                return Err(format!(
                    "{plan:?} {key:?} stored {stored:?}, the user's own is {want}"
                ));
            }
            let in_force = state.in_force(plan, key);
            if plan == state.active && in_force != Some(want) {
                return Err(format!(
                    "{plan:?} {key:?} in force {in_force:?}, the user's own is {want}"
                ));
            }
        }
    }
    Ok(())
}

fn writes(command: &Command) -> Option<&[(SettingKey, u32)]> {
    match command {
        Command::Apply { writes, .. } => Some(writes),
        _ => None,
    }
}

/// A released key is never written again in its episode, on any plan (power verify minor 9): no `Apply` sent after
/// the release names it.
fn released_keys_stay_released(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        let Some(episode) = seen.after.episode.as_ref() else {
            continue;
        };
        for writes in seen.commands().iter().filter_map(writes) {
            if let Some((key, _)) = writes.iter().find(|(k, _)| episode.released.contains(k)) {
                return Err(format!(
                    "{key:?} was released in this episode, yet an apply writes it, {}",
                    seen.where_()
                ));
            }
        }
    }
    Ok(())
}

/// Windows' side of minor 9 and product rule 3: once the core knows of a release and the IO side caught up, the
/// key is not ours in force on the active plan for the rest of the episode, unless a restore failed and waits for
/// its retry.
fn released_keys_are_not_ours_in_force(world: &World) -> Result<(), String> {
    let mut restore_failed = false;
    for seen in replay(world) {
        if let Some(step) = seen.step {
            match step.event {
                Event::Done(Outcome::Restored { result: Err(_), .. }) => restore_failed = true,
                Event::Done(Outcome::Restored { result: Ok(()), .. }) => restore_failed = false,
                _ => {}
            }
        }
        // A core entry's truth comes before its own commands ran; a checkpoint's after them.
        let truth = seen.truth();
        if !matches!(seen.entry, Entry::World { .. }) || !truth.in_sync || restore_failed {
            continue;
        }
        for key in KEYS {
            if seen.after.released(key) && truth.in_force(key).1 == SetBy::Override {
                return Err(format!(
                    "{key:?} was released in this episode, yet ours is in force on {:?}, {}",
                    truth.active,
                    seen.where_()
                ));
            }
        }
    }
    Ok(())
}

/// A restore retry never takes an entry a running hold uses (power verify minor 4): while the core applies, a
/// restore sent at a tick names a plan other than the active one it knows.
fn retries_spare_the_running_hold(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        let Some(step) = seen.step else {
            continue;
        };
        let Some(known) = seen.after.plan.map(|p| p.active) else {
            continue;
        };
        if step.event != Event::Tick || !seen.after.applies() {
            continue;
        }
        for command in seen.commands() {
            if let Command::Restore { plan, why } = command
                && plan.is_none_or(|p| p == known)
            {
                return Err(format!(
                    "the retry Restore {{ plan: {plan:?}, why: {why:?} }} takes the running hold's plan {known:?}, {}",
                    seen.where_()
                ));
            }
        }
    }
    Ok(())
}

/// No `Apply` after `Session(Ending)` until `EndCancelled` (power verify minor 6).
fn no_apply_while_the_session_ends(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        if seen.after.session_ending && seen.commands().iter().any(|c| writes(c).is_some()) {
            return Err(format!(
                "an apply while the session ends, {}",
                seen.where_()
            ));
        }
    }
    Ok(())
}

/// The requests as last sent, up to and with this step.
fn requests_now(last: &mut (bool, bool), seen: &Seen<'_>) -> (bool, bool) {
    for command in seen.commands() {
        if let Command::Requests { system, display } = *command {
            *last = (system, display);
        }
    }
    *last
}

/// No `Apply` and no display request while another session has the PC's screen.
fn nothing_for_another_sessions_console(world: &World) -> Result<(), String> {
    let mut last = (false, false);
    for seen in replay(world) {
        let (_, display) = requests_now(&mut last, &seen);
        if seen.step.is_none() || seen.after.on_console || seen.after.quit {
            continue;
        }
        if seen.commands().iter().any(|c| writes(c).is_some()) {
            return Err(format!(
                "an apply while another session has the console, {}",
                seen.where_()
            ));
        }
        if display {
            return Err(format!(
                "the display request is on while another session has the console, {}",
                seen.where_()
            ));
        }
    }
    Ok(())
}

/// No request is on with no hold, and none while stepped back; an apply only while a hold counts.
fn no_request_without_a_hold(world: &World) -> Result<(), String> {
    let mut last = (false, false);
    for seen in replay(world) {
        let (system, display) = requests_now(&mut last, &seen);
        let Some(step) = seen.step else {
            continue;
        };
        if seen.after.quit {
            continue;
        }
        let holding = seen.after.holding(step.at);
        if system && !holding {
            return Err(format!(
                "the system request is on with no hold, {}",
                seen.where_()
            ));
        }
        if system && seen.after.stepped_back {
            return Err(format!(
                "the system request is on while stepped back, {}",
                seen.where_()
            ));
        }
        if display && !system {
            return Err(format!(
                "the display request is on without the system one, {}",
                seen.where_()
            ));
        }
        if seen.after.episode.is_none() && seen.commands().iter().any(|c| writes(c).is_some()) {
            return Err(format!("an apply with no hold, {}", seen.where_()));
        }
    }
    Ok(())
}

/// No `Hibernate` within 5 minutes of input or of a resume.
fn no_hibernate_soon_after_input(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        let Some(step) = seen.step else {
            continue;
        };
        if !step
            .commands
            .iter()
            .any(|c| matches!(c, Command::Hibernate { .. }))
        {
            continue;
        }
        let truth = seen.truth();
        let quiet_since = truth
            .last_resume_mono
            .map_or(truth.last_input_mono, |r| r.max(truth.last_input_mono));
        let quiet = step.at.mono_ms.saturating_sub(quiet_since);
        if quiet < IDLE_MS {
            return Err(format!(
                "Hibernate after only {quiet} ms without input or a resume, {}",
                seen.where_()
            ));
        }
    }
    Ok(())
}

/// The core can know what is in force: the IO side has caught up, and its last plan read names the plan Windows has
/// active. On an answer still on its way, or a read that failed after a plan switch, the core errs toward a lock by
/// design (`p2-core-lid` design choice 1; `Event::ActivePlan` is acted on only through a read).
fn core_can_know(seen: &Seen<'_>) -> bool {
    let truth = seen.truth();
    truth.in_sync && seen.before.plan.is_some_and(|p| p.active == truth.active)
}

/// The lid-close and source lock (a `Lock` with the chime) never goes out while the lid action in force for the
/// current power source is the user's own Do nothing (decision 18). Judged only where [`core_can_know`].
fn no_lock_on_the_users_own_do_nothing(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        let truth = seen.truth();
        let lid_lock =
            seen.sent(&Command::Lock) && seen.sent(&Command::Play(Cue::LidClosedHolding));
        if !lid_lock || !core_can_know(&seen) {
            continue;
        }
        let (value, by) = truth.lid();
        if value == DO_NOTHING && by != SetBy::Override {
            return Err(format!(
                "a lid lock while the user's own Do nothing is in force ({by:?}), {}",
                seen.where_()
            ));
        }
    }
    Ok(())
}

/// The other side of the lid rule (section 3, "Lid closes while the lid action in force is our Do nothing"): a lid
/// close with our Do nothing in force for the current power source gives `Lock` and the chime in the same step, with
/// or without a hold (`p2-core-lid` design choice 4). Judged only where [`core_can_know`], which also leaves out a
/// close before the first plan read, which the strict form of [`closed_laptop_is_locked_in_time`] judges.
fn a_lid_close_on_ours_locks(world: &World) -> Result<(), String> {
    let mut before: Option<&Truth> = None;
    let seen = replay(world);
    for s in &seen {
        let truth = s.truth();
        let closes = s.step.is_some_and(|step| {
            matches!(step.event, Event::Lid { closed: true })
                && before.is_some_and(|b| !b.lid_closed)
        });
        before = Some(truth);
        if !closes || !core_can_know(s) || s.before.quit {
            continue;
        }
        let ours = truth.lid() == (DO_NOTHING, SetBy::Override);
        let locked = s.sent(&Command::Lock) && s.sent(&Command::Play(Cue::LidClosedHolding));
        if ours && !locked {
            return Err(format!(
                "the lid closed on our Do nothing with no lock and chime, {}",
                s.where_()
            ));
        }
    }
    Ok(())
}

/// Decision 0: every `Lock`, step back and user change comes with a notice or a sound in the same step (a notice
/// raised while the user is away comes at their return, track note 18), but for the one retry of a lid lock, whose
/// chime went with the lock it retries; every `Hibernate` is its own explanation, so the core never plays the going
/// to sleep sound; each run of failed restores gets one `RestoreFailed`.
fn every_action_is_explained(world: &World) -> Result<(), String> {
    // Notices raised while the user was away, owed at their return.
    let mut owed: Vec<Notice> = Vec::new();
    // A `RestoreFailed` was raised since the last restore of every plan that worked (which ends a run for sure).
    let mut warned = false;
    // A `RestoreFailed` was sent since the last restore that worked (any plan).
    let mut sent_since_ok = false;
    for seen in replay(world) {
        let Some(step) = seen.step else {
            continue;
        };
        let commands = &step.commands;
        let notified = |notice: &Notice| commands.contains(&Command::Notify(notice.clone()));
        let returning = seen.before.away() && !seen.after.away();
        if returning {
            if let Some(missing) = owed.iter().find(|n| !notified(n)) {
                return Err(format!(
                    "{missing:?}, raised while the user was away, did not come at the return, {}",
                    seen.where_()
                ));
            }
            owed.clear();
        }
        let mut expect = |notice: Notice| -> Result<(), String> {
            if seen.after.away() {
                if !owed.contains(&notice) {
                    owed.push(notice);
                }
                Ok(())
            } else if notified(&notice) {
                Ok(())
            } else {
                Err(format!("no {notice:?} in the step, {}", seen.where_()))
            }
        };

        if commands.contains(&Command::Lock)
            && !commands.iter().any(|c| {
                matches!(
                    c,
                    Command::Play(Cue::LidClosedHolding) | Command::Notify(Notice::IdleLocked)
                )
            })
            && !seen.before.lock_retry_due(step)
        {
            return Err(format!(
                "a Lock with no chime and no notice, {}",
                seen.where_()
            ));
        }
        if commands.contains(&Command::Play(Cue::GoingToSleep)) {
            return Err(format!(
                "the core played the going to sleep sound, {}",
                seen.where_()
            ));
        }
        if seen.edges.step_back
            && !commands.iter().any(|c| {
                matches!(
                    c,
                    Command::Play(Cue::BatteryFloor) | Command::Notify(Notice::StepBack { .. })
                )
            })
        {
            return Err(format!(
                "a step back with no sound and no notice, {}",
                seen.where_()
            ));
        }

        let changed = match step.event {
            Event::UserChanged(changed) => Some(changed.key),
            _ => None,
        };
        if let Some(key) = changed
            && !seen.before.released(key)
        {
            expect(Notice::UserChanged(key))?;
        }
        let spurious = commands
            .iter()
            .any(|c| matches!(c, Command::Notify(Notice::UserChanged(k)) if Some(*k) != changed));
        if spurious && !returning {
            return Err(format!(
                "a UserChanged notice with no user change, {}",
                seen.where_()
            ));
        }

        let sent = notified(&Notice::RestoreFailed);
        match step.event {
            Event::Done(Outcome::Restored { result: Err(_), .. }) => {
                if !warned {
                    expect(Notice::RestoreFailed)?;
                    warned = true;
                } else if sent && sent_since_ok {
                    return Err(format!(
                        "a second RestoreFailed in one run of failed restores, {}",
                        seen.where_()
                    ));
                }
                sent_since_ok |= sent;
            }
            Event::Done(Outcome::Restored {
                result: Ok(()),
                plan,
                ..
            }) => {
                sent_since_ok = false;
                if plan.is_none() {
                    warned = false;
                }
            }
            _ if sent && !returning => {
                return Err(format!(
                    "a RestoreFailed with no failed restore, {}",
                    seen.where_()
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// A closed laptop on battery on our Do nothing, with the session unlocked on the PC's own screen: our Do nothing alone
/// keeps it running, so nobody may carry it unlocked.
fn runs_closed_unlocked_on_ours(truth: &Truth) -> bool {
    truth.lid_closed
        && !truth.ac
        && truth.lid_dc == (DO_NOTHING, SetBy::Override)
        && !truth.locked
        && truth.on_console
}

/// A closed laptop never runs on battery on our Do nothing unlocked for more than 30 s (power verify minor 11; the
/// settle, Haik 2026-11-03), from a lid close or an AC to battery change that leaves it so. Judged from a change the
/// core heard where [`core_can_know`], and excused when the lock itself failed (section 8: then the retry and the step
/// back). `strict` also judges a change before the core's first plan read (finding 2: a crashed tray's values in the first
/// moments after the start).
fn closed_laptop_is_locked_in_time(world: &World, strict: bool) -> Result<(), String> {
    let seen = replay(world);
    let mut stretch: Option<u64> = None;
    let mut lock_failed = false;
    let mut before: Option<&Truth> = None;
    for s in &seen {
        let truth = s.truth();
        let runs = runs_closed_unlocked_on_ours(truth);
        if let Some(since) = stretch
            && !runs
        {
            let lasted = truth.at.mono_ms.saturating_sub(since);
            if lasted > SOURCE_SETTLE_MS && !lock_failed {
                return Err(format!(
                    "the closed laptop ran unlocked on battery on our Do nothing for {lasted} ms, until {}",
                    s.where_()
                ));
            }
            stretch = None;
            lock_failed = false;
        }
        if let Some(step) = s.step {
            if matches!(step.event, Event::Done(Outcome::Locked(Err(_)))) {
                lock_failed = true;
            }
            let change = match step.event {
                Event::Lid { closed: true } => before.is_some_and(|b| !b.lid_closed),
                Event::Power { ac: false, .. } => before.is_some_and(|b| b.ac),
                _ => false,
            };
            let knows = core_can_know(s) || (strict && truth.in_sync && s.before.plan.is_none());
            if stretch.is_none() && runs && change && knows {
                stretch = Some(truth.at.mono_ms);
            }
        }
        before = Some(truth);
    }
    if let Some(since) = stretch {
        let lasted = world.now().mono_ms.saturating_sub(since);
        if lasted > SOURCE_SETTLE_MS && !lock_failed {
            return Err(format!(
                "the closed laptop ran unlocked on battery on our Do nothing for {lasted} ms, to the end"
            ));
        }
    }
    Ok(())
}

/// No `ApplyUpdate` while a hold counts, a hold file names a live process, or a key is owned (decision 42). A Keep
/// awake file always names this tray, which quits for the update (`p2-core-lid` design choice 15).
fn no_update_while_anything_holds(world: &World) -> Result<(), String> {
    for seen in replay(world) {
        let Some(step) = seen.step else {
            continue;
        };
        if !seen.sent(&Command::ApplyUpdate) {
            continue;
        }
        let truth = seen.truth();
        if seen.after.holding(step.at) {
            return Err(format!(
                "ApplyUpdate while a hold counts, {}",
                seen.where_()
            ));
        }
        if truth.live_file {
            return Err(format!(
                "ApplyUpdate while a hold file names a live process, {}",
                seen.where_()
            ));
        }
        if truth.owned {
            return Err(format!(
                "ApplyUpdate while Windows holds a value of ours or an apply is on its way, {}",
                seen.where_()
            ));
        }
    }
    Ok(())
}

// The strategies.

fn plan_index() -> impl Strategy<Value = usize> {
    0..PLANS.len()
}

/// A value the user sets: never the target 0, since a user who sets exactly our value is not seen (section 5,
/// "Ownership", accepted).
fn user_sets() -> impl Strategy<Value = WorldStep> {
    let lid = (
        prop_oneof![Just(SettingKey::LidAc), Just(SettingKey::LidDc)],
        prop_oneof![Just(1u32), Just(2), Just(3)],
    );
    let sleep = (
        Just(SettingKey::SleepDc),
        prop_oneof![Just(300u32), Just(900), Just(3_600)],
    );
    // Mostly on the active plan, as Settings writes; sometimes on another one, as `powercfg` can.
    let plan = prop_oneof![3 => Just(None), 1 => plan_index().prop_map(Some)];
    (plan, prop_oneof![2 => lid, 1 => sleep]).prop_map(|(plan, (key, value))| WorldStep::UserSets {
        plan,
        key,
        value,
    })
}

fn agent() -> impl Strategy<Value = WorldStep> {
    // Mostly working, so holds last long enough for reads, user changes, lid closes and plan switches inside them.
    let now = prop_oneof![
        8 => Just(AgentNow::Working),
        2 => Just(AgentNow::Idle),
        1 => Just(AgentNow::Gone),
        1 => Just(AgentNow::Leaves),
    ];
    (1u32..=2, now).prop_map(|(pid, now)| WorldStep::Agent { pid, now })
}

fn run_file() -> impl Strategy<Value = WorldStep> {
    let now = prop_oneof![
        2 => Just(FileNow::Alive),
        1 => Just(FileNow::Gone),
        1 => Just(FileNow::Removed),
    ];
    (0u8..2, now).prop_map(|(n, now)| WorldStep::RunFile { n, now })
}

fn menu() -> impl Strategy<Value = WorldStep> {
    prop_oneof![
        1 => Just(MenuCommand::KeepAwake(KeepAwake::Minutes(30))),
        3 => Just(MenuCommand::KeepAwake(KeepAwake::UntilOff)),
        1 => Just(MenuCommand::KeepAwake(KeepAwake::Off)),
        2 => any::<bool>().prop_map(MenuCommand::KeepScreenOn),
        1 => any::<bool>().prop_map(MenuCommand::Automatic),
        2 => Just(MenuCommand::Restore),
    ]
    .prop_map(WorldStep::Menu)
}

fn run() -> impl Strategy<Value = WorldStep> {
    prop_oneof![
        10 => 0..40 * SECOND,
        4 => 40 * SECOND..6 * MINUTE,
        1 => 6 * MINUTE..12 * MINUTE,
    ]
    .prop_map(WorldStep::Run)
}

/// A failed call: code 2 (a plan call or a file that fails), or 5 (access denied: requests only for the run when an
/// apply meets it). Restores and reads make up to about 15 calls, an apply that restores about 30.
fn fault() -> impl Strategy<Value = WorldStep> {
    let on = prop_oneof![
        1 => Just(FaultOn::AnyCommand),
        1 => Just(FaultOn::ReadPlan),
        1 => Just(FaultOn::Apply),
        3 => Just(FaultOn::Restore),
    ];
    // The first calls load the journal and read the active plan: a fault there fails a routine before it acts.
    let at = prop_oneof![2 => 0usize..3, 3 => 0usize..16];
    (on, at, prop_oneof![6 => Just(2u32), 1 => Just(5)])
        .prop_map(|(on, at, code)| WorldStep::Fault { on, at, code })
}

fn battery() -> impl Strategy<Value = u8> {
    // Under the floor (20, or 30 on the docked plan) only now and then: a step back lasts until AC.
    prop_oneof![1 => Just(10u8), 1 => Just(25), 3 => Just(50), 3 => Just(80)]
}

fn world_step() -> impl Strategy<Value = WorldStep> {
    prop_oneof![
        12 => agent(),
        4 => run_file(),
        7 => menu(),
        8 => any::<bool>().prop_map(|closed| WorldStep::Lid { closed }),
        6 => any::<bool>().prop_map(WorldStep::Ac),
        4 => battery().prop_map(WorldStep::Battery),
        3 => Just(WorldStep::Input),
        14 => run(),
        2 => (MINUTE..3 * HOUR).prop_map(WorldStep::Suspend),
        1 => (-2 * 3_600_000i64..2 * 3_600_000).prop_map(WorldStep::WallJump),
        2 => Just(WorldStep::Lock),
        3 => Just(WorldStep::Unlock),
        // Leaving and ending last until undone, so the undoing comes more often.
        1 => Just(WorldStep::ConsoleLeft),
        2 => Just(WorldStep::ConsoleBack),
        1 => Just(WorldStep::Ending),
        2 => Just(WorldStep::EndCancelled),
        1 => Just(WorldStep::TakeOwner),
        10 => user_sets(),
        4 => plan_index().prop_map(WorldStep::Switch),
        4 => fault(),
        1 => Just(WorldStep::LockFails),
        1 => any::<bool>().prop_map(|allowed| WorldStep::Hibernation { allowed }),
        1 => any::<bool>().prop_map(|ok| WorldStep::UpdateReady { ok }),
        3 => Just(WorldStep::Io),
        3 => Just(WorldStep::IoLate),
        3 => Just(WorldStep::Deliver),
        3 => Just(WorldStep::Flush),
    ]
}

fn world_start() -> impl Strategy<Value = WorldStart> {
    (
        plan_index(),
        prop::bool::weighted(0.9),
        prop::bool::weighted(0.85),
        prop::bool::weighted(0.9),
        prop::bool::weighted(0.1),
        any::<bool>(),
        any::<bool>(),
        battery(),
        prop::bool::weighted(0.2),
        any::<bool>(),
        prop::bool::weighted(0.65),
    )
        .prop_map(
            |(
                active,
                automatic,
                owner,
                on_console,
                locked,
                lid_closed,
                ac,
                battery_pct,
                leftover,
                startup_fails,
                keep_up,
            )| WorldStart {
                active,
                automatic,
                owner,
                on_console,
                locked,
                lid_closed,
                ac,
                battery_pct,
                leftover,
                startup_fails,
                keep_up,
            },
        )
}

fn end() -> impl Strategy<Value = End> {
    prop_oneof![
        2 => Just(End::LastHold),
        1 => Just(End::Quit),
        1 => Just(End::SessionEnd),
    ]
}

fn case() -> impl Strategy<Value = Case> {
    (world_start(), vec(world_step(), 0..80), end()).prop_map(|(start, steps, end)| Case {
        start,
        steps,
        end,
    })
}

/// The same steps, weighted for the lid rules: a hold that lasts, the lid, the power source, signing in on an external
/// screen, docked use and its plan, and runs past the 30 s settle. The general mix meets the chain behind an
/// AC to battery lock (holding, our Do nothing in force, lid closed, signed in, unplugged, 30 s) about once in 25 cases.
fn lid_step() -> impl Strategy<Value = WorldStep> {
    prop_oneof![
        5 => Just(WorldStep::Agent { pid: 1, now: AgentNow::Working }),
        1 => prop_oneof![Just(AgentNow::Idle), Just(AgentNow::Leaves)]
            .prop_map(|now| WorldStep::Agent { pid: 1, now }),
        1 => Just(WorldStep::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff))),
        6 => any::<bool>().prop_map(|closed| WorldStep::Lid { closed }),
        6 => any::<bool>().prop_map(WorldStep::Ac),
        4 => Just(WorldStep::Unlock),
        1 => Just(WorldStep::Lock),
        6 => prop_oneof![4 => 0..45 * SECOND, 1 => 45 * SECOND..3 * MINUTE].prop_map(WorldStep::Run),
        2 => plan_index().prop_map(WorldStep::Switch),
        2 => user_sets(),
        1 => battery().prop_map(WorldStep::Battery),
        1 => Just(WorldStep::Input),
        1 => fault(),
        1 => Just(WorldStep::LockFails),
        1 => Just(WorldStep::IoLate),
        1 => Just(WorldStep::Flush),
    ]
}

/// The same steps, weighted for answers that come late: short holds (each agent turn is an episode) that start and end
/// while the IO thread lags, so applies and restores are still on their way at the next edge.
fn late_step() -> impl Strategy<Value = WorldStep> {
    prop_oneof![
        6 => (1u32..=2, prop_oneof![2 => Just(AgentNow::Working), 1 => Just(AgentNow::Idle)])
            .prop_map(|(pid, now)| WorldStep::Agent { pid, now }),
        2 => prop_oneof![
            Just(MenuCommand::KeepAwake(KeepAwake::UntilOff)),
            Just(MenuCommand::KeepAwake(KeepAwake::Off)),
            Just(MenuCommand::Restore),
        ]
        .prop_map(WorldStep::Menu),
        4 => Just(WorldStep::Io),
        4 => Just(WorldStep::IoLate),
        3 => Just(WorldStep::Deliver),
        1 => Just(WorldStep::Flush),
        3 => (0..25 * SECOND).prop_map(WorldStep::Run),
        3 => user_sets(),
        2 => plan_index().prop_map(WorldStep::Switch),
        2 => fault(),
        1 => any::<bool>().prop_map(|closed| WorldStep::Lid { closed }),
        1 => any::<bool>().prop_map(WorldStep::Ac),
        1 => prop_oneof![Just(WorldStep::ConsoleLeft), Just(WorldStep::ConsoleBack)],
        1 => any::<bool>().prop_map(|ok| WorldStep::UpdateReady { ok }),
    ]
}

fn late_case() -> impl Strategy<Value = Case> {
    let start = plan_index().prop_map(|active| WorldStart {
        active,
        keep_up: false,
        ..WorldStart::default()
    });
    (start, vec(late_step(), 0..60), end()).prop_map(|(start, steps, end)| Case {
        start,
        steps,
        end,
    })
}

fn lid_case() -> impl Strategy<Value = Case> {
    let start = (plan_index(), prop::bool::weighted(0.85), any::<bool>()).prop_map(
        |(active, keep_up, ac)| WorldStart {
            active,
            keep_up,
            ac,
            ..WorldStart::default()
        },
    );
    (start, vec(lid_step(), 0..60), end()).prop_map(|(start, steps, end)| Case {
        start,
        steps,
        end,
    })
}

/// How often each property's case comes up in 256 cases of the three mixes, for whoever changes a mix: a property
/// whose case is rare passes without testing much. Run it with `-- --ignored --nocapture coverage_tally`.
#[test]
#[ignore = "a coverage report, not a check"]
fn coverage_tally() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::default();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut bump = |k: &str, n: usize| *counts.entry(k.to_owned()).or_default() += n;
    let mixes = prop_oneof![case(), lid_case(), late_case()];
    for _ in 0..256 {
        let case = mixes.new_tree(&mut runner).unwrap().current();
        bump("cases", 1);
        bump("world steps", case.steps.len());
        let world = play(&case);
        bump("log entries", world.log().len());
        bump("core steps", world.sim().steps().len());
        for (_, c) in world.sim().commands() {
            let name = match c {
                Command::Apply { .. } => "Apply",
                Command::Restore { .. } => "Restore",
                Command::ReadPlan => "ReadPlan",
                Command::Lock => "Lock",
                Command::Hibernate { .. } => "Hibernate",
                Command::ApplyUpdate => "ApplyUpdate",
                Command::Notify(Notice::UserChanged(_)) => "Notify UserChanged",
                Command::Notify(Notice::RestoreFailed) => "Notify RestoreFailed",
                Command::Notify(Notice::StepBack { .. }) => "Notify StepBack",
                Command::Play(Cue::LidClosedHolding) => "Play chime",
                Command::Requests { system: true, .. } => "Requests on",
                _ => continue,
            };
            bump(name, 1);
        }
        for step in world.sim().steps() {
            let name = match &step.event {
                Event::Done(Outcome::Applied { result: Ok(()), .. }) => "Applied ok",
                Event::Done(Outcome::Applied { result: Err(_), .. }) => "Applied err",
                Event::Done(Outcome::Restored { result: Ok(()), .. }) => "Restored ok",
                Event::Done(Outcome::Restored { result: Err(_), .. }) => "Restored err",
                Event::UserChanged(_) => "UserChanged",
                Event::Plan(Err(_)) => "Plan err",
                _ => continue,
            };
            bump(name, 1);
        }
        let seen = replay(&world);
        let mut before: Option<&Truth> = None;
        for s in &seen {
            let truth = s.truth();
            if let Some(step) = s.step {
                let change = match step.event {
                    Event::Lid { closed: true } => before.is_some_and(|b| !b.lid_closed),
                    Event::Power { ac: false, .. } => before.is_some_and(|b| b.ac),
                    _ => false,
                };
                if change && truth.in_sync && runs_closed_unlocked_on_ours(truth) {
                    bump("P10 triggers", 1);
                }
                if step.event == Event::Tick && s.after.applies() && s.after.plan.is_some() {
                    bump("P3 ticks while applying", 1);
                    if s.commands()
                        .iter()
                        .any(|c| matches!(c, Command::Restore { .. }))
                    {
                        bump("P3 retries while applying", 1);
                    }
                }
                if let Event::UserChanged(_) = step.event
                    && s.before.episode.is_some()
                {
                    bump("P2 releases in an episode", 1);
                }
                if s.after
                    .episode
                    .as_ref()
                    .is_some_and(|e| !e.released.is_empty())
                    && s.commands().iter().any(|c| writes(c).is_some())
                {
                    bump("P2 applies after a release", 1);
                }
                if s.after.session_ending && s.after.episode.is_some() {
                    bump("P4 steps ending while holding", 1);
                }
                if !s.after.on_console && s.after.episode.is_some() {
                    bump("P5 steps off console while holding", 1);
                }
                if s.after.stepped_back && s.after.episode.is_some() {
                    bump("P6 steps stepped back while holding", 1);
                }
                if s.sent(&Command::Lock)
                    && s.sent(&Command::Play(Cue::LidClosedHolding))
                    && truth.in_sync
                {
                    bump("P8 lid locks in sync", 1);
                }
                if matches!(step.event, Event::Lid { closed: true })
                    && truth.in_sync
                    && truth.lid().0 == 0
                    && truth.lid().1 == SetBy::User
                {
                    bump("P8 lid closes on the user's own Do nothing", 1);
                }
            }
            before = Some(truth);
        }
        if world.quit() {
            bump("quit", 1);
        }
    }
    println!("{counts:#?}");
}

proptest! {
    // A regression file would be a file outside this card; a shrunk case that finds a bug becomes a named test.
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn owned_keys_come_back_after_the_last_hold_quit_and_session_end(
        case in prop_oneof![case(), late_case()]
    ) {
        let world = play(&case);
        verdict(&world, restored_at_the_end(&world))?;
    }

    #[test]
    fn a_released_key_is_never_written_again_in_its_episode(case in case()) {
        let world = play(&case);
        verdict(&world, released_keys_stay_released(&world))?;
    }

    #[test]
    fn a_restore_retry_never_takes_an_entry_the_running_hold_uses(case in case()) {
        let world = play(&case);
        verdict(&world, retries_spare_the_running_hold(&world))?;
    }

    #[test]
    fn no_apply_from_session_end_until_it_is_cancelled(case in case()) {
        let world = play(&case);
        verdict(&world, no_apply_while_the_session_ends(&world))?;
    }

    #[test]
    fn no_apply_and_no_display_request_while_another_session_has_the_console(case in case()) {
        let world = play(&case);
        verdict(&world, nothing_for_another_sessions_console(&world))?;
    }

    #[test]
    fn no_request_with_no_hold_and_none_while_stepped_back(case in case()) {
        let world = play(&case);
        verdict(&world, no_request_without_a_hold(&world))?;
    }

    #[test]
    fn no_hibernate_within_5_minutes_of_a_resume_or_of_input(case in case()) {
        let world = play(&case);
        verdict(&world, no_hibernate_soon_after_input(&world))?;
    }

    #[test]
    fn no_lock_when_the_lid_action_in_force_is_the_users_own(case in prop_oneof![case(), lid_case()]) {
        let world = play(&case);
        verdict(&world, no_lock_on_the_users_own_do_nothing(&world))?;
    }

    /// Not in the card's list: the converse of the one above.
    #[test]
    fn a_lid_close_on_our_do_nothing_locks_and_chimes_at_once(case in prop_oneof![case(), lid_case()]) {
        let world = play(&case);
        verdict(&world, a_lid_close_on_ours_locks(&world))?;
    }

    #[test]
    fn every_lock_step_back_and_user_change_is_explained(case in case()) {
        let world = play(&case);
        verdict(&world, every_action_is_explained(&world))?;
    }

    #[test]
    fn a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s(
        case in prop_oneof![case(), lid_case()]
    ) {
        let world = play(&case);
        verdict(&world, closed_laptop_is_locked_in_time(&world, false))?;
    }

    /// The strict form: a lid close before the core's first plan read, with a crashed tray's Do nothing in force.
    #[test]
    fn a_closed_laptop_is_locked_also_before_the_first_plan_read(
        case in prop_oneof![case(), lid_case()]
    ) {
        let world = play(&case);
        verdict(&world, closed_laptop_is_locked_in_time(&world, true))?;
    }

    #[test]
    fn no_apply_update_while_a_hold_counts_a_hold_file_lives_or_a_key_is_owned(
        case in prop_oneof![case(), late_case()]
    ) {
        let world = play(&case);
        verdict(&world, no_update_while_anything_holds(&world))?;
    }

    /// `handle` never panics, and `Sim` finds no deadline behind a `Tick` (it panics with the trace if one does).
    #[test]
    fn handle_never_panics_and_next_deadline_is_never_behind_a_tick(case in case()) {
        play(&case);
    }

    /// The strict, Windows-side form of the release rule.
    #[test]
    fn a_released_key_is_never_ours_in_force_again_in_its_episode(case in case()) {
        let world = play(&case);
        verdict(&world, released_keys_are_not_ours_in_force(&world))?;
    }
}
