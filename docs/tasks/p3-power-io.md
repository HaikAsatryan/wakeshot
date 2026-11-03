# Power IO: the IO queue and the power part of the IO thread

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
Runs after: `p3-journal`, `p3-power-plan`, and the core track's `p2-ownership-apply` (the routines). Parallel with:
`p3-recovery`, `p3-stop-points` and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-power-io\target`.

## Goal

Fill `io_queue` (the typed queue between the loop and the IO thread, with the end-of-session rule) and
`power::executor` (runs `ReadPlan`, `Apply` and `Restore` through the locked core routines and turns each into the
events the core expects), plus the tray startup restore.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 5 to 8, 10, 11).
- `docs/architecture.md`: section 4 "Tray threads" (IO thread, end of session), "Tray startup" step 6, section 3
  (`Event::Plan`, `UserChanged`, `Done`), section 5 "Guard" (a new guard when one ends while anything is owned).
- `docs/reviews/p1-contracts-domain.md`: M1 (IO queue), M4 (corrupt journal), m5 (leftover journal).
- `docs/phases/02-core.md` notes 8 and 9 (corrupt journal, `Restore { why: Leftover }`).
- After the contracts card and the cards above: `io_queue.rs` (stub: `IoQueue`), `power/executor.rs` (empty),
  `power/journal.rs` (locked entry points), `power/plan.rs` (`values()`), the core's `ownership` API and `FakePlans`.

## Deliverables

- `crates/wakeshot-windows/src/io_queue.rs`: `IoQueue` (`Send + Sync`, a mutex and a condition variable): `push`,
  `push_front`, `drop_queued(keep)`, `pop(timeout_ms)`; `session_ending()` puts `Restore { plan: None, why:
  SessionEnd }` at the front and drops queued `Notify` and `Apply` commands, and refuses new `Apply` and `Notify`
  pushes; `session_resumed()` lifts that (section 4).
- `crates/wakeshot-windows/src/power/executor.rs`:
  - `PowerExecutor::run(&Command) -> Vec<Event>`: `ReadPlan` gives `Plan(values)` then one `UserChanged` per key the
    ownership check dropped (track note 10 for read errors); `Apply` gives `Done(Applied)`; `Restore` gives
    `Done(Restored)`, plus `JournalCorrupt` when the journal was set aside (then `mark_corrupt_seen`, track note 7).
    `Restore { why: Leftover }` runs only when the journal's owner is Gone or this tray; otherwise it answers a failed
    `Restored` so the core retries on the tick (core note 9). Any other command: no events (not its job).
  - Before each command: if the journal holds a known entry, `ensure_guard` (a guard that ended is replaced, section
    5 "Guard").
  - `startup(liveness) -> Startup`: tray startup step 6 (owner Gone: restore at once; Unknown or alive: leave it, the
    core's `Leftover` retry covers it); a `journal.json.corrupt` left by another process: report it once and mark it
    seen. `Startup.journal` is the known entries after that, `None` when there are none (the `Started` contract).

## Acceptance checks

Tests in `io_queue::tests` and `power::executor::tests`, against the core's `FakePlans`, `FakeLiveness`, `MemoryLog`:

- `queue_keeps_order`, `push_front_runs_next`, `session_ending_drops_notices_and_applies`,
  `session_ending_refuses_new_applies`, `session_resumed_accepts_again`, `pop_times_out`, `queue_works_across_threads`.
- `read_plan_events_in_order` (`Plan` before `UserChanged`), `outcomes_carry_the_journal`,
  `failed_plan_read_is_plan_err`, `corrupt_journal_gives_event_once`, `leftover_runs_only_for_gone_or_own_owner`,
  `ensures_guard_while_owned`, `no_guard_when_nothing_owned`.
- `startup_restores_gone_owner`, `startup_leaves_unknown_owner`, `startup_leaves_live_owner`,
  `startup_reports_corrupt_left_by_restore_mode`, `startup_journal_holds_known_entries_only`.
- Stop-point test `startup_stop_points`: with `FakePlans`' script, stop at every call of `startup`, run it again, check
  track note 4.

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows -- io_queue:: power::executor::`. Record each
outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The IO thread loop itself, the file commands (`WriteState`, `FinishRestoreRequest` and others), `Command::Log`,
  notices (integration and tracks 4, 5); the loop's timers; the routines (core).
- Any file but `io_queue.rs` and `power/executor.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: calls the core routines through `power::journal`'s locked entry points.

## Result

What was done, files changed, checks and outcomes, open items.
