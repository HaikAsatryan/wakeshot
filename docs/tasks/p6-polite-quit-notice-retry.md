# The tray quits on a plain taskkill; a notice that fails is tried again

Phase: 6 (tray fixes). Status: done (2026-11-05 12:52). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens. Written by
the orchestrator, 2026-11-05, from `p6-notice-app-icon` "Found outside scope" 6 and 7 and `p6-screen-reruns` "Found
outside scope" 5 (both workers' default: a fix card; no product decision).
Runs after: `p2-6-sound-overlap-smoke-key` (its checks build `wakeshot-windows` over a changing `wakeshot-core`).
Parallel with: nothing that edits `wakeshot-windows` or `wakeshot-core`.
Real laptop: no power settings. Dry-run tray starts with a temp data folder only (`WAKESHOT_DATA_DIR`, debug build),
as `crates/wakeshot-tray/tests/e2e_dry_run.rs` does. Never against an installed Wakeshot.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-polite-quit-notice-retry\target`.

## Goal

Two small tray bugs found in the screen session of 2026-11-05: `taskkill` without `/F` leaves the tray running (and
holding), and a notice that Windows refuses once is lost.

## Read first

- `docs/tasks/p6-notice-app-icon.md`: "Found outside scope" 6 and 7.
- `docs/tasks/p6-screen-reruns.md`: "Found outside scope" 5; `docs/research.md` "Screen session of 2026-11-05".
- `crates/wakeshot-windows/src/tray/icon.rs` (the comment near line 573), `tray/menu.rs`, `app/window.rs` (how
  `WM_CLOSE` on `Wakeshot.Loop` becomes Quit), `notify.rs`.
- `docs/architecture.md` line 685 and section 5 "Notices"; `docs/phases/06-integration.md` line 80.

## Deliverables

### Part A: a plain taskkill quits the tray

- Find from current Microsoft docs, or a small test, which window `taskkill` without `/F` sends `WM_CLOSE` to. The
  worker who found it guessed the first top-level window of the thread (`Wakeshot.TrayMenu`), not verified.
- Fix in the tray: `WM_CLOSE` on the menu and icon windows ends in the same Quit as on `Wakeshot.Loop` (full
  restore first), or another fix if the docs show a cleaner one. A test that starts a dry-run tray, runs real
  `taskkill /PID <pid>` without `/F`, and sees it quit with the restore lines in its log.
- Docs that say a plain taskkill quits it today: `docs/architecture.md` line 685, `docs/phases/06-integration.md`
  line 80, the comment in `tray/icon.rs`. Make them true.

### Part B: a notice Windows refuses is tried again

- `notify.rs`: when `Shell_NotifyIconW` fails for a notice (seen: error 1462 at a lid open right after a resume, 6 ms
  after another notice), try it again once after a short wait. Pick the wait from the evidence and say why. Log the
  retry and its outcome. A test with the fake shows a refused first try gives one retry and no loop.
- If two notices in the same pass make the second fail (the 11:24:39 case), say whether spacing them is the better
  fix, and take the lean one.

### Part C: docs

- `docs/architecture.md` section 5 "Notices" and `docs/research.md` "Notice header icon checks": the header shows
  the app icon from both start paths; a balloon is a banner-only toast that Win+N never keeps (`p6-notice-app-icon`
  Found outside scope 7). Replace the old open line, do not append.

## Acceptance checks

- `cargo fmt --all --check`; clippy for `wakeshot-windows` and `wakeshot-tray`, with and without `--all-targets`,
  `-D warnings`; `cargo test -p wakeshot-windows -p wakeshot-tray --locked`; the new taskkill test passes (run it,
  even if `#[ignore]`).
- Whole-workspace checks on x64 and ARM64 at the end (clang is in: `p6-velopack-dependency` step 0).

## Out of scope

- Win+N history (balloons stay; Haik 2026-11-04). The Settings tray list on this laptop (Haik's later sign-out).
- `wakeshot-core`.

## Questions for Haik

None.

## Decisions (worker, 2026-11-05)

1. **The icon's and the menu's windows post `WM_CLOSE` on to this thread's loop window** (`app::window`
   `pass_close_to_loop`, a thread-local set by `LoopWindow::new` and cleared by its drop), so every close ends in the
   loop window's one Quit path. Not a check of `WM_CLOSE` in the pump: a close posted while the menu is open is
   dispatched by the menu's own modal loop and would be lost. Change it when the tray gets a top-level window on
   another thread.
