# Install, update and uninstall Wakeshot on the test laptop

Phase: 6. Status: done. Needs Haik: no (silent per-user installs need no admin). Agent: `wakeshot-worker`. Size:
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

None blocking. Decided by the worker, for Haik's review (principal designer calls):

1. `e2e.ps1` packs 0.1.0 and 0.1.1 itself from `-ExeDir`; the cargo build stays the caller's. Reason: one command
   after the build, and the build folder is the caller's choice. Change it when CI wants to run it on the workflow's
   own packs: add a `-PackDir`.
2. It refuses to run elevated (exit 2) unless `-AllowElevated`, for CI runners. Reason: a High install is not what a
   user gets, and the hooks inherit the token.
3. Its end always tidies, even after a failed step: uninstall, the test RunOnce value, the user Path bytes, the active
   plan and the real plan's lid and sleep values put back from BEFORE when they differ (each a FAIL line), the copy
   deleted, the data folder moved back. Reason: the orchestrator's rule (restore from BEFORE). Never triggered here.
4. Its output names Run values and the Path only with short SHA256 hashes, never the values: the Log goes into a
   public repo. The values themselves stay in `before.json` in the work folder.
5. The update is applied with `--package <the 0.1.1 full nupkg copied into packages\>` rather than letting Update.exe
   pick the newest there: the same staged state as the app's download, but the package under test is explicit.
6. The journal owner is a `PING.EXE -n 2` the script starts and waits for: a real process that has ended, with its
   real start time.
7. A second full run (run 2) after the help fix, to prove the script runs again as it is and to record its exit code.

## Found outside scope

Problems seen but not fixed.

1. `packaging/pack.ps1` (owner `p6-pack-and-release`, done): `Get-Help` shows only the syntax, because its help block
   sits right under `#Requires -Version 7`. Fix: one blank line between them (tested on a scratch copy). Fixed here
   in `smoke-test.ps1` and `e2e.ps1`. Fixed in the rerun below.
2. `packaging/README.md` (same owner): its table does not list `e2e.ps1`. Suggested row: "`e2e.ps1` | Installs,
   updates and uninstalls for real on a PC, with a journaled hold on a throwaway plan copy at each step, and checks
   that the PC is left as it was. Run it unelevated." Fixed in the rerun below.
3. This run's packs come from commit cd3d498. `p6-int-update-wiring` saved `tray_mode.rs`, `velo.rs` and `main.rs`
   after the build (03:41 to 03:44), so its changes are not covered. After it lands, run `e2e.ps1` again (build,
   then about 40 s; steps in its help). Done in the rerun below (commit 7b6d076, 81 of 81).
4. `CLAUDE.md` "Test laptop" gives sleep after 3 h on AC and 4 h on battery (AC 10800, DC 14400). Read here at 03:38
   and 03:53: 18000 on both (5 h), and energy saver DC 30, as the orchestrator's 01:39 read says. The line is stale.
5. Optional clean-up for Haik, outside the scratch folder (the worker rule kept me from deleting them): Velopack's own
   logs, which no uninstall removes, `Remove-Item "$env:LOCALAPPDATA\velopack\velopack.log",
   "$env:LOCALAPPDATA\velopack\velopack_HaikAsatryan.Wakeshot.log"` (52,725 and 75,128 bytes now, about 25 KB per
   run of this card).

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 03:37: worker started; read the card, CLAUDE.md, plan "How we work", `06-ship.md`, architecture
  sections 6 and 9, `pack.ps1`, `smoke-test.ps1`, `packaging/README.md`, research "Velopack checks for Wakeshot",
  `hooks.rs`, the restore routine and `velo.rs` (mtime 03:19, before the 03:32 commit). Plan: `e2e.ps1` runs steps 1,
  2 and 4 to 8 (and the pack part of 3) in one go at Medium, started through a `.cmd` opened with `explorer.exe` (the
  house way, research "Spike launch and stop checks"); the build of step 3 runs here first.
