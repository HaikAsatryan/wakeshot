# Journal property test: no held value is ever dropped

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" item 8: keep at least one start
  with E1 as the only entry on its plan. With E2 or E3 beside it, the next restore's `set_active` for them hides a
  dropped E1. Read `p2-ownership-apply`'s Result for the apply's take-over rule (an entry is taken over only while
  it is still ours).

## Result

What was done, files changed, checks and outcomes, open items.
