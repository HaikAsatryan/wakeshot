# Final check: the three verify majors and the sign-off list

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only; no git, no registry, power or screen use.

Scope, as briefed: power verify major 1 (no restore path deletes the journal, RunOnce value or guard after a failed
read or write), power verify major 2 (Question 10 and fast user switching), general verify major (Gate B), and the
sign-off list. Read: `docs/tasks/p1-design-verify-fixes.md`, both verify reviews, `docs/architecture.md` sections 3 to
6, 8 and 9, `docs/phases/01-design-skeleton.md`, `docs/product.md`, `docs/plan.md` "Git", the orchestrate skill's Git
lines.

Counts: 0 blocker, 1 major, 8 minor.

## The four checks

| Check | Result |
|---|---|
| 1. No deletion after a failed read or write | Fails on one path: an `Apply` that takes over an entry of a restore whose `PowerSetActiveScheme` failed (major 1). Every other path holds (table below) |
| 2. Question 10 and fast user switching | Covered; the default is safe in the normal path. Two gaps in the wording and one in the requests (minors 2 and 6) |
| 3. Gate B | Pass. Step 1 (Gate A) no longer reads as passed; step 3 is Haik's yes right before `git init`, covering steps 4 to 9, again before the push on a scan hit he is asked about. No commit or push step comes before it; `plan.md` and the skill point to the phase file before the first push |
| 4. Sign-off list | Counts right (1 blocker, 13 major, 47 minor); 25 lines; Questions 10 to 12 with defaults. Its main claim is false until major 1 is fixed; Q12's short form drops the side effects Haik must see (minor 1); "read only this list" but the values are by reference (minor 3) |

Restore paths walked on the current text:

| Path | Holds? |
|---|---|
| Restore step 1, journal read error other than missing | Yes: the run fails |
| Step 2, failed active id read or a list that stops on any code but 259 | Yes: fails before any entry is dropped or written |
| Step 3, failed entry read | Yes: entry kept, run fails |
| Step 4, write ahead (mark, then write) | Yes: a failed mark writes nothing; a failed write keeps the entry, and "marked, live equals written" restores again |
| Step 5, failed second id read or failed `PowerSetActiveScheme` | Yes: entries kept, run fails |
| Step 6 clean-up | Yes: only after the last entry is dropped on a successful read |
| Ownership check | Yes: drops only on a successful read that differs; a deleted plan's read fails and keeps the entry |
| Apply safety net, journal last; undo only when the journal is known absent | Yes, for fresh entries |
| Apply takes over a `restored` entry, then a write fails, the call fails, or a stop | No: major 1 |
| Guard and restore mode retries; Unknown owner | Yes: reload, owner re-check, Unknown is a failed run |
| Session end, Quit, ConsoleLeft, CLI | Yes: a failed restore keeps everything; the guard covers Quit |
| Uninstall | RunOnce removed after a failed restore, as stated (section 6); journal kept |
| Corrupt journal | Moved aside after a read that does not parse; accepted earlier, consequence not listed (minor 4) |

## Major

### 1. An `Apply` that takes over a failed restore's entry clears its mark before it writes

Where: `architecture.md:465-468` (Apply step 3), `:474-479` (steps 5 and 6), `:199` and `:763` ("restore what was
written"), `:509-516` (restore step 3), `:484-486` (ownership), `:232-233` and `:276-280` (takeover in the episode
flow); sign-off list `phases/01-design-skeleton.md:17-18`.

What breaks: a restore whose `PowerSetActiveScheme` fails leaves entry E marked `restored`, the stored value
`original`, and our Do nothing still in force (only the call applies it). While a hold runs, the retry skips the
active plan and the hold's `Apply` takes E over. Apply step 3 writes the journal with E's `restored` mark removed,
before step 5 writes E's target. From then on E reads "not marked `restored`, live equals `original`", which both the
ownership check and restore step 3 ("otherwise: drop it ... never written") treat as not ours. Three ways in:

- Step 5: a write for an earlier key fails, so E is never written. "Restore what was written" does not touch E.
- Step 6: `PowerSetActiveScheme` fails again (likely, since it just failed in the restore). "Restore what was
  written" writes E's `original` back. Read as "undo the writes", which `:199` suggests, E is left unmarked with live
  equal to `original`. Read as "run the restore routine", this case is safe; the text does not say which.
- A stop (End task, abort) between step 3 and step 5: the guard's routine drops E.

Result: the next ownership check drops E and posts a false `UserChanged` (a "you changed a setting" notice, the key
released), or the next restore drops it; as the last entry, step 6 deletes the journal and the RunOnce value and stops
the guard. Do nothing stays in force with no safety net, and the core now treats it as the user's own value, so a lid
close gets no lock (`:247`). That is the worst result, behind a failed write or call, which the acceptance check and
the sign-off list ("stay until a restore really works") rule out. The tests at `:801-806` name the takeover but not a
failure after it.

