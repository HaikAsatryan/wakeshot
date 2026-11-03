# Verify: fixes for the power and facts design review

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only; no git, no registry, power or screen use.

Scope: the fixes for `docs/reviews/p1-design-power.md`, as the card `docs/tasks/p1-design-review-fixes.md` logs them.
Read: `docs/architecture.md`, `docs/phases/01-design-skeleton.md` (summary, "Values I set", Questions 9 to 12),
`docs/product.md`, `docs/research.md`, `docs/research/distribution.md`, `docs/tasks/p1-lid-sound.md`. The general
review fixes are left to the second reviewer.

Counts: 2 major, 12 minor. All are fixes to the design text; none needs a test before sign-off.

## The 19 findings

| # | Card says | Verified |
|---|---|---|
| 1 | Fixed | Yes. The IO thread checks each entry against its own plan, `UserChanged(OwnedKey)`, no `Release`. Two side effects: minor 4 and 9 |
| 2 | Fixed | In part. Entry read errors keep the entry; `Plan(Result)`; `restored` flag. Open: plan list or active id read failure (major 1); write before mark (minor 3) |
| 3 | Fixed | Yes for release builds (minor 10 for the rest) |
| 4 | Fixed | Yes: Explorer as parent marked not verified, fallback, checklist 4, gap list, summary says End task not tested |
| 5 | Changed | Yes. The reason for skipping the write-back holds (below) |
| 6 | Fixed | In part. The check moved into the command, but at step 1, not right before the call (minor 7) |
| 7 | Fixed | Yes |
| 8 | Fixed | Yes, with loose ends (minor 8) |
| 9 | Fixed | Yes for guard and restore mode; the uninstall case is not in the gap list (minor 13) |
| 10 | Question | The question leaves out the security case (major 2) |
| 11 | Fixed | Yes |
| 12 | Changed | Yes |
| 13 | Fixed | In part. Queued notices are dropped, a queued `Apply` still runs (minor 6) |
| 14 | Fixed | Yes. Restore mode with an Unknown owner has no rule (minor 5) |
| 15 | Fixed | Yes |
| 16 | Fixed | Yes |
| 17 | Fixed | Yes for the case named; a close case is open (minor 11) |
| 18 | Fixed | Yes: lock at once, sound in parallel. Wording only (minor 12) |
| 19 | Fixed | Yes for the four paths; two longer gaps missing (minor 13) |

## Restore paths on the new text

| Path | Do the user's values come back, or does the journal stay until they do? | Open |
|---|---|---|
| Plan switch during a hold | Yes. The old plan's entries are checked on the old plan, restored with no apply; the new plan gets an `Apply` | A failed old-plan restore, retried on the tick, undoes the new plan's hold (minor 4); `released` keyed by plan (minor 9) |
| In-flight read at hold start | Yes. Commands run in order on the IO thread, so a read queued before an `Apply` never sees its entries | none |
| Read and apply errors | Entry read errors: entry kept. Safety-net failure: nothing written. Write failure: restored | A failed plan list or active id read drops entries (major 1); safety-net leftovers become false user changes (minor 8) |
| "Plan gone" | Yes, when the plan list is complete | A failed or partial list (major 1) |
| Failed `PowerSetActiveScheme` | Yes. The entry stays `restored`; the retry applies; an entry on a plan switched away from is dropped, correctly | A stop between the write and the mark (minor 3) |
| Panic | Yes in release: the hook logs and aborts; the guard waits on the process handle, which signals only when every thread is gone | Dev and test builds unwind (minor 10) |
| Guard parent, End task | End task on the tray: the guard restores. On the group: not tested; if both end, the journal stays and RunOnce restores at the next sign-in | Honest in summary, gap list and checklist 4 |
| Hold file with no `procStart` | Not a restore path. A reused pid never counts | none |
| `Apply` against a plan switch | Mostly. The check runs inside the command and the call takes the id it read | The window from step 1 to step 5 (minor 7) |

New paths the fixes leave open, beyond the table: guard and restore-mode retries that act on a new tray's journal
(minor 5), and an `Apply` after the end-of-session restore (minor 6).

## Major

### 1. A failed plan list or active plan read counts as "plan gone"

Where: `architecture.md:453-460` (restore steps 2 and 3); original finding 2.

