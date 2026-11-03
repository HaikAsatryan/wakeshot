# Watch thread: file reads, tails and finds

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 75K tokens (was 100K; the lock-holder
part moved to `p4-codex-source`, after v1, Haik 2026-11-03). Bundle: first card of `p4-watch-reads` +
`p4-watch-data` (`docs/phases/02-06-lighter.md`).
Runs after: the contracts card (it creates `watch/read.rs`, `lockowner.rs` and adds the Restart Manager feature).
Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-reads\target`. Uses the real laptop: no (temp folders and
processes the tests start).

## Goal

The I/O behind each `WantedFile` kind, so the watch thread can feed any source: whole reads that never block the
agent, tails that hand over complete lines only, a finder for files whose folder is not known, and name-only
listings. Restart Manager's answer to "which process holds this file open" serves only Codex and Copilot, so it
moved to `p4-codex-source` (after v1); leave the `lockowner.rs` stub as it is.

## Read first

- `docs/phases/04-sources.md`: track note 10.
- `crates/wakeshot-core/src/sources.rs` (`WantedFile`, `Feed`, `FileId`, after the contracts card).
- `spikes/claude-watch/src/names.rs` (`StatusFile::read`: share flags, the name check at the open), `tail.rs`,
  `watch.rs` (`find_transcript`).

## Deliverables

1. `crates/wakeshot-windows/src/watch/read.rs`:
   - `read_whole(path, accept) -> io::Result<Vec<u8>>`: opens only if `accept(file name)` passes, checked again at the
     open (the spike's guard against `.key` files); read sharing, write sharing and delete sharing, so a Claude write
     or delete is never blocked; size cap (1 MB: a status file is under 2 KB; over the cap is an error, not a read).
   - `Tail`: offset plus held-back partial line; `poll()` returns the new complete lines' bytes; a file that got
     shorter or was replaced (file id changed) restarts from 0 and says so; a first poll after start reads from 0
     (rule 5: replay from the start after a Wakeshot restart).
   - `find(root, depth, name_suffix) -> Vec<PathBuf>`: exactly `depth` folder levels under `root`, newest folders first,
     names compared case-insensitively; a cache so the tick does not walk Codex's date folders every 30 s (walk again
     only when a wanted name is not found or a cached path is gone).
   - `names(dir, depth, accept) -> Vec<PathBuf>`: lists names, never opens a file.
2. Moved to `p4-codex-source` (after v1): `crates/wakeshot-windows/src/lockowner.rs`, `holders(path) ->
   Result<Vec<ProcessRef>, WinError>` through `RmStartSession`, `RmRegisterResources`, `RmGetList` (retry on
   `ERROR_MORE_DATA`), `RmEndSession` always; a `ProcessRef` from `RM_UNIQUE_PROCESS`. Not built here.

Tests first: whole reads while another handle writes and deletes the file; the name guard refuses `x.key` even with a
crafted path; tails across partial lines, CRLF and LF, truncation, replacement, a 10 MB file; `find` at depth 1 and 3
with decoys; `names` on a folder with a file the test holds with an exclusive lock (`LockFileEx`), which must not be
disturbed. (The `holders` tests moved to `p4-codex-source` with deliverable 2.)

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- Every `unsafe` block has a comment that says why it is sound. Every child process a test starts is ended by its
  handle.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Which files to read and when (`p4-watch-agents`, `p4-watch-data`, `p4-watch-thread`). Process liveness (power
  track). Any file not listed above.

## Questions for Haik

None.

## Found outside scope

- Transient, gone at 21:03: clippy `--all-targets` failed on 3 lints in `p3-power-io` files (`io_queue.rs:347`,
  `power/executor.rs:993` and `:1371`); that card fixed them while it built. Not touched.
- `docs/architecture.md` section 8 says every function of this crate that calls Windows returns `WinError`; the card
  gives `read.rs` `io::Result` (std calls, and `NotFound` maps straight to `Feed::Gone`). Kept as the card says;
  the architecture line could name the `watch::read` exception when it is next edited.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 20:49: started by `wakeshot-worker` (bundle with `p4-watch-data`). Read the card, track notes 10 and 18,
  `sources.rs`, the spike's `names.rs`, `tail.rs` and `find_transcript`.
- 2026-11-03 21:01: `watch/read.rs` written with 18 tests; all pass on the first run (0.07 s). `rustfmt --edition 2024`
  run on `read.rs` only (the repo has no `rustfmt.toml`, so code wraps at 100). Clippy without `--all-targets`: clean
  on x64 and aarch64. Clippy `--all-targets` on both targets: 3 errors, all in `p3-power-io` files
  (`io_queue.rs:347` `type_complexity`, `power/executor.rs:993` `type_complexity`, `power/executor.rs:1371`
  `needless_range_loop`); none in `read.rs`. That card is building now: run again at the end of the bundle.
- 2026-11-03 21:03: checked the facts the comments state: a scratch probe of Win32 name trimming (`cmd /d /c type`
  and .NET `File.ReadAllText`, scratch folder, deleted after), and std's Windows source in the toolchain for the
  share mode default, `DirEntry` metadata and `fs::metadata`. `docs/research.md` "Watch reads checks" added. Docs in
  `read.rs` made exact: `Finder`'s "still there" check is an attribute-only open (std's `fs::metadata`).
- 2026-11-03 21:04: all checks run again, all clean (list in the Result). Card done; `p4-watch-data` next.

## Result

Status: done. Deliverable 1 built; deliverable 2 (`lockowner.rs`) left as the stub, as the card says.

Files changed: `crates/wakeshot-windows/src/watch/read.rs` (filled, 18 tests), `docs/research.md` (new section
"Watch reads checks", additions only), this card.

API for `p4-watch-agents`, `p4-watch-data` and `p4-watch-thread` (all in `wakeshot_windows::watch::read`):

- `read_whole(path, accept: impl Fn(&str) -> bool) -> io::Result<Vec<u8>>`, and `read_whole_written(path, accept)
  -> io::Result<(Vec<u8>, FileTime)>`, which adds the last write time from the same handle (hold files). The name
  check runs again at the open and also refuses a name that ends in `.` or a space or holds a `:` (Win32 would open
  another file: checked). Errors: `InvalidInput` refused name, `FileTooLarge` over `WHOLE_CAP` (1 MiB), `NotFound`
  gone (feed `Gone`), else the open's error (a writer that shares no read gives 32 at once).
- `Tail::new(path)`, `tail.path()`, `tail.poll() -> io::Result<TailRead>`; `TailRead { lines, restarted, more }`.
  `lines`: complete lines only, with their `\n` (and `\r` if present), ready for `Feed::Appended`. `restarted`: the
  file got shorter or its file id changed; `lines` start from byte 0 again. `Feed` has no restart kind, so my
  suggestion for `p4-watch-agents`: feed `Gone`, then `Appended(lines)`. `more`: the poll stopped at `POLL_CAP`
  (8 MiB); poll again. The first poll reads from 0 and is not a restart. `NotFound` while the file is gone; the tail
  keeps its place, so a new file under the name restarts.
- `find(root, depth, name_suffix) -> Vec<PathBuf>`: files at exactly `depth` folder levels, newest folders first
  (then newest files), suffix compared without case, folder links not followed, unreadable folders empty. `Finder`
  (`Finder::new()`, `finder.find(root, depth, suffix)`) caches each answer: it walks again only while nothing is
  found or once a cached path is gone; past 256 entries the cache starts over.
- `names(dir, depth, accept) -> io::Result<Vec<PathBuf>>`, sorted. Listing only, no open. A missing `dir` gives
  `Ok(empty)`, a subfolder that goes away is skipped, any other error is `Err`. That is a change from the card's
  `-> Vec<PathBuf>`: on a listing error the caller keeps its last list instead of feeding every name as `Gone`.
- `#[cfg(test)] pub(crate) TempDir` (`new(tag)`, `path()`, `write(rel, bytes)`) for the other watch tests.

