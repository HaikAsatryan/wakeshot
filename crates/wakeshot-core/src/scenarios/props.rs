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
use crate::ownership::fake::{FakeJournal, SetBy};
use crate::policy::ending::IDLE_MS;
use crate::policy::lid::{DO_NOTHING, SOURCE_SETTLE_MS};
use crate::policy::{floor, hold_files};
use crate::process::Alive;
use crate::settings::{Guid, JournalEntry, PlanValues, SettingKey};
use crate::sim::{
    self, AgentNow, End, Entry, FaultOn, FileNow, KEYS, PLANS, Truth, World, WorldStart, WorldStep,
};
use crate::sources::Activity;
use crate::time::Now;
use crate::view::{Cue, KeepAwake, MenuCommand, Notice};

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

/// The access denied codes the design names (section 8): requests only for the whole run, from a settings access
/// check or write; from any other call, for the episode (power safety review finding 8).
const ACCESS_DENIED: [u32; 2] = [5, 1260];
const SETTINGS_CALLS: [&str; 3] = [
    "PowerSettingAccessCheckEx",
    "PowerWriteACValueIndex",
    "PowerWriteDCValueIndex",
];

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
    /// Access denied at an apply's settings call: requests only for the run.
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
    /// The journal mirror's keys (section 3): what `Started`, `Done(Applied)` and `Done(Restored)` carry, less the keys
    /// `UserChanged` names. An apply or restore that failed with none read nothing and leaves it.
    mirror: BTreeSet<(Guid, SettingKey)>,
    /// A restore is still wanted (section 5, "A failed `Restore`"): `Started` brought a journal or one that did not
    /// read, or a restore failed. A restore of every plan that worked ends it; one of one plan leaves it, the side
    /// that excuses more. A reset that found the journal lost ends it too.
    retry: bool,
    /// A `ResetLost` went out and its answer has not come (section 3, "A journal that cannot be read").
    resetting: bool,
    /// A lid reading came, so the next one that says closed is a close.
    lid_known: bool,
    /// The lid closed while the journal was unknown and the session unlocked, and no lid opening or lock came since:
    /// the lock that close did not get is still owed once Wakeshot keeps the laptop running
    /// (`p2-6-sleep-route-fixes`, review major 1).
    window_close: bool,
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

    /// The core writes settings now (section 3, "Holding starts" and "A failed `Restore`"). Only the hold keys count:
    /// a short key the user changed in a stepped-back episode never stops the hold (`p2-6-sleep-route-fixes` minor 4).
    fn applies(&self) -> bool {
        self.episode
            .as_ref()
            .is_some_and(|e| !e.requests_only && !KEYS.iter().all(|key| e.released.contains(key)))
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

    /// The mirror may not know `journal.json` (section 3, "A journal that cannot be read"): a restore is still wanted
    /// and the mirror names nothing, or a reset is out. Then Wakeshot neither locks at a lid close nor counts down,
    /// and a failed restore gets no `RestoreFailed`: the window ends in a read or the reset (product rule 7).
    fn journal_unknown(&self) -> bool {
        self.resetting || (self.retry && self.mirror.is_empty())
    }

    /// The mirror and the restore retry through one answer or user change.
    fn take_journal(&mut self, event: &Event) {
        let keys = |journal: &[JournalEntry]| journal.iter().map(|e| (e.plan, e.key)).collect();
        match event {
            Event::Done(Outcome::Applied {
                result, journal, ..
            }) => {
                let read_nothing =
                    matches!(result, Err(ApplyError::SafetyNet(_) | ApplyError::Write(_)))
                        && journal.is_empty();
                if !read_nothing {
                    self.mirror = keys(journal);
                }
            }
            Event::Done(Outcome::Restored {
                plan,
                result,
                journal,
                ..
            }) => {
                if result.is_ok() || !journal.is_empty() {
                    self.mirror = keys(journal);
                }
                match result {
                    Ok(()) if plan.is_none() => self.retry = false,
                    Ok(()) => {}
                    Err(_) => self.retry = true,
                }
            }
            Event::UserChanged(changed) => {
                self.mirror.remove(&(changed.plan, changed.key));
            }
            // Entries: the file read after all, and the retry goes on; none: known and empty.
            Event::Done(Outcome::ResetLost { journal, .. }) => {
                self.resetting = false;
                self.mirror = keys(journal);
                if journal.is_empty() {
                    self.retry = false;
                }
            }
            _ => {}
        }
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
                journal,
                journal_unread,
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
                self.mirror = journal
                    .iter()
                    .flat_map(|j| &j.entries)
                    .map(|e| (e.plan, e.key))
                    .collect();
                self.retry = !self.mirror.is_empty() || *journal_unread;
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
                if ACCESS_DENIED.contains(&failure.code) && SETTINGS_CALLS.contains(&failure.call) {
                    self.requests_only_run = true;
                } else if let Some(episode) = self.episode.as_mut() {
                    episode.requests_only = true;
                }
            }
            Event::Session(change) => match change {
                SessionChange::Locked => {
                    self.locked = true;
                    self.window_close = false;
                }
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
            Event::Lid { closed } => {
                let close = *closed && self.lid_known && !self.lid_closed;
                self.window_close = match *closed {
                    true => self.window_close || (close && self.journal_unknown() && !self.locked),
                    false => false,
                };
                self.lid_closed = *closed;
                self.lid_known = true;
            }
            _ => {}
        }
        self.take_journal(&step.event);
        if step
            .commands
            .iter()
            .any(|c| matches!(c, Command::ResetLost { .. }))
        {
            self.resetting = true;
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

/// The sleep route's apply: short keys only, each at 1 minute.
fn short_writes(writes: &[(SettingKey, u32)]) -> bool {
    writes
        .iter()
        .all(|&(k, v)| SettingKey::SHORT.contains(&k) && v == crate::settings::SHORT_TIMEOUT_S)
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
/// its retry. Only a restore of every plan that works ends that wait: a scoped one that works (a plan switch's, or
/// the retry's while holding) leaves the other plans' entries to the retry (section 3, "A failed `Restore`").
fn released_keys_are_not_ours_in_force(world: &World) -> Result<(), String> {
    let mut restore_failed = false;
    for seen in replay(world) {
        if let Some(step) = seen.step {
            match step.event {
                Event::Done(Outcome::Restored { result: Err(_), .. }) => restore_failed = true,
                Event::Done(Outcome::Restored {
                    plan: None,
                    result: Ok(()),
                    ..
                }) => restore_failed = false,
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

/// The model steps back after a failed lid lock in this step, and the core surely does too: the lid is closed with
/// our Do nothing in force for the current power source where [`core_can_know`] (the model's step back also stands
/// where the core's own view finds no Do nothing of ours, and then the core holds on).
fn surely_steps_back_for_the_lock(seen: &Seen<'_>) -> bool {
    let truth = seen.truth();
    !seen.before.lock_stepped_back
        && seen.after.lock_stepped_back
        && truth.lid_closed
        && truth.lid() == (DO_NOTHING, SetBy::Override)
        && core_can_know(seen)
}

/// No request is on with no hold, and none while stepped back, at the battery floor or after a failed lid lock
/// (section 8, "Lock fails"); an apply only while a hold counts.
fn no_request_without_a_hold(world: &World) -> Result<(), String> {
    let mut last = (false, false);
    let mut lock_back = false;
    for seen in replay(world) {
        let (system, display) = requests_now(&mut last, &seen);
        let Some(step) = seen.step else {
            continue;
        };
        if seen.after.quit {
            continue;
        }
        lock_back =
            seen.after.lock_stepped_back && (lock_back || surely_steps_back_for_the_lock(&seen));
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
        if system && lock_back {
            return Err(format!(
                "the system request is on while stepped back after a failed lid lock, {}",
                seen.where_()
            ));
        }
        if display && !system {
            return Err(format!(
                "the display request is on without the system one, {}",
                seen.where_()
            ));
        }
        // With no hold only the sleep route writes: its short timeouts, with the lid closed (`p2-6-sleep-route`).
        let stray = seen
            .commands()
            .iter()
            .filter_map(writes)
            .any(|w| !short_writes(w) || !seen.after.lid_closed);
        if seen.after.episode.is_none() && stray {
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
/// current power source is the user's own Do nothing (decision 18). Judged only where [`core_can_know`]. Since
/// `p2-6-record-lost` no guess locks while the journal is unknown, so that window needs no excuse.
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
/// close before the first plan read, which the strict form of [`closed_laptop_is_locked_in_time`] judges; and not while
/// the journal is unknown, when Wakeshot does not lock by design (product "User settings safety" 7).
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
        if !closes || !core_can_know(s) || s.before.quit || s.before.journal_unknown() {
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
        match &step.event {
            // While the journal is unknown its window ends in a read or the reset, each with its own word.
            Event::Done(Outcome::Restored { result: Err(_), .. })
                if seen.after.journal_unknown() =>
            {
                if sent {
                    return Err(format!(
                        "a RestoreFailed while the journal is unknown, {}",
                        seen.where_()
                    ));
                }
            }
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
            // A lost journal's reset: one notice when it reset a value or a call failed, the run of failures over.
            Event::Done(Outcome::ResetLost {
                reset,
                result,
                journal,
            }) if journal.is_empty() => {
                sent_since_ok = false;
                warned = false;
                if result.is_err() || !reset.is_empty() {
                    expect(Notice::JournalLost {
                        reset: reset.clone(),
                        failed: result.is_err(),
                    })?;
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
/// core heard where [`core_can_know`]. When the lock and its retry fail, the laptop runs on until the step back's
/// restore lands, so a stretch is excused only by that step back done in time: the requests off and a `Restore` in the
/// step that ends the retry's wait, one tick after its failure (section 8, "Lock fails"; verify minor 4). `strict`
/// also judges a change before the core's first plan read (finding 2: a crashed tray's values in the first moments
/// after the start). A change while the journal is unknown starts no stretch: Wakeshot does not lock then, by design
/// (product "User settings safety" 7, whose cost names a closed laptop running unlocked until it sleeps). The lock
/// that close did not get is owed once the journal is known: any later step that finds the laptop so starts a stretch,
/// such as a hold whose apply lands after the window (`p2-6-sleep-route-fixes`, review major 1).
fn closed_laptop_is_locked_in_time(world: &World, strict: bool) -> Result<(), String> {
    let seen = replay(world);
    let mut stretch: Option<u64> = None;
    let mut stepped_back = false;
    let mut last = (false, false);
    let mut before: Option<&Truth> = None;
    for s in &seen {
        let truth = s.truth();
        let (system, _) = requests_now(&mut last, s);
        let runs = runs_closed_unlocked_on_ours(truth);
        if let Some(since) = stretch
            && !runs
        {
            let lasted = truth.at.mono_ms.saturating_sub(since);
            if lasted > SOURCE_SETTLE_MS && !stepped_back {
                return Err(format!(
                    "the closed laptop ran unlocked on battery on our Do nothing for {lasted} ms, until {}",
                    s.where_()
                ));
            }
            stretch = None;
            stepped_back = false;
        }
        if let Some(step) = s.step {
            let restores = step
                .commands
                .iter()
                .any(|c| matches!(c, Command::Restore { .. }));
            let steps_back = !s.before.lock_stepped_back && s.after.lock_stepped_back;
            if stretch.is_some() && steps_back && restores && !system {
                stepped_back = true;
            }
            let change = match step.event {
                Event::Lid { closed: true } => before.is_some_and(|b| !b.lid_closed),
                Event::Power { ac: false, .. } => before.is_some_and(|b| b.ac),
                _ => false,
            };
            let knows = core_can_know(s) || (strict && truth.in_sync && s.before.plan.is_none());
            let window = s.before.journal_unknown() || s.after.journal_unknown();
            let owed = change || s.after.window_close;
            if stretch.is_none() && runs && owed && knows && !window {
                stretch = Some(truth.at.mono_ms);
            }
        }
        before = Some(truth);
    }
    if let Some(since) = stretch {
        let lasted = world.now().mono_ms.saturating_sub(since);
        if lasted > SOURCE_SETTLE_MS && !stepped_back {
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

/// A failed call: code 2 (a plan call or a file that fails), or 5 (access denied; the fake's calls are no settings
/// calls by name, so an apply that meets it holds requests only for the episode). Restores and reads make up to about
/// 15 calls, an apply that restores about 30.
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
        prop::bool::weighted(0.2),
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
                startup_unread,
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
                startup_unread,
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

// Shrunk cases kept as named tests.

/// A case from its start, steps and end, checked by `property`, with the trace on failure.
fn check_case(
    start: WorldStart,
    steps: Vec<WorldStep>,
    end: End,
    property: fn(&World) -> Result<(), String>,
) {
    let case = Case { start, steps, end };
    let world = play(&case);
    if let Err(why) = property(&world) {
        panic!("{why}\n\ncase:\n{case:?}\n\ntrace:\n{}", world.trace());
    }
}

fn restore_fails() -> WorldStep {
    WorldStep::Fault {
        on: FaultOn::Restore,
        at: 0,
        code: 2,
    }
}

fn on_battery_at_25() -> WorldStart {
    WorldStart {
        ac: false,
        battery_pct: 25,
        ..WorldStart::default()
    }
}

/// `p2-core-lid` round A, Needs another card 2: a restore of one plan that works while a failed restore still waits
/// for its retry is no release breach; the retry, which takes every plan, settles it. This case and the next broke the
/// model's "restore failed" flag, which a good restore of another plan cleared.
#[test]
fn a_released_key_waits_for_the_retry_after_a_failed_restore() {
    check_case(
        on_battery_at_25(),
        vec![
            WorldStep::Switch(1),
            WorldStep::Ac(true),
            restore_fails(),
            WorldStep::RunFile {
                n: 0,
                now: FileNow::Alive,
            },
            WorldStep::Switch(2),
            restore_fails(),
            WorldStep::Menu(MenuCommand::Restore),
            WorldStep::Switch(1),
        ],
        End::LastHold,
        released_keys_are_not_ours_in_force,
    );
}

#[test]
fn a_released_key_waits_for_the_retry_after_a_failed_plan_switch_restore() {
    check_case(
        WorldStart {
            automatic: false,
            keep_up: false,
            ..on_battery_at_25()
        },
        vec![
            WorldStep::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))),
            WorldStep::Agent {
                pid: 1,
                now: AgentNow::Working,
            },
            WorldStep::Flush,
            WorldStep::Switch(1),
            restore_fails(),
            WorldStep::Run(10 * SECOND),
            WorldStep::Io,
            WorldStep::Menu(MenuCommand::Restore),
            WorldStep::Fault {
                on: FaultOn::Apply,
                at: 2,
                code: 2,
            },
            WorldStep::Switch(0),
            WorldStep::Flush,
        ],
        End::LastHold,
        released_keys_are_not_ours_in_force,
    );
}

/// `p2-6-props-flake`, the docs pass failure (seed 1 at 4096 cases): the floor countdown's short timeouts are on the
/// docked plan, and the user sets battery sleep there and switches to Balanced. The switch's read posts `Plan` before
/// its check's `UserChanged`, so the switch's short apply goes out with battery sleep, and the release must take it
/// back.
#[test]
fn a_short_key_released_while_the_switchs_short_apply_is_on_its_way_is_taken_back() {
    check_case(
        WorldStart {
            automatic: false,
            owner: false,
            on_console: false,
            ..on_battery_at_25()
        },
        vec![
            WorldStep::Lid { closed: true },
            WorldStep::ConsoleBack,
            WorldStep::Switch(2),
            WorldStep::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))),
            WorldStep::TakeOwner,
            WorldStep::Run(74_186),
            WorldStep::Run(121_826),
            WorldStep::Run(103_988),
            WorldStep::UserSets {
                plan: None,
                key: SettingKey::SleepDc,
                value: 300,
            },
            WorldStep::Switch(0),
        ],
        End::LastHold,
        released_keys_are_not_ours_in_force,
    );
}

/// `p2-6-props-flake`, seed 22: no plan switch. The user sets battery sleep over the hold's Never before the floor
/// restore ran, so the short apply sent behind it takes the user's value as its original, and the release comes after.
#[test]
fn a_short_key_released_while_the_short_apply_waits_behind_the_floor_restore_is_taken_back() {
    check_case(
        WorldStart {
            keep_up: false,
            ..on_battery_at_25()
        },
        vec![
            WorldStep::Agent {
                pid: 1,
                now: AgentNow::Working,
            },
            WorldStep::Lid { closed: true },
            WorldStep::Flush,
            WorldStep::Run(266_501),
            WorldStep::Battery(10),
            WorldStep::Run(33_499),
            WorldStep::UserSets {
                plan: None,
                key: SettingKey::SleepDc,
                value: 300,
            },
        ],
        End::LastHold,
        released_keys_are_not_ours_in_force,
    );
}

/// `p2-6-props-flake`, seed 29 once the short apply race was fixed: the reset of a journal the core lost runs after
/// the hold's apply wrote entries, and its own load fails, so it writes battery sleep's default over the hold's Never
/// and sets the file aside (product "User settings safety" 7). That default is no longer Wakeshot's to hold.
#[test]
fn a_reset_over_a_running_holds_values_leaves_nothing_of_ours() {
    check_case(
        WorldStart {
            automatic: false,
            startup_unread: true,
            keep_up: false,
            ..on_battery_at_25()
        },
        vec![
            restore_fails(),
            WorldStep::Agent {
                pid: 1,
                now: AgentNow::Working,
            },
            WorldStep::Menu(MenuCommand::Automatic(true)),
            WorldStep::Io,
            WorldStep::Run(6 * MINUTE),
            WorldStep::Menu(MenuCommand::Restore),
            WorldStep::Flush,
        ],
        End::LastHold,
        |world| {
            released_keys_are_not_ours_in_force(world).and_then(|()| restored_at_the_end(world))
        },
    );
}

/// The lid lock and its retry both fail with the lid closed on our Do nothing, on battery, while the IO thread lags:
/// the step back takes the requests off and sends the restore at once, and the laptop runs on until the restore lands
/// (verify minor 4, the props half).
fn lock_step_back_case(property: fn(&World) -> Result<(), String>) {
    check_case(
        WorldStart {
            ac: false,
            keep_up: false,
            ..WorldStart::default()
        },
        vec![
            WorldStep::Agent {
                pid: 1,
                now: AgentNow::Working,
            },
            WorldStep::Flush,
            WorldStep::LockFails,
            WorldStep::Lid { closed: true },
            WorldStep::LockFails,
            WorldStep::Run(45 * SECOND),
        ],
        End::LastHold,
        property,
    );
}

#[test]
fn a_lock_step_back_takes_the_requests_off() {
    lock_step_back_case(no_request_without_a_hold);
}

#[test]
fn a_lock_step_back_restores_within_one_tick() {
    lock_step_back_case(|world| closed_laptop_is_locked_in_time(world, false));
}

/// Power safety review finding 3: a crashed tray left its values and journal, and the start-up read of that journal
/// failed, so the core knows of nothing and only its `Leftover` retry, through the real restore routine, brings the
/// user's values back.
fn unread_leftover() -> WorldStart {
    WorldStart {
        leftover: true,
        startup_unread: true,
        ..WorldStart::default()
    }
}

#[test]
fn an_unread_journal_at_start_is_restored_at_the_first_tick() {
    let mut world = World::new(&unread_leftover());
    assert!(
        !world.plans().state().held().is_empty(),
        "the crashed tray's values are in force at start"
    );
    world.play(&WorldStep::Run(30 * SECOND));
    if let Err(why) = restored_at_the_end(&world) {
        panic!("{why}\n\ntrace:\n{}", world.trace());
    }
}

#[test]
fn an_unread_journal_whose_retry_fails_is_restored_at_a_later_tick() {
    check_case(
        unread_leftover(),
        vec![restore_fails(), WorldStep::Run(30 * SECOND)],
        End::LastHold,
        |world| restored_at_the_end(world).and_then(|()| every_action_is_explained(world)),
    );
}

#[test]
fn an_unread_journal_and_a_hold_before_the_first_tick() {
    check_case(
        unread_leftover(),
        vec![
            WorldStep::Agent {
                pid: 1,
                now: AgentNow::Working,
            },
            WorldStep::Run(45 * SECOND),
        ],
        End::LastHold,
        |world| restored_at_the_end(world).and_then(|()| retries_spare_the_running_hold(world)),
    );
}

// A journal that cannot be read (product "User settings safety" 7, `p2-6-record-lost`): through the real restore and
// reset routines, the values that look like Wakeshot's end at the plan's Windows defaults, with one notice.

/// The world after `start`, with `journal.json` gone bad right after the start, and `ms` of run time.
fn lost_journal(start: &WorldStart, bad: FakeJournal, ms: u64) -> World {
    let mut world = World::new(start);
    world.plans().state().journal = bad;
    world.play(&WorldStep::Run(ms));
    world.end(End::LastHold);
    world
}

/// The `JournalLost` notices the core sent.
fn lost_notices(world: &World) -> Vec<Notice> {
    world
        .sim()
        .notices()
        .into_iter()
        .filter(|n| matches!(n, Notice::JournalLost { .. }))
        .cloned()
        .collect()
}

fn check(world: &World, property: fn(&World) -> Result<(), String>) {
    if let Err(why) = property(world) {
        panic!("{why}\n\ntrace:\n{}", world.trace());
    }
}

#[test]
fn a_crashed_trays_journal_that_never_reads_ends_at_the_defaults() {
    let world = lost_journal(&unread_leftover(), FakeJournal::Unreadable(32), 3 * MINUTE);
    assert_eq!(
        lost_notices(&world),
        [Notice::JournalLost {
            reset: KEYS.to_vec(),
            failed: false
        }],
        "{}",
        world.trace()
    );
    for key in KEYS {
        let state = world.plans().state();
        let default = state.default_of(sim::BALANCED, key);
        assert_eq!(
            state.in_force(sim::BALANCED, key),
            Some(default),
            "{key:?}\n{}",
            world.trace()
        );
    }
    check(&world, restored_at_the_end);
    check(&world, every_action_is_explained);
}

#[test]
fn a_users_own_do_nothing_meets_a_lost_journal_and_is_reset_with_the_notice() {
    // The other plan: the user's own lid action on battery is Do nothing; their AC Hibernate and battery sleep stay.
    let start = WorldStart {
        active: 1,
        ac: false,
        startup_unread: true,
        ..WorldStart::default()
    };
    let world = lost_journal(&start, FakeJournal::Unreadable(32), 3 * MINUTE);
    assert_eq!(
        lost_notices(&world),
        [Notice::JournalLost {
            reset: vec![SettingKey::LidDc],
            failed: false
        }],
        "{}",
        world.trace()
    );
    let lid_dc = world
        .plans()
        .state()
        .default_of(sim::OTHER_PLAN, SettingKey::LidDc);
    assert_eq!(world.user_value(sim::OTHER_PLAN, SettingKey::LidDc), lid_dc);
    assert_eq!(world.user_value(sim::OTHER_PLAN, SettingKey::LidAc), 2);
    check(&world, restored_at_the_end);
    check(&world, every_action_is_explained);
}

#[test]
fn a_damaged_leftover_journal_ends_at_the_defaults() {
    // The first tick's retry sets the damaged file aside; its values are still in force, so the reset takes them.
    let world = lost_journal(&unread_leftover(), FakeJournal::Corrupt, MINUTE);
    assert_eq!(
        lost_notices(&world),
        [Notice::JournalLost {
            reset: KEYS.to_vec(),
            failed: false
        }],
        "{}",
        world.trace()
    );
    assert_eq!(world.plans().state().set_aside, 1);
    check(&world, restored_at_the_end);
    check(&world, every_action_is_explained);
}

// Review major 2 of `p2-6-sleep-route` (`p2-6-sleep-route-fixes`): a journal lost while the sleep route's 1-minute
// timeouts are in force. The orchestrator's call under rule 7, for Haik's review: the reset puts each short key that
// holds exactly 1 minute back to the default too.

/// Every short key, in the order the reset takes them.
const SHORT_IN_RESET_ORDER: [SettingKey; 4] = [
    SettingKey::SleepDc,
    SettingKey::SleepAc,
    SettingKey::VideoIdleAc,
    SettingKey::VideoIdleDc,
];

/// Each short key of Balanced is in force at the plan's Windows default.
fn short_keys_at_the_defaults(world: &World) {
    let state = world.plans().state();
    for key in SettingKey::SHORT {
        let default = state.default_of(sim::BALANCED, key);
        assert_eq!(
            state.in_force(sim::BALANCED, key),
            Some(default),
            "{key:?}\n{}",
            world.trace()
        );
    }
}

#[test]
fn one_minute_timeouts_a_crash_left_end_at_the_defaults() {
    // Case A: the PC lost power while the 1-minute timeouts were in force, and their journal does not read at the next
    // start.
    let start = WorldStart {
        startup_unread: true,
        ..WorldStart::default()
    };
    let mut world = World::new(&start);
    {
        let mut state = world.plans().state();
        for key in SettingKey::SHORT {
            let short = crate::settings::SHORT_TIMEOUT_S;
            state.set_stored(sim::BALANCED, key, short, SetBy::Override);
            state.set_in_force(sim::BALANCED, key, short, SetBy::Override);
        }
        state.journal = FakeJournal::Unreadable(32);
    }
    world.play(&WorldStep::Run(3 * MINUTE));
    world.end(End::LastHold);
    assert_eq!(
        lost_notices(&world),
        [Notice::JournalLost {
            reset: SHORT_IN_RESET_ORDER.to_vec(),
            failed: false
        }],
        "{}",
        world.trace()
    );
    short_keys_at_the_defaults(&world);
    check(&world, restored_at_the_end);
    check(&world, every_action_is_explained);
}

#[test]
fn a_damaged_journal_met_as_a_hold_starts_over_the_one_minute_timeouts_ends_at_the_defaults() {
    // Case B: work ended with the lid closed and the sleep route wrote its 1-minute timeouts; then `journal.json` is
    // damaged, and an agent turn starts. The hold's start puts the short timeouts back, which meets the damaged file
    // and sets it aside; the hold's apply, queued already, journals battery sleep with 1 minute as the user's own. The
    // reset finds that fresh journal: its entries go back, the reset runs again, then the hold applies again.
    let start = WorldStart {
        ac: false,
        ..WorldStart::default()
    };
    let mut world = World::new(&start);
    let agent = |now| WorldStep::Agent { pid: 1, now };
    world.play(&agent(AgentNow::Working));
    world.play(&WorldStep::Lid { closed: true });
    world.play(&agent(AgentNow::Idle));
    world.play(&WorldStep::Run(IDLE_MS + MINUTE));
    let short = crate::settings::SHORT_TIMEOUT_S;
    assert_eq!(
        world
            .plans()
            .state()
            .in_force(sim::BALANCED, SettingKey::SleepAc),
        Some(short),
        "the short timeouts are in force\n{}",
        world.trace()
    );
    world.plans().state().journal = FakeJournal::Corrupt;
    world.play(&agent(AgentNow::Working));
    {
        let state = world.plans().state();
        let sleep_dc = state
            .known_entries()
            .into_iter()
            .find(|e| e.key == SettingKey::SleepDc);
        let default = state.default_of(sim::BALANCED, SettingKey::SleepDc);
        assert_eq!(
            sleep_dc.map(|e| (e.original, e.written)),
            Some((default, 0)),
            "the hold's battery Never journals the default, not 1 minute\n{}",
            world.trace()
        );
        for key in [
            SettingKey::SleepAc,
            SettingKey::VideoIdleAc,
            SettingKey::VideoIdleDc,
        ] {
            let default = state.default_of(sim::BALANCED, key);
            assert_eq!(
                state.in_force(sim::BALANCED, key),
                Some(default),
                "{key:?}\n{}",
                world.trace()
            );
        }
    }
    world.end(End::LastHold);
    // The lid-close lock held the notice for the user's return.
    world.play(&WorldStep::Unlock);
    assert_eq!(
        lost_notices(&world),
        [Notice::JournalLost {
            reset: SHORT_IN_RESET_ORDER.to_vec(),
            failed: false
        }],
        "{}",
        world.trace()
    );
    short_keys_at_the_defaults(&world);
    check(&world, restored_at_the_end);
    check(&world, every_action_is_explained);
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
