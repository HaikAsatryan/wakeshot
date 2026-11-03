# Delta review: Phase 0 close, morning results

Reviewer, 2026-11-03, after 12:55. Read only: no spike run, no setting changed, no screen used. One extra read: the
System log for 10:36 to 10:41 with `Get-WinEvent`, to check F's sleep states.

Read: the lid and power cards (Results and Logs); the raw logs D `spikes/lid-sleep/out/lid-20261103-103006.log`, F
`suspend-20261103-103649.log` and `-103853.log`, H `resume-20261103-104117.log`, `-105721.log` and `-120914.log`, E
`lid-20261103-111112.log`, I `lid-20261103-114033.log`, C `spikes/power-settings/out/hold-20261103-112124.log`, the
Settings tour `watch-20261103-122958.log`; the requests log `requests.log` in the shared scratchpad; `00-facts.md`
from 10:22; `p0-close-proposals.md` parts 1 to 3; `product.md`, `architecture.md`, `research.md`.

Tray card: it finished at 13:08, during this review. I read its Result only for what touches this review (sound,
notifications); it is not reviewed in full here.

Counts: 0 blocker, 3 major, 10 minor.

## Findings, most severe first

### Major

**M1. The cure for the battery rule was never tested.** `product.md:60-61`; part 3 item 1.
The tests prove the problem. With a request held on battery, Windows suspended the laptop 916 s after the last input
(I, lid closed, Haik's 600 s timeout, `lid-20261103-114033.log:684`) and 380 s after it (C, lid open, 60 s copy,
`hold-20261103-112124.log:108`). No run tested the cure: a raised or Never battery sleep timer with a request held.
Every hold on battery, lid open or closed, rests on it. If Windows still ends requests with Never (for example from
another idle timer), a hold on battery still ends about 15 minutes in on Haik's laptop.
Settle: one Haik block on battery, DELL out, hands off: `spike hold --sleep-dc 0 --minutes 16` from the morning
shell (existing mode; `--sleep-dc 0` writes 0 = Never on a copy, `main.rs:904`, `943`, no range check). Pass: no GAP
in 16 minutes, past the 900 s where I failed. About 4% battery. If Haik prefers, move it to the Phase 3 manual
checklist and mark the `product.md` line "not tested: that a raised timer stops the rule".

**M2. The lid-close sound is not a working cue.** `product.md:65`, `product.md:12-13`; part 3 item 6.
`PlaySound` returned true in D, E and I, but no block shows that the user heard our sound. In D the default device
was the DELL, which has no speakers. In E the DELL cable came out 4.3 s before the close, so the device-remove sound
most likely played. In I, the only clean run (laptop speakers at 67%, nothing unplugged), Haik heard nothing. D is a
normal product case, not a test accident: the default device can be a monitor with no speakers, headphones, or muted.
Fix: Haik picks the cue (section 4; my pick: no sound, a notification after sign-in).

**M3. The proposed idle lock edit drops the lock for Keep screen on, with no fact.** `product.md:62-64`; part 3
item 3.
H and E show that Windows locks within 1 s of every display-off standby entry, with or without a held request. That
covers every hold where the screen turns off. It does not cover Wakeshot's own Keep screen on: the display request
keeps the screen on, so there is no standby entry and Windows never locks. Today's rule locks when idle passes the
user's sleep timeout; both branches of item 3 drop that. "Never lower security" is a fixed decision, and no test
touches this case. Fix: keep the idle lock only for Keep screen on (edit in section 1, item 3). If Haik wants Keep
screen on to act like a video player (no lock), he says so explicitly.

### Minor

**m1. Tool lines that misread their own data.** Besides E's "no standby" (`lid-20261103-111112.log:464`, against the
506 at `:480`):

- H run 1 (`resume-20261103-104117.log:799`): "hold-phase gaps 0 (request honored)". No hold phase ran: every tick
  is `[before-sleep]`, because no suspend came. The same line says the control "proves nothing"; the card and the
  phase log read it right.