Fix (text only): in Apply step 3, a taken-over entry keeps its `restored` mark; step 6 clears the marks (one journal
write) only after `PowerSetActiveScheme` succeeds. Say that "restore what was written" runs the restore routine on
this plan's entries, so an entry leaves the journal only after a successful call or on a complete plan list. Add a
test: takeover of a `restored` entry, then a failed write, a failed call, and a stop before step 5.

## Minor

### 1. Question 12's short form drops the side effects

Where: phase `:32-33`; full text `:168-173`; power verify minor 14.

What breaks: the list says the started tray is there "to warn you". It also holds for agents as usual, so with start
at login off Wakeshot is back and overrides at the next agent turn. The other option (retry and warn only, then exit)
is missing. Haik signs off from the list only, so he answers without what minor 14 asked to show him.

Fix: "That tray also works as usual: it overrides again at the next agent turn. Other option: it only retries and
warns, then exits."

### 2. Question 10's short form misses Remote Desktop and what is at stake

Where: phase `:28-29`; full text `:150-160`; `architecture.md:256-261`.

What breaks: the default also stops the override when Haik himself uses Remote Desktop into his session (most likely,
not tested), with no second user at all; the list speaks only of "another user". "Beyond that, v1 assumes one user"
does not say the risk it accepts: an override left behind keeps another user's closed laptop running unlocked. Also
not named anywhere: if the restore at `ConsoleLeft` fails, the warning shows in the session that left, not to the user
at the screen.

Fix: one clause each in the short form: "also over Remote Desktop into your own session"; "if your override is left
behind after a crash, another user's closed laptop can run unlocked".

### 3. "Read only this list", but the values are elsewhere

Where: phase `:11`, `:22`.

What breaks: the list sends Haik to "Values I set" (about 30 lines) for the values he has not seen, so it does not
stand alone. The card allowed the reference; the first line contradicts it.

Fix: "Read this list and 'Values I set' below to sign off."

### 4. A corrupt journal leaves the override for good, and no list says so

Where: `architecture.md:503-504`, `:766`; Apply step 2 `:463-464`; `:247`; gap list `:564-581`; Question 11.

What breaks: a journal that reads but does not parse is moved aside and values stay as they are. The next `Apply`
sees Do nothing with no entry and skips it as the user's own value, so nothing ever restores it, and a lid close gets
no lock. The earlier reviews accepted moving it aside; the consequence is not in "Gap left", Question 11 or the list,
and the list's "a failed read never deletes your saved values" reads as covering it.

Fix: name it in "Gap left" and Question 11: "until the user changes it by hand; the warning and the log give the old
values". Keep the move-aside.

### 5. The list's main claim is absolute, the design has an exception

Where: phase `:17-18` vs `architecture.md:579-581`, `:696-697`.

What breaks: "the sign-in restore entry ... stay until a restore really works", but uninstall removes RunOnce after a
failed restore. Question 11 covers the case, so this is wording.

Fix: add "(an uninstall whose restore fails is the exception, Question 11)".

### 6. Fast user switching keeps the screen request

Where: `architecture.md:256-257` ("hold requests only"), `:229`.

What breaks (unverified: whether Windows honors a screen request from a session not on the console): with Keep screen
on, A's tray keeps DISPLAY after B takes the screen. B's screen then never turns off, so Windows' lock at screen-off
never comes for B, and A's idle lock can lock only A's session. B walks away and stays unlocked.

Fix: while another session is on the console, hold the system request only.

### 7. Apply step 1 has no rule for a failed read

Where: `architecture.md:461-462`, `:760`.

What breaks: steps 2 to 6 name their failures; step 1 does not. Code that maps a failed read to "not the same plan"
reports `PlanChanged` and waits for a plan switch that never comes: the hold runs without the override and no notice
says why. Nothing is deleted.

Fix: "the read fails: write nothing, report it like `SafetyNet`".

### 8. Stale lines next to the list

- `product.md:125-127`: "nothing restores before the next sign-in" for every "Gap left" case. The list now has longer
  ones (until the next start, until a reinstall; Question 11 open).
- Phase `:82-83`: "Phase 1 work after your sign-off: repo files, Cargo workspace, CI". These ran under the 15:45 ahead
  sign-off and are done (work log 15:58, 16:14, 16:25).

Fix: `product.md`: "or longer in the cases of Question 11". Phase file: "Done under your ahead sign-off: repo files,
workspace, CI; still to come: their review, then Gate B ...".

## Checked and fine

- Restore step 2 judges "plan deleted" only from a list that ends with 259; a failed id read fails the run; section 8
  and the Windows layer tests name both.
- Restore step 4 marks first, then writes; step 3's "marked, live equals written" catches a stop after the mark.
- Apply writes the journal last; the undo takes back RunOnce and the guard only when the journal is known absent.
- Retries re-check the journal owner; Unknown in restore mode is a failed run.
- Gate A, Gate B, freeze, Gate C and step 11 as described in check 3; the commit-msg pattern and scan self-test are in
  place.
- Question 10's default: a tray overrides only while its session is on the console; the restore on `ConsoleLeft`
  takes the whole journal; the retry rule then uses `plan: None`; the settings mutex stays with the first tray, so the
  second user never overrides.

