# Contracts round 2: what the new agent sources share

Phase: 4. Status: done. Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 200K tokens. If the card runs past about 220K, stop after a finished change
with its tests, and list the rest in the Result.
Runs after: `p4-codex-source`, `p4-copilot-source`, `p4-opencode-source` Part A, `p4-copilot-vscode-research` (all
done). Parallel with: the docs-only cards (`p4-antigravity-research`, `p4-hooks-design`) and `p4-live-harness` Part A.
It is the only card that builds inside the Rust workspace while it runs: it may change `Cargo.toml`, `Cargo.lock` and
the contracts.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-contracts-r2\target`; delete it when done.
Real laptop: no agent installs, no power settings, no screen.

## Goal

Every shared change the new v1 sources need, in one card run alone, so the later source cards (OpenCode Part B, VS
Code, Cursor, Antigravity) and live cards touch only their own files and build with `--locked`.

## Read first

- `CLAUDE.md` (Checks, cargo path, ARM64 note, elevation note), `docs/architecture.md` sections 2, 6, 7 and 11.
- `crates/wakeshot-core/src/sources.rs` (the contracts), `crates/wakeshot-windows/src/watch/` (`thread.rs`,
  `agents.rs`, `read.rs`, `dir.rs`, `data.rs`), `crates/wakeshot-windows/src/lockowner.rs`.
- `docs/tasks/p4-opencode-source.md`, Result ("Design for Part B": contract changes, the `sqlite.rs` module, the
  watch thread lines) and "Haik's answers".
- `docs/tasks/p4-codex-source.md`, "Found outside scope" 1 (reverted threads) and the Result.
- `docs/research/copilot-vscode-signals.md`, "Recommended detection rules" (rules 3, 6, 7) and "For the source card".
- `docs/research/cursor-gemini-signals.md`, the Cursor sections, only to name the stubs below.

## Deliverables

1. **`SourceId` variants** `OpenCode`, `VsCode`, `Cursor`, `Antigravity`, and a stub module for each under
   `crates/wakeshot-core/src/sources/` (`opencode.rs`, `vscode.rs`, `cursor.rs`, `antigravity.rs`): a module doc with
   the `Owner:` line (`p4-opencode-source`, `p4-vscode-source`, `p4-cursor-source`, `p4-antigravity-source`) and
   nothing that would warn. Fix every `match` on `SourceId` in the workspace. `Antigravity` stays even if its research
   says no live test is possible; Phase 8 removes it then.
2. **`WantedFile::Query`** as the OpenCode design names it (`db`, `name`, `sql`; JSON `{"rows":[...]}` or
   `{"error":"<kind>"}` through `Feed::Whole`; `Gone` when `<db>` or `<db>-wal` is missing; `SQLITE_BUSY` gives no
   feed). The watch thread plumbing: runs a `Query` want only while `<db>-wal` exists, on a change in the db's folder
   (at most one read per second) and on the safety tick. The thread calls one function in a new module
   `crates/wakeshot-windows/src/sqlite.rs` (`Owner: p4-opencode-source`), which this card leaves as a stub that
   returns the `no_sqlite` error; Part B of the OpenCode card fills it with the `winsqlite3.dll` loader. Tests with a
   fake query function: when a read runs, when it does not, the rate limit, `Gone`.
3. **`Liveness::ImageSince`** (`images`, `since_wall_ms`, the 2 s slack, ASCII case-insensitive image names, `None`
   means any start), checked in the watch thread from the system process list with no admin rights, with a
   `QUERY_LIMITED | SYNCHRONIZE` wait on each match so an exit re-checks at once. Tests with the fake liveness and one
   real-process test. Note in the doc that VS Code rule 7 (`Code.exe` started before the request time) fits this
   variant, so the VS Code source may use it without another contract change.
4. **`FindTail` for Codex reverted threads**: also match `-<thread id>_<rollout id>.jsonl` next to the plain
   suffix, and when several files match, follow the newest by modified time (feed `Gone` for the old one, then the
   new file from its start). Update `sources/codex.rs`, `tests/codex_source.rs` and the module doc lines that say
   reverted threads are not followed (no running card owns them now). Tests for both names and the switch.
5. **Whole-file reads of recently changed files** (VS Code rule 3): a read mode that gives `Feed::Whole` for files
   under a folder pattern (`workspaceStorage\*\chatSessions\*.jsonl`, `globalStorage\emptyWindowChatSessions\*.jsonl`)
   only when the file changed since the thread started or within the last 24 hours, read whole again on each change,
   `RetryOnce` on a partial last line, `Gone` on delete. A name filter runs before any open, like `Folder::accept`.
   Tests: old files never opened at start, a changed file read whole, compaction (rewrite in place) read whole.
6. Any `windows` crate feature the new sources need (process list for `ImageSince`, anything `sqlite.rs` will need
   for `LoadLibraryExW` and `GetProcAddress`, if not on yet), so no later card edits a `Cargo` file. List each one.
7. A stub module for installing agent hook folders, `crates/wakeshot-windows/src/agent_hooks.rs`
   (`Owner: p4-cursor-source`, module doc only), so the hooks build adds no module list line. If `p4-hooks-design`
   (running beside you) has already named another place by the time you reach this step, use its name.
8. `docs/architecture.md` sections 6 and 7: the new variants and read modes, in the same short style (re-read the
   file right before each edit; another card may add its own section at the same time; additions and edits to these
   two sections only).

## Acceptance checks

- Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: `cargo fmt
  --all --check`; clippy with and without `--all-targets`, `-D warnings`; `cargo test --workspace --locked`; release
  build. The ARM64 run needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that command's PATH only. Record each
  result. After a `Cargo.lock` change, the first build may run without `--locked`; every check after it uses it.
- Every new behavior has a test that fails when the behavior is broken (say which ones you broke on purpose).
- No source reads anything it did not read before (Claude, Codex, Copilot tests pass unchanged except item 4).

## Out of scope

- The OpenCode, VS Code, Cursor and Antigravity parsers and `sqlite.rs`'s real body (their own cards).
- Registering sources in `crates/wakeshot-windows/src/app/tray_mode.rs` (the live cards do it).
- Hooks install code (design card first, then `p4-cursor-source`).
- `crates/wakeshot-windows/examples/agent-watch.rs` (owned by `p4-live-harness`).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`). Two workers slipped today.
- Never read Claude's `.key` files, OpenCode's `service.json` or any token file. Prompt text is never logged.
- Kill only processes you started, by PID. Start any test program at Medium through `explorer.exe` when the test
  depends on rights (`CLAUDE.md`, elevation note).
