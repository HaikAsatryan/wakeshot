# Review: are the spike safety fixes and the morning list ready?

Card: `docs/tasks/p0-spike-safety-fixes.md`. List: `docs/phases/00-morning-steps.md`. Launcher:
`spikes/morning-shell.cmd` and `.ps1`. Code: `spikes/power-settings/`, `spikes/lid-sleep/`, `spikes/tray/`.
Reviewer: wakeshot-reviewer, 2026-11-03 about 04:05 to 04:30.

How checked: read the list, the launcher, the card, the earlier safety review and the changed code. Ran only reads
(`powercfg /list`, `/qh`, registry reads, token reads of running processes), the three spikes' `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings` and `cargo test`, and one run that writes no setting: `spike watch`
through a function in `pwsh -NoExit`, stopped with Ctrl+C. No setting changed, no release exe rebuilt, PID 11284 left
alone.

State at about 04:05 (matches the list's table and Haik's values in the brief): Balanced
`381b4222-f694-41f0-9685-ff5bb260df2e` the only plan and active. LIDACTION 1/1, STANDBYIDLE 10800/600, RTCWAKE 0/0,
VIDEOIDLE 300/180, ESBATTTHRESHOLD 0/30, CONSOLELOCK 1/1, overlay AC `ded574b5-...`, DC empty. No journal in either
`out\`. No spike process. Keep-awake PID 11284 running.

## Fix before Haik starts

Both are a few lines in `docs/phases/00-morning-steps.md`. No code change needed.

1. Add a "stopping early" rule at the top (finding 1): if Haik stops before the Final check for any reason, he runs
   `restoreall` and `checkpower` before he leaves the laptop or closes the lid, and he does not close the lid or bag
   the laptop until `checkpower` says ALL OK.
2. Make the orchestrator steps a handshake with a way out (finding 2): at C1 and D1 Haik tells the orchestrator and
   waits for its OK. If no orchestrator session answers, he runs, in his normal (elevated) Windows Terminal:
   `Stop-Process -Id 11284` and `powercfg /requests`, and checks that SYSTEM, EXECUTION and DISPLAY are empty.

Cheap, same edit: finding 3 (say "in the morning shell" in restore step 7) and finding 4 (close media apps before the
request check).

## The 11 safety findings

| Finding | Status (my check) | Remaining risk in the list, with a way out? |
| --- | --- | --- |
| 1. Elevated Terminal | Fixed. Shell Explorer PID 11580 is Medium, not elevated; Terminal 13456 is High (tokens read now). A `.cmd` started by Explorer runs with Explorer's token, and nothing in the launcher elevates. All write modes refuse an elevated token (power `main.rs:144-151`, lid `main.rs:98-105`, tray `main.rs:205`). | Real mouse double-click not run by any agent. The shell prints its token and STOP (`morning-shell.ps1:114-120`); step 0.3 says what to do. Yes. |
| 2. No manual restore, no final check | Fixed. Every value in the restore block equals what I read now. | Step 7 wording gap (finding 3). |
| 3. Lid copy of any plan; restore can stick | Fixed. Refusals both ways (lid `plan.rs:79-97`, power `main.rs:69-86`); restore falls back to Balanced (`journal.rs` `switch_target` in both); lid unit tests for both cases pass. | Not needed. |
| 4. Restart, shutdown, sign-out | Partly fixed: the tools still cannot see sign-out or shutdown. | Yes: list lines 34-36 (open the shell again, `restoreall`, `checkpower`). |
| 5. Power spike has no ctrl handler | Fixed for `copy-test`, `hold`, `watch` (`ctrl.rs`, `main.rs:162-176`). My own test (below) of `watch` through a function in `pwsh -NoExit` worked. `write-test` still has none; it is not in the list. | Not needed. |
| 6. `copy-test` wrote "the active plan" | Fixed: every write names the copy; `/change` runs only while the copy is active (`main.rs:647-656`). A tiny race remains. | Covered: a stray write to Balanced shows in B6 `checkpower`; restore block step 3 fixes it. |
| 7. Step B can leave Power mode changed | Fixed in the list (B2, B3, restore step 5). | Yes. |
| 8. Step A guidance | Mostly fixed (900 s, Ctrl+C, table). | The battery-side Power mode is not in the table or in `checkpower` (finding 6). |
| 9. Small wording gaps | Fixed, all five. | |
| 10. Final build not dry-run | Fixed. Final lid exe (03:38:46) ran `lid`, `lid --heat-limit 1`, `resume`, `wake` at 03:45 to 03:51 (logs in `spikes\lid-sleep\out\`, all "restored", after values Haik's). Final power exe (03:54:06) ran `watch`, `copy-test`, `restore` at 03:58 and the `hold` Ctrl+C test at 03:55. | |
| 11. Docs not matching code | Fixed: both READMEs now say the journal is written right after the copy is made; `events.rs` comment right; lid card line marked moot. | |

## Findings

### 1. Major: the list has no rule for stopping partway

- Where: `docs/phases/00-morning-steps.md:8-9` ("You can take a break between any two blocks"), 30-40 (covers only
  a running tool, a restart and sign-out), A4 at 113-114, B1 to B5 at 141-151.
- What breaks: three places leave a changed state on purpose while Haik works by hand, and no line says what to do
  if he stops there.
  - A4 sets "Closing the lid" on battery to Do nothing on Haik's real Balanced plan for 5 s. Only A8 checks it. If a
    call or a meeting cuts in between A4 and A8, his real plan keeps lid Do nothing on battery. During A the
    keep-awake (a SYSTEM request) still runs, so a laptop closed into a bag on battery keeps running until the
    battery rule ends the request.
  - B1 leaves the "Wakeshot spike copy" plan active on purpose until B5, through the optional B4 restart.
  - H's copy (sleep and screen off after 60 s) stays active while the PC sleeps in the control phase, until Haik
    wakes it.
- `checkpower` would show each case, but only if Haik opens the shell again; nothing tells him to.
- Fix: one paragraph next to "To stop any run": "If you stop before the Final check, for any reason: run
  `restoreall`, then `checkpower`. Do not close the lid or put the laptop away until it says ALL OK. If the window is
  gone, double-click `morning-shell.cmd` again first."

### 2. Major: orchestrator steps have no handshake, and Haik has no way out without the orchestrator

- Where: list lines 157-161 (C1), 180-181 (D1); lines 4-5 say Orchestrator lines are "not for Haik".
- Facts: PID 11284 runs High and elevated (token read now: integrity 0x3000, elevated true). Haik's Medium morning
  shell cannot stop it, and `powercfg /requests` needs admin.
- What breaks: nothing tells Haik to stop at C1 and wait. If he goes on, or the orchestrator session has ended (a
  session limit), the keep-awake's SYSTEM request stays on until 13:26. Then:
  - C (the main battery test) proves nothing: the PC may stay awake because of the keep-awake, not the spike.
  - E and I's "after work ended" result is wrong: after the plan is restored with the lid closed, the keep-awake can
    keep the closed laptop running. On AC that lasts until 13:26 if Haik does not open the lid.
- Fix: C1 and D1 start with "Haik: tell the orchestrator you are at C1 and wait for its OK." Add the way out: "If no
  orchestrator answers: open your normal Windows Terminal (it runs as admin), run `Get-Process -Id 11284` (it must
  be `pwsh`), then `Stop-Process -Id 11284` and `powercfg /requests`. SYSTEM, EXECUTION and DISPLAY must be empty."

### 3. Minor: manual restore step 7 runs `checkpower` in a window where it does not exist

- Where: list lines 79-88. Step 6 sends Haik to the elevated Windows Terminal; step 7 says "Run `checkpower` again".
  `checkpower` is defined only in the morning shell (`morning-shell.ps1:39`).
- What breaks: in the Terminal it fails with "not recognized". A non-specialist may think the restore failed.
- Fix: "Close that window. In the morning shell, run `checkpower` again."

### 4. Minor: C1 checks power requests before C2 closes the apps that make them

- Where: list lines 157-162. C1 (orchestrator) checks that SYSTEM, EXECUTION and DISPLAY are empty; C2 then says to
  close apps that play sound or video. No line says what to do if the list is not empty.
- What breaks: a browser playing video makes C1 fail, or the check passes and an app started after it spoils C.
- Fix: move C2 before C1, and add "if anything is listed, close that app and check again".

### 5. Minor: the closed-lid "time limit" bounds the tool, not the laptop

- Where: list lines 194, 208-209, 246. Code: `--end release` and `--end none` do nothing after work ends
  (`spikes/lid-sleep/src/modes.rs:426-428`); the plan goes back to Balanced, with the lid still closed.
- What breaks: if Haik walks away with the lid closed, the run ends on time, but whether Windows then sleeps is the
  open question E measures. If it does not, the laptop runs closed until Balanced's idle sleep: 3 h on AC. Also
  nothing stops the orchestrator from running builds or agents during D to I, which adds heat while the lid is
  closed.
- Fix: one line in D: "Never leave during a closed-lid step. If you must, open the lid first." One orchestrator line:
  no builds or agent runs from D to I.
- Note, not a finding: the 85 C limit reads the ACPI zone `\_TZ.THM0` (`heat.rs`), which reached only 67 C under full
  load with the lid open. So it is a late backstop; the hand check in D8, E6 and I4 is the real heat check.

### 6. Minor: the battery-side Power mode is neither in the A table nor in `checkpower`

- Where: `spikes/power-settings/src/watch.rs:91` logs `overlay.dc.reg`; the list's table (lines 125-135) has no row
  for it; `morning-shell.ps1:78` checks only `ActiveOverlayAcPowerScheme`. Block A says "AC or battery" (line 104).
- What breaks: if Haik does A on battery, A6 changes the battery Power mode (today empty). A8 shows
  `STILL CHANGED: overlay.dc.reg ...` with no row to fix it, and `checkpower` still says ALL OK.
- Fix: write "on AC" for block A (simplest), or add the row and a DC overlay check in `checkpower`.

### 7. Minor: `copy-test --keep` keeps the copy active without checking its values

- Where: `spikes/power-settings/src/main.rs:681-688` (write-back of sleep and lid on the copy, results only logged)
  and 793-800 (`--keep` returns 0 with the copy active, no check).
- What breaks: if one write-back fails, the active copy keeps lid Do nothing (set at lines 659-679) through B2 to
  B5 and the optional restart. Unlikely, but nothing would show it.
- Fix in the list: after B1, run `checkpower`. Expected: only the two plan lines in red; lid and sleep OK. If lid or
  sleep is DIFFERENT, run `spike restore` now and skip B4.

### 8. Minor: `checkpower` prints a trailing number

- Where: `spikes/morning-shell.ps1:96` (`return $bad`). The list says it "must end with `checkpower: ALL OK`"
  (lines 39-40, 99-100, 151, 169).
- What breaks: typed at the prompt, it prints `0` (or the count) as the last line. Haik may read that as a fault.
- Fix: say "ALL OK, then a 0" in the list, or print without returning when called at the prompt.

### 9. Minor: the lid tool marks itself clean only after the slow event dump

- Where: `spikes/lid-sleep/src/main.rs:125` (`events::mark_clean()` after the mode returns); `modes.rs:651` restores,
  then `finish` runs `log_values` and a PowerShell `Get-WinEvent` dump. The power tool marks clean right after
  restore (`power-settings/src/main.rs:820-823`, 1076-1079).
- What breaks: on a window close the handler waits up to 4.5 s, then Windows ends the process, maybe during the dump.
  The plan is already back, so only the `after` lines and the events are lost from that log.
- Fix: call `events::mark_clean()` right after `copy.restore()` returns true in `lid`, `wake` and `resume`.

### 10. Minor, unverified: "console host, not Windows Terminal" is inferred from a missing `WT_SESSION`

- Where: `docs/research.md:532`, `spikes/morning-shell.ps1:110`, the card Result ("hosted by the console host").
- Why it may be wrong: Windows Terminal is the default terminal here (card Log 03:31). When Explorer starts a console
  app, the process exists before Terminal takes over the window, so Terminal cannot add `WT_SESSION` to it. A missing
  `WT_SESSION` then says nothing about the host. I did not test a handoff.
- Effect: Haik's real window may be a Terminal tab, where window close is untested. Safety is covered (line 32 says
  to run `restoreall` after a close); the token does not depend on the host. Fix the research line and the log field.

### 11. Minor: naming drift

- `docs/tasks/p0-spike-lid-sleep.md:58` and the fixes card call the 22 minute run "I2"; list block I has no I2, and
  I3 is the optional wake timer at the end. List line 5 says "A to J" but there is a K. The next worker reading the
  logs may mix them up.
- `docs/phases/00-facts.md:17` still says the tray steps are in the card (already noted in the card's "Found outside
  scope").

## Checked and fine

- Builds: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass in all three spikes
  (power 10, lid 36, tray 1). Release exes are newer than their sources (lid 03:38:46, power 03:54:06, tray
  03:41:04) and were not rebuilt by me.
- Launcher: `pwsh -NoExit -NoProfile -Command ". '<path>\morning-shell.ps1'"` dot-sources into the global scope, so
  the functions stay. The `.cmd` and `.ps1` have no Zone.Identifier stream, so RemoteSigned does not block them. Win+R
  also starts through Explorer. The only step that sends Haik to an elevated window is restore step 6, for two
  `powercfg` lines; the tools are not defined there, and they refuse an elevated token anyway.
- Ctrl+C through the shell's function wrapper: `spike watch --seconds 60` run as `function spike { & exe @args }`
  inside `pwsh -NoExit` in a new console host, Ctrl+C sent to the console group at 7 s. The exe logged event 0,
  "ended early by Ctrl+C" and "every value is back" within 200 ms and exited by itself; pwsh did not kill it and
  kept running. Its console writes after Ctrl+C did not fail (each log line prints before it is written to the
  file). Not checked: the screen itself. Script and log:
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\rv-ctrlc-func.ps1`
  and `...\scratchpad\rv-watch.log` (same folder).
