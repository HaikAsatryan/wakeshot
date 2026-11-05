# Phase 6, second part: integration

Planned 2026-11-03 by card `docs/tasks/p6-plan-integration.md` (cross-track review M10). `architecture.md` is the
design; this file holds only what it leaves open for the wiring, the cards, the order and the checklist map. Design
changes are proposals in that card's Result.

## Goal

Wire the five tracks into `wakeshot-tray.exe` (tray and dry-run modes) so the product runs end to end, test it in
layers from fakes to a real install, and run Haik's manual checklist (`architecture.md` section 9) on the test laptop.
`wakeshot.exe` needs no wiring: its dispatch and every command are the sources track's, and it talks to the tray only
through files; integration tests it against a running tray.

## Notes

Only what `architecture.md` leaves open. Each value has a reason and a trigger to change it.

### 1. Where the wiring lives

All wiring code goes in a new module `app` of `wakeshot-windows`; the tray bin only calls
`wakeshot_windows::app::tray_mode::run`. Why: the bins call Windows only through the lib (the tray declares only
`Win32_Foundation`), a bin crate flags every item no `main` calls yet (each card would need `#[expect(dead_code)]`), and
lib code tests with the `test-support` fakes. The lib already declares every `windows` feature the wiring needs
(`Win32_UI_WindowsAndMessaging`, `Win32_System_RemoteDesktop`, `Win32_System_Threading`), so no `Cargo` file and no
`Cargo.lock` change. Change it when the wiring needs a crate only the tray has (`velopack`): that part stays in
`velo.rs`.

| File | Holds | Card |
|---|---|---|
| `crates/wakeshot-windows/src/lib.rs` | one line, `pub mod app;` (a contract file) | `p6-int-skeleton` (run alone) |
| `crates/wakeshot-windows/src/app.rs` | module list; the seams between the integration cards: `Flow`, trait `LoopHandler`, `Ack` | `p6-int-skeleton`; frozen after it |
| `crates/wakeshot-windows/src/app/window.rs` | the loop's hidden top-level window, the message pump, the one timer, `WindowSink` (the real `EventSink`), `ask_quit` | `p6-int-window` |
| `crates/wakeshot-windows/src/app/dispatch.rs` | `Dispatcher`: the core, the clock, which runner gets each command, the tick, the settings mutex retry, end of session, quit; trait `LoopRunners` | `p6-int-dispatch` |
| `crates/wakeshot-windows/src/app/io_thread.rs` | the IO thread loop over `IoQueue`: the executor, file commands, `Log`, `Notify`, `OpenLog` | `p6-int-io-thread` |
| `crates/wakeshot-windows/src/app/startup.rs` | tray startup steps 3 to 7, `Started`, the corrupt journal left by others, the panic hook | `p6-int-startup` |
| `crates/wakeshot-windows/src/app/tray_mode.rs` | the composition root: real objects, thread starts, startup and shutdown order, `RealRunners` | `p6-int-tray-mode`, then `p6-int-update-wiring` |
| `crates/wakeshot-tray/src/main.rs` | the tray and dry-run arm calls `tray_mode::run` | `p6-int-skeleton`, then `p6-int-update-wiring` |
| `crates/wakeshot-tray/src/velo.rs` | the update feed, the restarted version, the local test feed | `p6-velopack-dependency` (track), then `p6-int-update-wiring` |
| `crates/wakeshot-tray/tests/smoke.rs` | one ignored dry-run start | `p6-int-tray-mode` |
| `crates/wakeshot-tray/tests/e2e/mod.rs` | helpers for the process tests | `p6-int-dry-run-e2e`, then `p6-int-real-e2e` |
| `crates/wakeshot-tray/tests/e2e_dry_run.rs` | dry-run process tests | `p6-int-dry-run-e2e` |
| `crates/wakeshot-tray/tests/e2e_real.rs` | real tray on plan copies | `p6-int-real-e2e` |
| `packaging/e2e-update.ps1` | update path and checklist 14 | `p6-int-update-e2e` |

Fixes found by the process tests: each e2e card may fix the `app/*` files and `velo.rs` (their cards are done by
then); `p6-int-dry-run-e2e` may also fix the CLI command files (`args.rs`, `run.rs`, `keep_awake.rs`, `automatic.rs`,
`status.rs`, `restore.rs`, `open_log.rs`; their cards are done by then). A bug in any other track file goes under
"Found outside scope" for the orchestrator to route. Every card may append to `docs/research.md` (additions only).

### 2. The seams in `app.rs` (written by `p6-int-skeleton`)

```rust
pub enum Flow { Go, Quit }

/// Called on the loop thread by `app::window` for every message the loop window owns.
pub trait LoopHandler {
    fn event(&self, event: Event) -> Flow;              // an event posted through `WindowSink`
    fn session(&self, change: SessionChange) -> Flow;   // from `session::decode`, end-of-session messages included;
                                                        // returns only after the end-of-session wait
    fn timer(&self) -> Flow;                            // the one timer fired
    fn next_timer_ms(&self) -> u32;                     // the window re-arms its timer after every call
}

/// A one-shot signal: the IO thread sets it after the end-of-session restore; the loop waits on it.
#[derive(Clone, Debug, Default)]
pub struct Ack(/* Arc<(Mutex<bool>, Condvar)> */);
impl Ack { pub fn new() -> Self; pub fn set(&self); pub fn reset(&self); pub fn wait(&self, timeout_ms: u32) -> bool; }
```

So `p6-int-window` and `p6-int-dispatch` build at the same time against `LoopHandler`, and `p6-int-io-thread` and
`p6-int-dispatch` against `IoQueue` (contract) and `Ack`. Change it only through the orchestrator, like a contract.

### 3. The loop window

- A hidden top-level window, never shown, class `Wakeshot.Loop` (`Wakeshot.Loop.DryRun` in dry run). Not message-only:
  `WM_QUERYENDSESSION` and `WM_ENDSESSION` are broadcast to top-level windows only.
- `WindowSink::post` boxes the event and posts `WM_APP + 1` with the pointer; the window procedure unboxes it. A failed
  post takes the box back and drops it (no leak) and returns the error. When the window closes, events still queued
  are taken out and dropped.
