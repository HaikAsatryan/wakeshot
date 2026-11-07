//! The dispatcher: the core in the loop, the runner for each command, the tick, end of session and quit.
//!
//! `Dispatcher` implements `LoopHandler`; trait `LoopRunners` holds the loop's runners.
//!
//! Each input (an event, a session change, a timer fire) goes to the core with a fresh `Now`, and each command to its
//! runner in the core's order: the loop's own through `LoopRunners`, the rest onto the IO thread's queue. A loop
//! runner's answer goes back into the core at once, before the next command runs. The commands that answer gives run
//! after the rest of the batch, so every runner still gets the core's commands in the order the core gave them: a
//! failed request's idle icon comes after the batch's holding icon, never before it.
//!
//! A runner may pump messages (a sent message gets through while it waits on another thread), so the window can call
//! in again before the outer call returns. Every entry point takes `&self` over cells, so such a call is sound: it
//! queues its input and returns, and the outer call feeds the queue, in order, after its own batch. So the core is
//! never called while a call into it is on the stack. The one exception is the end of the session (`on_session`).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use wakeshot_core::command::{Command, CountdownReason, Failure, HibernateEnd, Outcome};
use wakeshot_core::core::Core;
use wakeshot_core::event::{Event, SessionChange};
use wakeshot_core::time::{Clock, Deadline, Now};
use wakeshot_core::view::{Cue, TrayView};

use super::{Ack, Flow, LoopHandler};
use crate::WinError;
use crate::io_queue::IoQueue;

/// The safety tick (architecture section 3, "Clocks"), in run time.
const SAFETY_TICK_MS: u32 = 30_000;

/// `USER_TIMER_MINIMUM`: Windows raises a shorter timer to this.
const TIMER_MIN_MS: u32 = 10;

/// How long the end of the session waits for the restore (architecture section 4, "Tray threads").
const END_WAIT_MS: u32 = 4_000;

/// The loop thread's runners: power requests, the lock, the tray icon, sounds, the hibernate and update threads, input
/// idle, the settings mutex and the watch thread's tick.
///
/// `tray_mode` implements it over the real or dry-run objects. Every call comes on the loop thread, one at a time. A
/// runner may pump messages: a call into the dispatcher from inside one is queued. A call that answers at once returns
/// its result, and the dispatcher feeds it to the core; the others answer later through the sink, as `Done(Sound)`,
/// `Done(Hibernate)` and `Done(Update)`.
pub trait LoopRunners {
    /// `Command::Requests`: `system` is SYSTEM plus EXECUTION, `display` the screen request.
    fn requests(&mut self, system: bool, display: bool) -> Result<(), WinError>;

    /// `Command::Retake`: the same requests taken again after a resume.
    fn retake(&mut self) -> Result<(), WinError>;

    /// `Command::Lock`.
    fn lock(&mut self) -> Result<(), WinError>;

    /// `Command::Tray`.
    fn show_tray(&mut self, view: &TrayView);

    /// `Command::Play`: starts the sound and returns.
    fn play(&mut self, cue: Cue);

    /// `Command::Hibernate`: starts the hibernate thread and returns; the thread posts `Done(Hibernate)` once. Never
    /// called again before that event is fed. `Err`: no thread started, so nothing will post; the dispatcher feeds
    /// `Done(Hibernate(Failed))` at once.
    fn start_hibernate(
        &mut self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) -> Result<(), WinError>;

    /// `Command::SleepCheck`: starts the hibernate thread for the sleep check and returns; the thread posts
    /// `Done(SleepCheck)` once. One thread at a time with `start_hibernate`. `Err`: no thread started; the dispatcher
    /// feeds `Done(SleepCheck(Ok))` at once.
    fn start_sleep_check(
        &mut self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) -> Result<(), WinError>;

    /// `Command::ApplyUpdate`: hands the update to the update thread.
    fn apply_update(&mut self);

    /// Input idle for `Event::InputIdle`, read at every timer fire. A failed read gives 0, which counts as input (no
    /// hibernate), and is the runner's to log.
    fn input_idle_ms(&mut self) -> u64;

    /// Tries the settings mutex, at each safety tick while this tray does not own it; `true` once it does.
    fn try_settings_owner(&mut self) -> bool;

    /// At each safety tick: fresh readings of the system events whose post failed since the last call, so a lost
    /// `Lid` or `Power` does not stay lost (the tick reads neither state itself). Empty when no post failed.
    fn lost_events(&mut self) -> Vec<Event>;

    /// The watch thread's tick (liveness only the process list shows), last at each safety tick.
    fn watch_tick(&mut self);
}

