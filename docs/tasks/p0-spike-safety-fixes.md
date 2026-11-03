# Fix the spike tools and merge Haik's morning steps into one list

Phase: 0. Status: done. Needs Haik: no. Agent: `wakeshot-worker`.
Runs after: the safety review and the early architecture review (both done). Parallel with: `p0-unelevated-liveness`.

## Goal

Haik can follow one ordered list in the morning, from a normal (unelevated) process, and no step or failure leaves
his power settings changed without exact restore commands in front of him.

## Read first

- `docs/reviews/p0-spike-safety.md` (all of it: "Fix before morning" and findings 1 to 11).
- `docs/reviews/p0-architecture.md`: only the section "Tests to add before Haik's morning steps" and the findings it
  points to.
- The "Steps for Haik" and Result sections of `docs/tasks/p0-spike-power-settings.md`, `docs/tasks/p0-spike-lid-sleep.md`
  and `docs/tasks/p0-spike-tray.md`.
- Code in `spikes/power-settings/`, `spikes/lid-sleep/`, `spikes/tray/` as needed.

## Deliverables

1. Fixes in the spike code for every blocker and major in the safety review, and for the minors that are cheap. Each
   tool Haik runs must run at a Medium (unelevated) token, because Wakeshot runs unelevated and Haik's Windows
   Terminal elevates by default (safety finding 1). This includes the tray spike: an elevated tray app does not get
   `TaskbarCreated` from Explorer the same way. Pick the simplest reliable launch path and say why.
2. `docs/phases/00-morning-steps.md`: one ordered list of everything Haik does by hand in the morning, with the
   orchestrator's own actions marked as such (for example "Orchestrator: stop the keep-awake PID and check
   `powercfg /requests`"). Order: power spike steps, then lid steps, then tray steps, unless you find a safer order.
   Each step says exactly what to run (full path), what to look at, and what to write down. Include:
   - A manual restore block at the top with Haik's exact values and `powercfg` commands (safety finding 2), and a
     final check after the last step.
   - The tests the architecture review asks to add (undock and unplug while the lid is closed; a 20 minute
     lid-closed run on battery), with a time limit and a heat check for any closed-lid run.
   - Wake timer steps (lid card G and I3) moved to the end and marked optional. Do not delete them.
   - A time estimate per block and a total.
3. In each of the three spike cards, replace the manual steps with a pointer to `docs/phases/00-morning-steps.md`,
   so there is one source. Keep each card's Result and Log; add a Log line.

## Acceptance checks

- `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` pass in each spike you change.
- Each tool started the way the morning list says runs at Medium integrity (check the token and record it).
- Dry runs of every changed mode leave Haik's values unchanged (read before and after, record both).
- Every safety review finding is marked fixed, partly fixed (why) or not fixed (why) in this card's Result.

## Out of scope

- Product docs (`product.md`, `architecture.md`, `research.md` beyond facts you newly verify). The orchestrator
  takes review findings to Haik.
- `spikes/claude-watch/` (the parallel card uses it).
- Real writes to Haik's active plan outside a test mode that restores them.

## Questions for Haik

None blocking. The morning list leaves one choice to Haik on the spot: the optional heat run in D3 (only if he wants
data for a heat guard, architecture review finding 18).

## Found outside scope

- `docs/phases/00-facts.md` card table still says the steps for Haik live in each spike card (for example line 17,
  "13 steps for Haik in the card"). They now live in `docs/phases/00-morning-steps.md`. Not edited (orchestrator's
  file).
- `docs/product.md` "Restore on ... sign-out" still needs a hidden window (`WM_QUERYENDSESSION`, `WM_ENDSESSION`) or the
  guard process: a console ctrl handler never sees sign-out or shutdown when the exe loads user32 (safety finding 4,
  design note). Product doc, out of scope.
- `spikes\morning-shell.log` has one High line (03:56:39, chain ends in `claude.exe`): my elevated check of the STOP
  message, not Haik's shell.
