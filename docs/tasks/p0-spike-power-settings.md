# Prove how to change and protect power settings

Phase: 0. Status: done. Needs Haik: only the optional B5 (restart with a copy active) in the morning list.
Runs after: `p0-toolchain`. Parallel with: `p0-spike-tray`, `p0-distribution-facts`. Not with `p0-spike-lid-sleep`
(same real settings).

## Goal

Replace the guesses behind "User settings safety" in `docs/product.md` with tested facts, using a small Rust program.

## Read first

- `CLAUDE.md`
- `docs/research.md`: "Windows power facts"
- `docs/research/rivals-lidguard.md`: the settings safety sections
- `docs/product.md`: "Power behavior" and "User settings safety"

## Questions to answer

1. Write the lid action and the battery sleep timeout on the active plan as a normal user (`PowerWriteACValueIndex`,
   `PowerWriteDCValueIndex`, `PowerSetActiveScheme`). What does `PowerSettingAccessCheckEx` return here?
2. Detect a change to those values made in Windows Settings or with `powercfg` while our program runs. Compare a
   registry change notification on the power schemes key, `PowerSettingRegisterNotification`, and polling. Latency of
   each. Can we tell our own writes from the user's?
3. Temporary copy of the plan vs editing in place: what happens to Windows 11 Power mode, the Settings page and Lenovo
   Vantage while a copy is active? Does the copy survive a restart? Clean removal?
4. Read without admin: the energy saver threshold (and how Never and Always are stored), "require sign-in after
   sleep", user idle time (`GetLastInputInfo`), active plan change notification.
5. Battery rule: does Windows end a held `PowerRequestSystemRequired` about 5 minutes after the sleep timeout on
   battery? First find Microsoft's own doc text. Then test with a short battery sleep timeout, a held request and no
   input, logging what happens. This decides whether Wakeshot touches the sleep timer at all.

## Deliverables

- `spikes/power-settings/`: a standalone Cargo project (not part of the product), with a README on how to run it.
- Findings in `docs/research.md` with how each was checked.
- An "Implications for the design" list in Result.

## Acceptance checks

- Every question has a tested answer, or "could not test" with the reason.
- Haik's original values restored and verified: `powercfg /q` output for the lid action and sleep timeout, before
  and after, recorded in the Log.

## Out of scope

Product code. Tray. Lid-closed behavior (`p0-spike-lid-sleep`).

## Questions for Haik

None. Nothing here needs a product decision yet.

## Steps for Haik (morning)

Moved to `docs/phases/00-morning-steps.md`, the one list for all three spike cards. Since `p0-morning-trim` the list
holds only C and the optional B5 for this card; A and B moved to agents (below); the optional older keep-awake run
(`spike hold --request stes`, titled "C7" in the old list) is dropped. Haik runs his steps from
`spikes\morning-shell.cmd`, which the orchestrator opens through Explorer, so the tools run with the normal Medium
token.

### Moved to agent (card `p0-morning-trim`, 2026-11-03; safety fixes from `docs/reviews/p0-morning-trim-safety.md`)

Haik keeps only C3 to C7 (charger out, `spike hold`, hands off, `checkpower`; C runs after H and right before I, on
one battery charge) and the optional B5 at the very end.

Before Haik starts (step 0 of the list):

1. The BEFORE line, values read live by `powercfg /q`, plus `DelayLockInterval` and both overlay values.
2. Old C1 and C2: Haik closes media apps once; the orchestrator stops the keep-awake by PID with the guarded line,
   checks `powercfg /requests` and logs it every 30 s until the Final check (step 0). Battery at 70% or more.

After Haik's Final check: A and B below. The AFTER line (old C8) comes only after the end check of the last agent
step, with values read live by `powercfg /q`.

**Rules for A and B**

- One agent on the screen at a time: this card's A and B first, then the tray card's steps. Never during a Haik block.
  The lid card's log reads need no screen and may run alongside. (For the morning agent steps this overrides
  "Parallel with: `p0-spike-tray`" in the header.)
- On AC only. Check first; if the laptop is on battery, wait.
- Spike tools run with the Medium token: a `.cmd` in the scratchpad opened with `explorer.exe`, as `morning-shell.cmd`
  is (`research.md`, "Spike launch and stop checks"). The elevated session may read anything and may write any
  `powercfg` value, CONSOLELOCK too.
- Screen holder: one display request for this step only, in the pattern of the orchestrator's `keep-awake.ps1`
  (`PowerCreateRequest`, then `PowerSetRequest` with `PowerRequestDisplayRequired`). It ends by itself after 45
  minutes, and at once when the laptop goes on battery; it writes its PID to a file; the agent stops it by that PID
  at the end of the step. The end check's `powercfg /requests` must not list it.
