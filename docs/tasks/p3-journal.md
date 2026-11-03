# Journal: the real `SafetyNet` (journal file, RunOnce value, guard) and the journal lock

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (temp folders, a test registry key, no guard process).
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p3-store`, `p3-log`. Parallel with: `p3-hibernate`, the rest of wave 1, any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-journal\target`.

## Goal

Fill `power::journal` with `FileNet`, the Windows implementation of the core's `SafetyNet` seam (journal file, RunOnce
value, guard), its dry-run, guard and hook forms, and the locked entry points every process uses to call the core
routines under `Local\Wakeshot.Journal`.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 3, 5, 7 to 9).
- `docs/architecture.md`: section 4 "Processes" (`Local\Wakeshot.Journal`, `WAIT_ABANDONED`), section 5 "Apply"
  steps 3 and 4, "Restore routine" steps 1 and 6, "Logon restore"; section 6 "Journal format" and the version rule.
- `docs/reviews/p1-contracts-domain.md`: M2, M4, m2.
- After the contracts card: the core's `ownership::io::{SafetyNet, JournalRead}` and the routine signatures in
  `ownership::{apply, restore, check}`; `power/journal.rs` (empty, this card's); `store.rs`, `log.rs`;
  `wakeshot-core/src/files.rs`.

## Deliverables

`crates/wakeshot-windows/src/power/journal.rs`:

- `FileNet: SafetyNet` over `store`:
  - `load_journal`: missing, read, corrupt (read in full but does not parse), failed (any other read error). A file
    with keys this version does not know is logged once per load (correction 12; the core keeps them untouched);
  - `save_journal` through `store::write_atomic` (the core always passes `JournalFile::VERSION`), logged;
    `delete_journal` (missing counts as done); `set_aside_journal` (to `journal.json.corrupt`, replacing an older one);
  - `set_run_once` (`true` when this call added it) and `clear_run_once` (a missing value counts as cleared): value
    `WakeshotRestore` = `"<tray exe>" --restore` through `store::AutoStart`;
  - `ensure_guard` and `stop_guard` through a `GuardControl` (`ensure` returns whether it started one, `stop`);
    `NoGuard` for every process but the tray; the tray's `TrayGuard` comes from `p3-recovery`.
- Forms: tray (all of it); dry run (its own data folder and names, no RunOnce, `NoGuard`); guard, restore mode, hooks
  and the CLI's `wakeshot restore` (`p4-cli-restore`) (RunOnce cleared, never set, `NoGuard`; review M1).
- Locked entry points: their three signatures exist as stubs in this file (contract: each takes the core routine's
  arguments plus `names: &Names, log: &dyn Log` and returns the routine's result; a lock not taken comes back as the
  routine's failed result; `p1-contracts-split` edit 3). Fill the bodies: `restore_locked`, `apply_locked`,
  `check_locked` take `Local\Wakeshot.Journal` (10 s wait;
  `WAIT_ABANDONED` counts as taken and is logged; a timeout is a failed run, track note 9), call the core routine, and
  log the outcome. The CLI (track 4), the hooks (track 6), the executor and recovery call only these, never the core
  routines bare.
- `mark_corrupt_seen(dir)`: rename `journal.json.corrupt` to `journal.json.corrupt.seen` (track note 7); returns
  whether there was one.

## Acceptance checks

Tests in `power::journal::tests`, in a temp folder (`DataDir::at`) and under
`HKCU\Software\Wakeshot-test\<pid>\<test name>` (each test deletes only its own key; the leftover sweep deletes only
keys whose pid is not running; review M8):

- `load_tells_missing_corrupt_failed` (a folder in place of the file gives failed), `save_load_round_trip`,
  `unknown_keys_are_logged_once`, `delete_missing_is_done`, `set_aside_keeps_bytes`, `corrupt_seen_rename_once`.
- `run_once_set_reports_added`, `run_once_clear_missing_is_done` (test key), `dry_run_never_touches_run_once`,
  `guard_forms_never_set_run_once`.
- `ensure_guard_passes_through`, `no_guard_starts_nothing` (a recording `GuardControl`).
- `lock_excludes_second_caller`, `lock_abandoned_counts_as_taken`, `lock_timeout_fails_the_run` (test mutex names).
- `restore_locked_round_trip` is not here: it needs the core's routine, so `p3-recovery` (which runs after
  `p2-ownership-restore`) owns it (review m5: no later card owns this file to turn an ignored test on).

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows power::journal::`. Record each outcome. After
the tests no `HKCU\Software\Wakeshot-test` key of this run's pids and no file outside the temp folder.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The apply, the restore routine, the ownership check, the journal split and merge, the clean-up order and the core's
  fake (core track, `p2-ownership-*`); the real guard (`p3-recovery`); the stop-point test against these real parts
  (`p3-stop-points`).
- Any file but `power/journal.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: the routines are the core's (orchestrator decision 17:05); this card is the seam.

## Result

What was done, files changed, checks and outcomes, open items.
