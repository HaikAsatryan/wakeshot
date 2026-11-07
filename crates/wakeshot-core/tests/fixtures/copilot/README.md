# Copilot CLI event logs, built from the schema

Built from the schema, not copied from real logs: no Copilot CLI had run on the test laptop yet. Each file is a
session's `events.jsonl` as Copilot CLI 1.0.106's event schema describes it (`copilot-sdk/generated/session-events.d.ts`
and `schemas/session-events.schema.json` in the npm package, read 2026-11-06), with the event sketches of
`docs/research/other-agent-signals.md`, "Fixture sketches". Every event has the fields the schema requires; field
order and extra fields in real files will differ. Made by a script (not kept). Real logs are in `live/`.

All times are 2026-11-06 UTC unless named. Ids and text are made up. The tests use a PC zone of UTC+4.

| File | What it holds | `tests/copilot_source.rs` expects |
|---|---|---|
| `working.jsonl` | a prompt, a turn whose message asks for `powershell`, its turn end, the tool's start | working |
| `waiting.jsonl` | the research sketch: the same turn, then `permission.requested` at 10:00:04.200 | waiting since 10:00:04.200, not remote |
| `idle.jsonl` | `waiting.jsonl`, the approval, the tool run, a last turn whose message asks for no tool | idle |
| `aborted.jsonl` | `working.jsonl`, then `abort` (`user_initiated`) | idle |
| `subagent-open.jsonl` | a `task` tool starts a background sub-agent; the sub-agent's own turn (`agentId`); the main turn ends with no tool | working (the sub-agent is still open) |
| `subagent-done.jsonl` | `subagent-open.jsonl`, then `subagent.completed` | idle |
| `ask-user.jsonl` | a turn that starts the `ask_user` tool at 10:00:04.200 | waiting since 10:00:04.200 |
| `resumed.jsonl` | `working.jsonl` with the tool still running, then `session.resume` at 11:00 (a crash, then `--resume`) | idle |
| `schedule-at.jsonl` | a one-shot at 18:00 (`at`) | one timer at 18:00 |
| `schedule-every.jsonl` | every 10 minutes from 10:00:00.500, one fire (`user.message`, `source` `schedule-1`) at 10:10:00.750 and its turn | idle, one timer at 10:20:00.750 |
| `schedule-every-later.jsonl` | every 10 minutes from 10:00:00.500, no fire seen, a typed prompt and its turn ending 10:25:03.100 | one timer at 10:30:00.500 |
| `schedule-after.jsonl` | a one-shot interval of 30 minutes from 10:00:00.500 (`recurring: false`) | one timer at 10:30:00.500 |
| `schedule-cron.jsonl` | cron `0 9 * * 1-5` with `tz` `America/New_York`, made Friday 10:00:00.500 | one timer Monday 2026-11-09 05:00 UTC (09:00 in the PC's zone, not New York's) |
| `schedule-self-paced.jsonl` | a self-paced schedule armed for 10:45, its fire, then armed for 11:30 | one timer at 11:30 |
| `schedule-cancelled.jsonl` | an interval schedule and a one-shot at 18:00; the interval one is cancelled | one timer at 18:00 |
| `shutdown.jsonl` | a one-shot at 18:00, then `session.shutdown` | idle, no timer; the timer back after a `session.resume` |
| `junk-lines.jsonl` | a line that is not JSON, one with no `type`, an ephemeral `session.idle`, an unknown kind, a `subagent.started` with no `toolCallId`, then a prompt and a turn start | working, the problem `event`/`line` |
| `rotation-before.jsonl` | `schedule-every.jsonl`, then a background `subagent.started` at 10:12 | working, one timer at 10:20:00.750 |
| `rotation-head.jsonl` | the new head after a rotation: `session.journal_link` at 10:15, checkpoint copies of the start, the schedule and its fire, then `session.compaction_start` at 10:15:00.100 | after `rotation-before.jsonl` and a `Gone`: unchanged; read alone: idle, one timer at 10:20:00.500 |
