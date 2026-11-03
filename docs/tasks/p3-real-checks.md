# Real checks: the agent's checks on the test laptop

Phase: 3. Status: ready. Needs Haik: no (best while he is not using the laptop: the screen may flicker during the
cycles). Real laptop: **yes**: real plan tests on a copy, a copy made active for 50 cycles, a guard and RunOnce run
in the real data folder. Runs alone (no other real-laptop card in any track at the same time). Agent:
`wakeshot-worker`. Size: about 70K tokens.
Runs after: `p3-harness`, `p3-stop-points` (and through them `p3-power-plan`). Parallel with: cards of any track that
are not marked real laptop.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-real-checks\target`.

## Goal

Run every check of this track that needs the real laptop but not Haik, and record each fact with how it was checked.

## Read first

- `CLAUDE.md` ("Test laptop"); `docs/phases/03-windows-power.md` (track note 3, "Manual checklist").
- `docs/architecture.md`: section 9 (the plan copy rule, checklist 11 and 15), section 5 "Plan switch" (the two points
  not tested).
- `docs/tasks/p3-harness.md` Result (how to run each subcommand).

## Deliverables

Card Log and Result; a new section in `docs/research.md`. No code. Steps, in order:

1. Before: record Haik's values (our read and `powercfg /q` for lid, sleep, energy saver; active plan), `powercfg
   /list` (no "Wakeshot test"), no `%LocalAppData%\Wakeshot\journal.json`, no `WakeshotRestore` RunOnce value.
2. `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1 --skip real_lock_locks power:: process::
   session:: recovery::` and `cargo test --locked -p wakeshot-tray --test recovery -- --ignored` (the plan tests, the
   stop points against the real parts and the environment tests of review M9, once more on the finished tree). Filter by
   this track's modules, never by `real_` alone: `real_` with `--ignored` also runs any other track's ignored test with
   `real_` in its name (review m15, checked 2026-11-03).
3. `medium-probe` started with Explorer as its parent (as the Phase 0 spikes did): Medium integrity and the `Global\`
   mutex created (checklist 11, first part; architecture section 4 "not tested").
4. `fake-tray` in the real data folder; the agent ends it by its PID (a crash, not End task): the real guard restores
   the copy's entries; check the copy's values, journal gone, RunOnce gone, guard exited; delete the copy.
5. `cycles 50` (checklist 15), while no other card that takes power requests runs (any card whose tests take a real
   request, such as `p3-requests`' `request_sets_and_clears`; review B2): before and after each cycle read display
   brightness (`Get-CimInstance -Namespace root/wmi WmiMonitorBrightness`); after the run read the System and
   Application logs for
   the run's window (`Get-WinEvent`, errors and warnings, Kernel-Power, display); record the `PowerSetActiveScheme`
   times. Then `cleanup`.
6. After: the values of step 1 again; they must be equal; no copy, no journal, no RunOnce value. If anything differs,
   put it back to the step 1 values, check again, and record it.

## Acceptance checks

- Each step's outcome in the Log with the time (`Get-Date`).
- Step 6 equal to step 1.
- `docs/research.md` section "Power track real checks": each fact with how it was checked; "not verified" where a
  step could not show it.

## Out of scope

- Anything that needs Haik (`p3-haik-checks`); fixing code (record failures for the orchestrator).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
