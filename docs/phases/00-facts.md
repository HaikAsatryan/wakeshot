# Phase 0: Facts

Goal: replace every guess the design depends on with a tested fact, before writing product code.

## Done

- [x] Research reports in `docs/research/`: `rivals-lidguard.md`, `rivals-others.md`,
  `claude-code-signals.md`, `other-agent-signals.md`.

## Task cards (`docs/tasks/`)

| Card | Needs Haik | Runs after | Status |
|---|---|---|---|
| `p0-toolchain` | no (Build Tools installed) | none | done |
| `p0-distribution-facts` | no | none | done |
| `p0-spike-power-settings` | done by Haik (B5 dropped) | toolchain, claude live tests | done |
| `p0-spike-tray` | T13 optional look | toolchain | done |
| `p0-claude-live-tests` | optional VS Code UI check | toolchain | done |
| `p0-spike-lid-sleep` | done by Haik | power settings spike | done |
| `p0-architecture-review` (reviewer) | no | all above | done: early run (`docs/reviews/p0-architecture.md`) and delta review (`docs/reviews/p0-delta.md`) |
| `p0-close-results` | no (Haik's answers in the card) | delta review, M1 check | done |
| tray Result review (reviewer, no card; started by a parallel session) | no | tray card | started 2026-11-03 (`docs/reviews/p0-tray.md`) |
| safety review of spikes and morning steps (reviewer, no card) | no | spike cards blocked | done (`docs/reviews/p0-spike-safety.md`) |
| `p0-spike-safety-fixes` | no | both reviews | done (`docs/phases/00-morning-steps.md`) |
| `p0-unelevated-liveness` | no | claude live tests | done |
| verify the safety fixes (reviewer) | no | spike safety fixes | done (`docs/reviews/p0-spike-safety-verify.md`); round 2 check: safe to start |
| `p0-close-proposals` | Haik answers 2 decisions in `docs/reviews/p0-close-proposals.md` | early architecture review | done |
| `p0-close-recheck` | Haik says yes to the rechecked part 2 (3 questions in chat) | close proposals, Haik's answers | done |
| `p0-morning-trim` | no (Haik then does the cut list) | Haik's answers | done |
| `p0-close-apply` | no (Haik's yes recorded) | close recheck | done |
| safety check of the cut list (reviewer, no card) | no | morning trim | done: verify round says safe to start (`docs/reviews/p0-morning-trim-safety.md`) |

Suggested rounds: (1) toolchain and distribution facts in parallel; (2) power settings spike, tray spike, Claude live
tests in parallel; (3) lid spike; (4) architecture review.

## Close

- [x] Orchestrator applies review findings with Haik: `research.md`, `product.md`, `architecture.md` (cards
  `p0-close-apply` and `p0-close-results`).
- [x] Haik signs off; Phase 1 starts. (Signed off 2026-11-03 14:35, before the tray review in
  `docs/reviews/p0-tray.md` finished; its findings are fixed in Phase 1.)

## Work log

- 2026-11-03: Phase file created. Research agents B started.
- 2026-11-03: `rivals-others.md` done. Design inputs to weigh in Phase 1 (not yet decided):
  - Re-take the power request after every resume (PowerToys Awake lost its hold after manual sleep and resume).
    Renew by taking the new request before dropping the old one. (tested form: clear, then set on the same
    handle, H4; this order not tested)
  - Re-add the tray icon on `TaskbarCreated` and retry at login until the notification area is ready.
  - Menu item "Restore my power settings" and a warning when a restore fails.
  - Claude Code inside WSL is invisible to Windows-side tools. Open: do we read WSL homes? (decided: not in
    v1, product.md)
  - App rules must match by exe name, not full path.
  - Rejected by earlier decision: "hold 2h" menu item (product.md: no "Hold awake" mode; the CLI covers it).
    (superseded: Keep awake now has 2 hours, product.md UX)
- 2026-11-03: `rivals-lidguard.md` done. LidGuard is MIT: porting with attribution is allowed. Design inputs:
  - Claude status stays `busy` after a turn ends while background subagents run (seen on this laptop). Background
    shell tasks and `/loop` timers still untested. LidGuard does not handle timers or Remote Control.
  - Every "still working" signal needs a time limit. LidGuard issue #5: one lost "work ended" event left the lid on Do
    nothing for 59 hours; laptop got hot in a bag. (superseded: decision 3, signals are checked against facts)
  - Docked detection: also check external displays (LidGuard WMI rule), not only the user's lid action.
    (no v1 rule uses it after decisions 1 to 3)
  - Heat guard candidate: thermal zone reads without admin here (about 50 C). Verify in spikes.
    (not adopted: decision 2)
  - On Modern Standby, Sleep can fail with "not supported": use screen off or hibernate (feeds spike C5).
  - Do not copy: LidGuard denies the agent's permission prompts when the lid is closed.
- 2026-11-03: `claude-code-signals.md` done. Key facts (details and 13 live tests in the report):
  - `status` is `busy`, `idle`, `waiting` (with `waitingFor`) or `shell` (background shell or shell Monitor running
    while idle; code only, not seen live). File rewritten only on change: no heartbeat, so `updatedAt` cannot show
    staleness.
  - `busy` stays after the turn while background subagents, workflows or teammates run (seen live).
  - Pending `/loop`, `CronCreate`, `ScheduleWakeup` show as `idle`. Only trace: the transcript
    `projects\*\<sessionId>.jsonl` (`nextFireAt`, `scheduledFor`, `CronDelete`, `scheduled_task_fire`). Medium
    confidence; can miss an Esc cancel and the automatic fallback wakeup. Durable jobs: exact, in
    `<project>\.claude\scheduled_tasks.json`.
  - A job that was due while the PC slept fires once on wake.
  - `claude --bg` sessions: `claude agents --json` is documented and reports `working` while a wakeup is pending.
  - `bridgeSessionId` is null after Remote Control disconnects.
  - Desktop app not installed here; very likely writes the same files (unverified).
  - New product questions: timer policy and `shell` policy (product.md Open 3 and 6).
- 2026-11-03: `other-agent-signals.md` done (read from source, nothing run):
  - Reliable: Codex CLI (last turn event in `~/.codex/sessions/.../rollout-*.jsonl`; approval waits look like
    working), OpenCode 2.x (SQLite `time_busy`, `time_idle`, `schedule.next_due_at`).
  - Partly inferred: Copilot CLI (`~/.copilot/session-state/<id>/events.jsonl`; no idle event; `permission.requested`
    shows waiting). Qwen Code blocks sleep itself by default.
  - No reliable signal: Gemini CLI (now enterprise only), Cursor CLI, Aider, Amp, Kiro. Antigravity CLI not covered.
  - Codex and Copilot can block idle sleep themselves (opt-in, `PowerRequestSystemRequired`); none touches the lid.
  - Free live tests with a local model: Codex, OpenCode, Qwen.
- 2026-11-03: Haik's answers applied to `product.md`: battery floor, user change wins, timers within 1 hour, unsigned
  previews, personal Codex and Copilot access, public repo with history squash before launch, app rules moved after
  v1, menu controls everything the CLI does, Keep awake on demand added (PowerToys Awake style). Orchestrator workflow
  added to `plan.md`.
- 2026-11-03: Orchestrator: `/orchestrate` skill written. Build Tools 2026 18.11.2 installed by winget with the
  VCTools workload and the VC.Tools.ARM64 component (MSVC 14.51.36231, Windows SDK 10.0.26100.0). Round 1 started.
- 2026-11-03: Card `p0-toolchain` done (rustup 1.29.2, Rust 1.99.1, x64 and ARM64 builds pass). Round 2
  changed to tray spike and Claude live tests: the live tests card does not allow a parallel card that
  changes power settings, so the power settings spike runs after it.
- 2026-11-03: Card `p0-distribution-facts` done (`docs/research/distribution.md`). Velopack 1.2.161 tested here with
  Rust: pack, per-user install, update, uninstall and all hooks work. Design risk for the review and Phase 1:
  Velopack kills the app without warning on update, uninstall and reinstall, and Setup.exe over an existing install
  (what `winget upgrade` does) wipes the install folder with no uninstall hook; `architecture.md` keeps our data
  there. winget accepts unsigned installers; SignPath declined new projects in 2026, so v1 ships unsigned.
- 2026-11-03 01:39: Haik away until morning, lid open, on battery. Orchestrator holds a keep-awake power request
  (details in `plan.md` Status) instead of changing the sleep timer, so the power spike reads Haik's real values.
- 2026-11-03: Card `p0-spike-tray` blocked on Haik's 13 steps ("Steps for Haik" in the card; exe in
  `spikes\tray\target\x86_64-pc-windows-msvc\release\tray-spike.exe`). Done alone: icon, two-line tooltip, native
  dark and light menus, toast with only an HKCU app id key, sounds report success, 504 KB exe, about 1.6 MB idle. For
  the close step: the "not verified" note on hidden tray icons in `product.md` is now verified.
- 2026-11-03: Card `p0-claude-live-tests` done without Haik (tests 1 to 5 and 7 to 12 live; 6 impossible, durable
  jobs off in this build; 13 needs the Desktop app). New: SDK (VS Code style) sessions never show `shell`; an Esc
  cancel of a wakeup writes nothing to disk; `/clear` changes `sessionId` but jobs keep firing; a hidden loop fallback
  fires about 20 minutes after a loop fire with no reschedule. Watcher spike in `spikes/claude-watch/` (16 tests).
  Ten proposed rule changes in `docs/research/claude-code-signals.md` go to the review. Side effect: three
  `cctest*` scratch folders marked trusted in `~/.claude.json`.
- 2026-11-03 02:00: Card `p0-spike-power-settings` started (keep-awake hold stays; idle sleep tests go to Haik).
- 2026-11-03 02:25: Card `p0-spike-power-settings` blocked on Haik's morning steps A, B, C (in the card). Done
  alone: Haik's values read the same before and after (Balanced only plan, lid 1/1, sleep 10800/600). An unelevated
  user can write lid and sleep on the active plan. The registry notification sees every stored change within 62 ms;
  the setting notification sees only applied values; our writes and the user's look the same. Plan copy create,
  switch and delete is clean. The laptop was on AC from some point in the night.
- 2026-11-03 02:26: The orchestrator session is elevated, so `powercfg /requests` works. It showed no SYSTEM request
  from the first hold (`SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)` in a hidden `pwsh -File`
  script, call returned success). Cause not known; not investigated. Replaced it with `PowerCreateRequest` and
  `PowerSetRequest(SystemRequired)`, which `powercfg /requests` lists with its reason string.
- 2026-11-03 03:05: Card `p0-spike-lid-sleep` blocked on Haik's steps D to J (about 45 min, after A to C). Done
  alone, as a normal user: docked check by counting screens (`QueryDisplayConfig`); PDH thermal zone follows CPU load
  (41 to 67 C); lid callback works with no window and reports the current state at once; wake timers need an absolute
  due time and setting one succeeds even with "Allow wake timers" = Disable; SeShutdownPrivilege can be enabled from
  a normal Terminal. Test modes change only a plan copy; Haik's values matched before and after.
- 2026-11-03 03:10: All cards but the review are done or blocked on Haik. Orchestrator started the architecture
  review early, so missing tests surface before Haik's physical steps; a delta review covers his results later. Also
  started a narrow safety review of the spike tools and morning steps (`docs/reviews/p0-spike-safety.md`).
- 2026-11-03 03:40: Safety review: 1 blocker, 3 major, 7 minor. Blocker: Haik's Windows Terminal has
  `profiles.defaults.elevate: true` and UAC elevates with no prompt, so his "normal" Terminal is elevated and the lid
  tools refuse to run. Architecture review (early): 1 blocker, 7 major, 15 minor. Blocker: the journal sits in the
  Velopack install folder that `winget upgrade` deletes. Major, for tonight: all Claude liveness checks ran elevated,
  but Wakeshot runs unelevated. It also asks for new morning tests (undock and unplug with the lid closed, 20 min
  lid-closed run on battery) and calls the wake timer steps unused. New cards: `p0-spike-safety-fixes` (spike fixes
  and one morning list, `docs/phases/00-morning-steps.md`) and `p0-unelevated-liveness`, in parallel. The other
  review findings wait for the Phase 0 close with Haik.
- 2026-11-03: Card `p0-unelevated-liveness` done. A Medium process (started through Explorer, token checked) can do
  every Claude check the design needs against High-integrity sessions: status files, transcripts, folder change
  notifications, open with exactly `PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE` (any extra right such as
  `QUERY_INFORMATION` or `VM_READ` fails the whole open with 0x80070005), start time, image path, wait on the handle
  (0.4 s after a clean exit, 24 ms after a kill). A dead PID gives 0x80070057, so dead and denied differ; a denial
  must never count as dead. Medium to Medium works fully. For the close: proposed rule 2 in
  `docs/research/claude-code-signals.md` and review finding 2 still say "confirmed only from an elevated process".
- 2026-11-03 04:05: Card `p0-spike-safety-fixes` done. Haik's single morning list: `docs/phases/00-morning-steps.md`
  (about 115 min plus 35 min optional). Haik starts every step from `spikes\morning-shell.cmd`, double-clicked in File
  Explorer (shell and tools checked at Medium). Power spike gets Ctrl+C and close handling; each spike refuses to start
  over the other's copy or journal; restore falls back to Balanced; 85 C heat limit; tray refuses an elevated token.
  Safety finding 4 only partly fixed (no reaction to sign-out or shutdown). Not tested: a real mouse double-click,
  closing a Terminal tab. A reviewer now verifies the fixes and the list.
- 2026-11-03 04:25: Verify review: 0 blocker, 2 major, 9 minor; 10 of 11 safety findings fixed, finding 4 partly
  with a manual way out. Majors: no rule for stopping partway (A4 sets lid Do nothing on the real plan for seconds; B
  keeps a copy active on purpose); no handshake before C1 and D1 to stop the keep-awake (PID 11284 is High, so Haik's
  Medium shell cannot stop it). Round 2 of `p0-spike-safety-fixes` started.
- 2026-11-03 04:45: Round 2 done: "Stopping early" section first, keep-awake handshake at C2 and D1 with a 5 minute
  fallback in the admin Terminal, morning shell opens as a new non-admin Windows Terminal window. Closing that window
  during `hold` and `lid` restored the plan within 0.5 s. The verify reviewer now checks the changes.
- 2026-11-03 05:00: Round 2 check: "Safe to start". Haik's values unchanged, no journals, no leftover processes.
  Orchestrator fixed two doc notes (D1 way out line in the list; round 1 open items marked as superseded). Not tested
  by any agent: a real mouse double-click and Ctrl+C typed into the morning shell (both covered by the token line and
  `restoreall`). Card `p0-close-proposals` started: one list of decisions for Haik from the reviews.
- 2026-11-03 05:20: Card `p0-close-proposals` done (`docs/reviews/p0-close-proposals.md`): 2 decisions for Haik
  (docked laptop and the lid-closed time limit; heat guard), 25 doc changes to apply after his yes, 8 items waiting
  for morning steps, 2 review ideas not adopted because they contradict fixed decisions. Orchestrator stopped for the
  night at a clean point.
- 2026-11-03 morning: Haik answered decision 1 (docked: the lid-closed limit does not apply while an external screen
  is on). Decision 2: no heat guard (Wakeshot automates the manual "lid Do nothing, laptop in a bag" path, which has
  no heat check; hardware throttling and the 113 C shutdown apply in both). Decision 3, Haik's own: no safeguards
  for other programs' bugs; drop the fixed lid-closed time limit; signals are checked against facts (process alive,
  timer due time), not capped by time (details in `docs/reviews/p0-close-proposals.md`). Cleanup: three `cctest*` entries removed from
  `~/.claude.json` (backup in the orchestrator scratchpad), five test transcript folders deleted from
  `~/.claude/projects`.
