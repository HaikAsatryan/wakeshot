# Watch thread: folder change notices

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: the contracts card (it creates `watch/dir.rs`). Parallel with: every other card of this track and every
card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-folders\target`. Uses the real laptop: no (temp folders only).

## Goal

One small type that tells the watch thread which watched folders changed, with no polling: overlapped
`ReadDirectoryChangesW` on several folders, waited on together with the thread's own wake and stop events.

## Read first

- `docs/architecture.md` section 4, "Tray threads" (the watch row).
- `docs/research.md`: "Reading elevated Claude Code sessions from an unelevated process" (change notices work at
  Medium for files written by a High process).
- `spikes/claude-watch/src/watch.rs` (how the spike polled; this card replaces polling).
- `crates/wakeshot-windows/src/error.rs` (`WinError`).

## Deliverables

`crates/wakeshot-windows/src/watch/dir.rs`:

1. `FolderWatch`: add and remove folders by path, each either one level or its whole subtree (a flag: Copilot's
   `session-state` needs the subtree, since its lock files and `events.jsonl` sit one level down), each opened with
   `FILE_LIST_DIRECTORY` and full sharing, so Wakeshot never blocks a rename or delete by the agent. Overlapped reads
   with an event per folder. Names in a subtree come back relative to the watched folder.
2. `wait(timeout, extra: &[HANDLE]) -> Woke`: returns the folders that changed with the file names, or which extra
   handle fired (the thread's wake and stop events), or a timeout. Names come back as given; the filter is the
   caller's (a `.key` name is only a name here, never opened).
3. Overflow (`ERROR_NOTIFY_ENUM_DIR` or a zero-byte result) gives `Changed::Rescan(folder)`. A folder that does not
   exist yet (an agent never run) or is deleted later is kept on a retry list and opened again by `retry_missing()`,
   which the thread calls on each tick; it then reports `Rescan` once.
4. Clean shutdown: cancel I/O, wait for it, close handles; no leak across 1,000 add and remove cycles (handle count of
   the test process before and after).

Tests first, in temp folders: create, write, rename into place (how atomic writes look), delete; two folders at once;
a folder created after the watch starts; the watched folder deleted and created again; a burst of 5,000 files that
overflows a small buffer gives `Rescan`; the wake and stop events; timeouts. Notice latency is logged in the test
output (no assert below 1 s).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- Every `unsafe` block has a comment that says why it is sound.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Reading files (`p4-watch-reads`), the thread loop (`p4-watch-thread`). Any file not listed above.

## Questions for Haik

Each with the worker's default.

1. Decided by the worker, for Haik's review: a watched folder blocks renaming any folder above it, so while Wakeshot
   runs, renaming `~/.claude` (watched: `~/.claude/sessions`) or `~/.copilot` fails with Access denied, even with no
   agent working. Deleting them works, and renaming the watched folder itself works. Any open file does the same
   (`docs/research.md`, "Folder watch checks"). Default: keep the watches open all the time, since that is how
   Wakeshot sees an agent start without polling. The other way: close the agent watches while no agent runs and poll
   their folders on the tick instead (an agent's start is then seen up to one tick late). Change it when Haik wants
   the "as if Wakeshot were not there" rule to cover folder renames too.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` section 5, "OS events" table, row "Folder changes" still says "several folders on one
   thread not tested". This card tested it (`two_folders_are_watched_at_once` and the rest of `watch::dir::tests`).
   The doc fix belongs to whoever owns that file next.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:07: started by `wakeshot-worker`. Read the card, plan, architecture section 4, the research entry,
  the spike's `watch_dir`, `error.rs`, and `p4-watch-agents`' Result (folders come as `WatchFolder { dir, subtree }`,
  names relative to the folder, `None` for `Rescan`).
- 2026-11-04 00:14: design set: one directory handle, one manual-reset event and one boxed `OVERLAPPED` plus buffer
  per folder; extras first in the `WaitForMultipleObjects` array so stop wins; every completed folder taken in one
  wait; a lost folder goes to the retry list with its error. Writing `dir.rs` and its tests.
- 2026-11-04 00:16: module written. First clippy run stopped in `wakeshot-core/src/sources/claude/transcript.rs`
  (12 errors, another card mid-edit); waiting and writing the tests meanwhile.
- 2026-11-04 00:21: the core compiles again. Clippy clean after 6 lint fixes (`is_multiple_of`, `!3`, an unused
  helper). 20 tests pass on the first run. Found: deleting the watched folder ends its read with
  `ERROR_ACCESS_DENIED`, and the name is free at once; a read ends when its thread ends (so the watch is not `Send`);
  renaming a folder above a watched one fails with Access denied, as it does for any open file under it (probe with a
  plain open file). Turned the two printing tests into asserting ones.
