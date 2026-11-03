# Power safety review, finding 1: verify the fix

Reviewer: `wakeshot-reviewer`, 2026-11-04, started 00:20, written 00:27. Brief: check the fix of finding 1 (major) of
`docs/reviews/p2-6-power-safety.md` and what it touched, against Haik's rule: a closed laptop that keeps running
because of Wakeshot must be locked; a lock that fails, or reports `Ok` with no `Session(Locked)` within about 5 s, is
retried once at the next tick; if that fails, Wakeshot steps back (requests off, restore) and starts the hibernate
countdown, unless the user's own lid action for the current power source is Do nothing. Also: no loop, a clean end at
lid open or a late `Locked`, the docked case never locked or hibernated by this path, and the floor rule (finding 7)
on both paths.

Read: finding 1; `docs/tasks/p2-core-lid.md` "Power review fixes (2026-11-03)"; `docs/product.md:76-110`;
`docs/architecture.md` section 3 and the section 8 row (`:967`); `crates/wakeshot-core/src/core/lid.rs`,
`core/settings.rs`, `core.rs`, `policy/lid.rs`, `policy/ending.rs`, `policy/requests.rs`; the new scenarios in
`scenarios/lid.rs:1196-1434`, `:716-763` and the model in `scenarios/props.rs`; the loop's timer and `Lock` runner in
`crates/wakeshot-windows/src/app/dispatch.rs:201-344`; the start lock state in `app/startup.rs:359-378`.

Ran once: `cargo test -p wakeshot-core --locked` in a scratch build folder (deleted after): 561 passed, 0 failed, 2
ignored, plus 10 and 23 in `tests/`. No git, no power setting touched.

## Verdict on finding 1: Fixed

Each part of the rule, checked in the code:

- The watch. A lid-close or source lock into a session the core knows as unlocked starts a watch
  (`lid.rs:199-208`). `Session(Locked)` ends it (`lid.rs:281-285`). `Done(Locked(Err))` moves the check to now
  (`lid.rs:693-697`); an `Ok` waits 5 s (`LOCK_CHECK_MS`, `lid.rs:33`). The check is a deadline (`lid.rs:395`), and
  the loop turns every deadline into `InputIdle` then `Tick` (`dispatch.rs:201-216`, `:274-297`), so it runs.
- The retry. At the check, with the lid closed and our Do nothing in force for the current source (`lid_ours`), one
  `Lock`, no second chime, one `LockFailed` for both (`lid.rs:403-436`).
- The step back. A second failure sets `lock.stepped_back` (`lid.rs:418-425`). The same pass turns requests off
  (`holds_back`, `core.rs:287-297`), sends `Restore { plan: None }` when anything may be owned, and calls
  `work_ended` (`lid.rs:366-370`). No apply goes out while it lasts (`settings.rs:50`), and a restore retry takes
  every entry (`settings.rs:154-168`). Requests go off before the restore (test at `scenarios/lid.rs:1270`).
- "Unless their own action is Do nothing". `lid_ours` needs a journal entry with `original` not Do nothing
  (`policy/lid.rs:71-74`), so at the check the user's own action for the current source is never Do nothing and
  `work_ended` keeps the countdown. A later change to a source whose own action is Do nothing pauses it, and it
  restarts only after 30 s on a source whose own action is not (`lid.rs:541-557`).
- No loop. The step back ends only at `Session(Locked)` or a lid open (`lid.rs:219-220`, `:284`). After a `Locked`
  end the session is locked, so nothing locks again until a new lid close or a source lock, each a real event. A
  failure while already stepped back sends nothing new (`step_back_sent`). The lid proptest allows one lock without
  a chime per watched lock (`scenarios/lid.rs:1963-1983`).
- Clean end. `end_lock_step_back` clears both flags and asks for an apply (`lid.rs:439-447`). With a hold, the
  settle pass cancels the countdown (`Cancel::HoldingStarts`, `lid.rs:372-374`); with no hold the work-end countdown
  goes on, which is right. A lid open also cancels every countdown and the watch (`lid.rs:218-220`).
