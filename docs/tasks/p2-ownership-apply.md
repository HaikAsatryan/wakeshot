# The apply and the ownership check

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.

## Result

What was done, files changed, checks and outcomes, open items. Include the final public API of `apply.rs` and
`check.rs` for the power track.