- The safety review's finding 10 rerun command, `limited lid --simulate-close-after 5 --work 10 --no-lock --no-sound`,
  runs the full 12 minutes when the real lid is open (the run waits in "after work" for a lid open that never comes).
  Add `--max-minutes 1`. I ran it that way, so the orchestrator does not need to.
- `explorer.exe <file>` from an elevated session is served by a short-lived `explorer.exe /factory,{75dff2b7-...}`
  instance. Mine exited by itself. Two others (started 03:41:48 and 03:42:08) were not mine; likely the parallel card.
- The power spike's ctrl handler logs "stopping and restoring" in `watch` too, where nothing needs restoring.
  Cosmetic.
- Round 2: this card's Log at 03:42 and the round 1 Result say the morning shell was "hosted by the console host".
  Wrong: it runs in a Windows Terminal window (Round 2). Left as written in the Log; corrected in `research.md`.
- Round 2: `spikes\morning-shell.log` has two Medium lines from my tests (04:26 selftest False, 04:34 selftest True),
  not Haik's runs.
- Round 2: the keep-awake has its own `conhost.exe` child (PID 27556). It should end with PID 11284; the orchestrator
  may want to check it is gone after the stop (not verified).

## Log

- 2026-11-03: card written by the orchestrator from the two reviews.
- 2026-11-03 03:26: worker started. Haik away, lid open, on AC. Orchestrator keep-awake PID 11284 stays on. This
  session is elevated.