What fails: step 2 reads the active plan id and the plan list, then drops any entry whose "plan is not in the list".
Only an entry's own value read has a failure branch. If `PowerEnumerate` fails, or the loop stops early on an error
other than `ERROR_NO_MORE_ITEMS`, every entry looks "not in the list": all are dropped, and step 4 deletes the journal
and the RunOnce value and stops the guard. That is exactly original finding 2's result: overridden, with no safety
net. A failed active id read does the same in step 3: no entry is "on the active plan", so `restored` entries are
dropped with no `PowerSetActiveScheme`, and the value in force stays Do nothing (research, "Stored at once, applied
only on `PowerSetActiveScheme`"). Section 8 bans `unwrap`, so `unwrap_or_default()` on the list is the easy code to
write. A failing power API is rare, but restore mode at sign-in is where it would show, and the result is the worst
one.

Fix: in step 2, "the plan list or the active plan id cannot be read in full: this run fails before any entry is
dropped". Drop for "plan deleted" only from a complete list. Add both failures to the Windows layer tests (`:720-722`).

### 2. Question 10 leaves out fast user switching, where its default lowers security

Where: phase file `:114-118`, "Values I set" `:69-70`; `architecture.md:328-333`; original finding 10.

What fails: user A starts an agent and switches user; B signs in at the console. A's tray still owns
`Global\Wakeshot.Settings` and keeps Do nothing on the plan (plans are per machine). B closes the lid and bags the
laptop. A's tray can at most lock A's own session. B's tray does not own settings, so it reads Do nothing as B's own
value (docked use): no lock, no chime. B's session stays unlocked, running in a bag, until Windows locks it at B's
display-off timeout (research, "Lock"). The original review named this case. Question 10 names only an override left
behind after A's session ended, so Haik would answer without seeing it. Decision 18 says never lower security.

Fix: put the case in Question 10. A safer default, about as lean: the settings owner applies only while its session
is the console session; on `WTS_CONSOLE_DISCONNECT` it restores and holds requests only, and applies again on
`WTS_CONSOLE_CONNECT` (session notifications not tested yet; add to checklist 11).

## Minor

### 3. Restore writes `original` before it marks the entry `restored`

Where: `architecture.md:456-458`.

What fails: "live equals `written`: write `original`, mark the entry `restored`, rewrite the journal". A stop between
the write and the journal rewrite (End task, a panic on another thread, Windows ending the session) leaves the stored
value `original`, the value in force Do nothing (only `PowerSetActiveScheme` applies it), and the entry not marked.
The next run (guard, restore mode, next tray) sees "live equals `original`, not `restored`" and takes "otherwise: drop
it". No apply, journal deleted, and Do nothing stays in force until something applies the plan (a plan switch, a
Settings change; a restart most likely, not verified). The window is small; the result is the worst one.

Fix: write ahead. Mark the entries `restored` in the journal (write-through) first, then write `original`. On a later
run, "`restored` and live equals `written`" means the write never happened: write it again. That uses the blind spot
the design already accepts (a user who sets exactly our value). Add "stop between write and mark" to the Windows layer
tests.

### 4. A restore retry restores the current hold's entries

Where: `architecture.md:155` (`Restore { why }`, no plan or keys), `:179` and `:685` (retried on the tick),
`:219-222`, `:229-230`, `:448-461`.

What fails:

- Plan switch: `Restore(PlanSwitch)` for the old plan fails on one read; the `Apply` to the new plan works. The journal
  now holds both plans. The tick retries the routine, which restores every entry, so the new plan's override ends in
  the middle of the hold.
- Work ends, the restore fails, and a new hold starts before the next tick. `Apply` skips keys already at 0, so the
  hold rests on the kept entries; the tick retry restores them.

In both, `Done(Restored(Ok))` empties the core's mirror and nothing applies again until the next hold. The hold runs
on the user's lid Sleep: the PC sleeps at a lid close, and on battery the requests end at the user's sleep timeout
plus 5 minutes. No notice says so.

Fix: `Restore` names the plan it restores (`Restore { plan: Option<Guid>, why }`). A failed work-end restore is
retried only while nothing holds; a new hold adopts the kept entries, which the IO thread already checks. Let
`Done(Restored)` and `Done(Applied)` carry the journal's entries after the command, so the core's mirror is a copy,
not a guess.

### 5. Guard and restore-mode retries do not re-check the journal owner

Where: `architecture.md:325`, `:462-465`, `:484-486`.

What fails: tray T1 dies; its guard's first restore fails, and it retries every 10 s for 2 minutes. The user starts
Wakeshot again (T2): T2's start-up restore works, an agent is busy, T2 applies and writes a new journal that it owns.
The old guard's next retry runs the routine on T2's journal: it restores T2's live override and deletes T2's RunOnce
value. T2's next read sees our values gone and posts `UserChanged`: a false "you changed a setting" notice, and the
keys are released for the episode. The same can happen between restore mode at sign-in and the tray started by the
Run value. Also, restore mode with the owner Unknown has no rule: the Unknown rule (`:471`) says "never starts a
restore", and restore mode has no tick.

Fix: each retry reloads the journal and goes on only while its owner is still the dead process it started for;
otherwise it exits. In restore mode, Unknown counts as a failed run, so it is retried.

