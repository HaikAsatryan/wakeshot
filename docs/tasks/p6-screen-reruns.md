# Screen session extras: Settings icon, test leftovers, typing rerun, floor grace check

Phase: 6 (integration checks). Status: done (2026-11-05 11:3x; worker decisions for Haik's review). Needs Haik: yes, about 20 minutes at the screen, in the same session as
`p6-notice-app-icon` Part B (dock in, lid open, unlocked). Agent: `wakeshot-worker`. Size: about 80K tokens on top of
`p6-notice-app-icon` Part B. Written by the orchestrator, 2026-11-05, from `p6-int-haik-checks` "Found outside
scope" 4, 9 and 10 and `p2-6-floor-grace` "Found outside scope" 3. Haik's answers (2026-11-05): yes to deleting the
key and the leftovers, yes to both reruns.
Runs after: none; runs as a bundle with `p6-notice-app-icon` Part B (one worker, the order below). Parallel with:
nothing that builds, changes power settings or edits `packaging/`.
Real laptop: yes: the plan copy of `p6-notice-app-icon` Part B (step 3), a test install, Haik's own values never
written.

## Goal

Use the one screen session for four small leftovers of the integration checks: a fresh Settings icon, an empty
`%TEMP%`, a real typing test with the lid closed, and Haik's check of the 30 s floor grace.

## Read first

- `docs/tasks/p6-notice-app-icon.md`, Part B (the frame of this session; its steps 1 to 10 stay as written).
- `docs/tasks/p6-int-haik-checks.md`: "Found outside scope" 4, 9, 10; Part A step 5 (the `floor-on`, `floor-off`
  scripts); Part B blocks 4 and 6 (the original steps).
- `docs/tasks/p2-6-floor-grace.md`: Goal, "Found outside scope" 3; `docs/product.md` "Battery floor".
- `CLAUDE.md`: Test laptop, and the facts on elevation.

## Deliverables

Order inside the bundle:

1. `p6-notice-app-icon` steps 1 and 2 (guard, BEFORE). Add to BEFORE: the values of
   `HKCU\Control Panel\NotifyIconSettings\2334163346184210159`, and the list of `%TEMP%\wakeshot-*` folders and
   `%TEMP%\velopack` with their times.
2. Finding 10: delete the ten `%TEMP%\wakeshot-*` folders dated 2026-11-04 about 04:16 and the `%TEMP%\velopack`
   folder dated 2026-11-04 about 06:37. Anything with another date, or anything under `%TEMP%\claude`: leave it and
   list it. Record what went.
3. Finding 4: `reg export` the key above into the scratch folder, then delete it. This must happen before the tray's
   first start in step 6 of `p6-notice-app-icon`, so Windows takes a fresh snapshot at the icon's first add.
4. Before any cargo command: if the Visual Studio Installer runs (Haik is adding clang now), wait until its process
   ends (check every 30 s, up to 40 minutes; then stop and say so). Then `p6-notice-app-icon` steps 3 to 5 as written.
   Also write in the scratch folder `floor-on.cmd` and `floor-off.cmd` for the plan copy, as `p6-int-haik-checks`
   Part A step 5 did. Pick a threshold that puts the floor in force at the current charge, by the rule in
   `wakeshot-core` `policy/floor.rs` and the values Windows accepts; on the copy only. If no value does at 99 or 100%,
   the script says so and waits on battery as before. `floor-off` puts the copy's value back. Run each once and read
   the value back, then set it back.
5. Set Status `blocked` with Haik's steps for the first part (`p6-notice-app-icon` steps 6 and 7), and stop. The
   orchestrator relays and sends Haik's answers back.
6. `p6-notice-app-icon` steps 8 and 9 (the second start). Then, with that tray running, block for Haik's second part,
   in this order:
   - **Settings look:** Settings > Personalization > Taskbar > Other system tray icons. Say what picture the
     Wakeshot entry has: the Sunrise (violet square, gold sun), the coffee cup, or something else. Then read the key
     again and keep its new `IconSnapshot` as PNG, as `p6-int-haik-checks` Part C did.
   - **Typing rerun (finding 9, about 10 minutes):** block 6 step 3 as written, with these changes: open a new
     Terminal window first (the install added the Path entry), hover over the icon first and wait until it says
     "Not holding" (a working Claude Code session holds it), and type for 7 full minutes after the lid close by the
     clock. Expected: no "going to sleep" sound, no sleep, the external screen stays on.
   - **Floor grace check (about 12 minutes):** write the exact steps from `p2-6-floor-grace` "Found outside scope" 3:
     on the dock and under the floor (`floor-on`), Keep awake > 30 minutes; a dock drop under 2 s gives nothing; a pull
     left out gives the battery chime and the "Battery low" notice once, at about 30 s; back in, holding again with no
     sound; then lid closed (lid chime, lock), pull the dock and leave it out with the lid closed for 7 minutes, touch
     nothing, and note when the battery chime comes and whether and when Windows sleeps (the open fact: at the
     restore, or after the 5 minutes). Then `floor-off`, Keep awake > Off, dock in.
   Each step says what to do, what to see and hear, and what to report, as `p6-int-haik-checks` Part B does.
7. After Haik's answers: read the tray log and the Kernel-Power events for the two reruns (as `p6-int-haik-checks`
   Part C did), then `p6-notice-app-icon` step 10 (uninstall, plan back, AFTER equals BEFORE).
