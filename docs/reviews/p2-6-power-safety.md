# Power safety review, Phases 2 to 6

Reviewer: `wakeshot-reviewer`, 2026-11-03, started 22:33, written 22:44. Brief (`docs/phases/02-06-lighter.md`,
"Reviews"): find any path where the user's power settings are not restored, a user's own change is overwritten, or a
closed laptop keeps running unlocked or below the battery floor.

Read: the code and Result of `p2-ownership-restore`, `p2-ownership-apply`, `p2-ownership-props`, `p2-core-settings`,
`p2-core-lid`, `p3-journal`, `p3-power-plan`, `p3-power-io`, `p3-recovery`, `p3-stop-points`, `p3-hibernate`,
`p3-session`; `docs/architecture.md` sections 3, 5, 8; `docs/product.md`. No build, no test run, no power setting
touched. Line numbers are as on disk at 22:40 (`ownership/props.rs` had uncommitted edits; nothing here depends on
them).

Counts: 0 blockers, 1 major, 9 minors. The seven known items are ruled on after the findings.

## Findings

### 1. Major: a failed lid-close lock leaves a closed laptop running unlocked, with no fallback

- Where: `crates/wakeshot-core/src/core/lid.rs:529-536` (`done_locked`); the lock goes out at `lid.rs:94-98` (lid
  close) and `lid.rs:308-315` (source lock). Design row: `docs/architecture.md` section 8, "Lock fails: log, notice".
- What breaks: the lid closes while a hold runs on our Do nothing. `Lock` fails (`LockWorkStation` returns zero; per
  `docs/research.md` line 1735 it works only from a process on the interactive desktop, so for example not while a
  UAC prompt holds the secure desktop). The core logs and raises `LockFailed`. The laptop goes into a bag running and
  unlocked for as long as the hold runs (hours for a long build on AC), and the notice is shown to nobody. One
  failure gives exactly what decision 18 forbids. The same holds for an `Ok` lock that never locks: the call is
  asynchronous (research line 1735), and nothing checks that `Session(Locked)` follows.
- How likely: not verified. No test shows when `LockWorkStation` fails on this laptop.
- Fix: when `Locked(Err)` comes with the lid closed and our Do nothing in force (`lid_ours()`), send `Lock` once more
  at the next tick; if that fails too, step back as at the floor: `Requests` off, `restore_all`, and start the
  countdown, so the laptop hibernates after 5 minutes without input. Optional: treat "no `Session(Locked)` within
  5 s of an `Ok` lock" the same way. Needs a new line in section 8.

### 2. Minor: an apply whose write fails and whose own restore cannot load the journal leaves our value unknown to the core

- Where: `crates/wakeshot-core/src/ownership/apply.rs:366-379` (`undo` reports `restored.journal`);
  `ownership/restore.rs:54` (a failed load reports `journal: []`); `core/settings.rs:318-322` (an empty failed
  `journal` keeps the last copy), `settings.rs:368` (no restore when the copy is empty), `settings.rs:56-60`
  (`may_own`); `core.rs:251-256` (`written` is ignored). This extends `p2-ownership-props` finding 2.
- Scenario: first hold of the run, copy empty. Apply step 3 saves three entries; step 5 writes lid AC 0, then the lid
  DC write fails (or its read-back differs); `undo` runs the restore, whose `load_journal` fails. The answer is
  `Applied { written: [LidAc], result: Err(Write), journal: [] }`. The core keeps its empty copy and sends no
  restore; `applying` is cleared, so at the hold end `may_own()` is false and no restore goes out there either. Lid AC
  0 stays stored on the plan and comes into force at the next `PowerSetActiveScheme` by anyone (Settings, another
  app, a reboot). While it is in force, a lid close gets no lock (`in_force` has no entry). It heals at the next
  hold's apply (takeover), at Quit or session end (both restore always), or at the next start; RunOnce and the guard
  are set. A double fault, so rare.