### 6. After the end-of-session restore, a queued or new `Apply` still runs

Where: `architecture.md:362-364`; card row 13.

What fails: the fix drops queued notices only. An `Apply` queued just before `WM_ENDSESSION` (a hold that just
started) runs after the restore. Once the loop returns, it also keeps handling posted events until Windows ends the
process, so the core can issue a new `Apply`. Either writes Do nothing, a journal and RunOnce again; the override then
lasts until the next sign-in.

Fix: after `Session(Ending)` the core issues no `Apply`, and the IO thread drops queued `Apply` commands behind the
session-end restore. A cancelled session end (`WM_ENDSESSION` with `FALSE`) lifts both.

### 7. `Apply` checks the active plan at step 1, not right before `PowerSetActiveScheme`

Where: `architecture.md:422-430`; original finding 6.

What fails: between step 1 and step 5 the IO thread runs access checks, a write-through journal write, a RunOnce write
and often a guard `CreateProcess` (restore step 4 stops the guard at every hold end, so most hold starts start one). A
plan switch by the user or an OEM tool in that window: the writes go to the old plan, and `PowerSetActiveScheme` with
the step 1 id switches the user back. Nothing records the user's plan. The window is now milliseconds, not a queue
wait, so this is minor.

Fix: read the active plan again right before `PowerSetActiveScheme`. If it changed, skip the call and report
`PlanChanged`; the writes to the now inactive plan are journaled, and the plan switch path restores them.

### 8. Loose ends around the safety net

Where: `architecture.md:425-427` (Apply steps 3 and 4), `:435-436` (ownership drop), `:461` (step 4), `:480-481`.

What fails:

- A `SafetyNet` failure after the journal write (the RunOnce value or the guard start fails) leaves entries for keys
  never written. The next `ReadPlan` finds live (the original) not equal to `written`, drops them and posts
  `UserChanged`: the user gets "you changed a setting" for nothing. An error counted as the user's.
- When the ownership check drops the last entry, nothing runs step 4: the empty journal, the RunOnce value and the
  guard stay.
- Plan switch in the tray: `Restore(PlanSwitch)` empties the journal and sets the guard's stop event; the `Apply` right
  after "starts the guard if none runs" while the old one is still exiting. The restart rule covers it, after a short
  time with no guard.

Fix: on `SafetyNet`, undo what step 3 made (entries, RunOnce value, guard). Run step 4 whenever the journal becomes
empty, from any path. `Apply` starts a new guard when the current one's stop event is set.

### 9. `released` keyed by plan overrides the user again after a plan switch

Where: `architecture.md:197`, `:220-222`, `:261-262`; `product.md:116-118`; research, "What Windows Settings writes"
(Settings writes battery sleep and the battery lid action to the active plan and to the inactive Balanced).

What fails: `product.md` rule 3 says a user change stops the override "until the current work ends", and "Restore my
power settings" counts as one. With `released: Set<OwnedKey>`:

- On plan P the user sets the battery lid action to Sleep in Settings, which writes P and Balanced; (P, `LidDc`) is
  released. The user switches to Balanced: `Apply` writes Balanced's `LidDc` to 0, over the value they just set.
