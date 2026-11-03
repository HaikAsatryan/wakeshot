# Integration: the IO thread loop

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens.
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

1. `log.rs` has no public path for `wakeshot.log`, so `io_thread.rs` (for `OpenLog`) and the CLI's `open_log.rs`
   each keep a private copy of the name. Harmless while the name never changes; see "Needs another card".
2. `shell::open_with_default_app` waits up to 10 s (`OPEN_WAIT_MS`). On the IO thread, a shell that hangs holds every
   command behind `OpenLog` that long, a restore too. Kept as the card says (decision 14 below has the trigger).
3. `IoQueue` has no wake or close of its own, so `finish` wakes the thread by pushing `Command::Quit` as a stop marker
   (decision 6). An `IoQueue::close()` would be cleaner; it is a contract change, so only if a second user needs it.
4. Not a problem, for the record: during the 01:26 clippy run, `install.rs` (owned by the running
   `p6-login-and-path`, not this card) failed `clippy::chunks_exact_to_as_chunks`; that card had changed it to
   `as_chunks` by 01:27, and the final clippy runs here had no allowances.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-04 01:16: worker started. Read the card, the phase file notes 1 to 5, architecture sections 4, 6 and 8,
  `command.rs`, `files.rs`, `io_queue.rs`, `executor.rs`, `store.rs`, `log.rs`, `notify.rs`, `shell.rs`, `sink.rs`,
  `app.rs`, the Results of `p6-int-dispatch` and `p6-int-window` (its Found outside scope 1: every post fails with
  1400 after the quit). `BalloonNotifier` does not exist yet (`p5-notices` not done), so the thread takes a
  `Box<dyn Notifier>`. Next: the API and the tests.
- 2026-11-04 01:24: tests first: 17 tests in `app::io_thread::tests` over the API with the thread loop switched off:
  16 of 17 failed (the empty-queue `finish` test passes against a thread that does nothing). With the loop on, 17 of
  17 pass. rustfmt applied to the owned file, `--check` clean. Next: clippy on both targets, release build.
- 2026-11-04 01:28: the card's exact test command failed once: `finish(200)` returned after 196.5 ms by `Instant`
  (2 of 5 runs; a Windows wait can end a few ms early). The test's lower bound is now 150 ms; 10 of 10 runs pass.
  `docs/research.md` "IO thread checks" added.
- 2026-11-04 01:31: added a compile-time check that the shipped executor forms implement `PowerRunner`, and a test
  that `shell::open_with_default_app` fits `Opener` (18 tests). Final checks all pass (Result). Build folder deleted;
  no `wakeshot-io-thread-*` temp folder left. Status done.

## Result

Status: done 2026-11-04. `crates/wakeshot-windows/src/app/io_thread.rs` is filled: the IO thread, its runner for
every IO row of the phase file's note 4, and 18 tests in `app::io_thread::tests` over `FakeEventSink`, `FakeNotifier`
(read through a small shared wrapper), `MemoryLog`, `FakePlans` through the real `PowerExecutor`, the real `IoQueue`
and `Ack`, and a temp data folder. Tests first: 17 written, 16 of 17 failed with the loop switched off. Other files
changed: this card, `docs/research.md` (new section "IO thread checks"). Real laptop: never touched.

### API as built (for `p6-int-tray-mode`)

```rust
pub const FINISH_MS: u32 = 10_000;                 // what the tray gives `finish`; `Drop` waits as long
pub trait PowerRunner: Send { fn run(&mut self, command: &Command) -> Vec<Event>; }
impl<P: PlanIo + Send, N: SafetyNet + Send> PowerRunner for PowerExecutor<P, N> { .. } // real and dry run both fit
pub type Opener = Box<dyn Fn(&OsStr) -> Result<(), WinError> + Send>;
pub struct IoParts {
    pub queue: Arc<IoQueue>, pub power: Box<dyn PowerRunner>, pub dir: DataDir, pub notifier: Box<dyn Notifier>,
    pub open: Opener, pub log: Arc<dyn Log>, pub sink: Arc<dyn EventSink>, pub ack: Ack,
}
impl IoThread {
    pub fn start(parts: IoParts) -> Result<IoThread, WinError>; // thread `wakeshot-io`; Err only if it cannot start
    pub fn finish(self, timeout_ms: u32) -> bool;              // runs the queue, stops; false: time ran out or panic
}
impl Drop for IoThread { .. }                                  // finishes as `finish(FINISH_MS)` does
```

