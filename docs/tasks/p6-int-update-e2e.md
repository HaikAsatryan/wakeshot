# Integration: the update path and checklist 14 on a real install

Phase: 6. Status: done. Needs Haik: no (uses his answers 5 and 6: off after `winget upgrade` until the next sign-in;
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

1. The update notice after an update was not seen: the session was locked the whole night, and the core holds every
   notice until the unlock. `p6-int-haik-checks` (or Haik once) should see "Wakeshot updated to (version)" after an
   update with the session unlocked; `e2e-update.ps1` checks it by itself when the session is unlocked.
2. The product GUID's `NotifyIconSettings` entry (`2334163346184210159`) now points at the removed
   `%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe`. Harmless: Windows moves it to the next exe that
   starts (phase file note 7). Not touched.
3. Velopack's own logs in `%LocalAppData%\velopack` grew to 78,939 and 103,215 bytes (about 29 KB this run). Haik's
   optional clean-up command is in `p6-install-e2e`, Found outside scope 5.
4. Still stale: `CLAUDE.md` "Test laptop" gives sleep AC 10800 and DC 14400; read here at 05:36, 05:39 and 05:48:
   18000 on both (already reported by `p6-install-e2e`).
5. For the record, not a bug: Setup.exe and Update.exe print their whole log to the console when they have one, so the
   script's output holds Velopack's lines too (as in `e2e.ps1`'s runs). Left as is: they help when a step fails.

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
- 2026-11-04 05:39: stop 2 started (the orchestrator: the laptop lane is mine alone; `p6-int-real-e2e` done, test
  changes only). No `wakeshot-tray` or `wakeshot` process runs; Balanced active. Rebuilding the debug exes first.
- 2026-11-04 05:39: debug x64 rebuild of the tree as it was (`-p wakeshot-tray -p wakeshot-cli --locked`, RUSTFLAGS
  unset): exit 0 in 3 s, tray 12,585,984 bytes (SHA256 `6DA85198...`), CLI 1,744,384 (`BE325996...`). The newest
  source files were the running core card's (`state.rs` 05:36:41, `core/lid.rs` 05:36:07); they compiled. Elevated
  read with `state.ps1` (scratch, read only) at 05:39:35: no traces, no data folder, Run 7 names, RunOnce empty, Path
  REG_EXPAND_SZ without a HaikAsatryan entry, Balanced the only plan and active, lid 1/1, sleep 18000/18000, saver DC
  30, `/qh` hash `C64AFDB1CFDD13CF`, no Wakeshot process, session locked (LogonUI.exe).
- 2026-11-04 05:39:50 to 05:48:13: run 1 (`run1.cmd` opened with `explorer.exe`; `run1.log`, `run1.exit` 0): 125
  PASS, 0 FAIL, 1 SKIP, last line "e2e-update: all checks passed, 1 skipped". Medium ("Mandatory Label\Medium
  Mandatory Level"; script "elevated False"). Per step:
  1. BEFORE (05:39:52): the same values and hashes as the elevated read and as stop 1; nothing to move.
  2. Packs 3 to 4 s each; feed with 0.0.1 only.
  3. Copy `fad1c359-0bc9-40a6-a854-9495fe7ecd85`, lid 1/1, sleep DC 18000, made active.
  4. `Setup.exe --silent` 0.0.1: exit 0 in 5.2 s (hook 3.1 s by Velopack); 21 install checks pass; tray pid 23628 at
     Medium, `state.json` after 0.9 s, "Claude Code is watched in the folder CLAUDE_CONFIG_DIR names", "Wakeshot is
     installed, so it checks for updates", nothing held.
  5. `keep-awake 30m`: the copy overridden in 0.9 s, journal of 3 keys owned by 23628, RunOnce set. 0.0.2 put in the
     feed. First check at 05:45:15, 301 s after the start: `Update.exe patch` used the delta (0.36 s), then "update
     0.0.2 downloaded" and the core's "is ready" line. 120 s with no apply, tray alive, copy still overridden.
     `keep-awake off` 05:47:16.39: copy restored 0.7 s after the command (3 `power: wrote` lines, journal and RunOnce
     gone, guard stopped), then "Applying update 0.0.2", "the updater started", "Update applied", "Wakeshot ends with
     code 0" (exit code 0, 0.7 s after the command); obsolete hook 19 ms, updated hook 688 ms; new tray pid 33676 at
     05:47:17.53 with "Velopack restarted the tray after the update to 0.0.2" and the empty Claude folder (the `.cmd`'s
     variables reached it). Update notice: SKIP, the session was locked, so the core held it (decision 5). 0 WARN or
     ERROR lines. `current` same path, new folder; `sq.version` and DisplayVersion 0.0.2; Run and Path unchanged.
  6. 0.0.2 tray: `keep-awake on` (overridden 0.6 s), wrapper pid 38288 with ping pid 15476, hold file
     `run-15476.json`, tray held both. 0.0.3 `Setup.exe --silent`: exit 0 in 5.4 s. Setup killed tray 33676, guard
     36316 and the wrapper at once; the install hook restored the copy (730 ms). No Wakeshot process after (answer 5).
     Wrapper exit code 1; ping still ran and was ended by its pid. DisplayVersion and `sq.version` 0.0.3; Run and Path
     unchanged. Setup's rollback rename got "Access is denied" once and worked on its 1 s retry.
  7. Tray pid 26028 (0.0.3) at Medium, `keep-awake on` (0.6 s), QuietUninstallString exactly `"<root>\Update.exe"
     --uninstall --silent`: exit 0 in 0.5 s; Update.exe killed tray and guard, uninstall hook 149 ms. Copy back; Run,
     RunOnce, Path entry gone; Path bytes as BEFORE; data folder gone (answer 6); install folder gone 3.1 s after the
     exit; uninstall key, shortcut, Velopack temp folder gone.
  8. AFTER (05:48:09): Balanced made active again, copy deleted; Run, RunOnce, Path type and bytes, active plan, its
     values and `/qh` hash, plan list, no install trace, data folder (none) all equal BEFORE; none of the 18 listed
     processes left; the tidy found nothing to put back (no FAIL).
- 2026-11-04 05:48:32: elevated `state.ps1` read identical to 05:39:35 on every line. By pid: all 18 processes of the
  run, the tray's two Update.exe runs (patch 38248, apply 38372), the script's pwsh 36216 and my `explorer.exe` 17168
  are gone; no process has `p6-int-update-e2e` in its command line. None needed a kill from here.
- 2026-11-04 05:49: no check failed, so no Rust fix (deliverable 2: none needed). `docs/research.md` section
  "Integration update e2e" appended at the end (re-read first; LF; no line over 120). Build folder, packs and feed
  folders deleted (checked gone); kept in `<scratchpad>\p6-int-update-e2e\`: `run1.log`, `run1\` (`before.json`,
  launch `.cmd` files, wrapper output, the log saved before the uninstall, Velopack log lines), `prepare1.log`,
  `state.ps1`, `probe-functions.ps1`, `run1.cmd`.

## Result

Status: done (2026-11-04 05:49). `packaging/e2e-update.ps1` ran the whole card once on this laptop at Medium
(05:39:52 to 05:48:13): exit 0, 125 checks passed, 0 failed, 1 skipped (the update notice; the session was locked).
The laptop is as it was. No Rust fix was needed.

| Step | Outcome |
|---|---|
| 1 BEFORE | Run 7 values, RunOnce empty, Path REG_EXPAND_SZ 332 bytes, Balanced the only plan and active, lid 1/1, sleep 18000/18000, saver DC 30; no install; no data folder |
| 2 build and pack | x64 debug of the tree at 05:39; 0.0.1, 0.0.2 (with a delta) and 0.0.3; feed with 0.0.1 only |
| 3 plan copy | "Wakeshot int update", active from before the install to step 8, deleted at the end |
| 4 install and tray | pass: install checks; tray at Medium with the local feed and an empty Claude folder |
| 5 update waits for work | pass: first check after 301 s downloaded 0.0.2 (delta); no apply for 120 s while Keep awake held; after `keep-awake off` the copy was put back, then the apply; the old tray quit with 0, the new one started 1.1 s after the command and logged the restart to 0.0.2; `current` kept its path; Run unchanged. Notice: SKIP (locked) |
| 6 Setup over the install (checklist 14) | pass: copy back by the install hook, journal and RunOnce gone, no tray afterwards (answer 5); the wrapper was killed (code 1), its `ping` went on and was ended by its pid |
| 7 uninstall during a hold | pass: copy back; Run, RunOnce, Path entry gone; data folder gone (answer 6); install folder, uninstall key, shortcut, temp folder gone |
| 8 AFTER | equals BEFORE (below); no process of the run left |

AFTER equals BEFORE, by the script and by a separate elevated read before and after: Run key (7 values, same
hashes), RunOnce key (empty), Path type 2 and bytes (hash `A8FDC61D7FFE5745`), active plan Balanced
`381b4222-...`, lid 1/1, sleep 18000/18000, saver DC 30, all of `powercfg /qh` (hash `C64AFDB1CFDD13CF`). `powercfg
/l` lists only Balanced. No install folder, no uninstall key, no `%TEMP%\velopack_HaikAsatryan.Wakeshot`, no data
folder (none before). All 18 processes the script listed, the tray's two Update.exe runs and the launch chain are
gone, checked by pid. Haik's plan was never active while a Wakeshot of this run ran, and no hook or tray wrote it.

Sizes (debug x64): `Setup.exe` 14,921,322 bytes; full nupkg 7,363,690; delta 8,975; portable zip 7,356,214. Times:
first check 301 s after the start; restore 0.7 s after `keep-awake off`; new tray 1.1 s after it; Setup over the
install 5.4 s; uninstall 0.5 s, install folder gone 3.1 s after.

Files changed:

- `packaging/e2e-update.ps1` (new; committed in e6e37fb; in stop 2 only its help's run time changed from about 11
  to about 9 minutes, parse 0 errors): steps 1 to 8, `-PrepareOnly`, refusals, tidy-up. Rerun for Phase 7: build debug x64 (`cargo build --locked --target x86_64-pc-windows-msvc -p
  wakeshot-tray -p wakeshot-cli`), then run it at Medium through a `.cmd` opened with `explorer.exe` (its help says
  how); about 9 minutes.
- `docs/research.md`: new section "Integration update e2e".
- This card.

Checks: steps 5, 6 and 7 pass; AFTER equals BEFORE (Run, RunOnce, Path bytes and type, Haik's plan id and values); no
`Wakeshot int` plan, no install folder, no uninstall key, no Velopack temp folder. No Rust file changed, so the
rustfmt, clippy and test checks do not apply. Build folder, packs and feeds deleted (checked gone).

Open items: the update notice must be seen with the session unlocked (Found outside scope 1). Found outside scope 2 to
5 are for the record. Decisions for Haik's review: 1 to 8 in the Log (05:38).
