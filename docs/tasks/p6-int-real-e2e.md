# Integration: the real tray on active plan copies

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: **global lane** (power settings, one card at a time across all tracks): two plan copies made active in
turn, the real RunOnce value, a real guard process. Haik's own plan is never written; his values are read before and
after and must match. No sign-out, no sleep, no lid. Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p6-int-real-e2e\target`.
Runs after: `p6-int-dry-run-e2e` (takes over `tests/e2e/mod.rs`), `p3-real-checks` (50 apply and restore cycles on an
active copy passed first). Parallel with: code cards that own none of the files below; no other laptop card.

## Goal

Run the real `wakeshot-tray.exe` (not dry run) with real holds and prove, on throwaway active plan copies, every path
that writes or restores settings: apply at hold start, restore at hold end, a user change, a plan switch (checklist 9),
"Restore my power settings" through the CLI, quit, end of session, a killed tray and its guard, and the CLI restore
with a dead owner.

## Read first

- `docs/phases/06-integration.md`: notes 3, 6 and 7; the checklist map (rows 4, 6, 9).
- `docs/architecture.md` section 5 (apply, ownership, plan switch, restore routine, guard, logon restore) and the
  section 9 plan copy rule (read the real plan before and after; change nothing by hand during the test).
- `CLAUDE.md`, "Test laptop" (Haik's values and plan id).
- The Results of `p3-real-checks` (how it made a copy active and put everything back), `p3-harness` (`cleanup`,
  `end-session`), `p6-int-dry-run-e2e` (the helpers).

## Deliverables

1. `crates/wakeshot-tray/tests/e2e_real.rs`, owned whole, every test `#[ignore]` and named `real_e2e_*`, run with
   `--test e2e_real -- --ignored --test-threads=1`. The tray is a debug build started at Medium with
   `WAKESHOT_DATA_DIR` on a temp folder. Plan copies `Wakeshot int A` and `Wakeshot int B` (`powercfg
   /duplicatescheme` of Haik's active plan), values read with `powercfg /q`.
   - `real_e2e_cleanup` (first and last): Haik's recorded plan active, every `Wakeshot int` plan deleted, a
     `WakeshotRestore` RunOnce value that points into this card's target folder deleted, no tray or guard from this
     card's target folder left (by pid), and Haik's values equal to the BEFORE record (kept in the scratchpad). It is
     also the step the orchestrator runs if this card is cut off.
   - hold start and end: Keep awake on (CLI) gives on A, within 10 s, lid AC 0, lid DC 0, sleep DC 0 (stored and in
     force), a journal with three entries owned by this tray, the RunOnce value, a guard running; Keep awake off gives
     the original values, no journal, no RunOnce value, the guard gone within 15 s.
   - a user change: during a hold, `powercfg` sets A's lid DC to 1 and re-applies A; within 20 s the journal drops
     that entry, the log shows `UserChanged`, and at hold end lid DC stays 1 (then the test puts A's original back).
   - plan switch (checklist 9): during a hold, `powercfg /setactive` B; A gets its originals back, B gets the three
     writes; at hold end B is restored.
   - Restore my power settings: during a hold, `wakeshot restore`: A restored, the hold still on (requests only, the
     log says so), no new write until the hold ends.
   - quit: during a hold, `ask_quit`: restored, the Keep awake hold file gone, exit code 0, the guard gone.
   - end of session: during a hold, `WM_QUERYENDSESSION` then `WM_ENDSESSION` TRUE sent to the loop window (the
     harness's `end-session` or `SendMessageTimeoutW`); the send returns within 5 s with A restored; then the test
     ends the tray by pid, as Windows would.
   - killed tray (checklist 4, the agent's part): during a hold, the tray ended by pid; the guard restores A within
     15 s and clears RunOnce.
   - CLI restore, owner dead: a hand-written journal (`original` A's value, `written` 0, owner a process this test
     started and ended) with A's lid DC set to 0; no tray running; `wakeshot restore` restores it and prints so.
   - the product GUID's `NotifyIconSettings` entry exists after the first start; record its path value (left in
     place: phase file note 7).
2. Fixes the run needs, only in the `app/*` files and `tests/e2e/mod.rs`; anything else under "Found outside scope".
3. `docs/research.md`: a section "Integration real e2e" with each result and how it was checked.

## Acceptance checks

- BEFORE recorded: Haik's active plan id; lid AC and DC, sleep AC and DC, energy saver DC (`powercfg /q`); `powercfg
  /l`; the RunOnce key's values. AFTER equals BEFORE; no `Wakeshot int` or `Wakeshot test` plan left; no process this
  card started is left.
- Tests first: the suite is written before any fix; at the end every `real_e2e_` test of `e2e_real` passes (output in
  the Log).
- `rustfmt --edition 2024 --check` on the owned files and any fixed file; `cargo clippy --locked -p wakeshot-tray
  --all-targets --target <t> -- -D warnings` and without, both targets; plain `cargo test --locked -p wakeshot-tray`
  passes.
- Build folder deleted at the end.

## Out of scope

- Haik's own plan (never active, never written); sign-out, sleep, lid, battery (Phase 3 and `p6-int-haik-checks`);
  installs (`p6-int-update-e2e`); any track file.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