- 2026-11-03 09:30: Haik answered the decisions but has not yet said yes to part 2 of the close list. Decision 3 asks
  to recheck every part 2 item first. Card `p0-close-recheck` started (docs only, may run during the morning steps).
  Found: item 25 (live-test fixtures) said to copy shapes before the `cctest*` transcript folders were deleted; they
  were deleted this morning and no fixtures folder exists. The card rewrites item 25. Keep-awake PID 11284 still runs
  (ends 13:26). Morning steps not started yet as far as the files show.
- 2026-11-03 09:45: Haik has not started the morning steps and asked for the minimum manual work: only what a human
  must do. Card `p0-morning-trim` started in parallel with the recheck: tags each step Haik, Agent or Drop, rewrites
  the list to the Haik steps only, and removes the C2 and D1 keep-awake handshake (the orchestrator stops PID 11284
  right before Haik starts). The recheck card's deliverable 4 moved to this card. A reviewer checks the cut list for
  safety before Haik starts.
- 2026-11-03 (morning): Card `p0-close-recheck` done. Part 2: 17 keep, 5 change, 3 drop (busy cap, item 7 replaced by
  27, tray hang check), 8 new items (26 to 33). Part 3: all 8 items still needed. Moot morning parts: D3 heat run,
  the heat value from I, the screen count with the lid closed in D and E. Two questions for Haik (unknown Claude
  status value; tray hang check) plus one from its outside-scope notes (battery floor hibernates a docked, closed
  laptop while the user types). The live-test watcher log still existed in the old session scratchpad under %TEMP%;
  the orchestrator copied `live` and `ccstate` (no `.key` files) to `~/.claude/wakeshot-keep/2026-11-03-live-tests/`
  so a temp cleanup cannot lose them. Item 25 should read from there.