- 2026-11-04 03:38: state before anything: no `%LocalAppData%\Wakeshot`, no install folder, no
  `%TEMP%\velopack_HaikAsatryan.Wakeshot`, no uninstall key, no Wakeshot Run value, RunOnce key empty; one plan,
  Balanced `381b4222-...` active; lid 1/1, sleep 18000/18000, energy saver DC 30 (`powercfg /qh`; LIDACTION and
  ESBATTTHRESHOLD show only with `/qh` here, not `/q`). `vpk` 1.2.161. No source file under `crates/` newer than 03:20.
- 2026-11-04 03:39: step 3 build: `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p
  wakeshot-cli`, exit 0 in 77 s, exes written 03:39:31 (tray 2,026,496 bytes, SHA256 `7EB66FDB...`; CLI 667,648
  bytes, `54BB6B94...`). Built from the tree of commit cd3d498: `p6-int-update-wiring` saved `tray_mode.rs` at
  03:41:34, `velo.rs` at 03:42:14 and `main.rs` at 03:42:26 (later saves up to 03:44), after this build had compiled
  them.
- 2026-11-04 03:48: `packaging/e2e.ps1` written (LF). Parser: 0 errors. Elevated run refuses at once with exit 2
  ("run it unelevated (Medium)"), nothing created. Its read-only helpers run alone (`probe-functions.ps1` in the
  scratch folder): Run 7 values, RunOnce 0, Path type 2 (REG_EXPAND_SZ) 332 bytes, active Balanced with lid 1/1,
  sleep 18000/18000, saver DC 30; a ping owner ends and is gone; the journal JSON has the frozen field names. A strict
  mode bug found and fixed there (`.Count` on an empty function result). Velopack source at tag 1.2.161: Setup starts
  the app only when not silent (`commands/install.rs` line 252); `apply` takes `--package` as given and restarts only
  without `--norestart` (`commands/apply.rs`). Launcher `run1.cmd` in the scratch folder, opened with `explorer.exe`.
