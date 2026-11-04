# Integration: the real tray on active plan copies

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
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

1. The dry-run icon's keys are back. This card ran the dry-run suite once (05:21) to check the shared `mod.rs`.
   That run made the keys `p6-int-dry-run-e2e` had deleted at 04:46:
   - `HKCU\Control Panel\NotifyIconSettings\15983757285637097514` (only `IconSnapshot`);
   - `NotifyIconGeneratedAumid_15983757285637097514` under `HKCU\Software\Classes\AppUserModelId` (`DisplayName`
     "Wakeshot") and under `...\CurrentVersion\Notifications\Settings`.

   Left in place, decided by the worker, for Haik's review: the dry-run card still has its pictures to take with a
   dry-run tray, and its export-and-delete clean-up (phase file note 7) covers these keys after that. Change it when
   the orchestrator says no dry-run tray runs again; then export and delete them as that card's Log describes.

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
- 2026-11-04 05:07: stop 2 started (suite committed in 0932c60; `e2e/mod.rs` handed over; the laptop lane is this
  card's alone). No `wakeshot-tray.exe` or `wakeshot.exe` runs; Balanced active and the only plan.
- 2026-11-04 05:14: helpers moved into `tests/e2e/mod.rs` as a generalization, not as copies: `TempData::real`
  (data in the folder itself, CLI and tray without `--dry-run`), `Tray` for both kinds (loop window class and
  `ask_quit` by kind), `journal()`, `launch`, `assert_medium`, `terminate`, `exit_code_within`, `loop_window`,
  `full_path`, `wait_for` made public; `#![allow(dead_code)]` in `mod.rs`, since each suite uses part of it. The dry
  run's API is unchanged. Power-test logic (BEFORE record, copies, `Bench`) stays in `e2e_real.rs`. Log matches on
  `power/journal.rs` text (`p6-int-log-wording` may reword it) are now print-only: apply and ownership lines.
  rustfmt and clippy x64 both ways clean; `wakeshot-cli` built x64 into the target folder.
- 2026-11-04 05:14 to 05:20: runs with `WAKESHOT_REAL_E2E_BEFORE` = `before.env`, output in
  `<scratchpad>\p6-int-real-e2e\out\`. `real_e2e_cleanup` first: nothing to do, AFTER equals BEFORE. Hold test alone,
  try 1: test bug (`powercfg /duplicatescheme` ends its line without a newline, so the `.cmd`'s exit marker shared
  the line and was not counted); its teardown deleted the copy, nothing left. Fixed (marker `[exit N]` found anywhere);
  try 2 passed. Suite run 1 (05:16 to 05:17): 9 passed, 1 failed: the user-change test waited for a "UserChanged"
  notice line, but the core holds notices while the session is locked and shows them at the unlock (`Core::notify`;
  Haik's session was locked). Test bug: the notice line is now required only when `session::locked_now()` is
  `Some(false)`. Suite run 2 (05:18:53 to 05:20:25): **10 passed, 0 failed** in 92.3 s ("session locked: Some(true)").
  `real_e2e_cleanup` last: nothing to do, AFTER equals BEFORE. No product fix was needed.
- 2026-11-04 05:21 to 05:24: the dry-run suite run once as a check of the shared `mod.rs` (dry run only, nothing
  written): 12 of 12 passed in 38.3 s (`out\dry-run-suite.txt`). Then rustfmt check on both suites, clippy x64 both
  ways and plain `cargo test --locked -p wakeshot-tray` (8 + 5 pass, 25 ignored): all clean. Research section
  "Integration real e2e" added. After: no `wakeshot-tray.exe`, `wakeshot.exe` or `PING.EXE` runs; no
  `%TEMP%\wakeshot-e2e-*` folder; no `%LocalAppData%\Wakeshot`; Balanced the only plan and active; no Wakeshot value
  in Run or RunOnce. Build folder kept for the final record run, on the orchestrator's word. Decided by the worker,
  for Haik's review:
  9. `#![allow(dead_code)]` at the top of `tests/e2e/mod.rs`: each suite compiles the module on its own and uses
     part of it. Change it when a helper is unused by both suites: then delete the helper.
  10. The `UserChanged` notice line is required only in an unlocked session (the core holds notices while locked).
  11. The dry-run icon keys this card's check run made again are left for the dry-run card (Found outside scope 1).
- 2026-11-04 05:25: the orchestrator says no final record run is needed. Build folder, run outputs (`out\`) and
  scratch scripts deleted; only `before.txt` and `before.env` kept in `<scratchpad>\p6-int-real-e2e\`.

## Result

Status: done (2026-11-04 05:24), with one run still to come: the orchestrator asked for a final full suite run as the
record once `p2-6-unread-lid-countdown` and `p6-int-log-wording` end. The build folder is kept for it and deleted
after that.

The real tray works on an active plan for every path the card names, and no product fix was needed. Suite run 2:
10 of 10 passed. Run 1 had 9 of 10; the one failure was a test bug (a notice held in a locked session). Haik's plan
was never active while a tray ran and never written. AFTER equals BEFORE: Balanced active and the only plan; lid AC 1
and DC 1; sleep AC 18000 and DC 18000; energy saver AC 0 and DC 30; no RunOnce value.

### Files changed

- `crates/wakeshot-tray/tests/e2e_real.rs`: the suite (10 tests, all `#[ignore]`, all `real_e2e_*`). Plan copies
  are made at Medium; the BEFORE record is named by `WAKESHOT_REAL_E2E_BEFORE`; a teardown per test, also on a
  failure.
- `crates/wakeshot-tray/tests/e2e/mod.rs`: generalized for real runs, with the dry run's API unchanged.
  - `TempData::real`, and `Tray` for both kinds.
  - Now shared: `journal`, `launch`, `assert_medium`, `terminate`, `exit_code_within`, `loop_window`, `full_path`
    and `wait_for`.
- `docs/research.md`: section "Integration real e2e".
- This card.
- No `app/*` file changed.

### Checks and outcomes

| Check | Outcome |
|---|---|
| BEFORE recorded (04:39, 04:59) | `<scratchpad>\p6-int-real-e2e\before.txt` (raw) and `before.env` (the record) |
| Tests first | Suite written and committed (0932c60) before any run |
| `real_e2e_cleanup` first and last | Nothing to do both times; AFTER equals BEFORE |
| Hold start and end | Applied 60 to 90 ms after the CLI; values stored and in force; 3 journal entries owned by the tray; RunOnce and one guard; restored on off, guard gone: pass |
| User change | Entry dropped 8.9 s after the change; core logged the release; lid DC stayed 2 at the hold end: pass |
| Plan switch (checklist 9) | A restored and B applied in 162 and 573 ms; new guard; B restored at the hold end: pass |
| Restore my power settings | CLI answered in 138 ms; A restored; hold kept with the SYSTEM request; no write for 35 s; next hold applies: pass |
| Quit | Exit 0, restored, Keep awake file gone, guard gone: pass |
| End of session (checklist 6, agent's part) | `WM_ENDSESSION` send returned in 39 and 56 ms with A restored: pass |
| Killed tray (checklist 4, agent's part) | Guard restored A 63 and 87 ms after the end by pid, cleared RunOnce, exited 0: pass |
| CLI restore, owner dead | "Your power settings are restored.", exit 0, A back, journal gone: pass |
| Icon entry | `NotifyIconSettings\2334163346184210159`, `IconGuid` the product GUID (text), `ExecutablePath` this card's x64 debug exe; left in place |
| No `Wakeshot int` or `Wakeshot test` plan left; no process of this card left | Pass |
| `rustfmt --edition 2024 --check` (both suites) | Exit 0 |
| Clippy x64 `-p wakeshot-tray`, with and without `--all-targets` | Clean. ARM64: not run here (no clang; CI) |
| Plain `cargo test --locked -p wakeshot-tray` | 8 + 5 pass, 25 ignored |
| Dry-run suite (check of the shared `mod.rs`) | 12 of 12 pass |
| Build folder deleted | Not yet: kept for the final record run, on the orchestrator's word |

### Open items

- The final record run on the orchestrator's word, then delete the build folder.
- Found outside scope 1: the dry-run icon keys, left for the dry-run card's clean-up.
- Not tested here, by the card: a real sign-out, End task in Task Manager (checklists 4 and 6, Haik's part), and the
  `UserChanged` notice at an unlock.