- Never set `RUSTFLAGS`. Cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

1. **`docs/architecture.md` section 2 is stale now** (not my section: this card edits 6 and 7 only). The `sources`
   row of the core module table lacks `opencode`, `vscode`, `cursor`, `antigravity`; the `wakeshot-windows` table lacks
   `sqlite` and `agent_hooks`; the "Boundary traits" comment lists `WantedFile` without `FindNewest`, `Recent` and
   `Query`. One line each.
2. **Section 7's Codex paragraph** still says the Restart Manager mapping is "not verified for this lock" and that
   "until then" the rule can outlive its session; `p4-codex-source` built and tested it (its Found outside scope 2).
   Left as it was: not this card's change.
3. **The hooks design asks for more contracts** (`p4-hooks-design` Log, early note): `Command::Hook { agent, event }`
   in the frozen `crates/wakeshot-cli/src/command.rs`, its line in `main.rs`, the parse in `args.rs`, a stub
   `crates/wakeshot-cli/src/hook.rs` (`Owner: p4-cursor-source`), and the parent pid in `ProcessEntry` (`process.rs`).
   Not in this card's deliverables; a new `ProcessEntry` field changes every `ProcessEntry { .. }` literal in the
   tests of both libs. Its item 1 (`agent_hooks.rs`) is done here. Needs a short contracts card run alone.
