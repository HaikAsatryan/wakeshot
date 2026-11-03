# Power plan: the real `PlanIo`, the dry run, the test plan copy

Phase: 3. Status: done. Needs Haik: no. Real laptop: **yes**: creates a plan copy named "Wakeshot test <pid>", writes
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

1. Track note 2 ("Checks while other cards run") says a card checks only its own package, but `cargo clippy -p
   wakeshot-windows -- -D warnings` also fails on any warning in `wakeshot-core` (seen 20:26 with another card's
   unfulfilled `#[expect(dead_code)]` in `state.rs`; `docs/research.md` "Power plan checks"). So a core card's
   half-done file blocks every power track card's clippy check, not only a compile error does. Suggestion for the
   orchestrator: say so in the track note, so workers wait and retry instead of hunting for a cause in their file.
2. Transient, gone at 20:29: `power/journal.rs` imported `FILE_SHARE_*` from `Win32::Foundation` (fixed by its
   card), and `wakeshot-core` `sim.rs` and `core.rs` did not compile for a few minutes (core cards mid-edit).

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: implements the core's `PlanIo` (orchestrator decision 17:05).
- 2026-11-03 20:10: worker started (orchestrator: status running, real-laptop slot given, owned files
  `power/plan.rs` and this card).
- 2026-11-03 20:15: Haik's values before any code ran (`powercfg /getactivescheme`, `/list`, `/qh` on the GUIDs):
  active Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`, the only plan; lid AC 1, DC 1; sleep AC 10800, DC 600;
  energy saver DC 30 (AC 0). Our own read follows with the real run. Note: `powercfg /q` with the aliases LIDACTION
  and ESBATTTHRESHOLD prints no value here; `/qh` with the GUIDs does.
- 2026-11-03 about 20:20: first full version of `power/plan.rs` written (Win32Power, DryRunPower, TestPlan,
  CopyAsActive, 9 unit tests, 4 `real_` tests); building next.
- 2026-11-03 about 20:22: lib clippy clean (x64 and ARM64 without `test-support`, x64 with it). Test build blocked by
  other cards' half-done files: first `power/journal.rs` (`FILE_SHARE_*` imported from `Foundation`, fixed by its
  card within minutes), then `wakeshot-core/src/sim.rs` (`state::KEEP_AWAKE_FILE`, `TICKS_AT_ONE_MOMENT` missing).
- 2026-11-03 20:26: core builds again; `cargo test --locked -p wakeshot-windows power::plan::`: 9 passed, 4 ignored.
  `clippy --all-targets -D warnings` still fails in `wakeshot-core/src/state.rs` (unfulfilled `#[expect(dead_code)]`,
  another card mid-edit): clippy's workspace wrapper lints every workspace crate in the build, so it fails mine too.
  Waiting and retrying. Before the real run (20:26:44, `powercfg`): same as 20:15, on AC (battery status 2).
- 2026-11-03 20:27:02 to 20:27:06: real run, `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1
  --nocapture power::plan::`: 4 passed in 0.15 s, elevated (High) shell. Our read before
  (`real_write_to_inactive_copy_changes_only_the_copy`): active `381b4222-...`, lid AC 1, DC 1, sleep AC 10800,
  DC 600, saver DC 30, the same as `powercfg /qh` in the test. Copies `50df20c5-...`, `15920ad3-...`,
  `09f0c480-...`, one per test, each "Wakeshot test <pid>"; the write test wrote lid AC 1 to 0, lid DC 1 to 0, sleep
  DC 600 to 0 on its copy; our read and `powercfg /qh` saw each on the copy; the active plan stayed Balanced with
  the same five values (our read and `powercfg`). Sweeps deleted nothing (no leftovers).
- 2026-11-03 20:27:14, after (`powercfg /getactivescheme`, `/list`, `/qh`): active Balanced, the only plan; lid AC 1,
  DC 1; sleep AC 10800, DC 600; saver DC 30. Equal to before. No "Wakeshot test" plan in `/list`; none of the three
  copies' registry keys under `HKLM\...\Power\User\PowerSchemes` exists.
- 2026-11-03 20:29: the other cards' files compile clean again; all checks of this card pass (Result). Build folder
  deleted. Status done.

## Result

Done 2026-11-03 20:29. 9 unit tests and 4 `real_` tests, all pass. Haik's values before (20:15 and 20:26) and after
(20:27) are the same: Balanced active and the only plan, lid AC 1 and DC 1, sleep AC 10800 and DC 600, energy saver
DC 30. No "Wakeshot test" plan is left.

