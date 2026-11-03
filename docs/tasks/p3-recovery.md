# Recovery: guard mode, the tray's guard control, restore mode

Phase: 3. Status: ready. Needs Haik: no. Question 12 (a failed restore starts the tray) is answered with the
default, Haik 2026-11-03 18:09 (`docs/phases/02-06-signoff.md` item 11): build it.
Real laptop: no (child processes the tests start; a test data folder; no RunOnce). Agent: `wakeshot-worker`. Size:
about 140K tokens.
Runs after: `p3-journal`, `p3-process`, `p3-log`, `p3-power-plan` (`Win32Power`, which `guard.rs` and `restore.rs`
build; verify review Minor 3), and the core track's `p2-ownership-restore` (the restore routine).
Parallel with: `p3-power-io`, `p3-stop-points` and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-recovery\target`.

## Goal

Everything that restores when the tray cannot: the guard process, the tray's control of it, and restore mode at
sign-in. Both run from the real tray exe (`--guard`, `--restore`), so Haik's checks 4 and 5 test the shipped paths.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 4, 8, 9).
- `docs/architecture.md`: section 4 "Processes", section 5 "Restore routine" step 7, "Liveness", "Guard", "Logon
  restore", "Gap left"; section 8 table.
- `docs/phases/01-design-skeleton.md`: Question 12 (and Values "A restore that fails outside the tray").
- After the contracts card and the cards above: `recovery.rs` (stub), `wakeshot-tray/src/main.rs` (the dispatch the
  contracts card writes), `wakeshot-tray/src/guard.rs` and `restore.rs` (stubs), the core's `ownership::restore`
  (with `expect_owner`) and `FakePlans`, `power/journal.rs` (`FileNet`, `restore_locked`), `process.rs`, `store.rs`,
  `log.rs`.

## Deliverables

- `crates/wakeshot-windows/src/recovery.rs`:
  - `GuardArgs::parse(args)`: `<pid> <start> <stop event>`, all required.
  - `run_guard(args, deps) -> GuardEnd`: open the tray with exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION`,
    check its start time, open the stop event; wait on both. Stop event: exit. Tray gone (or already gone or another
    process at start): if the journal names that tray as owner, the retry loop below with `expect_owner` = that tray,
    then the give-up step; log; exit.
  - The retry loop: `restore_locked` every 10 s for at most 2 minutes, going on only while the journal's owner is still
    the dead process it started for (`Found::OtherOwner`: exit); in restore mode an owner whose liveness is Unknown
    counts as a failed run. Time comes in through a sleep function, so
    tests run with no wait.
  - `TrayGuard: GuardControl`: `ensure` creates `Local\Wakeshot.GuardStop.<tray pid>.<n>` (not set), starts
    `wakeshot-tray.exe --guard <pid> <start> <event>` with `process::spawn_outside_tree`, keeps the handle; `running` is
    "handle not signaled and event not set"; `stop` sets the event. A guard whose event is set counts as leaving, so
    `ensure` starts a new one.
  - `run_restore_mode(deps) -> RestoreModeEnd`: journal missing: done; owner alive: exit (another tray took it);
    Gone: the retry loop; Unknown: a failed run, retried. Never writes RunOnce
    (Windows deleted it before running the command).
  - The give-up step, one function: start the tray (`wakeshot-tray.exe`, Question 12: the default, Haik's answer).
- `crates/wakeshot-tray/src/guard.rs`, `restore.rs`: thin mode files that build the real parts (`Win32Power`,
  `FileNet` with the real RunOnce value, which the restore routine only ever clears, `NoGuard`, `SystemLiveness`,
  `LogWriter` with `guard` or `restore`) and call the functions above. No logic.
- `crates/wakeshot-tray/tests/recovery.rs`: tests that run the built tray exe (`CARGO_BIN_EXE_wakeshot-tray`) with
  `WAKESHOT_DATA_DIR` set to a temp folder.

## Acceptance checks

- Unit tests in `recovery::tests` with fakes (the core's `FakePlans` for plans and safety net, `FakeLiveness`, a fake
  sleep): `guard_args_parse`, `guard_exits_on_stop_event`,
  `guard_restores_when_tray_dies`, `guard_leaves_journal_of_another_owner`, `guard_retries_then_gives_up_and_starts_tray`,
  `restore_mode_missing_journal`, `restore_mode_live_owner_exits`, `restore_mode_unknown_owner_retries`,
  `restore_mode_never_writes_run_once`, `tray_guard_starts_new_when_old_is_leaving`, `tray_guard_stop_sets_event`.
- `restore_locked_round_trip` (moved from `p3-journal`, review m5): with the core's `FakePlans` as `PlanIo` and
  `FileNet` in a temp folder, a saved journal is restored through `power::journal::restore_locked` and leaves no file
  and no RunOnce value (test key `HKCU\Software\Wakeshot-test\<pid>\<test name>`).
- Stop-point tests `guard_stop_points` and `restore_mode_stop_points`: with `FakePlans`' script, stop at every call
  of each run (the guard or restore mode dies too), then the next start's restore; check track note 4.
- Exe tests in `wakeshot-tray/tests/recovery.rs` (real processes, a temp data folder, no plan touched): a sacrificial
  "tray" process the test starts; `--guard` for it; the test ends the sacrificial process through its handle; the
  guard exits within 2 s and logs "no journal"; a second run where the test sets the stop event: the guard exits
  "stopped". `--restore` with no journal exits 0 at once.
- `real_guard_parent_is_explorer`, `#[ignore]` (it needs Explorer; review M9): the guard's parent is Explorer (process
  list). This card runs it once with `cargo test --locked -p wakeshot-tray --test recovery -- --ignored`;
  `p3-real-checks` runs it again.
- Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows -p wakeshot-tray
  --all-targets` with `-D warnings` for both targets (the tray's ARM64 run is CI only once `velopack` is in and this
  laptop has no clang); `cargo test --locked -p wakeshot-windows recovery::`; `cargo test --locked -p wakeshot-tray`.
  Record each outcome. No child process of the tests left (by the pids they logged).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Task Manager End task and sign-out with RunOnce (`p3-haik-checks`); the tray mode itself (integration); the
  Velopack hooks (track 6).
- Any file but the four above.

## Questions for Haik

Each with the worker's default.

- Question 12 (phase 1 file) is answered with the default (Haik 2026-11-03 18:09): start the tray after 2 minutes.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