- 2026-11-04 03:48:41 to 03:49:15: run 1 (`run1.log`, work folder `run1\`), 81 checks, 81 PASS, last line "e2e: all
  checks passed". Token: `whoami /groups` in the `.cmd` "Mandatory Label\Medium Mandatory Level"; the script "integrity
  Medium, elevated False"; Setup.exe's manifest asks for `asInvoker`, so Setup, Update.exe and the hooks ran at Medium
  too. The `.cmd` lost the exit code (`echo %ERRORLEVEL%> file` reads `0>` as a redirect of handle 0); fixed in the
  script's help for later runs. Per step:
  1. BEFORE (03:48:43): Run 7 values (names and hashes only in the log: OneDrive, Chrome, Edge, LenovoVantage,
     TrafficMonitor, Docker Desktop, GoogleDriveFS), RunOnce 0 values, Path type 2, 332 bytes, hash
     `A8FDC61D7FFE5745`; one plan, Balanced `381b4222-...` active; lid AC 1 DC 1, sleep AC 18000 DC 18000, energy
     saver DC 30, all of `powercfg /qh` hash `C64AFDB1CFDD13CF`. No install. No `%LocalAppData%\Wakeshot`, so nothing
     moved. Full values in `run1\before.json` (scratch only).
  2. Copy `bcea1536-7a4a-4780-952a-f090304d8e9a` "Wakeshot test e2e", DC lid action 1; never made active; real plan
     unchanged.
  3. Packs (pack.ps1 with the repo icon): `Setup.exe` 11,347,640 bytes; full nupkg 3,790,008 (0.1.0 and 0.1.1); 0.1.1
     delta 8,975; portable zip 3,782,532.
  4. `Setup.exe --silent` 0.1.0: exit 0 in 3.8 s (the hook itself 1.57 s: a just-extracted exe). All smoke-test
     install checks pass: files, shortcut target, uninstall key (DisplayName, Publisher, DisplayVersion 0.1.0,
     QuietUninstallString), Run value = `"<root>\current\wakeshot-tray.exe"`, Path entry once, hook lines (install
     started, Run set, CLI folder added, "restore: no journal", done after 0.1 s), `wakeshot.exe` no args exit 0. No
     process from the install folder after the exit; real plan unchanged.
  5. Copy's DC lid set to 0; journal owner PING pid 3940 (ended); `Setup.exe --silent` over the install: exit 0 in
     0.4 s, rollback folder `HaikAsatryan.i5WplhHtuq0JjQPq` made and removed. Hook: "journal saved ... restored",
     "power: wrote lidDc on plan bcea1536-...: 0 to 1", "journal deleted", "restore: the journal's values are back".
     Copy back to 1, journal gone, real plan unchanged, no Wakeshot process, Run unchanged, Path entry once.
  6. Update path: `Update.exe` can apply a staged package (`p6-velopack-facts` point 8), so run. Same hold (owner pid
     38848); 0.1.1 full nupkg copied to `packages\`; `Update.exe apply --silent --norestart --package <it>`: exit 0 in
     1.5 s. Obsolete hook (0.1.0) then updated hook (0.1.1) with the same restore lines; copy back to 1, journal gone,
     real plan unchanged; `sq.version` and DisplayVersion 0.1.1; `current` at the same path (folder new: created
     23:48:58Z before, 23:49:04Z after); Run unchanged; Path entry once; nothing started ("Restart: false").
  7. Same hold (owner pid 4372) plus RunOnce `WakeshotRestore` = `"<root>\current\wakeshot-tray.exe" --restore`;
     QuietUninstallString is exactly `"<root>\Update.exe" --uninstall --silent`; run: exit 0 in 0.4 s (hook 128 ms);
     install folder gone 3.1 s after the exit (Velopack's `cmd.exe /C choice ... & rmdir`). Copy back to 1; Run,
     RunOnce and the Path entry gone; Path bytes equal BEFORE; data folder gone (Question 2: deleted after a good
     restore); uninstall key, shortcut and `%TEMP%\velopack_HaikAsatryan.Wakeshot` gone; real plan unchanged.
  8. AFTER (03:49:15): copy deleted; Run key, RunOnce key, Path type and bytes, active plan id, the real plan's values
     and all of its `/qh` text (same hash), and the plan list all equal BEFORE; no install trace; no data folder (as
     BEFORE). All 8 processes the script started are gone (Setup 15276 and 26912, wakeshot.exe 7052, PING 3940, 38848
     and 4372, Update.exe 37392 and 17080), checked again by PID from here at 03:50; so are the script's pwsh 36328 and
     the launcher's cmd and Explorer factory instance.
- 2026-11-04 03:50:18: independent elevated AFTER read from this session: no data, install, `HaikAsatryan.*` or
  Velopack temp folder, no uninstall key or shortcut; Run 7 names as before, RunOnce empty; Path REG_EXPAND_SZ without
  any HaikAsatryan entry; `powercfg /l` only Balanced (active); lid 1/1, sleep 0x4650/0x4650, saver DC 0x1e; no
  `wakeshot*` process.
- 2026-11-04 03:51: help fix. `Get-Help` showed only the syntax for `e2e.ps1`, `smoke-test.ps1` and `pack.ps1`: a
  comment help block right under `#Requires -Version 7` is not read (pwsh 7.6.9; a blank line between fixes it, a
  comment line does not; tested on scratch copies). Added the blank line to `e2e.ps1` and `smoke-test.ps1` (the only
  change to it); `pack.ps1` is not this card's (Found outside scope). Both parse, both help synopses show. `e2e.ps1`
  help now says how to run it at Medium and how to keep the exit code.
- 2026-11-04 03:52:16 to 03:52:47: run 2 with the final script (`run2.cmd` with `(echo %ERRORLEVEL%)`): exit code 0,
  81 of 81 PASS, Medium ("Mandatory Label\Medium Mandatory Level"; script "elevated False"). Same BEFORE values and
  hashes as run 1. Copy `9c091bbb-afb5-4e9d-b0d0-2974db395e5a`, deleted at the end. Times: install 0.6 s (Setup.exe
  already scanned once), Setup over the install 0.5 s, apply 1.0 s, uninstall 0.4 s, install folder gone 3.1 s after.
  Pack sizes the same as run 1. Its 8 PIDs (Setup 24824, 31520; wakeshot.exe 24624; PING 31904, 4980, 15780;
  Update.exe 36516, 36104) and its pwsh 18492 gone, checked by PID from here; the Explorer factory instance it used
  (34068) ended by itself by 03:53:21. A `RuntimeBroker.exe -Embedding` (24200, started 03:52:40 by DcomLaunch) is
  Windows', not touched.
