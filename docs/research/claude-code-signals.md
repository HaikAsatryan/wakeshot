# Claude Code signals on Windows

Research on 2026-11-03 on the Windows laptop. Goal: know from outside, without hooks and without editing Claude
settings, that a Claude Code session is working now or will start work soon.

Marks: **verified** means seen in the binary code, on disk, or in the official docs. **Unverified** means a guess or
an inference that still needs a live test.

Binary read: `C:\Users\haika\.local\bin\claude.exe`, version 2.1.316, build 2026-10-30, 335,469,216 bytes. It is a Bun
build. The readable JavaScript sits at byte offsets 280,000,000 and up. Offsets below point into that file. Minified
names (like `X3e`) are from this build only and change every release.

## Short answer

- `status` in the session file is `busy`, `idle`, `waiting` or `shell`. `busy` stays on while background subagents,
  workflows or teammates run after the turn ends. `shell` means a background shell or a shell Monitor runs while the
  main thread is idle. Both are working signals.
- An idle session with a pending `/loop`, `CronCreate` job or `ScheduleWakeup` wakeup shows `idle`. Nothing in the
  session file tells you a timer is pending. Session-only jobs (the default) live only in process memory.
- The only outside traces of pending session-only timers are in the session transcript `.jsonl`: the `CronCreate` and
  `ScheduleWakeup` tool calls with their results (next fire time in epoch ms), `CronDelete` calls, and
  `scheduled_task_fire` lines. On `--resume` Claude Code itself rebuilds its `CronCreate` jobs from these same lines,
  but not `ScheduleWakeup` wakeups for a terminal or SDK session ("Phase 4 live shapes", "Resume findings"). So
  idle-but-scheduled can be detected, with medium confidence, by replaying the transcript.
- Durable jobs (`durable: true`, rare) are in `<project>/.claude/scheduled_tasks.json`. Exact and easy to read.
- On the SDK path (VS Code extension, `claude -p` with stream-json) there is no `shell` value: a background shell
  shows `idle`. Only the transcript shows it. Verified live.
- Background sessions (`claude --bg`) are different: `claude agents --json` reports `state: working` while a `/loop`
  wakeup is pending. Haik does not use them today.

## Status file fields

File: `%USERPROFILE%\.claude\sessions\<pid>.json`. One per running session. Written by the registration function at
offset 285,742,500 (`jY`) and updated by `Rn` (offset 285,744,000) and `X3e` (offset 285,744,381).

| Field | Meaning | Values | When written | Mark |
|---|---|---|---|---|
| `pid` | Process id of the session | number | At registration | verified (code, disk) |
| `sessionId` | Conversation UUID; transcript file name | UUID | Registration; rewritten when the conversation changes (for example `/clear` or resume inside the session) | verified (code); `/clear` case verified live |
| `cwd` | Working folder | path | Registration; rewritten when the session changes folder | verified (code) |
| `startedAt` | Session start, epoch ms | number | Registration | verified |
| `procStart` | Process start time. On Windows a FILETIME as a string | string | Registration | verified: `134381211391959336` equals `Get-Process 20636` StartTime `2026-11-02T23:25:39.1959336+04:00` exactly |
| `procStartFt` | Alternative start time field. Readers use `procStartFt ?? procStart` | string | Not seen on Windows | verified (reader code at 299,481,500) |
| `version` | Claude Code version | `2.1.316` | Registration | verified |
| `peerProtocol`, `peerFeatures` | Session-to-session messaging protocol | `1`; `notify_idle`, `artifact_yield` (plus one more feature on macOS/Linux) | Registration | verified |
| `kind` | Session kind | `interactive`, `bg`, `daemon`, `daemon-worker` (zod schema `Wg` at 285,734,956) | Registration, from `CLAUDE_CODE_SESSION_KIND` | verified (code) |
| `entrypoint` | Which client started it | seen: `cli`, `claude-vscode`. Readers also know `claude-desktop`, `claude-desktop-3p`, `local-agent` (mapped to "desktop") | Registration, from `CLAUDE_CODE_ENTRYPOINT` | verified (disk for two values, code for the rest) |
| `hostSessionId` | Host session id for hosted runs | string | Registration, only in some hosted modes | verified (code) |
| `pidDomain` | Machine id for the pid | `win32:p16s` | Registration | verified |
| `asksForConvertedSkills` | Internal skills flag | bool | Registration and on change | verified |
| `tmux` | tmux pane id | string | Registration, only inside tmux | verified (code) |
| `messagingSocketPath` | Named pipe for peer messages. Never connect to it | `\\.\pipe\LOCAL\cc-msg-<hex>` | Registration and on change | verified |
| `name`, `nameSource`, `nameSince`, `formerNames` | Display name | `nameSource`: `derived`, `user`, `auto`, `peer` | Registration and on rename | verified |
| `logPath`, `agent`, `jobId`, `spare` | Debug log path, agent name, background job id, spare worker flag | strings, bool | Registration, only when the matching env var or bg mode is set | verified (code) |
| `parkedJobId` | Session was sent to the background; the job id now owns the work | string | When parked; cleared on return | verified (code) |
| `status` | Current activity, see next section | `busy`, `idle`, `waiting`, `shell` | Each time the value changes | verified (code); all four values seen live |
| `waitingFor` | What a `waiting` session waits on | see next section | With `status` | verified (code) |
| `updatedAt` | Last write, epoch ms | number | Most updates. Missing in the very first write. The write that adds `bridgeSessionId` keeps the old value | verified (code); exceptions seen live |
| `statusUpdatedAt` | Last status change, epoch ms | number | Only when `status` is part of the write | verified |
| `bridgeSessionId` | Remote Control session id (`session_...`, same id as the claude.ai/code URL) | string or `null` | When Remote Control connects; set to `null` when it disconnects (calls at 285,744,100 and in the bridge code) | verified (code); `null` case verified live |
| `state`, `detail`, `tempo`, `needs` | Text classifier result for agent view | `tempo`: `active`, `idle`, `blocked` | Only while agent view (`claude agents`) is open. It touches `sessions\.fleetview-heartbeat` and sessions write these only if that file is under 5 s old (offset 313,430,796) | verified (code) |

