# Claude Code live shapes

Redacted transcript lines and status file writes from live Claude Code sessions, for the transcript parser tests.
Made 2026-11-03. Findings and the rule changes: `docs/research/claude-code-signals.md`, "Live shapes (2026-11-03)".

- Claude Code 2.1.317, the native CLI (`claude.exe`), on the Windows 11 test laptop. Model Haiku 5.5
  (`claude-haiku-5-5`), `--permission-mode auto` for terminal sessions.
- Sessions ran from 18:18 to 18:40 local time (UTC+4). Transcript `timestamp` values are UTC.
- Terminal sessions ran in a pseudo console driven by a throwaway script (not kept). SDK sessions ran as
  `claude -p --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio
  --allowedTools PowerShell`, the way the VS Code extension runs it (`entrypoint: sdk-cli`).

## Files

Each case has `NN-name.transcript.jsonl` and `NN-name.status.jsonl`; the `/clear` case has two transcripts, `.1`
(before) and `.2` (after). Line numbers below are in the transcript file.

| Case | What ran | Lines that matter |
|---|---|---|
| `01-bg-shell` | PowerShell `run_in_background` (`Start-Sleep 45`) | call 2, result with `backgroundTaskId` 3, notification `completed` 5 (queue) and 7 (user) |
| `02-monitor` | Monitor on a 3-tick loop. The first two Monitors failed (Bash expanded `$_`) | calls 4, 10, 18; results with `taskId` 5, 11, 19; `failed` 9, 15; events 23, 27 (`<event>`, no `<status>`, no `<tool-use-id>`); last one `completed` 31 |
| `03-loop-1m` | `/loop 1m ...`, two fires, then `CronDelete` | `CronCreate` 5 and 6; fires 10, 15; `CronDelete` 19 and 20 |
| `04-dynamic-loop` | `/loop` with no interval; stop file made before the first wakeup | `ScheduleWakeup` 60 s with `noop: true` 5 and 6; fire with `taskKind: "loop"` 10; `ScheduleWakeup` `stop: true` 15 and its result 16 |
| `05-reminder` | "Remind me in 2 minutes": the model used a background Bash sleep. Then a `CronCreate` one-shot | Bash 2 and 3, its notification 15; `CronCreate` `recurring: false` 10 and 11; fire 19 |
| `06-clear-during-loop` | `/loop 1m`, one fire, `/clear`, one fire, `CronList`, `CronDelete` | `.1`: `CronCreate` 5 and 6, fire 10. `.2`: `session_cron_carry` 1, `/clear` 2 to 4, fire 7, `CronList` 13 and 14, `CronDelete` 15 and 16 |
| `07-subagent` | Two background `Agent` runs (the first one gave up after 8 s) | first: call 2, result `async_launched` 3, notification as a user line 13. Second: 17, 18; its notification only as an `attachment` `queued_command` 28, after `queue-operation` `remove` 27 |
| `08-sdk-bg-shell` | SDK path, background PowerShell | 4, 5, notification 8. No `turn_duration` lines on this path |
| `09-resume-clean` | `/loop 5m` and a `ScheduleWakeup` 240 s, `/exit` ("Exit and stop tasks"), then `claude --resume <id>` in a new process | `CronCreate` 28 and 29; `ScheduleWakeup` 34 and 35; written at exit 37 to 45; resumed process from 46; restored fires 50, 70; `CronList` after resume 65 and 66. The wakeup never fired |
| `10-resume-kill` | As 09, but `Stop-Process -Force` instead of `/exit` | `CronCreate` 28 and 29; `ScheduleWakeup` 34 and 35; resumed process from 37; fires 40, 51; `CronList` 46 and 47 |
| `11-resume-one-shot` | `CronCreate` one-shot (18:34) and daily `0 9 * * *`, `/exit`, resume | creates 35 to 38; `CronList` 40 and 41; written at exit 50 to 54; resumed process from 55; `CronList` 58 and 59; one-shot fire 63 |
| `12-sdk-close-during-shell` | SDK path, stdin closed while a 150 s shell ran | 4, 5; the process lived on until the shell ended; notification 8, then exit |
| `13-sdk-resume-kill` | SDK path, hard kill while a 300 s shell ran, then `--resume` over stream-json | task opened 11; resumed process from 15; at start, before any prompt, notification with `<status>stopped</status>` 16 to 18 |
| `14-desktop-session` | Claude Desktop 4.23118.0 Code session (`entrypoint: claude-desktop`, its own Claude Code 2.1.317, auto mode), 2026-11-08: a turn stopped during its command, a background shell, a turn with the window closed, a turn cut by the app's Quit, then the app's own resume in a new process (pid 21052) until Stop. Five helper processes of the app write status files with no transcript | calls 17, 30, 47, 54, 64; Stop results 18, 65; shell result with `backgroundTaskId` 31, its notification 33 (queue) and 35 (user); written at resume start 55; resumed process's error result for the cut call 58 |