/// One call into the dispatcher, kept while an earlier call runs.
enum Input {
    Event(Event),
    Session(SessionChange),
    /// A re-entered `Ended`: its restore and wait ran at once, only the core's part is left.
    Ended,
    Timer,
}

/// The tray's `LoopHandler`: the core, the clock, the IO queue, the loop's runners, and the loop's own state (the
/// safety tick, the settings mutex, the hibernate thread, what waits while a call runs).
///
/// Not `Send`: it lives on the loop thread.
pub struct Dispatcher<R> {
    core: RefCell<Core>,
    clock: Box<dyn Clock>,
    io: Arc<IoQueue>,
    runners: RefCell<R>,
    ack: Ack,
    /// A call runs; one that comes in meanwhile is a re-entry and only queues. A flag is depth enough: a re-entered
    /// call runs no runner, so it never goes deeper.
    busy: Cell<bool>,
    pending: RefCell<VecDeque<Input>>,
    /// This tray owns the settings mutex: from `Started`, then from the `SettingsOwner` fed.
    owner: Cell<bool>,
    /// A hibernate thread runs: from `start_hibernate` or `start_sleep_check` to its `Done`.
    hibernating: Cell<bool>,
    /// `Command::Quit` ran: nothing more reaches the core.
    quitting: Cell<bool>,
    /// When the next safety tick is due, in run time, so a clock change never moves it.
    next_safety_mono: Cell<u64>,
    /// Every event the core got, in order, for the tests.
    #[cfg(test)]
    fed: RefCell<Vec<Event>>,
}

impl<R: LoopRunners> Dispatcher<R> {
    /// Feeds `first` before anything else, in the order `event.rs` promises the core: `Started`, then
    /// `Session(Locked)`, `JournalCorrupt`, `Updated`, then any other as given. The core drops what comes before
    /// `Started`, so the order is kept here whatever order the caller built.
    pub fn new(
        core: Core,
        clock: Box<dyn Clock>,
        io: Arc<IoQueue>,
        runners: R,
        ack: Ack,
        mut first: Vec<Event>,
    ) -> Self {
        first.sort_by_key(first_rank);
        let owner = first.iter().find_map(|event| match event {
            Event::Started { owner, .. } => Some(*owner),
            _ => None,
        });
        let next_safety_mono = clock
            .now()
            .mono_ms
            .saturating_add(u64::from(SAFETY_TICK_MS));
        let dispatcher = Self {
            core: RefCell::new(core),
            clock,
            io,
            runners: RefCell::new(runners),
            ack,
            busy: Cell::new(false),
            pending: RefCell::new(VecDeque::new()),
            owner: Cell::new(owner.unwrap_or(false)),
            hibernating: Cell::new(false),
            quitting: Cell::new(false),
            next_safety_mono: Cell::new(next_safety_mono),
            #[cfg(test)]
            fed: RefCell::new(Vec::new()),
        };
        for event in first {
            dispatcher.on_event(event);
        }
        dispatcher
    }

    /// An event posted through the sink. Session changes come through `on_session`.
    pub fn on_event(&self, event: Event) -> Flow {
        self.enter(Input::Event(event))
    }

    /// A change from `session::decode`. `Ended` (`WM_ENDSESSION` TRUE) puts the restore first on the IO queue before
    /// the core sees the change, then waits up to 4 s for it. Windows may end the process once the window answers, so
    /// a re-entered `Ended` does not wait for the outer call: it puts the restore first and waits at once, and the
    /// core gets the change after the outer batch, if the process still runs. `EndCancelled` lifts the queue's end
    /// rule and clears the `Ack` before the core sees it, so the apply it gives is not refused.
    pub fn on_session(&self, change: SessionChange) -> Flow {
        if change == SessionChange::Ended && (self.busy.get() || self.quitting.get()) {
            self.restore_first();
            self.wait_for_restore();
            if !self.quitting.get() {
                self.pending.borrow_mut().push_back(Input::Ended);
            }
            return self.flow();
        }
        self.enter(Input::Session(change))
    }

    /// The one timer fired. On a safety tick first the settings mutex and the lost events; then `InputIdle` and
    /// `Tick`, since the countdown decides only on a fresh idle reading; on a safety tick then the watch thread's
    /// tick.
    pub fn on_timer(&self) -> Flow {
        self.enter(Input::Timer)
    }

