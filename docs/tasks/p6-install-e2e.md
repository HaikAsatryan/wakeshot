# Install, update and uninstall Wakeshot on the test laptop

Phase: 6. Status: ready. Needs Haik: no (silent per-user installs need no admin). Agent: `wakeshot-worker`. Size:
about 130K tokens.
Real laptop: yes. It installs Wakeshot, writes the real `HKCU` Run value and user Path (through the hooks), writes a
test RunOnce value, and uses a throwaway power plan copy that is never made active. The orchestrator runs it alone
among the real-laptop cards of all tracks, and never next to a card that writes power settings.
Runs after: `p6-velopack-dependency`, `p6-pack-and-release`, `p6-hooks`; the power track's `p3-power-plan` (its first
test shows that a write to an inactive copy leaves the real plan alone, `architecture.md` section 9). Parallel with:
cards that are not marked real laptop, except those that use the dry-run data folder
(`%LocalAppData%\Wakeshot\dry-run`, which step 1 moves away and the uninstall hook deletes): the built-exe `--dry-run`
checks of `p4-cli-run`, `p4-cli-commands` and `p4-cli-restore`, and the `p4-live-*` harness (review m11).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-install-e2e\target`; packs and logs in `<scratchpad>\p6-install-e2e\`.

## Goal

Prove on this laptop that the real hooks, through real Velopack installs, put back a journaled value, write and
remove the Run value, the RunOnce value and the Path entry, and leave the laptop as it was.

## Read first

- `docs/phases/06-ship.md`: "Hooks", "Package and install values".
- `docs/architecture.md` section 6 (journal format, hooks, registry values); section 9 (the plan copy rule: read the
  real plan before and after, change nothing by hand).
- `CLAUDE.md`, "Test laptop" (Haik's own values).
- `packaging/pack.ps1`, `packaging/smoke-test.ps1` (`p6-pack-and-release` Result); `docs/research.md`: "Velopack
  checks for Wakeshot".

## Deliverables

- `packaging/e2e.ps1`, owned whole, so Phase 7 can run it again. Fixes in `packaging/smoke-test.ps1` if this run finds
  a bug in it (that card is done by then).
- A run, step by step, each with its output in the Log:
  1. BEFORE: the real `Run` and `RunOnce` values, `HKCU\Environment` `Path` (bytes and type), the active plan id and
     its lid action and sleep values on AC and DC (`powercfg /q`); no `HaikAsatryan.Wakeshot` install. If
     `%LocalAppData%\Wakeshot` exists, move it to the scratch folder (the uninstall hook deletes that folder).
  2. A plan copy (`powercfg /duplicatescheme` of the active plan, named "Wakeshot test e2e", a name no `TestPlan`
     sweep or harness `cleanup` touches; review M7), never made active.
  3. Build release x64, pack 0.1.0 and 0.1.1 with `pack.ps1`.
  4. Silent install of 0.1.0; `smoke-test.ps1`'s install checks; the Run value names the exe the phase file says.
  5. Install hook restore: set the copy's DC lid action to Do nothing; write `journal.json` with that entry
     (`original` the copy's old value, `written` 0) and an owner that is a process this card started and ended; run
     `Setup.exe --silent` of 0.1.0 over the install (what `winget upgrade` does). Check: the copy's value is back,
     the journal is gone, the real plan is unchanged, no Wakeshot process was started.
  6. Update hooks: if `p6-velopack-facts` found that `Update.exe` can apply a staged package, apply 0.1.1 that way
     with the same journal setup, and check the updated hook restored it and `current` kept its path. Otherwise
     record "update path: integration" and skip.
  7. Uninstall during a hold: the same journal setup, plus a test `WakeshotRestore` RunOnce value pointing at the
     installed exe; `Update.exe --uninstall --silent`. Check: the copy's value is back; Run, RunOnce and the Path
     entry are gone; the data folder follows Question 2's answer; the install folder, uninstall key and Start menu
     shortcut are gone.
  8. AFTER: delete the plan copy; put the moved data folder back; compare every BEFORE value; check no process this
     card started is left (by PID).
- Sizes (`Setup.exe`, packages) and install and uninstall times in the Result.

## Acceptance checks

- Steps 4, 5 and 7 pass; step 6 passes or is recorded as moved to integration with the reason.
- AFTER equals BEFORE: Run and RunOnce values, Path bytes and type, the real plan's values and the active plan id.
  No plan copy left (`powercfg /l`), no install folder, no uninstall key, no `%TEMP%\velopack_HaikAsatryan.Wakeshot`.
- No Rust file changes here; a needed fix goes under "Found outside scope" for the owning card. No `--workspace` or
  `--all` check runs here; a step 3 build that fails in another card's file follows the check rule ("Checks while other
  cards run" in `06-ship.md`): Log, wait, run again.

## Out of scope

- Real holds by a running tray (integration, checklist 14 with Haik); `winget` itself (Phase 8); ARM64 installs (the
  release workflow smoke-tests them on the ARM64 runner).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
