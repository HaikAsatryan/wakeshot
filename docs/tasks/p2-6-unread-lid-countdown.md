# A closed lid still counts down when an unread journal's restore lands

Phase: 2 to 6 (core). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-int-wiring-fixes` "Found outside scope" 3 and 5.
Runs after: `p6-int-wiring-fixes` (done; it changed `core/lid.rs` and `scenarios/lid.rs`). Parallel with: cards that
own none of the files below. It may run beside `p6-int-dry-run-e2e` (that card fixes only `wakeshot-windows` `app/*`
files, the tray tests and the CLI command files).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-lid-countdown\target`.

## Goal

With an unread startup journal (`Event::Started { journal_unread: true }`), a lid closed before the `Leftover`
restore lands must end the same way as with a journal that read: once the restore puts the user's own lid action back
and nothing holds, the closed laptop must not run on until the user's battery sleep timeout.

Today (`p6-int-wiring-fixes`, Found outside scope 3): the lock rules now treat the unread case as "may be ours", but
the work-end countdown reads the Do nothing in force as the user's own (`work_ended_on` uses `in_force` with the
empty copy), so no countdown starts. When the retry then restores the user's Sleep, a laptop whose lid is already
closed does not sleep by itself. With a journal that read, the same case counts down (the copy has the `original`).

## Read first

- `docs/tasks/p6-int-wiring-fixes.md`: deliverable 5, its Result, Found outside scope 3 and 5.
- `docs/tasks/p2-6-unread-journal.md`: Goal 1 and its Result.
- `crates/wakeshot-core/src/core/lid.rs` (`in_force`, `work_ended_on`, the countdown rules, the unread guess from
  `p6-int-wiring-fixes`); `crates/wakeshot-core/src/scenarios/lid.rs`; `crates/wakeshot-core/src/scenarios/props.rs`
  (`WorldStart`, `startup_unread`).
- `docs/product.md` (lid rules); `docs/architecture.md` (grep "countdown" and "unread").

## Deliverables

1. The leanest fix in `core/lid.rs` (for example: a countdown when a `Leftover` restore lands with the lid closed and
   nothing holding, or the unread guess also covering `work_ended_on`), and why that one. It must start no countdown
   when the user's own lid action, once known, is Do nothing (the plan read tells), and must not change the read
   journal cases. Tests first, in `scenarios/lid.rs`: the case above fails before the fix.
2. `scenarios/props.rs`: try drawing `startup_unread` in `WorldStart` now that the lock rules have a guess for it.
   Keep it if the properties pass at their usual case counts; if not, say why in the Result and leave it as it is.
3. `docs/architecture.md`: only the line or lines that state this rule, if one changes.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and
  without `--all-targets`, both targets (core needs no clang); `cargo test --locked -p wakeshot-core`; then
  `cargo test --workspace --locked` once at the end.
- Mutants on the changed lines if the card's time allows (as `p2-core-lid` did); record the count.
- Build folder deleted at the end.

## Out of scope

- `wakeshot-windows` files, `core.rs`, `state.rs`, `event.rs` (a contract change: describe it and stop if the fix
  needs one).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result

What was done, files changed, checks and outcomes, open items.
