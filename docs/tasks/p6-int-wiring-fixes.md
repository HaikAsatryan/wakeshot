# Wiring fixes: a lost first-run notice, a stuck hibernate flag, zone errors, one log file name

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens. Written by the orchestrator
(session `wakeshot-46`, 2026-11-04) from `p6-int-tray-mode` "Found outside scope" items 1 to 3 and its "Needs another
card" (`log::log_file`).
Runs after: `p6-int-tray-mode` Part A (done), `p6-velopack-dependency` (it runs alone), `p6-int-update-wiring` (it
owns `tray_mode.rs` before this card). Parallel with: any card that owns none of the files below.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-wiring-fixes\target`.

## Goal

Close four small gaps the tray wiring found, each with the leanest fix and a test.

## Read first

- `docs/tasks/p6-int-tray-mode.md`: "Found outside scope" 1 to 3, "Needs another card".
- `crates/wakeshot-windows/src/app/dispatch.rs` (the `hibernating` flag), `crates/wakeshot-windows/src/notify.rs`
  (`BalloonNotifier`), `crates/wakeshot-core/src/core.rs` (`take`, `settle`: the `Started` batch order),
  `crates/wakeshot-windows/src/localtime.rs` (`SystemZone`, `take_error`), `crates/wakeshot-windows/src/log.rs`.

## Deliverables

1. First-run notice: when the first icon add fails (a start at sign-in before the taskbar exists), the notice must
   still show once the icon is added. Pick the leanest place (the core's batch order, `BalloonNotifier` keeping a
   notice that failed for want of an icon, or the tray holding it until `Note::Added`) and say why. Test with the fake
   shell.
2. Hibernate flag: a `Done(Hibernate)` whose post fails (queue full, 1816) must not leave the dispatcher's
   `hibernating` flag set for the rest of the run. Pick the leanest fix (a short retry of the post on the hibernate
   thread, or a clear on `Resumed` or on the next lid open) and say why. It must never start two hibernate threads at
   once. Test.
3. `SystemZone` errors: once the zone is boxed into `ClaudeSource`, a failing zone read is never logged. Log it (once
   per new error, not per read) without a contract change if you can; if not, describe the change and stop.
4. `log::log_file(logs_dir) -> PathBuf` in `log.rs`, used by `io_thread.rs`, `hooks.rs` and `tray_mode.rs` instead of
   their own copies of "wakeshot.log". Remove the copies and `hooks.rs`'s agreement test if it becomes moot.

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc`; ARM64 the same if clang is on the laptop, else "ARM64: CI only".
- `cargo test --workspace --locked`: all pass; record the counts.
- Only the files the deliverables name and this card changed.

## Out of scope

- Any real power setting, screen, sound or tray start. Tests use fakes only.
- The update thread wiring (`p6-int-update-wiring`).

## Questions for Haik

None expected. Haik is away: decide with your default and list it as "decided by the worker, for Haik's review".

## Found outside scope

## Log

## Result
