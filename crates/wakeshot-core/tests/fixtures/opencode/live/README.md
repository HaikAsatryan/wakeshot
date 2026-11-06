# OpenCode answers, live

Answers of the three queries in `crates/wakeshot-core/src/sources/opencode.rs`, as Wakeshot's own reader
(`crates/wakeshot-windows/src/sqlite.rs`) gave them on a real OpenCode 2.2.5 database, for `tests/opencode_source.rs`
(card `docs/tasks/p4-live-opencode.md`). Findings: `docs/research/opencode-signals.md`, "Phase 4 live test
(2026-11-06)".

- `@opencode/cli` 2.2.5 (npm) with Ollama 0.40.15 and `qwen3:4b-instruct`, on the Windows 11 test laptop, on 2026-11-06
  from 16:30 to 17:45 local time (UTC+4). The database was the card's own test copy (`XDG_DATA_HOME` and
  `XDG_STATE_HOME` on a scratch folder), not Haik's.
- Read by a scratch probe that compiles the repo's `sqlite.rs` and runs the same three SQL strings. Times are wall ms,
  as OpenCode writes them, unchanged.

| File | Query | When |
|---|---|---|
| `tables.json` | `tables` | the first read after the TUI made the database: both tables, in the order SQLite gave |
| `working-tui-turn.json` | `working` | a TUI turn runs (16:30:33) |
| `working-none.json` | `working` | the turn ended |
| `schedules-two.json` | `schedules` | two schedules of one session, made through `opencode api`: in 3 minutes and in 2 hours |
| `working-schedule-fired.json` | `working` | the 3 minute schedule fired: its turn runs (17:08:07) |
| `schedules-after-fire.json` | `schedules` | the same moment: the fired one is `done`, only the 2 hour one is left |
| `working-crash-and-newer.json` | `working` | the service and TUI were killed mid-turn; a newer standalone `opencode run` works beside the row the crash left |
| `working-resumed.json` | `working` | the managed service started again and resumed the claimed turn (a new `time_busy`) |

## Redaction

No prompt text, title, folder or path was ever selected: the queries take ids, a session id and integers only. The
probe replaced every id with a fake one of OpenCode's shape (`ses_live` or `sch_live` and a number, 30 characters),
the same fake for the same id within a run; the files use one fake per real session across the runs. Nothing else
was changed. No token table was read, and no `service.json` was opened.
