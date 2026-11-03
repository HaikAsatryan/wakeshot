# Watch thread: drive the agent sources and check their liveness

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens (was 100K; the
`AnyProcess` part moved to `p4-codex-source`, after v1, Haik 2026-11-03).
Runs after: `p4-watch-reads`. Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6. Uses
`FakeSource`, so it does not wait for the parser cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-agents\target`. Uses the real laptop: no (temp folders, fakes).

## Goal

For any set of `AgentSource`s: ask what they want, read it, feed it, and turn their sessions into one `Event::Sources`
per source with each session's liveness, as section 7 says ("checks liveness: the adapter, never the parser").

## Read first

- `docs/phases/04-sources.md`: track notes 6, 14, 16.
- `docs/architecture.md` section 5 "Liveness", section 7, section 8 (format problems logged at most once a minute per
  source; Liveness Unknown).
- `crates/wakeshot-core/src/sources.rs`, `testing.rs` (`FakeSource`), `event.rs`.
- `crates/wakeshot-windows/src/watch/read.rs` (from `p4-watch-reads`).
- From the contracts card: `process::Liveness` and `FakeLiveness`, the log seam and its fake.

## Deliverables

`crates/wakeshot-windows/src/watch/agents.rs`, driven by calls (no thread of its own):

1. `Agents::new(sources: Vec<Box<dyn AgentSource + Send>>)`; `folders()` lists the folders to watch (from `Folder`
   and `Names` wants, with the subtree flag for depth 1).
2. `on_change(folder, names)` and `on_tick()`: read and feed what changed: `Folder` files whole (a `Fed::RetryOnce`
   answer schedules one re-read 200 ms later: return it to the caller); `Names` as `Present` or `Gone`; every `Tail`
   and `FindTail` of the same source on a change in any of its folders and on each tick (section 7: transcripts are
   read on status changes and on the tick, not watched). `wanted()` is asked again after each feed.
3. Liveness for each session: `Process { pid, start }` through `process::Liveness`. Return the live `ProcessRef`s so
   the thread can wait on their exit. `AnyProcess { image, lock_file }` serves only Codex and Copilot, so it moved to
   `p4-codex-source` (after v1, Haik 2026-11-03): here it gives `Unknown`, with one log line per source per minute
   like a format problem, and a short "why" comment that names that card.
4. `snapshots() -> Vec<Event>`: one `Event::Sources` per source whose snapshot changed since the last one returned
   (always all of them on the first call), sessions paired with their `Alive`.
5. Format problems: a session with `Unreadable` or `UnknownValue` gives at most one log line per source per minute
   (field name and fixed text only, never file content). The "update Wakeshot" notice is the core's.

Tests first, with `FakeSource`, `FakeLiveness` and temp folders: a feed per want kind; a retry answered once; a
`Names` file appearing and going; a tail fed only new lines; `Process` liveness `Yes`, `Gone`, `Unknown`;
`AnyProcess` gives `Unknown`; no event when nothing changed; the log limit with a fake clock.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- No path in the module opens a file that a source did not ask for with `Folder`, `Tail` or `FindTail`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The parsers (core cards). The thread, folder watch and exit waits (`p4-watch-thread`). The data folder
  (`p4-watch-data`). Any file not listed above.

## Questions for Haik

None.

## Found outside scope

None. (A tail restart feeds `Gone`, which `ClaudeSource` drops for a transcript, then every line again from byte 0;
`p4-claude-transcript`'s Result says lines replayed twice are harmless, since its replay is keyed by ids.)

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:15, orchestrator, from `p4-watch-reads`' Result: when a `TailRead` says `restarted`, feed `Gone` and
  then `Appended`. `names` returns `io::Result`, so a listing error does not read as every name gone.
- 2026-11-03 22:20: started by `wakeshot-worker`. Read the card, the worker file, `docs/plan.md` "How we work" and
  `p4-watch-reads`' Result.
- 2026-11-03 22:31: read `sources.rs`, `testing.rs`, `event.rs`, `watch/read.rs`, `watch/data.rs`, `process.rs`,
  `log.rs`, the Claude source (`sources/claude/mod.rs`, for how it answers `RetryOnce` and tails), track notes 6, 14,
  16, 18 and architecture sections 5 to 8. Design set (see Result); writing `agents.rs` with its tests.
- 2026-11-03 22:36: `agents.rs` written with 18 tests (tests and code in one pass, not strictly tests first); one
  compile fix (the `Clock` trait import in the tests), then all 18 pass (0.04 s). `rustfmt --edition 2024` run on
  `agents.rs` only. Clippy next.
- 2026-11-03 22:38: clippy `type_complexity` on the name filter maps, fixed with an `Accept` alias. A retry answered
  with `RetryOnce` again now counts as settled (its bytes are not fed again until they change), so a bad file does not
  cost a retry on every tick; test changed to match. Added a compile-time `Send` check on `Agents`. All checks clean.
- 2026-11-03 22:41, orchestrator note: read `p4-claude-transcript`'s Result; a `FindTail` must be read from byte 0
  when it enters `wanted()`. Its "Found outside scope" 2 names a case my first version got wrong: state was dropped
  only between wants, so a resume whose leftover status file is fed `Gone` before the new one kept the old tail place.
  Fixed: after each feed the state of every want that left is dropped at once (`Slot::feed`, `prune`); the folder and
  listing being read are taken out of their maps while they feed. New test
  `a_tail_that_leaves_and_comes_back_within_one_call_starts_from_the_start`; it fails without the fix (checked by
  removing the `prune` call, then restoring the file).
- 2026-11-03 22:44: every check run again, all clean (list in the Result). Build folder deleted. Card done.

## Result

Status: done. Deliverables 1 to 5 built, driven by calls, no thread of its own.

Files changed: `crates/wakeshot-windows/src/watch/agents.rs` (filled, 19 tests), this card. No contract change and no
other file touched. `docs/research.md` unchanged: no new outside fact (the read and listing behavior the tests rely on
is in its "Watch reads checks").

API for `p4-watch-thread` (all in `wakeshot_windows::watch::agents`; `Agents` is `Send`, checked at compile time):

- `Agents::new(sources: Vec<Box<dyn AgentSource + Send>>, log: Arc<dyn Log>)`: the card's `new(sources)` plus the
  log, held like `DataFolder`'s.
- `folders() -> Vec<WatchFolder { dir, subtree }>`: sorted, each folder once; `subtree` for a `Names` want of depth 1
  or more. Tails are not watched. Call it after every call below that can feed (`on_tick`, `on_change`, `retry`) and
  add or drop watches to match: a feed can add a folder.
- `on_tick() -> Vec<Retry>`: every want of every source in full (list, read, feed `Gone` for files that left) and every
  tail. It is also the first round at start.
- `on_change(folder: &Path, names: Option<&[PathBuf]>) -> Vec<Retry>`: a notice in one of `folders()`. `names` are
  relative to `folder` as the folder watch gives them; `None` for `Rescan` (reads the folder in full). Runs only the
  sources that watch `folder`: their files there (only plain names that `accept` passes), their listings of it, and
  all their tails.
- `retry(&[Retry]) -> Vec<Retry>`: run it `RETRY_MS` (200) after a call returned retries. It feeds even unchanged
  bytes and never retries the same file again; what it returns is for other files the same call read.
- `snapshots(liveness: &dyn Liveness, now: Now) -> Snapshots { events, live }`: call it after each call above and
  after an exit wait fires. `events`: one `Event::Sources` per source whose `(SessionFacts, Alive)` list changed since
  the last call, every source on the first call (even one with no sessions). `live`: every `Alive::Yes` process of
  every source, sorted, changed or not: keep one exit wait per entry. `liveness` is passed per call like
  `DataFolder::holds`; `now.mono_ms` drives the log limit.
- `forget_posted()`: after a failed post, so the next `snapshots` returns every source.
- `Retry` has private fields (a caller cannot make one for a path no source asked for) and `file() -> FileId` for logs
  and tests.

How it feeds:

- `Folder`: names from `read::names` (full read) or from the notice (plain single names only); `accept` is checked
  before `read_whole`, which checks it again. `Whole` only when the bytes differ from the last bytes the source took. A
  file gone from the listing, or `NotFound` on read, is fed `Gone` once. `Fed::RetryOnce` gives a `Retry`; so does a
  read error other than a refused name or an oversized file (sharing violation, delete pending), and such a file is
  never fed `Gone`. A listing error keeps the last files.
- `Names`: listed on each tick and on any notice in the folder; `Present` when a name appears, `Gone` when it goes. A
  listing error keeps the last list.
- `Tail` and `FindTail`: polled on each tick and on any change in a folder of the same source; only complete new lines
  are fed as `Appended`, the poll repeated while `more`. A restart is fed `Gone`, then `Appended` from byte 0
  (orchestrator ruling); a file that goes is fed `Gone` once, and when it comes back its lines come from the start with
  no second `Gone`. A `FindTail` tails the first found file (`read::Finder`, newest folder first) and keeps it while it
  is still found. A tail feed's `RetryOnce` is not retried.
- `wanted()` is asked at the start of each call and again after each feed. After each feed the state of every want
  that left is dropped at once, so a want that leaves and comes back starts over: a `FindTail` that enters `wanted()`
  is read from byte 0, as `ClaudeSource` expects (`p4-claude-transcript` "Found outside scope" 2, the resume case
  included). A new want is read in full in the same call, so the transcript a status file names is fed in that call.
  At most 1024 wants per source per call.
- Liveness: `Process` through `process::Liveness::check`. `AnyProcess` gives `Unknown` with no check and a log line;
  the comment names `p4-codex-source`.
- Log (area `watch`, level Warn): at most one line per source per minute, naming each problem not logged yet: format
  problems (`Unreadable` text, `UnknownValue` field only, never its value), `AnyProcess`, liveness `Unknown`, read and
  listing errors (error code only, never a path). A problem logs once while it lasts and again if it clears and comes
  back. The "update Wakeshot" notice stays the core's.

Choices (reason; change it when):

- Time comes in as `Now` to `snapshots`, the only call that logs; the module reads no clock, like the core, and the
  test uses `FakeClock`. Change it when another call needs the time.
- Unchanged folder bytes are not fed again (the last taken bytes are kept per file; a status file is under 2 KB), so
  the 30 s tick costs no parse and a bad file no retry. Change it when a source needs every read fed.
- A read error that can pass gets the one 200 ms retry too, so a status write caught midway is not missed for a whole
  tick. Change it when the log shows retries for files that never read.
- Liveness `Unknown` is logged (architecture section 5 says it is), under the same per-source limit. Change it when
  the core logs it itself.
- Problems are logged as state, not per occurrence, so a lasting problem does not write a line every minute. Change it
  when Haik wants a repeat line while a problem lasts.
- A `FindTail` follows one file, not every match: `ClaudeSource` keeps only the first file it was fed for a session.
  Change it when a source needs all matches.

Acceptance checks (2026-11-03 22:44):

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/watch/agents.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean (the first run had one
  `type_complexity` in this file, fixed). No failure in another card's file came up.
- `cargo test --locked -p wakeshot-windows`: 275 passed, 0 failed, 17 ignored; the 19 below among them.
- No open of a file a source did not ask for, checked by reading every open path: `read_whole` takes only `dir.join`
  of a single name that `accept` passed, `dir` a wanted `Folder`; `Tail::new` takes a `Tail` path or a path
  `Finder::find` returned for a `FindTail` (plus `Finder`'s attribute-only check of its cached paths); `names` only
  lists. A `Retry` is made only inside the module and read only while its folder is still wanted.

Tests (all in `watch::agents::tests`): `folders_lists_each_watched_folder_once_with_subtree_for_depth`,
`tick_feeds_each_want_kind_and_never_a_refused_name`,
`folder_feeds_a_file_again_only_when_it_changed_and_gone_when_deleted`,
`change_reads_only_the_named_files_and_runs_only_that_folders_sources`,
`retry_reads_again_once_and_feeds_even_unchanged_bytes`,
`a_retry_answered_with_retry_once_again_is_not_retried_again`,
`a_read_that_fails_is_retried_logged_and_never_fed_as_gone`, `names_feeds_a_file_appearing_and_going`,
`a_folder_that_cannot_be_listed_keeps_its_last_files`, `tail_feeds_only_new_complete_lines_on_ticks_and_changes`,
`tail_restart_feeds_gone_then_the_lines_from_the_start`,
`a_tail_that_leaves_the_wants_starts_from_the_start_when_it_comes_back`,
`find_tail_keeps_its_file_and_feeds_gone_when_it_goes`, `wanted_is_asked_again_after_each_feed`,
`a_tail_that_leaves_and_comes_back_within_one_call_starts_from_the_start`,
`process_liveness_is_yes_gone_or_unknown_and_live_ones_are_returned`, `any_process_gives_unknown_without_a_check`,
`no_event_when_nothing_changed`, `format_problems_log_at_most_once_a_minute_per_source`.

Open items: none. Build folder and scratch scripts deleted.
