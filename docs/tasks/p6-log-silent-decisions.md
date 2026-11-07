# Log lines for the silent decisions the logs audit found

Phase: 6. Status: done (new 2026-11-07; items 1, 5 and part of 3 need a contract card, see Result). Needs Haik: no
(Haik, 2026-11-07: logs good enough that a user's report lets us find any problem; no questions overnight).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p6-logs-diagnostics` (done, 880268e). Parallel with: `p6-winget-upgrade-restart-fix`,
`p8-polish-core-engine`, `p8-polish-windows-power` (they own other files).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-log-silent-decisions\target`; delete it when done.

## Goal

`docs/tasks/p6-logs-diagnostics.md` "Found outside scope" items 1 to 6 and 8 list decisions the watch and source code
make without a log line. Add one clear line for each, so a diagnostics file shows why an agent did not hold. Also take
the token-shaped literals out of the diagnostics privacy test.

## Read first

- `docs/tasks/p6-logs-diagnostics.md`: Result and "Found outside scope" (the file and line of each item).
- `docs/architecture.md` section 8 (log rules, the four writers), `docs/product.md` "No guessing" and the Claude Code
  rules on format problems.

## Deliverables

1. A log line for items 1 to 6 and 8, at the place the decision is made, with the reason and a short id (never a
   prompt, a title, a token or a full path with the user name). Once per session, file or state change, never once per
   line or tick, so a quiet tray stays quiet and the log caps are never the thing that saves it.
2. Log lines only: no change to what holds, when, or what the menu and notices show. If an item looks like a real
   product gap (for example item 5: a renamed Codex event means "never holds"), write it under "Questions for Haik"
   with your default; do not change behavior.
3. Item 7 (power safety paths) is out of scope tonight: it waits for the power safety review.
4. `crates/wakeshot-windows/src/diagnostics.rs` tests: build the planted token strings at run time (for example by
   joining parts), so no token-shaped literal stays in the source. The test must still prove the same thing.
5. Tests: each new line is checked by a test that fails without it. Whole-workspace checks on x64 and ARM64
   (`CLAUDE.md`, Checks) before you finish.

## Files

Owns: `crates/wakeshot-windows/src/watch/` (all), `crates/wakeshot-windows/src/sqlite.rs`,
`crates/wakeshot-windows/src/app/io_thread.rs`, `crates/wakeshot-windows/src/diagnostics.rs`,
`crates/wakeshot-core/src/sources/` (all), additions to `docs/architecture.md` section 8 if a line kind is new, this
card. Other cards own `app/` (the rest), `hooks.rs`, `store.rs`, `power/`, `ownership/` and core's engine files right
now: a build failure in a file you do not own is reported, never fixed; wait a minute and build again.

## Rules

- Never run git, not even read-only. Never read token files. Prompt text is never logged. Never set `RUSTFLAGS`. Kill
  only processes you started, by PID. Never touch Haik's installed tray.
- Follow the Phase 8 polish rules in what you write (`docs/phases/08-launch-cleanup.md`, "Polish rules"): short "why"
  comments only, no card codes or names in code.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

Nothing blocks on these; each default is what stands until Haik says otherwise.

1. A Codex rollout whose turn event got a new name (item 5), or an OpenCode database without `session_v2` (item 3),
   means the agent never holds. Should Wakeshot show "update Wakeshot" then? Default: no notice, a log line only (once
   the contract allows it, see Open items). Wakeshot cannot tell a renamed event from a turn that has not started,
   and an OpenCode 1.x database is no format change that an update would mend; a wrong notice is worse than none.

## Found outside scope

1. `short_id` now lives twice: private in `crates/wakeshot-core/src/core/holds.rs` and copied in `watch/agents.rs`, so
   the watch's lines name a session as the core's do (a test pins the same FNV-1a values). One `pub` copy in core
   would end the twin; core's engine files belong to another card tonight.
2. The watch names agents "Codex CLI" and "Copilot CLI" (`source_name`), the core "Codex" and "Copilot" (their
   labels), so one agent has two names in one log. Changing either is a log text change, out of scope here.
3. `sqlite::load` does not tell why Windows' SQLite is not usable (no DLL, older than 3.22.0, a missing export, the
   shim not registered); the new line names all three. Change it when a user report needs which.

## Log

- 2026-11-07 04:42: card written by the orchestrator (session wakeshot-4e).
- 2026-11-07 04:58: worker started; reading the logs card, architecture section 8, product rules.
- 2026-11-07 05:07: read every item's code. Items 2, 3 (`no_sqlite`, missing `-wal`), 4, 6 and 8 sit in the Windows
  crate, which has the log. Items 1, 3 (no `session_v2`) and 5 are decisions inside core sources, which have no log
  and no way to hand a line out: `AgentSource` (`sources.rs`, a contract, not this card's file) has only `id`,
  `wanted`, `feed` and `sessions`, and the one fact a source can set, `FormatHealth`, shows "update Wakeshot" when not
  `Ok` (`policy::sessions::decide`), so it cannot carry a line without changing a notice. Plan: do the Windows items
  and the token literals now; the three core items need a contract change in a card run alone (design under Result).
- 2026-11-07 05:20: Windows items built (watch/agents.rs, read.rs, thread.rs, sqlite.rs, app/io_thread.rs) and the
  diagnostics tests' tokens made at run time. `cargo clippy -p wakeshot-windows --all-targets` clean; the targeted
  tests pass (187). Mutation check: with the new lines turned off, all 8 new or changed tests fail; files put back
  and their SHA-256 checked equal.
- 2026-11-07 05:31: whole-workspace checks on x64 and ARM64 pass (`cargo fmt --all --check` failed at 05:22 only in
  another card's `tray/menu_paint/tests.rs`, and passed at 05:31). `docs/architecture.md` section 8 passage added,
  re-read after the docs card's rewrite: still there. Build folder and scratch files deleted.

## Result

Status: done for items 2, 3 (`no_sqlite`, missing `-wal`), 4, 6, 8 and the token literals; items 1, 5 and the
`session_v2` part of 3 are not done: they need a contract change in a card run alone (Open items). No change to what
holds, when, or what the menu and notices show: only log lines.

**New log lines** (each once while its state lasts; the watch's lines share its existing per-source line, at most one
a minute per source)

- Item 2, `watch/agents.rs`: `Codex CLI: session 1ad7a9dc shows work but counts as ended: no codex.exe holds its lock
  file, only MsMpEng.exe, codex-next.exe` (the other holders' exe names, at most 4, show a renamed agent exe); for a
  lock name with a pid: `neither copilot.exe nor the process its name gives holds its lock file`; for `ImageSince`:
  `no opencode.exe or opencode-cli.exe that started before it runs` (or `... runs`). Only for a session whose files
  show work (not idle, a timer, or background tasks): an idle session's liveness decides nothing. Not when Restart
  Manager failed: that has its own line.
- Item 3, `watch/agents.rs` and `sqlite.rs`: `OpenCode: Windows' own SQLite (winsqlite3.dll) is missing, older than
  3.22.0 or not usable, so its database is not read`; any other error word as `a query of its database failed
  (prepare)`, next to the existing format problem. `OpenCode: its database is open but has no -wal file, so it is not
  read`, only while another process holds the database (with nobody there a missing `-wal` is a closed database).
  `sqlite::error_kind` maps an answer to one of its own words, `other` for anything else, so no database text can
  reach the log.
- Item 4, `watch/read.rs` and `watch/agents.rs`: a `FindTail` or `FindNewest` walk now keeps the first listing error
  (not a missing folder), and while it finds no file the source's line says `a folder it reads could not be listed
  (error 5)`, as `Folder` and `Recent` wants already did.
- Item 6, `app/io_thread.rs`: `holds\"run-42.json" deleted`, or `holds\"run-42.json" was already gone` (Info).
- Item 8, `watch/thread.rs`: `the holds folder is not watched (ReadDirectoryChangesW error ...), so its changes are
  read on each tick` (or the data folder; Warn), not when `holds\` is just not made yet.
- `docs/architecture.md` section 8, "Logged": one added passage naming these lines.

**Diagnostics test** (`diagnostics.rs`): every token-shaped string in its tests (the six in `tokens_go_and_ids_stay`
and the two planted in the bundle test) is joined from parts at run time by a `token(&[...])` helper, so the source
holds none; the tests assert the same words and outputs as before.

**Calls for Haik's review** (each reversible; why, then when to change it)

1. The watch's new lines join its existing per-source problem line (Warn, once while the state lasts, one line a
   minute per source) instead of a new line kind: one dedup and rate rule for every watch line keeps a quiet tray
   quiet. Change it when a line needs its own level.
2. "Counts as ended" only for sessions whose files show work. Change it when a report needs the liveness of idle
   sessions.
3. Other holders' exe names in that line: a renamed exe is the case the line is for; names only, no path. Change it
   when an exe name is found to carry user data.
4. A folder that cannot be listed counts only while the wanted file is not found, so an unrelated unreadable project
   folder says nothing once the transcript is found. Change it when a newer file hidden in such a folder matters.
5. Query errors log every error word, not only `no_sqlite`, in the same line as the format problem they cause.
6. The hold file line names a file that was already gone too: it tells a script that deleted its own hold file from
   one Wakeshot deleted.
7. The token literals: the card names the bundle test; `tokens_go_and_ids_stay` held six more (a GitHub token shape,
   a JWT, an AWS key id), so they are joined from parts too.

**Files**

- Changed: `crates/wakeshot-windows/src/watch/agents.rs`, `crates/wakeshot-windows/src/watch/read.rs`,
  `crates/wakeshot-windows/src/watch/thread.rs`, `crates/wakeshot-windows/src/sqlite.rs`,
  `crates/wakeshot-windows/src/app/io_thread.rs`, `crates/wakeshot-windows/src/diagnostics.rs`,
  `docs/architecture.md` (section 8, one passage), this card. Created or deleted: none in the repo.

**Checks**

- New tests, each failing with its line turned off (mutation run, then the files put back and checked by SHA-256):
  `a_working_lock_file_session_that_counts_as_ended_says_why_once`,
  `a_working_session_whose_named_lock_nobody_holds_says_so`,
  `image_since_says_nothing_of_an_idle_session_or_no_image_at_all`,
  `image_since_counts_a_process_of_the_image_that_started_early_enough` (now expects the line),
  `a_query_error_and_a_held_database_without_its_wal_are_logged`,
  `a_find_whose_folder_cannot_be_listed_says_so_while_nothing_is_found` (a deny ACE under `NoBackup`),
  `each_hold_delete_is_logged_whether_the_file_was_there_or_not`,
  `a_data_folder_that_cannot_be_watched_is_logged_and_one_not_made_yet_is_not`. Also
  `a_database_nobody_holds_without_its_wal_says_nothing`, `short_ids_are_a_pid_as_it_is_or_8_hex_digits`,
  `a_walk_tells_a_folder_it_could_not_list`, `an_error_answer_gives_only_a_word_of_this_module`, and the real-SQLite
  `nothing_read_from_the_database_reaches_a_log` now expects `a query of its database failed (prepare)`.
- Whole workspace (`CLAUDE.md`, Checks): `cargo fmt --all --check` passes; `cargo clippy --workspace --all-targets
  --locked -- -D warnings` and the same without `--all-targets`, x64 and ARM64: no warning; `cargo test --workspace
  --locked`: 1820 passed, 0 failed, 65 ignored over 22 binaries; `cargo build --release --workspace --locked`, x64
  and ARM64: built.

**Open items**

- Items 1, 5 and the `session_v2` part of 3 need a contract change, run alone. Proposed: `AgentSource::notes(&self)
  -> Vec<String>` in `sources.rs` with a default of none: the source's current silent conditions as short fixed
  texts, naming at most a short id, never file content; the watch logs each new one in the source's line, once while
  it lasts, as it does problems. Sources fill it: Claude and Copilot, a transcript problem kept back while the session
  is idle (`session <id>: a transcript line could not be read (<field>), not counted while it is idle`); OpenCode, a
  tables answer without `session_v2` (`its database has no session_v2 table, so no session is read`); Codex, a
  rollout with event lines but no turn event it knows, and one with an `event_msg` line that is not JSON. About one
  card of 60K to 80K tokens, alone among Rust cards.
