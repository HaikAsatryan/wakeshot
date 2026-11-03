# The apply and the ownership check

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-ownership-apply\target`.
Runs after: `p2-ownership-restore` (Apply step 6 runs the restore routine; this card takes over `ownership/fake.rs`).
Parallel with: `p2-core-holds`, and any card of another track.

## Goal

Write the one apply (architecture section 5, "Apply", steps 1 to 6) and the ownership check on each plan read
("Ownership") in the core, generic over `PlanIo` and `SafetyNet`, test-first against `ownership::fake::FakePlans`.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 1 to 3).
- `docs/architecture.md` section 5: "Power settings" (targets, skip rule), "Apply", the stop-point paragraph after it,
  "Ownership", "Restore routine" step 6; section 8 rows for apply; section 9 "Windows layer unit tests" (the apply
  and check cases now run here).
- `docs/reviews/p1-design-power.md` majors 1, 2, 6; `p1-design-power-verify.md` minors 7, 8;
  `p1-design-final-check.md` major 1; `p1-contracts-domain.md` M2, M3.
- `crates/wakeshot-core/src/ownership/` (all), `settings.rs`, `command.rs` (`ApplyError`, `Outcome::Applied`).
- `docs/tasks/p2-ownership-restore.md` Result.
- `docs/tasks/p1-design-verify-fixes.md` Log: the walk of every stop point.

## Owns

`crates/wakeshot-core/src/ownership/apply.rs`, `ownership/check.rs`; `ownership/fake.rs` (handed over, for any call
the apply needs that the fake lacks: additions only, no public signature change, since cards of other tracks test
against it while this card runs; review m6). Nothing else.

## Deliverables

1. `ownership/apply.rs`: fill the contract signature. Step 1 (active plan read; failure: `SafetyNet`; another plan:
   `PlanChanged`, nothing written). Step 2 (`can_write` and the live value per key; a key at its target with no entry
   is the user's: skipped). Step 3 (RunOnce value, guard, then the journal last, with this tray as owner; an entry for
   this plan's key kept with its `original` and `restored` mark: taken over). Step 4 (a failure in 2 or 3: nothing
   written; take back the RunOnce value and the guard this apply added only when the journal file is known absent).
   Step 5 (writes and read-backs for keys not at their target). Step 6 (read the active plan again; same: `set_active`
   with it, then clear this plan's marks in one save, whose failure is tolerated; changed: no call, `PlanChanged`; a
   write, read-back, the second read or the call fails: the restore routine on this plan's entries, then `Write`).
   Entries on this plan whose key is not in `writes` (a key the user released) are restored in the same run and the
   same `set_active`, as the power track decided (`docs/phases/03-windows-power.md` note 6): read in step 2, marked in
   step 3's journal save by the restore step 3 rules, `original` written in step 5, dropped after a successful call.
2. `ownership/check.rs`: fill the contract signature. Read the active plan id, then each entry's value from its own
   plan; skip marked entries; a value that differs from `written`: drop it and report `{ plan, key }` as the user's; a
   read error: keep it, report the failure; save after drops; the last known entry dropped: the clean-up in its order.
3. Unit tests, written before the code, one named test per case:
   - step 1 read fails; step 1 other plan;
   - `can_write` fails for the second key; a live read fails: `SafetyNet`, nothing written, nothing left behind;
   - the user's own value at the target with no entry: skipped, no entry;
   - RunOnce fails; guard fails; journal save fails: nothing written; with the journal known absent the added RunOnce
     and guard are taken back; with a journal present (or unreadable) they stay;
   - takeover of entries left by a failed restore, marked and not: `original` and the mark kept;
   - takeover, then a failed write; a failed read-back; a failed `set_active`; a stop at each later call: no entry
     whose value is still ours (stored or in force) is dropped (final check major 1);
   - a plan switch between the writes and `set_active`: no call, `PlanChanged`, entries kept for the plan switch path;
   - an entry on this plan for a key outside `writes`: restored in the same run; a stop at each call keeps it
     reachable;
   - a user change between steps 2 and 5 (script `Run`): the result the design gives, stated in the test;
   - `set_active` uses the id read in step 6, never the one from step 1 (power review 6, verify minor 7);
   - success clears the marks; a failed mark save keeps them and the next restore handles them;
   - a stop at every call index of one apply over three keys, then a fault-free restore: every value back as the
     design says (the walk in `p1-design-verify-fixes`);
   - check: a changed value dropped and reported; a read error keeps the entry (power review 2); marked entries
     skipped; a deleted plan's read fails and keeps; the last drop runs the clean-up; the user setting exactly our
     value is not seen (accepted, section 5); a check before an apply never reports that apply's entries.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core ownership`: every test above and every `p2-ownership-restore` test pass; list
  the new test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass.