Verdict: not ready for Haik. Fix major 1 (Apply step 3 and "restore what was written") and minor 1 (Question 12's
short form) first, since both change what the list tells him; take the other minors in the same pass.

## Round 2

Re-checked 2026-11-03 after round 2 of `p1-design-verify-fixes`, on the current `architecture.md` (910 lines), phase
file and `product.md`. Walked from the text first, then compared with the card's Log walk: they agree.

Counts for round 2: 0 blocker, 0 major, 2 minor (new). All 9 round 1 findings are fixed.

### Major 1: fixed

Text: Apply step 3 keeps a taken-over entry's `restored` mark (`architecture.md:470-473`); step 6 clears marks only
after a successful call, and any failed write, read-back, second plan read or call runs the restore routine on the
plan's entries (`:479-487`, `:199-200`, `:785`); restore step 3 names the takeover (`:532-533`); tests name the cases
(`:825-832`).

Stop points, E taken over from a failed restore (E1 marked, live `original`, our value in force; E2 marked, live
`written`; E3 unmarked, live `written`), F new:

| Stop or failure | State left | Next check | Right? |
|---|---|---|---|
| Step 1 read fails or `PlanChanged` | nothing changed | as before | yes |
| Steps 2 or 3 fail | journal unchanged (written last, atomic) | as before | yes |
| After the journal write, before step 5 | E as it was; F unmarked, live `original` | E finished by the routine; F dropped as never written, nothing was applied | yes |
| After E1's write in step 5 | E1 marked, live `written` | "restore again" | yes |
| A write fails before E1 is written | E1 marked, live `original` | routine keeps it for the call | yes |
| Read-back, second read or call fails | routine on the plan's entries; a failure keeps them | retried | yes |
| Plan changed before the call | E marked, F unmarked, both live `written` on an inactive plan | plan switch restore writes `original`, no apply | yes |
| After the call, before the marks are cleared | E marked, live `written`, in force `written` | restored again once the tray is gone | yes |
| Clearing write fails | marks stay | ownership skips them; the next restore handles them | yes, a user change on that key goes unseen until then (stated) |

No path drops E while our value is in force.

### Round 1 minors: all fixed

| # | Where now |
|---|---|
| 1 Question 12 short form | phase `:34-36`: works as usual, overrides at the next agent turn, other option |
| 2 Question 10 short form | phase `:27-30`: Remote Desktop, the accepted risk; the failed `ConsoleLeft` restore in the full text and `architecture.md:265` |
| 3 "Read only this list" | phase `:11-12` |
| 4 Corrupt journal | `architecture.md:518-523`, `:602-603`, `:789`; Question 11 full and short |
| 5 Absolute claim | phase `:18-19`: "(exceptions in Question 11)" |
| 6 Screen request | `architecture.md:258-263`: system request only while off the console (screen request from another session: not verified, marked) |
| 7 Apply step 1 | `architecture.md:466`, `:786`: a failed read is `SafetyNet` |
| 8 Stale lines | `product.md:125-127`; phase `:85-87` |

The list's counts are right: five reviews, 1 blocker, 14 major (4, 6, 1, 2, 1), 55 minor (13, 13, 9, 12, 8).

### New paths

None drops an entry while our value is in force. Checked: restore step 1's clean-up of an empty journal (nothing
owned); step 6's clearing of marks (it runs after the call that applied our values); the routine inside a failed
`Apply` (drops only on its own rules); the `SafetyNet` at step 1 (before any journal, RunOnce or guard change); the
console rule (requests only).

### R2-1 (minor). A failed apply leaves the retry waiting for an apply that never comes

Where: `architecture.md:281-285` (retry rule), `:476`, `:785-786` ("requests only for the episode").

What breaks: after `SafetyNet` or `Write`, the core holds requests only for the episode. The retry rule's "while the
core applies" lists holding, console, floor, session end and released keys, not a failed apply. Read as written, the
retry keeps skipping the active plan's entries "for the hold's `Apply`", which this episode never sends. Case: E1 is
left by a failed restore, then the next `Apply` fails at step 1 or its routine fails again. E1's Do nothing stays in
force until the hold ends, while the `RequestsOnly` notice says Wakeshot is not overriding. Nothing is dropped; the
restore only comes late.

Fix: add "no apply failed in this episode" to the condition at `:281-282`, so the retry is then `Restore { plan: None }`.

### R2-2 (minor). Question 11's short form leaves out the missing lock

Where: phase `:31-33` vs full Question 11 `:172-174` and `architecture.md:521-522`.

What breaks: the short form says a damaged saved-values file leaves the settings overridden "until you set them by
hand". The full text adds "and until then a lid close gets no lock", which is the security cost (decision 18). Haik
signs off from the short form, so he would accept it without seeing that.

Fix: end the short form's case with "; until then a lid close gets no lock".

Verdict: ready for Haik once the Question 11 line in the sign-off list adds "until then a lid close gets no lock"
(R2-2); R2-1 can go in the same edit or with the build-card notes.
