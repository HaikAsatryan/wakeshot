# Watch thread: the thread itself

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p4-watch-folders`, `p4-watch-data`, `p4-watch-agents`, and the power track's `p3-process` (the real
`ProcessHandle` and `ExitWait`: they have no fake, and the thread registers an exit wait for every live session and
hold; review M4). Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-thread\target`. Uses the real laptop: no (temp folders, fakes).

## Goal

The watch thread of section 4: one thread that waits on folder notices, process exits, the tick and a stop event,
and posts `Holds`, `Sources`, `ConfigChanged` and `Menu(Restore)` to the loop through the `EventSink`.

## Read first

- `docs/phases/04-sources.md`: track notes 14 to 16.
- `docs/architecture.md` section 4 ("Tray threads": watch row; posting rules), section 3 "Clocks" (safety tick).
- `docs/reviews/p1-contracts-domain.md` M1 (the `EventSink` seam).
- `crates/wakeshot-windows/src/watch/dir.rs`, `data.rs`, `agents.rs` (from the three cards before this one).
- From the contracts card: `EventSink` and its fake; the power track's `process::Liveness` (fake `FakeLiveness`),
  `ProcessHandle` and `ExitWait` (exit waits); `log::Log` (fake `MemoryLog`).

## Deliverables

`crates/wakeshot-windows/src/watch/thread.rs`:

