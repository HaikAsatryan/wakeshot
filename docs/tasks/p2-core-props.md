# Core property tests over random event sequences

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: no (pure core tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-props\target`.
Runs after: `p2-core-lid` and `p2-ownership-apply`. Parallel with: any card of another track.

## Goal

The property tests of architecture section 9 over the whole core, end to end in pure code: a `Sim` world answers
`Apply`, `Restore` and `ReadPlan` by running the real ownership routines on `FakePlans`, with outcomes that arrive
late, so a property is checked against what Windows would hold, not only against the commands. Write them from the
design: you are a second pair of eyes on the three core cards.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (all track notes).
- `docs/product.md` (the job; power behavior; user settings safety).
- `docs/architecture.md` section 3 (all), section 5 "Power settings", "Ownership", "Lock and hibernate"; section 9
  core property tests (the list this card turns into tests).
- `docs/reviews/p1-design-power-verify.md` minors 4, 6, 9, 11; `p1-contracts-domain.md` M5.
- `crates/wakeshot-core/src/`: `sim.rs`, `ownership/fake.rs`, the contract files; Results of the core and ownership
  cards.

## Owns

`crates/wakeshot-core/src/scenarios/props.rs`, `sim.rs` (the world). Nothing else.

## Deliverables

1. In `sim.rs`, a world: a `FakePlans` the routines run on; the IO side's answers (`Apply` through
   `ownership::apply`, `Restore` through `ownership::restore`, `ReadPlan` through `ownership::check` plus the plan
   values, each `UserChanged` the check reports); answers delivered now or after later events (late outcomes); lid,
   AC, battery, input, suspend and resume, session and console changes, user changes and plan switches in the world,
   each turned into its event.
2. `scenarios/props.rs`, random sequences of world steps and events (hold files and Claude sessions coming and going,
   menu choices, ticks and time passing), one `proptest!` per property:
   - owned keys are always restored after the last hold, Quit and end of session (checked on `FakePlans`: stored and
     in force back at the user's values once the retries settle);
   - a released key is never written again in its episode, on any plan (power verify minor 9);
   - a restore retry never takes an entry a running hold uses (power verify minor 4);
   - no `Apply` after `Session(Ending)` until `EndCancelled` (power verify minor 6);
   - no `Apply` and no display request while another session is on the console;
   - no request is on with no hold, and none while stepped back;
   - no `Hibernate` within 5 minutes of a resume or of input;
   - no `Lock` when the lid action in force is the user's own;
   - every `Lock`, step back and user change comes with a notice or a sound in the same step; every `Hibernate`
     counts as its own explanation (track note 4); each run of failed restores gets one `RestoreFailed` (decision 0);
   - a closed laptop never runs on battery on our Do nothing unlocked for more than 30 s (power verify minor 11; settle 30 s, Haik 2026-11-03);
   - no `ApplyUpdate` while a hold counts, a hold file names a live process, or a key is owned (if `p2-core-lid` built
     the update item);
   - `handle` never panics, and `next_deadline` is never in the past after a `Tick`.
3. Shrinking gives readable cases: steps print as short lines.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::props`: all pass at the default case count, in under 60 s in total on
  this laptop (record the time).
- One long run: `PROPTEST_CASES=10000 cargo test --locked -p wakeshot-core scenarios::props --release`: passes (record
  time).
- Mutants, each made by hand in `core.rs` or `src/core/` of a scratch copy of the workspace, never in the repo: those
  files are the core cards' and other cards test against them (verify review Major 2; the copy, its build folder and
  the clean-up are in `docs/phases/02-core.md` note 20). Each must fail a property (record which, and the shrunk
  case): `released` keyed by plan; the retry takes the active plan's entries while holding; `Apply`
  after `Session(Ending)`; a lid-close lock with the user's own Do nothing; no lock at AC to battery with the lid
  closed; `UserChanged` without a notice; requests left on after the last hold.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo
  clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for
  `aarch64-pc-windows-msvc`: exit 0.
