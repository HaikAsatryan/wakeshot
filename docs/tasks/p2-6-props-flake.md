# Find and fix the core property that failed once on SleepDc

Phase: 2 to 6 (core). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Written by the orchestrator (session `wakeshot-27`, 2026-11-04) from `docs/tasks/p2-6-docs-pass.md` "Found outside
scope" item 1.
Runs after: `p2-6-sleep-route-fixes` (done), docs pass round 6 (done). Parallel with: cards that own none of the files
below (none run now).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-props-flake\target`.

## Goal

`scenarios::props::a_released_key_is_never_ours_in_force_again_in_its_episode` failed once at the default 256 cases
in docs pass round 6: "SleepDc was released in this episode, yet ours is in force on 381b4222-..., at run 300000 ms".
One rerun at 256 and five runs at 4096 cases passed. The shrunk input was lost (`failure_persistence: None`). CI runs
this property on every push, so a rare failure makes CI flaky. Find the failing case, decide whether the core or the
property is wrong, and fix the one that is wrong.

## Read first

- `docs/tasks/p2-6-docs-pass.md`, "Found outside scope" item 1 (the failure text).
- `docs/research.md` around "Long property runs" (line 3110 or so): the same property failed 3 of 7 runs at 10,000
  cases before; that cause was in the property's "restore failed" flag. Check that this is not the same cause back.
- `docs/tasks/p2-6-sleep-route.md` and `docs/tasks/p2-6-sleep-route-fixes.md` Results: SleepDc is both a hold key and
  one of the four short keys of the sleep route, the likely area (inferred, not checked).
- `docs/product.md` "User settings safety" rules 1 to 7: what "released" and "ours in force" must mean.

## Deliverables

1. Catch the case. Run the property alone in a loop with full output kept to a file (release build is fine for the
   hunt), a larger `PROPTEST_CASES`, and a fixed or logged seed, until it fails or until about 40 runs at 4096 cases
   pass. Cap memory: one test thread for the hunt, stop if free memory drops under 10 GB. Keep the shrunk case.
2. Decide the cause in a few sentences in the Result: a core bug (a released key written again as ours in the same
   episode: Haik's rule is that a user's own change is never overwritten) or a property that checks the wrong thing
   (for example a short key that the sleep route takes on purpose in a new step of the same episode). Use
   `product.md` to decide which one is right.
3. Fix it in the leanest way. Keep the shrunk case as a named test in "Shrunk cases kept as named tests" in
   `props.rs`. If it is a core bug, also add one scenario in the matching `scenarios/` file.
4. If no failure shows in about 40 runs at 4096 cases: say so with the run counts, read the property and the sleep
   route code for the path the message points to (SleepDc released, then ours in force at run 300000 ms, that is 5
   minutes in), and either fix the path you find with a test that shows it, or report that none was found. Do not
   weaken the property to make it pass.

## Files this card owns

`crates/wakeshot-core/src/scenarios/props.rs`, and whatever core files the cause needs (`crates/wakeshot-core/src/`).
No other crate, no docs outside this card, except a line in `docs/research.md` if a fact about the core changes.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets` on x64; `cargo test --workspace --locked` (debug
  only, never `--release` for the whole workspace); the core props at 4096 cases three times.
- Mutants only if you change core code: one job, a low timeout, a memory cap; stop if free memory drops under 10 GB.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- Any product rule change. If `product.md` does not decide the case, record your choice as "decided by the worker,
  for Haik's review".

## Questions for Haik

None. Decide yourself and record each choice as "decided by the worker, for Haik's review".

## Found outside scope

1. **Stale doc comment** (`crates/wakeshot-core/src/ownership/fake.rs`, `FAKE_DEFAULT_OTHER`): "no routine reads them
   today, since a lost journal resets only the hold keys". The reset covers all six keys since
   `p2-6-sleep-route-fixes` (`ownership/reset.rs`, `KEYS = SettingKey::ALL`). Read in the code; not fixed.