Design choices (each the worker's, with the reason and when to change it):

1. `Win32Power::new(log: Arc<dyn Log>)`: it lives on one thread for the whole run, so it owns a shared log handle
   instead of a borrowed one. Change it when a caller cannot give an `Arc`.
2. Write log lines: `wrote lidDc on plan <guid>: 1 to 0`; a failure is a Warn line `write ... failed: <call> code
   <n>`; `set active plan <guid>` the same way. The old value is the stored value last read or written through this
   object for that plan and key ("not read" if none); no extra read. Change it when a caller writes without reading.
3. `plans()`: only 259 ends the list; any other code, a list that ends at index 0 (Windows always has an active
   plan), a GUID of the wrong size, or more than 4096 plans is an error (code 259 for the empty list, 13
   `ERROR_INVALID_DATA` for the others, our choice). Change it when a real system shows an empty list as valid.
4. `DryRunPower`: reads pass through to `P`; a write keeps `(base, value)` where base is the real stored value at
   write time (one extra read of `P`); a read returns the dry-run value only while the real value still equals
   base, else the real value wins and the entry is dropped and logged, as a user's Settings change overwrites a real
   write; writing the real value back drops the entry. `set_active` is logged and recorded
   (`set_active_calls()`), but `active_plan` answers from `P`: the routines only `set_active` the plan they just
   read as active, and a real plan switch must stay visible to a dry-run tray. Change it when a caller sets a plan
   active that is not the active one.
5. `CopyAsActive<P = Win32Power>` (generic so its unit tests run on `FakePlans`; built for real with
   `CopyAsActive::over(&TestPlan, Win32Power)`): `active_plan` is the copy with no Windows call; a `write` or
   `set_active` for any plan but the copy is refused (`CopyAsActive::write` / `::set_active`, code 5) before the
   inner object sees it, so a journal entry on the user's plan can never be written through it; `set_active` reads
   the copy's three stored values into "in force" (`in_force(key)`) and counts (`set_active_count()`); a failed
   read records nothing. `values()` needs all five values of the copy, so it goes through a small `ValuesOf` trait
   (Win32Power and, in tests, `FakePlans`). Change it when a test needs two copies (allow a set of plans).
6. `TestPlan::new()` duplicates the active plan and renames it "Wakeshot test <pid>"; a failed rename deletes the
   copy (drop). Drop deletes unless the copy is the active plan (then it says so on stderr and keeps it).
   `TestPlan::sweep()` deletes only plans named exactly "Wakeshot test <digits>" whose pid is not this process,
   that are not active, and whose pid `SystemLiveness::start_of_pid` reports `NotRunning` (Running and Unknown
   keep it). A name it cannot read is kept.

What was done:

- `crates/wakeshot-windows/src/power/plan.rs` filled (the only code file changed). `PlanSetting` keeps its contract
  shape and gains `ALL` and `name()`. `Win32Power: PlanIo` makes the calls of the card, each failure a `Failure`
  naming the Windows call. `values()` reads the active id once, then the five values from that plan. `DryRunPower<P:
  PlanIo>`, `TestPlan` and `CopyAsActive` as in the choices above; the last two and `ValuesOf` are behind
  `test-support` (`#[cfg(any(test, feature = "test-support"))]`), so no shipped exe can create or delete a plan.
- Section 9's first Phase 3 test ran and passed: a write to an inactive copy changes only that copy (stored values;
  `docs/research.md` "Power plan checks").
- Docs: `docs/research.md` "Power plan checks" (added); this card.

Public API for later cards (`p3-power-io`, `p3-stop-points`, `p3-recovery`, `p3-harness`, `p4-cli-restore`,
`p6-hooks`, `p6-int-tray-mode`), all in `wakeshot_windows::power::plan`:

- `Win32Power::new(Arc<dyn Log>)`; `DryRunPower::new(inner, Arc<dyn Log>)`, `.set_active_calls()`.
- `TestPlan::new() -> Result<TestPlan, WinError>`, `.guid()`, `TestPlan::name_for(pid)`, `TestPlan::sweep() ->
  Result<Vec<Guid>, WinError>` (only from `real_` tests).
- `CopyAsActive::over(&TestPlan, Win32Power) -> Result<CopyAsActive, Failure>`, `.copy()`, `.in_force(key)`,
  `.set_active_count()`.
- Notes: `CopyAsActive` refuses writes to any plan but its copy, so a stop-point state with entries on a second
  plan needs a second copy (choice 5's trigger). It keeps "in force" only for the three keys; a user change that is
  "in force at once" must be modelled by the test (write the copy through another `Win32Power`, then a recorded
  `set_active`), since a stored write alone does not move `in_force`. A dry-run write makes one extra inner read, so
  index scripts on an inner `FakePlans` count it.

Checks (20:26 to 20:29):

- Unit tests, `cargo test --locked -p wakeshot-windows power::plan::`: 9 passed (`setting_guids_match_the_spike`,
  `plan_list_ends_only_at_259`, `dry_run_never_writes_inner`, `dry_run_reads_its_own_writes`,
  `write_log_has_old_and_new`, `copy_as_active_reports_the_copy_and_keeps_in_force`,
  `copy_as_active_refuses_other_plans`, `test_pid_takes_only_digit_names`,
  `sweep_picks_only_dead_inactive_test_copies`), 4 ignored.
- Real tests, `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1 power::plan::`: 4 passed
  (`real_write_to_inactive_copy_changes_only_the_copy`, `real_plan_list_ends_with_259`,
  `real_access_check_passes_for_three_keys`, `real_copy_as_active_round_trip`). Before and after values: Log 20:15,
  20:26, 20:27; equal, by our read and by `powercfg`; no copy left.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/power/plan.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean (20:29). Also clean
  with `--features test-support` on x64 (20:22).

Open items:

- The access check ran only elevated (High); the Medium result for the same keys is the spike's (research, "Power
  settings spike"). The executor or `p3-real-checks` run at Medium.
- Not tested: that activating a plan later applies its stored values (the second "no apply" point under
  architecture "Plan switch"); `p3-real-checks` makes a copy active and can check it.
- The power safety review covers this file (`docs/phases/02-06-lighter.md`, "Reviews").
