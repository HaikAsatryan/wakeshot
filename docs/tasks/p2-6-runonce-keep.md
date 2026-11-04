# Keep the RunOnce value while a hold owns entries, then redo B2

Phase: 2 to 6 (power track fix). Status: blocked: waiting for Haik (fix round done; Part B prep done, B2 redo
steps ready in "Part B: B2 redo"). Needs Haik:
Part B only: about 5 minutes of his hands, one sign-out, after the review of Part A. Agent: `wakeshot-worker`. Size: about 120K tokens (Part A about 80K, Part B about 40K).
Written by the orchestrator (session after `wakeshot-27`, 2026-11-04 18:32) from `docs/tasks/p3-haik-checks.md`
"Found outside scope" item 2 and Result "Open items" 1 and 2.
Runs after: `p3-haik-checks` (done, 90794e2). Parallel with: Part A with cards that own none of the files below (none
run now). Part B runs alone on the whole laptop: no other agent and no cargo command (its sign-out ends every agent).
Real laptop: Part A no; Part B yes (a plan copy made active, one sign-out).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-runonce-keep\target`.

## Goal

Explorer runs HKCU RunOnce once per sign-in, after every Run entry (39 to 65 s after sign-in on this laptop). A
RunOnce value the tray sets before that pass is run at once: Windows deletes it, restore mode finds the owner alive and
exits, and nothing sets the value again for the rest of that hold. A crash, a power cut or End task on tray and guard
then leaves nothing to restore at the next sign-in when start at login is off. Close that gap, then show checklist 5
(restore at the next sign-in) on the fixed code, which B2 of `p3-haik-checks` did not show.

## Read first

- `docs/tasks/p3-haik-checks.md`: "Found outside scope" item 2 (cause, verified with log lines; the proposed fix and
  the other option), Result "Open items" 2 (the B2 changes), Log lines of 2026-11-04 18:19 onward.
- `docs/research.md` "Power track manual checks" (the RunOnce pass timings).
- `crates/wakeshot-windows/src/power/executor.rs` `keep_guard` (about line 243);
  `crates/wakeshot-windows/src/power/journal.rs` `set_run_once` (about line 536) and the "only the tray sets RunOnce"
  doc (about line 353); `crates/wakeshot-windows/src/power/recovery.rs` restore mode's owner-alive exit (about line
  756).
- `docs/architecture.md` section 5 ("Logon restore", "Gap left") and section 4 "Tray startup" step 6.

## Deliverables

Part A (code, no Haik):

1. The fix, worker's pick from the card above unless the code shows a reason against it: in `keep_guard`, when the
   journal holds a known entry this tray owns, call `set_run_once` next to `ensure_guard`; log a failure like a guard
   start failure and let the command run. The value comes back within one ReadPlan (10 s in a hold).
2. Tests: a unit test that a missing value is set again on the next ReadPlan while an entry is owned, and is not set
   when nothing is owned or another owner's journal is read. Check `stop_points` and the core props models: if the new
   write is a new stop point, cover it (a stop there must still leave Haik's values right).
3. `docs/architecture.md` section 5: the "Gap left" line for this case replaced by what the code now does. Fix any
   other doc line this makes wrong, in the same edit.
4. Stop after Part A: Status `blocked: waiting for review`. The orchestrator runs a narrow power safety check of the
   change. Fix its findings, then Part B.

Part B (Haik, after the review):

5. B2 redo on the harness in `<S2>` of `p3-haik-checks` (`<S2>\scripts`, `<S2>\make-cmd.py` were kept; rebuild into
   `<S2>\target` on today's code). Changes to B2's steps: start `B2-fake-tray.cmd` at least 2 minutes after sign-in;
   wait 2 minutes after the next sign-in before `0-check.cmd`; make `kill.ps1`'s `NOT READY` and `STOP` impossible to
   miss (Haik read B as all OK at 18:15). Write Haik's steps into this card, worker prep as in `p3-haik-checks`
   ("Before Haik starts", only what B2 needs), then block for Haik.
6. After his steps: read the logs as `p3-haik-checks` "After Haik's steps" for B2; cleanup; Haik's values against the
   baseline; delete the harness folders (the empty `%LocalAppData%\Wakeshot-harness\haik` too) and `<S2>\target`;
   add the outcome to `docs/research.md` "Power track manual checks".

## Acceptance checks

- `cargo fmt` on own files; `cargo clippy -p wakeshot-windows --all-targets --locked -- -D warnings` and without
  `--all-targets`; `cargo test -p wakeshot-windows --locked`; `cargo test -p wakeshot-core --locked` if a model
  changed. All pass.
- B2 redo: after the sign-in, the restore process's lines show the copy's entries written back, journal deleted,
  RunOnce removed; `0-check.cmd` `check: ALL OK`. Haik's values at the end equal the baseline.

## Out of scope

- Any other power behavior; the contracts (`SafetyNet` already has `set_run_once`); the tray or the core policy.
- The other option (restore mode sets the value again): only if the code shows the pick cannot work; say why.
- The real tray's Task Manager grouping (integration).

## Questions for Haik

None. The fix is the orchestrator's decision (2026-11-04), for Haik's review. So is the fix round's finding 6 (the
RunOnce value also for another owner's leftover; Decisions 9).

## Part B: B2 redo

Written 2026-11-04 19:08 by the worker of the fix round. `<S2>` is
`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\7c3512fd-d0b5-4439-a2c6-4dd5cf1b6c37\scratchpad\p3-haik-checks`
(build in `target`, scripts in `scripts`, `make-cmd.py`). The step files are in `%LocalAppData%\Wakeshot-harness\haik`;
the scripts' own records in `%LocalAppData%\Wakeshot-harness\haik-out`; the fake tray's, guard's and restore mode's log
in `%LocalAppData%\Wakeshot\logs\wakeshot.log`.

### Before Haik starts (orchestrator)

1. No other agent runs and no cargo command runs until Haik is done: the sign-out ends every agent and every Claude
   Code session. Write the state to files.
2. Give Haik the steps below and tell him "go".

### Worker prep (done 2026-11-04 19:08, see Log)

Builds of today's code (with the fix round) in `<S2>\target`, Haik's values saved as the baseline, the six step files
written and smoke-tested. Nothing runs in the background; no setting was changed.

### Steps for Haik

**Your time: about 6 minutes, one sign-out.**

Before you start:

1. The sign-out in step 4 ends every Claude Code session on this laptop, the orchestrator too. Finish or close every
   other Claude Code session first, and save your work in other apps.
2. Lid open; charger or dock, either is fine. Change no power setting by hand until step 5 is done.

Steps:

1. Win+R, type `%LocalAppData%\Wakeshot-harness\haik`, Enter. A folder opens.
2. Double-click `B2-1-fake-tray.cmd`. A window opens. If you signed in less than 2 minutes ago, it counts down
   (`please wait: ... s left`); let it. Then wait for a line that starts with `holding:`. One of the first lines ends
   with `integrity Medium (0x2000), elevation type Limited`; if it says High, close the window and tell the
   orchestrator. Leave this window open.
3. Double-click `B2-2-kill-tray-and-guard.cmd`. After about 3 seconds a dialog box opens on top of everything and the
   window shows a large green or red banner.
   - Green, `READY: SIGN OUT NOW`: click OK, go on with step 4.
   - Red, `STOP: DO NOT SIGN OUT` or `NOT READY: DO NOT SIGN OUT`: click OK and **do not sign out**. Close both B2
     windows, double-click `0-cleanup.cmd` (it must end with `check: ALL OK`), and tell the orchestrator what the red
     banner says.
4. Start > your picture > Sign out. If Windows lists apps that keep it from signing out, click "Sign out anyway". Sign
   in again.
5. Win+R, `%LocalAppData%\Wakeshot-harness\haik`, Enter. Double-click `B2-3-check-after-sign-in.cmd`. It counts down
   until 2 minutes after your sign-in, then checks. It must end with `check: ALL OK` (a line `test copy left, not
   active` is fine). If it ends with `check: NOT OK`, close it and do **not** run `0-cleanup.cmd`: the worker reads
   what is left first. The test writes only to a copy of your plan that is never active, so your own values stay.
6. Start Claude Code and the orchestrator (`/orchestrate`), and tell it "B2 done", with what steps 3 and 5 showed.

### After Haik's steps (a new worker: deliverable 6)

1. Before any change, copy to the worker's scratchpad: `%LocalAppData%\Wakeshot-harness\haik-out\` (`baseline.json`,
   `check.log`, `B2-kill.txt`, `wait-signin.txt`), `%LocalAppData%\Wakeshot\` (journal if left, `logs\wakeshot.log`),
   the System log (Winlogon 7001 and 7002), Shell-Core Operational 9705 to 9708 (the RunOnce passes), RunOnce and Run
   values, `powercfg /list`.
2. Pass or fail, with log lines: `B2-kill.txt` READY with the RunOnce command `"<S2>\target\debug\wakeshot-tray.exe"
   --restore`; after the sign-in (7001), Shell-Core 9707 runs `wakeshot-tray.exe" --restore`; restore mode's lines:
   the owner gone, the copy's three entries written back (lid AC 0 to 1, lid DC 0 to 1, sleep DC 0 to 18000),
   `journal deleted`, `RunOnce value WakeshotRestore removed`; `check.log` from step 5 `check: ALL OK`. Note whether the
   fake tray logged `RunOnce value was missing ... set again` (not expected: it starts after its sign-in's pass).
3. `power_check cleanup` (deletes the copy), then `check.ps1`: Haik's values against the baseline; put back any
   difference and read again. No "Wakeshot test" plan, no journal, no `WakeshotRestore` value.
4. Delete `%LocalAppData%\Wakeshot-harness` (all of it, the `haik` folder too), `%LocalAppData%\Wakeshot` and
   `<S2>\target`; `powercfg /qh` once more.
5. `docs/research.md` "Power track manual checks": the B2 outcome. This card's Result.

## Decisions

Worker's, Part A (2026-11-04):

| # | Decision | Reason | Change it when |
|---|---|---|---|
| 1 | The card's pick: `keep_guard` calls `set_run_once`, then `ensure_guard`, only when the journal reads and names this tray with a known entry | Apply step 3's order, one rule for both halves of the net. `FileNet::set_run_once` reads first and writes only a missing or wrong value, so a present value costs one registry read per command; the value is back within one plan read (10 s while anything is owned). Restore mode stays a pure reader of the key (`journal.rs` `RunOnce::ClearOnly`) | Something deletes the value more often than once per sign-in, or the read shows up in a profile |
| 2 | The other option (restore mode sets the value again) not built | The pick works (tests below); the card allows the other only if the pick cannot | Never, unless the pick fails in Part B |
| 3 | An Info line only when the value was missing (`Ok(true)`); `FileNet` also logs the write | Shows in a log when Windows (or anyone) took the value during a hold, which Part B reads; no line per plan read when it is there | Logs show it more than once per sign-in |
| 4 | A failed write: a Warn line per command, no notice, no throttle; the guard start and the command still run | The same as a failed guard start in `keep_guard`; the guard still covers a tray that dies alone | A field log shows a write that keeps failing (then log once until it works) |
| 5 | The name `keep_guard` kept | `docs/phases/03-windows-power.md` and the reviews name it; its doc now says it keeps both halves | The executor is next reworked: rename to `keep_safety_net` with those references |
| 6 | No change to `stop_points.rs`, `props.rs` or `fake.rs` | Both walk only the locked routines (apply, restore, check), whose starts already have RunOnce set while a known entry exists (`stop_points.rs:381`, `Prop::RunOnce`). `keep_guard` runs before the lock and writes no plan value and no journal; a stop there leaves a start those walks already cover. Its stops are walked in the executor instead (`plan_read_stop_points`) | `keep_guard` ever writes a plan value or the journal |

Worker's, fix round of the review (2026-11-04):

| # | Decision | Reason | Change it when |
|---|---|---|---|
| 7 | The dry-run line once per `FileNet` (a `dry_run_logged` bool, like `unknown_logged`) | The orchestrator's call on finding 1; one line tells a dry run has no value, a line every 10 s buries the log | Never, unless a dry run gets a RunOnce value |
| 8 | Finding 2, the lean fix: a journal that does not read still gets no value; "Gap left" names the case and the stronger fix | The orchestrator's call; such reads fail rarely and briefly, and setting a value for a file not read guesses at what it holds | A log shows journal reads failing for longer than one plan read: then keep the last read's answer and, on a failed read after one with a known entry, still set the value |
| 9 | Finding 6: the value for any journal that reads and holds a known entry, whatever its owner; the guard only for this tray's own journal. The orchestrator's decision, for Haik's review | Core `props.rs` `Prop::RunOnce` already asks for a value whenever the journal holds a known entry, whoever owns it; restore mode checks the owner (alive or Unknown: it exits; Gone: it restores), so the value is right for every owner. A guard waits on this tray, so for another owner it would never act | Two trays of one user run at once with different exe paths (two sessions): they would each rewrite the other's value every 10 s; then set the value only when it is missing |
| 10 | The Info line now reads "RunOnce value was missing while journal.json holds entries to restore" | It is logged for another owner's leftover too, so "this tray owns" would be wrong | The wording of the log is next reviewed |

Worker's, Part B prep (2026-11-04):

| # | Decision | Reason | Change it when |
|---|---|---|---|
| 11 | Only B2's step files (plus `0-check.cmd`, `0-cleanup.cmd`, `_run.cmd`), numbered `B2-1`, `B2-2`, `B2-3` | The folder shows the steps in their order; no file of A to E or B1 to pick by mistake | Another check needs the harness again |
| 12 | The two 2-minute waits are in the step files (`wait-signin.ps1`), not only in the text: it waits until 120 s after the shell Explorer's start and until Shell-Core 9706 (RunOnce) from that Explorer is logged; after 10 minutes without it, by the time alone | The card's timing rule, kept by the script so a fast hand cannot break it. Interactive users may read that log (channel SDDL: `IU` 0x3) | Shell-Core stops logging 9706 for RunOnce (then the time rule alone holds) |
| 13 | `kill.ps1`: every outcome recolours the window, prints a banner of 7 lines and opens a dialog on top of every window (`WScript.Shell` `Popup`, system modal) that must be clicked; green READY, red STOP and NOT READY | Haik read a one-line NOT READY as OK in the first B2 | Never for this harness |
| 14 | `check: NOT OK` after the sign-in: Haik does not run `0-cleanup.cmd` | Keeps the evidence for the worker; the fake tray writes only to a copy that is never active, so Haik's values are not at risk | Never for this harness |
| 15 | No `requests-log.ps1` and no C2 test build | B2 needs neither | Never for this redo |

## Found outside scope

Problems seen but not fixed.

1. **A dry-run hold now logs "dry run: RunOnce value not set" at every plan read.** `FileNet::dry_run`'s `set_run_once`
   logs that line on every call (`power/journal.rs`, the `RunOnce::Off` branch), and `keep_guard` now calls it every
   10 s while a dry-run journal is owned: about 25 KB an hour (the log rolls at 1 MB). Fix: log it once per net (a
   field, one line). `journal.rs` code is outside this round's files. Verified by reading the code; not run. Fixed in
   the fix round (review finding 1).
2. **`docs/architecture.md` section 8, row "Journal load, lock or write, RunOnce value or guard start fails"** reads as
   if every such failure gives a notice. It is the apply's row; the failures in `keep_guard` (the guard start since
   `p3-power-io`, the RunOnce write now) only log. Not changed: section 8 is outside this round's lines. Suggest "(in
   an apply; before each command they only log)". Fixed in the fix round (review finding 5).
