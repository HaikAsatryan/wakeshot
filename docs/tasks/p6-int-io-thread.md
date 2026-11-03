# Integration: the IO thread loop

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens.
Real laptop: no (temp folders and fakes; no power plan, no notice on screen, no file opened).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-io-thread\target`.
Runs after: `p6-int-skeleton`, `p3-power-io` (`IoQueue`, `PowerExecutor`), `p3-store`, `p3-log`, `p4-cli-commands`
(`shell::open_with_default_app`). Parallel with: `p6-int-window`, `p6-int-dispatch`, `p6-int-startup` and any track
card.

## Goal

The IO thread of the tray: it pops `IoQueue` and runs every IO command in order, posts each answer to the loop through
the `EventSink`, and signals the end-of-session restore, so the loop never does file, registry or plan I/O.

## Read first

- `docs/phases/06-integration.md`: notes 1, 2, 4 (the runner table: every IO row) and 5.
- `docs/architecture.md` section 4, "Tray threads" (IO row, end of session); section 6 (file names and formats);
  section 8 (log lines; never log file content).
- `crates/wakeshot-core/src/command.rs` (`Command`, `Outcome`, `FileKind`), `files.rs`.
- The Results of `p3-power-io` (`IoQueue`, `PowerExecutor::run`, the Leftover owner check, `JournalCorrupt`),
  `p3-store` (`DataDir`, `write_atomic`, `remove`), `p3-log` (`Log`, `MemoryLog`), `p4-cli-commands`
  (`open_with_default_app`), `p5-notices` (`Notifier`, `FakeNotifier`; the real `BalloonNotifier` is
  `p6-int-tray-mode`'s to pass in).

## Deliverables

`crates/wakeshot-windows/src/app/io_thread.rs`, owned whole:

1. `IoThread::start(..) -> IoThread`: one named thread (`wakeshot-io`). Inputs: `Arc<IoQueue>`, the executor (generic
   or boxed, as `p3-power-io` built it), `DataDir`, `Box<dyn Notifier>` (or an `Arc`, if `p6-int-tray-mode` must share
   it with the balloon click hook: then `Arc<dyn Notifier + Sync>`; pick what `BalloonNotifier` allows), an opener
   for `OpenLog` (a small trait or `fn`, so tests never open a window), `Arc<dyn Log>`, `Arc<dyn EventSink>`, and the
   end-of-session `Ack`.
2. The runner table of the phase file, note 4, every IO row:
   - `ReadPlan`, `Apply`, `Restore`: `PowerExecutor::run`; post each event it returns, in order.
   - `WriteState`, `WriteConfig`, `WriteHold`, `DeleteHold`: serde to bytes, `store::write_atomic` or `store::remove`
     (a missing file counts as removed); post `Done(Written { file, result })`.
   - `FinishRestoreRequest(state)`: write `state.json`, then remove `restore-request.json`; post one
     `Written { file: FileKind::RestoreRequest }`. A failed state write leaves the request file (the CLI then times
     out and says so), and the result is the failure.
   - `Notify(n)`: `Notifier::show`; post `Done(Notified(result))`.
   - `Log(text)`: `Log::line(Level::Info, "core", &text)`; no answer.
   - `OpenLog`: the opener with the log file path from `DataDir`; a failure is a log line; no answer.
   - Any other command reaching this thread is a wiring bug: log it at error level, never panic.
3. After a `Restore { why: RestoreWhy::SessionEnd, .. }` ran: `Ack::set`.
4. A failed post: one log line; nothing retried (the next safety tick recovers).
5. `IoThread::finish(timeout_ms) -> bool`: runs what is queued, then stops the thread; false when the time ran out
   (the thread is left running and the process exits; the guard covers what is left).

## Acceptance checks

- Tests first, in `app::io_thread::tests`, against `FakeEventSink`, `FakeNotifier`, `MemoryLog`, `FakePlans` (through
  the executor) and a `DataDir` on a temp folder:
  - each command of the table gives exactly the events listed, in queue order, with the right `FileKind`;
  - `FinishRestoreRequest`: `state.json` is written before `restore-request.json` goes; a failed state write keeps
    the request file and answers the failure;
  - `DeleteHold` of a missing file answers `Ok`;
  - a post that fails is logged once and the next command still runs;
  - `Restore { why: SessionEnd }` sets the `Ack`; no other command does;
  - after `IoQueue::session_ending()` queued `Notify` and `Apply` are dropped (the queue's rule) and the restore runs
    first;
  - `finish` returns true with an empty queue within the timeout and false when a scripted executor blocks;
  - a log line never holds file content (check the `MemoryLog` lines of a `WriteConfig`).
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/io_thread.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`; both with `--target aarch64-pc-windows-msvc`: nothing in the owned file.
- `cargo test --locked -p wakeshot-windows -- app::io_thread::` passes.
- Build folder deleted at the end.

## Out of scope

- The loop side (`p6-int-dispatch`), the real objects (`p6-int-tray-mode`), the executor's and the queue's own rules
  (`p3-power-io`), any track file. A missing signature in a track file goes under "Found outside scope".

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items. Include `IoThread`'s public API as built.
