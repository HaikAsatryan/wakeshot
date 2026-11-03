# Journal: the real `SafetyNet` (journal file, RunOnce value, guard) and the journal lock

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (temp folders, a test registry key, no guard process).
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

None.

## Found outside scope

Problems seen but not fixed.

1. Core restore routine, failed load (`wakeshot-core/src/ownership/restore.rs` line 54): `JournalRead::Failed` gives
   `Restored { found: Nothing, result: Err, journal: [] }`. The `Restored` doc allows it ("empty when ... could not be
   read"), but `Outcome` (`command.rs` line 168) says the core's mirror is a copy of `journal`. If the core reducer
   copies `[]` from a failed run, it forgets entries the file still holds and may stop restoring them. The reducer
   should keep its mirror on a failed `Restored` with `found: Nothing`, or the routine should not return `[]` there.
   For the core track (the routine and the reducer card). The lock-failure paths here avoid the same trap (decision 10).
2. `docs/tasks/p6-int-tray-mode.md` line 27 names `p3-recovery` for `NoGuard`; it is in `power::journal` (this card,
   as the card's Deliverables say). For the orchestrator or that card.
3. `store::NamedMutex::open` creates `Local\Wakeshot.Journal` with the default security. A mutex made by an elevated
   process (`wakeshot restore` from an admin terminal) may refuse a Medium tray's open while that process holds a
   handle: the tray's apply then fails at its safety net for that one try. Not tested (every agent run is High); for
   `p3-real-checks`. Recorded in `docs/research.md`, "Journal seam checks".
4. `wakeshot-core/src/state.rs` (another card's, mid-change at 21:00): `cargo clippy -p wakeshot-windows -- -D
   warnings` stops there (dead code, unfulfilled `#[expect(dead_code)]`), so this card's `-D warnings` runs could not
   finish while it lasted. Gone by the final run (Log, 21:25); nothing to do.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: the routines are the core's (orchestrator decision 17:05); this card is the seam.
- 2026-11-03 20:05: started by `wakeshot-worker`. Owns `power/journal.rs` and this card only.
- 2026-11-03 20:20: read the card's sources, `store.rs`, `log.rs`, the core's `ownership` (restore filled; `apply`
  and `check` still `unimplemented!`, `p2-ownership-apply` runs now). So the lock tests drive `restore` and the
  lock-failure paths of all three; the taken path of `apply` and `check` waits for that card. Writing the file.
- 2026-11-03 20:45: `journal.rs` written (FileNet, three forms, GuardControl, NoGuard, RecordingGuard, locked entry
  points, mark_corrupt_seen, 16 tests); rustfmt clean. First test build: `wakeshot-core/src/sim.rs` (another card's,
  mid-edit) does not compile (`KEEP_AWAKE_FILE`, `at_once`). Waiting, then retry.
- 2026-11-03 21:00: core compiles again. `cargo test --locked -p wakeshot-windows power::journal::`: 16 passed, first
  run. Clippy with `-D warnings` stops in `wakeshot-core/src/state.rs` (another card's: dead code and unfulfilled
  `#[expect(dead_code)]`, 26 errors). Clippy without `-D warnings`, both targets, with and without `--all-targets`:
  no warning in `wakeshot-windows`. Retry the `-D warnings` runs before closing.
- 2026-11-03 21:25: `state.rs` compiles clean again. Final runs: rustfmt check exit 0; the four clippy runs with
  `-D warnings` exit 0; tests 16 passed. No `HKCU\Software\Wakeshot-test` key, no `%TEMP%\wakeshot-journal-*`
  folder. Build folder deleted. Done.

## Result

What was done, files changed, checks and outcomes, open items.

Status: see the header. Contract signatures kept (`restore_locked`, `apply_locked`, `check_locked` unchanged).

What `power::journal` has now:

- `FileNet<G: GuardControl>: SafetyNet`, three forms:
  - `FileNet::tray(dir, auto, tray_exe, guard, log)`: RunOnce `WakeshotRestore` = `"<tray exe>" --restore` under
    `auto` (`AutoStart::real()` in the tray), set and cleared; `guard` is `TrayGuard` (`p3-recovery`).
  - `FileNet::dry_run(dir, log) -> FileNet<NoGuard>`: holds no registry key at all; `set_run_once` is `Ok(false)` and
    logged, `clear_run_once` does nothing. The caller passes `DataDir::dry_run()` and `Names::dry_run()`.
  - `FileNet::restore_only(dir, auto, log) -> FileNet<NoGuard>`: guard, restore mode, hooks, `wakeshot restore`.
    RunOnce cleared, never set: `set_run_once` fails (`FileNet::set_run_once`, 50), so an apply through it writes
    nothing.
  - `dir()`, `guard()` (tests read a `RecordingGuard` through it).
- `load_journal`: missing (2 or 3, also a missing folder), `Read`, `Corrupt` (read in full, does not parse; logged with
  the parse error), `Failed` (any other read error, for example a folder in place of the file: 5). Unknown keys: one
  Info line naming them. `save_journal`: `store::write_atomic`, pretty JSON, always `JournalFile::VERSION`, one Info
  line with owner and every entry (original, written, restored). `delete_journal`: missing counts as done.
  `set_aside_journal`: rename to `journal.json.corrupt`, replacing an older one.
- `ensure_guard` and `stop_guard` pass through `GuardControl` (`ensure(&mut self) -> Result<bool, WinError>`,
  `stop(&mut self) -> Result<(), WinError>`); `NoGuard`; `impl GuardControl for Box<G>`; `RecordingGuard` and
  `GuardCall` behind `test-support` for the stop-point and recovery tests.
- `restore_locked`, `apply_locked`, `check_locked`: take the journal mutex (`JOURNAL_WAIT_MS` = 10 000), log
  `WAIT_ABANDONED` and go on, run the core routine, release, log the outcome. Not taken: the routine's failed result
  (`WaitForSingleObject` 258 on a timeout, the `WinError` on an open or wait error).
- `mark_corrupt_seen(&DataDir) -> Result<bool, WinError>`; `RUN_ONCE_NAME` for the uninstall hook.

Decisions (mine, as principal designer):

| # | Choice | Reason | Change it when |
|---|---|---|---|
| 1 | `FileNet` keeps an `Arc<dyn Log>` | It lives as long as the tray's IO thread and shares the one `LogWriter` | A caller needs a borrowed log |
| 2 | The dry run holds no `AutoStart` at all | It cannot reach the registry by its type, not only by a flag | Never |
| 3 | `restore_only` refuses `set_run_once` with an error; the dry run answers `Ok(false)` | An apply through a restore-only net fails before any write; a dry-run apply goes on, as its tray needs | A non-tray process must apply |
| 4 | `set_run_once` reads first: same value, no write, `false`; missing, set, `true`; another value or type, overwrite, `false` | One registry read, not a write, per hold start; "added" only when known missing, so a failed apply takes back only what it added; refusing an odd value would fail every apply | A foreign `WakeshotRestore` value shows up in a report |
| 5 | An exe path that is not valid Unicode fails `set_run_once` (1113) | A lossy path would name no exe, and nothing would run at sign-in | Never in practice |
| 6 | Load skips a UTF-8 BOM and reads only the first JSON value, as `files::read` does | A set-aside leaves the user's values overridden until set by hand, so a hand edit must not cause one | A lenient read hides a real fault in a report |
| 7 | Save forces `JournalFile::VERSION` (Warn when the caller passed another) and writes pretty JSON | Review m2; a person may have to read the file | Never; size matters |
| 8 | Unknown entries are logged at the first load that sees them and again when they change, per `FileNet`, not at every load | The ownership check loads the journal every 10 s during a hold; a line each time would bury the log. A reading of "once per load" that a reviewer may call a deviation | A report needs every load logged |
| 9 | `set_aside_journal` and `mark_corrupt_seen` use `std::fs::rename` | It replaces a target a reader holds with delete sharing (tested); a rename in one folder is never a copy, so write-through adds nothing | A data folder on a file system without POSIX rename |
| 10 | A lock not taken returns the routine's failed result with `journal` = the file's known entries, read without the lock (`found: Entries` when there are some) | `Outcome` says the core's mirror copies `journal`; `[]` would tell the core nothing is owned while the file holds entries. The read is safe without the lock: every write is atomic | The core keeps its mirror on a failed outcome; then return `[]` |
| 11 | The mutex is opened on each call, taken and dropped on the calling thread | Simple; `HeldMutex` is not `Send`; a `CreateMutexExW` every 10 s costs nothing | Profiling shows the cost |
| 12 | Outcome lines: every restore and apply; the check only when the user changed a key or a read failed; a lock failure logs only its own line | The check runs every 10 s | A report needs the quiet checks |
| 13 | `FileNet` logs what it changed (save, delete, set aside, RunOnce set or removed, guard started), not its failures | The entry point's outcome line names the failing call, so no double lines | A failure outside the entry points goes unlogged (the caller of a bare `load_journal` logs it) |

Files changed: `crates/wakeshot-windows/src/power/journal.rs` (filled, 16 tests); `docs/research.md` (new section
"Journal seam checks", additions only); this card.

Checks (2026-11-03, `CARGO_TARGET_DIR` = this card's folder, no `RUSTFLAGS`):

- `rustfmt --edition 2024 --check crates\wakeshot-windows\src\power\journal.rs`: exit 0 (the file has no child
  modules).
- `cargo test --locked -p wakeshot-windows power::journal::`: 16 passed, 0 failed. The card's 15:
  `load_tells_missing_corrupt_failed`, `save_load_round_trip`, `unknown_keys_are_logged_once`,
  `delete_missing_is_done`, `set_aside_keeps_bytes`, `corrupt_seen_rename_once`, `run_once_set_reports_added`,
  `run_once_clear_missing_is_done`, `dry_run_never_touches_run_once`, `guard_forms_never_set_run_once`,
  `ensure_guard_passes_through`, `no_guard_starts_nothing`, `lock_excludes_second_caller`,
  `lock_abandoned_counts_as_taken`, `lock_timeout_fails_the_run`; added: `outcome_lines_name_the_result`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0 (final run, once
  `wakeshot-core/src/state.rs` stopped failing; see the Log).
- After the tests: `reg query HKCU\Software\Wakeshot-test` finds no key at all; no `%TEMP%\wakeshot-journal-*`
  folder; the real RunOnce value, the real data folder and the power settings were never touched. Build folder
  deleted.

Open items:

- The taken path of `apply_locked` and `check_locked` runs no test yet: the core's `apply` and `check` are still
  `unimplemented!` (`p2-ownership-apply`). `p3-stop-points` and `p3-power-io` drive them; `restore_locked_round_trip`
  is `p3-recovery`'s (card note).
- For `p3-recovery`: `TrayGuard` implements `GuardControl` as above. If the tray builds `FileNet<TrayGuard>` on one
  thread and moves it to the IO thread, `TrayGuard` must be `Send` (`Owned<HANDLE>` is not).
- The four items under "Found outside scope".
