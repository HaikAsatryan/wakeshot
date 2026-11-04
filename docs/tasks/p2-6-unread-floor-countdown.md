# The Floor countdown waits while the journal is unknown; two state.rs doc lines; the empty-copy retry interval

Phase: 2 to 6 (core). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p2-6-unread-lid-countdown` "Found outside scope" 2 and 3 and
`p2-6-unread-journal` "Found outside scope" 5.
Runs after: `p2-6-unread-lid-countdown` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-floor-countdown\target`.

## Goal

Close the same gap for the battery floor's countdown that `p2-6-unread-lid-countdown` closed for the work-end
countdown, fix two stale doc lines in a contract file, and look at how often an unread journal is retried.

## Read first

- `docs/tasks/p2-6-unread-lid-countdown.md`: Result, decisions, "Found outside scope" 2 and 3.
- `docs/tasks/p2-6-unread-journal.md`: "Found outside scope" 5.
- `crates/wakeshot-core/src/core/lid.rs` (`floor_countdown_starts`, `floor_countdown_wanted`, `floor_waits`,
  `work_end_waited`, `journal_unknown`); `crates/wakeshot-core/src/state.rs` (`LockWatch`, `work_end_waits`);
  `crates/wakeshot-core/src/core.rs` (`plan_read_due`).
- `docs/architecture.md` (grep "Floor" and "countdown").

## Deliverables

1. `core/lid.rs`: a step back with the lid closed on a Do nothing in force while the journal is unknown starts no
   `Floor` countdown today, so after the restore the closed laptop runs on battery under the floor until the user's
   battery sleep timeout. Use the same fix shape as the work-end case (wait while the journal is unknown; a want that
   waited takes no Do nothing as final), unless a leaner one exists; say why. Tests first in `scenarios/lid.rs`.
2. `state.rs`: doc lines only, no type or field change: `LockWatch` ("the countdown rules that wait for the first plan
   read") and `work_end_waits` (suggested text in `p2-6-unread-lid-countdown` "Found outside scope" 3), plus the floor
   field if deliverable 1 makes its doc stale.
3. `core.rs`: a retry with an empty copy (an unread journal, or a failed restore that read nothing) runs at the 30 s
   safety tick, not every 10 s, because `plan_read_due` sets a deadline only while the copy holds entries. Decide
   whether the 10 s retry is worth it now that a waited countdown starts only after a read; if yes, the leanest change
   and a test; if no, say why in the Result and change nothing.
4. `docs/architecture.md`: only the lines that state these rules, if one changes.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and
  without `--all-targets`, both targets; `cargo test --locked -p wakeshot-core`; the props at 4096 cases once; then
  `cargo test --workspace --locked` once at the end.
- Mutants only with one job and a low timeout while you watch free memory (a mutant of the last card grew to 35 GB and
  Claude Code then ended other processes for low memory); stop at once if free memory drops under 10 GB. If you cannot
  cap it, skip mutants and say so.
- Build folder deleted at the end.

## Out of scope

- `wakeshot-windows`; `event.rs`; any field or type change in `state.rs` (describe it and stop).
- `p6-int-wiring-fixes` decision 7 (a lock on a user's own Do nothing while the journal is unknown): Haik's call.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result

What was done, files changed, checks and outcomes, open items.
