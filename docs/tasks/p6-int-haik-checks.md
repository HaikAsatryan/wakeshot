# Integration: Haik's checklist session on the installed Wakeshot

Phase: 6. Status: done (2026-11-05 02:01: Parts A, B and C; findings for fix cards in "Found outside scope"). Needs Haik: **yes**, about 70
minutes, about 40 of hands (planned 42; the changes of 2026-11-04 added the sleep waits and the sign-out block), with
his USB-C dock and external screen in blocks 4 and 6 (always plugged in, Haik 2026-11-03). Agent: `wakeshot-worker`.
Size: about 70K tokens.
Real laptop: **whole laptop**: an install, lid closes, locks, one hibernate, Switch user, battery; no other agent and
no cargo command runs from Part A step 2 to the end of Part C. Haik's own plan is never written: the session runs on
an active plan copy.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-haik-checks\target`.
Runs after: `p6-int-update-e2e`, `p3-haik-checks`, `p5-haik-check`, `p2-6-sleep-route-fixes`. Parallel with: nothing.

## Goal

The checklist items that need Haik's hands, eyes and ears, on the real installed product: lid close with the chime and
the lock (16), the closed-lid report (10), console left and back (11), hibernate after work ends with the lid closed
(7), the battery floor (8, the part without a dock), the real exe's look (13, the real-exe part), and with a dock the
rest of 8 and 7.

## Read first

- `docs/phases/06-integration.md`: the checklist map.
- `docs/architecture.md` section 3, "Episode flow"; section 9, the checklist and the plan copy rule.
- `CLAUDE.md`, "Test laptop"; the Results of `p3-haik-checks` (how its steps were worded and what it found) and
  `p6-int-update-e2e` (install and uninstall commands).

## Deliverables

### Part A: agent, before Haik (no Haik)

1. BEFORE, as `p6-int-update-e2e` step 1 (Run, RunOnce, Path, Haik's plan and values, `powercfg /l`, the data folder).
2. Stop point for every other agent (the orchestrator confirms none runs). Build release x64 of the current tree; pack
   with `pack.ps1`; silent install. Do not start the tray: Haik starts it from Start.
3. Plan copy `Wakeshot int haik` of Haik's active plan, made active.
4. `powercfg /a`: record whether Hibernate is available. If not, block 4 checks the `HibernateFailed` notice instead,
   and no setting is changed to enable it.
5. Write `floor-on.cmd` and `floor-off.cmd` in the scratchpad: `floor-on` reads the battery charge, sets the copy's
   energy saver DC threshold to 99 and re-applies the copy, and says "battery too full, do block 5 last" when the
   charge is 99 or more; `floor-off` puts the copy's original threshold back. For block 6 also `dock-on.cmd` and
   `dock-off.cmd`: the copy's AC lid action to Do nothing and back (as if it were Haik's own value).
6. Set Status to `blocked`, with the steps below and the four script paths filled in; stop. The orchestrator relays.

#### What Part A left in place (2026-11-04 23:20)

Safe to sit for hours: nothing runs, and the active plan has Haik's own values.

- Installed, not started: `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot` (0.1.0, release x64 of the tree at
  23:01), Start menu shortcut `%AppData%\Microsoft\Windows\Start Menu\Programs\Wakeshot.lnk`, uninstall key
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\HaikAsatryan.Wakeshot`.
- Run entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Wakeshot` =
  `"C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe"`, so a sign-out or restart before
  the session starts the tray at sign-in. No RunOnce value, no `StartupApproved\Run\Wakeshot` value.
- User Path entry `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current` (the `wakeshot` command).
- Data folder `%LocalAppData%\Wakeshot` with only `logs\wakeshot.log` (the install hook's lines); no `config.json`, so
  the tray's first start is a first run.
- Active plan: the copy `d740a909-1f61-423a-8e53-3920307b091b` "Wakeshot int haik", every setting equal to Balanced's.
  Balanced (`381b4222-f694-41f0-9685-ff5bb260df2e`) not written, `/qh` hash as at BEFORE.
- The laptop has been on battery since 23:07:46 (98% at 23:15); no command of this card changes the power source.
- Scripts, in the folder
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\9e63f57e-1449-4ae9-b41f-f046e0b19a2e\scratchpad\p6-int-haik-checks`:
  `floor-on.cmd`, `floor-off.cmd`, `dock-on.cmd`, `dock-off.cmd`, the read-only `values.cmd`, and `undo-part-a.cmd`,
  all running `haik-power.ps1` or `undo-part-a.ps1` next to them. Their runs go to `scripts.log` there.
- To undo Part A without the session: double-click `undo-part-a.cmd` in that folder (silent uninstall, Balanced
  active, copy deleted; tested at 23:07, AFTER equal to BEFORE).

### Part B: Haik's steps

Final steps, written by Part A on 2026-11-04 with the changes of 2026-11-04 below applied. About 70 minutes, about 40
of them hands on; the rest is waiting with the lid closed.

Before you start:

- Wakeshot is installed but not started. Your own plan Balanced is not the active plan: its copy "Wakeshot int
  haik" is, with the same values. Until the end, change no power setting in Settings or Control Panel: they would
  write your own plan.
- If you signed out or restarted after 2026-11-04 23:08, Wakeshot started by itself at sign-in (start at login).
  Tell the orchestrator before block 1.