- A bug a property finds in another card's file: not fixed here; the shrunk case goes under "Found outside scope" and
  the Result says so. The orchestrator routes the fix.
- Only the owned files changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Fixing the core or the routines (findings are routed). The journal property test (`p2-ownership-props`).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed. Each was shown with this card's world; each keeps its strict property as an ignored test
in `scenarios/props.rs`.

1. **Finding 1 (minor; `core/settings.rs`, `p2-core-settings`): a key the user takes back can stay overridden for the
   rest of the episode.** Shrunk case of the ignored `a_released_key_is_never_ours_in_force_again_in_its_episode`:
   ```
   start on Balanced, lid open, on battery at 10 %, automatic-off, off-console, io-keeps-up
     1. console back
     2. menu KeepAwake(Minutes(30))
     3. AC in                                       (the step back ends: apply to Balanced)
     4. user sets the active plan's SleepDc = 300
     5. user switches to other plan
   -> SleepDc was released in this episode, yet ours is in force on the other plan
   ```
   The switch posts `ActivePlan`, the read posts `Plan(other)` and then `UserChanged(Balanced, SleepDc)` (the
   executor's order, `p3-power-io` design choice 4). The core handles `Plan` first: `Restore(Balanced)`,
   `Apply(other)` with all three keys; then `UserChanged` releases SleepDc, but the apply already on its way writes
   it, and nothing takes it back until the next apply (a later edge) or the work end. The notice says Wakeshot stopped
   overriding the key; product rule 3 ("on every power plan, until the current work ends") is broken. Two more ways
   in, from reading the code, not run as cases (not verified): a release that names an inactive plan only (an entry
   left there by a failed restore, then a value set with `powercfg`) leaves the active plan's override of that key;
   and an apply queued behind a read that drops an entry (a hold start right after a tick's read) takes the user's
   fresh value as `original` and writes over it.
   Proposed fix, in the core: when `take_user_changed` newly releases a key in an episode, and the copy holds an entry
   for that key or an apply is in flight, set the episode's apply to `Wanted`. The next `Apply` leaves the key out,
   and the apply restores this plan's entries for keys not in `writes` (track note 3). That covers all three ways in.
   The switch case alone would also go if the executor posted `UserChanged` before `Plan`; I would do the core fix.
2. **Finding 2 (minor; `core/lid.rs`, `p2-core-lid`): a lid close before the core's first plan read gets no lock,
   even with a crashed tray's Do nothing in force.** Shrunk case of the ignored
   `a_closed_laptop_is_locked_also_before_the_first_plan_read`:
   ```
   start on Balanced, lid open, on battery at 10 %, automatic-off, not-owner, leftover, startup-restore-fails, io-lags
     1. lid closes
     2. run 40.000 s
   -> the closed laptop ran unlocked on battery on our Do nothing for 40000 ms
   ```
   `Core::in_force` returns `None` before the first `Plan(Ok)` ("nothing was applied"), but a journal in `Started`
   means a crashed tray's values may be in force. The window runs from `Started` to the first read's answer: the
   first plan read deadline is due at once, so on the real loop about 10 ms plus the IO run (the case above needs the
   IO to lag). A lid close is an edge, so the lock never comes later. Proposed: with no plan read yet, treat a journal
   entry with `written` 0 for the lid key of the current source (on any plan) as ours, erring toward a lock as
   `p2-core-lid` design choice 1 does; or accept it and say so in the module doc.
3. **Seen, accepted by design (no change asked): a plan switch whose read fails leaves the core on the old plan's
   view.** Found by the 10000 case run, before the lid properties were narrowed to judge only where the core's last
   read names the active plan:
   ```
   start on other plan, lid open, on battery at 80 %, io-keeps-up
     1. user switches to Balanced
     2. agent 1 Working                             (applies to Balanced)
     3. IO call +12 of the next AnyCommand fails (code 2)
     4. run 20.000 s
     5. user switches to other plan                 (its read fails: Plan(Err))
     6. lid closes
   -> Lock and the chime, while the other plan's own battery lid action, Do nothing, is in force
   ```
   The core acts on `ActivePlan` only through a read (`event.rs`), and a failed read changes nothing, so until the
   next read (10 s while owned) it locks on the old plan's values: a docked user may get a lock and chime they did not
   want. It errs toward the lock, as `p2-core-lid` design choice 1 says. Worth a line in that card's list of accepted
   cases; the properties now leave it out (`core_can_know` in `scenarios/props.rs`).
4. **`p2-ownership-props` findings 1 and 2, seen again from the core's side.** An apply whose write fails runs the
   restore routine, whose journal load fails too: `Applied { result: Err(Write), journal: [] }`. The core keeps its
   empty copy (the failed-load rule), so `apply_failed` sends no restore and `may_own` is false: an `ApplyUpdate` may
   go out, and the next read reports the three entries the apply saved but never wrote as the user's changes, three
   "you changed a setting" notices for nothing. Shrunk case (from the update property, before "owned" was narrowed to
   values Windows holds):
   ```
   start on Balanced, lid open, on AC at 80 %, io-lags
     1. IO call +14 of the next AnyCommand fails (code 2)   (lands on the apply's restore routine, at its load)
     2. IO call +11 of the next Apply fails (code 2)        (the apply's first write)
     3. agent 1 Working
     4. IO runs one
     5. IO runs one
     6. update ready (applies: false)
     7. agent 1 Working
   ```
   No value of ours was in force, so nothing is lost. The apply fix that finding 1 there proposes (save a new entry
   marked `restored`) ends the false notices; finding 2 there (report the step 3 entries after a failed load) would
   let the core send its restore. Nothing new to route; it adds weight to both.
5. **For the power safety review (no change asked):** the world follows `p3-power-io`'s executor as its card
   describes it (values before the check, `Leftover` with its owner as `expect_owner`, `JournalCorrupt` after a set
   aside, the session end queue rule from `06-integration.md` section 4), read from the cards, not from its code.
6. No other card's file failed a build, clippy or a test during this card's runs.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 22:33: started by `wakeshot-worker`; reading the phase notes, the design and the core cards' Results.
- 2026-11-03 22:49: read done. Design: in `sim.rs` a `World` around `Sim`: `FakePlans` with three plans (Haik's
  Balanced, one with the user's own battery Do nothing, one docked with the user's own AC Do nothing), an IO queue
  run as `p3-power-io`'s executor runs it (`ReadPlan`: values then the check's `UserChanged`; `Leftover` with the
  owner as `expect_owner`), outcomes that wait (`IoLate`, `Deliver`) or come at once (`keep_up`), the session end
  queue rule (restore first, queued applies dropped), lock, hibernate (its last check) and update runners, and a log
  of entries each with a `Truth` snapshot of what Windows holds. `Sim::run` split into `fire_next` so the world acts
  between timers. Properties in `scenarios/props.rs` replay the log against a small model of the design rules.
- 2026-11-03 23:05: world and 12 properties written; all pass at 4000 cases (28 s, debug). Two findings, each kept
  as an ignored strict test: (1) the switch's apply writes a key the same plan read reports as the user's (`Plan`
  comes before `UserChanged`), and no later apply takes it back; (2) a lid close before the first plan read with a
  crashed tray's values in force gives no lock. A first tally showed thin coverage; generator rebalanced (sticky
  states undone more often, battery mostly above the floor, user changes mostly on the active plan, faults aimed at
  a command kind). Next: the seven mutants in `<scratchpad>\p2-core-props\mutants\` (runner `mutants.py`).
- 2026-11-03 23:16: mutants: 6 of 7 killed at once; the AC to battery lock survived (its chain came up about once
  in 25 cases), so the lid properties also draw from a lid-weighted mix: then 3 of 3 runs killed it, and the other
  six were killed in each of 3 runs. Eight extra mutants: 6 killed at once; "an apply in flight is not owned" needed
  a late-answer mix (then 2 of 3 runs); "a failed restore empties the mirror" needed one more property, the
  converse lid rule (a lid close on our Do nothing locks at once), then 1 of 2. 13 properties pass at 4000 cases
  (31 s, debug). Next: the acceptance checks.
- 2026-11-03 23:29: the first 10000 case release run failed twice, both on the properties, not the core: a failed
  read after a plan switch (the core locks on the old plan's view by design; observation 3) and a journal entry never
  written counted as owned (observation 4, `p2-ownership-props` findings 1 and 2). The lid properties now judge only
  where the core's last read names the active plan; "owned" means a value Windows holds. Three release runs of 10000
  passed; final mutant round: the card's seven killed, 7 of 8 extras. All checks green (Result). Status: done.

## Result

Status: done 2026-11-03. A world around the core in `sim.rs` and 13 property tests over it in `scenarios/props.rs`
(the card's twelve plus one converse lid rule), each checked against what Windows holds on a `FakePlans` run by the
real ownership routines, with answers that come late. All pass at the default count and in three release runs of
10000 cases. Every one of the card's seven mutants is killed; of 8 extra mutants, 6 are killed in every run, one in 3
of 4 runs, one in 1 of 3.
Two findings in other cards' files (Found outside scope 1 and 2), each kept as an ignored strict test; two more
observations (3 and 4).

### Files changed

- `crates/wakeshot-core/src/sim.rs`: the world (`World`, `WorldStart`, `WorldStep`, `End`, `Truth`, `Entry`,
  `FaultOn`, `AgentNow`, `FileNow`, plan constants `PLANS`, `DOCKED_PLAN`, `world_plan`, `ME`, `DEAD_TRAY`); `Sim::run`
  split into `Sim::fire_next` and `Sim::wait_until` (same behavior: every earlier scenario passes), and
  `Sim::folder`, `Sim::remark`.
- `crates/wakeshot-core/src/scenarios/props.rs`: the model, the properties, three mixes, an ignored coverage tally.
- `docs/research.md`: "Core property test checks" (addition only). This card.
- No `proptest-regressions/scenarios/` folder: persistence is off, as in the other property tests.

### The world (`sim::World`)

- **Plans**: Balanced with Haik's own values; another plan whose own battery lid action is Do nothing (energy saver
  Never, floor 20); a docked plan whose own AC lid action is Do nothing (floor 30). Each value differs, so a value put
  back on the wrong plan shows. The user changes values (on the active plan as Settings does, or on any plan) and
  switches plans (`ActivePlan` posted).
- **IO thread**: `ReadPlan` gives `Plan(values())` then one `UserChanged` per entry `ownership::check` drops; `Apply`
  runs `ownership::apply` as `ME`; `Restore` runs `ownership::restore`, a `Leftover` with the file's owner as
  `expect_owner`, and `JournalCorrupt` after a set aside. At a session end the queue drops queued applies and puts
  the end-of-session restore first. Faults: a failed call at an offset into the next command of a kind.
- **Late answers**: with `keep_up` every command runs and every answer is handled after each step and between
  timers; without it, commands wait for `Io` / `IoLate` and answers for `Deliver`. Answers keep their order.
- **Other runners**: `Lock` (may fail; Windows then posts `Session(Locked)`), the hibernate thread's last check (lid,
  idle with the resume clamp, AC and floor for `Floor`; then a 1 h hibernate and `Resumed`, or `NotAllowed`), the
  update thread, hold files at once through `Sim::watch`, and the IO thread's finish at `Quit`.
- **Start**: maybe a crashed tray's values in force with its journal, whose start-up restore (owner Gone) may fail,
  so `Started` carries it; then `Session(Locked)` when locked, the first lid and power readings, input.
- **Endings**: the last hold goes (then 2 minutes for the retries), Quit, or a session end; from there no faults
  (a failed Quit or session end restore is the guard's and RunOnce's, section 5 "Gap left").
- **Log**: an `Entry` per core step (the world when its event came), per IO run (after it), and a checkpoint after
  each step and flush, each with a `Truth`: lid, power, lock, console, the active plan and who set each value in
  force, in sync, owned, live hold files, last input and resume.

### Properties (`scenarios::props`, one `proptest!` each)

1. `owned_keys_come_back_after_the_last_hold_quit_and_session_end`: after the ending nothing of ours is stored on any
   plan or in force on the active one, every value is the user's own, no known entry is left.
2. `a_released_key_is_never_written_again_in_its_episode`: no `Apply` sent after a release in the episode names the
   key (minor 9).
3. `a_restore_retry_never_takes_an_entry_the_running_hold_uses`: while the core applies, a restore at a tick names a
   plan other than the active one it read (minor 4).
4. `no_apply_from_session_end_until_it_is_cancelled` (minor 6).
5. `no_apply_and_no_display_request_while_another_session_has_the_console`.
6. `no_request_with_no_hold_and_none_while_stepped_back`; also no `Apply` with no hold.
7. `no_hibernate_within_5_minutes_of_a_resume_or_of_input`.
8. `no_lock_when_the_lid_action_in_force_is_the_users_own`: a lid or source lock never goes out while the user's own
   Do nothing is in force for the current source.
9. `a_lid_close_on_our_do_nothing_locks_and_chimes_at_once` (added, the converse of 8).
10. `every_lock_step_back_and_user_change_is_explained` (decision 0): a `Lock` with the chime or `IdleLocked`; a step
    back with its sound or notice; a user change that releases a key with its notice; no going to sleep sound from
    the core; the first failed restore after a full one worked warns, no second warning before a restore works; a
    notice raised while the user is away comes at their return (note 18).
11. `a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s` (minor 11, settle 30 s): from a lid
    close or AC to battery that leaves it so; excused when the lock failed.
12. `no_apply_update_while_a_hold_counts_a_hold_file_lives_or_a_key_is_owned`.
13. `handle_never_panics_and_next_deadline_is_never_behind_a_tick`: `Sim` panics with the trace on a deadline behind
    a `Tick`, the world on a core and IO side that never quiet down (10000 rounds in one flush).

Ignored: `a_released_key_is_never_ours_in_force_again_in_its_episode` (finding 1),
`a_closed_laptop_is_locked_also_before_the_first_plan_read` (finding 2), `coverage_tally` (a report, run with
`-- --ignored --nocapture coverage_tally`).

### Design choices (mine), each with when to change it

1. Windows-side lid properties (8, 9, 11) judge only where the core can know: the IO side caught up and its last
   plan read names the active plan. On an answer on its way or a failed read after a switch the core errs toward a
   lock by design. Change it when commands carry an id, so a property can tell a stale view from a wrong one.
2. The model (which holds count, episode, released keys, step back, whether the core applies, away) is written from
   sections 3 and 5 and reads hold files through `policy::hold_files::decide` (that card's own rules). Change it when
   a design rule it mirrors changes.
3. User changes never set the target 0: a user who sets exactly our value is not seen (section 5, accepted).
4. Not in this world: corrupt journals, Unknown liveness, timers and waiting sessions, plan deletes, crashes inside a
   routine, the guard. They have their own cards' tests (`p2-ownership-props`, `p2-policy-holds`,
   `p2-core-settings`). Change it when one of them meets the episode rules in a way those tests cannot see.
5. Three mixes of the same steps: the general one for every property, a lid-weighted one for 8, 9 and 11, a
   late-answer one for 1 and 12. Why: the general mix built the AC to battery chain about once in 25 cases.
   Change it when `coverage_tally` shows a property's case getting rarer.
6. "Owned" for the update means a value of ours Windows holds or an apply on its way, not a journal entry never
   written (observation 4). Change it when the apply stops leaving such entries.
7. Requests, notices, sounds and state writes are taken and answer nothing; requests always work (a failed request is
   `p2-core-holds`' scenarios). Change it when a property needs `CannotHold`.
8. `failure_persistence: None`, as the other property tests; a shrunk case that finds a bug becomes a named test.

### Acceptance checks (final runs 2026-11-03 23:18 to 23:29, own build folder, every cargo command `--locked`)

- `cargo test --locked -p wakeshot-core scenarios::props` (default 256 cases): 13 passed, 3 ignored; test run 2.4 s,
  8.1 s with cargo and a built target, 28.3 s with a fresh build for the host target. Under 60 s.
- `PROPTEST_CASES=10000 cargo test --locked -p wakeshot-core scenarios::props --release`: 13 passed; test run 15.1 s,
  116.9 s with the release build. Twice more with new seeds: 15.1 s and 14.2 s, passed.
- Mutants, by hand in `core.rs` and `src/core/` of a scratch copy (note 20), own build folder, default case count,
  each reverted; runner `mutants.py`. The card's seven, each killed in every run once the lid mix was in (5 runs for
  the two lid mutants, 4 for the other five; the AC to battery one survived the one run before it); shrunk cases from
  the last run:
  1. `released` keyed by plan (a switch clears the releases): `a_released_key_is_never_written_again_in_its_episode`.
     `start on Balanced, on battery at 50 %, automatic-off, io-lags; KeepAwake(Minutes(30)); switch to other plan;
     menu Restore; IO runs one; switch to Balanced` -> the apply to Balanced writes LidAc.
  2. The retry takes the active plan's entries while holding (`restore_scope` always every plan):
     `a_restore_retry_never_takes_an_entry_the_running_hold_uses`. `start with a crashed tray's journal, start-up
     restore fails; KeepAwake(Minutes(30)); run 10 s` -> `Restore { plan: None, why: Leftover }` at a tick.
  3. `Apply` after `Session(Ending)` (`applies` without the session end): `no_apply_from_session_end_until_it_is_cancelled`.
     `start not-owner, off-console, on battery at 10 %; mutex taken; AC in; session ending; KeepAwake(Minutes(30));
     console back` -> an apply at the plan read.
  4. A lid-close lock with the user's own Do nothing (lock on any Do nothing in force):
     `no_lock_when_the_lid_action_in_force_is_the_users_own`. `start on docked plan; run 30 s; AC in; lid closes`.
  5. No lock at AC to battery with the lid closed: `a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s`.
     `KeepAwake(UntilOff); lid closes; user unlocks; AC in; AC out; run 45 s` -> unlocked on battery for 45 s.
  6. `UserChanged` without a notice: `every_lock_step_back_and_user_change_is_explained`. `start on docked plan;
     agent 1 Working; user sets LidDc = 1; run 10 s`.
  7. Requests left on after the last hold: `no_request_with_no_hold_and_none_while_stepped_back`. `agent 1 Working;
     menu Automatic(false)`.

  Extra mutants, last run: killed by the property named: the update ignores owned keys (12); the countdown without
  the resume clamp (7); `RestoreFailed` at every failure (10); no restore at the hold end (1); apply off the console
  (5); the lid lock without the chime (10, 9); an apply in flight not counted as owned (12, 1; killed in 3 of 4 runs).
  Survived: a failed restore's empty `journal` copied as empty (killed in 1 of 3 runs since property 9, by it): the
  retry at the next tick restores within 30 s, so only a lid close in that window shows it.
- `cargo test --locked -p wakeshot-core`: 525 passed, 4 ignored (this card's three and `p2-ownership-props`'
  finding), plus 10 and 19 in `tests/`.
- `rustfmt --edition 2024 --check` on `sim.rs` and `scenarios/props.rs`: no diff.
- `cargo clippy --locked -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0; also `--features test-support` (x64): 0.
- Only the owned files changed (from my own edit record; git not run), plus this card and the `research.md`
  addition. Build folders, the scratch copy and its scripts deleted.

### Open items

- Findings 1 and 2 for the orchestrator to route (1 to the core settings code, 2 to the core lid code); when one is
  fixed, remove its `#[ignore]`.
- Observation 3: a line for `p2-core-lid`'s accepted cases. Observation 4 adds weight to `p2-ownership-props`
  findings 1 and 2.
- None for Haik.