Metadata lines (`bridge-session`, `cost-state`, `last-prompt`, `mode`, `permission-mode`, `session-owner`,
`atis-latch`, `ai-title`, `tools-used`, `file-history-snapshot`) are kept in cases 09, 10, 11 and 13 only, because
they are the only lines written at exit and at resume start. They are also written after turns, so they mark
nothing by themselves.

## Format of `.status.jsonl`

One JSON object per line, in the order seen:

- `{"t":<ms>,"file":"sessions/<pid>.json","content":{...}}`: the whole status file after a write.
- `{"t":<ms>,"file":"sessions/<pid>.json","gone":true}`: the file was deleted.
- `{"t":<ms>,"file":".claude/scheduled_tasks.lock","content":{...}}` or `"gone":true`: the lock file in the test
  folder.

`t` is the time a poller (every 100 ms) saw the change, in epoch ms, not the write time. Only real changes are
listed: a re-read after a poller restart is dropped. Pids are real, so two cases can share one: pid 8784 was the
`05-reminder` session and later the resumed `09-resume-clean` session (different `procStart`).

## What was kept from each transcript

Every line except: assistant lines with no `tool_use` (text and thinking), bulky attachments (`prompt_snapshot`,
`deferred_tools_delta`, `deferred_tools_record`, `agent_listing_delta`, `mcp_instructions_delta`, `skill_listing`,
`instructions`, `environment`, `session_context`, `command_permissions`, `credential_org`, `model`, `date`,
`total_tokens_reminder`, `hook_success`, `remote_session_change`), `api-request*` lines, and the metadata lines
listed above (outside cases 09, 10, 11, 13). Kept lines are unchanged in shape: the same keys, in the same order,
only values redacted.

## Redaction

Done by a script that wrote only redacted text. No key file next to a status file was read.

- Every UUID (session, message, prompt, account and organization ids) is a fake `5e55NNNN-0000-4000-8000-...`,
  numbered per case and the same across that case's files. So a status file's `sessionId` matches its transcript.
- `cwd` is `C:\Users\USER\wakeshot-test\<folder>`. `pidDomain` is `win32:HOST`. `messagingSocketPath` is
  `\\.\pipe\LOCAL\cc-msg-PIPE1`. `bridgeSessionId` is `BRIDGE1`: under 8 characters, so the repo's secret scan
  pattern for that key does not match it.
- Every other string is `<str N>` (N = length in characters) unless it is one of these: a `toolu_` id; an ISO time;
  a short id under `id`, `tool_use_id`, `taskId`, `backgroundTaskId`, `agentId` and similar keys; a cron string
  under `cron`; `humanSchedule`; a short enum-like value under `type`, `subtype`, `role`, `status`, `entrypoint`,
  `version`, `kind`, `taskType`, `taskKind`, `cronKind`, `operation`, `model`, `name`, `waitingFor`,
  `permissionMode`, `peerFeatures` and similar keys.
- A string that starts with `<` keeps its tag names (`<task-notification>`, `<command-name>`, ...) and the values of
  `task-id`, `tool-use-id`, `status`, `command-name` and `command-message` only; every other value is `<str N>`.
- Kept as they are: pids, `procStart` FILETIMEs, epoch times (`nextFireAt`, `scheduledFor`, `createdAt`,
  `startedAt`), numbers, booleans, `msg_` message ids, and the derived session `name` (the test folder name).

## Case 14 (Claude Desktop) differs

- The transcript follows the rules above, by a new script that wrote only redacted text (not kept). Metadata lines
  are kept, since the case has a resume; this version adds three kinds: `relocated`, `custom-title`, `agent-name`.
  Dict keys that look like paths would be `<key N>` (there were none). `hostSessionId` is `local_<fake id>`.
- The status writes were not polled: they are rebuilt from `agent-watch --raw`, which logged, per write, the time
  read and the `status`, `entrypoint`, `kind`, `sessionId` start, `statusUpdatedAt` and whether `bridgeSessionId` was
  there. The other keys and their order are those of the live `21052.json`; `name` is `<str 17>`. Only pid 21052 has
  its real `procStart` and `startedAt`; for the others both are set 300 ms before the first write, since their files
  were gone.
- Pid 21052 has no `gone` line: its process and file were still there when the capture ended.
