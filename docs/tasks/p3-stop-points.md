# Stop points: the core routines against the real `PlanIo` and `SafetyNet`

Phase: 3. Status: done. Needs Haik: no. Real laptop: **yes**: every test writes to a plan copy named "Wakeshot
test <pid>" (`TestPlan`, review M7), never active, deleted at the end; reads Haik's active plan before and after. The
`real_` tests run in a real-laptop slot (one at a time across all tracks). Agent: `wakeshot-worker`. Size: about 110K
tokens.
Runs after: `p3-power-plan`, `p3-journal`, and the core track's `p2-ownership-apply` (the routines). Parallel with:
`p3-recovery`, `p3-power-io`, `p3-harness` and any card of tracks 2, 4, 5, 6 (the `real_` run only beside cards that
are not marked real laptop).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-stop-points\target`.

## Goal

The stop-point test the plan card asks for, run against the real Windows implementations: stop between every two
steps of Apply, of each restore path and of the ownership check, then run the next start's restore, and check that no
value Wakeshot still holds is dropped and that the journal, RunOnce value and guard stay until the restore succeeds.
The core track runs the same walk against its fake; this card proves the real parts behave as the fake says.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 3, 4).
- `docs/architecture.md`: section 5 "Apply", "Ownership", "Restore routine"; section 9.
- `docs/tasks/p1-design-verify-fixes.md`, Log 16:35: the stop-point walk (E1, E2, E3, F; A1 to A6; R1 to R6).
- The core track's `p2-ownership-restore` and `p2-ownership-apply` Results (the routines' public API and their own
  stop-point cases).
- After the cards above: `power/plan.rs` (`Win32Power`, `TestPlan`, `CopyAsActive`), `power/journal.rs` (`FileNet`,
  the locked entry points).

## Deliverables

`crates/wakeshot-windows/src/power/stop_points.rs` (test only):

- `StopAt` wrappers around a `PlanIo` and a `SafetyNet` that share one call counter and panic with a marker at call
  k (dev and test builds unwind; the test catches only that marker). The real parts below them are `CopyAsActive`
  over a `TestPlan` and `FileNet` in a temp data folder with a test RunOnce key and a recording `GuardControl`.
- For each routine and starting state, a loop: run once to count the calls N; for k in 0..N: fresh copy with the
  user's values (lid AC 1, lid DC 1, sleep DC 600, or the copy's own), the starting state, stop at k, then the next
  start's restore with fresh real parts (all plans, owner Gone), then the checks of track note 4 against the copy's
  stored values, its "in force" values (`CopyAsActive`), the journal file on disk, the RunOnce test value and the
  recorded guard calls; also with a failure scripted into that next restore (a wrapper that fails one call).
- Routines and starting states: Apply (fresh; E1, E2 and E3 left by a failed restore; a released key's entry on the
  plan, track note 6); restore with no plan and with one plan; the ownership check after a user change; an unknown-key
  entry present in every case (correction 12: never touched, never blocks the clean-up).
- Added by the orchestrator (`p2-ownership-restore` Found outside scope 2): "RunOnce is gone; the guard is told to
  stop" holds only for stops before the clean-up's journal delete. A stop between that delete and the RunOnce clear
  (or the guard's stop) leaves them, and the next restore finds no journal and does nothing (walk R6, accepted as
  harmless). Assert it that way, as the core's `a_stop_at_any_call_keeps_every_held_value_journaled` does.

## Acceptance checks

- `real_stop_points_apply`, `real_stop_points_restore`, `real_stop_points_check`, `real_stop_points_unknown_key`:
  `#[ignore]`, run with `cargo test --locked -p wakeshot-windows real_stop_points -- --ignored --test-threads=1`; all
  pass. Record N for each routine and the run time.
- Before and after the run, Haik's values (active plan; lid AC and DC; sleep AC and DC; energy saver DC) are equal;
  `powercfg /list` shows no "Wakeshot test <pid>" plan of this run; no `HKCU\Software\Wakeshot-test` key of this
  run's pids (the test RunOnce key is a `p3-store` `ScratchKey`, review M8); no temp data folder left.
