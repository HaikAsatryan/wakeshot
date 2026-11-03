# Integration: the update path and checklist 14 on a real install

Phase: 6. Status: ready. Needs Haik: no (uses his answers 5 and 6: off after `winget upgrade` until the next sign-in;
the data folder deleted after a good restore). Agent: `wakeshot-worker`. Size: about 120K tokens.
Real laptop: **global lane**, installs (never beside another install, `p6-install-e2e` or a card that uses the
dry-run data folder): Velopack installs into `%LocalAppData%\HaikAsatryan.Wakeshot`, the real Run and RunOnce values,
the user Path, the real data folder, one active plan copy. Haik's own plan is never written.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-update-e2e\target`.
Runs after: `p6-int-real-e2e`, `p6-install-e2e`, `p6-pack-and-release`, `p6-hooks`. Parallel with: code cards that own
none of the files below; no other laptop card.

## Goal

With an installed Wakeshot and real holds: an update found in a local feed waits while a hold counts, applies once the
work ends, restarts the tray and shows the update notice; Setup.exe over the install (what `winget upgrade` runs)
and an uninstall, each during a real hold, leave the user's values as they were (checklist 14).

## Read first

- `docs/phases/06-integration.md`: notes 6, 7 and 8; the checklist map (row 14).
- `docs/phases/06-ship.md`: "Package and install values", "Hooks", "Updates"; `docs/architecture.md` section 6
  (hooks, registry values, data folder).
- The Results of `p6-install-e2e` (its BEFORE and AFTER steps, which this card copies), `p6-pack-and-release`
  (`pack.ps1`), `p6-velopack-facts` (silent install and uninstall commands, `current` path), `p6-int-update-wiring`
  (`WAKESHOT_UPDATE_FEED`).

## Deliverables

1. `packaging/e2e-update.ps1`, owned whole, so Phase 7 can run it again. Steps, each with its output in the Log:
   1. BEFORE: Run and RunOnce values, `HKCU\Environment` `Path` (bytes and type), Haik's active plan and its values
      (`powercfg /q`: lid AC and DC, sleep AC and DC, energy saver DC), `powercfg /l`; no `HaikAsatryan.Wakeshot`
      install; `%LocalAppData%\Wakeshot` moved to the scratch folder if it exists.
   2. Build debug x64 exes (the feed seam exists in debug builds only); pack 0.0.1, 0.0.2 and 0.0.3 with `vpk`
      into scratch folders; a local feed folder holding only 0.0.1's release files.
   3. Plan copy `Wakeshot int update` made active.
   4. Silent install of 0.0.1; start the installed tray at Medium (`explorer.exe <launch .cmd>` that sets
      `WAKESHOT_UPDATE_FEED`).
   5. Update waits for work: `wakeshot keep-awake 30m` (the installed CLI); the copy is overridden; put 0.0.2 in the
      feed; after the first check (5 minutes after start) the log shows the update ready and no apply for 2 more
      minutes; `keep-awake off`: the copy is restored, then the tray exits, Velopack applies 0.0.2 and starts the new
      tray; its log shows `Updated` with 0.0.2 and the update notice; `current` kept its path; Run value unchanged.
   6. `winget upgrade` path (checklist 14): Keep awake on and a `wakeshot run -- ping -n 600 127.0.0.1` wrapper; run
      0.0.3's `Setup.exe --silent` over the install. Check: the copy's values back, journal and RunOnce gone, no tray
      running afterwards (answer 5), and record what happened to the wrapper and its `ping` child (the README says the
      wrapper dies; end a surviving `ping` by its pid).
   7. Uninstall during a hold: start the tray, Keep awake on; the uninstall command Settings > Apps runs (silent).
      Check: the copy's values back; Run, RunOnce and the Path entry gone; the data folder gone after the good restore
      (answer 6); install folder, uninstall key and Start menu shortcut gone.
   8. AFTER: Haik's plan active; the copy deleted; the moved data folder put back; every BEFORE value equal; no
      process this card started is left (by pid).
2. Fixes the run needs, only in `velo.rs`, `app/tray_mode.rs` and `app/dispatch.rs`; anything else (a hook, a pack
   script) under "Found outside scope".
3. `docs/research.md`: a section "Integration update e2e": sizes, times, each check and how.

## Acceptance checks

- Steps 5, 6 and 7 pass, or a failure is recorded with its log lines and routed under "Found outside scope".
- AFTER equals BEFORE (Run, RunOnce, Path bytes and type, Haik's plan id and values); no `Wakeshot int` plan
  (`powercfg /l`), no install folder, no uninstall key, no `%TEMP%\velopack_HaikAsatryan.Wakeshot`.
- Any fixed Rust file: `rustfmt --edition 2024 --check`, `cargo clippy --locked -p <package> --all-targets --target
  <t> -- -D warnings` and without, both targets; its package's plain tests pass.
- Build folder deleted at the end.

## Out of scope

- `winget` itself (Phase 8, after the submission); ARM64 installs (CI); release builds of the update path (phase file
  note 8 trigger); any track file other than deliverable 2.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