- F (`suspend-20261103-103649.log:83`, `-103853.log:82`): "the PC stopped (Slept)". Both were hibernates: my System
  log read gives Kernel-Power 42 and 107 with TargetState 5, and Kernel-Boot 27 BootType 2, for both, as the card
  says. The gap kind cannot tell standby from hibernate.
- I (`lid-20261103-114033.log:729`): "the lid was open when work ended, so no end method ran". Work end was due at
  12:00:43, inside the suspend (11:55:56 to 12:04:28); the tool saw the lid open first. The card says this.
- C (`hold-20261103-112124.log:113`): "GAP at t 945 s" names the gap by its end; it started at 379.6 s.

None changes a conclusion: each card reads the events, not only the RESULT line. Fix: none for throwaway spikes. If
Phase 3 reuses one, judge standby by `PBT_APMSUSPEND`, the S-state by the System log, and label a gap by its start.

**m2. `powercfg /requests` still lists a request that Windows has ended.** In H run 2, after the resume at 10:58:25,
the requests log lists the lid spike's SYSTEM and EXECUTION entries at 10:58:45 and 10:59:15 (`requests.log:1360`,
`:1381`), while `SystemExecutionState` read 0x0 in all 61 hold ticks and the PC suspended at 10:59:25. In I,
`SystemExecutionState` read 0x1 up to the last tick before the battery-rule suspend. So neither signal warns that a
request is about to be ignored. Fix: add both to `research.md` ("Resume" and "Battery rule"). Design effect: re-take
always and raise the battery timer; do not try to detect the loss.

**m3. `SetSuspendState` blocks until resume.** It returned after 73842 ms and 78438 ms
(`suspend-20261103-103649.log:47`, `-103853.log:47`), that is after the resume. `architecture.md:35-36` lists the calls
that must run off the loop thread; this one is missing. Fix: edit in section 1, item 2.

**m4. The re-take's reason names another program's bug.** `product.md:59` cites PowerToys Awake. Under decision 3 a
reader could drop the re-take as a guard for someone else's bug. The real reason is Windows: the PowerSetRequest docs
say requests "are terminated upon user-initiated system sleep entry" (`research.md:36-38`), and H run 2 showed it.
Also, only clear then set on the same handle was tested (H4, `resume-20261103-120914.log:63`), not "new before old"
(`00-facts.md:43-44`). Fix: edit in section 1, item 7; Phase 1 uses the tested form or tests the other.

**m5. Short AC drops on the USB-C dock hit the AC and DC rules.** Kernel-Power 105 at 12:27:54 and 12:28:00 (a 6 s
drop, no one touching, power card Log 12:33:48) and at 01:24:33, 01:50:47 and 01:50:52 (power card, night). D shows
that the hold itself survives an unplug and replug with the lid closed. Two rules key on the power source:

- Battery floor (`product.md:66-70`): a drop while the battery is under the floor (a drained laptop just docked)
  releases, restores, and then hibernates the closed laptop after a few minutes with no input, though AC came back
  6 s later. That ends an overnight agent run on the dock.
- A battery timer raised only while on battery: every drop writes and restores, and an unplug after a long idle may
  meet the rule before Wakeshot reacts (not tested).

Fix: write the battery timer with the lid action when a hold starts (section 1, item 1). For the floor, re-check AC
and the floor right before the hibernate, and hold again when AC returns (section 1, item 2).

**m6. `architecture.md` and the power card disagree with the power card's facts.**

- `architecture.md:90-92` gives the reason against a plan copy: "any setting the user changes lands on the copy and
  is lost". Settings wrote battery sleep and battery lid action to the inactive Balanced too (`research.md:438-440`;
  two registry events per change in `watch-20261103-122958.log:43-47` and `:58-63`), and the Control Panel lid page
  says its changes "apply to all of your power plans" (`p0-morning-trim-safety.md:278-282`). The pick (in place)
  stands; half of its reason is wrong.
