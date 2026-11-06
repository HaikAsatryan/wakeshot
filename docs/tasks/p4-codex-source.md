# Codex CLI source

Phase: 4. Status: done (back in v1, Haik 2026-11-06; see "Round 2" at the end). Needs Haik: no (his live test is `p4-live-codex`). Agent: `wakeshot-worker`. Size: about 70K
tokens.
Runs after: the contracts card (it creates `sources/codex.rs` and adds `WantedFile::Names`, `FindTail`,
`Feed::Present`). Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-codex-source\target`. Uses the real laptop: no.

## Goal

`CodexSource`, an `AgentSource` built from the Codex source code reading in `research/other-agent-signals.md`:
sessions from thread lock files, activity from the last turn event of the rollout. Not tested live until
`p4-live-codex`; the docs mark it so.

## Read first

- `docs/phases/04-sources.md`: track notes 7, 8, and Question 1 (answered with the default, Haik 2026-11-03 18:09:
  an open turn holds).
- `docs/architecture.md` section 7 ("Codex CLI and Copilot CLI").
- `docs/research/other-agent-signals.md`: "OpenAI Codex CLI" (all of it), "Design notes for Wakeshot", "Fixture
  sketches".
- `crates/wakeshot-core/src/sources.rs` (after the contracts card).

## Deliverables

1. `crates/wakeshot-core/src/sources/codex.rs`: `CodexSource::new(home: PathBuf)` where `home` is `CODEX_HOME` if set,
   else `%USERPROFILE%\.codex` (the caller resolves it). `wanted()`: `Names { dir: <home>\thread-writer-locks, depth:
   0, accept: <thread id>.lock }` (lock files are never opened: Codex holds an exclusive lock and treats a second
   holder as an error); for each present lock, `FindTail { root: <home>\sessions, depth: 3, name_suffix:
   "-<thread id>.jsonl" }` (`YYYY\MM\DD`). Skip `.jsonl.zst`. A forked rollout (`..._<rollout id>.jsonl`) is matched
   too if the live test shows it carries the same thread; until then not matched (say so in the Result).
2. Activity from the last turn event in the rollout: `task_started` (or `turn_started`) gives `Working`;
   `task_complete`, `turn_complete` or `turn_aborted` gives `Idle`; no turn event yet, or no rollout found yet
   (created at the first item), gives `Idle`. Approval waits are not on disk, so they read as `Working` (Question 1).
   Lines of other kinds are ignored; a line that is not JSON is skipped; an `event_msg` turn event without `type` or
   `turn_id` gives `UnknownValue("rollout", <kind>)`.
3. `SessionFacts`: `key.id` the thread id, `liveness: AnyProcess { image: "codex.exe", lock_file }` (the watch thread
   ties it to the holder, track note 7), `label: "Codex"`, `timers: []`, `open_tasks: 0`. Lock file gone: the
   session is gone.
4. Fixtures `crates/wakeshot-core/tests/fixtures/codex/` from the research sketches (running, complete, aborted, the
   `turn_*` aliases, a rollout before any turn, junk lines), each marked "built from the schema, not a real log" in a
   README. Tests first: `crates/wakeshot-core/tests/codex_source.rs` feeds lock names and rollout bytes (whole and in
   pieces) and checks the facts.
5. The module doc says "not tested live" until `p4-live-codex` changes it. `docs/research/other-agent-signals.md`
   belongs to the live cards; do not edit it here.
6. Moved here from `p4-watch-reads` when Codex and Copilot went after v1 (Haik, 2026-11-03): deliverable 2 of that
   card, `crates/wakeshot-windows/src/lockowner.rs` (Restart Manager `holders(path)`), with its tests and the
   research line on the Restart Manager answers. Size about 25K more.
7. Moved here from `p4-watch-agents` (done by then): the `AnyProcess { image, lock_file }` liveness in
   `crates/wakeshot-windows/src/watch/agents.rs` through `lockowner::holders`, as that card's deliverable 3 first
   said (cache until the lock file changes or the holder is gone; no holder gives `Gone`; a Restart Manager error
   falls back to "any live process with that image" while the lock file is `Present`; a failed process list gives
   `Unknown`), and its tests. Size about 20K more.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- No code path opens or reads a lock file (the source never asks for one with `Folder` or `Tail`).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Restart Manager and process checks (`p4-watch-reads`, `p4-watch-agents`). Goals (track note 8). The app-server
  daemon beyond what the lock files show. The live test (`p4-live-codex`). Any file not listed above.

## Questions for Haik

None. The open items in the Result are technical and go to the orchestrator.

## Found outside scope

Problems seen but not fixed.

1. **Contract gap: a reverted thread's rollout cannot be named.** After a TUI backtrack (editing an earlier prompt)
   of a paginated thread, the default for new TUI threads in Codex 0.167.0, the thread goes on in
   `rollout-<time>-<thread id>_<rollout id>.jsonl` (Log, change 4). `FindTail`'s `name_suffix` cannot match it, and
   the watch thread keeps the first file it found while that file is still there, so the source reads the old file:
   that session shows idle while it works, and nothing holds the PC for it. Proposal for a contract card run alone:
   `FindTail` also matches `-<thread id>_<uuid>.jsonl` (for example a `name_infix` next to the suffix), and the watch
   thread moves to the newest match by modified time (fed as `Gone`, then the new file from its start). Until then
   the module doc and the tests say it is not followed.
2. **Stale lines on "Codex and Copilot after v1".** `docs/architecture.md` section 7 ("after v1 ... their cards are
   `later`"; Restart Manager "not verified for this lock": now tested with a lock held the way Codex holds it, not
   with a real Codex), `docs/phases/04-sources.md` (goal, "Notes 7, 8 and 9 apply after v1", order step 5, the
   bundles paragraph) and `docs/plan.md` (Phase 4 row). Not my files.
3. **`Liveness::AnyProcess` contract doc** (`crates/wakeshot-core/src/sources.rs`): it says "the process that holds
   `lock_file` open"; the check now also asks that the holder runs `image` (Result, choice 4). Worth one word at the
   next contract change.
4. **For `p4-live-codex` to carry into `docs/research/other-agent-signals.md`** (Round 2 keeps them out of it here):
   the `_<rollout id>` name is a revert, not a fork; date folders are local time; `turn_aborted.turn_id` is optional;
   `.coordination.lock` sits in the lock folder; new TUI threads are paginated. Lines in the Log.
5. **A turn cut by a crash, then resumed.** From the source, Codex appends an abort to a fork cut mid-turn, but no
   such line was found on the resume path (not run). If a process dies mid-turn and the thread is resumed, the
   rollout's last turn event is still `task_started`, so the resumed, idle session reads working until its next
   turn ends or it closes: a hold too long, never too short. The parser cannot tell, since it does not know when the
   holder started (Claude's source has `procStart` for this). Whether closing the terminal mid-turn writes an abort
   first: not verified (`p4-live-codex`).

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-06 12:13: worker started (round 2). First step: re-read the latest `openai/codex` release source as data.
- 2026-11-06 about 12:25: read Codex 0.167.0 (latest release, tag `rust-v0.167.0`, published 2026-11-06 01:40 UTC, commit
  `086ccc8`), the GitHub tarball unpacked in the scratchpad, read only. Paths under `codex-rs/`. Same as the research:
  home (`utils/home-dir/src/lib.rs:13-62`: `CODEX_HOME` when set and not empty, else `~\.codex`); lock files
  `<home>\thread-writer-locks\<thread id>.lock`, opened read and write, `try_lock`, closed and deleted when done,
  stale ones cleaned at a process's first take (`rollout/src/writer_lock.rs:19,45-88,137-205`); line format
  (`history/src/lib.rs:375-381`, `history/src/rollout_payload.rs:31-75`), one write and flush per line
  (`rollout/src/recorder.rs:2137-2162`); turn events `task_started` (alias `turn_started`), `task_complete` (alias
  `turn_complete`), `turn_aborted`, kept in every history mode (`rollout/src/policy.rs:126-140`,
  `protocol/src/protocol.rs:1567-1578,1699`); approval prompts not kept (`policy.rs:197-203`); the file made at the
  first persisted item (`recorder.rs:1002-1003,1902`); image `codex.exe` (`codex-cli/bin/codex.js:93`).
  Changes or new facts: (1) a `.coordination.lock` sits in the same folder, taken around every lock take and release
  (`writer_lock.rs:20,110-135`): the name filter must refuse it (its stem is not a UUID). Thread ids are UUIDs written
  lowercase with hyphens (`protocol/src/thread_id.rs:16,76`). (2) `turn_aborted.turn_id` is optional
  (`protocol.rs:4526-4531`, test at 6450), so a `turn_aborted` without it is valid. (3) The date folders and the name
  time are local time (`recorder.rs:1770-1791`, `now_local`). (4) `..._<rollout id>.jsonl` is not a fork: it is a
  reverted thread, same thread id, new file (`rollout/src/rollout_file_name.rs:10-14,39-74`,
  `protocol/src/thread_id.rs:20-25`); the TUI makes one on every backtrack (edit an earlier prompt) of a paginated
  thread (`tui/src/app/event_dispatch.rs:803`, `thread/revert`), new TUI threads are paginated
  (`tui/src/app_server_session.rs:2392`), and Codex then finds the thread's file through SQLite, not by name
  (`thread-store/src/local/thread_rollout_resolver.rs:66-118`). (5) Compression to `.jsonl.zst` only touches files
  7 days old with no live writer, and a resumed one is made plain again before the first append
  (`rollout/src/compression.rs:114,371,925-929,988-1010`): a live thread's file is always `.jsonl`. (6) A new turn
  aborts the running one first (`core/src/tasks/mod.rs:279`), and a fork cut mid-turn gets a `turn_aborted` appended
  (`core/src/thread_manager.rs:2950-3000`), so "the last turn event wins" holds. (7) Subagent and guardian review
  threads are threads of their own, each with its lock file and rollout (`thread-store/src/local/live_writer.rs:26-72`).
- 2026-11-06 about 12:32: `sources/codex.rs`, 12 fixtures with a README, `tests/codex_source.rs` (16 tests) written;
  `cargo test --locked -p wakeshot-core --test codex_source`: 16 passed. Next: `lockowner.rs`, then the `AnyProcess`
  check in `watch/agents.rs`.
- 2026-11-06 about 12:39: `lockowner.rs` (`LockOwners` trait, `RestartManager`, `holders`) with 4 tests and a child role;
  `watch/agents.rs` `AnyProcess` check (`Agents::with_owners`, holder cache, image filter, fallback) with 5 tests in
  place of the old "gives Unknown" test. `cargo test --locked -p wakeshot-windows --lib -- lockowner:: watch::agents::`:
  28 passed, 1 ignored (the child role). Next: format and clippy checks, docs.
- 2026-11-06 about 12:42: `rustfmt --edition 2024 --check` on the four files: clean (formatting `agents.rs` also wrapped the
  `Problem::Unlisted` arm of the same `match`: rustfmt had skipped that whole `match` while the old `NoLockCheck`
  text was too long to fit). Clippy: one finding in `lockowner.rs` (`then` to `then_some`), fixed. Clippy of both
  packages also fails in `sources/copilot.rs` (the running `p4-copilot-source` card's file: unused `limit`, `zone`,
  `one_shot_due`, `ephemeral`); not mine, waiting and running again.
- 2026-11-06 12:49: an end-to-end test (`codex_from_a_held_lock_file_and_its_rollout`) added; all 8 clippy runs
  clean (`copilot.rs` settled); `cargo test --locked -p wakeshot-core` all passed; `-p wakeshot-windows --lib --
  watch:: lockowner::` 101 passed.
- 2026-11-06 12:54: deleted this card's scratchpad folder (build folder, 3.2 GB, and the downloaded Codex source).
  Card done.

## Result

Status: done. Not tested live: every Codex fact here comes from reading its source; `p4-live-codex` tests them.

**Codex version read:** 0.167.0 (tag `rust-v0.167.0`, commit `086ccc8`, released 2026-11-06), as data, never built
or run. The research's facts still hold; the changes and new facts are in the Log (about 12:25): a
`.coordination.lock` in the lock folder, `turn_aborted.turn_id` optional, local-time date folders, the
`_<rollout id>` name is a revert (made on every TUI backtrack of a paginated thread, the TUI default) and not a fork,
compression never touches a live rollout, a new turn aborts the old one first.

**What was built:**

1. `CodexSource::new(home)` and `codex_home(profile, CODEX_HOME)` (empty counts as unset, as in Codex). Wants
   `Names { <home>\thread-writer-locks, 0, is_lock_name }` (`<uuid>.lock` only, so `.coordination.lock` is refused)
   and per listed thread `FindTail { <home>\sessions, 3, "-<thread id>.jsonl" }`. One session per lock file, keyed by
   the lowercase thread id; lock gone, session gone. Facts: `AnyProcess { "codex.exe", lock_file }`, label `Codex`,
   no timers, no open tasks.
2. Activity from the last turn event: `task_started`/`turn_started` working; `task_complete`/`turn_complete`/
   `turn_aborted` idle; no rollout or no turn event yet idle. Only `event_msg` lines are parsed (a byte search first);
   other kinds and lines that are not JSON are skipped; bytes may come split anywhere (a partial line waits; over
   32 MiB it is skipped, as in Claude's transcript reader); CRLF reads the same. A rollout fed `Gone` forgets its
   turns; a second rollout of the same thread waits until the first is gone.
3. `lockowner.rs`: `LockOwners` trait, `RestartManager`, `holders(path)` (`RmStartSession`, `RmRegisterResources`,
   `RmGetList` with up to 4 tries on `ERROR_MORE_DATA`, `RmEndSession` on every path).
4. `watch/agents.rs`: `AnyProcess` liveness through `LockOwners`; `Agents::new` keeps its signature and uses
   `RestartManager`; `Agents::with_owners` for tests. Holders go to `Snapshots.live`, so the watch thread waits on
   their exit like any session process.
5. 12 fixtures with a README ("built from the schema, not a real log"); the Restart Manager facts in
   `docs/research.md` ("Restart Manager: who holds a lock file"), all verified by the tests below except where marked.

**Choices** (mine, as the designer), each with when to change it:

1. A turn event that cannot be read (`event_msg` without a payload `type`; `task_*`/`turn_*` start or complete
   without a text `turn_id`; `turn_aborted` with a `turn_id` that is neither text nor null) gives
   `UnknownValue("rollout", <kind>)`, and the next readable turn event clears it, so one odd old line does not hold
   forever. The kind is one of our fixed names, never file text. Change it when the live test shows a new turn event.
2. `turn_aborted` without `turn_id` is readable (Codex 0.167.0 itself accepts it). This differs from the card text,
   which named every turn event.
3. Reverted rollouts are not matched (Found outside scope 1). Change it when the `FindTail` contract change lands.
4. A holder counts only if it runs the session's image (`codex.exe`, case ignored), matched with the process list by
   pid and start. Why: a virus scanner or indexer may hold the file for a moment; cached as the holder, it would keep
   a crashed session alive for as long as it runs. Change it when the live test shows Codex holding the lock from an
   exe of another name.
5. The holders found are kept until a change notice names the lock file (or loses its names), a listing feeds it,
   or every holder is gone; a "nobody holds it" answer is asked again at each tick. Why: snapshots run after every
   Claude status write too, and one Restart Manager session per lock file is enough. Change it when a live test shows
   a lock file passed between processes with no notice.
6. Restart Manager fails: any live `codex.exe` counts while the lock file is listed, logged at most once a minute
   with the error code; nothing goes to `live`. A failed process list or holder check gives `Unknown`.
7. Rollouts are read on the 30 s safety tick and on lock folder notices, not watched (as the contract says for
   tails), so a turn start or end is seen up to 30 s late. Change it when that delay matters next to sleep timeouts.

**Files changed:** `crates/wakeshot-core/src/sources/codex.rs`; `crates/wakeshot-core/tests/codex_source.rs` (new);
`crates/wakeshot-core/tests/fixtures/codex/` (new: 12 `.jsonl`, `README.md`); `crates/wakeshot-windows/src/lockowner.rs`;
`crates/wakeshot-windows/src/watch/agents.rs` (the `AnyProcess` part, its tests, and rustfmt's wrap of the
`Problem::Unlisted` arm, Log about 12:42); `docs/research.md` (one new section at the end); this card.

**New tests:** `tests/codex_source.rs`: `every_fixture_is_listed_once`,
`wants_the_lock_folder_by_name_and_one_rollout_per_listed_thread`, `no_want_opens_a_lock_file`,
`a_listed_lock_file_is_an_idle_session_until_its_rollout_says_more`,
`a_lock_file_gone_ends_the_session_and_what_its_rollout_said`, `each_fixture_gives_the_activity_of_its_last_turn_event`,
`bytes_split_anywhere_give_the_same_answer`, `crlf_lines_read_the_same`,
`line_by_line_each_turn_event_moves_the_activity`, `a_line_without_its_newline_waits_for_the_rest`,
`a_readable_turn_event_clears_an_unreadable_one`, `a_gone_rollout_forgets_its_turns_and_a_restart_reads_from_the_start`,
`a_second_rollout_of_the_same_thread_waits_until_the_first_is_gone`, `files_it_did_not_ask_for_change_nothing`,
`an_over_long_line_is_skipped_and_the_next_one_counts`, `the_home_folder_is_codex_home_when_set_else_dot_codex`.
`lockowner::tests`: `a_file_this_process_holds_names_this_process_and_stays_locked`,
`a_file_nobody_holds_or_that_is_missing_has_no_holders`, `another_process_holding_the_file_is_named_with_its_start`
(also under a token with no privileges), `many_calls_leave_no_session_behind`, and the child role `hold_role`
(ignored). `watch::agents::tests`: `a_lock_file_session_lives_while_a_holder_running_its_image_lives`,
`a_holder_of_another_image_or_start_does_not_count`, `a_change_notice_or_a_listing_of_the_lock_file_asks_again`,
`a_restart_manager_error_counts_any_live_process_running_the_image`, `a_failed_process_list_or_holder_check_is_unknown`,
`codex_from_a_held_lock_file_and_its_rollout` (the real source, listing, finder, tail and Restart Manager on a temp
Codex home, with a lock held the Codex way and still locked afterwards). Removed: `any_process_gives_unknown_without_a_check`.

**Checks** (2026-11-06, own target folder):

- `rustfmt --edition 2024 --check` on `codex.rs`, `codex_source.rs`, `lockowner.rs`, `watch/agents.rs`: clean.
- `cargo clippy --locked -p wakeshot-core` and `-p wakeshot-windows`, each with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, `-D warnings`: all 8 clean at 12:49. Earlier runs failed only
  in `sources/copilot.rs` while `p4-copilot-source` was writing it.
- `cargo test --locked -p wakeshot-core`: 706 unit, 11 `claude_status`, 25 `claude_transcript`, 16 `codex_source`,
  42 `copilot_source`: all passed.
- `cargo test --locked -p wakeshot-windows --lib -- watch:: lockowner::`: 101 passed, 2 ignored (child roles).
- No lock file is read: the source wants only `Names` on the lock folder and `FindTail` under `sessions` (test
  `no_want_opens_a_lock_file`); the end-to-end test shows the lock still refuses a second taker after the watch
  code and Restart Manager ran.

**Open items:**

1. The `FindTail` contract change for reverted threads (Found outside scope 1), before or with `p4-live-codex`.
2. `p4-live-codex`: wire the source in the tray (`app/tray_mode.rs`, with `codex_home(profile, CODEX_HOME)`), check
   that Restart Manager sees a real `codex.exe` holder from an unelevated tray, the 30 s delay, a revert, and a
   crash then resume (Found outside scope 5).
3. The stale doc lines (Found outside scope 2 to 4).

## Round 2: back in v1 (2026-11-06)

Written by the orchestrator. Haik, 2026-11-06: Codex CLI and Copilot CLI ship in v1 after all. Changes to this card:

- The agent may have changed since the research of 2026-11-03. First re-read the current release's own source
  (`openai/codex` on GitHub, the latest release tag, read as data in the scratchpad, never built or run) for every fact this card relies on, and note the version and any change in this card's Log and Result
  (not in `docs/research.md` or `other-agent-signals.md`: the live cards carry them there).
- Haik's answers to sign-off items 1 and 2 (`docs/phases/02-06-signoff.md`) apply: Codex waiting for approval reads
  as working and holds with no limit; Copilot waiting for approval does not hold.
- The tray still starts the watch thread with the Claude source only (`crates/wakeshot-windows/src/app/tray_mode.rs`).
  Adding this source there is the live card's job, not this one's.
