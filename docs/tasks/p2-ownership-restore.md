# Restore routine, journal file form and the ownership fake

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-ownership-restore\target`.
Runs after: the contracts card (it creates `ownership/` with the `PlanIo` and `SafetyNet` traits and the routine
signatures). Parallel with: `p2-policy-holds`, `p2-policy-power`, `p2-present`, and any card of another track.

## Goal

Write the one restore routine every process runs (architecture section 5, "Restore routine", steps 1 to 7) in the
core, generic over `PlanIo` and `SafetyNet`, test-first against a fake that can fail or stop any single call. Also the
split and merge between `JournalFile` and the core's `Journal`.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 1 to 3, module layout);
  `docs/phases/03-windows-power.md` notes 4 to 7 (the power track's stop-point kit and decisions this card follows).
- `docs/architecture.md` section 5: "Apply" (steps 3 and 6, what the restore must handle after them), "Ownership",
  "Restore routine", "Liveness" (only what restore mode passes in); section 6 "Journal format"; section 8 rows for
  restore; section 9 "Windows layer unit tests" (the restore and stop cases now run here).
- `docs/reviews/p1-contracts-domain.md` M2, M3, M4, m2; `docs/reviews/p1-design-power-verify.md` majors 1 and minor 3;
  `docs/reviews/p1-design-final-check.md` major 1.
- `crates/wakeshot-core/src/`: `settings.rs`, `files.rs`, `command.rs`, `ownership/` (contracts card).
- `docs/tasks/p1-design-verify-fixes.md` Log: the walk of every stop point.

## Owns

`crates/wakeshot-core/src/ownership/journal.rs`, `ownership/restore.rs`, `ownership/fake.rs`. Nothing else.
`ownership/io.rs` and `ownership/mod.rs` are contract files: a change goes to the Result.

## Deliverables

1. `ownership/journal.rs`: split a `JournalFile` into the entries this version knows (`Journal`) and the rest (kept
   untouched), and merge them back for a save. A save always writes `JournalFile::VERSION` (review m2). Unknown keys,
   as the power track decided (`docs/phases/03-windows-power.md` note 5): skipped by restore, apply and check; when no
   known entry is left, the clean-up clears RunOnce and stops the guard but saves a file with only the unknown entries
   instead of deleting it.
2. `ownership/fake.rs` (behind `test-support`): the contracts card already wrote a working `FakePlans` (API in its
   doc and `architecture.md` section 2: a cheap clone handle over one shared state, `Arc<Mutex>`; tests pass two
   clones for the two seams, both log into one call list). Extend it; its public API is contract now: add, never
   change (`p1-contracts-split` edit 1). Still missing there: who set each stored value. It must cover, for `PlanIo`
   and `SafetyNet`:
   - stored values per plan and key, and "in force" values for the active plan (copied from stored at each
     `set_active` and at a plan switch), so a test can see that Windows still runs with our value;
   - who set each stored value (the user, Wakeshot's override, Wakeshot's restore), for the property tests;
   - a call log with arguments;
   - a script keyed by call index: `Fail(code)`, `Stop` (a panic at the call, as the contracts card chose: dev and
     test builds unwind, as a crash would, and the next start runs against the same state), and `Run(closure)` on the
     fake's state (a user change or a plan switch between two calls);
   - the journal file as missing, corrupt, unreadable or a `JournalFile`; the RunOnce value; the guard (running,
     stopping); helpers for a user change, a plan switch, a deleted plan;
   - `values()` from the fake's active plan (`PlanIo::values()` is in the core trait, contract C6; the routines never
     call it, so stop-point call indexes stay the routines' own; review m6).
   Later cards of every track test against this fake (`p3-journal`, `p3-recovery`, `p3-power-io`, `p4-cli-restore`,
   `p6-hooks`): keep its public signatures stable once this card is done.
3. `ownership/restore.rs`: fill the contract signature. Steps 1 to 7 of section 5 in order, including: the scope
   (`plan`), the expected owner for guard and restore-mode retries (another owner: stop, report it), a report that
   says what was found (nothing, entries, set aside, other owner), a corrupt journal set aside with values left as
   they are (the caller posts `Event::JournalCorrupt`), the write-ahead mark, the second active plan read before
   `set_active`, and the clean-up order (journal file first, then the RunOnce value, then the guard's stop event) so
   a stop never leaves a journal with a known entry but no RunOnce value.
4. Unit tests in `restore.rs`, `journal.rs` and `fake.rs`, written before the code. One named test per case:
   - missing journal: nothing to do; unreadable journal: the run fails, nothing touched; no known entries: clean-up;
   - corrupt journal: set aside, no write, `set_aside` reported;
   - failed active plan read, and a plan list that ends on an error: the run fails before any entry is dropped or
     written (power verify major 1);
   - a deleted plan's entries dropped only from a complete list;
   - a failed entry read: kept, the run fails;
   - each row of step 3 (not marked and live equals `written`; marked and live equals `written`; marked and live
     equals `original`; otherwise dropped);
   - marks saved before any `original` is written (power verify minor 3); a failed mark save writes nothing;
   - a stop after the mark and before the write: the next run restores again;
   - a failed `set_active` keeps the marked entries; a retry finishes them;
   - a plan switch between the writes and `set_active`: the now inactive plan's marked entries are dropped, no call;
   - marked entries on an inactive plan need no call;
   - `Some(plan)` takes only that plan's entries; two plans in one journal;
   - another owner on a retry: stop with nothing written;
   - unknown keys kept and saved back untouched; only unknown entries left: RunOnce cleared, guard stopped, file
     kept with them; a save writes `JournalFile::VERSION`;
   - the last entry dropped runs the clean-up once, in its order; a stop inside the clean-up leaves a state the next
     run finishes;
   - a stop at every call index of one run with two plans and three keys: afterwards, every value still ours in force
     or stored has an entry (a loop over the indexes, not one test per index);
   - the fake: each script action, the in-force copy at `set_active`, the call log.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core ownership`: every test above passes; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass.
