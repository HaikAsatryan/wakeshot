# Codex CLI rollout fixtures

Rollout files for `tests/codex_source.rs` (card `docs/tasks/p4-codex-source.md`). Every file here is built from
the schema, not a real log: written 2026-11-06 from the research sketch in `docs/research/other-agent-signals.md`
("Fixture sketches") and the line and event types of Codex 0.167.0's source (`history/src/lib.rs` `RolloutLine`,
`history/src/rollout_payload.rs`, `protocol/src/protocol.rs` `EventMsg`). Field order and extra fields will differ in
real files; `p4-live-codex` adds real ones in `live/`. UTF-8, LF, one JSON object per line, a newline after the last
line, as Codex writes them.

| File | Shape | Activity |
|---|---|---|
| `running.jsonl` | `task_started`, then items and a `token_count`: a turn running | working |
| `complete.jsonl` | `task_started` then `task_complete` | idle |
| `aborted.jsonl` | `task_started` then `turn_aborted` (`interrupted`) | idle |
| `aborted-no-turn-id.jsonl` | `turn_aborted` with no `turn_id`, valid in 0.167.0 (older records) | idle |
| `aliases-running.jsonl` | `turn_started` (the serde alias) | working |
| `aliases-complete.jsonl` | `turn_started` then `turn_complete` (both aliases) | idle |
| `before-turn.jsonl` | `session_meta` and `thread_settings_applied` only: no turn yet | idle |
| `junk.jsonl` | not JSON, a blank line, an array, a bare string, a torn `task_complete`, a turn event quoted inside a message, other line kinds, other event kinds, a numeric `type`; one real `task_started` | working |
| `replaced.jsonl` | a turn aborted as `replaced`, then the next `task_started` | working |
| `fork-mid-turn.jsonl` | a fork: the parent's copied `task_started` and its `turn_aborted` boundary, then the child's own turn, complete | idle |
| `odd-no-type.jsonl` | an `event_msg` whose payload has no `type` after a `task_started` | working, `UnknownValue("rollout", "event_msg")` |
| `odd-no-turn-id.jsonl` | a `task_complete` with `turnId` instead of `turn_id` | idle, `UnknownValue("rollout", "task_complete")` |
