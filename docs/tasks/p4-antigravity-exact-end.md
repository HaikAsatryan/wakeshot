# Antigravity: see the end of work exactly (Esc, background commands), or leave v1

Phase: 4. Status: blocked for Part B (Haik's answer to Question 1, and `p4-live-cursor` done); Part A done
2026-11-06 19:27 (new 2026-11-06 18:50, Haik's "No guessing" rule in `docs/product.md`). Needs Haik: no for
Part A; the IDE first-run pass only if Part A finds Antigravity can stay (`p4-live-antigravity` Result, Open items 1).
Agent: `wakeshot-worker`. Size: about 150K tokens (Part A about 70K, Part B about 80K).
Runs after: `p4-live-antigravity` (CLI part done). Part A: now, beside `p4-live-cursor`. Part B (code): after
`p4-live-cursor` is done, since that card owns the fixes in `sources/hooked.rs`, `wakeshot-cli/src/hook.rs` and
`agent_hooks.rs` this round.
Parallel with: `p4-live-cursor`, `p6-agents-video-readme` Part C. Not with `p6-agents-install-e2e` Part 2.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-antigravity-exact-end\target`; delete it when done.
Real laptop: Antigravity CLI 1.5.2 for Haik's user (signed in, Free). A scratch capture plugin in
`%USERPROFILE%\.gemini\config\plugins\` while Part A runs, removed at the end. No power settings.

## Goal

Haik, 2026-11-06 18:45: "I do not accept answer that we cannot track whether antigravity works or not. We cannot
guess or randomly sleep. It's better not support something or do so." Today, after an Esc, Antigravity fires no
`Stop`, so Wakeshot shows "working" until the next prompt ends or `agy` exits; and a background command holds only
when `agy` says `fullyIdle: false` (2 of 4 runs). Find an exact signal for both. Build what is exact. What is not
exact is not supported; if the end of a turn after Esc cannot be seen exactly, Antigravity leaves v1.

## Read first

- `docs/product.md` ("No guessing", the hooks line), `docs/agent-hooks.md` (privacy Gate, events, "Live test"),
  `docs/research/antigravity-signals.md` (events table: `PostInvocation`, `PreToolUse`, `PostToolUse` were never
  tested on Esc).
- `docs/tasks/p4-live-antigravity.md` Log (17:33 to 17:50: the Esc and background runs) and Result. Its scratch tools
  are in `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p4-live-antigravity\`
  (`run\`, `ideext\`, `addhooks\`, the capture plugin's source): reuse them.

## Part A: the facts (live, no product code)

1. A scratch capture plugin that logs every hook event `agy` offers (`SessionStart`, `PreInvocation`,
   `PostInvocation`, `PreToolUse`, `PostToolUse`, `Stop`) with times and the non-text fields only (event name,
   `conversationId`, `invocationNum`, `stepIdx`, tool name, `error`, `terminationReason`, `fullyIdle`). Never the
   prompt, `lastUserInput`, tool args or the transcript.
2. Esc in each place a turn can be: during a model call, during a tool run, during an approval wait, and during a
   long streamed answer. For each: which events fire after the Esc and with which fields (an `error` or a cancel
   marker on `PostInvocation` or `PostToolUse` would be exact). At least 3 runs each.
3. Background commands: a dev-server-like command (`ping -n 60`) started in the background, 4 runs or more: which
   events show its start and its end, and whether anything exact tells "a background command still runs".
4. Ctrl+C in the TUI, and a second prompt typed while one runs, if `agy` allows it.
5. Cost: the added time per model call and per tool call with the extra hooks (release `wakeshot.exe`, 10 runs).
6. Write the facts in the Log as you go, then a short table in the Result: case, events seen, exact or not.

Exact means: every state Wakeshot shows maps to an event or to the `agy` process's life, never to a timer or to the
absence of an event. A state that can stay "working" after the work ended is not exact.

## Part B: build or remove (after `p4-live-cursor` is done)

- **If Esc can be seen exactly:** the source follows the exact rule (`sources/antigravity.rs`, the shared hooked
  code, the plugin's event list in `agent_hooks.rs`, the hook helper), each case with a test that fails before; the
  privacy Gate still holds (a test). Background commands: hold only if Part A found an exact start and end; else no
  hold for them at all (ignore `fullyIdle`), with a test.
- **If not:** Antigravity is not supported in v1. Unwire it: the tray no longer watches it and `at_start` no longer
  adds its plugin folder (keep the remove path, so `wakeshot hook antigravity remove` and uninstall still clean an old
  folder). Keep the source code and its tests for later. No contract or `Cargo` change; if one seems needed, stop and
  tell the orchestrator.
- Either way: `docs/agent-hooks.md`, `docs/research/antigravity-signals.md`, `docs/research.md` short lines,
  `docs/product.md` (the agent list line and the hooks line). Checks: `cargo fmt` on your files, `cargo clippy -p`
  and `cargo test -p` for the packages you touched, x64 and ARM64 (ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin`
  folder on that cargo command's PATH only).
- The video and README list is not yours: write the outcome in the Result; the orchestrator passes it on.

## Acceptance checks

- Part A table complete for every case in steps 2 to 4; each "exact" backed by at least 3 runs.
- Part B: every new rule has a test that failed before; the privacy test passes; clippy and tests green on x64 and
  ARM64 for the touched packages.
- Clean-up: the capture plugin removed; every process you started ended by PID; build folder deleted.

## Out of scope

- The Antigravity IDE turn (only if Part A says Antigravity stays; then the orchestrator asks Haik for the IDE
  first-run pass and a worker runs it).
- Any timer, cap or "no event for N seconds" rule: not allowed by Haik's rule.
- `%USERPROFILE%\.cursor` and anything of `p4-live-cursor` (its monitors run until about 21:40).

## Rules

- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply.
- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never run `agent_hooks::at_start`, or a tray that runs it, on the real profile (`p4-live-cursor` tests hooks there).
  A dry-run tray takes turns through
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\dry-run.lock`.
- Start `agy` and drivers at Medium through `explorer.exe`. Kill only processes you started, by PID. Never read token
  files. Prompt text is never logged. Never set `RUSTFLAGS`; cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

1. **May Wakeshot read Antigravity's own log to see Esc and Ctrl+C?** No hook fires on them, but every `agy.exe`
   writes a log file (`%USERPROFILE%\.gemini\antigravity-cli\log\cli-<time>.log`) with a fixed line "Cancelling
   conversation <id>" at each Esc or Ctrl+C (22 of 22 here, about 50 ms after the key). The same file also holds
   every prompt in plain text. Wakeshot would read only those cancel lines (and the first line, which names the
   process) and never keep or log anything else, the same rule as Claude's transcripts, with a test for it. The log
   is not documented, so a later `agy` may change the line. Default: yes, build it this way. If no: Antigravity
   leaves v1, as the card says.

## Found outside scope

1. **`--effort low` is ignored by `agy` 1.5.2 with Gemini 3.8 Flash.** Its log says "failed to apply model override:
   failed to resolve effort: --effort is not supported for the current model" at each start with that flag (seen in
   TUI 2's log). The live cards' minimum-token rule assumes the flag works; the footer still showed "low". Not
   checked what effort the model really ran at.
2. **`agy` keeps every prompt in plain text in its own log** (`HandleUserInput called with text: ...`), besides the
   transcript and `history.jsonl`. Worth a line in the Antigravity privacy notes whatever Part B decides.

## Log

- 2026-11-06 18:50: card written by the orchestrator from Haik's rule.
- 2026-11-06 18:49 (laptop clock): started by a `wakeshot-worker`, Part A only this round (orchestrator). Scratch:
  `<scratchpad>\p4-antigravity-exact-end\`.
- 18:54: release `wakeshot.exe` built (`cargo build -p wakeshot-cli --release --locked`, own target, x64), copied to
  `<scratch>\bin`, build folder deleted. `plugins\` was empty. Scratch capture plugin `wakeshot-xcap` placed at
  Medium at 18:54:36 (`agy plugin list`: enabled; `validate --strict`: one note, no logo). It hooks all six events,
  logs one line per run (event, time, pids, key names, numbers and booleans, `conversationId` cut to 8 chars,
  `terminationReason`, `modelName`, `error` cut to 200 chars, tool name, the key names of `toolCall.args`; every other
  string only as its length; never `lastUserInput`, paths or arg values) and answers `{}`, or `{"decision":"allow"}`
  for `PreToolUse` (docs: "allow" hands the call to the default policy, so approvals still ask). Kernel process trace
  `wsae-proc` from 18:54:36. TUI under the harness driver at Medium (`drive\cli1`, driver 56356, agy 51384, token
  Medium), Gemini 3.8 Flash low. Prompts: 0.
- 18:55, baseline turn (`ping -n 2`, approved): `SessionStart` 44.107, `PreInvocation` 0 at 44.178, `PreToolUse`
  (`run_command`, stepIdx 2, arg keys `CommandLine`, `WaitMsBeforeAsync`, `toolSummary`) at 47.383, before the
  approval prompt showed (47.445); `PostToolUse` (error empty) at 49.207 after the command; `PostInvocation` 0 at
  49.271, so `PostInvocation` comes after the call's tools, not right after the model's answer; `PreInvocation` 1,
  `PostInvocation` 1 at 51.536, `Stop` (`NO_TOOL_CALL`, `fullyIdle` true, error empty) at 51.594. Prompts: 1.
- 18:56 to 18:57, **Esc during a model call, 3 runs**: Esc 1.1 s after Enter, the screen showed "Generating..." and
  "esc to cancel" each time, then "Interrupted · What should Antigravity CLI do instead?". Each run: `PreInvocation`
  0 only; **no `PostInvocation`, no `Stop`, nothing** in the 12 s after (and none came later before the next prompt's
  own events). Prompts: 4.
- 18:57 to 18:58, **Esc during a tool run, 3 runs** (`ping -n 60`, approved, Esc 4 s later at "Running command..."):
  each run `PreInvocation` 0 and `PreToolUse` (`run_command`) only; after the Esc **no `PostToolUse`, no
  `PostInvocation`, no `Stop`**; the screen said "Interrupted". Prompts: 7.
- 18:58 to 18:59, **Esc during the approval wait, 3 runs** (Esc 2 s after "Run this command?"; the menu's last line
  is "4. No, cancel"): each run `PreInvocation` 0 and `PreToolUse` only, then nothing; "Interrupted". Prompts: 10.
- 19:00 to 19:04, **Esc during a long streamed answer, 3 valid runs** (numbers one per line; Esc once a number
  like 150 was on screen under "Working..."; runs at 19:00:16, 19:02:28, 19:04:23): each `PreInvocation` 0 only,
  then nothing; "Interrupted" with the partial answer kept. Two more runs (19:00:38, 19:00:53) matched an old "150"
  still on screen, so their Esc came 0 to 0.1 s after Enter, at the model call's start: the same, `PreInvocation`
  only. Two runs whose marker never came finished normally (19:01:31, 19:03:15): `PostInvocation` 0 then `Stop`
  (`NO_TOOL_CALL`, `fullyIdle` true) about 50 ms later. Prompts: 17.
- 19:05, **background command run 1** (`ping -n 25` "as a background command", approved): `PreToolUse` stepIdx 39 at
  15.984; the command's `powershell.exe` 40416 started at 16.368 (trace); `PostInvocation` 0 at 16.708 with **no
  `PostToolUse` before it**; `PreInvocation` 1, `PostInvocation` 1, `Stop` (`fullyIdle` **false**) at 19.132. Then
  **`PostToolUse` stepIdx 39 at 19:05:41.035, when the ping ended** (25 pings from 16.9 s), then the agent woke
  itself: `PreInvocation` 0 at 41.132, `PostInvocation`, `Stop` (`fullyIdle` true) at 45.139. So an open
  `PreToolUse` (no `PostToolUse` of the same stepIdx yet) marks a running command, foreground or background.
  Prompts: 18.
- 19:06 and 19:07, **background runs 2 and 3**: the same order exactly. `PreToolUse` 45 at 06:39.090, command started
  39.176, `PostInvocation` without `PostToolUse`, `Stop` `fullyIdle` false at 42.827, `PostToolUse` 45 at 07:04.158
  (ping's end), self-wake, `Stop` `fullyIdle` true at 08.029. Run 3: `PreToolUse` 51 at 07:43.935, `Stop` false at
  47.621, `PostToolUse` 51 at 08:08.979, self-wake, `Stop` true at 13.178. Prompts: 20.
- 19:08, **background run 4, with an Esc'd turn while it ran** (`ping -n 35`): `PreToolUse` 57 at 08:51.158, `Stop`
  false at 54.103; footer "1 task(s) · /tasks"; a second prompt at 09:04.871 (`PreInvocation` 0) Esc'd 1 s later:
  nothing more for that turn, and **the background ping kept running**; its `PostToolUse` 57 came at its end,
  09:26.377. After an Esc'd turn the agent did **not** wake itself at the command's end (no `PreInvocation` after
  it, in 20 s). Prompts: 22.
- 19:10 to 19:12, **Ctrl+C, 3 valid runs**: during a model call twice (19:10:05, 19:10:56: "Generating...", then
  "Interrupted · What should Antigravity CLI do instead?"; `PreInvocation` only) and during a foreground tool run
  once (19:11:56, "Running command...": `PreInvocation`, `PreToolUse` only). `agy` stays open; no event after the
  Ctrl+C. One more try (19:10:37) went otherwise: the model put `ping -n 60` in the background by itself although the
  prompt did not ask (so `PostInvocation` came without `PostToolUse`, `Stop` `fullyIdle` false at 40.255), and the
  Ctrl+C at 41 s hit an idle TUI with "1 task(s)": nothing happened, the ping kept running through the next Ctrl+C'd
  turn, and its `PostToolUse` 64 came at its end, 19:11:37.389, with no self-wake after. Background runs: 5.
  Prompts: 26.
- 19:12 to 19:14, **a second prompt typed while one runs, 3 runs** (typed and Enter during a streamed answer): the
  footer says "Press up to edit queued messages"; the first turn ends normally (`PostInvocation`, `Stop` `fullyIdle`
  true), then the queued prompt runs as its own turn: `PreInvocation` 0 came 145, 460 and 285 ms after that `Stop`,
  then its own `Stop`. One more run with **Esc while a prompt was queued** (19:14:40): the first turn is
  "Interrupted", the queued prompt does not run (it goes back to the input line), no event. Prompts: 34 sent (each
  of these runs sends two), 33 reached a model.
- 19:15, trace cut (`proc1`): `agy` runs each command as `powershell.exe` (its child) with `PING.EXE` under it. On
  Esc or Ctrl+C during a foreground command the `powershell.exe` ends at once with exit 1 (18:57:39.422,
  18:58:10.511, 18:58:33.690, 19:11:56.942). A background command's `powershell.exe` runs to its own end; its
  `PostToolUse` came 50 to 65 ms after that exit (19:05:40.971 to 41.035; 19:07:04.097 to 04.158).
- 19:16, **a signal outside the hooks: `agy`'s own log.** Each `agy.exe` writes
  `%USERPROFILE%\.gemini\antigravity-cli\log\cli-<start time>.log` (glog lines; the first names its pid: "Starting
  language server process with pid 51384"), and `antigravity-cli\cli.log` is a **hard link** to the newest process's
  file (`fsutil hardlink list`: both names). Read with the prompt text cut out by `sed` (the log holds every prompt:
  "HandleUserInput called with text: ..."). **Every Esc and Ctrl+C of this card wrote, within 1 ms of the key,
  "Cancelling in-progress response for conversation <id>" and "Cancelling conversation <id>
  (killBackgroundTasks=false)"**: 19 of 19 in TUI 1 (model call 6, one of them in background run 4; tool run 3;
  approval 3; stream 3; Ctrl+C 3; queued prompt 1), and the approval ones add "Interrupt cleared pending tool
  confirmation: "RunCommand"". No normal turn and no Ctrl+C on an idle prompt wrote it. The
  earlier card's one Esc (17:33:32) is in its own file the same way. Not documented anywhere I found; the file:line
  part (`server.go:2498`) will change between builds, the message text may.
- 19:18 to 19:19, **two TUIs at once, the cancel line timed, 3 runs**: a second TUI (`drive\cli2`, driver 46684, agy
  51604, Medium) made its own `log\cli-20261106_191814.log`, and `cli.log` moved to it (hard link list). A scratch
  poller (`run\logtail.py`, reads by handle every 50 ms, prints only the time and the id of a cancel line) saw:
  Esc in TUI 2 during a model call (key 19:18:27.849, line logged 27.901, seen 27.965), Esc in TUI 1 during a model
  call (43.234, 43.286, 43.316: in TUI 1's own file, though `cli.log` names TUI 2's), Esc in TUI 2 during a tool run
  (19:19:05.476, 05.527, 05.556). So each process keeps its own file, the line is flushed at once (about 50 ms after
  the key), and a reader sees it within one poll. Hooks: `SessionStart` and `PreInvocation` for the new chat, then
  nothing after each Esc, as before. Prompts: 37.
- 19:19 to 19:20, **a background task killed from `/tasks`** (TUI 2, `ping -n 90` in the background): this run's
  `run_command` args had one more key, `IsDaemon`, and its `Stop` said **`fullyIdle` true** while the ping ran (the
  earlier runs without that key said false; values not logged, so "daemon tasks do not block idle" is likely, **not
  verified**). `/tasks` lists "Agent Backgrounded ... running" with "k Kill Task". `k` at 19:20:05.682: the list says
  "cancelled", the ping is gone (process list at 19:20:20), **no hook runs** (no `PostToolUse` for stepIdx 6), and
  the log wrote **"Cancelling steps [6] in conversation <id>"** at 19:20:05.685, the stepIdx of its `PreToolUse`. One
  run. Prompts: 38.
- 19:21 to 19:24, **cost, release exe**: dry-run lock taken 19:21:01. A scratch plugin `wakeshot-xcost` (placed at
  Medium 19:21:20, removed 19:23:33) ran the release `wakeshot.exe --dry-run hook antigravity <event>` for
  `PreInvocation`, `PostInvocation`, `PostToolUse` and `Stop` (not `PreToolUse`: Wakeshot answers `{}` and that
  event wants a decision). Three prompts of four `echo` tool calls each in TUI 2: 45 hook runs (15 `PreInvocation`,
  15 `PostInvocation`, 12 `PostToolUse`, 3 `Stop`), timed in the kernel trace (`run\cost.py`; the event of each run
  is the nearest capture line, so the split by event is off by a few runs). `cmd` start to end: 24 to 53 ms, one
  outlier 1138 ms (a `PreInvocation`; cause not checked); `wakeshot.exe` inside: 12 to 55 ms. The runs with a rule
  (`PreInvocation`, `Stop`) took about 30 ms in the exe (median; `cmd` about 48 ms); `PostInvocation` and
  `PostToolUse`, which have no rule today and only drain the input, about 16 ms (`cmd` about 30 ms). So with a rule
  on each, the extra hooks would add about 50 ms per model call (`PostInvocation`) and about 100 ms per tool call
  (`PreToolUse` and `PostToolUse`), against model calls of 2 to 12 s here. The hooks made
  `%LocalAppData%\Wakeshot\dry-run\agents\antigravity\` (empty now: each `Stop` deleted its file); the folder stays.
  Lock deleted 19:24:18. Prompts: 41.
- 19:24, **the agent stops its own background task, 1 run**: `PreToolUse` `run_command` 41 (with `IsDaemon`), then
  `PreToolUse`/`PostToolUse` `manage_task` 43 (arg keys `Action`, `TaskId`; values not logged), screen "Ran 1 command,
  killed 1 task (... task-41)", ping gone. **No `PostToolUse` for step 41 and no log line** (no "Cancelling steps").
  So after such a kill only `manage_task`'s args (not read) or the command's process end tell that the task ended.
  `Stop` `fullyIdle` true. Prompts: 42 sent, 41 reached a model.
- 19:26, clean-up: both TUIs ended by `/exit` (agy 51384 and 51604 gone), both drivers ended by `end` (result pass,
  `active_after` 0, exit 0), log poller stopped by its STOP file, trace session `wsae-proc` stopped (no such session
  left), capture plugin removed at Medium 19:26:17 (`agy plugin list`: `[]`; `plugins\` empty, as at the start).
  Every pid checked gone (56356, 46684, 51384, 51604; no `agy.exe`, `agent-drive.exe` or `PING.EXE` left). Release
  exe copy and build folder deleted. Part B does not need the capture plugin; its source stays in the scratch folder
  (`capture\`, `run\make_plugins.py`). `docs/research.md` lines are not written this round (orchestrator: card only);
  Part B writes them from this Log.

## Result

**Part A done, 2026-11-06 19:27.** Antigravity CLI 1.5.2, Gemini 3.8 Flash, two TUIs at Medium, 42 one-line test
prompts (41 reached a model). Every step is in the Log with times. No product code, no doc outside this card.

| Case | Hook events seen after the key or the end | Other exact signal | Exact? |
|---|---|---|---|
| Normal turn (baseline, 9 runs) | `PreInvocation` ... `PostInvocation` (after the call's tools) ... `Stop` | `agy` exit | Yes, by hooks |
| Esc during a model call (8 runs) | None after `PreInvocation` | Log line "Cancelling conversation <id>" | Hooks: no. With the log: yes |
| Esc during a tool run (4 runs) | None after `PreToolUse`; the command's process is killed | Log line; process end | Hooks: no. With the log: yes |
| Esc during the approval wait (3 runs) | None after `PreToolUse` | Log line, plus "Interrupt cleared pending tool confirmation" | Hooks: no. With the log: yes |
| Esc during a streamed answer (3 runs) | None after `PreInvocation` | Log line | Hooks: no. With the log: yes |
| Ctrl+C in a turn (3 runs) | Same as Esc; `agy` stays open | Log line | Hooks: no. With the log: yes |
| Second prompt typed while one runs (3 runs) | Queued; first turn's `Stop`, then the second's `PreInvocation` 0.15 to 0.46 s later, then its `Stop` | | Yes, by hooks |
| Esc with a queued prompt (1 run) | None; the queued prompt does not run | Log line | As Esc (1 run only) |
| Background command, natural end (4 runs, plus 2 seen in other runs) | `PreToolUse` N, then `PostInvocation` with no `PostToolUse` N (the command went to the background), `PostToolUse` N at the command's end (50 to 65 ms after its process exit) | The command's `powershell.exe`, a child of `agy.exe` | Yes, by hooks or by the process |
| Background command killed from `/tasks` (1 run) | None (no `PostToolUse` N) | Log "Cancelling steps [N]"; process end | Hooks: no. Process: yes |
| Background command stopped by the agent (`manage_task`, 1 run) | Only `manage_task`'s own pair (its `TaskId` is in the args, not read) | No log line; process end | Hooks: no. Process: yes |
| `fullyIdle` on `Stop` while a background command runs | `false` in 5 runs, `true` in 2 runs whose args had the `IsDaemon` key | | No: ignore it |

Exact here means Haik's bar: each state maps to an event or a process's life, never to a timer or to a missing
event. **By hooks alone, Esc and Ctrl+C are not exact** (no event in 22 interrupts), so by the card's rule Antigravity
would leave v1. **Agy's own log is exact for them**: each `agy.exe` writes its own `log\cli-<start time>.log` (first
line names its pid; `cli.log` is a hard link to the newest), the cancel line came for 22 of 22 interrupts I timed or
counted, about 50 ms after the key, flushed at once, and never for a normal turn.

**My call for Part B: build the exact rule, for the CLI, if Haik says yes to Question 1** (reading that log, which
also holds every prompt). Design for Part B, each with when to change it:

1. Working: `PreInvocation` to `Stop`, or to a cancel line for that chat in that `agy.exe`'s log with a time after
   the turn's last hook run, or to that process's end. If no log names the pid, the chat shows "update Wakeshot" and
   does not hold (no guess). Change it when an `agy` update drops or rewords the line (a live re-test sees it): then
   gate on the versions tested.
2. Background commands: hold while a command process that `agy.exe` started (its `powershell.exe` children) still
   runs after a turn ended, by process life; ignore `fullyIdle`. This covers the natural end, `/tasks` kill, the
   agent's own kill, Esc (background tasks live on: `killBackgroundTasks=false`) and `agy` exit, with no extra hooks.
   Not verified: whether other children of `agy` (MCP servers, plugin sidecars) are also `powershell.exe`; Part B
   must tell them apart (the command line form, checked without keeping text) or test that none exist. Change it
   when `agy` runs commands through another shell.
3. No new hooks needed: `PostInvocation`, `PreToolUse` and `PostToolUse` add nothing the process and the log do not
   give. Cost if they were added (release exe, 45 runs): about 50 ms per model call and 100 ms per tool call.
4. The IDE stays out until its live test (the hub's log place is unknown).

If Haik says no: Part B unwires Antigravity as the card's "If not" says.

**Files changed:** this card only. Scratch tools (capture plugin source, `make_plugins.py`, `logtail.py`,
`cost.py`, `tree.py`, step files) stay in `<scratchpad>\p4-antigravity-exact-end\` for Part B.

**Acceptance checks (Part A):** table complete for steps 2 to 4, each "exact" or "not exact" backed by 3 or more
runs, except the two kill paths and the Esc with a queued prompt (1 run each, marked). Cost measured on the release
exe (45 runs). Clean-up: capture and cost plugins removed (`plugins\` empty, as before); every process I started
ended (drivers by `end`, `agy` by `/exit`; pids checked gone); trace stopped; dry-run lock deleted; build folder and
exe deleted. Left behind: the empty folder `%LocalAppData%\Wakeshot\dry-run\agents\antigravity\` (made by the cost
hooks), two test chats and their logs under `%USERPROFILE%\.gemini\antigravity-cli\` (Antigravity's own).

**Open items:** Question 1. Not tested: `killBackgroundTasks=true` (when it comes), `/clear` or `/new` during a turn,
a turn that ends on a quota or network error, `/btw`, the IDE.
