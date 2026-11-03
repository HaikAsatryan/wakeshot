//! The dispatcher: the core in the loop, the runner for each command, the tick, end of session and quit.
//!
//! Owner: `p6-int-dispatch`; fixes by the e2e cards. `Dispatcher` implements `LoopHandler`; trait `LoopRunners` holds
//! the loop's runners (`docs/phases/06-integration.md` note 4).
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

use wakeshot_core::command::{Command, CountdownReason, Failure, Outcome};
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
/// idle, the settings mutex and the watch thread's tick (phase file note 4's table).
///
/// Owner: `p6-int-dispatch`; `p6-int-tray-mode` implements it over the real or dry-run objects. Every call comes on the
/// loop thread, one at a time. A runner may pump messages: a call into the dispatcher from inside one is queued. A call
/// that answers at once returns its result, and the dispatcher feeds it to the core; the others answer later through
/// the sink, as `Done(Sound)`, `Done(Hibernate)` and `Done(Update)`.
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
    /// called again before that event is fed.
    fn start_hibernate(
        &mut self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    );

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
/// Owner: `p6-int-dispatch`. Not `Send`: it lives on the loop thread.
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
    /// A hibernate thread runs: from `start_hibernate` to its `Done(Hibernate)`.
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

    /// The runners back, for the shutdown order after the pump (phase file note 6, step 9).
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
            Event::Done(Outcome::Hibernate(_)) => self.hibernating.set(false),
            _ => {}
        }
        #[cfg(test)]
        self.fed.borrow_mut().push(event.clone());
        let now = self.clock.now();
        self.core.borrow_mut().handle(now, event)
    }

    /// Runs `command` on the runner of the phase file's table; gives the answer of a loop runner that answers at once.
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
                } else {
                    self.runners.borrow_mut().start_hibernate(
                        reason,
                        floor_pct,
                        last_resume_mono_ms,
                    );
                }
                None
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
mod tests {
    use std::cell::RefMut;
    use std::collections::BTreeSet;
    use std::rc::{Rc, Weak};
    use std::thread;
    use std::time::{Duration, Instant};

    use wakeshot_core::command::{FileKind, HibernateEnd, Outcome, RestoreWhy};
    use wakeshot_core::files::{Config, HoldFile};
    use wakeshot_core::holds::HoldId;
    use wakeshot_core::policy::ending::IDLE_MS;
    use wakeshot_core::process::Alive;
    use wakeshot_core::sim::{
        self, BALANCED, KEYS, OTHER_PLAN, START, Start, applied, claude, entries, plan, restored,
        watched,
    };
    use wakeshot_core::sources::Activity;
    use wakeshot_core::testing::FakeClock;
    use wakeshot_core::time::Now;
    use wakeshot_core::view::{IconState, KeepAwake, MenuCommand, Notice};

    use super::*;

    const SECOND: u64 = 1_000;
    const MINUTE: u64 = 60 * SECOND;
    const HOUR: u64 = 60 * MINUTE;
    const TICK: u64 = 30 * SECOND;

    /// Not a multiple of the 30 s safety tick, so a time checked to the millisecond is the core's own deadline.
    const OFF_THE_TICK: u64 = 7 * SECOND;

    const SYSTEM: Call = Call::Requests {
        system: true,
        display: false,
    };
    const OFF: Call = Call::Requests {
        system: false,
        display: false,
    };
    const SESSION_END: Command = Command::Restore {
        plan: None,
        why: RestoreWhy::SessionEnd,
    };

    /// Every `Command` variant, by the name [`name`] gives.
    const ALL: [&str; 19] = [
        "Requests",
        "Retake",
        "ReadPlan",
        "Apply",
        "Restore",
        "Lock",
        "Hibernate",
        "Tray",
        "Notify",
        "Play",
        "WriteState",
        "WriteConfig",
        "WriteHold",
        "DeleteHold",
        "FinishRestoreRequest",
        "Log",
        "OpenLog",
        "ApplyUpdate",
        "Quit",
    ];

