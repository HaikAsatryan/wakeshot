# Haik's checks for the power track

Phase: 3. Status: done (2026-11-04 18:31; A, C, D, E and B1 pass; B2, checklist 5, not shown: see Result and Found
outside scope 2). Needs Haik: **yes**:
about 60 minutes elapsed, about 15 minutes of his hands (lid, charger, sign-out, Task Manager, lock); the two battery
runs need the laptop unplugged for about 20 minutes each (now D about 12 and E about 25 minutes). Real laptop:
**yes**: battery, lid, sign-out, lock, a plan copy made active. **Runs alone on the whole laptop** (cross-track
review B2): no agent of any track and no cargo command runs. A sign-out ends every process of Haik's session (the
orchestrator, every worker, their builds), Start > Sleep stops them mid-call, and another worker's test that takes a
power request would skew the battery timings of D, E and F. The orchestrator records its state and stops spawning
before step B and step D; after sign-in Haik starts the orchestrator again (`/orchestrate`).
Agent: `wakeshot-worker`. Size: about 60K tokens.
Runs after: `p3-real-checks`. Parallel with: nothing.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-haik-checks\target`.

## Goal

Run the checks from architecture section 9 that need Haik and can run before integration, with the real tray exe's
guard and restore modes and the `power_check` harness.

## Read first

- `CLAUDE.md` ("Test laptop"); `docs/phases/03-windows-power.md` ("Manual checklist").
- `docs/architecture.md`: section 9 checklist 1 to 6 and 11, and its plan copy rule; section 5 "Power requests",
  "Guard", "Logon restore"; section 4 "Tray threads" (end of session).
- `docs/research.md`: "Lid and sleep spike" (block I, H run 2), "Battery sleep Never stops the battery rule".
- `docs/phases/00-morning-steps.md`: the form of steps Haik already used.
- `docs/tasks/p3-harness.md` and `p3-real-checks.md` Results.

## Deliverables

1. Exact numbered steps for Haik, written into this card under "Steps for Haik", grouped so he does each physical
   action once, with what he should see and how long to wait. Then Status `blocked`; the orchestrator relays them.
   Checks to cover (the order and grouping are the worker's call):
   - A. Checklist 4: `fake-tray` running with its guard; Haik ends it in Task Manager with End task, then a second run
     ended with End task on its group. The guard must live on and restore the copy. Also: how Task Manager groups the
     guard (architecture "not verified").
   - B. Checklists 5 and 6 in one sign-out: one `fake-tray` ended by the agent (so only RunOnce can restore, in the
     real data folder, through `wakeshot-tray.exe --restore`), one `end-session` alive at sign-out. After sign-in: the
     first restored by RunOnce, the second before the session ended, with times.
   - C. Lock and session notices: `session-watch` running; Haik presses Win+L and signs in again; the agent runs
     `real_lock_locks` once and Haik signs in again.
   - D. Checklist 3: `hold` on battery; Haik does Start, Sleep, wakes the PC, then leaves it with no input; the tick log
     must show no suspend at the next screen off after resume (with no re-take it slept there in H run 2).
   - E. Checklist 1: `hold --active-copy` on battery with the lid closed and no input past 916 s (about 20 minutes).
   - F. Optional, checklist 2: `hold --no-request` on battery with the lid closed and Haik's own 600 s: how long it
     runs.
2. After Haik's steps: read the logs, record each outcome, restore and check Haik's values (active plan; lid AC and
   DC; sleep AC and DC; energy saver DC) against the values recorded before the first step.

## Acceptance checks

- Haik's values before and after are equal; no "Wakeshot test" plan, no journal, no `WakeshotRestore` RunOnce value
  left.
- Each check A to E (F if run) has a pass or fail with its log lines, recorded in the Result and in
  `docs/research.md` ("Power track manual checks"), with "not verified" for anything a step could not show.
- A failed check that changes the design goes under "Found outside scope" with the section it affects.

## Out of scope

- Checklist 7, 8, 9, 10, 16 and the second-session part of 11 (integration); 12 (later).
- Fixing code.

## Questions for Haik

Each with the worker's default.

- Run D and E now or at integration: answered, now in Phase 3 (the default; `docs/phases/02-06-signoff.md` item 10,
  Haik 2026-11-03 18:09), because a failure changes the battery design.

Decided by the worker while Haik slept (2026-11-04 01:10), for Haik's review:

| # | Decision | Reason | Change it when |
|---|---|---|---|
| 1 | Skip F (checklist 2, optional) | Haik's own battery sleep is now 5 h, so "his own 600 s" no longer exists; the harness has no option for a finite DC sleep on a copy; Wakeshot itself hibernates 5 minutes after a closed-lid work end (checklist 7, integration), so Windows' own timer is only the fallback. Saves 20 minutes of lid-closed battery time | Checklist 7 fails, or a design needs Windows' closed-lid timer |
| 2 | B as two sign-outs (B1 check 6, B2 check 5), not one | The harness refuses a second RunOnce-writing run while one is set (`p3-harness` choice 4), and the end-session run's own restore at sign-out would clear the one RunOnce value check 5 needs | Never with this harness |
| 3 | Haik starts each run by double-clicking a step file; no agent acts between steps | The sign-outs end every agent; with no relay between steps Haik is not kept waiting, and a new worker reads every log afterwards | A step needs an agent's judgment mid-way |
| 4 | Logs and step files in `%LocalAppData%\Wakeshot-harness` (the harness's default folder) | Survives the sign-outs and the orchestrator's new session; easy to open with Win+R; deleted at the end | Never |
| 5 | A1 and A2 run `power_check` alone in a classic console window | Its Task Manager group then holds only `power_check` and its console host, so "End task on its group" cannot reach the Terminal windows the agents run in (smoke test 01:03: `start "" conhost.exe ...` from a `.cmd` opened by Explorer gave a `ConsoleWindowClass` window, not a Terminal tab) | Never |

## Steps for Haik

Written 2026-11-04 01:10 by the worker of the bundle, after `p3-real-checks`. **Your time: about 60 minutes, about 15
of them hands-on.** In D you leave the laptop alone for 7 minutes; in E for 20 minutes with the lid closed (stay near
it). The two sign-outs in B end the orchestrator and every agent: you start the orchestrator again at the end. F is not
in the list (decision 1 above).

Paths: `<S2>` is `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\7c3512fd-d0b5-4439-a2c6-4dd5cf1b6c37\scratchpad\p3-haik-checks`
(build, scripts); the step files are in `%LocalAppData%\Wakeshot-harness\haik`; logs in
`%LocalAppData%\Wakeshot-harness` and `%LocalAppData%\Wakeshot\logs`. The scripts are `<S2>\scripts\check.ps1`,
`kill.ps1`, `C2-lock.ps1`, `requests-log.ps1`; `<S2>\make-cmd.py` writes the step files (all written 2026-11-04 and
smoke-tested; the build of 2026-11-04 16:44, on today's code, is in `<S2>\target`).

### Before Haik starts (orchestrator, then this card's worker)

**Orchestrator:**

1. Stop spawning. Wait until no worker or reviewer runs other than this card's worker; record the state in files.
2. Stop your keep-awake helper by PID if it still runs (`away-system-holder.ps1`, pwsh PID 35152 at 2026-11-04 00:30).
   Its SYSTEM request would hide a failure in E.
3. Haik: laptop on the desk, lid open, signed in, battery at 50% or more (D and E run on battery for about 40
   minutes). Nothing plays sound or video.
4. Resume this card's worker for its prep. On its "ready", tell Haik "go".

**Worker prep** (about 5 minutes, elevated shell):

1. `powercfg /requests`: DISPLAY, SYSTEM and EXECUTION must each say `None.` If an app holds one, ask Haik to close
   it and read again.
2. Build into `CARGO_TARGET_DIR=<S2>\target` (host x64, debug, no `RUSTFLAGS`):
   `cargo build --locked -p wakeshot-windows --features test-support --example power_check`,
   `cargo build --locked -p wakeshot-tray`, `cargo test --locked -p wakeshot-windows --no-run` (the C2 test binary,
   so no cargo command runs during Haik's steps).
3. `pwsh -NoProfile -File <S2>\scripts\check.ps1 -Save`: Haik's values become the baseline
   (`%LocalAppData%\Wakeshot-harness\haik-out\baseline.json`); copy them into the Log. Check that `HKCU\...\Run` has
   no Wakeshot value (checklist 5 needs start at login off).
4. `python -I <S2>\make-cmd.py`: writes the step files. Never set `LOCALAPPDATA` for a process that starts `python`:
   the Python install manager then installs into that folder (seen 01:05, Log).
5. Start `<S2>\scripts\requests-log.ps1 -Minutes 150` hidden in the background (elevated); write its PID in the Log.
6. `<S2>\target\debug\examples\power_check.exe fake-tray --plan-only`: values and paths print; nothing changes.
7. `explorer.exe %LocalAppData%\Wakeshot-harness\haik` (opens the folder for Haik), then hand back "ready".

### Rules for every step

- Laptop on a hard desk, lid open unless a step says to close it. Nothing plays sound or video.
- All step files are in one folder. To open it again (also after a sign-out): Win+R, type
  `%LocalAppData%\Wakeshot-harness\haik`, Enter. Double-click a file to run it.
- Each run's window starts with a line ending `integrity Medium (0x2000), elevation type Limited`. If it says High,
  close the window and tell the orchestrator.
- **To stop at any point:** close the run's window (the X): it puts everything back. Then double-click `0-check.cmd`;
  it must end with `check: ALL OK`. If not, double-click `0-cleanup.cmd`; if that does not end with `check: ALL OK`
  either, stop and tell the orchestrator. Do the same if the laptop restarted, ran out of battery, or signed out in the
  middle of a step.
- Change no power setting by hand during the steps: Settings writes to your real plan even while a test copy is
  active.
- Too hot (after E, the bottom is too hot to keep a hand on): close the E window, run `0-check.cmd`, let the laptop
  cool, and tell the orchestrator.

### A. End task in Task Manager (about 6 minutes)

Charger in or out, either is fine.

1. Double-click `A1-fake-tray.cmd`. A console window opens. Wait for a line that starts with `holding:` (about 2
   seconds).
2. Press Ctrl+Shift+Esc. In Task Manager click Processes, and type `power_check` in the search box at the top. Write
   down: is it under "Apps" or "Background processes"; does its row have an arrow (a group), and which rows are inside.
3. Search `wakeshot` instead. Write down the same for `wakeshot-tray.exe` (the guard).
4. Search `power_check` again. If its row has an arrow, open it and right-click the `power_check.exe` row inside;
   otherwise right-click the row itself. Click End task. The console window closes.
5. Wait 5 seconds. Double-click `0-check.cmd`: `check: ALL OK` (the guard put everything back). Close that window.
6. Double-click `A2-fake-tray.cmd`. Wait for `holding:`.
7. In Task Manager, right-click the top row of power_check's group (the row with the arrow) and click End task. If
   there is no group, do it as in step 4 and write down "no group".
8. Wait 5 seconds. `0-check.cmd`: `check: ALL OK`. Close Task Manager.

### C. Lock and sign in (about 3 minutes)

1. Double-click `C1-session-watch.cmd`. Wait for the line `registered for session notices ...`.
2. Press Win+L. Wait 10 seconds. Sign in.
3. Double-click `C2-lock.cmd`. The screen locks by itself within a second. Sign in.
4. The C2 window ends with `exit 0 (passed)`. Close it. Close the C1 window too (or leave it: it ends after 10
   minutes).

### D. Sleep and wake on battery (about 12 minutes, 7 of them hands off)

1. Unplug the charger and the DELL cable, whichever powers the laptop. Keep both out until the end of E.
2. Double-click `D-hold.cmd`. Wait for `SYSTEM and EXECUTION power requests set`, then a line starting `tick 2` (10
   seconds).
3. Start > Power > Sleep. Wait until the screen is dark, then 30 seconds more.
4. Press the power button briefly. Sign in.
5. Do not touch the laptop for 7 minutes. The screen turns off after about 3 minutes: expected, leave it.
6. After the 7 minutes, touch the touchpad and sign in if asked. In the D window, find `requests retaken after the
   resume`. Below it `tick` lines should come every 5 seconds. Write down whether any line below it contains `GAP`.
7. Close the D window. `0-check.cmd`: `check: ALL OK`.

### E. Lid closed on battery, 20 minutes (about 25 minutes)

Still on battery. Stay near the laptop while the lid is closed.

1. Double-click `E-hold-lid-closed.cmd`. Wait for `... is the active plan now`, `SYSTEM and EXECUTION power requests
   set`, then `tick 2`.
2. Close the lid. Do not touch the laptop for 20 minutes; leave it on the desk.
3. Open the lid. Sign in if asked. Touch the bottom of the laptop (too hot: see the rules).
4. In the E window: `lid closed`, `tick` lines every 5 seconds, `lid open`. Write down whether any line contains
   `GAP`.
5. Close the E window: it puts your plan back and deletes the copy. `0-check.cmd`: `check: ALL OK`.
6. Plug the charger (and the DELL) back in.

### B. Two sign-outs (about 8 minutes)

**Orchestrator, before B1:** both sign-outs end you and every agent. Write the state to files, then tell Haik "B ready".

B1, sign out during a run:

1. Double-click `B1-end-session.cmd`. Wait for `hidden top-level window up: sign out to run check 6`.
2. Start > your picture > Sign out. If Windows lists apps that keep it from signing out, click "Sign out anyway".
3. Sign in again. Do not start the orchestrator yet.

B2, restore at the next sign-in:

1. Win+R, `%LocalAppData%\Wakeshot-harness\haik`, Enter. Double-click `B2-fake-tray.cmd`. Wait for `holding:`.
2. Double-click `B2-kill-tray-and-guard.cmd`. It must end with `READY: sign out now`. If it says `STOP` or `NOT
   READY`, do not sign out: close the B2 windows, run `0-cleanup.cmd`, and tell the orchestrator.
3. Sign out as in B1. Sign in again. Wait 1 minute.
4. Double-click `0-check.cmd`: `check: ALL OK`.
5. Start the orchestrator (`/orchestrate`) and tell it "Haik checks done", with what you wrote down in A, D and E.

### After Haik's steps (a new worker, from this card)

1. Read: `%LocalAppData%\Wakeshot-harness\haik-out\` (`baseline.json`, `check.log`, `requests.log`, `C2-lock.txt`,
   `B2-kill.txt`); `%LocalAppData%\Wakeshot-harness\{session-watch,hold,end-session}\logs\wakeshot.log`;
   `%LocalAppData%\Wakeshot\logs\wakeshot.log` (A1, A2, B2: processes tray, guard, restore); the System log for the
   whole time (Kernel-Power 105, 506, 507, 566, 42, 107; Winlogon 7001 and 7002 for the sign-outs and sign-ins).
2. Pass or fail, with log lines:
   - A: for each run, guard lines "tray pid ... is gone", the three writes back, "journal deleted", "RunOnce value
     WakeshotRestore removed". If A2's End task closed the console instead, the tray's own "stopping: the console
     window closed" and a Restore (Quit) show instead: record which. Haik's notes give the grouping. The real tray and
     its guard are the same exe; how Task Manager groups those two is not shown by the harness (its tray is
     `power_check.exe`): not verified until integration.
   - C: session-watch lines for lock and unlock, twice each, with `locked_now`; `C2-lock.txt` exit 0.
   - D: in the hold log, the suspend and a "GAP slept" for Start > Sleep, the resume, "requests retaken after the
     resume", then ticks with no GAP through the next screen off (Kernel-Power 506 "Idle Timeout", about 180 s after the
     sign-in) and 3 minutes past it. A GAP there fails checklist 3.
   - E: "lid closed", then ticks every 5 s with no GAP while input idle passes 916 s, "lid open"; the copy deleted at
     exit. A GAP fails checklist 1.
   - B1: end-session's `WM_QUERYENDSESSION` and `WM_ENDSESSION` lines, each restore step's time, the copy deleted,
     against the sign-out time (Winlogon 7002). Checklist 6.
   - B2: `B2-kill.txt` READY; after sign-in (Winlogon 7001) the restore process's lines: the copy's entries written
     back, journal deleted, RunOnce removed. Checklist 5.
   - `requests.log`: any DISPLAY, SYSTEM or EXECUTION entry other than `power_check.exe` during D or E spoils that
     check (redo only that one). Except a SYSTEM `[DRIVER] Legacy Kernel Caller` in the one sample right after a
     resume that is gone by the next sample: the prep saw one 1 s after a Modern Standby exit (16:44:40), gone 11 s
     later. A driver entry still there through D's screen off, or at any time in E, spoils the check.
3. `power_check cleanup`, then `check.ps1`: Haik's values against the baseline; put back any difference and read
   again. No "Wakeshot test" plan, no journal, no `WakeshotRestore` value.
4. Copy the logs to the worker's scratchpad; delete `%LocalAppData%\Wakeshot-harness`, `%LocalAppData%\Wakeshot` and
   `<S2>\target`; read the values once more by `powercfg /qh`.
5. `docs/research.md` "Power track manual checks"; this card's Result.

## Found outside scope

Problems seen but not fixed.

1. **Checklist 4 cannot fully run on the harness.** The harness's tray is `power_check.exe`, while the real tray and its
   guard are one exe (`wakeshot-tray.exe`). Whether Task Manager puts the real tray and its guard in one group (so
   "End task on its group" would end both and leave only RunOnce) shows only with the real tray: integration
   (architecture section 9, checklist 4; section 4 "Processes").
2. **A hold that starts before Explorer's RunOnce pass of that sign-in loses its RunOnce value for the rest of the
   session.** Affects architecture section 5, "Logon restore" and "Gap left". Explorer runs HKCU RunOnce once per
   sign-in, after every Run entry: 64.6 s and 39.0 s after sign-in here (research, "Power track manual checks").
   A value the tray sets before that pass is run at once: Windows deletes it, restore mode finds the owner alive and
   exits (`recovery.rs:756`), and nothing sets it again until the next apply that writes a value, since
   `set_run_once` has one caller, apply step 3 (`wakeshot-core/src/ownership/apply.rs:144`). Then, for the rest of
   that hold, an OS crash, a power cut, or End task on tray and guard together leaves nothing to restore at the next
   sign-in when start at login is off: lid Do nothing and battery Never stay until Wakeshot next starts. With start at
   login on, the tray's own start restores (section 4 "Tray startup" step 6), so nothing is lost. The window is real
   with start at login on (the tray starts in the Run pass, right before the RunOnce pass) and with a manual start in
   the first minute. Cause **verified** (B2's logs: Shell-Core 9707 at 18:15:18.653, restore mode's `OwnerAlive` line
   at 18:15:18.683, `B2-kill.txt` `RunOnce: none`); that the real tray hits it: inferred from the code, not tested.
   Proposed fix, my pick: in `PowerExecutor::keep_guard` (`wakeshot-windows/src/power/executor.rs:255`), when the
   journal holds a known entry this tray owns, call `set_run_once` next to `ensure_guard` (it reads first and writes
   only a missing value, `power/journal.rs:556`), and log a failure like a guard start failure. The value then comes
   back within one ReadPlan (10 s in a hold), whoever removed it, the same rule as a guard that ended. Cost: one
   registry read per ReadPlan while something is owned. Other option: restore mode sets the value again when it finds
   the owner alive; no periodic read, but it breaks "only the tray sets RunOnce" (`journal.rs:353`) and races the
   owner's own clean-up (a stale value, harmless: the next sign-in's restore mode finds no journal). Not built or
   tested here (out of scope: no code fixes).

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:43, orchestrator: Haik's own battery sleep is now 4 h (DC 14400, read 20:41), not 600 s. Step F as
  written ("Haik's own 600 s") would wait over 4 h: run it on an active plan copy with DC sleep 600 s, or skip it and
  say so in the Result. Read his values live before and after; never assume 600.
- 2026-11-04 01:08, worker (bundle after `p3-real-checks`, done 00:54): steps for Haik written ("Steps for Haik"),
  with five decisions under "Questions for Haik". No step of this card was started; nothing of it touched the power
  settings. Prepared and smoke-tested in `<S2>` (path in "Steps for Haik"): `scripts\check.ps1` (run with
  `LOCALAPPDATA` pointed at a scratch folder: `-Save` and the compare gave ALL OK on Haik's live values),
  `scripts\kill.ps1` (with no fake-tray running it stopped with "found 0", ending nothing), `scripts\C2-lock.ps1` and
  `scripts\requests-log.ps1` (parse only: not run), `make-cmd.py` (wrote the 12 step files into a scratch folder). A
  `.cmd` opened by Explorer that runs `start "" conhost.exe cmd /c ...` got a `ConsoleWindowClass` window, not a
  Terminal tab (01:03). Not built yet: the worker prep builds at resume, so the run uses the final tree.
- 2026-11-04 01:05, worker, tool fact: running `python` (the WindowsApps alias) with `LOCALAPPDATA` set to a scratch
  folder made the Python install manager install Python 3.15.2 into that folder. The real
  `%LocalAppData%\Python` (last write 2026-11-02) and the `PythonManager` package (26.4.240.0) were not changed; the
  scratch folder was deleted. Never point `LOCALAPPDATA` elsewhere for a process that starts `python`.
- 2026-11-04 01:08, worker: Status `blocked: waiting for Haik`. Haik's values at 00:52:26 (end of `p3-real-checks`):
  Balanced active and the only plan, lid AC 1 and DC 1, sleep AC 18000 and DC 18000, energy saver DC 30. The prep reads
  them again live as the baseline. The laptop was on battery and undocked from 00:30 (see `p3-real-checks` Found
  outside scope 1).
- 2026-11-04 16:42, worker (prep): started "Worker prep" steps 1 to 7. Orchestrator steps 1 to 3 done (no other
  agent or build; battery 99% on AC; Haik signed in, lid open). Prep step 1: `powercfg /requests` at 16:42:08 says
  None for DISPLAY, SYSTEM, AWAYMODE, EXECUTION, PERFBOOST, ACTIVELOCKSCREEN. Next: the builds, and a check of the
  harness and step files against today's code (commits up to e819f4b).
- 2026-11-04 16:44, worker (prep): step 2, the three builds into `<S2>\target` (host x64, debug, no `RUSTFLAGS`) all
  exit 0 (16:42:22 to 16:44:02): `power_check.exe`, `wakeshot-tray.exe`, test binary
  `deps\wakeshot_windows-a7dbc494c7637936.exe` (the only `wakeshot_windows-*.exe` in `deps`, so `C2-lock.ps1` finds
  it). Step 3, `check.ps1 -Save` at 16:44:18, baseline: active Balanced `381b4222-f694-41f0-9685-ff5bb260df2e` (the
  only plan), lid AC 1 and DC 1, sleep AC 18000 and DC 18000, energy saver DC 30; no journal; no RunOnce
  `WakeshotRestore`. `HKCU\...\Run` has no Wakeshot value (7 values: OneDrive, Chrome, LenovoVantage, TrafficMonitor,
  Docker Desktop, GoogleDriveFS, Edge); RunOnce empty; `WAKESHOT_DATA_DIR` unset (user, machine, process). Display
  off DC 180 s (D's "about 3 minutes" holds); console lock display off 30 s.
- 2026-11-04 16:44, worker (prep): step texts against today's `power_check.rs` (changed 11:54) and `recovery.rs`,
  `power\journal.rs`: every line Haik waits for is still printed word for word (`holding:`, `integrity ... elevation
  type ...`, `registered for session notices`, `SYSTEM and EXECUTION power requests set`, `tick <n>`, `requests
  retaken after the resume`, `GAP`, `the copy <guid> is the active plan now`, `lid closed`, `lid open`, `hidden
  top-level window up: sign out to run check 6`); the guard command line is still `--guard <pid> <start> <event>`
  (`kill.ps1` matches it); the after-reading lines `tray pid ... is gone`, `journal deleted`, `RunOnce value
  WakeshotRestore removed` still exist; `session::tests::real_lock_locks` unchanged (session.rs last changed
  2026-11-03 20:45). The step files' arguments all parse in today's `cli::parse`.
- 2026-11-04 16:44, worker (prep): step 4, `python -I <S2>\make-cmd.py` (real `LOCALAPPDATA`) wrote the 12 step files
  to `%LocalAppData%\Wakeshot-harness\haik`. Step 5, `requests-log.ps1 -Minutes 150` started hidden and elevated at
  16:44:39: **pwsh PID 33660** (its own `requests-log.pid` says the same; ends by itself about 19:14, or at B1's
  sign-out). Its first sample (16:44:40) showed SYSTEM `[DRIVER] Legacy Kernel Caller` twice and PERFBOOST entries,
  1 s after a Modern Standby exit; read again at 16:44:51: None for every type.
- 2026-11-04 16:44, worker (prep), power source: Kernel-Power 105 at 16:44:25, the laptop is on battery since then
  (99%, `PowerOnline` False at 16:44:51; Haik or the DELL cable, not known). At 16:44:26 it entered Modern Standby
  ("Idle Timeout": input idle was already past the 180 s DC display timeout when DC began) and left it at 16:44:39
  ("Input Keyboard"). The prep's writes were done or not started then; nothing of it was cut.
- 2026-11-04 16:45, worker (prep): step 6, `power_check fake-tray --plan-only` (elevated, so its line says
  `integrity High (0x3000), elevation type Full`; Haik's Explorer runs say Medium) exit 0: values at start and exit
  the same (Balanced, lid 1 and 1, sleep 18000 and 18000, energy saver DC 30), RunOnce command and guard path are
  `<S2>\target\debug\wakeshot-tray.exe`, data folder `%LocalAppData%\Wakeshot`; afterwards no `Wakeshot` data folder,
  no RunOnce value, no test plan, the harness folder unchanged.
- 2026-11-04 16:46, worker (prep): step 7, `explorer.exe %LocalAppData%\Wakeshot-harness\haik` from the elevated
  shell opened the folder in a new Explorer folder host (pid 17292, `/factory,{75dff2b7-...} -Embedding`); its token
  reads Medium (0x2000), like the shell Explorer 12528, so a double-click there runs Medium (token read with
  `GetTokenInformation(TokenIntegrityLevel)`). `requests.log` samples every 30 s (16:44:40 to 16:46:10, battery 98%).
  Prep done, no cargo command from here on; nothing of this worker runs except `requests-log.ps1` (PID 33660).
  Status `blocked: waiting for Haik`.
- 2026-11-04 (orchestrator `wakeshot-27`, before B): Haik's notes from A, C, D and E, his words: "A. 2. Yes process
  has > and 1 row with no icon as the process itself and no name no usage. So basically it's group with something
  empty. 3. Wakeshot is 1 process with icon. all checks were ok. C. everything seems ok D. everything seems ok E.
  everything seems ok". Not said: "Apps" or "Background processes" in A2; GAP lines in D and E (read the logs).
  Haik swapped the DELL dock for the charger at 16:44 (the prep worker's battery note). B comes next; the
  sign-outs end the orchestrator, so the "After Haik's steps" worker reads this line.
- 2026-11-04 18:19, worker ("After Haik's steps"): started. Haik's report on B, his words: "in B everything was ok only
  the last check gave not ok". The last `0-check.cmd` (B2 step 4) at 18:16:28: plan, lid, sleep, energy saver OK,
  RunOnce `WakeshotRestore` none, but journal LEFT and the copy `40fece43-...` (Wakeshot test 41604) left, not active.
  The orchestrator told Haik not to run `0-cleanup.cmd`. Order: preserve the evidence first, then find why the
  sign-in restore left the journal and the copy, then steps 1 to 5.
- 2026-11-04 18:19, worker: evidence copied before any change to `<scratchpad>\p3-haik-checks\evidence` (this
  session's scratchpad): both data folders, `powercfg /list` and `/qh` of Balanced and of the copy, RunOnce and Run,
  the System, Application, Shell-Core and Winlogon logs from 16:40. At 18:19:33 the journal (owner pid 41604, session
  2, three entries on the copy, none restored) and the copy were still there, Balanced active, no RunOnce value: the
  restore was not slow, it never ran at the second sign-in. Haik did not run `0-cleanup.cmd`: `check.log` shows five
  `0-check` runs (18:16:28 to 18:17:16), all NOT OK the same way. `requests-log.ps1` (pwsh 33660) is gone; its log ends
  18:13:51, before B1's sign-out (7002 at 18:14:08).
- 2026-11-04 18:25, worker, B2 cause (verified from logs): Explorer's per-sign-in HKCU RunOnce pass ran after B2 had
  started, not at B2's sign-in. Shell-Core Operational, Explorer pid 34656 (the shell of B1's sign-in, 7001 at
  18:14:14): Run keys 18:14:22 to 18:15:12, then `9705 Started enumeration ... RunOnce` 18:15:18.653, `9707 Started
  execution of command 'wakeshot-tray.exe" --restore'`, `9708 Finished ... (PID 35732)` 18:15:18.670. B2's fake-tray
  had set the value at 18:15:02.337. Windows deleted the value and ran restore mode, which logged at 18:15:18.683
  `restore mode: the journal's owner (pid 41604 start 134382753022985918) runs and restores it itself` and exited
  (`recovery.rs:756`, `Alive::Yes` gives `OwnerAlive`). `B2-kill.txt` 18:15:21.964: `RunOnce: none`, then `NOT READY`;
  Haik signed out anyway (7002 18:15:53). The second sign-in's RunOnce pass (Explorer 19640, 18:16:37.290) found
  nothing. Harness timing, plus a real product gap: see Found outside scope 2.