- Before every UI Automation action or synthetic input: the session must be unlocked (`LogonUI` not running). If it is
  locked: run the stop rule and wait for Haik.
- UI Automation only on named controls (Invoke, SelectionItem, ExpandCollapse, Toggle read). No clicks at screen
  coordinates.
- Write a Log line in this card when A and B start ("agent B and A started") and when they end ("agent B and A done,
  end check ALL OK").

**Stop rule** (a failed step, a lock, a prompt that the agent cannot answer, or any agent that finds "started" in this
Log without "done"): in a Medium shell run `restoreall`, then `checkpower`. Put `DelayLockInterval` back with
`Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name DelayLockInterval -Value 0 -Type DWord`, and Power mode
back to Best performance (Settings, or `spike set-overlay ded574b5-45a0-4f42-8737-46345c09c238`). Use manual restore
steps 3 and 6 of the list for any plan value still different. Then the end check (step 8). If something cannot be put
back, tell Haik before he unplugs or closes the lid, and record it here.

**Steps, in this order**

1. Before values: `powercfg /q` for every row of the list's table, `DelayLockInterval`
   (`HKCU\Control Panel\Desktop`, 0 today), `ActiveOverlayAcPowerScheme` (`ded574b5-...`) and
   `ActiveOverlayDcPowerScheme` (empty) under `HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes`, and a
   screenshot of Settings > Accounts > Sign-in options showing the current "If you've been away" choice. Check: on AC,
   only Balanced, no journal, `checkpower` ALL OK.
2. B1: `spike copy-test --keep` (about 30 s, ends with `--keep: the copy stays active`). B2: `checkpower` shows
   exactly three red lines (the `Wakeshot spike copy` plan, `active plan is NOT Balanced`, the power journal). Any
   other red line: stop rule. From here, every plan value that Settings writes lands on this throwaway copy, not on
   Haik's Balanced, and `spike restore` removes it. Only two things the tour can touch are real: `DelayLockInterval`
   (per user) and the Power mode overlay (per machine). If the session dies here, Haik's `checkpower` shows the copy
   and `restoreall` removes it.
3. Start `spike watch --seconds 600`. It logs every change and checks at its end.
4. The tour, on the copy. One change at a time: set the test value, wait 5 s, put the value back at once, before the
   next page. A screenshot of each page before and after.

   | Step | Page and control | Test value | Put back |
   | --- | --- | --- | --- |
   | A2 | Settings > System > Power & battery > Screen, sleep & hibernate timeouts > "On battery power, put my device to sleep after" | 15 minutes | 10 minutes |
   | A3 | Same page, Energy saver > "Turn energy saver on automatically" | Never; then Always, if the list has it (put back after each) | 30% |
   | A4 | Settings > System > Power & battery > Lid & power button controls > "Closing the lid", on battery; if the page lacks it: Control Panel > Power Options > "Choose what closing the lid does", On battery, then Save changes | Do nothing | Sleep (Save again in Control Panel) |
   | A5 | Settings > Accounts > Sign-in options > "If you've been away, when should Windows require you to sign in again?" | Never; then one of the minute options (put back after each) | the choice from step 1 |
   | A6 and B3 | Settings > System > Power & battery > Power mode, the plugged-in choice only | First record whether the control is shown and enabled while the copy is active (this answers B3). If enabled: Balanced | Best performance |

   - A5: after each change and each put-back, read `DelayLockInterval` and CONSOLELOCK (`powercfg /qh SCHEME_CURRENT
     SUB_NONE CONSOLELOCK`). If `DelayLockInterval` is not 0 after the put-back, write 0 with the line in the stop
     rule. If a UAC, PIN or Windows Hello prompt shows, do not try to answer it: skip A5, run the stop rule, and give
     A5 to Haik as a short optional step.
   - A6: if the page shows separate "Plugged in" and "On battery" choices, never touch "On battery": its value
     (`ActiveOverlayDcPowerScheme`, empty today) cannot be set back to empty. After each change read both overlay
     values. If `ActiveOverlayDcPowerScheme` changed, stop and tell Haik.
5. Let the watch end by itself. Read its last lines. `every value is back to its start value` means done. For each
   `STILL CHANGED: <name> <old> -> <new>` line:

   | Name in the line | Haik's value | Put back |
   | --- | --- | --- |
   | `active` | the copy GUID during the tour | B6 below |
   | `sleep.ac`, `sleep.dc`, `lid.ac`, `lid.dc`, `display-off.ac`, `display-off.dc`, `energy-saver-threshold.ac`, `energy-saver-threshold.dc`, `consolelock.ac`, `consolelock.dc` | on the copy | B6 deletes the copy; the end check then reads Balanced |
   | `DelayLockInterval` | 0 | the line in the stop rule |
   | `overlay.ac.reg`, `overlay.actual`, `overlay.effective` | `ded574b5-...` (Best performance) | Settings Power mode, or `spike set-overlay ded574b5-45a0-4f42-8737-46345c09c238` |
   | `overlay.dc.reg` | `<absent>` | cannot be set back: tell Haik |

6. B4, with the copy still active: open Lenovo Vantage
   (`explorer.exe shell:AppsFolder\E046963F.LenovoCompanion_k1h2ywk1493x8!App`). Go to Device > Power and its thermal
   or performance page with UI Automation Invoke on named items only. Screenshot every page, and read every toggle's
   state, before and after. Change nothing. If a toggle differs afterwards, record it and tell Haik (Vantage values
   are not in `checkpower`). Close Vantage.
7. B6: `spike restore` (`done, journal removed`).
8. B7, the end check: `checkpower` ALL OK (it reads the active plan, Balanced again); `DelayLockInterval` 0;
   `ActiveOverlayDcPowerScheme` empty; no `power-settings-spike` process; stop the screen holder by PID, then
   `powercfg /requests` lists nothing from an agent. Log "agent B and A done, end check ALL OK".
9. Read the `hold` log (newest `out\hold-*.log`) by the guide below, and the requests log for the minutes of C: any
   DISPLAY, SYSTEM or EXECUTION entry other than the spike's own means C was spoiled and is redone.

Not verified: that UI Automation can drive these Settings pages and Vantage; whether A5 shows a prompt; whether this
build's Power mode page has an "On battery" choice; whether Settings shows Power mode while a copy is active (Microsoft
says the slider shows for Balanced and plans derived from it, `research.md`, "Agent reach"); whether Settings writes
CONSOLELOCK to the active plan only. A step that fails goes through the stop rule and then to Haik as a short
optional step.

