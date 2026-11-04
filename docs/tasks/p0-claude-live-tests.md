# Confirm Claude Code signals in live sessions

Phase: 0. Status: done. Needs Haik: no. The worker drove test sessions itself; only an optional VS Code UI check is left for him.
Runs after: `p0-toolchain`. Parallel with: any card that does not change power settings.

## Goal

Confirm what static reading of Claude Code could not: timers, background shells, waiting states, Remote Control.

## Read first

- `CLAUDE.md` (never read `.key` files)
- `docs/research/claude-code-signals.md`, especially the live tests list

## Deliverables

- `spikes/claude-watch/`: a small watcher that logs every change to `~/.claude/sessions/*.json` (never `*.key`) and
  the transcript lines about scheduling and background work for the matching session.
- Exact numbered steps for Haik for each live test, written into this card.
- Results table added to `docs/research/claude-code-signals.md`; summary in `docs/research.md`.
- Proposed changes to the detection rules in Result.

## Acceptance checks

- Each live test has an observed outcome or "could not run" with the reason.
- The watcher never opened a `.key` file (state how this is enforced in code).

## Out of scope

Codex and Copilot live tests (Phase 4). Product code.

## Questions for Haik

## Found outside scope

- Side effects of the test sessions on Haik's setup, all made by Claude Code itself: three scratch folders
  (`<scratch>\cctest`, `cctest2`, `cctest3`) are now marked trusted in `~/.claude.json`; the test sessions and their
  transcripts are in `~/.claude/projects/C--Users-haika-AppData-Local-Temp-claude-...-scratchpad-cctest*`; they
  showed up in the Remote Control session list while they ran. Harmless; delete those project folders if unwanted.
- In test session t3 (default permission mode), Claude Code's PowerShell permission check printed "Command contains
  malformed syntax that cannot be parsed: pwsh exited with code 1: The command line is too long." for a plain
  `New-Item -ItemType File -Path hello.txt`. Seen once; cause not checked. A Claude Code issue, not Wakeshot.
- Another Claude Code session not started by this worker (pid 32332, cwd `C:\Users\haika\code\wakeshot`, CLI)
  started at 01:50:44. Left alone.

## Log

- 2026-11-03: card written.
- 2026-11-03: worker started. Plan: Rust watcher in `spikes/claude-watch/`, then run as many live tests as possible
  without Haik by driving test `claude` sessions in a pseudo console; leave only the rest for Haik.
- 2026-11-03 01:27: watcher `spikes/claude-watch/` built (15 unit tests, clippy and fmt clean) and running with
  `--kinds`.
- 2026-11-03 01:28: test driver `spikes/claude-watch/ccdrive.py` (pywinpty + pyte, venv in the scratch folder). It
  strips `CLAUDE*`, `CLAUDECODE` and `AI_AGENT` from the env: this shell has `CLAUDE_CODE_CHILD_SESSION` set, which
  would stop the test session from writing a status file. Test sessions use `--model haiku` to save tokens; cwd is
  `<scratch>\cctest`.
- 2026-11-03 01:30: a plain `\r` does not press Enter in Claude Code 2.1.316 under ConPTY; the kitty keyboard
  sequence `ESC [13u` does. Arrow keys work as plain VT.
- 2026-11-03 01:31: session t1 (pid 33448). During the trust dialog only `33448.<hash>.key` exists; `33448.json`
  appears after the dialog, first without `status` and `updatedAt`, then `status: idle` 0.7 s later, then
  `bridgeSessionId` 1 s later in a write that did not change `updatedAt`.
- 2026-11-03 01:31: test 1 (background shell): `idle -> busy` on prompt, `busy -> shell` at turn end (01:31:58.5).
  Transcript: `tool_use PowerShell` with `run_in_background: true`, result `backgroundTaskId: bz2rc6s47`; task file
  `bz2rc6s47.output` created. At 01:33:57 the shell ended: `shell -> busy` (the task notification starts a short turn,
  transcript user line with `<task-notification>` status `completed`), then `busy -> idle` at 01:34:01. No direct
  `shell -> idle`.
- 2026-11-03 01:34: test 2 (shell Monitor, 6 ticks 20 s apart): `busy -> shell` at turn end; each tick gives
  `shell -> busy` (a 1.6 to 2 s turn) and back to `shell`; the last event (status `completed`) gives `busy -> idle`.
  Task file `b0pdi64jp.output` appears only with the first output line.
