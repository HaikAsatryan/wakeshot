# OpenCode answers

Each file is one answer of a query in `crates/wakeshot-core/src/sources/opencode.rs`, in the JSON that
`crates/wakeshot-windows/src/sqlite.rs` writes: `{"rows":[[...],...]}` with integer, text or null cells, or
`{"error":"<kind>"}`. Written by hand from the queries and the `@opencode/cli` 2.2.5 schema
(`core/src/database/schema.gen.ts`); the queries themselves run on a real database in the `sqlite` module's tests.
Ids have OpenCode's shape: `ses_` or `sch_` and 26 letters and digits. Times are wall ms.

| File | Query | What it is |
|---|---|---|
| `tables-both.json` | `tables` | a 2.x file: `session_v2` and `schedule` |
| `tables-sessions.json` | `tables` | `session_v2` only |
| `tables-none.json` | `tables` | neither: a file only OpenCode 1.x wrote |
| `working.json` | `working` | two sessions whose turn runs |
| `working-none.json` | `working` | no turn runs |
| `schedules.json` | `schedules` | three schedules of a working session (two due at the same time), one of an idle session, one with no session |
| `error-prepare.json` | any | a query that did not prepare (a renamed column) |
| `error-no-sqlite.json` | any | no usable `winsqlite3.dll`: no signal, and no problem entry |
| `working-bad-rows.json` | `working` | one good row and rows of the wrong shape: swapped cells, too few, a real number, a text that is no id, too many, a bare prefix, not a list |
| `schedules-bad-rows.json` | `schedules` | one good row and rows of the wrong shape: a number for the session, a session id for the id, no due time, a text that is no session id |
| `not-json.txt` | any | bytes that are not JSON |
| `rows-not-a-list.json` | any | JSON without a list of rows |