- 2026-11-03 (morning): Haik on the three recheck questions: "do as you think will be better for everyone (balanced)".
  Orchestrator took all three defaults (unknown status counts as working while the process lives, with an "update
  Wakeshot" notice; no tray hang check; at the battery floor release and restore at once, hibernate only after a few
  minutes with no input) and read it, with his "minimum work" request, as his yes to part 2. Recorded in
  `docs/reviews/p0-close-proposals.md`. Card `p0-close-apply` started, in parallel with the morning trim.
- 2026-11-03 (morning): Card `p0-morning-trim` done. Haik's list: about 80 minutes (about 30 hands-on), down from about
  2 hours plus 35 optional. Tags: 42 Haik (with the kept safety parts), 41 Agent, 8 Drop (D2, D3, the DELL question,
  the heat note, K1, T5, G, I3, C7). New order: T10, D, E, F, H, then C and I on one battery charge. Step 0 is all
  orchestrator: checks, tray start, volume to 50%, keep-awake stop, morning shell through `explorer.exe`, then "go".
  A, B (but B5) and most of T run by agents after Haik's Final check, with the laptop left unlocked about 40 minutes.
  Its two questions taken at the defaults (Haik delegated): Haik restarts Explorer himself in T10 (an agent killing
  Explorer would close his shell); Haik pastes the run commands himself. A reviewer checks the cut list for safety
  before step 0.
