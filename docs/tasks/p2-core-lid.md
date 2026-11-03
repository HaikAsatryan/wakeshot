# Core state machine, part 3: lid, battery floor, countdown, locks

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-lid\target`.
Runs after: `p2-core-settings`. Parallel with: any card of another track.

## Goal

The closed-lid and battery side of the episode flow: the lid-close lock and chime, the lock when a closed laptop
changes power source, the battery floor step back, the countdown to hibernate when work ends with the lid closed,
the Keep screen on idle lock, and the closed-lid report.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 4, 11, 16).
- `docs/product.md` "Power behavior" (never lower security, sounds, closed-lid notice, battery floor, work ends with
  the lid closed).
- `docs/architecture.md` section 3 "Clocks", "Episode flow" (lid closes, power source changes, battery floor, Keep
  screen on, `Resumed`, holding ends, countdown, closed-lid report); section 5 "Lock and hibernate"; section 8 rows
  for lock and hibernate.
- `docs/reviews/p1-design-power.md` minors 17, 18; `p1-design-power-verify.md` minor 11; `p1-contracts-domain.md` M5.
- `crates/wakeshot-core/src/` contract files; Results of `p2-policy-power`, `p2-core-holds`, `p2-core-settings`.

## Owns

`crates/wakeshot-core/src/core.rs` (bodies), `state.rs`, `sim.rs`, `scenarios/lid.rs`, and `src/core/` submodules.
Nothing else.

## Deliverables

1. `Lid { closed }`: closing while the lid action in force is ours: `Lock` and `Play(LidClosedHolding)` together, the
   lock not waiting for the sound; note when the lid closed (`mono_ms`). The user's own Do nothing: neither.
2. `Power { ac, battery_pct }` with the lid closed: the source change rule through `policy::lid`, lock and chime once
   the new source has lasted 5 s (`Deadline::Mono`); a change back inside 5 s: nothing.
3. Battery floor through `policy::floor`: step back (`Requests` off, `Restore { why: Floor }`, `Notify(StepBack)`,
   `Play(BatteryFloor)`; countdown with reason `Floor` if the lid is closed); no hold starts while stepped back; AC
   back: hold again (requests and apply as at a hold start).
4. Countdown through `policy::ending`: starts when holding ends with the lid closed and the user's own lid action is
   not Do nothing (reason `WorkEnded`; settings already restored by the hold end), or at a step back with the lid
   closed; `InputIdle` clamped at resume; cancelled by a new hold, lid open, AC back (`Floor`); due:
   `Hibernate { reason, floor_pct, last_resume_mono_ms }` and no `Play` (track note 4).
5. `Done(Hibernate(..))`: `NotAllowed` or `Failed`: `Notify(HibernateFailed)`; `Skipped`: the countdown goes on or
   ends as the reason says (write the rule in the Result); `Resumed`: as `Resumed`.
6. Keep screen on idle lock through `policy::lock`: `Lock` and `Notify(IdleLocked)` once per idle period (track note
   11). `Done(Locked(Err))`: `Notify(LockFailed)`.
7. Closed-lid report: at the first `Session(Unlocked)` after a hold that ran with the lid closed:
   `Notify(ClosedLidReport { ran_ms, held_by })`, `ran_ms` from `mono_ms`.
8. `Suspending` and `Resumed`: the last resume time for the idle clamp.
8a. Build it: Haik said yes to the tray track's proposal (`docs/phases/02-06-signoff.md` item 4, default, Haik
    2026-11-03 18:09; `docs/phases/02-core.md` note 18). Notices raised while the session is locked or another
    session is on the console wait and go out at `Unlocked` or `ConsoleBack`, in order; sounds never wait. A tray
    that starts while locked gets `Session(Locked)` right after `Started` (integration,
    `docs/reviews/p1-close-verify.md` Major 1): test that `Started`, `Session(Locked)`, `Updated` gives no `Notify`
    until `Unlocked`, and the same for a start-time `RestoreFailed`.
9. `Log` lines for each lock, step back, countdown start, cancel and due.
9a. Updates: the contracts cards took all four ship track contract changes (`p1-contracts-split` Result, 17:56:
    changes 1 to 3; `p1-close-contracts`: change 4, `Event::Updated` and `Notice::Updated`, after Haik's yes to ship
    Question 3, sign-off item 7, Haik 2026-11-03 18:09), so build the whole item. `Event::UpdateReady` gives
    `Command::ApplyUpdate` only when no hold counts, no hold file names a live process (any kind, also with Automatic
    off) and no key is owned, at once or as soon as that becomes true; `Done(Outcome::Update(Ok))` gives `Quit`; `Err`
    waits for the next `UpdateReady`. `Event::Updated { version }` gives `Notify(Notice::Updated { version })`, once
    (it follows 8a like every notice). Tests: `updated_gives_notice`, and an `Updated` before any `Sources` still
    gives the notice.
10. `scenarios/lid.rs`, written before the code, one named test per item and these cases: lid closed on AC and on
    battery while holding; docked user with their own Do nothing (no lock, no sound); lid closed then unplugged after
    signing in on an external screen (lock after 5 s on battery; none for a 3 s AC drop) (power verify minor 11);
    work ends with the lid closed: hibernate after 5 minutes idle, not while typing; a resume during the countdown
    restarts the idle count; floor reached with the lid open and closed, then AC back; hibernate not allowed; Keep
    screen on with sleep timeout Never (no lock); closed-lid report once, with the right time across a suspend.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::lid` and `cargo test --locked -p wakeshot-core`: all pass (including
  the earlier scenarios); list the new test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency (as in `p2-core-holds`).
- Mutants, each made by hand, run, then reverted (record the failing test): lock at lid close when the user's own
  action is Do nothing; no lock at AC to battery with the lid closed; hibernate without the resume clamp; send
  `Play(GoingToSleep)` with `Hibernate`. Each must fail a test. Mutants run only in a scratch copy of the workspace,
  never in the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- The hibernate thread's last check and its sound, `LockWorkStation` (power track); chimes and their device (tray
  track). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

None. Both questions the orchestrator carried were answered (Haik 2026-11-03 21:23; Log).

## Found outside scope

Problems seen but not fixed.

1. Docs that still say 5 s for the power source settle (`architecture.md` and `product.md` already say 30 s):
   `docs/tasks/p2-core-props.md` line 48 (the property "unlocked for more than 5 s");
   `docs/tasks/p6-int-haik-checks.md` lines 84 and 88 ("Hear after about 5 seconds"); `docs/phases/01-design-skeleton.md`
   lines 21, 68 and 108 (the signed-off phase text). History, to leave: `docs/tasks/p2-policy-power.md`,
   `docs/research.md` line 1532.
2. For the power track (`p3-hibernate`): the hibernate thread's last check does not see holds or the work-end source
   rule. A hold that starts, or a dock that brings back AC onto the user's own Do nothing, during its few seconds of
   checks and sound does not stop the hibernate; the core cancels its countdown at once but cannot stop a thread
   already started. A fix would be a cancel the core can raise (a contract change). Not this card's.
3. `crates/wakeshot-core/src/policy/lid.rs` module doc (this card may edit that file only for the settle): "Each rule
   reads the last plan read and the core's journal mirror as they are at that event" no longer holds; the core passes
   what is in force (`Core::in_force`). Proposed: "Each rule reads the plan and journal as in force at that event: the
   core sets each journal entry's key to `written`, since the last read can be up to 10 s old."
4. `docs/architecture.md` section 3, for the docs pass: the in-force view; the closed-lid report goes out at the first
   unlock with the lid open, or when the lid opens while signed in; a countdown pauses while another session has the
   PC's screen; a lid close on our Do nothing with nothing holding starts the countdown; `ApplyUpdate` also waits for a
   countdown and does not wait for an expired Keep awake file; `Update(Ok)` takes the Quit path; held notices keep one
   of each duplicate; the countdown restarts after a source change pause only once the source lasted 30 s.
