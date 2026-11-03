# Integration: tray mode, the composition root

Phase: 6. Status: blocked: waiting for the screen slot (Part A done 2026-11-04 02:55). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
Real laptop: Part A no. Part B: **screen**, about 10 minutes, one dry-run tray icon on the taskbar (one screen or sound
card at a time, across all tracks); no power setting, no sound on purpose, no Haik.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-tray-mode\target`.
Runs after: `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`,
`p3-system-events`, `p3-hibernate`, `p3-recovery`, `p4-watch-thread`, `p4-claude-transcript`, `p4-cron`,
`p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds`. Parallel with: any card that owns none of the files
below. The watch thread starts with the Claude source only: Codex and Copilot moved to after v1 (Haik, 2026-11-03;
`docs/phases/06-integration.md` note 6).

## Goal

`tray_mode::run`: build every real (or dry-run) object, start the threads in the startup order, run the pump, and shut
down in order, so `wakeshot-tray.exe` and `wakeshot-tray.exe --dry-run` run the whole product.

## Read first

- `docs/phases/06-integration.md`: all notes, mainly 4 (runner table) and 6 (startup and shutdown order).
- `docs/architecture.md` section 4 ("Tray threads", "Tray startup"), section 5 ("Lock and hibernate", "Tray, notices,
  sounds").
- `docs/phases/05-tray-ux.md` note 5 (integration notes) and note 4 (demo rules: Medium start, the dry-run GUID, the
  `NotifyIconSettings` clean-up).
- The Results of `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`,
  `p3-session` (`Locker`, `register`), `p3-system-events` (`Subscriptions`), `p3-hibernate` (`run_hibernate`,
  `HibernateDeps`), `p3-recovery` (`TrayGuard`), `p3-journal` (`FileNet`, `NoGuard`), `p3-power-io`, `p3-power-plan`, `p4-watch-thread`
  (`WatchThread::start`), the three source cards and `p4-cron` (`SystemZone`), `p5-tray-icon` (`TrayIcon`,
  `IconKind`), `p5-menu-view`, `p5-notices` (`BalloonNotifier`, `take_click`), `p5-sounds` (`WasapiPlayer`),
  `p4-cli-commands` (`shell`).

## Deliverables

1. `crates/wakeshot-windows/src/app/tray_mode.rs`, owned whole:
   - `run(dry_run: bool) -> ExitCode` in the order of note 6, steps 3 to 9. Real or dry run per object: `Win32Power`
     or `DryRunPower`, `PowerRequests` or `DryRunRequests`, `Win32Locker` or `DryRunLocker`, `Win32Hibernate` or
     `DryRunHibernate`, `TrayGuard` or `NoGuard` (and no RunOnce in dry run), `IconKind::Product` or `DryRun`.
   - `RealRunners: LoopRunners` over those objects, the `TrayIcon`, the `Arc<dyn SoundPlayer>`, the settings lock,
     the `WatchHandle` and the sink; `start_hibernate` spawns one thread that runs `run_hibernate` and posts
     `Done(Hibernate(end))`; `apply_update` answers `Done(Update(Err(..)))` with a log line until
     `p6-int-update-wiring` connects the update thread.
   - The balloon click hook: `take_click()`, then `shell::open_with_default_app` with its target (the log, or
     `ms-settings:taskbar` for the first-run notice). If `BalloonNotifier` cannot be shared with the IO thread (not
     `Sync`), stop and report it as a contract gap.
   - Each start failure that leaves the tray useless (window, IO thread) logs and exits non-zero, after the restore
     steps already done; an icon add failure is `TrayIcon`'s retry, not a failure.
2. `crates/wakeshot-tray/tests/smoke.rs`, owned whole: `real_e2e_smoke_dry_run` (`#[ignore]`): builds paths to the
   tray exe (`CARGO_BIN_EXE_wakeshot-tray`), starts it with `--dry-run` at Medium through `explorer.exe <launch .cmd>`
   with `WAKESHOT_DATA_DIR` on a temp folder, waits for `state.json`, writes a script hold `holds\smoke.json` with the
   test's own pid (`docs/hold-files.md`), checks `state.json` shows holding within 10 s, deletes the hold, checks idle,
   quits with `ask_quit`, and checks the dry-run journal is gone and the process ended with code 0.
3. Part B, in a screen slot: run the smoke test once; one picture of the taskbar with the dry-run icon holding
   (`docs/research/img/ux/int-smoke-*.png`, cropped as `05-tray-ux.md` note 4 says); then export and delete the
   dry-run GUID's `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_` key, and check both are gone a minute
   later.