- A deliberate break, in a scratch copy only: make `FileNet::clear_run_once` run before the journal delete (or skip
  one write), see the test fail at the right stop point, record it, revert. Nothing of it stays in the tree.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo test --locked -p wakeshot-windows power::stop_points::` (the `real_` tests stay
  ignored there).

A failure that shows a bug in a core routine or in another card's file: record the stop point, the state and the call
log under "Found outside scope", set Status `blocked`; do not fix it here.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The routines (core track); `PlanIo` and `SafetyNet` code (`p3-power-plan`, `p3-journal`); the real guard process
  (`p3-recovery`).
- Any file but `power/stop_points.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

1. `docs/phases/03-windows-power.md` track note 4 says that after the next restore, "for stops before the clean-up's
   journal delete, RunOnce is gone and the guard is told to stop". That does not hold for a fresh apply stopped after
   `set_run_once` or `ensure_guard` and before its first journal save: no journal exists yet, so the next restore does
   nothing and the RunOnce value and the guard stay (seen in the real walk, 21:41). It is the same harmless kind as walk
   R6 (both find no journal and do nothing), and the core's own apply walk does not assert the release at all.
   Proposed text: "...after the next restore succeeds: ...; whenever the next restore finds a journal, RunOnce is gone
   and the guard is told to stop. It finds none after a stop between the clean-up's journal delete and its last call
   (walk R6), or after a fresh apply stopped before its first journal save; what the stopped run added or had not
   cleared then stays, harmless." The walk asserts it that way (`assert_released`).
2. No bug found in a core routine or in `plan.rs` / `journal.rs`: the real parts behaved as the fake says at all
   3142 cases.

## Log

- 2026-11-03 17:17: card written by `p3-plan-windows-power` (orchestrator decision 17:05: this track runs the
  stop-point test against the real implementations).
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" item 8: keep at least one start
  with E1 as the only entry on its plan. With E2 or E3 beside it, the next restore's `set_active` for them hides a
  dropped E1.
- 2026-11-03 20:43, orchestrator: Haik's own battery sleep is now 4 h (DC 14400, read 20:41), not 600 s, so a copy
  of his plan starts with DC sleep 14400. Read the copy's values live; never hard-code 600.
- 2026-11-03 21:24, worker: started. Real-laptop slot given by the orchestrator (21:23). Reading the card's inputs.
- 2026-11-03 21:31, worker: Haik's values before any test (`powercfg /list`, `/getactivescheme`, `/qh`): Balanced
  `381b4222-...` active and the only plan; lid AC 1, DC 1; sleep AC 10800, DC 14400; energy saver DC 30. No
  `HKCU\Software\Wakeshot-test` key, no `%TEMP%\wakeshot-*` folder.
- 2026-11-03 21:39, worker: `power/stop_points.rs` written (wrappers, 10 starts, the walk with a next restore and a
  failing next restore per stop, 4 unit tests). Unit tests pass, clippy x64 `--all-targets` clean. Next: the first
  `real_` run (apply).
- 2026-11-03 21:41, worker: first real run failed in the test itself, not a routine: my release check wanted a
  guard stop after a fresh apply stopped before its first journal save, where the next restore finds no journal and
  does nothing. Rule fixed (a found journal: RunOnce gone and guard told to stop; none: only the cases named in
  `assert_released`). Then all four `real_stop_points_*` passed, 76.7 s.
- 2026-11-03 21:42, worker: Haik's values after the run (`powercfg`): same as 21:31. No `Wakeshot-test` key, no
  `%TEMP%\wakeshot-*` folder, only Balanced listed. Next: the deliberate break in a scratch copy.
