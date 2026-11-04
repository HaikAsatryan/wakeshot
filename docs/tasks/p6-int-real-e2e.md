# Integration: the real tray on active plan copies

Phase: 6. Status: in progress. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
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
- 2026-11-04 04:39: started (stop 1: write the suite, compile, lint, record BEFORE; no tray, no write, no run).
- 2026-11-04 05:05: stop 1 done; nothing started, written or run. BEFORE (read only, 04:39:41, again 04:59:46):
  Balanced `381b4222-...` active and the only plan; lid AC 1, DC 1; sleep AC 18000, DC 18000; energy saver AC 0,
  DC 30; RunOnce key with no values. Kept as `<scratchpad>\p6-int-real-e2e\before.txt` (raw `powercfg` output) and
  `before.env` (the record the suite reads, through `WAKESHOT_REAL_E2E_BEFORE`). `tests/e2e_real.rs` written (10
  tests, all `#[ignore]`, all `real_e2e_*`; 1844 lines); `rustfmt --edition 2024 --check` on it exit 0 (run on a
  scratch copy so `e2e/mod.rs` is never rewritten); clippy x64 `-p wakeshot-tray` with and without `--all-targets`
  clean (the first try without failed on `core/lid.rs` of `p2-6-unread-lid-countdown`, clean a minute later); plain
  `cargo test --locked -p wakeshot-tray`: 8 + 5 pass, the 10 new ones ignored. Helpers that belong in `e2e/mod.rs`
  (`RealTray`, `Bench`, the real CLI runner, `terminate`, the real loop window) are private in `e2e_real.rs` for now
  and move to `mod.rs` in stop 2. Decided by the worker, for Haik's review (none changes the product):
  1. The copies are made at Medium through Explorer (`powercfg /duplicatescheme <plan> <id>` with an id the test
     picks, then `/changename`), as a user could (research "powercfg as a normal user"): whether a Medium tray may
     write a copy an elevated process made is not verified. Change it when Medium creation turns out flaky.
  2. Stored values by `powercfg /qh <plan> <subgroup> <setting>` (about 15 ms each here, 420 ms for a whole plan);
     "in force" by `CallNtPowerInformation` `SystemPowerPolicyAc` and `Dc`, read as research "Power track real checks"
     did (lid Do nothing 0, Sleep 2; DC idle 2147483647 for Never).
  3. The user change sets A's lid DC to 2 (Hibernate), not 1: 1 is A's own original (Haik's Sleep), so a restore that
     wrongly wrote the original over the user's change would look the same. Only stored, on a copy, no lid closed.
     Change it when Haik wants the card's literal 1 (one constant).
  4. A test starts only when the PC equals the BEFORE record (and no process of this build or real tray runs), else
     it says to run `real_e2e_cleanup`. Each test makes its own copies and ends with a teardown: every process of this
     build ended before Haik's plan is active again, copies deleted, a RunOnce value of this build removed, data folder
     removed, AFTER compared; a failed test's drop does the same. `real_e2e_cleanup` finds a cut-off run's copies by
     the names A and B and by the ids written to the data folder before `/duplicatescheme` ran; `Wakeshot int haik`
     and `Wakeshot test` copies are never touched.
  5. Restore my power settings: "no new write" is checked for 35 s (one safety tick plus 5 s), the requests by
     `powercfg /requests`, and the next hold must apply again (a new episode).
  6. Plan switch: 15 s (no limit in the card); the switch may start a new guard (the restore of A empties the
     journal), so the test finds the one guard again after it.
  7. End of session: `SendMessageTimeoutW` with `ENDSESSION_LOGOFF` (sign-out, checklist 6), the 5 s limit on the
     `WM_ENDSESSION` send.
  8. Icon entry: one whose `IconGuid` is the product GUID (bytes or text), else a new one naming this build's exe;
     looked for 10 s while the tray runs, then 10 s after its quit.

## Result

What was done, files changed, checks and outcomes, open items.