- 2026-11-04 03:53:08: independent elevated AFTER read after run 2: the same as at 03:50:18 (no traces; Run 7
  values without Wakeshot; RunOnce empty; Path REG_EXPAND_SZ, 165 characters, no HaikAsatryan entry; one plan,
  Balanced active; lid 1/1, sleep 18000/18000, saver DC 30; no `wakeshot*` process).
- 2026-11-04 03:54: `docs/research.md` section "Install end to end checks" appended at the end (re-read first; another
  card's "Update wiring checks" had landed just before; LF, no line over 120).
- 2026-11-04 03:55: build folder `<scratchpad>\p6-install-e2e\target` deleted (checked gone). Kept in the scratch
  folder: `run1\`, `run2\` (packs, `before.json`, saved hook and Velopack logs), `run1.log`, `run2.log`, the two
  launchers, `probe-functions.ps1`. Outside it, not deleted (worker rule): Velopack's own logs in
  `%LocalAppData%\velopack` (now 52,725 and 75,128 bytes; each run adds about 25 KB), `%TEMP%\velopack` (vpk's work
  folder, there before).
- 2026-11-04 03:56: acceptance checks: steps 4, 5, 6 and 7 pass (both runs); AFTER equals BEFORE for Run, RunOnce,
  Path bytes and type, the real plan's values and the active plan id; no plan copy (`powercfg /l`), install folder,
  uninstall key or `%TEMP%\velopack_HaikAsatryan.Wakeshot`; no Rust file changed, no `--workspace` or `--all` run.
  Repo files this card changed (by write time since 03:37): `packaging/e2e.ps1`, `packaging/smoke-test.ps1`,
  `docs/research.md` (its section), this card; the rest are other cards' and the orchestrator's. After run 2 the
  script lost only two comment lines that said what, not why (parse 0 errors, help still read). Card done.

## Result

Status: done. `packaging/e2e.ps1` ran the whole card twice on this laptop at Medium (03:48 and 03:52), 81 of 81
checks passed each time, and the laptop is as it was. Steps (details in the Log):

| Step | Outcome |
|---|---|
| 1 BEFORE | Run 7 values, RunOnce empty, Path REG_EXPAND_SZ 332 bytes, Balanced the only plan and active, lid 1/1, sleep 18000/18000, saver DC 30; no install; no data folder, so nothing moved |
| 2 plan copy | "Wakeshot test e2e", never active, deleted at the end |
| 3 build and pack | x64 release of commit cd3d498 (77 s); packs 0.1.0 and 0.1.1 |
| 4 silent install | pass: every smoke-test install check; Run value is the quoted `current\wakeshot-tray.exe` |
| 5 Setup over the install | pass: the install hook put the copy's lid value back, journal gone, real plan unchanged, no process started |
| 6 update | pass (run, not moved to integration): `Update.exe apply --silent --norestart`; obsolete and updated hooks; restored; `current` kept its path |
| 7 uninstall during a hold | pass: restored; Run, RunOnce and Path entry gone; data, install and temp folders, uninstall key, shortcut gone |
| 8 AFTER | equals BEFORE (below); no process this card started is left |

AFTER equals BEFORE, each value (script check and a separate elevated read): Run key (7 values, bytes) equal; RunOnce
key (empty) equal; Path type 2 and bytes (hash `A8FDC61D7FFE5745`) equal; active plan `381b4222-...` equal; lid AC 1
DC 1, sleep AC 18000 DC 18000, saver DC 30, and all of `powercfg /qh` (hash `C64AFDB1CFDD13CF`) equal; `powercfg /l`
only Balanced. No install folder, no `HaikAsatryan.*` folder, no uninstall key, no shortcut, no
`%TEMP%\velopack_HaikAsatryan.Wakeshot`, no `%LocalAppData%\Wakeshot` (none before). All 16 PIDs the runs started
and both script shells are gone, checked by PID. The real plan was never written: the hooks wrote only the copy, and
logged no `set_active`.

Sizes (x64): `Setup.exe` 11,347,640 bytes; full nupkg 3,790,008; delta 0.1.0 to 0.1.1 8,975; portable zip 3,782,532.
Times: install 3.8 s the first time (new Setup.exe; hook 1.57 s), 0.6 s the second; Setup over the install 0.4 and
0.5 s; update 1.5 and 1.0 s; uninstall 0.4 s, install folder gone 3.1 s after.

Token: Medium (`whoami /groups` in the launcher; the script "elevated False"); Setup.exe asks for `asInvoker`, so the
installs and hooks ran at Medium. Nothing started the tray: silent Setup and `--norestart` never start the app
(Velopack source), and no process ran from the install folder after any step.

Files changed:

- `packaging/e2e.ps1` (new): steps 1 to 8; refuses when elevated (unless `-AllowElevated`), installed, or a journal
  exists; always tidies at the end.
- `packaging/smoke-test.ps1`: one blank line under `#Requires`, so `Get-Help` reads its help (no behavior change).
- `docs/research.md`: new section "Install end to end checks".
- This card.

Checks: the card's acceptance checks all pass (steps 4 to 7 pass, AFTER equals BEFORE, no copy, install folder,
uninstall key or temp folder left). No Rust file changed; no `--workspace` or `--all` run; the one cargo command was
`build -p wakeshot-tray -p wakeshot-cli --locked`. Build folder deleted. ARM64: out of scope (CI).

