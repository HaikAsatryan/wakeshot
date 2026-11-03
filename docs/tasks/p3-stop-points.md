# Stop points: the core routines against the real `PlanIo` and `SafetyNet`

Phase: 3. Status: ready. Needs Haik: no. Real laptop: **yes**: every test writes to a plan copy named "Wakeshot
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

## Log

- 2026-11-03 17:17: card written by `p3-plan-windows-power` (orchestrator decision 17:05: this track runs the
  stop-point test against the real implementations).
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" item 8: keep at least one start
  with E1 as the only entry on its plan. With E2 or E3 beside it, the next restore's `set_active` for them hides a
  dropped E1.
- 2026-11-03 20:43, orchestrator: Haik's own battery sleep is now 4 h (DC 14400, read 20:41), not 600 s, so a copy
  of his plan starts with DC sleep 14400. Read the copy's values live; never hard-code 600.

## Result

What was done, files changed, checks and outcomes, open items.
