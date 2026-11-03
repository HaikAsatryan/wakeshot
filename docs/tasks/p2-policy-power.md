# Power rules: requests, lid, lock, battery floor, countdown

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-policy-power\target`.
Runs after: the contracts card. Parallel with: `p2-ownership-restore`, `p2-policy-holds`, `p2-present`, and any card
of another track.

## Goal

The pure rules the episode flow uses (architecture section 2 `policy`: `requests`, `lid`, `lock`, `floor`,
`ending`), each a small function of plain values with its own tests, so `p2-core-holds` and `p2-core-lid` only store
state and call them.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 4, 11, 16).
- `docs/product.md` "Power behavior" (requests, never lower security, battery floor, work ends with the lid closed).
- `docs/architecture.md` section 3 "Clocks" and "Episode flow" (lid closes, power source changes, battery floor, Keep
  screen on, countdown); section 5 "Lock and hibernate" (what the hibernate thread re-checks, so the core's countdown
  matches it).
- `docs/research.md`: "AC can drop for a few seconds while docked", "Lock", "Clock" (Lid and sleep spike).
- `crates/wakeshot-core/src/`: `settings.rs` (`PlanValues`, `JournalEntry`), `time.rs`, `command.rs`
  (`CountdownReason`), `policy/` stubs.

## Owns

`crates/wakeshot-core/src/policy/requests.rs`, `policy/lid.rs`, `policy/lock.rs`, `policy/floor.rs`,
`policy/ending.rs`. Nothing else.

## Deliverables

1. `requests`: from (a hold counts, Keep screen on asked, stepped back, this session on the console) to `(system,
   display)`. No request with no hold; none while stepped back; display only with Keep screen on and on the console
   (decision 15, section 3 console bullet).
2. `lid`: the lid action in force for the power source (`PlanValues`, `ac`); whether it is ours (a journal entry for
   that plan and key, live value equal to `written`); lock and chime at lid close only when ours (decision 18; a user
   whose own action is Do nothing gets none); the power source change rule with the lid closed (the action in force
   for the new source is ours, and either the user's own was Do nothing before or the change is AC to battery), with
   its 5 s settle as a `Deadline::Mono`.
3. `lock`: Keep screen on idle lock: the user's own sleep timeout for the current source (the journal's `original`
   while we own the key, else the live value; none when 0, Never); due once per idle period (input after it starts a
   new period).
4. `floor`: the floor from `saver_threshold_dc` (20 when it is 0 or 100); stepped back when on battery and the
   percent is below the floor (strictly below, phase 1 "Values I set"); AC back ends it.
5. `ending`: input idle clamped at resume (min of the reported idle and the mono time since the last `Resumed`);
   countdown due at 5 minutes of clamped idle; cancel rules (a new hold, lid open, AC back for `Floor`); the
   `Deadline::Mono` for the next check. The core sends only `Hibernate`; the hibernate thread plays the sound (track
   note 4).
6. Unit tests, written first, one named test per rule and edge: each source with each lid action; ours versus the
   user's own Do nothing; AC to battery and back inside 5 s; battery to AC; the floor at 0, 100, 19, 20, 21; idle
   just under and at 5 minutes; a resume inside the idle period; input during the countdown; Keep screen on with
   sleep timeout Never; one lock per idle period.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core policy`: all pass; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the five
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the five owned files changed.

## Out of scope

- State, events, commands, notices (`p2-core-holds`, `p2-core-lid`). The hibernate thread's own checks (power
  track). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

1. **The 5 s settle at an unplug with the lid closed is too short for your dock.** The two short AC drops in the
   System log lasted 5.3 s and 6.3 s (`docs/research.md`, "Power rules checks"). With 5 s, each such drop locks you
   and plays the chime while you work on the external screen with the lid closed. I recommend 15 s: an unplugged,
   closed laptop then stays unlocked up to 15 s instead of 5 s, with no keyboard on it. Default until you answer: 5 s,
   as `product.md` says (one constant, `policy::lid::SOURCE_SETTLE_MS`; `product.md`, `architecture.md` and phase 1
   "Values I set" also say 5 s). Not blocking.