Open items: Found outside scope 1 to 5 (the `pack.ps1` help line, the README row, a rerun after
`p6-int-update-wiring`, the stale CLAUDE.md sleep values, Velopack's logs for Haik). Decisions for Haik's review:
Questions 1 to 7 above.

## Rerun (2026-11-04)

A short follow-up by the orchestrator's brief: build release x64 from commit 7b6d076 (adds `p6-int-update-wiring` and
`p6-int-wiring-fixes`), run `e2e.ps1` again at Medium as run 2 did, and fix Found outside scope 1 and 2 (the
`pack.ps1` help line, the README row). Build folder `<scratchpad>\p6-install-e2e-rerun\target`; packs and logs in
`<scratchpad>\p6-install-e2e-rerun\`.

Status: done.

### Log

- 2026-11-04 04:30: worker started; read this card, CLAUDE.md, plan "How we work", `e2e.ps1`, the packaging README
  and script heads. The hook log file is still `logs\wakeshot.log` and every hook line `e2e.ps1` checks is still in
  `hooks.rs` (grep), so the script needs no change before the run.
- 2026-11-04 04:31: state before anything (read-only script, elevated): no data, install, `HaikAsatryan.*` or
  Velopack temp folder, no shortcut, no uninstall key; Run 7 values (the same 7 names), RunOnce empty; Path
  REG_EXPAND_SZ, 165 characters, no HaikAsatryan entry; one plan, Balanced `381b4222-...` active; lid 1/1, sleep
  18000/18000, saver DC 30, all of `/qh` hash `C64AFDB1CFDD13CF` (as run 1 and 2); no `wakeshot*` process; Velopack
  logs 52,725 and 75,128 bytes. `.git\refs\heads\main` (7b6d076) written 04:29:38, after the newest file under
  `crates\` (04:24:11), so the build below is that commit. `vpk` 1.2.161.
- 2026-11-04 04:31: Found outside scope 1 and 2 fixed: `pack.ps1` has one blank line under `#Requires` (parse 0
  errors, LF; `Get-Help` now shows its synopsis and example, before it showed only the syntax); `packaging/README.md`
  table lists `e2e.ps1` after `smoke-test.ps1` (LF).