- 2026-11-03 21:44, worker: deliberate break done in a scratch copy (own build folder): `FileNet::delete_journal`
  and `FileNet::clear_run_once` swapped by name, so the clean-up clears RunOnce before it deletes the journal. All
  four `real_` tests failed: restore walk (fail variants off in the copy) at "restore all plans after an apply,
  stop at call 13 (clear_run_once)": "a known entry without RunOnce"; with them on, first at stop 0 with the next
  restore failing at its call 13 (clear_run_once), same message; apply at "fresh apply, stop at call 11, next
  restore fails at 7 (clear_run_once)"; check the same way; unknown key: the file with the unknown entry was gone
  after the restore (the swapped call deleted it). Scratch copy and its build folder deleted; tree files unchanged.
- 2026-11-03 21:46, worker: final checks. rustfmt check, clippy (both targets, with and without `--all-targets`),
  `cargo test -p wakeshot-windows` (229 passed, 17 ignored) all clean. Final real run 21:46:48 to 21:47:46, all four
  passed in 57.4 s; Haik's values read before and after it, the same as 21:31. Build folder and scratch files deleted.
  `docs/research.md`: new section "Stop point checks" (addition only).

## Result

Status: done (2026-11-03 21:48).

What was done: `crates/wakeshot-windows/src/power/stop_points.rs` (test only, the only code file changed):

- `Script` (one call counter shared by both wrappers, with a call log) and `StopAt<'_, T>`, a `PlanIo` and a
  `SafetyNet` over the real part: at the scripted index it panics with `StopMarker` before the call reaches the real
  part, or returns a scripted failure (`SCRIPTED`, code 31) without it. `until_stop` catches only `StopMarker`; a
  panic hook silences only that payload.
- `Rig` (per test): one `TestPlan` copy of the active plan, the user's values read live from it (today 1, 1, 14400),
  a `ScratchKey` RunOnce key, `Names::test`. `World` (per case): the copy set to the start's stored and in-force values
  through `CopyAsActive`, a fresh temp data folder with the start's journal, RunOnce and a `RecordingGuard` as the start
  has them, `FileNet::tray` over them.