- 2026-11-04 18:24, worker: logs read (A, C, D, E, B1, B2: outcomes in the Result). `power_check cleanup` (elevated)
  at 18:24:50: restored the journal in `%LocalAppData%\Wakeshot` (owner 41604; copy lidAc 0 to 1, lidDc 0 to 1,
  sleepDc 0 to 18000), journal deleted, run record removed, the copy `40fece43-...` deleted; exit 0. `check.ps1` at
  18:24:58: `check: ALL OK`. Logs after the cleanup copied to `<scratchpad>\p3-haik-checks\evidence\after-cleanup`.
- 2026-11-04 18:26, worker: deleted `%LocalAppData%\Wakeshot`, `<S2>\target` and every part of
  `%LocalAppData%\Wakeshot-harness` but the empty `haik` folder: it is the working folder of Haik's open
  `0-check.cmd` window (cmd pid 20600, started 18:16:27, waiting at `pause`), so Windows refuses the delete; not my
  process, left running. `<S2>\scripts` and `make-cmd.py` kept for a B2 redo. `powercfg` at 18:26:07: Balanced
  active and the only plan; lid AC 1, DC 1; sleep AC and DC 0x4650 (18000); energy saver DC 0x1e (30); no RunOnce
  value, no journal; `HKCU\...\Run` the same 7 values, no Wakeshot; `powercfg /requests` None for every type.
