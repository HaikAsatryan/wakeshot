# Claude parsing review (Phases 2 to 6)

Reviewer: `wakeshot-reviewer`, 2026-11-03 23:51 to 23:54. Brief: `docs/phases/02-06-lighter.md`, "Reviews": any file
content that makes Wakeshot hold forever or never, and any path that stores or logs prompt text or opens a `.key`
file.

Scope read: `p4-claude-status`, `p4-cron`, `p4-claude-transcript`, `p4-hold-files`, `p4-watch-reads`,
`p4-watch-agents` (code, tests, Results), `docs/research/claude-code-signals.md`, `docs/hold-files.md`, the core's use
of the facts (`policy/sessions.rs`, `core/holds.rs`) and `watch/data.rs` for the `names` callers. Claude Code 2.1.317
(`C:\Users\haika\.local\bin\claude.exe`, the build the live fixtures came from) was read, read only, for the ways a
background task ends; byte offsets below point into that file, and minified names are from that build only. Quotes
from it have their dashes replaced by colons. No cargo command was run.

Counts: 1 blocker, 1 major, 8 minor.

## Blocker

### B1. A task stopped with `TaskStop` stays open: VS Code and SDK sessions hold until the chat closes

- Where: `crates/wakeshot-core/src/sources/claude/transcript.rs:243-258` (only `CronCreate`, `CronDelete`,
  `ScheduleWakeup` and `Monitor` calls are kept; every other tool is `continue`), `:316-329` (results), `:1090-1119`
  (`ToolResult` reads `taskId` and `backgroundTaskId`, never `task_id`). The hold comes from `policy/sessions.rs:137`
  (`open_tasks > 0` holds while the process lives).
- What breaks: in a session whose `entrypoint` is not `cli`, the model starts a dev server or a watcher with
  `run_in_background`, then stops it with `TaskStop` (aliases `KillShell` and `KillBash`: `var za="TaskStop",
  vlr=["KillShell","KillBash"]`, offset 291,715,818). Claude Code then writes no `<task-notification>`:
  - `TaskStop.call` runs `ILe` (offset 302,203,845) with no `source`. The shell kill sets `notified: true`, appends a
    `[killed]` trailer to the task's output file and emits `_a(id, "stopped")`. `_a` (offset 287,115,090) builds the
    SDK stream event `{type: "system", subtype: "task_notification"}`, not a transcript line. The only path in `ILe`
    that queues a real notification (`tUr`, status `killed`, offset 302,206,557) runs only when `source` is set, which
    is a stop by the user or the client, never `TaskStop`. Once `notified` is set, the shell's own exit path (`z6r`)
    changes nothing, so the usual notification does not come either.
  - The only transcript trace is the `TaskStop` result `{message, task_id, task_type, command}`. Claude Code's own
    resume scanner (`hrn`, offset 319,308,573) uses exactly that as the stop marker: the result of a `TaskStop` or
    `KillShell` call with string `task_id` and `task_type` goes into `stoppedTaskIds`.
  - Wakeshot reads neither the tool name nor `task_id`, so `open_tasks` stays 1. The session holds while the VS Code
    window lives, often for days, with no notice, since nothing failed to parse.
- How likely: high for anyone who lets the model run a dev server or test watcher in VS Code; stopping it with
  `TaskStop` is a routine step. Verified in the 2.1.317 code. Not verified live; no fixture has a `TaskStop`.
