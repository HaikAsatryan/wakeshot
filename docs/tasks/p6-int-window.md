# Integration: the loop window, the message pump and the real event sink

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: no (hidden windows only; nothing on screen; end-of-session messages are sent to the test's own window).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-window\target`.
Runs after: `p6-int-skeleton`, `p3-session` (`decode`, `register`). Parallel with: `p6-int-io-thread`,
`p6-int-dispatch`, `p6-int-startup` and any track card.

## Goal

The loop thread's own window and pump: the real `EventSink` that every other thread posts through, the one timer, and
the session and end-of-session messages, all handed to a `LoopHandler`.

## Read first

- `docs/phases/06-integration.md`: notes 2 (the seams) and 3 (the loop window).
- `docs/architecture.md` section 4, "Tray threads" (post to the window, never the thread; the 10,000 message limit;
  `WM_ENDSESSION`); section 5, "OS events" (session and end-of-session rows).
- `crates/wakeshot-windows/src/sink.rs`, `app.rs`; the Result of `p3-session` (`decode`, `register`, which
  `SessionChange` each end-of-session message gives).
- `docs/research.md`: the tray spike's window and pump notes (search "TaskbarCreated", "hidden window").

## Deliverables

`crates/wakeshot-windows/src/app/window.rs`, owned whole:

1. `LoopWindow::new(dry_run: bool) -> Result<LoopWindow, WinError>`: registers class `Wakeshot.Loop` (or
   `Wakeshot.Loop.DryRun`) and creates a hidden top-level window on the calling thread. Not `Send`.
2. `LoopWindow::sink(&self) -> Arc<WindowSink>`; `WindowSink: EventSink` posts `WM_APP + 1` with a boxed `Event`. A
   failed `PostMessageW` takes the box back, drops it and returns the `WinError`. `LoopWindow::hwnd()` for
   `session::register` and the guard of tests.
3. `LoopWindow::run(self, handler: &dyn LoopHandler) -> Result<(), WinError>`: the pump
   (`GetMessageW`, `TranslateMessage`, `DispatchMessageW`, so `TrayIcon`'s and the menu's windows on this thread get
   theirs). The window procedure:
   - `WM_APP + 1`: unbox and call `handler.event`;
   - `WM_TIMER` of its one timer: `handler.timer`;
   - `WM_WTSSESSION_CHANGE`, `WM_QUERYENDSESSION`, `WM_ENDSESSION`: `session::decode`, then `handler.session`;
     `WM_QUERYENDSESSION` answers TRUE;
   - `WM_CLOSE`: `handler.event(Event::Menu(MenuCommand::Quit))`, never the default destroy (phase file note 3);
   - after every handler call: re-arm the timer with `handler.next_timer_ms()` (at least 10 ms);
   - a call that returns `Flow::Quit`: post `WM_QUIT`; `run` then takes every `WM_APP + 1` still queued out and drops
     its box, destroys the window and returns.
4. `ask_quit(dry_run: bool) -> Result<bool, WinError>`: finds this session's loop window by class and posts
   `WM_CLOSE`; false when none runs.
5. No call into the core here: the handler does that.

## Acceptance checks

- Tests first, in `app::window::tests`, each on its own thread with a recording handler:
  - 1,000 events posted from four threads all arrive, and each poster's events arrive in its order;
  - a post after `run` returned fails and does not leak (count live boxes with a test-only counter);
  - the timer fires after about `next_timer_ms` (tolerance 50 ms), and again after each re-arm;
  - `WM_QUERYENDSESSION` answers TRUE; `WM_ENDSESSION` TRUE and FALSE, sent with `SendMessageTimeoutW` from another
    thread, reach `handler.session` with the change `p3-session` decodes, and the sender returns only after the
    handler returned (the 4 s wait is the dispatcher's; here a handler that sleeps 300 ms shows the sender waits);
  - `WM_CLOSE` reaches the handler as `Menu(Quit)` and the window still exists until the handler returns `Quit`;
  - `ask_quit` finds the test window by class (use a test class name so a real tray is never hit) and false when none
    runs;
  - a handler that pumps messages inside `event` (a nested `PeekMessageW` loop) gets the nested events: the window
    itself never drops or reorders them (the queueing rule is the dispatcher's).
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/window.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`; both with `--target aarch64-pc-windows-msvc`: nothing in the owned file.
- `cargo test --locked -p wakeshot-windows -- app::window::` passes. These tests need no screen; record whether they
  pass in CI on the next push (hosted runners have a desktop session: not verified).
- Build folder deleted at the end.

## Out of scope

- What the handler does (`p6-int-dispatch`); `TrayIcon` and the menu windows (tray track); `session.rs`.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. For `p6-int-io-thread` and `tray_mode`: after `Flow::Quit` every `WindowSink::post` fails with `PostMessageW` 1400.
   The IO thread posts its answers while `IoThread::finish` drains the queue after `run` returned, so each of those
   answers will log a failed post at every quit. Harmless, but noisy; treating 1400 after the quit as expected would
   keep the log clean.
2. For `tray_mode` (not this card): the window does not call `session::register`; the composition root calls it
   with `LoopWindow::hwnd()` and owns the retry for error 1702 at early sign-in (the 20:50 Log line).
3. Not verified: an elevated tray and an unelevated `ask_quit` caller. UIPI blocks a post from a lower integrity
   level, so `ask_quit` would then return `Err` with code 5 instead of closing the tray. Agents here run elevated,
   so the test did not hit it.

## Design choices

- **The message carries a number, not a box pointer.** `post` puts the event in the sink's map under a new id and
  posts `WM_APP + 1` with `wparam` = a fixed tag and `lparam` = the id; the window takes the event out by id. A
  failed post takes its event back out of the map. Why: a stray `WM_APP + 1` (another program's broadcast) can never
  make the window read or free a bad pointer, there is no `unsafe` box code, and the map's size is the live-box count
  the tests read. Change it when profiling shows the map's lock matters (one lock per post and per take now).
- **A quit drops the waiting events at once.** On `Flow::Quit` the window closes the sink (later posts fail, every
  waiting event is dropped), kills the timer and posts `WM_QUIT`; `run` then destroys the window, after taking the
  remaining `WM_APP + 1` messages out. Why: `WM_QUIT` comes only when no posted message waits, so a busy poster could
  hold off the end; and no handler call happens after `Quit`. Change it when the dispatcher needs events posted after
  its quit.
- **A `WM_QUIT` the window did not post is handled like `WM_CLOSE`** (`Menu(Quit)` to the handler, then the pump goes
  on until the handler quits). Why: the loop never ends without the handler's quit, so the restore always runs.
  Change it when a component on the loop thread posts `WM_QUIT` for its own nested loop and expects the tray to go
  on.
- **Calls that come before `run`** (startup code that pumps messages) are kept in order and made first when `run`
  starts. Why: dropping or reposting them would lose or reorder events. Change it when startup is known never to pump
  (then the backlog is dead code).
- **No extended window style.** A hidden window has no taskbar button anyway, so `WS_EX_TOOLWINDOW` adds nothing.
  Title "Wakeshot", which is what the shutdown screen would show.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-03 20:50, orchestrator, from `p3-session`'s Result (not verified): `register` may fail with error 1702
  very early at sign-in, and `on_console` reads false while the console is attaching. A retry on the tick would
  cover both.
- 2026-11-03 23:43, worker: started. `LoopHandler` takes `&self`; `run(self, handler: &dyn LoopHandler)`.
- 2026-11-03 23:59, worker: `window.rs` written (window, pump, sink, `ask_quit`) and 16 tests; the libs clippy clean.
  Running the tests next.
- 2026-11-04 00:04, worker: 16 window tests pass, ten runs in a row; rustfmt, clippy on both targets and the crate's
  tests pass; facts in `docs/research.md` "Loop window checks"; build folder deleted. Status done.

## Result

Done 2026-11-04. Files: `crates/wakeshot-windows/src/app/window.rs` (whole), `docs/research.md` (new section "Loop
window checks"), this card. No other file changed; `app.rs` and `dispatch.rs` did not need a change.

Public API as built:

- `LoopWindow::new(dry_run: bool) -> Result<LoopWindow, WinError>`: registers class `Wakeshot.Loop` (dry run:
  `Wakeshot.Loop.DryRun`; a class already registered in the process is fine) and makes a hidden top-level window,
  title "Wakeshot", on the calling thread. Not `Send` (it holds an `Rc` and the `HWND`).
- `LoopWindow::sink(&self) -> Arc<WindowSink>`; `LoopWindow::hwnd(&self) -> HWND`.
- `LoopWindow::run(self, handler: &dyn LoopHandler) -> Result<(), WinError>`: arms the timer with `next_timer_ms`,
  makes the calls that came before `run`, then pumps (`GetMessageW`, `TranslateMessage`, `DispatchMessageW`).
  `WM_APP + 1` goes to `event`; timer id 1 to `timer`; `WM_WTSSESSION_CHANGE`, `WM_QUERYENDSESSION`, `WM_ENDSESSION`
  through `session::decode` to `session` (`WM_QUERYENDSESSION` answers 1); `WM_CLOSE` and a foreign `WM_QUIT` to
  `event(Menu(Quit))`. After every handler call that returns `Go` the timer is re-armed with
  `max(next_timer_ms, 10)`. `Err` only when `GetMessageW` returns -1.
- `WindowSink: EventSink` (`Send + Sync`). After the quit, or after the window went, `post` returns
  `WinError { call: "PostMessageW", code: 1400 }` and drops the event; a full queue gives 1816.
- `ask_quit(dry_run: bool) -> Result<bool, WinError>`: `FindWindowW` by class on this desktop, then posts `WM_CLOSE`.
  `Ok(false)` when no window is found, or it went before the post.

How it works and why: see "Design choices" above (the message carries an id, not a pointer; the quit drops waiting
events at once; a foreign `WM_QUIT` counts as Quit; calls before `run` are kept).

Checks (2026-11-04, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`):

- `cargo test -p wakeshot-windows --locked -- app::window::`: 16 passed, 0 failed, run ten times in a row with no
  failure. The tests: four posters, 1,000 events, each poster's order kept; a post after `run` returned fails with 1400
  and the live count stays 0; a failed `PostMessageW` (a destroyed window) takes its event back; a quit drops 99
  queued events; a full queue fails at the 10,000th post with 1816 and the quit frees the 9,999; stray `WM_APP + 1`
  messages are ignored and take no event; the timer fires at 150, 300, 80 ms (within 50 ms) and after a re-arm by an
  event; `WM_QUERYENDSESSION` answers TRUE, `WM_ENDSESSION` TRUE and FALSE and a lock reach `session` as `Ending`,
  `Ended`, `EndCancelled`, `Locked`, and the sender of `WM_ENDSESSION` waited 300 ms, until the handler returned;
  `WM_CLOSE` reaches the handler as `Menu(Quit)` and the window lives until the handler quits; a foreign `WM_QUIT`
  reaches it as `Menu(Quit)`; `ask_quit` finds the test window by a test class, and false before and after it; a
  nested `PeekMessageW` pump gets the nine nested events in order, at depth 2; calls dispatched before `run` come
  first, in order; a window dropped without `run` drops its events and goes.
- `cargo test -p wakeshot-windows --locked`: 314 passed, 0 failed, 17 ignored.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/window.rs`: exit 0.
- `cargo clippy -p wakeshot-windows --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- No line over 120 characters, no em or en dash, LF line ends. Build folder deleted.

Open items:

- Whether the tests pass in CI on the next push (hosted runners have a desktop session): not verified.
- Whether Windows itself sends `WM_QUERYENDSESSION` and `WM_ENDSESSION` to this hidden window at a real sign-out:
  not verified (the tests send them; an end-to-end check).
- "Found outside scope" 1 to 3.

## Flaky timer tests fix (2026-11-04)

Status: done. Scope: the test module of `crates/wakeshot-windows/src/app/window.rs` and this section only.
Problem: two `app::window` timer tests failed in 2 of 5 `cargo test --workspace --locked` runs under load (names not
recorded). Product code stays as it is.

### Log

- 2026-11-04 02:52, worker: started. Suspects from reading the tests:
  `the_timer_fires_after_next_timer_ms_and_again_after_each_re_arm` (each fire within 50 ms of its arm, both ways) and
  `an_event_re_arms_the_timer` (a fixed 100 ms sleep before the post, then the fire must come no later than 350 ms
  after the first arm).
- 2026-11-04 02:59, worker: before the fix. The test binary filtered to `app::window::` passed 30 of 30 runs without
  load and 30 of 30 beside 32 busy threads (a `pwsh` started and stopped by its PID). The whole crate binary, as
  `cargo test --workspace` runs it, passed 3 of 5 runs with no other load: both suspects failed together in runs 2
  and 3. Run 2: the 150 ms timer fired after 359 ms; the event reached the loop 363 ms after the first arm (100 ms
  sleep), so the re-armed fire came 560 ms after it (bound 350). Run 3: 266 ms, and 464 ms. Each re-armed fire itself
  came on time (195, 197 ms): the stall hit both loop threads at the same moment, while the crate's tests all start
  at once. Cause: the
  tests put tight upper bounds on wall-clock delays of `WM_TIMER`, which comes only when the loop thread runs. Not a
  product bug: nothing in `window.rs` runs between the arm and the fire. Fixed the two tests; rebuilt.
- 2026-11-04 03:06, worker: after the fix. Filtered to `app::window::`: 30 of 30 without load, 30 of 30 beside 32
  busy threads. Whole crate binary: 10 of 10 without load, 10 of 10 beside the busy threads. In 8 more whole-crate
  runs with `--nocapture` beside the busy threads the worst fire was 396 ms late (a 150 ms arm fired after 546 ms,
  and the other test's 200 ms re-arm at the same moment), the earliest 20 ms early (130 ms for 150). Clippy on both
  targets, rustfmt and one `cargo test --locked -p wakeshot-windows app::window` pass. Build folder deleted.

### Result

Done 2026-11-04. Only the test module of `crates/wakeshot-windows/src/app/window.rs` and this section changed.

- Failing tests: `the_timer_fires_after_next_timer_ms_and_again_after_each_re_arm` (a fire later than 50 ms past its
  arm) and `an_event_re_arms_the_timer` (the event came 269 to 363 ms after the first arm, so its fire missed the
  350 ms bound). They fail together, only in the whole crate binary: something in that process stalls every loop
  thread for 100 to 400 ms while all its tests start (what exactly: not verified). Not a product bug.
- Fix: new constants `EARLY_MS = 50` and `LATE_MS = 1_000`, with the reason in their comments. A fire must come in
  `[ms - 50, ms + 1000]`. The first test's plan is now 150, 1,500, 80 ms, so no fixed delay and no arm that kept the
  value before passes all three bounds. The second test arms 5,000 ms first, posts the event as soon as `run` armed
  (no fixed sleep), and checks the fire came 150 to 1,200 ms after the re-arm and before the 5,000 ms arm was due;
  a broken re-arm fails after 5 s instead of hanging.
- Pass counts, before and after: filtered `app::window::` runs 30/30 and 30/30 without load, 30/30 and 30/30 with
  load (the filter alone never reproduced it); whole crate binary 3/5 before, 20/20 after (10 without, 10 with load).
- Checks: rustfmt `--check` exit 0; clippy `--all-targets -D warnings` on `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc` exit 0. LF line ends, no line over 120 characters, no dashes.
- Cost: the `app::window` tests take 1.8 s instead of 0.6 s (the 1,500 ms arm).

Decided by the worker, for Haik's review:

1. A late timer is allowed up to 1 s (was 50 ms); the 50 ms early bound stays. The card's "tolerance 50 ms" now holds
   only for early fires. Why: lateness under load has no real bound, and the wide plan still tells every arm apart.
   Change it when a run shows a fire over 1 s late, or the tests need to be faster.
2. `an_event_re_arms_the_timer` no longer sleeps 100 ms before the event, so it no longer shows the re-arm counts
   from the event rather than from the first arm. `SetTimer` restarts the count on every call, so that case cannot
   happen. Change it when the window stops using one `SetTimer` id.

Found outside scope:

1. In 2 of 8 whole-crate runs with `--nocapture` beside the busy threads (before the fix), one test outside
   `app::window` failed; its name was not captured. It did not come back in the 20 whole-crate runs after the fix.
2. The 100 to 400 ms stall of every window thread in the crate's test binary at start: its cause is not known (not
   verified; perhaps the loader lock while other tests load DLLs). A product loop never shares a process with these
   tests.
