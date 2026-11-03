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
8. Loop window and `WindowSink`; `TrayIcon::new(kind, OwnerDrawnMenu::new(sink))`; `BalloonNotifier::new(identity)`
and the balloon click hook (`take_click`, then `shell::open_with_default_app`); the sound player; requests and locker
(real or dry run); `IoQueue`; the executor (real or dry run, `TrayGuard` or `NoGuard`); `IoThread::start`;
`session::register(hwnd)`; `Subscriptions::start(sink)`; `Dispatcher::new`, which feeds `Started` first, then
`Session(Locked)` if the session was locked at start (`startup::locked_at_start`; `docs/reviews/p1-close-verify.md`
Major 1), then `JournalCorrupt` if a corrupt journal was left by another process, then `Updated` if Velopack
restarted the tray;
`WatchThread::start` with the Claude source (home, `SystemZone`) and `SystemLiveness` (Codex and Copilot after v1); the update
thread (tray mode only, installed only); then the pump. Events posted before the pump runs wait in the window's queue,
so the core always sees `Started` first.
9. After the pump: `IoThread::finish(10 s)`, update thread stop, `WatchHandle::stop`, subscriptions, session
registration, icon (`NIM_DELETE`) and requests dropped in that order, a last log line, mutexes released, exit 0.

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
| `p6-int-tray-mode` | no | Part B: screen, about 10 min (a dry-run icon) | `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`, `p3-system-events`, `p3-hibernate`, `p3-recovery`, `p4-watch-thread`, `p4-claude-transcript`, `p4-cron`, `p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds` | in progress (Part A, started 02:28) |
| `p6-int-update-wiring` | no | no | `p6-int-tray-mode`, `p6-velopack-dependency`, `p6-update-schedule` | ready |
| `p6-int-dry-run-e2e` | no | screen (dry-run icon, balloons, pictures); agent-session lane (one Claude Code test session) | `p6-int-update-wiring`, `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore`, `p4-live-claude` | ready |
| `p6-int-real-e2e` | no | global: power settings on active plan copies, RunOnce, a guard | `p6-int-dry-run-e2e`, `p3-real-checks` | ready |
| `p6-int-update-e2e` | no | global: installs, Run, RunOnce, Path, a user environment value, an active plan copy | `p6-int-real-e2e`, `p6-install-e2e`, `p6-pack-and-release`, `p6-hooks` | ready |
| `p6-int-haik-checks` | yes: about 42 min (27, 18 of hands, plus 15 with the dock) | whole laptop: lid, lock, hibernate, switch user, battery; no other agent or cargo command | `p6-int-update-e2e`, `p3-haik-checks`, `p5-haik-check` | ready |

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
| 5 | Tray and guard killed, sign out and in: RunOnce restores | `p3-haik-checks` |
| 6 | Sign out during a hold: restored before the session ends | `p3-haik-checks`; also `p6-int-real-e2e` (end-of-session messages sent to the real tray) |
| 7 | Work ends with the lid closed: hibernate after 5 min; not while typing on an external screen | `p6-int-haik-checks` block 4; the external screen part in block 6 (dock) |
| 8 | Battery floor with a dock AC drop; dock lid cases, unplugged | `p6-int-haik-checks` block 5 (floor, no dock) and block 6 (dock; Haik's is always plugged in) |
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