- `architecture.md:77-78` still waits on the copy choice.
- `p0-spike-power-settings.md:378-380` recommends a registry watch; `architecture.md:39-41` (part 2 item 19, applied)
  says no registry watch. The card adds no fact that a 10 s poll is too slow, so item 19 stands and the card line is
  stale.

Fix: edits in section 1, item 4; a note under the card line pointing to item 19.

**m7. The sign-in choice cannot be read yet.** `product.md:63`: "No lock if 'require sign-in after sleep' is Never".
A5 could not run: Windows Hello greys the choice out (power card Log 12:28). So which value (CONSOLELOCK or
`DelayLockInterval`) means Never is not known. Also not tested: whether Windows' own lock at screen off keeps a delay
of 1 to 15 minutes while a request holds; only "Every Time" was seen. Fix: Phase 1 locks unless a value it reads
proves Never, and tests the mapping on an account without Windows Hello. Phase 3 checks one delay value.

**m8. Part 3 item 32 has nothing left to edit.** Its two old lines ("Docked check" and "Heat guard") are no longer in
`p0-spike-lid-sleep.md`: the Result was rewritten at 12:55 without them (a search finds neither). The new
implications list names screens and temperature only as things that read without admin. Fix: mark item 32 moot in
`p0-close-proposals.md`.

**m9. Input idle counts suspended time; the duration clock does not.** After the I suspend, `GetLastInputInfo` idle
read 1430 s at the first tick (`lid-20261103-114033.log:705`; 514 s of it suspended); after C it read 945 s.
Unbiased time skipped both suspends. `architecture.md:42` says durations use unbiased time, but the "few minutes with
no input" rule (`product.md:68-70`, `75-77`) reads input idle. Harmless with the lid check (I skipped the end method
because the lid was open), but Phase 1 must not compare the two. Fix: section 1, item 5.

**m10. Stale status lines and a thin fact cache.** `plan.md:88-92` (Next still lists done steps); the lid card says
"Status: done" in its header (line 3) but "Status: blocked. Only the AFTER line (old K3) is left" in its Result
(line 265) and "not written yet" in its Checks (line 360), though the AFTER line was written at 13:08. `CLAUDE.md` "Facts to keep in mind" lacks the two
tested facts most likely to be missed: on battery Windows ends requests about 5 minutes after the sleep timeout, and
a user's own sleep ends every request. Fix: the orchestrator updates them at the close.

## 1. Part 3 items: outcome, confidence, exact edit

**Item 1, battery sleep timer.** Outcome: the rule holds; Wakeshot must raise the battery timer. Confidence: high for
the rule (two runs, 60 s and 600 s timeouts, both plus 316 to 320 s; requests log clean). On AC there is no such rule:
H run 1 held with a 60 s AC timeout and idle up to 527 s with no suspend. The cure is untested (M1).

`product.md:60-61`. Old: "- Change only the power settings a test proves necessary: the lid action, and the sleep
timer on battery only if the "requests end 5 minutes after the sleep timeout on battery" rule is confirmed."
New: "- Change only the power settings a test proves necessary: the lid action and the sleep timer on battery. On
battery Windows ends every request about 5 minutes after the sleep timeout, lid open or closed (tested,
`research.md`, "Battery rule"). Wakeshot writes the battery sleep timer together with the lid action when a hold
starts, also on AC, so an unplug or a short AC drop on a USB-C dock never meets that rule first. Value in Phase 1:
Never, or a value Settings cannot set, so that it marks itself (Nightcrew's +7 s, `research.md:165-166`)."
If M1 is not run, append: "Not tested: that the raised timer stops the rule."

**Item 2, hibernate off or failing.** Outcome: `SetSuspendState(FALSE)` and `(TRUE)` both hibernate here (S4,
checked in the System log). E is the key fact for the end of work: at 11:12:52 the spike released its request and
put Balanced back with lid action Sleep while the lid was closed (`lid-20261103-111112.log:143-149`); the PC did not
sleep, and apps ran 263 s past the 506 until the lid opened. So putting the lid action back does nothing for a closed
lid, and a plain release leaves the laptop running until the user's sleep timeout (inferred from the H control
phases; not seen with the lid closed). F3 is not needed: E shows that screen off does not stop apps, so a screen-off
call only darkens the screen sooner. Confidence: high for the hibernate; medium for "stops at the sleep timeout with
the lid closed". Settle, optional, Phase 3 checklist: E again on battery with `--max-minutes 14`, past 600 s idle.

