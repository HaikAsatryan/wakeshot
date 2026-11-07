//! The update thread: check, download and apply over trait `UpdateFeed`.
//!
//! The first check comes after 5 minutes of run time, then one every 12 hours; `Event::UpdateReady` once per version;
//! on `Command::ApplyUpdate` it starts the updater and posts `Outcome::Update`. No `velopack` here: the tray's
//! `velo.rs` implements the feed, so this logic tests without it.

use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use wakeshot_core::command::{Failure, Outcome};
use wakeshot_core::event::Event;
use wakeshot_core::time::{Clock, Now};
use windows::Win32::Foundation::{ERROR_UNKNOWN_PRODUCT, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::WaitForSingleObject;

use crate::WinError;
use crate::log::{Level, Log};
use crate::sink::EventSink;

#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeFeedState, FakeUpdateFeed, FeedCall};

const AREA: &str = "update";

/// Run time before the first check, so sign-in is not slowed. Run time, so a PC put to sleep right after sign-in does
/// not check the moment it wakes, while its network may still be coming back.
const FIRST_CHECK_MS: u64 = 5 * 60 * 1000;

/// Between checks: 12 hours of wall time or of run time, whichever ends first. Wall time, so a check that fell due
/// while the PC slept runs soon after it wakes; run time, so a clock set back cannot put a check off.
const EVERY_MS: u64 = 12 * 60 * 60 * 1000;

/// The longest single wait. The thread reads the clocks again at least this often, so after a sleep or a clock change
/// a check that is due runs within this much run time.
const SLICE_MS: u64 = 60 * 1000;

/// How long [`UpdateHandle::stop`] waits for the thread. Stop wins every wait, so only a check, download or apply in
/// progress delays it; a thread still busy after this is left to end on its own, so the tray's shutdown never hangs.
const STOP_WAIT_MS: u32 = 2_000;

/// `Outcome::Update(Err)` when the feed says Wakeshot is not installed: no Windows call failed, so `call` names the
/// feed, and the code is Windows' "only valid for products that are currently installed".
const NOT_INSTALLED: Failure = Failure {
    call: "update feed",
    code: ERROR_UNKNOWN_PRODUCT.0,
};

/// Where updates come from. `velo.rs` implements it over Velopack's `UpdateManager`; tests use `FakeUpdateFeed`.
///
/// The thread calls it from its own thread only, one call at a time; a call may block for as long as the network
/// takes. What `velo.rs` must do:
///
/// - `check_and_download`: `check_for_updates`; on `UpdateAvailable`, `download_updates` at once and keep the
///   `UpdateInfo` for `apply`; answer the new version's text (as `Event::UpdateReady` carries it). `NoUpdateAvailable`
///   and an empty feed are `Ok(None)`.
/// - `apply`: `wait_exit_then_apply_updates` with the update the last `check_and_download` kept, silent, restart, no
///   arguments. It returns once `Update.exe` runs and waits for this process (about 0.1 s, research "Velopack checks
///   for Wakeshot", point 4). No update kept: `Failed`.
/// - `velopack::Error::NotInstalled(_)`, from `UpdateManager::new` or any call, is [`FeedError::NotInstalled`]
///   (research "Velopack checks for Wakeshot", point 5); every other error is `Failed`.
pub trait UpdateFeed {
    /// Checks for a newer version and downloads it. `Ok(None)`: none is newer than the running one.
    fn check_and_download(&mut self) -> Result<Option<String>, FeedError>;

    /// Starts the updater, which waits for this process to exit, applies the last downloaded update and starts the
    /// new tray.
    fn apply(&mut self) -> Result<(), FeedError>;
}

/// Why a feed call failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedError {
    /// No install: a build run from `target\`, or a copy without Velopack's files. The thread ends.
    NotInstalled,
    /// Anything else. `failure.call` names the step (in `velo.rs`, the `UpdateManager` method) and `failure.code` is
    /// an OS error when there is one, else 0; `detail` is the feed's own text, for the log line only.
    Failed { failure: Failure, detail: String },
}