- Fix: keep `TaskStop`, `KillShell` and `KillBash` `tool_use` ids as a new `Call::Stop`; on a result that is not
  `is_error`, close `toolUseResult.task_id` (fall back to the input's `task_id`, then `shell_id`). Never read
  `command`. Add a unit test and a case in `p4-live-claude` (SDK path: background shell, `TaskStop`, expect
  `open_tasks` 0).

## Major

### M1. Known item 1, confirmed: a Monitor that times out holds until the session ends; a stop from the UI likely until the next prompt

- Where: `transcript.rs:325-328` (a Monitor result opens a task with no end), `:188-195` (`open_tasks` is a count
  with no end time), `notice_ids` `:612-633` (a notification with no `<status>` closes nothing, which is right for
  Monitor events).
- What breaks, part a (Monitor timeout): the shell Monitor (`We`, offset 301,337,261) arms a timer at `timeout_ms`
  unless `persistent`. When it fires, Claude Code queues one event notification (`Aq`, with the "Monitor timed out"
  or "Monitor expired" text, `<task-id>` and `<summary>`, no `<status>`), then kills the task, which sets
  `notified: true`, so no status line follows. Claude Code's own resume text agrees: a shell "may have been stopped
  (via the UI, Monitor timeout, or agent teardown: these leave no transcript marker)" (offset 319,326,078).
  `timeout_ms` defaults to 300,000 ms (`TYe`) and is at most 3,600,000 ms (`eEn`; schema refine
  `persistent || timeout_ms <= eEn`). The result line carries both values, for example
  `{"persistent":false,"taskId":"b0pdi64jp","timeoutMs":180000}` (`spikes/claude-watch/fixtures/live-tests.log:40`).
  The WebSocket Monitor (`monitor_ws`, offset 301,330,935) returns the same `{taskId, timeoutMs, persistent}`.
- What breaks, part b (stop from the UI): a user or client stop calls `ILe` with a `source`, and then `tUr` queues a
  `<status>killed</status>` notification ("stopped by the user"), gated by the remote flag `tengu_rosy_donut`
  (`Ube`, default on, offset 293,636,218). It is queued as `passive`, which does not start a turn; whether it reaches
  the transcript before the user's next prompt is not verified. If it does not, the task holds until the next prompt
  in that chat or until the chat closes; with the flag off, until the chat closes.
- How likely: part a high in VS Code (the Monitor tool's own description steers the model to unbounded `tail -f`
  style commands, which end only at the timeout). Part b not verified, including whether the VS Code extension has a
  stop button for background tasks.
- Fix, part a, no contract change: report a Monitor result with `persistent: false` and `timeoutMs > 0` as a timer
  due at its open time plus `timeoutMs`, not as an open task, and drop that timer when a `<status>` notification for
  its id comes. `timeoutMs` is at most 1 hour, so the core's 1 hour lead (`TIMER_LEAD_MS`) covers the whole run and
  the 2 minute margin covers the kill. A persistent Monitor (`timeoutMs: 0`) stays an open task (B1's fix ends it on
  `TaskStop`). Add a fixture with a timed-out Monitor.
- Fix, part b: run the UI stop in `p4-live-claude` (VS Code, background shell, stop it from the panel, watch the
  transcript). If no line comes, the only on-disk end marker found is the trailer Claude Code appends to the task's
  output file (`pxt`, offset 289,923,391: `[killed]` on a kill, `[exited with code N]` on an exit). Reading the last
  bytes of `%TEMP%\claude\<folder>\<sessionId>\tasks\<id>.output` would end the task, but it is a new wanted-file
  kind and reads tool output, so it is a design call for the orchestrator. Without it, say it in `docs/product.md`
  (Claude Code sources) and the README.

## Minor

### m1. Known item 2, confirmed as decided; one case is silent for good

- Where: `crates/wakeshot-core/src/sources/claude/mod.rs:227-237`.
- What breaks: a transcript problem becomes `UnknownValue` only while the session already holds, as decided. For
  timers that is acceptable: a job's next fire makes the session `busy`, so "update Wakeshot" shows after at most one
  missed sleep. But a renamed `backgroundTaskId` is never a problem at all: no call is kept for Bash or PowerShell,
  so a result without the field looks like a plain foreground result. SDK background shells would then stop holding
  with no notice ever.
- How likely: needs a Claude Code format change; unverified.
- Fix: keep the `tool_use` ids whose input has `run_in_background: true`, and flag `TRANSCRIPT` with a fixed text
  when such a result has no `backgroundTaskId`. The notice-only `FormatHealth` variant in `p4-claude-transcript`'s
  Found outside scope 1 would make every transcript problem show at once.

### m2. Known item 3, confirmed: the remote jitter flag can only make Wakeshot miss a fire, never hold forever

- Where: `crates/wakeshot-core/src/sources/cron.rs:264-307` (`ClaudeScheduler::DEFAULT`), used for every computed due:
  `transcript.rs:441-460` (`fired`), `:555-583` (`Job::due`), `:590-606` (carry chain).
- What breaks: a due that comes from the transcript (`nextFireAt`, `scheduledFor`) already has Claude Code's real
  jitter. A due Wakeshot computes (after a fire, after a resume, from a carry line) uses the built-in values. If
  `tengu_kairos_cron_config` makes Claude Code fire more than 2 minutes later than computed (only a `recurring_frac`
  above 0.5 can: the cap's schema maximum, 30 minutes, is the built-in value), the hold ends before the fire and the
  PC may sleep; the job runs once on wake. An earlier fire is inside the 1 hour lead. If the flag raises
  `recurring_max_age_ms` (the schema allows 30 days), `fired` (`:452-460`) and `due` (`:559-562`) drop the job at 7
  days while Claude Code keeps firing it. No setting inside the schema can make a timer hold past its due plus 2
  minutes.
- How likely: unverified; no flag change seen.
- Fix: for a computed recurring due, report the latest fire the schema allows (`recurring_frac` 1.0, cap 30 minutes).
  The real fire then always falls inside the window (the lead is 1 hour), and its fire line moves the job on, so the
  normal case costs nothing. Log when a `CronCreate` result's `nextFireAt` differs from
  `ClaudeScheduler::DEFAULT.next_fire_ms` at its creation, so a flag change is seen. Take the 7 day end only from
  Claude Code's own fire lines.

### m3. Known item 4, refuted: no `names` caller reads an error as "nothing there"; one residual

- Where: `crates/wakeshot-windows/src/watch/read.rs:298-307` (`names`); callers `watch/agents.rs:555-565` (status
  folder), `agents.rs:649-655` (`Names` listings, Codex and Copilot, after v1), `watch/data.rs:136-151` (holds).
- Ruling: each caller keeps its last list on an error and logs it; a file that is really gone still drops through its
  own `NotFound` (`agents.rs:609-615`, `data.rs:160`). A missing folder is an empty list, which is right.
- Residual: on a listing error of `sessions\`, `fill_folder` returns at `agents.rs:564` before it reads any known file,
  so the tick reads no status file until the listing works again. Changes then come only from notices that carry
  names; an overflow notice (`names: None`) lists again and fails again. A `busy` seen before the error then holds
  until the process ends.
- How likely: low (a listing error on the user's own profile folder); unverified.
- Fix: on a listing error, read the names already in `state.known` instead of returning.

### m4. A notification stored with Claude Code's header, or joined with others, closes nothing (unverified)

- Where: `transcript.rs:612-633` (`notice_ids` needs the text to start with `<task-notification>` and reads only the
  first block).
- What breaks: 2.1.317 has a header `[SYSTEM NOTIFICATION - NOT USER INPUT]` plus three lines (`Dwe`, offset
  292,574,457) that it puts before task notifications on some paths (`Xee`), and its own resume scanner strips that
  header and splits several notices joined in one user line by `joinedNoticeLengths` (`qi`, offset 319,332,248). The
  live fixtures show only the plain form in main transcripts, so today this works. If a release stores the header
  form, every SDK task stays open, with no notice.
- How likely: unverified; Claude Code's own reader shows the shape exists on some path.
- Fix: accept a notification after a known header line, and read every `<task-notification>` block of a joined text.
  One fixture each.

### m5. A `sessionId` of a new shape drops the transcript with no notice

- Where: `crates/wakeshot-core/src/sources/claude/status.rs:250-262` (at most 64 ASCII letters, digits, `-`, `_`).
- What breaks: an id outside that shape gives `session_id: None`, so no transcript is wanted: no timers and no SDK
  tasks, and nothing is flagged. The charset check itself is right (it keeps a search inside `projects\`).
- How likely: low; Claude Code ids are UUIDs today.
- Fix: name it like a transcript problem (`UnknownValue("sessionId", <type or length>)` while the session holds).

### m6. `CLAUDE_CONFIG_DIR` gives no Claude signal at all (carried from `p4-claude-status`, unverified)

- Where: `mod.rs:68-76` (`<home>\.claude` only).
- What breaks: Claude Code's docs say `CLAUDE_CONFIG_DIR` moves its configuration folder, suggested for two
  accounts. A user who sets it gets no hold from Claude Code and no notice. Whether `sessions\` follows it is not
  verified.
- Fix: check it in `p4-live-claude`; if it moves, the tray takes the folder from the user's environment when set.

### m7. A hold file naming pid 4 holds until `expires` or a restart

- Where: `crates/wakeshot-core/src/holds/file.rs:72-75` (refuses pid 0 because it "never ends, so such a hold could
  never end"), `:171-181` (`hold_alive`).
- What breaks: pid 4 (System) never ends either. A file with `"pid": 4` and no `procStart` counts, since System
  started at boot, before any write. Only a user's own file can do this, and the user doc tells them to name their
  own process.
- How likely: low.
- Fix: refuse pid 4 with the same reason, or leave both and drop the "never ends" wording.

### m8. Docs that no longer match the code

- `docs/architecture.md:908`: "Rule 5 does not cover `claude --resume` in a new process. Phase 4 runs one live test
  and fixes the rule." Done: `p4-claude-live-shapes` ran it and `transcript.rs` applies the resume rule at
  `procStart`.
- `docs/research/claude-code-signals.md:227` and `:295`: rule 3 still closes a task only on `completed`, `failed` or
  `killed`; the code closes on any `<status>`, as the live shapes rule change (`:410`) says. "Short answer" and the
  `*/5` "15 s early" line are also stale (already in the phase file's work log).
- Once B1 and M1 are fixed, rule 3 and `docs/product.md` (Claude Code sources) should name the ends that write no
  notification: `TaskStop` (its result is the marker), a Monitor timeout, and possibly a stop from the UI.

## Checked, no finding

- `.key` files are never opened: the status folder filter takes `<digits>.json` only (`status.rs:24-37`), the open
  checks the name again and refuses `:` and trailing dots or spaces (`read.rs:69-82`), and transcript finds search
  `projects\` for `*<sessionId>.jsonl` only, with the id charset checked first. `names` and `find` open nothing.
- Prompt text is neither stored nor logged: the transcript shapes keep ids, cron texts (at most 128 characters,
  only from a successful `CronCreate` or a carry line), times and flags; the `prompt` of a `queued_command` is read
  only through `notice_ids`, which keeps task ids; a held partial line shows only its length in `Debug`. The watch log
  names only the field (`agents.rs:176-179`); the core logs a field and at most 32 characters of a status value or
  cron text (`core/holds.rs:90-100`).
- Timers cannot hold forever: each window ends 2 minutes after a fixed due time, and only a new transcript line moves
  a job on; a wakeup with no fire line ends by its time. An unknown `status` value holds while the process lives, and
  an unparsable status file gives no signal and the notice, both as decided; pid 0 and `procStart` 0 are refused.
- The resume hand-over: `Slot::feed` prunes wants after every feed (`agents.rs:426-439`), so a transcript whose want
  leaves and comes back in one call starts again from byte 0, as `p4-claude-transcript` asked.

Not checked: `/clear` in an SDK session with a background task open (whether Claude Code stops the task, and whether
it writes a line); a background shell started by a subagent that outlives it.