4. **Two FILETIME helpers in `watch/agents.rs`** repeat `clock::filetime_to_unix_ms`, which is private to `clock.rs`.
   Small; worth one shared helper when `clock.rs` next changes.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started; reading the contracts, the watch thread and the design notes.
- 2026-11-06: read done. Shape picks: `FindTail` keeps its three fields (the Claude tests match all three, so a new
  field would change them); Codex gets a new `WantedFile::FindNewest`. Whole reads of recent files are
  `WantedFile::Recent`. `examples/agent-watch.rs` matches every `Liveness` variant, so the new variant needs one arm
  there (owned by `p4-live-harness`, Part B not started; one added arm only). No `Cargo` change: `LoadLibraryExW`,
  `GetProcAddress`, `FreeLibrary` and the process list are already behind features that are on.
- 2026-11-06: code written: `SourceId` variants and four core stubs; `Liveness::ImageSince`; `WantedFile::FindNewest`,
  `Recent`, `Query`; `FileId::query`; Codex on `FindNewest` with `is_rollout_of`; `sqlite.rs` and `agent_hooks.rs`
  stubs; `watch/read.rs` (`Stamp`, `read_whole_stamped`, `recent_files`, `Finder::newest`); `watch/agents.rs`
  (`AgentIo`, the four new kinds, `next_due`, `run_due`); `watch/thread.rs` (held-back query runs). `cargo check
  --workspace --all-targets` clean on x64. Next: tests.
- 2026-11-06: 17 Windows tests and 3 core tests written, all pass (watch 118, codex 18, core unit 707). Broke 9
  behaviors on purpose in two rounds and saw their tests fail, then put the code back (fmt clean, the same tests
  pass): Recent window off, window without the start, query limit off, no `-wal` gate, oldest match instead of
  newest, no `ImageSince` slack, failed-folder files fed `Gone`, thread not waiting for held-back runs, reverted names
  not parsed. Next: the whole-workspace checks.
- 2026-11-06: x64: fmt, both clippy runs and `cargo test --workspace --locked` (1541 passed, 56 ignored, 0 failed)
  clean. `docs/architecture.md` sections 6 and 7 and a `docs/research.md` section written. ARM64 and the release
  builds running.
- 2026-11-06: every check passed; a doc line and a small `run_query` change came in during the runs, so all checks
  ran again on the final code: all clean. Build folder and scratch scripts deleted. Status done.

## Result

Status: done, 2026-11-06. Nothing here is tested with a real agent.

**What was built** (by deliverable):

1. `SourceId::{OpenCode, VsCode, Cursor, Antigravity}` and four stub modules in `crates/wakeshot-core/src/sources/`
   (`opencode.rs`, `vscode.rs`, `cursor.rs`, `antigravity.rs`), module doc and `Owner:` line only. The one `match` on
   `SourceId` (`watch/agents.rs` `source_name`, the log names): "OpenCode", "VS Code chat", "Cursor", "Antigravity".
2. `WantedFile::Query { db, name: &'static str, sql: &'static str }`, answers under `FileId::query(db, name)` (the
   path `<db>?<name>`; `?` cannot be in a file name). The watch layer runs it through `AgentIo::query`, which is
   `sqlite::query(db, sql) -> sqlite::Answer { Json(Vec<u8>), Busy }`; the stub answers `{"error":"no_sqlite"}`.
   Runs only while `<db>` and `<db>-wal` exist; on the tick; on a notice in the db's folder that names a file whose
   name starts with the db's file name (`-wal`, `-shm`, `-journal`; case ignored) or lost its names; at most once a
   second per want (`QUERY_EVERY_MS`), a held-back run due later through `Agents::next_due` and `Agents::run_due`,
   which the thread waits for. `Gone` once after an answer when either file is missing; `Busy` feeds nothing; the same
   answer is not fed twice. A query a feed asks for runs in the same call.
