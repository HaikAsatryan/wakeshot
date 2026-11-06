# Review: OpenCode database read (security and safety)

Reviewer: `wakeshot-reviewer`, 2026-11-06. Scope: `crates/wakeshot-windows/src/sqlite.rs`,
`crates/wakeshot-core/src/sources/opencode.rs`, `crates/wakeshot-core/tests/opencode_source.rs`, the `Query` plumbing
in `crates/wakeshot-windows/src/watch/agents.rs` and `thread.rs`. Read against `docs/tasks/p4-opencode-source.md`,
`docs/research/opencode-signals.md`, `docs/research.md` "OpenCode SQLite reader checks" and architecture sections 6
and 7. The real `opencode.db`, `service.json` and token files were never opened. No cargo run: the findings come from
the code, SQLite's source as I know it (marked), and three probes on scratch databases (below).

Counts: 0 blocker, 1 major, 7 minor.

## Probes (scratch only)

Scripts in `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p4-opencode-review\scripts\`
(PowerShell P/Invoke into this laptop's `winsqlite3.dll` 3.53.4, the default VFS; data folders deleted after):

- `probe.ps1`: a writer process holds a WAL database open; a reader process (`SQLITE_OPEN_READONLY | NOMUTEX`, 100 ms
  busy timeout, the working query) reads it while a `FileSystemWatcher` with Wakeshot's filter (file name, folder
  name, last write, size) records notices. Then the writer is killed by its pid and the reader reads the files it
  left, with the `-wal` marked read only so the default VFS opens it read only, as the shim does.
- `empty-db.ps1`: a read on a 0-byte database with a `-wal` and a `-shm` next to it.
- `scan-holes.ps1`: SQL forms the table test may miss, prepared on an in-memory database.

## Major

### 1. After OpenCode dies without closing the database, Wakeshot reads it three times a second, forever

`crates/wakeshot-windows/src/watch/agents.rs:595-601` and `:655-664` (a notice naming `opencode.db-shm` re-runs every
query), `:1254-1295` (the limit only delays it), `crates/wakeshot-windows/src/sqlite.rs:190-198` (`open_by_agent`).

What breaks: the gate assumes "the db, `-wal` and `-shm` exist" means "the agent has it open". A killed, crashed or
signed-out OpenCode leaves both files. Then every read is the first connection to the `-shm`, so SQLite truncates it,
grows it again and rebuilds the WAL index from the whole `-wal` (SQLite's dead-man-switch rule). Those writes raise
size and last-write notices on `opencode.db-shm`, which `names_db_file` accepts, so each read schedules the next one
at `QUERY_EVERY_MS`. Probe: with the writer open and idle, three reads gave 0 notices; after the kill, each read gave
3 `Changed opencode.db-shm` notices and a new `-shm` write time. In Wakeshot that is a loop of three reads a second
(tables, working, schedules), each rewriting OpenCode's 32 KB `-shm` and reading its whole `-wal`, with a timer wake
every second, until an OpenCode opens and cleanly closes the file again. It also survives a Wakeshot restart and a
reboot. How often a real OpenCode leaves the files: not verified (the live card's crash test will show it); the loop
itself is verified.

Fix (lean): run a query only on notices that name `<db>-wal` or `<db>` (an agent's commit always writes the `-wal`, a
checkpoint the db; the reader's own footprint is only the `-shm`), and say in the docs that the files can outlive the
agent. Change the `agents.rs` limit test that feeds `OPENCODE.DB-SHM`. Stronger option, change it when the live test
shows leftovers are common: skip `Query` runs while no process of the source's images lives (a contract field).
Add the crash-leftover case to the live card.

## Minor

### 2. The shim still lets SQLite delete the agent's `-wal`

`crates/wakeshot-windows/src/sqlite.rs:389-396` (every method but `xOpen` is the default VFS's own, `xDelete` too);
claim at `:15-16` ("its close never writes or deletes the agent's files") and `docs/research.md:5473-5476`.

What breaks: when the db file is 0 bytes and a `-wal` exists, SQLite deletes the `-wal` on the first read
(`pagerOpenWalIfPresent`, `nPage==0`; from SQLite's source as I know it). Probe `empty-db.ps1`: a read-only read of a
0-byte `opencode.db` with `-wal` and `-shm` present deleted the `-wal`. The shim copies `xDelete`, so Wakeshot does the
same, and the `-shm` check passes first. OpenCode should never make that state (switching to WAL writes page 1
first), so the harm is small, but "never deletes" is a source-reading claim, not a code guarantee.

Fix: give the shim its own `xDelete` that refuses (return `SQLITE_IOERR_DELETE`); the read then fails as `open` or
`prepare` and nothing is removed. Add a test with a 0-byte db, `-wal` and `-shm`.

### 3. The table test misses valid SQL that reads another table

`crates/wakeshot-core/tests/opencode_source.rs:326-358` (no comment handling), `:375-381` (a `(` after `FROM` or
`JOIN` ends the scan), and no check of a table after `IN`.

What breaks: these all name `account` and pass `foreign_tables` empty (traced through `tokens` and `tables_named`),
and `scan-holes.ps1` shows SQLite accepts and runs each of them:
`SELECT * FROM (account)`, `SELECT id FROM session_v2 JOIN (account) ON 1`, `SELECT * FROM/**/account`,
`SELECT * FROM -- c` + newline + `account`, `SELECT id FROM session_v2 WHERE id IN account`. The three shipped queries
are clean (checked by hand), so today nothing reads a token table; the gap is in the guard Haik asked for.

Fix (lean): in the scan, skip `--` and `/* */` comments, treat a word after `(` that follows `FROM` or `JOIN` as a
table, and treat a word after `IN` (not `(`) as a table; add the five cases to
`the_table_scan_finds_every_table_a_query_names`. Stronger, change it when a query gets more complex: let SQLite name
the tables. A `sqlite3_set_authorizer` callback in `run_on` that allows `SQLITE_READ` only on `sqlite_master`,
`session_v2` and `schedule` and denies `PRAGMA`, `ATTACH` and the rest makes the rule hold at run time too, also
against a crafted file whose `session_v2` is a virtual table over another table (not verified that `winsqlite3` has
FTS5).

### 4. Each close tries to take an exclusive lock on `opencode.db`

`crates/wakeshot-windows/src/sqlite.rs:222-227`, `:309-310` (plain `sqlite3_close`, no `NO_CKPT_ON_CLOSE`).

What breaks: on close of a WAL connection SQLite asks for `EXCLUSIVE` on the db file before its checkpoint on close
(`sqlite3WalClose`; from SQLite's source, not measured). While OpenCode is open this fails, but Windows' lock code
takes the pending byte first and keeps it until the handle closes, so a new OpenCode connection opening in that
instant gets `SQLITE_BUSY` (OpenCode's 5 s busy timeout covers it). When Wakeshot is the last connection (finding 1,
or OpenCode closing under the read) it gets the lock; only the read-only `-wal` then stops the checkpoint.

Fix: after the open, `sqlite3_db_config(conn, SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE (1006), 1, null)` (since 3.16.0; a
variadic `extern "C"` export). The close then skips the lock and the checkpoint, so the read-only `-wal` is no longer
the only guard.

### 5. No bound on a cell's size or a scan's cost

`crates/wakeshot-windows/src/sqlite.rs:320-332` (any length of text is copied), `:56-58` (`MAX_ROWS` bounds rows, not
bytes), `crates/wakeshot-windows/src/watch/agents.rs:1286-1287` (the answer is kept again as `QueryFed::Answer`).

What breaks: an `id` of hundreds of MB in a strange file is loaded by SQLite, copied into a `String`, into the JSON,
into `QueryFed` and parsed again by the source, only to be refused at 128 characters (`opencode.rs:305-312`). Separate
point, not verified: the working query scans every `session_v2` row, and `time_busy` sits after `summary_diffs`,
`metadata`, `revert` and `permission` (`sqlite.rs:556-566`), so a long history of large rows means walking overflow
pages on every read, up to three times a second. The real row sizes are unknown.

Fix: `sqlite3_limit(conn, SQLITE_LIMIT_LENGTH, 4096)` after the open (a longer value fails the step, answer `size`),
or check `column_bytes` against a small cap in `cell`. Have the live card time one read on a real, long-used
`opencode.db` and log it if it passes, say, 50 ms.

### 6. The SQLite tests never cross a process

`crates/wakeshot-windows/src/sqlite.rs:453-519` (the `Writer` runs in the test's own process).

What breaks: SQLite shares one `-shm` mapping per process, so the reader in these tests reuses the writer's mapping
and never opens the `-shm` itself, never meets the first-opener truncation, and never sees files a dead writer left.
That is how finding 1 passed. The "creates no file" and checkpoint tests are right for what they cover.

Fix: one test with the writer in a child process (the crate already has child roles), covering: writer open (no
notice from a read), writer killed (finding 1), and writer closing during reads.

### 7. Docs that do not match the code

- `docs/architecture.md:1132-1134` and `:1225-1227`: "only while its `-wal` file exists"; `sqlite.rs` also needs the
  `-shm` (the card noted it, not fixed).
- `docs/architecture.md:1129`: "Every open shares read, write and delete". SQLite's own opens share read and write
  only (`winOpen`, from SQLite's source), so an OpenCode delete or rename of its files fails while a read runs.
- `crates/wakeshot-windows/src/sqlite.rs:8-9`, `docs/research/opencode-signals.md:213-215`: "while the `-wal` exists
  the agent has it open" is false after a crash (finding 1).
- `docs/research/opencode-signals.md:54-57` and the card's Question 1: "SQLite never loads their pages". A WAL
  rebuild (finding 1) reads every frame in the `-wal`, token pages included, into Wakeshot's memory (nothing is kept
  or logged).

Fix: one line each.

### 8. A Windows without a usable `winsqlite3.dll` shows "update Wakeshot"

`crates/wakeshot-windows/src/sqlite.rs:3-4` ("the source reads that as no signal"), `crates/wakeshot-core/src/sources/opencode.rs:252-258`,
`crates/wakeshot-core/tests/opencode_source.rs:211-234`.

What breaks: `no_sqlite` is fed like any error, so it becomes `Unreadable("opencode.db")`, the core logs "cannot read
opencode.db" and the user gets the format notice while OpenCode runs. Updating Wakeshot cannot fix a missing or old
system DLL. Rare on Windows 11 (an x64 build on ARM64 is the likely case; not verified).

Fix: have the source treat `{"error":"no_sqlite"}` as no signal with no problem entry (or a fixed log line once), and
fix the doc line either way.

## Checked, no finding

- FFI: every signature matches `sqlite3.h` (`open_v2`, `close`, `busy_timeout`, `prepare_v2`, `step`, `finalize`,
  `column_count/type/int64/text/bytes`, `libversion_number`, `vfs_find`, `vfs_register`, `xOpen`); `extern "system"`
  is the C convention on x64 and ARM64. `Vfs` matches `sqlite3_vfs` version 3 (3 ints, 3 pointers, `xOpen`, 15 more
  pointers); `ptr::read` of a version 3 or later VFS reads only those fields. `Shim` puts `Vfs` first, so the cast in
  `open_read_only` is sound. `flags & (A | B) != 0` parses as intended in Rust. Text is read before its length, and
  copied before the next step. Statements drop before the connection on every path. No panic can cross the callback.
- Read only: the main db and `-wal` get `READONLY` without `CREATE`; the pager's read-only flag stops a hot-journal
  rollback; a read-only `-wal` stops every checkpoint (from SQLite's source). The `-shm` is opened by the default VFS
  directly with create, but only after a shared lock on the db, and OpenCode deletes both files only under an
  exclusive lock, so a close racing a read cannot leave a new `-shm` (reasoned from SQLite's locking order; the
  `-wal`-missing half is tested).
- DLL: `LOAD_LIBRARY_SEARCH_SYSTEM32` only; any missing export, a version under 3.22.0, a default VFS under version
  3 or a failed register gives `None`, so `no_sqlite`. Not verified on ARM64.
- Logs: query failures reach the log only as "OpenCode: format problem in opencode.db" and the core's "cannot read
  opencode.db"; no path, no row value, no SQLite text; the core logs labels, never session ids
  (`core.rs:339-397`). The panic hook logs only panic text, and nothing on this path panics on file data.
- Hostile content: wrong types, `REAL`, blobs, non-id text, views or triggers named like the tables (the probe asks
  `type = 'table'`), corrupt files and the 10,000-row cap all end in `Unreadable` with good rows kept, no crash.
- Watch thread: one run per want per second, a held-back run fires without a tick, `Busy` feeds nothing, `Gone` once
  when the `-wal` goes, and no handle stays open between reads.
