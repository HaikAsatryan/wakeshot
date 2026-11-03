# Restore routine, journal file form and the ownership fake

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.

## Result

What was done, files changed, checks and outcomes, open items. Include the final public API of `restore.rs` and
`fake.rs` for the power, sources and ship tracks.