- `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- `cargo tree -p wakeshot-core -e normal`: no `windows` crate, no new dependency.
- Mutants, each made by hand, run, then reverted (record the failing test for each): write the journal before the
  RunOnce value; reset the `restored` mark on takeover; call `set_active` with the step 1 id; drop an entry on a read
  error in the check. Each must fail at least one test. Mutants run only in a scratch copy of the workspace, never in
  the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed (list them).

## Out of scope

- The restore routine (done; a bug found there goes under "Found outside scope" with a failing test name).
- The journal property test (`p2-ownership-props`). Windows calls, the mutex, the IO thread (power track).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`: proposals go to the Result.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` section 5, Apply step 3, read literally ("a key that already has an entry for this plan
   keeps its `original` and its `restored` mark"), keeps the `original` of an entry the user changed while a restore
   waited (a marked entry, which the check skips), so the next restore puts back an older value. The apply takes an
   entry over only while it is ours (`docs/research.md`, "Apply and ownership check checks"). Proposed text: "A key
   that already has an entry for this plan that is still ours (live equals `written`, or marked with live equal to
   `original`) keeps its `original` and its `restored` mark: the hold takes the entry over (...). Any other entry for
   the key is the user's or never written: it is replaced by a new entry with the live value as `original`."
2. Same section, Apply step 4: "if the journal file is known to be absent". The apply reads that as "holds no known
   entry" (missing, or only a newer version's entries), the rule `JournalParts` and power note 5 use for an empty
   journal. Proposed: "if the journal held no known entry (a read error does not count)".
3. For `p2-core-settings` (same as `p2-ownership-restore` Found outside scope 3): an apply or check whose journal
   load fails or finds a corrupt file reports `journal: []`, because nothing is known. The core must not read that
   as "nothing owned". The power track's `apply_locked` lock timeout is the same case.
4. A journal that turns corrupt while the tray runs: every apply fails with `SafetyNet(JOURNAL_CORRUPT)` and every
   check logs it, until a restore sets the file aside; the core may never send that restore, since its mirror may
   be empty. Rare (atomic saves). Proposal for the core and power tracks: after an apply fails with that failure,
   run one `Restore { plan: None }`, so the routine sets the file aside and `JournalCorrupt` reaches the user. This
   needs a design line, since the core branches on codes only where the design names them.
5. `Outcome::Applied` cannot carry a user change the apply sees (an entry no longer ours, item 1): the key is
   overridden again, though the user's value is then kept as `original` and comes back at the restore. Proposal, low
   priority: a `user_changed` field in `Applied` (contract change).
6. A user change between Apply steps 2 and 5 (a few calls) is overwritten, and the restore puts back the step 2
   value (`apply::tests::a_user_change_between_steps_2_and_5_is_overwritten`). The design does not name this window.
   Proposed: add it to the accepted cases under "Ownership", beside "a user who sets exactly our value".
7. `docs/phases/02-core.md` note 20: the mutant copy list (`Cargo.toml`, `Cargo.lock`, `.cargo\`,
   `rust-toolchain.toml`, `assets\`, `crates\`) now needs `docs\` too: `holds/file.rs` tests `include_str!`
   `docs/hold-files.md`.
8. Stop-point tests of other tracks (`p3-stop-points`, `p2-ownership-props`): keep one start with E1 as the only
   entry on its plan. With E2 or E3 beside it, the next restore's `set_active` for them hides a dropped E1.
9. Transient build breaks from other cards while this card ran, each gone on retry: `core.rs` declared `mod holds;`
   and `mod view;` before their files existed (E0583, about 20:58); `scenarios/holds.rs:939` had a macro error
   (about 21:30).

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:10: started by `wakeshot-worker`. Owned this round: `ownership/apply.rs`, `ownership/check.rs`,
  `ownership/fake.rs`, this card.
- 2026-11-03 20:25: read the card's list. Design picks (details in Result): the journal loads first (call 0), so
  every way out reports the file's entries; corrupt or unreadable journal: `SafetyNet`, nothing touched; an entry on
  a key in `writes` is taken over only while it is ours (live `written`, or marked with live `original`); any other
  is the user's or never written, so it becomes a new entry with the live value as `original`; "known absent" for the
  take-back means no known entry; the check reports a user change even when its save fails. Codes checked in the
  `windows` 0.62.2 source: `ERROR_FILE_CORRUPT` 1392, `ERROR_INVALID_DATA` 13.
- 2026-11-03 20:50: 34 apply tests written in `apply.rs` against the stub: all compile and fail (`not implemented`).
- 2026-11-03 20:58: 12 check tests written in `check.rs`. Build blocked by another card's `core.rs` (`mod holds;`,
  `mod view;` with no `core/holds.rs` or `core/view.rs` yet, E0583). Not mine: waiting, writing the code meanwhile.
- 2026-11-03 21:15: apply and check written. Build unblocked (the other card's files exist now).
  `cargo test --locked -p wakeshot-core ownership`: 91 passed (apply 34, check 12, restore 33, fake 9, journal 3).
  The check tests could not be run against the stub (build blocked then): to be shown in the scratch copy.
- 2026-11-03 21:30: rustfmt applied to the two owned files (no child modules); clippy found two test-code lints
  (fixed). A second outside break meanwhile: `scenarios/holds.rs:939` (macro error) failed `--all-targets`; gone on
  retry. Now: package tests 327 passed; clippy exit 0 for both targets, with and without `--all-targets`; `cargo tree
  -e normal`: serde, serde_json only.
- 2026-11-03 21:40: mutants in `<scratchpad>\p2-ownership-apply\mutants\` (runner `mutants.py` beside it). The first
  run did not compile: `holds/file.rs` tests `include_str!` `docs/hold-files.md`, and note 20's copy list has no
  `docs\`. Added `docs\` to the copy; rerunning.
- 2026-11-03 21:55: 11 mutants run: 10 killed, 1 survived (the literal "step 1 id" mutant, equivalent; its variant
  killed). "Mark reset on takeover" was killed by only 2 tests: the stop walk missed it, because the other entries'
  `set_active` in the next restore also brought E1's `original` into force. Added walks and a failed write with each
  taken-over entry alone; rerun: killed by 4, among them the stop walk and the failed write.
- 2026-11-03 22:00: final checks rerun on the final code (all pass, below). `docs/research.md` section "Apply and
  ownership check checks" added. Scratch folder `<scratchpad>\p2-ownership-apply` (build folder, mutant copy, its
  build folder, runner) deleted. Status: done.

## Result

Done. The apply (steps 1 to 6, with track note 3's released entries) and the ownership check, test-first against
`FakePlans`: 34 apply tests failed against the stub, then passed. The 12 check tests were written first too; the
stub run was blocked by another card's build, so their failing against the stub was shown in the scratch copy (all
12 failed there).

Files changed: `crates/wakeshot-core/src/ownership/apply.rs`, `crates/wakeshot-core/src/ownership/check.rs`, this
card, `docs/research.md` (new section "Apply and ownership check checks", addition only). `ownership/fake.rs`
unchanged: the apply needed no call the fake lacks. No contract signature changed (docs and two constants added).

### Public API (for the power track)

`ownership::apply`:

- `pub fn apply(plans: &mut impl PlanIo, net: &mut impl SafetyNet, me: JournalOwner, plan: Guid, writes: &[(SettingKey,
  u32)]) -> Applied`; `pub struct Applied { pub written: Vec<SettingKey>, pub result: Result<(), ApplyError>, pub
  journal: Vec<JournalEntry> }` (unchanged).
- `written`: keys whose target write returned `Ok`, in call order, whatever `result` says. `journal`: the known
  entries the file holds after the run, every plan; empty when the file was missing, corrupt or unreadable.
- `pub const JOURNAL_CORRUPT: Failure` (`call: "journal.json (does not parse)"`, `code: 1392`, `ERROR_FILE_CORRUPT`);
  `pub const READ_BACK_DIFFERS: Failure` (`call: "read-back (another value than written)"`, `code: 13`,
  `ERROR_INVALID_DATA`).
- Results: `SafetyNet(f)`: the load failed or the file is corrupt, step 1's read, a `can_write`, a step 2 read, the
  RunOnce value, the guard or the journal save failed; nothing written. `PlanChanged`: step 1 (nothing written) or
  step 6 (written, journaled, no call). `Write(f)`: a write, a read-back, the second plan read or `set_active`
  failed; the restore routine already ran on this plan's entries and `journal` is what it left. `Ok`: our values are
  in force on `plan`, or there was nothing to own.
- Call order: `load_journal`; `active_plan`; `can_write` per key in `writes`; `read` per key; `read` per entry on
  `plan` for a key not in `writes`. Nothing to own or restore: no safety net call (only a save, or the clean-up,
  when released entries the user changed are dropped). Else `set_run_once`, `ensure_guard`, `save_journal` (owner
  `me`); per entry on `plan` in journal order (taken-over entries first, new ones last) a `write` of the target and
  its read-back `read`, or a released entry's `original` (no read-back, as in the restore routine), stopping at the
  first failure; `active_plan`; `set_active` with that id; one save (or the clean-up) only when a mark or a released
  entry is left to clear. On failure after the journal save: `restore(plans, net, Some(plan), None)`.

`ownership::check`:

- `pub fn check(plans: &mut impl PlanIo, net: &mut impl SafetyNet) -> Checked`; `pub struct Checked { pub
  user_changed: Vec<OwnedKey>, pub failures: Vec<Failure>, pub journal: Vec<JournalEntry> }` (unchanged).
- Call order: `load_journal` (missing: nothing else; corrupt: `failures: [JOURNAL_CORRUPT]`, file left for the
  restore; failed: that failure); with no unmarked known entry, nothing else; `active_plan` (fails: nothing read or
  dropped); `read` per unmarked entry from its own plan; after a drop, one save, or the clean-up of restore step 6
  when no known entry is left. `journal`: the entries kept.

### Design choices (each with its "change it when")

1. The journal loads first, before step 1, so every way out reports the file's entries (the core's mirror copies
   `journal`). Change it when the core ignores `journal` on a failed apply.
2. Corrupt or unreadable journal: `SafetyNet`, nothing touched, the file left for the restore routine, which alone
   can report `Found::SetAside`. Change it when a journal that turns corrupt while the tray runs shows up (Found
   outside scope 4).
3. An entry on a key in `writes` is taken over (its `original` and mark kept, mark cleared after a successful call)
   only while it is ours: live equals `written`, or marked with live equal to `original`. Any other entry is the
   user's or never written, and becomes a new entry with the live value as `original`, in its place. Change it when
   the core wants such a change reported as `UserChanged` (Found outside scope 5).
4. "Known absent" for step 4's take-back means no known entry (missing, or only a newer version's entries). Only
   what this apply added (`set_run_once` or `ensure_guard` returned `true`) is taken back; a failed take-back is not
   reported over the first failure (walk R6: harmless). Change it when restore mode handles unknown keys.
5. Released entries (track note 3) follow restore step 3: ours stored (marked or not): marked in step 3's save,
   `original` written in step 5; marked with `original` stored: only the call; any other: dropped in step 3's save.
   Dropped from the journal only after the successful call. No `can_write` for them, as in the restore routine.
   A failed read of one fails the apply with `SafetyNet`. Change it when a policy block shows up on restore writes.
6. `set_active` runs even when nothing was written: an entry with ours stored and `original` in force (an earlier
   takeover stopped before its call) needs it. The final save runs only when something changed, and its failure is
   tolerated (`journal` is then the step 3 entries). Change it when the extra call shows a side effect (checklist 15).
7. The check reports a user change even when its save fails: the entry left in the file reads as the user's to
   every routine, so the cost is a repeated notice, while not reporting it would let the next apply override the
   user. A file with no known entry is left to the restore (no clean-up in the check). Change it when repeated
   notices show up.
8. Targets are fixed (section 5, all 0), so a taken-over entry's `written` already is the target; a future target
   change would need its own step (one `written` per entry cannot cover a stop between save and write). Change it
   when a target other than 0 is added.

### Checks

- `cargo test --locked -p wakeshot-core ownership`: 91 passed (apply 34, check 12, restore 33, fake 9, journal 3).
  New apply tests: `step_1_read_fails_writes_nothing`, `step_1_other_plan_writes_nothing`,
  `unreadable_journal_fails_and_touches_nothing`, `corrupt_journal_fails_and_is_left_for_the_restore`,
  `can_write_fails_for_the_second_key_and_nothing_is_left`, `a_failed_live_read_leaves_nothing`,
  `the_users_own_value_at_the_target_is_skipped`, `nothing_to_override_makes_no_safety_net_call`,
  `a_failed_run_once_write_leaves_nothing`, `a_failed_guard_start_takes_back_the_run_once_value`,
  `a_failed_journal_save_takes_back_what_this_apply_added`, `a_failed_journal_save_keeps_what_was_there_before`,
  `with_a_journal_present_the_added_run_once_value_and_guard_stay`,
  `a_journal_with_only_unknown_entries_counts_as_absent`, `a_fresh_apply_journals_last_then_writes_and_activates`,
  `takeover_keeps_each_entrys_original_and_mark_until_the_call`,
  `takeover_with_nothing_to_write_still_calls_set_active`,
  `other_plans_entries_and_unknown_keys_are_saved_back_untouched`,
  `an_entry_no_longer_ours_gets_the_live_value_as_original`, `takeover_then_a_failed_write_keeps_every_held_value`,
  `takeover_then_a_failed_read_back_keeps_every_held_value`,
  `takeover_then_a_failed_second_plan_read_keeps_every_held_value`,
  `takeover_then_a_failed_set_active_keeps_every_held_value`,
  `a_stop_at_every_call_of_a_fresh_apply_then_a_restore_puts_every_value_back`,
  `takeover_then_a_stop_at_any_call_drops_no_held_value`, `a_released_entry_stays_reachable_at_every_stop`,
  `a_plan_switch_between_the_writes_and_set_active_makes_no_call`, `set_active_uses_the_id_read_in_step_6`,
  `a_released_entry_on_this_plan_is_restored_in_the_same_run`,
  `a_released_entry_the_user_changed_is_dropped_without_a_write`,
  `only_released_entries_are_restored_and_the_safety_net_released`,
  `a_failed_read_of_a_released_entry_writes_nothing`, `a_user_change_between_steps_2_and_5_is_overwritten`,
  `a_failed_mark_clearing_save_keeps_the_marks_for_the_next_restore`. New check tests:
  `a_changed_value_is_dropped_and_reported`, `a_read_error_keeps_the_entry_and_reports_the_failure`,
  `a_deleted_plans_read_fails_and_keeps_the_entry`, `marked_entries_are_skipped`,
  `the_last_drop_runs_the_clean_up_in_its_order`, `unknown_keys_are_kept_and_never_read`,
  `the_user_setting_exactly_our_value_is_not_seen`, `a_check_before_an_apply_never_reports_that_applys_entries`,
  `a_plan_switch_never_looks_like_a_user_change`, `a_failed_active_plan_read_drops_nothing`,
  `an_unreadable_or_corrupt_journal_is_reported_and_left`, `a_failed_save_still_reports_the_users_change`.
  Every `p2-ownership-restore` test still passes.
- `cargo test --locked -p wakeshot-core`: 332 passed.
- `rustfmt --edition 2024 --check` on `apply.rs`, `check.rs`, `fake.rs`: no diff.
- `cargo clippy --locked -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: exit 0 (all four).
- `cargo tree --locked -p wakeshot-core -e normal`: serde and serde_json only; no `windows` crate, no new dependency.
- Mutants, in a scratch copy with its own build folder, each reverted, both deleted after (failing tests named):
  - journal saved before the RunOnce value: 8 failed, among them `a_failed_run_once_write_leaves_nothing`,
    `a_fresh_apply_journals_last_then_writes_and_activates`,
    `a_stop_at_every_call_of_a_fresh_apply_then_a_restore_puts_every_value_back`;
  - `restored` mark reset on takeover: 4 failed: `takeover_keeps_each_entrys_original_and_mark_until_the_call`,
    `a_failed_mark_clearing_save_keeps_the_marks_for_the_next_restore`,
    `takeover_then_a_failed_write_keeps_every_held_value`, `takeover_then_a_stop_at_any_call_drops_no_held_value`;
  - `set_active` with the step 1 id: as worded (`set_active(plan)` in place of the id just read) it survived, and no
    test can kill it: the call runs only when that id equals `plan` (an equivalent mutant). The variant that calls
    with the step 1 id whatever the second read returns: 2 failed:
    `a_plan_switch_between_the_writes_and_set_active_makes_no_call`, `set_active_uses_the_id_read_in_step_6`;
  - check drops an entry on a read error: 2 failed: `a_read_error_keeps_the_entry_and_reports_the_failure`,
    `a_deleted_plans_read_fails_and_keeps_the_entry`;
  - extra: the check as the stub: all 12 check tests failed; RunOnce and the guard taken back with a journal present:
    `with_a_journal_present_the_added_run_once_value_and_guard_stay`; no `set_active` when nothing was written:
    `takeover_with_nothing_to_write_still_calls_set_active`,
    `only_released_entries_are_restored_and_the_safety_net_released`; any existing entry taken over with its
    `original`: `an_entry_no_longer_ours_gets_the_live_value_as_original`; the failure path without the restore
    routine: the four `takeover_then_a_failed_*` tests; the check reading marked entries:
    `marked_entries_are_skipped`, `a_failed_mark_clearing_save_keeps_the_marks_for_the_next_restore`.
- Only the owned files changed: `apply.rs`, `check.rs`, this card; plus the addition to `docs/research.md`.
- Build folder and the whole scratch folder deleted at the end.

### Open items

- Found outside scope 1, 2 and 6: proposed `architecture.md` text (Apply steps 3 and 4, the accepted window).
- Found outside scope 3 and 4: for `p2-core-settings` and the power track (an empty `journal` on a failed load; a
  journal that turns corrupt mid-run).
- Found outside scope 7 and 8: note 20's copy list, and an E1-alone start for other stop-point tests.
- For the power safety review: the takeover rule (design choice 3) differs from a literal reading of Apply step 3.