Dropped: the optional older keep-awake run (`spike hold --request stes`, old "C7"): no consumer
(`docs/reviews/p0-close-proposals.md` part 3).

B4 was not run before the 12:33:48 lock; the agent ran it at 12:42 to 12:45 after Haik woke the laptop and the
orchestrator said hands off again (Log). Nothing of B4 is left for Haik.

How to read the `hold` log (for the next worker): if the 5 s ticks stop or show a GAP starting about 360 s after the
last input (sleep 60 s + 5 minutes), the documented rule holds on this laptop and Wakeshot must also set the battery
sleep timer. If the ticks run the full 12 minutes with no gap, the rule did not apply. Compare with the Kernel-Power
506 and 507 lines at the end of the log and the `display-state` events.

## Found outside scope

- This Claude Code session runs elevated (High Mandatory Level, Administrators enabled; `whoami /groups`). Facts other
  cards record as "works without admin" may have been tested elevated. It also means `powercfg /requests` would work
  from an agent shell; I did not run it, as told.
- The laptop was on AC at 02:05 (Online, charging, 96%), not on battery as the brief said. Kernel-Power 105 events at
  01:24:33, 01:50:47 and 01:50:52.
- Lenovo Smart Standby service is running. It may change standby or lid behavior; relevant for `p0-spike-lid-sleep`.
  Not checked.
- (step 0 worker, 10:21) The Result's first line still says "blocked on Haik's morning steps A, B and C"; since
  `p0-morning-trim` A and B are agent steps and only C is Haik's. Not edited.
- (agent steps worker, 12:40) The screen holder rule "ends at once when the laptop goes on battery" plus a 5 s
  `PowerLineStatus` check ends the holder on a short AC drop. One came at 12:27:54 to 12:28:00 on the DELL USB-C
  cable with no one touching the laptop, so my holder ended silently and the session locked 6 minutes later at
  screen off. The tray card's agent steps use the same holder pattern and will hit the same risk. For B4 I used a
  form that clears the request at once on battery but sets it again if AC is back within 60 s
  (`ab-screen-holder2.ps1` in the scratchpad). The written rule in this card and the tray card is unchanged; that is
  the orchestrator's call.
- (agent steps worker, 12:40) Settings writes sleep after and lid action on battery to the inactive Balanced plan
  too, not only to the active copy (Log 12:31, 12:32). So during a plan copy, Settings changes can still reach
  Haik's Balanced. The step list's premise "every plan value that Settings writes lands on this throwaway copy" is
  wrong for those two values; every put-back here was verified on Balanced too.