- 2026-11-03 (morning): Card `p0-close-apply` done. Part 2 applied to `product.md`, `architecture.md`, `research.md`,
  `claude-code-signals.md` and `CLAUDE.md`, plus item 34 (battery floor waits for no input). Dropped: 3, 7, 14.
  Waiting: 32 (when the lid card closes) and part 3 (morning results). Items 24 and 33 applied here by the
  orchestrator (superseded notes on the early design inputs). Fixtures in `spikes/claude-watch/fixtures/` (redacted;
  two of Haik's own transcript lines with every text field replaced, kept as the only raw task notification shape).
  For Phase 4: signals rule 11 (transcript growth when the status file is unknown) now contradicts the unknown-status
  rule, and rule 5 does not cover a `--resume` in a new process.
- 2026-11-03 (morning): Safety check of the cut list: not safe to start, 0 blocker, 5 major, 8 minor, all doc edits.
  Haik's own blocks are safe as written; the majors are in the agent steps: the post-Haik Settings tour changes his
  real plan after the AFTER lines and nothing checks the end state; the tour's exact changes were lost with the old
  list; power and tray agent steps may run in parallel; the agents' display request has no time cap; other apps'
  power requests are checked only at step 0 (Chrome holds DISPLAY and EXECUTION now). Battery is enough (70% of 80.3
  Wh leaves about 40% after C and I). The trim worker was resumed to fix them; the reviewer verifies after.
- 2026-11-03 (morning): Trim worker fixed all 5 majors and 8 minors: the Settings tour now writes only to a plan copy
  (B1 first), with a put-back table and an end-state check before the AFTER lines; agent screen steps run one at a
  time (power, then tray); each display holder stops itself after 45 minutes or on battery; Haik pauses media once in
  step 0 and the orchestrator logs `powercfg /requests` every 30 s. Haik's time unchanged (about 80 minutes); agents
  need the screen about 60 minutes after his Final check. Its question (add `DelayLockInterval` and the battery
  Power mode to `checkpower`, a tool change) taken at its default: no, agents read them. Reviewer verifying.
- 2026-11-03 (morning): Verify round: "Safe to start", all 13 findings fixed, 4 new minors. Orchestrator notes: the
  Control Panel lid page writes every plan, so the A4 fallback briefly touches Balanced too (the end check catches
  it); restart the requests log after any break; the spike card headers still say "Parallel with", but their agent
  steps run one at a time (power, then tray) as the list says. A worker started step 0.2 items 1 to 3 (BEFORE
  lines, tray start, volume); the orchestrator does items 4 to 8.
- 2026-11-03 10:22: Correction: the orchestrator's log times from 10:05 to 11:55 above were guesses, not clock
  reads; those lines now say "(morning)". Real clock: step 0 ran at 10:18 to 10:22. Step 0 done: BEFORE lines from
  live reads (all values match `CLAUDE.md`, DelayLockInterval 0); tray spike PID 11944 at Medium, icon moved onto the
  taskbar by `IsPromoted`=1 (T12 must delete it); battery 100% on AC; volume unchanged (Speakers 67%, DELL 24%);
  keep-awake PID 11284 stopped at 10:22, then `powercfg /requests` all `None.` and `SystemExecutionState` 0x0;
  requests log PID 33632 (orchestrator scratchpad `requests.log`, ends by itself after 3 hours); morning shell open
  at Medium (10:22:40). Haik told "go".
- 2026-11-03 12:15: Haik's morning steps, from 10:23. His only power source is the DELL USB-C cable (no separate
  charger), so the order changed: D (charger and DELL unplug in one go), F and H on AC with the DELL in, then E, C, I
  on battery with the DELL out, DELL back in at I5. H first run (10:41) proved nothing (the screen-off at 10:42 was
  the 60 s copy timeout; control did not sleep). H second run (10:57): `request NOT honored` after resume, control
  slept, test valid. H4 (`--retake`) skipped at first, then started at about 12:10 on the orchestrator's ask. E: no
  standby after work ended until the lid opened. I: the PC slept 912 s after lid close with the request held, for
  514 s (matches the battery rule: 600 s timeout plus 5 minutes). Sounds, Haik's words: D "from laptop"; E "yes I
  heard on lid close the laptop sound again (not windows one)"; I none, though `PlaySound` returned true in E and I.
  Plan Balanced and no journals at 12:06.
