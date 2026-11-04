# Power safety check: p2-6-floor-grace

Reviewer: `wakeshot-reviewer`, 2026-11-05. Brief: the orchestrator's narrow power safety check of commit e82389f
(under the battery floor, the step back waits until the PC has been on battery for 30 s since the last AC loss;
`policy::floor::decide`, `GRACE_MS`; `settle_floor` gives the grace end as a deadline). Read: the card (decisions,
Log, Result), `docs/product.md` "Battery floor", the lid lock lines and the work end rule,
`docs/tasks/p6-int-haik-checks.md` "Found outside scope" 6 and 7, `policy/floor.rs`, `policy/lid.rs`, `core.rs`
(settle pass, `next_deadline`), `core/lid.rs` (`settle_floor`, `settle_lid`, `settle_source_lock`,
`settle_window_lock`, `check_lock`, `done_locked`, the countdown), `core/settings.rs` (`restore_all`), `state.rs`,
`sim.rs` (timers, no auto answer for `Restore`), `scenarios/lid.rs` (the nine new scenarios), `scenarios/props.rs`
(model, `no_step_back_inside_the_grace`, the lock watch), `wakeshot-windows` `power/hibernate.rs`, `power/system.rs`
(power source decode, `GetSystemPowerStatus`), `app/dispatch.rs` (timer, `Lock`), `app/tray_mode.rs` (fresh
readings after lost posts), `clock.rs`, `docs/architecture.md` "Episode flow" floor and lock lines, `docs/research.md`
"Clock" and "Battery floor grace checks". No build, no test run, no git (brief).

Counts: 0 blockers, 0 majors, 4 minors.

## Findings

### 1. Minor: AC that flaps with battery stretches under 30 s never steps back

- Where: `crates/wakeshot-core/src/policy/floor.rs:70-73` (the grace counts from `on_battery_since_mono`);
  `core/lid.rs:401` (`source_since_mono` set at every source change); card decision 4; `docs/product.md:118-119`
  ("AC that comes and goes starts them again").
- What breaks: a charger or cable that drops in and out, on battery 20 to 29 s at a time with short AC blips between,
  restarts the grace at every loss, so `decide` never answers `SteppedBack`. Holds hold, Wakeshot's battery sleep
  Never and lid Do nothing stay in force, and the battery drains under the floor until Windows' critical battery
  action, which Wakeshot never writes (whether a held request delays it: not verified, `floor.rs:50-51`). Before
  this card such a pattern stepped back at each loss. Haik's night is the other shape (drops of 4 to 10 s, minutes
  of AC between), so his case is not affected. How often real chargers flap this way: not verified. Decision 4 is
  open for Haik's review; this is its cost.
- Fix, lean: an AC return restarts the grace only once AC has lasted `SOURCE_SETTLE_MS`, the same settle the source
  lock uses; keep the start of the first loss of a run otherwise. Haik's night gives the same result. Cost under a
  flapping charger: a step back at 30 s from the first loss, and each later blip ends it and the next loss steps back
  at once, with sound and notice each time. Stronger, quieter: also end a step back only once AC has lasted 30 s.
  Haik's call (product decision).

### 2. Minor: in the closed-lid unplug, a lock the session does not confirm gets no retry and no notice once the restore lands