- 2026-11-04 18:31, worker: `docs/research.md` "Power track manual checks" added; Found outside scope 2 and the Result
  written; Status `done`. The empty `haik` folder was still in use at 18:28 (second try). No product code changed, no
  git.

## Result

Status: done (2026-11-04 18:31). Deliverable 1 (steps for Haik, prep) done 16:46; Haik ran A, C, D, E, B1, B2
(16:55 to 18:17); deliverable 2 (reading, cleanup, values) done 18:19 to 18:31 by a new worker. F skipped (decision
1). Facts in `docs/research.md`, "Power track manual checks".

Outcomes, with the log lines in the Log and in research:

| Check | Outcome |
|---|---|
| A, checklist 4 | Values back both runs (pass for "End task leaves Haik's values right"). The guard path did **not** run: A1 and A2 both logged `stopping: the console window closed` and `restore (Quit): ok` (15.6, 12.0 ms), the guard `stopped`; Task Manager's End task closed the console. Guard path not verified here (shown before by pid kill, `p6-int-real-e2e`). Haik: `power_check` has an arrow with one unnamed row; the guard `wakeshot-tray.exe` is one row of its own with an icon. Apps or Background: not noted. Real tray's grouping: integration (Found outside scope 1) |
| C, session notices | Pass: lock 17:30:51 and 17:31:18, unlock 17:31:03 and 17:31:34, each `locked yes` or `no`; `real_lock_locks` exit 0 |
| D, checklist 3 | Pass: one `GAP slept` for Start > Sleep (17:33:29 to 17:33:58), `requests retaken after the resume`, then no GAP through the screen off (Kernel-Power 506 17:37:15) and 4 min 49 s past it, up to Haik's touch at 17:42:04; no other request |
| E, checklist 1 | Pass: `lid closed` 17:43:50 to `lid open` 18:08:25, 307 ticks, 0 GAP, input idle up to 1477.6 s; the copy deleted at exit; no other request |
| B1, checklist 6 | Pass: `WM_QUERYENDSESSION` 18:14:02.769, restore done in 19.3 ms (journal deleted, RunOnce removed), `WM_ENDSESSION` 18:14:02.817, copy deleted 18:14:02.820; Winlogon 7002 at 18:14:08.509 |
| B2, checklist 5 | **Not shown.** Harness timing: B2 started 48 s after B1's sign-in, before Explorer's RunOnce pass of that sign-in (18:15:18.653, after all Run entries), which ran restore mode at once; it found the owner alive and exited, and Windows had deleted the value. `B2-kill.txt` said `RunOnce: none`, `NOT READY` at 18:15:21; Haik signed out anyway. The pass also shows a real product gap: Found outside scope 2 |

