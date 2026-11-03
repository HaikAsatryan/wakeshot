# Real checks: the agent's checks on the test laptop

Phase: 3. Status: done. Needs Haik: no (best while he is not using the laptop: the screen may flicker during the
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

None blocked the card. Decided by the worker, for Haik's review: see Result, "Decisions".

## Found outside scope

Problems seen but not fixed.

1. **The laptop is on battery and undocked** since 00:30:07 (Kernel-Power 105; 93% at 00:45; no external screen).
   The orchestrator's keep-awake helper stops below 25%, and every agent's build drains the battery. For the
   orchestrator.
2. **Someone used the laptop after the orchestrator's 22:55 read**: Settings changed sleep AC and DC to 18000 (5 h) at
   00:31:08, input at 00:34:06 and 00:34:50, a touchpad wake at 00:48:56. If that was Haik, `CLAUDE.md` "Test laptop"
   ("sleep after 3 h on AC and 4 h on battery ... DC 14400, AC 10800") is stale; if it was not Haik, someone else is
   at the laptop. Not changed here (not this card's file); `p3-haik-checks` reads the values live.
3. **Docs made stale by these checks** (for the docs pass): `docs/architecture.md` line 652 ("'No apply' rests on two
   points not tested") and line 1017 ("That a write to an inactive plan changes only that plan is not tested yet"):
   both points are now tested (research, "Power track real checks"). Line 656 ("Side effects of that many calls ...
   not known either way"): now brightness unchanged over 50 cycles, no error in the System and Application logs, one
   System log event 12 per `PowerSetActiveScheme` (a visible flicker not verified). Section 4 could say the journal
   lock opens across High and Medium in both directions.
4. **One System log event per plan switch.** Each `PowerSetActiveScheme` writes `UserModePowerService` event 12
   (Information). The tray will add about two per hold, so a busy day of agent turns adds hundreds. Harmless as far as
   seen; worth a line in the user docs or FAQ if a user asks about the log. Product call, not changed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:27, orchestrator, from `p3-journal`'s "Found outside scope" item 3: add a check that a Medium tray
  can open the journal lock while an elevated process (for example `wakeshot restore` from an admin terminal) holds
  it, and the reverse. Not tested by `p3-journal`.
- 2026-11-03 20:50, orchestrator, from `p3-session`'s Result: rerun `real_locked_now_here` on the final code (its
  unlocked pass ran before a clippy-only change); `real_lock_locks` was never run. The ignored tests' filter needs the
  full path, `session::tests::...`; `session::real_on_console_here` matches no test.
- 2026-11-04 00:32, worker: started (bundle with `p3-haik-checks`). Laptop lane held. The orchestrator's keep-awake
  helper (pwsh pid 35152, SYSTEM request "Wakeshot orchestrator: agents keep working while Haik is away") runs all
  through; never stopped, filtered out of every `powercfg /requests` read.
- 2026-11-04 00:36, step 1 (before): `powercfg /qh` (00:32:57) and our read (`power_check fake-tray --plan-only`,
  00:36:38) agree: Balanced `381b4222-...` active and the only plan, lid AC 1 and DC 1, **sleep AC 18000 and DC
  18000**, energy saver DC 30; no "Wakeshot test" plan, no `%LocalAppData%\Wakeshot` (so no journal), no
  `%LocalAppData%\Wakeshot-harness`, no `WakeshotRestore` RunOnce value. Sleep differs from the orchestrator's 22:55
  read (AC 10800, DC 14400): System log, `UserModePowerService` event 12 at 00:31:03, 00:31:05 and 00:31:08,
  "Process C:\Windows\ImmersiveControlPanel\SystemSettings.exe (process ID:34496) reset policy scheme" (Balanced to
  Balanced), and the sleep key's last write time is 00:31:08; no other Balanced key changed (key times read with
  `RegQueryInfoKeyW`). So someone changed both sleep values to 5 h in Settings at 00:31, before this card started; not
  an agent of this card. These are the baseline for step 6. `GetLastInputInfo` showed input at 00:34:06 and 00:34:50,
  so someone may still be at the laptop (or another card sends input; not checked).
- 2026-11-04 00:36, build: `cargo build --locked -p wakeshot-windows --features test-support --example power_check`
  and `cargo build --locked -p wakeshot-tray` (host x64, debug, this card's target folder): both exit 0.
- 2026-11-04 00:38, step 2 first try: `cargo test -p wakeshot-windows` does not compile: E0583, `mod tests;` in
  `tray/menu_paint.rs:899` with no file (the `p5-menu-paint` card, mid-edit). Waiting; steps 3 and the mutex check
  first, with the exes built at 00:36.
- 2026-11-04 00:40, step 3: `explorer.exe <scratch>\medium.cmd` (00:39:36). Chain: pwsh 22336 <- cmd 27080 <-
  `explorer.exe /factory,{75dff2b7-...} -Embedding` 30496 <- svchost 2308; `whoami /groups`: Medium Mandatory Level.
  `power_check medium-probe` (pid 29824, child of that cmd): "integrity Medium (0x2000), elevation type Limited",
  "Global\Wakeshot.Probe.29824: created and taken", exit 0; values the same at start and exit. Pass.
- 2026-11-04 00:40, journal lock across integrity (orchestrator item 20:27), with `<scratch>\mutex-probe.ps1`, which
  makes the same call as `NamedMutex::open` (`CreateMutexExW`, default security, flags 0, `SYNCHRONIZE |
  MUTEX_MODIFY_STATE`) and takes with `WaitForSingleObject`; probe names `Local\Wakeshot.Probe.Journal.A` and `.B`, not
  the real name, so no other card's run waits on it. A: High holder (pid 26916) took it; the Medium opener (from the
  same `medium.cmd`) got a handle with error 183, `take(0)` gave 258 (held), and after the holder's release
  `WaitForSingleObject` gave 0 (taken), `ReleaseMutex` true. B (reverse): Medium holder (pid 10408) took it; the High
  opener got 183, 258, then 0 and true. Security as created: High creator `O:BA D:(A;;0x1f0001;;;SY)
  (A;;0x1f0001;;;BA)(A;;0x1f0001;;;S-1-5-5-0-930782)`; Medium creator owner and full access the user SID instead of
  BA, plus SY and the same logon SID. Both work because the default DACL grants the logon session SID full access,
  and both tokens carry it. Pass both ways.
- 2026-11-04 00:42, step 2: the tree compiled again at 00:41. `cargo test --locked -p wakeshot-windows -- --ignored
  --test-threads=1 --nocapture --skip real_lock_locks power:: process:: session:: recovery::` (00:41:18 to 00:42:19):
  16 passed, 0 failed, 361 filtered out, exit 0: `power::plan` 4 (the inactive copy write: user's plan before and
  after lid 1/1, sleep 18000/18000, saver 30), `power::stop_points` 4 (apply 12.0 s, check 1.5 s, restore 11.2 s,
  unknown key 31.3 s), `power::system` 2, `process` 2 real (outside-tree child under Explorer 12528, Medium and
  Limited; the nested runner at Medium passed) plus the `sleep_role` helper, `recovery::tests::child_role` helper,
  `session::tests::real_locked_now_here` and `real_on_console_here` (the rerun the 20:50 note asks; `real_lock_locks`
  skipped by the card: it is `p3-haik-checks` C). `cargo test --locked -p wakeshot-tray --test recovery -- --ignored
  --nocapture`: 2 passed (`real_guard_parent_is_explorer`, `sleep_role`), exit 0. Values at 00:42:44 the same as
  step 1; only Balanced in the list; no journal, no RunOnce value. Pass.
- 2026-11-04 00:44, step 4: `power_check fake-tray --home <scratch>\home` started at Medium the same way
  (`explorer.exe <scratch>\faketray.cmd`, 00:43:45): pid 35200, "integrity Medium (0x2000), elevation type Limited",
  data folder `%LocalAppData%\Wakeshot`. Copy `4e75f8c9-...` "Wakeshot test 35200", never active; apply lid AC 0, lid
  DC 0, sleep DC 0: `powercfg /qh` on the copy read lid 0/0, sleep AC 18000 DC 0; journal owner pid 35200 session 1
  with three entries (originals 1, 1, 18000); RunOnce `WakeshotRestore` = `"<target>\debug\wakeshot-tray.exe"
  --restore`; guard pid 5904, `wakeshot-tray.exe --guard 35200 <start> Local\Wakeshot.GuardStop.35200.1`, parent the
  shell Explorer 12528; two ownership checks at 00:43:56 and 00:44:06 (each step under 2 ms). `taskkill /PID 35200 /F`
  at 00:44:15.366: guard log "tray pid 35200 is gone" at .636, the three writes back (0 to 1, 0 to 1, 0 to 18000),
  "journal deleted", "RunOnce value WakeshotRestore removed" at .646; journal file gone by .661 and guard process gone
  by .668 (polled every 100 ms). Copy read 1/1, 18000/18000 after it. Balanced active all through. Then `power_check
  cleanup --home <scratch>\home`: removed the run record, "deleted the test copy 4e75f8c9-...", forgot the recorded
  plan, exit 0. Pass.
- 2026-11-04 00:45, found: **the laptop is on battery and undocked.** `GetSystemPowerStatus`: ACLineStatus 0, 93%;
  `Win32_Battery` status 1; Kernel-Power 105 "Power source change" (AcOnline false) at 00:30:07; only the internal
  panel `LEN41B7` in `WmiMonitorBrightness` (no external screen); Kernel-Power 506 "entering Modern Standby, Reason:
  Idle Timeout" at 00:42:37 (screen off; DC screen off is 180 s). The lid was open at about 00:42
  (`real_lid_once_returns_open_here` passed). So steps 2 to 4 ran on battery, and step 5 runs on battery with the
  screen off. Neither changes what the steps check; recorded for the orchestrator (battery drain overnight).
- 2026-11-04 00:47, architecture section 5 "Plan switch", second point (`<scratch>\plan-switch.ps1`, 00:46:51 to
  00:46:53): the policy in force read with `CallNtPowerInformation` `SystemPowerPolicyAc` (0) and `SystemPowerPolicyDc`
  (1): Balanced AC lid 2 (Sleep), DC lid 2, DC idle Sleep after 18000 s. A `powercfg /duplicatescheme` copy, renamed
  "Wakeshot test 18928", got lid AC 0, lid DC 0, sleep DC 0 by `powercfg /set*valueindex` while inactive: the policy in
  force stayed the same (0.5 s later). `powercfg /setactive <copy>`: AC lid 0, DC lid 0, DC idle "after 2147483647 s"
  (Never). Balanced active again: back to lid 2, 18000 s. Copy deleted; only Balanced listed; values the same as step
  1. Pass: activating a plan later applies its stored values, and a write to an inactive plan leaves the values in
  force alone.
- 2026-11-04 00:51, step 5: `powercfg /requests` at 00:47:46: only the orchestrator's SYSTEM request (pwsh, its
  reason). `power_check cycles 50 --home <scratch>\home` (elevated, default pause 1000 ms), 00:48:04.777 to
  00:49:52.969, exit 0: copy `8cba940d-...` "Wakeshot test 34672" made active, 50 of 50 cycles "apply ok; restore
  ok", 100 `PowerSetActiveScheme` calls min 33.2, median 51.5, max 121.8 ms (apply median 50.3, restore median 52.6;
  11 cycles had a call over 100 ms, spread from cycle 6 to 50); Balanced active again, copy deleted, run record
  removed, "values at exit: the same as at start". A sampler (`<scratch>\monitor.ps1`, every about 220 ms, 519 samples
  00:48:02 to 00:49:57): `WmiMonitorBrightness` read 100 in every sample (one panel, `LEN41B7`), so 100 before, during
  and after each of the 50 cycles (`<scratch>\analyze.py`); the policy in force (`SystemPowerPolicyAc/Dc`) changed
  103 times, DC lid 2 to 0 and DC idle 18000 s to Never at each apply and back at each restore. Another card's
  `wakeshot_windows-*.exe` test binary ran 00:49:27 to 00:49:45 (cycles 39 to 47; it may take the sub-second request of
  `request_sets_and_clears`); those cycles' times (37 to 67 ms) are not off. Event logs 00:47:55 to 00:50:05: System
  and Application, no error or warning; 102 `UserModePowerService` event 12 (Information), "power_check.exe (process
  ID:34672) reset policy scheme", one per `PowerSetActiveScheme`; Kernel-Power 507 "exiting Modern Standby, Reason:
  Input Touchpad" at 00:48:56.372 (566 "InputPoUserPresent"), then the face sign-in trustlet `FsIso.exe` ran 2 s:
  **someone touched the touchpad at 00:48:56** (cycle 25), so the screen was on from cycle 25. Then `power_check
  cleanup`: nothing left ("delete every test copy whose process is gone", none), exit 0. Pass.