- `WM_CLOSE` (also `taskkill /PID` without `/F`) is handled as `Event::Menu(MenuCommand::Quit)`: restore, delete the
  Keep awake hold, exit. Why: the default handling destroys the window, and a tray whose loop window is gone gets no
  events and restores nothing. It also gives tests a clean quit. Change it when a field report shows a tool closing
  Wakeshot when it should not.
- `ask_quit(dry_run)` finds this session's loop window by class and posts `WM_CLOSE` (tests and tools).
- `WM_WTSSESSION_CHANGE`, `WM_QUERYENDSESSION` and `WM_ENDSESSION` go through `session::decode` (`p3-session`) to
  `LoopHandler::session`. `WM_QUERYENDSESSION` always answers TRUE.
- Tray icon, menu and theme messages never reach it: `TrayIcon` and `OwnerDrawnMenu` own their windows on the same
  thread, and the one pump dispatches to all of them (`05-tray-ux.md` note 2).

### 4. The dispatcher

Which runner gets each command (`Command` order is kept per runner, as `command.rs` promises):

| Command | Runner | Thread | Answer |
|---|---|---|---|
| `Requests`, `Retake` | `RequestApi` (`p3-requests`) | loop | `Done(Requests)` at once |
| `Lock` | `Locker` (`p3-session`) | loop | `Done(Locked)` at once |
| `Tray(view)` | `TrayIcon::show` (`p5-tray-icon`) | loop | none |
| `Play(cue)` | `SoundPlayer::play` (`p5-sounds`) | loop; the sound thread answers | `Done(Sound)` through the sink |
| `Hibernate { .. }` | `run_hibernate` (`p3-hibernate`) on a new thread, one at a time | hibernate | `Done(Hibernate)` through the sink |
| `ApplyUpdate` | the update thread (`update.rs`) | update | `Done(Update)` through the sink |
| `Quit` | ends the loop (`Flow::Quit`); shutdown order below | loop | none |
| `ReadPlan`, `Apply`, `Restore` | `PowerExecutor::run` (`p3-power-io`) | IO | the events it returns |
| `WriteState`, `WriteConfig`, `WriteHold`, `DeleteHold` | `store::write_atomic`, `store::remove` | IO | `Done(Written)` |
| `FinishRestoreRequest(state)` | write `state.json`, then remove `restore-request.json` | IO | `Done(Written { file: RestoreRequest })` |
| `Notify` | `BalloonNotifier` (`p5-notices`) | IO | `Done(Notified)` |
| `Log(text)` | `Log::line(Info, "core", text)` | IO | none |
| `OpenLog` | `shell::open_with_default_app(<log path>)` (`p4-cli-commands`) | IO | none; a failure is logged |

- Answers of loop runners go back into the core at once, in order, before the next command of the same batch runs.
- One timer, set to the nearer of the core's `next_deadline()` and the next 30 s safety tick, at least 10 ms
  (`USER_TIMER_MINIMUM`). At each fire: on a safety tick first the settings mutex retry when not owner (got it:
  `SettingsOwner`); then `InputIdle { ms }` and `Tick`; on a safety tick then `WatchHandle::tick`. Why `InputIdle`
  before every `Tick`: the hibernate countdown's deadline needs a fresh idle reading.
- Re-entry: a runner call that pumps messages (the modal menu, a sent message) can call the handler again. A re-entered
  call puts the event in a queue and returns; the outer call drains the queue before it returns. So the core is never
  called while a call into it is on the stack.
- End of session (`session` with the change `p3-session` maps to `WM_ENDSESSION` TRUE): `IoQueue::session_ending()`
  (the queue puts the restore first and drops queued notices and applies), feed the change to the core, run its
  commands, then wait on the `Ack` up to 4 s and return. `EndCancelled`: `session_resumed()`, `Ack::reset`, feed it.
- Quit: the dispatcher runs every command of the batch, then returns `Flow::Quit`.
- Second start while a tray runs: exit 0, silent (architecture step 4). Change it when a Phase 7 user starts Wakeshot
  from Start and thinks it is broken: then the second start asks the first to show a notice (a new `Notice`).

### 5. The IO thread

`IoThread::start(..)` pops `IoQueue` and runs the commands in the table above; it posts every answer through the
sink and logs a failed post (the next tick recovers). After a `Restore { why: SessionEnd }` it sets the `Ack`.
`IoThread::finish(timeout_ms)` runs what is queued and stops the thread; the tray gives it 10 s (the journal mutex wait
is 10 s, `03-windows-power.md` note 9), then exits anyway: the guard covers what is left.

### 6. Startup and shutdown order (`tray_mode::run`)