Cleanup: Haik did not run `0-cleanup.cmd`. `power_check cleanup` (18:24:50) restored the B2 journal on the copy and
deleted the copy; `check.ps1` `ALL OK` (18:24:58). Haik's values at 18:26:07 by `powercfg` equal the 16:44 baseline:
Balanced active and the only plan, lid AC 1 and DC 1, sleep AC and DC 18000, energy saver DC 30; no journal, no
`WakeshotRestore` value, Run unchanged (7 values, no Wakeshot), `powercfg /requests` None. No difference to put back.
`requests-log.ps1` (pwsh 33660) ended at B1's sign-out by itself.

Acceptance checks: values before and after equal: **pass**. No test plan, no journal, no RunOnce value: **pass**. A to
E with pass or fail and log lines, in the Result and research: **pass**, with B2 "not shown" and A's guard path "not
verified". A design-changing finding under Found outside scope with its section: **pass** (item 2, section 5).

Files changed: this card; `docs/research.md` (new section "Power track manual checks", additions only). Deleted:
`%LocalAppData%\Wakeshot`, `<S2>\target`, all of `%LocalAppData%\Wakeshot-harness` but its empty `haik` folder.
Evidence (logs, event exports, journal, `powercfg` reads, before and after the cleanup) in this session's scratchpad:
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\af61f686-7f48-4802-a60d-f1a8fa567c5f\scratchpad\p3-haik-checks\evidence`.

Other findings (research, no design change): at sign-out Windows ends the guard first (exit code 0x40010004,
`DBG_TERMINATE_PROCESS`; cause not verified), and the tray starts a new one during its end-session restore, stopped
11 ms later. The first guard start of a new exe took 3.4 s (later 8 to 10 ms; cause not verified).

Open items:

1. Found outside scope 2: the RunOnce value lost to Explorer's per-sign-in pass. A fix card (power track, executor) or
   an entry in "Gap left": orchestrator's and Haik's call. My pick: the fix, one `set_run_once` call in `keep_guard`.
2. Checklist 5 still needs a B2 redo, best after that fix so it tests the final code: about 5 minutes of Haik, one
   sign-out, a worker prep first (build into `<S2>\target`, baseline, `make-cmd.py`). Two changes to the B2 steps:
   start `B2-fake-tray.cmd` at least 2 minutes after the sign-in, and wait 2 minutes after the next sign-in before
   `0-check.cmd` (Haik's first check came 30 s after sign-in, the pass 39 s). Also make `kill.ps1`'s `NOT READY`
   impossible to miss (Haik read B as all OK).
3. The empty folder `%LocalAppData%\Wakeshot-harness\haik` stays until Haik closes the open "Wakeshot check: values"
   window (cmd pid 20600); then `rd /s /q "%LocalAppData%\Wakeshot-harness"`. `<S2>\scripts` and `<S2>\make-cmd.py`
   kept for the redo.
4. Found outside scope 1 (the real tray's Task Manager group) at integration.