3. `Liveness::ImageSince { images, since_wall_ms }` and `IMAGE_SINCE_SLACK_MS = 2_000` in the core; checked in
   `Agents::snapshots` from `Liveness::list` (asked once per round), ASCII case ignored; every match goes to
   `Snapshots::live`, so the thread waits on its exit with `SYNCHRONIZE | QUERY_LIMITED` (`ProcessHandle`). No list:
   `Unknown`. The core doc and section 7 say VS Code rule 7 fits it.
4. `WantedFile::FindNewest { root, depth, key, matches: fn(&str, &str) -> bool }`: newest match by the listing's
   modified time (greater name on a tie); a switch feeds the old file `Gone`, then the new one from byte 0. The folders
   are walked on each call, once per root and depth (`Finder::newest`, `forget_walks`). Codex uses it with
   `codex::is_rollout_of` (`-<thread id>.jsonl` or `-<thread id>_<tag>.jsonl`, the tag 1 to 64 letters, digits or
   hyphens, case ignored); `rollout_id` reads both names. The module doc says reverted threads are followed.
5. `WantedFile::Recent { root, depth, sub: Option<&'static str>, accept }`: `root\<depth folders>\<sub>\<name>`;
   `recent_files` lists with sizes and write times and opens nothing; a file not fed yet is read only when written at
   or after `min(start, now - 24 h)`; a known file is read again when its size or write time changed, fed only when
   its bytes' hash changed; a file a notice names is read (its change is new), `sub` matched without case and spelled
   as the want spells it; `Gone` on delete; `RetryOnce` reads once more 200 ms later; a subfolder that cannot be listed
   keeps its files (not `Gone`) and logs the listing problem. Cap `RECENT_CAP` = 64 MiB.
6. No `windows` feature was missing: the process list and the exit-wait rights were in use already, and
   `LoadLibraryExW`, `GetProcAddress`, `LOAD_LIBRARY_SEARCH_SYSTEM32` (`Win32_System_LibraryLoader`) and
   `FreeLibrary` (`Win32_Foundation`) are on (research, "Contracts round 2 checks"). No `Cargo.toml` or `Cargo.lock`
   change.
7. `crates/wakeshot-windows/src/agent_hooks.rs` (`Owner: p4-cursor-source`, module doc only); `p4-hooks-design` keeps
   the name (its Log).
8. `docs/architecture.md`: section 6, a paragraph on how agent files are read; section 7, the new liveness in the
   struct, the read modes, `ImageSince`, and the four new sources.

**Shapes that differ from the card:**

- Item 4 is a new variant, `FindNewest`, not a change to `FindTail`: the Claude tests match all three `FindTail`
  fields, so a new field would change them, and the switch to the newest file must not change how Claude
  transcripts are found. Claude keeps `FindTail` as it was.
- Item 5's mode is `WantedFile::Recent` with `sub` for the fixed folder (`chatSessions`); `emptyWindowChatSessions`
  is `root = globalStorage, depth 0, sub = Some(..)` or `root = ...\emptyWindowChatSessions, depth 0, sub = None`.
- `sqlite::query` returns the JSON bytes itself, so Part B changes only `sqlite.rs`.
- `Agents::with_io(sources, log, AgentIo { owners, query, now, wall })` for tests; `new` and `with_owners` stay.
- One arm added to `crates/wakeshot-windows/examples/agent-watch.rs` (owned by `p4-live-harness`, Part B not started):
  its `match` on `Liveness` would not build otherwise. Nothing else there changed.

**Choices** (mine), each with when to change it:

1. `FindNewest` lists the date folders on every call (a new file comes with no notice). Change it when a live Codex
   home makes a tick slow: then list only the newest date folder between full walks.
2. `Recent` keeps a stamp and a 64-bit hash per file, not the bytes. Change it when a live test shows a stale
   listing time hiding a change (research, "Not verified").
3. 64 MiB read cap for `Recent`. Change it when a live chat session file is larger.
4. A query's file check counts as a run for the limit. Change it when the live OpenCode test shows a start seen late.