`product.md:75-77`. Old: "- When work ends with the lid closed, the laptop sleeps or hibernates once there has been
no input for a few minutes (value in Phase 1), so someone typing on an external screen with the lid closed is not
put to sleep (method: Phase 0). If the user's own lid action is Do nothing, Wakeshot changed nothing, so it only
releases its hold."
New: "- When work ends with the lid closed, Wakeshot hibernates the laptop once there has been no input for a few
minutes (value in Phase 1), so someone typing on an external screen with the lid closed is not put to sleep. It must
act itself: putting the lid action back does not sleep a laptop whose lid is already closed, and after a release its
apps run until the user's own sleep timeout (tested). Method: `SetSuspendState(TRUE)` after `IsPwrHibernateAllowed`
and enabling SeShutdownPrivilege; on this Modern Standby laptop `SetSuspendState(FALSE)` hibernates too (tested).
Check the lid right before the call. If hibernate is not allowed or the call fails: release every request, restore
settings and notify; Windows then stops the laptop at the user's sleep timeout. If the user's own lid action is Do
nothing, Wakeshot changed nothing, so it only releases its hold."

`product.md:68-70`. Old: "If the lid is closed, it then hibernates the laptop, but only after the same few minutes
with no input as at work end (below), so someone typing on an external screen with the lid closed is not put to
sleep."
New: the same, plus: "Right before it hibernates, it checks again that the lid is closed and the PC is on battery
under the floor: a USB-C dock can drop AC for a few seconds (seen on the test laptop). When AC comes back, the hold
resumes."

`architecture.md:35-36`. Old: "Calls that can block (file reads, WinRT) run off the loop thread. Phase 1 lists
which."
New: "Calls that can block (file reads, WinRT, and `SetSuspendState`, which returns only after resume) run off the
loop thread. Phase 1 lists which."

**Item 3, idle lock.** Outcome: Windows locks within 1 s of every display-off standby entry, with or without a
request (H run 1 three times, H4, E, the H control phases; LogonUI at 12:33:48 in the power card). With lid action
Do nothing it does not lock at the close (E: 176 s unlocked). Seen only with "Every Time" (Windows Hello), CONSOLELOCK
1/1, `DelayLockInterval` 0. Confidence: high for this setup; medium for other sign-in delays (m7).

`product.md:62-64`. Old: "- **Never lower security.** Whenever Windows would have slept (lid closed, or idle past the
user's own sleep timeout), Wakeshot locks the PC instead. No lock if "require sign-in after sleep" is Never. No lock
on lid close in docked use (the user's own lid action is Do nothing)."
New: "- **Never lower security.** When the lid closes while Wakeshot holds, Wakeshot locks the PC: Windows would have
slept, and with the lid action Do nothing it does not lock by itself (tested). When the screen turns off for idle,
Windows locks by itself, also while a request is held (tested with sign-in "Every Time"), so Wakeshot adds no idle
lock. Only with Keep screen on, where the screen never turns off, Wakeshot locks when idle passes the user's own
sleep timeout. No lock if "require sign-in after sleep" is Never. No lock on lid close in docked use (the user's own
lid action is Do nothing)."

**Item 4, plan copy or in place.** Outcome: in place stays. Confidence: medium. Reasons in section 5.

`architecture.md:77-78`. Old: "- A second copy of the original values lives outside the data folder (an HKCU key, as
Nightcrew does), unless the plan copy is picked after the power settings spike (it marks itself)."
New: "- A second copy of the original values lives outside the data folder (an HKCU key, as Nightcrew does). It is
needed: Wakeshot edits the plan in place (Power settings ownership below)."