2. **The menu's window ignores `WM_SYSCOMMAND` `SC_CLOSE`.** Each open makes that hidden window the foreground window,
   and `DefWindowProcW` turns `SC_CLOSE` into `WM_CLOSE` (probe), so an Alt+F4 meant for another window could now
   quit the tray. Today it does nothing, and this keeps it so. Change it when a real Alt+F4 check shows the key never
   reaches the hidden window (then the guard can go).
3. **One retry after 1 s, plus the old one at once after an add during the call; at most three calls.** The wait:
   the refused notice came 6 ms after another, the same pair 21 ms apart showed, and 1 s is the settle time the icon
   gives display and theme broadcasts (`notify.rs` `RETRY_AFTER`). The retry builds the balloon again, so the app
   icon is drawn at the size of that moment. Change it when a log shows "the notice was refused again" after the
   wait.
4. **Retry, not spacing.** Spacing would delay every pair of notices and would not help if Explorer's state right
   after the resume was the cause; the retry costs nothing when the first try works. Lean pick.
5. **The notifier logs at Info, area `notify`, without the notice's name:** the IO thread's own line right after it
   names the notice and is the warning when it is still not shown. One IO thread runs every notice, so the lines never
   interleave with another notice's. Change it when a notice's name is needed in those lines (then move
   `io_thread::notice_name` where both can use it).
6. **The taskkill test refuses a run where the loop window is first in z-order**, since the old code would quit then
   too and the test would pass for the wrong reason.
7. **One line outside my file list**: `app/tray_mode.rs` line 301, `.with_log(Arc::clone(log))`, the same log the
   IO thread writes to. Needed to log the retry; nothing else ran.

## Found outside scope

1. **The `Notifier` trait doc** (`notify.rs`, a contract) says `show` "returns once Windows took the notice or
   refused it". Still true, but a refusal now holds the IO thread up to about 1 s more, and a session-end restore
   queued behind it waits as long (inside its 4 s budget). Left as is; a line there could say so.
2. **`docs/tasks/p6-notice-app-icon.md` step 8** said a plain taskkill "quits in about 0.2 s"; wrong then, true now. A
   finished card, left alone.
3. **The cause of the 1462 is not verified.** An icon size check during the lid open's display change is a guess.
   Checking it needs a real added icon (a tray icon and a `NotifyIconSettings` entry for a probe exe, and maybe a
   banner on Haik's screen). If the retry ever logs "refused again", that check is next.
4. **A real Alt+F4 on the menu's hidden window was not tried**: a key sent to the foreground window could close one of
   Haik's programs.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started. The orchestrator added four small doc items to Part C (from
  `p2-6-sound-overlap-smoke-key` Found outside scope 1, 3 and 5).
- 2026-11-05 12:17: Part A fact: `taskkill /PID` without `/F` posts one `WM_CLOSE`, to the process's first top-level
  window in `EnumWindows` order, not to every window. Probe `<S>\probe\closeprobe.exe` (C#, three hidden windows in
  the tray's order and styles, each `WM_CLOSE` logged and ignored), elevated: order Icon, Menu, Loop gave one posted
  `WM_CLOSE` to the icon window (twice, one per taskkill); after `SetForegroundWindow(menu)` the order was Menu,
  Icon, Loop and the menu window got it. A sent `WM_SYSCOMMAND` `SC_CLOSE` to the menu window became a sent
  `WM_CLOSE` through `DefWindowProcW`. `<S>` = `<scratchpad>\p6-polite-quit-notice-retry`.
- 2026-11-05 12:25: Part B evidence: the two saved tray logs (`p6-int-haik-checks` partC 00:47:52, `p6-notice-app-icon`
  partB 11:24:39) show the same pair after a resume and a lid open: StepBack (a warning, no app icon) then
  ClosedLidReport (app icon, `NIIF_USER | NIIF_LARGE_ICON`): 21 ms apart both showed, 6 ms apart the second failed
  with 1462. Probe `<S>\probe\sizeprobe.exe`: `NIM_MODIFY` on an icon never added gives `E_FAIL` (0x80004005) for
  every balloon icon size and flag set, so the client side gives no 1462 for a small icon there; whether the shell
  gives 1462 for an icon size on an added icon: not tested (it would add an icon and maybe show a banner).
- 2026-11-05 12:25: code: `window.rs` `pass_close_to_loop` (thread-local loop window), icon and menu pass `WM_CLOSE`
  on, menu ignores `SC_CLOSE`; `notify.rs` retry with `RETRY_AFTER` 1 s and a log. One line outside my file list:
  `app/tray_mode.rs` line 301 gives the notifier the log (`.with_log(Arc::clone(log))`); no other card runs.
- 2026-11-05 12:30: new `real_e2e_plain_taskkill_quits_with_the_restore` passed twice (z-order `Wakeshot.TrayIcon`,
  `Wakeshot.TrayMenu`, `Wakeshot.Loop.DryRun`, `IME`; exit 0, restore line, Keep awake file and journal gone).
- 2026-11-05 12:40: hand mutants, 6 of 6 killed: icon no pass (e2e test: "did not quit", and the window test), menu
  no pass, no `SC_CLOSE` guard (window test), no wait retry, retry with no wait, an endless retry (notify tests).
  The endless one first hung the tests; the fakes now cap their calls so it fails. Lesson: `Copy-Item` keeps the old
  write time, so a restore must touch the file or cargo runs the mutant's binary as up to date (hit once, caught by a
  hang, files touched, every later run rebuilt).
