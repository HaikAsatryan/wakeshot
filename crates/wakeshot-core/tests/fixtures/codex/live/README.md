# Codex CLI live rollouts

Redacted rollouts of live Codex CLI sessions, for `tests/codex_source.rs` (card `docs/tasks/p4-live-codex.md`).
Findings: `docs/research/other-agent-signals.md`, "Phase 4 live test (2026-11-06)".

- Codex CLI 0.167.0 (npm `@openai/codex`), the TUI and `codex exec`, on the Windows 11 test laptop, at Medium in a
  pseudo console (`spikes/agent-drive/`). Model `gpt-6-luna`, reasoning `low`, `-s read-only` (the TUI with
  `-a on-request`).
- Sessions ran from 14:26 to 14:52 local time (UTC+4) on 2026-11-06. `timestamp` values are UTC.
- UTF-8, LF, one JSON object per line, a newline after the last line, as Codex writes them.

| File | What ran | Turn events (line) |
|---|---|---|
| `tui-four-turns.jsonl` | 1: `Start-Sleep -Seconds 60`. 2: a file write that needed an approval, left waiting 60 s, then approved. 3: the sleep again. 4: the sleep, Esc after 12 s | `task_started` 2, `task_complete` 31; 33, 52; 54, 71; `task_started` 73, `turn_aborted` (`interrupted`) 81; an `item_completed` of the sleep after the abort, 82 |
| `tui-reverted.jsonl` | The same thread after a backtrack (Esc Esc, left, Enter) to before turn 3, then one edited prompt with no tool. Codex wrote it as `rollout-<time>-<thread id>_<rollout id>.jsonl` | `task_started` 4, `task_complete` 12 |
| `tui-crash-mid-turn.jsonl` | A second TUI: the sleep, then the TUI's `codex.exe` killed by pid while it ran | `task_started` 2, no end |
| `exec-one-turn.jsonl` | `codex exec` (14:51) with the sleep, beside a dry-run tray that held for it (`originator` `codex_exec`) | `task_started` 2, `task_complete` 31 |

The thread id of the first two files is `0199cccc-0000-7000-8000-000000000002` (fake; one id map for all four files).

## Redaction

Done by a script that wrote only redacted text; nothing of a value was printed. No `auth.json`, `.sandbox-secrets` or
other token file was read.

- Every line is kept, with the same keys in the same order. Every UUID is a fake `0199cccc-0000-7000-8000-<n>`.
- Every other string is `<str N>` (N = length) unless it is an ISO time or a short lowercase word under `type`,
  `role`, `status`, `reason`, `phase`, `originator`, `source`, `collaboration_mode_kind`, `approval_policy`,
  `approvals_reviewer`, `effort`, `summary`, `mode`, `thread_source`, `history_mode`, `model_provider`, `model`,
  `cli_version`, `multi_agent_version`, `sandbox` or `kind`. So no prompt, answer, path, account id or plan is left.
- A key that holds a path separator, a colon or the user name would be `<key N>` (none did).
- Objects and arrays nested 5 levels deep or more are emptied (`{}`, `[]`): tool schemas and settings trees carry no
  turn signal. Numbers, booleans and nulls are kept.
