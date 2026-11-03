# Session: lock, unlock, console, end of session, and the lock call

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (the real lock test is `#[ignore]`; `p3-haik-checks` runs
it). Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-session\target`.

## Goal

Fill `session`: decoders the loop window calls for session messages, the session notification registration, the
console check, and `Command::Lock` with a dry-run form and a fake.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 8, 11).
- `docs/architecture.md`: section 3 (`SessionChange`, `Started.on_console`, episode flow "Another user's session"),
  section 4 "Tray threads" (end of session), section 5 "Lock and hibernate", "OS events".
- `docs/research.md`: "Design verify checks" (console notices), "Lid and sleep spike" (lock).
- `spikes/lid-sleep/src/sys.rs` (`session_locked`).

## Deliverables

`crates/wakeshot-windows/src/session.rs`:

- `register(hwnd)`: `WTSRegisterSessionNotification(NOTIFY_FOR_THIS_SESSION)`; drop unregisters.
- `decode(msg, wparam, lparam) -> Option<SessionChange>`: `WM_WTSSESSION_CHANGE` with `WTS_SESSION_LOCK` (Locked),
  `WTS_SESSION_UNLOCK` (Unlocked), `WTS_CONSOLE_DISCONNECT` (ConsoleLeft), `WTS_CONSOLE_CONNECT` (ConsoleBack);
  `WM_QUERYENDSESSION` (Ending); `WM_ENDSESSION` TRUE (Ended), FALSE (EndCancelled). Others: `None`. The window's
  owner (integration or the tray track) calls it; this card writes no window procedure.
- `on_console()`: `WTSGetActiveConsoleSessionId()` equals this process's session.
- `locked_now() -> Option<bool>` (added by the orchestrator, `docs/reviews/p1-close-verify.md` Major 1): this
  session's lock state at start, a port of the spike's `session_locked` (`WTSQuerySessionInformationW` with
  `WTSSessionInfoEx`, `SessionFlags`; feature `Win32_System_RemoteDesktop`, declared). `None` when the query fails.
  Integration feeds `Session(Locked)` right after `Started` when it gives `Some(true)`, so notices wait (Haik's
  answer 4) also for a tray that starts while the session is locked.
- Trait `Locker { lock() -> Result<(), WinError> }` with `Win32Locker` (`LockWorkStation`), `DryRunLocker` (logs) and
  `FakeLocker` (behind `test-support`).

## Acceptance checks

Tests in `session::tests`:

- `decode_session_messages` (every row above, plus unknown codes and other messages: `None`).
- `real_on_console_here` (real: true when run from the desktop; record it). `#[ignore]` (review M9: a CI runner's
  console session is not verified); this card runs it once with `cargo test --locked -p wakeshot-windows -- --ignored
  session::real_on_console_here`, and `p3-real-checks` step 2 runs it again.
- `register_on_hidden_window` (real: a hidden top-level window made by the test; register and drop return no error).
- `real_locked_now_here` (real: `Some(false)` when run from an unlocked desktop; record it). `#[ignore]` like
  `real_on_console_here`; this card runs it once with `--ignored`.
- `fake_and_dry_run_lock_never_lock`.
- `real_lock_locks`: `#[ignore]`; never run by this card (it locks Haik's screen). `p3-haik-checks` runs it.

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows session::`. Record each outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The window and its procedure; the end-of-session restore and its 4 s wait (integration, with `p3-power-io`'s queue);
  when to lock (core).
- Any file but `session.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
