# The tray quits on a plain taskkill; a notice that fails is tried again

Phase: 6 (tray fixes). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens. Written by
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

## Found outside scope

## Log

- 2026-11-05: card written by the orchestrator.

## Result
