# VS Code chat session logs, built from the schema

Built from the schema, not copied from real chat files: no real session file was opened. Each file is a chat's
`<session id>.jsonl` as VS Code 1.145.1 writes it (`chatSessionOperationLog.ts` for the fields,
`objectMutationLog.ts` for the line kinds), with the sketch in `docs/research/copilot-vscode-signals.md`, "Fixture
sketch". Real lines carry many more fields, mostly text; the values of fields the source does not read (`message`,
`response`, `inputState`, `agent`, `result`) are made up and their shapes are not checked. Made by a script (not
kept). Real files are in `live/`.

Times are 2026-11-06 UTC. The session id is `0b1c2d3e-0000-4000-8000-000000000001`; `requestId`s are made up.

| File | What it holds | `tests/vscode_source.rs` expects |
|---|---|---|
| `working.jsonl` | an Initial line with no request, request 1 pushed at 10:00:05 with `modelState` 0, answer text pushed | working, since 10:00:05 |
| `waiting.jsonl` | `working.jsonl`, a tool part, `modelState` 4 (the tool waits for approval) | waiting, no time, not remote |
| `working-again.jsonl` | `waiting.jsonl`, `modelState` 0 (approved) | working |
| `idle.jsonl` | `working-again.jsonl`, the last answer text, `modelState` 1 with `completedAt`, a `result` and a new draft in `inputState` | idle |
| `compacted.jsonl` | one Initial line: request 1 done (1), request 2 cancelled (2), request 3 at 10:10 pending (0) | working, since 10:10 |
| `several-requests.jsonl` | request 1 done in the Initial line, request 2 pushed and cancelled, then a resend: a Push with `i` 1 puts request 3 (10:10, pending) in its place | working, since 10:10 |
| `reopened.jsonl` | request 1 pushed, failed (3), then back to 0 (a retry from the error card) | working, any start |
| `unknown-kind.jsonl` | request 1 done in the Initial line, a kind 5 line in the answer text, a kind 7 line on request 1 | idle, unknown `kind` 7 |
| `unknown-state.jsonl` | request 1 pushed, then `modelState` 9 | idle, unknown `modelState` 9 |
| `empty-window.jsonl` | an Initial line with no request and a draft: an empty window's chat | idle, any start |
| `partial-last-line.jsonl` | the first two lines of `working.jsonl`, then a `modelState` write cut after 30 bytes, with no line end | one retry, then unreadable |