1. (main) `velo::startup()`; 2. (main) the mode.
3. `startup`: data folder and mutex names, real or dry run; 4. `Local\Wakeshot.Tray` taken, or exit 0;
5. log roll and open (process `tray`), then the panic hook (log, abort); 6. the startup restore
(`PowerExecutor::startup`), on this thread, before any other thread starts; 7. `Global\Wakeshot.Settings` tried.
8. As built by `p6-int-tray-mode` (its card, Result; why in `tray_mode.rs`'s module doc): `LoopWindow` and
`WindowSink`; `IoQueue`; `OwnerDrawnMenu::new(sink)?` and `TrayIcon::new(kind, Box::new(menu))?`, each with a note
hook; `Arc<BalloonNotifier>` and the balloon click hook (`take_click`, then `shell::open_with_default_app` on a short
thread of its own); the sound player (`Arc<dyn SoundPlayer>`, shared with the hibernate thread);
`session::register(hwnd)` with its retry; `Subscriptions::start(sink)`; `WatchThread::start` with the Claude source
only (`ClaudeSource::in_folder(claude_folder(home, CLAUDE_CONFIG_DIR), SystemZone::current())`) and
`SystemLiveness`, before `Dispatcher::new` so its handle goes into the runners; right after it the update thread
(`p6-int-update-wiring`: tray mode only, installed only); `started` and `locked_at_start`;
`JournalCorrupt` when `Startup.corrupt`; `Dispatcher::new`, which feeds `Started` first, then `Session(Locked)` if
the session was locked at start (`docs/reviews/p1-close-verify.md` Major 1), then `JournalCorrupt`, then `Updated` if
Velopack restarted the tray (`p6-int-update-wiring`); then `IoThread::start`, after `Dispatcher::new`, so the core's
first `Tray` is queued before its first `Notify`; then the pump. Events posted before the pump runs wait in the
window's queue, so the core always sees `Started` first.
9. The session registration is dropped at Quit, before `LoopWindow::run` destroys the window (Microsoft: unregister
before that). After the pump: `IoThread::finish(10 s)`, update thread stop, `WatchHandle::stop`, subscriptions, icon
(`NIM_DELETE`), requests, the settings mutex, in that order; a last log line; the tray mutex; exit 0, or 1 after a
start failure.

### 7. Tests in layers

| Layer | Where | Runs in |
|---|---|---|
| Unit and wiring tests with fakes, the real `Core`, hidden windows | `app/*.rs` tests | plain `cargo test`, CI |
| One dry-run start of the real exe | `tests/smoke.rs`, `#[ignore]` | `p6-int-tray-mode` Part B, screen lane |
| Process tests: real exes, `--dry-run`, a temp data folder (`WAKESHOT_DATA_DIR`, debug builds) | `tests/e2e_dry_run.rs`, `#[ignore]` | `p6-int-dry-run-e2e`, screen lane |
| Real tray on active plan copies | `tests/e2e_real.rs`, `#[ignore]` | `p6-int-real-e2e`, global lane |
| Installed: update path, Setup.exe over an install and uninstall during a hold | `packaging/e2e-update.ps1` | `p6-int-update-e2e`, global lane |
| Haik's checklist | the card's steps | `p6-int-haik-checks`, whole laptop |

- Ignored tests are named `real_e2e_*` and run with their test target as the filter (`--test e2e_dry_run -- --ignored
  --test-threads=1`), so no other track's `real_` test runs with them.
- Process tests start the tray at Medium through `explorer.exe <launch .cmd>`, as the Phase 5 demos do (the `.cmd`
  sets `WAKESHOT_DATA_DIR`), find its pid by the exe's full path in the card's own target folder, and end it with
  `ask_quit` or, for crash tests, by that pid. They build `wakeshot-cli` first: `wakeshot.exe` sits next to the tray
  exe in the same target folder.
- Plan copies for integration are named `Wakeshot int A`, `Wakeshot int B` and `Wakeshot int haik`, names no
  `Wakeshot test` sweep or harness `cleanup` touches (review M7).
- The product GUID's `NotifyIconSettings` entry made by runs from `target\` is left: Windows keeps one entry per GUID
  and points it at the newest path (`docs/research.md`, "`NIF_GUID` and the exe path"). The dry-run entry is exported
  and deleted after each laptop card, as `05-tray-ux.md` note 4 says for demos.

### 8. The update path test seam

velo's feed is `GithubSource` with pre-releases off (`p6-velopack-dependency`), so no test release reaches an
installed test build. In debug builds only, `WAKESHOT_UPDATE_FEED=<folder>` makes `velo.rs` use that local folder as
the feed, the same way `WAKESHOT_DATA_DIR` works in debug builds (`p3-store`). Release builds never read it, so a
shipped exe updates only from GitHub. That velopack has a local folder source: not verified (`p6-int-update-wiring`
checks docs.rs first). Change it when the release workflow needs an end-to-end update test of a release build: then
two real GitHub releases.

### 9. Checks while other cards run

The same rules as every track (`03-windows-power.md` note 2): own files only (`rustfmt --edition 2024 --check
<files>`), `cargo clippy --locked -p <package>` with and without `--all-targets` for both targets, `cargo test --locked
-p <package> -- <filter>`, never `--workspace` while other cards run, never `cargo fmt` without `--check` or any
`--fix`, every save compiles, a failure in another card's file goes in the Log. Each card builds into
`CARGO_TARGET_DIR=<scratchpad>\<card name>\target` and deletes it when done. After `p6-velopack-dependency`, ARM64
builds of the tray need clang (Haik's answer 8: the Build Tools clang component).

## Cards

| Card | Needs Haik | Laptop | Runs after | Status |
|---|---|---|---|---|
| `p6-int-skeleton` | no | no; runs alone (edits a contract module list) | `p1-close-contracts` | done (19:28): whole-workspace checks green, closes wave 1 |
| `p6-int-io-thread` | no | no | `p6-int-skeleton`, `p3-power-io`, `p3-store`, `p3-log`, `p4-cli-commands` | done (committed 01:32): 18 tests (16 of 17 failed with the loop off) |
| `p6-int-window` | no | no (hidden windows only) | `p6-int-skeleton`, `p3-session` | done (00:05): 16 window tests, 314 in the crate |
| `p6-int-dispatch` | no | no | `p6-int-skeleton`, `p2-core-lid`, `p3-power-io` | done (22:18): 21 tests, 22 of 22 mutants caught |
| `p6-int-startup` | no | no | `p6-int-skeleton`, `p3-store`, `p3-log`, `p3-process`, `p3-session`, `p3-journal`, `p3-power-io` | done (00:05): 22 tests; journal read retried once after 500 ms |
| `p6-int-tray-mode` | no | Part B: screen, about 10 min (a dry-run icon) | `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`, `p3-system-events`, `p3-hibernate`, `p3-recovery`, `p4-watch-thread`, `p4-claude-transcript`, `p4-cron`, `p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds` | done (12:12: the taskbar picture with the Sunrise while a script hold runs) |
| `p6-int-update-wiring` | no | no | `p6-int-tray-mode`, `p6-velopack-dependency`, `p6-update-schedule` | done (committed 03:54): 15 `tray_mode` tests; smoke passed; ARM64 tray CI only |
| `p6-int-wiring-fixes` | no | no | `p6-int-tray-mode` Part A, `p6-velopack-dependency`, `p6-int-update-wiring` | done (04:29; 1276 tests; smoke test passed) |
| `p6-int-log-wording` | no | no | `p6-int-wiring-fixes` | done (05:16) |
| `p6-int-dry-run-e2e` | no | screen (dry-run icon, balloons, pictures); agent-session lane (one Claude Code test session) | `p6-int-update-wiring`, `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore`, `p4-live-claude` | done (12:18: four pictures; the old dry-run icon entry is left for Haik's call) |
| `p6-int-real-e2e` | no | global: power settings on active plan copies, RunOnce, a guard | `p6-int-dry-run-e2e`, `p3-real-checks` | done (05:25: 10 of 10 on an active copy; AFTER equals BEFORE; no product fix) |
| `p6-int-update-e2e` | no | global: installs, Run, RunOnce, Path, a user environment value, an active plan copy | `p6-int-real-e2e`, `p6-install-e2e`, `p6-pack-and-release`, `p6-hooks` | done (05:51: 125 checks passed, 1 skipped: the update notice, held while the session was locked) |
| `p6-int-haik-checks` | yes: about 42 min (27, 18 of hands, plus 15 with the dock) | whole laptop: lid, lock, hibernate, switch user, battery; no other agent or cargo command | `p6-int-update-e2e`, `p3-haik-checks`, `p5-haik-check` | done (2026-11-05 ~02:00): blocks 1, 2, 3, 5, 7, 8 pass; 4 safe, with findings; 6 steps 1 and 2 pass, step 3 not shown (lid opened at 4 min 51 s); AFTER equals BEFORE; findings 3 to 11 in the card |
| `p2-6-floor-grace` | no | no | `p6-int-haik-checks` | done (2026-11-05 ~03:20): `policy::floor::decide`, 30 s from the last AC loss; 695 core tests, props 10000 cases; narrow power safety check running |
| `p6-real-install-guard` | no | no (read-only checks) | `p2-6-runonce-keep` | done (2026-11-05 ~03:00): shared check in `store.rs` (test-support), `power_check` and `e2e.ps1` refuse; switch bytes fixed; 579 windows tests pass |
| `p6-notice-app-icon` | Part B: about 5 min, a look at the screen | Part A: test installs (HKCU keys); Part B: screen | `p6-real-install-guard` (both own `packaging/e2e.ps1`) | blocked: Part A done (2026-11-05 ~04:00: hooks register the app id with `DisplayName` and a 48 px `IconUri`; `e2e.ps1` 102 of 102); Part B waits for Haik at the screen |
| `p6-screen-reruns` | yes: about 20 min at the screen, same session as `p6-notice-app-icon` Part B | the plan copy and test install of that Part B; deletes the stale Settings icon key and the 2026-11-04 `%TEMP%` leftovers (Haik's yes) | bundle with `p6-notice-app-icon` Part B | ready |
| `p2-6-sound-overlap-smoke-key` | no | no | `p6-screen-reruns`; Part C after Haik's clang install | ready |

Sizes, one worker's context: skeleton 50K, io-thread 120K, window 100K, dispatch 140K, startup 100K, tray-mode 140K,
update-wiring 90K, dry-run-e2e 130K, real-e2e 130K, update-e2e 120K, haik-checks 70K. About 1.19M tokens in all.
Wave 1 peaked at 2 to 3 times such estimates (`docs/phases/02-06-lighter.md`). No bundles here: every card is 90K to
140K by its estimate. No separate review: the e2e cards and Haik's checks cover the wiring.

Hand-overs (a card edits a file another card owned, only after that card is done): `crates/wakeshot-tray/src/main.rs`
(contracts card, then `p6-int-skeleton`, then `p6-int-update-wiring`); `crates/wakeshot-tray/src/velo.rs`
(`p6-velopack-dependency`, then `p6-int-update-wiring`, then fixes by `p6-int-update-e2e`); `app/tray_mode.rs`
(`p6-int-tray-mode`, then `p6-int-update-wiring`, then fixes by the e2e cards); `tests/e2e/mod.rs`
(`p6-int-dry-run-e2e`, then `p6-int-real-e2e`); the CLI command files (`p4-cli-args`, `p4-cli-run`,
`p4-cli-commands`, `p4-cli-restore`, then fixes by `p6-int-dry-run-e2e`); the `app/*` files (their card, then fixes by
the e2e cards, which run one at a time).

## Order

1. `p6-int-skeleton`, alone, right after `p1-close-contracts` (it needs no track code, so the earlier the better).
2. As their track cards finish, in parallel (four files, no overlap): `p6-int-window` (after `p3-session`, early),
   `p6-int-startup` and `p6-int-io-thread` (after `p3-power-io`), `p6-int-dispatch` (after `p2-core-lid`).
3. `p6-int-tray-mode`: Part A beside anything; Part B in a screen slot.
4. `p6-int-update-wiring` (after `p6-velopack-dependency`).
5. Laptop cards, one at a time: `p6-int-dry-run-e2e` (screen), `p6-int-real-e2e` (global), `p6-int-update-e2e`
   (global, installs).
6. Last: `p6-int-haik-checks`, when Haik is at the laptop and nothing else runs.

Critical path, from today's track plans to the end of integration: 11 cards, about 1.35M tokens:
`p2-policy-holds`, `p2-core-holds`, `p2-core-settings`, `p2-core-lid` (the core chain, 530K), then `p6-int-dispatch`,
`p6-int-tray-mode`, `p6-int-update-wiring`, `p6-int-dry-run-e2e`, `p6-int-real-e2e`, `p6-int-update-e2e`,
`p6-int-haik-checks` (820K). The real pace is set by the laptop slots, one at a time across all tracks (four of
integration plus Haik's session), and by Haik's time. Computed by a script over every card's "Runs after" and size
(`p6-plan-integration`, Log).

## Manual checklist map (architecture section 9)

| # | Check | Covered by |
|---|---|---|
| 1 | Lid closed, on battery, request held, battery sleep Never | `p3-haik-checks` |
| 2 | Lid closed, no request, on battery, past the timeout | `p3-haik-checks` (optional) |
| 3 | Re-take after Start > Sleep, on battery | `p3-haik-checks` |
| 4 | End task on the tray, then its group: the guard restores | `p3-haik-checks` (Task Manager); also `p6-int-real-e2e` (the real tray killed by pid during a hold) |
| 5 | Tray and guard killed, sign out and in: RunOnce restores | `p3-haik-checks` B2 (not shown there); shown by the `p2-6-runonce-keep` B2 redo (2026-11-04) |
| 6 | Sign out during a hold: restored before the session ends | `p3-haik-checks`; also `p6-int-real-e2e` (end-of-session messages sent to the real tray) |
| 7 | Work ends with the lid closed: hibernate after 5 min; not while typing on an external screen | `p6-int-haik-checks` block 5 (sleep; hibernate only as the fallback); the external screen part in block 6 step 3 (dock) |
| 8 | Battery floor with a dock AC drop; dock lid cases, unplugged | `p6-int-haik-checks` block 4 (floor, the lid closed under the floor, the dock's short AC drop) and block 6 steps 1 and 2 (dock; Haik's is always plugged in) |
| 9 | Plan switch during a hold | `p6-int-real-e2e` (two active plan copies; no hands needed) |
| 10 | Unlock after a closed-lid hold: the report notice | `p6-int-haik-checks` block 2 |
| 11 | `Global\` mutex; fast user switching; Remote Desktop | mutex: `p3-real-checks`; lock and unlock notices: `p3-haik-checks`; console left and back: `p6-int-haik-checks` block 3 (Switch user, one account); Remote Desktop: not tested before v1 (Haik 2026-11-03; needs a second PC; Phase 7 if one is at hand) |
| 12 | Sign-in delay and the stored Never | later (no test account) |
| 13 | Theme, high contrast, scales, sounds by ear, toast colours | `p5-haik-check`; in the real exe: `p6-int-dry-run-e2e` (pictures: balloon header, exe icon) and `p6-int-haik-checks` block 1; toast colours: none in v1 (balloons only) |
| 14 | `winget upgrade` and uninstall during a hold | `p6-int-update-e2e` (Setup.exe over the install during a real hold, which is what `winget upgrade` runs; uninstall during a real hold); `winget upgrade` itself after the Phase 8 submission |
| 15 | About 50 apply and restore cycles | `p3-real-checks` |
| 16 | Lid close while holding: chime heard, session locks | `p6-int-haik-checks` block 2 |

## Work log

- 2026-11-03: planned by `p6-plan-integration`: 11 cards, about 1.19M tokens, one contract change (the `app` module,
  run alone as `p6-int-skeleton`). Haik: about 27 minutes plus 15 optional, in one session at the end.
- 2026-11-03 18:58: Haik signed off the plan. His dock with an external screen is always plugged in, so block 6 of
  `p6-int-haik-checks` is in (about 42 minutes in all). Remote Desktop: not tested before v1 (default kept).
  `p6-int-skeleton` runs alone once wave 1 of the track builds ends.
- 2026-11-03 19:28: `p6-int-skeleton` done alone: `app` module and seams, five stub files, the tray-mode call, owner
  and design lines (deliverables 7 to 10). Whole-workspace fmt, 4 clippy runs, tests (core 63, windows 55 plus 3
  ignored, cli 1, tray 1) and the x64 release build green; `Cargo.lock` unchanged. Its note on a re-entered end of
  session went to `p6-int-dispatch` item 5. Wave 2 of the track builds started 19:29.
- 2026-11-03 22:20: `p6-int-dispatch` done and committed. Before `p6-int-window` starts: a contract change, run
  alone (no card building `wakeshot-windows`): `LoopHandler::event`, `session` and `timer` in `app.rs` take `&self`, and
  the window takes `&dyn LoopHandler` (with `&mut self` a re-entered call is undefined behavior; `p6-int-dispatch`'s
  Result). Only the dispatcher's three forwarding methods change. Added for `p6-int-tray-mode`:
  `LoopRunners::lost_events()` on each safety tick, `Dispatcher::into_runners()` for the shutdown order.
- 2026-11-03 23:36: the LoopHandler `&self` contract change is running (worker, Log in `p6-int-dispatch`), alone among cards that build `wakeshot-windows`. Then `p6-int-window` and the other Windows cards start.
- 2026-11-03 23:42: the LoopHandler `&self` contract change is done (276 tests in `wakeshot-windows`, workspace check green; Log in `p6-int-dispatch`). Nested calls only queue and run after the outer batch. Note 2 above and `p6-int-window` deliverable 3 now say `&dyn LoopHandler`. `p6-int-window` and `p6-int-startup` start now; `p6-int-startup` also builds review finding 3 (the startup journal read retry).
- 2026-11-04 00:05: `p6-int-window` and `p6-int-startup` done and committed (the window's research section went into the startup commit). For `p6-int-tray-mode` (in its Log): call `session::register` with `hwnd()` and own the 1702 retry, then `started` and `locked_at_start`; after Quit every post fails with 1400, so the IO thread's answers during `finish` each log a failed post; `try_settings` gives `None` for both a held and a denied mutex and logs neither. Finding 3 core side: `Startup` gains `unread: bool` in `executor.rs`, `started()` copies it (about three lines); goes to the second core fix round with `core.rs:159` (an Automatic toggle writes version 1 over a newer `config.json`).
- 2026-11-04 01:14: `p6-int-io-thread` running (owns `app/io_thread.rs`).
- 2026-11-04 01:32: `p6-int-io-thread` done and committed. For `p6-int-tray-mode`: `IoThread::start(IoParts) ->
  Result<IoThread, WinError>`; `finish(self, timeout_ms) -> bool`; a drop finishes with `FINISH_MS` (10 s); `IoParts`
  holds `queue`, `power: Box<dyn PowerRunner>` (any `PowerExecutor`), `dir`, `notifier`, `open` (pass
  `Box::new(shell::open_with_default_app)`), `log`, `sink`, `ack`. Decided by the worker, for Haik's review (14, all in
  the card; none changes how the laptop behaves with no work going on), among them: `start` returns a `Result`;
  `finish` pushes a `Quit` marker plus a stop flag (the idle thread never wakes on a timer); after the quit, a failed
  post (code 1400) gives one log line in total; a hold id that is not a bare file name is refused. Needs another card
  (not urgent): `log.rs` should add `pub fn log_file(logs_dir: &Path) -> PathBuf`, so `OpenLog` and the CLI's
  `open_log.rs` drop their own copies of "wakeshot.log". Outside scope: `OpenLog` can hold the IO thread up to 10 s if
  the shell hangs, which delays a restore queued behind it (weigh at `p6-int-tray-mode`). The laptop is on battery
  (73% at 01:32): the heavy `p6-pack-and-release` bundle waits for AC; a docs pass (round 2) runs.
- 2026-11-04 01:41: docs pass round 2 done and committed (card `p2-6-docs-pass`, "Round 2"): the folder row, the UI
  stop line in `product.md`, the `timers` and `open_tasks` comments, the run exit codes, `p5-menu-paint`'s out of scope
  line, the Competitors table, free paths only (`product.md` Signing, `launch.md`, `p8-store`), the dead `product.md`
  cites in `claude-code-signals.md`. Left for the next docs pass: `architecture.md` 652, 656, 1017 (now tested by
  `p3-real-checks`), `sim.rs:79` old values, rule 2's `research.md:82-130` cite, the rest of the launch research
  proposals (with `p6-readme`), track note 3.2 of `05-tray-ux.md` (a 256 px frame no longer needs a crate).
- 2026-11-04 02:30 (orchestrator, session `wakeshot-46`): `p6-int-tray-mode` Part A started 02:28 (code and unit
  tests; no smoke run, no picture). Orchestrator decision, for Haik's review: the card's "Runs after" lists the four
  tray cards; their Part A (API, code, tests) is done and committed, their Part B waits for the screen and Haik (the
  session locks when the display turns off, `05-tray-ux.md` 02:20). Their APIs are fixed by Part A, so the wiring
  starts now; a Part B fix that changes an API gets a small follow-up here.
- 2026-11-04 02:56 (orchestrator, session `wakeshot-46`): `p6-int-tray-mode` Part A done and committed: `tray_mode::run`
  wires the whole product, 11 unit tests, clippy clean 8 ways; the wiring itself is first run by the Part B smoke test.
  Note 6 above now gives the built order: the watch thread before `Dispatcher::new`, the IO thread after it (the
  core's first `Notify(FirstRun)` comes before its first `Tray`), the session registration dropped at Quit. Decided by
  the worker, for Haik's review (16 in the card; main ones): that order; the session register retried on each safety
  tick; the tray makes its own settings mutex tries so the log says why it was not taken; OpenLog stays on the IO
  thread (risk weighed); the balloon click opens on a short thread. Found outside scope, for a small fix card: a
  first-run notice is lost for good when the first icon add fails (start at sign-in before the taskbar); a
  `Done(Hibernate)` whose post fails (queue full) leaves `hibernating` set, so no later hibernate starts in that run;
  `SystemZone` errors are never logged once boxed in the Claude source; `log::log_file` (a third copy of
  "wakeshot.log").
- 2026-11-04 03:03 (orchestrator, session `wakeshot-46`): `p6-int-tray-mode` smoke test passed in the locked session
  (02:58, 4.1 s): the dry-run tray at Medium went idle, holding 0.9 s after a hold file was written (`state.json` is
  written at most once a second), idle 1.0 s after it was deleted, quit with code 0, dry-run journal gone. Its log has
  no warning or error; the only failed post is the expected 1400 at Quit. Leftovers removed and checked; no real data
  folder, Run or RunOnce value. The smoke test now prints the tray's log before it removes its temp folder (decided by
  the worker, for Haik's review). Left: the taskbar picture (needs Haik to unlock). Found outside scope, wording only:
  `journal.rs` logs "restored" for the save made before any write; the startup restore logs nothing when there is no
  journal, while the restore at Quit does.
- 2026-11-04 03:07 (orchestrator, session `wakeshot-46`): the flaky `app::window` timer tests fixed and committed (Log in
  `p6-int-window`, "Flaky timer tests fix"): `the_timer_fires_after_next_timer_ms_and_again_after_each_re_arm` and
  `an_event_re_arms_the_timer` failed only in the whole crate binary (3 of 5), where loop threads stall 100 to 400 ms
  while all tests start (cause not verified; not a product bug). A fire may now be up to 50 ms early and 1,000 ms
  late; 20 of 20 whole-crate runs pass. Seen once before the fix: another test failed in 2 of 8 loaded runs (name not
  captured; not seen in the 20 runs after).
- 2026-11-04 03:54 (orchestrator, session `wakeshot-46`): `p6-int-update-wiring` done and committed. `velo::updates()`
  gives the feed only when installed, plus the version Velopack restarted the tray with (`on_restarted`, a
  `OnceLock`); debug builds only: `WAKESHOT_UPDATE_FEED=<folder>` makes the feed a local folder (the release exe holds
  no such string, checked). `tray_mode::run(dry_run, Updates)`: the update thread starts right after the watch thread
  (note 6 updated), `ApplyUpdate` goes to it (or `Err` when none runs), it stops first at shutdown; `Updated` is fed
  after `Started`, `Session(Locked)` and `JournalCorrupt`. Dry run never starts it. x64: fmt, clippy, 15 `tray_mode`
  tests, release build; smoke test passed; ARM64 tray CI only. The release tray is now 3.95 MB (the HTTPS stack).
  Worker notes: the local feed works only for an installed debug tray, so it is `p6-int-update-e2e`'s, not
  `p6-int-dry-run-e2e`'s; an installed tray left running past 5 minutes asks GitHub once (no release yet; what
  `GithubSource` answers then is not verified). 7 worker decisions, for Haik's review, in the card.
- 2026-11-04 04:02 (orchestrator `wakeshot-12`, took over from `wakeshot-46` at 04:00): `p6-int-dry-run-e2e` started
  at 04:01 for its suite only (deliverables 1 and 2) beside `p6-int-wiring-fixes`, which owns `dispatch.rs`,
  `io_thread.rs`, `tray_mode.rs` and the `startup.rs` log lines. The worker starts no tray until the orchestrator
  resumes it, after `p6-int-wiring-fixes` and the `packaging/e2e.ps1` rerun.
- 2026-11-04 04:29: `p6-int-wiring-fixes` done (worker of `wakeshot-46`, relayed): refused notices kept by
  `BalloonNotifier` and shown again at each icon add; the hibernate thread retries its end post (1 s, 60 tries);
  zone errors logged once each; `log::log_file`; the lid lock rules treat an unread journal as may be ours (errs
  toward a lock); journal line "being changed", "startup restore done". x64 fmt, clippy, 1276 tests; ARM64 tray CI
  only. 8 worker decisions for Haik's review in the card. Left for other cards: `open_log.rs` keeps its own
  "wakeshot.log" copy; with an unread journal no countdown starts after a lid close (core lid); the dry-run icon
  clean-up must compare ids before and after (the entry held only `IconSnapshot`). One watch test
  (`a_process_exit_posts_the_session_and_the_hold_as_gone_at_once`) failed once under load, then passed 5 of 5.
- 2026-11-04 04:37: `p6-int-dry-run-e2e` stop 2 started (same worker): the suite run, fixes, the Claude Code test
  session, clean-up. The laptop lane is free (`p6-install-e2e` rerun done). Pictures stay for Haik.
- 2026-11-04 04:39 (orchestrator `wakeshot-12`): `p6-int-real-e2e` started for stop 1 only (`e2e_real.rs` and a read-only
  BEFORE record), beside `p6-int-dry-run-e2e` stop 2 and `p2-6-unread-lid-countdown`. Its run (global lane: plan
  copies made active, RunOnce, a guard) waits until `p6-int-dry-run-e2e` ends; then it is alone on the laptop.
- 2026-11-04 04:51: `p6-int-dry-run-e2e` stop 2 done except the pictures (session locked): 12 of 12 `real_e2e_` tests
  passed before any fix and again (44 s, 36 s), so no app, CLI or tray file changed. A Haiku Claude Code session
  with a 60 s tool call: the dry-run tray went 3 to 4 sessions and back, the orchestrator's 3 left alone. Icon
  entry found by comparing ids, exported, deleted, gone a minute later. Found outside scope (minor, log text):
  `journal.rs` prints keys as a debug list; `watch/thread.rs` logs a false WARN (1400) when a hold file changes at
  a quit (4 of 23 quits), e.g. a quit during Keep awake. Both go to a small log wording card.
- 2026-11-04 04:51 (orchestrator `wakeshot-12`): new card `p6-int-log-wording` (journal key names, the false WARN at
  quit in `watch/thread.rs`, `open_log.rs` on `log::log_file`, the watch test that failed once under load), started
  beside `p2-6-unread-lid-countdown` and `p6-int-real-e2e` stop 1.
- 2026-11-04 05:06: `p6-int-real-e2e` stop 2 started (same worker), alone on the laptop: BEFORE recorded at 04:39
  and 04:59 (Balanced only and active, lid 1/1, sleep 18000/18000, saver DC 30, RunOnce empty); 10 tests. A final
  full run follows once `p2-6-unread-lid-countdown` and `p6-int-log-wording` end.
- 2026-11-04 05:16: `p6-int-log-wording` done: journal log lines use the file's key names (`wrote lidAc, lidDc,
  sleepDc`); a 1400 post at quit logs Info, not WARN; `open_log.rs` on `log::log_file`; the flaky watch exit test
  fixed at its cause (a folder notice right after start read the fake before the test changed it; 5 of 40 failed
  before, 0 of 100 after). x64 clippy, ARM64 windows and cli, 1280 workspace tests. The worker saw one
  `scenarios::props` lid lock over a user's own Do nothing while `p2-6-unread-lid-countdown` was mid-edit; relayed
  to that card.
- 2026-11-04 05:16 (orchestrator `wakeshot-12`): `p6-int-update-e2e` started for stop 1 only (`packaging/e2e-update.ps1`,
  parse checks, the debug build and the three packs; nothing installed). Its run (installs, a plan copy made active)
  waits until `p6-int-real-e2e` ends; then it is alone on the laptop.
- 2026-11-04 05:25: `p6-int-real-e2e` done: suite run 2 (05:18 to 05:20) 10 of 10; run 1 failed only the user-change
  test (a test bug: the core holds notices while the session is locked). `real_e2e_cleanup` first and last; AFTER
  equals BEFORE (Balanced only and active, lid 1/1, sleep 18000/18000, saver DC 30, RunOnce empty). Apply 60 to
  90 ms after the CLI; plan switch settled in 162 to 573 ms; session-end send back in 39 to 56 ms, restored; the
  guard restored 63 to 87 ms after the tray was killed. `e2e/mod.rs` generalized; the dry-run suite still 12 of 12.
  Orchestrator decision: no final record run (the lid card changes only lid handling, which the suite never
  drives). Its dry-run check recreated the dry-run icon keys: left for `p6-int-dry-run-e2e`'s clean-up after its
  pictures.
- 2026-11-04 05:38: `p6-int-update-e2e` stop 1 done (`packaging/e2e-update.ps1` with a `-PrepareOnly` switch; debug
  build and three packs; nothing installed; 8 worker decisions for Haik's review). Stop 2 started, alone on the
  laptop. Expected: the update notice is held while the session is locked, so that check may SKIP.
- 2026-11-04 05:51: `p6-int-update-e2e` done: `packaging/e2e-update.ps1` at Medium (05:39 to 05:48), exit 0, 125 passed,
  1 skipped (the update notice: the core holds notices while the session is locked; `p6-int-haik-checks` or Haik
  sees it unlocked). The first check came 301 s after start and took 0.0.2 by delta; no apply during 120 s of Keep
  awake; after `keep-awake off` the copy was restored, the update applied and the new tray started 1.1 s later,
  `current` kept its path. Setup over the install (checklist 14): the hook restored, no tray after; the `run`
  wrapper died (exit 1), its `ping` went on (ended by pid), as the README says. Uninstall during a hold: all gone,
  the data folder too. AFTER equals BEFORE (script and an elevated read). No Rust fix. The product icon entry now
  points at the removed exe (harmless, note 7); Velopack's own logs grew about 29 KB.
- 2026-11-04 12:18 (orchestrator `wakeshot-12`): the screen slot, part 2 (12:07 to 12:18, built from an export of
  f47391d): `p6-int-tray-mode` done (the Sunrise on the taskbar while a script hold runs; run 1 had the icon in the
  overflow) and `p6-int-dry-run-e2e` done (four pictures; tray quit by `WM_CLOSE`, no mouse). The balloon header's
  small glyph is not the Sunrise, likely a cached PNG from 04:38 (not verified). The dry-run icon entry
  `15983757285637097514` and its two AUMID keys, recreated by an earlier test run, are still in Haik's registry:
  his call whether to delete them (orchestrator default: delete, with a `.reg` export first).
- 2026-11-04 13:37 (orchestrator `wakeshot-12`): Haik's yes (defaults): the dry-run icon entry `15983757285637097514`
  (`NotifyIconSettings`, `AppUserModelId` and `Notifications\Settings` keys, display name "Wakeshot") deleted
  after a `.reg` export to the orchestrator's scratchpad; all three checked gone. Windows' cached
  `%TEMP%\NotifyIconGeneratedAumid_15983757285637097514.png` is left (Windows writes it again). The README keeps
  "Codex CLI and Copilot CLI next". `CLAUDE.md` notes that the Bash tool's shell has the backup and restore
  privileges enabled (checked).
- 2026-11-05 (orchestrator): `p6-int-haik-checks` Part B under way. Block 1 restarted once (Haik missed the first-run
  notice: Quit, `config.json` deleted, started again). Haik reached block 8 and signs out; no agent runs. His block
  results come with his "done". Next: a Part C worker (logs, cleanup, the checklist map's block numbers).
- 2026-11-05 (orchestrator): Haik's "done" with his notes; `p6-int-haik-checks` Part C done (worker ~312K tokens):
  findings 3 to 11 in the card, facts in `research.md` "Integration checklist with Haik". Haik's answer: a 30 s grace
  on battery before the floor step back (yes; `product.md` "Battery floor"). New cards: `p2-6-floor-grace` (findings
  6 and 7 b), `p6-notice-app-icon` (findings 5 and 11); `p6-real-install-guard` promoted from later to ready, with
  finding 8. Orchestrator decisions, for Haik's review: finding 6 docs only (no code keeps the lid key until the
  5-minute check; with the lid closed under the floor Windows' own lid action may sleep the laptop at once, safe);
  finding 7 a not done (no code withdraws the step back balloon when AC comes back: with the grace it needs a drop
  over 30 s and a pointer on the banner; change it when a user reports a stale battery balloon); finding 3 is an OS
  limit (only `ms-settings:taskbar`). Waits for Haik: finding 4 (delete the stale Settings icon key on this laptop),
  finding 9 (rerun block 6 step 3, about 10 min), finding 10 (`%TEMP%` test leftovers).
- 2026-11-05 ~03:20 (orchestrator): `p6-real-install-guard` done (03415b6); worker decisions for Haik's review in its
  card (an installed but quit Wakeshot is only a note for `power_check`; start at login on stops only the sign-in
  checks; `cleanup` still runs over its own leftovers). `p2-6-floor-grace` done (e82389f); worker decisions for Haik's
  review in its card (a tray start under the floor steps back at once; with the lid closed the lock goes out before
  the restore, floor chime only; the 30 s restart at each AC loss and count run time). Running: its narrow power
  safety check, `p6-notice-app-icon` Part A (it also fixes the old switch bytes in `hooks.rs` tests).
- 2026-11-05 ~03:35 (orchestrator): the floor grace's narrow power safety check: 0 blockers, 0 majors, 4 minors
  (`docs/reviews/p2-6-floor-grace.md`). Orchestrator decisions, for Haik's review: minor 1 (AC that flaps under 30 s
  never steps back) takes the lean fix, an AC return restarts the grace only once AC has lasted 30 s (replaces worker
  decision 4); minor 2 takes the code fix, the closed-lid unplug lock keeps its retry and notice during a floor step
  back. All four go to the same worker; no re-check (no blocker or major).
- 2026-11-05 ~04:30 (orchestrator): `p6-notice-app-icon` Part A done (17c8eb4): the install and updated hooks write
  `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` (`DisplayName`, `IconUri` to a 48 px PNG
  shipped beside the tray), uninstall removes it; Velopack gives that id to every start of an installed exe, so no
  tray code; `e2e.ps1` 102 of 102; 8 worker decisions for Haik's review in the card; Part B waits for Haik at the
  screen. `p2-6-floor-grace` fix round done (c0cea9a): 704 core tests, 37 props at 10000 cases three runs, 10 of 10
  hand mutants; worker decision 7 (a lock after the last unplug of a flapping charger stays and fires after 30 s on
  battery) for Haik's review. Left as a known minor, for Haik's call (default: next core round): two sounds overlap
  when a lid close in the unknown journal window meets an unplug under the floor (`p2-6-floor-grace` Found outside
  scope 4). Running: the whole-workspace checks, docs pass round 7.
- 2026-11-05 ~05:10 (orchestrator): docs pass round 7 done (5f1b1db). Pushed 5f1b1db: CI green on all three jobs;
  release dry run 41931334598 built, packed (with the notice PNG) and smoke-tested both CPUs. Nothing runs now; every
  card left needs Haik.
- 2026-11-05 (orchestrator, after a /clear): no `git pull --rebase` (the tree has an uncommitted 09:52 line in
  `06-ship.md`, velopack #1146; asked Haik whose). Checked: clang still missing (vswhere finds none), no tray runs, no
  install folder. `06-ship.md` table row of `p6-real-install-guard` set to done (it said later). Orchestrator plan,
  for Haik's yes: one screen session worker does `p6-notice-app-icon` Part B, plus finding 4 (delete the stale
  Settings icon key, then Haik looks at Settings), finding 10 (`%TEMP%` leftovers), finding 9 (block 6 step 3 rerun)
  and the floor re-check (`p2-6-floor-grace` Found outside scope 3); after it, one small card for the two known
  minors (two sounds overlap; the smoke test skips the notice key) and the ARM64 rerun of `p6-velopack-dependency`
  once clang is in.
- 2026-11-05 (orchestrator): Haik's answers: the 09:52 line is his (committed, 04223b0); yes to both deletions; yes
  to both reruns; the two minors fixed now; his design review is a separate session of his own, on brand and UI
  only, so `p6-media` Part B, the README and Pages finish wait for it. He is installing clang now. New cards
  `p6-screen-reruns` (bundle with `p6-notice-app-icon` Part B) and `p2-6-sound-overlap-smoke-key` (after it).