/// Starts the update thread.
pub struct UpdateThread;

impl UpdateThread {
    /// Starts the thread; it posts nothing until its first check. `make_feed` runs on the new thread, so a feed that
    /// is not `Send` (Velopack's `UpdateManager` may not be: research "clang for `ring` on ARM64, and the Velopack
    /// update API") still works: pass `velo::feed`. Start it in tray mode only, when Velopack found an install.
    pub fn start<M, F, C>(
        make_feed: M,
        clock: C,
        sink: Arc<dyn EventSink>,
        log: Arc<dyn Log>,
    ) -> Result<UpdateHandle, WinError>
    where
        M: FnOnce() -> F + Send + 'static,
        F: UpdateFeed,
        C: Clock + Send + 'static,
    {
        let (requests, received) = mpsc::channel();
        let thread_log = Arc::clone(&log);
        let thread = std::thread::Builder::new()
            .name("wakeshot-update".to_owned())
            .spawn(move || {
                let mut feed = make_feed();
                let mut wait = ChannelWait(received);
                run(&mut feed, &clock, &mut wait, &*sink, &*thread_log);
            })
            .map_err(|e| WinError {
                call: "CreateThread",
                code: e.raw_os_error().map_or(u32::MAX, |c| c as u32),
            })?;
        Ok(UpdateHandle {
            requests,
            thread: Some(thread),
            log,
        })
    }
}

/// The running thread. Dropping it stops the thread like [`stop`](Self::stop).
pub struct UpdateHandle {
    requests: Sender<Request>,
    thread: Option<JoinHandle<()>>,
    log: Arc<dyn Log>,
}

impl UpdateHandle {
    /// Hands `Command::ApplyUpdate` to the thread, which answers `Event::Done(Outcome::Update(_))` through the sink.
    /// Never blocks. `false`: the thread has ended (not installed, an update already started, or stopped), so no
    /// answer will come and the caller posts `Outcome::Update(Err)` itself.
    pub fn apply(&self) -> bool {
        self.requests.send(Request::Apply).is_ok()
    }

    /// Ends the thread and waits for it, at most 2 s: at once while it waits; a check, download or apply in progress
    /// runs to its end first. `false`: it did not end in time (it is left to end on its own) or it panicked; both are
    /// logged.
    pub fn stop(mut self) -> bool {
        self.end()
    }

    fn end(&mut self) -> bool {
        let Some(thread) = self.thread.take() else {
            return true;
        };
        // A thread that ended on its own dropped the receiver; then the send fails and the wait below returns at once.
        let _ = self.requests.send(Request::Stop);
        // SAFETY: the join handle owns the thread's handle and lives through the call.
        let waited = unsafe { WaitForSingleObject(HANDLE(thread.as_raw_handle()), STOP_WAIT_MS) };
        if waited != WAIT_OBJECT_0 {
            self.log.line(
                Level::Warn,
                AREA,
                "the update thread did not stop within 2 s, so it is left to end on its own",
            );
            return false;
        }
        if thread.join().is_err() {
            self.log
                .line(Level::Error, AREA, "the update thread ended with a panic");
            return false;
        }
        true
    }
}

impl Drop for UpdateHandle {
    fn drop(&mut self) {
        self.end();
    }
}

// The loop owns the handle and may share it; a field that is not `Send` and `Sync` fails here instead.
const _: () = {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<UpdateHandle>();
};

/// What ends a wait early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Apply,
    Stop,
}

/// How the thread waits: the real one on a channel, tests on the fake clock, so no test sleeps.
trait Wait {
    /// Waits up to `ms` for a request; `None` when the time ran out.
    fn wait(&mut self, ms: u64) -> Option<Request>;
}

struct ChannelWait(Receiver<Request>);

impl Wait for ChannelWait {
    fn wait(&mut self, ms: u64) -> Option<Request> {
        match self.0.recv_timeout(Duration::from_millis(ms)) {
            Ok(request) => Some(request),
            Err(RecvTimeoutError::Timeout) => None,
            // The handle is gone, so nobody can ask for anything any more.
            Err(RecvTimeoutError::Disconnected) => Some(Request::Stop),
        }
    }
}

