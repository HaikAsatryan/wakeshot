# When to check for, download and apply an update

Phase: 6. Status: done. Needs Haik: no (Question 3 of `p6-plan-ship` decides only the core's notice, not this card).
Agent: `wakeshot-worker`. Size: about 70K tokens.
Real laptop: no.
Runs after: the contracts card (`update.rs` with its module line; `Event::UpdateReady`, `Command::ApplyUpdate`,
`Outcome::Update`). Parallel with: any card that does not own `crates/wakeshot-windows/src/update.rs`.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-update-schedule\target`.

## Goal

The update thread's logic, with no `velopack` in it: when to check, what to post to the loop, and how to run an apply
the core asked for. `velo.rs` implements the feed later (`p6-velopack-dependency`).

## Read first

- `docs/phases/06-ship.md`: "Updates".
- `docs/architecture.md` section 3 ("Clocks"), section 4 ("Tray threads": other threads post to the loop window,
  never to the thread), section 8 (errors and log).
- `crates/wakeshot-core/src/event.rs`, `command.rs`, `time.rs`; `crates/wakeshot-core/src/testing.rs` (`FakeClock`).
- `docs/research.md`: "Ship track planning checks" (`UpdateManager` API).

## Deliverables

`crates/wakeshot-windows/src/update.rs`, owned whole:

- Trait `UpdateFeed`: one call checks and downloads, giving the new version or none; one call starts the updater that
  waits for this process to exit; an error that means "not installed". Names are this card's to pick; the doc comment
  says what `velo.rs` must do.
- The thread body: first check 5 minutes after start, then every 12 hours of wall time; a due check missed during
  sleep runs on the next wake-up; one `Event::UpdateReady { version }` per version found; on an apply request, call
  the feed and post `Event::Done(Outcome::Update(result))`; a failed check or download is a log line and waits for the
  next check; "not installed" ends the thread after one log line; a stop request ends it at once.
- Clock, wait, post and log are injected, so tests use the fake clock and never sleep for real.

## Acceptance checks

- Tests written first, with a fake feed and the fake clock: the schedule (5 minutes, then 12 hours); a wall clock
  jump forward (sleep) and back (a clock change); one `UpdateReady` per version; a failed check, then a good one; apply
  success and failure posting `Outcome::Update`; "not installed"; stop while waiting.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target <t>
  -- -D warnings` for both targets; `cargo test --locked -p wakeshot-windows update::`. Clean in the owned files; the
  own tests pass.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- `velopack` and `velo.rs`; when the core issues `ApplyUpdate` (core track); starting the thread from the tray
  (integration).

## Questions for Haik

None open. The choices the worker made are under Result, "Decided by the worker, for Haik's review".

## Found outside scope

- `crates/wakeshot-core/src/command.rs` (`Outcome` doc) and `core/update.rs` (`done_update`) say that after
  `Update(Err)` "the next `UpdateReady` tries again". `Event::UpdateReady` comes at most once per version per run
  (`event.rs`), and this thread keeps that rule, so after a failed apply the same version is not offered again until
  a newer version is found or the tray starts again. The doc line is true only in that sense. If a retry of the same
  version in the same run is wanted, the contract in `event.rs` must allow one repeat after `Update(Err)`; then this
  thread would post it again at its next check. Not changed here: the contract is not this card's.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 01:30: started by `wakeshot-worker`, second card of the bundle after `p6-login-and-path` (done).
  Build folder `<scratchpad>\p6-login-update\target`, shared by both cards.
- 2026-11-04 01:37: tests written first in `update.rs` (fake feed, fake clock driven by a scripted fake wait, three
  real-thread tests) with the thread body as `todo!()`: `cargo test --locked -p wakeshot-windows update::` 16 failed
  on `todo!()`, 1 passed (the drop test, trivially: the panicking thread drops its sink too).
- 2026-11-04 01:40: implemented; two of my own asserts were off by one wait (a check right at the wake-up is
  correct: sleep takes no run time) and were fixed, and the sleep test moved mid-wait. 17 of 17 pass, 3 runs in a
  row. Checks done (Result). Status done.
- 2026-11-04 01:42: the bundle's shared build folder deleted; the copies of the registry exports deleted too.

## Result

Done 2026-11-04 01:40.

What was built: `crates/wakeshot-windows/src/update.rs`, the update thread with no `velopack` in it.

API for `p6-velopack-dependency` (`velo.rs`), `p6-int-update-wiring` and `p6-int-tray-mode`:

```rust
pub trait UpdateFeed {
    /// Checks and downloads at once. Ok(None): nothing newer.
    fn check_and_download(&mut self) -> Result<Option<String>, FeedError>;
    /// Starts the updater that waits for this process to exit (the last downloaded update).
    fn apply(&mut self) -> Result<(), FeedError>;
}
pub enum FeedError { NotInstalled, Failed { failure: Failure, detail: String } }

pub struct UpdateThread;
impl UpdateThread {
    pub fn start<M, F, C>(make_feed: M, clock: C, sink: Arc<dyn EventSink>, log: Arc<dyn Log>)
        -> Result<UpdateHandle, WinError>
    where M: FnOnce() -> F + Send + 'static, F: UpdateFeed, C: Clock + Send + 'static;
}
pub struct UpdateHandle; // Send + Sync; dropping it stops the thread
impl UpdateHandle {
    pub fn apply(&self) -> bool; // false: the thread has ended; the caller posts Outcome::Update(Err) itself
    pub fn stop(self) -> bool;   // at once while waiting; waits at most 2 s, else leaves it (logged)
}
// cfg(test or feature "test-support"):
pub struct FakeUpdateFeed { pub state: Arc<Mutex<FakeFeedState>> } // Clone shares the state
pub struct FakeFeedState { pub checks: VecDeque<..>, pub applies: VecDeque<..>, pub calls: Vec<FeedCall> }
pub enum FeedCall { Check, Apply }
```

- `velo.rs`: the trait doc says what to do. `make_feed` runs on the new thread, so pass `velo::feed` itself (the
  `velo::feed() -> impl UpdateFeed` of `06-ship.md` fits as is), and an `UpdateManager` that is not `Send` still
  works. `velopack::Error::NotInstalled(_)` maps to `FeedError::NotInstalled`; `apply` uses the `UpdateInfo` its last
  `check_and_download` kept, with `wait_exit_then_apply_updates(.., true, true, [])`.
- Tray mode: `UpdateThread::start(velo::feed, SystemClock, sink, log)` when installed; `Command::ApplyUpdate` calls
  `handle.apply()`, and on `false` posts `Event::Done(Outcome::Update(Err(_)))` itself; shutdown calls
  `handle.stop()`. The thread posts `UpdateReady` and `Done(Outcome::Update(_))` through the sink, log area `update`.
- Behavior: first check after 5 minutes of run time; then 12 hours of wall time or of run time, whichever ends first;
  the thread reads both clocks at least every 60 s of waiting, so a check that fell due during a sleep runs within
  60 s of the wake-up. `UpdateReady` once per version (a lost post is posted again at the next check). A failed check
  is one warn line; the next check stays 12 hours later. `NotInstalled` ends the thread after one line (at an apply it
  first answers `Update(Err(Failure { call: "update feed", code: 1605 }))`). A successful apply posts `Update(Ok)` and
  ends the thread: the updater now waits for this process.

Checks:

- `cargo test --locked -p wakeshot-windows update::`: 17 passed, 0 failed (3 runs). 14 on the fake clock (the
  schedule; first check counts run time; sleep; clock set back; clock set forward; one `UpdateReady` per version; a
  lost post posted again; a failed check then a good one; apply success; apply failure; not installed at a check and
  at an apply; stop while waiting; an apply before any check) and 3 on the real thread (apply reaches it and the thread
  ends after a success; a failed apply is answered and `stop` takes under 1 s; dropping the handle ends it).
- With card 1: `cargo test --locked -p wakeshot-windows -- install:: update::` 38 passed; real-keys hash unchanged
  around it (`B4D918DF...06955078` before and after, 01:40).
- `rustfmt --edition 2024 --check` on `update.rs` and `install.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo build --release --locked -p wakeshot-windows --target x86_64-pc-windows-msvc`: exit 0.
- No other card's file failed any of these runs.

Decided by the worker, for Haik's review (his rule: with no work going on, the laptop behaves as if Wakeshot were not
there; nothing here keeps the PC awake or wakes it):

1. The first check waits 5 minutes of run time, not wall time: a PC put to sleep right after sign-in does not check
   the moment it wakes, while its network may still be coming back.
2. Between checks, 12 hours by the wall clock or by run time, whichever ends first: sleep cannot delay a check past
   the wake-up, and a clock set back cannot put one off. A clock set forward runs one check early (harmless: two
   GitHub calls instead of one).
3. The thread wakes every 60 s at most to read the clocks. Change it when a power trace shows it.
4. A successful apply ends the thread, so no check or download runs while the updater waits for Wakeshot to quit.
5. After a failed apply, the same version is not offered again in this run (the `UpdateReady` contract; see Found
   outside scope). Change it when a failed apply shows up in a log.
6. `stop` waits at most 2 s, as the watch thread does; a download in progress is left to end with the process.

Needs another card (optional): `crates/wakeshot-windows/src/testing.rs` (contracts card) gathers the fakes; add
`pub use crate::update::FakeUpdateFeed;` there. Until then the fake is `wakeshot_windows::update::FakeUpdateFeed`
under the same feature.

Open items: none for this card.
