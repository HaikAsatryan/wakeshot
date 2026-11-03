# Haik's checks for the power track

Phase: 3. Status: blocked: waiting for Haik (2026-11-04 01:08; steps under "Steps for Haik"). Needs Haik: **yes**:
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
smoke-tested; not yet run with the build).

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
     check (redo only that one).
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

## Result

Status: blocked: waiting for Haik (2026-11-04 01:08). Deliverable 1 is done: "Steps for Haik" (A, C, D, E, B1, B2;
about 60 minutes, about 15 hands-on), the orchestrator's and the worker's prep before them, and the reading after them.
No check has run yet; deliverable 2 and the acceptance checks wait for Haik's steps.

Files changed: this card. Scratch: `<S2>\make-cmd.py` and `<S2>\scripts\` (`check.ps1`, `kill.ps1`, `C2-lock.ps1`,
`requests-log.ps1`). Nothing on the laptop changed: Haik's values read at 01:09:12 are the same as at 00:52:26
(Balanced only and active; lid 1 and 1; sleep 18000 and 18000; energy saver DC 30; no journal, no RunOnce value, no
`%LocalAppData%\Wakeshot` or `Wakeshot-harness`).

Decided by the worker, for Haik's review: the five rows under "Questions for Haik" (F skipped; B as two sign-outs;
Haik starts each run himself; logs in `%LocalAppData%\Wakeshot-harness`; A in a classic console window).

Open items: the whole run; Found outside scope 1 (the real tray's Task Manager group shows only at integration).