- Fix (core, one line): on `Err(Write(_))`, set `retry` (or send one `Restore { plan: None }`) whatever `journal`
  holds; a restore of a missing journal costs nothing. Or, in `undo`, report the step 3 entries when the routine's
  load failed (props finding 2's proposal).

### 3. Minor: a startup journal read that fails is never retried, and the core sees nothing owned (items 2b and 3)

- Where: `crates/wakeshot-windows/src/power/executor.rs:152-159` (`Failed` gives `journal: None`);
  `crates/wakeshot-core/src/core.rs:99-103` (`retry` is set only for a non-empty journal).
- Scenario: a tray starts with a journal left by a dead tray, and that one read fails (delete pending, antivirus,
  I/O error). `Started.journal` is `None`, so no `Leftover` retry and an empty copy: the override stays, and a lid
  close gets no lock and no countdown, until a hold's apply takes the entries over, or Quit or session end restores
  (their restore ignores the owner). No guard covers it meanwhile (`keep_guard` guards only a journal this tray
  owns). At sign-in the likely cause, restore mode writing the same file, also means restore mode is restoring it.
- Fix: the planned "retry the read once" (`p6-int-startup` Log) narrows the gap but does not close it. Better: a third
  state in `Started` (or a flag `journal_unread`) that makes the core set `retry = Some(Leftover)` with an empty copy.
  The executor's `Leftover` path already reads the file, checks the owner and restores, on each tick.

### 4. Minor: work that ends on a source whose own lid action is Do nothing never gets a countdown after an unplug

- Where: `crates/wakeshot-core/src/core/lid.rs:340-346` (`work_end` is kept only when the countdown would start on
  the current source); `lid.rs:389-405` (the pause and restart work only on a kept `work_end`). No scenario covers it:
  `scenarios/lid.rs:755` ends on AC and never unplugs; `:804` ends on battery.
- Scenario: docked, own lid action Do nothing on AC and Sleep on battery, lid closed, hold on AC. Work ends: own AC
  action is Do nothing, so no countdown, which is right. Then the user unplugs and bags the laptop: nothing is ours
  any more, so no source lock, and `work_end` was never kept, so no countdown either. The same work ended on battery,
  then docked and undocked, does get the countdown after 30 s (test at `:804`). The brief's rule ("a work-end
  countdown at a power source change follows the user's own lid action for the new source") holds only for the
  second order. Wakeshot changed nothing that keeps this laptop running (the user's own AC Do nothing did), so this
  is a gap in the promise, not an override left.
- Fix: in `work_ended`, keep `WorkEnd` whenever the lid is closed and `in_force` is known, with `paused: true` when no
  current source starts it; `settle_countdown` then starts it once a source whose own action is not Do nothing lasted
  30 s. Add the scenario.

### 5. Minor: a stale plan copy after a restore makes a work end read the user's own lid action as Do nothing

- Where: `crates/wakeshot-core/src/core/lid.rs:327-347` (`work_ended` reads `in_force`), `lid.rs:43-69`; a known limit
  in `p2-core-lid` design choice 16.
- Scenario: Haik's values (Sleep on AC and battery). Hold running, lid closed (locked), Haik signs in on the external
  screen and picks "Restore my power settings": the copy empties, but the plan copy shows 0 for up to about 40 s. Work
  ends inside that window: both own lid actions read 0, so no countdown, and none starts after an unplug either
  (finding 4). His Sleep is back, but the lid was already closed, so the laptop runs until his own sleep timeout (3 h
  on AC, 4 h on battery now, `p3-stop-points` Log 20:43), unlocked if he unplugs and bags it at once.
- Fix: at `Done(Restored)` with `Ok`, set the plan copy's key to the `original` of each entry that left the copy (the
  old copy has them); or send `ReadPlan` right after each restore outcome.

### 6. Minor: the hibernate thread cannot be stopped once started (item 4)

- Where: `crates/wakeshot-windows/src/power/hibernate.rs:93-101` (last checks), `:115-125` (sound, up to about 3 s per
  `p3-hibernate` Found outside scope 3), `:128-138` (`SetSuspendState`, no check after the sound);
  `core/lid.rs:271-273` cancels only the core's countdown.
- Scenario: the user hears the going to sleep sound and opens the lid, or an agent turn starts in those seconds; the
  laptop hibernates anyway. This errs toward sleep, so it never leaves a laptop running; the cost is a surprise
  hibernate during work.
- Fix: an `Arc<AtomicBool>` cancel flag the loop sets on lid open and on `Cancel::HoldingStarts`, read by the thread
  right before `SetSuspendState`; or a second short lid check after the sound. A contract change, so low priority.

### 7. Minor: the battery floor countdown ignores the user's own battery lid action, and only when the lid was already closed (item 7)

- Where: `crates/wakeshot-core/src/core/lid.rs:266-268` (step back with the lid closed: `floor_countdown = true`, no
  check) against `lid.rs:94-106` (a lid close after the step back starts it only on our Do nothing).
- Scenario: own lid DC Do nothing, on battery, lid closed, user typing on an external screen. Under the floor:
  hibernate after 5 minutes idle. Close the lid a minute after the step back instead: no countdown. Neither case is
  unsafe: the settings are restored, so the user's own battery sleep timer applies, and the built rule errs toward
  sleep.
- Ruling: safe either way. If Haik keeps the default "no hibernate then", build it as the work-end rule does (pause
  while the own action for the current source is Do nothing), so both entry paths agree.

### 8. Minor: any code 5 in an apply's safety net turns off overrides for the whole run (item 6)

- Where: `crates/wakeshot-core/src/core/settings.rs:35` and `:346-348` (5 or 1260 from any call means "access denied,
  requests only for this run"); `crates/wakeshot-windows/src/power/journal.rs:184-187` (a mutex open error is passed
  on as is); `store.rs:412-418` (`CreateMutexExW` with the default security).
- Scenario: an elevated `wakeshot restore` holds `Local\Wakeshot.Journal` while the Medium tray applies (`p3-journal`
  item 3). If the elevated mutex refuses the Medium open (not tested), the open fails with 5, the apply answers
  `SafetyNet(CreateMutexExW, 5)`, and the core sets `requests_only_run`. A transient `ERROR_ACCESS_DENIED` from the
  journal write or the RunOnce write does the same. The safe side (no override), but the lid then sleeps the laptop
  during agent work for the rest of the run, after one notice. A restore in the same spot fails once and is retried,
  which is fine.
- Fix: count 5 and 1260 as "for the run" only when `failure.call` is `PowerSettingAccessCheckEx` or a `PowerWrite*`
  call; any other failure is "for the episode". `p3-real-checks` should still test the elevated mutex.

### 9. Minor: a failed apply whose restore also fails reports a false user change (item 5, props finding 1)

- Where: `crates/wakeshot-core/src/ownership/apply.rs:150-176` with `ownership/check.rs:65-82`; ignored test
  `ownership::props::a_failed_apply_whose_restore_fails_reports_no_false_user_change`.
- Confirmed as rated: an entry saved in step 3 and never written reads "not marked, live equals `original`", so the
  next check reports it as the user's. The cost is a wrong "you changed a setting" notice and a release for an episode
  that is already requests only. No value is lost and no override is left. The proposal (save new entries marked in
  step 3, clear the marks after `set_active`) is sound. Props finding 2 is folded into finding 2 above, which shows
  its real cost.

### 10. Minor: an apply overrides an entry the user changed in the last 10 s (known, `p2-ownership-apply` Found outside scope 5)

- Where: `crates/wakeshot-core/src/ownership/apply.rs:298-308`.
- Scenario: a hold runs; the user sets lid AC to Hibernate in Settings; within 10 s, before the ownership check, an
  edge applies again to the same plan (`ConsoleBack` or `EndCancelled` after a restore that failed, or the
  `PlanChanged` re-read). The apply sees "entry no longer ours", makes a new entry with the user's value as
  `original` and writes 0 again, with no `UserChanged`. The user's value comes back at the hold end, but decision 26
  ("the latest change wins at once") is broken for the episode. Narrow window.
- Fix: when step 2 finds an entry on this plan whose live value is neither `written` nor `original`, skip the key and
  report it as a user change (`Applied.user_changed`, the card's proposal).

## The known items

1. **`Leftover` with an Unknown owner answers `Ok`: refuted.** `executor.rs:317-328` answers `Err(OWNER_UNKNOWN)`
   (alive: `Err(OWNER_ALIVE)`), with the file's known entries; a journal taken over before the lock maps to
   `Err(OWNER_ALIVE)` (`executor.rs:281-285`). Only a missing file answers `Ok` with `[]`, which is right. The core
   keeps retrying on each tick (`settings.rs:397-402`). Tested by `leftover_runs_only_for_gone_or_own_owner`.
2. **Orchestrator decisions.** (a) One `Restore { plan: None }` after `SafetyNet(JOURNAL_CORRUPT)`: built and right
   (`settings.rs:364-373`, test `a_corrupt_journal_at_apply_sends_one_restore_per_file`). Once per file, reset by
   `JournalCorrupt` or a full restore that works; while not holding, the hold end's restore is already queued behind
   the apply. A failed set-aside keeps `retry`, so it is tried again. The `architecture.md` line is in (section 5,
   restore step 1). A note, not a finding: after the set-aside the core's copy empties though it held the right
   originals; a file that turns corrupt while the tray runs is rare (atomic writes), so the documented gap stands.
   (b) The startup read retried once: not built; see finding 3 for why a tick retry is the better fix.
3. **`Started.journal` cannot say "did not read": confirmed, minor.** Finding 3.
4. **The hibernate thread cannot be cancelled: confirmed, minor.** Finding 6. It errs toward sleep.
5. **`p2-ownership-props` Found outside scope 1 and 2: confirmed, both minor.** Finding 9 (item 1) and finding 2 (item
   2, with the core side that turns it into a missed restore).
6. **Medium tray and an elevated journal lock: not verified, minor.** Finding 8. The open asks only `SYNCHRONIZE |
   MUTEX_MODIFY_STATE` (`store.rs:415`), which helps; whether the elevated default DACL grants the Medium token is not
   known. Worse than the denied try itself is the core reading its code 5 as a policy block for the whole run.
7. **Floor countdown with the user's own battery Do nothing: confirmed, minor.** Finding 7.

## Checked and found sound

- Apply order: RunOnce, guard, journal before any write (`apply.rs:132-154`); a failure there writes nothing and
  takes back only what this apply added. The takeover keeps `original` and the mark only while the entry is ours.
- Restore: marks saved before any `original` is written; "plan deleted" only from a complete list (`plan.rs:163-181`,
  only 259 ends it); "inactive" judged by the second active plan read (`restore.rs:180-201`); drops saved even in a
  failed run; the clean-up deletes the file before RunOnce and the guard.
- Check: a read error keeps the entry; marked entries are skipped.
- Core: a failed outcome with an empty `journal` keeps the copy; the retry never takes the active plan's entries while
  the core applies; Quit and session end always restore; `ConsoleLeft` restores; no apply while stepped back, ending
  or off the console; the in-force view errs toward a lock.
- Lid and source: lock and chime at a lid close on our Do nothing, also with nothing holding (then the countdown); the
  AC to battery lock after 30 s (`SOURCE_SETTLE_MS = 30_000`); a running work-end countdown follows the user's own
  action for the current source and restarts only after 30 s on it.
- Guard and restore mode: owner checked before and under the lock; Unknown never restores; the give-up starts the
  tray.
- End of session: `session_ending` is called only at `WM_ENDSESSION` TRUE (`dispatch.rs:258-262`), so the
  `p3-power-io` worry (Found outside scope 5) about a dropped `Apply` left in `applying` after `EndCancelled` cannot
  happen with today's dispatcher.
