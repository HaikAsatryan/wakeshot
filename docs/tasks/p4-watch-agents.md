# Watch thread: drive the agent sources and check their liveness

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p4-watch-reads`. Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6. Uses
`FakeSource`, so it does not wait for the parser cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-agents\target`. Uses the real laptop: no (temp folders, fakes).

## Goal

For any set of `AgentSource`s: ask what they want, read it, feed it, and turn their sessions into one `Event::Sources`
per source with each session's liveness, as section 7 says ("checks liveness: the adapter, never the parser").

## Read first

- `docs/phases/04-sources.md`: track notes 6, 7, 14, 16.
- `docs/architecture.md` section 5 "Liveness", section 7, section 8 (format problems logged at most once a minute per
  source; Liveness Unknown).
- `crates/wakeshot-core/src/sources.rs`, `testing.rs` (`FakeSource`), `event.rs`.
- `crates/wakeshot-windows/src/watch/read.rs`, `lockowner.rs` (from `p4-watch-reads`).
- From the contracts card: `process::Liveness` and `FakeLiveness`, the log seam and its fake.

## Deliverables

`crates/wakeshot-windows/src/watch/agents.rs`, driven by calls (no thread of its own):

1. `Agents::new(sources: Vec<Box<dyn AgentSource + Send>>)`; `folders()` lists the folders to watch (from `Folder`
   and `Names` wants, with the subtree flag for depth 1).
2. `on_change(folder, names)` and `on_tick()`: read and feed what changed: `Folder` files whole (a `Fed::RetryOnce`
   answer schedules one re-read 200 ms later: return it to the caller); `Names` as `Present` or `Gone`; every `Tail`
   and `FindTail` of the same source on a change in any of its folders and on each tick (section 7: transcripts are
   read on status changes and on the tick, not watched). `wanted()` is asked again after each feed.
3. Liveness for each session: `Process { pid, start }` through `process::Liveness`; `AnyProcess { image, lock_file }`
   through `lockowner::holders(lock_file)`: a holder whose image is `image` (from the process list) gives that holder's
   liveness and is cached until the lock file changes or the holder is gone; no holder gives `Gone`; a Restart Manager
   error falls back to "any live process with that image" (track note 7) while the lock file is `Present`, and a failed
   process list gives `Unknown`. Return the live `ProcessRef`s so the thread can wait on their exit.
4. `snapshots() -> Vec<Event>`: one `Event::Sources` per source whose snapshot changed since the last one returned
   (always all of them on the first call), sessions paired with their `Alive`.
5. Format problems: a session with `Unreadable` or `UnknownValue` gives at most one log line per source per minute
   (field name and fixed text only, never file content). The "update Wakeshot" notice is the core's.

Tests first, with `FakeSource`, `FakeLiveness` and temp folders: a feed per want kind; a retry answered once; a
`Names` file appearing and going; a tail fed only new lines; `Process` liveness `Yes`, `Gone`, `Unknown`;
`AnyProcess` with a holder (a child process the test starts holding a file), with no holder, with a Restart Manager
error (inject it), with a failed process list; no event when nothing changed; the log limit with a fake clock.

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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