- 2026-11-03 12:20: H4 (`lidspike resume --retake`, 12:09 to 12:18, DELL in): `request honored` in the hold phase
  after resume, control slept twice, test valid. So a request held across a sleep is not honored after resume, and a
  fresh one taken after resume is. Sound, Haik: "I think it was not windows notification sound" (D and E), so the
  sound he heard may be the laptop's own, not our `PlaySound(SystemNotification)`. For the delta review.
- 2026-11-03 12:25: Haik's Final check: `checkpower` ALL OK. His hands-on part is done. Requests log PID 33632
  stopped (179 samples): no other app held DISPLAY, SYSTEM or EXECUTION during the blocks, only a `[DRIVER] Legacy
  Kernel Caller` SYSTEM entry at 12:15:09 right after an H4 resume. The power spike's C request lasted 11:21:29 to
  11:27:30 (about 60 s plus 5 minutes). Quick check: only Balanced, active; no requests; no journals; only the tray
  spike (PID 11944) runs. Started the power card worker (agent steps B then A, on the screen) and the lid card worker
  (log reads only) in parallel; the tray card follows the power card.
- 2026-11-03 (after 12:25): Lid card worker recorded Haik's results (card Result). Part 3 outcomes: 1 battery
  sleep timer confirmed (I suspended 916 s after the last input, C about 380 s); 2 `SetSuspendState(FALSE)` and
  `(TRUE)` both hibernate here, a closed PC with no request stays awake until the sleep timeout, F3 untested; 3
  Windows locks within 1 s of display-off standby with a request held (CONSOLELOCK 1, DelayLockInterval 0); 4 no
  input; 5 unbiased time skips suspend, no edit; 6 our lid-close sound not shown to play (D went to the DELL, E was
  most likely the device-remove sound, I silent): another cue needed; 7 Windows does not lock at lid close, our lock
  works, a request is not honored after a manual sleep and a re-take fixes it; 8 work does not pause with the screen
  off. Found: E started on AC (cable out 4.3 s before close), and the tool's E line "no standby" is wrong (event 506
  logged). Its defaults taken: no sound retest, F3 to the delta review, one H run on AC is enough.