- After "Restore my power settings", any plan switch (or an OEM tool's switch on an AC change) applies to the new plan
  again.

Fix: key `released` by `SettingKey` alone, and let "Restore my power settings" stop all writes for the episode. Journal
entries stay keyed by plan, so finding 1's fix stands.

### 10. The panic hook must abort by itself

Where: `architecture.md:373-374`, `:694`, `:771`; `:334-335`.

What fails: `panic = "abort"` is set only in the release profile (section 10). Dev and test builds unwind (proptest
most likely needs that to shrink; not verified). There, a panic on the IO thread ends only that thread: the tray lives,
the guard sees it alive, and nothing restores. Phase 3 checks on the test laptop could run such a build. Also, a tray
that aborts while holding `Local\Wakeshot.Journal` leaves it abandoned; the next wait returns `WAIT_ABANDONED`, which
means the caller now owns it. The text does not say so.

Fix: the hook calls `std::process::abort()` after logging, in every profile. Say that `WAIT_ABANDONED` counts as taken
(the journal is whole: atomic write).

### 11. Lid closed, unlocked on the external screen, then undocked: no lock

Where: `architecture.md:231-239`; phase file summary `:33-35`.

What fails: Haik's own values are lid Sleep on AC and DC. Docked to the DELL, an agent holds, Haik closes the lid: lock
and chime, right. He signs in on the DELL and keeps working with the lid closed, which only Wakeshot's Do nothing
allows. Then he unplugs the dock and bags the laptop. The new rule locks on a power source change only when the action
in force "was the user's own before"; on AC it was ours, so no lock and no chime. The session stays unlocked in the bag
until Windows locks at the battery display timeout (research, "Lock": within 1 s of the display-off standby entry), or
with Keep screen on until Wakeshot's idle lock at the user's battery sleep timeout. "The same rule" in the text can
also be read as the plain lid-close rule, so code could go either way.

Fix: on AC to DC with the lid closed and our Do nothing in force on DC, lock and play the chime once DC has lasted a
few seconds (for example 5 s, so a dock's short AC drop does not lock someone typing on the external screen). Add the
case to checklist 8.

### 12. The lock and chime sentence cites the alias case for the chime

Where: `architecture.md:231-235`; research, "Lid-close sound", "Lid-close results" (`research.md:792-797`);
`tasks/p1-lid-sound.md:176-180`, `:213`.

Check: the design does what the research supports: `Lock` and `Play` at once, no wait, and a "change it when". The
lock does not cut sound (close 1). But in close 1 the lock came 0.2 ms after `PlaySound` (the Windows alias), and our
chime played 3 s later, into an already locked session; in close 2 the chime's stream was running before the lock. So
the v1 order, the lock landing during the chime's 0.3 to 0.8 s stream start (`IAudioClient::Initialize`), was tested
only with the alias, and both closes had a loopback capture open on the endpoint, which the card calls "not quite the
product case". "With the lock 0.2 ms after the sound call, the sound still played in full" reads as if it were the
chime.

Fix: name the alias in that sentence, mark the chime in the v1 order "not tested", and add a checklist line (Phase 3 or
5): lid close while holding, no loopback running, chime heard, session locked. In the same pass, mark the two points
behind "an inactive plan needs no apply" (`:442-443`, `:459`): a write to an inactive plan changes only that plan
(section 9 marks it not tested) and activating the plan later applies its stored values (inference from "Stored at
once, applied only on `PowerSetActiveScheme`").

### 13. "Gap left" and Question 11 miss two longer gaps

Where: `architecture.md:497-505`, `:618-621`; phase file `:119-122`.

What fails:

- Uninstall during a hold: Velopack kills the tray and the guard, then the hook restores (`distribution.md:36-39`). If
  that restore fails, the hook keeps the journal and removes the RunOnce value; the exe is gone and hooks show no UI.
  The override stays, with no warning, until a reinstall: not "until the next sign-in".
- Restore mode fails for 2 minutes, or panics (abort, no retry), at sign-in with start at login off: Windows already
  deleted the RunOnce value (research, "RunOnce"), so nothing restores at the next sign-in either, only at a manual
  start. The gap list says "before the next sign-in".

Fix: the uninstall hook retries within its 30 s limit. List both cases, with their real length, in "Gap left" and in
Question 11.

### 14. Question 12 does not say the started tray works as usual

Where: phase file `:123-125`; `architecture.md:262-263`, `:462-464`.

What fails: the tray started after 2 minutes of failed restores also holds for agents (Automatic is on by default),
so a user who turned start at login off gets Wakeshot back, overriding at the next agent turn. It also starts 2 minutes
after the user picked Quit, when Quit's restore failed. The default may still be right, but Haik should see both.

Fix: add both to Question 12.

## Checked and fine

- Hold file with no `procStart`: a pid is reused only after its writer has exited, which is after the writer's last
  write, so the start-before-write test never counts a reused pid, and it keeps no state. Only a later write to the
  file by some other tool would make it count again; acceptable.
- Ownership on the IO thread: a value is dropped only on a successful read that differs from `written`; read errors
  release nothing; entries marked `restored` are skipped.
- Liveness: only a successful process list without the pid and start time counts as Gone; a failed list is Unknown and
  deletes nothing.
- Guard parent: Velopack kills by exe path inside the install folder (`distribution.md:36-49`), so Explorer as parent
  does not keep the guard alive through an update or uninstall.
- Facts the new text leans on point to `research.md` or are marked: `Global\` mutex creation, RunOnce delete, posted
  messages in modal loops, `LockWorkStation` 0.5 ms, battery Never for 16 minutes, Settings writes, AC drop on the
  dock, lid notification doc text, timers after a failed hibernate (inferred), boot-time task needs admin (not
  verified), Explorer as parent (not verified), inactive-plan write (not tested, section 9), checklist 15.
- Lid lock and sound: lock at once, chime in parallel, no wait, as the research supports (minor 12 is wording and a
  checklist line).
- Question 11's three listed paths are real and marked; Question 12's default keeps the user warned.

Verdict: 15 of 19 findings are fixed as stated and the lock-at-once change is right; fix major 1 and reword Question 10
before Haik signs off, and take the minors in the same pass.