    /// Milliseconds until the timer should fire: the nearer of the core's next deadline and the next safety tick, from
    /// 10 to 30,000. A `Wall` deadline is read against the wall clock, so a clock change moves it, and the safety tick
    /// bounds the wait whatever the wall clock does. After `Quit` the minimum, so the loop ends soon.
    pub fn timer_ms(&self) -> u32 {
        if self.quitting.get() {
            return TIMER_MIN_MS;
        }
        let now = self.clock.now();
        let safety = self.next_safety_mono.get().saturating_sub(now.mono_ms);
        // `Core::handle` calls nothing out, so the core is never borrowed here; if it ever were, the safety tick
        // stands in rather than a panic, and the outer call re-arms the timer once it returns.
        let deadline = self
            .core
            .try_borrow()
            .ok()
            .and_then(|core| core.next_deadline());
        let ms = deadline.map_or(safety, |d| ms_until(d, now).min(safety));
        u32::try_from(ms).map_or(SAFETY_TICK_MS, |ms| ms.clamp(TIMER_MIN_MS, SAFETY_TICK_MS))
    }

    /// The runners back, for the shutdown after the pump (architecture, "Tray startup" step 8).
    pub fn into_runners(self) -> R {
        self.runners.into_inner()
    }

    fn flow(&self) -> Flow {
        if self.quitting.get() {
            Flow::Quit
        } else {
            Flow::Go
        }
    }

    /// Runs `input`, then every input that came in meanwhile, in order.
    fn enter(&self, input: Input) -> Flow {
        if self.quitting.get() {
            return Flow::Quit;
        }
        if self.busy.replace(true) {
            self.pending.borrow_mut().push_back(input);
            return Flow::Go;
        }
        let mut next = Some(input);
        while let Some(input) = next
            .take()
            .or_else(|| self.pending.borrow_mut().pop_front())
        {
            self.run(input);
            if self.quitting.get() {
                // Nothing reaches the core after `Quit`.
                self.pending.borrow_mut().clear();
            }
        }
        self.busy.set(false);
        self.flow()
    }

    fn run(&self, input: Input) {
        match input {
            Input::Event(event) => self.feed(event),
            Input::Session(SessionChange::Ended) => {
                self.restore_first();
                self.feed(Event::Session(SessionChange::Ended));
                self.wait_for_restore();
            }
            Input::Session(SessionChange::EndCancelled) => {
                self.io.session_resumed();
                self.ack.reset();
                self.feed(Event::Session(SessionChange::EndCancelled));
            }
            Input::Session(change) => self.feed(Event::Session(change)),
            Input::Ended => self.feed(Event::Session(SessionChange::Ended)),
            Input::Timer => self.fire(),
        }
    }

    fn fire(&self) {
        let mono = self.clock.now().mono_ms;
        let safety = mono >= self.next_safety_mono.get();
        if safety {
            self.next_safety_mono
                .set(mono.saturating_add(u64::from(SAFETY_TICK_MS)));
            if !self.owner.get() {
                let got = self.runners.borrow_mut().try_settings_owner();
                if got {
                    self.feed(Event::SettingsOwner);
                }
            }
            let lost = self.runners.borrow_mut().lost_events();
            for event in lost {
                self.feed(event);
            }
        }
        let ms = self.runners.borrow_mut().input_idle_ms();
        self.feed(Event::InputIdle { ms });
        self.feed(Event::Tick);
        if safety && !self.quitting.get() {
            self.runners.borrow_mut().watch_tick();
        }
    }

    /// One event and its batch.
    fn feed(&self, event: Event) {
        if self.quitting.get() {
            return;
        }
        let mut batch = VecDeque::from(self.handle(event));
        while let Some(command) = batch.pop_front() {
            if let Some(answer) = self.send(command)
                && !self.quitting.get()
            {
                batch.extend(self.handle(answer));
            }
        }
    }

    fn handle(&self, event: Event) -> Vec<Command> {
        match &event {
            Event::SettingsOwner => self.owner.set(true),
            Event::Done(Outcome::Hibernate(_) | Outcome::SleepCheck(_)) => {
                self.hibernating.set(false);
            }
            _ => {}
        }
        #[cfg(test)]
        self.fed.borrow_mut().push(event.clone());
        let now = self.clock.now();
        self.core.borrow_mut().handle(now, event)
    }

