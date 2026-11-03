# Review: spike safety for Haik's morning steps

Cards: `p0-spike-power-settings` (steps A to C), `p0-spike-lid-sleep` (D to J), `p0-spike-tray` (Steps for Haik).
Code: `spikes/power-settings/`, `spikes/lid-sleep/`, `spikes/tray/`. Reviewer: wakeshot-reviewer, 2026-11-03 about
03:20.

How checked: read all three cards and the spike code. Ran only read-only commands (`powercfg /q`, `/list`, registry
reads, dumpbin) and one run that writes nothing (`lid-sleep-spike suspend --dry`) to test a window close. No setting
changed. State at 03:16: Balanced `381b4222-...` the only plan and active, LIDACTION 1/1, STANDBYIDLE 10800/600,
RTCWAKE 0/0, VIDEOIDLE 300/180, ESBATTTHRESHOLD AC 0 / DC 30, no `journal.txt` in either `out\` folder.

## Fix before morning

1. Haik's "normal" Terminal opens elevated, so every lid step D to I refuses to run, and the card does not say what
   to do. Give him an unelevated shell or `--allow-elevated` (finding 1).
2. Add a manual restore block with exact `powercfg` commands and Haik's values, and a final check step after J
   (finding 2).
3. Before step D: both `spike restore` and `lidspike restore` must print "no journal", and only Balanced may be in
   `powercfg /list` (finding 3).
4. One line in both cards: after any restart, shutdown, forced power off or sign-out during a step, paste the
   function again and run both restores (finding 4).
5. Cheap card lines, if time: B2 and B3 "change nothing or put it back" (finding 7), `spike` function line, H3 and
   F1 wording, C1 request check (finding 9).

## Findings

### 1. Blocker: Haik's Terminal opens elevated; lid steps D to I refuse to run

- Where: `docs/tasks/p0-spike-lid-sleep.md:66` ("Open Terminal normally, not as administrator").
  `spikes/lid-sleep/src/main.rs:98-104` refuses `lid`, `suspend`, `wake` and `resume` with an elevated token.
- Fact: Windows Terminal `settings.json` has `profiles.defaults.elevate: true`, and
  `HKLM\...\Policies\System\ConsentPromptBehaviorAdmin` is 0 (elevate with no prompt). So Terminal opened from Start
  or the taskbar runs as administrator with no prompt. This is also why the agent sessions run elevated.
- What breaks: D1 prints "refusing: elevated token" and exits 3. The lid card has no fallback (the power card has
  one, for `copy-test` and `hold`). If Haik follows the message and uses `lidspike limited lid ...`: the child runs
  hidden (`CREATE_NO_WINDOW`, `sys.rs:319`), so READY shows only when the run is over; Ctrl+C and closing the window
  do not reach the child; and under `limited` SeShutdownPrivilege cannot be enabled (README Notes), so F1 and F2
  test nothing. No settings risk by itself, but the steps cannot be followed.
- Fix, pick one and write it in the card:
  - Preferred: a launcher, for example `spikes\morning-shell.cmd`, that Haik double-clicks in File Explorer. It
    starts `pwsh -NoExit` with both functions (`spike`, `lidspike`) defined and runs
    `whoami /groups | findstr /i Mandatory`, which must show "Medium Mandatory Level". Basis: the worker's 02:48 run
    showed a `.cmd` started by Explorer gets a Medium token that can enable SeShutdownPrivilege
    (`spikes/lid-sleep/out/read-explorer.log`). Unverified: whether a double-clicked `.cmd` is handed to Windows
    Terminal and still stays Medium; the `whoami` line checks it.
  - Fallback: add `--allow-elevated` to every `lidspike` test command. Settings safety is the same (same plan copy
    and journal; the worker's Ctrl+C test ran this way), but the run is no longer "as a normal user".
- Power card, same cause: `copy-test --keep` and `hold` will refuse and need `limited`, which the card covers. Under
  `limited` the `hold` window shows nothing for 12 minutes and then prints the whole log; C3 should say so. The
  launcher above removes the need for `limited`.

### 2. Major: no manual restore commands and no final check

- Where: power card lines 57-59 and lid card line 56 say only "run `spike restore`" or "run `lidspike restore`". If
  restore prints "FAILED, journal kept" or "bad journal", Haik has nothing. No step re-checks the real values after
  J; the acceptance checks expect it, but no one is told to do it.
- Failure scenario: a restore fails (finding 3 gives a concrete path). A lid copy with lid action Do nothing stays
  active. Later the laptop goes into a bag while any app holds a power request (a build, a call, the orchestrator
  keep-awake). It runs closed with no time limit.
- Fix: add this block to both cards. It works from a normal or elevated shell.

  ```
  powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
  powercfg /list
  ```

  For every plan in the list other than `381b4222-f694-41f0-9685-ff5bb260df2e` (Balanced, Haik's only real plan):
  `powercfg /delete <that GUID>`. Then check:

  ```
  powercfg /qh SCHEME_CURRENT SUB_BUTTONS LIDACTION     (expect AC 0x00000001, DC 0x00000001)
  powercfg /q  SCHEME_CURRENT SUB_SLEEP STANDBYIDLE     (expect AC 0x00002a30, DC 0x00000258)
  powercfg /qh SCHEME_CURRENT SUB_SLEEP RTCWAKE         (expect AC 0x00000000, DC 0x00000000)
  powercfg /q  SCHEME_CURRENT SUB_VIDEO VIDEOIDLE       (expect AC 0x0000012c, DC 0x000000b4)
  ```

  If lid or sleep on Balanced is wrong:

  ```
  powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_BUTTONS LIDACTION 1
  powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_BUTTONS LIDACTION 1
  powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP STANDBYIDLE 10800
  powercfg /setdcvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_SLEEP STANDBYIDLE 600
  powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
  ```

  Then delete `spikes\power-settings\out\journal.txt` and `spikes\lid-sleep\out\journal.txt` if they exist.
- Also add step K after J, and the same check after C: the orchestrator runs the check above plus `powercfg /list`,
  confirms both journals are gone and no spike process runs, and writes the AFTER line in both cards.

### 3. Major: the lid tool copies whatever plan is active, and its restore can get stuck

- Where: `spikes/lid-sleep/src/plan.rs:79-97`. `PlanCopy::make` checks only the lid journal, then duplicates
  `power::active_scheme()` and records it as `active_before`. `journal.rs:50-61` and `plan.rs:152-172`: restore
  switches to `active_before` and fails if that plan no longer exists.
- Failure scenario: step C `hold` is cut short (window closed, Ctrl+C, or NOT CLEAN) and Haik goes on to D without
  `spike restore`. The active plan is the power copy (battery sleep 60 s). D makes a lid copy of that copy, with
  `active_before` = the power copy. Later Haik runs `spike restore`: the power copy is not active, so it is deleted
  and the power tool says "done". If a lid run then dies, or `lidspike restore` runs with the lid copy still active:
  `PowerSetActiveScheme(<deleted power copy>)` fails, `PowerDeleteScheme(<active lid copy>)` fails, and it prints
  "FAILED, journal kept". The lid copy with lid action Do nothing stays active. Even with no failure, a normal lid
  run ends by putting the power copy back, not Balanced, and the battery results in I are wrong (sleep after 60 s).
  The logic is from the code; the exact error codes are not verified.
- Fix before morning (card only): in the lid card "Before you start", run `spike restore` and `lidspike restore`;
  both must print "no journal". The orchestrator checks that `powercfg /list` shows only Balanced, active.
- Code fix (small, optional): in `PlanCopy::make`, refuse if the active plan's name starts with "Wakeshot spike" or
  if `spikes\power-settings\out\journal.txt` exists. In `restore_journal`, if `active_before` no longer exists,
  switch to a plan that is not in `copies` (Balanced here) before deleting. Add a unit test for that case in
  `journal.rs`.

### 4. Major: restart, shutdown or sign-out during a run leaves the copy active; the cards do not say what to do

- Where: `spikes/lid-sleep/src/events.rs:207-222`. The exe imports `user32.dll` (dumpbin `/imports`). Microsoft's
  `HandlerRoutine` docs: a console app that loads user32 or gdi32 does not get `CTRL_LOGOFF_EVENT` or
  `CTRL_SHUTDOWN_EVENT`. So the comment at line 213 ("Close, logoff and shutdown end the process...") is wrong for
  logoff and shutdown: the handler never runs then. The power spike has no handler at all.
- Failure scenario: in E and I the lid copy (lid Do nothing) is active from the start until 60 s after the lid
  closes, or longer if the process was paused before work end. A Windows Update restart, a long press on the power
  button (E5 tells Haik to press it if the screen stays black), or a crash in that window leaves the lid copy active
  after boot. The `lidspike` function is gone with the old shell, and no step says to run restore after a restart.
  Same for `hold` (battery sleep 60 s stays) and `copy-test`.
- Fix (card): one line at the top of both step lists: "If the PC restarted, shut down or signed out during a step,
  open a shell, paste the function line again, and run `lidspike restore` and `spike restore`. Each must print
  `no journal` or `done, journal removed`." Plus step K from finding 2.
- Design note: `docs/product.md` "Restore on ... sign-out" needs a hidden window for `WM_QUERYENDSESSION` and
  `WM_ENDSESSION`, or the guard process. A console ctrl handler alone will not see sign-out or shutdown.

### 5. Minor: the power spike has no Ctrl+C or close handler

- Where: `spikes/power-settings/src/main.rs` has no `SetConsoleCtrlHandler`. `hold` at line 822, `copy-test` at 642.
- What breaks: run directly, Ctrl+C or closing the window kills it at once. `hold` leaves its copy active with
  battery sleep 60 s. `copy-test` killed inside `detect_sequence` can leave the copy active with lid Do nothing on
  AC and DC (its last steps, about 5 s), until restore. The card's "run `spike restore`" covers this, so minor.
  Under `limited` the hidden child cannot be stopped from the window; it cleans up by itself at the end. If Haik also
  runs `spike restore` while it runs, both are harmless, but he does not see the result; the card should say the
  child keeps running.
- Fix: port the lid spike's handler, or keep it and add the card note.

### 6. Minor: `copy-test` writes to "the active plan", not to the copy

- Where: `spikes/power-settings/src/main.rs:553, 560, 614, 623` (`power::active_scheme().unwrap()`) and 593
  (`powercfg /change standby-timeout-ac`, which always targets the active plan).
- What breaks: if anything switches the active plan during the 20 s of `detect_sequence` (Vantage, a person in
  Control Panel), these writes land on Balanced. The `copy-test` journal has no `value` lines, so `restore` cannot
  undo them. The final write-back would fix them if it runs; a kill before it leaves, for example, lid AC 0 on
  Balanced. Unlikely in step B (Vantage opens after the run).
- Fix: pass the copy GUID to every write; drop `/change`, or check `active == copy` before each step and stop if not.

### 7. Minor: step B can leave Power mode changed

- Where: power card line 78 ("Can you change it?") and line 79 (Vantage). `restore_from` does not touch the
  overlay.
- What breaks: if Haik changes Power mode, or a Vantage thermal mode, to see if it works, the change stays after
  `spike restore`. Haik's values now (registry, 03:16): `ActiveOverlayAcPowerScheme`
  `ded574b5-45a0-4f42-8737-46345c09c238`, `ActiveOverlayDcPowerScheme` empty.
- Fix: B2: "Note the current Power mode. If you change it, put it back." B3: "Look only; change nothing in Vantage."
  Name the repair in the card: `spike set-overlay ded574b5-45a0-4f42-8737-46345c09c238`.

### 8. Minor: step A's "STILL CHANGED" guidance is not exact

- Where: power card lines 63 and 73; `spikes/power-settings/src/watch.rs:51-120`.
- What breaks: the diff prints internal keys (`energy-saver-threshold.ac 0 -> 30`, `DelayLockInterval 0 -> <absent>`,
  `overlay.ac.reg ...`). Some of these cannot be set in Settings. Also 420 s is tight for steps 2 to 6; if the program
  ends first, a value Haik forgets to put back after the end is not reported.
- Fix: use `--seconds 900`. Add a short table: key, where in Settings, Haik's value (lid 1/1 = Sleep, sleep
  10800/600, energy saver AC 0 / DC 30, sign-in and Power mode as noted in A5 and A6). For values Settings cannot
  set, give the line, for example
  `powercfg /setacvalueindex 381b4222-f694-41f0-9685-ff5bb260df2e SUB_ENERGYSAVER ESBATTTHRESHOLD 0` then
  `powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e`. Unverified whether Settings writes the AC energy saver
  value at all.

### 9. Minor: small gaps in step wording

- Power card line 56: `spike` is only a name; there is no function line like the lid card's. Add
  `function spike { & "C:\Users\haika\code\wakeshot\spikes\power-settings\target\release\power-settings-spike.exe" @args }`
  and say to paste it again after the B4 restart.
- Power card C6 names `out\hold-<time>.log`; under `limited` it is `out\hold-limited-<time>.log`.
- Lid card H3 (line 108): "Do not touch the laptop until DONE (about 8 minutes)". In the control phase the PC is
  meant to sleep, and the run only reaches DONE after a wake (the resume loop checks its end time only while it
  runs). Say: "After about 8 minutes, press a key, sign in and wait for DONE."
- Lid card F1 (line 89): "If nothing happens, DONE comes within 30 seconds" holds only if `SetSuspendState` fails.
  If it returns TRUE without sleeping, the run waits for input for up to 6 minutes. Add: "If nothing happens after
  30 s, press a key."
- Power card C1 (line 88) checks only for no SYSTEM request. A DISPLAY or EXECUTION request from another app also
  blocks idle sleep and spoils the battery test. Use the lid card's wording (SYSTEM, EXECUTION and DISPLAY empty).
  In both cards add: the keep-awake stays off until Haik reports J.

### 10. Minor: the final lid build was not dry-run in `lid`, `wake` and `resume`

- Where: `events.rs` changed at 02:58:51 and `modes.rs` at 03:02:47; the exe is from 03:02:50. The `lid`, `resume`
  and `wake` dry runs in `out\` are from 02:50 to 02:56; after the last build only `suspend --dry` ran (03:03).
- Risk: a regression in the modes Haik runs first. Unit tests cover the state machine, not the run.
- Fix: before Haik starts, the orchestrator reruns `limited lid --simulate-close-after 5 --work 10 --no-lock
  --no-sound` and `limited resume --idle 0 --simulate-resume-after 5` and checks the `after` lines. These make a plan
  copy, so it is the orchestrator's call, not mine.

### 11. Minor: docs that no longer match the code

- `spikes/lid-sleep/README.md` Safety says the journal is written "before it exists". The code duplicates first,
  then writes the journal (`plan.rs:84-86`; same in power `main.rs:675/693` and `842/853`; the power README says
  "before any write"). A kill in between leaves an inactive copy, named like Balanced, that no journal lists. The
  window is tiny; fix the text.
- `spikes/lid-sleep/src/events.rs:213` comment: logoff and shutdown are not delivered (finding 4).
- Lid README Safety and the lid card Result say window close is not tested. It works in a classic console (below);
  a Windows Terminal tab close is still not tested.
- Lid card "Found outside scope", line 134: says the power card names PID 16788; it now says 11284. Stale.

## Checked and fine

- The lid spike never writes Haik's plan: every `PowerWrite*` call goes to a fresh copy (`plan.rs:106`; grep of all
  write call sites). Power `copy-test` and `hold` write only the copy, except finding 6. `write-test`, which writes
  the real plan, is not in the morning steps.
- Lid spike restore paths: Ctrl+C (worker's test, restored in 40 ms); panic on the main thread (unwind runs
  `PlanCopy::drop`; no `panic = "abort"` in `Cargo.toml`); early returns (`Drop`); time caps by wall clock (12 min
  `lid`, 6 min `wake` and `suspend`, 20 min `resume`); sleep or hibernate in the middle (the process resumes and
  finishes).
- Window close in a classic console: I ran `lid-sleep-spike suspend --method screen-off --dry --allow-elevated
  --countdown 30` in conhost and posted WM_CLOSE to its console window. The handler logged "console control event 2"
  and the run reached DONE 0.9 s later; `println!` did not fail. In `lid` mode restore runs right after the loop
  stops, before the slow log dump, so it fits the 5 s close limit. Log:
  `%TEMP%\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\close-test-conhost2.log`.
- Order: the lid card says A to C first. C1 and the lid "Before you start" both stop the keep-awake. PID 11284 is
  running and is the only keep-awake process (15664 and 16788 from `keep-awake.log` are gone).
- Heat and battery: the longest planned closed run is D (request held until the lid opens, 12 min cap; Haik opens
  at 30 s), then E and I (request held 60 s after close, lid opened at 4 min). C runs 12 min on battery with the lid
  open. The spikes have no heat cutoff; fine for these times. The only unbounded case is a leftover lid copy plus
  another app's power request (findings 2 to 4).
- Tray: writes only its own HKCU keys (`toast.rs`), removed on Quit or `--cleanup`; no power calls. Theme: apps dark,
  taskbar dark, accent on taskbar off (`ColorPrevalence` 0), so step 9 "Dark" puts it all back.
- All three exes are newer than their sources.