2. **Not verified, inferred from reading `done_reset`:** on cause 2's path (a reset whose own load fails right after a
   running hold's apply wrote entries), a reset that is not for a damaged file sends no apply again, so the hold may
   run on with the plan's defaults (lid Sleep) until the next apply edge. It needs a good load and a failed load back
   to back; rare. A core worker could check it with one scenario.
3. **Not verified, inferred from reading `take_back`:** in a stepped-back episode where the other hold keys are
   released, a lid key released while a short apply is out goes to the "every key released" branch, whose
   `Restore { plan: None }` also takes back the short timeouts while the countdown still wants them. Cost: no idle
   sleep, the fallback hibernate 5 minutes later. Needs a failed restore that left a lid entry; rare.

## Log

- 2026-11-04 14:13: started. Read the card, the docs pass item, research "Long property runs", the property and its
  model in `props.rs`. proptest is 1.12.0.
- About 14:16: hunt script in the scratchpad: release test binary, `PROPTEST_CASES=4096`, `PROPTEST_RNG_SEED=<n>`, one
  test thread, full output to a file per seed, free memory checked before each run. Seed 1 failed at once (after 4051
  successes), same text: "SleepDc was released in this episode, yet ours is in force on 381b4222-..., at run 300000 ms".
  Shrunk case (start on Balanced, lid open, battery 25 %, automatic off, not owner, off console, IO keeps up): lid
  closes; console back; user switches to docked plan; menu Keep awake 30 min; settings mutex taken; run 74.186 s; run
  121.826 s; run 103.988 s; user sets the active plan's SleepDc = 300; user switches to Balanced; end LastHold. Trace:
  docked plan floor 30 % so step back; at 300 s the sleep route applies the four short keys on the docked plan; the user
  sets SleepDc there and switches to Balanced; the plan read carries `Plan(Balanced)` before `UserChanged(docked,
  SleepDc)`, so the core sends the plan switch restore and a short-key Apply on Balanced (with SleepDc) before it learns
  of the release; the release then comes, and nothing takes SleepDc on Balanced back.
- About 14:20: seeds 2 to 40 on the same binary, keep going: 11 more failures (12 of 40 seeds fail at 4096 cases, far
  more than the docs pass saw). Shrunk cases differ: most are the short apply on its way when the release comes (with a
  plan switch, or with the IO lagging: the user sets SleepDc over our hold's Never before the floor restore ran, the
  short apply then takes it). Seed 33 (240000 ms) looks different: a lost journal's reset over entries. Seed 6 (360000
  ms) not read yet. Plan: fix the short apply race in the core, rerun the failing seeds, read what is left.
- About 14:24: core fix: `take_back` (core/settings.rs) now also covers the short timeouts: a short key released while
  they are on, with an apply on its way or the key in the journal, sends `short_again` (core/lid.rs): the short apply to
  the active plan without the released key, which restores that plan's entry for it, or a restore when nothing is left
  to write. `take_user_changed` calls `take_back` also for a new short release in a work-end countdown (no episode).
  Rerun of the 12 failing seeds: 10 pass; seeds 29 and 33 fail with a second cause at 240000 and 360000 ms.
- About 14:27: second cause (seeds 29, 33): a lost journal's reset runs after the hold's apply wrote entries, and its
  own load fails (a fault on the next restore-kind call), so it writes battery sleep's default (180) over our Never and
  sets the file aside, by design (rule 7). The fake labels that write `Override`, since the file still named the key
  when the reset wrote (writes come before the set-aside), while the sim already counts the value as the user's own.
  `restored_at_the_end` fails on the same case ("still held at the end"). So the world's label is wrong, not the core
  and not the property: `FakeState::mark_reset` (ownership/fake.rs, an addition) relabels a reset key `Default`, and the
  sim's `ResetLost` runner calls it. Named tests: two short apply cases (seeds 1 and 22) and the reset case (seed 29),
  plus one scenario in `scenarios/lid.rs` for the work-end countdown. All four pass.
