# Integration: Haik's checklist session on the installed Wakeshot

Phase: 6. Status: ready. Needs Haik: **yes**, about 42 minutes: 27 (about 18 of hands) plus 15 with his USB-C
dock and external screen (always plugged in, Haik 2026-11-03). Agent: `wakeshot-worker`. Size: about 70K tokens.
Real laptop: **whole laptop**: an install, lid closes, locks, one hibernate, Switch user, battery; no other agent and
no cargo command runs from Part A step 2 to the end of Part C. Haik's own plan is never written: the session runs on
an active plan copy.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-haik-checks\target`.
Runs after: `p6-int-update-e2e`, `p3-haik-checks`, `p5-haik-check`. Parallel with: nothing.

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

### Part B: Haik's steps

Start plugged in, lid open, no external screen, sound on. After each block write one line to the orchestrator: what
you saw and heard, or "as expected".

**Block 1. The real exe (about 4 minutes).**
1. Press Start, type `Wakeshot`, check the entry has the Wakeshot icon, press Enter.
   See: a notice with the header "Wakeshot" that says how to keep the icon on the taskbar.
2. Click the notice. See: Settings opens at Taskbar, "Other system tray icons". Turn Wakeshot on. Close Settings.
3. See: the Wakeshot icon on the taskbar, idle. Click it. See: the menu opens next to the icon, in the taskbar's
   colours, with round corners and a small icon on each item.
4. Menu: Keep awake > 30 minutes. See: the icon changes to holding. Hover over it: the tooltip names Keep awake.

**Block 2. Lid close while holding, then the report (checklists 16 and 10; about 4 minutes).**
1. Keep awake is on. Close the lid. Hear: Wakeshot's lid chime within about a second.
2. Wait 30 seconds. Open the lid. See: the lock screen (the PC kept running; it did not sleep).
3. Sign in. See: a notice that the PC kept running about 30 seconds with the lid closed, held by Keep awake.

**Block 3. Switch user (checklist 11; about 3 minutes).**
1. Keep awake is still on. Press Ctrl+Alt+Del, choose "Switch user". See: the sign-in screen.
2. Wait 20 seconds. Sign in as yourself. See: your desktop; the icon still shows holding.
   If "Switch user" is not in the list, skip this block and say so.

**Block 4. Work ends with the lid closed: hibernate (checklist 7; about 10 minutes, 2 of hands; stay plugged in).**
1. Menu: Keep awake > Off. Open Terminal and type `wakeshot run -- ping -n 91 127.0.0.1`, then close the lid right
   away. Hear: the lid chime.
2. Leave the lid closed for 8 minutes, and touch nothing. (The hold ends after about 90 seconds; about 5 minutes
   later Wakeshot plays its "going to sleep" chime and hibernates.)
3. Open the lid. If the screen stays dark, press the power button. See: Windows resumes from hibernate (the Lenovo
   logo, then the lock screen) with your apps still open. Sign in.

**Block 5. Battery floor (checklist 8, without a dock; about 4 minutes).**
1. Unplug the charger. Run `<scratchpad>\floor-on.cmd`. If it says the battery is too full, do this block last.
2. Menu: Keep awake > 30 minutes. See within about 10 seconds: a notice that Wakeshot stepped back to protect the
   battery. Hear: the battery chime.
3. Plug the charger in. See within about 10 seconds: the icon shows holding again.
4. Run `<scratchpad>\floor-off.cmd`. Menu: Keep awake > Off.

**Block 6. With a USB-C dock, an external screen and a keyboard on it (rest of checklists 8 and 7; about 15
minutes).** Haik's dock is always plugged in (2026-11-03), so this block runs.
1. Short AC drop: docked, lid open, run `floor-on.cmd`, Keep awake > 30 minutes. Pull the dock's cable and plug it
   back within 2 seconds. Say what you saw and heard. Run `floor-off.cmd`.
2. Your own AC lid action Do nothing: Keep awake > Off; run `dock-on.cmd`; Keep awake > 30 minutes; close the lid and
   work on the external screen for a minute; pull the dock's cable. Hear after about 5 seconds: the lid chime. Open
   the lid. See: the lock screen. Sign in; Keep awake > Off; run `dock-off.cmd`.
3. Signed in on the external screen: plug the dock in, Keep awake > 30 minutes, close the lid (chime and lock, as in
   block 2), sign in on the external screen. Pull the dock's cable and plug it back within 2 seconds. See and hear:
   nothing. Pull it again and leave it out. Hear after about 5 seconds: the lid chime. Open the lid. See: the lock
   screen. Sign in, plug the dock in.
4. Not while typing: docked, lid closed, Keep awake > Off, in Terminal `wakeshot run -- ping -n 31 127.0.0.1`, then
   keep typing on the external keyboard for 6 minutes (any text in Notepad). See: no hibernate. Open the lid.

**Block 7. Finish (about 2 minutes).**
1. Menu: Keep awake > 30 minutes. Menu: Restore my power settings. See: a notice that your power settings are back;
   the icon still shows holding.
2. Menu: Quit. See: the icon goes away.
3. Tell the orchestrator "done", with anything that looked or sounded wrong.

### Part C: agent, after Haik

1. Read the tray's log for each block and record the timeline: lid close, lock and chime; the closed-lid report;
   console left (restore) and back (apply); the hold end, the countdown, the hibernate checks, `SetSuspendState`, the
   resume; the floor step back and the hold again; the dock cases; Restore my power settings; Quit and its restore.
2. Check the copy's values are its originals after Quit; Haik's plan values are unchanged.
3. Haik's plan active; delete the copy; uninstall silently; check Run, RunOnce, the Path entry and the data folder are
   gone; AFTER equals BEFORE; delete the scripts.
4. `docs/research.md`: a section "Integration checklist with Haik": each item, Haik's words, the log facts.

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

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