## Log

- 2026-11-03: card written.
- 2026-11-03 (night, Haik away, on battery): worker started. Orchestrator holds ES_SYSTEM_REQUIRED (pwsh PID 16788),
  so idle sleep and request tests move to morning steps.
- 2026-11-03 02:04 BEFORE (`powercfg /getactivescheme`, `/qh SCHEME_CURRENT SUB_BUTTONS LIDACTION`,
  `/q SCHEME_CURRENT SUB_SLEEP STANDBYIDLE`): active Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`, the only plan
  in `powercfg /list`. LIDACTION AC 0x00000001, DC 0x00000001. STANDBYIDLE AC 0x00002a30 (10800), DC 0x00000258 (600).
- 2026-11-03 02:05: the laptop is on AC, not battery: `SystemInformation.PowerStatus` says Online, charging, 96%.
  Kernel-Power 105 (power source change) at 01:24:33, 01:50:47 and 01:50:52. So AC values are the live ones tonight.
  Kernel-Power 506 (entering Modern Standby) at 02:01:26 with no 507 after it, yet shells keep running.
- 2026-11-03 02:10: this Claude Code session runs elevated (`whoami /groups`: High Mandatory Level, Administrators
  enabled). So the spike has a `limited` mode that starts itself with Explorer as parent (Medium integrity, not
  elevated) for every write test. Found the battery rule text in the PowerSetRequest docs.
- 2026-11-03 02:16: spike builds, clippy and tests clean. `read` and `limited read` run (no writes).
- 2026-11-03 02:17: `limited write-test` (Medium integrity) on the real Balanced plan: wrote lid 0/0, sleep
  10807/607, all rc 0, then restored. powercfg inside the run, before: LIDACTION 0x1/0x1, STANDBYIDLE 0x2a30/0x258;
  while written: 0x0/0x0, 0x2a37/0x25f; after restore: 0x1/0x1, 0x2a30/0x258. Log
  `spikes/power-settings/out/write-test-limited-20261103-021636.log`.
- 2026-11-03 02:18: `limited copy-test`: copy `7efbae22-...` made, active for 23 s, change triggers run on it
  (API, `powercfg /setacvalueindex`, `/setactive`, `/change`), switched back, deleted. Overlay and every watched value
  equal before and after. Log `out/copy-test-limited-20261103-021755.log`. The run left lid.ac 0 on the copy until
  the copy was deleted (spike bug, fixed: the write-back now restores lid.ac too).
- 2026-11-03 02:19: check from the shell: Balanced active and the only plan; LIDACTION 0x1/0x1, STANDBYIDLE
  0x2a30/0x258. Same as before.
- 2026-11-03 02:20: `limited hold --allow-ac --sleep-dc 600 --minutes 1` as a dry run on AC (copy with Haik's own
  600 s, so nothing sooner): request held, ticks logged, copy removed, plan back.
- 2026-11-03 02:24: `limited copy-test --keep` then `limited restore`: the journal path works (switch back, delete
  copy, journal removed).
- 2026-11-03 02:24 AFTER (same commands as BEFORE): active Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`, the only
  plan. LIDACTION AC 0x00000001, DC 0x00000001. STANDBYIDLE AC 0x00002a30, DC 0x00000258. Overlay registry
  unchanged (`ActiveOverlayAcPowerScheme` `ded574b5-...`). Matches BEFORE.