/// Why the thread ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    Stopped,
    NotInstalled,
    /// The updater runs and waits for this process to exit: nothing is left to check or apply.
    Applied,
}

/// The thread body.
fn run<F: UpdateFeed>(
    feed: &mut F,
    clock: &dyn Clock,
    wait: &mut dyn Wait,
    sink: &dyn EventSink,
    log: &dyn Log,
) -> End {
    let mut due = Due::first(clock.now());
    // Versions the core got `UpdateReady` for. A post that failed is not in it, so the next check posts again.
    let mut posted: Vec<String> = Vec::new();
    loop {
        let left = due.left(clock.now());
        if left > 0 {
            match wait.wait(left.min(SLICE_MS)) {
                None => {}
                Some(Request::Stop) => return End::Stopped,
                Some(Request::Apply) => {
                    if let Some(end) = apply(feed, sink, log) {
                        return end;
                    }
                }
            }
            continue;
        }
        match feed.check_and_download() {
            Ok(None) => log.line(Level::Info, AREA, "no update found"),
            Ok(Some(version)) if posted.contains(&version) => log.line(
                Level::Info,
                AREA,
                &format!("update {version} is still ready"),
            ),
            Ok(Some(version)) => {
                log.line(Level::Info, AREA, &format!("update {version} downloaded"));
                let event = Event::UpdateReady {
                    version: version.clone(),
                };
                if post(sink, log, event, "UpdateReady") {
                    posted.push(version);
                }
            }
            Err(FeedError::NotInstalled) => {
                log.line(
                    Level::Info,
                    AREA,
                    "Wakeshot is not installed, so it does not check for updates",
                );
                return End::NotInstalled;
            }
            Err(FeedError::Failed { failure, detail }) => log.line(
                Level::Warn,
                AREA,
                &format!(
                    "update check failed ({}, code {}): {detail}; trying again at the next check",
                    failure.call, failure.code
                ),
            ),
        }
        due = Due::after(clock.now());
    }
}

/// Runs one apply and answers it. `Some`: the thread ends.
fn apply<F: UpdateFeed>(feed: &mut F, sink: &dyn EventSink, log: &dyn Log) -> Option<End> {
    let (result, end) = match feed.apply() {
        Ok(()) => {
            log.line(
                Level::Info,
                AREA,
                "the updater started; it applies the update once Wakeshot quits",
            );
            (Ok(()), Some(End::Applied))
        }
        Err(FeedError::NotInstalled) => {
            log.line(
                Level::Warn,
                AREA,
                "update not applied: Wakeshot is not installed",
            );
            (Err(NOT_INSTALLED), Some(End::NotInstalled))
        }
        Err(FeedError::Failed { failure, detail }) => {
            log.line(
                Level::Warn,
                AREA,
                &format!(
                    "update not applied ({}, code {}): {detail}",
                    failure.call, failure.code
                ),
            );
            (Err(failure), None)
        }
    };
    post(
        sink,
        log,
        Event::Done(Outcome::Update(result)),
        "the update result",
    );
    end
}

/// `false`: the event is lost; logged.
fn post(sink: &dyn EventSink, log: &dyn Log, event: Event, what: &str) -> bool {
    match sink.post(event) {
        Ok(()) => true,
        Err(e) => {
            log.line(
                Level::Warn,
                AREA,
                &format!("{what} was not posted ({}, code {})", e.call, e.code),
            );
            false
        }
    }
}

/// When the next check is due: whichever of the two clocks gets there first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Due {
    mono_ms: u64,
    wall_ms: i64,
}

impl Due {
    /// Run time only (`FIRST_CHECK_MS`).
    fn first(start: Now) -> Self {
        Self {
            mono_ms: start.mono_ms.saturating_add(FIRST_CHECK_MS),
            wall_ms: i64::MAX,
        }
    }

