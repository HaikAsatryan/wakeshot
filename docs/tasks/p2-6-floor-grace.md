# Wait 30 seconds on battery before the battery floor step back

Phase: 2 to 6 (core fix). Status: done (2026-11-05; worker decisions for Haik's review). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Written by the orchestrator, 2026-11-05, from `docs/tasks/p6-int-haik-checks.md` "Found outside scope" 6 and 7 and
Haik's answer of 2026-11-05 (yes to a 30 s grace; `docs/product.md` "Battery floor" already says it).
Runs after: `p6-int-haik-checks` (done, decfd5a). Parallel with: `p6-real-install-guard`, `p6-notice-app-icon` (they
own none of the files below).
Real laptop: no (the core's sim, scenarios and props; fakes only).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-floor-grace\target`.

## Goal

Under the battery floor, a USB-C dock's short AC drop now gives a step back, the battery chime and a balloon each
time, and a restore of the user's settings. Haik's checks saw drops of 4.3, 4.4, 4.5, 4.7 and 9.65 s; the 9.65 s
drop's balloon was still on screen after AC came back (his 00:50 photo). Haik decided (2026-11-05): under the floor,
Wakeshot steps back only once the PC has been on battery for 30 seconds, the same `SOURCE_SETTLE_MS` the closed-lid
lock already waits. If AC comes back within the 30 seconds, nothing happens: no step back, no chime, no notice, no
restore, no apply. Holds keep holding during the 30 seconds.

Also fix the docs of finding 6: with the lid closed under the floor, the restore right after an AC loss made
Windows' own lid action sleep the laptop 1.4 s after the unplug, so the 5-minute route never ran. Orchestrator
decision (for Haik's review): docs only, no code keeps the lid key until the 5-minute check.

## Read first

- `docs/product.md` "Battery floor" and the work end rule; `docs/tasks/p6-int-haik-checks.md` "Found outside scope"
  6 and 7; `docs/research.md` "Integration checklist with Haik".
- `crates/wakeshot-core/src/policy/floor.rs` (`stepped_back`), `policy/lid.rs` (`SOURCE_SETTLE_MS`),
  `core/lid.rs` (`settle_floor`, `settle_window_lock`, the source lock near line 1011), `core.rs` (the settle pass and
  its deadline), `state.rs`, `sim.rs`, `scenarios/lid.rs`, `scenarios/props.rs`.

## Deliverables

1. The grace in the core. The step back starts when the PC is on battery under the floor and has been on battery for
   at least `SOURCE_SETTLE_MS`, timed from the AC loss the core already records for the source lock. The settle pass
   gives the grace end as a deadline, so the step back comes on time with no other event. Decide and record, with
   tests:
   - the lid closed case: the source lock and the step back both fall due 30 s after the unplug; their order;
   - a percent that falls under the floor after more than 30 s on battery: at once, as now;
   - the tray starting on battery under the floor (no AC loss seen): at once or after 30 s from the start;
   - AC flapping (lost, back, lost again within the 30 s): the grace starts again at the last loss.
2. Tests: `policy::floor` units; scenarios: a 5 s and a 29 s drop give no step back and no `Notify`, sound, restore
   or apply command; a 30 s drop steps back at 30 s; the same with the lid closed; the props model and its
   invariants updated. Any `wakeshot-windows` test that asserts a step back at once on an AC loss: update it in
   `app/dispatch.rs` or `app/tray_mode.rs` tests only.
3. Docs: `docs/product.md` "Battery floor": the rest of the bullet after the 30 s sentence, and the work end rule's
   "putting the lid action back does not sleep a laptop whose lid is already closed (tested)": add that right after a
   power source change Windows slept the laptop at once (Haik's block 4, 2026-11-05; which of the two made it act:
   not verified), and that with the 30 s grace it is not known whether Windows still acts at the restore or the
   5-minute route runs; both are safe. `docs/architecture.md`: the floor lines and "Episode flow" (edit only those
   lines; re-read right before each edit, another card may edit section 6). `text.rs` only if a text says "at once".

## Acceptance checks

- `cargo fmt` on own files; `cargo clippy -p wakeshot-core` (with and without `--all-targets`) and `cargo test -p
  wakeshot-core`; the same for `wakeshot-windows` if you changed its tests. Core props at the usual case counts.
- A scenario shows Haik's night: five drops of 4 to 10 s under the floor give zero chimes and zero balloons.

## Out of scope

- Taking a step back balloon away when AC comes back (finding 7 a; orchestrator decision: not now).
- Keeping the lid key until the 5-minute check (finding 6 code; orchestrator decision: docs only).
- Files of other running cards: `hooks.rs`, `notify.rs`, the tray crate, `packaging/`, `README.md`, `CLAUDE.md`.

## Questions for Haik

None.

## Decisions

Worker decisions, for Haik's review (Haik asleep; the orchestrator's brief: take the default and go on).

1. **A tray that starts on battery under the floor steps back at once.** No AC loss was seen, so there is no drop to
   wait out, and a hold that ran for 30 s would apply the settings only to take them back with a chime and a notice.
   The other settle rules read "no source change seen" the same way (`relock`, `settle_window_lock`, the work-end
   restart). Test: `a_tray_that_starts_on_battery_under_the_floor_steps_back_at_once`. Change it when a tray start
   inside a dock drop is seen (for example an update restart while docked).
2. **Lid closed: the source lock and the step back fall due in the same settle pass, 30 s after the unplug. The lock
   goes first, and with the floor chime only.** The source lock now settles before the step back's actions, so `Lock`
   leaves ahead of the step back's `Restore`, which can make Windows sleep the closed laptop at once (block 4). The
   lid chime says the laptop keeps running, wrong at a step back, and two `Play`s in one pass overlap (one thread per
   cue in `WasapiPlayer::play`; `text.rs` already keeps the step back balloon silent so two sounds never overlap).
   Decision 0 holds: the floor chime and the step back notice go with that `Lock`. The lock is still watched and
   retried. Tests: `an_unplug_with_the_lid_closed_under_the_floor_locks_and_steps_back_at_30_s` (Haik's own values,
   session locked) and the docked twin (the user's own AC Do nothing, signed in, the retry). Change it when Haik wants
   both sounds: pass `true` to `settle_source_lock`, and the model's `lid_lock_sent` goes back to the lid chime only.
3. **A percent that falls under the floor after more than 30 s on battery steps back at once; one that falls under it
   inside the 30 s waits for their end**, counted from the AC loss, not from the reading: the wait is for the source.
   Tests: `a_percent_that_falls_under_the_floor_after_30_s_on_battery_steps_back_at_once`,
   `..._inside_the_grace_waits_for_its_end`, and units in `policy::floor`.
4. **AC that flaps (lost, back, lost within 30 s) starts the 30 s again at the last loss**: the grace counts from
   `source_since_mono`, which every source change sets. Test: `ac_that_flaps_inside_the_grace_starts_it_again_at_the_last_loss`.
5. **The 30 s are run time**, as every core clock: a sleep inside them does not count, and an AC loss seen at a resume
   starts them at the resume. Change it when a resume on battery under the floor must step back at once.
6. **Log lines** at the start of the wait and at an end with no step back (`State::floor_grace`, log only), so a
   dock's drop shows in the log though nothing else happens.

## Found outside scope

Problems seen but not fixed.

1. `docs/product.md`, the sounds list ("AC lost or back (except when it leaves a closed laptop running on Wakeshot's
   Do nothing: then as a lid close, with the lock)"): under the floor that lock now comes with the battery floor
   sound, not the lid chime (decision 2). Outside this card's two product sections; a one-line docs fix, for example
   "then as a lid close, with the lock; under the battery floor with its sound (Battery floor)".
2. `docs/research.md`: `p6-real-install-guard`'s section was the file's last when this card appended "Battery floor
   grace checks" after it, so a later addition of theirs must go above that heading.
3. Haik's checklist (`docs/phases/06-integration.md` row 8, `p6-int-haik-checks` block 4) expected a chime and a
   notice at each AC drop under the floor. A re-check, if wanted (about 5 minutes of Haik's time): under the floor, a
   short dock drop gives nothing; an unplug of more than 30 s gives the floor sound and notice once, at 30 s; with the
   lid closed also the lock, and whether Windows sleeps at the restore or after the 5 minutes is the open fact.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started; reading the floor, lid and settle code.
- 2026-11-05: design: `policy::floor::decide` gives Holds, Grace (due) or SteppedBack from `source_since_mono`; the
  settle pass returns the grace end as a deadline; the source lock settles before the step back in one pass, so the
  `Lock` goes out before the step back's `Restore`. No `wakeshot-windows` test asserts a step back at an AC loss
  (grep for StepBack, BatteryFloor, Floor in `app/`): none to change there.
- 2026-11-05: code in: `policy::floor::decide` and `GRACE_MS`, `settle_floor` gives the grace deadline, the source lock
  before the step back and without the lid chime when the step back's sound goes with it; props model and three
  props checks updated, one new property, nine new scenarios, `sounds_never_wait` and `lid_events_in_any_order`
  updated. `cargo test -p wakeshot-core`: 695 + 11 + 25 passed; fmt and clippy (both ways) clean. Docs next.
- 2026-11-05: docs in (`product.md` Battery floor and work end rule, `architecture.md` state line, source lock and
  floor bullets of "Episode flow", `research.md` new section). Hand mutants: 6 of 6 killed after one fix (the 30 s
  scenario now unplugs off the 10 s plan reads). Long run at 10000 cases passed. Build folder deleted.

## Result

Done, 2026-11-05.

What was done:

- `policy::floor`: `GRACE_MS` (= `SOURCE_SETTLE_MS`, 30 s), `Floor { Holds, Grace { due_mono_ms }, SteppedBack }` and
  `decide(was, ac, pct, floor_pct, on_battery_since_mono, now)`, in place of `stepped_back`. `below` and `floor_pct`
  are unchanged (the Windows hibernate check uses `below`).
- Core: `settle_floor` decides from `source_since_mono`, logs the wait's start and an end with no step back, and
  gives the grace's end as a deadline that `settle` folds into `next_deadline`. In `settle_lid` the source lock settles
  before the step back's actions; when they go out in the same pass the lock has no lid chime (decisions 1 to 6
  above). Holds hold during the 30 s; AC back inside them sends nothing.
- Tests: 10 new `policy::floor` units (the old 9 kept, through a helper). 9 new scenarios in `scenarios/lid.rs`: a 5 s
  and a 29 s drop (no request change, restore, apply, lock, notice or sound, `state.json` not stepped back); a 30 s
  drop (the step back at 30 s to the millisecond, then AC back holds again); Haik's night (drops of 4.3, 4.4, 4.5,
  4.7, 9.65 s: zero sounds, zero notices); AC flapping; a percent under the floor after and inside the 30 s; a tray
  start on battery; the lid closed with Haik's values and docked. `sounds_never_wait` now unplugs and runs 30 s.
  Props: the model keeps `source_since` and uses `floor::decide`; new property
  `no_floor_step_back_before_30_s_on_battery` (judged on the model's own source time, not its flag); the lid lock
  watch, `every_action_is_explained`, `no_lock_on_the_users_own_do_nothing` and `lid_events_in_any_order` take a
  `Lock` with the floor chime as a lid lock; two coverage counters.
- Docs: `product.md` Battery floor (the rest of the bullet) and the work end rule (block 4's sleep right after a source
  change, not verified which made Windows act, unknown with the 30 s, both safe); `architecture.md` the state line,
  the source lock bullet and the floor bullet of "Episode flow"; `research.md` "Battery floor grace checks".
  `text.rs`: no change (no text says "at once").

Files changed: `crates/wakeshot-core/src/policy/floor.rs`, `core.rs`, `core/lid.rs`, `state.rs`, `scenarios/lid.rs`,
`scenarios/props.rs`; `docs/product.md`, `docs/architecture.md`, `docs/research.md`, this card. No
`wakeshot-windows` file: none of its tests asserts a step back at an AC loss.

Checks:

- `cargo fmt -p wakeshot-core --check`: clean.
- `cargo clippy -p wakeshot-core --locked --target x86_64-pc-windows-msvc -- -D warnings`, with and without
  `--all-targets`: clean.
- `cargo test -p wakeshot-core --locked` (props at the default 256 cases): 695 passed, 1 ignored (`coverage_tally`);
  11 and 25 passed in the two test files.
- `PROPTEST_CASES=10000`, release, `scenarios::props` and `lid_events_in_any_order`: 36 passed in 20 s.
- Haik's night as a scenario: five drops of 4 to 10 s under the floor, zero chimes and zero balloons: passed.
- Hand mutants: 6 of 6 killed (`research.md`).
- `wakeshot-windows` not built: other cards edit it now, and nothing there uses the removed `floor::stepped_back`
  (search of all crates).

Open items: Haik's review of decisions 1 to 6; Found outside scope 1 (a one-line product.md fix) and 3 (an optional
re-check by Haik). The whole-workspace check runs when the orchestrator closes the wave.
