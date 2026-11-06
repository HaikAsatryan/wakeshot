# Live test: OpenCode 2.x

Phase: 4. Status: done (2026-11-06). Needs Haik: no (OpenCode 2.2.5 runs on Ollama with `qwen3:4b-instruct`, no login).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-opencode-source`, `docs/reviews/p4-opencode-db-read.md` fixes, `p4-live-harness`; one live card at
a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-opencode\target`; delete it when done.
Real laptop: OpenCode and Ollama for Haik's user. No power settings, no screen captures.

## Goal

The live test of the OpenCode 2.x source: the rules of `docs/research/opencode-signals.md` and the build of
`p4-opencode-source` on a real OpenCode, redacted fixtures, fixes with tests, and the source wired into the tray.

## Read first

- `docs/research/opencode-signals.md` ("What is left" and the 2.x rules), `docs/tasks/p4-opencode-source.md` Result
  and Haik's answers, `docs/reviews/p4-opencode-db-read.md`, `docs/tasks/p4-live-harness.md` Result (runbook,
  driver, the OpenCode steps it already ran).

## Deliverables

1. Sessions at Medium, each timed (write to post): a working turn in the TUI and one with `opencode run`, a
   permission prompt left 1 minute then approved, Esc during a turn, a clean exit, a crash (kill by PID an
   `opencode.exe` you started), a schedule due in a few minutes (fires while the server runs; the 1 hour rule), and a
   crash leftover row with a newer OpenCode running (the `ImageSince` limit in the Result).
2. Privacy and safety, live: while Wakeshot reads, OpenCode is never blocked (no `SQLITE_BUSY` error in OpenCode's
   own log), Wakeshot creates no `-wal` or `-shm` file and opens the database read only (Process Monitor is not
   installed: use handle and file time checks you can run at Medium; say what you used), and no row value reaches the
   harness or app log.
3. Run `agent-watch` with a new `--source opencode` row and record what it posted per step.
4. Fixtures: redacted rows into `crates/wakeshot-core/tests/fixtures/opencode/live/` (no prompt text, no user
   paths, fake ids), a test that replays them, fixes in `sources/opencode.rs` or `sqlite.rs` each with a test that
   failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`
   (never beside another dry-run card; never quit Haik's installed tray).
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/opencode-signals.md` and short lines in
   `docs/research.md`; anything not run stays "not tested live".

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never read
OpenCode's `service.json`; never open `opencode.db` yourself except through Wakeshot's own read path.

## Owns

`crates/wakeshot-core/tests/fixtures/opencode/live/`; fixes in `sources/opencode.rs`, `tests/opencode_source.rs` and
`crates/wakeshot-windows/src/sqlite.rs`; the OpenCode line in `tray_mode.rs` and in `agent-watch.rs`; the research
sections above.

## Questions for Haik

## Found outside scope

1. **A notice in OpenCode's data folder still publishes a snapshot**, and a snapshot with an `ImageSince` session lists
   every process (about 14 ms here). Seen only when my own probe rewrote a leftover `-shm` every 100 ms or so (11 % of
   a core in the watcher). Nothing in a real OpenCode does that: its `-shm` writes go through the mapping and gave no
   notice. Change it when a live report shows the cost: then a notice that names only a `-shm` publishes nothing.
   (`watch/*`, mine this round; left as is on purpose.)
2. **The harness driver ends the whole tree when the root child exits**, even in `serve` mode (`spikes/agent-drive`,
   `p4-live-harness`), so a TUI's `/exit` or a one-shot's end also kills OpenCode's background service. I used
   `opencode service start` and `stop` at Medium for the clean stops.
3. **`agent-watch --raw`** prints a Claude status-file line (`status=- ...`) for each OpenCode answer, which says
   nothing here (as `p4-live-vscode` item 4). `examples/agent-watch.rs` is `p4-antigravity-source`'s this round.
4. **The dry-run data folder** `%LocalAppData%\Wakeshot\dry-run` is shared by every card's dry-run tray, so its
   `logs\wakeshot.log` mixes cards; I read from the file's size at my start. Not deleted (not mine).
