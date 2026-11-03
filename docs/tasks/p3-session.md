# Session: lock, unlock, console, end of session, and the lock call

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (the real lock test is `#[ignore]`; `p3-haik-checks` runs
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

None.

## Found outside scope

- This card's own command `cargo test --locked -p wakeshot-windows -- --ignored session::real_on_console_here` matches
  no test: the path is `session::tests::real_on_console_here`. The run here used the full path with `--exact`.
  `p3-real-checks` step 2 filters by `session::`, which does match.
- `testing.rs` (contracts card) does not re-export `FakeLocker` (nor `p3-requests`' `FakeRequests`); both are
  reachable at their module path with `test-support`.
- The Win32 error to `WinError` mapping now has four private copies (`log.rs`, `store.rs`, `power/requests.rs`,
  `session.rs`). One helper in `error.rs` (contracts card) would serve all.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:40: started by `wakeshot-worker` (bundle with `p3-requests`, done first). Never locks, signs out
  or sleeps; `real_lock_locks` stays `#[ignore]` and is never run here.
- 2026-11-03 20:44: `session.rs` written; `session::` tests: 4 passed, 3 ignored. Read-only real tests run once with
  `--ignored --exact --skip real_lock_locks session::tests::real_on_console_here session::tests::real_locked_now_here`:
  both passed at 20:44:20 (`on_console()` gave `Ok(true)`, `locked_now()` gave `Some(false)`). The card's filter
  `session::real_on_console_here` matches no test (the path is `session::tests::...`), so the full path was used.
- 2026-11-03 20:45: clippy flagged `bool::then` with a closure in `locked_now`; changed to read the union field first
  and `then_some` (same result). Then clippy clean on both targets, with and without `--all-targets`.
- 2026-11-03 20:45: a rerun of the two read-only real tests at 20:45:33: `real_on_console_here` passed,
  `real_locked_now_here` read `Some(true)`. The System log shows why: Kernel-Power 506 "entering Modern Standby,
  Reason: Idle Timeout" at 20:45:29.8 and `LogonUI.exe` started at 20:45:29, so Windows locked the session at screen
  off (Haik away). No test of this card locks: `real_lock_locks` was skipped by name and by `--exact` in every run.
- 2026-11-03 20:48: research note "Session checks" added. Card done; build folder deleted.

## Result

Done 2026-11-03. No lock, sign-out, sleep or power setting change by this card; `real_lock_locks` never ran.

What was built, in `crates/wakeshot-windows/src/session.rs` (the only code file changed):

- `register(hwnd: HWND) -> Result<SessionNotifications, WinError>`: `WTSRegisterSessionNotification(hwnd,
  NOTIFY_FOR_THIS_SESSION)`. Drop unregisters (error dropped); `SessionNotifications::unregister(self) -> Result<(),
  WinError>` does it with the result. Drop it before the window is destroyed (Microsoft docs).
- `decode(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<SessionChange>`: the card's table. `lparam` is not
  read: it is always this session's id, and every end-session flag (log off, Restart Manager close, forced,
  shutdown) means the tray is about to end. A wparam past `u32` and any other code or message: `None`.
- `on_console() -> Result<bool, WinError>`: `WTSGetActiveConsoleSessionId()` equals `ProcessIdToSessionId` of this
  process. `Result` rather than a bare `bool`, so the one failing call is logged. The doc says integration counts
  `Err` as `false` (the safe side: requests only, the user's settings untouched).
- `locked_now() -> Option<bool>`: port of the spike's `session_locked`, with a size check on the returned buffer and
  an unaligned read. `None` when the query fails or the state is unknown.
- `trait Locker { fn lock(&self) -> Result<(), WinError>; }` (`&self`: no state), `Win32Locker` (`LockWorkStation`),
  `DryRunLocker::new(log: Arc<dyn Log>)` (logs `session` Info "dry run: LockWorkStation"), `FakeLocker { pub locks:
  Cell<u32>, pub fail_with: Option<WinError> }` behind `test-support` (counts every call, failed ones too). Runners
  answer `Outcome::Locked(result.map_err(Failure::from))`.

Checks, all run 2026-11-03 with the shared build folder:

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/session.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-windows session::`: 4 passed (`decode_session_messages`,
  `session_flags_read_as_lock_state`, `register_on_hidden_window`, `fake_and_dry_run_lock_never_lock`), 3 ignored
  (`real_on_console_here`, `real_locked_now_here`, `real_lock_locks`).
- `real_on_console_here`: passed at 20:44:20 and 20:45:33 (`Ok(true)`, from Haik's desktop session).
- `real_locked_now_here`: passed at 20:44:20 (`Some(false)`, unlocked). After Windows locked the session itself at
  screen off (20:45:29, see Log) it read `Some(true)` at 20:45:33 and 20:45:40, as it should. The 20:44:20 pass ran
  before the clippy-only change in `locked_now`; `p3-real-checks` step 2 runs it again on the final code.
- `real_lock_locks`: written (`LockWorkStation`, then polls `locked_now()` up to 5 s), never run.

Docs: `docs/research.md` "Session checks" (the message codes, `WM_ENDSESSION` flags, the 0xFFFFFFFF console id, the
lock flags, `LockWorkStation` being asynchronous, the real reads).

Notes for integration (`p6-int-startup`, `p6-int-tray-mode`), not verified here:

- `register` may fail with `RPC_S_INVALID_BINDING` (1702) when called before the Remote Desktop services are ready
  (Microsoft docs). Whether a tray started at sign-in can hit it is unknown; if it does, the tray gets no lock,
  unlock or console changes for that run. A retry on the tick would cover it.
- `on_console()` reads `false` while the console session is being attached or detached (the docs' 0xFFFFFFFF). If a
  tray at sign-in ever sees that, it runs as off the console until a `ConsoleBack` that may never come.

Open items: the three under "Found outside scope". Build folder deleted.