- 2026-11-03 12:40: The laptop entered Modern Standby at 12:33:48 (300 s AC display timeout after Haik's last
  input); the power worker's screen holder started at 12:33:58, 10 s too late. Haik woke it at 12:39:40 and signed
  in. Worker told to check and redo any step from that window. Lesson for the tray worker and later agent steps:
  start the screen holder first, before reading anything.
- 2026-11-03 12:50: Card `p0-spike-power-settings` done (findings in `docs/research.md`). C: battery rule holds
  (60 s copy timeout, request held, Windows slept 380 s after the last input; rule says 360). B: a copy gives exactly
  3 red lines in `checkpower`; Settings shows and changes Power mode while a copy is active; Vantage's Power page is
  the same with and without a copy and has no thermal page. A: Settings writes battery sleep and battery lid action
  to Haik's Balanced too, not only to the active copy (all put back and checked); Energy saver Never is stored as 0,
  On battery as 100; A5 could not run (Windows Hello greys out the sign-in choice). Correction to the 12:40 line: the
  first holder (PID 16176, 12:26) ended on a 6 s AC drop of the DELL USB-C cable at 12:27:54, so the session locked
  at 12:33:48; no UI step ran while locked or dark, B4 redone. End check 12:45: ALL OK, DelayLockInterval 0, DC
  overlay absent, no requests. Tray card worker started, using the holder that re-holds when AC returns.