**Files changed:** `crates/wakeshot-core/src/sources.rs`; `crates/wakeshot-core/src/sources/{opencode,vscode,cursor,
antigravity}.rs` (new); `crates/wakeshot-core/src/sources/codex.rs`; `crates/wakeshot-core/tests/codex_source.rs`;
`crates/wakeshot-windows/src/lib.rs` (module list); `crates/wakeshot-windows/src/sqlite.rs` and `agent_hooks.rs`
(new); `crates/wakeshot-windows/src/watch/{agents,read,thread}.rs`; `crates/wakeshot-windows/examples/agent-watch.rs`
(one arm); `docs/architecture.md` (sections 6 and 7); `docs/research.md` (new last section); this card.

**New tests:** core: `sources::tests::a_query_id_names_its_db_and_query_and_no_real_file`;
`codex_source.rs`: `rollout_names_match_the_plain_and_the_reverted_form_only`,
`a_reverted_thread_goes_on_in_its_new_rollout` (and three changed: the wants test, `no_want_opens_a_lock_file`, the
strangers list). `watch::agents::tests`: `find_newest_follows_the_newest_match_and_feeds_the_old_one_gone_first`,
`folders_watch_a_recent_root_and_a_query_database_folder`,
`recent_reads_only_recent_files_at_start_and_never_opens_an_old_one`,
`recent_reads_a_changed_file_whole_again_even_rewritten_in_place_and_feeds_gone`,
`recent_reads_a_file_again_once_when_the_source_asks`, `recent_reads_files_written_since_the_start_even_days_later`,
`recent_files_under_a_folder_that_cannot_be_listed_are_not_gone`,
`a_query_runs_only_while_the_wal_exists_and_its_end_is_fed_gone_once`,
`a_change_of_the_database_files_runs_a_query_at_most_once_a_second`,
`a_busy_database_gives_no_feed_and_an_unchanged_answer_is_not_fed_again`, `a_query_a_feed_asks_for_runs_in_the_same_call`,
`image_since_counts_a_process_of_the_image_that_started_early_enough`,
`image_since_asks_for_the_process_list_once_per_call`, `image_since_finds_a_real_process_by_its_image_and_start` (the
real-process test), and a revert step in `codex_from_a_held_lock_file_and_its_rollout`. `watch::thread::tests`:
`a_query_held_back_by_the_limit_runs_without_a_tick`, `an_image_since_session_is_posted_gone_at_once_when_its_process_exits`.
`sqlite::tests::the_stub_answers_no_sqlite_and_opens_nothing`. Broken on purpose (Log): 9 behaviors, each failed its
tests.

**Checks:**

All run on the final code, own target folder, all `--locked`, no `RUSTFLAGS`; no `Cargo.lock` change, so no run
without `--locked`.

- `cargo fmt --all --check`: clean.
- `x86_64-pc-windows-msvc`: clippy with `--all-targets` and without, `-D warnings`: clean; release build: done.
- `aarch64-pc-windows-msvc` (Build Tools `VC\Tools\Llvm\x64\bin` on that command's PATH only): clippy with
  `--all-targets` and without, `-D warnings`: clean; release build: done.
- `cargo test --workspace --locked` (x64 host): 1541 passed, 0 failed, 56 ignored, 16 test binaries. In it: core unit
  707, `claude_status` 11, `claude_transcript` 25, `codex_source` 18, `copilot_source` 42; `wakeshot-windows` watch,
  sqlite and lockowner tests 118 (2 ignored child roles). Claude and Copilot tests unchanged and passing.
- No source reads more than before: Claude and Copilot wants are as they were; Codex's only change is the reverted
  file of a thread it already followed.

**Open items:** Part B of `p4-opencode-source` fills `sqlite::query`; the live cards check `FindNewest` with a real
Codex revert and `Recent` with real VS Code writes (listing times while the writer holds the file); `ImageSince` at
Medium was not run here (research "Process module checks": the list worked under a token whose opens were denied, a
real Medium caller is not checked); Found outside scope 1 and 3.