- Restore block: the commands are the ones the worker ran at Medium on a throwaway copy (exit 0, values read back);
  the earlier `write-test-limited` log shows Medium writes to Balanced itself work. Copies are deleted by GUID, after
  Balanced is made active.
- Order: no idle sleep before C1 (A and B only edit settings and run a 30 s copy test). C, H, D, E, I and the optional
  G and I3 all come after C1. The keep-awake stays off until the end. Every closed-lid run has a tool limit (D 8 min,
  E 8 min, I 30 min), a hand heat check (D8, E6, I4), and the hard-desk line.
- Lid code: every write goes to a copy; Ctrl+C or close in `lid` restores before the result lines; `--max-minutes`
  and the heat limit use the wall clock and the `Working` state; `restore` falls back to Balanced and never to a spike
  copy.
- Tray: logs its token and refuses an elevated token in interactive mode (`main.rs:197-217`). Unverified: Task
  Manager in T10 probably starts elevated here (no prompt policy), but Explorer is expected to come back at Medium.
  Even if not, no later step needs a new morning shell, and the shell's token line would catch it.

## Round 2 check

Verdict: Safe to start.

Checked about 04:40 to 04:55, changes only: `docs/phases/00-morning-steps.md`, `spikes/morning-shell.ps1`,
`docs/research.md`, `docs/tasks/p0-spike-lid-sleep.md`, `docs/phases/00-facts.md`, the card's "Round 2" section. No
Rust source changed since round 1 (timestamps), so the build and test results above still hold. `morning-shell.cmd`
is unchanged. `morning-shell.ps1` parses with no errors. Values now: Balanced the only plan and active, LIDACTION
1/1, STANDBYIDLE 10800/600, RTCWAKE 0/0, VIDEOIDLE 300/180, ESBATTTHRESHOLD 0/30, CONSOLELOCK 1/1, overlay AC
`ded574b5-...`. No journal, no spike process, no extra Windows Terminal left (only Haik's 13456), PID 11284 running.

| Finding | Round 2 status (my check) |
| --- | --- |
| 1. No rule for stopping partway | Fixed. "Stopping early" is the first section (list lines 3-15), and every block ends with a stop line. The T block's own line (Quit the tray, theme back to Dark) fits it. |
| 2. No keep-awake handshake, no way out | Fixed. C2 and D1: Haik writes in chat and waits; the way out after 5 minutes is for the admin Terminal. I checked the guarded stop line: PID 11284's command line contains `keep-awake.ps1`, so the match is true now; a missing PID gives `$null` and the "already gone" branch. Terminal's default profile is PowerShell 7, so the line runs as pasted. |
| 3. Restore step 7 in the wrong window | Fixed (line 112). |
| 4. Request check before closing media apps | Fixed: C1 closes the apps, C2 checks, "close it and check again". |
| 5. Tool limit is not a laptop limit | Fixed: D intro (lines 230-234), "stay near it" in E4 and I3, and no builds, agents or keep-awake from C2 to K3. |
| 6. Battery Power mode not checked | Fixed by doing A and B on AC (lines 129-132). The reason for not checking the DC overlay in `checkpower` holds: today it is absent, which Settings may not be able to set back. |
| 7. `--keep` copy not checked | Fixed: B2 runs `checkpower` and names the expected three red lines. That matches the code: copy and "not Balanced" lines, plus the power journal; the values come from the copy, which equals Balanced. In the 03:58 `copy-test` log the AC overlay stays `ded574b5-...` while the copy is active, so no fourth line. |
| 8. Stray number after `checkpower` | Fixed: the function now emits nothing to the pipeline; the count goes to `$global:WakeshotCheckBad`. Startup and self-test use it correctly. |
| 9. Lid tool marks clean late | Left as is, and I agree. After a close event Windows ends the process when the handler returns, so marking clean right after restore (my suggestion) would cut the RESULT lines. The worker's close test in the real host wrote the whole log in 1.1 s (`r2-wtclose-lid-run.log` in the scratchpad: restored 70 ms after event 2, DONE 1 s later). |
| 10. Host guessed from `WT_SESSION` | Fixed and confirmed. `morning-shell.log` 04:26 and 04:34: window class `PseudoConsoleWindow`, owner a new `WindowsTerminal`, Medium, `WT_SESSION` false. `research.md` corrected. Closing that window was tested for `hold` (restore 0.15 s after event 2, both API calls rc 0) and `lid`. |
| 11. Naming drift | Fixed: "A to K", block I titled as the review's I2, lid card line 58, `00-facts.md` table points to the list. |

Did the changes break anything? No. Small notes, none blocking:

- List lines 30-31 say the admin Terminal is used only in restore step 6 and the C2 way out; the D1 way out uses it
  too. Harmless, because D1 points to the C2 lines.
- The card's round 1 "Open items" still says the test launch "was hosted by the console host" and that closing a
  Terminal tab is untested. The Round 2 section corrects both, so a reader who stops at round 1 gets old facts.
- Still not tested by any agent: a real mouse double-click, and Ctrl+C typed into the morning shell's Terminal
  window (only sent with `GenerateConsoleCtrlEvent` in a classic console). Both are covered: the shell prints its
  token, and every stop path ends with `restoreall` and `checkpower`.
