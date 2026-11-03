# Review: safety of the trimmed morning list

Card: `docs/tasks/p0-morning-trim.md`. List: `docs/phases/00-morning-steps.md`. "Moved to agent" lists in
`docs/tasks/p0-spike-power-settings.md`, `p0-spike-lid-sleep.md`, `p0-spike-tray.md`. Earlier reviews:
`docs/reviews/p0-spike-safety.md`, `docs/reviews/p0-spike-safety-verify.md`. Reviewer: wakeshot-reviewer, 2026-11-03
09:50 to 10:05.

How checked: read the card, the list, the three cards, both earlier reviews, `spikes/morning-shell.ps1` and `.cmd`, the
keep-awake script, and the spike code where a claim depends on it. Reads only: `powercfg /list`, `/qh`, `/requests`,
registry, WMI battery data, process command lines, file times, and two read-only Core Audio scripts in my scratchpad
(`rv-audio-default.ps1`, `rv-audio-volumes.ps1`). No spike tool run, no setting changed, no process stopped.

## Haik's values now (brief item 6)

Read 09:50 (and requests again 10:03). They match `CLAUDE.md` "Test laptop" and the list's table.

| Setting | AC | Battery |
| --- | --- | --- |
| Plans | Balanced `381b4222-f694-41f0-9685-ff5bb260df2e` only, active | same |
| LIDACTION | 1 (Sleep) | 1 (Sleep) |
| STANDBYIDLE | 10800 (3 h) | 600 (10 min) |
| VIDEOIDLE | 300 | 180 |
| RTCWAKE | 0 | 0 |
| ESBATTTHRESHOLD | 0 | 30 |
| CONSOLELOCK | 1 | 1 |
| Overlay (registry) | `ded574b5-45a0-4f42-8737-46345c09c238` | empty |
| Low battery level / action | 6% / Do nothing | 6% / Do nothing |
| Reserve, critical level / action | 4%, 2% / Hibernate | 4%, 2% / Hibernate |
| `DelayLockInterval` (HKCU Desktop) | 0 | |