- 2026-11-04 00:24: Microsoft docs read for the overflow, buffer and event rules (`docs/research.md`, "Folder watch
  checks", added). All acceptance checks run, see Result. Build folder and probe folder deleted; no test folder left
  in `%TEMP%`.

## Result

Status: done. Deliverables 1 to 4 built in `crates/wakeshot-windows/src/watch/dir.rs`, tests written first.

Files changed: `crates/wakeshot-windows/src/watch/dir.rs` (filled, 20 tests), `docs/research.md` (new section
"Folder watch checks", added at the end), this card. No other file touched.

API for `p4-watch-thread` (all in `wakeshot_windows::watch::dir`):

- `FolderWatch::new()`; `add(dir: &Path, subtree: bool) -> Result<State, WinError>`: `State::Watching`, or
  `State::Missing(WinError)` with the open error (the folder is kept on the retry list); `Err` only past
  `MAX_FOLDERS` (60), and the folder is then not added. Adding the same path again reopens it only when it is missing
  or the flag differs. `remove(dir) -> bool`; `state(dir) -> Option<State>`.
- `wait(timeout: Option<Duration>, extra: &[HANDLE]) -> Result<Woke, WinError>`: `Woke::Changed(Vec<Changed>)` (one
  entry per folder, never empty), `Woke::Extra(index)`, `Woke::Timeout`. At most `MAX_EXTRA` (4) extra handles. An
  extra handle wins over folder changes that wait, so stop is never held up; use auto-reset events (one that stays
  set wins every wait). `None` waits with no limit.
- `Changed::Names { folder, names }`: names relative to `folder`, each once, sorted, as Windows gave them (old and new
  name of a rename; folder names too; in a subtree watch `<sub>\<file>`). Feed it as
  `agents.on_change(folder, Some(&names))`. `Changed::Rescan(folder)`: feed `on_change(folder, None)`.
  `Changed::folder()` gives the folder of either.
- `retry_missing() -> Vec<Changed>`: call it on each tick; one `Rescan` for each folder that opened.
- Make the `FolderWatch` on the watch thread and keep it there: it is not `Send`, because a read ends when the thread
  that queued it ends (tested). Dropping it is the clean shutdown: every read cancelled and waited for, then the
  handles closed.
- A folder is watched only from `add` on. Add a folder before its first full read (or read it in full after `add`),
  or a change in between waits for the next tick. A missing folder that `add` opens gives no `Rescan`.
- A watched folder that is deleted gives `Rescan` and goes `Missing` (`ReadDirectoryChangesW`, `ERROR_ACCESS_DENIED`);
  when it is made again, `retry_missing` opens it and gives `Rescan`.

Choices (reason; change it when):

- 64 KB buffer per folder: the most a network folder takes, and an overflow costs one `Rescan`. Change it when the
  log shows rescans in normal use.
- Extra handles first in the wait array, so stop and wake win over a busy folder. Change it when a caller needs
  changes before its wake.
- Every completed folder is taken in one wait, so one busy folder never starves another.
- When a folder's next read cannot be queued, its names give way to `Rescan` and the folder goes on the retry list;
  a lost folder reports `Rescan` when it goes and again when it comes back. Change it when the caller needs names and
  loss apart.
- A missing folder is retried on the tick only, with no watch on its parent: the agents' tick reads every want in
  full anyway. Change it when a first session must be seen faster than one tick.
- Folders are keyed by the path as given (no case folding or long-path form): the callers build paths from one base.
  Change it when a caller passes the same folder in two spellings.
- `wait` with nothing to wait on sleeps out its timeout, and fails with no timeout, instead of blocking for good.

Decided by the worker, for Haik's review: "Questions for Haik" 1 (a watch blocks renaming the folders above it).

Acceptance checks (2026-11-04 00:23 to 00:24):

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/watch/dir.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean. The first run at 00:16
  stopped in `wakeshot-core/src/sources/claude/transcript.rs` (another card mid-edit); it compiled again by 00:21.
- `cargo test --locked -p wakeshot-windows watch::`: 76 passed. `cargo test --locked -p wakeshot-windows`: 343 passed,
  0 failed, 18 ignored.
- Every `unsafe` block (15, the test `Send` wrapper included) has a `SAFETY` comment that says why it is sound.

Tests (all in `watch::dir::tests`): `a_new_file_a_write_and_a_delete_are_named`,
`a_rename_into_place_names_the_old_and_the_new_name`, `names_come_back_as_given`,
`a_subtree_watch_names_deeper_files_relative_to_the_folder`, `a_one_level_watch_never_names_deeper_files`,
`adding_again_with_the_subtree_flag_watches_deeper`, `two_folders_are_watched_at_once`,
`a_folder_made_after_the_watch_starts_is_opened_by_retry_missing`,
`the_watched_folder_deleted_and_made_again_is_watched_again`, `a_burst_that_overflows_the_buffer_gives_rescan`,
`the_wake_and_stop_events_end_the_wait`, `an_extra_handle_wins_over_changes_that_wait`, `waits_time_out`,
`too_many_folders_or_extra_handles_fail`, `a_path_that_is_a_file_is_missing`,
`a_read_cancelled_by_its_thread_ending_gives_rescan_and_goes_on`, `the_watch_never_blocks_a_rename_or_a_delete`,
`a_watch_blocks_renaming_a_folder_above_but_never_deleting_it`, `parse_reads_every_record_and_refuses_a_broken_one`,
`a_thousand_add_and_remove_cycles_leak_no_handle` (runs in a child test process: 122 handles before and after).
Latency lines in the test output (`--nocapture`): 0.3 to 2.6 ms from the file call to the notice.

Open items: none in this card. The architecture line under "Found outside scope" 1.