8. Facts in `docs/research.md` "Integration checklist with Haik" (additions only, above the "Battery floor grace
   checks" heading if that one is still last; `p2-6-floor-grace` "Found outside scope" 2). Results in this card and
   in `p6-notice-app-icon`.

## Acceptance checks

- The leftovers and the key are gone, and nothing else was deleted.
- The typing rerun and the floor check each have Haik's words and the log's times.
- AFTER equals BEFORE: the active plan, Balanced's lid and sleep values and its `/qh` hash; no install traces.

## Out of scope

- Any code change. A result that differs from the expected one goes to "Found outside scope" with the evidence; the
  orchestrator decides.
- Haik's own power values: never written.

## Questions for Haik

None yet.

## What is in place (2026-11-05 11:31)

Nothing of the test: uninstalled, Balanced active, the copy deleted, no process of mine (both grab loops ended by
themselves after their grabs). Left by Windows: the incomplete `NotifyIconSettings\2334163346184210159` entry (Found
outside scope 3) and its own `Notifications\Settings\velopack.HaikAsatryan.Wakeshot` key. In `<S>`: logs, reads,
grabs, crops, the `.reg` export and the scripts (the scratch folder is temporary: copy the export out to keep it).

## Decisions

Worker decisions (test design), each with its reason and when to change it.

1. **`floor-on` waits for 97% or lower, not 98%.** On the dock the battery charges again. Windows does not cut the
   percent down (80,180 of 80,290 mWh read 100% at 10:07), so a fresh 98% reads 99% again within about a minute of
   charging, and at 99% the floor of 99 is not in force when the dock is pulled. One percent is 803 mWh; the dock
   charged at about 25 W at 92% and about 7 W at 98% last night (battery report), so 97% leaves 2 to 5 minutes on AC.
   The cost: from 100% about 10 to 15 minutes on battery first (about 11 to 13 W drain last night). Change it when
   the AC part of the floor check grows past about 2 minutes (then 96%). Seen in the run: from 100% it took 6 min to
   97% (11:05:56 to 11:11:56, after 3 min on battery before), and the dock did not charge at 96 to 97% at all
   (research, "Screen session of 2026-11-05"), so 98% would have done on this laptop today; 97% stays the safe pick
   while that charge rule is not known.
2. **Haik starts the second tray (card step 8) himself, with `<S>\start-direct.cmd`** (it runs `explorer.exe
   "<exe>"`, the start the card names), not the agent. The orchestrator relays by message, so an agent start comes
   minutes before Haik looks, and a banner stays up about 5 s. The agent still quits the first tray, deletes
   `config.json` and starts the grab loop first. Change it when Haik and the agent work at the same time.
3. **The grab loops run 15 s from the tray's start, not 10 s.** Margin for a slow first banner; costs nothing.
4. **The battery drain is the first step of the floor check**, in the card's order (Settings look, typing rerun,
   floor check). Overlapping it with the Settings look would save about a minute and change the order.