- The scripts are in this folder. Open it in File Explorer and keep it open; run a script by double-clicking it. Each
  opens a window that ends with "Press any key to continue": read it, then press a key.
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\9e63f57e-1449-4ae9-b41f-f046e0b19a2e\scratchpad\p6-int-haik-checks`
- Wakeshot keeps the PC awake while a Claude Code session works, the orchestrator's too. So keep every other Claude
  Code session idle during the session. After each block, write one line to the orchestrator (what you saw and heard,
  or "as expected") and wait until its answer is done before you start the next block.
- For `wakeshot` commands, use a new, normal (not admin) Terminal window. If it says `wakeshot` is not found, type
  `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot.exe` instead of `wakeshot`.
- Start on battery (charger and dock unplugged), lid open, no external screen, sound on. Blocks 1 to 3 run on battery,
  so the battery is down to 98% or lower for block 4. Blocks 4 and 5 use the laptop's own charger; if the dock is
  your only charger, use the dock there with the screen's cable pulled out of it.
- If something goes wrong and you want to stop: Menu: Quit, then double-click `undo-part-a.cmd` and tell the
  orchestrator.

**Block 1. The real exe (about 5 minutes).**

1. Double-click `values.cmd`. See: "the test copy: all its own values", "your own plan (Balanced): all its own
   values", the active plan "Wakeshot int haik", and "start at login switch: no value (on by default)".
2. Press Start, type `Wakeshot`, check the entry has the Wakeshot icon, press Enter. See: a notice headed "Wakeshot",
   titled "Wakeshot is running", that says how to keep the icon on the taskbar. (No notice: check Do not disturb is
   off, and say so.)
3. Click the notice. See: Settings opens at Taskbar, "Other system tray icons". Turn Wakeshot on. Close Settings.
4. See: the Wakeshot icon on the taskbar, idle. Hover over it: "Not holding, the PC sleeps as usual". Click it. See:
   the menu opens next to the icon, in the taskbar's colours, with round corners and a small icon on each item.
5. Menu: Keep awake > 30 minutes. See: the icon changes to holding. Hover over it: "Holding awake: Keep awake (30 min
   left)" (or 29).

**Block 2. Lid close while holding, then the report (checklists 16 and 10; about 4 minutes).**

1. Keep awake is on. Close the lid. Hear: Wakeshot's lid chime within about a second.
2. Wait about 90 seconds. Open the lid. See: the lock screen (the PC kept running; it did not sleep).
3. Sign in. See: a notice "The PC ran with the lid closed": "The PC kept running for 1 min with the lid closed. What
   held it: Keep awake."

**Block 3. Switch user (checklist 11; about 3 minutes).**

1. Keep awake is still on. Press Ctrl+Alt+Del, choose "Switch user". See: the sign-in screen.
2. Wait 20 seconds. Sign in as yourself. See: your desktop; the icon still shows holding.
   If "Switch user" is not in the list, skip this block and say so.

**Block 4. Battery floor, then a short AC drop on the dock (checklist 8; about 18 minutes, 6 of hands).**

1. Menu: Keep awake > Off. Double-click `floor-on.cmd`. If the battery is at 99% or more, the window says "battery
   too full" and checks again every 30 seconds: stay on battery and use the laptop as normal until it says "done:
   battery floor now 99%". (From now on, energy saver may dim the screen a little.)
2. Menu: Keep awake > 30 minutes. See within about 10 seconds: a notice "Battery low: the PC may sleep now". Hear: the
   battery chime. Hover over the icon: "Battery low, not holding: Keep awake ...".
3. Plug the charger in. See within about 10 seconds: the icon shows holding again.
4. The lid closed under the floor: close the lid. Hear: the lid chime, as in block 2. Wait 10 seconds, unplug the
   charger, and leave the lid closed for 10 minutes; touch nothing. Listen (faint with the lid closed): the battery
   chime soon after you unplug, and about 5 minutes later the "going to sleep" sound; Windows should then sleep the
   laptop. Say what you heard and about when.
5. Open the lid. See: the screen comes on by itself with the lock screen. If it stays dark, press the power button once
   and say so. Sign in. Say which notices you see.
6. Short AC drop on the dock: plug the dock in (the external screen comes on). See within about 10 seconds: the icon
   shows holding (Keep awake is still on). Pull the dock's cable and plug it back within 2 seconds. Say what you saw
   and heard.
7. Double-click `floor-off.cmd`. Menu: Keep awake > Off. Unplug the dock and plug the charger in.

**Block 5. Work ends with the lid closed: sleep (checklist 7; about 16 minutes, 2 of hands; on the charger).**

1. The charger is in, the dock is out, Keep awake is Off. Hover over the icon: "Not holding, the PC sleeps as usual";
   if it names a hold, wait until it does not. In Terminal type `wakeshot run -- ping -n 91 127.0.0.1`, press Enter,
   and close the lid right away. Hear: the lid chime.
2. Leave the lid closed for 13 minutes and touch nothing. (The hold ends after about 90 seconds. About 5 minutes later
   Wakeshot plays its "going to sleep" sound and sets 1-minute screen and sleep timeouts; Windows should then sleep the
   laptop within a few minutes. If it has not slept 5 minutes after the sound, Wakeshot hibernates it.)
3. Open the lid. See: the screen comes on by itself with the lock screen (it slept). If it stays dark, press the power
   button once; the Lenovo logo, then the lock screen, means it hibernated: say which. Sign in. Say which notices you
   see.
4. Double-click `values.cmd`. See: both plans "all its own values" (the 1-minute timeouts are gone).

**Block 6. With the dock, the external screen and its keyboard (rest of checklists 8 and 7; about 15 minutes).**

1. Your own AC lid action Do nothing: unplug the charger and plug the dock in. Keep awake is Off. Double-click
   `dock-on.cmd` (it says "Do nothing (0), as if it were your own setting"). Menu: Keep awake > 30 minutes. Close the
   lid. See and hear: no chime and no lock (the lid action is your own). Work on the external screen for a minute.
   Pull the dock's cable. Hear after about 30 seconds: the lid chime. Open the lid. See: the lock screen. Sign in.
   Menu: Keep awake > Off. Double-click `dock-off.cmd` (it says "Sleep (1), your own value again").
2. Signed in on the external screen: plug the dock in. Menu: Keep awake > 30 minutes. Close the lid. Hear: the lid
   chime; the external screen shows the lock screen. Sign in on the external screen. Pull the dock's cable and plug it
   back within 2 seconds. See and hear: nothing (no chime, no lock). Pull it again and leave it out. Hear after about
   30 seconds: the lid chime. Open the lid. See: the lock screen. Sign in. Plug the dock in.
3. Not while typing: the dock is in, the lid open. Menu: Keep awake > Off. In Terminal type `wakeshot run -- ping -n 61
   127.0.0.1`, press Enter, and close the lid right away. Hear: the lid chime; the external screen shows the lock
   screen. Sign in on the external screen, open Notepad and keep typing on the external keyboard (any text) until 7
   minutes after the lid close. See and hear: no "going to sleep" sound and no sleep (the external screen stays on).
   Open the lid.

**Block 7. Finish (about 2 minutes).**

1. Menu: Keep awake > 30 minutes. Menu: Restore my power settings. See: a notice "Your power settings are back"; the
   icon still shows holding.
2. Menu: Quit. See: the icon goes away.
3. Double-click `values.cmd`. See: both plans "all its own values".

**Block 8. Start at login off (needs one sign-out; about 6 minutes).**

1. Settings > Apps > Startup: turn Wakeshot off. Close Settings. Double-click `values.cmd`. See: "start at login
   switch: off (03-...)".
2. Tell the orchestrator "block 8: signing out", and wait for its answer: a sign-out ends every agent, so it stops
   starting new ones first. Then Start > your name > Sign out.
3. Sign in. Wait 2 minutes. See: no Wakeshot icon on the taskbar, also not under the ^ arrow.
4. Settings > Apps > Startup: turn Wakeshot on again. Double-click `values.cmd`. See: "start at login switch: on
   (02-...)".
5. Start `/orchestrate` again and tell it "done", with anything that looked or sounded wrong.

### Part C: agent, after Haik

1. Read the tray's log for each block and record the timeline: lid close, lock and chime; the closed-lid report;
   console left (restore) and back (apply); the hold end, the countdown, the hibernate checks, `SetSuspendState`, the
   resume; the floor step back and the hold again; the dock cases; Restore my power settings; Quit and its restore.
2. Check the copy's values are its originals after Quit; Haik's plan values are unchanged.
3. Haik's plan active; delete the copy; uninstall silently; check Run, RunOnce, the Path entry and the data folder are
   gone; AFTER equals BEFORE; delete the scripts.
4. `docs/research.md`: a section "Integration checklist with Haik": each item, Haik's words, the log facts.

Notes from Part A for Part C (2026-11-04 23:20):

- Blocks 4 and 5 swapped places against the plan (block 4 is now the floor, block 5 the work end), and the dock's
  short AC drop moved from block 6 into block 4 step 6: only right after the floor block is the battery both under
  the floor and on AC. Blocks 1 to 3 run on battery to bring it to 98% or lower.
- Times of Haik's script runs: `scripts.log` in the scratch folder (Part A's own tests are in
  `scripts-partA-test.log`). `values.cmd` lines there also give the `StartupApproved\Run\Wakeshot` bytes the real
  switch wrote (block 8).
- BEFORE is `before.json` there; `state.ps1` (elevated is fine, read only) reads the same fields for AFTER. The data
  folder did not exist at BEFORE: the install made it, and the uninstall deletes it after a good restore.
- Block 1 step 3 makes Windows promote the `NotifyIconSettings` entry `2334163346184210159` (Haik's own taskbar
  choice; at BEFORE it had no `IsPromoted`). Record it; it is not Wakeshot's to undo.
- `undo-part-a.ps1` is the tested teardown (uninstall, Balanced active, copy deleted); run it at Medium.
- The installed build is release, so no update feed seam; with no GitHub release its update check finds nothing.

### Changes of 2026-11-04 (orchestrator `wakeshot-12`; apply them when you write the final steps in Part A)

Applied in Part B on 2026-11-04 23:20: 1 is block 5 (13 minutes closed, `values.cmd` after sign-in); 2 is block 4
step 4; 3: the copy's dim is AC 585 s and battery 225 s (screen off 300 s and 180 s); block 5 runs on AC and block 4
step 4 on battery, so Part C can answer for both; 4 is block 8, with `values.cmd` reading the switch bytes after off
and after on (the bytes before: no value).

Haik decided on 2026-11-04 that after a closed-lid work end, and at the battery floor, the laptop sleeps, not
hibernates (`docs/product.md` work end rule; cards `p2-6-sleep-route` and `p2-6-sleep-route-fixes`). So:

1. The work end block (Part B, `wakeshot run -- ping -n 91` with the lid closed): expect sleep, not hibernate. About
   5 minutes with no input after the work ends, the "going to sleep" sound, then Windows sleeps the laptop within
   about a minute (Modern Standby; the screen comes on by itself at lid open; locked). After sign-in, check that the
   copy's screen and sleep timeouts are its own again. Part C reads the `SleepCheck` and the short timeout write and
   restore in the log, and Kernel-Power 506 and 507 in the System log (no hibernate expected).
2. The battery floor block: the same sleep route once the lid is closed under the floor; record what happens.
3. The dim timeout (review note in `docs/reviews/p2-6-sleep-route-review.md`): Part A records the copy's dim timeout
   on battery; Part C says whether the screen went off at 60 s or only after the dim time.
4. New block, last (needs one sign-out): Settings > Apps > Startup: turn Wakeshot off. Part A reads the
   `StartupApproved\Run\Wakeshot` bytes before and after (`p6-startup-switch-check`: the off value was only written by a
   script there). Haik signs out and in once. See: no Wakeshot icon after sign-in (start at login is off). Then he turns
   it on again in the same page. A sign-out ends every agent: the orchestrator stops spawning first, and Haik starts
   `/orchestrate` again after sign-in.

## Acceptance checks

- Each block passes, or its failure is recorded with Haik's words and the log lines, and routed as a fix card in
  "Found outside scope". Skipped blocks are named with the reason.
- AFTER equals BEFORE; no `Wakeshot int` plan; no install; no process this card started is left.
- Checklist items 7, 8, 10, 11 (console part), 13 (real exe) and 16 each have a result line in the Result.

## Out of scope

- Fixing code (the orchestrator plans fix cards from the findings); Remote Desktop (later); item 12 (later).

## Questions for Haik

None (the dock question is in `p6-plan-integration`'s Result).

## Found outside scope

Problems seen but not fixed.

1. `docs/phases/06-integration.md`, checklist map rows 7 and 8, name this card's blocks 4 and 5 and say "hibernate
   after 5 min" for row 7. Now: row 7 is block 5 (sleep, hibernate only as the fallback) and block 6 step 3; row 8 is
   block 4 (floor, the lid closed under the floor, the dock's short AC drop) and block 6 steps 1 and 2. Done in Part C
   at the orchestrator's word (2026-11-05): only the block column of those two rows changed.
2. For the record: the laptop has been on battery since 23:07:46 (Kernel-Power 105); 98% at 23:15. Not caused by this
   card; it sits safely so (Haik's own values on the active copy), but it drains.

Findings of Part C (2026-11-05). Facts and how they were checked: `docs/research.md`, "Integration checklist with
Haik". Evidence in the scratch folder named in Part A: `partC-wakeshot.log` (tray log), `partC-system-power.txt`
(Kernel-Power events), `partC-icon-evidence\`.

3. **The first-run click does not open "Other system tray icons" (block 1).** Haik: "Clicking the notification moves
   to the correct section in Settings but not directly Other system tray icons. But I guess if it's a limitation by
   the OS it's ok." Log: "notice click opened taskbar settings" 00:23:46. Root cause: an OS limit. Microsoft's
   `ms-settings:` list (Learn, updated 2026-11-02) has only `ms-settings:taskbar` for the page and no way to open a
   section; a web search found no other way. Fix: none. The first-run text already says to turn Wakeshot on under
   "Other system tray icons" (`wakeshot-core` `text.rs`). No product decision.
4. **Settings shows a coffee cup, off centre, for Wakeshot (block 1).** Haik: "the Wakeshot logo in Settings differs
   from the logo in the tray. It's a cup of coffee, and not centered." Facts: Settings draws the picture stored in
   `HKCU\Control Panel\NotifyIconSettings\2334163346184210159\IconSnapshot` (the product GUID's entry). It is a 24 px
   idle Cup (saved as `partC-icon-evidence\IconSnapshot.png`), drawn low because the idle Cup leaves room for the
   steam. Root cause: a stale snapshot from a run of the product GUID before the Sunrise replaced the Cup
   (2026-11-04); Windows did not renew it while the Sunrise build ran 00:20 to 01:33. A real user's fresh install
   gets the Sunrise (the snapshot is taken at the first add; inferred, not checked on another PC), so only this dev
   laptop. Fix: no code. On this laptop, delete that key (Explorer's own; Haik's OK; whether Windows then stores the
   Sunrise at the next run: not verified). Note for any later change of the tray art: existing users keep the old
   picture in Settings. No product decision.
5. **The balloon header shows a white square, not the Wakeshot icon (block 1, also in the 00:50 photo).** Facts:
   started from the Start menu, the tray carries the shortcut's AppUserModelID `velopack.HaikAsatryan.Wakeshot`, and
   Windows filed all 14 balloons under it (`Notifications\Settings\velopack.HaikAsatryan.Wakeshot`, count 14, last
   01:33:03), not under the shell's `NotifyIconGeneratedAumid_<id>` that the dry-run runs used (that path shows the
   tray glyph from a PNG in `%TEMP%`). No `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` key,
   so no `IconUri`. The exe's icon resource is right (violet plate, gold sun, 16 to 256 px). The header shows the
   plate's shape in plain white. Root cause: found only in part. The attribution to the Start shortcut's id is
   verified; why Windows draws that id's icon as a white square is not. Who sees it: every user who starts Wakeshot
   from Start, not only this laptop; a start from the Run value at sign-in most likely shows the tray glyph instead
   (inferred). Fix, in two steps: (a) a check card with the screen (about 30K): one notice after a Start launch, one
   after a direct exe launch, a screen grab of each header; (b) then one of: the install hook
   (`wakeshot-windows` `hooks.rs`, install, update and uninstall) writes
   `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` with `DisplayName` "Wakeshot" and an
   `IconUri` to a PNG of the app icon shipped in the install folder (the two values Windows wrote itself for the
   generated id), removed at uninstall; or the tray sets its own id at start (`SetCurrentProcessExplicitAppUserModelID`)
   so both start paths behave the same. About 60 to 80K with an e2e check. No product decision (design call).
6. **No "going to sleep" sound with the lid closed under the floor (block 4 step 4).** Haik: "When the lid was
   closed, after 5 min I did not hear the sleep sound, but I think it slept, as on lid open I got a message that less
   than a minute it took the hold as battery was low." Timeline: lid close 00:36:47.8 (lock, chime); unplug 00:37:07.47
   (Kernel-Power 105); step back 00:37:07.50; the user's lid actions (Sleep) and battery sleep written back 00:37:08.49;
   during the plan re-apply Windows turned the screen off for the lid (506 reason 15, 00:37:08.80) and slept (566 to
   sleep 00:37:08.89), 1.4 s after the unplug; the 5-minute countdown had started 5 ms before and was cancelled at the
   lid open; the battery chime's audio call was cut by the suspend (WARN `GetCurrentPadding` 1460 at resume), so no
   chime either; asleep 10 min 42 s (disconnected standby, 140 mWh); lid open 00:47:50; the step back balloon and the
   report "The PC kept running for less than a minute with the lid closed. What held it: Keep awake." at 00:47:52.
   Root cause: Windows' own lid action slept the laptop at once when Wakeshot put it back with the lid closed, right
   after the AC loss, so Wakeshot's sleep route never ran. Safe: yes; it slept on battery under the floor, locked
   since the close, sooner than designed. What the docs got wrong: `docs/product.md` "Battery floor" (the 5-minute
   route then sleep) and its work end rule ("putting the lid action back does not sleep a laptop whose lid is already
   closed (tested)"); that holds without a power source change (Phase 0 test E), not right after one; which of the
   two made Windows act here: not verified. The Part B step text expected the chime and the sound; both were wrong
   for this case. The cost: someone typing on an external screen with the lid closed, under the floor, is slept at
   once when AC drops, not after 5 minutes without input. Fix: docs (`product.md` Battery floor and work end rule,
   `architecture.md` "Episode flow"): with the lid closed, the restore at a floor step back can make Windows sleep at
   once. Item 7 (b) would keep a dock's short drop from getting there. A code change (keep the lid key until the
   5-minute check, `wakeshot-core` `core/lid.rs` and the power side, medium size, the laptop then runs up to 5 more
   minutes on battery under the floor) only if Haik wants it: **product decision**; my default: docs only.
7. **A battery-low notice on screen while on AC and holding (block 4 step 6).** Haik: "On plugging in, the
   keep-awake mode is on again automatically; on removal, instantly a sound and a notice that it stopped as the
   battery is low." And: "for more than 10 seconds, the dock was on and Wakeshot was active, but I got a wrong
   notification". Photo (`Screenshot 2026-11-05 005234.png`, taskbar clock 12:50): the step back balloon, battery
   charging at 92%, the pointer resting on the balloon. Log: with the floor on and the lid open AC was lost once,
   00:50:01.89 to 00:50:11.55 (9.65 s for a pull and replug "within 2 s"); step back 23 ms after the loss; chime
   audible 1.4 s after it; one step back balloon at 00:50:03.76; AC back: hold in force at 00:50:12.63, no sound, no
   balloon. The only other step back balloon (00:47:52) was the one held over the sleep. Root cause: the 00:50:03
   balloon stayed after AC came back: Wakeshot never takes a step back balloon away when the step back ends, and
   Windows kept the banner up with the pointer on it (common behaviour, not verified in docs). No queue of late
   balloons. Noise: per AC loss under the floor, one chime and one balloon within 2 s; nothing at the return. The
   dock's drops this night: 4.3, 4.4, 4.5, 4.7 and 9.65 s (the other four with the floor off: nothing). Fix: (a) when
   the step back ends because AC is back, remove its balloon (`NIM_MODIFY` with `NIF_INFO` and an empty `szInfo`; a
   new command from `wakeshot-core` `core/lid.rs` `settle_floor` to `wakeshot-windows` `notify.rs`; a contracts
   change, so a card the orchestrator runs alone; about 30 to 40K; whether the Windows 11 banner then goes: to test);
   no product decision. (b) A grace before the floor step back on an AC loss, as the closed-lid lock already waits
   30 s for a dock's drop (`core/lid.rs`), so a drop of under 30 s gives no chime, balloon or restore: about 30 to 40K
   with tests; **product decision** (`product.md` says "at once"); my default: yes, 30 s.
8. **The real Startup switch writes `01` and `00`, not `03` and `02` (block 8).** Off at 01:34:04:
   `StartupApproved\Run\Wakeshot` = `01-00-00-00-9D-AA-39-37-A4-6C-DD-01`; on again: 12 zero bytes. Windows honoured
   `01` (no tray at the 01:38:35 sign-in). No code depends on the bytes (the uninstall deletes the value; AFTER: gone).
   Fix: `research.md` "Where Windows keeps the switch" and the comments in `packaging/e2e.ps1` step 5 (it seeds the
   `03` form; seeding the `01` form is a one-line change). Tiny; no product decision. `values.cmd` printed the `01`
   form as a blank state (my Part A script mapped only 02 and 03).
9. **Block 6 step 3 did not test typing (block result, for the orchestrator).** The lid opened 4 min 51 s after the
   close (01:27:26 to 01:32:17), before 5 minutes without input could pass even with no typing (due about 01:32:21 to
   01:32:26). No sleep came, but the run does not show that typing holds it off. Fix: rerun only that step with 7
   full minutes if wanted (about 10 minutes of Haik's time).
10. **Test leftovers in `%TEMP%`, not this card's.** Ten `wakeshot-*` test folders from 2026-11-04 04:16 (a
    `cargo test` run) and a `velopack` folder from 2026-11-04 06:37 (the time of `p6-uninstall-startup-approval`'s
    e2e run). Not deleted (not mine).
11. **Windows keeps `HKCU\...\Notifications\Settings\velopack.HaikAsatryan.Wakeshot` after the uninstall**, like
    `NotifyIconSettings`. `README.md` lines 482 to 483 name `NotifyIconSettings\<id>` and
    `Notifications\Settings\NotifyIconGeneratedAumid_<id>` as left, not this key. Tiny docs change; no product
    decision.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-04 23:0x: Part A started (worker). Orchestrator confirms no other agent runs until Part C ends. Haik is away;
  Part A leaves the laptop installed but not started, the plan copy active.
- 2026-11-04 22:59:38: A1 BEFORE, elevated read-only `state.ps1` (scratch), saved to `before.json` there. Run 7 values
  (no `Wakeshot`), RunOnce empty, `StartupApproved\Run` 1 value (no `Wakeshot`), Path type 2, 332 bytes, hash
  `A8FDC61D7FFE5745` (as in `p6-int-update-e2e`). `powercfg /l`: Balanced only, active. Haik's plan: lid 1/1, sleep
  18000/18000, hibernate after 0/2147483647, saver DC 30, screen off 300/180, dim 585/225, `/qh` hash
  `C64AFDB1CFDD13CF` (same as the update card's runs). No data folder, no install folder, no uninstall key, no
  shortcut, no Wakeshot process. One `NotifyIconSettings` entry from an earlier install (`2334163346184210159`, no
  `IsPromoted`, path of the removed exe; `p6-int-update-e2e` Found outside scope 2). Session locked (LogonUI since
  22:47:42). Battery 99%, on AC.
- 2026-11-04 23:01: A2 build: release x64 of the tree as it is (`-p wakeshot-tray -p wakeshot-cli --locked`, RUSTFLAGS
  unset, `CARGO_TARGET_DIR` my scratch `target`): exit 0 in 91 s; tray 2,740,224 bytes (SHA256 `21E4AD8CB53BB061...`),
  CLI 558,080 (`5D58FED22EFD56FF...`); newest source `power/journal.rs` 19:00:49. `pack.ps1 -Version 0.1.0 -Cpu x64
  -Icon assets\wakeshot.ico` into scratch `pack`: exit 0 in 4 s, `Setup.exe` 11,763,094 bytes, full nupkg 4,205,462.
  No GitHub release exists (`gh release list` empty), so the installed tray's update check finds nothing.
- 2026-11-04 23:03:14: A3 then A2 install, in one script at Medium (`a2.cmd` opened with `explorer.exe`; `a2-medium.log`
  in scratch; "Mandatory Label\Medium Mandatory Level"). Copy before install, as `e2e-update.ps1` does, so a hook
  that acted would act on the copy. Copy `d740a909-1f61-423a-8e53-3920307b091b` "Wakeshot int haik" made from Balanced
  and made active 23:03:17. Its 1,736 `/qh` setting lines equal Balanced's but the alias line. `Setup.exe --silent`
  exit 0 in 3.3 s; no process from the install folder 3 s later (tray not started). Hook log: Run value set, CLI
  folder added to the user Path, no journal. After (23:03:33): Run 8 values (`Wakeshot` REG_SZ, quoted
  `current\wakeshot-tray.exe`), RunOnce empty, no `StartupApproved\Run\Wakeshot`, Path 450 bytes with the entry,
  uninstall key DisplayVersion 0.1.0, Start menu shortcut to `current\wakeshot-tray.exe` (icon index 0), data folder
  now exists with `logs\wakeshot.log` only (the hook's). Haik's plan unchanged (`/qh` `C64AFDB1CFDD13CF`). The session
  showed unlocked at 23:03:33 (no LogonUI).
- 2026-11-04 23:04: A4 `powercfg /a`: available: Standby (S0 Low Power Idle) Network Connected, Hibernate, Fast
  Startup. So the hibernate fallback is possible; no setting changed.
- 2026-11-04 23:06 to 23:09: A5 scripts in scratch: `haik-power.ps1` (Windows PowerShell 5.1) with one `.cmd` per
  action (`floor-on`, `floor-off`, `dock-on`, `dock-off`, and a read-only `values` added for the checks below), and
  `undo-part-a.cmd`. They write only the copy `d740a909-...` (id fixed in the script), refuse if it is missing or not
  the active plan, re-apply it with `/setactive` after each write, log to `scripts.log` next to them, and end with
  `pause`. `dock-on` and `dock-off` refuse while `journal.json` exists (Wakeshot owns a setting). `floor-on` reads the
  percent from `GetSystemPowerStatus` (Wakeshot's source); at 99% or more it says "battery too full, do block 5 last"
  and waits, checking every 30 s, then sets the floor once the battery is 98% or lower (the floor is strictly below).
  Test at Medium (`t5.cmd` with `explorer.exe`, twice; `t5-medium.log`): dock-on 1 to 0, dock-off back to 1; dock-on
  with a journal in a fake `LOCALAPPDATA` refused and wrote nothing; floor-on with a test percent of 99 still waiting
  after 5 s (stopped by its pid, nothing written); at 98 it set 30 to 99; floor-off back to 30. After: the copy's
  `/qh` hash `6E496610E3BB3951`, the same as right after it was made; Haik's plan unchanged. The test lines are in
  `scripts-partA-test.log`, so `scripts.log` holds only Haik's runs.
- 2026-11-04 23:07:13: undo test at Medium (`undo-part-a.ps1`): uninstall exit 0, Balanced active, copy deleted, 7 s in
  all; then a `state.ps1` read equal to BEFORE on all 20 fields (Run, RunOnce, StartupApproved, Path type, bytes and
  hash, plans, active, Haik's values, data and install folders, uninstall key, shortcut, icons, processes); no
  Velopack temp folder, no `HaikAsatryan.*` folder. 23:07:47: copy and install again at Medium with the same copy id
  (`/duplicatescheme` with the destination id): Setup exit 0 in 1.1 s, no process started; state equal to the first
  install on every field; installed tray SHA256 `21E4AD8CB53BB061...` (the build above).
- 2026-11-04 23:07:46: Kernel-Power 105, power source change: the laptop has been on battery since (99% at 23:09:34;
  battery saver flag 0). No command of this card changes the power source; likely Haik or the dock (not verified).
  The floor test at 23:09:16 to 23:09:19 had the floor at 99% for 2 s on battery at 99%: not under the floor, and the
  battery saver flag read 0 after.
- 2026-11-04 23:14:46: third Medium test after a wording change in `floor-on` (it no longer names a block) and the
  `values` line for the start at login switch: same outcomes as above. This time the real battery was 98% on battery,
  so the floor sat at 99% from 23:14:57 to 23:15:00, under it; the battery saver flag (`GetSystemPowerStatus`) read 0 at
  23:15:10. The System log shows Kernel-Power 506 at 23:14:31 (Modern Standby entry; likely the 180 s battery screen
  timeout with nobody at the laptop, not verified) and UserModePowerService 12 for each of my `/setactive` calls.
- 2026-11-04 23:10: A3 detail for the dim question (change 3): the copy's dim timeout is AC 585 s, battery 225 s; its
  screen off AC 300 s, battery 180 s (the review's 165 s battery dim is no longer the value).
- 2026-11-04 23:17:34: end of Part A: elevated `state.ps1` read equal to the second install's on every field; session
  locked; no process with this card's folder in its command line; build folder `target` deleted (checked gone); `pack`
  kept for Part C. `docs/research.md`: section "Integration checklist with Haik: preparation" appended (re-read
  first; LF). Part B written in place with the changes of 2026-11-04 applied; Status `blocked`.
- 2026-11-05: Part C started (worker). Haik did blocks 1 to 8 (about 23:30 to 01:xx) and signed back in; the
  orchestrator relayed his words. I own the whole laptop until Part C ends; no cargo build.
- 2026-11-05 01:43 to 01:56: evidence saved to the scratch folder (`partC-wakeshot.log`, `partC-system-power.txt`,
  `partC-config.json`, `partC-state.json`, `partC-icon-evidence\`). Tray log read whole; System log Kernel-Power
  events 00:15 to 01:45 dumped. No Wakeshot process; session locked since 01:53:22. Icon evidence: the product GUID's
  `NotifyIconSettings` entry holds a 24 px idle Cup snapshot; notices filed under the Start shortcut's AUMID
  `velopack.HaikAsatryan.Wakeshot`. 01:56:10 pre-teardown read: the copy `/qh` `6E496610E3BB3951` (as made), Haik's
  `C64AFDB1CFDD13CF` (as BEFORE).
- 2026-11-05 01:56:37 to 01:56:43: C3 teardown at Medium ("Mandatory Label\Medium Mandatory Level"): `undo-part-a.ps1`
  through a `.cmd` opened with `explorer.exe`: uninstall exit 0, Balanced active, copy deleted. 01:56:50 `state.ps1`
  AFTER compared field by field with `before.json`: equal on every field but `Icons` (`IsPromoted` 1, Haik's block 1
  choice) and the time. No `wakeshot` process, no data or install folder, `powercfg /l` Balanced only.
- 2026-11-05 02:01: `docs/research.md` section "Integration checklist with Haik" appended (re-read first; LF kept).
  `docs/phases/06-integration.md` map rows 7 and 8: block column only. Scripts deleted (21 `.ps1`, `.cmd`, `.py`),
  `pack\` deleted, my copy of the notification database deleted; logs, JSON reads and `partC-icon-evidence\` kept.
  For the record: one read-only check command of mine also ran a stray `git --version` (no repo access, nothing
  changed); against the card's "never run git", noted here.

## Result

**Done 2026-11-05.** Haik did blocks 1 to 8 (2026-11-04 23:30 to 2026-11-05 01:38); Part C read the logs, tore
down and found AFTER equal to BEFORE. No block failed on safety; four findings need fix cards or decisions (Found
outside scope 4 to 8). Facts: `docs/research.md`, "Integration checklist with Haik".

| Block (checklist item) | Outcome |
|---|---|
| 1 real exe (13) | Pass, with findings. Start entry icon right; first-run balloon; click opened Settings > Taskbar but not the "Other system tray icons" section (OS limit, item 3); tooltip "30 min left" then 29 (as the step allows). Settings shows an old Cup (stale snapshot, this laptop only, item 4); the balloon header shows a white square (Start shortcut's app id; seen by real users, item 5). Block 1 ran twice (orchestrator restart). |
| 2 lid close and report (16, 10) | Pass. Lock and chime 1.1 s after the close; report balloon at sign-in. The lid stayed closed 2 min 32 s, not 90 s. |
| 3 Switch user (11, console part) | Pass. Keys put back when the console left (00:32:50), applied again on return (00:33:18). |
| 4 battery floor (8) | Pass on safety, with findings. Floor step back and hold again on AC as designed. Lid closed under the floor: Windows slept at once by its own lid action when Wakeshot put it back after the unplug; no chime and no "going to sleep" sound (item 6). The dock's pull of about 2 s was a 9.65 s AC loss; its step back balloon stayed on screen after AC returned (item 7). |
| 5 work end with the lid closed (7) | Pass. Sound 00:59:55 (heard), 1-minute timeouts, Windows slept 18 s later; no hibernate; timeouts back at the lid open; both plans their own values after. |
| 6 dock (8, 7) | Steps 1 and 2 pass (no lock with the own Do nothing; lock and chime 30 s after a lasting AC loss; nothing for a 4.7 s drop). Step 3: no sleep while the lid was closed, but the lid opened 4 min 51 s after the close, too early to show that typing holds the sleep off (item 9). |
| 7 Restore and Quit | Pass. Keys back, still holding by request; Quit with no journal left; both plans their own values. |
| 8 start at login off | Pass. No tray at the 01:38:35 sign-in. The real switch writes `01` (off) and `00` (on), not `03` and `02` (item 8). |

Checklist result lines: item 7 pass (block 5; block 6 step 3 inconclusive on typing); item 8 pass on safety, with
items 6 and 7; item 10 pass; item 11 console part pass; item 13 real exe pass, with items 4 and 5; item 16 pass. The
dim timeout question: it played no part; the screen went off 31.5 s after each lid close at the lock screen's own
timeout, before any 1-minute timeout.

Part C checks: C1 timeline in `research.md`; C2 values after Quit: the copy and Haik's plan their own values
(`values.cmd` 01:33:28; `/qh` hashes at 01:56:10 equal to the copy as made and to BEFORE); C3 teardown at Medium,
uninstall exit 0, Run, RunOnce, `StartupApproved\Run`, the Path entry and the data folder gone, AFTER equals BEFORE
but the icon entry's `IsPromoted` (Haik's choice), scripts deleted; C4 `research.md` section written.

Files changed in Part C: this card; `docs/research.md` (new section "Integration checklist with Haik");
`docs/phases/06-integration.md` (map rows 7 and 8, block column only). Left on the laptop: nothing of the install;
Windows' own `NotifyIconSettings\2334163346184210159` (promoted) and
`Notifications\Settings\velopack.HaikAsatryan.Wakeshot` keys; in the scratch folder only logs, JSON reads and `partC-icon-evidence\`.

Open items: fix cards or decisions for items 5, 6, 7 (two product decisions: item 6 code change, default no; item 7
(b) a 30 s grace, default yes), docs for items 6, 8 and 11, and a possible rerun of block 6 step 3 (item 9).

**Part A (2026-11-04 23:20)**, kept for the record:

What is in place: "What Part A left in place" under Part A (install path, Run entry, Path entry, data folder, active
copy `d740a909-1f61-423a-8e53-3920307b091b`, scripts, the undo). Haik's steps: "Part B: Haik's steps" (blocks 1 to
8, about 70 minutes, about 40 of hands).

| Step | Outcome |
|---|---|
| A1 BEFORE | Run 7 values, RunOnce empty, `StartupApproved\Run` 1 value (no Wakeshot), Path type 2, 332 bytes, hash `A8FDC61D7FFE5745`; Balanced the only plan and active, lid 1/1, sleep 18000/18000, saver DC 30, screen off 300/180, dim 585/225, `/qh` `C64AFDB1CFDD13CF`; no data folder, no install, no Wakeshot process |
| A2 build, pack, install | release x64 (91 s), 0.1.0 packed (4 s), `Setup.exe --silent` at Medium exit 0, tray not started |
| A3 plan copy | `d740a909-...` "Wakeshot int haik", every setting equal to Balanced's, active |
| A4 `powercfg /a` | Hibernate available (with S0 Low Power Idle and Fast Startup) |
| A5 scripts | `floor-on`, `floor-off`, `dock-on`, `dock-off`, read-only `values`, `undo-part-a`; tested at Medium three times, values back each time; undo tested (AFTER equal to BEFORE), then copy and install again, equal to the first install |
| A6 | Part B written, Status `blocked` |

Script paths, in the folder
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\9e63f57e-1449-4ae9-b41f-f046e0b19a2e\scratchpad\p6-int-haik-checks`:
`floor-on.cmd`, `floor-off.cmd`, `dock-on.cmd`, `dock-off.cmd`; also `values.cmd` and `undo-part-a.cmd`. The undo
of Part A without the session: double-click `undo-part-a.cmd`.

