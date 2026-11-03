# CLI: `wakeshot restore`

Phase: 4. Status: ready. Needs Haik: no (see Questions: Question 12 of Phase 1 may add one line). Agent:
`wakeshot-worker`. Size: about 60K tokens.
Runs after: `p3-journal` (`restore_locked`, `FileNet`), `p3-power-plan` (`Win32Power`, `DryRunPower`), the core's
`p2-ownership-restore` (`ownership::restore`, `FakePlans`), `p3-store`, `p3-process`, `p3-log` (review M1: these
replace the power track's restore card, dropped at 17:17). Parallel with: every other card of this track and every card
of tracks 2, 3, 5, 6, except `p6-install-e2e` while this card's built-exe `--dry-run` checks use the dry-run data folder
(review m11).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cli-restore\target`. Uses the real laptop: no. Tests run the restore
routine only against the core's `FakePlans` and `FileNet` in a temp folder, never `Win32Power` and never the real
data folder.

## Goal

`wakeshot restore` as the table in `architecture.md` section 4 says: the tray does it when the journal's owner is
alive; the CLI does it itself when the owner is dead; "nothing to restore" when there is no journal.

## Read first

- `docs/architecture.md` section 4 ("How they talk": the `wakeshot restore` row), section 5 (restore routine,
  "Liveness"), section 6 (journal format, `restore-request.json`), section 8.
- `docs/phases/04-sources.md`: track notes 13 and 15.
- `docs/reviews/p1-contracts-domain.md` M2 (the routine's signature), m4 (the request file is deleted after
  `state.json`).
- `crates/wakeshot-core/src/files.rs` (`JournalFile`, `RestoreRequest`, `StateFile`); the power track's
  `power/journal.rs` (`restore_locked`, `FileNet`) and `power/plan.rs` (`Win32Power`, `DryRunPower`); the core's
  `ownership::restore` and `FakePlans`.

## Deliverables

`crates/wakeshot-cli/src/restore.rs`, `main(global) -> ExitCode`:

1. No journal: "Nothing to restore." exit 0.
2. Journal owner (pid and start) alive: write `restore-request.json` atomically; wait up to 5 s (poll every 100 ms) for
   the tray to delete it; then read `state.json` and print what is still overridden ("Your power settings are
   restored." when nothing is). No answer in 5 s: say the tray did not answer and exit 1 (the request stays; the tray
   still acts on it).
3. Owner gone: run `power::journal::restore_locked` (it takes `Local\Wakeshot.Journal` and calls
   `ownership::restore` with `expect_owner` = the dead owner; `.DryRun`, the dry-run folder and `DryRunPower` under
   `--dry-run`; RunOnce cleared, never set; `NoGuard`), then print the result: restored, or
   failed with the reason (journal kept, the next Wakeshot start retries), exit 1 on failure.
4. Owner liveness unknown: "Cannot tell whether Wakeshot is running. Try again." exit 1 (section 5: Unknown never
   starts a restore).
5. A corrupt or newer journal: whatever the power track's routine reports, printed plainly.
6. One log line per run (path taken and result).

Tests first, with the core's `FakePlans`, `FileNet` and a temp data folder: each of the five paths; the request file
deleted by a fake tray thread after 1 s (success) and never (timeout); a dead owner whose restore fails (exit 1,
message).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-cli --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-cli`: clean as the check rule below says, new tests named in the Result.
- A grep shows no `Win32Power` and no real data folder path in any test of this card.
- From a built exe with `--dry-run` and no dry-run journal: "Nothing to restore." (recorded).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The restore routine itself, the journal, the tray side of the request (power and core tracks). `main.rs`,
  `command.rs`. Any file not listed above.

## Questions for Haik

Each with the worker's default.

- Phase 1 Question 12 (a failed restore starts the tray) covers the guard and the sign-in restore. Default here: the
  CLI does not start the tray after its own failed restore; the person who typed the command sees the failure at
  once. If Haik's answer to Question 12 should cover the CLI too, add that one step.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