5. `sources::cron::tests::times_outside_the_js_range_give_none` (`p4-cron`'s file) failed once at about 21:43 and
   passed at 21:45. Nothing left.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:12, orchestrator: read `p2-policy-power`'s "Found outside scope" items 1 to 4 (countdown at a power
  source change, `lid::ours` after a failed restore, a second `Lock` after the lid-close lock, the first `Power`
  event has no source before). Item 1 went to Haik as a question; until he answers, keep the architecture's rule (not
  cancelled). The 5 s settle (`policy::lid::SOURCE_SETTLE_MS`) also waits for Haik (recommended 15 s).
- 2026-11-03 21:08: started by `wakeshot-worker`. Status: running. Reading the card's sources.
- 2026-11-03, between 21:08 and 21:42 (clock not read): sources read. Finding: the core's plan copy is the last read, up to 10 s old after an apply, so
  `policy::lid::ours` on it says "not ours" for a lid close in the first seconds of a hold (live 1, written 0): no
  lock. Fix in the core: the lid and lock rules get an "in force" view (journal entries on the active plan set their
  key to `written`; an apply in flight counts as written). Design: new `core/lid.rs` (lid, power, floor, countdown,
  idle lock, closed-lid report, held notices) and `core/update.rs` (9a); `Sim` gets an input world (`InputIdle`
  before each `Tick`, as the loop sends it, only after `Sim::input`). Next: `scenarios/lid.rs` first.
- 2026-11-03, between 21:23 and 21:42 (clock not read), orchestrator messages (21:22, 21:23): (1) Haik: the settle is 30 s; this card now also owns
  `policy/lid.rs` for that change only. Done: `SOURCE_SETTLE_MS = 30_000` and its doc; the file's tests use the
  constant, so none needed a change. (2) Orchestrator decision, confirmed by Haik: at a power source change during a
  running `WorkEnded` countdown, the user's own lid action for the new source decides: Do nothing cancels (only
  release), anything else keeps it; a flip back to a source whose own action is not Do nothing, lid still closed and
  work still ended, starts it again. Built as a derived rule: "work ended with the lid closed" is kept (with the user's
  own lid actions read at the work end, before the restore lands) and the countdown runs while the own action for the
  current source is not Do nothing.
- 2026-11-03, before 21:42 (clock not read): `scenarios/lid.rs` written first: 53 named tests and one proptest (`lid_events_in_any_order`),
  plus the `Sim` input world. Against the current core: 44 fail, 10 pass (nine "nothing happens" cases and the
  proptest, which only checks no panic and no unexplained lock there). Counts corrected at 21:47 from the run's
  output: first written here as 52 and 43. A flip back onto a source whose own action is
  not Do nothing restarts the countdown only once that source has lasted `SOURCE_SETTLE_MS`: else a dock's 3 s AC
  drop would hibernate a docked laptop left idle. Next: the code.
- 2026-11-03, before 21:42 (clock not read): code written: `core/lid.rs`, `core/update.rs`, wiring in `core.rs` and `core/settings.rs`
  (`take_session` gets `now`, `Locked`/`Unlocked` arms, `ConsoleBack` sends held notices), fields in `state.rs`. All
  54 lid tests pass, 445 in the crate. Traces read by eye (lid-close lock, unplug lock at +30 s, countdown due at
  300 s idle, report 391 s; docked cancel and restart; floor countdown). Added from the floor trace: after an
  `AboveFloor` skip the next `Floor` hibernate waits for a new battery reading (test added, 55). rustfmt on the owned
  files; clippy four runs plus `test-support`: exit 0. Next: review pass, mutants in a scratch copy.