5. **OpenCode 2.2.5 did not give the model its schedule tools** with `experimental.schedules` true in the project's
   `opencode.jsonc`, while its server's schedule API worked. OpenCode's behavior; why: not verified.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started. Orchestrator rule: the tray and `agent-watch --source opencode` lines are already in
  (`p4-contracts-r3`); this card tests through them and does not edit `tray_mode.rs` or `agent-watch.rs`. Dry-run
  step takes turns through `<scratchpad>\dry-run.lock`.
- 2026-11-06 16:27: built `agent-watch` and `wakeshot-tray` (debug, x64) into my target folder. Probe
  `<scratch>\p4-live-opencode\probe` (outside the repo): compiles the repo's own `sqlite.rs` by `#[path]`, so every
  read is Wakeshot's read path (shim, read-only open, file checks); prints ids as fake ids, never another text column.
- 2026-11-06 16:28, Haik's real `opencode.db` (no OpenCode running since 14:06): db 11.6 MB, `-wal` 11.7 MB and
  `-shm` are there, left by `opencode service stop` at 14:06. One read at Medium of the three fixed queries: 11.4 ms
  (first, with the DLL load), 1.7 ms, 2.8 ms; no working row, no schedule. Db and `-wal` size and write time
  unchanged; only the `-shm` write time moved.
- 2026-11-06 16:29, choice: every session I start runs on its own `XDG_DATA_HOME` and `XDG_STATE_HOME` (scratch), so
  the test db, its `-wal` and the service registration are mine to delete and Haik's file is not written. Change it
  when a fact depends on the default folder (the tray's default path has its unit test; the real file had the read
  above).