Wiring in `tray_mode`: `power: Box::new(executor)` after the startup restore, `dir` the same `DataDir` as the
executor's, `open: Box::new(shell::open_with_default_app)` (a test checks it fits), `sink: window.sink()`, the same
`Arc<IoQueue>` and an `Ack` clone the dispatcher gets. `BalloonNotifier` does not exist yet: `Box<dyn Notifier>` takes
it as is; if the click hook must share it, wrap an `Arc<BalloonNotifier>` in a local newtype that implements
`Notifier` (that needs `BalloonNotifier: Sync`, `p5-notices`' call).

### How it works

- The thread pops with the longest wait (`u32::MAX` ms), so it never wakes on a timer. `finish` sets a stop flag, then
  pushes `Command::Quit` as a marker and waits on the thread handle (`WaitForSingleObject`, as `WatchHandle::stop`
  does). The thread runs everything before the marker, then anything queued after it, and ends. A `Quit` without the
  flag is a wiring bug like any loop command.
- `ReadPlan`, `Apply`, `Restore`: every event of `PowerExecutor::run`, in order. After a `Restore { why: SessionEnd }`
  (also a failed one): the answer is posted first, then `Ack::set`.
- `WriteState`, `WriteConfig`, `WriteHold`, `DeleteHold`: pretty JSON, `store::write_atomic`; `store::remove` with a
  missing file (or `holds\`) as `Ok`; one `Done(Written)` each. `FinishRestoreRequest`: `state.json`, then the request
  removed; a failed state write keeps the request and answers that failure; a missing request is `Ok`.
- `Notify`: `Done(Notified)`, and a line "notice shown: <name>" or a warning. `Log(text)`: info, area `core`, no
  answer. `OpenLog`: the opener on `<data>\logs\wakeshot.log`; a line either way, no answer.
- Loop commands (`Requests`, `Retake`, `Lock`, `Hibernate`, `Tray`, `Play`, `ApplyUpdate`, `Quit`): one error line
  "wiring bug: ...", dropped. The `match` has no wildcard.
- A failed post: one warning line naming the command, nothing retried. A post that fails with 1400 (the loop window is
  gone, at every quit while `finish` runs) gives one info line for it and every later answer.
- Lines name files and commands, never content: notice names only, hold ids quoted, serde errors never logged.

### Checks (final state, 01:31)

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/io_thread.rs` | clean |
| `cargo clippy --locked -p wakeshot-windows --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | clean |
| the same without `--all-targets` | clean |
| both again with `--target aarch64-pc-windows-msvc` | clean |
| `cargo test --locked -p wakeshot-windows -- app::io_thread::` | 18 passed |
| the `finish` blocking test, alone, 10 runs | 10 passed |
| `cargo build --release --locked -p wakeshot-windows --target x86_64-pc-windows-msvc` | built (ARM64 only through clippy, to spare the battery) |
| Build folder deleted, no test temp folder left | yes |

Acceptance tests, by the card's list: `every_command_answers_in_queue_order` and
`power_events_post_in_the_order_the_executor_gives_them` (events and `FileKind` in queue order, compared with the same
executor run directly); `finish_restore_request_writes_the_state_then_removes_the_request`,
`failed_state_write_keeps_the_restore_request`; `delete_hold_of_a_missing_file_answers_ok`;
`a_failed_post_is_logged_once_and_the_next_command_runs`; `only_a_session_end_restore_sets_the_ack`;
`session_end_runs_the_restore_first_and_drops_notices_and_applies`; `finish_with_an_empty_queue_returns_true_at_once`,
`finish_gives_up_when_the_executor_blocks`; `a_log_line_never_holds_file_content`. More:
`hold_ids_that_are_not_bare_file_names_are_refused`, `posts_after_the_loop_window_went_log_one_line`,
`loop_commands_are_logged_as_wiring_bugs_and_dropped`, `open_log_failure_is_one_log_line`,
`a_notice_that_fails_answers_the_failure`, `dropping_the_thread_runs_its_queue`, `the_real_opener_fits`.

### Decided by the worker, for Haik's review

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | `start` returns `Result<IoThread, WinError>` (the card wrote `-> IoThread`) | Starting a thread can fail; `tray_mode` must log it and exit non-zero (its card) | Never |
| 2 | The inputs come in one struct, `IoParts` | Eight inputs; clippy's limit is seven arguments, and named fields read better at the call | Never |
| 3 | The executor is boxed behind `PowerRunner` | The real and the dry-run executor are two types; one `start` takes both | Never |
| 4 | `Box<dyn Notifier>` | `BalloonNotifier` is not built; a box takes it, or a shared newtype over it | `p5-notices` makes sharing need something else |
| 5 | `finish` wakes the thread with a `Quit` marker plus a stop flag | `IoQueue` has no wake of its own, and `Quit` never comes this way | `IoQueue` gets a `close()` |
| 6 | The idle thread waits with no timeout | No timer wake while idle: Wakeshot should cost no battery when nothing happens | Never |
| 7 | Dropping `IoThread` finishes it (10 s) | An early exit of the tray still runs a queued restore; `WatchHandle` does the same | A drop must not wait |
| 8 | 1400 on a post: one info line, then silence | Every quit hits it while `finish` runs (`p6-int-window` finding 1); the log stays clean | A 1400 shows while the tray still runs |
| 9 | `Ack::set` after the answer is posted, also after a failed restore | The wait is for the restore to end, not to succeed; the loop then finds the answer queued | Never |
| 10 | Logged: notices shown, file and notice failures, the restore request answered, the log opened | Architecture section 8 logs each notice; the core leaves notice and config failures to the runner | The log grows too noisy |
| 11 | Files are written as pretty JSON | As the journal: small files a person may read | Never |
| 12 | A hold id that is not a bare file name is refused (`Written(Err)`, "hold file name check" 123) | A bug can then never write or delete a file outside `holds\` | Never |
| 13 | Own log area `io` | Lines say which part wrote them | Never |
| 14 | `OpenLog` runs on the IO thread | The card asks it; `shell` already caps the wait at 10 s | A log shows `ShellExecuteExW failed with 1460`: then open on a short thread of its own |

None of these makes the laptop behave differently when no work is going on.

### Needs another card

- `log.rs` (`p3-log`, done): add `pub fn log_file(logs_dir: &Path) -> PathBuf { logs_dir.join(LOG_FILE) }` (and
  `use std::path::PathBuf`). Then the `OpenLog` arm here uses `log::log_file(&self.dir.logs())` and drops its private
  `LOG_FILE`, and `wakeshot-cli/src/open_log.rs` `log_path` does the same. Not urgent: both copies say `wakeshot.log`.

### Open items

- None for this card. `p6-int-tray-mode` passes the real objects (API above).
