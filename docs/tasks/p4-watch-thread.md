# Watch thread: the thread itself

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
