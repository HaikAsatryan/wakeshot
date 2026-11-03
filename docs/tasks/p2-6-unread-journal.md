# Retry an unread startup journal; check the lid again after the going to sleep sound; keep a newer config

Phase: 2 to 6 (core and Windows power). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 60K
tokens. Written by the orchestrator (session `wakeshot-46`, 2026-11-04) from power safety review findings 3 and 6.
Runs after: core round B (`docs/tasks/p2-core-settings.md`, "Core round B (2026-11-04)"), since both edit `core.rs`
and `scenarios/props.rs`. Parallel with: no card that builds `wakeshot-core` or `wakeshot-windows` (a contract
change: every caller of `Event::Started` changes in the same pass). Cards that do not build may run beside it.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-journal\target`.

## Goal

1. Finding 3: when the startup read of `journal.json` fails (delete pending, antivirus, I/O error), the core retries
   it on each tick as `Restore { why: Leftover }`, instead of treating the journal as empty for the whole run.
2. Finding 6: the hibernate thread checks the lid once more after the going to sleep sound, right before
   `SetSuspendState`, so a lid opened during the sound skips the hibernate.

## Read first

- `docs/reviews/p2-6-power-safety.md`, findings 3 and 6.
- `docs/tasks/p6-int-startup.md`, "Power safety review finding 3: what the core-side fix needs from startup".
- `crates/wakeshot-windows/src/power/executor.rs` (`Startup`, `startup`, the `Leftover` path);
  `crates/wakeshot-windows/src/app/startup.rs` (`started`, `startup_restore`, the 500 ms retry);
  `crates/wakeshot-core/src/event.rs` (`Event::Started`); `crates/wakeshot-core/src/core.rs` (the `Started` arm);
  `crates/wakeshot-windows/src/power/hibernate.rs` (`run_hibernate`, `check_lid`).
- `docs/architecture.md`: the lines on `Started` and the hibernate thread's steps (grep for them).

## Deliverables

1. `Startup` gains `unread: bool` (`power/executor.rs`): true when `JournalRead::Failed`. Its doc says what it means.
2. `Event::Started` gains `journal_unread: bool` (`event.rs`, a contract change); `started()` copies `Startup.unread`
   into it (`app/startup.rs`). Every place that builds `Event::Started` (core tests, `sim.rs`, `scenarios/props.rs`,
   `app/dispatch.rs`, `app/startup.rs`) gets the field.
3. The core (`core.rs`, `Started` arm): `journal_unread` sets `retry = Some(RestoreWhy::Leftover)` with an empty copy.
   Tests: an unread journal retries on the next tick; a later good read that restores ends the retry; a read that
   keeps failing keeps the retry and never panics or loops inside one tick.
4. Decide whether the 500 ms startup retry in `app/startup.rs` stays (quick first try) or goes; record why.
5. `run_hibernate`: after the sound and before `SetSuspendState`, one more `check_lid`; open is
   `Skipped(LidOpen)`, no answer is `Skipped(LidUnknown)` as in step 1. Logged. Tests with the fake deps: lid opens
   during the sound gives `Skipped(LidOpen)` and no hibernate call; the doc list in `run_hibernate` gets the new step.
   No cancel flag: the review's lean option needs no contract change. An agent turn that starts in those seconds is
   not covered; say so in the Result.
6. `docs/architecture.md`: only the lines on `Started`'s fields and the hibernate steps, changed in place.
7. Added by the orchestrator after core round B: the `WriteConfig` runner never writes over a newer `config.json`.
   The exact change, its test and the two doc lines are in `docs/tasks/p2-core-settings.md`, "Core round B", "Needs
   another card" item 1 (`crates/wakeshot-windows/src/app/io_thread.rs`, the `Command::WriteConfig` arm; one sentence
   in `docs/architecture.md` section 6; one doc line on `WriteConfig` in `crates/wakeshot-core/src/command.rs`).

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- `cargo test --workspace --locked`: all pass; record the counts.
- Only the files above and this card changed.

## Out of scope

- A cancel flag for the hibernate thread (`Command` or `Cancel` changes); a sleep fallback after a failed hibernate.
- Power findings 5, 8, 9, 10 (core rounds A and B).
- Any real power setting, lid, screen or sound: tests use fakes only.

## Questions for Haik

None expected. Haik is away: decide with your default and list it as "decided by the worker, for Haik's review".

## Found outside scope

## Log

## Result