    /// Runs `command` on its runner, the loop's own or the IO queue; gives the answer of a loop runner that answers at
    /// once.
    fn send(&self, command: Command) -> Option<Event> {
        match command {
            Command::Requests { system, display } => {
                let result = self.runners.borrow_mut().requests(system, display);
                Some(Event::Done(Outcome::Requests(
                    result.map_err(Failure::from),
                )))
            }
            Command::Retake => {
                let result = self.runners.borrow_mut().retake();
                Some(Event::Done(Outcome::Requests(
                    result.map_err(Failure::from),
                )))
            }
            Command::Lock => {
                let result = self.runners.borrow_mut().lock();
                Some(Event::Done(Outcome::Locked(result.map_err(Failure::from))))
            }
            Command::Tray(view) => {
                self.runners.borrow_mut().show_tray(&view);
                None
            }
            Command::Play(cue) => {
                self.runners.borrow_mut().play(cue);
                None
            }
            Command::Hibernate {
                reason,
                floor_pct,
                last_resume_mono_ms,
            } => {
                // A second thread could put the PC down again as soon as it wakes from the first.
                if self.hibernating.replace(true) {
                    self.log("A hibernate was asked while one runs: not started".to_owned());
                    return None;
                }
                let started = self.runners.borrow_mut().start_hibernate(
                    reason,
                    floor_pct,
                    last_resume_mono_ms,
                );
                // Fed here, not posted: a lost post would leave the flag set for the rest of the run.
                started
                    .err()
                    .map(|e| Event::Done(Outcome::Hibernate(HibernateEnd::Failed(e.into()))))
            }
            Command::SleepCheck {
                reason,
                floor_pct,
                last_resume_mono_ms,
            } => {
                if self.hibernating.replace(true) {
                    self.log(
                        "A sleep check was asked while the hibernate thread runs: not started"
                            .to_owned(),
                    );
                    return None;
                }
                let started = self.runners.borrow_mut().start_sleep_check(
                    reason,
                    floor_pct,
                    last_resume_mono_ms,
                );
                // The core checked the lid, idle and floor itself before it asked; only the sound and the second look
                // are lost, so the short timeouts still go, and a closed laptop still sleeps.
                started.err().map(|e| {
                    self.log(format!(
                        "The sleep check thread did not start ({}, code {}): the core's own checks stand, no sound",
                        e.call, e.code
                    ));
                    Event::Done(Outcome::SleepCheck(Ok(())))
                })
            }
            Command::ApplyUpdate => {
                self.runners.borrow_mut().apply_update();
                None
            }
            Command::Quit => {
                self.quitting.set(true);
                None
            }
            command @ (Command::ReadPlan
            | Command::Apply { .. }
            | Command::Restore { .. }
            | Command::ResetLost { .. }
            | Command::WriteState(_)
            | Command::WriteConfig(_)
            | Command::WriteHold { .. }
            | Command::DeleteHold(_)
            | Command::FinishRestoreRequest(_)
            | Command::Notify(_)
            | Command::Log(_)
            | Command::OpenLog) => {
                self.io.push(command);
                None
            }
        }
    }

    /// The end of the session's restore goes first on the IO queue, and the wait is for that one: the IO thread also
    /// set the `Ack` after the restore `Session(Ending)` gave, and that must not end this wait before a second try ran.
    fn restore_first(&self) {
        self.ack.reset();
        self.io.session_ending();
    }

    fn wait_for_restore(&self) {
        let start = Instant::now();
        let line = if self.ack.wait(END_WAIT_MS) {
            format!(
                "Session end: the restore ran in {} ms",
                start.elapsed().as_millis()
            )
        } else {
            format!("Session end: the restore did not finish within {END_WAIT_MS} ms")
        };
        self.log(line);
    }

    /// The loop's own lines go through the IO thread, like the core's.
    fn log(&self, line: String) {
        self.io.push(Command::Log(line));
    }
}

/// Where an event of `first` goes (`event.rs`, "The core may rely on").
fn first_rank(event: &Event) -> u8 {
    match event {
        Event::Started { .. } => 0,
        Event::Session(SessionChange::Locked) => 1,
        Event::JournalCorrupt => 2,
        Event::Updated { .. } => 3,
        _ => 4,
    }
}

/// Milliseconds from `now` to `deadline` on its own clock; 0 when it is past.
fn ms_until(deadline: Deadline, now: Now) -> u64 {
    match deadline {
        Deadline::Wall(wall_ms) => {
            let ahead = (i128::from(wall_ms) - i128::from(now.wall_ms)).max(0);
            u64::try_from(ahead).unwrap_or(u64::MAX)
        }
        Deadline::Mono(mono_ms) => mono_ms.saturating_sub(now.mono_ms),
    }
}

/// Each method forwards to the `&self` method of the same job, so a window that calls in again from a nested pump
/// holds only shared references to the dispatcher, and the inner call queues.
impl<R: LoopRunners> LoopHandler for Dispatcher<R> {
    fn event(&self, event: Event) -> Flow {
        self.on_event(event)
    }

    fn session(&self, change: SessionChange) -> Flow {
        self.on_session(change)
    }

    fn timer(&self) -> Flow {
        self.on_timer()
    }

    fn next_timer_ms(&self) -> u32 {
        self.timer_ms()
    }
}

#[cfg(test)]
mod tests;
