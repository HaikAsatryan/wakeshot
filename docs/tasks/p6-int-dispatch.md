# Integration: the dispatcher (the core in the loop)

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens (split trigger: past 100K
with the end-of-session part not started, stop at a clean point and ask the orchestrator for a second card).
Real laptop: no (the real `Core`, a fake clock, fake runners and the real `IoQueue`; no window).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-dispatch\target`.
Runs after: `p6-int-skeleton`, `p2-core-lid` (every handler of the core), `p3-power-io` (`IoQueue`). Parallel with:
`p6-int-io-thread`, `p6-int-window`, `p6-int-startup` and any track card.

## Goal

`Dispatcher`, the `LoopHandler` of the tray: it feeds every event to the core with a fresh `Now`, sends each command to
its runner, keeps the one timer, runs the safety tick and the settings mutex retry, and handles end of session and
quit, so the tray's behavior is the core's, tested here without Windows.

## Read first

- `docs/phases/06-integration.md`: notes 2 and 4 (runner table, timer, tick order, re-entry, end of session, quit).
- `docs/architecture.md` section 3 ("Clocks", "Episode flow"), section 4 ("Tray threads", "Tray startup").
- `crates/wakeshot-core/src/core.rs`, `event.rs`, `command.rs`, `time.rs`; `crates/wakeshot-windows/src/app.rs`,
  `io_queue.rs`; the Results of `p2-core-holds`, `p2-core-settings`, `p2-core-lid` (what each event gives) and
  `p3-power-io` (the queue's end-of-session rule), `p3-session` (which `SessionChange` is the end of the session).

## Deliverables

`crates/wakeshot-windows/src/app/dispatch.rs`, owned whole:

1. Trait `LoopRunners` (implemented by `p6-int-tray-mode` with the real objects, and by a recording fake in the tests
   here): `requests(system, display) -> Result<(), WinError>`, `retake() -> Result<(), WinError>`,
   `lock() -> Result<(), WinError>`, `show_tray(&TrayView)`, `play(Cue)`, `start_hibernate(reason, floor_pct,
   last_resume_mono_ms)`, `apply_update()`, `input_idle_ms() -> u64`, `try_settings_owner() -> bool`, `watch_tick()`.
   Names may change; the set may not shrink. Results that come later (sound, hibernate, update) arrive as events.
2. `Dispatcher::new(core: Core, clock: Box<dyn Clock>, io: Arc<IoQueue>, runners: R, ack: Ack, first: Vec<Event>)`:
   feeds `first` in order (`Started`, then `Session(Locked)` when the session was locked at start, then
   `JournalCorrupt` or `Updated` when `p6-int-tray-mode` passes them) before any other event
   (`docs/reviews/p1-close-verify.md` Major 1). `impl LoopHandler for Dispatcher<R>`.
3. Routing: every `Command` variant to the runner of the phase file's table, by an exhaustive `match` (no wildcard
   arm, so a new variant fails to compile here). Loop runners' answers go back into the core at once, in order.
   `Hibernate` while one runs: one log line (through `Command::Log` on the IO queue), no second thread.
4. Timer and tick (note 4): `next_timer_ms` from `Core::next_deadline` (`Wall` against `now.wall_ms`, `Mono` against
   `now.mono_ms`) and the next 30 s safety tick, at least 10 ms; on each fire `InputIdle`, then `Tick`; on a safety
   tick also the settings retry first and `watch_tick` last.
5. Re-entry (note 4): a depth count; a re-entered call queues its event and returns `Flow::Go`; the outer call drains
   the queue, in order, before it returns. Exception (`p6-int-skeleton` Found outside scope 4; the `LoopHandler::session`
   doc): a re-entered end of session cannot wait for the outer call, or Windows may end the process before the
   restore runs. It pushes the restore itself (`IoQueue::push_front` plus `session_ending`, as the `io_queue.rs` doc
   says), queues the change for the core and still waits on the `Ack`, with a test.
6. End of session (note 4): the end change from `p3-session`: `IoQueue::session_ending()`, feed it, run the commands,
   `Ack::wait(4_000)`, log whether the restore finished in time; `EndCancelled`: `session_resumed()`, `Ack::reset()`,
   feed it. Other session changes are plain events.
7. `Quit`: run the rest of the batch, then return `Flow::Quit`; no event is fed after that.

## Acceptance checks

- Tests first, in `app::dispatch::tests`, with the real `Core`, `FakeClock`, a recording `LoopRunners` and the real
  `IoQueue` (popped by the test):
  - every `Command` variant reaches the runner of the table (one test drives a scenario that yields each; the
    exhaustive match covers the rest at compile time);
  - Keep awake from the menu: `requests(true, false)`, a `WriteHold` on the queue, `show_tray` with the holding icon;
    its `Done(Requests(Ok))` is fed before the next command;
  - lid closed while holding (with the core's journal mirror owning the lid key): `lock` and `play(LidClosedHolding)`;
  - a countdown: after the hold ends with the lid closed, `next_timer_ms` points at the 5 minute idle mark,
    `InputIdle` comes before `Tick`, and `start_hibernate` is called once;
  - safety tick: not owner and `try_settings_owner` true gives `SettingsOwner` and an `Apply` on the queue while
    holding; `watch_tick` once per safety tick, not per deadline;
  - a wall clock jump forward of 2 hours and a suspend gap of mono time: the timer never asks for more than 30 s;
  - re-entry: a runner that calls the handler from inside `show_tray` gets its event fed after the outer batch, in
    order, and the core is never called twice at once;
  - end of session: `session_ending` happens before the core sees the change; a test thread that sets the `Ack` after
    200 ms ends the wait at 200 ms; no `Ack` returns at about 4 s; no `Apply` after it until `EndCancelled`;
  - `Menu(Quit)` gives `requests(false, false)`, the restore and `DeleteHold` on the queue, then `Flow::Quit`;
  - `Started` is always the core's first event, also when `first` holds `Session(Locked)`, `JournalCorrupt` and
    `Updated`; with the real core, `Started`, `Session(Locked)`, `Updated` gives no `Notify` until `Unlocked`.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/dispatch.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`; both with `--target aarch64-pc-windows-msvc`: nothing in the owned file.