Write behavior:

- No heartbeat. The file changes only when a field changes. This session stayed `busy` with `statusUpdatedAt`
  00:03:57 and no rewrite through 00:20:45. **Verified** on disk.
- Writes use plain `fs.writeFile` (truncate, then write), not write-then-rename (`writeFile as jg`, offset 285,732,236).
  A reader can catch an empty or half-written file. Issue anthropics/claude-code#96438 (open) shows files with
  leftover bytes after a valid JSON object. Parse the first JSON value only, and retry once after a short wait on
  failure. **Verified** (code and issue).
- Deleted on clean exit (`process.on("exit")` with unlinkSync, plus a cleanup handler). A crash leaves the file.
  **Verified** (code and live). Live: `/exit`, closing the console, and closing stdin on the SDK path all remove the
  `.json` and the `.key`. After a hard kill both stay, and the next Claude Code start removes them.
- No file at all for: diskless mode, teammate agents running in their own process (`dynamicTeamContext.agentId`), and
  child sessions (`CLAUDE_CODE_CHILD_SESSION`). Check `Yg()` at 285,744,200, `hdt()` at 284,807,239, `X()` returns
  `launchOptions.diskless()`. **Verified** (code).
- The VS Code extension runs its own copy (`...\.vscode\extensions\anthropic.claude-code-2.1.314-win32-x64\resources\native-binary\claude.exe`)
  and writes the same file with `entrypoint: claude-vscode`. **Verified** (disk, pid 3032).

## Status values and transitions

Interactive (terminal) sessions. Function `VAe` at offset 318,615,892, written from a React effect at 318,627,389:

```js
function VAe(h){let x=oBo(h);if(x!==void 0)return{status:"waiting",waitingFor:x,working:!1};
 return{status:h.isLoading||h.delegatedActive?"busy":"idle",waitingFor:void 0,working:h.isQueryActive}}
...
Yo = lo==="idle" && hasRunningShellTask ? "shell" : lo
X3e({status:Yo,waitingFor:Co})
```

| Status | Set when | Cleared when | Mark |
|---|---|---|---|
| `busy` | A turn runs (`isLoading`), or any task of type `local_agent`, `remote_agent`, `in_process_teammate`, `local_workflow` is not finished (`j2o` set at 289,898,085). Excluded: idle teammates, and `remote_agent` with `isLongRunning` | The turn ends and no such task runs | verified (code). Verified live: this session's turn ended at 00:17:39, status stayed `busy` while 4 background subagents ran |
| `shell` | Status would be `idle`, but a `local_bash` task is not finished (`W2o` at 289,898,696). That covers `run_in_background` shells and the shell `Monitor` tool (it creates `local_bash` with `kind: "monitor"`) | The shell task ends, or a turn starts (then `busy`) | verified (code and live) |
| `waiting` | A dialog needs the user. `waitingFor` is: `permission prompt` (default for dialogs), `input needed` (AskUserQuestion or MCP input request), `worker request`, `sandbox request`, `goal proposal`, `dialog open` (a slash command dialog, not a real block) | The dialog closes | verified (code at 318,615,892 and `uVe`) |
| `idle` | None of the above | | verified |
| (missing) | Right after registration, before the first status write | | verified (code) |

Not counted as work in the status (they leave it `idle`): pending `/loop`, `CronCreate` and `ScheduleWakeup` timers,
MCP monitors (`monitor_mcp`, `monitor_ws`), MCP tasks, `dream` (memory consolidation) and other internal tasks.
**Verified** (code).

SDK path (VS Code extension, `claude -p`, likely Desktop). Function `px` at 320,295,916 maps the SDK session state:
`running` to `busy`, `requires_action` to `waiting` (`waitingFor` = `input needed` for AskUserQuestion or dialogs,
else `permission prompt`), `idle` to `idle`. No `shell` value on this path. Verified live on the stream-json path:
`busy` stays on while a background subagent runs after the turn, but a background shell shows `idle`.

Claude Code's own `claude agents --json` reads the same files and maps any status other than `idle` and `waiting` to
`busy` (function `p` at 324,351,452). So `shell` shows as `busy` there. **Verified** (code). Good rule to copy:
unknown status means busy.

Claude Code's own macOS sleep inhibitor (`caffeinate`) is acquired only while status is `busy`
(`if(lo==="busy"){return ien(),XHo}`, offset 318,626,986; `ien` and `aen` at 316,931,432 call the inhibitor
singleton). It does not hold the machine awake for `shell`, `waiting`, or pending timers. **Verified** (code).

## Timers

| Timer | Where it lives | Visible from outside | Next fire time readable | Mark |
|---|---|---|---|---|
| `CronCreate` session-only (the default, and what `/loop 5m ...` creates) | Process memory (`sessionCron` store, offset 281,841,055) | Only in the transcript | Yes: tool result field `nextFireAt` (epoch ms, jitter included, a float such as `1793655671863.1191`: seen live) at creation. Later fires: compute from `cron`, last `scheduled_task_fire` time and the job id | verified (code, docs) |
| `CronCreate` durable (`durable: true`, only when the user asks to keep it) | `<project>/.claude/scheduled_tasks.json` | Yes | Computable: fields `id`, `cron`, `prompt`, `createdAt`, `lastFiredAt`, `recurring`, `permanent`, `createdBySessionId`, `createdByPid`, `createdByProcStart`, `createdInProject`, `plugin` (reader `Ubn` at 291,058,027) | verified (code); no such file on this laptop. Off in 2.1.316 here: the live CronCreate tool description says "durable persistence is not available. All jobs are session-only" |
| `ScheduleWakeup` (dynamic `/loop` with no interval) | Process memory, stored as a one-shot job with `kind: "loop"` and `scheduledFor` (offset 300,025,404) | Only in the transcript | Yes: tool result field `scheduledFor` (epoch ms). Delay clamped to 60 to 3600 s, then rounded up to a whole minute (seen live). The result carries no job id | verified (code and live) |
| Loop fallback wakeup | Memory. If an iteration neither reschedules nor stops, Claude Code arms one wakeup about 20 minutes later, then ends the loop if that one does not reschedule | No tool call for it in the transcript; only its `scheduled_task_fire` when it fires | Not readable, but predictable: last loop fire plus about 20 min | verified (code `$Fs`, docs, live: fired 20 min 24 s later) |
| Desktop app scheduled tasks | Desktop app. Prompt in `~/.claude/scheduled-tasks/<name>/SKILL.md`; schedule, folder and on/off state are not in that file | Prompt file only | No (schedule kept by the app) | verified (docs); app not installed here |
| Cloud scheduled tasks (routines) | Anthropic cloud | Not local | Not needed: they do not need this PC | verified (docs) |