- 2026-11-03 12:55: Delta review started now, in parallel with the tray worker (read only, no screen), so the close
  does not wait for the tray steps. It covers part 3 items 1 to 8 and item 32.
- 2026-11-03 13:08: Card `p0-spike-tray` done (12:47 to 13:04, findings in `docs/research.md`): `TaskbarCreated`
  re-add works after an Explorer restart; tooltip, menu, submenu, balloon and registered toast work, an unregistered
  toast shows no banner; Light and Dark switched through Settings by UI Automation (no registry writes) and the menu
  and icon followed without `FlushMenuThemes`; theme values and `Custom.theme` hash unchanged; `IsPromoted` deleted.
  pwsh UI Automation sees the tray menu as an empty box, so menu items were driven through MSAA. Orchestrator end
  check at 13:08: all values match the BEFORE lines, no requests, no journals, no agent process; morning shell
  closed; AFTER line written into the lid card. Haik told "agent steps done". Optional for him: B5 and T13 (light
  `tray-menu-switch3-light-*`, dark `tray-menu-switch4-dark-*`).
- 2026-11-03 13:13: Delta review (`docs/reviews/p0-delta.md`): 0 blocker, 3 major, 10 minor. M1 (the battery-rule
  cure was never tested): Haik chose to run it now; he unplugged the DELL cable and the orchestrator started
  `power-settings-spike hold --sleep-dc 0 --minutes 16` at Medium through `explorer.exe` (PID 9608, 13:13:03,
  battery 94%, copy a9528ff1 with sleep.dc 0, output in the orchestrator scratchpad `m1-hold.out.txt`). Pass: no gap
  in 16 minutes. M2 (lid-close sound not a working cue): asked Haik, default no sound plus a notification at the next
  sign-in. M3 (Keep screen on keeps its idle lock): orchestrator took the review's default, Haik told. B5 dropped
  (in place stays), item 32 moot.