`architecture.md:90-92`. Old: "Lid strategy: edit the active plan in place, not a temporary copy of the plan, unless
the Phase 0 test shows a reason. Why: while a temporary plan is active, any setting the user changes lands on the copy
and is lost when we switch back."
New: "Lid strategy: edit the active plan in place, not a temporary copy (checked in the power settings spike). Why: a
copy does not keep Windows off the user's plan (Settings writes battery sleep and battery lid action to the inactive
plan too, and the Control Panel lid page writes every plan), while other edits made during a hold, such as the energy
saver threshold, land on the copy only and are lost when it is deleted. A copy also switches the plan at every hold.
Its one gain, that it marks itself, the journal plus the HKCU copy cover."

Morning list: drop B5 (restart with a copy active). It has no consumer once in place is fixed.

**Item 5, clock across standby.** Outcome: unbiased time skipped every suspend (wall 40 to 571 s against unbiased
0.15 to 4.9 s; the two hibernates 7.4 and 7.8 s). In screen-off standby, where apps run, both clocks move together.
`architecture.md:42` stands. Confidence: high.

`architecture.md:42-43`. Old: "- Durations use a clock that ignores sleep time. Suspend and resume are handled so a
late timer never hibernates the PC in front of the user."
New: "- Durations use a clock that ignores sleep time: `QueryUnbiasedInterruptTime` skips suspended time, and
screen-off standby, where apps still run, counts as awake (tested). Idle time from `GetLastInputInfo` includes
suspended time, so never compare the two. Suspend and resume are handled so a late timer never hibernates the PC in
front of the user."

Phase 1 also picks the clock per duration: "Keep awake 1 hour" reads as wall time to a user, and with unbiased time a
manual sleep in the middle moves its end.

**Item 6, sound at lid close.** Outcome: not shown to work (M2). Edit after Haik's pick. With my pick, `product.md:65`.
Old: "- Short sound when the lid closes while Wakeshot is holding."
New: "- No sound at lid close: in the Phase 0 tests it was not heard, and the default audio device can be a monitor
with no speakers or headphones. When the user signs in after a closed-lid hold, a notification says how long the PC
kept running with the lid closed and what held it."

**Item 7, lock at lid close and re-take after resume.** Outcome: as the card says. `LockWorkStation` at the close
works (I, locked within 1.06 s). Windows does not lock at a Do-nothing close (E). Without a re-take the request was
not honored after Start > Sleep (H run 2: suspend 17 ms after the next display off, `SystemExecutionState` 0x0); with
one it was (H4). Confidence: high for the lock (one run, plain result); medium for the re-take (one run each, AC
only), backed by Microsoft's text. The rule stays; only its reason changes (m4).

`product.md:59`. Old: "- Take the request again after every resume (PowerToys Awake lost its hold after a manual
sleep and resume)."
New: "- Take the request again after every resume. Windows ends every request when the user sleeps the PC (Start >
Sleep, power button), and it is not honored after resume (PowerSetRequest docs; tested: without a re-take the PC slept
at the next screen off, with one it stayed awake)."

**Item 8, letting the screen turn off.** Outcome: confirmed. Work did not pause with the screen off or the lid closed
while a request held (D, E, I up to 912 s, H run 1, H4); the 1 s timer fired about every 1.5 s with the screen off.
Pauses came only from the battery rule and the lost request after a manual sleep. No edit. Confidence: high.

**Item 32, lid spike design notes.** Moot (m8).

