# Power IO: the IO queue and the power part of the IO thread

Phase: 3. Status: done. Needs Haik: no. Real laptop: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
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

1. `Event::Started.journal` cannot say "the journal did not read". When the startup's first read of `journal.json`
   fails (logged), `Startup.journal` is `None`, so the core takes nothing as owned: a lid close with our Do nothing
   still in force gets no lock (decision 18) until an apply takes the entries over or the next sign-in's RunOnce
   restores them. Rare (a read failure of a small file in the user's own folder). Proposal for the orchestrator: a
   three-way `journal` in `Started` (none, entries, unread), or a startup retry of the read. Change it when such a
   failure shows up in a log.
2. `p2-ownership-apply` Found outside scope 3 is still open for the core: a routine whose journal load fails answers
   `journal: []`. `run` passes the routines' `journal` through as it is; only `startup` keeps the first read's entries
   (it has no result for the core to see). The core (`p2-core-settings`) must not take `[]` on a failed outcome as
   "nothing owned".
3. `p2-ownership-apply` Found outside scope 4 is unchanged: a journal that turns corrupt while the tray runs is set
   aside only by a `Restore` the core sends. Until then each `ReadPlan` logs three Warn lines (the guard check's load,
   the check's load, the check's failure).
4. For the integration cards, wording only: `p6-int-dispatch` item 5 and the old `io_queue.rs` stub doc have the loop
   call `push_front` with the restore and then `session_ending`; `session_ending` alone now queues it, and an earlier
   `push_front` of the same restore is not doubled. `p6-int-startup` item 5 names `startup_restore(executor,
   liveness)`: `PowerExecutor::startup()` takes no argument (the executor holds its `Liveness`); its item 8
   (`corrupt_left`) is covered by `Startup.corrupt`, so that function only says so, as the card allows.
5. For the core and `p6-int-dispatch`: the queue's own `SessionEnd` restore answers `Done(Restored { why: SessionEnd
   })`, maybe before the core sends its own, so the core gets two such answers, one it did not ask for. A command the
   queue drops or refuses at the end of a session gets no answer: after `EndCancelled` the core must not wait for the
   `Done(Applied)` of a dropped `Apply`.
6. A `Leftover` whose owner stays alive or Unknown is answered failed on every tick with one log line (Info for alive,
   Warn for Unknown), with no end: the card's design. Fine for v1 (one signed-in user per PC); worth a look if a
   report shows the line repeating.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 17:17: reworked: calls the core routines through `power::journal`'s locked entry points.
- 2026-11-03 20:40: worker started. Owns `io_queue.rs`, `power/executor.rs` and this card only.
- 2026-11-03 20:55: read the card's inputs and the Results of `p3-journal`, `p3-power-plan`, `p2-ownership-apply`.
  The core's handlers for `Plan`, `UserChanged` and `JournalCorrupt` are still stubs (`core.rs`), so the events
  follow the contract docs in `event.rs` and `command.rs`. Design picked (reasons in Result): the executor holds its
  `Liveness`, so `startup()` takes no argument; the guard is ensured only for a journal this tray owns (the guard
  restores only its own tray's journal, `p3-recovery`); `ReadPlan` reads the values first, then runs the check.
- 2026-11-03 21:05: `io_queue.rs` filled: 9 tests pass (the card's 7, plus `drop_queued_drops_only_what_it_picks`
  and `session_ending_keeps_one_restore_first`). Next: `power/executor.rs`.
- 2026-11-03 21:30: `power/executor.rs` filled: 15 tests pass (the card's 13, plus
  `failed_entry_read_keeps_the_entry_and_plan_is_ok` and `other_commands_give_no_events`). The build shows 3
  dead-code warnings in `wakeshot-core/src/state.rs` (another card mid-edit). Next: mutants in a scratch copy.
- 2026-11-03 21:45: listing mutants showed three gaps; tests added: a failed set-aside gives no `JournalCorrupt`
  (in `corrupt_journal_gives_event_once`), a journal taken over between the startup's read and the lock (in
  `startup_journal_holds_known_entries_only`), and push and push_front each wake a waiting pop (in
  `queue_works_across_threads`). 24 tests pass. Mutant run started in
  `<scratchpad>\p3-power-io\mutants` (a copy of the workspace; the repo is never mutated).
- 2026-11-03 22:00: mutants M1 to M20 all killed (list in Result). Added `docs/research.md` "Power IO checks".
  Renamed the helper `text` to `error_text` (a local of the same name sat beside it).
- 2026-11-03 22:15: M21, M22 killed (22 of 22). Fixed three clippy lints in test code. All checks pass (Result);
  build folder and scratch copy deleted. Done.

## Result

Done 2026-11-03 22:15. 24 tests (the card's 20 plus 4), all pass in three runs; 22 mutants, all killed. No contract
signature in another file changed. Real laptop: never touched (fakes, temp folders, `Names::test` mutexes; no
registry key, no plan, no RunOnce value).

### API (for `p6-int-io-thread`, `p6-int-dispatch`, `p6-int-startup`, `p3-harness`)

`wakeshot_windows::io_queue::IoQueue` (`Send + Sync`, `Debug`, `Default`; share it as `Arc<IoQueue>`), signatures as
the stub had them:

- `new()`; `push(Command)` last; `push_front(Command)` first; `drop_queued(&dyn Fn(&Command) -> bool) -> usize`;
  `pop(timeout_ms) -> Option<Command>` (each push wakes a waiting pop; `0` returns at once).
- `session_ending()`: exactly one `Restore { plan: None, why: SessionEnd }` queued first (one already queued is moved,
  not doubled), queued `Notify` and `Apply` dropped, new ones refused (by `push` and `push_front`) until
  `session_resumed()`. Everything else is kept. Dropped and refused commands get no answer.

`wakeshot_windows::power::executor`:

- `PowerExecutor::new(plans: P, net: N, me: JournalOwner, names: Names, dir: DataDir, liveness: Arc<dyn Liveness>,
  log: Arc<dyn Log>)`, with `P: PlanIo, N: SafetyNet`. `me` is this tray (pid, start, session), written as owner by
  every apply; `names` the process's mutex names (real or dry run); `dir` the net's data folder. `plans()` and
  `net()` give shared access (the harness reads `CopyAsActive::in_force`, tests read `FileNet::guard()`).
- `run(&mut self, &Command) -> Vec<Event>`: `ReadPlan` gives `Plan(values)` then one `UserChanged` per dropped entry;
  `Apply` gives `Done(Applied)`; `Restore` gives `Done(Restored)`, then `JournalCorrupt` when it set the journal aside
  (the file is then renamed `.seen`). Any other command: no event, no call. Before each of the three: a journal with a
  known entry owned by `me` gets `ensure_guard` (a failed start is a Warn line; the command still runs).
  `Restore { why: Leftover }`: owner `me` or Gone runs with that owner as `expect_owner`; alive answers
  `Err(OWNER_ALIVE)`, Unknown `Err(OWNER_UNKNOWN)` (both code 170), each with the file's known entries; no file
  answers `Ok` with `[]` without taking the lock; a journal taken over before the lock answers `Err(OWNER_ALIVE)`.
- `startup(&mut self) -> Startup { journal: Option<Journal>, corrupt: bool }`: run on the tray's main thread before
  any other thread, then move the executor to the IO thread. Owner Gone: restored at once; alive or Unknown: left
  (`journal` holds the known entries for the core's retry). A corrupt `journal.json` is set aside. `corrupt` is true
  when this start set one aside or found a `journal.json.corrupt` left by others; either is renamed `.seen` already,
  so the tray posts `JournalCorrupt` once after `Started` (after `Session(Locked)` when that comes).
- Compile-time check: `PowerExecutor<Win32Power, FileNet<NoGuard>>` and the `DryRunPower` form are `Send`.
  `FileNet<TrayGuard>` must stay `Send` too (`p3-recovery`).

### Design choices

| # | Choice | Reason | Change it when |
|---|---|---|---|
| 1 | The executor holds its `Liveness`; `startup()` takes no argument | `run`'s `Leftover` check needs it too; one source | Startup needs another liveness than the IO thread |
| 2 | A guard only for a journal `me` owns | The guard restores only a journal naming its tray (`p3-recovery`), so one for another owner's leftover never acts | The guard restores any dead owner's journal |
| 3 | The guard check's read and the `Leftover` read are plain loads, outside the lock | Every write is atomic; the owner goes to the routine as `expect_owner`, so a change before the lock is caught | The extra load per command shows a cost (finding 3) |
| 4 | `ReadPlan`: values first, then the check; the check runs also when the values fail | A change the values show was there for the check, so its `UserChanged` follows; a dropped entry must reach the core | The core wants `UserChanged` before `Plan` |
| 5 | `Leftover` with no file answers without the routine; corrupt runs it with no owner; unreadable answers `Err` with `[]` | A journal written in between has an owner nobody checked; a set-aside writes no setting | Never likely |
| 6 | Other restore reasons ignore the owner | The card limits the check to `Leftover`; the core sends the others for a restore it wants now | Two live trays per user become possible |
| 7 | `OWNER_ALIVE`, `OWNER_UNKNOWN`: code 170 (`ERROR_BUSY`), the reason in `call` | The core branches on codes only where the design names them | The core must tell them apart |
| 8 | `Startup.journal` keeps the first read's entries when the routine could not read the file | `Started` carries no result, so `[]` would read as "nothing owned" and never be retried | `Started` gets a result |
| 9 | A failed `.seen` rename counts as reported and is logged | A repeated notice rather than none | A report shows a repeated notice |
| 10 | `push` keeps its signature (no return value) | The core sends no `Apply` after `Session(Ending)`; the refusal is a safety net | The loop must know a command was refused |

### Files changed

`crates/wakeshot-windows/src/io_queue.rs` (filled, 9 tests), `crates/wakeshot-windows/src/power/executor.rs`
(filled, 15 tests), `docs/research.md` (section "Power IO checks", added at the end), this card.

### Checks (2026-11-03, `CARGO_TARGET_DIR` = this card's folder, no `RUSTFLAGS`)

- `rustfmt --edition 2024 --check` on both owned files: exit 0 (neither declares a child module).
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0 (22:10). The first run
  flagged three lints in my test code (two complex tuple types, an index loop); fixed.
- `cargo test --locked -p wakeshot-windows -- io_queue:: power::executor::`: 24 passed, 0 failed, three runs.
  - `io_queue::tests`: `queue_keeps_order`, `push_front_runs_next`, `session_ending_drops_notices_and_applies`,
    `session_ending_refuses_new_applies`, `session_resumed_accepts_again`, `pop_times_out`,
    `queue_works_across_threads`; added `drop_queued_drops_only_what_it_picks`, `session_ending_keeps_one_restore_first`.
  - `power::executor::tests`: `read_plan_events_in_order`, `outcomes_carry_the_journal`,
    `failed_plan_read_is_plan_err`, `corrupt_journal_gives_event_once`, `leftover_runs_only_for_gone_or_own_owner`,
    `ensures_guard_while_owned`, `no_guard_when_nothing_owned`, `startup_restores_gone_owner`,
    `startup_leaves_unknown_owner`, `startup_leaves_live_owner`, `startup_reports_corrupt_left_by_restore_mode`,
    `startup_journal_holds_known_entries_only`, `startup_stop_points`; added
    `failed_entry_read_keeps_the_entry_and_plan_is_ok` (track note 10), `other_commands_give_no_events`.
  - `startup_stop_points`: three starting states (three entries plus a newer version's; marked entries with ours in
    force; one entry alone on the active plan plus one on an inactive plan). A stop at every call of `startup`, then
    the next start: track note 4's first point after the stop, its second after the next start (RunOnce may stay only
    when the journal was deleted before the stop, walk R6), and the journal mutex free for another thread after every
    stop. Then, per stop, the next start failing at each of its calls in turn: the first point still holds,
    `Startup.journal` equals the file's known entries (`None` only when the first read failed), and a clean start
    after it restores everything. The newer version's entry stays in the file in every case.
- Mutants, in a scratch copy of the workspace (the repo never mutated), each reverted: 22 of 22 killed. Executor:
  leftover runs for any owner; guard for any owner; no guard before a command; check before values; no check when
  values fail; no `JournalCorrupt`; `JournalCorrupt` on a failed move; startup restores an Unknown owner; startup
  keeps no first read (killed by two tests, the stop walk among them); startup marks nothing seen; leftover passes no
  expected owner; a taken-over journal answered as done; a missing leftover runs the routine; a leftover not run
  answers `[]`; startup passes no expected owner; startup leaves a corrupt journal. Queue: refuses nothing while
  ending; `session_resumed` lifts nothing; `push_front` pushes last; a push wakes nobody; `session_ending` drops no
  applies; `session_ending` stacks restores. Three of these needed tests I added after listing them (Log 21:45).
- After the tests: no `%TEMP%\wakeshot-power-io-*` folder left. Build folder and scratch copy deleted.

### Open items

- The six findings under "Found outside scope", the first two for the core and the orchestrator.
- The power safety review covers both files (`docs/phases/02-06-lighter.md`, "Reviews").
- Not tested here: the executor on real plans and the real guard (`p3-harness`, `p3-real-checks`, `p3-recovery`).
