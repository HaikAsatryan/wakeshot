# Verify: OpenCode database read, major 1

Reviewer: `wakeshot-reviewer`, 2026-11-06. Checks the fix of major 1 in `docs/reviews/p4-opencode-db-read.md` (made by
`p4-contracts-r3`, item 6) and looks for a new blocker or major in the minor fixes (`p4-opencode-source`, "Review
fixes"). Read only: code, the two cards, architecture sections 6 and 7. No cargo run, no git, the real `opencode.db`
not opened.

## Major 1: reads loop on `-shm` notices after a crashed writer

**Fixed.**

- `crates/wakeshot-windows/src/watch/agents.rs:661-673`: `names_db_file` now takes only `<db>` and `<db>-wal`, case
  ignored, right in the folder. `-shm`, `-journal`, `-wal2`, `opencode.dbx` and a name in a subfolder no longer run a
  query. `covers_query` (`:597-607`) uses it for every named notice, so a `-shm` notice makes no `Unit::Query`, never
  calls `run_query` and never sets a held-back `due`. A notice that lost its names still runs every query, as before.
- This breaks the loop because a read changes only the `-shm`. My earlier probe (`probe.ps1`) showed it: on files a
  killed writer left, each read gave 3 notices, all `opencode.db-shm`, and none for the db or `-wal`. With the writer
  open, a read gave none. The new cross-process `sqlite.rs` tests check the same: db and `-wal` bytes and write times
  stay.
- Tests: `only_a_notice_of_the_database_or_its_wal_runs_a_query` (`agents.rs:3563-3589`, the names above, and
  `next_due` stays `None`) and `a_query_that_rewrites_the_shm_runs_once_not_in_a_loop` (`thread.rs:1517-1550`, real
  notices from a fake read that rewrites the `-shm`, one run in three seconds). The limit test now feeds
  `OPENCODE.DB-WAL` instead of the `-shm` (`agents.rs:3538`). The card's Log says both new tests failed with the old
  rule. I did not run them.
- Docs match: the `agents.rs` module doc (`:13-16`) and architecture `:1139-1146` and `:1240-1247`.

What is left (known, bounded, not a loop): a crash leaves the files, so each 30 s safety tick still runs three reads,
and each rebuilds the `-shm` from the whole `-wal`. The stronger option (no `Query` run while no OpenCode process
lives) is still open, as `p4-contracts-r3` "Found outside scope" item 2 says. This matters now: the same card
registers the OpenCode source in the tray (its item 8), so the live card should run the crash-leftover case.

## New blocker or major from the fixes

None found. Checked in `crates/wakeshot-windows/src/sqlite.rs`:

- The shim's own `xDelete` (`:111`, `:124-126`, `:438`, `:468-475`): the signature matches `xDelete`, and `Vfs` still
  spells version 3 (`xOpen`, `xDelete`, then 14 pointers). A read-only connection deletes nothing else through the
  VFS (temp files close with delete-on-close; the `-shm` purge calls the default VFS directly and runs only after a
  close checkpoint, which no longer runs).
- `NO_CKPT_ON_CLOSE` (`:148-149`, `:288-299`): a variadic `extern "C"` type for the one cdecl export, called with an
  `int` and a null `int*`, as option 1006 takes. A failed call ends the read as `open`, so nothing reads without it.
- `MAX_CELL` 1,024 (`:356-363`): checked before the copy. The selected columns are ids, names and integers, so a real
  OpenCode row stays far under it.