## Acceptance checks

- Tests first: the smoke test above, written before the body. Unit tests for `RealRunners` parts that need no screen
  (the hibernate thread posts exactly one `Done(Hibernate)` with `DryRunHibernate`; `apply_update` answers `Err`).
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/tray_mode.rs crates/wakeshot-tray/tests/smoke.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and without `--all-targets`;
  `cargo clippy --locked -p wakeshot-tray --all-targets --target <t> -- -D warnings` and without; both targets (the
  tray's ARM64 runs need clang once `velopack` is in).
- `cargo test --locked -p wakeshot-windows -- app::tray_mode::` passes.
- Part B: `cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored
  --test-threads=1` passes; the picture exists; the dry-run entries are gone a minute later; no process this card
  started is left (by pid).
- Build folder deleted at the end.

## Out of scope

- The update thread and `velo.rs` (`p6-int-update-wiring`); the process test suite (`p6-int-dry-run-e2e`); any real
  power setting; any track file (a bug there goes under "Found outside scope").

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. Core order (`core.rs` `take` and `settle`): in the `Started` batch `Notify(FirstRun)` comes before the first
   `Tray`. `tray_mode` works around it (the IO thread starts after `Dispatcher::new`, so the icon is added first). Left:
   when that first add fails (no taskbar yet, such as a start at sign-in), the first-run notice fails and
   `BalloonNotifier` never retries it, so the pin hint is lost for good. Minor; core or `p5-notices`.
2. `app/dispatch.rs`: a `Done(Hibernate)` whose post fails (queue full, 1816) leaves the dispatcher's `hibernating`
   flag set for the rest of the run, so no later hibernate starts, the battery floor one included. `tray_mode` logs the
   lost post. Minor; a retry of the post on the hibernate thread, or a clear of the flag on `Resumed`, would close it.
3. `localtime::SystemZone` keeps its first error for `take_error`, but once it is boxed into `ClaudeSource` nobody can
   call `take_error`, so a failing zone read is never logged (cron times then use the offset read last). Minor;
   `p4-cron` or the Claude source.
4. `startup::try_settings` hides why the mutex was not taken (held or denied); `tray_mode` therefore makes its own tries
   with `NamedMutex`. If `try_settings` ever returns the reason, `tray_mode` can switch back to it.
5. `docs/phases/06-integration.md` note 6 (read only for me) lists `IoThread::start` before `session::register` and
   `Dispatcher::new`, `WatchThread::start` after it, and the session registration in step 9. The built order differs
   for the three reasons in `tray_mode.rs`'s module doc (Result, choices 1 to 3); the note should follow.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-04 00:05, orchestrator, from the Results of `p6-int-window` and `p6-int-startup`: the window does not call
  `session::register`; this card calls it with `hwnd()` and owns the retry for error 1702, then calls `started` and
  `locked_at_start`. After the handler returns Quit, the window drops waiting events and every later post fails with
  `PostMessageW` 1400, so the IO thread's answers during `finish` each log a failed post (keep that quiet or expected).
  `try_settings` gives `None` both when another session holds the mutex and when the open is denied, and logs
  neither: log which one it was. From `p6-int-dispatch`: call `LoopRunners::lost_events()` on each safety tick and use
  `Dispatcher::into_runners()` for the shutdown order. `LoopHandler` methods take `&self` (contract change 23:41).
- 2026-11-04 02:28, worker (session wakeshot-46): Part A started (deliverables 1 and 2, no `--ignored` run, no
  picture). Read the card, CLAUDE.md, plan "How we work", integration notes 1 to 9, architecture 4 and 5, tray UX
  notes 2, 3, 4, 5.
- 2026-11-04 02:39: read the Results named and the code of every API used. Found: the core gives `Notify(FirstRun)`
  before the first `Tray` in the `Started` batch (`core.rs` `take`, then `settle` ends with `view::show`), so an IO
  thread already running could show the notice before the icon exists; plan: start the IO thread right after
  `Dispatcher::new`. `LoopWindow::run` destroys the window before it returns, so the session unregister must come
  from a handler wrapper at Quit. `event.rs` and `startup.rs` changed while I read (the `journal_unread` contract card
  runs); `started()` keeps its signature. Writing `tests/smoke.rs` first.
- 2026-11-04 02:45: `tests/smoke.rs` written (before the body), then `tray_mode.rs` with its unit tests in the same
  pass (the unit tests were not run against an empty body first). Clippy x64 `--all-targets` clean at first build;
  11 unit tests pass.
- 2026-11-04 02:50: Microsoft's `WTSRegisterSessionNotification` page and the Rust and Node docs for the home folder
  re-read; `docs/research.md` "Tray mode checks" added.
- 2026-11-04 02:52: smoke test fixed before any run: it sets `CLAUDE_CONFIG_DIR` to an empty temp folder (else the
  agent sessions on this PC hold the dry run and the "idle at start" check fails), and it looks for another dry-run
  tray with `FindWindowW` instead of `ask_quit`, which would have closed a tray it did not start.
- 2026-11-04 02:55: all Part A checks pass (Result). Status `blocked: waiting for the screen slot (Part A done)`.
  Build folder kept for Part B.

## Result

**Part A done (2026-11-04 02:55). Part B not run** (orchestrator's scope for this round: a sound card measures
loopback audio, and a tray balloon could play a sound into it). No tray was started, real or dry run; no power setting,
registry key or real data folder was touched.

Files: `crates/wakeshot-windows/src/app/tray_mode.rs` (filled), `crates/wakeshot-tray/tests/smoke.rs` (new),
`docs/research.md` (new section "Tray mode checks", addition only), this card.

### What `run` does

- Steps 3 to 5 as note 6: `Boot::new` (fails: exit 1, no log), `take_tray` (taken elsewhere: exit 0; error: one line
  through `LogWriter::open` without a roll, exit 1), `open_log`, a start line with version, mode and pid, a line when
  the last tray ended without quitting, `install_panic_hook`.
- Real or dry run per object: `Win32Power` or `DryRunPower<Win32Power>`; `FileNet::tray(.., AutoStart::real(),
  current_exe, TrayGuard)` or `FileNet::dry_run` (no RunOnce, no guard); `PowerRequests` or `DryRunRequests`;
  `Win32Locker` or `DryRunLocker`; `Win32Hibernate` or `DryRunHibernate`; `IconKind::Product` or `DryRun`. One
  generic `serve<P, G>` runs steps 6 to 9 for both executor types.
- Step 6 `startup_restore`; step 7 the settings mutex; then `read_config`, `remove_stale_temps`.
- Step 8 in this order: `LoopWindow`, `IoQueue`, `OwnerDrawnMenu::new(sink)` with its note hook,
  `TrayIcon::new(kind, Box::new(menu))` with its note hook, `Arc<BalloonNotifier>` and the balloon click hook,
  `WasapiPlayer::new(sink, log)` as one `Arc<dyn SoundPlayer>` (loop runner and hibernate thread), `session::register`
  with retry, `Subscriptions::start` (failures logged), `WatchThread::start` with the Claude source only
  (`ClaudeSource::in_folder(claude_folder(home, CLAUDE_CONFIG_DIR), SystemZone::current())`), `started` and
  `locked_at_start`, `JournalCorrupt` when `Startup.corrupt`, `Dispatcher::new`, then `IoThread::start` (`power:
  Box::new(executor)`, `notifier: Box::new(Arc::clone(&n))`, `open: Box::new(shell::open_with_default_app)`), then the
  pump through a `TrayLoop` handler that wraps the dispatcher.
- Step 9: `IoThread::finish(FINISH_MS)`, then from `into_runners()`: `WatchHandle::stop`, subscriptions, session
  registration (already gone at Quit), icon (`NIM_DELETE`), requests, settings mutex; a last line "Wakeshot ends with
  code N"; the tray mutex. Exit 0, or 1 after a start failure or a failed pump.
- `RealRunners: LoopRunners`: requests, retake, lock and tray icon straight to their objects; `play` to the player;
  `start_hibernate` spawns `wakeshot-hibernate`, which runs `run_hibernate` and posts `Done(Hibernate(end))` once (a
  thread that cannot start posts `Done(Hibernate(Failed(CreateThread)))`, so the dispatcher's one-at-a-time flag
  clears); `apply_update` logs and posts `Done(Update(Err(no update thread, 50)))`; `input_idle_ms` gives 0 on a failed
  read; `try_settings_owner` tries the mutex; `lost_events` (each safety tick) retries the session registration and,
  when `Subscriptions::lost_posts()` grew, gives a fresh `Lid` (200 ms wait), `Power` and `Resumed`; `watch_tick`
  ticks the watch thread.
- Balloon click hook: `take_click()`, then `shell::open_with_default_app` on a short thread `wakeshot-open` (the log
  file, or `ms-settings:taskbar` for the first-run notice), which logs the result. `BalloonNotifier` is `Send + Sync`
  and `Arc<BalloonNotifier>` is a `Notifier`, so no contract gap.

### Decided by the worker, for Haik's review

None of these makes the laptop behave differently when no work is going on.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | The IO thread starts right after `Dispatcher::new`, not before it | The core gives the first-run `Notify` before the first `Tray` in the `Started` batch; the loop adds the icon inside `Dispatcher::new`, so the notice reaches the shell only after the add | The core puts the first `Tray` first |
| 2 | The watch thread starts before `Dispatcher::new` | Its handle goes into `RealRunners`; its posts wait in the window queue, and `Started` is fed directly, so the core still sees it first | Never likely |
| 3 | The session registration goes when the dispatcher returns Quit (a `TrayLoop` wrapper), not in step 9 | `LoopWindow::run` destroys the window before it returns; Microsoft: unregister before the window is destroyed | `LoopWindow` gets a hook before its destroy |
| 4 | Session register retried on each safety tick, any error; first failure and the late success logged; a late success reads the lock state and feeds `Session(Locked)` | 1702 before the Remote Desktop services start; a lock in the gap reached nobody | A log shows the retry never working: then wait on `Global\TermSrvReadyEvent` |
| 5 | The settings mutex tries are made here with `NamedMutex`, one line per new reason (held by another session, denied, other error) and one at the take | `try_settings` hides the reason; a try every 30 s must not bury the log | `try_settings` returns the reason |
| 6 | Lost posts give a fresh `Lid` (200 ms), `Power` and a spare `Resumed` | A lost `Resumed` leaves the requests not honored while work runs; a spare one costs a retake and restarts the idle count (a countdown waits up to 5 min more) | The core tells a resume by itself, or logs show lost posts often |
| 7 | The loop thread's own lines go to the IO thread as `Command::Log`, text "area, warning: ..." | Architecture: the loop never writes the log file; the IO thread writes them in area `core`, so the text keeps area and level | `Command::Log` carries a level and an area |
| 8 | The dry-run objects (`DryRunRequests`, `DryRunLocker`) keep the direct log on the loop thread | One append per call, dry run only; their lines are the dry run's record, keep their area, and outlive the IO thread at shutdown | The e2e cards want them in order with the core's lines |
| 9 | `OpenLog` stays on the IO thread (the orchestrator's risk, weighed) | Only a hung shell holds it, up to 10 s; a restore behind it then waits; at the end of a session it may miss the 4 s wait, and RunOnce restores at the next sign-in. Rare, and covered | A log shows `ShellExecuteExW failed with 1460`, or "the restore did not finish within 4000 ms" right after "log opened": then the IO opener hands off to a short thread, as the balloon click does |
| 10 | The balloon click opens on a short thread of its own; the line names "the log" or "taskbar settings", never the path | The loop never waits on the shell; the path holds the user's name | Never likely |
| 11 | Fatal at start: the loop, menu and icon windows, the watch thread, the IO thread, and (real tray) the exe path; one Error line, exit 1 | Each leaves a tray with no window, no menu, no agent signal or no restore path; the startup restore already ran | A field report shows a tray that should have run without one of them |
| 12 | A failed pump (`GetMessageW` -1) feeds `Menu(Quit)`, runs step 9 and exits 1 | The core then restores and lets the requests go while the IO thread still runs | Never likely |
| 13 | Icon notes logged: `AddFailed` at attempt 1 only, `Added` after a retry, `ModifyFailed`, `TaskbarCreated`; menu notes: `Failed`, `Opened` without the foreground | One line per outage, not per 30 s retry | A field log needs every retry |
| 14 | A failed input idle read gives 0 (counts as input), one line per run of failures | No hibernate on a failed read (`p3-system-events` item 2) | Never |
| 15 | The profile folder from `std::env::home_dir` (`USERPROFILE` first), the same rule as Node's `os.homedir` | Claude Code builds its folder from `homedir()` | A user's Claude folder is not found |
| 16 | Exit code 1 for every start failure | The guard and restore mode use 1 for a give-up | The e2e cards need to tell failures apart |

### Checks (2026-11-04, build folder `<scratchpad>\p6-int-tray-mode\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check` on `tray_mode.rs` and `smoke.rs` | exit 0 (after one `rustfmt` of both owned files) |
| `cargo clippy -p wakeshot-windows --all-targets --target <t> -- -D warnings`, x64 and ARM64 | clean |
| the same without `--all-targets`, x64 and ARM64 | clean |
| `cargo clippy -p wakeshot-tray --all-targets --target <t> -- -D warnings` and without, x64 and ARM64 | clean (no `velopack` in the tray yet, so no clang needed) |
| `cargo test -p wakeshot-windows -- app::tray_mode::` | 11 passed |
| `cargo test -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke` (not `--ignored`) | 1 ignored, so it builds and is skipped |
| Part B checks (smoke run, picture, dry-run entries, build folder deleted) | not run (Part B) |

Unit tests: `dry_run_hibernate_thread_posts_one_done_and_never_hibernates` (every check passes on `FakeHibernate`,
the end is `Resumed`, the chime played, `SetSuspendState` only logged),
`dry_run_hibernate_on_real_reads_posts_exactly_one_done` (`DryRunHibernate::new`),
`a_hibernate_end_that_cannot_be_posted_is_logged`, `apply_update_answers_err_until_the_update_thread_is_wired`,
`loop_lines_go_to_the_io_queue_with_area_and_level`, `balloon_clicks_open_the_log_or_taskbar_settings`,
`settings_mutex_says_why_once_and_takes_it_when_free` (another thread holds a `Names::test` mutex),
`lost_posts_count_only_growth`, `fresh_readings_give_lid_power_and_resumed`,
`a_failed_idle_read_counts_as_input_and_logs_once_per_run`, `icon_and_menu_notes_log_only_what_matters`. The wiring
itself (order, threads, shutdown) has no unit test: the smoke test in Part B is its first run.

### Needs another card

- `log.rs` (`p3-log`): the `log_file(logs_dir)` helper `p6-int-io-thread` asked for; `tray_mode.rs` is now a third
  copy of "wakeshot.log" (`LOG_FILE`, for the balloon click).
- `p6-int-update-wiring` (owns this file next): start the update thread after `IoThread::start` (tray mode only,
  installed only); stop it right after `io_thread.finish` in `serve`; replace `answer_update` in
  `RealRunners::apply_update` with the hand-over; add `Updated` to `first` when Velopack restarted the tray.
- Orchestrator: `docs/phases/06-integration.md` note 6 order (Found outside scope 5).

### Notes for the Part B run

- Build folder kept: `<scratchpad>\p6-int-tray-mode\target` holds the x64 debug builds of the tray exe and the smoke
  test; cargo rebuilds what changed. Delete it at the end of Part B (acceptance check).
- Run alone in a screen slot, session unlocked, no other dry-run tray in this session (the test checks with
  `FindWindowW` and fails at once if one runs): `$env:CARGO_TARGET_DIR = '<that folder>'; cargo test --locked -p
  wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored --test-threads=1 --nocapture`.
- What it does: a temp folder `%TEMP%\wakeshot-tray-smoke-<pid>` with `dry-run\config.json` written first (no
  first-run balloon, so no notification sound) and an empty `claude` folder; a `launch.cmd` that sets
  `WAKESHOT_DATA_DIR` and `CLAUDE_CONFIG_DIR`, writes `whoami /groups` to `groups.txt` and `start`s the tray with
  `--dry-run`; opened with `explorer.exe`, so a Windows Terminal window flashes up and closes. It waits up to 30 s for
  `state.json`, opens the pid it names and checks the exe path, checks Medium (`S-1-16-8192`) and idle, writes
  `holds\smoke.json` with its own pid and start, waits up to 10 s for holding, deletes it, waits for idle, posts
  `ask_quit(true)`, waits up to 30 s for exit code 0, and checks `dry-run\journal.json` is gone. On a failure its drop
  posts Quit, then ends the tray by pid (the open handle keeps the pid from reuse). It prints the tray pid.
- The picture: take it while the hold is on (pause there, or run the same steps by hand with the same `.cmd`), crop
  to the taskbar as `05-tray-ux.md` note 4 says, `docs/research/img/ux/int-smoke-*.png`.
- After the last run: `reg export` to the scratchpad, then delete `HKCU\Control Panel\NotifyIconSettings\<id>`
  whose `IconGuid` is the dry-run GUID `c0d07dcd-dfd4-4a9d-b139-ad1e2ce9f987`, and
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\NotifyIconGeneratedAumid_<id>`; check both
  gone, and again a minute later. Check no tray pid is left.
- What could go wrong first (not verified, none run yet): the hold may need the watch thread's 200 ms retry; the
  Medium tray may not open the elevated test's process (then the liveness list path counts it); a dry-run tray of
  another card running in this session makes the new one exit 0 at once. The tray's log is
  `dry-run\logs\wakeshot.log` in the temp folder; copy it to the scratchpad before the test's drop removes the folder
  if the test fails (run with a breakpoint or copy from a second shell).