- 2026-11-04 00:52, step 6 (after): copied `%LocalAppData%\Wakeshot\logs\wakeshot.log` (88 lines, tray and guard of
  step 4 only) to `<scratch>\out\` and removed `%LocalAppData%\Wakeshot`, which step 4 made. `powercfg /qh` at
  00:52:26: Balanced active and the only plan, lid AC 1 and DC 1, sleep AC 18000 and DC 18000, energy saver DC 30; no
  "Wakeshot test" plan, no data folder (so no journal), no harness folder, no `WakeshotRestore` value. Equal to step 1.
  Every Balanced key's last write time is the same as at 00:32 (the sleep key still 00:31:08), so no run of this card
  wrote to Balanced. The policy in force reads as before. Every process this card started has ended (the two Explorer
  factory instances, 30496 and 31644, ended by themselves). Pass.
- 2026-11-04 00:54, worker: build folder deleted; research section added; Result written; Status done.

## Result

Status: done (2026-11-04 00:54). Every step passed; Haik's values after equal those before. No code changed.

Files changed: this card; `docs/research.md` (new section "Power track real checks", additions only). Scratch
scripts and logs in `<scratchpad>\p3-real-checks\` (`values.ps1`, `keytime.ps1`, `keytimes-all.ps1`, `policy.ps1`,
`copyvals.ps1`, `mutex-probe.ps1`, `plan-switch.ps1`, `monitor.ps1`, `events.ps1`, `analyze.py`, `make-cmd.py`, the
two `.cmd` files, `out\`). Build folder deleted.

### Checks and outcomes

| Step | Outcome |
|---|---|
| 1 Before (00:32:57) | Balanced active and only plan; lid AC 1, DC 1; sleep AC 18000, DC 18000; energy saver DC 30; no copy, journal, RunOnce value. Sleep was changed in Settings at 00:31:08 (not 10800 and 14400 as at 22:55) |
| 2 Real tests | `wakeshot-windows` filter: 16 passed; `wakeshot-tray --test recovery`: 2 passed. First try blocked 3 min by another card's half-done file |
| 3 Medium probe | Medium (0x2000), Limited, started through Explorer; `Global\` mutex created and taken: pass |
| Journal lock across integrity (20:27 item, review finding 8) | High holds, Medium opens and waits, then takes; and the reverse: pass both ways (pwsh with the same `CreateMutexExW` call, probe names) |
| 4 Guard after a crash | Medium `fake-tray` killed by `taskkill /F`: the guard restored the copy's three entries, deleted the journal and the RunOnce value, and exited within 300 ms; copy deleted by `cleanup`: pass |
| Plan switch, second point (architecture section 5) | Values written to an inactive copy took effect only when it was made active (policy in force read by `CallNtPowerInformation`): pass |
| 5 `cycles 50` | 50 of 50; `PowerSetActiveScheme` min 33.2, median 51.5, max 121.8 ms; brightness 100 in all 519 samples; no error or warning in System or Application; one event 12 per call; then `cleanup`: nothing left: pass |
| 6 After (00:52:26) | Equal to step 1; no copy, journal, RunOnce value; Balanced keys not written since 00:31:08: pass |

The orchestrator's SYSTEM request (pwsh 35152) was present all through and was left alone.

### Decisions (decided by the worker, for Haik's review)

| # | Decision | Reason | Change it when |
|---|---|---|---|
| 1 | The baseline is the values read at 00:32 (sleep 18000 both), not the 22:55 values | Settings wrote them at 00:31, before this card; putting 10800 and 14400 back would undo a user's own change | Haik says the 00:31 change was not his: then he sets his values in Settings |
| 2 | `fake-tray` ran at Medium, started through Explorer | The tray ships at Medium; an elevated tray would test a mix the product does not have | A check needs an elevated tray |
| 3 | `cycles 50` ran while three non-laptop cards built and tested | Their only real request is the sub-second one of `request_sets_and_clears`, which cannot change the plan or brightness; the overlap was logged (cycles 39 to 47) and those cycles look like the rest | A cycles result looks off where another card's test ran: rerun alone |
| 4 | Steps ran on battery, undocked, as the laptop was found | Nothing in the steps needs AC; on AC: not run | Haik wants checklist 15 on AC too (2 minutes, the same command) |
| 5 | The lock check used pwsh with the same Win32 call on probe names, not the real name | The real name would make other cards' runs wait up to 10 s; the security depends on the call, not the name | A Rust-level test of the lock across integrity is wanted |
| 6 | Removed `%LocalAppData%\Wakeshot` (made by step 4) after saving its log | The step 1 state had no such folder | Never |
| 7 | Added the plan switch test | The card's "Read first" names the two points not tested; 2 s with a copy active | Never |

### Open items

- A visible flicker during the cycles: not verified (nobody watched). The cycles ran on battery only.
- A denied `Global\` open across users: not tested (no second account; integration).
- Found outside scope 1 to 4 above: battery and undocked; someone at the laptop and a stale `CLAUDE.md` line; stale
  architecture lines for the docs pass; one System log event per plan switch.