    /// No wildcard: a new variant fails to build here until the test knows where it goes.
    fn name(command: &Command) -> &'static str {
        match command {
            Command::Requests { .. } => "Requests",
            Command::Retake => "Retake",
            Command::ReadPlan => "ReadPlan",
            Command::Apply { .. } => "Apply",
            Command::Restore { .. } => "Restore",
            Command::Lock => "Lock",
            Command::Hibernate { .. } => "Hibernate",
            Command::Tray(_) => "Tray",
            Command::Notify(_) => "Notify",
            Command::Play(_) => "Play",
            Command::WriteState(_) => "WriteState",
            Command::WriteConfig(_) => "WriteConfig",
            Command::WriteHold { .. } => "WriteHold",
            Command::DeleteHold(_) => "DeleteHold",
            Command::FinishRestoreRequest(_) => "FinishRestoreRequest",
            Command::Log(_) => "Log",
            Command::OpenLog => "OpenLog",
            Command::ApplyUpdate => "ApplyUpdate",
            Command::Quit => "Quit",
        }
    }

    /// One runner call.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        Requests {
            system: bool,
            display: bool,
        },
        Retake,
        Lock,
        Tray(IconState),
        Play(Cue),
        Hibernate {
            reason: CountdownReason,
            floor_pct: u8,
            last_resume_mono_ms: Option<u64>,
        },
        ApplyUpdate,
        InputIdle(u64),
        TryOwner,
        Lost,
        WatchTick,
    }

    impl Call {
        /// The command this call ran, if it ran one.
        fn command(&self) -> Option<&'static str> {
            match self {
                Call::Requests { .. } => Some("Requests"),
                Call::Retake => Some("Retake"),
                Call::Lock => Some("Lock"),
                Call::Tray(_) => Some("Tray"),
                Call::Play(_) => Some("Play"),
                Call::Hibernate { .. } => Some("Hibernate"),
                Call::ApplyUpdate => Some("ApplyUpdate"),
                Call::InputIdle(_) | Call::TryOwner | Call::Lost | Call::WatchTick => None,
            }
        }

        /// A call the timer makes, not a command.
        fn ticks(&self) -> bool {
            self.command().is_none()
        }
    }

    /// Runs inside one `show_tray` call, as a message the runner's wait lets through would.
    type Hook = Box<dyn FnOnce(&Dispatcher<Recorder>)>;

    /// The loop's runners as a record: each call with the event the core got last, answered as the test set.
    struct Recorder {
        clock: Rc<FakeClock>,
        back: Weak<Dispatcher<Recorder>>,
        calls: Vec<(Call, Option<Event>)>,
        /// What `requests` answers.
        requests: Result<(), WinError>,
        /// Run time of the user's last input; input idle grows from it.
        input_mono: u64,
        /// The answers of `try_settings_owner`, in turn; `false` once used up.
        owner: VecDeque<bool>,
        /// What the next `lost_events` gives.
        lost: Vec<Event>,
        /// One per `show_tray` call, in turn.
        on_show: VecDeque<Hook>,
    }

    impl Recorder {
        fn record(&mut self, call: Call) {
            // `None` while `Dispatcher::new` runs: the back pointer works only once it returned.
            let last = self
                .back
                .upgrade()
                .and_then(|d| d.fed.borrow().last().cloned());
            self.calls.push((call, last));
        }
    }

    impl LoopRunners for Recorder {
        fn requests(&mut self, system: bool, display: bool) -> Result<(), WinError> {
            self.record(Call::Requests { system, display });
            self.requests
        }

        fn retake(&mut self) -> Result<(), WinError> {
            self.record(Call::Retake);
            Ok(())
        }

        fn lock(&mut self) -> Result<(), WinError> {
            self.record(Call::Lock);
            Ok(())
        }

        fn show_tray(&mut self, view: &TrayView) {
            self.record(Call::Tray(view.icon));
            if let Some(hook) = self.on_show.pop_front()
                && let Some(d) = self.back.upgrade()
            {
                hook(&d);
            }
        }

        fn play(&mut self, cue: Cue) {
            self.record(Call::Play(cue));
        }

        fn start_hibernate(
            &mut self,
            reason: CountdownReason,
            floor_pct: u8,
            last_resume_mono_ms: Option<u64>,
        ) {
            self.record(Call::Hibernate {
                reason,
                floor_pct,
                last_resume_mono_ms,
            });
        }

        fn apply_update(&mut self) {
            self.record(Call::ApplyUpdate);
        }

        fn input_idle_ms(&mut self) -> u64 {
            let ms = self.clock.now().mono_ms.saturating_sub(self.input_mono);
            self.record(Call::InputIdle(ms));
            ms
        }

        fn try_settings_owner(&mut self) -> bool {
            self.record(Call::TryOwner);
            self.owner.pop_front().unwrap_or(false)
        }

        fn lost_events(&mut self) -> Vec<Event> {
            self.record(Call::Lost);
            std::mem::take(&mut self.lost)
        }

        fn watch_tick(&mut self) {
            self.record(Call::WatchTick);
        }
    }

    /// Runs at each clock read the dispatcher makes, when set.
    type Probe = Rc<RefCell<Option<Box<dyn Fn()>>>>;

    /// The fake clock the test moves; `probe` runs at each read the dispatcher makes.
    struct TestClock {
        clock: Rc<FakeClock>,
        probe: Probe,
    }

    impl Clock for TestClock {
        fn now(&self) -> Now {
            if let Some(probe) = self.probe.borrow().as_ref() {
                probe();
            }
            self.clock.now()
        }
    }

    /// The dispatcher over the real core, a fake clock, the recorder and the real queue, which the test pops as the
    /// IO thread would.
    struct World {
        clock: Rc<FakeClock>,
        probe: Probe,
        io: Arc<IoQueue>,
        ack: Ack,
        d: Rc<Dispatcher<Recorder>>,
        /// Everything popped so far, and how many runner calls [`World::calls_seen`] gave so far.
        popped: RefCell<Vec<Command>>,
        calls_read: Cell<usize>,
    }

    impl World {
        fn new(first: Vec<Event>) -> Self {
            Self::with(first, |_| {})
        }

        fn with(first: Vec<Event>, setup: impl FnOnce(&mut Recorder)) -> Self {
            let clock = Rc::new(FakeClock::new(START));
            let probe = Rc::new(RefCell::new(None));
            let io = Arc::new(IoQueue::new());
            let ack = Ack::new();
            let d = Rc::new_cyclic(|back| {
                let mut recorder = Recorder {
                    clock: Rc::clone(&clock),
                    back: back.clone(),
                    calls: Vec::new(),
                    requests: Ok(()),
                    input_mono: START.mono_ms,
                    owner: VecDeque::new(),
                    lost: Vec::new(),
                    on_show: VecDeque::new(),
                };
                setup(&mut recorder);
                let test_clock = TestClock {
                    clock: Rc::clone(&clock),
                    probe: Rc::clone(&probe),
                };
                Dispatcher::new(
                    Core::new(Config::default()),
                    Box::new(test_clock),
                    Arc::clone(&io),
                    recorder,
                    ack.clone(),
                    first,
                )
            });
            Self {
                clock,
                probe,
                io,
                ack,
                d,
                popped: RefCell::new(Vec::new()),
                calls_read: Cell::new(0),
            }
        }

        fn post(&self, event: Event) -> Flow {
            self.d.on_event(event)
        }

        /// Pops everything queued now, as the IO thread would.
        fn queue(&self) -> Vec<Command> {
            let got: Vec<Command> = std::iter::from_fn(|| self.io.pop(0)).collect();
            self.popped.borrow_mut().extend(got.iter().cloned());
            got
        }

        /// The runner calls since the last look, each with the event the core got last before it.
        fn calls_seen(&self) -> Vec<(Call, Option<Event>)> {
            let recorder = self.d.runners.borrow();
            let new = recorder.calls[self.calls_read.get()..].to_vec();
            self.calls_read.set(recorder.calls.len());
            new
        }

        fn calls(&self) -> Vec<Call> {
            self.calls_seen()
                .into_iter()
                .map(|(call, _)| call)
                .collect()
        }

        fn recorder(&self) -> RefMut<'_, Recorder> {
            self.d.runners.borrow_mut()
        }

        fn fed(&self) -> Vec<Event> {
            self.d.fed.borrow().clone()
        }

        /// What the window does: wait as long as the dispatcher asks, then fire. Gives the wait.
        fn fire(&self) -> u32 {
            let ms = self.d.timer_ms();
            self.clock.run(u64::from(ms));
            self.d.on_timer();
            ms
        }

        fn mono(&self) -> u64 {
            self.clock.now().mono_ms
        }

        /// Empties the queue and the calls, so a test sees only what comes next.
        fn settle(&self) {
            self.queue();
            self.calls();
        }
    }

    fn started() -> Event {
        Start::default().event()
    }

    fn working(pid: u32) -> Event {
        sim::sources(vec![(claude(pid, Activity::Working), Alive::Yes)])
    }

    fn idle(pid: u32) -> Event {
        sim::sources(vec![(claude(pid, Activity::Idle), Alive::Yes)])
    }

    fn lid(closed: bool) -> Event {
        Event::Lid { closed }
    }

    fn power(ac: bool) -> Event {
        Event::Power {
            ac,
            battery_pct: Some(80),
        }
    }

    fn menu(command: MenuCommand) -> Event {
        Event::Menu(command)
    }

    fn written(id: &HoldId) -> Event {
        Event::Done(Outcome::Written {
            file: FileKind::Hold(id.clone()),
            result: Ok(()),
        })
    }

    fn notices(commands: &[Command]) -> Vec<&Notice> {
        commands
            .iter()
            .filter_map(|c| match c {
                Command::Notify(notice) => Some(notice),
                _ => None,
            })
            .collect()
    }

    fn applies(commands: &[Command]) -> usize {
        commands
            .iter()
            .filter(|c| matches!(c, Command::Apply { .. }))
            .count()
    }

    fn logs(commands: &[Command]) -> Vec<&str> {
        commands
            .iter()
            .filter_map(|c| match c {
                Command::Log(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Started on AC with the lid open and Haik's own values read; nothing holds. Queue and calls emptied.
    fn ready() -> World {
        let w = World::new(vec![started()]);
        w.post(power(true));
        w.post(lid(false));
        w.post(Event::Plan(Ok(plan(BALANCED))));
        w.settle();
        w
    }

    /// A Claude Code session holds and the apply worked, so the core's journal mirror owns the three keys.
    fn holding() -> World {
        let w = ready();
        w.post(working(1));
        w.post(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
        w.settle();
        w
    }

    /// Keep awake from the menu, then what the IO and watch threads do: the file written, the folder posted.
    fn keep_awake(w: &World, choice: KeepAwake) -> (HoldId, HoldFile) {
        w.post(menu(MenuCommand::KeepAwake(choice)));
        let (id, hold) = w
            .queue()
            .into_iter()
            .find_map(|c| match c {
                Command::WriteHold { id, hold } => Some((id, hold)),
                _ => None,
            })
            .expect("a WriteHold on the queue");
        w.post(written(&id));
        w.post(Event::Holds(vec![watched(&id, &hold, Alive::Yes)]));
        (id, hold)
    }

    /// Sets the `Ack` after `ms`, as the IO thread does once the restore ran.
    fn ack_after(ack: &Ack, ms: u64) -> thread::JoinHandle<()> {
        let ack = ack.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(ms));
            ack.set();
        })
    }

    #[test]
    fn every_command_reaches_the_runner_of_the_table() {
        let w = World::new(vec![
            Start {
                first_run: true,
                ..Start::default()
            }
            .event(),
        ]);
        w.post(power(true));
        w.post(lid(false));
        // The first tick reads the plan.
        w.fire();
        w.post(Event::Plan(Ok(plan(BALANCED))));
        w.post(working(1));
        w.post(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
        w.post(Event::Resumed);
        w.post(lid(true));
        keep_awake(&w, KeepAwake::Minutes(30));
        w.post(menu(MenuCommand::KeepAwake(KeepAwake::Off)));
        w.post(Event::Holds(Vec::new()));
        w.post(menu(MenuCommand::OpenLog));
        w.post(menu(MenuCommand::Restore));
        w.post(restored(None, RestoreWhy::UserAsked, Ok(()), Vec::new()));
        // The work ends with the lid closed: the countdown runs to the idle mark.
        let input = w.mono();
        w.recorder().input_mono = input;
        w.post(idle(1));
        for _ in 0..40 {
            if w.mono() > input + IDLE_MS + MINUTE {
                break;
            }
            w.fire();
        }
        w.post(Event::Done(Outcome::Hibernate(HibernateEnd::Resumed)));
        w.post(lid(false));
        w.post(Event::UpdateReady {
            version: "1.1.0".to_owned(),
        });
        let flow = w.post(Event::Done(Outcome::Update(Ok(()))));
        w.queue();

        let at_runners: BTreeSet<&str> =
            w.d.runners
                .borrow()
                .calls
                .iter()
                .filter_map(|(call, _)| call.command())
                .collect();
        let at_io: BTreeSet<&str> = w.popped.borrow().iter().map(name).collect();
        let loop_side = BTreeSet::from([
            "Requests",
            "Retake",
            "Lock",
            "Tray",
            "Play",
            "Hibernate",
            "ApplyUpdate",
        ]);
        let io_side = BTreeSet::from([
            "ReadPlan",
            "Apply",
            "Restore",
            "WriteState",
            "WriteConfig",
            "WriteHold",
            "DeleteHold",
            "FinishRestoreRequest",
            "Notify",
            "Log",
            "OpenLog",
        ]);
        assert_eq!(at_runners, loop_side);
        assert_eq!(at_io, io_side);
        assert_eq!(flow, Flow::Quit, "Quit ends the loop");
        let mut every: BTreeSet<&str> = loop_side.union(&io_side).copied().collect();
        every.insert("Quit");
        assert_eq!(every, BTreeSet::from(ALL));
    }

    #[test]
    fn keep_awake_from_the_menu_takes_the_requests_and_shows_holding() {
        let w = ready();
        w.post(menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
        let queued = w.queue();
        let (id, hold) = queued
            .iter()
            .find_map(|c| match c {
                Command::WriteHold { id, hold } => Some((id.clone(), hold.clone())),
                _ => None,
            })
            .expect("a WriteHold on the queue");
        w.post(written(&id));
        w.post(Event::Holds(vec![watched(&id, &hold, Alive::Yes)]));

        let calls = w.calls_seen();
        let requests = calls
            .iter()
            .position(|(call, _)| *call == SYSTEM)
            .expect("requests(true, false)");
        let tray = calls
            .iter()
            .position(|(call, _)| *call == Call::Tray(IconState::Holding))
            .expect("show_tray with the holding icon");
        assert!(requests < tray, "{calls:?}");
        let answer = Event::Done(Outcome::Requests(Ok(())));
        assert_eq!(
            calls[tray].1.as_ref(),
            Some(&answer),
            "the answer went in before the next command ran"
        );
        let fed = w.fed();
        let holds = fed
            .iter()
            .rposition(|e| matches!(e, Event::Holds(_)))
            .expect("the folder was fed");
        assert_eq!(fed.get(holds + 1), Some(&answer), "fed at once: {fed:?}");
    }

    #[test]
    fn a_failed_request_shows_idle_after_the_batchs_holding_icon() {
        let w = ready();
        w.recorder().requests = Err(WinError {
            call: "PowerSetRequest",
            code: 5,
        });
        w.post(working(1));
        let icons: Vec<Call> = w
            .calls()
            .into_iter()
            .filter(|c| matches!(c, Call::Tray(_)))
            .collect();
        assert_eq!(
            icons,
            [Call::Tray(IconState::Holding), Call::Tray(IconState::Idle)],
            "the answer's view comes last, so the icon tells the truth"
        );
        assert_eq!(notices(&w.queue()), [&Notice::CannotHold]);
    }

    #[test]
    fn lid_closed_while_holding_locks_and_chimes() {
        let w = holding();
        w.post(lid(true));
        let calls = w.calls_seen();
        let only: Vec<&Call> = calls.iter().map(|(call, _)| call).collect();
        assert_eq!(
            only,
            [&Call::Lock, &Call::Play(Cue::LidClosedHolding)],
            "the lock first, then the chime"
        );
        assert_eq!(
            calls[1].1,
            Some(Event::Done(Outcome::Locked(Ok(())))),
            "the lock's answer went in before the chime"
        );
    }

    #[test]
    fn countdown_hibernates_at_the_idle_mark_with_input_idle_before_each_tick() {
        let w = holding();
        w.post(lid(true));
        w.clock.run(OFF_THE_TICK);
        let input = w.mono();
        w.recorder().input_mono = input;
        w.post(idle(1));
        w.post(restored(None, RestoreWhy::WorkEnded, Ok(()), Vec::new()));
        w.settle();
        let fed_before = w.fed().len();

        let mark = input + IDLE_MS;
        let mut waits = Vec::new();
        let mut hibernated_at = None;
        while w.mono() < mark + 3 * TICK {
            let at = w.mono();
            let ms = w.fire();
            waits.push((at, ms));
            let started = w
                .calls()
                .iter()
                .filter(|c| matches!(c, Call::Hibernate { .. }))
                .count();
            if started > 0 {
                assert_eq!(started, 1);
                assert_eq!(hibernated_at, None, "a second hibernate: {waits:?}");
                hibernated_at = Some(w.mono());
            }
        }
        assert_eq!(hibernated_at, Some(mark), "{waits:?}");
        assert!(
            waits
                .iter()
                .any(|&(at, ms)| at + u64::from(ms) == mark && ms < 30_000),
            "the timer pointed at the idle mark: {waits:?}"
        );
        assert!(waits.iter().all(|&(_, ms)| (10..=30_000).contains(&ms)));
        let started: Vec<Call> =
            w.d.runners
                .borrow()
                .calls
                .iter()
                .map(|(call, _)| call.clone())
                .filter(|call| matches!(call, Call::Hibernate { .. }))
                .collect();
        assert_eq!(
            started,
            [Call::Hibernate {
                reason: CountdownReason::WorkEnded,
                floor_pct: 20,
                last_resume_mono_ms: None,
            }]
        );

        let fed = w.fed();
        let since = &fed[fed_before..];
        for (i, event) in since.iter().enumerate() {
            if *event == Event::Tick {
                assert!(
                    i > 0 && matches!(since[i - 1], Event::InputIdle { .. }),
                    "InputIdle right before each Tick: {since:?}"
                );
            }
        }
    }

    #[test]
    fn safety_tick_takes_the_settings_mutex_and_ticks_the_watch_once() {
        let w = World::with(
            vec![
                Start {
                    owner: false,
                    ..Start::default()
                }
                .event(),
            ],
            |r| r.owner = VecDeque::from([false, true]),
        );
        w.post(power(true));
        w.post(lid(false));
        w.post(Event::Plan(Ok(plan(BALANCED))));
        w.post(working(1));
        assert_eq!(applies(&w.queue()), 0, "requests only while not the owner");
        w.calls();

        let mut next_safety = START.mono_ms + TICK;
        let mut owner = false;
        let mut fires = 0;
        let mut safety_fires = 0;
        let mut applied_at = None;
        while w.mono() < START.mono_ms + 95 * SECOND {
            w.fire();
            fires += 1;
            let now = w.mono();
            let safety = now >= next_safety;
            let ticks: Vec<Call> = w
                .calls()
                .into_iter()
                .filter(Call::ticks)
                .map(|c| match c {
                    Call::InputIdle(_) => Call::InputIdle(0),
                    other => other,
                })
                .collect();
            let mut expected = Vec::new();
            if safety {
                safety_fires += 1;
                next_safety = now + TICK;
                if !owner {
                    expected.push(Call::TryOwner);
                }
                expected.push(Call::Lost);
            }
            expected.push(Call::InputIdle(0));
            if safety {
                expected.push(Call::WatchTick);
            }
            assert_eq!(ticks, expected, "fire {fires} at {now}");

            let queued = w.queue();
            if applies(&queued) > 0 {
                assert_eq!(applied_at, None);
                applied_at = Some(now);
                owner = true;
                w.post(applied(BALANCED, Ok(()), entries(BALANCED, &KEYS)));
            }
        }
        assert_eq!(applied_at, Some(START.mono_ms + 2 * TICK));
        assert_eq!(safety_fires, 3);
        assert!(fires > safety_fires, "deadlines fired between the ticks");
        let owners = w
            .fed()
            .iter()
            .filter(|e| **e == Event::SettingsOwner)
            .count();
        assert_eq!(owners, 1);
    }

    #[test]
    fn an_owner_from_the_start_never_tries_the_settings_mutex() {
        // A runner answers `true` for a mutex it already holds: each try would feed `SettingsOwner`, so an apply.
        let w = World::with(vec![started()], |r| {
            r.owner = VecDeque::from([true, true, true]);
        });
        while w.mono() < START.mono_ms + 3 * TICK {
            w.fire();
        }
        let calls = w.calls();
        assert_eq!(calls.iter().filter(|c| **c == Call::WatchTick).count(), 3);
        assert!(!calls.contains(&Call::TryOwner), "{calls:?}");
        assert!(!w.fed().contains(&Event::SettingsOwner));
    }

    #[test]
    fn lost_events_are_fed_on_the_safety_tick_before_input_idle() {
        let w = holding();
        w.recorder().lost = vec![lid(true)];
        let mut fired_lost = false;
        while !fired_lost {
            assert!(w.mono() < START.mono_ms + 2 * TICK, "no safety tick came");
            w.fire();
            let calls = w.calls();
            let Some(lost) = calls.iter().position(|c| *c == Call::Lost) else {
                assert!(!calls.contains(&Call::Lock), "{calls:?}");
                continue;
            };
            fired_lost = true;
            let at = |wanted: &Call| calls.iter().position(|c| c == wanted);
            let lock = at(&Call::Lock).expect("the mended lid locks");
            let chime = at(&Call::Play(Cue::LidClosedHolding)).expect("and chimes");
            let input = calls
                .iter()
                .position(|c| matches!(c, Call::InputIdle(_)))
                .expect("input idle read");
            assert!(lost < lock && lock < chime && chime < input, "{calls:?}");
        }
    }

    #[test]
    fn the_timer_never_asks_for_more_than_30_s() {
        let w = ready();
        let check = |w: &World| {
            let ms = w.d.timer_ms();
            assert!((10..=30_000).contains(&ms), "{ms}");
            ms
        };
        keep_awake(&w, KeepAwake::Minutes(30));
        check(&w);
        // Past the 1 s `state.json` wait, the Keep awake end is the core's only deadline.
        for _ in 0..5 {
            if matches!(w.d.core.borrow().next_deadline(), Some(Deadline::Wall(_))) {
                break;
            }
            w.fire();
            check(&w);
        }
        assert!(matches!(
            w.d.core.borrow().next_deadline(),
            Some(Deadline::Wall(_))
        ));
        // The nearer one wins: the first safety tick, not the Keep awake end 30 minutes away.
        assert!(w.mono() < START.mono_ms + TICK);
        assert_eq!(u64::from(check(&w)), START.mono_ms + TICK - w.mono());

        // The clock jumps 2 hours ahead: the Keep awake end is past, so the timer fires at once.
        let wall = w.clock.now().wall_ms;
        w.clock.set_wall(wall + 2 * HOUR as i64);
        assert_eq!(check(&w), 10);
        for _ in 0..5 {
            w.fire();
            check(&w);
        }

        // A new Keep awake, then the clock 2 hours back: its end is far, the safety tick still comes in 30 s.
        keep_awake(&w, KeepAwake::Minutes(30));
        let wall = w.clock.now().wall_ms;
        w.clock.set_wall(wall - 2 * HOUR as i64);
        for _ in 0..5 {
            check(&w);
            w.fire();
        }

        // A suspend gap: only the wall clock moves, so the safety tick is still at most 30 s of run time away.
        w.clock.suspend(3 * HOUR);
        check(&w);
        w.post(Event::Resumed);
        for _ in 0..5 {
            check(&w);
            w.fire();
        }
    }

    #[test]
    fn reentered_events_are_fed_after_the_outer_batch_in_order() {
        let w = ready();
        let during: Rc<RefCell<Vec<String>>> = Rc::default();
        let note = Rc::clone(&during);
        w.recorder().on_show.push_back(Box::new(move |d| {
            let before = d.fed.borrow().len();
            let a = d.on_event(menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))));
            let b = d.on_event(menu(MenuCommand::OpenLog));
            let mut note = note.borrow_mut();
            note.push(format!("{a:?} {b:?}"));
            note.push(format!("fed {}", d.fed.borrow().len() - before));
            note.push(format!("core free {}", d.core.try_borrow_mut().is_ok()));
        }));
        let outer = working(1);
        assert_eq!(w.post(outer.clone()), Flow::Go);
        assert_eq!(
            *during.borrow(),
            ["Go Go", "fed 0", "core free true"],
            "the nested calls only queued"
        );

        let fed = w.fed();
        let at = |wanted: &Event| fed.iter().position(|e| e == wanted).unwrap();
        let outer_at = at(&outer);
        let first = at(&menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))));
        let second = at(&menu(MenuCommand::OpenLog));
        assert_eq!(
            fed[outer_at + 1],
            Event::Done(Outcome::Requests(Ok(()))),
            "{fed:?}"
        );
        assert_eq!((first, second), (outer_at + 2, outer_at + 3), "{fed:?}");

        let names: Vec<&str> = w.queue().iter().map(name).collect();
        let pos = |wanted: &str| names.iter().position(|n| *n == wanted).unwrap();
        assert!(pos("Apply") < pos("WriteHold"), "{names:?}");
        assert!(pos("WriteHold") < pos("OpenLog"), "{names:?}");
    }

    /// The window's own path: the pump holds a shared `&dyn LoopHandler` and a nested pump calls it again, every
    /// method of the trait, while the outer call runs. With `&mut self` this test would not build.
    #[test]
    fn the_window_can_call_in_again_through_the_trait() {
        let w = ready();
        let during: Rc<RefCell<Vec<String>>> = Rc::default();
        let note = Rc::clone(&during);
        w.recorder().on_show.push_back(Box::new(move |d| {
            let nested: &dyn LoopHandler = d;
            let before = d.fed.borrow().len();
            let event = nested.event(menu(MenuCommand::OpenLog));
            let timer = nested.timer();
            let session = nested.session(SessionChange::Locked);
            let ms = nested.next_timer_ms();
            let mut note = note.borrow_mut();
            note.push(format!("{event:?} {timer:?} {session:?}"));
            note.push(format!("fed {}", d.fed.borrow().len() - before));
            note.push(format!(
                "timer {}",
                (TIMER_MIN_MS..=SAFETY_TICK_MS).contains(&ms)
            ));
            note.push(format!("core free {}", d.core.try_borrow_mut().is_ok()));
        }));
        let outer = working(1);
        let handler: &dyn LoopHandler = &*w.d;
        assert_eq!(handler.event(outer.clone()), Flow::Go);
        assert_eq!(
            *during.borrow(),
            ["Go Go Go", "fed 0", "timer true", "core free true"],
            "the nested calls only queued"
        );

        let fed = w.fed();
        let outer_at = fed.iter().position(|e| *e == outer).unwrap();
        let after = &fed[outer_at + 1..];
        assert!(after.len() >= 5, "{fed:?}");
        assert_eq!(after[0], Event::Done(Outcome::Requests(Ok(()))), "{fed:?}");
        // Fed after the outer batch, in the order they came: the event, the timer's two, the session change.
        assert_eq!(after[1], menu(MenuCommand::OpenLog), "{fed:?}");
        assert!(matches!(after[2], Event::InputIdle { .. }), "{fed:?}");
        assert_eq!(after[3], Event::Tick, "{fed:?}");
        assert_eq!(after[4], Event::Session(SessionChange::Locked), "{fed:?}");
        assert_eq!(handler.next_timer_ms(), w.d.timer_ms());
    }

    #[test]
    fn started_is_always_the_cores_first_event() {
        let updated = Event::Updated {
            version: "1.1.0".to_owned(),
        };
        let w = World::new(vec![
            updated.clone(),
            Event::JournalCorrupt,
            Event::Session(SessionChange::Locked),
            started(),
        ]);
        assert_eq!(
            w.fed()[..4],
            [
                started(),
                Event::Session(SessionChange::Locked),
                Event::JournalCorrupt,
                updated.clone(),
            ]
        );
        assert!(notices(&w.queue()).is_empty(), "held while locked");
        w.d.on_session(SessionChange::Unlocked);
        assert_eq!(
            notices(&w.queue()),
            [
                &Notice::JournalCorrupt,
                &Notice::Updated {
                    version: "1.1.0".to_owned()
                }
            ]
        );
    }

    #[test]
    fn started_locked_updated_gives_no_notice_until_unlocked() {
        let w = World::new(vec![
            started(),
            Event::Session(SessionChange::Locked),
            Event::Updated {
                version: "1.1.0".to_owned(),
            },
        ]);
        assert_eq!(w.fed()[0], started());
        for _ in 0..4 {
            w.fire();
        }
        assert!(notices(&w.queue()).is_empty());
        w.d.on_session(SessionChange::Unlocked);
        assert_eq!(
            notices(&w.queue()),
            [&Notice::Updated {
                version: "1.1.0".to_owned()
            }]
        );
    }

    #[test]
    fn quit_from_the_menu_releases_restores_deletes_and_ends() {
        let w = ready();
        let (id, _) = keep_awake(&w, KeepAwake::UntilOff);
        w.settle();
        assert_eq!(w.post(menu(MenuCommand::Quit)), Flow::Quit);
        assert_eq!(w.calls(), [OFF]);
        assert_eq!(
            w.queue(),
            [
                Command::Restore {
                    plan: None,
                    why: RestoreWhy::Quit
                },
                Command::DeleteHold(id),
            ]
        );

        // Nothing reaches the core after it.
        let fed = w.fed().len();
        assert_eq!(
            w.post(menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30)))),
            Flow::Quit
        );
        assert_eq!(w.d.on_session(SessionChange::Locked), Flow::Quit);
        assert_eq!(w.d.on_timer(), Flow::Quit);
        assert_eq!(w.fed().len(), fed);
        assert!(w.queue().is_empty());
        assert!(w.calls().is_empty());
        assert_eq!(w.d.timer_ms(), 10, "the loop ends at once");
    }

    #[test]
    fn a_session_end_after_quit_still_restores_first_and_waits() {
        // The pump may still answer `WM_ENDSESSION` after `Quit`, while the IO thread runs what is queued.
        let w = holding();
        assert_eq!(w.post(menu(MenuCommand::Quit)), Flow::Quit);
        w.settle();
        let fed = w.fed().len();
        let setter = ack_after(&w.ack, 100);
        let start = Instant::now();
        assert_eq!(w.d.on_session(SessionChange::Ended), Flow::Quit);
        let took = start.elapsed();
        setter.join().expect("the setter ends");
        assert!(took >= Duration::from_millis(100), "{took:?}");
        assert_eq!(w.queue().first(), Some(&SESSION_END));
        assert_eq!(w.fed().len(), fed, "the core got nothing after Quit");
    }

    #[test]
    fn a_second_hibernate_while_one_runs_logs_and_starts_nothing() {
        let w = ready();
        let hibernate = Command::Hibernate {
            reason: CountdownReason::WorkEnded,
            floor_pct: 20,
            last_resume_mono_ms: None,
        };
        let started = Call::Hibernate {
            reason: CountdownReason::WorkEnded,
            floor_pct: 20,
            last_resume_mono_ms: None,
        };
        assert_eq!(w.d.send(hibernate.clone()), None);
        assert_eq!(w.d.send(hibernate.clone()), None);
        assert_eq!(w.calls(), std::slice::from_ref(&started));
        assert_eq!(logs(&w.queue()).len(), 1);

        // Once its answer came, the next one starts.
        w.post(Event::Done(Outcome::Hibernate(HibernateEnd::Resumed)));
        w.settle();
        w.d.send(hibernate);
        assert_eq!(w.calls(), [started]);
        assert!(w.queue().is_empty());
    }

    #[test]
    fn session_end_puts_the_restore_first_before_the_core_sees_it() {
        let w = holding();
        let seen: Rc<RefCell<Vec<Vec<Command>>>> = Rc::default();
        let note = Rc::clone(&seen);
        let io = Arc::clone(&w.io);
        // At each clock read, what the queue holds; the first read in the call is the one for the core.
        *w.probe.borrow_mut() = Some(Box::new(move || {
            let queued = RefCell::new(Vec::new());
            io.drop_queued(&|c| {
                queued.borrow_mut().push(c.clone());
                false
            });
            note.borrow_mut().push(queued.into_inner());
        }));
        let setter = ack_after(&w.ack, 20);
        assert_eq!(w.d.on_session(SessionChange::Ended), Flow::Go);
        setter.join().expect("the setter ends");
        *w.probe.borrow_mut() = None;

        let first = seen
            .borrow()
            .first()
            .cloned()
            .expect("the core got the end");
        assert_eq!(
            first,
            [SESSION_END],
            "the restore was first before the core"
        );
        assert!(w.fed().ends_with(&[Event::Session(SessionChange::Ended)]));
        let queued = w.queue();
        assert_eq!(queued.first(), Some(&SESSION_END));
        assert_eq!(applies(&queued), 0);
        assert!(notices(&queued).is_empty());
    }

    #[test]
    fn session_end_wait_ends_when_the_ack_comes() {
        let w = holding();
        let setter = ack_after(&w.ack, 200);
        let start = Instant::now();
        assert_eq!(w.d.on_session(SessionChange::Ended), Flow::Go);
        let took = start.elapsed();
        setter.join().expect("the setter ends");
        assert!(took >= Duration::from_millis(200), "{took:?}");
        assert!(took < Duration::from_secs(2), "{took:?}");
        let queued = w.queue();
        assert!(
            logs(&queued).iter().any(|l| l.contains("restore ran")),
            "{queued:?}"
        );
    }

    #[test]
    fn session_end_without_an_ack_returns_after_about_4_s() {
        let w = holding();
        let start = Instant::now();
        assert_eq!(w.d.on_session(SessionChange::Ended), Flow::Go);
        let took = start.elapsed();
        assert!(took >= Duration::from_millis(4_000), "{took:?}");
        assert!(took < Duration::from_secs(6), "{took:?}");
        let queued = w.queue();
        assert!(
            logs(&queued).iter().any(|l| l.contains("did not finish")),
            "{queued:?}"
        );
    }

    #[test]
    fn session_end_waits_for_its_own_restore_not_the_one_at_ending() {
        let w = holding();
        w.d.on_session(SessionChange::Ending);
        // The IO thread ran the restore `Ending` gave and set the `Ack`.
        assert!(w.queue().contains(&SESSION_END));
        w.ack.set();
        let setter = ack_after(&w.ack, 200);
        let start = Instant::now();
        w.d.on_session(SessionChange::Ended);
        let took = start.elapsed();
        setter.join().expect("the setter ends");
        assert!(took >= Duration::from_millis(200), "{took:?}");
    }

    #[test]
    fn no_apply_after_the_end_until_end_cancelled() {
        let w = holding();
        let setter = ack_after(&w.ack, 20);
        w.d.on_session(SessionChange::Ended);
        setter.join().expect("the setter ends");
        w.settle();

        // A plan switch wants an apply; neither the core nor the queue lets one out.
        w.post(Event::Plan(Ok(plan(OTHER_PLAN))));
        w.post(Event::SettingsOwner);
        assert_eq!(applies(&w.queue()), 0);

        w.d.on_session(SessionChange::EndCancelled);
        assert!(!w.ack.wait(0), "the Ack is cleared");
        let queued = w.queue();
        assert_eq!(applies(&queued), 1, "{queued:?}");
    }

    #[test]
    fn a_reentered_session_end_restores_and_waits_at_once() {
        let w = ready();
        let during: Rc<RefCell<Vec<String>>> = Rc::default();
        let note = Rc::clone(&during);
        let ack = w.ack.clone();
        let io = Arc::clone(&w.io);
        w.recorder().on_show.push_back(Box::new(move |d| {
            let setter = ack_after(&ack, 200);
            let start = Instant::now();
            let flow = d.on_session(SessionChange::Ended);
            let took = start.elapsed();
            setter.join().expect("the setter ends");
            let front = io.pop(0);
            let core_saw = d
                .fed
                .borrow()
                .contains(&Event::Session(SessionChange::Ended));
            let mut note = note.borrow_mut();
            note.push(format!("{flow:?}"));
            note.push(format!("waited {}", took >= Duration::from_millis(200)));
            note.push(format!("front {}", front == Some(SESSION_END)));
            note.push(format!("core saw it {core_saw}"));
        }));
        w.post(working(1));
        assert_eq!(
            *during.borrow(),
            ["Go", "waited true", "front true", "core saw it false"]
        );
        let fed = w.fed();
        assert_eq!(
            fed.last(),
            Some(&Event::Session(SessionChange::Ended)),
            "fed after the outer batch: {fed:?}"
        );
        let queued = w.queue();
        assert_eq!(applies(&queued), 0, "{queued:?}");
    }
}