Scheduler facts (module at 328,582,721, `createCronScheduler`):

- Checks every 1 s. Fires only when the session is idle (`if(i()&&!S)return`), so a due job waits for the current
  turn. **Verified** (code, docs).
- When the scheduler starts it takes a lock file `<project>/.claude/scheduled_tasks.lock` with `sessionId`, `pid`,
  `procStart`, `acquiredAt`. It starts on the first `CronCreate` or `ScheduleWakeup` in a session, even for
  session-only jobs. Released on clean stop. Only the lock holder runs durable jobs; session-only jobs run in any case.
  **Verified** in code and live: created at the first `CronCreate` or `ScheduleWakeup`, kept after the last job is
  deleted, removed on clean exit. It means "this session has used the scheduler", not "a job is pending".
- Jitter (constants `rJ` at 291,060,870): recurring jobs run late by `min(frac(id) * 0.5 * period, 30 min)`, where
  `frac(id) = parseInt(id[0..8], 16) / 2^32`. Exception: a job whose text is exactly `*/N * * * *` with a 5 minute
  gap fires 285 s after its last fire, or after its creation before the first fire, wherever that falls on the clock;
  it is "15 s early" only when that start is on a 5 minute mark (Claude Code 2.1.317 code, `docs/research.md` "Cron
  checks"; live fixtures 09 and 10, card `p4-cron`). One-shot jobs on minute :00 or :30 run up to 90 s early.
  **Verified** (code and docs; the 285 s rule also live).
- Recurring jobs expire 7 days after creation (fire once more, then delete). Up to 50 jobs per session. **Verified**
  (code constant 604,800,000 ms; docs).
- After sleep, an overdue job fires once on wake. No catch-up for each missed run. **Verified** (code: compares `now`
  with the stored next time; docs: "No catch-up for missed fires").
- Esc cancels pending dynamic loop wakeups (`aMr`, "cancelled N pending loop wakeup(s) on user abort"). Esc and Ctrl+C
  do not cancel fixed-interval jobs. **Verified** (code, docs). Live: Esc at the idle prompt cancelled a pending
  wakeup ("Pending wakeup cancelled" on screen) and wrote no transcript line.

Transcript lines that matter (all in `%USERPROFILE%\.claude\projects\<folder>\<sessionId>.jsonl`; the folder is the
original cwd with every non-letter, non-digit replaced by `-`, for example `C--Users-haika-code-wakeshot`; casing
follows the cwd, so VS Code gives `c--Users-...`):

| Line | Shape | Mark |
|---|---|---|
| Job created | `type: "assistant"`, content item `{"type":"tool_use","id":..,"name":"CronCreate","input":{"cron","prompt","recurring","durable"}}`, then a `type: "user"` line with `toolUseResult: {"id","humanSchedule","recurring","durable","nextFireAt"}` | verified (output schema at 314,993,464; resume scanner at 316,941,158; tool_use and `toolUseResult` shapes seen in local transcripts) |
| Job deleted | `tool_use` with `name: "CronDelete"`, `input.id` | verified (resume scanner) |
| Wakeup armed or loop stopped | `tool_use` with `name: "ScheduleWakeup"`, `input: {delaySeconds, reason, prompt, noop}` or `{stop: true}`; `toolUseResult: {scheduledFor, clampedDelaySeconds, wasClamped, stopped?, cancelledWakeups?}` | verified (schema at 300,122,434). A resume re-arms it from `scheduledFor` only for `bg` sessions; terminal and SDK resumes do not (live, "Resume findings"; code, `docs/research.md` "Claude transcript checks") |
| Job fired | `{"type":"system","subtype":"scheduled_task_fire","timestamp":"<ISO>","taskId","cron","prompt"(cut to 200 chars),"taskKind":"loop"?,"cronKind"?,"noOpStreak"?}` (`KQn` at 296,529,132) | verified (code) |
| Turn ended | `{"type":"system","subtype":"turn_duration",...}` | verified (this session's transcript) |
| Background task running | `type: "attachment"`, `attachment.type: "task_status"`, `taskType: "local_bash"` or `"local_agent"`, `status: "running"` | verified (resume scanner) |

The lines above were all seen live in test sessions on 2026-11-03 (see "Live test results"), plus two more:
`attachment` type `session_cron_carry` after `/clear`, and `queue-operation` lines (`enqueue`, `dequeue`) around each
fired prompt and task notification.

Claude Code itself uses two outside methods, which shows there is no better hidden signal:

1. On `--resume` it rebuilds jobs from the transcript lines above (scanner at 316,941,158): its `CronCreate` jobs;
   wakeups only for `bg` sessions, not for terminal or SDK ones (`docs/research.md`, "Claude transcript checks").
2. Agent view estimates a loop's next run from the transcript: it counts `"subtype":"scheduled_task_fire"` lines and
   predicts `last fire + median gap of the last 7` (`scanLoopTranscript`, offset 302,832,742). **Verified** (code).

## Background work

| Work | Status file | Other trace on disk | Mark |
|---|---|---|---|
| Background subagent (`run_in_background` Agent, Task) | `busy` until it ends | `%LOCALAPPDATA%\Temp\claude\<folder>\<sessionId>\tasks\a<hex>.output`; subagent transcript `projects\<folder>\<sessionId>\subagents\agent-<id>.jsonl` grows | verified (live, this session) |
| Workflow | `busy` | | verified (code) |
| In-process teammate | `busy` unless the teammate is idle | | verified (code) |
| Teammate in its own process (tmux mode) | No file for the teammate; the lead may read `idle` | | verified (code) for "no file"; lead status unverified |
| Remote (cloud) agent | `busy`, except long-running remote agents | | verified (code) |
| Background shell (`run_in_background` Bash or PowerShell) | `shell` when the main thread is idle | `tasks\b<9 chars>.output` grows. Files stay after the task ends | verified (code; file names on disk) |
| Monitor tool, shell kind | `shell` | `tasks\b....output` | verified (code) |
| Monitor tool, MCP or WebSocket kind | `idle` | | verified (code) |
| Background session (`claude --bg`, agent view) | Own file with `kind: "bg"`, `jobId` | `~/.claude/jobs/<id>/state.json` (docs: not a stable interface), `~/.claude/daemon/roster.json`. `claude agents --json` gives `state` `working` / `blocked` / `done` / `failed` / `stopped`. Docs: "A running subagent, workflow, or monitor counts as working, and so does a pending session-scoped scheduled task, such as a /loop wakeup" | verified (docs, code); not used on this laptop |
| Session sent to background from inside | `parkedJobId` set; `claude agents` hides that row and shows the job instead | jobs folder | verified (code) |

## Waiting and Remote Control

- `waiting` means the session cannot go on without a person. It does no work while waiting. If Remote Control is on,
  the person may answer from the phone, and the session continues only if the PC is awake and online. **Verified**
  (code for values; behavior after answer is common sense, not tested).
- `bridgeSessionId` is present while Remote Control is connected and is written as `null` on disconnect. Both
  sessions on this laptop have one (CLI pid 20636 and VS Code pid 3032), so Remote Control seems to start with every
  session here. **Verified** (disk, code). Live: every test session got one about 1 s after start, and "Disconnect
  this session" wrote `bridgeSessionId: null`.
- An idle session with Remote Control can receive a new prompt from the phone at any time. Nothing on disk predicts
  that. Whether to keep the PC awake for it is a product choice.
- Related open issues: #100743 (Desktop on Windows holds "prevent sleep" for Remote Control only in 5 min windows),
  #102024, #100187 (Remote Control transcript regressions).

## Desktop app

- Not installed on this laptop: no `AnthropicClaude` or `Claude` folders in `%LOCALAPPDATA%` / `%APPDATA%`, no Appx
  package, no winget entry, no running process. **Verified**.
- Docs: "In a local session, the Code tab runs its own copy of Claude Code, which has its own version number. The
  desktop app downloads and updates that copy." Issue #90007 shows it at `...\Claude\claude-code\<version>\claude.exe`
  and calls it the "CCD session host". **Verified** (docs, issue).
- Same engine, and the binary's readers map `entrypoint` `claude-desktop`, `claude-desktop-3p`, `local-agent` to
  "desktop". Registration runs for SDK sessions too (the VS Code SDK session writes one). So Desktop Code sessions very
  likely write the same `sessions\<pid>.json`. **Unverified**: needs a live test with the app.
- The Desktop keeps its own session list; docs say Desktop's cross-session tools do not see CLI or VS Code sessions,
  so Desktop may use extra state of its own. **Verified** (docs) for that sentence; meaning for status files
  unverified.
- Desktop scheduled tasks: the app checks every minute while it runs and starts a fresh session per run. "Tasks only
  run while the desktop app is running and your computer is awake. If your computer sleeps through a scheduled time,
  the run is skipped." On wake it starts one catch-up run for the latest missed time. The schedule is not in
  `SKILL.md`, so the next run time is not readable from outside. **Verified** (docs).
- Desktop keep-awake on Windows takes only an execution power request, so Modern Standby still sleeps (#103669, open).
  #105873: the per-session switch resets on relaunch. **Verified** (issues).

## Recommended detection rules

Run every 5 to 10 s, or on a folder change event for `sessions\` plus a slow timer.

| # | Rule | Signal | Confidence |
|---|---|---|---|
| 1 | List `%USERPROFILE%\.claude\sessions\*.json` only. Skip `*.key` and `.fleetview-heartbeat` without opening them. Parse the first JSON value; on failure retry once after 200 ms, then treat the file as "no signal" for this round | Live session list | High |
| 2 | Count a file only if the pid is alive and the process creation FILETIME equals `procStart` (or `procStartFt`). Confirmed from an unelevated (Medium) reader against elevated sessions too. Open with exactly `PROCESS_QUERY_LIMITED_INFORMATION \| SYNCHRONIZE`; one extra right fails the open. `0x80070057` means gone. `0x80070005` means no access, never dead: fall back to the system process list (create time equal to `procStart` means alive). A leftover also vanishes at the next Claude Code start | Drops crash leftovers and reused pids | High (exact match seen; unelevated check in `research.md:82-130`) |
| 3 | `status` is `busy` or `shell`, or any value Wakeshot does not know: working while the session process lives, no cap. An unknown value also shows "update Wakeshot" (`product.md`, UX). Sessions whose `entrypoint` is not `cli` (`claude-vscode`, `sdk-*`, `claude-desktop`, `claude-desktop-3p`, `local-agent`): a tool result with `backgroundTaskId`, or a Monitor `taskId`, opens a task. A `<task-notification>` at the start of a user line's text, or of the `prompt` of an `attachment` of type `queued_command`, closes each task id it lists when it has a `<status>` with any value (seen: `completed`, `failed`, `stopped`; `killed` in the code); one with no `<status>` (a Monitor event) closes nothing (Phase 4 rule change, "Phase 4 live shapes"). An open task is working while the session process lives. No time cap (Haik, 2026-11-03, `product.md:44-50`) | Working now | High |
| 4 | `status` is `waiting`. `waitingFor: "dialog open"` is a slash command dialog: treat it like `idle`. Otherwise `waiting` holds only while `bridgeSessionId` is set, for at most 1 hour (`product.md:33`) | Blocked on the user. Not working | High for meaning |
| 5 | `status` is `idle` or missing: find the transcript as `%USERPROFILE%\.claude\projects\*\<sessionId>.jsonl` (glob, do not rebuild the folder name). Read only new bytes since the last scan; keep parser state between reads, and after a Wakeshot restart read each transcript from its start. Remember the `tool_use` ids of `CronCreate`, `CronDelete`, `ScheduleWakeup` and the background task tools. Take each result by `tool_use_id`; skip `is_error` results. Also match `scheduled_task_fire`, `session_cron_carry`, and `<task-notification>` at the start of a user line. A reminder comes as `CronCreate` with `recurring: false` or as `ScheduleWakeup`: handle both. Keep per job: id, cron, recurring, createdAt, last fire, `nextFireAt` / `scheduledFor` (both floats). Key all job state by pid, not by `sessionId`. When `sessionId` changes, read the new transcript from its start and take `session_cron_carry` (`otherArmed`, `tasks`) as the armed jobs. Drop deleted, stopped, fired one-shots, and recurring jobs older than 7 days. Drop a wakeup at its `scheduled_task_fire` (`taskKind: "loop"`), or at `scheduledFor` plus a margin of a few minutes, which also bounds the invisible Esc cancel. Never hold past that. Loop fallback: no hold (`product.md:30-32`) | Pending timer and its next fire time | Medium. Misses: Esc cancel of a dynamic loop (maybe), loop fallback wakeup, tasks restored after resume under a new session id |
| 6 | If rule 5 finds a recurring job but no exact next time: a cron period of 1 hour or less holds. A longer period uses the next cron time after the last fire. A one-shot uses its exact `nextFireAt` or `scheduledFor` | Next fire estimate | Medium |
| 7 | Durable jobs (`<cwd>\.claude\scheduled_tasks.json`, the git root's, `~/.claude/scheduled_tasks.json`): dropped for v1; add when a `scheduled_tasks.json` is seen. Fact: durable jobs are off in 2.1.316 (live test 6) | Durable jobs, exact | High, but rare |
| 8 | `<cwd>\.claude\scheduled_tasks.lock` with a live `pid` that matches a session | That session has used `/loop` or cron at some point. Use only to decide to scan its transcript | Low as a work signal |
| 9 | Treat a pending timer as "work soon" only if it fires within 1 hour (decided, `product.md:30-31`). Otherwise let the PC sleep; the job fires once on wake | Policy | Decided |
| 10 | `claude agents --json`: dropped for v1. It starts a 335 MB process, has open bug #101182, and `bg` sessions write status files too (line 182; that they write transcripts the same way is not verified) | Background sessions incl. pending loops | Not used in v1 |
| 11 | Fallback when the status file is missing or unknown: transcript or `tasks\*.output` size grew in the last 60 s. Dropped (Phase 1 design, `architecture.md` section 7): it contradicts the unknown-format rule, where a file that does not parse gives no signal and an unknown `status` counts as working while the process lives | Activity | Not used in v1 (Nightcrew uses this) |

`updatedAt` is not a heartbeat or a last-write time (change 9 below).

Privacy: transcripts and `CronCreate` inputs hold user text. Parse only the listed fields, never store or log prompts.
Never read `*.key`, `.credentials.json`, or connect to `messagingSocketPath`.

## Live test results (2026-11-03)

Claude Code 2.1.316 on the Windows laptop. Test sessions ran in throwaway folders in the worker's scratch folder.
`spikes/claude-watch/ccdrive.py` drove them in a pseudo console (or over stream-json for the SDK path), and the
watcher `spikes/claude-watch` logged every status file change and the matching transcript lines. Model: Haiku, to save
tokens; the status code does not depend on the model. Nightcrew was not running.

| # | Test | Observed | Verdict |
|---|---|---|---|
| 1 | Background shell | `busy` in the turn, `shell` from turn end. When the shell ends, a user line that starts with `<task-notification>` (status `completed`) starts a short turn: `shell -> busy -> idle` (4 s) | As expected, but the end goes through `busy` |
| 2 | Shell Monitor | `shell` while idle. Each event: `shell -> busy` (2 s turn) `-> shell`. Last event: `busy -> idle`. The task output file appears with the first output line | As expected |
| 3 | Fixed loop `/loop 1m` | `CronCreate` input `{cron: "*/1 * * * *", prompt, recurring: true}`, result `{id, humanSchedule, recurring, durable: false, nextFireAt}`. `nextFireAt` 01:41:11.863 is the next minute plus `frac(id) * 30 s`, exact. `idle` between fires. Each fire: `system` line `scheduled_task_fire` `{taskId, cron, prompt, content}` 0.1 to 1 s after the due time, then a 4 s `busy`. The lock file holds the pid. `CronDelete` input `{id}`, result `{id}` | As expected |
| 4 | Dynamic loop and Esc | `ScheduleWakeup` input `{delaySeconds: 120, reason, prompt, noop: false}`, result `{scheduledFor, clampedDelaySeconds, wasClamped}`, no id. `scheduledFor` is rounded up to a whole minute (01:46:27.7 plus 120 s gave 01:49:00.000). Esc at the idle prompt: "Pending wakeup cancelled" on screen, no transcript line, no fire | An Esc cancel is invisible on disk |
| 5 | One-shot reminder ("in 3 minutes") | Haiku used `ScheduleWakeup` (180 s), not `CronCreate`. The fire: `scheduled_task_fire` with `taskKind: "loop"`, `cronKind: "loop"`, `cron: "39 1 * * *"` and a `taskId` not seen before. The fired prompt is a user line with `isMeta: true` | The model picks the tool; both shapes must work |
| 6 | Durable job | Could not run as planned: the live CronCreate tool description says "durable persistence is not available. All jobs are session-only". The model made a session-only daily job instead (`nextFireAt` 30 min after 9:07: the jitter cap). No `scheduled_tasks.json` | Durable jobs are off in this build |
| 7 | Waiting | Permission prompt: `waiting` / `permission prompt`. AskUserQuestion: `waiting` / `input needed`. A slash command dialog (`/remote-control`): `waiting` / `dialog open`. Esc gives `idle` | As expected |
| 8 | Remote Control off | "Disconnect this session" wrote `bridgeSessionId: null` (key kept) and `idle` in one write | As expected |
| 9 | `/clear` during a loop | `sessionId` rewritten in a write of its own. The job kept firing in the same process; the next `scheduled_task_fire` landed in the new transcript. The new transcript starts with an `attachment` of type `session_cron_carry`: `{tasks: [], otherArmed: [{id, cron, prompt, createdAt, recurring}]}` | As expected, plus the carry line |
| 10 | SDK path | Run the way the VS Code extension runs it: `claude -p --input-format stream-json --output-format stream-json --permission-prompt-tool stdio`. `entrypoint: "sdk-cli"`, `kind: "interactive"`. A `can_use_tool` request: `waiting` / `permission prompt`. Background shell: `idle` all the 90 s it ran; its end gave a 1.5 s `busy`. Background subagent: `busy` stayed on from the turn result (01:51:46) until the subagent ended (01:52:16) and its notification turn finished (01:52:20) | Done on the engine path; the VS Code UI itself was not used |
| 11 | Crash leftover | `Stop-Process -Force`: `.json` and `.key` stay. The pid plus start time check said dead within 1 s. The next Claude Code start removed both files | As expected, plus the sweep |
| 12 | `claude agents --json` | 0.21 s, exit 0, no new `git` processes. Per session: `pid`, `cwd`, `kind`, `name`, `sessionId`, `startedAt`, `status`. A `shell` session is reported as `busy`. No `state` field for interactive sessions | As expected |
| 13 | Desktop app | Could not run: the app is not installed | |
| 14 | Loop fallback wakeup (seen in test 5's session) | The wakeup fired at 01:39:00 and its turn did not reschedule. With no transcript line in between, a new `scheduled_task_fire` (`taskKind: "loop"`, new `taskId`, `cron: "0 2 * * *"`) came at 01:59:24.6: about 20 minutes later, 36 s before its cron time (one-shots on :00 run up to 90 s early) | The fallback exists and is invisible until it fires |

Other facts seen live:

- Start: during the folder trust dialog only `<pid>.<hash>.key` exists. `<pid>.json` appears after the dialog, first
  with no `status` and no `updatedAt`. `status: idle` follows 0.2 to 0.7 s later, and `bridgeSessionId` about 1 s
  later in a write that keeps the old `updatedAt`.
- About 25 minutes of polling every 200 ms: no half-written or unparsable read, and no write without a field change
  (no heartbeat). Changes were seen 24 to 235 ms after their `updatedAt`.
- `queue-operation` lines (`enqueue`, `dequeue`) come with each fired prompt and each task notification.
- The `/loop` command text quotes `<task-notification>` in its instructions, so match a task notification only at the
  start of the user text.
- Raw shape of a task notification (one line from a CLI session of 2026-11-02, read 2026-11-03 for the fixtures):
  `message.content` is a string that starts with `<task-notification>`, then `<task-id>`, `<tool-use-id>`,
  `<output-file>`, `<status>` and `<summary>` tags, one per line. The line also carries `origin.kind:
  "task-notification"`. The background shell result it closes has `toolUseResult.backgroundTaskId` and the same
  `tool_use_id`. Seen in one pair only; whether `origin.kind` is always set: not verified. Redacted copy:
  `spikes/claude-watch/fixtures/raw-transcript-lines.jsonl`.
- Background shells started by a subagent write their output under `%TEMP%\claude\<folder>\<another id>\tasks`, not
  under the main session id. Seen for this worker's own shells; what that id is: unverified.
- Driving tests: under ConPTY, Claude Code takes Enter only as the kitty keyboard sequence `ESC [13u`; a plain CR does
  nothing. The driver must also drop `CLAUDE_CODE_CHILD_SESSION` and the other `CLAUDE*` variables, or the test
  session writes no status file. Closing the pseudo console ends the session (clean exit).

### Proposed rule changes

Changes to "Recommended detection rules" above. Adopted at the Phase 0 close; Phase 4 refines. Changes 6 and 7 were
adopted in a stricter form: both readers are dropped for v1 (rules 7 and 10 above).

1. Rule 3: `shell` exists only for terminal sessions. For SDK-path sessions (`entrypoint` `sdk-*` or `claude-vscode`)
   add a transcript check: a tool result with `backgroundTaskId` (shell) or a Monitor `taskId` opens a task; a user
   line that starts with `<task-notification>` for that task id with status `completed`, `failed` or `killed` closes
   it. An open task counts as working. Phase 4 widened the close to any `<status>` value and to `queued_command`
   prompts (rule 3 above; "Phase 4 live shapes", "Rule changes").
2. Rule 4: `waitingFor: "dialog open"` is a slash command dialog, not a block. Treat it like `idle`.
3. Rule 5: read `nextFireAt` and `scheduledFor` as floats. A `ScheduleWakeup` result has no id: drop the pending
   wakeup when a `scheduled_task_fire` with `taskKind: "loop"` arrives at its time, or when `scheduledFor` has passed
   by more than a margin. The margin also covers the invisible Esc cancel. Never hold for a wakeup past
   `scheduledFor` plus the margin.
4. Rule 5 after `/clear`: when `sessionId` changes for the same pid, keep that pid's job list, read the new transcript
   from its start, and take `session_cron_carry` (`otherArmed` and `tasks`) as the armed jobs at that point.
5. Rule 5: a reminder may come as `CronCreate` with `recurring: false` or as `ScheduleWakeup`. Handle both.
6. Rule 7: durable jobs are off in 2.1.316 here. Keep the reader (cheap), low priority.
7. Rule 10: `claude agents --json` was fast here (0.21 s) and left no `git` processes, but adds nothing for
   interactive sessions. Keep it for `bg` and daemon kinds only.
8. Rule 2: confirmed. A leftover also vanishes at the next Claude Code start.
9. Do not use `updatedAt` as "last write" or as a heartbeat: it can stay old across a write, and files are not
   rewritten without a field change.
10. Rule 5, loop fallback: after a `scheduled_task_fire` with `taskKind: "loop"` whose turn ends with no new
    `ScheduleWakeup`, one hidden fire follows about 20 minutes later. Wakeshot can predict it (fire time plus 20 min,
    on a minute boundary, up to 90 s early) but should not hold the PC awake for it by default: it is one extra run
    that ends the loop. Haik's call if that should change.

## Live tests still open

- VS Code UI (optional). Test 10 ran the same engine path, but not through the extension, which sets
  `entrypoint: claude-vscode`. Steps:
  1. Open VS Code and start a Claude Code chat in any folder.
  2. Send: "Use the PowerShell tool with run_in_background set to true to run Start-Sleep 60, then end your turn."
  3. Within 60 s, in PowerShell run:
     `Get-ChildItem "$env:USERPROFILE\.claude\sessions\*.json" | ForEach-Object { Get-Content $_ -Raw | ConvertFrom-Json } | Where-Object entrypoint -eq 'claude-vscode' | Select-Object pid, status`
  4. Expected: `idle` while the sleep runs. Report the value you see.
- Desktop app, only if it gets installed later: open a Code tab session and look for a new `sessions\<pid>.json` with
  `entrypoint: claude-desktop`.

## Sources

- Binary: `C:\Users\haika\.local\bin\claude.exe` 2.1.316, offsets as cited.
- Disk: `C:\Users\haika\.claude\sessions\20636.json`, `3032.json`; `C:\Users\haika\.claude\projects\` (structure and
  key names only); `C:\Users\haika\AppData\Local\Temp\claude\` (file names only).
- Docs: https://code.claude.com/docs/en/scheduled-tasks, https://code.claude.com/docs/en/agent-view,
  https://code.claude.com/docs/en/desktop-scheduled-tasks, https://code.claude.com/docs/en/desktop (read 2026-11-03).
- Issues (anthropics/claude-code): #96438, #66384, #87883, #101182, #90007, #103669, #105873, #100743, #96679.
- Live tests 2026-11-03, 01:27 to 02:00: the test session transcripts were deleted. The watcher log was kept: full
  copy (not redacted, outside the repo) in `~/.claude/wakeshot-keep/2026-11-03-live-tests/live/watch.log`; redacted
  extract in `spikes/claude-watch/fixtures/`. Method and tools in `spikes/claude-watch/`; step log in
  `docs/tasks/p0-claude-live-tests.md`.

## Phase 4 live shapes (2026-11-03)

Card `docs/tasks/p4-claude-live-shapes.md`, 18:18 to 18:40. Claude Code 2.1.317 (Phase 0 had 2.1.316). Thirteen cases
(17 Claude Code processes with the resumed ones), model Haiku, one scratch folder each. Terminal sessions ran in a
pseudo console (`spikes/claude-watch/ccdrive.py`, `--permission-mode auto`); SDK sessions ran over stream-json as in
test 10. A small poller logged every status file write of these pids only, the lock file of each test folder and the
transcript sizes. Redacted raw lines and status writes: `crates/wakeshot-core/tests/fixtures/claude/live/` (its
`README.md` names the line of each shape). The test transcripts and folders were deleted afterwards.

Marks as above: **verified** means seen live in these sessions; **not verified** means inferred or not run.

### What was run

| Fixture | Case | Observed | Mark |
|---|---|---|---|
| `01` | Background shell | As Phase 0 test 1: `busy`, `shell`, notification `completed`, `busy`, `idle` | verified |
| `02` | Shell Monitor | The model's first two Monitors failed (Monitor runs its command through Bash, which expanded `$_`): each gave a notification with `<status>failed</status>`. Event notifications carry `<task-id>`, `<summary>` and `<event>`, but no `<status>` and no `<tool-use-id>`. The last one has `<status>completed</status>` | verified |
| `03` | `/loop 1m` | `nextFireAt` = next minute plus `frac(id) * 30 s` (exact, as Phase 0). Fires 0.05 s after it, then every 60 s. `CronDelete` `{id}` | verified |
| `04` | Dynamic `/loop` | First `ScheduleWakeup` had `noop: true`. Fire: `taskKind: "loop"`, `cron` like `21 18 * * *`, a new `taskId`. Stop: input `{stop: true}`, result `{scheduledFor: 0, clampedDelaySeconds: 0, wasClamped: false, stopped: true, cancelledWakeups: 0}` | verified |
| `05` | One-shot reminder | "Remind me in 2 minutes" became a background Bash sleep: a reminder can also be a background shell. Asked for `CronCreate`: `recurring: false`, cron `23 18 3 11 *`, `humanSchedule` equals the cron string, `nextFireAt` on the whole minute; fired 0.85 s after it | verified |
| `06` | `/clear` during a loop | As Phase 0 test 9 (`session_cron_carry`, the next fire in the new transcript). New: `CronList` result `{jobs: [{id, cron, humanSchedule, prompt, recurring, durable, nextFireAt}]}`; the lock file kept the old `sessionId` and was not removed at the clean exit, so a lock with a dead pid stays | verified (one case) |
| `07` | Background subagent | Result `{isAsync: true, status: "async_launched", agentId, ...}`. The first notification came as a user line. The second came while the main thread was busy: a `queue-operation` `remove`, then an `attachment` of type `queued_command` whose `prompt` starts with `<task-notification>`, and no user line for it | verified (one case each) |
| `08` | SDK path, background shell | As Phase 0 test 10. No `turn_duration` lines at all on this path (three SDK sessions) | verified |
| `12` | SDK path, stdin closed while a 150 s shell ran | The process did not exit: it lived until the shell ended, ran the notification turn (`busy` 1.7 s), then exited and removed its status file | verified (one case) |
| `13` | SDK path, hard kill while a shell ran, then `--resume` | The shell's `pwsh` outlived Claude Code (killed by pid afterwards). See the resume findings | verified |

Other facts seen:

- `/exit` with armed jobs opens a dialog "Background work is running" (Exit and stop tasks, Move to background and
  exit, Stay); the status is `waiting` / `dialog open` while it shows. **Verified**.
- `*/5` job: `nextFireAt` = creation plus 285 s (created 18:23:26.767, next 18:28:11.763), not the next 5 minute
  mark. That is the 285 s rule in "Timers". **Verified** (two jobs).
- The folder trust dialog now has "No, exit" selected; a driver must press Down first. The PowerShell tool refuses a
  standalone `Start-Sleep 40` ("use Monitor with an until-loop"). **Verified**.
- Pid 8784 was used by one test session and, two minutes after it ended, by another (different `procStart`).
  **Verified**. Rule 2 already covers it.
- A hard-killed session's status file was removed at the same millisecond as another test session's clean exit
  (once) and at the resumed process's start (once). Whether Claude Code sweeps at exit too: **not verified**.

### Resume findings

Three terminal resumes (fixtures `09` to `11`; `claude --resume <sessionId>` in a new process, same folder): after `/exit` with a `*/5` job
and a 240 s `ScheduleWakeup`; the same after `Stop-Process -Force`; after `/exit` with a one-shot `CronCreate` and a
daily job. One SDK resume after a hard kill with a background shell running (`13`).

| Question | Answer | Mark |
|---|---|---|
| New status file | New pid, new `startedAt` and `procStart`, the same `sessionId`. Its first write has no `status` and no `updatedAt`, as at a fresh start | verified (4 resumes) |
| Which transcript grows | The same `<sessionId>.jsonl`; no new file | verified (4) |
| A line that marks the resume or lists restored jobs | None. At start the resumed process writes only metadata lines (`atis-latch`, `bridge-session`, `tools-used`); a clean exit writes `cost-state`, `last-prompt`, `ai-title`, `mode`, `permission-mode`, `session-owner` and the like. These lines also follow turns, so they mark nothing. The only exception: the SDK resume after a hard kill wrote, at start and before any prompt, a user line `<task-notification>` with `<status>stopped</status>` (and `queueTranscriptOnly: true`) for the background task the dead process left open | verified |
| Do the jobs fire | `CronCreate` jobs come back and fire, after a clean exit and after a hard kill. The `*/5` jobs fired first at resume start plus 285 s (18:29:41; the old `nextFireAt` 18:28:11 was dropped), then every 5 minutes; `CronList` showed the new time. The one-shot (18:34) and the daily job (09:30 with jitter) kept their old times; the one-shot fired. The `ScheduleWakeup` (18:28:00) never fired and `CronList` did not show it | verified |
| Lock file | The resumed process took `scheduled_tasks.lock` at start, before any fire, also over the dead pid's lock after the hard kill | verified |
| Would old lines wrongly count as armed in a replay from the start | `CronCreate` lines: no, the jobs are restored; but a recurring job's old `nextFireAt` can be wrong. `ScheduleWakeup` lines: yes, a replay holds the dead wakeup until `scheduledFor` plus the margin (here from 18:24:56 to 18:30:00). Background tasks (SDK path): the `stopped` line closes them if the rule accepts that status; rule 3 as written (`completed`, `failed`, `killed`) would keep the task open forever | verified |

Not run: `claude --continue`, `/resume` inside a running session, a resume after a job's time passed while no
process ran (docs: missed runs do not fire), a terminal resume with a background shell left open, and the VS Code
extension's own resume.

### Rule changes

Proposed exact text for "Recommended detection rules". **Verified** where the table above says so; the "never open
a task" guard is an inference (the SDK resume closed the task anyway).

- Rule 5, add at the end: "Resume: a status file whose `sessionId` names a transcript that already has lines older
  than the file's `startedAt` is a resumed session (`--resume` in a new process; no line marks it). Read that
  transcript from its start as usual, then apply this at `startedAt`: keep `CronCreate` jobs that were not deleted
  (recurring ones under 7 days old, one-shots not yet fired); for a recurring job ignore the old `nextFireAt` and use
  rule 6; drop every `ScheduleWakeup` wakeup and every open background task from lines older than `startedAt`
  (Claude Code does not restore wakeups, and a task does not outlive its process)."
- Rule 3, replace "with `completed`, `failed` or `killed`" by: "with any `<status>` value (seen: `completed`,
  `failed`, `stopped`)". Add: "Match `<task-notification>` at the start of a user line's text and at the start of
  the `prompt` of an `attachment` of type `queued_command`. A notification with no `<status>` (a Monitor event)
  closes nothing."
- Rule 5, reminders: "A reminder may also be a background shell that sleeps (handled by rule 3)."

Parts of this file that the resume test contradicts, left for the parser card or a docs pass: "Short answer" and
"Claude Code itself uses two outside methods" say Claude Code rebuilds pending jobs on `--resume`: true for
`CronCreate` jobs, not for `ScheduleWakeup` wakeups. The "Wakeup armed" row says "resume re-arms from
`scheduledFor`": not seen live for a resume in a new process. All three were corrected in the Phases 2 to 6 docs
pass (2026-11-04), with rule 3's close on any `<status>`; rule 5's resume and reminder texts above are not yet in
the rules table.