3. **The fact under Log 2026-11-04 (Explorer's pass takes 17 ms) belongs in `docs/research.md` "Power track manual
   checks"**, which is not in this round's files; Part B adds it with the B2 outcome. Added in the fix round.
4. `power/executor.rs:128`, a doc line from `p3-power-io`, is 121 characters (one over the wrap). Harmless.
5. **`README.md`'s gap list lags architecture's "Gap left"** (review finding 3): it does not list the up to 10 s after
   a sign-in's RunOnce pass, a value that cannot be written back, or a journal that does not read. Not changed: the
   orchestrator skipped it this round, as `README.md` holds someone else's uncommitted edits. Open.

## Log

- 2026-11-04 18:32: card written by the orchestrator.
- 2026-11-04, worker: Part A started (deliverables 1 to 4 only; Part B after the review, in a later round).
- 2026-11-04, worker: fix in `keep_guard` (`set_run_once` before `ensure_guard`, Apply step 3's order; Info line when
  the value was missing, Warn on failure, command still runs). Executor tests: call indices moved by one where a
  journal is owned; three new tests. `stop_points` and core props walk only the locked routines, which this does not
  touch: no change there (see Decisions). Executor tests 21/21 pass; with the call disabled, the three new tests and
  four old ones fail (mutation check), then restored.
- 2026-11-04, worker: fact (verified, B2's event export `<scratchpad>\p3-haik-checks\evidence\shell-core.txt` lines
  217 to 220): Explorer's HKCU RunOnce pass of B1's sign-in started 18:15:18.653 and finished 18:15:18.670, with 9708
  "Finished execution" of `wakeshot-tray.exe" --restore` at 18:15:18.670, before restore mode's own owner line at
  18:15:18.683. So Explorer does not wait for the command to end, and the pass is long over when the tray sets the value
  again (its next plan read, up to 10 s later): the value is not run twice in one pass. Whether Explorer would pick up a
  value written inside those 17 ms: not verified (the plan read cadence makes it unlikely, and restore mode would only
  find the owner alive again).
- 2026-11-04, worker: `docs/architecture.md` section 5 "Logon restore" (what the tray now does; "Not tested (Phase 3)"
  replaced by what `p3-haik-checks` tested and what is left for checklist 5) and "Gap left" (the up to 10 s window and
  a value that cannot be written back). Full package checks pass; build folder deleted. Status set to blocked: waiting
  for review.
- 2026-11-04 18:55, worker: fix round started on `docs/reviews/p2-6-runonce-keep.md` (0 blockers, 0 majors, 6
  minors), with the orchestrator's calls: fix 1 (dry-run line once per net), 2 (lean: Gap left line and an
  `Unreadable` start in a test; the stronger fix named with its trigger), 4 and 5 (architecture wording), 6 (set the
  value for any readable journal with a known entry, whatever its owner; the guard only for this tray's own journal:
  the orchestrator's decision, for Haik's review); skip 3 (`README.md` has someone else's uncommitted edits). Then
  Part B prep.
- 2026-11-04 19:01, worker: findings 1, 2, 4, 5, 6 fixed. Finding 6 is the orchestrator's decision, for Haik's
  review: the code showed no reason against it (core `props.rs` `Prop::RunOnce` already asks for a value whenever the
  journal holds a known entry, whatever its owner; restore mode checks the owner before it restores). Package checks
  pass (570 passed, 0 failed, 23 ignored). Mutation checks: the old owner rule, a guard for any owner, an unreadable
  read counted as known, and the dry-run line on every call each fail at least one test; files restored after.
- 2026-11-04 19:02, worker: `docs/research.md` "Power track manual checks": the 17 ms RunOnce pass added (checked
  again against `shell-core.txt` lines 217 to 220 and the restore mode's log line at 18:15:18.683). Fix round build
  folder deleted.
- 2026-11-04 19:06, worker (Part B prep): `powercfg /requests` None for all six types (19:03 and 19:06:41). Builds into
  `<S2>\target` on the code after the fix round (host x64, debug, no `RUSTFLAGS`), 19:04:44 to 19:06:13, both exit 0:
  `power_check.exe` (`--features test-support --example power_check`) and `wakeshot-tray.exe`. Baseline
  (`check.ps1 -Save`, 19:06:41): Balanced `381b4222-f694-41f0-9685-ff5bb260df2e` active and the only plan, lid AC 1
  and DC 1, sleep AC 18000 and DC 18000, energy saver DC 30; no journal; no `WakeshotRestore`. `HKCU\...\Run`: the
  same 7 values as at 16:44, no Wakeshot; RunOnce empty; no `%LocalAppData%\Wakeshot`; `WAKESHOT_DATA_DIR` unset
  (user, machine, process).
- 2026-11-04 19:07, worker (Part B prep): `kill.ps1` rewritten (banner, colour, dialog; `-NoDialog` for the smoke test
  only); new `scripts\wait-signin.ps1`; `make-cmd.py` now writes only B2's six files; `python -I make-cmd.py` wrote
  them to `%LocalAppData%\Wakeshot-harness\haik`. Smoke tests, with `LOCALAPPDATA` pointed at a scratch folder for the
  `pwsh` runs only: `wait-signin.ps1` found the shell Explorer (pid 19640, started 18:15:58) and its RunOnce pass
  (9706 at 18:16:37.290, as in B2's evidence) and returned in 1 s; `kill.ps1 -NoDialog` with no fake tray running gave
  the STOP banner ("Found 0 ... Nothing was ended"), exit 1. The dialog call (`Popup`, Stop icon, system modal) shown
  once with a 2 s timeout: returned -1 (closed by the timeout). `power_check fake-tray --plan-only` exit 0: values at
  start and exit the same, RunOnce and guard name `<S2>\target\debug\wakeshot-tray.exe`, data folder
  `%LocalAppData%\Wakeshot`; afterwards no data folder, no RunOnce value, no test plan. Scratch folders deleted.
  Nothing of mine runs. Steps for Haik written ("Part B: B2 redo"). Status `blocked: waiting for Haik`.

## Result

Part A done (2026-11-04); fix round of the review done (19:01); Part B prep done (19:08), Haik's B2 redo next.

Fix round (review `docs/reviews/p2-6-runonce-keep.md`, 0 blockers, 0 majors, 6 minors):

- Finding 1, fixed: the dry-run line is logged once per `FileNet` (`power/journal.rs:378`, `:551`); test
  `dry_run_never_touches_run_once` now expects one line for two calls.
- Finding 2, lean fix: a journal that does not read still gets no value. `docs/architecture.md:939` names the case and
  the stronger fix with its trigger; test `no_safety_net_without_a_known_entry` (`executor.rs:1332`, renamed from
  `no_safety_net_when_nothing_owned`) has an `Unreadable` start.
- Finding 3, skipped (orchestrator): `README.md` holds someone else's uncommitted edits. Open (Found outside scope 5).
- Finding 4, fixed: `docs/architecture.md:912` "a value that is there and right is only read".
- Finding 5, fixed: section 8's row (`docs/architecture.md:1196`) is the apply's; before each command a failed write
  or guard start only logs.
- Finding 6, fixed (orchestrator's decision, for Haik's review): `keep_guard` (`executor.rs:253`) sets the value for
  any journal that reads and holds a known entry, whatever its owner (`:264`), and starts a guard only for this tray's
  own (`:285`). New test `another_owners_leftover_gets_the_value_not_a_guard` (`executor.rs:1390`);
  `leftover_runs_only_for_gone_or_own_owner` now expects the value. Docs: `executor.rs:70`, `:241`;
  `journal.rs:348`; architecture "Logon restore".
- Checks: `rustfmt --edition 2024 --check` on both files pass; `cargo clippy -p wakeshot-windows --all-targets --locked
  -- -D warnings` pass, without `--all-targets` pass; `cargo test -p wakeshot-windows --locked` 570 passed, 0 failed,
  23 ignored. Mutation checks: each of the four fixes undone fails at least one test. `wakeshot-core` not run (no
  model changed). Build folder deleted.
- `docs/research.md`: the 17 ms RunOnce pass fact (Part A's finding).

Part B prep: builds in `<S2>\target`, baseline saved, B2's step files with the timing waits in the scripts and a
STOP or NOT READY that cannot be missed (Decisions 11 to 15), smoke-tested. Steps for Haik in "Part B: B2 redo".

Part A, as reported before the review:

What was done: while the journal reads and names this tray with a known entry, `PowerExecutor::keep_guard` now calls
`set_run_once`, then `ensure_guard`, before each plan read, apply and restore. A value Windows deleted at a sign-in's
RunOnce pass is back within one plan read (10 s while anything is owned); a present value is only read. A missing value
logs an Info line, a failed write a Warn line, and the command still runs. Nothing changes when nothing is owned or the
journal names another owner.

Files changed:

- `crates/wakeshot-windows/src/power/executor.rs`: `keep_guard` (line 249, the call at 265) and its doc; the struct's
  "Callers may rely on" bullet (line 69). Tests: call indices moved by one in `outcomes_carry_the_journal`,
  `failed_plan_read_is_plan_err`, `failed_entry_read_keeps_the_entry_and_plan_is_ok`, `ensures_guard_while_owned`;
  `no_guard_when_nothing_owned` renamed `no_safety_net_when_nothing_owned` and now also checks no RunOnce call and no
  value; new `run_once_set_again_while_owned` (fake: all four owned commands, value missing or present, the log line,
  a failed write, the next try), `run_once_comes_back_with_the_real_net` (the tray's `FileNet` over a scratch HKCU key:
  apply, value deleted, back on the next plan read, not rewritten when present, gone after the restore, none when
  nothing is owned), `plan_read_stop_points` (a stop at every call of a plan read with the value deleted: no plan
  write, entries intact with the user's originals, value back only after its call, the next start restores all).
- `crates/wakeshot-windows/src/power/journal.rs`: doc of `RunOnce::SetAndClear` only (line 347).
- `docs/architecture.md` section 5: "Logon restore" (line 905) and "Gap left" (new paragraph, line 933).
- Not changed: `stop_points.rs`, core `props.rs` and `fake.rs` (Decision 6); no model had to learn the call.

Checks (target x86_64, build folder `<scratchpad>\p2-6-runonce-keep\target`, deleted after):

- `rustfmt --edition 2024 --check` on `executor.rs` and `journal.rs`: pass.
- `cargo clippy -p wakeshot-windows --all-targets --locked -- -D warnings`: pass. Without `--all-targets`: pass.
- `cargo test -p wakeshot-windows --locked`: 569 passed, 0 failed, 23 ignored (the `real_` and manual tests).
- Mutation check: with the call replaced by `Ok(false)`, the 3 new tests and 4 old ones fail.
- `cargo test -p wakeshot-core`: not run, no model changed.

For the review: the call runs outside the journal lock, like `ensure_guard` already did. The only race is a restore by
another process between `keep_guard`'s read and its write, which would leave a value with no journal (walk R6,
harmless). While the tray lives, no other process restores its journal: the guard and restore mode wait until it is
gone, and `wakeshot restore` hands the job to the tray. A restore command may now write the value right before its own
clean-up removes it. That costs two registry calls, and the value is there if a session end cuts the restore short.

Open items (updated 2026-11-04 19:08):

1. Haik's B2 redo ("Part B: B2 redo", "Steps for Haik"), then deliverable 6 by a new worker ("After Haik's steps").
2. Haik's review of finding 6's fix (the value also for another owner's leftover; Decisions 9).
3. Found outside scope 5: the README's gap list (review finding 3), once `README.md`'s other edits are committed.
4. Done in the fix round: the review's findings 1, 2, 4, 5, 6 and Found outside scope 1 to 3.
