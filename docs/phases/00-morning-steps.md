# Phase 0: Haik's morning steps

**Your time: about 80 minutes, about 30 of them hands-on.** In C and H you may walk away (about 23 minutes). In E
and I you stay near the closed laptop (about 26 minutes). Optional at the end: B5 (5 minutes) and T13 (1 minute).
After your Final check, agents use the laptop for about an hour: leave it on AC and unlocked, and do not unplug it or
close the lid until the orchestrator says "agent steps done". Before the trim (card `p0-morning-trim`, 2026-11-03) the
list took about 2 hours plus 35 minutes of optional steps.

## Stopping early: read this first

If you stop before the Final check, for any reason (a call, a meeting, done for now):

1. If a tool is still running, press Ctrl+C once in its window and wait until it ends.
2. In the morning shell, run `restoreall`, then `checkpower`.
3. Do not close the lid or put the laptop away until `checkpower` ends with `checkpower: ALL OK`. If it does not,
   follow "Manual restore" below.
4. If the morning shell window is gone, double-click `C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd` in File
   Explorer to open it again first.
5. Tell the orchestrator where you stopped. To go on later, tell it and wait for its "go": it repeats the step 0
   checks first (power requests, no other agent running).

Every block ends with `checkpower`, so stopping between two blocks costs nothing more.

## About this list

Only the steps that need your hands, eyes or ears. Agents do the rest before you start and after you finish (lists in
the three spike cards, under "Moved to agent"). Steps keep the letters the cards use, so the cards' reading guides
still match. Where a letter is gone, the block says so; the full map is in "Letters that moved or are gone" at the end.

New order: T10 first, then D, E, F and H on AC, then C and I back to back on one battery charge. So the DELL is
unplugged once and the charger comes out for one long stretch.

Lines that start with **Orchestrator** are for the orchestrator session. You wait for it only three times: its "go"
in step 0, its "go" after a break, and its "agent steps done" after the Final check.

One window: the **morning shell**. The orchestrator opens it for you in step 0, through File Explorer, so it has your
normal, non-admin token. Copy each command from this file and paste it there. The admin Terminal (Start > Terminal)
is used only in manual restore step 6.

Checked before (card `p0-spike-safety-fixes`): the morning shell gives every tool the Medium token; every command in
the manual restore block works from it (CONSOLELOCK needs admin); Ctrl+C and closing the morning shell's window both
put the plan back.

## Read first: how to stop and put everything back

These are your own values today (read 02:04, 02:31, 03:04, 03:30 and 04:26 on 2026-11-03):

