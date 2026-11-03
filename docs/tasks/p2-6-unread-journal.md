# Retry an unread startup journal; check the lid again after the going to sleep sound; keep a newer config

Phase: 2 to 6 (core and Windows power). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 60K
tokens. Written by the orchestrator (session `wakeshot-46`, 2026-11-04) from power safety review findings 3 and 6.
Runs after: core round B (`docs/tasks/p2-core-settings.md`, "Core round B (2026-11-04)"), since both edit `core.rs`
and `scenarios/props.rs`. Parallel with: no card that builds `wakeshot-core` or `wakeshot-windows` (a contract
change: every caller of `Event::Started` changes in the same pass). Cards that do not build may run beside it.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-journal\target`.

## Goal

1. Finding 3: when the startup read of `journal.json` fails (delete pending, antivirus, I/O error), the core retries
   it on each tick as `Restore { why: Leftover }`, instead of treating the journal as empty for the whole run.
2. Finding 6: the hibernate thread checks the lid once more after the going to sleep sound, right before
   `SetSuspendState`, so a lid opened during the sound skips the hibernate.

## Read first

- `docs/reviews/p2-6-power-safety.md`, findings 3 and 6.
- `docs/tasks/p6-int-startup.md`, "Power safety review finding 3: what the core-side fix needs from startup".
- `crates/wakeshot-windows/src/power/executor.rs` (`Startup`, `startup`, the `Leftover` path);
  `crates/wakeshot-windows/src/app/startup.rs` (`started`, `startup_restore`, the 500 ms retry);
  `crates/wakeshot-core/src/event.rs` (`Event::Started`); `crates/wakeshot-core/src/core.rs` (the `Started` arm);
  `crates/wakeshot-windows/src/power/hibernate.rs` (`run_hibernate`, `check_lid`).
- `docs/architecture.md`: the lines on `Started` and the hibernate thread's steps (grep for them).

## Deliverables

1. `Startup` gains `unread: bool` (`power/executor.rs`): true when `JournalRead::Failed`. Its doc says what it means.
2. `Event::Started` gains `journal_unread: bool` (`event.rs`, a contract change); `started()` copies `Startup.unread`
   into it (`app/startup.rs`). Every place that builds `Event::Started` (core tests, `sim.rs`, `scenarios/props.rs`,
   `app/dispatch.rs`, `app/startup.rs`) gets the field.
3. The core (`core.rs`, `Started` arm): `journal_unread` sets `retry = Some(RestoreWhy::Leftover)` with an empty copy.
   Tests: an unread journal retries on the next tick; a later good read that restores ends the retry; a read that
   keeps failing keeps the retry and never panics or loops inside one tick.
4. Decide whether the 500 ms startup retry in `app/startup.rs` stays (quick first try) or goes; record why.
5. `run_hibernate`: after the sound and before `SetSuspendState`, one more `check_lid`; open is
   `Skipped(LidOpen)`, no answer is `Skipped(LidUnknown)` as in step 1. Logged. Tests with the fake deps: lid opens
   during the sound gives `Skipped(LidOpen)` and no hibernate call; the doc list in `run_hibernate` gets the new step.
   No cancel flag: the review's lean option needs no contract change. An agent turn that starts in those seconds is
   not covered; say so in the Result.
6. `docs/architecture.md`: only the lines on `Started`'s fields and the hibernate steps, changed in place.
7. Added by the orchestrator after core round B: the `WriteConfig` runner never writes over a newer `config.json`.
   The exact change, its test and the two doc lines are in `docs/tasks/p2-core-settings.md`, "Core round B", "Needs
   another card" item 1 (`crates/wakeshot-windows/src/app/io_thread.rs`, the `Command::WriteConfig` arm; one sentence
   in `docs/architecture.md` section 6; one doc line on `WriteConfig` in `crates/wakeshot-core/src/command.rs`).

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- `cargo test --workspace --locked`: all pass; record the counts.
- Only the files above and this card changed.

## Out of scope

- A cancel flag for the hibernate thread (`Command` or `Cancel` changes); a sleep fallback after a failed hibernate.
- Power findings 5, 8, 9, 10 (core rounds A and B).
- Any real power setting, lid, screen or sound: tests use fakes only.

## Questions for Haik

None expected. Haik is away: decide with your default and list it as "decided by the worker, for Haik's review".

## Found outside scope

1. `app::window::tests::an_event_re_arms_the_timer` and
   `the_timer_fires_after_next_timer_ms_and_again_after_each_re_arm` (real timers, 50 ms slack) failed in 2 of the 5
   runs of the `wakeshot-windows` tests that included them, while two other cards built; they passed alone at once and
   in the last two full runs. Flaky under load; not my file, not touched.
2. `cargo fmt --all --check` shows diffs only in `crates/wakeshot-windows/src/app/tray_mode.rs` and
   `crates/wakeshot-tray/tests/smoke.rs` (`p6-int-tray-mode`, mid-edit); one ARM64 `--all-targets` clippy run failed
   there too (`type_complexity`) and passed on the retry. Not touched.
3. With an unread journal the lid lock rules have no guess before the first plan read (`core/lid.rs` uses the
   `Started` journal for that), so a lid close in the first seconds, before the first tick's retry lands, gets no lock
   for a crashed tray's Do nothing. The 500 ms start-up retry narrows this; closing it needs `core/lid.rs` to treat
   "unread" as "may be ours". Not my file.
4. `docs/architecture.md` section 4, "Tray startup" step 6, does not name the unread case (the `Started` block now
   does). Left: the card allows only the `Started` and hibernate lines.
5. A retry with an empty copy runs at the 30 s safety tick, not every 10 s: `plan_read_due` sets a deadline only while
   the copy holds entries. Same as a failed restore that read nothing today; left as it is.

## Log

- 2026-11-04 02:33: started (orchestrator `wakeshot-46`: runs beside `p6-int-tray-mode` Part A and `p5-sounds` Part
  B, which build `wakeshot-windows` but never construct `Event::Started` or `Startup`; the contract change goes in
  one pass before the first cargo run). Read the card, CLAUDE.md, How we work, findings 3 and 6, the startup card's
  finding 3 section, core round B's Needs another card 1, and the code. Builders of `Event::Started` in the whole
  workspace: `sim.rs` (`Start::event`), `app/startup.rs` (`started` and its test); patterns only in `core.rs` (the
  arm), `scenarios/props.rs` and `app/dispatch.rs` (`..`, no edit needed). `Startup` literals: `executor.rs` and
  `startup.rs` only.
- 2026-11-04 02:36: contract pass done in one go: `Event::Started.journal_unread` (`event.rs`, doc), the core arm,
  `sim::Start`, `Startup.unread` (`executor.rs`, set on `JournalRead::Failed`), `started()` copies it, every
  `Startup` literal in tests. `cargo check --workspace --all-targets --locked`: exit 0, so the other cards' builds
  are not broken. The startup retry keeps its 500 ms try, now keyed on `first.unread` (the private
  `unreadable_journal` second read is gone).
- 2026-11-04 02:40: core done. `journal_unread` sets `retry = Some(Leftover)` with an empty copy. Tests: three Sim
  scenarios in `scenarios/settings.rs` and three named World cases in `scenarios/props.rs` (the World gained
  `WorldStart::startup_unread`: the crashed tray's file stays, no start-up restore, through the real restore routine
  at the tick). The random `world_start` does not draw it (see Result). `cargo test -p wakeshot-core`: 593 passed,
  1 ignored. Mutant "ignore `journal_unread`": 4 of the 6 new tests fail (the other two check the retry's end and a
  hold beside it, which work the same either way); reverted.
- 2026-11-04 02:45: Windows side done. `executor.rs`: test `startup_says_when_the_journal_does_not_read`.
  `hibernate.rs`: the lid check once more after the sound (`check_lid(.., after_sound)`, lines end "after the sound"),
  doc step 7, test `lid_opened_during_the_sound_skips` (open and no answer), call lists updated. `io_thread.rs`:
  `write_config` keeps a newer file (round B item 1), two tests. `command.rs`: the `WriteConfig` line and the
  `Hibernate` line (it said the sound comes after the last check; now the lid is read once more). `cargo test -p
  wakeshot-windows --lib -- power:: app::`: one run had 2 `app::window` real-timer tests fail (not my file, passed
  alone at once: load from the other builds); the rerun of those passed. Clippy x64 both ways and ARM64 libs: exit 0;
  ARM64 `--all-targets` failed in `app/tray_mode.rs` (`type_complexity`, lines 1245 and 1247: `p6-int-tray-mode`'s
  file, being edited now). Not touched; retried at the end.
- 2026-11-04 02:49: docs: `docs/architecture.md` (the `Started` block, the `power::hibernate` row, the section 5
  hibernate paragraph, the section 6 newer-file sentence) and the `Startup` and `startup` docs. Final checks in
  Result. Build folder and scratch script deleted.

## Result

Done 2026-11-04 02:49.

### What was done

1. `Startup.unread` (`power/executor.rs`): true exactly when the startup read gives `JournalRead::Failed`; its doc says
   what it means. The executor's warning now says the core retries it.
2. `Event::Started.journal_unread` (`event.rs`, contract, doc). `started()` copies `Startup.unread`. Builders:
   `sim::Start` (default `false`), `started()` and its test. `core.rs`, `scenarios/props.rs` and `app/dispatch.rs`
   only match with `..` or build through `Start`, so `dispatch.rs` needed no edit.
3. Core: `journal_unread` sets `retry = Some(Leftover)` with an empty copy. The executor's `Leftover` path then reads
   the file on each tick, checks the owner and restores.
4. The 500 ms startup retry stays (decision 1); it is now keyed on `first.unread`, and the private
   `unreadable_journal` re-read is gone. After the retry, `Startup.unread` is the second try's.
5. `run_hibernate`: after the sound, one more `check_lid`: open is `Skipped(LidOpen)`, no answer `Skipped(LidUnknown)`,
   each logged with "after the sound"; doc list step 7. Not covered: an agent turn that starts during the sound (only
   the core knows of it, and nothing cancels the thread); no cancel flag, as the card says.
6. `docs/architecture.md`: the `Started` block, the `power::hibernate` row, the section 5 hibernate paragraph.
7. `WriteConfig` (`app/io_thread.rs`, new `write_config`): a newer `config.json` is never written over (warning
   `config.json not written: version 2 is newer than 1; Automatic off for this run only`, answer
   `Err(Failure { call: "config.json (newer version kept)", code: 13 })`); a read error keeps the file too; a bad,
   older or missing file is written. Architecture section 6 sentence; `command.rs` `WriteConfig` line. Also the
   `Hibernate` doc line in `command.rs`, which the second lid check made wrong (doc only).

### Tests added

- Core, `scenarios/settings.rs`: `unread_journal_is_retried_on_the_next_tick`,
  `unread_journal_retry_ends_at_a_good_read` (a restore that works ends it; an alive owner's answer fills the copy and
  keeps the retry), `unread_journal_that_keeps_failing_keeps_the_retry` (6 ticks, exactly one `Leftover` each, one
  `RestoreFailed`; `Sim::send` panics if a tick would spin).
- Core, `scenarios/props.rs`, with the World's new `WorldStart::startup_unread` (the crashed tray's file stays, no
  start-up restore): `an_unread_journal_at_start_is_restored_at_the_first_tick`,
  `an_unread_journal_whose_retry_fails_is_restored_at_a_later_tick`,
  `an_unread_journal_and_a_hold_before_the_first_tick`. On the mutant "ignore `journal_unread`" 4 of the 6 core tests
  fail.
- `power/executor.rs`: `startup_says_when_the_journal_does_not_read` (and its first `Leftover` retry restores).
- `power/hibernate.rs`: `lid_opened_during_the_sound_skips` (open and no answer: no hibernate call); the old tests'
  call lists and log list gained the second lid read.
- `app/startup.rs`: `started_carries_this_tray` checks `journal_unread`; the two retry tests check `unread`.
- `app/io_thread.rs`: `write_config_never_writes_over_a_newer_file` (byte for byte),
  `write_config_replaces_a_bad_or_older_file_and_keeps_one_that_does_not_read`.

### Files changed

`crates/wakeshot-core/src/event.rs`, `core.rs`, `sim.rs`, `command.rs` (doc lines), `scenarios/settings.rs`,
`scenarios/props.rs`; `crates/wakeshot-windows/src/power/executor.rs`, `power/hibernate.rs`, `app/startup.rs`,
`app/io_thread.rs`; `docs/architecture.md`; this card. No `docs/research.md` entry: no new outside fact.

### Checks (own build folder, every cargo command `--locked`, final run 02:46 to 02:49)

- `cargo fmt --all --check`: my files clean (`rustfmt --check` on my four Windows files and `cargo fmt -p
  wakeshot-core -- --check`: exit 0); the only diffs are in `p6-int-tray-mode`'s two files (Found outside scope 2).
- `cargo clippy --workspace --all-targets --target <t> -- -D warnings` and without `--all-targets`, x64 and ARM64:
  all four exit 0.
- `cargo test --workspace`: exit 0. Core 593 passed, 1 ignored, plus 11 and 25 in `tests/`; windows 538 passed, 21
  ignored; cli 75 passed (3 ignored), 3, and 5 (2 ignored); tray 1, recovery 5 (2 ignored), smoke 0 (1 ignored).
  Total 1256 passed, 0 failed, 30 ignored. Two earlier runs had the two flaky timer tests fail (Found outside
  scope 1).
- Only the files above changed (from my own edit record). No real power setting, lid, screen or sound was used.

### Decided by the worker, for Haik's review

1. The 500 ms startup retry stays. Why: the core's retry first runs at the 30 s safety tick, and until then a lid
   close finds a crashed tray's Do nothing with no lock (Found outside scope 3); a failure of a moment is caught
   before the tray is up, and it costs 500 ms only when the read failed. Change it when the core's lid rules treat an
   unread journal as "may be ours"; then the core's retry alone is enough.
2. No notice at start for an unread journal: it behaves as a journal the start-up restore could not restore, so the
   first failed retry shows `RestoreFailed` once and a good read shows nothing. Change it when users report a silent
   leftover.
3. The second lid check also runs when the sound was skipped (No sounds): the same rule every time. It adds at most
   `LID_WAIT_MS` (2 s) when the lid does not answer, and then no hibernate. Change it when a log shows that read is
   slow.
4. The random World start does not draw `startup_unread`: the lock properties would need the excuse that the core
   cannot know a crashed tray's values while the file does not read. Named cases cover the path. Change it when
   `core/lid.rs` guesses for an unread journal.
5. `startup_restore` keeps its public signature (`FileNet` forms only) although it no longer reads the file itself.
   Change it when a caller needs another net.

### For other cards

- `p6-int-tray-mode`: nothing to change. `started()` copies `Startup.unread` itself; `startup_restore` and `started`
  keep their signatures. A hand-built `Startup` or `Event::Started` needs the new field.
