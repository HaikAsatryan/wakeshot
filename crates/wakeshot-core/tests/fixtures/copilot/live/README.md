# Copilot CLI live event logs

Redacted `events.jsonl` files of live Copilot CLI sessions, for `tests/copilot_source.rs`. Findings:
`docs/research/other-agent-signals.md`, "Copilot CLI live test (2026-11-06)".

- Copilot CLI 1.0.106 (npm `@github/copilot`) on the Windows 11 test laptop, at Medium: the TUIs in a pseudo
  console driven by a throwaway script (not kept) through VS Code's `copilot` shim, the one-shot from a script opened
  through `explorer.exe`. Copilot Free; Copilot Fusion picked `gpt-6-luna`.
- Sessions ran from 15:51 to 16:31 local time (UTC+4) on 2026-11-06. `timestamp` values are UTC.
- UTF-8, LF, one JSON object per line, a newline after the last line, as Copilot writes them.

| File | What ran | Lines the tests check |
|---|---|---|
| `tui-a.jsonl` | 1: `Start-Sleep -Seconds 60` behind an approval left 60 s, then "Yes". 2: an `ask_user` question left 2 min 40 s, answered. 3: `Start-Sleep -Seconds 40`, approved, stopped by Ctrl+C. Then `/every 2m <prompt>`, its one fire, its delete, `/exit` | `user.message` 5; `permission.requested` 19, `.completed` 20; last `assistant.turn_end` 26; 28; `tool.execution_start` (`ask_user`) 40, `tool.execution_complete` 42, turn end 47; 49; 62, 63; `abort` 65; `session.schedule_created` 67; the fire (`source` `schedule-1`) 68, its turn end 72; `session.schedule_cancelled` 74; `session.shutdown` 75 |
| `tui-crash-mid-turn.jsonl` | A second TUI with `--allow-tool "shell(Start-Sleep)"`: `Start-Sleep -Seconds 60`, then its `copilot.exe` killed by pid while it ran | `user.message` 5, `tool.execution_start` 18, no end |
| `oneshot-npm.jsonl` | `copilot -p ... -s --trust-folder --allow-tool "shell(Start-Sleep)"` through npm's `copilot.cmd` (no shim), in a scratch `COPILOT_HOME`, beside a dry-run tray: `Start-Sleep -Seconds 30`, then a `read_powershell` call, then the answer | `user.message` 5; turn ends 12 and 18 that tools follow; the last `assistant.turn_end` 22; `session.shutdown` 24 |

## Redaction

Done by a script that wrote only redacted text; nothing of a value was printed. No `gh` hosts file, Copilot
`config.json` or other token file was read, and no line of a session the test did not start is in these files.

- Every line is kept, with the same keys in the same order. Every UUID is a fake `0199c0de-0000-4000-8000-<n>`, and
  every `toolCallId` a fake `call_<n>` (one map for all files).
- Every other string is `<str N>` (N = length) unless it is an ISO time or a short lowercase word under `type`,
  `source`, `toolName`, `reason`, `shutdownType`, `kind`, `delivery`, `agentMode`, `permissionMode`,
  `decisionSource`, `actor`, `origin`, `producer`, `copilotVersion`, `model`, `role`, `status` or `mode`. So no
  prompt (all were one-line test prompts), answer, path, account or plan is left.
- A key that holds a path separator, a colon or the user name would be `<key N>` (none did).
- Objects and arrays nested 5 levels deep or more are emptied (`{}`, `[]`). Numbers, booleans and nulls are kept;
  the only numbers over 10^6 are times (`created`, `sessionStartTime`, `timestampMs`) and `totalNanoAiu`.
