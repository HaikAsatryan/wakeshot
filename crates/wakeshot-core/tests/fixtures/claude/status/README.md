# Claude Code status file fixtures

Whole `sessions\<pid>.json` files for `tests/claude_status.rs`. Made 2026-11-03 from the redacted log of a throwaway
watcher program (Claude Code 2.1.316; the program and its log are not kept): each file is the log's `NEW` line for that
pid with its later `CHANGE` lines applied up to the line named below. Key order is the one Claude Code writes, as the
2.1.317 live fixtures in `../live/` show (the log prints keys sorted). UTF-8, no byte order mark, no trailing newline.

| File | Shape | Log lines |
|---|---|---|
| `before-status.json` | CLI, first write: no `status`, no `updatedAt` | 2 |
| `idle-remote.json` | CLI, `idle` with Remote Control | 30 |
| `waiting-permission.json` | CLI, `waiting` / `permission prompt`, Remote Control on | 196 to 207 |
| `waiting-input.json` | CLI, `waiting` / `input needed`, Remote Control on | 196 to 217 |
| `waiting-dialog.json` | CLI, `waiting` / `dialog open` (a slash command dialog) | 196 to 235 |
| `remote-off.json` | CLI, Remote Control off: `bridgeSessionId: null` and `idle` in one write | 196 to 236 |
| `sdk-idle.json` | SDK path (`entrypoint: sdk-cli`), `idle`, no `bridgeSessionId` | 336 |
| `sdk-waiting-permission.json` | SDK path, `waiting` / `permission prompt`, no Remote Control | 336 to 344 |
| `empty.json` | Broken: 0 bytes (a read between Claude's truncate and write) | made |
| `half-written.json` | Broken: the first half of `idle-remote.json` | made |
| `trailing-bytes.json` | `remote-off.json` followed by the tail of the longer `waiting-dialog.json`, as a shorter write over a longer one leaves it (issue anthropics/claude-code#96438). Parses: only the first JSON value counts | made |
| `pid-mismatch.json` | `idle-remote.json` with `pid` 21600; the test feeds it as `33448.json` | made |
| `unknown-status.json` | `idle-remote.json` with `status: "compacting"`, a value Wakeshot does not know | made |
| `no-proc-start.json` | `idle-remote.json` without `procStart` | made |

## Redaction

As the watcher log was redacted, plus one change (`bridgeSessionId`):

- The Windows user name is `USER`, the host `HOST`. Session ids are the log's fake UUIDs `5e55....-0000-4000-8000-...`.
  Pipe names are the log's numbered ones.
- `bridgeSessionId` is `BRIDGE1` or `BRIDGE4` (the log's `session_BRIDGE01` and `session_BRIDGE04`), under 8
  characters, so the repo's secret scan pattern for that key does not match it. The parser only checks that the value
  is a string.
- Kept as they are: pids, `procStart` FILETIMEs, epoch times, `name` (made from the test folder name), enum values.
