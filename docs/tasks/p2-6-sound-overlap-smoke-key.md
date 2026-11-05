# One chime at a lid close plus a floor unplug; the smoke test checks the notice key; ARM64 checks with clang

Phase: 2 to 6 (core and ship fixes). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K
tokens. Written by the orchestrator, 2026-11-05, from `p2-6-floor-grace` "Found outside scope" 4 and
`p2-6-docs-pass` "Found outside scope" 1 and 2. Haik's answer (2026-11-05): fix both now.
Runs after: `p6-screen-reruns` and `p6-notice-app-icon` Part B (that session builds from the tree and owns
`packaging/` meanwhile). Part C also after Haik's clang install (`p6-velopack-dependency` "Steps for Haik").
Parallel with: nothing (Part C runs the whole-workspace checks).
Real laptop: no (unit and property tests; `smoke-test.ps1` is run against a local pack in a temp install only if the
script itself allows it on a dev laptop, else CI's release dry run checks it).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-sound-overlap-smoke-key\target`.

## Goal

Three small fixes before the first release: a single sound where two overlap today, a CI check that catches a hook
that stops registering the notice app id, and the ARM64 checks on this laptop once clang is in.

## Read first

- `docs/tasks/p2-6-floor-grace.md`: "Found outside scope" 4, Result.
- `crates/wakeshot-core/src/core/lid.rs`: `settle_window_lock`, `settle_source_lock`; the props that hold
  `lid_lock_sent`.
- `docs/tasks/p2-6-docs-pass.md`: "Found outside scope" 1 and 2.
- `packaging/smoke-test.ps1`, `packaging/e2e.ps1` (how it checks the key and the PNG), `packaging/README.md` "What
  the smoke test checks".
- `docs/tasks/p6-velopack-dependency.md`: step 0 and the ARM64 items.

## Deliverables

### Part A: one chime (core)

- `crates/wakeshot-core/src/core/lid.rs`: a lid close in the unknown journal window plus an unplug under the floor
  plays one sound, not the lid chime and the floor chime in the same pass. Pass the step back's state to
  `settle_window_lock` as `settle_source_lock` gets it. The lock still goes out before the restore. Which sound stays:
  the floor chime, as `settle_source_lock` does for an unplug under the floor (`docs/product.md` sounds list).
- A scenario test for that case, and the same `lid_lock_sent` rule in the props.

### Part B: the smoke test checks the notice app id

- `packaging/smoke-test.ps1`: after install, the key
  `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` with `DisplayName` and `IconUri`, and
  `current\wakeshot-notice.png`; after uninstall, the key gone; the key in its refusal list of traces. Port the checks
  from `e2e.ps1`.
- `packaging/README.md`: "What the smoke test checks" names them; line 81 wrapped to 120 characters.

### Part C: ARM64 with clang (`p6-velopack-dependency` step 0, after Haik)

- If `vswhere -all -products * -find 'VC\Tools\Llvm\**\clang.exe'` still finds nothing, skip Part C and say so.
- Else: the two checks of step 0 and `clang --version` in `docs/research.md` (additions only), then the card's ARM64
  items: clippy and the release build of the workspace for `aarch64-pc-windows-msvc`. Record the outcome in the
  `p6-velopack-dependency` Result and set its Status line to done without the open clang item if they pass.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets`, `-D warnings`, on x64, and on ARM64 if Part C
  runs; `cargo test --workspace --locked`; core props at 10000 cases, three runs.
- A hand mutant: revert the Part A fix and see the new scenario test fail.
- `smoke-test.ps1` parses (`[System.Management.Automation.Language.Parser]::ParseFile`), and passes on a local pack
  if the script allows a run here; else say CI's release dry run is its first real run.

## Out of scope

- Any other sound, lid or floor rule. `release.yml`, `ci.yml`.

## Questions for Haik

None.

## Found outside scope

## Log

- 2026-11-05: card written by the orchestrator.

## Result