1. `WatchThread::start(dir: DataDir, sources, liveness, sink: Arc<dyn EventSink>, log) -> WatchHandle`. The thread
   watches `holds\` and the data folder (for `config.json` and `restore-request.json`) and every agent folder
   `Agents::folders()` names; folders added later (a new `wanted()`) are added to the watch.
2. First round at start: read everything and post `Holds`, every `Sources` and `ConfigChanged` if the file differs
   from the default, so the loop has a full picture right after `Started` (integration posts `Started` first).
3. Events: a folder notice runs `data` or `agents` for that folder; a `Rescan` runs everything for it; the 200 ms
   retries run when due; a process exit (the power track's `ExitWait`, registered for every live session and hold
   process; the callback only sets the thread's wake event) re-checks liveness at once; `WatchHandle::tick()` (the
   loop's safety tick) runs `on_tick`, the holds' liveness again and `retry_missing()`.
4. Posts only changed snapshots (track note 14). A failed post is logged; the next tick posts the full state again.
5. `WatchHandle::stop()` ends the thread within 1 s and drops every exit wait and folder handle.

Tests first, with `FakeEventSink`, `FakeLiveness`, `FakeSource` and a temp data folder: start posts the first
round; writing a hold file posts `Holds` within 1 s; deleting it posts `Holds` without it; a restore request posts one
`Menu(Restore)`; a config change posts `ConfigChanged`; a fake process exit posts the session or hold as `Gone` at
once; a tick re-posts nothing when nothing changed; a failed post is retried on the next tick; stop is clean (no
thread or handle left).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- The thread makes no power call and touches no file outside the data folder and the folders sources ask for.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Starting the thread in the tray, `Started`, `DeleteHold` (integration). The real `process::Liveness` (power track) and
  the real `EventSink` (integration, which the orchestrator plans after the tracks; review M10). Any file not listed
  above.

## Questions for Haik

Each with the worker's default. Haik was asleep, so the worker decided both by his rule (with no work going on, the
laptop behaves as if Wakeshot were not there); they are in the Result for his review.

1. A restore request whose post to the loop failed is posted again at the next safety tick (up to 30 s later) while
   `restore-request.json` is still there. The CLI has said by then "It still restores your power settings when it
   gets to the request", so the late restore keeps that promise. Change it when a late restore turns out to surprise.
2. An agent folder that does not exist at start (Claude Code never run yet) is first watched at the next tick, so a
   first ever session is seen up to 30 s late (the folder card's choice, kept). A hold that starts 30 s late costs
   nothing against sleep timeouts of minutes. `holds\` is the exception: a notice in the data folder opens it at once,
   since `wakeshot run` makes it and a hold must show in the menu at once.

## Found outside scope

1. `crates/wakeshot-windows/src/watch/data.rs` (owner `p4-watch-data`): 6 tests fail since the Claude fixes' m7 change
   in `wakeshot-core/src/holds/file.rs` refuses pid 4 ("pid 4 is the System process"): their hold files use
   `"pid":4` (`a_bad_file_is_retried_once_then_logged_once`, `a_bad_file_fixed_before_its_retry_counts_with_no_line`,
   `a_bad_expires_is_left_out_like_a_bad_file`, `an_unreadable_file_is_retried_then_logged`,
   `temp_names_and_folders_are_never_opened`, `a_holds_folder_that_cannot_be_listed_is_logged_once`). Fix: another
   pid in those tests (any but 0 and 4). Not touched; it fails `cargo test -p wakeshot-windows` until fixed.
2. The folder card's question 1 stays open for Haik: the thread keeps every watch open all the time, so while the tray
   runs, renaming `~/.claude` fails with Access denied (deleting it works).

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:29: started by `wakeshot-worker`. Read the card, `docs/plan.md` "How we work", track notes 14 to 18,
  architecture sections 3 "Clocks" and 4, the Results of `p4-watch-folders`, `p4-watch-data`, `p4-watch-agents`, and
  `dir.rs`, `data.rs`, `agents.rs`, `process.rs`, `sink.rs`, `log.rs`, `store.rs` (`DataDir`).
- 2026-11-04 00:44: `watch/thread.rs` written with 14 tests (plus one ignored child role); code and tests in one pass,
  not strictly tests first. Clippy x64 `--all-targets` clean on the first run; all 14 pass on the first run (1.25 s).
  Orchestrator added the review's m3 fix in `watch/agents.rs` to this card (after the main work). Next: mutation
  checks, then m3.
- 2026-11-04 00:46: mutation check (scratch script, file restored, hash checked): no exit waits, no `retry_missing`
  on a data folder notice, no resync on the tick: exactly the three matching tests fail.
- 2026-11-04 00:48: m3 done in `watch/agents.rs` (`fill_folder`: on a listing error the known names are the
  targets). New test with a deny-list ACE on the folder only (`SetFileSecurityW`; the denial checked first, error 5,
  so the elevated shell does not pass it for the wrong reason); the folder-replaced-by-a-file test now expects `Gone`,
  as the fix card said. Both fail with the old `return true` (mutation, file restored). All checks next.
- 2026-11-04 00:50: the first `watch::` run after the m3 mutation still ran the mutated binary: `Copy-Item` kept the
  backup's write time, so cargo saw nothing new (`docs/research.md`, "Watch thread checks"). Touched both files and
  ran everything again. 6 `watch::data` tests fail: their hold files use pid 4, which the core now refuses (Claude
  fixes m7; Found outside scope 1). Not this card's file.
- 2026-11-04 00:52: every check run, outcomes in the Result. `docs/research.md` "Watch thread checks" added. Card done.

## Result

Status: done. Deliverables 1 to 5 built in `crates/wakeshot-windows/src/watch/thread.rs`; the orchestrator's extra
item (Claude parsing review m3) built in `crates/wakeshot-windows/src/watch/agents.rs`.

Files changed: `crates/wakeshot-windows/src/watch/thread.rs` (filled, 14 tests and one ignored child role),
`crates/wakeshot-windows/src/watch/agents.rs` (m3 fix, one new test, one test changed), `docs/research.md` (new
section "Watch thread checks", at the end), this card. No contract change, no `Cargo` file.

API for `p6-int-tray-mode` and `p4-live-claude` (all in `wakeshot_windows::watch::thread`):

- `WatchThread::start(dir: DataDir, config: Config, sources: Vec<Box<dyn AgentSource + Send>>, liveness: Arc<dyn
  Liveness>, sink: Arc<dyn EventSink>, log: Arc<dyn Log>) -> Result<WatchHandle, WinError>`. One parameter more than
  the card: `config` is the value the core has (pass `Started`'s config; a harness passes `Config::default()`), so a
  `config.json` change between integration's read and the thread's is still posted. `Err` only when an event or the
  thread cannot be made.
- Start it after posting `Started`: the first round posts at once, in this order: `ConfigChanged` (only when the file
  differs from `config`), `Holds` (always, `[]` too), one `Sources` per source (always), `Menu(Restore)` when a request
  waits. Later rounds post in the same order, each only when it changed (track note 14). Config first, so the core
  never acts on a hold that a new Automatic setting turns off; restore last.
- `WatchHandle::tick()`: call it on each safety tick (the dispatcher's `watch_tick`). Never blocks. The thread has no
  timer of its own besides the 200 ms retries, so a harness without a loop (the `agent-watch` example of
  `p4-live-claude`) must call it every 30 s itself.
- `WatchHandle::stop(self) -> bool`: ends the thread, waits at most 2 s, `false` (and one log line) when the thread
  did not end in time (it is left to end on its own) or panicked. Dropping the handle stops it the same way.
  `WatchHandle` is `Send + Sync` (checked at compile time).
- Log lines (area `watch`, Warn): a failed post (one line per run of failures), an exit wait that could not be set
  (once per process), the watch full (once), a failed wait (once per run of failures). No path, no file content.

How it works:

- One thread, made by `start`; the `FolderWatch` is made on it and stays there. It waits on, in this order: a stop
  event (manual reset, so no later wait misses it), a tick event and a wake event (both auto reset), then the folders.
- Folders: the data folder and `holds\` (one level each), then every folder `Agents::folders()` names, before the
  first full read. After each call that can feed, the watch is made to match `folders()` again; a folder that opens
  then is read in full at once (`on_change(folder, None)`), since the call that asked for it read it before its watch
  existed. A folder that leaves is removed (never the two data folders).
- A data folder notice, any name, reads `config.json` and the restore request (a short 8.3 name or a lost notice must
  not hide a change; both files are tiny) and runs `retry_missing()`, so `holds\` made by the first `wakeshot run` is
  watched at once. A `holds\` notice reads every hold file. An agent folder notice runs `Agents::on_change` with the
  names, or `None` for `Rescan`.
- Retries: `Agents` retries in batches, each due 200 ms after the call that returned it; the hold retry 200 ms after a
  read that returned one. The wait's timeout is the nearest due time.
- Exit waits: one per live session and live hold process (`ProcessHandle::open`, `ExitWait::register`; the callback
  only sets the wake event). A process no longer live loses its wait. A wake reads the holds again; every round checks
  each session's liveness in `snapshots`. A process that ended between its check and the open sets the wake once; one
  that cannot be opened is logged and seen at the next tick; neither is tried again while it stays live, so a
  liveness answer that disagrees with the open never spins the thread.
- Tick: after a failed post, first everything is marked unposted (holds, every source, the config; the restore if its
  post failed and the file is still there). Then `retry_missing()`, `Agents::on_tick`, holds, config, restore.
- A failed post: one log line, no retry until the tick, so a full queue gets no burst of posts.
- Stop: the exit waits drop first (each waits for a running callback), then the folder watch (every read cancelled and
  waited for).
- Files: the thread opens only the data folder's files (through `DataFolder`; one `is_file` on the restore request)
  and what sources ask for (through `Agents`). No power call (grep of the non-test code).

m3 (Claude parsing review, `agents.rs` `fill_folder`): on a listing error the known names are read one by one
instead of returning, so a status file that changes or goes still reaches its source; a new file waits until the folder
lists again. The listing error is still logged once. A folder replaced by a file now feeds its known files `Gone` (the
read finds no file), as the fix card said.

Choices (reason; change it when):

- `start` takes the core's `config` (above). Change it when integration has no config at hand.
- Stop waits at most 2 s, then leaves the thread, so a read in progress never hangs the tray's quit. Change it when a
  read can legally take longer.
- No retry before the tick after a failed post: the post failed because the queue is full or the window is gone.
  Change it when the log shows lost posts in normal use.
- Any data folder notice reads both small files, not only their names. Change it when the data folder gets busy
  (state writes are the only other traffic).
- A failed wait pauses 1 s (stop still ends it), then reads everything as on a tick. It never happened in the tests.

Decided by the worker, for Haik's review: "Questions for Haik" 1 (a lost restore request is posted again at the next
tick) and 2 (an agent folder made after start is first watched at the next tick; `holds\` at once).

Checks (2026-11-04 00:50 to 00:52):

- `rustfmt --edition 2024 --check` on `watch/thread.rs` and `watch/agents.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-windows watch::`: 85 passed, 6 failed (all in `watch::data`, Found outside scope 1),
  1 ignored. `cargo test --locked -p wakeshot-windows`: 367 passed, 6 failed (the same 6), 20 ignored; doc tests 0.
- Mutation checks (scratch scripts, files restored and checked by hash): without exit waits, without `retry_missing`
  on a data folder notice, or without the resync on the tick, exactly the three matching thread tests fail; with the
  old `return true` in `fill_folder`, both m3 tests fail.
- No test touches `%LocalAppData%\Wakeshot` (every `DataDir` is `DataDir::at(<temp folder>)`); every child process a
  test starts is its own test exe, ended by its own handle; no `%TEMP%\wakeshot-watch-*` folder left.

Tests in `watch::thread::tests`: `start_posts_the_first_round`, `the_first_round_posts_no_config_the_core_already_has`,
`a_hold_file_posts_holds_within_a_second_and_its_delete_posts_holds_without_it`,
`a_holds_folder_made_after_start_is_watched_without_a_tick`, `a_restore_request_posts_one_menu_restore`,
`a_config_change_posts_config_changed`, `a_process_exit_posts_the_session_and_the_hold_as_gone_at_once` (real child
processes; 4 ms from the end to the post), `a_tick_posts_nothing_when_nothing_changed`,
`a_failed_post_is_logged_once_and_the_next_tick_posts_everything_again`, `a_retry_runs_200_ms_later_without_a_tick`,
`a_folder_a_source_asks_for_later_is_watched_and_read_in_full`,
`an_agent_folder_made_after_start_is_read_at_the_next_tick`, `stop_ends_the_thread_within_a_second`,
`start_and_stop_leave_no_handle` (50 cycles in a child test process: 122 handles before and after); `sleep_role`
(ignored, the child role). In `watch::agents::tests`: `a_folder_that_cannot_be_listed_reads_its_known_files` (new),
`a_folder_replaced_by_a_file_feeds_its_known_files_gone` (was `a_folder_that_cannot_be_listed_keeps_its_last_files`).

Open items: Found outside scope 1 (the `watch::data` tests, another card's file). Build folder and scratch files
deleted.