Other state: no journal in either `out\`; no spike process; keep-awake PID 11284 is `pwsh ... keep-awake.ps1`, started
02:26:43. On AC, battery 99%, full charge capacity 80.3 Wh. Theme Dark (`AppsUseLightTheme` 0,
`SystemUsesLightTheme` 0, `CurrentTheme` `Custom.theme`). `powercfg /requests` at 09:50 and 10:03: SYSTEM = the
keep-awake; **DISPLAY = Chrome "Video Wake Lock"; EXECUTION = Chrome "Playing audio"**.

## Fix before step 0

All are doc lines in the list and the three cards. No code change.

1. After the agents' steps: AFTER lines and `checkpower` at the very end, and one line for Haik to check if no "done"
   comes (finding 1).
2. Write the A2 to A6 changes into the power card; the card points to a list that no longer exists (finding 2).
3. One agent on the screen at a time after the Final check (finding 3).
4. A time cap on the agents' display request (finding 4).
5. A media line for Haik before D and C, and a way back in after a break (finding 5).

## Findings

### 1. Major: agents change Haik's real plan after he is "done", and nobody checks the end state

- Where: list 260 ("You are done with the required steps"), 264-267 (orchestrator writes the AFTER lines, **then**
  runs the agent steps); power card 76-82 (C8 AFTER line is item 3, the Settings tour is item 4).
- Fact: A2 to A6 change Haik's real Balanced plan through Windows Settings (battery sleep, energy saver, lid on
  battery, sign-in, Power mode). In the old list Haik did this himself, and "Stopping early" covered him if he stopped.
  Now an agent does it unattended, and "Stopping early" is a rule for Haik only.
- What breaks: the agent session ends (session limit, crash, a UI Automation step that hangs) between A4 and its
  put-back. Haik's real plan keeps lid action Do nothing on battery (and maybe a changed battery sleep). The AFTER lines
  already say "matches BEFORE", because they were written before the tour. Haik was told he is done. Next time he
  unplugs and closes the lid into a bag, the laptop keeps running. This is the scenario of the earlier major finding 1
  in `p0-spike-safety-verify.md`, with the human safety net removed.
- Same gap for B: if the session ends between B1 (`copy-test --keep`) and B6, the copy stays active with the power
  journal. Values equal Balanced, but every Settings change Haik makes later lands on the copy, and both spikes refuse
  to start.
- Fix:
  - Power card A: put each value back right after its 5 s wait, before the next page (in Settings, or with the
    elevated `powercfg` lines of manual restore steps 3 and 6), not at the end of the watch.
  - Move the AFTER lines to after the last agent step (and after the volume is put back). Write values read live with
    `powercfg /q`, not copied from the table (step 0.1 also says "the values in the table above").
  - Final check, new last Haik line: "Wait for the orchestrator's message 'agent steps done, checkpower ALL OK'. If it
    does not come within an hour, double-click `morning-shell.cmd`, run `checkpower` (manual restore if not ALL OK)
    before you unplug or close the lid."

### 2. Major: the power card's Settings tour points to a list that was overwritten

- Where: `p0-spike-power-settings.md:78-79`: "Make A2 to A6 ... same changes and 5 s waits as the old list". The old
  list was rewritten at 09:47; there is no git repo and no copy (searched `docs\` and `spikes\`). The tag table rows
  (card 138-143) name the steps only.
- What breaks: the agent must guess which values to set on Haik's real plan, and the old put-back table (earlier
  review, finding 8: key, Settings place, Haik's value, the AC energy saver line) is gone with it. Two changes Settings
  may not undo, and `checkpower` does not see:
  - `DelayLockInterval` (0 now): A5 may change or remove it. `checkpower` does not read it
    (`morning-shell.ps1:58-84`); only `spike watch` does.
  - The battery Power mode (`ActiveOverlayDcPowerScheme`, empty now). Unverified: whether this build's Power mode page
    has separate "Plugged in" and "On battery" choices. If it does, a UI Automation pick on the battery control writes
    a value that cannot be set back to empty, and `checkpower` checks the AC overlay only (line 78).
- Fix: write A2 to A6 into the power card: page and control, test value, wait, Haik's value, put-back line. Do A only
  on AC (check first). Power mode: touch only the plugged-in control; read `ActiveOverlayDcPowerScheme` and
  `DelayLockInterval` before and after; if either changed, stop and give it to Haik.

### 3. Major: power and tray agent steps may run at the same time on one screen

- Where: power card header line 5 ("Parallel with: `p0-spike-tray`"), tray card line 5 ("Parallel with: the other
  spike cards"); `.claude/skills/orchestrate/SKILL.md:18-20` (parallel where the cards allow); `docs/plan.md:90`
  ("resume the three spike cards").
- What breaks: both cards drive the same desktop with UI Automation, synthetic mouse and keys (power: Settings,
  Control Panel, Vantage; tray: tray menu, Win+N, Settings Colors). A click or key from one lands in the other's
  window, for example a tray-menu click on an open Settings Power & battery dropdown. That is an unplanned change to a
  real setting, and only the values in `checkpower` would show it. The tray card's "No input for 10 s" (line 83) also
  breaks, and each agent's screenshots catch the other's windows.
- Fix: one line in both "Moved to agent" lists and in Final check 3: "One agent on the screen at a time: power A and
  B, then the tray steps. The lid card's log reads need no screen and may run in parallel."

### 4. Major: the agents' display request has no time cap

- Where: power card 73-74, tray card 58-59 ("Hold a display request from the agent's own process ... stop it by PID
  at the end").
- Fact: a hold that spans 40 minutes must be a background process, and such a process outlives the agent session:
  PID 11284 was started in session `1828f2de-...` and still runs now in another session. The keep-awake has a time
  limit and a battery floor for exactly this reason (`keep-awake.ps1:1-5`).
- What breaks: the session ends before "stop it by PID". The screen stays on and the laptop stays awake and unlocked
  with no end, while Haik is away. If finding 1 also happened (lid on battery left at Do nothing), a later unplug and
  close into a bag keeps running with nothing to end it (unverified how a display request acts with the lid closed).
- Fix: one shared holder, started once, that ends by itself after 60 minutes and at 20% on battery, writes its PID to a
  file, and is stopped by PID at the end. The end check includes `powercfg /requests` with nothing from it.

### 5. Major: power requests are checked once, at step 0, about 35 minutes before C and with no recheck after a break

- Where: list 130-138 (step 0.4), 222-225 (C1 and C2 gone), 168-169 (D1 gone), 9-17 ("Stopping early" has no way back
  in).
- Fact: Chrome holds DISPLAY ("Video Wake Lock") and EXECUTION ("Playing audio") now (09:50 and 10:03). Step 0.4 would
  ask Haik to close it, but he works at the laptop for T10, D, E, F and H (about 35 minutes) before C, and the old C1
  "close media apps" ran right before C. If he starts music or a video again:
  - C, the main battery rule test, proves nothing (the PC stays awake for Chrome, not for the spike).
  - In E and I the sound at lid close may be masked, and after work ends a DISPLAY request can keep the closed laptop
    from idle sleep (Haik is near, so bounded, but the E result is wrong).
- Same for a break: after "Stopping early" the orchestrator may start a build, an agent or a keep-awake (rule 0.6 runs
  only "until the Final check"). When Haik goes on, nothing repeats step 0.4.
- Fix:
  - One Haik line at the top of D and before C3: "Pause or close anything that plays sound or video."
  - "Stopping early" step 5: "To go on later, tell the orchestrator and wait for its go (it repeats step 0.4)."
  - Optional, cheap: from step 0.4 to the Final check the orchestrator logs `powercfg /requests` every 30 s to a
    scratchpad file (read only, no screen, no request), so a spoiled run shows afterwards.

### 6. Minor: the volume step changes the DELL, not the laptop speakers

- Where: list 128-129; lid card 73-74; card Log 09:45 ("24%").
- Fact: the default playback device is "4 - DELL U2723QE" (AMD HD Audio, DisplayPort), at 24%, not muted. The laptop
  "Speakers" (Realtek) are at 67%, not muted. Step 0.3 reads and writes "the default device".
- What breaks: D's lid-close sound plays on the DELL (connected until D6). Unverified whether anything is on the
  DELL's audio out; if not, Haik's "no" for D is about the DELL. E and I play on the laptop speakers at 67%, which is
  fine. After the Final check, "put the volume back" on the default device sets whichever device is default then;
  if the DELL is not default yet, the speakers go 67% to 24% and the DELL stays at 50%. If the agent session ends, the
  DELL stays at 50%.
- Fix: read and write by endpoint ID; note both values (DELL 24%, Speakers 67%); in J2 ask for D's sound "from the
  DELL or the laptop". The lid spike plays with `SND_SYSTEM`, so the System Sounds level in the volume mixer also
  counts (not checked).

### 7. Minor: the heat lines say "stop and tell the orchestrator" without saying what stop means

- Where: D8 (list 175-176), E6 (190), I4 (243, no "stop" at all).
- What breaks: "stop here" can be read as "skip D9 `checkpower`". The run has ended at DONE, so the plan should be
  back, but no check runs. And the orchestrator may not be there; nothing says what to do while waiting.
- Fix: "Too hot: keep the lid open, run `checkpower` (Stopping early), let it cool on the desk, then tell the
  orchestrator."

### 8. Minor: block I does not check that the charger is still out

- Where: list 237-240 (I1 gone, "Still on battery from C").
- What breaks: Haik plugs in after C by habit; I runs on AC and the battery result (GAP about 900 s after the last key)
  is wrong. The lid tool does not refuse on AC (it logs `on AC` only, `lid-sleep/src/main.rs:89-96`).
- Fix: I2: "Check that the battery icon is not charging."

### 9. Minor: B5 uses the morning shell after the orchestrator closed it

- Where: list 267 (orchestrator closes the morning shell), 274 (B5 step 1 "In the morning shell"); also line 30-31
  ("You never wait for one, except its go") against B5's wait for "agent steps are done".
- Fix: B5 step 0: "Double-click `morning-shell.cmd` first." Reword line 30-31 to name both waits.

### 10. Minor: step 0 does not check that no worker runs

- Where: list 121-124 (0.1 checks spike processes only), 144-145 (0.6).
- Fact: a worker was writing `spikes\claude-watch\fixtures\` at 09:54 to 09:56 (`p0-close-apply` is in progress).
- Fix: 0.1: "No worker or reviewer agent is running." A doc worker adds little heat, but rule 0.6 should be checked,
  not assumed.

### 11. Minor: agent screen steps have no lock check

- Where: power card 71-74, tray card 58-59; list 262-263 says "If you lock it, they wait", but neither card says so.
- What breaks: if Haik comes back and locks the PC, synthetic keys (Esc, typing, Win+N) go to the lock screen PIN box.
  A UI Automation put-back fails half way.
- Fix: before each synthetic input, check the session is unlocked (`LogonUI` not running, as `probe.ps1` did); if
  locked, put back any changed value with `powercfg` and wait.

### 12. Minor: small real changes the agents do not put back

- T2 sets `IsPromoted` = 1 on the spike's own `NotifyIconSettings` key and never removes it. Harmless, but say so.
- T11 (optional) high contrast on and off with `SystemParametersInfo`: unverified that off restores `Custom.theme`.
  Record `CurrentTheme` first and compare, or drop T11.
- B4 in Vantage: "change nothing" depends on clicks landing right; its Power page has real toggles (charge threshold
  and others) that `checkpower` does not see. Use UI Automation Invoke on named items only, no coordinate clicks, and
  screenshot the toggles before and after.

### 13. Minor: naming drift

- "C7" is both kept (`checkpower`, list 296) and dropped ("the older keep-awake call", list 311; card tag table 158
  and 216). A log reader will mix them up. Rename the dropped one (it was optional).
- Lid card 64: "Haik keeps D4 to D8"; D9 (`checkpower`) is also his.

## The brief, item by item

1. **`checkpower` per block and "Stopping early".** Every power block ends with it: D9, E7, F4, H5, C7, I6, then the
   Final check; T10 changes no power value. "Stopping early" works from each block in the new order: Ctrl+C restores
   in `lid`, `suspend`, `resume`, `hold`; `restoreall` and `checkpower` run from the morning shell; `restore` in both
   tools does not refuse an elevated token (power `main.rs:144`, lid `main.rs:100`), so it also works if the shell had
   to be reopened elevated. Gap: no way back in after a stop (finding 5).
2. **Dropped or moved steps that a later step needs.** D2: covered (step 0.1 checks plans and journals; T10 changes
   nothing; `PlanCopy::make` refuses either journal or an active spike copy, `lid-sleep/src/plan.rs:104-119`). J1/K1:
   covered by I6 and the Final check. I1: merged, but unchecked (finding 8). C1 and C2: moved 35 minutes earlier with no
   recheck (finding 5). C8 and K3: run before the agents change things (finding 1).
3. **C and I on one charge.** 70% of 80.3 Wh is about 56 Wh. C, I and the steps between are about 45 to 50 minutes.
   At 15 W that is about 12 Wh (15%), at 30 W about 25 Wh (31%): I ends at about 40 to 55% (watts are my estimate, not
   measured). Energy saver (30%), low (6%, Do nothing), reserve (4%) and critical (2%, Hibernate) are not reached. No
   tool in C or I has a battery floor (`hold` and `lid` only log the percentage). If critical were reached with the lid
   closed, Windows hibernates (the copies are duplicates of Balanced, so they carry the same battery actions; not read
   on a live copy); the lid copy stays active until the tool resumes or `restoreall` runs, and list line 60-62 covers
   that. Enough margin.
4. **Agent reach.** Everything listed is doable from the elevated session: stop PID 11284 (High, same user), Core
   Audio volume, HKCU writes (same hive), `powercfg` for every value including CONSOLELOCK, UI Automation and
   `SendInput` into Medium windows (higher to lower is allowed), Medium tool launches through `explorer.exe` (factory
   instance from `svchost`, `morning-shell.log` 04:26 and 04:34). Not tried by anyone: UI Automation in Settings and
   Vantage, synthetic tray input, the theme switch, a volume write; the cards say a failed step goes back to Haik.
   Problems: findings 1 to 4, 6, 11, 12. No tool conflict: A ends with `restoreall` before B1; tray and lid agent steps
   make no plan copy.
5. **"Tell the orchestrator" and wait.** Step 0.4 (close a named app) is real and clear: Haik waits for "go". Manual
   restore step 7 is real. J2 and "Stopping early" 5 do not wait. The heat lines are unclear (finding 7). B5's wait is
   real but contradicts line 30-31 (finding 9). Missing: a wait after a break (finding 5) and a "done" message after the
   agents (finding 1).
6. **Values.** Table above; all match.

## Checked and fine

- T10 cannot end the morning shell: its chain is `pwsh <- cmd <- explorer.exe (factory) <- svchost`
  (`morning-shell.log` 04:26, 04:34), those factory instances have exited, and only the shell Explorer 11580 runs now.
  Not tested with a real restart.
- Task Manager's manifest is `asInvoker` (autoElevate set), so Ctrl+Shift+Esc likely starts it at Medium, and the
  restarted Explorer would be Medium. Not checked on a running instance; the shell's token line would catch it.
- Step 0.4's guarded stop line: PID 11284's command line contains `keep-awake.ps1` now, so it matches.
- Spike sources and exes unchanged since the last reviews (exes 03:38 to 03:54; the only files changed under
  `spikes\` since 04:40 are the new `claude-watch\fixtures\`, outside the three spikes).
- No dynamic lock, no screen saver, no inactivity lock policy, so the 40 minutes unlocked will not lock by itself.
- The plan copies still inherit Balanced's battery actions, and the closed-lid runs keep their tool limits (D 8, E 8,
  I 30 minutes) and the hand heat check.

Haik's own blocks are safe as written; findings 1 to 4 sit in the agent steps after his Final check, and finding 5 in
his blocks' test results. All fixes are short doc edits.

Verdict: Not safe to start.

## Verify round

Reviewer: wakeshot-reviewer, 2026-11-03 10:14 to 10:20. Checked the files as they are now: the list (written 10:11),
the three "Moved to agent" lists (10:11 to 10:12), the card's Round 2 table (10:13), `research.md` (10:13). Same rules:
reads only, no spike run, no setting change, no process stop.

State at 10:16: Balanced the only plan and active; LIDACTION 1/1, STANDBYIDLE 10800/600, RTCWAKE 0/0, VIDEOIDLE
300/180, ESBATTTHRESHOLD 0/30, CONSOLELOCK 1/1; overlay AC `ded574b5-...`, DC empty; `DelayLockInterval` 0; no
journal; keep-awake PID 11284 still runs. `powercfg /requests`: only the keep-awake (Chrome's two requests are gone).
No file under `spikes\` changed since 04:40 except the new `claude-watch\fixtures\` (not one of the three spikes).

| Finding | Status | Check |
| --- | --- | --- |
| 1. Agents change the real plan, no end check | Fixed, with one wrong claim (new finding N1) | B1 runs first, so the tour runs on the `copy-test --keep` copy (power card steps 2 to 4). Each value goes back at once. Stop rule, also for a new session that finds "started" without "done" (power card 96-101). AFTER lines come last, values read live (list 298-301, power card 73-74, lid card 88-89). Haik waits for "agent steps done" before he unplugs or closes the lid; fallback after 90 minutes (list 281-292). `watch` is not a write mode, so it runs with the copy active (`main.rs:144`), and it reads `DelayLockInterval` and both overlays (`watch.rs:83-104`). |
| 2. Tour content lost | Fixed | Page, control, test value and put-back for A2 to A6 (power card 120-126); `STILL CHANGED` table (138-144); battery Power mode never touched, both overlays and `DelayLockInterval` read before and after. |
| 3. Two agents on one screen | Fixed | List 296-297; power card 78-80; tray card 59-62 (T3 only after the power card's end check). Note: the card headers still say "Parallel with", and the orchestrator skill reads only header lines when it picks cards (`SKILL.md:16-20`). The body rule and the list line cover it; changing the two header lines would make it hold for a fresh orchestrator too. |
| 4. Display request without a cap | Fixed | Power card 85-88, tray card 65-68: 45 minute cap, ends on battery, PID file, end check lists nothing from an agent. |
| 5. Requests checked once | Fixed (small gap, N3) | Haik closes media apps in step 0 (list 123-124), reminders in D (176) and C (240-241), 30 s requests log (149-153), "go" again after a break with checks 1 and 4 repeated (19-20, 157-159). |
| 6. Volume on the DELL | Fixed | By endpoint ID; Speakers 67% so nothing is written (list 134-137); J2 asks where D's sound came from (274). |
| 7. "Stop" on heat unclear | Fixed | One "Too hot" line (list 182-183), used by D8, E6, I4. |
| 8. I on AC unnoticed | Fixed | I1 (list 258-259). |
| 9. B5 after the shell is closed | Fixed | B5 step 0 (list 308-309); the three waits named (33-34). |
| 10. Workers not checked | Fixed | Step 0 check 1 (list 128). |
| 11. No lock check | Fixed | Power card 89-90, tray card 69-70. |
| 12. Small leftovers | Fixed | T12 deletes `IsPromoted` (tray card 103-104); T11 dropped; Vantage by named controls with toggles read before and after (power card 146-150). |
| 13. Naming drift | Fixed | "C7" is now only `checkpower` (list 333, 349); lid card says D4 to D9. |

### New findings

**N1. Minor: the Control Panel path of A4 writes Haik's real Balanced, not only the copy.** Power card 112-114 says
every plan value Settings writes lands on the copy, and only `DelayLockInterval` and the AC overlay are real. A4's
fallback is Control Panel > "Choose what closing the lid does". That page says "The changes you make to the settings
on this page apply to all of your power plans" (text read from `C:\Windows\System32\en-US\powercpl.dll.mui`). So
lid on battery Do nothing lands on Balanced too, for the 5 s until the put-back. Not checked: whether the Settings lid
page and the sign-in option (CONSOLELOCK) also write all plans. The safety net still holds: the stop rule, the end
check and Haik's fallback all run `checkpower` on Balanced after the copy is gone, and manual restore step 3 sets it.
Fix: correct the claim in the power card step 2 and the Round 2 row; use the Settings page for A4, and if Control
Panel is needed, treat it as a write to the real plan.

**N2. Minor: Haik's 90 minute fallback can meet a slow agent that is still in the tour.** If the agents run late,
Haik's `restoreall` deletes the copy and makes Balanced active while the agent still drives Settings. The agent's next
test value then lands on Balanced. Its own put-back and end check would fix it if it gets there. Fix: before each test
value, the agent checks that the active plan is still the copy; if not, stop rule. Optional: Haik tells the
orchestrator before the fallback.

**N3. Minor: the requests log is not restarted after a break.** It ends after 3 hours (list 150) and step 0.7 repeats
only checks 1 and 4 before a new "go". After a long break, the blocks after it have no log. Fix: "repeat checks 1, 4
and 5".

**N4. Minor: the card's Log times are wrong.** `p0-morning-trim.md` says "10:40: round 2 done" and "about 10:10 to
10:40", but the files were written by 10:13:41 and this check ran at 10:16. Fix the two times.

### Did Haik's part grow?

Hands-on: plus about 1 minute (close media apps in step 0) and a few seconds in I1; the table still says about 80
minutes, about 30 hands-on. "Too hot" and the fallback after 90 minutes run only if needed. New limit: after the
Final check he must leave the laptop on AC, open and unlocked for about an hour (was 40 minutes), but he may walk away.
That is acceptable.

### The open question: `DelayLockInterval` and the battery Power mode in `checkpower`

The default (no tool change; agents read both) is not unsafe. Only the agent's A5 can change `DelayLockInterval`;
the agent reads it after each A5 change and in the end check, and Haik's fallback writes 0 without reading it (0 is
his value, read 09:50 and 10:16). Nothing touches the battery Power mode, which is now on named controls only. If it
did change, it cannot be set back to empty, so a `checkpower` row would stay red forever. Changing
`morning-shell.ps1` now would also mean a new self-test of the one shell that every safety path uses. Change it when:
an agent reports that either value changed, or Phase 3 builds the real restore check.

Verdict: Safe to start.