- 10 starts. Apply: fresh; E1 alone (writes only its key, so E1 is the only entry on the plan for the whole walk); E1,
  E2, E3; lid AC released (track note 6). Restore: all plans after an apply; all plans, E1 alone; E1, E2, E3 plus an
  entry on a deleted plan, with all plans and with one plan. Check: after a user change (beside a marked E1 and a held
  key); after the user changed the last key (the check's clean-up).
- For each start: a clean run (counts N, checks the start did what it says), then for every k in 0..N and no stop:
  stop at k, track note 4's first point, the next start's restore (fresh `FileNet` and guard, owner on disk as
  restore mode passes it), the second point and the release rule; then for every call j of that restore: the same
  stop, the restore failing at j (first point still holds), a retry (second point).
- Tests: `real_stop_points_apply`, `_restore`, `_check`, `_unknown_key` (the 10 starts again, each with an unknown
  `sleepAc` entry on the copy), all `#[ignore]`; unit tests `both_wrappers_share_one_counter_and_stop_before_the_call`,
  `a_scripted_failure_skips_only_that_call`, `only_the_stop_marker_is_caught`,
  `the_count_matches_the_routines_own_calls` (against the core's `FakePlans`).

N per routine and run time (final run, debug build):

| Test | Start | N | Next restore calls (max) | Cases | Time |
|---|---|---|---|---|---|
| apply | fresh | 19 | 15 | 155 | 1.6 s |
| apply | E1 alone | 12 | 11 | 141 | 1.4 s |
| apply | E1, E2, E3 | 16 | 15 | 261 | 5.0 s |
| apply | lid AC released | 14 | 15 | 236 | 4.2 s |
| restore | all plans after an apply | 15 | 15 | 200 | 3.6 s |
| restore | all plans, E1 alone | 9 | 9 | 77 | 0.8 s |
| restore | all plans, E1, E2, E3, deleted plan | 14 | 14 | 177 | 3.1 s |
| restore | one plan, E1, E2, E3, deleted plan | 12 | 14 | 178 | 3.0 s |
| check | user changed lid DC | 5 | 13 | 84 | 1.3 s |
| check | user changed the last key | 6 | 7 | 39 | 0.4 s |

Totals: apply 12.2 s, restore 10.6 s, check 1.6 s, unknown key 32.9 s (same N; 1594 cases), all 3142 cases in
57.4 s.

Design choices (mine, as principal designer):

| # | Choice | Reason | Change it when |
|---|---|---|---|
| 1 | One plan copy per test, reset per case, not a new copy per case | The same start state with a third of the Windows calls and no create and delete of 3142 plans on Haik's PC; a routine writes only the three keys, and every case writes all three | A routine can change any other plan setting |
| 2 | The next restore keeps the stopped run's `CopyAsActive`; `FileNet` and guard are fresh | "In force" is Windows state, not process state; a fresh `CopyAsActive` takes the stored values as in force, which would hide a dropped E1 | `CopyAsActive` can be built with given in-force values |
| 3 | Next restore = restore mode's call (`expect_owner` = the owner on disk) over the tray form of `FileNet` | Records the guard stop; the routine never sets RunOnce, so the forms differ only in the guard | A restore call differs between `tray` and `restore_only` |
| 4 | The unknown key is in every case of a fourth test, not in the other three | With one in the file the clean-up never deletes, so the delete and the R6 window would go unwalked | A reviewer wants it in all four (then add a delete-path test) |
| 5 | A failure at every call of the next restore, for every stop, plus a retry | 57 s in all is cheap, and it shows the retry finishes | The run grows past a few minutes |
| 6 | Release rule by "the next restore found a journal" (then RunOnce gone and guard stop), with exact expectations for the no-journal cases | Matches the routine: a missing journal makes no other call; covers the fresh-apply window (Found outside scope 1) | Track note 4 gets other text |
| 7 | A user value at the target gets a stand-in on the copy (lid 1, sleep DC 600) | Else the apply skips that key and the walk shows nothing for it; not needed today | Never (only the copy is written) |
| 8 | A user's Settings change modelled as the value stored and the whole plan in force | `CopyAsActive` notes say a test must model it; not verified on Windows | A Windows test shows otherwise |

Checks (2026-11-03, `CARGO_TARGET_DIR` = this card's folder, no `RUSTFLAGS`):

- `cargo test --locked -p wakeshot-windows real_stop_points -- --ignored --test-threads=1 --nocapture`: 4 passed
  (242 filtered out, so the filter matched only these four), 57.4 s.
- Haik's values before (21:31, 21:46:48) and after (21:42, 21:47:46), by `powercfg /list`, `/getactivescheme` and
  `/qh`: Balanced active and the only plan, lid AC 1 and DC 1, sleep AC 10800 and DC 14400, energy saver DC 30; the
  same each time. Each test also compares the active plan's five values before and after itself and checks its copy is
  gone from the plan list. No "Wakeshot test" plan, no `HKCU\Software\Wakeshot-test` key, no `%TEMP%\wakeshot-*`
  folder after the runs. Haik's plan was never written: every write went through `CopyAsActive`, which refuses any
  other plan.
- Deliberate break (scratch copy, own build folder, both deleted): `FileNet::delete_journal` and `clear_run_once`
  swapped. All four failed; the stop walk at "restore all plans after an apply, stop at call 13 (clear_run_once)": a
  known entry without RunOnce (Log 21:44). Nothing of it is in the tree.
- `rustfmt --edition 2024 --check crates\wakeshot-windows\src\power\stop_points.rs`: exit 0.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- `cargo test --locked -p wakeshot-windows power::stop_points::`: 4 passed, 4 ignored. Whole package: 229 passed,
  17 ignored.
- Build folder deleted.

Open items:

- Found outside scope 1: track note 4 text (the orchestrator owns the phase file).
- For the power safety review: design choices 2, 4 and 6.