- 2026-11-03 02:26: findings in `docs/research.md` ("Power settings spike", and the re-read doc text in "Windows
  power facts"). Status blocked on the morning steps.
- 2026-11-03 03:52: card `p0-spike-safety-fixes` moved the morning steps to `docs/phases/00-morning-steps.md` and
  changed the spike: Ctrl+C and close handler for `copy-test`, `hold` and `watch` (Ctrl+C ends a watch early with its
  check), `copy-test` writes name the copy, refusal while the lid spike's journal exists or a spike copy is active,
  restore falls back to Balanced. Details in that card.
- 2026-11-03 10:18: step 0 worker started (morning list step 0.2, items 1 to 3 only). Reads only from the elevated
  session (High); no setting written, no power request held.
- 2026-11-03 10:18 BEFORE (elevated, reads only: `powercfg /list`, `/getactivescheme`, `/qh SCHEME_CURRENT <sub>
  <setting>` per row, registry reads; script `s0-before.ps1` in the step 0 worker's scratchpad
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`):
  on AC (Online), battery 100%. Balanced `381b4222-f694-41f0-9685-ff5bb260df2e` active and the only plan. LIDACTION AC
  0x1, DC 0x1. STANDBYIDLE AC 0x2a30 (10800), DC 0x258 (600). VIDEOIDLE AC 0x12c (300), DC 0xb4 (180). RTCWAKE AC 0x0,
  DC 0x0. ESBATTTHRESHOLD AC 0x0, DC 0x1e (30). CONSOLELOCK AC 0x1, DC 0x1. `DelayLockInterval` 0.
  `ActiveOverlayAcPowerScheme` `ded574b5-45a0-4f42-8737-46345c09c238`, `ActiveOverlayDcPowerScheme` absent. Both
  journals absent. No `power-settings-spike`, `lid-sleep-spike` or `tray-spike` process. `LogonUI` not running. No
  other agent running (the orchestrator's word; not checked by a read). Matches the list's table.
- 2026-11-03 10:21: battery 100%, on AC (`SystemInformation.PowerStatus`), so over the 70% that C needs. Step 0 items
  1 to 3 done by this worker; items 4 to 8 (keep-awake stop, requests log, morning shell, "go") are the
  orchestrator's.
- 2026-11-03 12:30: agent steps worker started (after Haik's Final check, `checkpower` ALL OK per the orchestrator).
  Reads C's logs first, then step 1 (before values), then B and A.
- 2026-11-03 12:26: C read. `hold-20261103-112124.log`: on battery (98%), copy with sleep.dc 60 s, SYSTEM and
  EXECUTION request set 11:21:24.7 (idle 0). Ticks every 5 s with no gap to t 374 s. Screen dim at 165 s, off at 180 s
  (Kernel-Power 506 "Idle Timeout" 11:24:24, ticks go on). `PBT_APMSUSPEND` at 11:27:44.1, t 379.6 s after the last
  input. Next tick 11:37:10 after the power button (507), wall step 571 s, unbiased step 4.9 s: a real sleep. Plan
  back, copy deleted, LIDACTION 0x1/0x1, STANDBYIDLE 0x2a30/0x258. Requests log (orchestrator scratchpad
  `requests.log`, my scan `ab-reqscan.ps1`): 11:15:29 to 11:20:59 all None; 11:21:29 to 11:27:30 only the spike's
  "battery rule test" (SYSTEM and EXECUTION); no sample 11:27:30 to 11:37:26 (the sampler slept too); 11:37:26 on all
  None. So C is clean and the documented rule holds here.
- 2026-11-03 12:26: screen holder started (my `ab-screen-holder.ps1`, pwsh PID 16176, DISPLAY request "Wakeshot power
  card agent steps: screen holder", 45 min cap, ends on battery). 12:27: Medium runner opened through `explorer.exe`
  (`ab-runner.cmd`, pwsh PID 19688, `whoami`: Medium Mandatory Level). Both in my scratchpad.
- 2026-11-03 12:27 step 1 before (elevated reads, `ab-read.ps1`): only Balanced `381b4222-...`, active. LIDACTION
  0x1/0x1, STANDBYIDLE 0x2a30/0x258, VIDEOIDLE 0x12c/0xb4, RTCWAKE 0x0/0x0, ESBATTTHRESHOLD 0x0/0x1e, CONSOLELOCK
  0x1/0x1. `DelayLockInterval` 0. `ActiveOverlayAcPowerScheme` `ded574b5-45a0-4f42-8737-46345c09c238`,
  `ActiveOverlayDcPowerScheme` absent. No journal. Only `tray-spike` 11944 runs (the tray card's). Unlocked. On AC,
  95%. `checkpower` from the Medium runner: ALL OK.
- 2026-11-03 12:28: Sign-in options screenshot (`ab-shots\122851-signin-before.png`): "If you've been away" shows
  "Every Time", greyed out, with "Windows Hello is preventing some options from being shown." UI Automation: ComboBox
  `SystemSettings_Users_DelayLock_ComboBox`, IsEnabled False. So A5 cannot run; nothing to put back there.
- 2026-11-03 12:30: agent B and A started.
- 2026-11-03 12:29 B1: `spike copy-test --keep` from the Medium runner, exit 0, ends `--keep: the copy stays active`
  (`out\copy-test-20261103-122921.log`). Copy `42ffeeaa-7cfe-4ee3-bfbf-1866da247eef`. B2: `checkpower` shows exactly
  the three expected red lines (copy plan, active plan NOT Balanced, power journal), every value OK.
- 2026-11-03 12:29:58: `spike watch --seconds 600` started from the runner (PID 27664, Medium, `out\watch-20261103-122958.log`).
- 2026-11-03 12:30 B3: Power & battery with the copy active shows Power Mode "Best Performance", with separate
  "Plugged in" (Best Performance) and "On battery" (Balanced) combo boxes, both enabled. Combo items (expanded and
  collapsed, nothing selected): sleep after 1 minute to 5 hours and Never; energy saver Never, 10% to 50%, On Battery;
  lid Do Nothing, Sleep, Hibernate, Shutdown; Power mode Best Power Efficiency, Balanced, Best Performance. The tour
  also used UIA ScrollItem (ScrollIntoView) to bring a control into the screenshot; it changes no setting.
- 2026-11-03 12:31 A2 (sleep on battery, combo `SystemSettings_PowerTimeouts_Sleep_DC2_ComboBox`): 15 minutes at
  12:31:42.9, 5 s later STANDBYIDLE DC 0x384 on the copy AND on Balanced `381b4222` (`powercfg /qh <guid>`). Watch:
  two registry events per change. Put back 10 minutes: both 0x258 again. So Settings wrote Haik's Balanced too; the
  card's premise "every plan value Settings writes lands on the copy" is wrong for this value.
- 2026-11-03 12:32 A3 (energy saver threshold, `SystemSettings_BatterySaver_SettingsPage_ThresholdValue_ComboBox`):
  Never: ESBATTTHRESHOLD DC 0x0 on the copy only, AC stays 0x0. Back 30%: 0x1e. On Battery: DC 0x64 (100) on the copy
  only. Back 30%: 0x1e. Balanced stayed 0x0/0x1e throughout. "Always use energy saver" is a separate toggle (Off), read
  only.
- 2026-11-03 12:32 A4 (lid on battery, `SystemSettings_PowerActions_Lid_DC_ComboBox`, on the Settings page): Do Nothing:
  LIDACTION DC 0x0 on the copy AND on Balanced. Back Sleep: both 0x1.
- 2026-11-03 A5: not run, the control is disabled by Windows Hello (12:28 line). DelayLockInterval read 0 and
  CONSOLELOCK 0x1/0x1 on both plans at every tour read.
- 2026-11-03 12:33 A6 (Power mode, Plugged in only): Balanced: `ActiveOverlayAcPowerScheme` becomes
  `00000000-0000-0000-0000-000000000000`; DC overlay stays absent. Back Best Performance: `ded574b5-...`, DC absent.
  "On battery" not touched.
- 2026-11-03 12:33:48: the session LOCKED (LogonUI started 12:33:48; Kernel-Power 506 "Idle Timeout" at 12:33:48;
  watch: display-state 2 at 12:33:33, 0 at 12:33:48). Cause: the screen holder had ended at 12:27:55 "on battery",
  as built: Kernel-Power 105 (power source change) at 12:27:54 and 12:28:00, so AC dropped for about 6 s (the DELL
  USB-C cable is the only power). I saw it only at 12:33:48. A new holder (PID 28492) started 12:33:59, too late.
  The tour itself (A2 to A6) was complete and every value was back before the lock.
- 2026-11-03 12:34:34 stop rule, from the Medium runner: `restoreall`: lid `no journal: nothing to restore`, power
  `PowerSetActiveScheme(Balanced) rc 0`, `PowerDeleteScheme(42ffeeaa-...) rc 0`, `done, journal removed`.
  `checkpower`: ALL OK. `DelayLockInterval` was 0, written 0 by the stop rule line, reads 0. Power mode overlay
  already Haik's (checkpower OK). B4 (Vantage with the copy active) not run: it needs the screen and the copy.
- 2026-11-03 12:39:59: the watch ended by itself. Only line: `STILL CHANGED: active 42ffeeaa-... -> 381b4222-...`
  (expected: B6 row of the table, done by the 12:34 restore). Every other watched value back to its start value.
  Display on again at 12:39:40 (Kernel-Power 507 "Input Keyboard": Haik woke it); LogonUI gone by 12:40:30.
- 2026-11-03 12:40:27 end check: holder 28492 stopped by PID (16176 already gone). `checkpower` from the Medium
  runner ALL OK; no `power-settings-spike` process. Elevated reads: only Balanced, active; LIDACTION 0x1/0x1,
  STANDBYIDLE 0x2a30/0x258, VIDEOIDLE 0x12c/0xb4, RTCWAKE 0x0/0x0, ESBATTTHRESHOLD 0x0/0x1e, CONSOLELOCK 0x1/0x1;
  `DelayLockInterval` 0; overlay AC `ded574b5-...`, DC absent; no journal; theme 0/0; `powercfg /requests` all None.
- 2026-11-03 12:41: orchestrator's check request (steps between 12:33 and 12:40 against the put-back table). What ran
  then: A6 12:33:08 to 12:33:21, screen on (dim came 12:33:33), values back and read. Holder 28492 at 12:33:59 (no
  effect). Stop rule 12:34:34 (no UI: `restoreall`, `checkpower` ALL OK, `DelayLockInterval` 0) while locked; the
  12:40:27 reads confirm it. Watch end 12:39:59 (no UI). No UI Automation step ran while the screen was off or
  locked, so nothing needs a redo. Not run at all: B4. Haik is hands off again, so I run B4 now with a fixed holder
  (`ab-screen-holder2.ps1`, PID 3680, 12:41:09: on battery it clears the request at once, sets it again if AC is
  back within 60 s, ends after 60 s on battery or 45 min; logs each change). Before each UI step: holder alive,
  unlocked.
- 2026-11-03 12:42: closed the Settings window I opened (named Close button). Opened Lenovo Vantage
  (`explorer.exe shell:AppsFolder\...!App`, LenovoVantage PID 32632). A feedback survey covered the page; dismissed
  with its Close button (not "No, thanks"). Baseline without a copy, Device settings > Power (`ab-shots\124340-B4-x-device.png`):
  Smart standby Off (Auto selected, greyed), Battery charge threshold Off (75%/80% default, Edit greyed), Always on USB
  On, "Enable this feature even when device is shut off" Off. Menu: Power, Display, Sound, Input, Device details; no
  thermal or performance page. UI Automation `FindAll` did not find the web items; a `TreeWalker` walk did.
- 2026-11-03 12:43:53 B1 again: `spike copy-test --keep`, exit 0 (`out\copy-test-20261103-124353.log`), copy
  `fa098892-7987-4344-b8b6-0e06970d014c`. `checkpower`: exactly the three expected red lines.
- 2026-11-03 12:44 B4: Vantage Home, then Device settings > Power with the copy active (`ab-shots\124442-B4-copy-power.png`):
  the whole UI Automation tree of the page is the same text as the baseline, every toggle the same, no warning.
  Nothing invoked except the navigation items. Vantage closed with the Window pattern; its process is gone.
- 2026-11-03 12:44:56 B6: `spike restore`: `PowerSetActiveScheme(Balanced) rc 0`, `PowerDeleteScheme(fa098892-...) rc 0`,
  `done, journal removed`. `checkpower` ALL OK; no `power-settings-spike` process.
- 2026-11-03 12:45 B7 end check: holder 3680 stopped by PID at 12:45:27 (my end check script first missed it: it
  matched `ab-screen-holder.ps1`, not `ab-screen-holder2.ps1`; caught by `powercfg /requests` and fixed). Then
  `powercfg /requests` all None. Runner 19688 ended by its stop file. Watch 27664 gone. Only `tray-spike` 11944 (the
  tray card's) runs. Settings and Vantage windows closed. Morning shell left open for the orchestrator.
- 2026-11-03 12:45:16 AFTER (elevated, live `powercfg /list`, `/getactivescheme`, `/qh SCHEME_CURRENT <sub> <setting>`,
  registry): only Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`, active. LIDACTION AC 0x00000001, DC 0x00000001.
  STANDBYIDLE AC 0x00002a30, DC 0x00000258. VIDEOIDLE 0x12c/0xb4, RTCWAKE 0x0/0x0, ESBATTTHRESHOLD 0x0/0x1e,
  CONSOLELOCK 0x1/0x1. `DelayLockInterval` 0. `ActiveOverlayAcPowerScheme` `ded574b5-45a0-4f42-8737-46345c09c238`,
  `ActiveOverlayDcPowerScheme` absent. No journal. Unlocked, on AC, 95%. Theme 0/0. Matches BEFORE (10:18, 12:27).
- 2026-11-03 12:45: agent B and A done, end check ALL OK.

## Result

Status: done. Open: only the optional B5 (restart with a copy active) in the morning list.

What was done: `spikes/power-settings/`, a Rust spike with modes `read`, `write-test`, `copy-test`, `watch`, `hold`,
`restore`, `set-overlay` and `limited`. Night (02:04 to 02:26): API and `powercfg` tests with a Medium token, on AC.
Step 0 (10:18 to 10:21): BEFORE line from live reads. Haik's block C (11:21 to 11:37, battery). Agent steps B and A
(12:26 to 12:45, on AC): Settings tour on a plan copy with UI Automation and a `watch`, Lenovo Vantage with a copy,
end check. The session locked once at 12:33:48 (a short AC drop ended the screen holder, Log); the tour was complete
by then, the stop rule ran, and B4 ran after Haik woke the laptop.

Answers:

1. Writes as a normal user: yes. Lid and sleep, AC and DC, on the active Balanced plan: rc 0, applied by
   `PowerSetActiveScheme`. `PowerSettingAccessCheckEx` returns 0 for these and 5 for a CONSOLELOCK write.
2. Detection: registry notification 0 to 62 ms, sees every stored change, from API, `powercfg` and Windows Settings
   alike; setting notification 4 to 34 ms but only for live values after SetActive; poll as its interval. Our writes
   and the user's look the same.
3. Plan copy: create, activate, switch back and delete work as a normal user; delete of the active copy fails with 50;
   removal is clean. With a copy of Balanced active, Settings shows and enables Power mode (Plugged in and On
   battery), and Vantage's Power page is unchanged. But Settings writes sleep after and lid action on battery to the
   inactive Balanced too, not only to the copy; the energy saver threshold goes to the copy only. Restart with a copy
   active: not tested (optional B5).
4. Reads without admin: energy saver threshold (Settings stores Never as 0, "On Battery" as 100), CONSOLELOCK,
   `DelayLockInterval`, idle time, active plan change notification. Which value the sign-in choice changes: could not
   test, Windows Hello greys the choice out ("Every Time").
5. Battery rule: holds here. On battery with sleep after 60 s and SYSTEM plus EXECUTION requests held, the laptop
   slept 380 s after the last input (rule: 360 s). C was clean: no other request in the requests log.

Implications for the design:

- On battery a power request alone is not enough: Windows sleeps about 5 minutes after the battery sleep timeout. So
  Wakeshot must raise the battery sleep timer (or set Never) while it holds on battery, and put it back after.
- No admin needed for Wakeshot's power work: writes on the active plan, plan copies and all reads work unelevated.
- Always call `PowerSetActiveScheme` after a write. A written value is stored at once but not live before that.
- Watch with the registry notification on the `PowerSchemes` key, then read again with `PowerRead*ValueIndex`. Add
  the setting notification for the active plan GUID and the power source, and a slow poll as a safety net. Change
  it when: the registry path stops working on a new Windows build (it is not a documented API).
- Never use a notification payload as the value: the lid payload is a `POWER_ACTION` (Sleep came as 2).
- To tell our writes from the user's, compare with the value we wrote; nothing else exists. A user who sets exactly
  our value cannot be told apart.
- Plan copy vs in place: a copy does not keep Settings off the user's plan (sleep and lid on battery land on Balanced
  too) and splits user edits across two plans (energy saver stays on the copy), and it needs its own crash guard. My
  pick: edit in place, with a journal and restore only where the value still equals ours. Change it when: a case shows
  that in-place restore cannot tell a user edit from ours. Not verified: the same Settings behavior with a custom
  (non-Balanced) user plan.
- Read the energy saver threshold as 0 = Never, 100 = always on battery.
- Delete a copy only after switching away from it.
- Treat any non-zero `PowerSettingAccessCheckEx` as "cannot write": run with requests only and tell the user.
- Wakeshot never needs to write CONSOLELOCK, and cannot as a normal user.
- On this laptop screen off starts a Modern Standby session even with a SYSTEM_REQUIRED request, and work goes on.
  With sign-in "Every Time" (Windows Hello) the session also locks then. So Wakeshot can let the screen turn off.
- Agents that test "works without admin" from this elevated session must de-elevate, or the result is wrong.

Files: `spikes/power-settings/` (`Cargo.toml`, `Cargo.lock`, `.gitignore`, `README.md`, `src/main.rs`, `log.rs`,
`power.rs`, `sys.rs`, `watch.rs`, `journal.rs`, logs in `out/`, today also `copy-test-20261103-122921.log`,
`watch-20261103-122958.log`, `copy-test-20261103-124353.log`, two `restore-*` logs), `docs/research.md` ("Windows
power facts", "Power settings spike", "Agent reach on this laptop"), this card. Agent step scripts and screenshots
are in the scratchpad
`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`
(`ab-*.ps1`, `ab-shots\`).

Checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` clean; `cargo test` 9 passed (night run; no
code changed since). Acceptance "every question has a tested answer, or could not test with the reason": yes; the
open parts are the restart (optional B5) and the sign-in mapping (greyed out by Windows Hello). Acceptance "Haik's
values restored and verified": BEFORE 02:04, 10:18 and 12:27; AFTER 02:24 and 12:45:16 (live `powercfg` reads,
Log): all equal. End check 12:45: `checkpower` ALL OK, `DelayLockInterval` 0, `ActiveOverlayDcPowerScheme` absent,
theme 0/0, `powercfg /requests` all None, no spike process of this card, no holder.
