# Watch thread: file reads, tails and finds

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 75K tokens (was 100K; the lock-holder
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