| Setting | AC | Battery |
| --- | --- | --- |
| Power plan | Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`, the only plan, active | same |
| Lid action | 1 (Sleep) | 1 (Sleep) |
| Sleep after | 10800 s (3 h) | 600 s (10 min) |
| Screen off after | 300 s | 180 s |
| Wake timers | 0 (Disable) | 0 (Disable) |
| Energy saver turns on at | 0 | 30% |
| Sign-in after sleep (CONSOLELOCK) | 1 | 1 |
| Power mode (overlay) | `ded574b5-45a0-4f42-8737-46345c09c238`, "Best performance" (Microsoft's GUID for that mode) | none set |

**To stop any run:** press Ctrl+C once in its window. The tool puts your plan back and ends. Closing the window also
works: the plan was back within half a second when tested. Closing it also closes the morning shell, so open it again
(double-click `morning-shell.cmd`). Then, in the morning shell, run `restoreall` and `checkpower`.

**If the PC restarted, shut down, ran out of battery, or you signed out during a step:** these do not reach the
tools, so a test plan copy can still be active (in the lid steps: one with lid action Do nothing). Double-click
`C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd`, then run `restoreall`, then `checkpower`.

**`restoreall`** runs `lidspike restore` and `spike restore`. Each must print `no journal: nothing to restore` or
`done, journal removed`. **`checkpower`** compares everything above with your values, shows each difference in red
and must end with `checkpower: ALL OK`.

**Manual restore**, only if `restoreall` prints `FAILED` or `bad journal`, or `checkpower` is not ALL OK after
`restoreall`. Steps 1 to 5 go in the morning shell.

1. Run:

   ```
   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
   powercfg /list
   ```

2. For every plan in the list other than `381b4222-f694-41f0-9685-ff5bb260df2e` (Balanced), run
   `powercfg /delete <that GUID>`.
3. Run all of these. Each one sets your own value, so running a line that was already right is harmless:

   ```
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_BUTTONS LIDACTION 1
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_BUTTONS LIDACTION 1
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP STANDBYIDLE 10800
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP STANDBYIDLE 600
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_VIDEO VIDEOIDLE 300
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_VIDEO VIDEOIDLE 180
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP RTCWAKE 0
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP RTCWAKE 0
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_ENERGYSAVER ESBATTTHRESHOLD 0
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_ENERGYSAVER ESBATTTHRESHOLD 30
   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
   ```

4. Delete the journals if they exist:

   ```
   Remove-Item C:\Users\haika\code\wakeshot\spikes\power-settings\out\journal.txt, C:\Users\haika\code\wakeshot\spikes\lid-sleep\out\journal.txt -ErrorAction SilentlyContinue
   ```

5. Only if `checkpower` says Power mode is DIFFERENT: set Settings > System > Power & battery > Power mode back to
   Best performance. If that does not fix it, run `spike set-overlay ded574b5-45a0-4f42-8737-46345c09c238` in the
   morning shell (an undocumented Windows call; not tried yet).
6. Only if `checkpower` says sign-in after sleep is DIFFERENT: this one needs admin. In the admin Terminal, run:

   ```
   powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_NONE CONSOLELOCK 1
   powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_NONE CONSOLELOCK 1
   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
   ```

7. Go back to the morning shell (`checkpower` does not exist in the admin Terminal) and run `checkpower` again. If it
   is still not ALL OK, stop and tell the orchestrator what it printed.

## 0. Start (you close media apps, then wait for "go")

Laptop on AC, DELL connected, lid open, signed in.

1. Pause or close anything that plays sound or video, and keep it that way until the Final check. (At 09:50 and
   10:03 Chrome held a "Video Wake Lock" and a "Playing audio" request. Either spoils C, E and I.)
2. **Orchestrator**, before Haik starts, in this order (details in the spike cards, "Moved to agent"):
   1. Check that `powercfg /list` shows only Balanced, active; that neither `spikes\power-settings\out\journal.txt`
      nor `spikes\lid-sleep\out\journal.txt` exists; that no `power-settings-spike`, `lid-sleep-spike` or `tray-spike`
      process runs; and that no worker or reviewer agent is running. Write a BEFORE line into
      `p0-spike-power-settings` and `p0-spike-lid-sleep` with the values read live by `powercfg /q` (not copied from
      the table), plus `DelayLockInterval` and both overlay values.
   2. Tray card T1 and T2: start the tray spike with the Medium token through `explorer.exe`, check its `token:` line,
      move its icon onto the taskbar, check with a screenshot. It stays running, idle, until the tray steps after the
      Final check. It holds no power request.
   3. Battery at 70% or more (D to H run on AC, so it only has to be charged before C). Volume, per device by endpoint
      ID (the lid spike's sound goes to the default device): the laptop Speakers were at 67% and the DELL U2723QE
      (default while connected) at 24% at 09:50, neither muted. E and I play on the Speakers. Change nothing unless
      the Speakers are muted or under 50%; then set them to 50% and note the old value to put back at the end.
   4. Stop the keep-awake by PID (`pwsh` PID 11284 since 02:26; check `docs/plan.md` Status for the current PID). From
      the elevated shell:

      ```
      if ((Get-CimInstance Win32_Process -Filter 'ProcessId=11284').CommandLine -match 'keep-awake') { Stop-Process -Id 11284; 'keep-awake stopped' } else { 'keep-awake already gone' }
      powercfg /requests
      ```

      DISPLAY, SYSTEM and EXECUTION must each say `None.`. If one names an app, ask Haik to close it and check again.
      Run `C:\Users\haika\code\wakeshot\spikes\lid-sleep\target\release\lid-sleep-spike.exe limited read` and note the
      `SystemExecutionState` line (0x0 expected with no holds).
   5. Start the requests log: a background `pwsh` from the elevated shell that appends the time and the output of
      `powercfg /requests` to a file in the orchestrator's scratchpad every 30 s, ends by itself after 3 hours, and
      writes its PID to a file. Read only: it holds no request and needs no screen. Stop it by PID at the Final
      check. Afterwards, any DISPLAY, SYSTEM or EXECUTION entry during a block from D to I, other than the spike's
      own, marks that block as spoiled; only that block is redone.
   6. Open the morning shell: `explorer.exe C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd`. Check that the
      newest line of `spikes\morning-shell.log` says `integrity Medium admin-enabled False`, and that the values read
      as in the table above.
   7. From "go" until the Final check, or until Haik says he stopped: start no builds, no other agent, no keep-awake
      and no screen steps. They can add a power request, and heat while the lid is closed. Before Haik goes on after
      a stop: repeat checks 1 and 4, then say "go" again.
   8. Tell Haik "go".
3. A Windows Terminal window titled "Wakeshot morning shell (Medium)" is open. Every command below goes there.

Gone here: 0.2 to 0.4 as your steps (the orchestrator opens the shell and checks its token and values).

## T10. Explorer restart (1 minute)

The white circle in the taskbar is the tray spike. The orchestrator started it; leave it until the end.

1. Press Ctrl+Shift+Esc. In Task Manager > Processes, right-click "Windows Explorer" > Restart. Open File Explorer
   windows close. Close Task Manager.

Nothing to write down: the spike logs and captures its icon by itself.

## D. Dock, then undock and unplug with the lid closed (about 7 minutes)

Charger in, DELL connected. Nothing plays sound or video. Keep the laptop on a hard desk for every closed-lid step
(not in a bag, not on a bed). In D, E and I, never leave the laptop while the lid is closed. If you must go, open the
lid, press Ctrl+C in the morning shell, then follow "Stopping early". After a run ends, Windows may not sleep by
itself with the lid closed (that is what E measures), so a closed laptop left alone can keep running. The tools' 85 C
stop is only a late backstop; your hand on the bottom of the laptop is the real heat check.

**Too hot** (in D8, E6 or I4, too hot to keep a hand on): keep the lid open, run `checkpower` (as in "Stopping
early"), let the laptop cool on the desk, and tell the orchestrator before you go on.

Gone here: D1 (the orchestrator stopped the keep-awake in step 0), D2 (step 0 checked the same, and the lid tool
refuses to start with a leftover copy or journal), D3 (the heat run: no heat guard, decision 2).

4. Run `lidspike lid --end none --no-lock --max-minutes 8`.
5. At `>>> READY: close the lid now <<<`, close the lid. Did you hear a short sound as it closed?
6. Wait 30 seconds. With the lid still closed, unplug the DELL cable. Wait 60 seconds.
7. With the lid still closed, unplug the charger. Wait 60 seconds. Plug the charger back in. Wait 30 seconds.
8. Open the lid. Wait for the line that starts with `DONE` (about 20 seconds). Touch the bottom of the laptop (too
   hot: see "Too hot" above).
9. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

Time limit: the run ends by itself after 8 minutes. The DELL stays unplugged until the end of I.

Write down: sound in D: from the DELL, from the laptop, or none. (While the DELL is connected it is the default sound
device.)

## E. Lid closed, laptop alone, on AC (about 7 minutes)

1. Charger in, DELL unplugged (as D left them).
2. Run `lidspike lid --no-lock --max-minutes 8`. (No lock on purpose: does Windows lock by itself?)
3. At READY, close the lid. Listen for the sound.
4. Leave it closed for 4 minutes. Do not touch it, but stay near it.
5. Open the lid. If the screen stays black, press a key; if still black, press the power button once.
6. Sign in if asked. Wait for DONE. Touch the bottom of the laptop (too hot: see "Too hot" in D).
7. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

Time limit: the power request ends 60 seconds after the lid closes, and the run ends by itself after 8 minutes.

Write down: sound in E. (What the screen showed at open is in the log.)

## F. Sleep calls, lid open, on AC (about 7 minutes)

1. Run `lidspike suspend --method sleep`. 10 seconds later it asks Windows to sleep. If the screen goes dark: wait 30
   seconds, press the power button, sign in, wait for DONE. If nothing happens after 30 seconds: press a key and wait
   for DONE.
2. Run `lidspike suspend --method hibernate`. If the laptop turns off: wait until the power light is off plus 30
   seconds, press the power button, sign in, wait for DONE. If nothing happens after 30 seconds: press a key and wait
   for DONE.
3. Only if the RESULT line of step 1 says `did not sleep` (scroll up): run `lidspike suspend --method screen-off`.
   After the screen goes dark, do not touch the laptop for 3 minutes. Then press a key, sign in and wait for DONE.
4. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

Nothing to write down: the RESULT lines hold what each call did.

## H. Resume, on AC (about 10 minutes, 8 of them away)

1. Run `lidspike resume`. When it prints `>>> Now: Start > Power > Sleep`, do that.
2. Wait 20 seconds. Press the power button and sign in.
3. Do not touch the laptop; you can walk away. The screen turns off after 1 minute; that is expected. After about 8
   minutes, press a key, sign in and wait for DONE.
4. Only if the RESULT line says `request NOT honored`: run `lidspike resume --retake` and repeat steps 1 to 3.
5. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

## C. Battery rule, lid open (about 17 minutes, 15 of them away)

C and I run back to back on battery. Look at the battery icon: 70% or more. If less, charge first. Nothing plays
sound or video.

Gone here: C1 (step 0 closed media apps and checked power requests and the volume), C2 (the orchestrator stopped the
keep-awake in step 0), C8 (it checks after the agent steps).

3. Unplug the charger.
4. Run `spike hold`. It makes a temporary plan copy with battery sleep after 60 s, holds a power request for 12
   minutes, logs every 5 s, then puts your plan back and deletes the copy.
5. Do not touch the keyboard, touchpad or mouse for 15 minutes. Keep the lid open. You can walk away.
6. Come back after 15 minutes or more. If the screen is dark, wake the laptop and sign in. Wait until the window
   prints `clean: original plan active, copy removed`.
7. Leave the charger out: I follows. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

## I. On battery, lid closed for 22 minutes (the architecture review's "I2")

About 26 minutes. Still on battery from C, DELL unplugged.

1. Check that the charger is still out: the battery icon shows no plug. (The lid tool does not refuse on AC, and on
   AC this block proves nothing.)
2. Run `lidspike lid --work 1200 --max-minutes 30`. This one locks the PC at lid close.
3. At READY, close the lid and listen for the sound. Keep the laptop on a hard desk, stay near it, and leave it closed
   for 22 minutes.
4. Open it, sign in and wait for DONE. Touch the bottom of the laptop (too hot: see "Too hot" in D).
5. Plug the charger back in. Plug the DELL back in.
6. Run `checkpower`: ALL OK. If not: `restoreall`, then `checkpower` again.

Time limit: work ends 20 minutes after the lid closes, and the run ends by itself after 30 minutes.

Write down: sound in I.

## J and K. Report (1 minute)

1. Gone: I6 already ran `checkpower`.
2. Tell the orchestrator: the sound at lid close in D (from the DELL, from the laptop, or none), E and I (yes or no
   each), and anything odd. The logs hold the rest.
3. **Orchestrator:** the K3 check runs after the agent steps.

## Final check (1 minute)

1. Run `restoreall` and `checkpower`: `no journal` twice, then ALL OK. Your hands-on part is done.
2. Leave the laptop open, signed in, unlocked and on AC. The agents now use the screen for about an hour (plan copy
   and Settings tour, then the tray steps). Do not unplug it, close the lid or lock it until the orchestrator says
   "agent steps done, checkpower ALL OK". If you must lock it, the agents put their changes back and wait.
3. If that message has not come within 90 minutes: double-click
   `C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd`, run `restoreall` and `checkpower` (manual restore if not
   ALL OK), then paste this line, which sets your sign-in delay (0; harmless if it is already 0):

   ```
   Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name DelayLockInterval -Value 0 -Type DWord
   ```

   Then tell the orchestrator.
4. **Orchestrator**, in this order:
   1. Stop the requests log by PID. Quick check: `powercfg /list` (only Balanced, active), `powercfg /requests`, both
      journals gone, no `power-settings-spike` or `lid-sleep-spike` process. Do not restart the keep-awake.
   2. Run the agent steps one at a time, never two on the screen at once: the power card's B and A, then the tray
      card's steps. The lid card's log reads need no screen and may run alongside.
   3. Last: the end check (`checkpower` ALL OK, `DelayLockInterval` 0, `ActiveOverlayDcPowerScheme` empty, theme
      values as before, `powercfg /requests` with nothing from an agent, no spike or holder process), the Speakers
      volume back if step 0 changed it, then the AFTER lines in both power and lid cards (old C8 and K3) with values
      read live by `powercfg /q`. Close the morning shell. Then tell Haik "agent steps done, checkpower ALL OK".

## Optional, at the end

**B5. Restart with a plan copy active (about 5 minutes).** Only after the orchestrator says "agent steps done": a
restart ends the orchestrator session. Part 3 item 4 of `docs/reviews/p0-close-proposals.md` lists it.

0. The orchestrator closed the morning shell, so double-click `C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd`
   first.
1. In the morning shell, run `spike copy-test --keep`. It takes about 30 seconds and ends with
   `--keep: the copy stays active`.
2. Restart Windows. Sign in.
3. Double-click `C:\Users\haika\code\wakeshot\spikes\morning-shell.cmd` in File Explorer. Red lines for the
   `Wakeshot spike copy` plan, `active plan is NOT Balanced` and the power journal are expected here.
4. Run `spike read` and write down the `active scheme` line.
5. Run `spike restore` (`done, journal removed`), then `checkpower`: ALL OK.

**T13. Look at the tray pictures (1 minute, any time after the orchestrator's tray steps).** Open
`C:\Users\haika\code\wakeshot\docs\research\img` and look at the `tray-*.png` pictures. Do the menus look native to
you, in light and in dark? Tell the orchestrator.

## Letters that moved or are gone

| Old step | Now |
| --- | --- |
| 0.1 | Orchestrator, step 0 |
| 0.2 to 0.4 | Orchestrator, step 0 (it opens the shell and checks it) |
| A1 to A8 | Agent, after the Final check, on a throwaway plan copy (power card) |
| B1 to B4, B6, B7 | Agent, after the Final check (power card); B1 runs first, B3 is done together with A6 |
| B5 | Optional, at the end |
| C1 | Yours in step 0 (close media apps once); the orchestrator checks requests in step 0 and logs them every 30 s |
| C2 | Orchestrator, step 0 |
| C3 to C7 | Yours; C7 (`checkpower`) keeps the charger out for I |
| C8 | Orchestrator, after the agent steps |
| D1, D2 | Gone (step 0 covers both) |
| D3 | Dropped: no heat guard (decision 2) |
| D5 "did the DELL stay on" | Dropped: no rule counts screens any more |
| E5, I4 "note what you see" | Agent reads the log (`locked` field, display events) |
| F, E and I notes | Agent reads the RESULT lines |
| I1 | Now: check the charger is still out (it is out since C3) |
| J1/K1 | Gone: I6 ran `checkpower` |
| K3 | Orchestrator, after the agent steps |
| T1, T2 | Agent, step 0 |
| T3, T4, T6 to T9, T12 | Agent, after the power card's agent steps (tray card) |
| T5 (sounds by ear), T11 (high contrast) | Dropped: see the tray card |
| T10 | Yours, first |
| T13 | Yours, optional, at the end |
| G, I3 | Dropped: nothing in the design uses them (part 3, "No consumer") |
| Optional older keep-awake run (`spike hold --request stes`, titled "C7" in the old list) | Dropped: no consumer either |

## Time per block

| Block | Minutes | Hands-on |
| --- | --- | --- |
| 0. Start (close media apps, then wait for "go") | 1 | 1 |
| T10. Explorer restart | 1 | 1 |
| D. Dock and undock, lid closed | 7 | 7 |
| E. Lid closed on AC | 7 | 3 (4 near the closed laptop) |
| F. Sleep calls | 7 (10 with F3) | 7 |
| H. Resume | 10 | 2 (8 away) |
| C. Battery rule | 17 | 2 (15 away) |
| I. Battery, lid closed 22 minutes | 26 | 4 (22 near the closed laptop) |
| J and K. Report | 1 | 1 |
| Final check | 1 | 1 |
| **Required total** | **about 80** | **about 30** |
| Optional B5, T13 | about 6 | about 6 |
| Agents after the Final check (laptop on AC and unlocked, you may leave) | about 60 | 0 |
