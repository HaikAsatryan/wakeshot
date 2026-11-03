# Prove safe behavior with the lid closed

Phase: 0. Status: done (2026-11-03; AFTER line written by the orchestrator). Needs Haik: done (his morning steps).
Runs after: `p0-spike-power-settings`. Parallel with: `p0-spike-tray`, `p0-distribution-facts`.

## Goal

Find, with a small Rust program, how Wakeshot keeps a closed laptop safe and puts it to sleep when work ends.

## Read first

- `CLAUDE.md`
- `docs/research.md`: "Windows power facts" and what `p0-spike-power-settings` added
- `docs/research/rivals-nightcrew-lidguard.md`: lid closed sections
- `docs/research/rivals-others.md`: traps list
- `docs/product.md`: "Power behavior"

## Questions to answer

1. Work ends with the lid closed (lid action Do nothing on the test setup): which works on this Modern Standby
   laptop: sleep through `SetSuspendState` (may fail as "not supported"), screen off plus releasing requests, or
   hibernate? What if hibernate is off? Detect only; do not turn hibernate off on Haik's laptop.
2. Wake timers: does a waitable timer with resume wake this laptop from standby, on AC and on battery? Which "allow
   wake timers" setting matters?
3. Lid notifications (`GUID_LIDSWITCH_STATE_CHANGE`): timing. Does `LockWorkStation` work right at lid close? Is a
   short sound played at lid close audible before the screen turns off?
4. Docked use: count active external displays without admin.
5. Heat: read a temperature without admin. Which API, what values, is it meaningful?
6. Resume: after a manual sleep and resume, is a held request still honored? Does taking it again fix it (PowerToys
   Awake bug)?

## Deliverables

- `spikes/lid-sleep/`: standalone Cargo project with a README.
- Findings in `docs/research.md` with how each was checked.
- An "Implications for the design" list in Result.

## Acceptance checks

- Every question has a tested answer, or "could not test" with the reason.
- Haik's original power values restored and verified (`powercfg /q` before and after, in the Log).

## Out of scope

Product code. Tray.

## Questions for Haik

None. Nothing here needs a product decision yet.

## Steps for Haik (morning)