- Docked. A user whose own lid action is Do nothing on the current source gets no lock (`lid_ours` false), so this
  path never starts. The source lock needs our Do nothing on battery for 30 s first. After a step back on battery, a
  re-dock to their own AC Do nothing pauses the countdown (code read; no test, see minor 4).
- Floor rule (finding 7). One rule, `floor_countdown_starts` (`lid.rs:154-159`): lid closed and the user's own
  battery lid action (journal `original`) not Do nothing. Used on both paths: the step back with the lid closed
  (`lid.rs:360-362`) and a lid close after it (`lid.rs:185-187`). The second path already needed our Do nothing on
  battery, which implies the same, as the card says.
- Facts that make it hold: `Lock` runs on the loop thread and answers in the same batch (`dispatch.rs:341-344`), so
  no late `Done(Locked)` can confuse the watch. A tray that starts in a locked session gets `Session(Locked)` right
  after `Started`; an unknown state counts as unlocked, so the error is a false step back, the safe side
  (`startup.rs:359-378`).

Counts of new findings: 0 blocker, 0 major, 5 minor.

## Minor

### Minor 1. Hibernate not allowed or failed after a lock step back leaves the closed laptop unlocked for hours

- Where: `core/lid.rs:720-735` (`done_hibernate` ends the countdown and notifies); `lid.rs:366-370` (the countdown
  is the step back's only fallback); nothing sends `Lock` again while the step back lasts.
- What breaks: lid closed, unlocked, lock fails twice, step back, 5 minutes idle, `Hibernate`. On a PC with
  hibernation off (`IsPwrHibernateAllowed` false) or a failed `SetSuspendState`, the countdown ends with
  `HibernateFailed`. The user's settings are back, but Windows does not sleep a laptop whose lid is already closed
  (`product.md:117`, tested), so it runs unlocked until the user's own sleep timeout (3 h on AC, 4 h on battery on
  the test laptop; `architecture.md:770-771` says inferred, not tested with the lid closed). That is the "unlocked
  in a bag" case the fix is for.