- `cargo test --locked -p wakeshot-windows -- app::dispatch::` passes.
- Build folder deleted at the end.

## Out of scope

- The core's rules (a core bug goes under "Found outside scope" with the scenario); the window (`p6-int-window`); the
  IO thread (`p6-int-io-thread`); the real runners (`p6-int-tray-mode`).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. **The seam `LoopHandler` (`app.rs`) takes `&mut self`, so a re-entered call is undefined behavior.** A window that
   calls the handler again from a nested pump makes a second `&mut` to the dispatcher while the outer call's `&mut`
   is live. The Rust Reference ("Behavior considered undefined", checked 2026-11-03): a reference passed to a function
   "is live at least as long as that function call", and a live `&mut T` must point to memory "not read or written by
   any pointer not derived from the reference". This holds even when the inner call only queues. The dispatcher here is
   built for the fix (contract proposal 1 in Result). Urgent: `p6-int-window` is ready and builds against
   `&mut dyn LoopHandler`.
2. `Command::Log`'s doc says the runner writes the line "at info level, area `core`" and that it is "a decision only
   the core knows". The dispatcher's own two lines (a hibernate refused while one runs, the end-of-session wait) go the
   same way, as the card asks, so they show under area `core`. Harmless; a later contract change could add a `loop`
   area.
3. For the power safety review, not a bug: at an `Ended` with no `Ending` before it, the queue holds the front restore
   from `session_ending()` and then the core's own `Restore { why: SessionEnd }` (the core pushes it after the queue's
   one-restore rule ran), so the IO thread runs two; the second finds nothing to do (read from `core/settings.rs`, the
   `Ended` arm; not asserted by a test).

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-03 20:50, orchestrator, from `p3-system-events`' Result: a lost Lid, Power, Suspending or Resumed post is
  never sent again, since the 30 s tick reads none of these states (decide whether the tick should read lid and
  source); `LoopRunners::input_idle_ms() -> u64` cannot return an error, so the runner must pick a value for a failed
  read (0 is safe). `testing.rs` does not re-export `FakeRequests` or `FakeLocker` (`p3-session`): a contract
  change the orchestrator runs alone, if you need them.