- 2026-11-03 21:42 (clock read): the four times above were written without reading the clock; corrected to what is
  known. Review pass added: a lid close on our Do nothing with nothing holding (a failed restore) starts the work-end
  countdown, and during a step back the `Floor` one (our battery sleep Never would keep it running for good); held
  notices keep one of each exact duplicate; a failed lock while the session is already locked logs only. Tests added
  for the countdowns, two existing tests extended for the other two (57 in `scenarios::lid`). Mutant run started in `<scratchpad>\p2-core-lid\mutants\` (copy taken before
  the failed-lock change); results file `mutants-results.txt` beside it.
- 2026-11-03 21:46 (clock read): mutants: batch 1 (whole crate) 16 of 17 killed, one with a pattern that did not
  compile; batch 2 on the final code (`-- scenarios::`, see below) 3 of 3, the fixed one among them: 19 of 19. Copy
  and its build folder deleted. A full `cargo test -p wakeshot-core` at about 21:43 failed once in
  `sources::cron::tests::times_outside_the_js_range_give_none` (`p4-cron`'s file, building now); the rerun at 21:45
  passed (475). Long property run passed. `docs/research.md` "Core lid checks" added. Result written. Status: done.

## Result

Status: done 2026-11-03. The closed-lid and battery side of the episode is built test-first in a new private module
`core/lid.rs`, updates in `core/update.rs`: 56 named scenarios plus one property test. The first 53 and the
property were written before the code: 44 failed against the core then, 10 passed (nine "nothing happens" cases and
the property). Three tests came with the review additions; two others were extended.

### Files changed

- `crates/wakeshot-core/src/core.rs`: `mod lid; mod update;`; every event and outcome arm that named this card; the
  settle pass gets `tick`, runs the floor before the requests and the lid part after the episode edges, sends the
  update; `holding_ends` starts the countdown; `notify` keeps notices while the user is away; `label` shared by the
  log and the report. Public signatures unchanged.
- `crates/wakeshot-core/src/core/lid.rs` (new), `crates/wakeshot-core/src/core/update.rs` (new): the rules below.
- `crates/wakeshot-core/src/core/settings.rs`: `KEYS`, `TARGET` visible to `core`; `take_session(change, now, out)`
  with the `Locked` and `Unlocked` arms; `ConsoleBack` sends the held notices.
- `crates/wakeshot-core/src/state.rs`: the lid fields filled (every `expect(dead_code)` gone), new ones
  (`source_since_mono`, `step_back_shown`, `work_end`, `floor_countdown`, `hibernating`, `above_floor_skipped`,
  `idle`, `idle_lock_check`, `closed_lid`, `update_ready`, `updating`, `updated_shown`; types `WorkEnd`,
  `IdleReading`, `ClosedLid`; `Countdown { reason, check }`). Removed: `floor_pct` (read from the plan) and
  `Episode.lid_closed_since_mono` (the report must outlive the episode: after work ends the closed laptop still runs
  until the hibernate).
- `crates/wakeshot-core/src/sim.rs`: the input world (`Sim::input`, `Sim::idle_ms`).
- `crates/wakeshot-core/src/scenarios/lid.rs`: the tests.
- `crates/wakeshot-core/src/policy/lid.rs`: `SOURCE_SETTLE_MS = 30_000` and its doc only (Haik 21:23, orchestrator
  grant); its tests use the constant, so none changed.
- `docs/research.md`: "Core lid checks" (addition only). This card.

### How it works

- **What the lid rules see.** The plan copy is the last read, and after an apply it shows the user's values for up to
  10 s, so `policy::lid::ours` on it said "not ours" for a lid close in the first seconds of a hold: no lock (found
  here, fixed). `Core::in_force` gives the policy the plan with each journal entry on the active plan set to
  `written`, and an apply not answered yet counted as written. The lid rules, the source lock, the idle lock timeout
  and the work-end countdown all read it.
- **Lid closes** (a change from open, not the first reading) while our Do nothing is in force on the current source
  (either source before the first `Power`): `Log`, `Lock`, `Play(LidClosedHolding)`, the lock first. The user's own Do
  nothing: nothing. While holding, the closed-lid stretch starts. With nothing holding (a failed restore left our Do
  nothing and battery sleep Never), the work-end countdown starts too; during a step back, the `Floor` one.
- **Power source changes** (not the first reading): `policy::lid::source_change` with the lid closed, kept in place of
  an earlier one; `policy::lid::settle` at each later step: `Lock` and the chime once the new source lasted 30 s
  (`Deadline::Mono`), dropped by a change back, a lid open, a release or a plan switch. AC back cancels a `Floor`
  countdown.
- **Battery floor** (`policy::floor`, the floor from the plan's energy saver threshold), in the settle pass before the
  requests. Stepping back logs; its actions come once per step back, at the first moment a hold counts in it: requests
  off (settle), `Restore { plan: None, why: Floor }` when anything may be owned, `Notify(StepBack)`,
  `Play(BatteryFloor)`, and the `Floor` countdown if the lid is closed. No hold holds while stepped back. AC back: log,
  `want_apply`, so requests and the apply come as at a hold start.
- **Countdown**, derived in each settle pass from two wants. Work ended with the lid closed (set at the hold end when
  the user's own lid action is not Do nothing, read before the restore lands; both sources' own actions kept) and a
  step back with the lid closed. Reason `WorkEnded` wins. Orchestrator decision (21:22, Haik's yes 21:23): at a source
  change the user's own action for the new source decides: Do nothing pauses it (only the release), anything else
  keeps it; it starts again once a source whose own action is not Do nothing lasted 30 s. Paused while another
  session has the PC's screen. Cancelled by holding again (`Cancel::HoldingStarts`), the lid opening, AC back for
  `Floor`. Due only at a fresh `InputIdle` (clamped at the last resume): `Hibernate { reason, floor_pct,
  last_resume_mono_ms }`, no `Play`, one at a time; `Floor` also needs the core's own percent under the floor.
- **`Done(Hibernate(..))`, the rule asked for in item 5:** `Resumed`: a resume (idle clamp, `Retake` if any request
  is on), the countdown is done. `NotAllowed`, `Failed`, `Skipped(LidUnknown)`: log, `Notify(HibernateFailed)`, the
  countdown ends (section 8 names the unknown lid). `Skipped(LidOpen)`: it ends. `Skipped(InputSeen)`: it goes on, the
  next reading decides. `Skipped(OnAc)`: ends `Floor` only. `Skipped(AboveFloor)`: goes on, but no second `Floor`
  hibernate before a new battery reading (else one per reading while the two readings disagree).
- **Keep screen on idle lock**: at each `InputIdle`, `policy::lock::idle_lock` with the user's own timeout (the
  shorter of the two before the first `Power`): `Log`, `Lock`, `Notify(IdleLocked)`, once per idle period; its next
  check is a deadline. `Done(Locked(Err))`: log and `Notify(LockFailed)`, but only log while the session is locked.
- **Closed-lid report**: a stretch runs from a lock that leaves the closed laptop running on our Do nothing while
  holding (lid close or source lock) to the lid opening; run time only, so a hibernate in it does not count. It goes
  out at the first `Unlocked` with the lid open, or when the lid opens while signed in:
  `ClosedLidReport { ran_ms, held_by }`, `held_by` the names of everything that held in the stretch. An unlock with
  the lid still closed (a sign-in on the external screen) sends none: the stretch goes on.
- **Notices while away**: raised while the session is locked or off the console, they wait and go out in order at
  `Unlocked` or `ConsoleBack` (whichever makes both true); one of each exact duplicate. Sounds never wait.
- **Updates**: `UpdateReady` waits until nothing holds, no hold file names a live or Unknown process (any kind, also
  with Automatic off), nothing may be owned (`may_own`), no countdown, work end or hibernate is on, and the session
  is not ending; then `ApplyUpdate`, one at a time. `Update(Ok)`: the menu's Quit path (requests off,
  `Restore { why: Quit }`, Keep awake file deleted, `Quit`). `Update(Err)`: log, wait for the next `UpdateReady`.
  `Updated`: `Notify(Updated)` once.
- **`Suspending`**: nothing (run time stops by itself). **`Resumed`**: the last resume time.

### Design choices (mine), each with when to change it

1. The in-force view errs toward a lock: an entry the user changed in the last 10 s, or an apply in flight that
   fails, still counts as ours. Change it when a plan read reports the value in force right after each apply.
2. The first `Lid` and `Power` readings are states, not changes: a tray that starts with the lid closed locks
   nothing. Change it when a tray start with the lid closed and no lock is seen to matter.
3. Before the first `Power` event: lock if ours on either source, the shorter idle timeout, the countdown if either
   own action is not Do nothing. Change it when `Started` carries the power source.
4. A lid close on our Do nothing locks and chimes with no hold too (a failed restore), and starts the countdown (work
   end, or `Floor` while stepped back): our Do nothing with battery sleep Never would keep a closed laptop running for
   good. Change it when a failed restore can no longer leave our values in force.
5. `Lock` goes also into a session already locked (a lost `Unlocked` must never skip a lock); what `LockWorkStation`
   returns then is not verified, so a failure while locked is logged only. Change it when the power track shows the
   return value.
6. Order `Lock` then `Play`: both orders were heard to work; this one never leaves the session unlocked behind a slow
   sound start.
7. Step back actions once per step back, at the first moment a hold counts; a step back with nothing held only logs.
   Change it when Haik wants a notice whenever the floor is reached.
8. The `Floor` countdown follows the architecture literally (the lid closed at the step back), with no exception for a
   user whose own battery lid action is Do nothing, unlike the work-end rule. Change it when Haik wants the same
   exception there.
9. The restart after a source change pause waits 30 s (`SOURCE_SETTLE_MS`): without it a dock's 3 s AC drop would
   hibernate a docked laptop left idle after work ended. Change it with the settle value.
10. The countdown pauses while another session has the PC's screen: input idle is this session's, so the other
    user's typing would not count and their PC would hibernate. Change it never for safety.
11. Only a fresh `InputIdle` can make the countdown due; a timer that fires without one waits for the next reading,
    at the latest the safety tick. The loop sends `InputIdle` before every `Tick` (`06-integration.md`).
12. `IdleLocked` goes out at once (it is raised before the lock), not held to the unlock. Change it when the Phase 5
    or 6 checks show that balloon is lost behind the lock screen: then push it to the held list.
13. The closed-lid report waits for the lid to open (or goes out when it opens while signed in), so a docked user who
    signs in on the external screen gets no report at each lid close. Change it when Haik wants it at every sign-in.
14. `held_by` names are the log's (agent label, hold reason, "Keep awake", "wakeshot run", "a hold file"); the tray's
    `text` words the notice around them. Change it when the notice needs the kinds (a contract change).
15. An expired Keep awake file does not hold an update back: it always names this tray, which quits for the update,
    and Keep awake ends with the tray anyway (decision 32). Change it when other processes write that file.
16. Known limit: after a restore the plan copy may show our values until the next read (up to 30 s once nothing is
    owned), so a work end in that window after an earlier restore (say "Restore my power settings" while docked with
    the lid closed) reads the user's own action as Do nothing and starts no countdown. Change it when a restore's
    outcome carries the values it put back, or the core reads the plan after each restore.
17. Known limit: `Notify(FirstRun)` from `Started` goes out before a start-time `Session(Locked)` can arrive (the
    holds scenarios pin it to `Started`).

### Seams for `p2-core-props` and `p6-int-dispatch`

- `Sim::input()` (input now; from then on every loop timer posts `InputIdle` before its `Tick`), `Sim::idle_ms()`;
  `Sim::suspend` adds the sleep to input idle, as `GetLastInputInfo` does. Without `input()`, no `InputIdle`.
- The core decides the countdown only on an `InputIdle` at the same `now`, so the loop must keep "`InputIdle`, then
  `Tick`" at each fire. `Done(Hibernate(Resumed))` counts as a resume, like `Resumed`.
- `p2-core-props`: the property "never on battery on our Do nothing unlocked for more than 5 s" is now 30 s
  (`policy::lid::SOURCE_SETTLE_MS`).

### Acceptance checks (final run 2026-11-03 21:45, own build folder, every cargo command `--locked`)

- `cargo test -p wakeshot-core scenarios::lid`: 57 passed. Tests: `a_failed_lock_shows_a_notice`,
  `a_failed_update_waits_for_the_next_update_ready`, `a_hibernate_that_resumed_counts_as_a_resume`,
  `a_new_hold_cancels_the_countdown`,
  `a_power_source_change_onto_the_users_own_do_nothing_cancels_the_countdown_and_back_starts_it`,
  `a_power_source_change_onto_the_users_own_sleep_keeps_the_countdown`,
  `a_resume_during_the_countdown_restarts_the_idle_count`, `a_short_ac_drop_with_the_lid_closed_gives_no_lock`
  (3 s and 6.3 s), `a_start_time_restore_failure_waits_for_the_unlock`,
  `a_tray_started_while_locked_holds_the_updated_notice_until_unlock`,
  `ac_back_cancels_the_floor_countdown_and_holds_again`, `an_expired_keep_awake_file_does_not_hold_the_update_back`,
  `another_session_on_the_console_holds_the_countdown_back`,
  `battery_floor_with_the_lid_closed_hibernates_after_5_minutes_idle`,
  `battery_floor_with_the_lid_open_steps_back_then_ac_back_holds_again`,
  `closed_lid_report_once_with_the_right_time_across_a_suspend`,
  `closed_then_unplugged_after_signing_in_on_an_external_screen_locks_once_battery_lasted`,
  `docked_user_with_their_own_do_nothing_gets_no_lock_and_no_sound`,
  `docked_with_their_own_ac_do_nothing_then_unplugged_locks_once_battery_lasted`,
  `hibernate_not_allowed_shows_a_notice_and_ends_the_countdown` (not allowed, failed, lid unknown),
  `hibernate_skipped_above_the_floor_waits_for_a_new_battery_reading`, `hibernate_skipped_for_input_goes_on`,
  `hibernate_skipped_on_ac_ends_only_the_floor_countdown`, `hibernate_skipped_with_the_lid_open_ends_the_countdown`,
  `keep_screen_on_locks_once_per_idle_period`, `keep_screen_on_with_sleep_timeout_never_never_locks`,
  `lid_closed_after_a_failed_restore_still_locks_and_chimes`,
  `lid_closed_during_a_step_back_with_our_do_nothing_left_counts_down`,
  `lid_closed_in_the_first_seconds_of_a_hold_locks`, `lid_closed_on_a_failed_restore_after_work_ended_counts_down`,
  `lid_closed_on_ac_while_holding_locks_and_chimes`, `lid_closed_on_battery_while_holding_locks_and_chimes`,
  `lid_closed_while_the_apply_is_in_flight_locks`, `lid_closed_with_nothing_of_ours_gives_nothing`,
  `lid_events_in_any_order` (proptest), `no_closed_lid_report_for_the_users_own_do_nothing`,
  `no_closed_lid_report_while_the_lid_stays_closed_then_one_when_it_opens`, `no_hold_starts_while_stepped_back`,
  `notices_raised_off_the_console_go_out_when_it_comes_back`,
  `notices_raised_while_locked_go_out_at_unlock_in_order`, `opening_the_lid_cancels_the_countdown`,
  `sounds_never_wait`, `the_closed_lid_report_names_everything_that_held`, `the_first_lid_reading_is_no_close`,
  `the_first_power_reading_is_no_source_change`, `the_floor_follows_the_energy_saver_threshold`,
  `update_ready_applies_when_nothing_holds_and_ok_quits`,
  `update_ready_waits_for_the_hold_to_end_and_the_settings_to_come_back`,
  `update_waits_for_a_live_hold_file_even_with_automatic_off` (alive and Unknown), `update_waits_for_the_countdown`,
  `update_waits_while_a_key_is_owned`, `updated_before_any_sources_gives_the_notice`, `updated_gives_notice`,
  `work_ended_after_a_long_idle_hibernates_at_once`, `work_ended_with_the_lid_closed_does_not_hibernate_while_typing`,
  `work_ends_with_the_lid_closed_hibernates_after_5_minutes_idle`,
  `work_ends_with_the_users_own_do_nothing_gives_no_countdown`. Timing tests check times off the 30 s safety tick.
- `cargo test -p wakeshot-core`: 475 passed, 0 failed (`scenarios::holds` and `scenarios::settings` unchanged and
  passing; `policy::lid` with the 30 s settle passing).
- `lid_events_in_any_order` (no panic, no deadline behind a `Tick`, every `Lock` with a chime or `IdleLocked` in its
  step, no `Hibernate` within 5 minutes of input or a resume, never `Play(GoingToSleep)`, nothing after `Quit`):
  `PROPTEST_CASES=10000`, 27.4 s, passed. No regression file (`failure_persistence: None`).
- `rustfmt --edition 2024 --check` on `core.rs` (and its children `core/holds.rs`, `core/lid.rs`,
  `core/settings.rs`, `core/update.rs`, `core/view.rs`), `state.rs`, `sim.rs`, `scenarios/lid.rs`, `policy/lid.rs`:
  exit 0.
- `cargo clippy -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0. Also `--features test-support` (x64): 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal --depth 1` shows serde and
  serde_json only; a grep of the owned files for `std::time`, `std::fs`, `std::env`, `std::net`, `std::process`,
  `std::thread`: none.
- Mutants, in a scratch copy (`<scratchpad>\p2-core-lid\mutants\`, own build folder), each reverted; 19 of 19 killed.
  The card's four: a lock at a lid close on the user's own Do nothing (lock when the action in force is Do nothing,
  whoever wrote it): `docked_user_with_their_own_do_nothing_gets_no_lock_and_no_sound` and 2 more; no lock at AC to
  battery with the lid closed: `closed_then_unplugged_after_signing_in_on_an_external_screen_locks_once_battery_lasted`,
  `docked_with_their_own_ac_do_nothing_then_unplugged_locks_once_battery_lasted`; a hibernate without the resume
  clamp: `a_resume_during_the_countdown_restarts_the_idle_count`, `lid_events_in_any_order`; `Play(GoingToSleep)` with
  `Hibernate`: `work_ends_with_the_lid_closed_hibernates_after_5_minutes_idle`,
  `battery_floor_with_the_lid_closed_hibernates_after_5_minutes_idle`, `lid_events_in_any_order`. Extra: the plain
  plan read instead of the in-force view (14 tests, among them `lid_closed_in_the_first_seconds_of_a_hold_locks`); an
  apply in flight not counted (`lid_closed_while_the_apply_is_in_flight_locks`); notices never wait (6); the first lid
  reading as a close (`the_first_lid_reading_is_no_close`); the report at an unlock with the lid closed (2); the
  orchestrator rule removed and the restart with no settle (both
  `a_power_source_change_onto_the_users_own_do_nothing_cancels_the_countdown_and_back_starts_it`); the countdown off
  the console (`another_session_on_the_console_holds_the_countdown_back`); a fired timer keeping an old check and a
  stale reading making it due (`lid_events_in_any_order`, the second also `a_hibernate_that_resumed_counts_as_a_resume`);
  the update ignoring owned keys (2); no `AboveFloor` gate (1); no countdown at a lid close on a failed restore (1); a
  failed lock while locked notifying (1); held duplicates (1). Batch 2 ran `-- scenarios::` because another card's
  test failed at the time (Log 21:46).
- Only the owned files changed (from my own edit record; git not run), plus this card and the `research.md`
  addition. Build folder, mutant copy and scratch scripts deleted.

### Contract proposals

None needed. Optional, for design choice 16: `Outcome::Restored` could carry the values it put back on the active
plan, so the plan copy is right at once.

### Open items

- For Haik, none blocking. Design choices 8 and 13 are product-near readings he may want to see.
- Found outside scope 1 to 5.
- This card's code is in the power safety review scope (`docs/phases/02-06-lighter.md`, "Reviews"): the in-force view,
  design choices 4, 5, 9, 10, 11, 16, and the `Done(Hibernate)` rules.

## Power review fixes (2026-11-03)

Status: done. Agent: `wakeshot-worker` (work name `p2-power-fixes`). Brief from the orchestrator: power safety
review (`docs/reviews/p2-6-power-safety.md`) findings 1, 2, 4 and 7, and `p2-core-props` findings 1 and 2. Owns
`core.rs`, `core/`, `scenarios/lid.rs`, `scenarios/settings.rs`, `scenarios/props.rs`, `sim.rs`, the section 8 "Lock
fails" row. Not `state.rs`.

### Log

- 2026-11-03 23:41: started. Read the review, this card's Log and Result, `p2-core-props` Result and Found outside
  scope, section 8, and the core code. `state.rs` is not mine this round, so the new lock state lives in a private
  field of `Core`.
- 2026-11-03 23:45 (orchestrator message): finding 7 added, Haik's answer: no floor countdown when the user's own
  battery lid action is Do nothing, on both paths. Haik's principle: with no work going on, the laptop behaves as if
  Wakeshot were not there.
- 2026-11-03 23:50: design. Finding 1: a closed-lid `Lock` (lid close or source lock) into an unlocked session starts
  a watch; `Session(Locked)` ends it; `Done(Locked(Err))` or 5 s with no `Locked` makes the next tick send `Lock`
  once more, and the same again steps back (requests off, `Restore { plan: None, why: Floor }`, the work-end
  countdown) until the session locks or the lid opens. Finding 2: a failed write always sends one restore. Finding 7:
  the floor countdown needs the user's own battery lid action not Do nothing, one rule for both paths. Props 1: a new
  release while an apply is on its way or an entry holds the key asks for a new apply (or a restore when every key is
  released). Props 2: before the first plan read the lock rules read a guess from the `Started` journal. Finding 4:
  not built, see Decisions: in that case the user's own AC Do nothing kept the laptop running at the close, so
  Haik's 23:45 principle and the product text ("Wakeshot changed nothing, so it only releases its hold") say no
  countdown; a test pins it.
- 2026-11-03 23:58: 15 new scenarios written first (11 in `scenarios/lid.rs`, 4 in `scenarios/settings.rs`).
  Against the unchanged core 12 fail and 3 pass, as expected: `a_failed_lock_needs_no_retry_once_the_lid_opens_or_the_session_locks`
  (nothing to retry), the finding 4 pin, and finding 7's second path (a lid close after the step back already needs
  our Do nothing in force on battery, which means the user's own battery action is not Do nothing).
- 2026-11-04 00:01: code done (`core/lid.rs`, `core/settings.rs`, `core.rs`). Four older lid scenarios closed the lid
  and never sent the `Session(Locked)` Windows posts after a lock, so the new watch retried: each now sends it. The two
  "every lock is explained" properties (lid proptest and props property 10) allow only the one retry; the props model
  learned the lock watch and its step back from the section 8 row. Both ignored strict props tests un-ignored.
  `cargo test -p wakeshot-core --lib`: 552 passed, 2 ignored (`coverage_tally`, `p2-ownership-props`' finding).
  `PROPTEST_CASES=10000` release run of `scenarios::props`, the lid proptest and `scenarios::settings`: 54 passed.
  `cargo mutants` is not installed and the card used hand-made mutants, so the mutants run by hand next.
- 2026-11-04 00:04: `lock_view` simplified: every journal entry has `written` 0 and a non-zero `original` (the apply
  never journals a key already at its target), so it takes the first entry per lid key. One more case in
  `a_failed_lock_needs_no_retry_once_the_lid_opens_or_the_session_locks`: the user takes the lid key back after the
  failure, so no retry and the hold goes on (it pins the "our Do nothing in force" guard). 552 lib tests pass.
- 2026-11-04 00:04 (orchestrator message, 00:02 or so): `text.rs` is mine for the `LockFailed` words. They now say what
  follows a failed closed-lid lock: "Windows did not lock the PC. Lock it with Windows key + L. With the lid closed,
  Wakeshot tries once more; if that fails, it stops keeping the PC awake and hibernates it after 5 minutes with no
  input." (199 characters, under the 200 the balloon guideline asks for; it also fits the Keep screen on lock with the
  lid open). Left out "unless your own lid action is Do nothing": when this notice comes for a closed lid, the user's
  own action for that power source is never Do nothing (else no lock goes out), so it matters only after a later
  dock, and the words would pass 200. No other notice is raised in a new case: `HibernateFailed` after a lock step
  back still holds ("Your power settings are back"). `wording_snapshot` updated.
- 2026-11-04 00:11: orchestrator message: section 3's "Lid closes" bullet now also says lock and chime with nothing
  holding (verified in `take_lid`: the lock goes out on our Do nothing before the episode is looked at; test
  `lid_closed_after_a_failed_restore_still_locks_and_chimes`). Section 8's "Lock fails" row gets the 5 s check and when
  the step back ends; re-read right before each edit. Mutants by hand in a scratch copy (runner `mutants.py`, own build
  folder): the card's four and 17 for this round, 20 killed at once; "applies during the lock step back" survived (no
  test had an edge that wants an apply then), so `nothing_is_applied_while_stepped_back_after_a_failed_lock` was
  added: then killed, 21 of 21.
- 2026-11-04 00:14: final checks green (Result). `docs/research.md` "Power review fix checks" added. Build folder,
  mutant copy and scratch scripts deleted. Status: done.

### Result

Status: done 2026-11-04. Findings 1, 2 and 7 of the power safety review and findings 1 and 2 of `p2-core-props` are
fixed, each with scenarios written first; finding 4 is kept as it is, by Haik's 23:45 principle (Decisions). 16 new
scenarios, both strict props tests un-ignored, the `LockFailed` words updated.

#### What changed

- **Finding 1 (major), a failed or unconfirmed closed-lid lock.** A lid-close or source lock into an unlocked session
  is watched. `Session(Locked)` ends the watch. `Done(Locked(Err))` brings the check to the next tick (at once), and
  no `Locked` within 5 s counts the same, since `LockWorkStation` only starts the lock. At the check, with the lid
  still closed on our Do nothing: `LockFailed` (once for the lock and its retry), then `Lock` once more; when that one
  goes the same way, the step back: requests off and no apply (as at the floor), `Restore { plan: None, why: Floor }`,
  and the work-end countdown (hibernate after 5 minutes with no input; it pauses on a source whose own lid action is
  Do nothing). The step back ends when the session locks (a late `Locked`, or Windows' own lock at screen off) or the
  lid opens: then the hold applies again as at a hold start. If the lid opened or our Do nothing is gone at the check
  (a release, or the hold ended and the restore landed), it only logs.
- **Finding 2 (minor), a failed write whose own restore could not load the journal.** A `Write` failure always sends
  one `Restore { plan: None, why: WorkEnded }`, whatever the copy holds; a restore of a missing journal costs nothing.
  A `SafetyNet` failure writes nothing, so it still restores only a copy that holds entries. Why this over setting
  `retry`: the restore goes at once instead of up to 10 s later, and if it fails it sets `retry` by the usual rule.
- **Finding 7 (minor, Haik 23:45), the floor countdown and the user's own battery Do nothing.** One rule for both
  paths (`floor_countdown_starts`): the lid closed and the user's own battery lid action (the journal's `original`
  while we own it) not Do nothing; no plan read means nothing was applied, so no countdown. The step back with the lid
  already closed was the path that broke it; a lid close after the step back already needed our Do nothing in force
  on battery, which implies the same, so its test passed before too and now guards it.
- **Props finding 1, a release while an apply is on its way.** A key newly released in an episode, while an apply is
  not answered or another plan's entry holds the key, sets the apply to `Wanted`: the next apply leaves the key out and
  restores this plan's entry for it (track note 3). With every key released no apply comes, so a restore goes out.
  This covers the plan switch case (`Plan` before the check's `UserChanged`), a release named on an inactive plan, and
  an apply queued behind the read that dropped the entry.
- **Props finding 2, a lid close before the first plan read.** The lid-close and source lock rules read `lock_view`:
  what is in force, or before the first read a guess from the `Started` journal (each lid key a journal entry names is
  ours in force on one stand-in plan, any other lid key Sleep). The work-end countdown and the idle lock still need a
  real read.
- **Docs.** Section 3 "Lid closes": also with nothing holding. Section 8 "Lock fails": the 5 s check, the work-end
  countdown, when the step back ends. `text.rs`: the `LockFailed` words (Log 00:04).

#### Decisions (mine), each with when to change it

1. Finding 4 not built: docked with their own AC Do nothing, work ends on AC, then unplugged: no countdown. Their own
   Do nothing kept the laptop running at the close, so Wakeshot changed nothing (product, work end: "it only releases
   its hold"), and with no work going on the laptop behaves as if Wakeshot were not there (Haik 23:45). The cost: that
   laptop runs on battery with the lid closed until the user's own battery sleep timeout, unlocked, as it would
   without Wakeshot. Pinned by `work_ended_on_the_users_own_ac_do_nothing_then_unplugged_gives_no_countdown`. Change
   it when Haik wants the countdown there: keep `WorkEnd` paused at the work end (the review's fix, a few lines in
   `work_ended`).
2. The lock watch lives in a private field of `Core` (`lid::LockWatch`), not in `State`: `state.rs` was not this
   round's file. Change it when a card owns `state.rs`: move the three fields there.
3. The retry goes at the next tick, at once after an `Err` (the review's wording), 5 s after an `Ok` with no
   `Locked`. Change it when a real check shows `LockWorkStation` failures that clear within seconds: then wait that
   long.
4. One `LockFailed` per lock and its retry, at the first failure; the step back adds a log line, no second notice.
   Change it when Haik wants a notice of its own for the step back (a new `Notice` variant, a contract change).
5. The lock step back's restore uses `RestoreWhy::Floor`, the closest reason; the core logs why. Change it when the
   contract gets a reason for it (Contract proposals).
6. The props model learns the lock watch from the section 8 row and the step's `Lock` commands, and says "stepped
   back" whenever the core may have (it cannot see the core's in-force view), so it is never stricter than the core.
   Change it when a property needs the exact step back.

#### Tests

- New in `scenarios/lid.rs` (12): `a_lid_close_before_the_first_plan_read_locks_on_a_crashed_trays_do_nothing`,
  `unplugged_before_the_first_plan_read_locks_on_a_crashed_trays_do_nothing_once_battery_lasted`,
  `battery_floor_with_the_lid_closed_and_the_users_own_battery_do_nothing_gives_no_countdown`,
  `a_lid_close_after_the_step_back_with_the_users_own_battery_do_nothing_gives_no_countdown`,
  `work_ended_on_the_users_own_ac_do_nothing_then_unplugged_gives_no_countdown`,
  `a_failed_lid_close_lock_is_sent_once_more_at_the_next_tick`,
  `a_lid_close_lock_that_fails_twice_steps_back_and_hibernates_after_5_minutes_idle`,
  `a_lock_the_session_does_not_confirm_within_5_s_is_sent_once_more_then_steps_back`,
  `nothing_is_applied_while_stepped_back_after_a_failed_lock`,
  `the_session_locking_or_the_lid_opening_ends_the_step_back`,
  `a_failed_lock_needs_no_retry_once_the_lid_opens_or_the_session_locks`, `a_failed_source_lock_is_sent_once_more_too`.
- New in `scenarios/settings.rs` (4): `a_failed_write_whose_own_restore_cannot_load_the_journal_still_restores`,
  `a_release_while_the_switchs_apply_is_on_its_way_applies_again_without_the_key`,
  `a_release_named_on_another_plan_takes_the_active_plans_override_back`,
  `every_key_released_while_an_apply_is_on_its_way_restores`.
- Before the fix: of the first 15, 12 failed and 3 passed by design (Log 23:58); the 16th came from a surviving
  mutant and kills it.
- Un-ignored in `scenarios/props.rs`: `a_released_key_is_never_ours_in_force_again_in_its_episode` (props finding 1)
  and `a_closed_laptop_is_locked_also_before_the_first_plan_read` (props finding 2). Both pass.
- Changed: four lid scenarios now send the `Session(Locked)` Windows posts after a lock
  (`lid_closed_on_ac_while_holding_locks_and_chimes`, `lid_closed_during_a_step_back_with_our_do_nothing_left_counts_down`,
  `the_first_power_reading_is_no_source_change`, `work_ended_after_a_long_idle_hibernates_at_once`); the lid proptest
  and props property 10 allow only the one retry of a lid lock; `text::tests::wording_snapshot` has the new words.

#### Checks (final runs 2026-11-04 00:11 to 00:14, own build folder, every cargo command `--locked`)

- `rustfmt --edition 2024 --check` on `core.rs` (with `core/`), `scenarios/lid.rs`, `scenarios/settings.rs`,
  `scenarios/props.rs`, `sim.rs`, `text.rs`: exit 0.
- `cargo clippy -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same without
  `--all-targets`: both exit 0.
