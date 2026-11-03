# Core state machine, part 2: power settings in the episode

Phase: 2. Status: done. Needs Haik: no. Question 10 (Phase 1 file, "New questions") is answered: the default
(restore and requests only while another session is on the PC's screen), Haik 2026-11-03 18:09
(`docs/phases/02-06-signoff.md` item 11); build it. Agent: `wakeshot-worker`. Size: about 150K tokens. Split trigger
(review m13: no margin): if the worker passes about 120K before the session end and console part, it stops at a clean
point and that part (session end, fast user switching, console left and back) becomes its own card, run before
`p2-core-lid`.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-settings\target`.
Runs after: `p2-core-holds`. Parallel with: `p2-ownership-props`, and any card of another track.

## Goal

Everything the core decides about power settings: when to `Apply` and `Restore`, the journal mirror, released keys,
plan switches, retries, the user's own actions, session end and fast user switching, with the notices that explain
them. The IO side runs the commands; this card tests with hand-fed `Done` outcomes.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 3, 5, 7, 8, 9, 15).
- `docs/product.md` "User settings safety", "Power behavior" (what is written and when).
- `docs/architecture.md` section 3 (episode flow: hold start, while holding, hold end, restore retry, user actions;
  `ApplyError`; `Outcome`), section 4 (processes, the settings mutex, end of session, `wakeshot restore`), section 5
  "Power settings" and "Ownership" (what the IO side does with `Apply`, `Restore`, `ReadPlan`), section 8 (settings
  rows).
- `docs/phases/01-design-skeleton.md` Questions 10 to 12; `docs/phases/03-windows-power.md` notes 5 to 7, 10 (what
  the IO side does with unknown keys, released-key entries, a corrupt journal, a failed entry read).
- `docs/reviews/p1-design-power-verify.md` minors 4, 6, 9; `p1-contracts-domain.md` m4, m5, m7.
- `crates/wakeshot-core/src/` contract files; Result of `p2-core-holds` (seams, `Sim`).

## Owns

`crates/wakeshot-core/src/core.rs` (bodies), `state.rs`, `sim.rs`, `scenarios/settings.rs`, and `src/core/`
submodules. Nothing else.

## Deliverables

1. Apply at hold start (and at `SettingsOwner`, `ConsoleBack`, `EndCancelled`, after a plan switch) only while: this
   tray owns settings, its session is on the console, not stepped back, not at session end, and some key is not
   released. Writes: lid AC, lid DC, sleep DC to 0, less the released keys. No plan values yet: `ReadPlan` first
   (track note 15).
2. `ReadPlan` on every `Tick` and every 10 s (`Deadline::Mono`) while anything is owned. `Event::ActivePlan(id)`
   (contract C8, posted by `p3-system-events` within milliseconds of a switch; review M2): when `id` differs from the
   last `Plan`'s active id and anything is owned or a hold counts, send `ReadPlan` at once; never act on it alone (the
   `Plan` answer drives item 3). Test `active_plan_event_reads_plan`; mutant "ignore `ActivePlan`" must fail it.
3. `Plan(Ok)`: live values; a new active id is a plan switch, handled first: `Restore { plan: Some(old) }`, then
   `Apply` to the new plan. `Plan(Err)`: releases nothing.
4. Journal mirror: a copy of the `journal` in `Done(Applied)` and `Done(Restored)`, less the keys `UserChanged` names.
5. `UserChanged`: the key released on every plan for the episode (power verify minor 9); `Notify(UserChanged)`.
6. `Done(Applied)`: `SafetyNet` or `Write`: requests only for the episode, `Notify(RequestsOnly)` once; access denied
   (5 or 1260): for this run; `PlanChanged`: wait for the next plan read.
7. Holding ends: `Requests` off and `Restore { plan: None }` at once.
8. Retry of a failed restore on each `Tick`: while the core applies, only entries on other plans (one `Restore { plan:
   Some(p) }` each); the active plan's entries for released keys go with the next `Apply` there (track note 3);
   otherwise `Restore { plan: None }`. Never an entry a running hold uses (power verify minor 4).
   `Notify(RestoreFailed)` once per failure streak. Leftover journal from `Started`: `Restore { why: Leftover }` on
   each tick until it succeeds (track note 9).
9. `Event::JournalCorrupt`: `Notify(JournalCorrupt)`.
10. `Menu(Restore)` (menu or `restore-request.json`): `Restore { plan: None, why: UserAsked }`; all three keys released
    for the episode; requests kept; `Notify(RestoredByUser)`; after its `Done`: `FinishRestoreRequest` with the fresh
    `StateFile`, not debounced (track note 5).
11. `Menu(Quit)`: requests off, `Restore { why: Quit }`, `DeleteHold(keep-awake.json)`, `Quit`.
12. `Session(Ending)`: `Restore { why: SessionEnd }`; no `Apply` until `EndCancelled` (power verify minor 6).
    `Ended`: nothing more.
13. `Session(ConsoleLeft)`: `Restore { why: ConsoleLeft }`, the system request only, no display request;
    `ConsoleBack`: the display request if asked, `Apply` as at a hold start (Question 10: the default, Haik's answer).
14. `owner: false`: requests only; `SettingsOwner`: apply as at a hold start.
15. `Log` lines for each release and each requests-only switch.
16. `scenarios/settings.rs`, written before the code, one named test per item above and these cases: an apply in
    flight when the hold ends; a plan switch while an apply is in flight; `UserChanged` then a plan switch: the key
    stays released on the new plan; a restore that fails twice then works (one warning); a hold that starts while a
    restore fails (the retry skips the active plan's held entries); session end during a hold, then cancelled; console
    left and back during a hold; Restore my power settings, then a plan switch: nothing written until work ends; Quit
    with Keep awake on.
    Added by the orchestrator (`p2-ownership-restore` Found outside scope 3): a restore whose journal load fails
    reports `Outcome::Restored` with `result: Err` and `journal: []`, since nothing is known. Never read that empty
    list as "nothing owned": on `Restored` with `Err`, keep retrying while a restore is wanted, whatever `journal`
    holds. One scenario test for it.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::settings` and `cargo test --locked -p wakeshot-core`: all pass
  (including `scenarios::holds`); list the new test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency (as in `p2-core-holds`).
- Mutants, each made by hand, run, then reverted (record the failing test): key `released` by plan; let the retry
  take the active plan's entries while holding; apply after `Session(Ending)`. Each must fail a test. Mutants run
  only in a scratch copy of the workspace, never in the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- How the IO side runs `Apply`, `Restore`, `ReadPlan` (the ownership routines and the power track). Lid, lock, floor,
  countdown (`p2-core-lid`). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` section 3 and 5 say `ReadPlan` "on each tick" and the retry "on each tick". The core holds
   both to one per 10 s from ticks (design choice 2), so a 1 s `state.json` tick or a Keep awake minute tick adds no
   read. Proposed text for the docs pass: "`ReadPlan` every 10 s while anything is owned, and at a tick when the
   last read is 10 s old or more; a failed restore is retried at the same moments."
2. Same section, "Holding ends: `Requests` off and `Restore { plan: None }` at once": the core sends that restore only
   when something may be owned (design choice 3). Proposed: "... and, when anything may be owned, `Restore`".
3. For the power track (`p3-power-io`, the `Restore` runner): a `Leftover` restore that is not run because the owner
   is Unknown must answer `Restored { result: Err(..) }`, not `Ok`: the core stops the leftover retry at the first
   `Ok` of a restore of every plan. The core then shows `RestoreFailed` once, which is right: the override stays.
4. For the power track: `Session(Ended)` without an `Ending` first makes the core send the session end restore then
   (design choice 7). If the loop already puts a restore at the front of the IO queue at `WM_ENDSESSION`, this one is
   a second, harmless run.
5. No other card's file failed a build, clippy or a test during this card.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" items 3 and 4:
  - An apply or check whose journal load fails or finds a corrupt file also reports `journal: []` (same rule as the
    restore above). Never read it as "nothing owned"; `p3-journal`'s lock timeout is the same case.
  - Orchestrator decision: after an apply fails with `SafetyNet(JOURNAL_CORRUPT)`, the core sends one
    `Restore { plan: None }`, so the routine sets the file aside and `JournalCorrupt` reaches the user, instead of
    every later apply failing. Once per corrupt file, not per apply. The power safety review checks it; the
    `architecture.md` line comes with the docs pass.
- 2026-11-03 20:40, worker: started. Reading the card, the `p2-core-holds` Result and the contracts.
- 2026-11-03 20:50: sources read. Design: a new private `core/settings.rs` (an `impl Core` block). An apply waits in
  the episode as `Wanted` (sent by settle after the requests, at each edge: hold start, `SettingsOwner`,
  `ConsoleBack`, `EndCancelled`, plan switch) or `WaitPlan` (no plan known, or after `PlanChanged`: a fresh read
  first). A failed restore sets `retry` (its why), retried on each `Tick`, scoped by "the core applies now". A
  failed outcome with an empty `journal` keeps the last mirror copy (unknown, not empty). Next: scenario tests.
- 2026-11-03 20:55: 33 tests in `scenarios/settings.rs` and builders in `sim.rs` (`BALANCED`, `OTHER_PLAN`, `KEYS`,
  `plan`, `entries`, `applied`, `restored`). Against the current core: 30 fail, 3 pass (the "nothing happens" cases).
  Plan reads and retries at a `Tick` are held to one per 10 s, so a 1 s `state.json` tick adds none. Next: the code.
- 2026-11-03 21:01: code written (`core/settings.rs`, wiring in `core.rs`, `FinishRestoreRequest` in `core/view.rs`,
  fields in `state.rs`; `leftover` folded into `retry`). One fix found by a test: Quit left the old deadline set.
  `cargo test -p wakeshot-core`: 365 passed (33 new). rustfmt on the owned files done; clippy four runs exit 0.
  Next: a short settings proptest, then mutants in a scratch copy.
- 2026-11-03 21:04: `settings_events_in_any_order` (proptest) added: passes; `PROPTEST_CASES=10000` passed in 9.9 s.
  Mutant run started in `<scratchpad>\p2-core-settings\mutants\` (runner `mutants.py`, results file beside it).
- 2026-11-03 21:05: 10 of 10 mutants killed. Added a log line at `ConsoleLeft` (item 15: it is a requests-only switch)
  with its assert, then a second batch on the final code: 2 of 2 killed. Copy and its build folder deleted.
  `docs/research.md` "Core settings checks" added. Final checks all green (Result).
- 2026-11-03 21:06: Result written; build folder and scratch scripts deleted. Status: done.

## Result

Status: done 2026-11-03. Everything the core decides about power settings is built in a new private module
`core/settings.rs`, test-first: 33 scenario tests (30 failed against the core before, 3 "nothing happens" cases
passed) plus one property test. The session end and console part is in; no split was needed.

### Files changed

- `crates/wakeshot-core/src/core.rs`: `mod settings;`, the event, menu and outcome arms of this card, the apply step
  in settle, the plan read deadline, the hold end restore, Quit stops the core. Public signatures unchanged.
- `crates/wakeshot-core/src/core/settings.rs` (new): every rule below.
- `crates/wakeshot-core/src/core/view.rs`: `FinishRestoreRequest` path.
- `crates/wakeshot-core/src/state.rs`: settings fields filled (`owner`, `plan`, `session_ending`, `released`), new
  ones (`plan_read_mono`, `applying`, `retry`, `retry_warned`, `requests_only_warned`, `corrupt_restore_sent`,
  `restore_request`, `finish_restore_request`, `quit`, `Episode.apply`, `ApplyStep`); `leftover` removed (now
  `retry: Some(Leftover)`).
- `crates/wakeshot-core/src/sim.rs`: builders `BALANCED`, `OTHER_PLAN`, `KEYS`, `plan(id)` (Haik's own values),
  `entries(plan, keys)`, `applied(plan, result, journal)`, `restored(plan, why, result, journal)`.
- `crates/wakeshot-core/src/scenarios/settings.rs`: the tests.
- `docs/research.md`: "Core settings checks" (addition only). This card.

### How it works

- **Apply.** `Episode.apply` is `Wanted` at each edge (hold start, `SettingsOwner`, `ConsoleBack`, `EndCancelled`, a
  plan switch). Settle sends it after the requests when `Core::applies()`: a hold counts, owner, on the console, not
  stepped back, not at session end, no apply failed in the episode or run, some key not released. `writes` is the
  three keys less the released ones, each to 0. No plan read yet: `ReadPlan`, `WaitPlan`, apply at the next
  `Plan(Ok)`.
- **Plan reads.** `ReadPlan` at a tick when the last one is 10 s old or more; a `Deadline::Mono` 10 s after the last
  read while the copy holds an entry or an apply waits for a plan; at once on `ActivePlan(id)` when `id` differs from
  the last read and anything may be owned or a hold counts.
- **`Plan(Ok)`** with a new active id: `Restore { plan: Some(old), why: PlanSwitch }` when the old plan may hold
  entries (copy or an apply in flight), then the apply to the new plan. `Plan(Err)`: nothing.
- **Journal copy.** Each `Applied` and `Restored` outcome's `journal`, less `UserChanged` keys. A failed outcome
  (`Restored` Err, `Applied` `SafetyNet` or `Write`) with an empty `journal` keeps the last copy: nothing was read.
- **`UserChanged`**: the key released for the episode on every plan, one log line, `Notify(UserChanged)` once per key.
- **Failed apply.** `SafetyNet` or `Write`: requests only for the episode; code 5 or 1260: for the run. One log line,
  `Notify(RequestsOnly)` once per failure streak (until an apply works). The copy's entries go back at once with
  `Restore { plan: None }`. `SafetyNet(JOURNAL_CORRUPT)`: one such restore per corrupt file (reset by
  `JournalCorrupt` or a full restore that works). `PlanChanged`: if the core still thinks that plan is active, a
  fresh `ReadPlan` and `WaitPlan`; if a switch was already seen, nothing (its restore is queued behind the apply).
- **Hold end**: `Requests` off, then `Restore { plan: None, why: WorkEnded }` when anything may be owned.
- **Retry.** A failed restore sets `retry` (its why); `Started` with a journal sets `retry: Leftover`. At each tick
  read moment: while the core applies, one `Restore { plan: Some(p) }` per other plan in the copy, never the active
  plan; otherwise `Restore { plan: None }`. Cleared by an `Ok` of a restore of every plan, or an `Ok` that leaves
  nothing for the retry. `Notify(RestoreFailed)` once per streak.
- **Restore my power settings**: `Restore { plan: None, why: UserAsked }`, all keys released for the episode, requests
  kept, log line, `Notify(RestoredByUser)`; its `Done` sends `FinishRestoreRequest` with the fresh `StateFile` at once
  (a later retry with that why does not). A failed `Written { RestoreRequest }` writes `state.json` again.
- **Quit**: exactly `Requests` off, `Restore { why: Quit }`, `DeleteHold` of the Keep awake file, `Quit`; then the
  core answers nothing and has no deadline.
- **Session**: `Ending`: `Restore { why: SessionEnd }` always, no apply until `EndCancelled` (which applies as at a
  hold start). `Ended`: nothing, unless no `Ending` came. `ConsoleLeft`: log line, `Restore { why: ConsoleLeft }`
  when anything may be owned, the display request drops (settle). `ConsoleBack`: display request, then the apply.

### Seams for `p2-core-lid`

`Core::applies()` (reads `stepped_back`), `Core::want_apply()` (call it when the step back ends: AC back),
`Core::restore_all(why, out)` (the step back: `RestoreWhy::Floor`), `Core::may_own()` (the "no key is owned" test
for `ApplyUpdate`, decision 42), `state.plan` and `state.journal` (for `policy::lid` and `policy::lock`), `Core::notify`
for every notice. `Session(Locked | Unlocked)` is left empty in `take_session` for that card.

### Design choices (mine), each with when to change it

1. A failed outcome with an empty `journal` keeps the last copy, and the retry goes on whatever `journal` holds. The
   orchestrator's rule; the copy keeps showing in `state.json` what was last known. Change it when the outcome tells
   a failed load apart from an empty journal (a contract field).
2. Plan reads and retries at a tick at most once per 10 s, not at every `Tick`: other deadlines (1 s `state.json`,
   Keep awake minutes) would each add a read. Values are then at most 10 s old while owned and at most about 40 s
   otherwise. Change it when a rule needs fresher plan values.
3. The hold end restore, the `ConsoleLeft` restore and the switch's old-plan restore go out only when something may be
   owned (copy, apply in flight, or a failed restore). The session end and Quit restores always go out. Change it
   when a journal can hold entries this tray did not see (it cannot now: only this tray's applies add entries).
4. `RequestsOnly` once per failure streak, not once per episode: each agent turn is an episode, and the tooltip says
   requests only anyway. Change it when Haik wants a notice for every failed episode.
5. The restore after a failed apply uses `RestoreWhy::WorkEnded`: no reason fits better. Change it with contract
   proposal 1.
6. `PlanChanged` with no switch seen yet sends `ReadPlan` at once, not only "wait for the next read": a switch away
   and back before the read would otherwise leave the hold without its apply. Change it when `ActivePlan` is shown to
   cover every switch.
7. `Ended` without `Ending` sends the session end restore. Change it when the power track shows `Ending` always comes.
8. An outcome is read as the current episode's. Known limit: a failed apply answered after its hold ended and a new
   hold started makes the new episode requests only (the safe side). Change it when a log shows it: tag applies with
   an episode number.
9. Quit always sends `Requests` off, even when none is on: a failed earlier request may have left the state unknown.

### Acceptance checks (final run 2026-11-03 21:06, own build folder, every cargo command `--locked`)

- `cargo test -p wakeshot-core scenarios::settings`: 34 passed. Tests:
  `a_corrupt_journal_at_apply_sends_one_restore_per_file`, `a_failed_first_plan_read_waits_for_the_next_one`,
  `a_failed_write_restores_what_the_apply_left_at_once`, `a_hold_that_starts_while_a_restore_fails_keeps_its_entries`,
  `a_plan_switch_while_an_apply_is_in_flight`, `a_plan_switch_with_nothing_owned_restores_nothing`,
  `a_released_key_stays_released_after_a_plan_switch`, `a_restore_that_fails_twice_then_works_warns_once`,
  `a_restore_whose_journal_load_fails_is_retried_whatever_journal_holds`,
  `access_denied_holds_with_requests_only_for_the_run`, `active_plan_event_reads_plan`,
  `an_apply_in_flight_when_the_hold_ends`, `console_left_and_back_during_a_hold`,
  `failed_apply_holds_with_requests_only_for_the_episode`, `failed_plan_read_releases_nothing`,
  `hold_start_applies_the_three_keys_after_the_requests`, `hold_start_before_the_first_plan_read_reads_first`,
  `holding_ends_releases_and_restores_at_once`, `holding_ends_with_nothing_owned_restores_nothing`,
  `journal_corrupt_shows_a_notice`, `journal_mirror_is_a_copy_of_each_outcome_less_user_changes`,
  `leftover_journal_is_restored_on_each_tick_until_it_succeeds`, `plan_changed_waits_for_the_next_plan_read`,
  `plan_switch_restores_the_old_plan_then_applies_to_the_new`, `quit_with_keep_awake_on`,
  `read_plan_on_every_tick_and_every_10_s_while_owned`,
  `restore_my_power_settings_releases_every_key_and_keeps_the_requests`,
  `restore_my_power_settings_then_a_plan_switch_writes_nothing_until_work_ends`,
  `session_end_during_a_hold_then_cancelled`, `session_ended_does_nothing_more`, `settings_events_in_any_order`
  (proptest), `settings_owner_with_nothing_holding_applies_nothing`, `user_change_releases_the_key_and_says_so`,
  `without_the_settings_owner_requests_only_until_it_is_taken`. Timing tests check reads 7 s off the safety tick.
- `cargo test -p wakeshot-core`: 366 passed, 0 failed (all 40 `scenarios::holds` among them, unchanged).
- Property long run: `PROPTEST_CASES=10000`, 9.9 s, passed. No regression file (`failure_persistence: None`).
- `rustfmt --edition 2024 --check` on `core.rs` (with its children `core/holds.rs`, `core/settings.rs`,
  `core/view.rs`), `state.rs`, `sim.rs`, `scenarios/settings.rs`: exit 0.
- `cargo clippy -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0. Also `--features test-support` (x64): 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` shows serde and serde_json only;
  a grep of the owned files for `std::time`, `std::fs`, `std::env`: none.
- Mutants, in a scratch copy with its own build folder, each reverted (failing tests named); 12 of 12 killed:
  - key released by plan (a switch clears the releases): `a_released_key_stays_released_after_a_plan_switch`,
    `restore_my_power_settings_then_a_plan_switch_writes_nothing_until_work_ends`;
  - the retry takes the active plan's entries while holding:
    `a_hold_that_starts_while_a_restore_fails_keeps_its_entries`;
  - apply after `Session(Ending)`: `session_end_during_a_hold_then_cancelled`, `session_ended_does_nothing_more`,
    `settings_events_in_any_order`;
  - ignore `ActivePlan`: `active_plan_event_reads_plan`;
  - extra: a failed restore's empty journal copied as empty:
    `a_restore_whose_journal_load_fails_is_retried_whatever_journal_holds`; no retry when that journal is empty: the
    same test; no 10 s read deadline: `a_failed_first_plan_read_waits_for_the_next_one`,
    `read_plan_on_every_tick_and_every_10_s_while_owned`; no hold end restore: 3 tests, among them
    `holding_ends_releases_and_restores_at_once`, `an_apply_in_flight_when_the_hold_ends`; no old-plan restore at a
    switch: 4 tests, among them `plan_switch_restores_the_old_plan_then_applies_to_the_new`; `RestoreFailed` at every
    failure: `a_restore_that_fails_twice_then_works_warns_once`; a corrupt journal restored at every apply:
    `a_corrupt_journal_at_apply_sends_one_restore_per_file`; no `ConsoleLeft` log line:
    `console_left_and_back_during_a_hold`.
- Only the owned files changed (from my own edit record; git not run), plus this card and the `research.md`
  addition. Build folder, mutant copy and scratch scripts deleted.

### Contract proposals

1. `RestoreWhy::RequestsOnly` (or `ApplyFailed`) for the restore after a failed apply, so the IO log does not say
   "work ended" while the hold goes on (design choice 5).
2. Optional, for design choice 1: a way for `Applied` and `Restored` to say the journal could not be read (for
   example `journal: Option<Vec<JournalEntry>>`), so the copy needs no rule.

### Open items

- Found outside scope 1 and 2 (architecture lines for the docs pass), 3 and 4 (power track).
- For the power safety review: design choices 1, 3, 6, 7 and 8, and the `JOURNAL_CORRUPT` restore.
- Next: `p2-core-lid` builds on the seams above.