    fn after(check: Now) -> Self {
        Self {
            mono_ms: check.mono_ms.saturating_add(EVERY_MS),
            wall_ms: check.wall_ms.saturating_add_unsigned(EVERY_MS),
        }
    }

    /// Milliseconds left, by the nearer of the two clocks; 0 when the check is due.
    fn left(self, now: Now) -> u64 {
        let by_mono = self.mono_ms.saturating_sub(now.mono_ms);
        // A wall time already past is negative here, which `try_from` turns into 0.
        let by_wall = u64::try_from(self.wall_ms.saturating_sub(now.wall_ms)).unwrap_or(0);
        by_mono.min(by_wall)
    }
}

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    use super::{FeedError, UpdateFeed};

    /// One call the thread made.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FeedCall {
        Check,
        Apply,
    }

    /// What the fake answers and what it was asked.
    #[derive(Debug, Default)]
    pub struct FakeFeedState {
        /// Answers to `check_and_download`, in order; once none is left, `Ok(None)`.
        pub checks: VecDeque<Result<Option<String>, FeedError>>,
        /// Answers to `apply`, in order; once none is left, `Ok(())`.
        pub applies: VecDeque<Result<(), FeedError>>,
        /// Every call, in order.
        pub calls: Vec<FeedCall>,
    }

    /// A feed the test scripts. Clones share one state, so a test keeps a clone to read the calls of the one it
    /// moved into the thread.
    #[derive(Clone, Debug, Default)]
    pub struct FakeUpdateFeed {
        pub state: Arc<Mutex<FakeFeedState>>,
    }

    impl FakeUpdateFeed {
        /// The shared state, also after a caller panicked.
        pub fn state(&self) -> MutexGuard<'_, FakeFeedState> {
            self.state.lock().unwrap_or_else(PoisonError::into_inner)
        }

        pub fn calls(&self) -> Vec<FeedCall> {
            self.state().calls.clone()
        }
    }

    impl UpdateFeed for FakeUpdateFeed {
        fn check_and_download(&mut self) -> Result<Option<String>, FeedError> {
            let mut state = self.state();
            state.calls.push(FeedCall::Check);
            state.checks.pop_front().unwrap_or(Ok(None))
        }

        fn apply(&mut self) -> Result<(), FeedError> {
            let mut state = self.state();
            state.calls.push(FeedCall::Apply);
            state.applies.pop_front().unwrap_or(Ok(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use wakeshot_core::testing::FakeClock;

    use super::*;
    use crate::clock::SystemClock;
    use crate::log::MemoryLog;
    use crate::sink::FakeEventSink;

    const MIN: u64 = 60 * 1000;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;

    /// 2026-11-04 00:00 UTC, and some run time already behind.
    const START: Now = Now {
        wall_ms: 1_793_750_400_000,
        mono_ms: 7 * HOUR,
    };

    /// What happens to the fake PC at a run time (`mono_ms`) during a wait.
    #[derive(Clone, Copy, Debug)]
    enum Step {
        Ask(Request),
        /// The PC sleeps this long: only the wall clock moves.
        Sleep(u64),
        /// Someone moves the wall clock by this much.
        SetWall(i64),
    }

    /// The fake wait: it runs the fake clock instead of sleeping, and plays the script on the way.
    struct FakeWait<'a> {
        clock: &'a FakeClock,
        /// `(mono_ms, step)`, in time order.
        script: Vec<(u64, Step)>,
    }

    impl Wait for FakeWait<'_> {
        fn wait(&mut self, ms: u64) -> Option<Request> {
            assert!((1..=SLICE_MS).contains(&ms), "a wait of {ms} ms");
            assert!(
                !self.script.is_empty(),
                "the script ran out: the thread did not end"
            );
            let end = self.clock.now().mono_ms + ms;
            while let Some(&(at, step)) = self.script.first()
                && at <= end
            {
                self.script.remove(0);
                let now = self.clock.now().mono_ms;
                if at > now {
                    self.clock.run(at - now);
                }
                match step {
                    Step::Ask(request) => return Some(request),
                    Step::Sleep(ms) => self.clock.suspend(ms),
                    Step::SetWall(by) => self.clock.set_wall(self.clock.now().wall_ms + by),
                }
            }
            self.clock.run(end - self.clock.now().mono_ms);
            None
        }
    }

    /// The fake feed, with the time of each call.
    struct Timed<'a> {
        feed: FakeUpdateFeed,
        clock: &'a FakeClock,
        at: Vec<(FeedCall, Now)>,
    }

    impl UpdateFeed for Timed<'_> {
        fn check_and_download(&mut self) -> Result<Option<String>, FeedError> {
            self.at.push((FeedCall::Check, self.clock.now()));
            self.feed.check_and_download()
        }

        fn apply(&mut self) -> Result<(), FeedError> {
            self.at.push((FeedCall::Apply, self.clock.now()));
            self.feed.apply()
        }
    }

    /// Posts like `FakeEventSink`, but the first `fail` posts fail.
    #[derive(Default)]
    struct FlakySink {
        fail: std::sync::Mutex<u32>,
        inner: FakeEventSink,
    }

    impl EventSink for FlakySink {
        fn post(&self, event: Event) -> Result<(), WinError> {
            let mut fail = self.fail.lock().unwrap();
            if *fail > 0 {
                *fail -= 1;
                return Err(WinError {
                    call: "PostMessageW",
                    code: 1816,
                });
            }
            self.inner.post(event)
        }
    }

    struct Rig {
        clock: FakeClock,
        feed: FakeUpdateFeed,
        sink: FlakySink,
        log: MemoryLog,
    }

    struct Ran {
        end: End,
        at: Vec<(FeedCall, Now)>,
        /// The clocks when the thread ended.
        last: Now,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                clock: FakeClock::new(START),
                feed: FakeUpdateFeed::default(),
                sink: FlakySink::default(),
                log: MemoryLog::default(),
            }
        }

        fn checks(self, answers: Vec<Result<Option<String>, FeedError>>) -> Self {
            self.feed.state().checks = answers.into();
            self
        }

        fn applies(self, answers: Vec<Result<(), FeedError>>) -> Self {
            self.feed.state().applies = answers.into();
            self
        }

        /// Runs the thread body; `script` times are run time after `START`.
        fn run(&self, script: &[(u64, Step)]) -> Ran {
            let mut feed = Timed {
                feed: self.feed.clone(),
                clock: &self.clock,
                at: Vec::new(),
            };
            let mut wait = FakeWait {
                clock: &self.clock,
                script: script
                    .iter()
                    .map(|&(at, step)| (START.mono_ms + at, step))
                    .collect(),
            };
            let end = run(&mut feed, &self.clock, &mut wait, &self.sink, &self.log);
            Ran {
                end,
                at: feed.at,
                last: self.clock.now(),
            }
        }

        fn events(&self) -> Vec<Event> {
            self.sink.inner.events()
        }

        fn lines(&self, level: Level) -> Vec<String> {
            self.log
                .lines
                .lock()
                .unwrap()
                .iter()
                .filter(|(l, area, _)| *l == level && *area == AREA)
                .map(|(_, _, text)| text.clone())
                .collect()
        }
    }

    /// Run time after `START` of each call.
    fn times(ran: &Ran) -> Vec<(FeedCall, u64)> {
        ran.at
            .iter()
            .map(|&(call, now)| (call, now.mono_ms - START.mono_ms))
            .collect()
    }

    fn found(version: &str) -> Result<Option<String>, FeedError> {
        Ok(Some(version.to_owned()))
    }

    fn ready(version: &str) -> Event {
        Event::UpdateReady {
            version: version.to_owned(),
        }
    }

    fn failed(call: &'static str, detail: &str) -> FeedError {
        FeedError::Failed {
            failure: Failure { call, code: 0 },
            detail: detail.to_owned(),
        }
    }

    fn stop_at(at: u64) -> (u64, Step) {
        (at, Step::Ask(Request::Stop))
    }

    fn apply_at(at: u64) -> (u64, Step) {
        (at, Step::Ask(Request::Apply))
    }

    const FIRST: u64 = 5 * MIN;

    #[test]
    fn checks_5_minutes_after_start_then_every_12_hours() {
        let rig = Rig::new();
        let ran = rig.run(&[stop_at(FIRST + 36 * HOUR + MIN)]);
        assert_eq!(ran.end, End::Stopped);
        assert_eq!(
            times(&ran),
            [
                (FeedCall::Check, FIRST),
                (FeedCall::Check, FIRST + 12 * HOUR),
                (FeedCall::Check, FIRST + 24 * HOUR),
                (FeedCall::Check, FIRST + 36 * HOUR),
            ]
        );
        assert_eq!(rig.events(), []);
        assert_eq!(rig.lines(Level::Info).len(), 4, "one line per check");
    }

    #[test]
    fn the_first_check_counts_run_time_not_sleep() {
        // Put to sleep 2 minutes after sign-in for an hour: the first check still waits for 5 minutes of run time.
        let rig = Rig::new();
        let ran = rig.run(&[(2 * MIN, Step::Sleep(HOUR)), stop_at(FIRST + MIN)]);
        assert_eq!(times(&ran), [(FeedCall::Check, FIRST)]);
        assert_eq!(ran.at[0].1.wall_ms, START.wall_ms + (FIRST + HOUR) as i64);
    }

    #[test]
    fn a_check_missed_during_sleep_runs_soon_after_the_wake_up() {
        let rig = Rig::new();
        // Mid-wait, so the check waits for that wait to end.
        let asleep = FIRST + HOUR + 30 * 1000;
        let ran = rig.run(&[
            (asleep, Step::Sleep(20 * HOUR)),
            stop_at(asleep + 12 * HOUR + 2 * MIN),
        ]);
        let calls = times(&ran);
        assert_eq!(calls.len(), 3, "{calls:?}");
        // Due by the wall clock since the wake-up: it runs within one wait of it (sleep takes no run time).
        let second = calls[1].1;
        assert!(
            (asleep..=asleep + SLICE_MS).contains(&second),
            "second check {second} ms after start"
        );
        // Then 12 hours from that check, on both clocks alike.
        assert_eq!(calls[2], (FeedCall::Check, second + 12 * HOUR));
    }

    #[test]
    fn a_clock_set_back_cannot_put_the_check_off() {
        let rig = Rig::new();
        let ran = rig.run(&[
            (FIRST + HOUR, Step::SetWall(-2 * DAY as i64)),
            stop_at(FIRST + 12 * HOUR + MIN),
        ]);
        // 12 hours of run time after the first check, though the wall clock says 2 days less.
        assert_eq!(
            times(&ran),
            [
                (FeedCall::Check, FIRST),
                (FeedCall::Check, FIRST + 12 * HOUR)
            ]
        );
    }

    #[test]
    fn a_clock_set_forward_runs_a_check_that_fell_due() {
        let rig = Rig::new();
        let ran = rig.run(&[
            // An hour forward: nothing is due yet.
            (FIRST + HOUR, Step::SetWall(HOUR as i64)),
            // A day forward: the check is due by the wall clock.
            (FIRST + 2 * HOUR, Step::SetWall(DAY as i64)),
            stop_at(FIRST + 3 * HOUR),
        ]);
        let calls = times(&ran);
        assert_eq!(calls.len(), 2, "{calls:?}");
        let second = calls[1].1;
        assert!(
            (FIRST + 2 * HOUR..=FIRST + 2 * HOUR + SLICE_MS).contains(&second),
            "second check {second} ms after start"
        );
    }

    #[test]
    fn one_update_ready_per_version() {
        let rig = Rig::new().checks(vec![
            found("1.2.0"),
            found("1.2.0"),
            found("1.3.0"),
            found("1.3.0"),
            Ok(None),
        ]);
        let ran = rig.run(&[stop_at(FIRST + 48 * HOUR + MIN)]);
        assert_eq!(ran.at.len(), 5);
        assert_eq!(rig.events(), [ready("1.2.0"), ready("1.3.0")]);
    }

    #[test]
    fn a_lost_update_ready_is_posted_at_the_next_check() {
        let rig = Rig::new().checks(vec![found("1.2.0"), found("1.2.0"), found("1.2.0")]);
        *rig.sink.fail.lock().unwrap() = 1;
        rig.run(&[stop_at(FIRST + 24 * HOUR + MIN)]);
        assert_eq!(rig.events(), [ready("1.2.0")]);
        assert_eq!(
            rig.lines(Level::Warn).len(),
            1,
            "{:?}",
            rig.lines(Level::Warn)
        );
    }

    #[test]
    fn a_failed_check_is_logged_and_the_next_one_comes_12_hours_later() {
        let rig = Rig::new().checks(vec![
            Err(failed("check_for_updates", "network unreachable")),
            found("1.2.0"),
        ]);
        let ran = rig.run(&[stop_at(FIRST + 12 * HOUR + MIN)]);
        assert_eq!(
            times(&ran),
            [
                (FeedCall::Check, FIRST),
                (FeedCall::Check, FIRST + 12 * HOUR)
            ]
        );
        assert_eq!(rig.events(), [ready("1.2.0")]);
        let warned = rig.lines(Level::Warn);
        assert_eq!(warned.len(), 1);
        assert!(
            warned[0].contains("check_for_updates") && warned[0].contains("network unreachable"),
            "{warned:?}"
        );
    }

    #[test]
    fn an_applied_update_posts_ok_and_ends_the_thread() {
        let rig = Rig::new().checks(vec![found("1.2.0")]);
        let ran = rig.run(&[apply_at(FIRST + HOUR), stop_at(FIRST + 13 * HOUR)]);
        assert_eq!(ran.end, End::Applied);
        assert_eq!(
            ran.last.mono_ms,
            START.mono_ms + FIRST + HOUR,
            "ended at once"
        );
        assert_eq!(
            times(&ran),
            [(FeedCall::Check, FIRST), (FeedCall::Apply, FIRST + HOUR)]
        );
        assert_eq!(
            rig.events(),
            [ready("1.2.0"), Event::Done(Outcome::Update(Ok(())))]
        );
    }

    #[test]
    fn a_failed_apply_posts_the_failure_and_checks_go_on() {
        let failure = Failure {
            call: "wait_exit_then_apply_updates",
            code: 2,
        };
        let rig = Rig::new()
            .checks(vec![found("1.2.0")])
            .applies(vec![Err(FeedError::Failed {
                failure,
                detail: "Update.exe not found".to_owned(),
            })]);
        let ran = rig.run(&[apply_at(FIRST + HOUR), stop_at(FIRST + 12 * HOUR + MIN)]);
        assert_eq!(ran.end, End::Stopped);
        assert_eq!(
            times(&ran),
            [
                (FeedCall::Check, FIRST),
                (FeedCall::Apply, FIRST + HOUR),
                (FeedCall::Check, FIRST + 12 * HOUR),
            ]
        );
        assert_eq!(
            rig.events(),
            [ready("1.2.0"), Event::Done(Outcome::Update(Err(failure)))]
        );
        assert_eq!(rig.lines(Level::Warn).len(), 1);
    }

    #[test]
    fn not_installed_ends_the_thread_after_one_log_line() {
        let rig = Rig::new().checks(vec![Err(FeedError::NotInstalled)]);
        let ran = rig.run(&[stop_at(5 * DAY)]);
        assert_eq!(ran.end, End::NotInstalled);
        assert_eq!(times(&ran), [(FeedCall::Check, FIRST)]);
        assert_eq!(rig.events(), []);
        assert_eq!(rig.log.lines.lock().unwrap().len(), 1);
    }

    #[test]
    fn not_installed_at_apply_answers_and_ends_the_thread() {
        let rig = Rig::new()
            .checks(vec![found("1.2.0")])
            .applies(vec![Err(FeedError::NotInstalled)]);
        let ran = rig.run(&[apply_at(FIRST + HOUR), stop_at(5 * DAY)]);
        assert_eq!(ran.end, End::NotInstalled);
        assert_eq!(
            rig.events(),
            [
                ready("1.2.0"),
                Event::Done(Outcome::Update(Err(NOT_INSTALLED)))
            ]
        );
    }

    #[test]
    fn stop_while_waiting_ends_at_once() {
        let rig = Rig::new();
        let ran = rig.run(&[stop_at(MIN)]);
        assert_eq!(ran.end, End::Stopped);
        assert_eq!(ran.last.mono_ms, START.mono_ms + MIN);
        assert_eq!(ran.at, []);
        assert_eq!(rig.events(), []);
    }

    #[test]
    fn an_apply_before_any_check_still_answers() {
        // The core sends `ApplyUpdate` only after `UpdateReady`; if one came early, the feed says what it can.
        let rig = Rig::new().applies(vec![Err(failed("apply", "no update downloaded"))]);
        rig.run(&[apply_at(MIN), stop_at(2 * MIN)]);
        assert_eq!(
            rig.events(),
            [Event::Done(Outcome::Update(Err(Failure {
                call: "apply",
                code: 0
            })))]
        );
    }

    // --- The real thread: a channel and the system clock. Nothing here waits for a check (5 minutes away). ---

    /// Hands each event to the test as it comes, so the test waits on a channel, not a sleep.
    struct ChannelSink(Sender<Event>);

    impl EventSink for ChannelSink {
        fn post(&self, event: Event) -> Result<(), WinError> {
            let _ = self.0.send(event);
            Ok(())
        }
    }

    fn start(feed: &FakeUpdateFeed) -> (UpdateHandle, Receiver<Event>) {
        let (events, received) = mpsc::channel();
        let feed = feed.clone();
        let handle = UpdateThread::start(
            move || feed,
            SystemClock,
            Arc::new(ChannelSink(events)),
            Arc::new(MemoryLog::default()),
        )
        .unwrap();
        (handle, received)
    }

    #[test]
    fn the_real_thread_takes_an_apply_and_then_ends() {
        let feed = FakeUpdateFeed::default();
        let (handle, events) = start(&feed);
        assert!(handle.apply());
        assert_eq!(
            events.recv_timeout(Duration::from_secs(10)),
            Ok(Event::Done(Outcome::Update(Ok(()))))
        );
        assert_eq!(feed.calls(), [FeedCall::Apply]);
        // An applied update ends the thread: then `apply` says no answer will come.
        let deadline = Instant::now() + Duration::from_secs(10);
        while handle.apply() {
            assert!(Instant::now() < deadline, "the thread did not end");
            std::thread::yield_now();
        }
        assert!(handle.stop());
        assert_eq!(feed.calls(), [FeedCall::Apply], "nothing after the end");
    }

    #[test]
    fn the_real_thread_answers_a_failed_apply_and_stops_at_once() {
        let feed = FakeUpdateFeed::default();
        let failure = Failure {
            call: "wait_exit_then_apply_updates",
            code: 5,
        };
        feed.state().applies.push_back(Err(FeedError::Failed {
            failure,
            detail: String::new(),
        }));
        let (handle, events) = start(&feed);
        assert!(handle.apply());
        assert_eq!(
            events.recv_timeout(Duration::from_secs(10)),
            Ok(Event::Done(Outcome::Update(Err(failure))))
        );
        let asked = Instant::now();
        assert!(handle.stop());
        assert!(
            asked.elapsed() < Duration::from_secs(1),
            "stop took {:?}",
            asked.elapsed()
        );
        assert_eq!(feed.calls(), [FeedCall::Apply]);
    }

    #[test]
    fn dropping_the_handle_stops_the_thread() {
        let feed = FakeUpdateFeed::default();
        let (handle, events) = start(&feed);
        drop(handle);
        // The thread is gone, so the sink it held was dropped with it and the channel closed.
        assert_eq!(
            events.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        assert_eq!(feed.calls(), []);
    }
}
