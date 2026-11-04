# Does Windows' own idle sleep put a closed laptop to sleep?

Phase: 3. Status: ready. Needs Haik: **yes**, about 10 minutes: pause any video, unplug the dock, double-click two
files, close the lid for 4 minutes, open it and sign in, plug the dock back. Agent: `wakeshot-worker`. Size: about
60K tokens. Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from Haik's answer of 2026-11-04: when
Wakeshot is done and the lid is closed, the laptop sleeps, not hibernates (`docs/product.md`, work end rule).
Runs after: nothing. Parallel with: code cards (their builds pause while the laptop sleeps); no other card that uses
the power settings, the screen or a sign-out.
Real laptop: **yes**: a plan copy made active, lid closed, battery. Haik's own plan is never written.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-idle-sleep-check\target` (only if a small program is needed).

## Goal

On Modern Standby, Windows gives apps no call that starts sleep (Microsoft Q&A; `SetSuspendState(FALSE)` hibernates
on this laptop, tested). The new design (`p2-6-sleep-route`) uses Windows' own idle sleep instead: after a work end
with the lid closed, Wakeshot sets the sleep timeout to 1 minute. Find out on the real laptop, before the code card
builds on it:

1. With the lid closed, lid action Do nothing, on battery, no external screen, and a sleep timeout of 1 minute, does
   Windows enter sleep (Modern Standby) by itself? How long after the last power request ends?
2. The real case: a SYSTEM power request held for the first 2 minutes after the lid closes (the agent's work), then
   released, while the 1-minute timeout is already set. How long from the release to sleep?
3. What does a desktop app see at that sleep and at the wake: `WM_POWERBROADCAST` (`PBT_APMSUSPEND`,
   `PBT_APMRESUMEAUTOMATIC`, `PBT_APMRESUMESUSPEND`, `PBT_POWERSETTINGCHANGE` for `GUID_CONSOLE_DISPLAY_STATE` and
   `GUID_LIDSWITCH_STATE_CHANGE`), with wall times? The code card needs to tell "the laptop slept" from "it did not".
4. Did the session lock, and did the wake need a sign-in?

## Read first

- `CLAUDE.md` (Test laptop, facts), `docs/product.md` (work end rule and "User settings safety" 7, both 2026-11-04).
- `docs/research.md`: "Power settings spike" (plan copy), "Lid and sleep spike".
- `docs/tasks/p3-haik-checks.md` "Steps for Haik" for the house form of step files Haik double-clicks, and
  `crates/wakeshot-windows/examples/power_check.rs` (it may already do part of this).
- `crates/wakeshot-windows/src/power/system.rs` (which power messages the tray already handles).

## Deliverables

1. Stop 1 (no laptop change): BEFORE record (active plan, the lid and sleep values of Balanced on AC and DC,
   `powercfg /requests`, the list of plans); a recorder that logs the messages of question 3 with wall times to a file
   in `%LocalAppData%\Wakeshot-idle-check`, run at Medium from Explorer (agents run elevated; `CLAUDE.md`); a start
   file and an end file. The start file makes a plan copy (`powercfg /duplicatescheme` of Balanced) with lid AC and DC
   Do nothing and sleep AC and DC 60 s, makes it active, starts the recorder and a SYSTEM request that ends 2 minutes
   after the lid closes (the recorder sees the lid message). The end file makes Balanced active again, deletes the
   copy, stops what the start file started, and reads AFTER. Write the steps for Haik into this card with exact
   clicks, set Status `blocked`, and end your turn with "ready for Haik".
2. Stop 2 (after Haik's steps, the orchestrator resumes you): read the recorder log and the System event log
   (Kernel-Power 506 and 507 for Modern Standby entry and exit, 42 and 107 for sleep and wake, 1 for a resume from
   hibernate) for the window; answer questions 1 to 4 with times; AFTER equals BEFORE; leftovers removed (folder,
   copy, processes started, by pid).
3. `docs/research.md`: a new section "Idle sleep with the lid closed" with the answers, what was measured and what was
   not.

## Steps for Haik (draft; the worker writes the final ones)

1. Pause any video or music (Chrome's video keeps the screen and the PC awake).
2. Unplug the DELL dock cable, so the laptop runs on battery with no external screen.
3. Double-click the start file.
4. Close the lid. Wait 4 minutes. Open the lid and sign in.
5. Plug the dock back in. Double-click the end file. Tell the orchestrator "done".

## Acceptance checks

- Haik's own plan (`381b4222-f694-41f0-9685-ff5bb260df2e`) is never written: BEFORE and AFTER equal, every value.
- Nothing left: no copy plan, no recorder process, no request, no folder.
- If a program is built: `cargo fmt` on it; never `RUSTFLAGS`; build folder deleted.

## Out of scope

- Product code; the code card `p2-6-sleep-route`. A run on AC with the dock (not the case that matters: a closed
  laptop in a bag is on battery).

## Questions for Haik

None.

## Found outside scope

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result