- `cargo test -p wakeshot-core`: 553 passed, 2 ignored (`coverage_tally`, `p2-ownership-props`' open finding), plus
  10 and 19 in `tests/`.
- `PROPTEST_CASES=10000 cargo test -p wakeshot-core --lib --release -- scenarios::props
  scenarios::lid::lid_events_in_any_order scenarios::settings`: 54 passed in each of three runs, the last two on the
  final code.
- Mutants by hand, 21 of 21 killed (Log 00:11). The card's four: a lock on any Do nothing in force
  (`docked_user_with_their_own_do_nothing_gets_no_lock_and_no_sound` and 5 more); no lock at AC to battery
  (`a_failed_source_lock_is_sent_once_more_too` and 9 `policy::lid` tests); the hibernate without the resume clamp
  (`a_resume_during_the_countdown_restarts_the_idle_count` and 2 more); the going to sleep sound with `Hibernate`
  (`work_ends_with_the_lid_closed_hibernates_after_5_minutes_idle` and 4 more). 17 for this round, each killed by at
  least one of the new tests.
- No clock read, no I/O, no new dependency. Only the owned files changed (from my own edit record; git not run), plus
  this card, `docs/architecture.md` (two lines), `docs/research.md` (an addition) and `text.rs` (granted 00:02).

#### Contract proposals

- `RestoreWhy::LockFailed` (or `StepBack`), so the lock step back's restore is not logged as `Floor` on the IO side.

#### Found outside scope

1. `docs/tasks/p5-text.md` lines 101 and 222 still quote the old `LockFailed` words; line 222 is the wording table
   Haik reads. That is `p5-text`'s file.
2. While the lock step back lasts, the tray shows "Cannot hold awake: ..." (holds count, no request), which reads like
   a failed request. A status of its own needs a `Status` field (`text.rs`, `present.rs`): a contract change, low
   priority, since the `LockFailed` notice says why.

#### Open items

- Decision 1 (finding 4) is my reading of Haik's 23:45 principle; the orchestrator may want Haik's word on it.
- Not in this round: review findings 3, 5, 6, 8, 9, 10.

## Core round A, lid fixes (2026-11-04)

Status: done. Agent: `wakeshot-worker`. Brief from the orchestrator: power safety review
(`docs/reviews/p2-6-power-safety.md`) finding 5, and minors 1 to 5 of `docs/reviews/p2-6-power-safety-verify.md`.
Owns `core/lid.rs`, `policy/lid.rs`, `scenarios/lid.rs`, `text.rs` (only for new notice words), section 3 lid lines of
`docs/architecture.md`, `docs/product.md` lid lines only if a rule changes, this card, and a new section in
`docs/research.md`. Not `core.rs`, `core/settings.rs`, `ownership/`, `scenarios/settings.rs`, `scenarios/props.rs`
(core round B), nor `crates/wakeshot-windows/`.

### Log

- 2026-11-04 01:08: started. Read the card, both reviews, the brief.
- 2026-11-04 01:23: read `core/lid.rs`, `core.rs`, `core/settings.rs`, `state.rs`, `policy/lid.rs`, the lid scenarios
  and the props model (read only). Design:
  - Finding 5: `done_restored` is not mine, so the lid side does it by diff: each settle pass ends by keeping the
    journal copy it saw; an entry that left it since, on the copy's active plan, whose key the copy still shows at
    `written`, puts its `original` back in the plan copy (`in_force` sees it at once, the fold makes it last). A real
    read overwrites it, so a value the user set since wins.
  - Minor 3: a countdown rule that meets the lid closed before the first plan read waits for that read when anything
    may be owned (the `Started` journal), then decides on the read: exact, where the `lock_view` guess could take a
    docked user's own AC Do nothing for Sleep. The floor rule had the same hole; fixed the same way.
  - Minor 1: decision 0 (every `Lock` with a notice or a sound) rules out silent lock retries. Picked: after a
    hibernate that is not allowed, failed or found the lid unknown, with the lid still closed, `Lock` with the chime
    once input has been idle 5 minutes (a fresh reading), and again every 5 minutes, while the session is unlocked,
    on the console and the user's own lid action for the source is not Do nothing; it ends when the lid opens. Not
    the sleep fallback: it needs the hibernate thread and maybe a contract change, and on Modern Standby
    `SetSuspendState(FALSE)` hibernates too (research), so it adds nothing where hibernation is off there.
  - Minor 2: `LockFailed` at the lock step back with the lid closed, and `HibernateFailed` raised with the lid closed
    and the session unlocked, are kept and shown again at the user's return, with the closed-lid report.
  - Minor 4: the two scenarios in `scenarios/lid.rs`; the props part is `scenarios/props.rs` (round B).
- 2026-11-04, between 01:23 and 01:44 (clock not read; corrected at 01:44): 12 new scenarios written first in `scenarios/lid.rs`. Against the unchanged core 8 fail (finding
  5's work end after a restore; minor 3's lock step back, no-hold lid close and floor step back before the first
  read; minor 1's three relock tests; minor 2's kept notices) and 4 pass by design (minor 4's two, which pin code
  that is right today, and finding 5's two guards: a later read wins, and a value a read already shows stays).
  `scenarios::lid`: 73 passed, 8 failed. Next: the code.
- 2026-11-04 01:44 (clock read): code done in `core/lid.rs`; three more cases added from the mutant plan (a lid open
  drops a want waiting for the read; no relock off the console; a `HibernateFailed` held while locked is not shown
  twice). `scenarios::lid` 81 passed; `cargo test -p wakeshot-core` 577 passed, 2 ignored, plus 10 and 25 in
  `tests/`. Long runs (`PROPTEST_CASES=10000`, release, `scenarios::props`, the lid proptest, `scenarios::settings`):
  `scenarios::props::a_released_key_is_never_ours_in_force_again_in_its_episode` failed in three of seven runs,
  each with plan switches and failed restores and no lid command in the trace; outside this round (Found outside
  scope). rustfmt on the owned files, clippy four ways: exit 0. `docs/architecture.md` section 3 and the two
  `docs/product.md` lid lines updated. Next: mutants in a scratch copy.
- 2026-11-04 01:51 (clock read): mutants in a scratch copy: 27 of 27 killed (the first run stopped at the baseline:
  `holds/file.rs` tests read `docs/hold-files.md`, so the copy needs `docs/`). Final `cargo test -p wakeshot-core`:
  577 passed, 2 ignored. `docs/research.md` "Core lid round A checks" added. Result written. Status: done.

### Result

Status: done 2026-11-04. Finding 5 of the power safety review and minors 1 to 5 of its verify are fixed, each code
fix with scenarios written first (12, of which 8 failed before the fix), three more from the mutant plan; minor 5 and
the new rules are in the docs.

#### What changed

- **Finding 5, a stale plan copy after a restore.** `done_restored` is not this round's file, so the lid side does
  what the review asks by diff: each settle pass ends by keeping the journal copy it saw (`journal_seen`); an entry
  that left it since, on the copy's active plan, whose key the copy still shows at `written`, puts its `original` back
  in the plan copy (`Core::plan_copy`, read by `in_force` at once, written to `state.plan` at the end of the pass by
  `keep_plan_copy`). A key the copy shows at another value was read since, and the read wins; a later read replaces
  the copy. Only the lid side reads plan values (the settings side reads `.active` only; checked), so nothing else
  changes. Haik's case now counts down: "Restore my power settings" while closed and signed in on the external
  screen, then the work ends before the next read.
- **Minor 1, a hibernate that did not happen.** Picked: after `Hibernate` not allowed, failed or with the lid unknown,
  while the lid stays closed, `Lock` with `Play(LidClosedHolding)` at an input reading 5 minutes idle (clamped at
  resume), at most every 5 minutes, until the lid opens; only while the session is unlocked and on the PC's screen, no
  watched lock waits, the user's own lid action for the power source is not Do nothing, and the source lasted 30 s.
  A lock that works shows as `Session(Locked)` (it also ends a lock step back, so the hold applies again on a locked
  laptop). Not watched: a failure logs, no notice, the next one comes anyway. It covers the review's case (two failed
  locks, the step back, then no hibernate) and the plain work end where the user signed in on an external screen.
  Not the sleep fallback: see Decisions.
- **Minor 2, the step back's notice on a dark screen.** `LockFailed` at the lock step back, and `HibernateFailed`
  raised with the lid closed and the session unlocked, are kept (`LockWatch::kept`) and go out again when the user is
  back at the open laptop (lid opens while unlocked, or the first unlock with the lid open), right before the
  closed-lid report, also when no report is due. A notice raised while locked waits through the held list as before,
  never twice.
- **Minor 3, a countdown rule before the first plan read.** `work_ended` with no plan read and something possibly
  owned (`may_own`: a crashed tray's `Started` journal) keeps the power sources it ended on and decides at the first
  `Plan(Ok)`, on the read's values; the floor rule the same (`floor_countdown_wanted`, not named by the review, same
  hole). A lid open or a new hold ends a waiting want as it ends a countdown. Both callers the review names (the lock
  step back and the no-hold lid close) and the hold end go through it.
- **Minor 4, tests across a power source change.** Two scenarios (below). The props half is round B's file (Needs
  another card 1).
