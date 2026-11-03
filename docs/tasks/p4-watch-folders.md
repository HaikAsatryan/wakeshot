# Watch thread: folder change notices

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