- About 14:29: each change switched off by hand, then put back: no short branch in `take_back` fails the two short props
  tests and the scenario; no take back for a work-end short release fails the scenario; no `mark_reset` fails the reset
  test. All three restored.
- About 14:31: second scenario in `scenarios/lid.rs`: every short key released while a short apply is on its way
  restores (the mirror of the hold's `every_key_released_while_an_apply_is_on_its_way_restores`); it kills the mutant
  that drops `short_again`'s restore, which nothing else did. Restored. `cargo fmt --all --check` clean.
- 14:38: the times above from "About 14:16" on were first written as guesses; corrected to estimates from the order of
  work (this line read from the clock). Hunt on the fixed code, seeds 1 to 100 at 4096 cases, one thread: 100 of 100
  passed (377 s). clippy with and without `--all-targets` on x64: exit 0.
- 14:45: acceptance checks run (see Result); two more hand mutants on `mark_reset`, both killed and reverted; Haik's
  plan read back unchanged; scratch folder with the build folder deleted.

## Result

**Caught.** The property failed at once with fixed seeds: 12 of 40 seeds at 4096 cases (seed 1 gave the docs pass
text exactly; about 2 % of 256-case CI runs, inferred from that rate). Two causes, neither the old "restore failed"
flag.

**Cause 1, a core bug** (the docs pass failure; product "User settings safety" 3 and architecture "The sleep route"
4: a user's change to a short key is released as a hold's is). A sleep route apply sent before the core heard of the
release wrote the released key, and nothing took it back: `take_back` covered only a hold's apply. Two orderings: a
plan switch's read posts `Plan` before its check's `UserChanged`; or, with the IO behind, the user sets battery sleep
over the hold's Never before the floor restore runs. Fix: a short key released while the short timeouts are on, with
an apply out or the key in the journal, sends the short apply again without it (which restores that plan's entry for
it), or a restore when no short key is left; also in a work-end countdown, which has no episode and which the
property does not check.

**Cause 2, the test world was wrong** (seeds 29, 33): a lost journal's reset whose own load fails runs after a hold's
apply wrote entries and writes the defaults, by design (rule 7). It writes before it sets the file aside, so the fake
labeled battery sleep's default `Override`, while the sim already counted it as the user's own;
`restored_at_the_end` failed on the same case. The sim now marks a reset key `Default`. The property is unchanged.

Decided by the worker, for Haik's review: cause 1 is fixed in the core, not by excusing the property; the take back
also covers the work-end countdown. Change it when the IO side posts a check's `UserChanged` before its `Plan`, or
an apply learns to skip keys released since it was sent; then the race is gone at the source.

**Files:** `crates/wakeshot-core/src/core/settings.rs` (`take_user_changed`, `take_back`),
`crates/wakeshot-core/src/core/lid.rs` (`short_again`), `crates/wakeshot-core/src/ownership/fake.rs`
(`FakeState::mark_reset`, an addition), `crates/wakeshot-core/src/sim.rs` (the `ResetLost` runner calls it),
`crates/wakeshot-core/src/scenarios/props.rs` (three named tests: seeds 1, 22, 29),
`crates/wakeshot-core/src/scenarios/lid.rs` (two scenarios), `docs/research.md` ("The release property's two
failures"), this card.

**Checks:** `cargo fmt --all --check` exit 0. Clippy x64 with and without `--all-targets`, `-D warnings`: exit 0.
`cargo test --workspace --locked` (debug, from PowerShell): exit 0, core 666 passed, 1 ignored. Core tests at 4096
cases, random seeds: 3 of 3 passed. The property alone, seeds 1 to 100 at 4096: 100 passed. Mutants by hand, 6 of 6
killed (no `cargo-mutants`, so no memory risk). Haik's plan read back: Balanced, lid Sleep, sleep 18000 s, AC and DC.
No tray started; scratch build folder deleted.

**Open:** three items under "Found outside scope" (one stale comment, two inferred and not verified).