- 2026-11-03 01:35: session t2 (pid 21600, folder `cctest2`). Test 5 (one-shot reminder "in 3 minutes"): Haiku chose
  `ScheduleWakeup` (`delaySeconds: 180`), not `CronCreate`. Result `scheduledFor` 01:39:00.000 (a whole minute),
  `clampedDelaySeconds: 180`. `cctest2\.claude\scheduled_tasks.lock` appeared at the same moment with pid,
  procStart, sessionId, acquiredAt.
- 2026-11-03 01:36: test 6 (durable job): not possible in this build. The CronCreate tool description in the live
  session says "durable persistence is not available. All jobs are session-only". The model created a session-only
  job (`durable: false`, cron `7 9 * * *`, id `db297479`, `nextFireAt` 09:37:00: the 30 min jitter cap on a daily
  job). No `scheduled_tasks.json` was written. Deleted it with `CronDelete` (`input.id`, result `{id}`), both seen.
- 2026-11-03 01:38: the first t1 died: my driver crashed on a file-replace race, its pseudo console closed, and
  Claude Code exited cleanly (status file, `.key` and `scheduled_tasks.lock` all removed). Fixed the driver. Found:
  `nextFireAt` is a float (`1793655541131.567`); watcher fixed to read it as f64. New t1: pid 33008.
- 2026-11-03 01:39: test 5 fire: `scheduled_task_fire` at 01:39:00.391 with `taskKind: "loop"`, `cronKind: "loop"`,
  `cron: "39 1 * * *"`, `taskId: "52d5f0e6"`. That id was not in the `ScheduleWakeup` result, so a reader must match
  a wakeup to its fire by time. The fired prompt is a user line with `isMeta: true`. Status `idle -> busy -> idle`.
- 2026-11-03 01:40: test 3 (`/loop 1m ...`): `CronCreate` cron `*/1 * * * *`, id `653b6114`, `nextFireAt`
  01:41:11.863 (jitter `frac(id) * 30 s` = 11.86 s, exact). Fires at 01:41:12.6 and 01:42:12.6 (`scheduled_task_fire`
  with `taskId`, no `taskKind`), each a 4 s `busy` turn; `idle` between. Lock file in `cctest\.claude` holds pid
  33008.
- 2026-11-03 01:41: session t3 (pid 8516, `--permission-mode default`). Test 7: permission prompt gives
  `busy -> waiting`, `waitingFor: "permission prompt"`; Esc gives `idle`. AskUserQuestion gives `waiting`,
  `waitingFor: "input needed"`.
- 2026-11-03 01:42: test 8: opening `/remote-control` gives `waiting`, `waitingFor: "dialog open"`. "Disconnect this
  session" writes `bridgeSessionId: null` (key kept, value null) and `idle` in one write.
- 2026-11-03 01:43: test 12: `claude agents --json` (env scrubbed) took 0.21 s, exit 0, no new `git` processes.
  Output per session: `pid`, `cwd`, `kind`, `name`, `sessionId`, `startedAt`, `status`. t1 had status file `shell`
  (loop plus background shell) and was reported `busy`. No `state` field for interactive sessions.
- 2026-11-03 01:44: test 9 (`/clear` with the loop armed): status file `sessionId` changed to `d267121d-...` in a write
  of its own. The new transcript starts with an `attachment` of type `session_cron_carry`:
  `{"tasks":[],"otherArmed":[{"id","cron","prompt","createdAt","recurring"}]}`. Background shell kept running.
  The next fire (01:45:11.9) went on in the same process, and its `scheduled_task_fire` line landed in the new
  transcript. Deleted the job with `CronDelete`; `scheduled_tasks.lock` stays after the last job is gone.
- 2026-11-03 01:46: test 4 (dynamic loop, "wait about 2 minutes"): `ScheduleWakeup` `delaySeconds: 120` at 01:46:27.7
  gave `scheduledFor` 01:49:00.000, so wakeups are rounded up to a whole minute (also seen in test 5). Esc at the
  idle prompt at 01:46:58: the screen said "Pending wakeup cancelled", no transcript line was written, and nothing
  fired at 01:49:00. So an Esc cancel is invisible to rule 5.
- 2026-11-03 01:47: test 11 (crash): `Stop-Process -Force` on t3 (pid 8516). `8516.json` and its `.key` stayed. Watcher:
  `alive, start time matches -> dead (exited)` within 1 s, then `OpenProcess` fails (0x80070057). At 01:47:48 a new
  Claude Code process started and the stale `8516.json` and `.key` vanished in the same instant: Claude Code sweeps
  dead sessions' files at start.