Choices (reason; change it when):

- Name check before the open only, no check of the opened file's real name (`GetFinalPathNameByHandleW`). A planted
  symlink or hard link `1.json` to a `.key` file would pass, but planting one needs the user's own rights. Change it
  when a source reads a folder other users can write.
- `POLL_CAP` 8 MiB per poll, no cap on one line's length: memory stays near the larger of 8 MiB and the longest line.
  Change it when a live transcript shows lines of tens of MB.
- Truncation is seen by length, replacement by file id; a file truncated and regrown past the old offset between two
  polls is not seen. Claude transcripts are append-only. Change it when a source rewrites its tail file in place.

Tests (all in `watch::read::tests`): `whole_read_shares_with_a_writer_that_writes_renames_and_deletes`,
`whole_read_fails_at_once_while_a_writer_shares_no_read`, `name_guard_refuses_a_key_file_even_with_a_crafted_path`,
`whole_read_stops_at_the_cap`, `whole_read_gives_the_write_time_of_the_bytes_read`,
`tail_hands_over_complete_lines_only_for_lf_and_crlf`, `tail_first_poll_reads_from_the_start`,
`tail_restarts_when_the_file_gets_shorter`, `tail_restarts_when_the_file_is_replaced`,
`tail_waits_for_a_missing_file_and_restarts_on_a_new_one`, `tail_reads_a_10_mb_file_in_capped_polls`,
`find_takes_exactly_one_level_with_decoys`, `find_takes_exactly_three_levels_with_decoys`,
`finder_walks_again_only_when_not_found_or_gone`, `finder_cache_starts_over_past_its_cap`,
`names_lists_locked_files_without_disturbing_the_lock`, `names_at_depth_one_lists_subfolder_files_only`,
`names_of_a_missing_folder_is_empty_and_a_file_is_an_error`.

Checks (2026-11-03 21:03 to 21:04):

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/watch/read.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean (the first run's
  failures were in `p3-power-io` files, see "Found outside scope").
- `cargo test --locked -p wakeshot-windows`: 162 passed, 0 failed, 12 ignored; the 18 above among them.
- `unsafe`: one block in the module (`file_key`), two in the tests (`lock`, `unlock`), each with a `SAFETY` comment.
  No test starts a child process. No test needs a Medium token: share modes and byte range locks apply to every
  token.

Open items: none for this card. `lockowner.rs` waits for `p4-codex-source` (after v1).
