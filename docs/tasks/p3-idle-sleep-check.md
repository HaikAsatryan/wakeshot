# Does Windows' own idle sleep put a closed laptop to sleep?

Phase: 3. Status: done (2026-11-04 10:50; Haik ran the steps 10:30 to 10:37; one follow-up run proposed in Result).
Needs Haik: **yes**, about 12 minutes: pause any video, unplug the dock, double-click two files, close the lid for 5
minutes, open it and sign in, plug the dock back. Agent: `wakeshot-worker`. Size: about
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

## Stop 1: BEFORE and the files

`<S>` is `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\ca4fed46-8b10-44c1-a588-0f38d878cabe\scratchpad\p3-idle-sleep-check`.

**BEFORE** (2026-11-04 10:16:56, elevated, read only; `<S>\read-state.ps1`, output in `<S>\before\summary.txt` and
every Balanced value in `<S>\before\balanced-qh.txt`, 1738 lines):

- Active plan Balanced `381b4222-f694-41f0-9685-ff5bb260df2e`; `powercfg /list` shows only Balanced.
- Balanced lid action AC 1 DC 1 (Sleep); sleep after AC 18000 DC 18000; hibernate after AC 0 DC 2147483647; display
  off AC 300 DC 180.
- `powercfg /requests`: None in every section (the Chrome requests seen by the orchestrator at 10:13 were gone).
- On AC (dock), battery 99%.