- 2026-11-06 16:30 to 16:33, TUI turn 1 (prompt 1, no tool): `time_busy` 16:30:33.535, idle 16:31:10.280.
  `agent-watch` posted working only at the 16:30:48.7 tick (15.2 s) and idle at the 16:31:18.7 tick (8.5 s). Turn 2
  (prompt 2; the model declined the tool, 2.8 s turn) was not posted at all. No read ran on a change notice: a
  `FileSystemWatcher` on the db folder (same filter as Wakeshot's) logged no notice in 46 s with two commits. The
  `-wal` keeps its size (11,684,352 bytes from the first start; SQLite rewrites it from the start after each
  checkpoint) and its listed write time stays stale while OpenCode holds it, so no size or last-write notice comes.
- 2026-11-06 16:30 to 16:33, handle check (my `handles.ps1` at Medium: `NtQueryInformationProcess` class 51 on
  `agent-watch`'s own handle table, then `DuplicateHandle` and `GetFinalPathNameByHandleW`, 12 M polls): db opened
  `0x120089` (FILE_GENERIC_READ) and `0x100080` (attributes), `-wal` `0x120089`, `-shm` `0x12019F` (read and write,
  as every WAL reader). Seen only at the tick reads, none held between reads.
- 2026-11-06 16:35: sent the notice finding to the orchestrator. Its decision: the fix is mine, later; `watch/*` is
  `p4-live-copilot`'s until it ends. Run every live step through the tick now, end with Status "fix pending" and the
  poll design in the Result; I get resumed with ownership of `WantedFile::Query`, `watch/*`, `sources/opencode.rs`.
- 2026-11-06 16:37: TUI turn 2's step timed out waiting for the approval the model never asked (prompt count 2).
- 2026-11-06 16:38: the orchestrator: `p4-live-copilot` is done, the fix is mine now (the `Query` variant in
  `sources.rs` and the `AnyProcess` doc there, `watch/*`, `sources/opencode.rs`, `sqlite.rs`, their tests).
- 2026-11-06 16:40 to 16:50, the poll built: `WantedFile::Query` gains `poll_while: &'static [&'static str]`
  (OpenCode: `IMAGES`); `watch/agents.rs` `QUERY_POLL_MS` 2,000; each run, while `<db>` exists, sets the next run 2 s
  later; a read needs the db, the `-wal` and a listed process of `poll_while` (`AgentIo::processes`, the system list,
  asked at most once per call; a list that fails counts as yes); no such process after an answer feeds `Gone` once.
  The `AnyProcess` doc in `sources.rs` now says the pid-in-the-name rule. Tests: agents
  `a_query_reads_only_while_a_process_of_its_agent_runs`, `a_query_is_polled_while_its_database_is_there`, thread
  `a_commit_with_no_notice_is_read_by_the_poll` (real thread: read 2,009 ms after the commit); old tests moved to the
  poll (`next_due` is now the poll, the `-shm` test counts 2 runs in 3 s). Broken on purpose: with the poll off 5
  tests failed, with the process check off 3 failed; file put back (hash equal).
- 2026-11-06 16:48, fixed build (`agent-watch` restarted, pid 54412): TUI turn 3 (prompt 3, new session) busy
  16:48:24.724, working posted 16:48:26.602 (1.9 s); idle 16:48:53.713, posted 16:48:54.649 (0.9 s).
- 2026-11-06 16:49 to 16:54: turn 4 (prompt 4): the model again said it cannot run a tool. In 2.2.5 the tool is
  `shell`, not `bash` (`core/src/tool/plugin/shell.ts:21`).
- 2026-11-06 16:49:57: dry-run lock taken; dry-run tray (my build) at Medium, pid 50868, with `CLAUDE_CONFIG_DIR`,
  `CODEX_HOME`, `COPILOT_HOME`, `VSCODE_APPDATA` on empty scratch folders and `XDG_DATA_HOME` on my test folder, so
  only OpenCode can hold. Log: "OpenCode is watched in the folder XDG_DATA_HOME names". The dry-run log file was
  another card's; I read only from its size at my start (3,485 bytes).
- 2026-11-06 16:54:14 to 16:55:20, turn 5 (prompt 5, "use the shell tool to run echo hi"): busy 16:54:14.942,
  working posted 16:54:15.014 (0.07 s), tray "Holding starts: OpenCode" 16:54:15.988 (1.0 s). Approval on screen
  16:54:18.7, left 60 s: no second busy time, the session stays working, the hold stays. Approved 16:55:18.7; idle
  16:55:20.150, posted 16:55:21.061 (0.9 s), "Holding ends" 16:55:21.990.
- 2026-11-06 16:55:32 to 16:56:18, turn 6 (prompt 6): `ping -n 61` approved, ran 40 s (`PING.EXE` in the tree);
  working posted 0.44 s after busy. Two Esc 0.35 s apart at 16:56:17.5: OpenCode ran `taskkill.exe`, idle
  16:56:17.997, posted 16:56:19.217 (1.2 s), "Holding ends" 16:56:18.021 (the tray's own read).
- 2026-11-06 16:56 to 17:03, turns 7 and 8 (prompts 7, 8): asked the model for `schedule_create`, then
  `opencode_schedule_create` (the tool's real name, `core/src/tool/runtime.ts:288-292`): both times it said the tool
  is not in its list. So I made the schedules through OpenCode's own server: `opencode api POST
  /api/experimental/schedule` at Medium (`run\oc-schedule.ps1`; `opencode api` finds the server itself, I read no
  file of it), for my test session.
- 2026-11-06 17:04:30, a schedule in 2 hours: row 17:04:30.699, timer posted 17:04:31.696 (1.0 s); the tray logged
  no hold (due past 1 hour). 17:05:07.652, a schedule in 3 minutes: timer posted 17:05:07.733 (0.08 s), "Holding
  starts: OpenCode" 17:05:08.259. It fired while the service ran: `status` done, `last_fired_at` 17:08:07.654 (due
  .652); busy 17:08:07.664, working posted 17:08:07.845 with only the 2 h timer left; the turn (prompt 9) answered 0;
  idle 17:08:39.868, posted 17:08:41.840 (2.0 s), "Holding ends" 17:08:40.379. No gap: the hold ran from the timer
  into the turn.
- 2026-11-06 17:09 to 17:10, CPU (debug builds, OpenCode running idle, one idle session with the 2 h timer listed):
  `agent-watch` 906 ms a minute, the dry-run tray 1,000 ms a minute; that is about 30 ms a poll. Too much to ship as
  is; measured again in release below.
- 2026-11-06 17:10:45: dry-run tray quit by `taskkill /PID 50868` (no force), code 0; lock released 17:10:55.
- 2026-11-06 17:11:29 to 17:11:58, crash with the TUI alive (prompt 10): `ping -n 121` approved; busy 17:11:29.619
  (`time_suspended` set with it), working posted 0.36 s later. 17:11:50.381 the driver killed the service
  `opencode.exe` 52312 by pid; the TUI (started 16:30:05, before `time_busy`) lived, so the row stayed working
  (`ImageSince`, the known limit). 5.1 s later the TUI started a new service (26928); it resumed the claimed turn:
  new `time_busy` 17:11:55.681, `resume_attempts` 1, screen "Tool execution interrupted: shell", "Continuing after
  restart", answer 0; idle 17:11:56.927, posted 17:11:58.006. So the stale stretch lasted 5 s, then it was real work.
- 2026-11-06 17:12:31, `/exit`: the TUI ended with exit 0 in 0.13 s; the service stayed (by design) until the
  driver ended its tree at 17:12:33.182 (a kill). `agent-watch` posted the idle session `alive=Gone` 0.06 s later
  (exit wait) and `n=0` 2 s later (the poll's `Gone`). Db, `-wal` and `-shm` stay.
- 2026-11-06 17:13 to 17:15, release `agent-watch` (opt z, LTO), no OpenCode running: 859 ms of CPU in 120 s, about
  14 ms a poll, all in the process list (`NtQuerySystemInformation` on 413 processes, 8,539 threads; no read runs).
  That is 0.7 % of one core while nothing happens, and the background service can live for days. Rejected: see the
  next line.
- 2026-11-06 17:15 to 17:30, the poll rebuilt on the `-shm` instead (my call; reason and trigger in the Result):
  `sqlite::wal_index_header` reads the first 48 bytes of `<db>-shm`, SQLite's WAL-index header, whose `iChange` is
  "incremented with each transaction" (sqlite.org/walformat.html, read 2026-11-06; bytes 0-47 first copy, read marks
  100-119, locks 120-127). Every 2 s while the db exists the watch thread reads those bytes; the query reads the db
  only when they differ from the bytes taken right after its last read (a busy read takes none), or a notice named
  the db or `-wal`. The tick follows the same rule, so files a stopped OpenCode left are read once, not every 30 s.
  No process list, no `poll_while` field (removed again from `WantedFile::Query`), no `AgentIo::processes`. Tests:
  sqlite `the_wal_index_header_changes_with_each_commit_and_never_with_a_read` (real WAL db: three reads leave it,
  each commit changes it, a missing or short `-shm` gives none and none is made), the cross-process kill test now
  also checks a read beside the live child leaves the header and that two rebuilds on the killed child's files
  write the same header (so two readers never wake each other); agents
  `a_query_is_polled_while_its_database_is_there`, `a_query_reads_again_only_when_the_wal_index_header_changed`,
  `a_read_that_rewrites_the_header_is_no_change_for_the_next_look`; thread `a_commit_with_no_notice_is_read_by_the_poll`; the `-shm` loop test counts 1 run again. Broken on purpose
  (file restored, hash equal): poll off, 4 tests failed; header check off, 3; header taken before the read, 2.
- 2026-11-06 17:25 to 17:28, release `agent-watch` with the header poll, no OpenCode running: 7.7 s of CPU in 120 s.
  Cause: my own timeline probe rebuilt the leftover `-shm` every 500 ms, each rebuild gave `-shm` notices, and every
  wake published a snapshot, whose `ImageSince` entry (the idle session with the 2 h timer) lists every process
  (about 14 ms). Fix in `watch/thread.rs`: a wake that was only the `Query` poll and read nothing publishes nothing
  (`Agents::changes`, feeds and query reads so far). Test `a_poll_that_finds_no_commit_lists_no_process` (a counting
  `Liveness`: one list in two quiet polls; a header change lists again within one poll); with the gate off it failed
  (3 lists, not 1). Then, probe stopped, release, no OpenCode: 172 ms in 120 s (0.14 % of one core, mostly the four
  tick snapshots).
- 2026-11-06 17:31:35: dry-run lock taken again; dry-run tray `wakeshot-tray-3.exe` (header poll and gate) pid 41520,
  same environment as before. 17:33:19, one-shot `opencode run` (prompt 11) through the driver: it started the
  service (5 s), busy 17:33:25.647, working posted 17:33:25.853 (0.21 s), "Holding starts: OpenCode" 17:33:25.856;
  answer 0; idle 17:33:53.896, posted 17:33:54.741 (0.85 s), "Holding ends" 17:33:54.684 (0.79 s). The driver then
  ended the service with its tree.
- 2026-11-06 17:34:25, `run\oc-service.ps1` at Medium: `opencode service start`, a one-shot through it (prompt 12,
  2 s, between two polls, so not posted: a turn under 2 s can be missed), `opencode service stop` (exit 0): db,
  `-wal` and `-shm` stay after a clean stop too. **Bug**: at the stop `agent-watch` logged "format problem in
  opencode.db" and posted the Unreadable entry for 2 s; again at a second stop (17:36:02).
- 2026-11-06 17:37 to 17:39, cause found with a scratch build of the probe that printed SQLite's code (a print in
  `sqlite.rs`'s `fail`, put back after the build, hash equal): reads in a tight loop during `opencode service stop`
  failed at prepare with `SQLITE_IOERR` (code 10) 14 and 10 times in a window of about 20 ms, then read fine. A second
  build (a print in the shim's `xDelete`, put back) showed no refused delete in that window, so it is OpenCode's
  close, not the shim. Fix in `sqlite.rs`: extended result codes on for each connection
  (`sqlite3_extended_result_codes`), and an I/O error other than the shim's own `SQLITE_IOERR_DELETE` answers `Busy`
  (no feed; the header is not taken, so the next look reads again). Test
  `an_io_error_of_the_agents_close_is_busy_but_the_shims_refused_delete_is_not`; `an_empty_database_keeps_its_wal`
  still gives `prepare`. Broken on purpose: extended codes off, the empty-db test failed; the mapping off, the new
  test failed; restored, hash equal.
- 2026-11-06 17:41 to 17:43, final build (`agent-watch-r4` release, dry-run `wakeshot-tray-4`): one more service run
  and stop (prompt 13) and three start and stop cycles with no prompt: no format problem at any of the 4 stops; the
  idle session with the 2 h timer went `alive=Yes` at each start and `Gone` at each stop; no hold (due past 1 hour).
- 2026-11-06 17:43:57 to 17:44:16, crash of the whole tree (prompt 14): TUI continuing my test session, `ping -n 121`
  approved, working posted 1 s after busy, "Holding starts" with it; the driver's `kill-tree` at 17:44:16.389:
  `alive=Gone` and "Holding ends" 66 ms later. The row stays busy with `time_suspended` set.
- 2026-11-06 17:44:32, a newer `opencode run --standalone` beside it (prompt 15, 1.2 s turn, not posted: under the
  2 s poll): the left row stayed `alive=Gone`, no hold. 17:45:12, `opencode service start`: the managed service
  resumed the claimed turn (new `time_busy` 17:45:17.888, `resume_attempts` 1, 0.24 s after the service started);
  working posted 0.09 s later with "Holding starts"; it ended at 17:45:21.092, idle posted and "Holding ends" 0.9 s
  later; then `opencode service stop`.
- 2026-11-06 17:45 to 17:47: second handle check (final build, 60 s, 3.8 M polls): db and `-wal` `0x120089` and
  `0x100080` only; `-shm` `0x12019F` (SQLite) and `0x120089` (the header read). Dry-run tray quit by pid (code 0),
  lock released 17:47:41. Measured after stopping my probes: release `agent-watch`, no OpenCode, the left row and the
  schedule entry listed: 172 ms in 120 s. With my probes rebuilding the leftover `-shm` every 100 ms or so, the
  watcher and the debug tray had spent 11 % and 12 % of a core (each notice publishes, each publish lists processes):
  an artifact of the probe, written under Found outside scope.
- 2026-11-06 17:48: OpenCode's own log in my folder: 944 lines, all `INFO`, no busy, locked or I/O error; Haik's real
  log: none either. Harness and tray logs: no prompt word, no marker `ZQX7`, no user name outside the harness start
  lines, no session id in the tray log or in a harness `log` line.
- 2026-11-06 17:49 to 17:52: 8 fixtures in `tests/fixtures/opencode/live/` from the probe's redacted answers (fake ids,
  real times) with a README; tests `every_live_fixture_is_listed_once` and
  `the_live_answers_give_the_facts_the_live_test_saw` (17 pass). No source fix needed: every live answer read with no
  format problem. Secret patterns, user name, paths, prompt words: no hit.
- 2026-11-06 17:52 to 17:55: research section, `docs/research.md` lines, architecture lines; all checks clean (Result).
  Every pid I started is gone (76 checked by pid; two were reused by Chrome and the search indexer after mine ended);
  Haik's two installed trays untouched. Deleted: build folder, my binaries, my OpenCode test folders (the db, its
  `-wal`, `-shm`, the service registration, never read), the `sqlite.rs` copy.

## Result

Done, 2026-11-06. OpenCode 2.2.5 (npm) on Ollama `qwen3:4b-instruct`, everything at Medium, about 20 prompts to the
local model (no cloud quota). Each OpenCode I started had its own scratch `XDG_DATA_HOME` and `XDG_STATE_HOME`;
Haik's real `opencode.db` was read once through Wakeshot's reader and never written. Detail:
`docs/research/opencode-signals.md`, "Phase 4 live test (2026-11-06)".

**What held** (after the fixes below; times are from the row's write to the post):

| Step | Outcome |
|---|---|
| TUI turn, no tool | working 1.9 s after `time_busy`, idle 0.9 s after `time_idle` |
| `opencode run` one-shot | starts the background service (5 s); working 0.21 s, idle 0.85 s; tray "Holding starts/ends" 0.21 s / 0.79 s |
| Permission prompt left 60 s, then approved | one busy span; working and held all through the wait; idle 0.9 s after the end |
| Esc during a running tool | two Esc 0.35 s apart; OpenCode ran `taskkill.exe`; idle posted 1.2 s later |
| Clean exit (`/exit`, `opencode service stop`) | TUI exit 0 in 0.13 s; the service stays until stopped; db, `-wal`, `-shm` stay after every stop |
| Crash, service killed, TUI alive | row counted 5 s through the older TUI (the known limit); the TUI restarted the service, which resumed the turn |
| Crash, whole tree killed | `alive=Gone` and "Holding ends" 66 ms later |
| Crash leftover with a newer OpenCode | a newer `opencode run --standalone` left the row `Gone` (no hold): the `ImageSince` rule holds; the managed service started later resumed and finished it |
| Schedule in 3 min | timer posted 0.08 s after the row, hold at once (1 hour rule), fired on time while the service ran, the hold ran on into its turn; a schedule in 2 h gave a timer and no hold |
| Never blocked | OpenCode's log: no busy, locked or I/O error (944 lines), with up to three readers |
| Read only | handle check at Medium (own `handles.ps1`: process handle table, `DuplicateHandle`, `GetFinalPathNameByHandleW`): db and `-wal` read rights only, nothing held between reads; db and `-wal` size and time unchanged by reads; Process Monitor not used (not installed) |
| No row value in a log | none in the tray log or the harness `log` lines; session ids only in the harness console's `sources` lines, by design |
| Through the app | dry-run tray at Medium with only OpenCode able to hold: "OpenCode is watched in the folder XDG_DATA_HOME names", hold lines as above |

**Fixes**, each with tests that failed with it switched off:

1. **No change notice for a commit** (the big one): OpenCode keeps its `-wal` at one size and its listed write time
   stale while open, so Wakeshot saw turns only at the 30 s tick and missed short ones. The watch thread now looks
   every 2 s (`QUERY_POLL_MS`) at the first 48 bytes of `<db>-shm`, SQLite's WAL-index header that every commit
   changes (`sqlite::wal_index_header`), and reads the db only when they changed since its last read (taken after
   that read, so a rebuild on leftover files is no change). The tick follows the same rule, so files a stopped
   OpenCode left are read once, not every 30 s (the review's leftover concern). Tests: sqlite
   `the_wal_index_header_changes_with_each_commit_and_never_with_a_read`, the cross-process kill test (same header
   after two rebuilds); agents `a_query_is_polled_while_its_database_is_there`,
   `a_query_reads_again_only_when_the_wal_index_header_changed`,
   `a_read_that_rewrites_the_header_is_no_change_for_the_next_look`; thread `a_commit_with_no_notice_is_read_by_the_poll`.
2. **A quiet poll publishes nothing** (`watch/thread.rs`, `Agents::changes`): a snapshot of an `ImageSince` session
   lists every process (about 14 ms), so a poll that read nothing must not post. Test
   `a_poll_that_finds_no_commit_lists_no_process`. Cost: 172 ms of CPU in 120 s (release, no OpenCode).
3. **`SQLITE_IOERR` during `opencode service stop`** showed "format problem" and the "update Wakeshot" entry for 2 s at
   2 of 2 stops. `sqlite.rs` turns on extended result codes and answers `Busy` for an I/O error other than the shim's
   own refused delete. Test `an_io_error_of_the_agents_close_is_busy_but_the_shims_refused_delete_is_not`; 4 later
   stops clean.

Contract: `WantedFile::Query` keeps its three fields; its doc now says the poll (no `poll_while` field in the end,
reason below). The `Liveness::AnyProcess` doc in `sources.rs` now says Copilot's pid-in-the-name rule
(`p4-live-copilot`). Tray and harness wiring (`p4-contracts-r3`) worked as built: no wiring bug.

**Fixtures**: `crates/wakeshot-core/tests/fixtures/opencode/live/` (8 answers and a README; fake ids, real times),
tests `every_live_fixture_is_listed_once`, `the_live_answers_give_the_facts_the_live_test_saw`. No parser fix was
needed: every live answer read with no format problem.

**Files changed**: `crates/wakeshot-core/src/sources.rs` (docs of `Query` and `AnyProcess` only),
`crates/wakeshot-core/src/sources/opencode.rs` (docs), `crates/wakeshot-core/tests/opencode_source.rs`,
`crates/wakeshot-core/tests/fixtures/opencode/live/` (new), `crates/wakeshot-windows/src/sqlite.rs`,
`crates/wakeshot-windows/src/watch/agents.rs`, `crates/wakeshot-windows/src/watch/thread.rs`,
`docs/architecture.md` (the `Query` lines in sections 6 and 7), `docs/research/opencode-signals.md` (new section),
`docs/research.md` (new last section "OpenCode live test"), this card. Not touched: `tray_mode.rs`, `agent-watch.rs`,
any `Cargo` file.

**Checks** (own target folder, `--locked`, no `RUSTFLAGS`, final code, 17:52 to 17:55): `rustfmt --edition 2024
--check` on my six Rust files: clean. `cargo clippy --locked -p wakeshot-core -p wakeshot-windows` with and without
`--all-targets`, `-D warnings`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (Build Tools `Llvm\x64\bin` on
that command's PATH only): all four clean. `cargo test --locked -p wakeshot-core -p wakeshot-windows --no-fail-fast`:
all pass (core unit 730; `opencode_source` 17, `copilot_source` 48, `vscode_source` 33, `codex_source` 22,
`claude_transcript` 25, `antigravity_source` 15, `claude_status` 11, `cursor_source` 8; `wakeshot-windows` 716 with
28 ignored). New core test paths use one `Path::join` per level. Build folder deleted.

**Not tested live**: the desktop app (`opencode-cli.exe`), a 1.x file, `job` rows, `OPENCODE_DB` or channel files,
subagent sessions, an elevated OpenCode beside a Medium Wakeshot, `retry_at`, the `/schedule` command, a long-used
database, ARM64 (clippy only).

**Decisions for Haik's review** (mine, each with when to change it):

1. **The poll reads 48 bytes of the `-shm`, not the process list.** The orchestrator's sketch (a field naming the
   images, read every 2 s while one lives) was built first and measured: 859 ms of CPU in 120 s with no OpenCode
   running (a process list is about 14 ms here, 413 processes and 8,539 threads), and OpenCode's background service
   can run for days. The header costs a 48-byte read and needs no new contract field. Change it when an OpenCode
   build opens its database without a `-shm` (exclusive locking): then the process gate.
2. **An I/O error is busy, not a format problem**, so a lasting I/O error gives no notice (the last answer stands
   while OpenCode lives). Change it when a user report shows one: then retry once, then report.
3. **A turn shorter than 2 s can be missed** (two short one-shots were). Nothing needs holding then. Change it when a
   short turn must hold (for example a sleep timeout under a few seconds).
4. **A permission wait holds as long as OpenCode waits** (not on disk, the house rule); a resumed turn after a crash
   did not wait here (it ended in 3 to 4 s), but one that asks again with no TUI open would hold while the service
   lives. Not tested; change it when that shows up.