## 2. Cards against the raw logs

Checked and matching: D (close 10:31:10.189, reaction 561 ms, internal path 1 to 0 at the close, ext stayed 1 after
the cable came out, only two Kernel-Power 105, no gap, heat 40 to 52 C); E (cable out 11:11:47.5, close 11:11:51.8,
release and restore 11:12:52, dim 161 s and off 176 s after the close, 506 at 11:14:48, idle 453 s at the open, no
gap, heat 39 to 60 C); F (both TargetState 5, HiberWriteDuration 7950 and 8156 ms, clock jumps 59 and 64 s); H run 1
(display off three times, locked within 1 s each, no suspend); H run 2 (exec-state 0x0 in all 61 hold ticks, gap
319 s); H4 (retake at 12:10:16, exec-state 0x1 in all 185 hold ticks, no hold gap); I (close 11:40:43.449, reaction
370 ms, lock Ok, display off 31.4 s after the close, suspend at idle 914 s plus one tick, gap 514 s, heat 35 to 53 C);
C (suspend at 379.6 s, wall 571 s against unbiased 4.9 s). In all eight lid logs the 13 `after` lines equal the 13
`before` lines (my own compare); no journal in either `out` folder. The requests log has no entry outside the spikes
other than `[DRIVER] Legacy Kernel Caller` (PERFBOOST after resumes, SYSTEM twice at 12:15:09).

Tool misreads: m1. Extra facts the cards do not state: m2, and the E restore with the lid closed (section 1, item 2).
Also not in the card: Kernel-Power 40 ("The driver \Driver\ACPI ... stopped the power transition") before both F
hibernates; both still completed. Meaning not checked.

## 3. Results against applied part 2 edits and fixed decisions

- Haik's decisions 1 to 3: no result contradicts them. Closed on a desk the zone read 35 to 60 C; the one 87 C was
  right after a hibernate resume with the lid open.
- "Never lower security" (fixed): the proposed item 3 edit would change it for Keep screen on with no fact (M3).
- `product.md:12-13` ("a notification or a sound explains every action"): with no working sound, the lid-close
  action needs another explanation (M2).
- Part 2 item 19 (no registry watch): the power card recommends one, with no new fact. Item 19 stands (m6).
- Part 2 item 13 (loop thread): holds, but its list lacks `SetSuspendState` (m3).
- Part 2 item 34 (battery floor waits for no input): holds, but needs an AC re-check (m5).
- Part 2 item 12 (HKCU copy unless the plan copy): settled by item 4: needed.
- `product.md:59` (re-take): the rule holds; its reason should be the Windows fact (m4).

## 4. Lid-close cue

The cue must say "this laptop keeps running" at a moment when the lid is closed. With the lid closed only sound can
reach the user, and sound is not reliable: the default device may be a speakerless monitor (D), headphones, or muted,
and the one clean run (I) was silent.

| Option | Reaches the user | Cost |
|---|---|---|
| No sound; a notification at the next sign-in after a closed-lid hold (my pick) | Yes, after the fact | One toast, which Phase 5 builds anyway |
| Keep a sound: our own WAV as a resource, no `SND_SYSTEM`, on the default device | Not shown; same device problem | Small; needs the tray card's by-ear result and a 2 minute retest |
| Nothing at all | No | None, but breaks `product.md:12-13` |

No Phase 0 test shows any sound being heard: the tray card did not check its two sounds by ear either (T5
dropped). A registered toast shows a banner and an unregistered one does not (tray card, item 4), so the sign-in
notification needs the HKCU app id the tray card describes.

My pick: no sound at the close. Before the close the tray already says what holds (`product.md:114`), and the lock at
the close shows the user on reopening that Wakeshot acted. The notification at sign-in explains the action, which
meets `product.md:12-13`. Not verified: whether a toast raised while locked shows after sign-in; raising it at sign-in
(session unlock) avoids the question. Change it when: users ask for a cue at the close itself.