- Where: `core/lid.rs:860-868` (`check_lock` returns "nothing to do" once `lid_ours()` is false);
  `core/lid.rs:797-800` (the source lock, then the floor `Restore` in the same pass); the test
  `a_docked_unplug_with_the_lid_closed_under_the_floor_locks_and_steps_back_at_30_s` (`scenarios/lid.rs:1900-1917`);
  card decision 2 ("The lock is still watched and retried"); `docs/architecture.md:419` ("The same for the source lock
  below"); `docs/product.md:91-94` ("a closed laptop never runs unlocked in a bag") and `:141-142` ("both are safe").
- What breaks: docked, signed in on the external screen, lid closed, holding, under the floor; the dock is unplugged.
  At 30 s the `Lock` goes out and the floor `Restore` right behind it. The restore's `Done(Restored)` comes back in
  about 1 s (block 4: written 0.99 s after the step back), well inside the 5 s lock check. At the check the journal
  copy is empty, `lid_ours()` is false, and `check_lock` logs "nothing to do": no second `Lock`, no `LockFailed`
  notice, no lock step back. If `LockWorkStation` returned OK but the session never locked, and Windows does not sleep
  the laptop at the restore (the open fact of the work end rule), the closed laptop runs unlocked on battery until the
  Floor countdown sleeps it: 5 minutes without input, then the 1-minute timeouts (about 6 minutes, not measured). The
  docked test asserts the retry only because the `Sim` never answers a `Restore` by itself (`sim.rs:406-422`) and the
  test does not send `restored(...)` before `LOCK_CHECK`. A `LockWorkStation` error is still retried: `done_locked`
  (`core/lid.rs:1279-1293`) shows the notice at once and makes the check due at the next tick, before the restore
  lands. Not caused by this card in kind: before it, the restore came at the unplug and the source lock was dropped,
  so this path had no lock at all; the same "restore within 5 s ends the watch" pattern exists for a lid-close lock
  whose hold ends within 5 s. Likelihood low.
- Fix: in `check_lock`, keep a lock that went out with a floor step back as still guarding while the lid stays closed
  and the step back lasts (for example a flag on `PendingLock` set in `settle_source_lock` when `chime` is false), so
  the retry and the notice come. Make the docked scenario answer the `Restore` before `LOCK_CHECK`, so it pins the
  real timing. If Haik prefers no code: correct decision 2, `architecture.md:419` and product "both are safe" to say
  the retry does not come once the restore landed.

### 3. Minor: the props judge only the lower bound of the grace

- Where: `scenarios/props.rs:1023-1061` (`no_step_back_inside_the_grace`: no floor chime, notice or restore before
  30 s); `props.rs:446-456` and `:747` (the model's step back flag comes from the same `floor::decide`, so the request
  check only asks for agreement at each step, not that a step comes at the grace end); `docs/research.md:4474-4476`
  (mutant M2, no grace deadline, survived until a scenario was moved off the plan reads).
- What breaks: the safety half of the rule (no hold on battery under the floor past 30 s after the loss) is pinned by
  scenarios only. A later change that drops the grace deadline on some path (a new early return in `settle`, a branch
  that overwrites `state.next`) would pass the props: the 10 s plan reads and the 30 s safety tick still bring the step
  back, up to 30 s late.
- Fix: one property: while the model is on battery, under the floor and holding, with `source_since` set, the step at
  `source_since + GRACE_MS` (or the step of the percent reading, when later) must exist and have the system request
  off. The sim already fires a `Tick` at every core deadline, so the property fails when that tick is missing.

### 4. Minor: product "Battery floor" still says Windows may sleep the closed laptop "right after an AC loss"

- Where: `docs/product.md:125-126` ("right after an AC loss Windows may sleep it at once instead (work end, below)").
- What breaks: the restore now comes 30 s after the loss, and the work end rule (`:138-142`) says it is not known
  whether Windows still acts then. The bullet reads as if the sleep comes at the unplug. Doc precision only.
- Fix: "at the step back, 30 seconds after the AC loss, Windows may sleep it at once instead (not known; work end,
  below)".

## Answers to the brief

1. **Can the grace hold the PC on battery under the floor past 30 s after an AC loss?** Only in the flapping case
   (finding 1) and, by up to 30 s more, after lost posts. Per case:
   - Missed deadline: none found. `settle_floor` returns the grace end (`core/lid.rs:767`), `settle` folds it into
     `state.next` (`core.rs:294`, `:333`), and the loop arms its one timer from it (`app/dispatch.rs:212-227`, 10 ms
     at least). A timer that fires a little early gets `Grace` again and a 10 ms re-arm. The 30 s safety tick bounds
     any miss.
   - Sleep inside the 30 s: run time is `QueryUnbiasedInterruptTime` (`clock.rs:39`), which skips suspended time
     (research "Clock"). No request is honored while suspended; after the resume `Retake` and the rest of the 30 s.
     Awake time held stays at most 30 s; wall time is longer, by design (decision 5).
   - AC loss seen at a resume: 30 s from the resume. By design.
   - Clock edge: `now < due` is the grace, `due` itself steps back (`floor.rs:71`); the add saturates; scenarios pin
     the step back to the millisecond.
   - Lost power posts: a failed post is replayed only at the next safety tick (`app/tray_mode.rs:778-803`), so the
     core hears the loss up to 30 s late and the grace counts from then: up to about 60 s. Needs a full message queue;
     `architecture.md:447` already says "the AC loss the core heard". Not a finding.