- 2026-11-03 03:30 BEFORE (elevated shell, reads only: `powercfg /list`, `/getactivescheme`, `/qh` and `/q` on
  SCHEME_CURRENT, overlay registry values): Balanced `381b4222-f694-41f0-9685-ff5bb260df2e` the only plan and
  active. LIDACTION AC 0x1, DC 0x1. STANDBYIDLE AC 0x2a30, DC 0x258. RTCWAKE 0x0/0x0. VIDEOIDLE 0x12c/0xb4.
  ESBATTTHRESHOLD AC 0x0, DC 0x1e. Overlay AC `ded574b5-45a0-4f42-8737-46345c09c238`, DC empty. No journal in either
  `out\`. Keep-awake PID 11284 running.
- 2026-11-03 03:31: environment: Windows Terminal is the default terminal (`HKCU\Console\%%Startup` delegation
  GUIDs), `profiles.defaults.elevate: true`, EnableLUA 1, ConsentPromptBehaviorAdmin 0. pwsh is the Store build
  (app alias in `WindowsApps`). LocalMachine execution policy RemoteSigned.
- 2026-11-03 03:40: lid spike code: plan copy refuses while the power journal exists or the active plan is a spike
  copy; restore never switches to a missing plan or a spike copy (falls back to Balanced); `--heat-limit` (default
  85 C) ends closed-lid work and the `heat` load early; ctrl handler comment fixed; refusal text points to the
  morning shell. fmt, clippy `-D warnings` clean, 36 tests pass.
- 2026-11-03 03:44: power spike code: Ctrl+C and close handler for `copy-test` and `hold` (restore first, journal
  removed before the slow log dump); `copy-test` writes name the copy, `/change` runs only while the copy is active;
  same cross-spike refusal and restore fallback as the lid spike. `write-test` loop left as it was (not in Haik's
  steps; I do not run it, it writes the real plan). Tray: token logged in every mode, interactive mode refuses an
  elevated token with a message box, new `--token` mode. All three: fmt, clippy `-D warnings` clean, tests pass
  (power 10, lid 36, tray 1). Tray ARM64 clippy clean.
- 2026-11-03 03:42: launcher `spikes\morning-shell.cmd` + `.ps1`. Self-test started the way a double-click does
  (`explorer.exe <scratchpad>\selftest.cmd`, which `call`s the real `morning-shell.cmd -SelfTest`): shell pwsh
  Medium, not admin, hosted by the console host (not Windows Terminal); `spike read` start line "elevated false,
  integrity Medium (0x2000)"; `lidspike read` the same; `trayspike --token` "elevated false, integrity Medium
  (0x2000)"; `checkpower` 0 different. Chain: pwsh <- cmd <- explorer.exe (a `/factory` COM instance, Medium) <-
  svchost DcomLaunch. Haik's shell Explorer (PID 11580) is Medium, not elevated; Windows Terminal (13456) is High,
  elevated (token read from this elevated session). `SeparateProcess` 0, so File Explorer windows run in 11580.
- 2026-11-03 03:45 to 03:53: Medium dry-run batch (`<scratchpad>\sf-dryrun.cmd` opened with `explorer.exe`; log
  `<scratchpad>\sf-dryrun.log`). Every exe start line: "elevated false, integrity Medium (0x2000)". Values before
  and after the batch identical (Balanced only and active, LIDACTION 1/1, STANDBYIDLE 0x2a30/0x258, RTCWAKE 0/0,
  VIDEOIDLE 0x12c/0xb4, ESBATTTHRESHOLD 0/0x1e, no journals). Runs: both restores with no journal; `lid` dry run
  (restored, DONE); `lid --heat-limit 1` (HEAT LIMIT at 55.1 C, work ended, restored, RESULT line); `resume --idle 0`
  dry run; `wake --wake-timers 1` dry run; `suspend --dry` (SeShutdownPrivilege enabled, last error 0); `heat` load
  stopped early at 53.1 C; power `copy-test` (clean; `/change` ran with the copy active); `hold --allow-ac` 1 min
  (clean); `copy-test --keep`; lid refused while the power journal existed; then the review's finding 3 case built
  by hand (lid copy of the power copy, active, lid journal naming the power copy); power `hold` refused while the lid
  journal existed; `spike restore` deleted the power copy (done); `lidspike restore` switched to Balanced (plan before
  gone) and deleted the lid copy (done). Before the fix that last step would fail with "FAILED, journal kept".
- 2026-11-03 03:54: power spike: `watch` gets the handler (Ctrl+C ends it early and still prints the diff); the end
  line now prints the log path. Rebuilt; fmt, clippy, 10 tests clean. Morning list written:
  `docs/phases/00-morning-steps.md`; steps in the three spike cards replaced by a pointer.
- 2026-11-03 03:55 to 03:56: Ctrl+C and close tests, the exe started by pwsh in a new conhost window like the
  morning shell (elevated with `--allow-elevated`; the handler does not depend on the token; scripts
  `<scratchpad>\sf-ctrltest.ps1`, `sf-console.ps1`, logs `sf-hold.log`, `sf-watch.log`, `sf-copyclose.log`).
  `hold` + Ctrl+C at 10 s: event 0, plan back and copy deleted 63 ms later, "clean", journal gone, pwsh and conhost
  exited. `watch` + Ctrl+C at 7 s: "ended early by Ctrl+C", diff printed ("every value is back"). `copy-test` +
  WM_CLOSE to the console window at 15 s (copy active, inside the change triggers): event 2, "skipping the rest",
  switch back and delete 130 ms later, journal gone, process ended by Windows after the handler returned.
- 2026-11-03 03:56: manual restore block checked at Medium (`<scratchpad>\sf-restoreblock.cmd` via `explorer.exe`, log
  `sf-restoreblock.log`) on a throwaway copy of Balanced in place of Balanced: every `/setacvalueindex` and
  `/setdcvalueindex` line exit 0 and the values read back as Haik's; `/setactive` of the copy, `/setactive` Balanced,
  `/list`, `/delete` of the copy all exit 0; the journal `Remove-Item` line prints nothing with no journals. The
  CONSOLELOCK lines fail at Medium ("You do not have the permissions required to perform this action.", exit 1) and
  work elevated (second throwaway copy from this session, read back 1/1, deleted).
- 2026-11-03 03:56: final self-test with the final exes (same launch as 03:42): shell, `spike read`, `lidspike read`,
  `trayspike --token` all "elevated false, integrity Medium (0x2000)"; `lidspike read` enables SeShutdownPrivilege
  (last error 0); `checkpower` 0 different. From this elevated session the shell prints the STOP line and `lidspike
  lid` refuses (checked).
- 2026-11-03 03:57 AFTER (elevated shell, reads only): Balanced the only plan and active. LIDACTION 0x1/0x1,
  STANDBYIDLE 0x2a30/0x258, RTCWAKE 0/0, VIDEOIDLE 0x12c/0xb4, ESBATTTHRESHOLD 0/0x1e, CONSOLELOCK 1/1, overlay AC
  `ded574b5-...`, DC empty, DelayLockInterval 0. No journal, no spike process. Matches BEFORE. `powercfg /requests`:
  only the orchestrator's SYSTEM request (PID 11284, still running).
- 2026-11-03 03:58: Medium runs of the final power exe (`<scratchpad>\sf-final.cmd` via `explorer.exe`, log
  `sf-final.log`, pwsh 7.6.9): `watch --seconds 3` (every value back), `copy-test` (clean), `restore` (no journal);
  all "elevated false, integrity Medium".
- 2026-11-03 04:00: the "active plan is a spike copy" refusal with no journal present (`<scratchpad>\sf-namecheck.cmd`
  via `explorer.exe`, log `sf-namecheck.log`): a copy of Balanced named "Wakeshot spike leftover test" made active by
  hand; `lidspike lid` and `spike hold` both refused with that reason; copy removed, Balanced active.
- 2026-11-03 04:18: round 2: fixing docs/reviews/p0-spike-safety-verify.md.
- 2026-11-03 04:22: `morning-shell.ps1`: `checkpower` no longer returns its count (it printed a stray `0` at the
  prompt; the count goes to `$global:WakeshotCheckBad`); the log field now reads the console window class and its
  owner process instead of guessing from `WT_SESSION`.
- 2026-11-03 04:26 BEFORE (elevated, reads only, `<scratchpad>\r2-read.ps1`): Balanced only and active, LIDACTION
  1/1, STANDBYIDLE 0x2a30/0x258, RTCWAKE 0/0, VIDEOIDLE 0x12c/0xb4, ESBATTTHRESHOLD 0/0x1e, CONSOLELOCK 1/1, overlay
  AC `ded574b5-...`, DC empty, no journals, no spike process, PID 11284 running.
- 2026-11-03 04:26: Medium test through Explorer (`<scratchpad>\r2-test.cmd`, transcript `r2-test.log`): token
  Medium; console window class `PseudoConsoleWindow`, owner `WindowsTerminal` (new PID 29564), `WT_SESSION` empty. So
  the morning shell runs in a Windows Terminal window, not a classic console; the old "console host" line was wrong.
  Bare `checkpower` ends at the ALL OK line, no number. `spike copy-test --keep` exit 0, then `checkpower` shows
  exactly 3 red lines (the copy plan, "active plan is NOT Balanced", the power journal) and `3 DIFFERENT`; every
  value line OK. `spike restore` exit 0, `checkpower` ALL OK.
- 2026-11-03 04:28: window close in that Windows Terminal window (`<scratchpad>\r2-wtclose-run.ps1`: `spike hold
  --allow-ac --sleep-dc 600 --minutes 2` through a function, then WM_CLOSE to the Terminal window it ran in): event 2
  0.33 s later, plan back and copy deleted 0.47 s after the close, journal gone, process, shell and that Terminal
  ended. Same for `lidspike lid` (`r2-wtclose-lid-run.ps1`, closed while waiting for the lid): restored 0.1 s after
  event 2, and the whole log (RESULT, after values, events, DONE) was written before Windows ended it 1.1 s after
  the close.
- 2026-11-03 04:29 AFTER (same reads): equal to BEFORE. Haik's own Terminal (PID 13456) untouched.
- 2026-11-03 04:30: morning list rewritten for the review: stop rule first, two named windows, keep-awake handshake
  with a way out at C2 and D1, C1 and C2 swapped, B2 `checkpower` with the expected three red lines, A and B on AC,
  D note on closed-lid stays and the 85 C backstop, restore step 7 wording, I named as the review's I2, "A to K".
  Guarded stop line checked with `Stop-Process` swapped for a print: true for 11284, false for PID 4 and a missing
  PID. `research.md`, lid card and `00-facts.md` card table updated.
- 2026-11-03 04:34: final self-test (`<scratchpad>\selftest.cmd` through Explorer, the real `morning-shell.cmd
  -SelfTest`): shell, `spike read`, `lidspike read` (SeShutdownPrivilege ok), `trayspike --token` all Medium, not
  elevated; `checkpower: 0 different`; host logged as `PseudoConsoleWindow` owned by a new `WindowsTerminal`. After:
  values equal BEFORE, no spike process, no extra Terminal left.

## Result

Status: done. Haik's morning list is `docs/phases/00-morning-steps.md`.

What was done:

- Launch path: `spikes\morning-shell.cmd` (double-click in File Explorer, or Win+R) opens `pwsh` with `spike`,
  `lidspike`, `trayspike`, `restoreall` and `checkpower`, prints the token and a full check of Haik's values. Why this
  path: Explorer always starts processes with Haik's Medium token, while his Windows Terminal opens elevated; `limited`
  hides the window and blocks SeShutdownPrivilege; `--allow-elevated` would test the wrong token. Tested for real: the
  shell and all three tools started from it ran at Medium, and the lid tool enabled SeShutdownPrivilege.
- Spike code: power `copy-test`, `hold` and `watch` get a Ctrl+C and close handler (tested both); `copy-test` writes
  only name the copy; both spikes refuse to start a test on top of the other spike's journal or copy, and restore
  falls back to Balanced when the plan before is gone (the review's case rebuilt by hand and cleaned up); `lid` and
  `heat` get `--heat-limit` (default 85 C); tray logs its token, refuses an elevated token in interactive mode, new
  `--token` mode; power end line prints the log path; README and comment fixes.
- Morning list: restore block with Haik's values and exact `powercfg` lines (tested at Medium on a throwaway copy),
  checks after B, C, D (start), K and at the end, new D (undock and unplug, lid closed), block I (the architecture
  review's I2: 22 minutes closed on battery), time limits and heat checks on every closed-lid run, G, I3 and C7
  optional at the end, time per block (about 115 minutes required, 35 optional).
- The three spike cards point to the list; Results kept; Log lines added. Facts in `docs/research.md` ("Spike launch
  and stop checks", plus one stale line fixed in "Lid and sleep spike").

Files changed: `spikes/morning-shell.cmd`, `spikes/morning-shell.ps1` (new), `spikes/power-settings/src/ctrl.rs`
(new), `src/main.rs`, `src/journal.rs`, `Cargo.toml`, `README.md`; `spikes/lid-sleep/src/main.rs`, `plan.rs`,
`journal.rs`, `modes.rs`, `heat.rs`, `ticks.rs`, `events.rs`, `README.md`; `spikes/tray/src/token.rs` (new),
`src/main.rs`, `Cargo.toml`, `README.md`; `docs/phases/00-morning-steps.md` (new); `docs/research.md`; the three
spike cards; this card. New logs: `spikes/morning-shell.log`, `spikes/morning-shell-selftest.log`,
`spikes/tray/out/log-token.txt`, run logs in both `out\` folders. Test scripts and their logs are in the scratchpad
(`sf-*`, `selftest.cmd`, `token-of.ps1`).

Acceptance checks:

| Check | Outcome |
| --- | --- |
| `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` in each changed spike | Pass at 04:01. Power 10 tests, lid 36, tray 1; clippy with `--all-targets`; tray also ARM64 clippy |
| Each tool started the way the list says runs at Medium | Pass: shell, `spike`, `lidspike`, `trayspike` all "elevated false, integrity Medium (0x2000)" (03:42 and 03:56, final exes) |
| Dry runs of every changed mode leave Haik's values unchanged | Pass: BEFORE 03:30 and 03:45, AFTER 03:53, 03:57 and 04:01 all equal (table in the list). Not run: `write-test` (writes the real plan, not in the list, its loop is unchanged) |
| Every safety finding marked | Below |

Safety review findings:

| Finding | Status |
| --- | --- |
| 1. Elevated Terminal, lid steps refuse | Fixed: morning shell, tested at Medium; refusal text points to it; tray refuses elevated too |
| 2. No manual restore, no final check | Fixed: restore block (tested at Medium on a copy; CONSOLELOCK needs admin, said so), `checkpower`, checks after C, K, final |
| 3. Lid tool copies any active plan; restore can stick | Fixed in code and steps: refusals both ways (tested), restore fallback (unit tests and the case rebuilt by hand), `restoreall` before D |
| 4. Restart, shutdown, sign-out leave the copy | Partly fixed: steps say what to do; the shell shows leftovers at start; comments fixed. The tools still cannot react to sign-out or shutdown. Stronger option: an HKCU RunOnce entry that runs both restores at the next sign-in while a copy is active; change it when a step is lost to a restart |
| 5. Power spike has no ctrl handler | Fixed for `copy-test`, `hold`, `watch` (Ctrl+C and close tested). Not for `write-test` (not in the list) |
| 6. `copy-test` writes "the active plan" | Fixed: every write names the copy; `/change` runs only while the copy is active (a short race remains between that check and the command) |
| 7. Step B can leave Power mode changed | Fixed in the list (B2 and B3 wording, repair line) |
| 8. Step A guidance not exact | Fixed: 900 s with Ctrl+C to end early, table of names, values and where to fix. Open: whether Settings writes the AC energy saver value (step A answers it) |
| 9. Small wording gaps | Fixed: shell defines `spike`; C log name printed; H3, F1 and F2 wording; C1 checks SYSTEM, EXECUTION and DISPLAY; keep-awake stays off until the final check |
| 10. Final lid build not dry-run | Fixed: final builds dry-run at Medium in `lid`, `resume`, `wake`, `suspend`, `heat`, power `copy-test`, `hold`, `watch`, `restore` |
| 11. Docs that do not match the code | Fixed: both READMEs (journal timing, window close, refusals), journal comments, `events.rs` comment, lid card stale line |

Architecture review tests: 1 (new D) added; 2 (I2 long hold, block I in the list) added with `--max-minutes 30`
instead of 25, because 22 minutes closed plus start and sign-in can pass 25; 3 (H `locked` note) added to the lid card's reading guide; 4
(heat run) added as optional D3; 5 (G, I3, C7) moved to the end as optional.

Open items (round 1; round 2 below corrects the host and tests the window close): a real mouse double-click was not
run by an agent (the shell prints the token, so Haik sees it); closing a Windows Terminal tab is not tested (the test
launch was hosted by the console host); the tray's elevated message
box was not shown (it would add a line to `log-interactive.txt`, the log Haik's run writes).

### Round 2 (fixes for `docs/reviews/p0-spike-safety-verify.md`)

Status: done. Changed: `docs/phases/00-morning-steps.md`, `spikes/morning-shell.ps1`, `docs/research.md` ("Spike
launch and stop checks"), `docs/tasks/p0-spike-lid-sleep.md` (one line, one Log line), `docs/phases/00-facts.md`
(three "in the card" pointers in the card table), this card. No Rust code changed, so no rebuild; release exes are
the ones the reviewer checked.

| Finding | Status |
| --- | --- |
| 1. Major: no rule for stopping partway | Fixed: "Stopping early" is the first section of the list (Ctrl+C, `restoreall`, `checkpower`, lid stays open until ALL OK, reopen the shell, tell the orchestrator); a one-line reminder ends every block from A to Optional |
| 2. Major: no keep-awake handshake, no way out | Fixed: at C2 and D1 Haik writes `stopping keep-awake` in chat and waits; orchestrator stops PID 11284 and checks DISPLAY, SYSTEM, EXECUTION. Way out after 5 minutes: two lines for the admin Terminal. The stop line only stops 11284 if its command line still names `keep-awake`, because after the optional B5 restart that PID can belong to another program (a plain `Stop-Process -Id 11284` could close one of Haik's apps). Checked as a dry run |
| 3. Minor: restore step 7 in the wrong window | Fixed: "go back to the morning shell" |
| 4. Minor: request check before closing media apps | Fixed: C1 closes them, C2 checks; "if one names an app, close it and check again" |
| 5. Minor: time limit bounds the tool, not the laptop | Fixed: D says never leave during D, E, I with the lid closed (open it, Ctrl+C, stop rule) and that the 85 C stop is a late backstop; orchestrator line: no builds, agents or keep-awake from C2 to K3 |
| 6. Minor: battery Power mode not checked | Fixed: A and B are "on AC". Not added to `checkpower`: today the DC overlay is absent, and picking a mode in Settings may write a value that cannot be set back to absent, so `checkpower` could never say ALL OK again |
| 7. Minor: `--keep` copy not checked | Fixed: new B2 runs `checkpower`; expected output tested at Medium (exactly 3 red lines, `3 DIFFERENT`, every value OK); any other red line: `spike restore`, skip to B7 |
| 8. Minor: `checkpower` prints a trailing number | Fixed in `morning-shell.ps1`: no return value; the count goes to `$global:WakeshotCheckBad` (self-test uses it). Tested: nothing after the ALL OK line |
| 9. Minor: lid tool marks clean after the event dump | Not changed, on purpose. On a close event Windows ends the process as soon as the handler returns (Microsoft's `HandlerRoutine` page; seen in `hold`). The lid handler waits for the end of the log (up to 4.5 s), so the earlier mark would cut the RESULT lines. Test in the real host: plan back 0.1 s after the close, the whole log written, process gone 1.1 s after the close |
| 10. Minor: host guessed from `WT_SESSION` | Fixed and verified: the morning shell runs in a new Windows Terminal window (console window class `PseudoConsoleWindow`, owner `WindowsTerminal`), with `WT_SESSION` empty. The research line is corrected; the shell log now records class and owner. Closing that window tested for `hold` and `lid`: plan back within 0.5 s |
| 11. Minor: naming drift | Fixed: list says A to K; block I is titled as the review's I2; lid card and this card say "block I (the architecture review's I2)"; `00-facts.md` card table points to the list |

Checks run:

| Check | Outcome |
| --- | --- |
| Values before and after every run (04:26, 04:29, 04:34) | Equal to Haik's: Balanced only and active, LIDACTION 1/1, STANDBYIDLE 10800/600, RTCWAKE 0/0, VIDEOIDLE 300/180, ESBATTTHRESHOLD 0/30, CONSOLELOCK 1/1, overlay AC `ded574b5-...`, DC absent, no journals, no spike process |
| Tools at Medium from the morning shell | Pass (04:26 and 04:34): shell, `spike`, `lidspike`, `trayspike` "elevated false, integrity Medium" |
| `cargo fmt`, clippy, tests | Not rerun: no Rust code changed this round (reviewer ran them at about 04:05, all pass) |
| PID 11284 | Not touched; still running at 04:34 |

Open items: Ctrl+C was tested only in classic console windows (sent with `GenerateConsoleCtrlEvent`); in the morning
shell's Windows Terminal window only a window close was tested. A real mouse double-click was still not run by an
agent. Haik's own steps
are unchanged in scope: blocks 0 to T, then the Final check.
