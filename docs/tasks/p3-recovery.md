# Recovery: guard mode, the tray's guard control, restore mode

Phase: 3. Status: done. Needs Haik: no. Question 12 (a failed restore starts the tray) is answered with the
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

1. `p3-journal` Found outside scope 3 (a journal mutex made by an elevated process may refuse a Medium open): for
   the guard of an elevated tray it should not matter, because the tray's handle closes when the tray dies and a
   named object goes with its last handle, so the guard makes a fresh mutex. That last part was checked here for
   events only (`docs/research.md`, "Recovery checks"), not for the mutex: inference. Still open for
   `p3-real-checks` while both processes live (an admin `wakeshot restore` beside a Medium tray).
2. `docs/architecture.md` section 5, "Guard": "Tray gone with a journal left: run the restore routine". This card
   narrows it to a journal that names the tray, so the guard leaves a corrupt journal (it names no one) for the
   next sign-in's restore mode, which sets it aside (Result, choice 3). The architecture line could say so. For the
   orchestrator.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:48: worker started; reading the card, the phase file and the APIs of the cards it runs after.
- 2026-11-03 21:02: read done; baseline `cargo build --locked -p wakeshot-tray --tests` passes. Design: one retry
  loop for both modes behind a `RecoveryOs` seam (the guard's wait, a clock with sleep, the tray start);
  `TrayGuard` keeps a leaving guard's event open until that guard ends; the stop event gets an explicit DACL (the
  user's SID) and a Medium label, so a Medium guard opens it even when the tray runs elevated. Writing
  `recovery.rs` now.
- 2026-11-03 21:15: `recovery.rs`, `guard.rs`, `restore.rs` written; 23 unit tests pass. One fix in my own walk: a
  run's scripted failure that a stop pre-empted stayed in the fake's script and hit the next restore; the walk now
  clears it after the stop. A one-off experiment (removed after): a Medium guard child under Explorer opened an
  event a High test made with default security, so I dropped the explicit DACL for the lean default (trigger in the
  `StopEvent` doc).
- 2026-11-03 21:18: `wakeshot-tray/tests/recovery.rs`: 5 exe tests pass; `real_guard_parent_is_explorer` ran once
  with `--ignored`: parent `explorer` pid 12528.
- 2026-11-03 21:21: final checks: rustfmt, clippy for both targets with and without `--all-targets`, both test
  runs, no child or key left. `docs/research.md` "Recovery checks" added. Result written; build folder deleted.

## Result

Status: done (2026-11-03 21:21). 23 unit tests and 5 exe tests pass, plus the ignored Explorer test run once.

### API (for `p3-harness`, `p6-int-tray-mode`), all in `wakeshot_windows::recovery`

- `TrayGuard::new(exe: PathBuf, tray: ProcessRef, log: Arc<dyn Log>)`: `GuardControl` (`ensure`, `stop`), plus
  `running()` and `guard_pid()`. `Send`. The tray passes `std::env::current_exe()` and `process::current()`; the
  harness passes the built tray exe. Each `ensure` that starts a guard makes `Local\Wakeshot.GuardStop.<tray
  pid>.<n>` and runs `<exe> --guard <pid> <start> <event>` through `process::spawn_outside_tree`. A drop stops no
  guard.
- `GuardArgs { tray: ProcessRef, stop_event: String }`: `parse(&[OsString]) -> Result<Self, String>` (exactly three
  arguments, digits only, pid not 0, the event named `Local\Wakeshot.GuardStop.`...), `command()` (`--guard` first).
- `run_guard(&GuardArgs, &mut RecoveryDeps) -> GuardEnd { Stopped, TrayGone(Recovered) }`;
  `run_restore_mode(&mut RecoveryDeps) -> RestoreModeEnd` (= `Recovered`: `Nothing`, `Restored`,
  `OtherOwner(owner)`, `OwnerAlive(owner)`, `Corrupt { set_aside }`, `GaveUp(Result<TrayStarted, WinError>)`). Both
  have `exit_code()`: 0, or `EXIT_FAILED` (1) after a give-up; a bad guard command line is `EXIT_BAD_ARGS` (2).
- `RecoveryDeps { plans, net, names, log, liveness, os }`; `RecoveryOs` (`wait_for_tray`, `now_ms`, `sleep_ms`,
  `start_tray`), real `Win32Recovery::this_exe()` or `::new(exe)`; `wait_for_tray(args, log)`; `give_up(os, log)`,
  the one give-up step; `StopEvent` (`create`, `open`, `set`, `is_set`, `wait`); `open_log(dir, proc)`;
  `RETRY_EVERY_MS` (10 s), `RETRY_FOR_MS` (120 s), `STOP_EVENT_PREFIX`.
- Note for the harness: a guard inherits its starter's environment, and a debug build of the guard reads
  `WAKESHOT_DATA_DIR`. So a harness run with its own `--dir` must set that variable before `ensure`, or its guard
  reads the real data folder. A release guard always reads the real one.

### Design choices (mine, as principal designer)

| # | Choice | Reason | Change it when |
|---|---|---|---|
| 1 | One retry loop for both modes. Each try: a look at the journal without the lock to learn its owner, then `restore_locked` with that owner as `expect_owner` | One rule; the routine checks the owner again under the lock | Never |
| 2 | A deadline of 120 s from the first try on the `RecoveryOs` clock, a try every 10 s: 13 tries when each is quick; tries that wait on the journal mutex make fewer | "For 2 minutes" is time, not a count | A report shows restores that would have worked after 2 minutes of mutex waits |
| 3 | The guard leaves a corrupt journal; restore mode sets it aside | The card's rule: the guard restores only a journal that names its tray, and a corrupt one names no one. The RunOnce value is still set then, so the next sign-in sets it aside; at sign-in Windows already deleted the value, so restore mode must | A report shows a corrupt journal left for long |
| 4 | Restore mode checks the owner's liveness at every try (Unknown is a failed try); the guard checks none | The guard's wait already says the tray is gone | Never |
| 5 | The guard never leaves its tray unguarded through a failure of its own: a stop event it cannot open means a wait on the tray alone; a tray it cannot open means a look once a second (event, then liveness) | Safety first; the cost is at worst a guard that stays until its tray ends | A log shows guards that wait on the tray alone piling up |
| 6 | The tray's handle comes first in the wait | A tray that ends right after it set the event still has its journal checked | Never |
| 7 | `TrayGuard` keeps a leaving guard (event set, still running) on a list, with its event open, until it ends | A guard told to stop before it opened its event still leaves; otherwise it would stay until the tray ends, one extra process per quick hold cycle | Never |
| 8 | On any doubt (a failed wait on a guard's handle or event) `TrayGuard` starts a new guard | Two guards only restore twice, one after the other under the journal mutex | Never |
| 9 | Stop event: manual reset, default security; a name already taken (a reused pid) moves to the next `n`, 8 tries | A set before the guard opens it still counts. A Medium guard opened an event an elevated test made with default security (research, "Recovery checks"), so no DACL code | A guard logs that it cannot open its stop event: then a DACL for the user's SID |
| 10 | The give-up starts the tray through `spawn_outside_tree` | The tray then runs under Explorer, as a Start menu start would, never in the guard's or restore mode's tree | Never |
| 11 | No data folder: exit 1 with no log; a log that cannot open: the mode runs with no log | A mode that cannot log must still restore | Never |
| 12 | The modes log in area `recovery`; the routine's own lines stay in `journal` | Each mode's end is one line a reader and the exe tests find: "guard for tray pid N: ..." and "restore mode: ..." | Never |

### Files changed

`crates/wakeshot-windows/src/recovery.rs` (filled, with its tests); `crates/wakeshot-tray/src/guard.rs`,
`crates/wakeshot-tray/src/restore.rs` (wiring only); `crates/wakeshot-tray/tests/recovery.rs` (new);
`docs/research.md` (new section "Recovery checks", addition only); this card.

### Checks (2026-11-03, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`)

- `rustfmt --edition 2024 --check` on the four owned files: exit 0.
- `cargo clippy --locked -p wakeshot-windows -p wakeshot-tray --all-targets --target <t> -- -D warnings` and the same
  without `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0 (the tray's
  ARM64 run needs no clang yet: `velopack` is not in).
- `cargo test --locked -p wakeshot-windows recovery::`: 23 passed, 1 ignored (the child role). The card's tests:
  `guard_args_parse`, `guard_exits_on_stop_event` (the real wait and a real event; also a set made before the guard
  opens it), `guard_restores_when_tray_dies` (a real child ended through its handle; also a tray already gone),
  `guard_leaves_journal_of_another_owner`, `guard_retries_then_gives_up_and_starts_tray` (13 tries, 12 pauses, one
  tray start; also a failed tray start), `restore_mode_missing_journal`, `restore_mode_live_owner_exits`,
  `restore_mode_unknown_owner_retries`, `restore_mode_never_writes_run_once` (restored, unknown entry kept, retried,
  gave up, taken over, set aside), `tray_guard_starts_new_when_old_is_leaving`, `tray_guard_stop_sets_event`,
  `restore_locked_round_trip` (both `FileNet` forms, test key `HKCU\Software\Wakeshot-test\<pid>\<test name>`),
  `guard_stop_points`, `restore_mode_stop_points`. Added: `guard_stops_retrying_when_another_tray_takes_over`,
  `guard_retry_finishes_a_failed_try`, `guard_leaves_a_corrupt_journal`, `guard_with_no_journal_does_nothing`,
  `restore_mode_sets_a_corrupt_journal_aside`, `open_log_without_a_folder_still_logs_nowhere`,
  `stop_event_create_open_set`, `tray_guard_is_send`, `tray_guard_replaces_a_guard_that_ended`. The two
  `tray_guard_` tests start this test exe's guard role outside the tree (`outside_tree: true`), so a Medium guard
  waited on the High test process and its event, and left on `stop` (exit 10).
- Stop-point walks (track note 4): two starting states (all ours; mixed, with marks and an unknown key entry), each
  as a clean run and as a run whose first try fails at its first write (so stops also fall in the retry), for the
  guard and for restore mode. A stop at every call of the run; then the tray's startup restore, once whole and once
  with each of its calls failing in turn. Guard: every held value has a known entry, and while one exists RunOnce is
  set and the guard runs; after the next restore every value is back, and for stops up to the journal delete
  RunOnce is gone and the guard told to stop. Restore mode: Windows deleted RunOnce before it ran, so its first
  point is "every held value has a known entry, and RunOnce is never set".
- `cargo test --locked -p wakeshot-tray`: 1 unit test and 5 exe tests passed, 2 ignored:
  `guard_exits_when_its_tray_dies` (left 11 to 13 ms after the end, logged "no journal"),
  `guard_exits_stopped_when_its_event_is_set` (10 ms, "stopped"), `guard_with_a_bad_command_line_exits_2`,
  `restore_without_journal_exits_at_once` (exit 0 in 26 ms; 0.95 s on the first start of a new exe),
  `tray_guard_runs_the_real_guard_until_stopped` (the real exe under Explorer).
- `cargo test --locked -p wakeshot-tray --test recovery -- --ignored`: `real_guard_parent_is_explorer` passed, parent
  `explorer` pid 12528.
- After the runs: none of the 21 logged child pids runs; no process runs from the build folder; no
  `HKCU\Software\Wakeshot-test` key; no real RunOnce `WakeshotRestore` value; no `%LocalAppData%\Wakeshot` folder.
  No power plan was read or written by these tests. Build folder deleted.

### Open items

- Not tested: the guard's fallback when its tray cannot be opened (needs a token without SeDebugPrivilege); that
  `WaitForMultipleObjects` returns the lowest index when both handles are signaled (Microsoft's documented rule).
- Task Manager grouping of the guard (checklist 4) and RunOnce after a sign-out (checklist 5, which runs the restore
  mode built here): `p3-haik-checks`.
- No contract change needed: the mode files use only existing APIs.
- The two items under "Found outside scope".
