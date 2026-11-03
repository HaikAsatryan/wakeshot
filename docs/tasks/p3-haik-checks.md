# Haik's checks for the power track

Phase: 3. Status: ready. Needs Haik: **yes**: about 60 minutes elapsed, about 15 minutes of his hands (lid, charger,
sign-out, Task Manager, lock); the two battery runs need the laptop unplugged for about 20 minutes each. Real laptop:
**yes**: battery, lid, sign-out, lock, a plan copy made active. **Runs alone on the whole laptop** (cross-track
review B2): no agent of any track and no cargo command runs. A sign-out ends every process of Haik's session (the
orchestrator, every worker, their builds), Start > Sleep stops them mid-call, and another worker's test that takes a
power request would skew the battery timings of D, E and F. The orchestrator records its state and stops spawning
before step B and step D; after sign-in Haik starts the orchestrator again (`/orchestrate`).
Agent: `wakeshot-worker`. Size: about 60K tokens.
Runs after: `p3-real-checks`. Parallel with: nothing.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-haik-checks\target`.

## Goal

Run the checks from architecture section 9 that need Haik and can run before integration, with the real tray exe's
guard and restore modes and the `power_check` harness.

## Read first

- `CLAUDE.md` ("Test laptop"); `docs/phases/03-windows-power.md` ("Manual checklist").
- `docs/architecture.md`: section 9 checklist 1 to 6 and 11, and its plan copy rule; section 5 "Power requests",
  "Guard", "Logon restore"; section 4 "Tray threads" (end of session).
- `docs/research.md`: "Lid and sleep spike" (block I, H run 2), "Battery sleep Never stops the battery rule".
- `docs/phases/00-morning-steps.md`: the form of steps Haik already used.
- `docs/tasks/p3-harness.md` and `p3-real-checks.md` Results.

## Deliverables

1. Exact numbered steps for Haik, written into this card under "Steps for Haik", grouped so he does each physical
   action once, with what he should see and how long to wait. Then Status `blocked`; the orchestrator relays them.
   Checks to cover (the order and grouping are the worker's call):
   - A. Checklist 4: `fake-tray` running with its guard; Haik ends it in Task Manager with End task, then a second run
     ended with End task on its group. The guard must live on and restore the copy. Also: how Task Manager groups the
     guard (architecture "not verified").
   - B. Checklists 5 and 6 in one sign-out: one `fake-tray` ended by the agent (so only RunOnce can restore, in the
     real data folder, through `wakeshot-tray.exe --restore`), one `end-session` alive at sign-out. After sign-in: the
     first restored by RunOnce, the second before the session ended, with times.
   - C. Lock and session notices: `session-watch` running; Haik presses Win+L and signs in again; the agent runs
     `real_lock_locks` once and Haik signs in again.
   - D. Checklist 3: `hold` on battery; Haik does Start, Sleep, wakes the PC, then leaves it with no input; the tick log
     must show no suspend at the next screen off after resume (with no re-take it slept there in H run 2).
   - E. Checklist 1: `hold --active-copy` on battery with the lid closed and no input past 916 s (about 20 minutes).
   - F. Optional, checklist 2: `hold --no-request` on battery with the lid closed and Haik's own 600 s: how long it
     runs.
2. After Haik's steps: read the logs, record each outcome, restore and check Haik's values (active plan; lid AC and
   DC; sleep AC and DC; energy saver DC) against the values recorded before the first step.

## Acceptance checks

- Haik's values before and after are equal; no "Wakeshot test" plan, no journal, no `WakeshotRestore` RunOnce value
  left.
- Each check A to E (F if run) has a pass or fail with its log lines, recorded in the Result and in
  `docs/research.md` ("Power track manual checks"), with "not verified" for anything a step could not show.
- A failed check that changes the design goes under "Found outside scope" with the section it affects.

## Out of scope

- Checklist 7, 8, 9, 10, 16 and the second-session part of 11 (integration); 12 (later).
- Fixing code.

## Questions for Haik

Each with the worker's default.

- Run D and E now or at integration: answered, now in Phase 3 (the default; `docs/phases/02-06-signoff.md` item 10,
  Haik 2026-11-03 18:09), because a failure changes the battery design.

## Steps for Haik

Written by the worker of this card once the harness exists.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:43, orchestrator: Haik's own battery sleep is now 4 h (DC 14400, read 20:41), not 600 s. Step F as
  written ("Haik's own 600 s") would wait over 4 h: run it on an active plan copy with DC sleep 600 s, or skip it and
  say so in the Result. Read his values live before and after; never assume 600.

## Result

What was done, files changed, checks and outcomes, open items.
