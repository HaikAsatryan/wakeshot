# Make real-settings tests refuse to run beside an installed Wakeshot

Phase: 6 (ship). Status: later (start before Haik installs the first release). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 60K tokens.
Written by the orchestrator, 2026-11-04 18:43, from Haik's question: once the real Wakeshot is installed, do dev tests
clash with it?
Runs after: `p2-6-runonce-keep`. Parallel with: cards that own none of the files below.
Real laptop: no (checks read state only; the tests use fakes or scratch keys).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-real-install-guard\target`.

## Goal

`cargo test` and the `--dry-run` tray are already apart from a real Wakeshot (own lock names, own data folder). These
are not: the harness (`power_check`, Haik's step files), the real end-to-end cards, and `packaging/e2e.ps1` (it uses
the real pack id `HaikAsatryan.Wakeshot`; on a real install it logs a failed check at step 1 and, as far as the
orchestrator read, goes on to install over it and uninstall it). Each of them must stop before it changes anything
when a real Wakeshot runs or is installed, with one clear line: quit Wakeshot from its tray first (and, for a
sign-out test, turn start at login off). Even with separate names, two Wakeshots would fight over the same power
settings, so a full dev edition is not the fix.

## Read first

- `crates/wakeshot-windows/src/store.rs` `Names::real` (`Local\Wakeshot.Tray`, `Global\Wakeshot.Settings`) and
  `DataDir::real`; `crates/wakeshot-windows/src/hooks.rs` `RUN_NAME`; `power/journal.rs` `RUN_ONCE_NAME`.
- `crates/wakeshot-windows/examples/power_check.rs` (its start checks); `packaging/e2e.ps1` step 1.
- `docs/architecture.md` section 6 "Files" (install folder and data folder).

## Deliverables

1. One shared check: a real Wakeshot runs (the tray lock name is held) or is installed (the Velopack install folder or
   its HKCU Uninstall key exists), or a real journal or `WakeshotRestore` value exists. Read-only.
2. `power_check` real-settings commands and `packaging/e2e.ps1` call it first and stop on a hit, changing nothing.
3. A line in `CLAUDE.md` "Test laptop": quit the installed Wakeshot before real-settings tests (the same rule as
   for any other keep-awake app).

## Acceptance checks

- Tests for each hit and for the clean case (fakes or scratch keys, never the real lock or folder of a real install).
- `cargo fmt`, `cargo clippy -p` (with and without `--all-targets`), `cargo test -p` on own packages pass.

## Out of scope

- A separate dev edition with other names everywhere. Change to it when Haik wants the real Wakeshot running all day
  while he tests dev builds for real (power tests would still need it quit).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04 18:43: card written by the orchestrator.

## Result

What was done, files changed, checks and outcomes, open items.
