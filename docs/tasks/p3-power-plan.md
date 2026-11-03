# Power plan: the real `PlanIo`, the dry run, the test plan copy

Phase: 3. Status: ready. Needs Haik: no. Real laptop: **yes**: creates a plan copy named "Wakeshot test <pid>", writes
only to it, never makes it active, deletes it; reads Haik's active plan before and after. Only its `real_` tests touch
the
laptop; the orchestrator gives them a real-laptop slot (one at a time across all tracks). Agent: `wakeshot-worker`.
Size: about 110K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6 (the `real_` run only beside cards that are not marked real laptop).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-power-plan\target`.

## Goal

Fill `power::plan`: the Windows implementation of the core's `PlanIo` seam (the apply, the restore routine and the
ownership check run in the core over it), its dry-run form, the `PlanValues` read for `Event::Plan`, and the throwaway
plan copy every real plan test uses. Run section 9's first Phase 3 test: a write to an inactive copy changes only that
copy.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 3, 8, 9).
- `docs/architecture.md`: section 2 "Ported from the spikes" (the plan list rule), section 5 "Power settings" (table,
  "Plan switch"), section 8 (log every settings write with old and new values), section 9 "Windows layer (Phase 3)".
- `docs/research.md`: "Power settings spike" (writes, access check, stored vs applied, plan copy), "Design verify
  checks" (259 ends the plan list).
- `spikes/power-settings/src/power.rs`: GUIDs, `read`, `write`, `set_active`, `access_check*`, `schemes`. Port by
  reading and rewriting; the port's plan list ends only at 259 and fails on any other code.
- After the contracts card: the core's `ownership::io::PlanIo` and `ownership::fake::FakePlans`;
  `crates/wakeshot-windows/src/power/plan.rs` (`PlanSetting`). The contracts card settled it: `PowerApi`, `FakePower`,
  `PowerCall` and the Windows `FAKE_MISSING` are gone; `Win32Power` and `DryRunPower` implement the core's `PlanIo`
  directly, `values()` included, with no local trait (`p1-contracts-split` edit 2).

## Deliverables

`crates/wakeshot-windows/src/power/plan.rs`:

- `Win32Power: PlanIo`: `PowerGetActiveScheme` (free the pointer), `PowerEnumerate` with `ACCESS_SCHEME` until 259
  (any other code is an error, never a short list), `PowerReadACValueIndex` / `DCValueIndex`, `PowerWriteACValueIndex`
  / `DCValueIndex` only through `SettingKey`, `PowerSetActiveScheme`, `PowerSettingAccessCheckEx` per key. Each error is
  a `Failure` naming the call. Each write is logged with the value last read for that key and the new one (the
  routines always read before they write).
- `values()` (the core trait's `PlanIo::values()`): the active plan id and the five values (`PlanSetting`) in one go,
  for `Event::Plan`; the routines never call it.
- `DryRunPower<P: PlanIo>`: reads from `P` (real in the dry-run tray, `FakePlans` in tests); writes and `set_active`
  go to an in-memory overlay that its reads return, each logged; it never calls the inner `write` or `set_active`.
- `TestPlan` (behind `test-support`): `PowerDuplicateScheme` of the active plan, `PowerWriteFriendlyName` "Wakeshot
  test <pid>" (this process's pid), `PowerDeleteScheme` on drop (refused while active); `TestPlan::sweep()` deletes
  only leftovers named "Wakeshot test <pid>" whose pid is not running and that are not active, and runs only in
  `real_` tests (review M7: a sweep must never delete a copy another card is walking). Never `PowerSetActiveScheme`.
  `p6-install-e2e` names its copy "Wakeshot test e2e", which no sweep touches.
- `CopyAsActive` (behind `test-support`): a `PlanIo` over `Win32Power` that reports the copy as the active plan and
  records `set_active` without calling Windows, keeping "in force" as the copy's stored values at the last recorded
  `set_active`. `p3-stop-points` and the harness run the core routines through it.

## Acceptance checks

Unit tests in `power::plan::tests` (no Windows writes):

- `setting_guids_match_the_spike`; `plan_list_ends_only_at_259` (the loop over a scripted call: 259 ends, 5 is an
  error, never a short list); `dry_run_never_writes_inner` (the inner `FakePlans` call log has no write or
  `set_active`); `dry_run_reads_its_own_writes`; `write_log_has_old_and_new`.

Real tests, `#[ignore]`, run with `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1 power::plan::`
in the real-laptop slot (by module, never by `real_` alone: review m15):

- `real_write_to_inactive_copy_changes_only_the_copy`: read Haik's active plan (the five values) before; copy; write
  all three keys on the copy; read back; read the active plan again: unchanged; delete the copy. Section 9's first
  Phase 3 test; record the result in `docs/research.md`.
- `real_plan_list_ends_with_259`, `real_access_check_passes_for_three_keys`, `real_copy_as_active_round_trip`.

Before and after the real run, record in the Log: active plan GUID, lid AC and DC, sleep AC and DC, energy saver DC
(our read and `powercfg /q`); they must be equal, and `powercfg /list` shows no "Wakeshot test <pid>" plan of this run.

Also: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
warnings` for both targets; `cargo test --locked -p wakeshot-windows power::plan::`.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The apply, restore and ownership check (core track, `p2-ownership-*`); the journal file and RunOnce (`p3-journal`);
  making any plan active; Haik's real values.
- Any file but `power/plan.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: implements the core's `PlanIo` (orchestrator decision 17:05).

## Result

What was done, files changed, checks and outcomes, open items.