## Found outside scope

Problems seen but not fixed. All are for `p2-core-lid` unless named.

1. A `WorkEnded` countdown is not re-checked at a power source change. Example: your own AC lid action is Do nothing,
   battery is Sleep; work ends on battery with the lid closed (countdown), then you dock and work on the external
   screen: 5 minutes without input hibernates a laptop your own AC setting would keep running. The architecture names
   no rule for this. Default in the code: not cancelled, as the architecture says.
2. `lid::ours` is also true when no hold runs but a failed restore left our Do nothing (the laptop still runs because
   of us). The core decides whether the chime ("keeps running") plays then; the lock should stay.
3. After the lid-close lock, an unplug gives a second `Lock` and chime from `lid::settle`. The second `Lock` is a no-op
   on a locked session; the chime matches `product.md` ("then as a lid close, with the lock"). The core may skip the
   `Lock` while `Session(Locked)` holds.
4. `lid::source_change` needs the source before the change. The first `Power` event after start has none: the core
   should treat it as no change.
5. For `p3-hibernate`: its `AboveFloor` check can call `policy::floor::below` (strictly below, unknown percent not
   below), so the core and the hibernate thread cannot drift apart. Its idle check must match
   `policy::ending::clamped_idle_ms` and `IDLE_MS`.
6. No other card's file failed a build or a check during this card.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started; reading the card's sources.
- 2026-11-03 19:29 to 19:45: first run stopped by Haik before any code.
- 2026-11-03 19:55: restarted fresh (lid-closed mode stays in v1, so all five rules are in scope).
- 2026-11-03 20:05: sources read. `cargo tree -p wakeshot-core -e normal` baseline saved (serde, serde_json only).
- 2026-11-03 20:10: System log read (Kernel-Power 105): the two short dock AC drops lasted 5.3 s and 6.3 s, both
  longer than the 5 s settle. Kept 5 s (it is in `product.md`); question for Haik in the Result.