2. **Can a step back, restore or apply be lost or doubled when AC comes back inside the grace?** No. Inside the grace
   `stepped_back` never flips, so the return sends nothing but a log line (`core/lid.rs:739-753`). A hold that starts
   inside the grace sends its apply as usual; a step back after it queues its `Restore` behind that `Apply` on the IO
   thread, which runs in order. After a step back, AC back runs `want_apply` once and clears `step_back_shown`
   (`:761-764`), so each step back shows once.
3. **Lid closed: lock, restore and sleep route in a safe order?** Yes, apart from the retry (finding 2). In the pass
   at 30 s: `Requests` off (`core.rs:302-306`), then `Lock` (`core/lid.rs:797`, a synchronous `LockWorkStation` in
   `dispatch.rs:354-357`), then `Restore` queued for the IO thread (`:800`), notice and `Play(BatteryFloor)`, then the
   Floor countdown want. Requests off first is safe: our Do nothing and battery sleep Never are in force until the
   restore lands. The lock call returns before the restore is even queued. `LockWorkStation` only starts the lock
   (about 1 s in the lid spike); if Windows sleeps at once after the restore, whether the lock lands before the
   suspend: not verified (Windows' own sign-in after sleep normally locks too). The sleep route is unchanged: a Floor
   countdown starts only at a step back.
4. **Does the Windows re-check agree?** Yes. `check_floor` (`power/hibernate.rs:292-329`) asks on battery and
   `floor::below` only. A Floor countdown exists only after a step back, so after at least 30 s on battery; AC back
   cancels it (`core/lid.rs:413-415`); AC back and lost again during the going to sleep sound leaves no countdown, so
   `Done(SleepCheck(Ok))` writes nothing (`:593-599`). `app/dispatch.rs` has no floor rule of its own.
5. **Tray start, unknown percent, unknown AC.** Nothing unsafe.
   - Tray start on battery under the floor: step back at once, the safe side (decision 1).
   - Before the first `Power` event `settle_floor` returns early (`core/lid.rs:715`), as before; the subscriptions post
     the source at once.
   - Unknown percent: never starts a step back (`floor.rs:67-69`), unchanged; inside the grace it ends the wait; a
     known percent under the floor after the 30 s steps back at once.
   - Unknown AC: a source value other than 0, 1 or 2 posts nothing (`power/system.rs:489-495`). `ACLineStatus` 255
     counts as AC (`:138-143`); it reaches the core only through the fresh readings after lost posts. There it ends a
     grace or a step back, and the next battery reading starts a new 30 s. Rare; the safe side for the hibernate check.

## Checked, no finding

- `settle_source_lock` moved ahead of the step back: `restore_all` changes no state (`core/settings.rs:142-146`), so
  only the command order changed, not any decision in the pass.
- The source lock and the grace use the same `now.mono_ms` (`core/lid.rs:401-406`, `policy/lid.rs:112`), so they
  always fall due in the same pass.
- Outside this brief (sound only): a lid close in the unknown journal window plus an unplug under the floor makes
  `settle_window_lock` (`core/lid.rs:197-234`) lock with the lid chime in the same pass as the step back's
  `Play(BatteryFloor)`, so two sounds overlap there. The lock still goes before the restore.
- If Windows sleeps at once after the restore, the floor sound may be cut, as block 4's was; the `Lock` then has only
  the held notice. Not a safety issue.