- **Minor 5, docs.** `docs/architecture.md` section 3: the state paragraph names the lid side's memory beside the
  state (the lock watch and the rest); the in-force paragraph gets the plan copy after a restore and the wait for the
  first read; the lid-close bullet gets the lock watch, the 5 s check, the retry and when the step back ends; the
  countdown line says a new hold does not cancel it while stepped back after a failed lock, and AC back does not end
  that one; the failed hibernate's lock; the closed-lid report gets the kept notices. `docs/product.md`, the two lid
  lines: the 5 s check and "until the session locks or the lid opens", the notice shown again, and the lock after a
  hibernate that did not happen (marked as the worker's call for Haik's review).
- `text.rs` not changed: `LockFailed` still says what happens ("tries once more; if that fails, it stops keeping the
  PC awake and hibernates it after 5 minutes with no input") and `HibernateFailed` that settings are back; both fit
  when shown again at the return.

#### Decisions (worker's, for Haik's review), each with when to change it

1. Minor 1: the lock with the chime after a hibernate that did not happen, every 5 minutes while idle. Over the sleep
   fallback, which needs the hibernate thread and a product change (sleep instead of hibernate), and helps little on
   Modern Standby, where `SetSuspendState(FALSE)` hibernates too (research, tested here); what it does there with
   hibernation off: not verified. Silent retries are ruled out by decision 0 (every `Lock` with a notice or a sound),
   hence the chime, and the 5 minutes keep it from being noise. Known limit: a user who sets Do nothing by hand right
   after the restore may get one lock before the next read (up to 30 s). Change it when Haik wants the sleep fallback
   (Needs another card 4) or finds the chime every 5 minutes too much.
2. Minor 2: `HibernateFailed` is kept too, not only the review's `LockFailed`: the same dark screen. A user at an
   external screen with the lid closed sees each twice. Change it when that repeat is seen to annoy.
3. Minor 3: wait for the first read rather than read the `Started` journal's guess: the guess takes a lid key the
   journal does not name for Sleep, and an apply skips exactly the keys already at Do nothing (the user's own), so a
   docked user would be counted down. The decision uses the sources at the work end, so the pause on the user's own
   Do nothing works as at any work end. Known limit: if every plan read fails, no countdown starts. Change it when the
   journal carries the user's values for every key.
4. Finding 5 built by diff on the lid side, not at `Done(Restored)`: `core/settings.rs` is round B's. Change it when
   round B patches the copy at `Done(Restored)` and `Done(Applied)` (then `plan_copy` and `journal_seen` go).

#### Tests

- New in `scenarios/lid.rs` (12): `a_source_lock_that_fails_twice_steps_back_and_a_redock_onto_the_users_own_do_nothing_pauses_it`,
  `ac_back_after_a_lock_step_back_keeps_the_work_end_countdown` (minor 4);
  `a_work_end_right_after_restore_my_power_settings_reads_the_users_own_lid_action` (and the docked case),
  `a_plan_read_after_the_restore_wins_over_the_values_put_back`, `a_value_a_read_already_shows_stays_when_its_entry_leaves`
  (finding 5); `a_lock_step_back_before_the_first_plan_read_counts_down_once_the_plan_is_read`,
  `a_lid_close_with_nothing_holding_before_the_first_plan_read_counts_down_once_the_plan_is_read` (and the docked
  case, and a lid open that drops the want), `a_floor_step_back_before_the_first_plan_read_counts_down_once_the_plan_is_read`
  (minor 3); `a_hibernate_not_allowed_after_a_lock_step_back_locks_again_with_the_chime_every_5_minutes`,
  `a_failed_hibernate_waits_for_5_minutes_without_input_before_it_locks`,
  `a_failed_hibernate_gives_no_lock_on_the_users_own_do_nothing_or_off_the_console` (minor 1);
  `a_lock_step_back_keeps_its_notices_for_the_users_return` (minor 2, three cases).
- Before the fix: 8 failed, 4 passed by design (Log 01:23 to 01:44). The three cases added from the mutant plan
  (the lid open in minor 3's no-hold test, off the console in minor 1's, a held `HibernateFailed` in minor 2's) each
  kill a mutant that survived on paper.
- No existing test changed.

#### Checks (final runs 2026-11-04 01:44 to 01:51, own build folder, every cargo command `--locked`)

- `cargo test -p wakeshot-core --locked` (01:51): 577 passed, 2 ignored (`coverage_tally`, `p2-ownership-props`'
  open finding), plus 11 and 25 in `tests/`. `scenarios::lid`: 81 passed (69 before this round, 12 new).
- `rustfmt --edition 2024 --check` on `core/lid.rs`, `scenarios/lid.rs`, `policy/lid.rs`, `text.rs`: exit 0.
  `cargo fmt --all --check`: diffs only in other cards' files (Found outside scope 1).
- `cargo clippy -p wakeshot-core --locked --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- `PROPTEST_CASES=10000`, release, `scenarios::props`, `scenarios::lid::lid_events_in_any_order`,
  `scenarios::settings`: every lid property passed in each run; `a_released_key_is_never_ours_in_force_again_in_its_episode`
  failed in three of seven runs (Needs another card 2; older than this round).
- Mutants by hand in a scratch copy (own build folder, each reverted): 27 of 27 killed. The card's four: a lock on any
  Do nothing in force (12 tests, among them the new docked source lock test); no lock at AC to battery (12); the
  hibernate without the resume clamp (3); the going to sleep sound with `Hibernate` (5). This round's 23: no value
  put back, put back over a read value, the seen journal only growing, the put back value not lasting (finding 5);
  no work-end wait, no floor wait, the read deciding on the sources now, the journal guess instead of the read, a
  cancel keeping the wait (minor 3); no relock, no idle gate, relock on the user's own Do nothing, before the source
  lasted, at every reading, without the chime, kept past a lid open, into a locked session, off the console, a notice
  per failed relock (minor 1); `LockFailed` not kept, `HibernateFailed` not kept, kept notices never sent, kept while
  locked (minor 2). Each killed by at least one new test.
- No clock read, no I/O, no new dependency in the core. Only the owned files changed (from my own edit record; git
  not run): `core/lid.rs`, `scenarios/lid.rs`, `docs/architecture.md` section 3, `docs/product.md` two lid lines,
  this card, a new `docs/research.md` section. `policy/lid.rs` and `text.rs` unchanged. Build folder, mutant copy and
  scratch scripts deleted.

#### Needs another card

1. `scenarios/props.rs` (round B), verify minor 4's props half: `no_request_without_a_hold` should also check the
   model's `lock_stepped_back`, and `closed_laptop_is_locked_in_time` should replace its blanket `lock_failed` excuse
   with "requests off and a `Restore` within one tick of the retry's failure". Relocks need no model change: each is a
   `Lock` with the chime, which the model already reads as a lid lock.
2. `scenarios/props.rs` (round B): `released_keys_are_not_ours_in_force` clears `restore_failed` at any
   `Done(Restored { result: Ok(()) })`, also of another plan while the core still retries (`state.retry` stays set
   until a `plan: None` restore works or a scoped one leaves nothing), so it fails now and then at 10,000 cases. Change:
   clear it only at `Done(Restored { plan: None, result: Ok(()), .. })`. Minimal cases: (a) start Balanced on battery
   25 %, io keeps up: user switches to other plan; AC in; next Restore fails at call 0; run-4000.json Alive; user
   switches to docked plan; next Restore fails at call 0; menu Restore; user switches to other plan; end LastHold.
   (b) start Balanced on battery 25 %, Automatic off, io lags: menu Keep awake 30 min; agent 1 Working; IO flush;
   user switches to other plan; next Restore fails at call 0; run 10 s; IO runs one; menu Restore; next Apply fails
   at call 2; user switches to Balanced; IO flush; end LastHold.
3. `state.rs`: move `lid::LockWatch` (now nine fields) into `State`, and the `State::plan` doc: "The last plan read
   that worked, with the values restores put back since (`core::lid`, power safety review finding 5)."
4. Optional, Haik's call first: a sleep fallback when hibernate is not allowed, in the hibernate thread
   (`crates/wakeshot-windows`, `p3-hibernate`) and `docs/product.md`.
5. `docs/architecture.md` section 8 and section 5 (not this round's lines): the "Hibernate not allowed, fails, or lid
   state unknown" row: "notice; settings were already restored; while the lid stays closed, `Lock` with the chime at
   each input reading 5 minutes idle with the session unlocked, at most every 5 minutes, until the lid opens (power
   safety verify minor 1)". The "Lock fails" row, at its end: "`LockFailed` is shown again with the closed-lid report
   (verify minor 2)". Section 5 "Lock and hibernate", the last sentence: add "; with the lid still closed the core
   then locks it whenever it is left idle (section 3, Countdown)".

#### Found outside scope

1. `cargo fmt --all --check` shows diffs in other cards' files, not touched: `crates/wakeshot-windows/examples/agent-watch.rs`,
   `crates/wakeshot-windows/examples/ux_icon.rs`, `crates/wakeshot-windows/src/tray/icon.rs`.
2. The props failure in Needs another card 2 is older than this round: neither trace has a lid command, and only the
   lid side reads the plan values this round changes.

#### Open items

- Decisions 1 to 3 are product-near; the orchestrator may want Haik's word on decision 1 (it adds to his rule).
- Needs another card 1 to 5.