Decided by the worker (design calls, for Haik's review):

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | Copy and install at Medium through `explorer.exe`, the copy before the install | A real user installs at Medium; a hook that acted would act on the copy | Never likely |
| 2 | Blocks 1 to 3 on battery; the floor block before the work end block; the dock's short AC drop inside the floor block | The always-plugged dock keeps the battery at 99% to 100%, and the floor test needs 98% or lower; only right after the floor block is it under the floor and on AC | The battery can be kept under 99% another way, or Haik wants the plan's order |
| 3 | `floor-on` waits, checking every 30 s, instead of only saying "do block 5 last" | Doing it last does not drain a docked battery | Never likely |
| 4 | Block 2 waits about 90 s, not 30 s | The report then says "1 min", so its number is checked, not only "less than a minute" | Haik wants the shorter wait |
| 5 | Block 5 keeps the lid closed 13 minutes, not 8 | If the screen goes off only at the AC dim (585 s), sleep comes after the 5-minute fallback; 13 minutes fits the hibernate at about 11.5 | Part C shows the screen goes off at 60 s: then 9 minutes is enough |
| 6 | Block 6 step 3 starts `wakeshot run` before the lid close, with `ping -n 61` | With nothing holding, the own Sleep lid action would sleep the docked laptop at the close; 60 s leaves time to sign in on the external screen | Never likely |
| 7 | A fifth, read-only `values.cmd` | The check after sign-in (change 1) and the real switch bytes (change 4), each with its time in `scripts.log` | Never likely |
| 8 | `dock-on` and `dock-off` refuse while `journal.json` exists | A write while Wakeshot owns the lid key would mix Haik's value with Wakeshot's | Never likely |
| 9 | Scripts in Windows PowerShell 5.1 (`powershell.exe`) with `-ExecutionPolicy Bypass` | On every Windows; `pwsh` here is a Store alias | Never likely |

Files changed: this card; `docs/research.md` (new section "Integration checklist with Haik: preparation"). Scratch
only: `state.ps1`, `a2.cmd`, `a2-medium.ps1`, `t5.cmd`, `t5-medium.ps1`, `haik-power.ps1`, the six `.cmd` files,
`undo-part-a.ps1`, logs and JSON reads, `pack\` (my doc helpers deleted).

Checks so far: Haik's plan never written (`/qh` hash equal at every read); no process of this card left; build folder
deleted. The card's acceptance checks run in Part C.

Open items: Part B (Haik), then Part C. Found outside scope 1 (phase file map rows 7 and 8) for the orchestrator.