Moved to `docs/phases/00-morning-steps.md` (blocks D, E, F, H, I, J and K), the one list for all three spike cards.
Since `p0-morning-trim` the optional G and I3 are dropped and the order is D, E, F, H, then C and I on battery. Haik
runs his steps from `spikes\morning-shell.cmd`, which the orchestrator opens through Explorer, so the tools run with
the normal Medium token. Changes from the steps that were here (from `docs/reviews/p0-architecture.md`
"Tests to add" and `docs/reviews/p0-spike-safety.md`): D is now dock, then undock and unplug the charger with the lid
closed (`lid --end none --no-lock --max-minutes 8`); block I (the architecture review's I2) is a 22 minute closed hold
on battery (`lid --work 1200 --max-minutes 30`); every closed-lid run has `--max-minutes` and the new 85 C heat
limit.

### Moved to agent (card `p0-morning-trim`, 2026-11-03; safety fixes from `docs/reviews/p0-morning-trim-safety.md`)

Haik keeps D4 to D9, E, F, H and I (new order: T10, D, E, F, H, C, I), each ending with `checkpower`, plus the sound
notes. Agents do the rest. Nothing of this card needs the screen, so its log reads may run alongside the power or
tray card's agent steps, but never during a Haik block.

Before Haik starts (step 0 of the list):

1. The BEFORE line, values read live by `powercfg /q`.
2. Old C1, C2 and D1: Haik closes media apps once; the orchestrator stops the keep-awake by PID with the guarded line,
   checks `powercfg /requests` (DISPLAY, SYSTEM, EXECUTION `None.`), runs `lid-sleep-spike.exe limited read` for the
   `SystemExecutionState` line, and starts the 30 s requests log (step 0).
3. Old C1, the volume, per device by endpoint ID (read through `IAudioEndpointVolume`; a write was not tried): the
   laptop Speakers 67% and the DELL U2723QE 24% at 09:50, neither muted. The lid spike plays `SystemNotification`
   with `SND_SYSTEM` on the default device: the DELL while it is connected (D5), the Speakers in E and I. Change
   nothing unless the Speakers are muted or under 50%; then set 50% and put the old value back at the end. Not
   checked: the System Sounds level in the volume mixer, which also counts.

After Haik's Final check (no screen needed):

4. Read the logs by the guide below. What the screen showed at open in E and I comes from the `locked` field in the
   ticks and the `display` and `session-display` events, not from Haik's notes. F's "what the screen did" comes from
   the RESULT lines and gaps. D's sound: Haik says whether it came from the DELL, the laptop, or nothing.
5. Read the requests log for the minutes of D, E, H and I: any DISPLAY, SYSTEM or EXECUTION entry other than the lid
   spike's own means that block was spoiled; only that block is redone.
6. K3, the AFTER line: last of all, after the power and tray cards' end checks, with values read live by
   `powercfg /q`.

Dropped: D2 (step 0 checks the same, and the tool refuses a leftover copy or journal), D3 and the heat value from I
(no heat guard, decision 2), "did the DELL stay on" and the `displays` count (no rule counts screens any more), G and
I3 (wake timers: no consumer). So question 2 stays "could not test: no consumer after Haik's answers", and question
5's closed-lid heat is read only as a test-safety check. The hand heat check in D8, E6 and I4 stays for safety. The
`displays` field still shows in the ticks; it is not needed for a decision.

How to read the logs (for the next worker): each run ends with `RESULT:` lines, the `gaps:` line, the Kernel-Power
and Power-Troubleshooter events of the run, and `after` lines with the plan values (compare with `before`). Map:
question 1 = E, I (`--end release`: does the PC stop after work ends?), F1, F2, F3; question 2 = G and I3 (check 507
reasons too: the SoC may wake while our process stays paused); question 3 = D, E, I (time from the lid callback to
the `display` and `session-display` callbacks, the PlaySound and LockWorkStation lines, the `locked` field in ticks,
and Haik's notes); question 4 = the `displays` field in D and E ticks while closed; question 5 = the `heat` field in
D, E and I ticks while closed (and Haik's heat by hand); question 6 = H. In E and I the ticks also show whether the
held request kept the closed laptop running during work (first RESULT line).

New in D: the `displays` and `AC` fields per tick, any GAP and when it started against the DELL unplug (about 30 s
after close) and the charger unplug (about 90 s after close), and the Kernel-Power 105, 506 and 507 lines. New in I:
a GAP that starts about 900 s after the last key press (Haik's 600 s battery sleep plus 5 minutes) means the battery
rule also stops a closed laptop. New in H (from the architecture review, item 3): note when the `locked` field turns
true against the `display` off callback after resume; it answers whether Windows locks by itself at the screen-off
standby entry while a request is held. Any run with `RESULT: work ended early at the heat limit` hit 85 C.

## Found outside scope

- `p0-spike-power-settings` step C says "The orchestrator stops its keep-awake process (PID 16788)". The process now
  is PID 11284 (seen in `Get-Process`, started 02:26:43). Not edited, as told. (Moot since 03:52: the steps now live
  in `docs/phases/00-morning-steps.md`, which names 11284.)
- `docs/research.md` lists Nightcrew's "Clock that ignores sleep time (`QueryUnbiasedInterruptTime`)" as worth
  copying. Tonight interrupt time and unbiased time were equal after 9.5 hours with several Modern Standby sessions, so
  that clock may count Modern Standby as awake time on this laptop. Morning gaps settled it: unbiased time skips the
  suspended time; the night hold kept the PC from suspending. So the Nightcrew line stays right.
- The lid spike's second RESULT line in E says "no standby" while the System log has 506 Idle Timeout: the spike
  judges standby by gaps only. Not fixed (log reads only).
- Step F3 (screen off) runs only if F1 "did not sleep". F1 hibernated instead (TargetState 5), so the Modern Standby
  sleep path for the end of work was never tested by an app call.
- The thermal zone read 87 C once (5 s samples) after the F1 resume, above the 85 C limit the lid and heat modes use;
  `suspend` mode has no heat limit. A read right after one H resume gave 7.1 C.
- `docs/phases/00-morning-steps.md` block E says "on AC"; with one USB-C cable for power and the DELL, E can only run on
  battery when the DELL is out.
- `docs/plan.md` Status still says `p0-spike-power-settings` is running and this card is next.
- A Lenovo scheduled task `LvfGuiRemoval` holds a wake timer for 17:17 today (`powercfg /waketimers`). It may wake
  the laptop from standby at that time.

## Log

- 2026-11-03: card written.
- 2026-11-03 02:31: worker started. Haik away, lid open, on AC. Orchestrator holds SYSTEM_REQUIRED (pwsh PID 11284)
  until 13:26, so idle sleep, real sleep and request tests move to the morning steps. Must not sleep the laptop tonight.
- 2026-11-03 02:31 BEFORE (elevated shell, reads only: `powercfg /getactivescheme`, `/list`,
  `/qh SCHEME_CURRENT SUB_BUTTONS LIDACTION`, `/q SCHEME_CURRENT SUB_SLEEP STANDBYIDLE`, `/q ... SUB_VIDEO VIDEOIDLE`,
  `/qh ... SUB_SLEEP RTCWAKE`): Balanced `381b4222-...` active and the only plan. LIDACTION AC 0x1, DC 0x1.
  STANDBYIDLE AC 0x2a30, DC 0x258. VIDEOIDLE AC 0x12c (300), DC 0xb4 (180). RTCWAKE AC 0x0, DC 0x0 (Disable).
  `powercfg /a`: S0 Low Power Idle Network Connected, Hibernate, Fast Startup; no S1 to S3, no Hybrid Sleep.
- 2026-11-03 02:33: System log (7 days): Kernel-Power 187 at 2026-11-01 23:46:42, caller `winlogon.exe`,
  SystemAction 3 (hibernate), then 42 "Application API", 107, Kernel-Boot type 0x1: a hibernate and resume worked.
  42 "Hibernate from Sleep - Standby Battery Budget Exceeded" on 2026-11-02 08:48. 506/507 "Reason: Lid" on
  2026-11-02 22:57.
- 2026-11-03 02:42: `spikes/lid-sleep/` builds; fmt, clippy `-D warnings` clean, tests pass.
- 2026-11-03 02:44: `limited read` (Medium token). Docked now: internal `N160JCA-GT1` (INTERNAL) plus DELL U2723QE
  (DISPLAYPORT_EXTERNAL); WMI agrees. Lid open, display off, session locked. `SystemExecutionState` 0x1 (the
  orchestrator's hold). `LastSleepTime` and `LastWakeTime` 0. MSAcpi_ThermalZoneTemperature: Access denied.
  SeShutdownPrivilege could not be enabled (OpenProcessToken denied) under `limited`.
- 2026-11-03 02:45: `limited heat` with 8 threads for 60 s: `\_TZ.THM0` 41 to 47 C idle, 67 C after 60 s of load,
  51 C 8 s after. Throttle reasons 0, passive limit 100% throughout. Log `out/heat-limited-20261103-024517.log`.
- 2026-11-03 02:48: `read` started by Explorer itself (a `.cmd` in the scratchpad opened with `explorer.exe`):
  Medium token, SeShutdownPrivilege enables (last error 0). So the `limited` failure is an artifact of parent
  spoofing. Log `out/read-explorer.log`.
- 2026-11-03 02:49: `limited timer-check --after 25`: `SetWaitableTimerEx` with resume ok, last error 0, with
  RTCWAKE Disable on both. Fired at 25.03 s. `powercfg /waketimers` needs admin; elevated it listed the timer with
  our reason, plus a Lenovo task `LvfGuiRemoval` that asks to wake at 17:17.
- 2026-11-03 02:50 to 02:56: dry runs, all with the Medium token and all restored (copy deleted, Balanced active,
  values equal BEFORE in each run's `after` lines): `lid --simulate-close-after 5 --work 10 --no-lock --no-sound`
  (re-check saw the real lid open and skipped the end method), `resume --idle 0 --simulate-resume-after 5 --retake`,
  `wake --after 15 --wake-timers 1`, `suspend --method sleep --dry`.
- 2026-11-03 02:56: kill test: `limited lid` killed by PID while its copy was active. The copy stayed active with
  LIDACTION 0/0 until `limited restore`, which switched back, deleted it and removed the journal.
- 2026-11-03 02:57: Ctrl+C test: `lid --allow-elevated` in its own console, Ctrl+C sent with
  `AttachConsole` + `GenerateConsoleCtrlEvent`. The handler stopped the loop and restored within 100 ms. Window close
  (CTRL_CLOSE_EVENT) not tested.
- 2026-11-03 02:59: added `GUID_SESSION_DISPLAY_STATUS` (the docs say user-mode apps should use it); it registers
  and fires at once with the Medium token.
- 2026-11-03 03:00: SetWaitableTimer doc: since Windows 8 a relative due time does not count down while the PC
  sleeps. Changed the spike to an absolute UTC due time with `SetWaitableTimer(..., fResume TRUE)`. Re-run of
  `limited timer-check --after 20`: ok, last error 0, fired at 20.04 s; elevated `powercfg /waketimers` listed it.
- 2026-11-03 03:02: findings in `docs/research.md` ("Lid and sleep spike"). Morning steps written above.
- 2026-11-03 03:04 AFTER (elevated shell, same commands as BEFORE): Balanced `381b4222-...` active and the only plan.
  LIDACTION AC 0x1, DC 0x1. STANDBYIDLE AC 0x2a30, DC 0x258. VIDEOIDLE AC 0x12c, DC 0xb4. RTCWAKE AC 0x0, DC 0x0.
  Matches BEFORE. No `out/journal.txt`, no spike process left, orchestrator hold PID 11284 still running.
- 2026-11-03 03:05: status blocked on Haik's morning steps D to J.
- 2026-11-03 03:53: card `p0-spike-safety-fixes` moved the morning steps to `docs/phases/00-morning-steps.md` (new D
  and I2 from the architecture review, G and I3 optional at the end) and changed the spike: plan copy refused while
  the power spike's journal exists or a spike copy is active, restore falls back to Balanced when the plan before is
  gone, `--heat-limit` (default 85 C) for `lid` and `heat`, ctrl handler comment fixed. Details in that card.
- 2026-11-03 04:30: round 2 of `p0-spike-safety-fixes`: the 22 minute run is block I of the list (the architecture
  review's I2); the list now has a stop-partway rule and a keep-awake handshake at C2 and D1. No code change here.
  Closing the morning shell's Windows Terminal window during `lid` restored 0.1 s after the close event and kept the
  whole log (details in `docs/research.md`, "Spike launch and stop checks").
- 2026-11-03 10:18: step 0 worker started (morning list step 0.2, items 1 to 3 only). Reads only from the elevated
  session; no setting written, no power request held.
- 2026-11-03 10:18 BEFORE (elevated, reads only: `powercfg /list`, `/getactivescheme`, `/qh SCHEME_CURRENT <sub>
  <setting>` per row, registry reads; script `s0-before.ps1` in the step 0 worker's scratchpad
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`):
  on AC (Online), battery 100%. Balanced `381b4222-...` active and the only plan. LIDACTION AC 0x1, DC 0x1.
  STANDBYIDLE AC 0x2a30, DC 0x258. VIDEOIDLE AC 0x12c, DC 0xb4. RTCWAKE AC 0x0, DC 0x0. ESBATTTHRESHOLD AC 0x0, DC
  0x1e. CONSOLELOCK AC 0x1, DC 0x1. `DelayLockInterval` 0. `ActiveOverlayAcPowerScheme` `ded574b5-...`,
  `ActiveOverlayDcPowerScheme` absent. Both journals absent, no spike process. Same as the 02:31 BEFORE and the list's
  table.
- 2026-11-03 10:21 volume (step 0 item 3; reads only through `IAudioEndpointVolume` by endpoint ID, script
  `s0-volume-read.ps1` in the same scratchpad): default playback device for all three roles is
  `{0.0.0.00000000}.{ca5b6419-0bd3-4065-b7b7-0b7fc5450811}` "4 - DELL U2723QE" (AMD High Definition Audio Device),
  24.0%, not muted. Laptop Speakers `{0.0.0.00000000}.{fb152119-7d6e-4352-a9bf-882ae54afa03}` (Realtek(R) Audio),
  67.3%, not muted. Speakers not muted and over 50%, so nothing written; no old value to put back. System Sounds level
  in the mixer: not checked. Battery 100%, on AC.
- 2026-11-03 12:30: log-read worker started (morning list step 4 and 5). Reads of `out/` logs and the requests log
  only; no spike run, no setting change, no screen use. AFTER line (old K3) skipped until the orchestrator asks.
  Also read, all read-only: the System event log for 10:36 to 10:41 (`Get-WinEvent`, for F's sleep states), the
  sound scheme in `HKCU\AppEvents\Schemes`, and the power card's C log
  `spikes/power-settings/out/hold-20261103-112124.log`.
- 2026-11-03 12:34 D (`out/lid-20261103-103006.log`, AC, DELL in): close 10:31:10.189, spike reacted 561 ms later,
  PlaySound true (35 ms). At close the internal path dropped (int 1 to 0, ext 1). Cable out 10:31:43.8 (33 s after
  close, AC false), back 10:33:09.9; ext stayed 1 until the lid opened 10:33:53. No display callback, no lock, no
  506/507, only Kernel-Power 105 twice. Gaps none, ticks about 1 s throughout. RESULT: kept running with the lid
  closed. Restored, `after` equals `before`. Heat 40 to 52 C.
- 2026-11-03 12:34 F (`out/suspend-20261103-103649.log` sleep, `out/suspend-20261103-103853.log` hibernate; F3 not
  run because F1 slept): both stopped the PC 0 s after the call (gaps 67 s and 71 s, unbiased 7.4 s and 7.8 s),
  `SetSuspendState` returned true only after resume. System log: both are hibernate. 187 SystemAction 2 (sleep) and 3
  (hibernate), then 42 and 107 with TargetState 5 (S4), Power-Troubleshooter HiberWriteDuration 7950 and 8156 ms,
  Kernel-Boot 27 after each, Kernel-General 1 clock jump 59 s and 64 s. After the resume: 506 "Sleep, Hibernate, or
  Shutdown", 507 Power Button 1 s later. Session locked at resume. Heat 73 to 87 C for 30 s after each resume.
- 2026-11-03 12:34 H run 1 (`out/resume-20261103-104117.log`): no suspend at all, so Start > Power > Sleep did not
  happen or did not register. Display off three times at the copy's 60 s timeout (506 Idle Timeout), the request
  held, no gaps; `locked` turned true within 1 s of each display-off callback. Ended by Ctrl+C at 10:57:11.
- 2026-11-03 12:34 H run 2 (`out/resume-20261103-105721.log`): Start > Sleep 10:57:44 (506 "Screen Off Request",
  PBT_APMSUSPEND 0.13 s later), power button 10:58:23. Hold phase: display off 10:59:25.899, PBT_APMSUSPEND 17 ms
  later, gap 319 s: REQUEST NOT HONORED. `exec-state` was 0x0 in all 61 hold ticks (0x1 before the sleep). Control
  phase slept (valid).
- 2026-11-03 12:34 H4 retake (`out/resume-20261103-120914.log`, AC, DELL in): Start > Sleep 12:09:29, resume
  12:10:14, retake 12:10:16. Hold phase: display off 12:11:21 (506 Idle Timeout), locked 0.6 s later, no suspend for
  3 min until the release at 12:14:16: request honored. `exec-state` 0x1 in all 185 hold ticks. Control phase: the PC
  suspended at once on release (display already off), and again 67 s after the last input. Valid.
- 2026-11-03 12:34 E (`out/lid-20261103-111112.log`): started on AC with the DELL in; cable out 11:11:47.3, close
  11:11:51.8 (4.3 s later), so the closed part ran on battery with no external screen. Reacted 536 ms after the
  callback, PlaySound true (11 ms). Lid closed with no external screen: int stayed 1, display stayed on. Release at
  11:12:52. Display dimmed 161 s and off 176 s after close (506 Idle Timeout 11:14:48.351); `locked` true at the next
  tick. No PBT_APMSUSPEND and no gap until the lid opened at 11:19:11 (507 Lid); the idle count was then 455 s, below
  the 600 s DC sleep timeout. The RESULT line "no standby" only means no gap: Windows did enter Modern Standby.
- 2026-11-03 12:34 I (`out/lid-20261103-114033.log`, battery 96%, DELL out): close 11:40:43.449, reacted 370 ms later,
  PlaySound true (7 ms), LockWorkStation Ok 2 ms later, `locked` at the next tick (1.06 s); one tick read
  `QueryDisplayConfig` rc 31 during the lock. Display off 31.4 s after close (506 Idle Timeout). Ticks ran until
  PBT_APMSUSPEND at 11:55:56, 916 s after the last input (600 s battery sleep plus 316 s), with the request held and
  the lid closed. Gap 514 s (unbiased 1.3 s) until the lid opened 12:04:28 (507 Lid). Work end was due during the
  gap, so no end method ran. Heat 35 to 53 C.
- 2026-11-03 12:34 C cross-check (power card's log, read only): sleep.dc 60 s on a copy, request held, battery: display
  off 11:24:25, PBT_APMSUSPEND 11:27:44, about 380 s after the last input (60 s plus 320 s). Same rule as I.
- 2026-11-03 12:34 Requests log (178 samples, 10:22:30 to 12:24:06): no DISPLAY, SYSTEM or EXECUTION entry other than
  the spikes' own during D, E, H or I. Only `[DRIVER] Legacy Kernel Caller` under SYSTEM at 12:15:09 (H4 control
  phase, our request already released, just after a resume) and under PERFBOOST after several resumes. No block is
  spoiled. Sampling stops while the PC is suspended (gaps match every suspend).
- 2026-11-03 12:34 Sound: sound scheme `SystemNotification` = `Windows Background.wav`; the toast sound
  (`Notification.Default`) is `Windows Notify System Generic.wav`; device remove is `Windows Hardware Remove.wav`.
  The DELL U2723QE has no built-in speakers (web: Macworld review, Dell community staff post).
- 2026-11-03 12:50: every morning log (D, F1, F2, H run 1, H run 2, E, I, H4): its 13 `after` lines equal its 13
  `before` lines (script compare). `restore` at 12:23 found no journal; no `out/journal.txt` now. Step 0 read at 10:22:
  `SystemExecutionState` 0x0. Findings written to `docs/research.md`, "Lid and sleep spike", "Morning results", and
  the "morning" notes in that section replaced.
- 2026-11-03 12:55: Result written. Status blocked: only the AFTER line (old K3) is left, on the orchestrator's word.

- 2026-11-03 13:08 (orchestrator): AFTER (old K3), live reads after all agent steps: only Balanced, active;
  LIDACTION 1/1, STANDBYIDLE 10800/600, VIDEOIDLE 300/180, RTCWAKE 0/0, ESBATTTHRESHOLD 0/30, CONSOLELOCK 1/1;
  DelayLockInterval 0; AC overlay ded574b5-45a0-4f42-8737-46345c09c238, DC overlay absent; `powercfg /requests` all
  None; no journals; no spike, holder, keep-awake or requests-log process. Matches the BEFORE line. Morning shell
  closed.

## Result

Status: done. The AFTER line (old K3) was written by the orchestrator at 13:08 (Log).

What was done:

- Night (02:31 to 04:30): `spikes/lid-sleep/`, a Rust spike with modes `read`, `heat`, `timer-check`, `lid`,
  `suspend`, `wake`, `resume`, `restore` and `limited`. Test modes never write Haik's plan: they use a plan copy with
  a journal, print plain `RESULT:` lines and dump the Kernel-Power events of the run. Dry runs, a kill test and a
  Ctrl+C test all left the plan as it was.
- Step 0 (10:18 to 10:21): BEFORE line from live reads (Log, 10:18). Volume: Speakers 67.3%, DELL 24.0% (default),
  neither muted; nothing changed.
- Morning (12:30 to 12:50): read Haik's D, F, H, E and I logs, the 30 s requests log, the System log for F, the sound
  scheme and the power card's C log. Reads only. Findings: `docs/research.md`, "Lid and sleep spike", "Morning
  results".

Answers:

1. Work ends with the lid closed. `SetSuspendState(FALSE)` and `(TRUE)` both hibernate this laptop (S4; normal
   user; tested lid open on AC; the call returns after resume). Release only: Windows enters Modern Standby at the
   display timeout, but apps keep running until the sleep timeout; in E nothing paused for 263 s after the 506 (lid
   opened at 455 s idle, below 600 s). Screen-off method: could not test (F3 runs only if F1 does not sleep, and F1
   hibernated). Hibernate off: could not test, by rule.
2. Wake timers: could not test. The morning trim dropped G and I3: no design item uses wake timers. Night facts stay.
3. Lid notifications: the callback arrives at once. A close with Do nothing is no standby entry and does not turn the
   display off. `LockWorkStation` at the close works (locked within 1 s). Without it Windows does not lock at the
   close; it locks within 1 s of the display-off standby entry. Sound: `PlaySound` returns true, but our sound was
   most likely never heard (item 6 below).
4. Docked: when docked, the internal path drops at the close. After an undock with the lid closed the count stays
   stale until the lid opens. No rule needs it now.
5. Heat: 35 to 60 C closed on a desk. One reading of 87 C right after a hibernate resume (lid open).
6. Resume: a held request is not honored after a manual sleep (one run: the PC suspended 17 ms after the next display
   off; `SystemExecutionState` 0x0). A re-take fixes it (one run: 3 minutes with the display off and no suspend). Both
   on AC.

Part 3 items of `docs/reviews/p0-close-proposals.md`:

1. Battery sleep timer: confirmed. I: the PC suspended 916 s after the last input with the request held and the lid
   closed (600 s plus 316 s). C (power card's log): about 380 s (60 s plus 320 s). So Wakeshot raises the battery
   sleep timer while it holds on battery, lid open or closed.
2. Hibernate off or failing: F2 hibernate works for a normal user. F1 sleep also hibernates here, so
   `SetSuspendState(FALSE)` is no Modern Standby sleep and no fallback when hibernate is off (not tested). E: a closed
   lid with no request reaches Modern Standby at the display timeout but stays awake (apps run) until the sleep
   timeout, which E did not reach. So the release-only fallback leaves a closed laptop awake until the user's sleep
   timeout (3 h on AC here). Method for line 59: `SetSuspendState`, which gives hibernate on this laptop. Whether the
   fallback needs another call is for the delta review; the screen-off call is untested.
3. Idle lock: Windows locks by itself within 1 s of the display-off standby entry while a request is held (H run 1,
   H4), and also with none (E, H control phases). Seen with Haik's CONSOLELOCK 1/1 and `DelayLockInterval` 0. So, as
   the item says: drop the idle part, keep only the lid-close lock. Other sign-in settings were not tested.
4. Plan copy or in place: no input from this card (step B is in the power settings card).
5. Clock across standby: unbiased time skipped the suspended time in every gap (for example I: 514 s wall, 1.3 s
   unbiased). `architecture.md:36` keeps its clock: no edit.
6. Sound at lid close: not shown to play. In D the default device was the DELL, which has no speakers. The E sound
   was most likely Windows' device remove sound (the DELL cable came out 4.3 s before the close; in D it came out 33 s
   after). In I nothing was unplugged and Haik heard nothing. So the delta review proposes another cue.
7. Lock at close, re-take after resume: I: the lock at close works. E: Windows does not lock at the close with Do
   nothing (176 s unlocked), only at the display-off standby entry. H: request NOT honored after a manual sleep
   without a re-take, honored with one. A Windows fact, so line 49 stays. No edit.
8. Letting the screen turn off: work did not pause with the screen off or the lid closed while the request held (D,
   E, I up to 912 s, H run 1, H4); the 1 s timer fired about every 1.5 s. Pauses came only from the battery rule (item
   1) and the lost request after a manual sleep (item 7). Confirmed: no edit.

Implications for the design:

- Everything the lid logic reads works as a normal user with no window: lid state, console and session display
  state, power source, battery, active screens, temperature, lock state.
- Act on the lid change in the callback (or wake the loop from it). The spike's loop added up to 561 ms.
- Lock with `LockWorkStation` at the lid close; Windows does not lock then. Allow one failed `QueryDisplayConfig`
  during the lock.
- Re-take every held request after each resume. Without it the request was ignored after a manual sleep.
- On battery a held request ends about 5 minutes after the sleep timeout, lid closed too: raise the DC sleep timer
  while holding.
- On Modern Standby, screen off is not suspend: apps run until the sleep timeout. Treat `PBT_APMSUSPEND` as "paused
  now", not the display-off callback or a 506.
- After a release with the lid closed the PC stays awake until the user's sleep timeout. The end of work needs its
  own action.
- `SetSuspendState(FALSE)` hibernates this laptop. Check `IsPwrHibernateAllowed` and enable SeShutdownPrivilege
  first. The image write took about 8 s and the resume about 19 s (firmware plus image read).
- `QueryUnbiasedInterruptTime` skips suspended time: fine for durations. Detect a suspend by wall clock against
  unbiased time, plus the suspend notifications.
- `SystemExecutionState` reads 0x1 for up to 30 s after some resumes because of Windows' own request. Do not read
  that as another app's hold.
- `PlaySound` true does not prove the user heard anything.
- A killed process leaves a lid Do nothing plan copy active: the guard process must cover this.
- Spikes that test privileges must not use the parent-spoofing `limited` launch.

Files: night: `spikes/lid-sleep/` (`Cargo.toml`, `Cargo.lock`, `.gitignore`, `README.md`, `src/main.rs`, `modes.rs`,
`plan.rs`, `journal.rs`, `power.rs`, `events.rs`, `ticks.rs`, `display.rs`, `heat.rs`, `sys.rs`, `log.rs`, logs in
`out/`). Morning: `docs/research.md` ("Lid and sleep spike": "Morning results" added, "morning" notes replaced) and
this card. Scratch helpers, not in the repo: `logsum.py`, `reqsum.py`, `heat.py`, `beforeafter.py`, `f-events.ps1`.

Checks:

- Every question has a tested answer or "could not test" with the reason: yes. Question 1 has two "could not test"
  parts (screen-off method, hibernate off) and question 2 is "could not test".
- Haik's values: BEFORE at 10:18 (Log) matches the night's. Each morning run's 13 `after` lines equal its 13 `before`
  lines (D, F1, F2, H run 1, H run 2, E, I, H4). No journal left. The card's AFTER line (old K3): written by the
  orchestrator at 13:08 (Log); it matches the BEFORE line.
- Requests log: no other app's DISPLAY, SYSTEM or EXECUTION request in D, E, H or I. No block spoiled.
- No code changed this round. Night: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean,
  `cargo test` 30 passed.

Open items, each with my default:

- Sound: a 2 minute retest (lid open, play the sound on the Speakers, then close with nothing unplugged and
  `--no-lock`) would settle what Haik hears. Default: no retest; the delta review proposes another cue.
- The screen-off end method (F3) is untested. Default: the delta review decides whether the end of work needs it; if
  yes, a 5 minute Haik block.
- Resume (H) is one run each, on AC only. Default: accept; the re-take stays either way.
- Window close (CTRL_CLOSE_EVENT) restore in Windows Terminal: tested at 04:30 (restored in 0.1 s). Closing a single
  tab: not tested.
