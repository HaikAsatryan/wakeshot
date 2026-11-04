# Review: lost journal reset and the sleep route

Reviewer: `wakeshot-reviewer`, 2026-11-04. Brief: a narrow fresh look at `docs/tasks/p2-6-record-lost.md` and
`docs/tasks/p2-6-sleep-route.md` against `docs/product.md` ("Never lower security", battery floor, work end, "User
settings safety" 1 to 7) and `docs/research.md` ("Idle sleep with the lid closed", "Windows defaults of the three
keys").

Read: both cards' Results; `crates/wakeshot-core/src/core/lid.rs`, `core/settings.rs`, `core.rs`, `settings.rs`,
`state.rs` (fields), `policy/ending.rs`, `policy/lock.rs`, `policy/lid.rs` (rules used), `ownership/apply.rs`
(`meets`, `own`, `release`), `ownership/reset.rs`; `crates/wakeshot-windows/src/power/plan.rs` (keys, GUIDs,
`read_default_value`, `DryRunPower`), `power/hibernate.rs` (`run_hibernate`, `run_sleep_check`), `power/executor.rs`
(startup, reset, `mark_seen`), `power/journal.rs` (`reset_within`, `mark_corrupt_seen`), `power/system.rs`
(suspend decode), `app/dispatch.rs` (hibernate and sleep check runners), `hooks.rs` (uninstall, `try_restore`); the
scenarios named below and the lock property in `scenarios/props.rs`. No test run, no git, no setting touched.
Findings come from reading the code paths; each says where it is inferred.

Counts: 0 blocker, 3 major, 5 minor, plus 1 unverified note.

## Major

### 1. A closed lid in the unknown window, then a hold that starts or resumes after it: the closed laptop runs unlocked

- Where: `crates/wakeshot-core/src/core/lid.rs:175-190` (`unknown_journal_ended`), `:196-204`
  (`settle_window_lock`), `:275-281` (`take_lid`); `crates/wakeshot-core/src/core.rs:336-347` (`holding_starts`).
- Case (main): the tray starts with a journal that does not read, a crashed tray's battery Do nothing in force, the
  user's own action Sleep. The user closes the lid in the 2 minutes: no lock, by design. The window ends with no hold,
  so the hook only sets the work-end want (`:180-184`) and drops `window_close` (`:176`). The countdown starts after
  the reset and the read. Before it is due, an agent turn or a scripted hold starts: `Cancel::HoldingStarts` ends the
  countdown, the hold's apply writes Do nothing and battery sleep Never, and nothing ever locks: `take_lid` needs a lid
  edge, `holding_starts` has no lock, and the window flag is gone. The closed laptop runs on battery on Wakeshot's
  values, unlocked, until the hold ends.
- Case (second path): the window ends while a battery floor step back holds the episode. The hook takes the floor
  branch (`:185-186`) and drops `window_close` again; `settle_window_lock` would also clear itself on `holds_back()`
  (`:200-203`). When AC comes back the hold holds again on requests, with the lid closed and no lock.
- Why it breaks: "Never lower security": without Wakeshot the user's own Sleep would have slept this laptop at the
  close; it runs only because of Wakeshot's leftover and then Wakeshot's hold. Rule 7 accepts "up to about 7 minutes"
  unlocked with nothing holding; a hold makes it unbounded. Deliverable 8 of the sleep-route card covers only a hold
  that holds at the window's end.
- Tests: none. `scenarios/lid.rs:954` covers only the hold that holds at the end. The property in
  `scenarios/props.rs:1011-1020` starts a stretch only at a lid close or an unplug outside the window, so a hold start
  after the window is never judged.
- Inferred, not seen: whether Windows locks a closed, running laptop at the user's own screen-off timeout. If it does,
  the unlocked time is bounded by that timeout; nothing in `docs/research.md` measures it.
- Leanest fix: keep `window_close` until the lid opens or `Session(Locked)`, not until the window ends. In
  `settle_window_lock`, wait (do not clear) while there is no episode or the hold is held back; lock once a hold holds
  and a read shows the user's own action as not Do nothing. Add one scenario for each path.

### 2. A lost journal while the 1-minute timeouts are in force leaves them for good, and can make 60 s an "original"

- Where: `crates/wakeshot-core/src/ownership/reset.rs:20-23` (only `LidAc`, `LidDc`, `SleepDc` at 0);
  `crates/wakeshot-core/src/core/settings.rs:346-385` (`done_reset` empties the copy and never looks at
  `state.short`); `crates/wakeshot-core/src/core/lid.rs:468-476` (`end_short` restores only when `may_own()`);
  `crates/wakeshot-core/src/ownership/apply.rs:322-333` (a new entry takes the live value as `original`).
- Case A (next start): the short timeouts are written, the PC loses power or the OS crashes, and `journal.json` does
  not read or is damaged at the next start. The reset finds sleep and screen off at 60, not 0, so it keeps all four.
  Screen off and sleep after 1 minute, AC and battery, stay until the user notices and sets them.
- Case B (in a run, inferred from the code paths): with the short timeouts in force, a hold starts. `end_short`'s
  `Restore` meets a damaged file, sets it aside and posts `JournalCorrupt`; the hold's `Apply` is already queued behind
  it, finds no file and journals battery sleep with `original` 60. The `ResetLost { damaged: true }` then finds the
  hold's fresh journal and resets nothing. At the hold's end battery sleep goes back to 60, and the other three short
  keys were never in any journal again.
- Why it breaks: "User settings safety" 2 and 5 and Haik's rule for no work going on: Wakeshot's own values stay in
  force with no record. Rule 7 names only Do nothing and Never, so this case has no product rule. The worker flagged
  it (sleep-route Found outside scope 4, decision 17; architecture section 5 "Gap left"); Haik has not decided it.
  Rare: needs a damaged or unreadable journal in the minutes the timeouts are written.
- Tests: none show what happens; a scenario would pin the chosen behavior.
- Leanest fix (Haik's call, rule 7 is his): add the four short keys at exactly `SHORT_TIMEOUT_S` to the reset's list
  and to the notice words, with the same accepted cost (a user's own 1 minute goes to the default). Case B also needs
  `take_journal_corrupt` to treat a set-aside file while `state.short` is set as lost for the short keys.

### 3. Uninstall with a damaged journal leaves Wakeshot's values with nothing that will ever reset them

- Where: `crates/wakeshot-windows/src/hooks.rs:576-580` (`try_restore` returns `DoesNotParse` before any routine),
  `:200-203` (keeps `journal.json`, resets nothing).
- Case: a damaged `journal.json` at uninstall, while a hold's Do nothing and Never, or the 1-minute timeouts, are in
  force. The hook keeps the file and exits. No tray starts after an uninstall, so the "next start resets" path of rule
  7 never comes. Lid Do nothing stays: a closed laptop then runs in a bag with no Wakeshot to lock or sleep it.
- Why it breaks: product "User settings safety" 5 (restore on uninstall) and 7 (reset a lost journal). Architecture
  section 5 "Gap left" lists it (`docs/architecture.md:904-906`), but Haik's acceptance of the crash gap
  (`docs/product.md:170`) is dated 2026-11-03 and predates this case. Rare: a damaged file at the moment of uninstall.
- Leanest fix: on `DoesNotParse` in `on_uninstall`, run `power::journal::reset_locked` with `damaged: false` (it loads
  the file, finds it corrupt, resets the three keys at Wakeshot's value to the plan's defaults and sets the file aside)
  and log each key, as the tray does; no notice is possible from a hook. One hook test with a corrupt file.

## Minor

### 4. Short keys count against the three hold keys in `episode.released`

- Where: `crates/wakeshot-core/src/core/settings.rs:66` (`applies`: `released.len() < KEYS.len()`), `:289-293` (any
  key, short ones too, goes into `released`), `:325` (`take_back`: `released.len() == KEYS.len()`); the props model
  copies the same count at `scenarios/props.rs:178`, so it cannot catch it.
- Case: during a floor or failed-lock step back with the short timeouts in force, three `UserChanged` for short keys
  (say `SleepAc`, `VideoIdleAc`, `VideoIdleDc`). `released` has 3 entries and no hold key. When AC comes back or the
  session locks, `applies()` is false, so the hold never writes the lid Do nothing again in that episode, and
  `take_back` logs "Every key is the user's" and may restore. I found no likely source of such changes without input
  (input restores the short timeouts first), so the trigger is rare.
- Why it breaks: rule 3 releases only the keys the user changed; here hold keys the user never touched stop being
  applied.
- Fix: count only hold keys, for example `e.released.iter().filter(|k| KEYS.contains(k)).count()`, in both places and
  in the props model.

### 5. A reset that fails is never retried; with the lid closed on a leftover Do nothing nothing ever locks or sleeps it

- Where: `crates/wakeshot-core/src/core/settings.rs:354-376` (known and empty whatever the result);
  `crates/wakeshot-core/src/core/lid.rs:905-909` (the want waits for a read that never shows anything but Do
  nothing); `crates/wakeshot-windows/src/power/journal.rs:174-187` (a journal mutex not taken in 10 s is a failed
  reset too).
- Case: the lid closed in the window, the reset fails on any call (a busy mutex, `PowerReadACValueIndex` of the plan's
  kind on an unknown index, a write). The journal counts as known and empty, so `lid::ours` is false for good: no lock,
  no countdown, until the lid opens. The test `scenarios/lid.rs:927` asserts this on purpose ("as without Wakeshot"),
  but the Do nothing is most likely Wakeshot's, so it is not "as without Wakeshot".
- Why it matters: brief item 1, "no path to sleep or lock". Decision 4 accepts it for policy failures; a busy mutex or
  a plan-kind read is not that.
- Fix: decision 4's own trigger: while the lid is closed, send the reset again on the 2-minute clock after a failed
  one; or, after a failed reset with a Do nothing in force and the lid closed, lock once.

### 6. A damaged-file report that reaches the tray late resets values the user set since

- Where: `crates/wakeshot-windows/src/power/executor.rs:378-390` (a failed `.corrupt` to `.seen` rename counts as a
  new report), `:198` (startup reports any `journal.json.corrupt` left by others);
  `crates/wakeshot-core/src/ownership/reset.rs:110-111` (`damaged` with no file goes on to reset).
- Case: the rename keeps failing (an ACL or a locked `.seen`): every tray start posts `JournalCorrupt`, and each
  `ResetLost { damaged: true }` with no journal resets any lid Do nothing or battery Never in force, which by then may
  be the user's own docked setting, with a notice each time. Same for a file `wakeshot restore` set aside while a tray
  ran: it is reported only at that tray's next start, maybe days later. Before `p2-6-record-lost` both gave only a
  repeated notice.
- Why it matters: Haik's rule for no work going on. Rare.
- Fix: when the rename fails, log and do not report again (a missed notice is cheaper than a second reset); or skip the
  reset when the `.corrupt` file is older than the tray's last clean quit.

### 7. The window lock skips the 30 s source settle and gets no retry or step back

- Where: `crates/wakeshot-core/src/core/lid.rs:205-217` (locks on any settle pass once one source's own action is not
  Do nothing), `:747` (`check_lock` gives up when `!lid_ours()`).
- Case A: a docked user (own AC Do nothing, own battery Sleep) whose hold ran through the window with the lid closed.
  The dock drops AC for 5 s: `sources()` is battery, and the lock comes at once. Product line 89-91 asks for 30 s on
  battery first (Haik was locked by exactly this drop).
- Case B: a window lock that `LockWorkStation` starts but the session never reports. The hold is requests only (its
  apply failed in the window), so `lid_ours()` is false and `check_lock` logs "nothing to do": no retry, no step back,
  although the hold's requests keep the closed laptop running.
- Fix: in `settle_window_lock`, require `source_since_mono` plus `SOURCE_SETTLE_MS` as `relock` does (`:794-796`); in
  `check_lock`, treat a pending window lock as ours while `episode` is held.

### 8. `docs/product.md` work-end line no longer matches the code

- Where: `docs/product.md:139-142`: "the short sleep timeout stays until the lid opens".
- Code: after a failed fallback the short timeouts also go back at input and at an unlock, and are written again (with
  the sound) after the next 5 idle minutes (`core/lid.rs:384-395`, `:587-595`; decision 7). The README note in
  sleep-route Found outside scope 6 says the same. Fix the line (the orchestrator's file).

## Unverified note

- The dim timeout is not lowered. Haik's battery dim is 165 s (`docs/research.md:3997`); the route writes screen off
  60 s and leaves dim. Whether Windows turns the screen off at 60 s with a longer dim, or only after it, is not known.
  The fallback covers it either way. Worth one look in `p6-int-haik-checks`.

## Checked and fine

- Six keys, one path. `SettingKey::ALL`, the journal names, `PlanSetting` GUIDs for the screen timeout
  (`GUID_VIDEO_SUBGROUP`, `GUID_VIDEO_POWERDOWN_TIMEOUT`), the ownership check reading each entry by key, and the one
  restore routine used by the tray, guard, restore mode, hooks and CLI: every restore point takes the short keys.
  Quit, sign-out (`Ending`, `Ended`), next start (startup restore and `Leftover`), guard (the short apply sets RunOnce
  and the guard), uninstall (except finding 3) all go through it.
- Restore points of the short timeouts in the core: lid open (`lid.rs:326-327`), unlock (`:587-590`), input (a
  reading under the one before, `:389-391`), hold start before its apply (settle order `core.rs:325-326`, test
  `scenarios/lid.rs:1880`), countdown end or pause (`lid.rs:1012-1015`), resumed hibernate (`:1227-1229`). Not at a
  wake alone (`:401-411`, test `:1900`).
- User's own value wins: the ceiling rule (`apply.rs:30-36`) skips a value at or under 60 and not Never; a hold's
  leftover Never over the user's shorter value goes back to it (`apply.rs:305-307`); a user change in the countdown is
  not written again (`settings.rs:285-287`, test `scenarios/lid.rs:1956`); the restore only takes values still ours.
- A short apply in flight is never overtaken: every `Restore` goes behind it on the IO queue, and `done_applied`
  matches answers to applies in order (`settings.rs:473-478`).
- Fallback: run time from the write and again at each resume or clock gap (`lid.rs:401-463`); a mono deadline cannot
  pass during a sleep, since run time stops then. A lost `Suspending` and `Resumed` together only make it fire up to 5
  minutes of awake time after the write, never during a sleep. Lid open ends it in the same pass (`lid.rs:326-328`),
  and `run_hibernate` reads the lid twice, the second right before `SetSuspendState` (`hibernate.rs:97`, `:133`). An
  unknown lid at the fallback is a failure, not a hibernate.
- Floor and failed lock: both end in the same countdown and short write (tests `scenarios/lid.rs:1985`, `:2077`); a
  session lock during a failed-lock step back restores the short timeouts before the hold's apply.
- Session end: no short write while ending (`lid.rs:480-490`), and `Ending` restores whatever the copy says.
- Lost journal: 2 minutes of run time (`settings.rs:94-127`), a damaged file at once, only exact 0 values reset
  (`reset.rs:138`), defaults from the plan's personality as research verified (`plan.rs:249-302`), the writes before
  the set-aside, the set-aside file marked seen (`executor.rs:163-165`), one `JournalLost` notice (kept for the
  user's return), no `RestoreFailed` while unknown.
- A hold that holds at the window's end: locked with the chime after the read that shows the user's own action
  (test `scenarios/lid.rs:954`); the user's own Do nothing gets no lock (`:1010`).
- No-work behavior: the short timeouts, the clock gap and the unlock clamp act only while a countdown runs (it needs
  work that ended, a floor step back or Wakeshot's own Do nothing with the lid closed). The reset is the only new
  change with no work going on, and rule 7 accepts it (apart from finding 6).
