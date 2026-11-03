# Integration skeleton: the `app` module, its seams and the tray-mode call

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens.
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

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
