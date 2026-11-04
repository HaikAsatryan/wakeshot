# Uninstall removes the start at login switch value

Phase: 6. Status: ready. Needs Haik: no (orchestrator decision for his review: an uninstall leaves nothing, so a
reinstall starts as on a new PC). Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the orchestrator
(session `wakeshot-12`, 2026-11-04) from `p6-startup-switch-check` "Found outside scope" 1.
Runs after: `p6-startup-switch-check` (done). Parallel with: code cards that own none of the files below; no other
laptop card while step 4 runs (it installs Wakeshot).
Real laptop: step 4 only: one `packaging/e2e.ps1` run (installs at Medium, as `p6-install-e2e` did).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-uninstall-startup-approval\target`.

## Goal

Once the user has used the Settings > Apps > Startup switch, Windows keeps a `Wakeshot` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run`. Uninstall removes the Run value but
leaves that one (`p6-startup-switch-check`, research "Start at login switch check"). Make the uninstall hook remove
it too. Install and update hooks must keep leaving it alone: that is what keeps the user's off choice across
`winget upgrade`.

## Read first

- `docs/tasks/p6-startup-switch-check.md` (Result, Found outside scope 1: the suggested fix).
- `crates/wakeshot-windows/src/store.rs` (`AutoStart`, `StartKey`, `remove`: the one writer of the start values,
  review m12), `crates/wakeshot-windows/src/hooks.rs` (`HookEnv`, `RealHookEnv`, `on_uninstall`, the fake and its
  `Call` list, the order tests).
- `docs/architecture.md` section 6, "Velopack hooks" (the uninstall order).

## Deliverables

1. `store.rs`: `AutoStart` can delete the `Wakeshot` value under `<base>\Explorer\StartupApproved\Run`, with the same
   "missing is not an error" rule as `remove`. A test over the scratch key, like the `Run` tests.
2. `hooks.rs`: a `HookEnv` method for it, called by `on_uninstall` only, right after the Run value is removed, with log
   lines in the style of the others ("... removed" or "was not there"). The fake's `Call` list and the order tests
   updated; a test that install and update never call it.
3. `packaging/e2e.ps1` step 7: seed a `StartupApproved\Run\Wakeshot` value (the off form `03` plus 8 bytes, as the
   check card wrote) before the uninstall, then check it is gone after. Its BEFORE and AFTER cover that key too.
4. Run `packaging/e2e.ps1` once at Medium (the way `p6-install-e2e` did, release x64 of the tree); record the outcome.
5. `docs/architecture.md` section 6: the uninstall order line. `README.md`: only the uninstall line that lists what is
   removed and what stays, if it names start at login (the README is a draft that waits for Haik; touch nothing else).

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64; ARM64 for `wakeshot-windows` (the tray's ARM64 needs clang: CI only); `cargo test --locked
  -p wakeshot-windows`; `cargo test --workspace --locked` once at the end.
- `e2e.ps1`: every check passes; AFTER equals BEFORE.
- Build folder deleted at the end.

## Out of scope

- Any other hook step; the install and update hooks' behavior.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result

What was done, files changed, checks and outcomes, open items.
