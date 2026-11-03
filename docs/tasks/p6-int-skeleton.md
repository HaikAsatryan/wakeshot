# Integration skeleton: the `app` module, its seams and the tray-mode call

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens.
Real laptop: no. **Runs alone**: it edits a contract module list (`wakeshot-windows/src/lib.rs`) and the tray's
`main.rs`, so no other card and no cargo command runs beside it (`docs/plan.md`, "Git": contracts change only through
a card the orchestrator runs alone). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-skeleton\target`.
Runs after: `p1-close-contracts`. Parallel with: nothing.

## Goal

Create every integration file with its owner line, the seams the integration cards share, and the tray-mode call in
`main.rs`, so the integration cards never edit a module list, a contract file or each other's files.

## Read first

- `docs/phases/06-integration.md`: notes 1 (owner table), 2 (the seams), 3 and 4.
- `crates/wakeshot-windows/src/lib.rs`, `sink.rs`, `io_queue.rs`; `crates/wakeshot-tray/src/main.rs`, `mode.rs`.
- `crates/wakeshot-core/src/event.rs` (`Event`, `SessionChange`).

## Deliverables

1. `crates/wakeshot-windows/src/lib.rs`: add `pub mod app;` (first in the list). Nothing else changes there.
2. `crates/wakeshot-windows/src/app.rs`: module doc (owner: this card; frozen after it), `pub mod dispatch;`,
   `pub mod io_thread;`, `pub mod startup;`, `pub mod tray_mode;`, `pub mod window;`, and the seams of the phase file's
   note 2, with these doc lines:
   - `Flow { Go, Quit }`.
   - trait `LoopHandler` with `event(&mut self, Event) -> Flow`, `session(&mut self, SessionChange) -> Flow`,
     `timer(&mut self) -> Flow`, `next_timer_ms(&self) -> u32`. Doc: called on the loop thread only; a call can
     re-enter when a runner pumps messages, and the handler then queues the event (phase file note 4); `session`
     returns only after the end-of-session wait.
   - `Ack`: `new`, `set`, `reset`, `wait(timeout_ms) -> bool` over `Arc<(Mutex<bool>, Condvar)>`, `Clone`, `Send`,
     `Sync`; a poisoned lock counts as set (a panicking IO thread aborts the process anyway). Built here, with tests.
3. Five module files, each only a module doc whose first line says what it holds and whose owner line names its card:
   `app/window.rs` (`p6-int-window`), `app/dispatch.rs` (`p6-int-dispatch`), `app/io_thread.rs` (`p6-int-io-thread`),
   `app/startup.rs` (`p6-int-startup`), `app/tray_mode.rs` (`p6-int-tray-mode`, then `p6-int-update-wiring`). Only
   `tray_mode.rs` has code: `pub fn run(dry_run: bool) -> std::process::ExitCode` returning `ExitCode::SUCCESS`, doc:
   tray startup steps 3 to 9 (phase file note 6).
4. `crates/wakeshot-tray/src/main.rs`: the arm `Some(Mode::Tray | Mode::DryRun)` calls
   `wakeshot_windows::app::tray_mode::run(matches!(m, Mode::DryRun))` (bind the mode); its comment and the file's doc
   line say the integration cards own the file from now on (`p6-int-skeleton`, then `p6-int-update-wiring`).
5. No `Cargo.toml` and no `Cargo.lock` change; no new `windows` feature.
6. `docs/research.md`: nothing unless a fact was checked.
7. Added by the orchestrator (2026-11-03): `docs/architecture.md` gets the five design changes in
   `docs/tasks/p6-plan-integration.md` Result, "Design changes proposed for `architecture.md`". Re-read each spot
   first (`p1-close-contracts` edited the file just before this card).
8. Added by the orchestrator: the stale lines in `p6-plan-integration` "Found outside scope" 1 to 3:
   `docs/phases/02-core.md` (the IO queue is `p3-power-io`'s; integration only uses it), `docs/phases/05-tray-ux.md`
   (checklist 16 maps to integration), and `docs/tasks/p0-architecture-review.md` Status (done: Phase 0 is signed off).
   Change only those lines.
9. Added by the orchestrator, from `docs/reviews/p1-close-verify.md`: (a) Major 1: one rely-on line in the `Event`
   doc (`crates/wakeshot-core/src/event.rs`, `Started`): a tray that starts while the session is locked gets
   `Session(Locked)` right after `Started`, before `JournalCorrupt` and `Updated`; no shape change. (b) Minor 1: owner
   lines name the integration hand-overs: "then `p6-int-update-wiring`; fixes by the e2e cards" in
   `crates/wakeshot-tray/src/velo.rs`, "fixes by `p6-int-dry-run-e2e`" in the seven CLI command files (`args.rs`,
   `run.rs`, `keep_awake.rs`, `automatic.rs`, `status.rs`, `restore.rs`, `open_log.rs`), "fixes by the e2e cards" in
   the `app/*` owner lines you write. (c) Minor 2: `crates/wakeshot-cli/src/main.rs` gets "Owner: contracts card."
   in its module doc; the tray `main.rs` gets its `Owner:` line through deliverable 4. Doc lines only.
10. Added by the orchestrator: stale `architecture.md` lines that wave 1 cards found (each card's "Found outside
    scope"; change only those lines, from what the card's Result shows): `p3-log` (section 8 still calls the no-mix
    result of several writers "not verified"; `docs/research.md` "Log writer checks" has it); `p3-process` (section
    5 still lists "Medium may use Explorer as parent" as not verified; `docs/research.md` "Power track plan checks"
    has an out-of-date line on `Wdk_*` features; the card's Result has both facts); `p2-ownership-restore` (section
    5 restore step 5: replace it with the proposed text in that card's "Found outside scope" 1, which the code
    follows; `docs/phases/03-windows-power.md` note 4: "RunOnce is gone; the guard is told to stop" only for stops
    before the journal delete, as its item 2 says); `p3-store` (section 6: `write_atomic` falls back to a rename by
    handle with POSIX semantics when `MoveFileExW` gives error 5 because a reader has the target open; its Result).

## Acceptance checks

Run alone, so whole-workspace checks are right here:

- Tests first: `app::tests` for `Ack` (set before wait returns true at once; wait times out false; set from another
  thread wakes a waiter; reset clears). `cargo test --locked -p wakeshot-windows -- app::tests` passes.
- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` and the same without
  `--all-targets`; both again with `--target aarch64-pc-windows-msvc` (if `velopack` is in by then and clang is
  missing on this laptop, the tray's ARM64 runs are CI only: record it).
- `cargo test --workspace --locked` passes (other cards' `unimplemented!` stubs are not called by any test).
- `cargo build --release --workspace --locked --target x86_64-pc-windows-msvc`.
- `git` is not run; `Cargo.lock` unchanged (check its hash before and after).
- No em or en dashes; no new line over 120 characters outside tables.

## Out of scope

- Any body beyond `Ack` and the `tray_mode::run` stub. Any track file, except the two phase file lines in
  deliverable 8 and the doc lines in deliverable 9. In `crates/wakeshot-cli/`, only the doc lines of deliverable 9.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. `CLAUDE.md` is stale: "Status: Phase 1 (detailed design and skeleton)" although Phase 1 was signed off at 19:13;
   "skeleton only, no product logic yet" although wave 1 filled `log.rs`, `store.rs`, `process.rs` and the restore
   routine; "each file with one owner named in its first doc line" where the owner is in the `Owner:` line. The
   orchestrator's file.
2. `docs/research.md` "Power track plan checks" still says `Win32_System_Time` "is not declared" (`p3-log` Found
   outside scope 2). `p3-log` added a correcting line in "Log writer checks"; the old line was not in this card's
   list, so it stays.
3. `p3-store` Found outside scope 2 (temp files `<name>.<pid>.<n>.tmp` left by a stopped process are never removed;
   proposal: the tray deletes those of dead pids at start) has no card yet. It belongs to `p6-int-startup` if taken.
4. For `p6-int-dispatch`: the `LoopHandler::session` doc now says it returns only after the end-of-session wait also
   when the call re-enters (design pick 1 below). A re-entered `Ending` cannot feed the core, so the dispatcher must
   push the restore itself (`IoQueue::push_front` plus `session_ending`, as the `io_queue.rs` doc describes), queue the
   change and still wait on the `Ack`.
5. `architecture.md` section 4 step 8 lists "watchers, first reads, then `Started`", while phase note 6 starts
   `WatchThread` after `Dispatcher::new` fed `Started`. Same effect (events posted before the pump wait in the queue,
   so the core sees `Started` first); the wording differs. Not in the proposals, so left.
6. Old lines over 120 characters, not mine: `architecture.md` 372 and 385, `03-windows-power.md` 36, and eleven in
   `research.md`. Harmless; left.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-03: started (run alone after wave 1). Read the card, the phase file notes 1 to 6 and plan.md "How we work".
- 2026-11-03: `Cargo.lock` SHA-256 before: `384FD119240A8ADC49056A2DCE0A72A213D9571E6AC6CE14130C6D96F9248894`.
- 2026-11-03: wrote `app.rs` (module list, `Flow`, `LoopHandler`, `Ack` with 5 tests), the five `app/*` files,
  `pub mod app;` in `lib.rs`, the tray `main.rs` arm. Next: first test run.
- 2026-11-03: `cargo test --locked -p wakeshot-windows -- app::tests`: 5 passed. Deliverable 9 doc lines written
  (`event.rs` rely-on line; owner lines in `velo.rs`, the seven CLI files and the CLI `main.rs`). Next: docs.
- 2026-11-03: docs done: `architecture.md` (the five proposals; step 8 also names `Session(Locked)`, as phase note 6
  and the `event.rs` line now do; the "every module from `p1-contracts-split`" line names `app`), deliverable 10's
  lines (section 5 guard and restore step 5, section 6 writes, section 8 log), `03-windows-power.md` note 4,
  `research.md` `Wdk_*` line, `02-core.md`, `05-tray-ux.md`, `p0-architecture-review.md` Status (its review is
  `docs/reviews/p0-architecture.md`). Next: mutants on `Ack`, then the whole-workspace checks.
- 2026-11-03: four mutants in `app.rs` by a scratch script (file restored and compared after): no `notify_all`,
  `reset` that sets, poison read as unset each failed one test; "the timeout decides, not the value" survived (a set
  at the exact timeout is not testable on purpose).
- 2026-11-03: `cargo fmt --all --check` failed once on my own `wait` line; rewritten with a local `timeout`. Then every
  whole-workspace check passed (Result). Fixed two of my own lines over 120 in `architecture.md`. Release tray exe
  run by hand: `--dry-run` exit 0, no arguments exit 0, `--nope` exit 2. Build folder and scratch scripts deleted.
  Done.

## Result

Status: done.

What was done:

- `crates/wakeshot-windows/src/app.rs`: module doc with the owner and freeze rule; `pub mod dispatch, io_thread,
  startup, tray_mode, window`; `Flow { Go, Quit }`; trait `LoopHandler` (`event`, `session`, `timer`,
  `next_timer_ms`) with the loop-thread, re-entry and end-of-session-wait doc; `Ack` (`new`, `set`, `reset`,
  `wait(timeout_ms) -> bool`) over `Arc<(Mutex<bool>, Condvar)>`, `Clone`, `Debug`, `Default`, with a compile-time
  `Send + Sync` check and 5 tests.
- Five `app/*.rs` files, doc only, each first line saying what it holds and an `Owner:` line naming its card plus
  "fixes by the e2e cards". `tray_mode.rs` also has `pub fn run(dry_run: bool) -> ExitCode` returning `SUCCESS`, the
  old tray behavior.
- `lib.rs`: `pub mod app;` first. Tray `main.rs`: the arm `Some(m @ (Mode::Tray | Mode::DryRun))` calls
  `wakeshot_windows::app::tray_mode::run(matches!(m, Mode::DryRun))`; the arm comment and an `Owner:` line name the
  integration cards.
- Deliverable 9: `event.rs` rely-on line (`Session(Locked)` right after `Started` when locked at start, before
  `JournalCorrupt` and `Updated`; no shape change); owner hand-overs in `velo.rs` and the seven CLI command files;
  "Owner: contracts card." in the CLI `main.rs`.
- Deliverables 7, 8 and 10: the lines listed in the Log, nothing else.

Design picks (mine):

| Pick | Why | Change it when |
|---|---|---|
| 1. `session` waits also when re-entered | Windows may end the process once `WM_ENDSESSION` is answered (architecture section 4); a queued `Ending` would skip the restore | `p6-int-dispatch` shows the wait cannot run re-entered: then the window defers its answer instead |
| 2. A poisoned lock counts as set, also after `reset` | One simple rule; a panic aborts the tray anyway, so the waiter must not block for nothing | A panic no longer aborts the tray |
| 3. `wait` returns the value read last, not "not timed out" | A set landing at the timeout still counts | Never |
| 4. Step 8 in `architecture.md` also names `Session(Locked)` | Proposal 4 predates review Major 1; without it the doc contradicts `event.rs` and phase note 6 | Never; it follows the contract |
| 5. Restore step 5: the proposed text, plus the old pointer to the two points under "Plan switch" | The pointer says why other plans need no apply | Never |

Files changed: `crates/wakeshot-windows/src/lib.rs` and `app.rs`; new `crates/wakeshot-windows/src/app/window.rs`,
`dispatch.rs`, `io_thread.rs`, `startup.rs`, `tray_mode.rs`; `crates/wakeshot-tray/src/main.rs` and `velo.rs`;
`crates/wakeshot-core/src/event.rs`; `crates/wakeshot-cli/src/main.rs`, `args.rs`, `run.rs`, `keep_awake.rs`,
`automatic.rs`, `status.rs`, `restore.rs`, `open_log.rs`; `docs/architecture.md`, `docs/research.md` (one line),
`docs/phases/02-core.md`, `03-windows-power.md`, `05-tray-ux.md`; `docs/tasks/p0-architecture-review.md` (Status);
this card.

Checks (2026-11-03, alone, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`; final run after the last edit):

- `cargo test --locked -p wakeshot-windows -- app::tests`: 5 passed (`set_before_wait_returns_true_at_once`,
  `wait_times_out_false`, `set_from_another_thread_wakes_a_waiter`, `reset_clears`, `poisoned_lock_counts_as_set`).
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0. `velopack` is not in
  yet, so ARM64 needed no clang.
- `cargo test --workspace --locked`: exit 0. `wakeshot-core` 63 passed; `wakeshot-windows` 55 passed, 3 ignored;
  `wakeshot` (CLI) 1 passed; `wakeshot-tray` 1 passed; doc tests 0. No test process left (checked by path).
- `cargo build --release --workspace --locked --target x86_64-pc-windows-msvc`: exit 0.
- `Cargo.lock` SHA-256 before and after: `384FD119240A8ADC49056A2DCE0A72A213D9571E6AC6CE14130C6D96F9248894`. No
  `Cargo.toml` change, no new `windows` feature. `git` not run.
- No em or en dash in any changed file; no new line over 120 characters outside tables (script over every changed
  file; old long lines in Found outside scope 6).
- Wave 1 close: no whole-workspace check failed in any file.

Open items: Found outside scope 1 to 5 for the orchestrator to route.