- `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- `cargo tree -p wakeshot-core -e normal`: no `windows` crate, no new dependency.
- Mutants, each made by hand in this card's files, run, then reverted (record the failing test for each): treat a
  failed plan list as an empty list; write `original` before the mark; drop marked entries when `set_active` fails;
  run the clean-up after a failed entry read. Each must fail at least one test. Mutants run only in a scratch copy
  of the workspace, never in the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the three owned files changed (list them).

## Out of scope

- The apply and the ownership check (`p2-ownership-apply`), the journal property test (`p2-ownership-props`).
- Windows calls, the mutex, liveness checks, restore mode's 10 s retries and 2 minute limit (power track).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`: proposals go to the Result.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` section 5, restore step 5: "entries marked `restored` on an inactive plan need no apply:
   drop them" reads as judged by step 2's plan read. A switch to that plan after step 2 and before step 4 writes its
   `original` leaves our value in force with no entry (`docs/research.md`, "Restore routine checks"). The routine
   judges by the second read. Proposed text: "5. If any entry is marked `restored` with its `original` stored, read
   the active plan id again right before the call. The read fails: keep them, this run fails. Entries on any other
   plan need no apply: drop them. Entries on that plan: `PowerSetActiveScheme` with it; success: drop them; failure:
   keep them, this run fails."
2. `docs/phases/03-windows-power.md` note 4 says that after the next restore "RunOnce is gone; the guard is told to
   stop". A stop between the clean-up's file delete and the RunOnce clear (or before the guard's stop event) leaves
   them, and the next restore finds no journal, so it does nothing (walk R6, accepted as harmless). `p3-stop-points`
   should assert it only for stops before the delete, as `a_stop_at_any_call_keeps_every_held_value_journaled` does.
3. For `p2-core-settings`: a restore whose journal load fails reports `result: Err` with `journal: []`, since nothing
   is known. `Outcome::Restored` says the core's mirror copies `journal`; the core should not read that empty list
   as "nothing owned". Proposed: on `Restored` with `Err`, keep retrying while a restore is wanted, whatever `journal`
   holds.
4. Tooling: in this session `cargo` was not on `PATH` in PowerShell or Git Bash; `%USERPROFILE%\.cargo\bin\cargo.exe`
   works by path.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started. Reading the card's sources.
- 2026-11-03: sources read. Design picks: `JournalParts { known, other }` with `split` and `merge` in `journal.rs`;
  step 5 judges "inactive" from the second active plan read, not step 2's (a switch to a plan before our write of its
  `original` would leave our value in force with no entry); the fake labels each stored and in-force value with
  `SetBy` (user, override, restore), a write's label taken from the fake's own journal entry. Next: tests first.
- 2026-11-03: `journal.rs` (split, merge, 3 tests) and the fake's additions (labels, `set_stored`, `set_in_force`,
  `known_entries`, `held`, `held_without_entry`, 3 tests) written and passing. 33 restore tests written; against the
  `unimplemented!` stub all 33 fail (run once). `cargo` is not on this shell's PATH; runs use
  `~\.cargo\bin\cargo.exe` through a scratch script. Next: the routine.
- 2026-11-03: routine written; `cargo test --locked -p wakeshot-core ownership`: 45 passed. Next: format, clippy, full
  crate tests, `cargo tree`, mutants.
- 2026-11-03: owned files formatted (`rustfmt --edition 2024`); clippy fixed two test lints (a `type Start`, an
  `enumerate`); clippy exit 0 on both targets with and without `--all-targets`, and with `--features test-support`;
  `cargo test --locked -p wakeshot-core`: 63 passed; `cargo tree -e normal`: serde and serde_json only. Next: mutants
  in a scratch copy.
- 2026-11-03: five mutants (the card's four, the last in two forms) planted by a script in
  `<scratchpad>\p2-ownership-restore\mutants\` with its own build folder; each failed at least one test (list in the
  Result); copy and build folder deleted. Next: `docs/research.md`, Result.
- 2026-11-03: `docs/research.md` "Restore routine checks" added; Result written; build folder and scratch scripts
  deleted. Done.

## Result

Done. The restore routine, the journal split and merge, and the fake's additions, test-first (33 restore tests failed
against the stub, then passed).

Files changed: `crates/wakeshot-core/src/ownership/journal.rs`, `ownership/restore.rs`, `ownership/fake.rs`. Docs:
this card, `docs/research.md` (new section "Restore routine checks", addition only). No contract file changed.

### Public API

`ownership::journal`:

- `pub struct JournalParts { pub known: Journal, pub other: Vec<JournalFileEntry> }`;
  `JournalParts::split(file: JournalFile) -> Self`; `merge(&self) -> JournalFile` (always `JournalFile::VERSION`,
  known entries first, then `other` as read). The journal counts as empty when `known.entries` is empty.

`ownership::restore` (contract signatures unchanged; docs added):

- `restore(plans, net, plan: Option<Guid>, expect_owner: Option<JournalOwner>) -> Restored`.
- `Found::Nothing`: no file, or one with no known entry (the clean-up ran), or a load that failed (`result: Err`).
  `Entries`: a file with a known entry, in scope or not. `SetAside`: did not parse; `result: Ok` means moved, `Err`
  means the move failed, so post `Event::JournalCorrupt` only on `Ok`. `OtherOwner(owner)`: nothing done, `Ok`.
- `Restored.result`: the first failure. `Restored.journal`: the known entries the file holds after the run, every plan.
- `pub(crate) save_or_clean_up(net, &JournalParts) -> Result<(), Failure>`: save after a drop; with no known entry
  left, the clean-up (file deleted or saved with only unknown entries, then `clear_run_once`, then `stop_guard`). For
  `p2-ownership-apply`'s check and apply.
- Call order of one run: `load_journal`; nothing in scope: stop. `active_plan`, `plans`; `read` per in-scope entry on
  a listed plan; if any entry reads `written`: `save_journal` (marks), then `write` of each `original`; if any entry
  waits for the call: `active_plan` again, then `set_active` only if one is on that plan; then one `save_journal` or
  the clean-up when entries were dropped. A missing journal makes no other call (RunOnce is not touched).

`ownership::fake` (everything from the contracts card unchanged; added):

- `SetBy { User, Override, Restore }`; `SetByKeys { lid_ac, lid_dc, sleep_dc }` with `SetByKeys::USER` and
  `get(key)`.
- `FakePlan` gains `stored_by` and `in_force_by: SetByKeys` (a field addition; no code outside the fake builds a
  `FakePlan`). Labels move with the values at `set_active` and `switch_plan`; `user_write` sets `User`. A `write` is
  `Restore` when the fake's journal file has an entry for that plan and key whose `original` is the value (and not its
  `written`); every other write is `Override`.
- `FakeState::stored_by`, `in_force_by`, `set_stored(plan, key, value, by)` and `set_in_force(..)` (starting states,
  no call logged), `known_entries()`, `held()` (overrides stored on any plan or in force on the active plan),
  `held_without_entry()` (the stop-point and property check).

### Design choices

- Step 5 judges "inactive" by its second plan read, not step 2's (Found outside scope 1). Step 2's read stays, so a
  failing plan API fails the run before anything changes. Change it when the architecture text says otherwise for a
  reason, or a Windows test shows a switch does not apply stored values.
- A failed read, write or call fails only its own entry; the run goes on with the rest and reports the first
  failure. Drops decided on good reads are saved even in a failed run. Change it when a Windows failure is shown to
  make later calls unsafe.
- A loaded file with only unknown entries is not rewritten; RunOnce and the guard are still released. Change it when
  a newer version needs the file rewritten to read it.
- The mark save runs whenever an entry is to be written, even if all are already marked: one rule, one extra save in
  a rare case. Change it when the save shows up as a cost.
- The fake takes a write's label from its own journal entry, so it needs no hint from the routine. Change it when a
  test needs a label the journal cannot give.

### Checks

- `cargo test --locked -p wakeshot-core ownership`: 45 passed. Restore: `missing_journal_does_nothing`,
  `unreadable_journal_fails_and_touches_nothing`, `no_known_entries_runs_the_clean_up`,
  `corrupt_journal_is_set_aside_and_values_stay`, `failed_active_plan_read_fails_before_any_change`,
  `failed_plan_list_fails_before_any_change`, `deleted_plans_entries_are_dropped_from_a_complete_list`,
  `failed_entry_read_keeps_the_entry_and_fails`, `failed_entry_read_keeps_only_that_entry`,
  `unmarked_live_written_is_restored`, `marked_live_written_is_restored_again`,
  `marked_live_original_only_needs_set_active`, `any_other_live_value_is_dropped`,
  `marks_are_saved_before_any_original_is_written`, `failed_mark_save_writes_nothing`,
  `stop_after_the_mark_before_the_write_restores_again`, `failed_write_keeps_the_entry_marked_and_fails`,
  `failed_set_active_keeps_the_marked_entries_and_a_retry_finishes`, `failed_second_active_plan_read_keeps_the_entries`,
  `plan_switch_before_set_active_drops_the_now_inactive_plans_entries`,
  `plan_switch_before_the_write_applies_the_plan_now_active`, `marked_entries_on_an_inactive_plan_need_no_set_active`,
  `a_plan_scope_takes_only_that_plans_entries`, `nothing_in_scope_makes_no_plan_call`,
  `two_plans_in_one_journal_are_both_restored`, `another_owner_on_a_retry_is_left_alone`,
  `unknown_keys_are_kept_and_saved_back_untouched`, `only_unknown_entries_left_keeps_the_file_and_releases`,
  `a_file_with_only_unknown_entries_is_not_rewritten`, `a_save_writes_the_current_version`,
  `the_last_drop_runs_the_clean_up_once_in_its_order`, `a_stop_at_any_call_keeps_every_held_value_journaled`,
  `a_stop_inside_the_clean_up_is_finished_by_the_next_run`. Journal: `split_keeps_known_and_unknown_entries_apart`,
  `merge_keeps_unknown_entries_untouched_and_writes_this_version`, `only_unknown_entries_count_as_an_empty_journal`.
  Fake (new): `writes_take_their_label_from_the_journal_entry`, `labels_move_with_the_values`,
  `held_counts_stored_values_and_the_active_plans_in_force_values`; the contracts card's six fake tests still pass.
- `cargo test --locked -p wakeshot-core`: 63 passed.
- `rustfmt --edition 2024 --check` on the three files: no diff.
- `cargo clippy --locked -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: exit 0 (also with `--features test-support`, x64).
- `cargo tree --locked -p wakeshot-core -e normal`: serde and serde_json only; no `windows` crate, no new dependency.
- Mutants, in a scratch copy with its own build folder, each reverted, both deleted after:
  - failed plan list read as an empty list: `failed_plan_list_fails_before_any_change` failed;
  - `original` written before the mark: 6 failed, among them `marks_are_saved_before_any_original_is_written`,
    `failed_mark_save_writes_nothing`, `a_stop_at_any_call_keeps_every_held_value_journaled`;
  - marked entries dropped when `set_active` fails: `failed_set_active_keeps_the_marked_entries_and_a_retry_finishes`,
    `failed_second_active_plan_read_keeps_the_entries` failed;
  - clean-up after a failed entry read, as a drop of that entry: `failed_entry_read_keeps_the_entry_and_fails`,
    `failed_entry_read_keeps_only_that_entry` failed; as a release after any failure: 3 failed, among them
    `failed_entry_read_keeps_only_that_entry`.
- Build folder deleted at the end.

### Open items

- The three proposals under "Found outside scope" (architecture step 5 text, the note 4 RunOnce check, the core's
  reading of an empty `journal` on a failed load).
- `fake.rs` passes to `p2-ownership-apply` now.