- 2026-11-03 01:47: test 10, SDK path (`claude -p --input-format stream-json --output-format stream-json
  --permission-prompt-tool stdio`, the way the VS Code extension runs). Status file: `entrypoint: "sdk-cli"`,
  `kind: "interactive"`, `status: "idle"` in the first write. A `can_use_tool` request gives `waiting` /
  `permission prompt`; the answer gives `busy`. Closing stdin ends the process cleanly (file removed). A background
  shell (`bjd8jcv33`) gives `busy -> idle` at the result: no `shell` value on this path.
- 2026-11-03 01:51: test 10 cont.: the shell's end gave a 1.5 s `busy`. A background subagent kept `busy` from the turn
  result (01:51:46) until it ended (01:52:16) and its notification turn finished (01:52:20).
- 2026-11-03 01:59: loop fallback (t2, after the 01:39:00 wakeup fire with no reschedule): a hidden
  `scheduled_task_fire` (`taskKind: "loop"`, `cron: "0 2 * * *"`, new `taskId`) at 01:59:24.6, with no transcript line
  before it. All test sessions closed with `/exit` or stdin close; watcher and drivers stopped by PID.
- 2026-11-03 02:01: results table, proposed rule changes and open steps written to
  `docs/research/claude-code-signals.md`; summary in `docs/research.md`. Final fmt, clippy, tests pass.

## Result

Done. All live tests ran without Haik, except the optional VS Code UI check and the Desktop app (not installed).

What was done:

- `spikes/claude-watch/` (Rust): polls `~/.claude/sessions` every 200 ms and logs each status file change (field
  diff, write lag), the pid plus start time check, the matching transcript lines about timers and background work
  (text redacted to lengths), task output files, and `scheduled_tasks.json` / `.lock` in each session folder.
  `--kinds` logs every other transcript line kind. 16 unit tests.
- `spikes/claude-watch/ccdrive.py`: test driver. Runs `claude` in a pseudo console (pywinpty + pyte) or over
  stream-json (SDK path), so the tests ran with no person at the keyboard.
- 14 live results (13 planned plus the loop fallback) in `docs/research/claude-code-signals.md`, section "Live test
  results"; marks in that file updated; summary in `docs/research.md`.

How `.key` files are never opened: in `names.rs`, `StatusFile::read` is the only code that reads a file in the
sessions folder. A `StatusFile` can only be made from a name that is exactly `<digits>.json`, and the name is
checked again right before the open. Listing the folder reads names, not contents. Test
`only_pid_json_files_are_opened` records every open and checks that only `<pid>.json` was opened while a
`.key` file with valid JSON sat next to it.

Acceptance checks:

- Each live test has an outcome: pass. Tests 1 to 5 and 7 to 12 observed. Test 6 could not run as planned: durable
  jobs are off in this build (the tool says so); that state was observed. Test 13 could not run: Desktop not
  installed. Test 10 ran on the stream-json engine path, not through the VS Code UI.
- Watcher never opens `.key`: pass, enforced as above.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (16): pass.

Proposed detection rule changes (full text in `docs/research/claude-code-signals.md`, "Proposed rule changes"):

1. SDK-path sessions have no `shell`: track background shells and Monitors from the transcript (`backgroundTaskId`
   or `taskId` opens, a `<task-notification>` with a final status closes).
2. `waitingFor: "dialog open"` counts as `idle`.
3. Read `nextFireAt` and `scheduledFor` as floats. A `ScheduleWakeup` has no id: drop it at its fire or once
   `scheduledFor` passes plus a margin, which also covers the invisible Esc cancel.
4. On a `sessionId` change for the same pid, keep the jobs and read `session_cron_carry` in the new transcript.
5. Handle reminders made with either `CronCreate` or `ScheduleWakeup`.
6. Durable jobs are off: rule 7 is low priority.
7. `claude agents --json` only for `bg` and daemon kinds.
8. Rule 2 confirmed; leftovers also vanish at the next Claude Code start.
9. Do not use `updatedAt` as a heartbeat.
10. Loop fallback: one hidden fire about 20 min after a loop fire with no reschedule. Proposed default: do not hold
    the PC awake for it.

Open items:

- Optional, Haik: the VS Code UI check, steps in `docs/research/claude-code-signals.md`, "Live tests still open".
  Not blocking; expected result `idle` during a background shell.
- The test session side effects listed under "Found outside scope".