- How likely: needs two failed locks and no hibernate. How often hibernation is off on user PCs: not verified.
- Fix (Haik's call, it adds to his rule): while the lock step back lasts with the lid closed, send `Lock` again at
  each 30 s safety tick, or at least once after a failed hibernate. A lock that works ends the step back through
  `Session(Locked)`, the state the rule wants, so it cannot loop.

### Minor 2. The step back's only notice may be shown on a dark screen

- Where: `core.rs:355-363` (`notify` holds a notice only while locked or off the console); `lid.rs:415-417` and
  `:693-702` (`LockFailed` at the first failure); card decision 4 (no notice of its own for the step back).
- What breaks: lid closed, no external screen, session unlocked: `LockFailed` goes out at once, to nobody. The step
  back then drops the hold and the laptop hibernates. Back at the laptop, the closed-lid report says how long it ran
  and what held it, but nothing says why the hold stopped. Finding 1 named "the notice is shown to nobody"; the fix
  leaves that part. Whether Windows keeps the balloon in the Notification Center: not verified.
- Fix: when the step back starts with the lid closed, also keep `LockFailed` for the user's return, at the same
  moment as the closed-lid report (lid open while unlocked, or the first unlock with the lid open).

### Minor 3. A step back before the first plan read starts no countdown

- Where: `core/lid.rs:142-148` (`lid_ours` reads `lock_view`, a guess from the `Started` journal) against
  `lid.rs:484-486` (`work_ended` reads `in_force`, `None` before the first read, and returns).
- What breaks: a tray starts with a journal its start-up restore could not clear (a crashed tray's Do nothing in
  force), lid open. The lid closes before the first `Plan` answer, and `Lock` fails twice; after an `Err` the retry
  goes about 10 ms later (`dispatch.rs:215`, the timer minimum), so both can come before that answer. The step back
  sends the restore but no countdown, and the later `Plan` answer does not start one. If the restore lands, the
  laptop runs closed and unlocked until the user's own sleep timeout; if it keeps failing, our battery Never stays.
  The same `None` return hits the no-hold lid close in `take_lid` (`lid.rs:183-184`).
- How likely: very low (three coincidences in the first moments after a start).
- Fix: in `work_ended`, read `lock_view()` when `in_force()` is `None` (it has the lid keys and their `original`);
  or call `work_ended` again at the first `Plan(Ok)` while the lid is closed and a step back or a lid close without
  a hold is waiting for it.

### Minor 4. No test pins this path across a power source change, and two properties excuse it

- Where: `scenarios/lid.rs:1196-1434` (every step back test stays on one source;
  `a_failed_source_lock_is_sent_once_more_too` stops at the retry); `scenarios/props.rs:577`
  (`no_request_without_a_hold` checks the floor flag only, not the model's `lock_stepped_back`); `props.rs:838`,
  `:865` (`closed_laptop_is_locked_in_time` excuses the whole stretch once any lock failed).
- What breaks: the brief's docked check and the step back's own effects rest on code reading and single-source
  scenarios. A regression that keeps the countdown running after a re-dock to the user's own AC Do nothing, or that
  keeps requests on or skips the restore after the second failure, would pass every property. The code is right
  today.
- Fix: one scenario with own lid AC Do nothing and DC Sleep: unplug, source lock fails twice, step back; re-dock: no
  `Hibernate` for 20 minutes; unplug for 30 s: the countdown restarts. One with own AC Sleep: AC back after the step
  back keeps the `WorkEnded` countdown. In props, check `lock_stepped_back` in the request property, and replace the
  blanket excuse with "requests off and a `Restore` within one tick of the retry's failure".

### Minor 5. Docs do not say when the step back ends

- Where: `docs/product.md:85-88`; `docs/architecture.md:316-320` and `:412`.
- What breaks: the product text says the step back hibernates the laptop, but not that an `Ok` lock with no lock
  seen in 5 s counts as failed, nor that the step back ends when the session locks or the lid opens, after which the
  hold runs again. Section 3's state list does not name the lock watch (it sits beside the state, card decision 2),
  and its countdown line says a new hold cancels it, which is not so while stepped back after a failed lock. The
  section 8 row (`:967`) has it all.
- Fix: one clause in `product.md:86-88` ("until the session locks or the lid opens"; "a lock Windows does not confirm
  within 5 s counts as failed"); in section 3, name the lock watch and add "or after a failed lock" to the countdown
  line's stepped back case.

## Checked and found sound

- The retry after an `Err` goes about 10 ms later, not at the 30 s tick, so a cause that lasts (for example the
  secure desktop) fails the retry too and the step back follows at once. The card names it (decision 3); it errs
  toward safety, and the step back ends by itself once the session locks.
- A floor step back and a lock step back at once: two restores (harmless); the `WorkEnded` reason wins and AC back
  does not cancel it until the lock step back has ended, then `HoldingStarts` does.
- A source lock while stepped back (only when the restore failed and our Do nothing is still in force): one more
  lock with a chime and its retry; no second restore, no loop.
- A plan switch, `SettingsOwner`, `ConsoleBack` or `EndCancelled` during the step back sends no apply
  (`nothing_is_applied_while_stepped_back_after_a_failed_lock`).

## Outside this check

- The lock watch and step back live only in memory (`core.rs:44-47`), and a tray's first lid reading is no edge
  (`lid.rs:170-172`). So a tray that starts with the lid closed and the session unlocked never locks when a hold then
  puts our Do nothing in force on battery. This is older than the fix (any start with the lid closed), mostly covered
  by the AC to battery lock; the fix only adds one more way in (a restart during a step back). Haik's call whether a
  first apply with the lid closed and the session unlocked should lock.
- The tray shows "Cannot hold awake" during a lock step back (`core/view.rs:29-32` reads only the floor flag); the
  card lists it (Found outside scope 2).
