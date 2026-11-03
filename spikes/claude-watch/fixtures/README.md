# Claude Code live-test fixtures

Real shapes for the Phase 2 and Phase 4 parser tests. Claude Code 2.1.316 on the Windows test laptop. Made
2026-11-03 (card `docs/tasks/p0-close-apply.md`, item 25 of `docs/reviews/p0-close-proposals.md`).

## Files and where they came from

| File | Source |
|---|---|
| `live-tests.log` | 401 of the 1078 lines of the live-test watcher log (01:27 to 02:00, 2026-11-03; tests in `docs/research/claude-code-signals.md`, "Live test results"). Copied from `C:\Users\<user>\.claude\wakeshot-keep\2026-11-03-live-tests\live\watch.log`, the orchestrator's copy of the old scratch folder. Kept: every status file event of the six test sessions (pids 33448, 21600, 33008, 8516, 30436, 29104), the lock file lines, and their transcript lines that matter to the rules. Dropped: the owner's own two sessions (pids 20636 and 32332) and bulky transcript lines (API requests, tool, skill and agent listings, instructions, environment, hooks, file history, titles, thinking) |
| `agents.json` | `claude agents --json` during the tests (test 12), from the same folder. Session 33008 shows `busy` while its status file says `shell` |
| `raw-transcript-lines.jsonl` | Two raw transcript lines from one of the owner's own sessions in this project (2026-11-02): a background shell result and the task notification that closes it (same task id and `tool_use_id`). The watcher log has these only as summaries |

Not kept: the test transcripts themselves (the `cctest*` folders in `~/.claude/projects` were deleted on 2026-11-03),
the raw `session_cron_carry` line (the watcher logged only its kind; the shape is quoted in `claude-code-signals.md`,
test 9), and the raw text of Monitor event notifications.

## Redaction

Done by a script that printed only redacted text. No `.key` file was read or copied.

- The Windows user name is `USER` in every path, also in folder names (`C--Users-USER-...`). The host in `pidDomain`
  is `HOST`.
- Session ids are fake UUIDs `5e55....-0000-4000-8000-...`, the same fake for the same id across `live-tests.log`
  and `agents.json`. The 8 hex characters after a pid (`[33448 5e550002]`) are the fake id's prefix. Fake
  `5e550001` is the orchestrator session whose scratch folder held the test folders.
- `bridgeSessionId` values are `session_BRIDGE01` and up. Pipe names in `messagingSocketPath` are numbered.
- Prompts and other text were already `<str N>` (length only) in the watcher log. In the raw lines every string is
  `<str N>` except UUIDs (faked), `toolu_` ids, task ids, ISO times and short enum values (`type`, `role`,
  `entrypoint`, `version`, `status`, `userType`, `permissionMode`, `kind`). In the task notification text only the
  tag names and the values of `task-id`, `tool-use-id` and `status` are kept.
- Kept as they are: pids, `procStart` FILETIMEs, epoch times, task ids, `toolu_` ids, cron strings, session names
  made from the test folder names.

## Format of `live-tests.log`

- `[pid] NEW {...}`: the full status file the first time the watcher saw it. `(no updatedAt)`: the file had none yet.
- `[pid] CHANGE (write <updatedAt> seen +N ms) field: old -> new`: the fields one write changed. `(none)` means the
  field was absent, `(gone)` that it was removed.
- `[pid] process: ...`: the pid plus start time check. `GONE`: the status file was deleted.
- `[pid sid] TRANSCRIPT`, `TASK FILE`, `SIDE FILE`: transcript opened, task output file size, lock file content.
- `[pid sid] T <line time> <kind> ...`: the watcher's summary of one transcript line, not raw JSONL (`src/summary.rs`).
  `*` marks lines about timers or background work. Strings are `<str N>` except under the keys in
  `KEEP_STRING_KEYS`. Epoch ms under keys that end in `At` or `For` are shown as `"<ms> (hh:mm:ss.mmm)"` in local
  time (UTC+4); in the transcript they are plain JSON numbers. One early result shows `nextFireAt` as a bare float,
  from a watcher build before that formatting handled fractions.
- `claude-watch started`: the watcher was restarted; it re-reads every status file (`NEW` again).

## Where each shape is

Line numbers in `live-tests.log`.

| Shape | Lines |
|---|---|
| Status file, CLI, before the first `status` | 2 |
| Status file, CLI, `idle` with `bridgeSessionId` | 30 |
| Status file, SDK path (`entrypoint: sdk-cli`) | 336, 351 |
| `idle`, `busy`, `shell` around a background shell (test 1) | 4 to 26 |
| Background shell result (`backgroundTaskId`), its notification | 14 and 24; 254 and 332; 362 and 369 (SDK path) |
| Monitor call and result, then event notifications (`status=?`) and the last one (`completed`) | 39, 40, 47 to 119 |
| `ScheduleWakeup` call and result (one-shot reminder, test 5) | 81, 82 |
| `ScheduleWakeup` (dynamic loop, test 4) | 322, 323 |
| `CronCreate` call and result: daily job (test 6), then `CronDelete` | 111, 112, 131, 132 |
| `CronCreate` `*/1` (fixed loop, test 3) | 145, 146; 178, 180 |
| `scheduled_task_fire` of a cron job (no `taskKind`) | 189, 222, 243, 264, 282 |
| `scheduled_task_fire` with `taskKind: "loop"`: the wakeup, then the hidden loop fallback (test 14) | 158, 393 |
| `waitingFor`: `permission prompt`, `input needed`, `dialog open` | 207, 217, 235 |
| Remote Control off (`bridgeSessionId` to `null`) | 236 |
| `/clear`: `sessionId` change, then `session_cron_carry` in the new transcript | 271, 275 |
| `CronDelete` after `/clear` | 299, 300 |
| Background subagent (`Agent`, `async_launched`) and its notification | 378, 379, 383 |
| Crash leftover: process dead, then `0x80070057`, then `GONE` | 327, 328, 338 |
| Lock file: changed, present after a watcher restart, removed | 85, 169, 400 |
