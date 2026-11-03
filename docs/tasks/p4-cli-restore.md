# CLI: `wakeshot restore`

Phase: 4. Status: done. Needs Haik: no (Phase 1 Question 12 is answered with the default, Haik 2026-11-03 18:09; it
adds nothing here). Agent: `wakeshot-worker`. Size: about 60K tokens.
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

- Phase 1 Question 12 (a failed restore starts the tray) is answered: the default, Haik 2026-11-03 18:09. It covers
  the guard and the sign-in restore only. Build: the CLI does not start the tray after its own failed restore; the
  person who typed the command sees the failure at once.

None open. Decided by the worker, for Haik's review (each with its reason and a "change it when"):

1. **A corrupt journal is set aside by the CLI**, through the routine with no expected owner, as restore mode does:
   a corrupt file names no owner, so no tray would ask for it. It prints the same advice as the tray's notice ("set
   the lid and sleep settings again by hand; the log has the old values") and exits 1. Change it when a running tray
   must be the one that sets it aside (then ask the tray when `state.json` says one runs).
2. **A newer journal**: the keys this version knows are restored; for the others it adds "1 setting is kept as a newer
   Wakeshot set it. Update Wakeshot to put it back." and exits 1. Why: the user asked for everything back, and part
   stays changed. Change it when a downgrade path is decided that restores unknown keys some other way.
3. **The tray's answer read as "still changed"**: when `state.json` still lists overridden keys after the tray
   answered, it says "Wakeshot could not put back every power setting yet. It keeps trying." plus one "Changed by
   Wakeshot: <setting>" line per key, and exits 1. Change it when the tray can answer with a failure reason.
4. **The journal owner check reads the journal without the lock first**, as restore mode does (every write is atomic;
   the routine checks the owner again under the lock). A journal taken over in between: "Another Wakeshot took over
   your power settings just now. Run 'wakeshot restore' again." exit 1.

## Found outside scope

1. **`p4-cli-args` is not done**, so the built-exe `--dry-run` check runs through `main` instead (Result, check 3), as
   in `p4-cli-commands`.
2. **The tray side does not exist yet**: no watch thread posts `Menu(Restore)` for `restore-request.json` and no IO
   thread runs `FinishRestoreRequest` (`p4-watch-thread`, `p6-int-io-thread`). Until they land, `wakeshot restore`
   against a live tray times out after 5 s and leaves the request. The tests use a fake tray thread.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:27: started by `wakeshot-worker`, second card of the bundle after `p4-cli-commands` (done); builds into
  that card's folder (`<scratchpad>\p4-cli-commands\target`, one folder for the bundle).
- 2026-11-04 00:33: `restore.rs` written, 10 tests pass (three first failed only because `restore_locked` logs its own
  `journal` line into the same log; the tests now count `cli` lines). `status::overridden` made crate-visible for the
  "still changed" lines (own file of the bundle). Dry-run check through `main`: "Nothing to restore."; the
  `%LocalAppData%\Wakeshot` it made was removed. All checks clean; done.

## Result

Done 2026-11-04 00:33.

What was built: `crates/wakeshot-cli/src/restore.rs`, `main(global) -> ExitCode`, with the work in `run(dir, parts)`
over `Parts { plans, net, names, log, liveness }`, so tests pass `FakePlans` and a `FileNet` on a temp folder.

1. No journal: "Nothing to restore." exit 0.
2. Owner (pid and start from the journal, not `state.json`) alive: writes `restore-request.json` (`{"version":1}`)
   atomically, looks every 100 ms for up to 5 s for it to go, then reads `state.json`: "Your power settings are
   restored." exit 0, or the still-changed lines, exit 1. No answer: "Wakeshot did not answer within 5 seconds. It
   still restores your power settings when it gets to the request." exit 1; the request stays.
3. Owner gone: `power::journal::restore_locked(plans, net, None, Some(owner), names, log)`. Real: `Win32Power`,
   `FileNet::restore_only(dir, AutoStart::real(), log)` (RunOnce cleared, never set; no guard), `Names::real()`. Dry run:
   `DryRunPower` over `Win32Power`, `FileNet::dry_run`, `Names::dry_run()` (`.DryRun` mutexes). Restored: exit 0. Failed:
   "Could not restore your power settings: <call> failed with <code>. Wakeshot tries again when it next starts." exit
   1, journal kept. The CLI does not start the tray.
4. Owner liveness unknown: "Cannot tell whether Wakeshot is running. Try again." exit 1; nothing written.
5. Corrupt or newer journal: Questions, items 1 and 2.
6. One `cli` log line per run, such as `restore: owner pid 9120 gone, restored here` or `restore: owner pid 9120 alive,
   asked the tray: no answer within 5 s` (the routine adds its own `journal` lines).

New tests (`restore::tests`): `no_journal_is_nothing_to_restore`, `a_dead_owner_is_restored_here`,
`a_dead_owner_whose_restore_fails_keeps_the_journal`, `unknown_owner_liveness_starts_nothing`,
`a_live_owner_is_asked_and_its_answer_read` (fake tray thread deletes the request after 1 s),
`a_tray_that_could_not_restore_says_what_is_still_changed`, `a_tray_that_does_not_answer_leaves_the_request` (the
real 5 s), `a_corrupt_journal_is_set_aside_and_said_plainly`, `a_newer_journal_restores_what_this_version_knows`,
`a_journal_taken_over_meanwhile_is_left_alone`; ignored: `dry_run_folder_check`. The restore tests use
`FileNet::restore_only` with a RunOnce key that does not exist (`HKCU\Software\Wakeshot.Test.CliRestore.<pid>`), so
the clean-up's remove finds nothing and nothing is written to the registry (checked: no `Wakeshot*` key under
`HKCU\Software` after the run).

Checks:

1. `rustfmt --edition 2024 --check` on the owned files: clean. `cargo clippy --locked -p wakeshot-cli -p
   wakeshot-windows` with and without `--all-targets`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, `-D
   warnings`: clean (4 runs). `cargo test --locked -p wakeshot-cli`: 40 passed, 3 ignored (this card 10 passed, 1
   ignored).
2. Grep of `restore::tests`: no `Win32Power`, `DataDir::real`, `Names::real` or `AutoStart::real`. The one ignored
   test names the dry-run folder (`status::data_dir` with `--dry-run`): it is the stand-in for check 3; it calls
   `main`, which builds `DryRunPower` over `Win32Power`, and with no journal no plan call is made.
3. `--dry-run` with no dry-run journal, through `main` (`restore::tests::dry_run_folder_check`; the exe cannot route
   it yet): "Nothing to restore.", exit 0, log `cli: restore: no journal, nothing to do`.
   `%LocalAppData%\Wakeshot` did not exist before; the run made only `dry-run\logs\wakeshot.log`, removed after.

Open items: the built-exe check once `p4-cli-args` lands; the end-to-end check against a running tray after
`p4-watch-thread` and `p6-int-io-thread`. The bundle's build folder is deleted.
