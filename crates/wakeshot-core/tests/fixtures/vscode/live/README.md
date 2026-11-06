# VS Code chat session logs, live

Redacted copies of real chat session logs, for `tests/vscode_source.rs` (card `docs/tasks/p4-live-vscode.md`).
Findings: `docs/research/copilot-vscode-signals.md`, "Phase 4 live test (2026-11-06)".

- VS Code 1.145.1 (commit `6bab64e2d12b`), built-in Copilot extension 0.73.0, Copilot Free (model `copilot/auto`), on
  the Windows 11 test laptop. The chats ran in the default "Local" harness, in a window of a test folder, started with
  `code chat` (VS Code ran elevated, see the card).
- Chats from 16:02 to 16:08 local time (UTC+4) on 2026-11-06. Times are epoch milliseconds, as VS Code writes them.
- Each file is the whole log as VS Code had written it at the time it was copied: UTF-8, one JSON object per line, a
  newline after the last line.

| File | What ran | What the log says |
|---|---|---|
| `terminal-call-waiting.jsonl` | Agent mode, one terminal call (`sleep 60`, auto approved by VS Code's default rule), then the chat waited for a click | one Initial line; the request's `modelState` is 4 (NeedsInput) |
| `ask-turn-done.jsonl` | Ask mode, a one-word answer in 3 s | one Initial line; `modelState` 1 with `completedAt` |

The session ids are fake (`0b1c2d3e-0000-4000-8000-0000000000<n>`); the tests feed each file under one made-up
path.

## Redaction

Done by a script that wrote only redacted text; no value was printed. No `state.vscdb`, `agent-host.db`,
`session.db` or token file was opened.

- Every line is kept, with its `kind`, its path `k` and its `i`, and every value keeps its shape: the same keys in the
  same order, the same list lengths.
- Kept as they are: numbers, `true`, `false`, `null`, and short plain words under `kind`, `modelId`, `agentId`,
  `toolId`, `mode`, `modeId`, `modeKind`, `extensionId`, `publisherName`, `extensionPublisherId`,
  `extensionDisplayName`, `location` and `type`.
- `sessionId` is the fake session id; each `requestId`, `responseId`, `id`, `toolCallId`, `callId` and
  `chatSessionId` is a fake id
  that stays the same within a file. Every other string is `"x"`, and an object key that is not a plain word is
  `k<n>`. So no prompt, answer, path, user name or account is left.
