# Journal property test: no held value is ever dropped

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
Real laptop: no (pure core tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-ownership-props\target`.
Runs after: `p2-ownership-apply`. Parallel with: `p2-core-settings`, and any card of another track.

## Goal

The guard for the settings ownership rules. Three design reviews found ordering bugs in them on paper; this property
test runs the real `ownership::apply`, `restore` and `check` on random sequences and must catch that kind of bug.
Write it from the design, not from the code: you are a second pair of eyes on the two cards before.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 1 to 3, 19).
- `docs/architecture.md` section 5: "Power settings", "Apply" and the stop-point paragraph, "Ownership", "Restore
  routine"; section 6 "Journal format"; section 9 core property tests.
- Reviews, for the bugs this test must catch: `docs/reviews/p1-design-power.md` majors 1, 2, 6;
  `p1-design-power-verify.md` major 1, minors 3, 7, 8; `p1-design-final-check.md` major 1.
- `crates/wakeshot-core/src/ownership/` (all); Results of `p2-ownership-restore` and `p2-ownership-apply`.

## Owns

`crates/wakeshot-core/src/ownership/props.rs` (`#[cfg(test)]`). Nothing else. A missing hook in `ownership/fake.rs`
goes to the Result as a request, or into a test-only wrapper inside `props.rs`.

## Deliverables

1. `ownership/props.rs`, a `proptest` model over `FakePlans`:
   - World: three plans (one may be deleted while inactive), each with user values for the three keys drawn from a
     small set that includes the targets (0), plus the active plan.
   - Operations, any order, up to about 40 per case: `Apply` (to the active plan or a stale id; any subset of keys);
     user change (any plan, key, value, including our target and the original); plan switch; plan delete; `check`;
     `restore` with each scope (`None`, `Some(plan)`); each routine call may carry one scripted fault
     (fail call n with a code) or a stop after call n. Faults hit plan reads, entry reads, `plans`, writes,
     read-backs, `set_active`, journal load and save, RunOnce, guard and clean-up calls alike. After a stop, the next
     routine runs as a new tray (new owner), as after a crash.
   - Ending: one restore with no faults (and the expected-owner rules of a guard retry where the case says so).
2. Properties, each its own `proptest!` test:
   - **Never drops a held value** (this plan's acceptance check): after every operation, every key whose value
     Wakeshot still holds on a plan that exists (stored by our override, or in force on the active plan) has a known
     entry in the journal file. A failed journal load is a fault of one call; the file keeps its content. The
     operations never corrupt the file: a corrupt journal is the accepted gap of section 5, tested in
     `p2-ownership-restore`.
   - **The final restore puts everything back**: after the fault-free ending, no value is held anywhere; every key the
     user did not change is back at its original, stored and in force; the journal file is gone, the RunOnce value is
     cleared, the guard's stop event is set.
   - **A journal with a known entry never lives without its RunOnce value** after any operation (the clean-up
     order; a file with only unknown keys keeps no RunOnce value, `03-windows-power.md` note 5).
   - **`SafetyNet` writes nothing**: an apply that reports it changed no stored or in-force value.
   - **`set_active` only on the active plan**: no call names a plan other than the one active at that moment
     (power review 6).
   - **No false user change** in a case with no stop: `check` reports a key as the user's only if the user changed it
     since Wakeshot's last write.
3. Shrinking gives readable cases: operations print as short lines.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core ownership::props`: all properties pass at the default case count, in under 30 s
  in total on this laptop (record the time).
- One long run: `PROPTEST_CASES=20000 cargo test --locked -p wakeshot-core ownership::props --release`: passes (record
  time).
- Mutants, each made by hand in `ownership/` (restore, apply or check) of a scratch copy of the workspace, never in
  the repo: those files are other cards' and other cards test against them (verify review Major 2; the copy, its
  build folder and the clean-up are in `docs/phases/02-core.md` note 20). Each must fail a property (record which,
  and the shrunk case):
  1. a failed or short plan list counts as "plan deleted" (verify major 1);
  2. the restore writes `original` before it saves the mark (verify minor 3);
  3. the apply's takeover resets the mark or `original` (final check major 1);
  4. the restore drops marked entries when `set_active` fails;
  5. the apply calls `set_active` with the id from step 1 (power review 6);
  6. the check drops an entry on a read error (power review 2);
  7. the clean-up clears the RunOnce value before it deletes the journal.
  A mutant no property catches is a finding: add the missing property, or record why the design makes it harmless.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo
  clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for
  `aarch64-pc-windows-msvc`: exit 0.
- A bug the properties find in `apply.rs`, `check.rs` or `restore.rs`: not fixed here; the failing case goes under
  "Found outside scope" with the shrunk sequence, and the Result says so. The orchestrator routes the fix.
- Only `ownership/props.rs` changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Fixing the routines (their cards are done; findings are routed). Core state machine properties (`p2-core-props`).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed. Each was found with this card's harness on the fake (`docs/research.md`, "Journal property
test checks").

1. **Finding 1 (minor; the apply and the design): a failed apply whose restore routine fails too leaves a false user
   change.** Apply step 5 or 6 fails, so the apply runs the restore routine on its plan; that routine fails too (its
   load, a plan read, its mark save or its drop save). An entry saved in step 3 and never written stays, unmarked,
   live equal to `original`, and the next check reports it as the user's: a "you changed a setting" notice for
   nothing, and the key is released for the episode, so the hold stops holding it. No held value is lost. A read-back
   that differs (the user in that moment, or anything that resets the value) counts as the first failure. Shrunk
   case, from the user-change property with its excuse taken out (every routine with a fault and a user action):
   ```
   start: P0 (active) lidAc 2 lidDc 1 sleepDc 600; P1 lidAc 1 lidDc 1 sleepDc 600; P2 lidAc 1 lidDc 2 sleepDc 600;
    1. apply to the active plan [lidAc], call 11 fails (5), user sets P0 lidAc = 1 at call 0
    2. restore all, call 10 fails (5), user switches to P2 at call 0
    3. apply to the active plan [lidAc lidDc], call 11 fails (5), user sets P2 lidAc = 2 at call 10
    4. check, call 2 fails (5), user sets P0 lidAc = 1 at call 0
   -> the check reports P2 lidDc, which was never written
   ```
   Smallest form: the ignored test `ownership::props::a_failed_apply_whose_restore_fails_reports_no_false_user_change`.
   Proposal (needs a design line in section 5, Apply steps 3 and 6, and the stop-point paragraph): save a new entry
   marked `restored` in step 3, and clear its mark with the taken-over entries' marks after the successful
   `set_active` (step 6 already does that one save). "Marked, live equals `original`" is what restore step 3 keeps for
   step 5 and what the check skips, so neither a stop nor a failed restore turns it into a user change. Cost: one
   journal save per apply that adds an entry, and one `set_active` in the rare restore that finds such an entry. When
   it is fixed: remove the `#[ignore]` and the excuse in `Run::apply` (props.rs, "Finding 1").
2. **Finding 2 (minor; the apply's report): `Applied.journal` is empty when the apply's own restore routine cannot load
   the journal**, though the file holds the entries the apply saved in step 3. Same case as `p2-ownership-apply`
   Found outside scope 3 (the core must not read an empty `journal` after a failed load as "nothing owned"). A cheap
   improvement for the apply: in `undo`, when the routine's load failed, report the step 3 entries (the most the file
   can hold). The report property here excuses a failed load, as the contract says.
3. **Finding 3 (doc): the restore routine has the apply's accepted window too.** A user change between restore step
   3's read and step 4's write of `original` is replaced by `original` (shown with the harness: a change at the
   restore's call 4 or 5). Section 5, "Ownership", names only "a user change between Apply steps 2 and 5". Proposed:
   "a user change between Apply steps 2 and 5, or between restore steps 3 and 4, a few calls apart, is overwritten".
   The user-change property here excuses exactly a change that the same routine then overwrote.

No failure in another card's file during this card's runs (two release runs rebuilt after other cards' edits and
passed).

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" item 8: keep at least one start
  with E1 as the only entry on its plan. With E2 or E3 beside it, the next restore's `set_active` for them hides a
  dropped E1. Read `p2-ownership-apply`'s Result for the apply's take-over rule (an entry is taken over only while
  it is still ours).
- 2026-11-03 21:38, worker: started. Read the card, CLAUDE.md, plan "How we work".
- 2026-11-03 21:48, worker: read architecture section 5 (as updated 20:51), 6, 9; the reviews named; the ownership
  code and both cards' Results. Model: three plans, a start that may hold a crashed tray's entries (E1 alone among
  them) or a newer version's entry; ops with one fault and one interleaving each; invariants checked after every
  call through a test-only wrapper.
- 2026-11-03 21:56, worker: `ownership/props.rs` written, 7 properties; first run: all pass in 0.4 s. Next: a
  coverage tally, then the long run and the mutants in a scratch copy.
- 2026-11-03 22:02, worker: tally of 3000 cases: about 19K applies (6.7K wrote a target, 3.3K PlanChanged, 1.4K
  SafetyNet, 650 Write), 1.9K stops, 1.1K OtherOwner, 470 user-change reports. Long run (20000 cases, release):
  all 7 pass in 2.7 s. Found by reasoning, then shown with the harness: finding 1 (a failed apply whose restore
  routine fails too leaves an entry never written; the next check reports it as the user's). The property excuses
  exactly that case; an ignored named test keeps the strict form. Next: a denser fault search and the mutants, in a
  scratch copy.
- 2026-11-03 22:19, worker: added two properties: each routine's reported `journal` holds every held value's entry
  (what the core's mirror copies), and a key the user last changed between routines keeps that value. Dense search
  in the scratch copy (every routine with a fault and a user action, 100000 cases, with and without stops): all 9
  pass; with the finding 1 excuse taken out, the user-change property fails in about 4000 cases (shrunk case in the
  Result). Mutant runs started.
- 2026-11-03 22:22, worker: first mutant round (default 256 cases): 1, 2, 3b, 4, 5, 6 each failed properties; 3a
  (takeover resets the mark) survived, and 1 slipped past `never_drops` by sampling (caught by two others). Both
  need a rare start or a fault on one exact call. Generator biased: E1 alone on the active plan in about a quarter of
  the starts, fault and interleaving points mostly in the first 18 calls. The user-change property now excuses a
  change only when the same routine then wrote that key. Second mutant round running.
- 2026-11-03 22:25, worker: second round: all 13 mutant runs (the 7 of the card, 3 split in 3a and 3b, and 5
  extra) fail at least one property. 3a then survived one of three reruns, so a focused property was added: E1
  alone on the active plan, an apply that takes it over under a fault, every property checked; 3a failed it in 3
  of 3 runs. Finding 3 shown with the harness: a user change between restore steps 3 and 4 is overwritten by
  `original`. Next: the acceptance checks in the repo.
- 2026-11-03 22:31, worker: acceptance checks done (Result). One clippy fix (a redundant closure). Research note
  added. Build folders and the scratch copy deleted. Status done.

## Result

Done. `ownership/props.rs`: a `proptest` model over `FakePlans` that runs the real `apply`, `check` and `restore`,
written from sections 5 and 6. Ten properties pass; one ignored test keeps finding 1's strict form. All 13 mutant
runs fail at least one property. Three findings, none fixed here (Found outside scope 1 to 3).

Files changed: `crates/wakeshot-core/src/ownership/props.rs`, this card, `docs/research.md` (new section "Journal
property test checks", addition only). No `proptest-regressions/ownership/` folder: persistence is off, as in the
other property tests.

### The model

- World: three plans (P0 active; P1 and P2 may be deleted while inactive), each key's user value from a set holding
  the target. A start may hold a crashed tray's entries in each state a stop leaves (ours; never written; marked with
  ours stored; marked with `original` stored, E1), E1 alone on the active plan in about a quarter of the starts, and
  maybe a newer version's entry for an unknown key.
- Operations, up to 39: apply (active plan or a stale id, any key order and subset, rarely empty), user change, plan
  switch, plan delete, check, restore (`None` or `Some(plan)`, maybe as the guard of the tray before, so with
  `expect_owner`). Each routine carries at most one fault (a failed call with a code, or a stop) and one user action
  (change, switch or delete) at the start of one of its calls. After a stop the next routine runs in a new tray.
- Ending: one restore with no fault; in half the cases as a guard retry: first a guard started for another tray
  (must load and leave everything as it was), then the guard of the file's owner.
- A wrapper (`Probe`) over both seams checks the call-level properties after every call, so every stop point is
  covered, not sampled. Shrunk cases print one operation per line, then the case as it ran with each outcome.

### Properties (`ownership::props`)

1. `never_drops_a_held_value`: after every call, every held value (stored by our override, or in force on the active
   plan) has a known entry in the file.
2. `the_final_restore_puts_everything_back`: after the ending nothing is held; every key the user never changed is
   back at its original, stored and in force on the active plan; the file is gone (or holds only the newer version's
   entry); RunOnce is clear and the guard's stop event set; the stranger guard leaves the file alone.
3. `a_known_entry_never_lives_without_run_once`: after every call.
4. `safety_net_writes_nothing`: an apply that reports `SafetyNet` made no write or `set_active` call and changed no
   value.
5. `set_active_only_on_the_active_plan`: checked at each call.
6. `no_false_user_change_without_a_stop`: until the first stop, each user change the check reports is one the user
   made since Wakeshot's last write of that key.
7. `one_entry_per_plan_and_key` (added): after every call.
8. `the_report_never_drops_a_held_value` (added): each routine's reported `journal`, which the core's mirror copies,
   holds every held value's entry, but after a failed journal load (reported empty by contract).
9. `a_users_change_is_never_undone` (added): after the ending, a key whose last user change no write of the same
   routine overwrote, to anything but the target, still has that value.
10. `e1_alone_survives_a_takeover_with_any_fault` (added): every property, on cases that start with E1 alone on the
    active plan and open with an apply that takes it over under a fault on one of its first 18 calls.

Ignored: `a_failed_apply_whose_restore_fails_reports_no_false_user_change` (finding 1, strict form; it fails today).

### Design choices (each with its "change it when")

1. Invariants after every call, not only after each operation. Change it when a routine must pass through a state
   between two calls that breaks one on purpose (none found).
2. One fault and one user action per routine; a plan switch right before `set_active` is not generated (section 5
   reads the id right before the call; no routine can close that race). Change it when two faults in one routine
   must be covered in general (finding 1 is such a case).
3. Fault and action points: four in five within the first 18 calls, the rest up to 32. Change it when the routines
   grow longer (an apply that runs the restore routine makes about 30 calls).
4. Excuses, each named in the code: property 6 skips a stop's leftovers, the start's never-written entries and
   finding 1's entries; property 9 skips a change the same routine then overwrote (the accepted windows) and a change
   to the target (section 5: not seen); property 2's RunOnce and guard check allows a leftover with no journal only
   after a stop or a failed clean-up call (walk R6); property 8 skips a failed load. Change the finding 1 excuse when
   finding 1 is fixed.
5. `failure_persistence: None`, as in the other property tests: a shrunk case that finds a bug becomes a named test.
   Change it when the crate keeps regression files under review.

### Checks

- `cargo test --locked -p wakeshot-core ownership::props` (default 256 cases): 10 passed, 1 ignored; test run 0.46 s,
  6.5 s with cargo (well under 30 s).
- Long run, `PROPTEST_CASES=20000 cargo test --locked -p wakeshot-core ownership::props --release`: 10 passed, test
  run 3.75 s (the build before it about 1.5 min).
- Extra, in the scratch copy: a dense search (every routine with a fault and a user action, 100000 cases per
  property, release), with and without stops: all pass. With finding 1's excuse removed, property 6 failed after
  about 4000 cases (the shrunk case in Found outside scope 1).
- Mutants, each planted by hand in a scratch copy of the workspace (note 20's list with `docs\`), own build folder,
  default case count; the property named first gave the shrunk case shown:
  1. a failed plan list counts as "plan deleted" (restore): `never_drops`, `the_report_never_drops`,
     `the_final_restore`. Shrunk: `apply [sleepDc]; restore all, call 2 (plans) fails` -> held P0 sleepDc with no
     entry. (An earlier, unbiased run caught it only in the latter two.)
  2. `original` written before the mark is saved (restore): `never_drops`, `the_report_never_drops`. Shrunk: a start
     with entries left on P2; `apply [sleepDc]; restore all, call 8 fails` -> held P0 sleepDc with no entry at the
     end.
  3. a. the takeover clears the mark (apply): `e1_alone_survives...` in 3 of 3 runs; the general properties caught it
     in 2 of 3 (`never_drops`). Shrunk: E1 alone on P0 (lidAc); `apply [lidAc sleepDc], call 9 (write lidAc) fails`
     -> the routine drops E1, ours stays in force with no entry (final check major 1).
     b. the takeover resets `original` to the live value (apply): `never_drops`, `the_final_restore`,
     `the_report_never_drops`, `a_users_change_is_never_undone`. Shrunk: `apply [lidAc], stop at call 8; apply
     [lidAc]`.
  4. the restore drops marked entries when `set_active` fails: `never_drops`, `the_report_never_drops`. Shrunk: E1
     on P0; `restore all, call 4 (set_active) fails`.
  5. the apply calls `set_active` with the step 1 id (the second read's result ignored):
     `set_active_only_on_the_active_plan`. Shrunk: `apply [sleepDc], user switches to P1 at call 2` -> `set_active
     P0` while P1 is active.
  6. the check drops an entry on a read error: `never_drops`, `the_final_restore`, `the_report_never_drops`,
     `no_false_user_change`. Shrunk: `apply [lidDc]; check, call 2 (read) fails`.
  7. the clean-up clears RunOnce before it deletes the journal: `a_known_entry_never_lives_without_run_once`. Shrunk:
     `apply [lidDc]`, then the ending restore.

  Extra mutants, all caught: 8. restore step 5 judged by step 2's plan read: `set_active_only_on_the_active_plan`
  (a switch before the step 5 call); 9. Apply step 3 read literally (every existing entry keeps its `original`):
  `a_users_change_is_never_undone` (on P1: `apply [lidDc]; user sets lidDc = 1; apply [lidDc]`, then the restore
  puts back 2); 10. the journal saved before the RunOnce value: `a_known_entry_never_lives_without_run_once`,
  `no_false_user_change`; 11. the check reads marked entries: `no_false_user_change`, `never_drops` and two more
  (E1 alone, one check); 12. the apply's failure path without the restore routine: `no_false_user_change`. No mutant
  survived, so no property is missing for them.
- `cargo test --locked -p wakeshot-core`: 512 passed, 1 ignored (plus 10 in `tests/claude_status.rs`).
- `rustfmt --edition 2024 --check crates/wakeshot-core/src/ownership/props.rs`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: exit 0 (all four, after one fix: a
  redundant closure).
- Only `ownership/props.rs` changed in code. The build folder, the scratch copy, its build folder and the scripts
  were deleted at the end.

### Open items

- Findings 1 to 3 (Found outside scope), for the orchestrator to route: 1 to the apply (with a section 5 line), 2 to
  the apply or `p2-core-settings`, 3 to `docs/architecture.md`.
- For the power safety review: the excuses in design choice 4 are where this test accepts the design's gaps; each
  names its reason in the code.
