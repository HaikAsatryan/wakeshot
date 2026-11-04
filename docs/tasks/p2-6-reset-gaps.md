# Check two rare core gaps found next to the release property fix

Phase: 2 to 6 (core). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Written by the orchestrator (session `wakeshot-27`, 2026-11-04) from `docs/tasks/p2-6-props-flake.md` "Found outside
scope" items 1 to 3.
Runs after: `p2-6-props-flake` (done). Parallel with: cards that own none of the files below (none run now).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-reset-gaps\target`.

## Goal

The worker of `p2-6-props-flake` read two rare paths in the core that look wrong (inferred, not verified). Prove or
disprove each one with a scenario. Fix the ones that are real, in the leanest way. Haik's rules: with no work going
on, the laptop behaves as if Wakeshot were not there; a user's own change is never overwritten; a closed laptop never
runs unlocked with no path to sleep or lock.

## Read first

- `docs/tasks/p2-6-props-flake.md`: "Found outside scope" and Result (cause 1 and cause 2, the files they touched).
- `docs/product.md` "User settings safety" rules 1 to 7, and the work end rule.
- `docs/architecture.md` "The sleep route".

## Deliverables

1. Gap A (`done_reset`): a lost journal's reset whose own load fails right after a running hold's apply wrote entries.
   A reset that is not for a damaged file sends no apply again, so the hold may run on with the plan's defaults (lid
   Sleep) until the next apply edge. Write one scenario for it. If the hold does run on without its values, fix it
   (for example: the reset asks for the apply again while a hold holds) and keep the scenario.
2. Gap B (`take_back`): in a stepped-back episode where the other hold keys are released, a lid key released while a
   short apply is out goes to the "every key released" branch, whose `Restore { plan: None }` also takes back the short
   timeouts while the countdown still wants them. Cost: no idle sleep, the fallback hibernate 5 minutes later. Write one
   scenario. If it happens, fix it so the short timeouts stay while the countdown wants them, and keep the scenario.
3. The stale doc comment on `FAKE_DEFAULT_OTHER` in `crates/wakeshot-core/src/ownership/fake.rs`: the reset covers all
   six keys since `p2-6-sleep-route-fixes` (`ownership/reset.rs`, `KEYS = SettingKey::ALL`).
4. For each gap, one or two sentences in the Result: real or not, and why. If a gap is not real, keep its scenario
   only if it guards a path no other test covers.

## Files this card owns

`crates/wakeshot-core/src/core/` (any file), `crates/wakeshot-core/src/scenarios/` (any file),
`crates/wakeshot-core/src/ownership/fake.rs`, `crates/wakeshot-core/src/sim.rs`, this card, and a line in
`docs/research.md` or `docs/architecture.md` if a core fact changes. No other crate.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets` on x64; `cargo test --workspace --locked` (debug
  only, never `--release` for the whole workspace); the core props at 4096 cases three times, and the release
  property alone at seeds 1 to 100 as `p2-6-props-flake` ran it.
- Mutants by hand on the lines you change (no `cargo-mutants` run).
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- Any product rule change. If `product.md` does not decide a case, record your choice as "decided by the worker, for
  Haik's review".

## Questions for Haik

None. Decide yourself and record each choice as "decided by the worker, for Haik's review".

## Found outside scope

1. **Not verified, inferred from reading `menu_restore` and from the retry scenario's last part:** "Restore my power
   settings" during a work-end countdown with the short timeouts on takes them back, but `state.short` stays, so the
   core still counts the fallback hibernate 5 minutes from their write. A click in the menu is input, which ends them
   the normal way, so in practice only a `wakeshot restore` with no input since (a script) would meet the hibernate.
   Whether the user's restore should end the sleep route for this countdown is a product question; `product.md` rule 3
   speaks of "until the current work ends", and the countdown runs after the work ended.
