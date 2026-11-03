# Integration: the loop window, the message pump and the real event sink

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
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
3. `LoopWindow::run(self, handler: &mut dyn LoopHandler) -> Result<(), WinError>`: the pump
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

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and the class names.