- 2026-11-05 12:43: Part C docs: `architecture.md` the lid-chime lines (368 to 370), the `WM_CLOSE` bullet (about line
  685) and section 5 "Notices" (the open Part B line replaced with its answer; the retry); `06-integration.md` the
  line 80 bullet (rewrapped, its other words kept); `research.md` "Notice header icon checks" intro (open line
  replaced), two pointers in "Screen session of 2026-11-05", new "Plain taskkill and the tray's windows" and "Notice
  retry"; `CLAUDE.md` the ARM64 PATH fact; `packaging/README.md` the smoke test sentence (matches the script's
  refusal list and exit 2); `p6-velopack-dependency.md` "Steps for Haik" first line.

## Result

Done. A plain `taskkill /PID` now quits the tray with the full restore, and a notice the shell refuses is tried again.

- **Part A.** Fact: taskkill without `/F` posts one `WM_CLOSE`, to the process's first top-level window in z-order
  (probe; the dry-run tray's order is TrayIcon, TrayMenu, Loop). Fix: the icon's and the menu's windows post
  `WM_CLOSE` on to this thread's loop window (`app::window::pass_close_to_loop`); the menu's window ignores
  `SC_CLOSE` so Alt+F4 never quits. The `tray/icon.rs` comment, `architecture.md` (about line 685) and
  `06-integration.md` line 80 now say what happens.
- **Part B.** `BalloonNotifier::show`: a refusal is tried once more after 1 s (at once if the icon was added during
  the call), at most three calls, the app icon drawn again; still refused, it is kept for the next add as before. The
  notifier logs the retry and its outcome (Info, area `notify`). Retry, not spacing (decision 4).
- **Part C.** Section 5 "Notices" and research "Notice header icon checks": the open Part B line replaced with its
  answer. The orchestrator's four doc items done.

Files: `crates/wakeshot-windows/src/app/window.rs`, `tray/icon.rs`, `tray/menu.rs`, `notify.rs`, `app/tray_mode.rs`
(one line, decision 7), `crates/wakeshot-tray/tests/e2e_dry_run.rs` (new ignored test), `docs/architecture.md`,
`docs/research.md`, `docs/phases/06-integration.md`, `CLAUDE.md`, `packaging/README.md`,
`docs/tasks/p6-velopack-dependency.md`, this card.

Checks, all passed: `cargo fmt --all --check`; clippy `--workspace` x64 with and without `--all-targets`,
`-D warnings` (covers both crates; also run per crate); `cargo test --workspace --locked` (wakeshot-windows lib 591
passed, 24 ignored; every suite ok); `real_e2e_plain_taskkill_quits_with_the_restore` three times (exit 0, restore
line, no Keep awake file or journal); hand mutants 6 of 6 killed; x64 release build; ARM64 clippy both ways and release
build with the Llvm `x64\bin` folder on that command's PATH. Build folder deleted; no process or temp folder left.

Open: Found outside scope 1 to 4 (the 1462 cause stays a guess; a real Alt+F4 check not tried).
