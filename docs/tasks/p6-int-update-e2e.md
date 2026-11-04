# Integration: the update path and checklist 14 on a real install

Phase: 6. Status: in progress. Needs Haik: no (uses his answers 5 and 6: off after `winget upgrade` until the next sign-in;
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
- 2026-11-04 05:16: worker started, in two stops (orchestrator): stop 1 writes `packaging/e2e-update.ps1` and runs
  only what leaves the laptop as it is (parse, `Get-Help`, the read-only BEFORE, the build and packs into scratch);
  stop 2 runs steps 1 to 8 alone on the laptop after `p6-int-real-e2e`.
- 2026-11-04 05:38: stop 1 done. `packaging/e2e-update.ps1` written (steps 1 to 8, modeled on `e2e.ps1`: same BEFORE
  and AFTER checks, refusals, Medium rule and tidy-up; LF, ASCII). Parser: 0 errors; `Get-Help` shows synopsis, 3
  examples, 7 parameters. Debug x64 build of the tree at 05:25 (`-p wakeshot-tray -p wakeshot-cli --locked`, exit 0,
  100 s; tray 12,585,984 bytes, CLI 1,744,384; the tray holds the string `WAKESHOT_UPDATE_FEED`). `-PrepareOnly` run
  elevated (`<scratchpad>\p6-int-update-e2e\prepare1.log`, exit 0, 15 s): no refusal; BEFORE: Run 7 values, RunOnce
  empty, Path type 2, 332 bytes, hash `A8FDC61D7FFE5745`; one plan, Balanced `381b4222-...` active, lid 1/1, sleep
  18000/18000, saver DC 30, `/qh` hash `C64AFDB1CFDD13CF` (same as the install card's runs); no install, no data folder.
  Packs: `Setup.exe` 14,921,322 bytes, full nupkg 7,363,690, delta 0.0.1 to 0.0.2 8,975, zip 7,356,214; 4 to 6 s per
  pack; feed lists 0.0.1 Full only. Helpers tried alone (`probe-functions.ps1`): log patterns against the install
  card's saved hook log, process finders, lock check, `Wait-Until` scope: all as expected. Refusals tried: elevated
  full run exits 2 before making its work folder; a non-empty work folder exits 2. Nothing installed, no plan made, no
  tray started; Run, RunOnce, Path and `%LocalAppData%\Wakeshot` not touched. The session is locked (LogonUI.exe since
  02:00:38), so the update notice will likely be a SKIP in stop 2 (decision 5). Decided by the worker, for Haik's
  review:
  1. The build stays the caller's (`-ExeDir`), as in `e2e.ps1`; the script refuses a tray exe without the string
     `WAKESHOT_UPDATE_FEED` (a release build ignores the feed). Change it when CI runs it on its own build.
  2. The tray's launch `.cmd` points `CLAUDE_CONFIG_DIR` at an empty folder, as `e2e_real.rs` does: the agent sessions
     on this laptop would hold the tray, and the update would never apply. Change it when a step must check Claude
     holds on an install.
  3. 0.0.2 is packed with 0.0.1 as the earlier release (a delta, as a real release has), 0.0.3 with 0.0.2. The feed
     holds what Velopack's `FileSource` reads: the index, `RELEASES` and the nupkgs, not Setup or the zip.
  4. A `-PrepareOnly` switch: refusals reported only, the BEFORE read, the packs; it may run elevated. For a check
     before a real run, as this stop needed.
  5. The update notice is a SKIP, not a FAIL, when the session is locked (LogonUI.exe in the session): the core keeps
     every notice while locked (`core.rs`, `notify`). Then Haik's checks must see it. Change it when a run can unlock.
  6. The helpers are copied from `e2e.ps1`, not shared: that script is done and owned by `p6-install-e2e`. Change it
     when a third script needs them.
  7. "The copy is put back before the apply" is checked by log order: the first line after keep-awake off that names
     the copy's plan id (journal and power lines do; nothing else writes the copy then) must come before "Applying
     update". No journal wording is matched.
  8. It refuses while any `wakeshot-tray` or `wakeshot` process runs, from any folder: a second tray stops at once on
     the tray mutex.

## Result

What was done, files changed, checks and outcomes, open items.