## 5. Settings writes the user's plan during a copy: plan copy or in place, and restore

Fact: with a copy active, Settings wrote battery sleep and battery lid action to the copy and to the inactive
Balanced (`research.md:438-440`). The energy saver threshold went to the copy only. The Control Panel lid page writes
every plan by its own text. Not tested: AC values, a custom (non-Balanced) user plan, and whether Settings writes every
plan or only Balanced.

For the copy-or-in-place choice:

- It removes half of the stated cost of the copy (`architecture.md:90-92`): for the two values Wakeshot overrides, a
  user's change in Windows' own pages reaches the user's plan anyway. Wakeshot still sees it, because the copy is
  written too.
- It does not make the copy cheap. Other edits during a hold (energy saver threshold, display timeout, the advanced
  settings dialog) land on the copy only. Rule 3 ("the user's latest change wins") then needs a copy-back of every
  changed value before the delete, which needs a record of what Wakeshot wrote: a journal again. Add a plan switch per
  hold, a visible extra plan in Control Panel, and an untested restart with a copy active (B5).
- So no test shows a reason for the copy, and the rule at `architecture.md:90` keeps in place. The HKCU second copy
  (part 2 item 12) is then needed.

For restore:

- In place: no change. A user's change in Settings lands on the active plan, where rule 2 keeps it; other plans are
  never Wakeshot's to touch.
- One known gap gets sharper: a user who picks exactly Wakeshot's value cannot be told apart, and rule 2 would then
  restore the old value over the user's choice. Never on battery is a likely user choice. A battery timer value that
  Settings cannot set (item 1) avoids this for that setting; the lid action has no such value.
- For Phase 3 tests: a change made in Settings or the Control Panel lid page during a test with a copy is a real
  write to the user's plan. The morning steps already learned this; keep it in the Phase 3 checklist.

## 6. Untested items that matter for Phase 1

| Item | Matters for | Blocks the close? |
|---|---|---|
| A raised battery timer stops the battery rule (M1) | every hold on battery | Yes, unless Haik moves it to Phase 3 (16 minutes) |
| Screen-off end method (F3) | nothing after item 2 | No: not needed |
| Hibernate off | the fallback in item 2 | No: fallback defined; cannot be tested here by rule |
| Closed lid, no request, past the sleep timeout | how long the fallback leaves the PC running | No: Phase 3 checklist |
| Short AC drops on the USB-C dock | battery floor, battery timer | No: design input (m5); D showed the hold survives |
| Unplug after a long idle with a request held | only if the timer is written on battery only | No: moot with the item 1 edit |
| Sign-in mapping (A5) and other lock delays | the lock exceptions | No: safe default (m7) |
| Restart with a copy active (B5) | nothing once in place | No: drop B5 |
| Re-take on battery; "new before old" | resume | No: Phase 3 |
| Settings with a custom user plan | in place | No |
| Sound by ear (tray card) | item 6 only if a sound stays | No, with my pick |
| Panel state with the lid closed and no lock (sign-in Never) | battery, heat | No: Windows reports the display on until the display timeout (E); Phase 1 may turn it off at the close |

## Before Haik signs off Phase 0

1. Haik: pick the lid-close cue (M2). Default: no sound, a notification at the next sign-in.
2. Haik: run the 16 minute battery check (M1), or say "Phase 3". Default: run it now, one cable pull.
3. Haik: confirm that Keep screen on keeps its idle lock (M3), or say it should not lock. Default: keep it.
4. Orchestrator: apply section 1 (items 1, 2, 3, 4, 5, 7; item 6 after the pick), mark item 32 moot, drop B5, add m2
   to `research.md`, fix the stale lines (m10).
5. Optional: a full review of the tray card's Result (done at 13:08), which this review covered only for sound and
   notifications. The lid card's AFTER line is already written (13:08).