2. **A second rare props failure, not from this card (verified: the same case fails with this card's three call sites
   taken out).** `a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s` failed in 1 of the 3
   core runs at 4096 random cases ("ran unlocked on battery on our Do nothing for 44000 ms, until at run 55000 ms").
   Shrunk case: start on the other plan, lid open, battery 80 %, IO keeps up; agent 1 working; IO call +6 of the next
   command fails (code 2); user sets the active plan's SleepDc = 300; user switches to the docked plan; user sets the
   active plan's LidDc = 1; run 10 s; lid closes; agent 1 idle; agent 1 working; run 45 s; end LastHold. My reading
   (inferred, not verified further): the plan switch's restore fails, its retry `Restore { plan: Some(other) }` works
   at 10 s and the core ends the retry (nothing left on other plans, `done_restored`), but the props `Model` ends its
   `retry` flag only on a restore of every plan, so with an empty mirror after the user's LidDc change it thinks the
   journal is unknown at the lid close, owes the window lock, and blames the core when the restarted hold applies Do
   nothing with the lid already closed. The core's view matches architecture "A failed `Restore`". In real life the
   lid close on the user's own Sleep sleeps the laptop, which the world does not model. Likely fix: the model clears
   `retry` as the core does; a props card should check it.
3. **Two more rare props failures, not from this card (verified the same way: both shrunk cases fail with the three
   call sites taken out).** Three more core runs at 4096 random cases gave run 4 and run 5 one failure each, run 6
   none. So 3 of 6 runs at 4096 failed on some property, which makes CI at 256 cases flaky now and then (rate not
   measured). Not analyzed in depth:
   - `a_closed_laptop_is_locked_also_before_the_first_plan_read` ("ran unlocked on battery on our Do nothing for
     39000 ms, until at run 40000 ms"). Shrunk case: start on the docked plan, lid open, battery 50 %, IO keeps up;
     IO call +0 of the next Restore fails; IO call +12 of the next Restore fails; `run-4001.json` alive, then gone;
     agent 2 working; user switches to Balanced; lid closes; agent 2 idle; session end cancelled; agent 1 working;
     run 40 s; end LastHold. Same family as item 2: the lid closes while nothing of Wakeshot's is in force (the
     episode's apply failed), a new hold applies Do nothing with the lid already closed, and no lock comes.
   - `no_hibernate_within_5_minutes_of_a_resume_or_of_input` ("Hibernate after only 10000 ms without input or a
     resume, at run 310000 ms"). Shrunk case: start on Balanced, lid open, battery 10 %, not owner, IO lags; settings
     mutex taken; lid closes; agent 1 working; AC in; IO flush; AC out; run 286.616 s; run 13.384 s; session ending;
     input; run 10 s; end LastHold. From the trace: the floor countdown's `SleepCheck` answer arrives after the input
     and the session end, the short timeouts cannot be written then, so the fallback is due at once and `Hibernate`
     goes out 10 s after input; the hibernate thread's own check then skips it (`InputSeen`), so nothing hibernates.
     Core or property: not decided here.

## Log

- 2026-11-04 14:47: started. Read the card, `p2-6-props-flake` (outside scope items 1 to 3, Result) and the plan's
  "How we work".
- About 14:50: read product "User settings safety" 7, architecture "The sleep route" and the unknown journal part,
  `core/settings.rs` (`done_reset`, `take_back`, `restore_scope`), `core/lid.rs` (the short timeouts),
  `ownership/reset.rs`, the World in `sim.rs`. Plan: a World scenario per gap in `scenarios/props.rs` (next to the
  sleep route's review major 2 cases, which show what Windows holds, not only the commands). While reading `take_back`:
  the tick's restore retry has the same shape as gap B (`restore_scope` gives `Restore { plan: None }` whenever the
  core does not apply, short timeouts or not); to check with the same scenario.
- About 14:53: gap A is real. World scenario in `scenarios/props.rs`
  (`a_lost_journals_reset_over_a_running_holds_values_applies_the_hold_again`): journal unread at start, a hold, its
  apply runs with a late answer, the reset goes out at 120 s, its load fails. Result on the current core: the agent
  still works, yet Balanced has lid Sleep (1) on AC and battery and battery sleep 180 s, all `Default`, still at 181 s;
  no apply after the reset's answer.
- About 14:56: gap B is real, and so is the retry path next to it. Two scenarios in `scenarios/lid.rs`: (1) the floor
  restore fails and keeps failing, the user takes back LidAc and SleepDc, the countdown's short apply goes out, the user
  takes back LidDc: `take_back` sends `Restore { plan: None, WorkEnded }`; after its `Ok` nothing writes the short
  timeouts again (next deadline +300 s: the fallback). (2) The work end's restore fails until the short timeouts are
  written; the next tick's retry `Restore { plan: None }` takes them too, and nothing writes them again. Gap B's
  precondition (a lid entry left 5 minutes after the step back) means a retry is pending anyway, so a fix of gap B alone
  would still lose the short timeouts to the retry.
- About 14:58: fix design, decided by the worker: one place for both paths. When a restore of every plan answers `Ok`
  while the countdown still wants the short timeouts (`state.short`), not for the user's own restore, and no short
  apply is out (one still out runs after the restore: answers come in order), the short timeouts go out again
  (`Core::rewrite_short`, `core/lid.rs`; called from `done_restored`). Gap A: `done_reset`'s known-and-empty branch
  asks for the hold's apply again (settle drops it when the core does not apply) and calls the same `rewrite_short`.
  Rejected: scoping the retry and `take_back` to spare the active plan's short entries; a restore cannot leave out
  keys, so it would need the short apply to take over the active plan's entries and per-plan restores for the rest,
  more code and more cases for the same result.
- About 15:01: fix in. `core/lid.rs` `rewrite_short`; `core/settings.rs` `done_restored` calls it after an `Ok` restore
  of every plan (not `UserAsked`); `done_reset`'s known-and-empty branch asks for the hold's apply (unless a hold
  apply is out, which runs after the reset) and calls `rewrite_short`. Doc comments of `done_reset`, `take_back` and
  `restore_scope` follow. Scenarios, all pass: World gap A (`props.rs`); `lid.rs` gap B, the retry path (with the user's
  own restore and the order where the retry runs before the short apply), and gap A for the short timeouts (a plan
  switch whose old plan's restore fails before the applies answered); `settings.rs` a damaged file's second reset that
  answers before the apply the first one sent behind it (no second apply). `FAKE_DEFAULT_OTHER` doc fixed.
- About 15:03: dropped `plan.is_none()` from the `done_restored` call: no reachable order lets a scoped restore answer
  while the short timeouts are on and no short apply is out (a plan switch sends its restore before the new plan's short
  apply), so the condition could not be tested; a restore that worked is enough. Two assertions added for mutants: a
  failed retry writes nothing again; `every_short_key_released_while_a_short_apply_is_on_its_way_restores` now answers
  everything in order and checks no apply follows. Core lib tests: 671 passed, 1 ignored.
- About 15:05: hand mutants by a scratch script (one at a time, file restored byte for byte after each): 10 of 10
  killed (`rewrite_short`'s three guards, the `done_restored` call and its two conditions, `done_reset`'s apply ask, its
  apply-out skip and its `!a.short`, and its `rewrite_short` call).
- About 15:08: `cargo fmt --all --check` exit 0; clippy x64 with and without `--all-targets`, `-D warnings`: exit 0.
  Haik's plan before the workspace tests (read only: `powercfg /q`, the lid key from the registry): Balanced active,
  lid Sleep (1) AC and DC, sleep after 18000 s AC and DC, screen off 300 s AC and 180 s DC. `cargo test --workspace
  --locked` (debug): exit 0, core 671 passed, 1 ignored; every other binary passed.
- 15:09:40 (from the script's own clock line): the long property runs started (script in the scratchpad, release test
  binary of the core only). About 15:10: architecture "The sleep route" item 4 and "A journal that cannot be read" each
  got one sentence on the new behavior.
- 15:10:41, read from the clock: the times from 14:50 on were first written as guesses, up to an hour too late;
  corrected to estimates from the order of work.
- About 15:11 to 15:14 (15:14:05 from the clock at the end): core release test binary at 4096 random cases, 8
  threads: run 1 passed (671), run 2 failed one test
  (`a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s`), run 3 passed. Kept the shrunk
  case as a temporary named test, ran it with this card's change and with its three call sites taken out (scratch
  script, file put back byte for byte): fails the same both ways, so not this card's. Temporary test removed; recorded
  under "Found outside scope" item 2.
- 15:18:06 (script clock): the release property alone, seeds 1 to 100 at 4096 cases, one test thread: 100 of 100
  passed, each log "1 passed". Free memory stayed about 35 GB.
- 15:19:14 (clock): three more core runs at 4096 random cases with the same binary: run 4 and run 5 failed one other
  property each, run 6 passed. Both shrunk cases checked as for run 2 (temporary tests, scratch script): they fail the
  same without this card's change. Temporary tests removed, `cargo fmt --all --check` exit 0. "Found outside scope"
  item 3.
- About 15:20: `docs/research.md` "Two rare core gaps next to the release property fix" added. Core tests (debug) and
  core clippy `--all-targets` again after the temporary tests went: 671 passed, 1 ignored; exit 0. Haik's plan read
  back unchanged (Balanced, lid Sleep AC and DC, sleep 18000 s AC and DC, screen off 300 s and 180 s); no Wakeshot
  process runs. 15:21:40: build folder and every scratch file of this card deleted.

## Result

**Gap A, real.** A lost journal's reset whose own load fails right after a running hold's apply wrote puts the hold's
values to the defaults, and nothing applied them again: in the World scenario the agent still works while Balanced
holds lid Sleep and battery sleep 180 s, a minute later too. Fix: the reset's answer with no entries asks for the
hold's apply again (settle drops it when the core does not apply) and writes the short timeouts again, unless an apply
is still out, which runs after the reset. The same holds for the short timeouts (a scenario shows it).

**Gap B, real, with a second path of the same shape.** A restore of every plan sent while the countdown's short
timeouts are on takes them too: the take back of a released hold key (gap B) and the tick's restore retry, which gap
B's precondition always leaves pending. Cost as the card says: no idle sleep, the fallback hibernate. Fix, one place
for both: when a restore that is not the user's own works while the countdown still wants the short timeouts and no
short apply is out, they go out again (`Core::rewrite_short`). Decided by the worker, for Haik's review: repair after
the restore, not a scoped restore; between the two there is a moment of milliseconds with the user's own timeouts. A
restore that fails does not trigger it; the next retry that works does. Change it when a restore can leave out keys,
then the retry can spare the short timeouts at the source.

**Stale comment** on `FAKE_DEFAULT_OTHER` fixed.

**Files:** `crates/wakeshot-core/src/core/lid.rs` (`rewrite_short`), `crates/wakeshot-core/src/core/settings.rs`
(`done_reset`, `done_restored`, three doc comments), `crates/wakeshot-core/src/ownership/fake.rs` (doc comment),
`crates/wakeshot-core/src/scenarios/props.rs` (World gap A), `crates/wakeshot-core/src/scenarios/lid.rs` (gap B, the
retry path, gap A for the short timeouts; `every_short_key_released_while_a_short_apply_is_on_its_way_restores` now
answers everything and checks no apply follows), `crates/wakeshot-core/src/scenarios/settings.rs` (no second
apply), `docs/architecture.md` (two sentences), `docs/research.md` (one entry), this card.

**Checks:** `cargo fmt --all --check` exit 0. Clippy x64 with and without `--all-targets`, `-D warnings`: exit 0.
`cargo test --workspace --locked` (debug): exit 0, core 671 passed, 1 ignored. Release property alone, seeds 1 to 100
at 4096: 100 passed. Core at 4096 random cases: 3 of 6 runs passed; the 3 failures are three other properties, each
shrunk case failing the same with this card's change taken out ("Found outside scope" 2 and 3). Hand mutants: 10 of 10
killed. Haik's plan never written, read back unchanged; no tray started; build folder deleted.

**Open:** three items under "Found outside scope": the menu restore during the countdown (product question), and three
rare props failures not from this card that make CI flaky now and then (a props card should look at them).
