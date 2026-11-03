# Check the Velopack facts the ship track builds on

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: yes. It installs, updates and uninstalls a throwaway app under the pack id `HaikAsatryan.Wakeshot`. No
power settings, no screen, no sound. The orchestrator runs it alone among the real-laptop cards of all tracks.
Runs after: Haik's sign-off of the track plans; the contracts card (review m10: points 9 and 10 build the repo's
release exes, and the card installs software; the trial crate itself lives outside the repo). Parallel with: every
card not marked real laptop.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-velopack-facts\target`; the trial crate in
`<scratchpad>\p6-velopack-facts\trial`.

## Goal

Settle the Velopack points the ship cards depend on, on this laptop, before any Wakeshot code relies on them.

## Read first

- `docs/research/distribution.md`, section 1 (the Phase 0 trial and its commands).
- `docs/architecture.md` section 6: the data folder paragraph and "Velopack hooks".
- `docs/phases/06-ship.md`: "Package and install values", "Hooks", "Updates".
- `docs/research.md`: "Ship track planning checks".

## Deliverables

1. A throwaway Rust crate outside the repo with `velopack = "=1.2.161"`, built for x64 only (an ARM64 build needs
   clang, which this laptop lacks). Two exes, named like Wakeshot's: a GUI `wakeshot-tray.exe` that calls
   `VelopackApp::build()` with the four fast callbacks plus `on_restarted`, and a console `wakeshot.exe`. Every hook
   and start logs its arguments, `VELOPACK_*` variables, working folder and exe path to a file outside the install
   folder, in `<scratchpad>\p6-velopack-facts\`.
2. Answers, each marked verified (how) or not verified:
   1. The pack id with a dot, `HaikAsatryan.Wakeshot`: `vpk pack`, silent install, update and silent uninstall all
      work. The install folder name; the uninstall key name (winget's `ProductCode`), its `DisplayName` and
      `Publisher`.
   2. With `--packTitle Wakeshot`: the root stub's name, what it starts, and whether `wakeshot.exe` in `current`
      and a root `Wakeshot.exe` cause any clash. `--shortcuts StartMenuRoot`: only a Start menu shortcut, and its
      target.
   3. The `current` folder: its full path before and after an update from a local folder (`FileSource`). Same path or
      not decides what the Run value and the Path entry point at.
   4. `wait_exit_then_apply_updates(asset, true, true, [])`: the hook order, that the new version starts, what
      `on_restarted` gets; the app exits 5 s after the call; the app does not exit for 70 s (the 60 s wait).
   5. `UpdateManager::new` in an exe run from the build folder, not installed: the error, so the tray can tell "not
      installed" from a real failure.
   6. A hook that panics, and a hook that sleeps past its limit: what Velopack logs, whether the operation goes on,
      and where hook stdout and stderr go.
   7. `Setup.exe --silent` over a running install (what `winget upgrade` runs): only the install hook runs and the app
      is not started (Phase 0 saw this with another id; check it with the dotted one).
   8. Whether `Update.exe` can apply a staged package by itself (`Update.exe apply -h`), so `p6-install-e2e` can test
      the update hooks without a running tray.
   9. ARM64 packing on this laptop: `vpk pack -r win-arm64 --channel win-arm64` with the ARM64 exes of the repo's own
      release build (no `velopack` in them yet). Does `vpk` accept them, and which CPU are the package's `Setup.exe`
      and `Update.exe` (read the PE header)?
   10. `vpk pack` with a main exe that never calls `VelopackApp` (the skeleton tray): a warning, an error or nothing.
   11. The options of `vpk upload github -h` and `vpk download github -h`: putting both channels in one release,
       draft or published, pre-release, the tag; `vpk pack -h` for `--icon`, `--releaseNotes` and deltas.
3. `docs/research.md`: a new section "Velopack checks for Wakeshot", each fact with how it was checked. Additions
   only; re-read the end of the file right before the edit (other cards append too).
4. Clean up: every trial install uninstalled.

## Acceptance checks

- Each of the 11 points answered with its mark.
- An AFTER check in the Log, with the command output: no `%LocalAppData%\HaikAsatryan.Wakeshot`, no
  `%TEMP%\velopack_HaikAsatryan.Wakeshot`, no `HKCU\...\Uninstall\HaikAsatryan.Wakeshot` key, no Start menu
  shortcut, no `Wakeshot` value under `Run` or `RunOnce`, no trial process running (by the PIDs this card started).
- No file in the repo changed except `docs/research.md` and this card.

## Out of scope

- Wakeshot code, `crates/`, `Cargo.toml`. Power settings. Any upload to GitHub (`-h` only). ARM64 builds with
  `velopack`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
