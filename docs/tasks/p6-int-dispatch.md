# Integration: the dispatcher (the core in the loop)

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens (split trigger: past 100K
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

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-03 20:50, orchestrator, from `p3-system-events`' Result: a lost Lid, Power, Suspending or Resumed post is
  never sent again, since the 30 s tick reads none of these states (decide whether the tick should read lid and
  source); `LoopRunners::input_idle_ms() -> u64` cannot return an error, so the runner must pick a value for a failed
  read (0 is safe). `testing.rs` does not re-export `FakeRequests` or `FakeLocker` (`p3-session`): a contract
  change the orchestrator runs alone, if you need them.
- 2026-11-03 21:06, orchestrator, from `p3-power-io`'s Result: `session_ending()` queues the restore itself, so
  dispatch needs no `push_front` first (one is not doubled).

## Result

What was done, files changed, checks and outcomes, open items. Include `LoopRunners` and `Dispatcher::new` as built.
