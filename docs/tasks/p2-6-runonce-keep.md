# Keep the RunOnce value while a hold owns entries, then redo B2

Phase: 2 to 6 (power track fix). Status: in progress (Part A). Needs Haik: Part B only: about 5 minutes of his hands, one sign-out,
after the review of Part A. Agent: `wakeshot-worker`. Size: about 120K tokens (Part A about 80K, Part B about 40K).
Written by the orchestrator (session after `wakeshot-27`, 2026-11-04 18:35) from `docs/tasks/p3-haik-checks.md`
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04 18:35: card written by the orchestrator.
- 2026-11-04, worker: Part A started (deliverables 1 to 4 only; Part B after the review, in a later round).

## Result

What was done, files changed, checks and outcomes, open items.