5. **The worker quits the tray with a `WM_CLOSE` posted to its `Wakeshot.Loop` window**, not `taskkill /PID`: taskkill
   without `/F` did not quit it (`p6-notice-app-icon` Found outside scope 6). It is the message taskkill means to
   send, so the tray's own Quit path runs (restore, journal gone, guard stopped). Change it when taskkill works again.
6. **During the floor check a Claude Code hold is harmless**, unlike in the typing rerun: a floor step back releases
   every hold at once. So step D only asks Haik to make his other session idle before the floor starts, and goes on
   if a hold shows; the typing rerun waits for "Not holding", since a hold there would hide the work end.

## Haik's steps as sent, part 2 (2026-11-05 10:3x)

About 45 minutes; 10 to 15 of them are on battery with your own work, before section D's test.

Before you start:

- Wakeshot is not running now. The DELL dock is plugged in, the lid is open, you are signed in, Do not disturb is
  off. Change no power setting in Settings or Control Panel until the orchestrator says the session is over.
- The scripts are in this folder. Open it in File Explorer and keep it open; run a script by double-clicking it. The
  floor windows end with "Press any key to continue": read it, then press a key.
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\845942db-47c9-4611-8481-99965de4381d\scratchpad\p6-notice-app-icon\partB`
- A working Claude Code session makes Wakeshot hold. Where a step says so, keep your other session idle.

**A. The second banner (about 2 minutes).**

1. Double-click `start-direct.cmd`. It starts Wakeshot the way sign-in does: the program itself, not the Start menu
   entry. A banner comes up at the bottom right of the DELL screen within a few seconds. Do not click it. Say what
   the small picture left of "Wakeshot" in its header looks like, as in part 1, and whether its colours look right.
2. Before you click anything, press Win+N and say whether a Wakeshot notice is listed there, then press Esc.

**B. Settings look (about 2 minutes).**

3. Close Settings if it is open. Open Settings again > Personalization > Taskbar > Other system tray icons. Say
   whether Wakeshot is in the list now, and if so what picture its entry has: the Wakeshot tray icon (a half sun over
   a line, as on the taskbar), the coffee cup, or something else, and whether it looks centred. Close Settings.

**C. Typing with the lid closed (about 10 minutes).**

4. The dock is in, the lid open. Open a new Terminal window (normal, not admin): the install added the `wakeshot`
   command, and only a new window has it.
5. Hover over the Wakeshot icon and wait until it says "Not holding, the PC sleeps as usual". From now until step 8
   ends, keep your other Claude Code session idle.
6. Keep awake is Off. In Terminal type `wakeshot run -- ping -n 61 127.0.0.1`, press Enter, and close the lid right
   away. Note the clock time. Hear: the lid chime; the external screen shows the lock screen. (If it says `wakeshot`
   is not found, type `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot.exe` instead of
   `wakeshot`.)
7. Sign in on the external screen, open Notepad and keep typing on the external keyboard (any text, a key every few
   seconds) until 7 full minutes after the lid close, by the clock. See and hear: no "going to sleep" sound and no
   sleep (the external screen stays on).
8. Open the lid. Say the time of the lid close, what you heard, and whether the external screen stayed on.

**D. Battery floor grace (about 12 minutes after the battery is low enough).**

9. Pull the dock's cable (the external screen goes dark; the laptop runs on battery). On the laptop's own screen,
   double-click `floor-on.cmd`. It says the battery is too full and checks every 30 seconds. Keep using the laptop on
   its own screen (your own work is fine) until it says "done: battery floor now 99%"; from 100% that takes about 10
   to 15 minutes. A few minutes before that, let your other Claude Code session finish, and keep it idle until step
   16 ends. Then press a key. (If a hold runs when it says done, you hear the battery chime and see "Battery low: the
   PC may sleep now": expected. From now on, energy saver may dim the screen a little on battery.)
10. Right away, plug the dock in. Hover over the icon: it should say "Not holding"; if it names a hold, say which and
    go on.
11. Menu: Keep awake > 30 minutes. See within about 10 seconds: the icon shows holding; hover: "Holding awake",
    with Keep awake and its minutes left.
12. Short drop: pull the dock's cable and plug it back at once (within 2 seconds). See and hear: nothing, no chime
    and no notice; the icon keeps holding. Say what you saw and heard.
13. Wait 20 seconds. Pull the dock's cable and leave it out; note the clock. See and hear about 30 seconds after the
    pull, once: the battery chime and the notice "Battery low: the PC may sleep now"; hover: "Battery low, not
    holding: Keep awake ...". Say about how many seconds after the pull it came.
14. Plug the dock back in. See within about 10 seconds: the icon shows holding again, with no sound.
15. Close the lid. Hear: the lid chime (it locks). Wait 10 seconds. Pull the dock's cable and leave it out; note the
    clock. Leave the lid closed for 7 minutes and touch nothing. Listen (faint with the lid closed): the battery chime
    about 30 seconds after the pull, and maybe the "going to sleep" sound about 5 minutes after that. If you can see
    the dot of the ThinkPad logo on the lid, say whether and when it goes from steady to slow blinking (on many
    ThinkPads that means asleep; not checked on this model). Say what you heard and about when.
16. Open the lid. See: the lock screen. If the screen stays dark, press the power button once and say so. Sign in.
    Say which notices you see.
17. Double-click `floor-off.cmd`; it says "done: battery floor back to 30%". Menu: Keep awake > Off. Plug the dock
    in.
18. Leave Wakeshot running. Send the orchestrator your answers for A to D.

## Found outside scope

1. **`%TEMP%\velopack` left in place, not deleted.** Its dates are not the card's (made 2026-11-03 21:56:07, written
   2026-11-05 02:49:51 at BEFORE), so the card's rule leaves it. It is empty and it is `vpk pack`'s own work folder:
   its write time moved to 10:14:54.205 when `pack.ps1` ran, and `Setup.exe --silent` did not change it. Every pack
   touches it again, so deleting it gains nothing. Not a Wakeshot install trace; `e2e.ps1` rightly ignores it.
2. **Left and listed, not this card's:** under `%TEMP%\claude`, `C--Users-haika-code-wakeshot\4fa582c2-...\scratchpad\`
   `wakeshot-tray.exe.extracted.manifest` and `wakeshot-tray.exe.stdout.txt` (2026-11-03 16:09 and 16:10),
   `f7c2c08a8f04f07b\ca4fed46-...\scratchpad\velopack-issue.md` (2026-11-04 10:08), and the old
   `p6-int-haik-checks` scratch folder with Part C's evidence. Also five `%TEMP%\NotifyIconGeneratedAumid_*.png`
   (2026-11-01 to 11-04): Windows' own header pictures, not named by the card.
3. **Since the key was deleted, Wakeshot is missing from Settings > Taskbar > Other system tray icons** (this laptop;
   caused by this card's deliverable 3). At the first add after the delete (10:24:25.659) Explorer made the entry
   `2334163346184210159` again with only `IconSnapshot`, a fresh picture of the tray glyph (the Cup is gone), and no
   `ExecutablePath`, `IconGuid` or `IsPromoted`; a second tray start at 10:45 added nothing. Haik found Wakeshot
   in the list neither time. Likely cause: Explorer keeps its tray icon records in memory and writes back only
   values it changes, so a key deleted while Explorer runs comes back with only the new snapshot; and Settings lists
   only entries with an `ExecutablePath` (both **not verified**). A real user never deletes this key, so likely this
   laptop only. Likely fix, to check: an Explorer restart or a sign-out and sign-in, then a tray start, then read the
   key and look at Settings (a step that restarts Explorer or signs out runs only when no other agent runs). The
   export `<S>\NotifyIconSettings-2334163346184210159.reg` (the old Cup entry, `IsPromoted` 1) can put the old entry
   back with `reg import`, but only with Explorer closed, or Explorer may write over it (not verified). My default:
   leave it to Haik's later sign-out step, as the orchestrator said.
4. **My step 13 text was wrong, not Wakeshot.** It said the chime comes "about 30 seconds after the pull" after a
   20 s wait; AC then lasted only 18.4 s (the dock needs about 7 s to come back), so by the rule of
   `product.md` "Battery floor" the wait kept the short drop's loss and the step back came 4.7 s after the pull. A
   later step text should wait at least 40 s on AC before the long pull. No code change.
5. **The closed-lid report was lost at the lid open after the floor sleep.** 11:24:39.004 StepBack notice shown; 6 ms
   later "notice ClosedLidReport not shown: Shell_NotifyIconW failed with 1462", and no retry; Haik named no
   notices. Last night both showed together (00:47:52), so it is timing or Explorer right after a resume (not
   verified). Who sees it: anyone whose closed-lid run ends in a floor sleep; they miss the "kept running" report.
   Fix to weigh: retry a notice that failed once, after a short wait, or space two notices sent in one pass
   (`wakeshot-windows` `notify.rs`, small, with a fake test). My default: a fix card, no product decision.
6. **Typing on the external screen with the lid closed is still not shown** (finding 9, again). Input held the
   countdown off, but the evidence cannot tell typing from other input (Log 11:3x). My default: leave it, since the
   rule is "no input", not "typing", and the run shows the countdown waits while input comes; rerun only if Haik
   wants it, then 7 minutes of typing with no video.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05 09:5x: worker started, bundled with `p6-notice-app-icon` Part B. Orchestrator: Haik said yes to the key
  and leftover deletes and both reruns; clang is installed and no installer runs (deliverable 4's wait passes at once);
  no ARM64 checks here; Haik uses the DELL dock only (AC through it, no separate charger); Haik runs his own Claude
  Code session meanwhile, so each rerun step starts with "wait until it says Not holding".
- 2026-11-05 10:11: BEFORE additions (elevated, read only, `<S>\before.json`): the key
  `NotifyIconSettings\2334163346184210159` held `IconGuid` `{7DB9403E-D4B9-4165-A99E-A69B8EA56744}`, `ExecutablePath`
  `...\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe`, `IconSnapshot` 271 bytes (SHA256 `36AA3CEDE7D23C7A...`, the
  same bytes as Part C's `IconSnapshot.png`, the Cup), `IsPromoted` 1; no other Wakeshot entry. `%TEMP%`: ten
  `wakeshot-*` folders, each made and written 2026-11-04 04:16:15; `velopack` made 2026-11-03 21:56:07, written
  2026-11-05 02:49:51 (the time of `p6-notice-app-icon` Part A's e2e run), empty. Battery 100% on AC.
- 2026-11-05 10:11:29: finding 10: deleted the ten `wakeshot-*` folders (cargo test leftovers: journals, logs, 4.2 MB
  in all, every file dated 2026-11-04 04:16). Left `%TEMP%\velopack`: its dates are not the card's 2026-11-04 06:37.
- 2026-11-05 10:11:51: finding 4: `reg export` to `<S>\NotifyIconSettings-2334163346184210159.reg` (2,382 bytes),
  then `reg delete /f` at 10:11:57; the key reads gone.
- 2026-11-05 10:12:22: deliverable 4: no Visual Studio Installer process; `p6-notice-app-icon` steps 3 to 5 done
  (that card's Log: build, pack, the copy `78a8cd32-8e80-47ba-9dea-6fb189411195` "Wakeshot test notice" active,
  install at Medium, grab loop pid 23188). `vpk` touches `%TEMP%\velopack`: its write time became 10:14:54.205, the
  moment `pack.ps1` ran, and Setup did not change it (read before and after Setup at Medium); still empty.
- 2026-11-05 10:18: `floor-on.cmd` and `floor-off.cmd` in `<S>` (they run `<S>\tools\haik-power.ps1`, Windows
  PowerShell 5.1; write only the copy, by its id; refuse if it is gone or not active; `/setactive` the copy after the
  write and read the value back; log to `<S>\scripts.log`). Threshold rule from `policy/floor.rs`: the floor is the
  DC energy saver threshold when 1 to 99 (else 20) and a battery must be strictly below it, so 99 is the highest
  floor; at 99 or 100% no value puts it in force. `floor-on` sets 99 once the battery is 97% or lower (decision 1),
  else says so and checks every 30 s; `floor-off` puts back Balanced's own value (30), which nothing writes. Test at
  Medium (`<S>\tools\test-floor.cmd` via `explorer.exe`, `test-floor-medium.log`): at a test 99% and 98% it waited,
  wrote nothing, stopped by its pid after 5 s; at 97% it set 30 to 99 (read back 99); `floor-off.cmd` set 99 back to
  30 (read back 30). The copy's `/qh` then equal to Balanced's but the name lines. Test lines are in
  `<S>\scripts-test.log`, so `scripts.log` holds only Haik's runs.
- 2026-11-05 10:2x: Status blocked: Haik's part 1 is `p6-notice-app-icon` steps 6 and 7 (that card).
- 2026-11-05 10:29:13: Settings read 1 (the orchestrator's ask, after Haik found no Wakeshot in "Other system tray
  icons" while the Start menu tray ran). `HKCU\Control Panel\NotifyIconSettings` (18 entries, written 10:24:25.654)
  has one Wakeshot entry, the same id `2334163346184210159`, made again 10:24:25.659, 49 ms after the tray started,
  with **only** `IconSnapshot` (290 bytes, SHA256 `5CDAA2A20CA06971...`): no `ExecutablePath`, `IconGuid` or
  `IsPromoted`. The snapshot is a 24 px picture of the idle tray icon, a white sunrise outline (half sun over a line),
  not the Cup (`<S>\IconSnapshot-1029.png`, enlarged `IconSnapshot-1029-x12.png`). Read again at 10:31, after the
  tray quit: unchanged. That Settings lists only entries with an `ExecutablePath`: **not verified** (a likely reason
  Wakeshot is missing from the list).
- 2026-11-05 10:3x: Status blocked on Haik's part 2 (below, "Haik's steps as sent, part 2").
- 2026-11-05 10:50: A and B answered (A in `p6-notice-app-icon`: both headers show the Wakeshot icon by the grabs;
  not kept in Win+N). B, Haik: "it's again not visible in other tray" (Settings closed and opened again, the second
  tray running since 10:45:33). Settings read 2 at 10:50:12: the entry `2334163346184210159` is still as at 10:24:25.659,
  only `IconSnapshot` (the same 290 bytes); it is the only one of the 18 `NotifyIconSettings` entries with no
  `ExecutablePath`. So the second start wrote nothing to it (Found outside scope 3). The `IconSnapshot` as PNG:
  `<S>\IconSnapshot-1029.png` (the tray sunrise outline, not the Cup). C and D: Haik goes on; answers to come.
- 2026-11-05 11:03: C (typing rerun, finding 9), Haik: "C done: lid closed 10:41, chime heard, screen stayed on". The
  log puts the close at 10:52:50.256, not 10:41 (likely a misread clock; the tray started only at 10:45:33). Tray log:
  holds for Claude Code ended 10:50:56.093 (idle from then); `wakeshot run` ping hold 10:52:37.926; lid close and lock
  10:52:50.256; lid chime 10:52:52.095 on the speakers; ping ended and the hold ended 10:53:38.652; "Countdown to
  sleep starts (WorkEnded): due after 5 minutes without input" 10:53:38.719; no countdown end, no "going to sleep"
  sound, no sleep check; "Countdown to sleep cancelled: the lid opened" 11:02:38.484 (closed 9 min 48 s, 9 min after
  the work end); closed-lid report 11:02:38.485. System log: no Kernel-Power 506 or 507 from 10:44 to 11:03 (no
  Modern Standby entry, so no sleep and the screen stayed on), then 105 at 11:03:07 (AC lost: section D begins).
  Without input the countdown would have ended at about 10:58:39; it did not, so the typing held it off. **Pass.**
  (Corrected at 11:3x below: Haik did not type all along.)
- 2026-11-05 11:3x: C correction, Haik: "I was not typing whatching youtube vide :)", then "No it was tested as I
  typed for searching video", then "Take it from logs". What held the countdown off: the countdown reads only
  `GetLastInputInfo`, clamped to the last resume or unlock (`core/lid.rs` `check_countdown`, `countdown_clamp`;
  `policy/ending.rs`). No hold, pause or console change is logged from 10:53:38.7 to 11:02:38.5, so Windows saw
  input at least every 5 minutes (searching, clicks or the mouse), unless a playing video moves the input tick, which
  no doc I know of says and nothing here measured (not verified). No Kernel-Power 506 either: the copy's AC screen
  timeout is 300 s, and a video's display request keeps the screen on too, so that does not tell them apart. A
  browser request is not in the log (`powercfg /requests` was not read then; at 11:28 no display request). Verdict:
  the evidence cannot tell typing from other input, so C counts as **not shown for typing**; it does show that the
  countdown waits while input comes, as `product.md` "work end" says ("no input for 5 minutes").
- 2026-11-05 11:3x: D, Haik: "D is done as well. sound after 30 second and around 5 min when I pluged out dock. I
  think everything is working but you can check logs. on login part laptop automatically on lid open moved to sign in
  page no button click needed." Log and System log (`<S>\partB-wakeshot.log`, `<S>\partB-system-power.txt`): dock
  out 11:03:07.5; `floor-on` 11:05:56 (99%) to "done" 11:11:56 (97%; 98% from 11:08:56); the tray stepped back at once
  with nothing held 11:12:03.9 (on battery for 9 min); dock in 11:12:48.5, the step back ended. Keep awake 11:13:00.2.
  Step 12: AC lost 11:13:33.4 to 11:13:40.3 (6.9 s): "step back once the PC has been on battery for 30 s" then "wait
  ends with no step back: AC is back"; no sound, no notice. Step 13: pull 11:13:58.7 (AC had lasted 18.4 s); step
  back 11:14:03.4, 4.7 s after the pull and 30.0 s after the step 12 loss (the run rule, Found outside scope 4);
  StepBack notice 11:14:03.9; floor chime 11:14:04.4. Step 14: dock in 11:15:18.8, holding again 11:15:19.7, no
  sound. Step 15: lid close 11:16:12.5 (lock), lid chime 11:16:14.2; screen off 11:16:44.1 (506 reason 12); pull
  11:16:24.75; source lock and grace wait 11:16:24.78; at 11:16:54.80 "lock, with the step back's sound and no
  chime", lid actions and battery sleep restored, plan applied 11:16:55.22, Floor countdown started 11:16:55.23;
  floor chime 11:16:56.5; no sleep at the restore; countdown due 11:20:58.9 (300 s without input, 96%), sleep check
  passed, "going to sleep" sound 11:21:00.8, 1-minute timeouts 11:21:00.8 to 11:21:01.3; Windows slept 11:21:13.9
  (566 to session type 2). Step 16: lid open 11:24:36.4 (507, 566 reason 15), asleep 3 min 22.6 s; the 1-minute
  timeouts back 11:24:38.3; StepBack notice 11:24:39.0; the closed-lid report failed at 11:24:39.010 (Found outside
  scope 5). Step 17: `floor-off` 11:25:33 to 11:25:34, the copy's threshold 99 back to 30 (read back by the script;
  AFTER read 30 on Balanced, the copy deleted). Dock in 11:25:40.5: the step back ended and Keep awake applied again.
  "Keep awake > Off" is not in the log: Keep awake still ran at 11:28 (15 min left) until the quit. AC lost again at
  11:28:39.1 and still off at 11:30:32 (the dock out, or a drop; not this card's).
- 2026-11-05 11:29:58: `p6-notice-app-icon` step 10 (that card's Log): quit by `WM_CLOSE`, uninstall at Medium,
  Balanced active, copy deleted, AFTER equals BEFORE. `<S>\target`, `<S>\pack` and my copy of the notification
  database deleted 11:30:48. `docs/research.md`: subsection "Screen session of 2026-11-05" added at the end of
  "Integration checklist with Haik" (re-read first; "Battery floor grace checks" is no longer the last section, so
  the card's placement rule did not apply; LF kept).

## Result

**Done 2026-11-05**, with `p6-notice-app-icon` Part B in one session (10:11 to 11:31). Facts: `docs/research.md`,
"Integration checklist with Haik", subsection "Screen session of 2026-11-05".

| Item | Outcome |
|---|---|
| Finding 10, `%TEMP%` leftovers | The ten `wakeshot-*` folders of 2026-11-04 04:16 deleted. `velopack` left: its dates are not the card's, it is empty and is `vpk pack`'s own work folder (Found outside scope 1). Nothing else deleted. |
| Finding 4, Settings icon | Key exported to `<S>`, then deleted before the first tray start. Windows renewed the picture (now the tray glyph, not the Cup), but it made the entry again with only `IconSnapshot`, and Settings no longer lists Wakeshot at all (Haik, twice; Found outside scope 3; his later sign-out step). |
| Finding 9, typing rerun | No sleep, the screen stayed on, the countdown waited 9 min with the lid closed. Haik searched a video, then watched it, so typing is **not shown**; input of some kind came at least every 5 minutes (Found outside scope 6). |
| Floor grace check | Pass. A 6.9 s dock drop under the floor: nothing. A lasting pull: one chime and one notice at 30 s, counted from the first loss as `product.md` says (my step text was wrong, Found outside scope 4). Back on AC: holding, no sound. Lid closed and pull: lock, the floor chime at 30 s, no sleep at the restore, then Wakeshot's 5-minute route: sound at 11:21:00.8, Windows slept 12.6 s later, lid open woke it to the sign-in page. Open fact answered for this run: with the 30 s wait, Windows did not sleep at the restore. |

Acceptance checks:

- Leftovers and key gone, nothing else deleted: pass (the ten folders and the key; `velopack` left by the card's
  rule; Explorer made its own entry again, Found outside scope 3).
- The typing rerun and the floor check each have Haik's words and the log's times: pass (Log 11:03 and 11:3x).
- AFTER equals BEFORE: pass (11:30:32 against 10:11:06: active plan Balanced, lid 1 and 1, sleep 18000 and 18000,
  energy saver DC 30, `/qh` `C64AFDB1CFDD13CF`, Run, RunOnce, StartupApproved, Path; no install trace, data folder,
  journal or process). Differ, as expected: Windows' notification counts, the icon entry, the deleted leftovers, the
  battery (96%, on battery since 11:28:39).

Files changed: this card, `docs/tasks/p6-notice-app-icon.md`, `docs/research.md` (the subsection). No code. Scratch
only: the scripts in `<S>` and `<S>\tools`, logs, reads, grabs and crops; build folder, packs and my copy of the
notification database deleted.

Open items, for the orchestrator: Found outside scope 3 (Settings entry; Haik's sign-out step), 5 (lost closed-lid
report; fix card, default yes), 6 (typing not shown; default leave it), and `p6-notice-app-icon` Found outside scope
6 (taskkill does not quit the tray; fix card). Decisions 1 to 6 are for Haik's review.
