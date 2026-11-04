# Keep the RunOnce value while a hold owns entries, then redo B2

Phase: 2 to 6 (power track fix). Status: blocked: waiting for review (Part A done; Part B after it). Needs Haik:
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

None. The fix is the orchestrator's decision (2026-11-04), for Haik's review.

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

## Found outside scope

Problems seen but not fixed.

1. **A dry-run hold now logs "dry run: RunOnce value not set" at every plan read.** `FileNet::dry_run`'s `set_run_once`
   logs that line on every call (`power/journal.rs`, the `RunOnce::Off` branch), and `keep_guard` now calls it every
   10 s while a dry-run journal is owned: about 25 KB an hour (the log rolls at 1 MB). Fix: log it once per net (a
   field, one line). `journal.rs` code is outside this round's files. Verified by reading the code; not run.
2. **`docs/architecture.md` section 8, row "Journal load, lock or write, RunOnce value or guard start fails"** reads as
   if every such failure gives a notice. It is the apply's row; the failures in `keep_guard` (the guard start since
   `p3-power-io`, the RunOnce write now) only log. Not changed: section 8 is outside this round's lines. Suggest "(in
   an apply; before each command they only log)".
3. **The fact under Log 2026-11-04 (Explorer's pass takes 17 ms) belongs in `docs/research.md` "Power track manual
   checks"**, which is not in this round's files; Part B adds it with the B2 outcome.
4. `power/executor.rs:128`, a doc line from `p3-power-io`, is 121 characters (one over the wrap). Harmless.

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

## Result

Part A done (2026-11-04); Part B not started (after the review).

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

Open items:

1. The orchestrator's narrow power safety check of this change, then its fixes.
2. Part B (B2 redo with Haik), deliverables 5 and 6.
3. Found outside scope 1 to 3 (dry-run log line every 10 s; section 8 row wording; the 17 ms fact for research).
