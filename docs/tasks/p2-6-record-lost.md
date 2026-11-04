# A journal that cannot be read: try for 2 minutes, then reset Wakeshot's own values to Windows defaults

Phase: 2 to 6 (core and power). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens.
Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from Haik's answer of 2026-11-04 (`docs/product.md`,
"User settings safety" 7). It replaces `p6-int-wiring-fixes` decision 7 (a lock on a Do nothing that may be ours
while the journal is unknown).
Runs after: nothing. Parallel with: `p3-idle-sleep-check` (laptop only, no shared files). Not with
`p2-6-sleep-route` (same files; it runs after this card).
Real laptop: no (reads of the real plan's defaults are fine; writes only to a plan copy, never Haik's plan).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-record-lost\target`.

## Goal

Haik's rule: with no work going on, the laptop behaves as if Wakeshot were not there. A journal that cannot be read
most likely means Wakeshot's values are still in force and their originals are lost. So, after a short retry, put
every value that looks like Wakeshot's back to Windows' default, tell the user once, and log it. Haik's words: "put it
to windows defaults and notify the user and log the error".

## Read first

- `docs/product.md`: "User settings safety" 1 to 7, the lid rules ("Never lower security", work end).
- `docs/tasks/p6-int-wiring-fixes.md` Result item 5 and decision 7; `docs/tasks/p2-6-unread-journal.md`,
  `p2-6-unread-lid-countdown.md`, `p2-6-unread-floor-countdown.md` (Result and "Found outside scope").
- `crates/wakeshot-core/src/core.rs` (start, `Leftover` retry), `core/settings.rs` (restore results, `retry`,
  `RestoreFailed`), `core/lid.rs` (`journal_unknown`, `lock_view`, the waited countdowns), `state.rs`, `command.rs`,
  `event.rs`, `text.rs` (`JournalCorrupt`, `RestoreFailed`).
- `crates/wakeshot-windows/src/power/journal.rs` (load: missing, corrupt, failed; the `.corrupt` set-aside),
  `power/plan.rs`, `power/executor.rs`.
- `docs/architecture.md` sections 3 and 4 (grep "unread", "corrupt", "journal").

## Deliverables

1. Lost journal, in the core. A journal load that fails (any read error) and keeps failing on each retry for
   2 minutes from the first failure (core clock, not tick count), or a load that finds the file damaged, makes the
   journal "lost". Then the core sends one command (new, in `command.rs`) to reset the values that look like
   Wakeshot's, and treats the journal as known and empty once the result comes back.
2. The reset, in `wakeshot-windows`. On the plan in use, for each key Wakeshot writes (`LidAc`, `LidDc`, `SleepDc`):
   if the value now is exactly the value Wakeshot writes (lid Do nothing, sleep Never), write the plan's Windows
   default (`PowerReadACDefaultIndex` or `PowerReadDCDefaultIndex`; check the current Microsoft docs), then
   `PowerSetActiveScheme`; any other value is the user's: leave it. Log each key with its old and new value, or why it
   was left. Set the bad file aside the way a damaged one is set aside today; if that fails, log it and go on (the
   next save replaces the file). `DryRunPower` does the same on its copy.
3. Notice. One notice, only when at least one value was reset (nothing reset: log only, no notice). Draft words
   (the worker may improve them within `text.rs` rules): title "Power settings set to Windows defaults", body
   "Wakeshot could not read its record of your power settings, so it put the lid and sleep settings that looked like
   its own back to Windows defaults. If you had your own, set them again. Click to see the old values in the log."
   It replaces `JournalCorrupt` (a damaged file now takes the same path). Click opens the log.
4. The unknown window. While the journal is unknown (before it reads or is lost), no lock at a lid close and no
   countdown: remove the `p6-int-wiring-fixes` item 5 lock guess and keep the countdown waits. When the journal
   becomes known or lost and the lid is closed with nothing holding and the user's lid action (after any reset) not Do
   nothing, start the same countdown a work end with the lid closed starts (today it ends in a hibernate;
   `p2-6-sleep-route` turns it into sleep).
5. A hold that starts while the journal is unknown: keep what is built today, and say in the Result what happens (one
   paragraph); change it only if it breaks rule 7.
6. Tests first: core scenarios (retry then lost at 2 minutes; damaged file lost at once; a read that works at 1:59
   ends it; reset of only Wakeshot-looking values; no notice when nothing reset; no lock in the window; the countdown
   at the end with the lid closed; a user's own Do nothing reset and noticed); Windows unit tests with the fakes; one
   real read of Balanced's defaults (read only); a real reset on a plan copy only if the harness makes it easy.
7. `docs/architecture.md`: the lines of sections 3 and 4 that state these rules.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64 and ARM64 (the tray's ARM64 needs clang: CI only); `cargo test --workspace --locked` (debug
  only, never `--release`); the core props at 4096 cases once.
- Mutants only with one job, a low timeout and a memory cap (a mutant once grew to 35 GB); stop if free memory drops
  under 10 GB; skip and say so if you cannot cap it.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The sleep route and `SettingKey::SleepAc` (`p2-6-sleep-route`). Keys other than the three. Other plans than the
  plan in use (say in the Result if a lost journal can leave Wakeshot values on another plan, and how likely).
- `README.md`, phase files, `docs/product.md` (the orchestrator owns them; tell it if a product line is wrong).

## Questions for Haik

None (his answer is in `docs/product.md`, "User settings safety" 7). Decide the rest yourself and record each choice
as "decided by the worker, for Haik's review".

## Found outside scope

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result