- 2026-11-04 04:33: build: `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p
  wakeshot-cli` (RUSTFLAGS unset), exit 0 in 128 s (04:31:30 to 04:33:38). Tray 3,952,640 bytes (run 1 and 2:
  2,026,496), SHA256 `0EBCCA9F...`; CLI 668,672 bytes (667,648), `4B0ABC89...`. No file under `crates\` written after
  04:24:11, so it is commit 7b6d076. The tray nearly doubled: the update wiring brings in Velopack's update code (the
  build compiled `url`, `idna` and the `icu_*` crates for it). Launcher `rerun.cmd` (like run 2's, exit code with
  `(echo %ERRORLEVEL%)`), opened with `explorer.exe` by `launch.ps1`, which records the PIDs of the chain.
- 2026-11-04 04:33:55 to 04:34:37: the rerun (`rerun.log`, work folder `rerun\`): exit code 0, 81 checks, 81 PASS,
  0 FAIL, last line "e2e: all checks passed". The same 81 check names as run 2 (compared line by line, plan ids
  masked). Token: `whoami /groups` in the `.cmd` "Mandatory Label\Medium Mandatory Level"; the script "integrity
  Medium, elevated False"; Setup.exe still asks for `asInvoker`. Per step:
  1. BEFORE (04:33:57): Run 7 values with the same 7 name hashes as run 2, RunOnce 0 values, Path type 2, 332 bytes,
     hash `A8FDC61D7FFE5745`; one plan, Balanced `381b4222-...` active; lid AC 1 DC 1, sleep AC 18000 DC 18000, saver
     DC 30, all of `/qh` hash `C64AFDB1CFDD13CF`. All equal to run 1 and 2. No install, no data folder.
  2. Copy `a3f6c9ce-b45c-4088-84b3-73b21d17ec76`, DC lid 1, never active.
  3. Packs: `Setup.exe` 12,364,401 bytes (run 2: 11,347,640); full nupkg 4,806,769 (3,790,008); delta 8,975 (same);
     portable zip 4,799,293 (3,782,532). Each grew by about 1 MB, the tray exe's growth.
  4. Install exit 0 in 7.7 s (run 1, also a new Setup.exe: 3.8 s; run 2: 0.6 s). Velopack logged Setup's start 3 s
     after the launch and the hook at 3,559 ms, while the hook's own timer says 0.1 s, so the time is process start
     of new, larger exes; Defender scanning them is a guess, not verified. All 22 install checks pass, hook lines as
     before.
  5. Hold (owner PING 24304, ended); Setup over the install exit 0 in 0.5 s (hook 277 ms): "power: wrote lidDc on
     plan a3f6c9ce-...: 0 to 1", journal deleted, "restore: the journal's values are back". 12 checks pass.
  6. Hold (owner 36380); `Update.exe apply --silent --norestart --package <0.1.1 full>`: exit 0 in 1.0 s; obsolete
     hook 20 ms, updated hook 672 ms (run 1: 197 ms), the same restore lines; "Restart: false"; `current` same path,
     new folder (00:34:19Z before, 00:34:23Z after). 16 checks pass.
  7. Hold (owner 24540) plus the test RunOnce value; `Update.exe --uninstall --silent`: exit 0 in 0.4 s (hook
     127 ms); install folder gone 3.1 s after. 17 checks pass.
  8. AFTER (04:34:35): 9 checks pass: Run, RunOnce, Path type and bytes, active plan, the real plan's values and all
     of its `/qh` text (same hash), plan list, no install trace, data folder as BEFORE (none), none of the 8 started
     processes left. The tidy part found nothing to put back (no FAIL line).
  The hook log (saved before the uninstall) has 2 `power: wrote` lines, both on the copy, no `set_active` line and no
  WARN or ERROR line. Velopack's own log lines as before (`Skipping killing self` WARNs, harmless).
- 2026-11-04 04:35:12: every PID of the run is gone, checked by PID from here: my `explorer.exe` 35776; the Explorer
  factory instance 31772, its `cmd.exe` 37356 and the script's pwsh 35680; the Windows Terminal 34720 and
  OpenConsole 22432 that DcomLaunch started for that window; the script's 8 (Setup 30504 and 5152, wakeshot.exe 24264,
  PING 24304, 36380 and 24540, Update.exe 38292 and 23708); Velopack's delete timer `cmd.exe` 5480. None needed a
  kill.
- 2026-11-04 04:35:15: independent elevated AFTER read (the same read-only script as 04:31): identical to the 04:31
  read on every line (traces none; Run 7 names; RunOnce empty; Path REG_EXPAND_SZ 165 characters, same hash, no
  HaikAsatryan entry; one plan, Balanced active; lid 1/1, sleep 18000/18000, saver DC 30, `/qh` hash
  `C64AFDB1CFDD13CF`; no `wakeshot*` process). Velopack's logs now 65,527 and 87,223 bytes (+12,802 and +12,095).
- 2026-11-04 04:36: `docs/research.md` "Install end to end checks": one dated rerun bullet added at its end (re-read
  first; LF; no line over 120). Build folder deleted (checked gone). Kept in `<scratchpad>\p6-install-e2e-rerun\`:
  `rerun\` (packs, `before.json`, saved hook and Velopack logs), `rerun.log`, `rerun.exit`, `rerun-whoami.txt`,
  `pids.txt`, `rerun.cmd`, `launch.ps1`, `state.ps1`. `e2e.ps1` and `smoke-test.ps1` unchanged; their `Get-Help`
  synopses still show.

### Decided by the worker, for Haik's review

1. No change to `e2e.ps1` or `smoke-test.ps1`: the rerun found no failure, and every hook line they check is still in
   `hooks.rs`.
2. The README row is the one Found outside scope 2 suggested, plus "`Get-Help ./packaging/e2e.ps1` says how", since
   the Medium launch is not obvious and the help explains it. Change it when the README gets a section for `e2e.ps1`.
3. The 7.7 s first install is recorded, not chased: the script's limit is 120 s, and the hook's own timer (0.1 s)
   shows the time is outside Wakeshot's code. Change it when a first install on a clean PC takes more than about
   15 s.

### Found outside scope (rerun)

1. Still stale: `CLAUDE.md` "Test laptop" gives sleep AC 10800 and DC 14400; read here at 04:31 and 04:35: 18000 on
   both (Found outside scope 4 above).
2. For the record, not a bug: the release tray exe grew from 2,026,496 to 3,952,640 bytes with `p6-int-update-wiring`
   (the build now compiles `url`, `idna` and the `icu_*` crates), so `Setup.exe` is 12.4 MB. If size matters, its
   owner can check which Velopack feature pulls them.
3. Velopack's own logs in `%LocalAppData%\velopack` are now 65,527 and 87,223 bytes; Haik's optional clean-up command
   is in Found outside scope 5 above.

### Result

Status: done. Commit 7b6d076 built (x64 release, 128 s) and `e2e.ps1` ran once at Medium (04:33:55 to 04:34:37):
exit 0, 81 of 81 checks passed, the same 81 checks as run 2. Steps 4, 5, 6 and 7 pass; the laptop is as it was.

AFTER equals BEFORE, each value as the card recorded it: Run 7 values (same name hashes), RunOnce empty, Path type 2,
332 bytes, hash `A8FDC61D7FFE5745`; active plan Balanced `381b4222-...`, the only plan; lid AC 1 DC 1, sleep AC 18000
DC 18000, saver DC 30, all of `/qh` `C64AFDB1CFDD13CF`. A separate elevated read before and after matched. No plan
copy, install folder, uninstall key, shortcut, data or temp folder left. All 15 PIDs of the run gone, none killed.

Sizes: tray 3,952,640 (was 2,026,496), CLI 668,672; `Setup.exe` 12,364,401; full nupkg 4,806,769; delta 8,975;
portable zip 4,799,293. Times: install 7.7 s (new exes; hook 3.6 s by Velopack, 0.1 s by its own timer), Setup over
the install 0.5 s, update 1.0 s (updated hook 672 ms, was 197 ms), uninstall 0.4 s, install folder gone 3.1 s after.

Files changed: `packaging/pack.ps1` (one blank line under `#Requires`; `Get-Help` now shows its synopsis),
`packaging/README.md` (the `e2e.ps1` row), `docs/research.md` (one rerun bullet), this card. No Rust file; no
`--workspace` run; build folder deleted.

Open items: Found outside scope (rerun) 1 to 3; the three decisions above.