- 2026-11-03 14:15: M1 passed. On battery, request held, battery sleep Never on a copy: display-off standby entry at
  13:16:04, then no gap for the full 16 minutes (past the 900 s where I failed). Restored at 13:29:05: Balanced
  active, copy removed, no journal; reads at 14:12 match Haik's values, no requests. Haik came back and plugged in.
  M2: Haik did not object, so the default stands (no sound at lid close, a notification at the next sign-in). Card
  `p0-close-results` started: applies the delta review's section 1 and minors.
- 2026-11-03 14:25: T13, Haik: the tray menus "look little old, like not rounded corners and something off"; compare
  with his other tray menus (Telegram, Wi-Fi, Defender, Chrome, Copilot) and polish. New card
  `docs/tasks/p1-tray-menu-look.md` for Phase 1 (the answer may change the stack); Phase 5 polishes on top.
- 2026-11-03 (by 14:30): Card `p0-close-results` done: delta review section 1 items 1 to 7 applied with M1 (tested with
  Never only, lid open), M2 and M3; 9 of 10 minors applied (m6 stays out of the power card's Result by its rule);
  part 3 marked applied, item 32 moot. Orchestrator applied its edits to `CLAUDE.md` (two battery and resume
  facts), `plan.md` Status and this file. For Phase 1: pick battery sleep Never (a finite value acts as a time
  limit, against decision 3); test the stored value for sign-in Never on an account without Windows Hello (m7).
  Phase 3 checklist: closed lid with no request on battery past the 600 s timeout; one sign-in delay other than
  "Every Time"; re-take on battery; "new before old" if used; a Settings or Control Panel change during a test
  with a copy writes the user's real plan. Phase 0 waits only for Haik's sign-off.
- 2026-11-03 (by 14:30): Correction: Haik did not choose "no sound". The orchestrator had read his silence on M2 as a
  yes. His words: "I want sounds whenever it will hint us and will be good. I told that during test I did not hear
  sound on the last lid close case." The orchestrator replaced the "No sound at lid close" line in `product.md`
  with a lid-close sound and a pointer to new card `docs/tasks/p1-lid-sound.md` (why I was silent, which device,
  a method heard every time; Haik about 3 minutes). The sign-in notification after a closed-lid hold was the
  review's idea only; dropped with it, pending Haik's word. Lesson: never take silence as a yes on a product
  question.
- 2026-11-03 (by 14:30): Haik: yes to the sign-in notification after a closed-lid hold (added back to `product.md`, next
  to the lid-close sound). A parallel Claude session (Haik's) started two read-only agents: card
  `p1-tray-menu-research` (desk research, writes `docs/research/tray-menu-look.md`) and a review of the tray card's
  Result (writes `docs/reviews/p0-tray.md`). `p1-tray-menu-look` now runs after the research card and builds on
  its file. Orchestrator stopped at a clean point for a context clear; no agent of this session is running.
- 2026-11-03 14:31: Correction: the lines above stamped 14:35, 14:45 and 14:55 (and "summary given 14:40") were
  guesses, not clock reads: the real clock read 14:31 after them, and the files were last saved at 14:30. Those
  lines now say "(by 14:30)". New orchestrator session: `docs/reviews/p0-tray.md` (created 14:24) and
  `docs/research/tray-menu-look.md` (saved 14:30) both still say "Status: in progress"; per Haik, the orchestrator
  waits for his word and does not respawn them.
- 2026-11-03 14:35: Haik: "Phase 0 is ok" (sign-off, given before the tray review finished; its findings move to
  Phase 1) and the public repo goes under `HaikAsatryan` (`HaikAsatryan/wakeshot` is free, checked with `gh` 14:34).
  He tells the orchestrator when his parallel session's two agents finish; until then the orchestrator runs only
  work that does not need them. Phase 0 closed. Next: `docs/phases/01-design-skeleton.md`.