- 2026-11-03 21:06, orchestrator, from `p3-power-io`'s Result: `session_ending()` queues the restore itself, so
  dispatch needs no `push_front` first (one is not doubled).
- 2026-11-03 21:49: worker started; reading the seams (`app.rs`, `io_queue.rs`, core) and the Results named above.
- 2026-11-03 22:02: sources read. Design: every entry point takes `&self` over cells (a sound re-entered call; the
  seam's `&mut self` makes two live `&mut`, a contract proposal); a loop runner's answer is fed at once and its
  commands run after the rest of the batch (keeps each runner's order); the `Ack` is reset at `Ended` too, since the
  IO thread also sets it after the `Ending` restore; the safety tick also asks the runners for lost events
  (`p3-system-events` finding 1). Next: the code, then the tests.
- 2026-11-03 22:07: tests first: 18 tests in `app::dispatch::tests` over the API with empty bodies: 18 of 18 fail.
  Next: the bodies.
- 2026-11-03 22:12: bodies written; 17 of 18 passed at once, the wall jump test had my own mistake (the 1 s
  `state.json` wait is the nearer deadline right after a pick; the test now waits it out). Added a 19th test: a failed
  request's idle icon comes after the batch's holding icon. 19 pass; rustfmt and the four clippy runs clean. Next:
  mutants in a scratch copy of the workspace.
- 2026-11-03 22:14: two more tests from listing mutants (an owner from the start never tries the mutex; an `Ended`
  after `Quit` still restores first and waits): 21 pass. Mutant run started: 22 mutants by
  `<scratchpad>\p6-int-dispatch\mutants.py` in a copy of the workspace, results in `mutants-results.txt` beside it.
- 2026-11-03 22:18: mutants 21 of 22 killed; M10 (timer from the deadline alone) survived behind the 30 s clamp. The
  timer test now checks the safety tick wins over a farther deadline; M10 rerun killed (22 of 22). `docs/research.md`
  "Dispatcher checks" added (the Reference's aliasing rule read at doc.rust-lang.org). Final checks pass (Result).
- 2026-11-03 22:19: build folder and scratch files deleted. Status done.

## Result

Status: done 2026-11-03. `crates/wakeshot-windows/src/app/dispatch.rs` is filled: trait `LoopRunners`, `Dispatcher`,
`impl LoopHandler for Dispatcher<R>`, and 21 tests in `app::dispatch::tests` over the real `Core`, `FakeClock`, a
recording `LoopRunners` and the real `IoQueue` (popped by the test). Tests first: 18 written over empty bodies, all 18
failed; 3 more came from the review and the mutant list. Real laptop: never touched. No other file changed except this
card and `docs/research.md` (addition only).

### API as built (for `p6-int-tray-mode`)

```rust
pub trait LoopRunners {
    fn requests(&mut self, system: bool, display: bool) -> Result<(), WinError>;
    fn retake(&mut self) -> Result<(), WinError>;
    fn lock(&mut self) -> Result<(), WinError>;
    fn show_tray(&mut self, view: &TrayView);
    fn play(&mut self, cue: Cue);
    fn start_hibernate(&mut self, reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64>);
    fn apply_update(&mut self);
    fn input_idle_ms(&mut self) -> u64;
    fn try_settings_owner(&mut self) -> bool;
    fn lost_events(&mut self) -> Vec<Event>; // added, design choice 8
    fn watch_tick(&mut self);
}

impl<R: LoopRunners> Dispatcher<R> {
    pub fn new(core: Core, clock: Box<dyn Clock>, io: Arc<IoQueue>, runners: R, ack: Ack, first: Vec<Event>) -> Self;
    pub fn on_event(&self, event: Event) -> Flow;
    pub fn on_session(&self, change: SessionChange) -> Flow;
    pub fn on_timer(&self) -> Flow;
    pub fn timer_ms(&self) -> u32;
    pub fn into_runners(self) -> R; // for the shutdown order after the pump (note 6, step 9)
}
// `LoopHandler::{event, session, timer, next_timer_ms}` forward to the four `&self` methods above.
```

What `RealRunners` must do: `requests`, `retake`, `lock` return the `WinError`; the dispatcher maps it to `Failure`
and feeds `Done(Requests)` or `Done(Locked)` at once. `show_tray` and `play` answer nothing (the sound thread posts
`Done(Sound)`). `start_hibernate` spawns the thread that runs `run_hibernate` and posts `Done(Hibernate)` exactly once;
the dispatcher never calls it again before that event is fed, and a second `Hibernate` meanwhile becomes one log line.
`apply_update` hands over to the update thread (`Done(Update)`). `input_idle_ms`: `clock::input_idle_ms()`, on `Err`
log it and return 0. `try_settings_owner`: try `Global\Wakeshot.Settings`, `true` when held; it is called only while
the dispatcher thinks this tray is not the owner (from `Started.owner`, then the `SettingsOwner` it fed).
`lost_events`: compare `Subscriptions::lost_posts()` with the last value; when it grew, log it and return a fresh
`Lid { closed }` from `lid_once(..)` (when `Some`) and `Power` from `power_status()`; otherwise nothing (whether to add
`Resumed` is that card's call: a spare one retakes the requests and restarts the idle count). `watch_tick`:
`WatchHandle::tick`. `first`: `Started`, plus `Session(Locked)`, `JournalCorrupt`, `Updated` when they apply, in any
order (`new` sorts them into the `event.rs` order). `Dispatcher` is not `Send`: build it on the loop thread.

### How it works

- **Batch.** Each input gets a fresh `Now`. Commands run in the core's order: loop commands on `LoopRunners`, the rest
  pushed on the `IoQueue`, by an exhaustive `match` with no wildcard. A loop runner's answer is fed at once (before the
  next command runs); the commands that answer gives go to the end of the batch, so each runner keeps the core's order
  across calls. Shown by a test: a failed request's idle icon comes after the batch's holding icon, not before it.
- **Timer.** `timer_ms` is the nearer of the core's `next_deadline` (`Wall` against `wall_ms`, `Mono` against
  `mono_ms`) and the next safety tick, clamped to 10..=30,000. The safety tick is kept in run time, due 30 s after the
  last one. At each fire: on a safety tick first `try_settings_owner` (while not owner; `true` feeds `SettingsOwner`)
  and `lost_events`; then `InputIdle { ms }` and `Tick`; on a safety tick then `watch_tick`.
- **Re-entry.** Every entry point takes `&self`; the state sits in `Cell` and `RefCell`. A call while another runs
  (a `busy` flag stands for the depth count, since a re-entered call runs no runner and so never goes deeper) queues
  its input and returns `Flow::Go`; the outer call drains the queue in order before it returns. The core is in a
  `RefCell`, so a second call into it would panic rather than run.
- **End of session.** `Ended` (`WM_ENDSESSION` TRUE): `Ack::reset`, `IoQueue::session_ending()`, feed it, run the
  batch, `Ack::wait(4_000)`, then a log line ("the restore ran in N ms" or "did not finish within 4000 ms").
  Re-entered, or after `Quit`: the same restore and wait at once, and the core gets the change after the outer batch
  (not at all after `Quit`). `EndCancelled`: `session_resumed()`, `Ack::reset()`, then feed it. Other changes are plain
  events.
- **Quit.** `Command::Quit` marks the end; the rest of the batch still runs, answers are no longer fed, queued inputs
  are dropped, every later call returns `Flow::Quit` without feeding, and `timer_ms` gives 10.

### Design choices (mine), each with when to change it

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | All entry points `&self` over cells; the trait forwards | A re-entered call through `&mut self` is undefined behavior (Found outside scope 1) | Never; the seam should follow (proposal 1) |
| 2 | An answer is fed at once, its commands run after the rest of the batch | Fed at once as the card says, while every runner keeps the core's order (the icon tells the truth) | A runner needs an answer's command before the batch's next one |
| 3 | `first` sorted into the `event.rs` order | The core drops everything before `Started` | Never |
| 4 | The `Ack` is reset at `Ended` too, before `session_ending()` | The IO thread sets it after every session end restore, also the one `Ending` gave, so the wait would end at once before the end's own restore (a second try) ran | The IO thread can tell the two restores apart |
| 5 | An `Ended` after `Quit` still restores first and waits | The pump may answer `WM_ENDSESSION` while the IO thread still runs the queue | Never |
| 6 | Safety tick due 30 s of run time after the last one; a fire is a safety tick once run time reached it | A clock change never moves it; a timer that fires a little early costs one more 10 ms fire | A log shows early fires often |
| 7 | The owner flag starts from `Started.owner`; no try while owner | A runner answers `true` for a mutex it holds, so each try would feed `SettingsOwner` and an apply (test added) | Never |
| 8 | `lost_events` on each safety tick, fed after the settings try and before `InputIdle` (the orchestrator's question from `p3-system-events`) | A lost `Lid { closed: true }` while holding means no lock and no countdown; asking only after a loss costs nothing, while reading the lid each tick would register a notification and leak 16 bytes each time (`p3-system-events` decision 2) | Windows offers a cheap lid read, or logs show lost posts often |
| 9 | The dispatcher's own lines go out as `Command::Log` on the IO queue | The card asks for it; one log path | The dispatcher gets its own `Log` (then area `loop`) |
| 10 | An `Event::Session(..)` given to `on_event` is a plain event | Only the window sends session changes, through `on_session` | Another sender appears |

### Contract proposals (the orchestrator runs them alone)

1. `app.rs`: `LoopHandler`'s `event`, `session` and `timer` take `&self` (as `next_timer_ms` already does), and
   `p6-int-window` builds `LoopWindow::run(self, handler: &dyn LoopHandler)`. Why: Found outside scope 1. Cost here:
   the impl's three forwards lose `mut`; nothing else changes. Do it before `p6-int-window` starts.
2. None for `testing.rs`: the recorder fakes the runners, so `FakeRequests` and `FakeLocker` were not needed.

### Checks (2026-11-03, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`, every cargo command `--locked`)

- `cargo test --locked -p wakeshot-windows -- app::dispatch::`: 21 passed, 0 failed, in three runs at 22:18 (each
  about 4.0 s, the no-`Ack` wait). Tests:
  `every_command_reaches_the_runner_of_the_table` (one scenario yields all 19 variants: 7 at runners, 11 on the queue,
  `Quit` as `Flow::Quit`), `keep_awake_from_the_menu_takes_the_requests_and_shows_holding`,
  `a_failed_request_shows_idle_after_the_batchs_holding_icon`, `lid_closed_while_holding_locks_and_chimes`,
  `countdown_hibernates_at_the_idle_mark_with_input_idle_before_each_tick`,
  `safety_tick_takes_the_settings_mutex_and_ticks_the_watch_once`,
  `an_owner_from_the_start_never_tries_the_settings_mutex`, `lost_events_are_fed_on_the_safety_tick_before_input_idle`,
  `the_timer_never_asks_for_more_than_30_s` (2 h forward, 2 h back, a 3 h suspend; the safety tick wins over a farther
  deadline), `reentered_events_are_fed_after_the_outer_batch_in_order`,
  `started_is_always_the_cores_first_event` (shuffled `first`), `started_locked_updated_gives_no_notice_until_unlocked`,
  `quit_from_the_menu_releases_restores_deletes_and_ends`, `a_session_end_after_quit_still_restores_first_and_waits`,
  `a_second_hibernate_while_one_runs_logs_and_starts_nothing`,
  `session_end_puts_the_restore_first_before_the_core_sees_it` (a clock probe reads the queue at the core's clock
  read), `session_end_wait_ends_when_the_ack_comes` (200 ms),
  `session_end_without_an_ack_returns_after_about_4_s`, `session_end_waits_for_its_own_restore_not_the_one_at_ending`,
  `no_apply_after_the_end_until_end_cancelled`, `a_reentered_session_end_restores_and_waits_at_once`.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/dispatch.rs`: exit 0.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0 (22:18). The first
  run had flagged three lints in my test code (two complex types, a slice clone); fixed.
- Mutants, in a scratch copy of the workspace (the repo never mutated), each reverted: 22 of 22 killed. M10 (the
  deadline alone sets the timer) survived the first round, since the 30 s clamp hid it; the timer test now checks that
  the safety tick wins over a farther deadline, and the rerun killed it. The others: an answer's commands before the
  rest of the batch; answers fed after the batch; `first` not sorted; `session_ending` after the core saw `Ended`; no
  `Ack` reset at the end; `session_resumed` after the core saw `EndCancelled`; a re-entered call run at once; the watch
  tick at every fire; `Tick` before `InputIdle`; events fed after `Quit`; no hibernate guard; a re-entered `Ended`
  waiting for the outer call; lost events at every fire; the settings try while owner; the owner from `Started`
  ignored; no 10 ms minimum; re-entered inputs drained last first; an `Ended` after `Quit` not waiting;
  `SettingsOwner` not marking the owner; a `Done(Hibernate)` not ending the guard; the safety tick on the wall clock.
- No line over 120 characters and no em or en dash in the owned file or my lines here. Build folder, mutant copy,
  script and results file deleted (22:19).

### Open items

- Contract proposal 1, before `p6-int-window` starts.
- `p6-int-tray-mode`: the runner notes above, `lost_events` among them.
- Found outside scope 2 and 3 (minor).

## Contract change: LoopHandler takes &self (2026-11-03)

Status: done. Run by the orchestrator alone among the cards that build `wakeshot-windows` (contract proposal 1
above). Owned: `crates/wakeshot-windows/src/app.rs`, `app/dispatch.rs`, the signature in `app/window.rs`. Build folder:
`<scratchpad>\p6-loop-handler-ref\target`.

### Log

- 2026-11-03 23:37: worker started; read this card's Result, the phase file's 22:20 line, `app.rs`, `dispatch.rs`,
  the `window.rs` stub and `p6-int-window`'s card. Only `app.rs` and `dispatch.rs` name `LoopHandler` in code.
- 2026-11-03 23:39: trait and forwards changed, the window's signature written into the stub's doc, one test added;
  rustfmt, both clippy runs, the crate's tests and the workspace check pass.
- 2026-11-03 23:41: build folder deleted. Status done.

### Result

- `app.rs`: `LoopHandler::event`, `session` and `timer` take `&self` (`next_timer_ms` already did). The trait doc
  now says why: a re-entered call through `&mut self` makes a second live `&mut`, so an implementation keeps its
  state in cells and the window holds a `&dyn LoopHandler`.
- `app/dispatch.rs`: the three forwards lose `mut`; no new interior mutability was needed (the `&self` methods and
  their cells were already there). The impl's doc no longer talks about `&mut`. New test
  `the_window_can_call_in_again_through_the_trait`: the outer call goes through a `&dyn LoopHandler`, and a nested
  call from inside `show_tray` calls `event`, `timer`, `session` and `next_timer_ms` through the trait. The nested
  three return `Go` and feed nothing, the core is free, the timer reads 10..=30,000, and after the outer batch the core
  gets `OpenLog`, `InputIdle`, `Tick`, `Session(Locked)` in that order.
- `app/window.rs`: the stub has no code, so the signature went into its module doc:
  `LoopWindow::run(self, handler: &dyn LoopHandler) -> Result<(), WinError>`, with the reason. No window code written.
- Checks (2026-11-03, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`): `rustfmt --edition 2024 --check` on the three files
  exit 0 (no other file rewritten; checked by write time); `cargo clippy -p wakeshot-windows --all-targets --locked
  --target x86_64-pc-windows-msvc -- -D warnings` and the same without `--all-targets`: exit 0;
  `cargo test -p wakeshot-windows --locked`: 276 passed, 0 failed, 17 ignored (`app::`: 27, of them 22 in
  `app::dispatch::`); `cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc`: exit 0 (tray
  and CLI build). No line over 120 characters, no em or en dash. Build folder deleted.
- No other file had to change to compile.
- Stale docs I do not own (for the orchestrator): `docs/tasks/p6-int-window.md` deliverable 3 still says
  `handler: &mut dyn LoopHandler`; `docs/phases/06-integration.md` note 2 (the seam listing, lines 56 to 60) still
  shows `&mut self` on `event`, `session` and `timer`; `docs/tasks/p6-int-skeleton.md` deliverable text too (history,
  harmless).