**Files.** Haik's folder `%LocalAppData%\Wakeshot-idle-check` holds `1-start.cmd`, `2-end.cmd`, `start.ps1`, `end.ps1`
and `idle-recorder.exe`. Sources: `<S>\recorder\` (the recorder crate) and `<S>\deploy\` (`start.ps1`, `end.ps1`,
`deploy.ps1`, which writes the folder). Build folder `<S>\target`.

- `1-start.cmd` runs `start.ps1` at Medium (Explorer starts it). It stops before any write if: integrity is not
  Medium; Balanced is not active; a plan named `Wakeshot idle check` or a `recorder.pid` is left. It waits up to 2
  minutes for battery (`GetSystemPowerStatus` through .NET; `Win32_Battery` can read "fully charged" on AC) and
  stops under 40% charge. Then it starts `idle-recorder.exe` in its own window, writes its pid and start time to
  `recorder.pid`, and waits for the recorder's `ready: requests set`. Only then: `powercfg /duplicatescheme` of
  Balanced, `/changename` to `Wakeshot idle check`, lid AC and DC 0, sleep AC and DC 60, `/setactive` the copy,
  read back. Any failure prints `STOP:` and points to `2-end.cmd`. Log: `start.log`.
- `idle-recorder.exe <folder> 120 40`: holds SYSTEM and EXECUTION requests until 120 s after the first lid-closed
  message (from either route), then clears them. It logs, with local wall times to the ms, to `recorder.log`:
  - window route: a hidden top-level window in the tray's style (`WS_EX_TOOLWINDOW`, `WS_OVERLAPPED`) with
    `WM_POWERBROADCAST` (every `PBT_*`) and `PBT_POWERSETTINGCHANGE` for lid, console display, session display, power
    source and active plan (`RegisterPowerSettingNotification`), and `WM_WTSSESSION_CHANGE` (lock, unlock);
  - callback route, as `power::system` does: `PowerRegisterSuspendResumeNotification` and the lid through
    `PowerSettingRegisterNotification`, both `DEVICE_NOTIFY_CALLBACK`;
  - a tick every 5 s: wall step, unbiased step, idle seconds, `SystemExecutionState`, power source and charge, held
    or released, and `GAP` when a wall step passes 15 s; the policy in force (`CallNtPowerInformation`, lid action
    and idle timeout, AC and DC) whenever it changes.
  It ends at a `stop` file in its folder, after 40 minutes, or when its window closes.
- `2-end.cmd` runs `end.ps1`: writes the `stop` file and waits 10 s for the recorder to end; if it does not, ends
  only the pid in `recorder.pid`, and only if its start time matches. Sets Balanced active only if it is not; deletes
  the copy in `copy.guid` and any plan with the copy's name. Reads AFTER into `after.txt` and every Balanced value
  into `after-balanced-qh.txt`, compares lid and sleep with BEFORE, and prints `end: ALL OK` or `end: NOT OK: ...`.
  Safe to run more than once. Log: `end.log`.
- Requests log: `<S>\requests-log.ps1`, read only, elevated, started 10:26:21 hidden, pwsh pid 13088, writes the
  non-empty `powercfg /requests` sections every 30 s to `<S>\requests.log` and ends by itself at 11:26:22.

**Choices** (mine, as principal designer):

1. The recorder is a small Rust program on the workspace's `windows` 0.62.2, outside the repo in the scratchpad, with
   the root's toolchain and CRT files copied in. Why: the same calls the code card will use, built from the cargo
   cache; this card owns no repo code. Change it when the check must be repeatable: move it to
   `crates/wakeshot-windows/examples/` through a card that owns that folder.
2. It holds SYSTEM plus EXECUTION, not SYSTEM alone: that is what Wakeshot holds for work (`core::command`,
   `Requests`). Change it when the product holds something else.
3. It logs both routes (window and callback): question 3 asks about `WM_POWERBROADCAST`, while the tray's
   `power::system` uses callbacks. The code card can then pick either with data.
4. The lid stays closed 5 minutes, not 4: release at 120 s; if Windows then waits a full 60 s timeout, or for the DC
   display off (180 s after the last input), the sleep comes near 180 s, so 5 minutes leaves 2 minutes of margin.
   Change it when the measured release-to-sleep time is known.
5. The end file runs before the dock goes back in: the copy sleeps after 60 s with no input, and the dock's AC and
   screen events would land in the log after the measurement.
6. At least 40% charge: Balanced's energy saver starts at 30% on battery (research, "Power settings spike"); what it
   does to idle sleep is not known, so the run stays above it.

**Checked in stop 1** (no power setting written):

- `cargo fmt --check`, `cargo clippy --all-targets --offline -- -D warnings` clean; `cargo test`: 5 passed (argument
  parsing, setting text, plan GUID bytes, gap rule, message names); `cargo build --release`. No `RUSTFLAGS`.
- Recorder smoke run (elevated, hidden, its own scratch folder, 10:24:57 to 10:25:05): `powercfg /requests` listed
  `idle-recorder.exe` "Wakeshot idle check: stands in for agent work" under SYSTEM and EXECUTION; both routes logged
  the current values at start (lid open, console and session display on, AC, active plan Balanced); a tick read
  exec state 0x1 and the policy in force AC and DC lid action 2, idle 18000 s. `end.ps1` in that folder: the recorder
  ended through the stop file within 1 s, no plan write (Balanced already active), AFTER equal, `end: ALL OK`;
  requests None after. Smoke folder deleted.
- `start.ps1` run elevated stops at its integrity guard before any write; its read parts (active plan, lid and sleep
  values, copy name check, screens, GUID parse) give the right answers. Plans unchanged after (`powercfg /list`).
- Not verified: `powercfg` `/duplicatescheme`, `/changename`, `/set*valueindex`, `/setactive` and `/delete` at
  Medium. The API calls behind them work at Medium (research, "Power settings spike", plan copy). If one fails,
  `start.ps1` stops at that step and `2-end.cmd` puts everything back. Not tried in stop 1: no write allowed.

## Steps for Haik

Written 2026-11-04 10:30. **Your time: about 12 minutes**; for 5 of them the lid is closed and you do nothing.

Before you start: no other card that uses the power settings, the screen or a sign-out runs (orchestrator). Laptop on
a hard desk, lid open, signed in, battery 40% or more.

1. Pause any video or music in Chrome (a playing video keeps the screen and the PC awake).
2. Unplug the DELL dock cable from the laptop. The laptop now runs on battery with no external screen.
3. Press Win+R, type `%LocalAppData%\Wakeshot-idle-check`, press Enter. In that folder double-click `1-start.cmd`.
   A second window opens (the recorder); leave both open. Wait until the first window shows
   `READY: close the lid now. Keep it closed for 5 minutes.` (about 10 seconds).
   If it shows `STOP:` instead, do not close the lid: go to step 6.
4. Close the lid at once. Do not touch the laptop for 5 minutes; leave it on the desk.
5. Open the lid. If the screen stays dark for 10 seconds, press Shift. If it is still dark 10 seconds later, press the
   power button once, briefly. Write down: did the lock screen ask you to sign in (yes or no), and did you need Shift
   or the power button. Sign in.
6. Within a minute of signing in (the test plan sleeps after 60 seconds with no input), double-click `2-end.cmd` in
   the same folder. Wait for `end: ALL OK`. If it shows `NOT OK`, write down that line.
7. Plug the DELL dock cable back in. Close the windows. Tell the orchestrator "done", with your notes from step 5 (and
   any `STOP:` or `NOT OK` line).

To stop at any point: double-click `2-end.cmd`. It stops the recorder, makes Balanced active again and deletes the
copy. Change no power setting by hand during the steps: Settings writes your real plan too.

**Orchestrator:** the requests log ends at 11:26. If Haik starts after 11:05, tell the worker at resume; the recorder's
exec state ticks still show whether another process held a SYSTEM request.

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
- 2026-11-04 10:16, worker: started stop 1 (BEFORE, recorder, start and end files, steps). No power setting changed.
- 10:17: BEFORE read (Balanced active, lid 1/1, sleep 18000/18000, requests None, on AC 99%).
- 10:22: recorder built in `<S>\target` (fmt, clippy, 5 tests clean). Needed `Win32_Graphics_Gdi` for the window
  class.
- 10:25: smoke runs passed (recorder requests listed and cleared; `end.ps1` on a clean state writes nothing and says
  ALL OK; `start.ps1` elevated stops at its guard). `start.ps1` now reads battery through `GetSystemPowerStatus`.
- 10:26: requests log started (pwsh pid 13088, ends 11:26:22). Step files in `%LocalAppData%\Wakeshot-idle-check`.
- 10:30: steps for Haik written; Status `blocked` until Haik's run. Stop 2 next: read `recorder.log`, `start.log`,
  `end.log`, `after.txt`, `<S>\requests.log` and the System log (Kernel-Power 506, 507, 42, 107, 1, 105; Power-
  Troubleshooter 1) for the run; AFTER elevated into `<S>\after` and compared with `<S>\before`; end pid 13088 if it
  still runs; delete the folder and `<S>\target`; research section.
- 2026-11-04 10:40, worker: stop 2 started. Haik ran the steps before 10:38 ("done and everything kind of ok"); his
  step 5 notes are still to come.
- 10:41: logs read. `start.log` and `end.log` clean (`end: ALL OK` 10:37:30); the recorder slept 10:33:14.9 to
  10:37:01.9; System log 506 Idle Timeout and 507 Lid, no 42, 107 or 1. Requests log stopped by pid 13088. AFTER
  (elevated, 10:40:35) equals BEFORE on all 1738 `powercfg /qh` lines; requests None. Run logs copied to `<S>\run\`;
  `%LocalAppData%\Wakeshot-idle-check` and `<S>\target` deleted (the exe kept in `<S>\deploy\` for a rerun).
- 10:4x: Haik's step 5 notes (through the orchestrator): "yes it asked to sign in, screen came on by itself".
- 10:50: research section written, Result filled, Status done. One follow-up run proposed (Result, "Open").

## Result

Stop 1 done 2026-11-04 10:30 (no power setting written); Haik's run 10:30 to 10:37; stop 2 done 10:50. The end file
finished cleanly (`end: ALL OK`, 10:37:30). Full timeline and sources: `docs/research.md`, "Idle sleep with the lid
closed".

**Answers.**

1. Yes. Lid closed and Do nothing, battery, no external screen, sleep after 60 s: Windows entered Modern Standby by
   itself (Kernel-Power 506 "Idle Timeout" 10:33:14.842) and slept (`PBT_APMSUSPEND` 10:33:14.881). It was a sleep
   (507 `SleepEntered=true`, 224 s, DRIPS 93%), not a hibernate (no 42, 107 or 1).
2. Release to sleep: 55.4 s (released 10:32:19.482, 120.2 s after the lid closed). The sleep came at the DC display
   off (180 s after the last input), not at the release, although the 60 s timeout had passed 64 s before. So the
   display timer most likely set the time; whether Windows also restarts the 60 s timer at the release, one run
   cannot tell (that would have fired 4.6 s later).
3. The callback route (`PowerRegisterSuspendResumeNotification`, as `power::system`) got `PBT_APMSUSPEND` at the
   sleep and `PBT_APMRESUMEAUTOMATIC` then `PBT_APMRESUMESUSPEND` at the wake (10:37:01.911, 2.6 s after 507). The
   window got `PBT_POWERSETTINGCHANGE` for lid and both displays (dimmed 10:32:59.8, off 10:33:14.8, on 10:37:01.96)
   but no `PBT_APM*` message at all; a Microsoft Q&A answer says a window needs `RegisterSuspendResumeNotification`
   for them on Modern Standby. After the wake the tick showed wall 229.4 s against unbiased 2.4 s.
4. Yes. The session was locked at the wake (`WTS_SESSION_LOCK` delivered 10:37:01.990, after the resume); Haik: "yes
   it asked to sign in, screen came on by itself": opening the lid woke it, no key or power button needed. Unlock
   10:37:03.2.

**For `p2-6-sleep-route`:** "slept" is `Event::Suspending` from the callback route, confirmed by the next tick's gap
(wall step far above unbiased step). Not the window's `WM_POWERBROADCAST` (none came), not display off. "Did not
sleep" is a deadline with no `Event::Suspending`; release plus 60 s is too short while the display is still on.

**Acceptance checks.**

- Haik's plan never written: only the copy `810c1627-...` was written (start.log; UserModePowerService 12 shows only
  the two switches). AFTER (elevated, 10:40:35) equals BEFORE on all 1738 `powercfg /qh` lines of Balanced; active
  Balanced; lid 1/1, sleep 18000/18000. **Pass.**
- Nothing left: copy deleted (`powercfg /list` shows only Balanced), recorder pid 40348 ended through the stop file,
  requests None, requests logger pid 13088 stopped by its pid, `%LocalAppData%\Wakeshot-idle-check` deleted. **Pass.**
- Program built: `cargo fmt --check`, clippy `-D warnings` clean, 5 tests pass, no `RUSTFLAGS`, `<S>\target`
  deleted. **Pass.**

**Files.** This card; `docs/research.md` (new section "Idle sleep with the lid closed"). Scratch only: `<S>\recorder\`,
`<S>\deploy\` (with the release exe, for a rerun), `<S>\run\` (run logs), `<S>\before\`, `<S>\after\`,
`<S>\requests.log`.

**Open: one follow-up run is worth 15 minutes of Haik's time before `p2-6-sleep-route` builds.** Not measured: (1) the
common case, work longer than the display timeout, so the display is already off at the release; (2) a display
timeout longer than the sleep timeout, or Never. Proposal: the same files from `<S>\deploy\` with (A) a hold of 300 s
(display off at 180 s while held; lid closed 7 minutes) and (B) a copy with DC display off 600 s and a hold of 120 s
(lid closed 6 minutes): B tells whether the 60 s timer restarts at the release or the sleep waits for the display.
Lean alternative for the code card, untested: set DC display off to 60 s together with sleep after, so both readings
give a sleep about 60 s after the release.