- 2026-11-03 20:20: tests written first with `todo!()` bodies; all failed as expected. Then the bodies.
- 2026-11-03 20:30: `policy::` tests pass (137 with the other policy card's), clippy clean on both targets.
  `docs/research.md` "Power rules checks" added. Running the acceptance checks.
- 2026-11-03 20:40: every acceptance check passed (below). Build folder deleted. Done.

## Result

Status: done. All five rules are built as pure functions of plain values, with 59 unit tests written first.

### Files changed

- `crates/wakeshot-core/src/policy/requests.rs`, `lid.rs`, `lock.rs`, `floor.rs`, `ending.rs` (the five owned files).
- `docs/research.md`: "Power rules checks" added at the end (addition only).
- This card.

### Public API for `p2-core-holds` and `p2-core-lid`

All in `wakeshot_core::policy`. No state is kept here: the core stores what each rule returns and passes it back.

- `requests`: `RequestFacts { holding, keep_screen_on, stepped_back, on_console }`; `Requests { system, display }` with
  `NONE`, `any()` (a resume needs `Retake`) and `command()` (`Command::Requests`); `wanted(RequestFacts) -> Requests`.
  System when holding and not stepped back; display also needs Keep screen on and the console.
- `lid`: `DO_NOTHING = 0`; `SOURCE_SETTLE_MS = 5_000`; `key(ac) -> SettingKey`; `action(&PlanValues, ac) -> u32`;
  `owned_entry(journal, plan, key, live) -> Option<&JournalEntry>`; `user_value(journal, plan, key, live) -> u32`;
  `user_action(&PlanValues, ac, journal) -> u32`; `ours(&PlanValues, ac, journal) -> bool` (lid close: `Lock` and
  `Play(LidClosedHolding)` exactly when true); `SourceLock { ac, due_mono_ms }` with `deadline()`;
  `source_change(plan, journal, lid_closed, from_ac, to_ac, now_mono_ms) -> Option<SourceLock>` (store it in place of
  any earlier one); `settle(pending, plan, journal, lid_closed, ac, now_mono_ms) -> Settle { Wait(Deadline), Lock,
  Drop }` (call it at each event while one waits, and at its deadline).
- `lock`: `sleep_timeout_ms(&PlanValues, ac, journal) -> Option<u64>`; `IdleFacts { display_on, session_locked,
  idle_ms, timeout_ms, now_mono_ms }`; `idle_lock(locked_at_mono_ms, IdleFacts) -> IdleLock { lock, locked_at_mono_ms,
  next_check }`. On `lock`: `Lock` and `Notify(IdleLocked)`. Pass `display_on` from `requests::wanted(..).display` and
  `idle_ms` clamped by `ending::clamped_idle_ms`.
- `floor`: `DEFAULT_PCT = 20`; `floor_pct(saver_threshold_dc) -> u8`; `below(battery_pct, floor_pct) -> bool`;
  `stepped_back(was, ac, battery_pct, floor_pct) -> bool`.
- `ending`: `IDLE_MS = 300_000`; `clamped_idle_ms(reported_ms, now_mono_ms, last_resume_mono_ms) -> u64`;
  `starts_at_work_end(lid_closed, user_lid_action) -> bool` (pass `lid::user_action` read before the restore lands);
  `merged(running, starting) -> CountdownReason`; `Cancel { HoldingStarts, LidOpened, AcBack }`;
  `cancels(reason, Cancel) -> bool`; `check(clamped_idle_ms, now_mono_ms) -> Check { Due, Wait(Deadline) }`.

### Choices I made (each: why; change it when)

- A `restored` journal entry counts as ours: its original is stored but may not be applied yet (only
  `PowerSetActiveScheme` was left), so our Do nothing may still be in force; this errs toward a lock. Change it when
  the plan read reports the value in force instead of the stored one.
- `lid::ours` also needs `written` to be Do nothing and `original` not Do nothing, so a user's own Do nothing never
  gets a lock, even from an odd entry.
- A step back ends only at AC, as the card says, so a percent that reads one higher for a moment does not restart the
  hold, its notice and sound. Change it when a lowered threshold must end a step back without AC.
- An unknown battery percent never starts a step back, or a laptop with no percent reading could never hold on
  battery. Change it when such a laptop is seen.
- A floor from a threshold outside 1 to 99 is 20 (Settings writes only 0, 10 to 50 and 100).
- The idle lock is skipped while the session is locked (a lock and its notice would mislead) and while the display
  request is off (Windows locks by itself at screen off). Input after our lock shows as an idle shorter than the time
  since the lock, which starts a new period; a resume does the same through the clamp.
- "A new hold" cancels a countdown as `Cancel::HoldingStarts`: a hold that appears while stepped back does not start
  holding, so it must not cancel a `Floor` countdown while the battery is still under the floor.
- `ending::merged`: `WorkEnded` wins over `Floor`, so AC back cannot cancel a work-end hibernate.
- Every rule that waits returns a `Deadline::Mono`, so the loop's next `InputIdle` and `Tick` come on time, not up to
  a 30 s safety tick late.

### Acceptance checks (run 2026-11-03 20:35, own build folder, every cargo command with `--locked`)

- `cargo test -p wakeshot-core policy`: ok, 138 passed (59 of this card, the rest `sessions` and `hold_files`).
  This card's tests: `ending`: `idle_just_under_five_minutes_is_not_due`, `idle_at_five_minutes_is_due`,
  `no_idle_waits_the_whole_five_minutes`, `a_resume_inside_the_idle_period_clamps_idle`,
  `input_after_the_resume_counts_as_reported`, `no_resume_yet_uses_the_reported_idle`,
  `a_resume_at_now_means_no_idle`, `input_during_the_countdown_moves_the_check`,
  `work_end_countdown_needs_the_lid_closed_and_the_users_own_action`,
  `holding_again_or_the_lid_opening_cancels_both_reasons`, `ac_back_cancels_only_the_floor_countdown`,
  `work_ended_wins_over_floor`. `floor`: `floor_is_20_when_the_threshold_is_never_or_on_battery`,
  `floor_follows_the_threshold_in_between` (1, 10, 19, 20, 21, 50, 99), `floor_is_20_for_values_settings_never_writes`,
  `below_the_floor_is_strict` (19, 20, 21), `steps_back_on_battery_below_the_floor`,
  `on_ac_below_the_floor_is_no_step_back`, `ac_back_ends_the_step_back`, `a_step_back_lasts_on_battery_until_ac`,
  `an_unknown_percent_never_steps_back`. `lid`: `keys_and_actions_follow_the_power_source`,
  `lid_close_on_ac_locks_only_for_our_do_nothing`, `lid_close_on_battery_locks_only_for_our_do_nothing` (each source
  with each of the four lid actions), `users_own_do_nothing_gets_no_lock_when_docked`,
  `a_value_the_user_changed_is_not_ours`, `an_entry_on_another_plan_is_not_ours`, `a_restored_entry_still_counts_as_ours`,
  `user_value_is_the_original_while_ours_else_the_live_value`, `ac_to_battery_locks_once_battery_lasted_the_settle`,
  `ac_to_battery_and_back_inside_the_settle_gives_nothing`, `ac_to_battery_locks_after_the_users_own_do_nothing_on_ac`,
  `battery_to_ac_locks_when_the_users_own_battery_action_was_do_nothing`,
  `battery_to_ac_gives_nothing_after_our_lid_close_lock`, `battery_to_ac_onto_the_users_own_do_nothing_gives_nothing`,
  `ac_to_battery_onto_a_released_key_gives_nothing`, `a_source_change_with_the_lid_open_gives_nothing`,
  `the_same_source_twice_is_no_change`, `the_lid_opening_in_the_settle_drops_the_lock`,
  `a_release_in_the_settle_drops_the_lock`, `a_plan_switch_in_the_settle_drops_the_lock`. `lock`:
  `timeout_on_ac_is_the_live_value`, `timeout_on_battery_is_the_original_while_ours`,
  `timeout_on_battery_is_the_live_value_when_not_ours`, `keep_screen_on_with_sleep_timeout_never_never_locks`,
  `locks_at_the_timeout_not_before`, `one_lock_per_idle_period`, `input_after_the_lock_starts_a_new_period`,
  `a_resume_after_the_lock_starts_a_new_period`, `no_lock_without_the_display_request`,
  `no_lock_while_the_session_is_locked`. `requests`: `no_hold_takes_no_request`,
  `a_hold_takes_the_system_request_only`, `keep_screen_on_adds_the_display_on_the_console`,
  `off_the_console_keep_screen_on_holds_the_system_request_only`,
  `stepped_back_takes_no_request_even_with_keep_screen_on`, `every_combination_follows_the_rule` (all 16),
  `any_request_on_means_a_retake_after_resume`, `requests_become_the_command`.
- `cargo test -p wakeshot-core`: ok, 246 passed, 0 failed.
- `rustfmt --edition 2024 --check` on the five files: exit 0, no diff.
- `cargo clippy -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- `cargo tree -p wakeshot-core -e normal`: identical to the baseline taken before any edit (serde, serde_json).
- A grep of the five files for `std::time`, `std::fs`, `std::env`: no match.
- Only the five owned files changed in code; besides them only this card and the `docs/research.md` addition. Not
  checked with git (workers never run git).
- Not done (the card does not ask): a mutant run.

### Open items

- Question 1 (the settle value) for Haik.
- Found outside scope 1 to 5 for `p2-core-lid` and `p3-hibernate`.
- This card's code is in the power safety review scope (`docs/phases/02-06-lighter.md`, "Reviews").
