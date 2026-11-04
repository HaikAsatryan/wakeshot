# Find the cause of three rare core property failures

Phase: 2 to 6 (core). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens.
Written by the orchestrator (session `wakeshot-27`, 2026-11-04) from `docs/tasks/p2-6-reset-gaps.md` "Found outside
scope" items 2 and 3.
Runs after: `p2-6-reset-gaps` (done, cc40a49). Parallel with: cards that own none of the files below (none run now).
Must end before `p3-haik-checks` starts (that card needs the laptop alone).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-props-rare\target`.

## Goal

Three core properties failed now and then at 4096 random cases (3 of 6 core runs), each on a case that fails the
same without `p2-6-reset-gaps`. CI runs them at 256 cases on every push, so they make CI flaky now and then. Two of
them guard Haik's rule "a closed laptop never runs unlocked with no path to sleep or lock", so a real core gap there
matters more than the flake. For each one decide: core bug or property (model) wrong, and fix the one that is wrong.

## Read first

- `docs/tasks/p2-6-reset-gaps.md` "Found outside scope" items 2 and 3: the three shrunk cases in words and the
  worker's first reading of each (inferred, not verified).
- `docs/tasks/p2-6-props-flake.md` Result: how that card caught its case with fixed seeds.
- `docs/product.md` "Never lower security", the work end rule, the battery floor, "User settings safety".
- `docs/architecture.md` "A failed `Restore`" and "The sleep route".

## Deliverables

1. `a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s` (item 2). First reading: the props
   model ends its `retry` flag only on a restore of every plan, while the core ends the retry on a restore of the
   other plan that worked. Check it; if the model is wrong, fix the model as the core does (architecture "A failed
   `Restore`" decides); if the core is wrong, fix the core.
2. `a_closed_laptop_is_locked_also_before_the_first_plan_read` (item 3, first): the lid closes while nothing of
   Wakeshot's is in force (the episode's apply failed), then a new hold applies Do nothing with the lid already
   closed, and no lock comes. Decide by `product.md`: a hold that puts our Do nothing in force on a lid that is
   already closed must lock (the window lock rule of `p2-6-sleep-route-fixes`), unless the model is wrong about what
   is in force.
3. `no_hibernate_within_5_minutes_of_a_resume_or_of_input` (item 3, second): the floor countdown's `SleepCheck`
   answer comes after input and a session end, so the fallback is due at once and `Hibernate` goes out 10 s after
   input; the hibernate thread's own check then skips it. Decide whether the core should not send it (input restarts
   the 5 minutes) or the property should accept a command the thread will skip. Haik's rule: no hibernate within 5
   minutes of input. Prefer the core not sending it, unless that is not lean.
4. Each case kept as a named test in "Shrunk cases kept as named tests" in `props.rs`; a core fix also gets one
   scenario in the matching `scenarios/` file.
5. Then the rate: the core props at 4096 cases, 10 runs with random seeds, and the props at 256 cases (CI's count),
   50 runs, all pass. If any other property fails, catch its case the same way and say in the Result whether you fixed
   it or left it (with its shrunk case in words).

## Files this card owns

`crates/wakeshot-core/src/` (any file), this card, and a line in `docs/research.md` or `docs/architecture.md` if a
core fact changes. No other crate.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets` on x64; `cargo test --workspace --locked` (debug
  only, never `--release` for the whole workspace); deliverable 5's runs.
- Mutants by hand on the lines you change (no `cargo-mutants` run).
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The menu "Restore my power settings" during a work-end countdown (`p2-6-reset-gaps` "Found outside scope" 1): a
  question for Haik, asked by the orchestrator.
- Any product rule change. If `product.md` does not decide a case, record your choice as "decided by the worker, for
  Haik's review".

## Questions for Haik

None. Decide yourself and record each choice as "decided by the worker, for Haik's review".

## Found outside scope

1. **Not verified, inferred from reading `core/settings.rs`:** `Core::journal_unknown` is `retry && copy empty`. While
   the core does not apply (stepped back, off the console, requests only), a pending retry stays after a plan switch's
   restore of one plan that works (`restore_scope` is `[None]`, never empty), so if that restore empties the copy the
   journal counts as unknown until the next tick's `Restore { plan: None }` answers, though the restore just read the
   file. Effects in that window, at most one tick (10 to 30 s): a failed restore shows no `RestoreFailed`, the
   2-minute clock starts, and a lid close sets the window lock (with nothing of Wakeshot's in force, so at worst one
   extra lock later). The model does the same, so no property sees it. Architecture "A journal that cannot be read"
   lists only the two intended cases. Left as is: rare and harmless as far as read; a core card could end the retry
   when a restore that worked leaves the copy empty.

## Log

- 2026-11-04 15:23:59 (clock): started. Read the card, `p2-6-reset-gaps` (outside scope 2 and 3, Log, Result),
  `p2-6-props-flake` (Log, Result), the plan's "How we work". Free memory 35 GB.
- About 15:26: the three shrunk cases written from the words of `p2-6-reset-gaps` as temporary tests in `props.rs`;
  all three fail in debug with the exact messages recorded there (44000 ms until 55000 ms; 39000 ms until 40000 ms;
  Hibernate after 10000 ms at 310000 ms). Next: read each trace.
- About 15:29: traces read against `core/settings.rs` (`done_restored`, `restore_scope`, `journal_unknown`) and
  `core/lid.rs` (`take_lid`, `settle_window_lock`, `take_input_idle`, `check_countdown`, `done_sleep_check`).
  Deliverable 1: the model is wrong, as first read. The plan switch's restore of the other plan fails; the tick's
  retry (scoped, the core applies) works and leaves no entry on another plan, so the core ends the retry
  (`done_restored`: `Ok` and `restore_scope()` empty), as architecture "A failed `Restore`" says (the hold's apply
  takes over the active plan's entries). The model keeps `retry` for any scoped restore, so after the user's LidDc
  change empties the mirror it thinks the journal unknown at the lid close and owes the window lock. The core's
  journal is known then (the restore read it), the close is on the user's own Sleep, and the core sets no window close.
  Deliverable 2: the same model gap, not a core one. Trace: the work end's restore fails with a known mirror; a new
  hold; the user switches plan; the switch's restore of the old plan works and empties the copy (core: retry over,
  scope empty); the new plan's apply fails its safety net (requests only); the lid closes on the user's own Sleep;
  the model again thinks the journal unknown (retry, empty mirror) and owes the window lock when a later hold applies
  Do nothing. Product rule 7 makes the window lock owed only for a close while the journal is unknown; outside it a
  close on the user's Sleep sleeps the laptop (Windows does that, the world does not model it), and a hold that starts
  with the lid already closed is no lock trigger in architecture "Episode flow" (someone on an external screen).
  Deliverable 3: a core gap. The sleep check goes out at 300 s; input at 300 s; the reading at 310 s shows the drop
  while the check is out (`check_countdown` waits while `hibernating`, the drop is used up); the answer `Ok` (the
  thread checked before the input) comes after it, `write_short` cannot write (session ending), so the fallback is due
  at once and the next reading sends `Hibernate` 10 s after input. With the session not ending the same order writes
  the 1-minute timeouts right after input, which product's work end rule takes back at input.
- 15:31:14 (clock; the two lines above first said 15:30 and 15:40, guesses, now corrected): fixes in. Model
  (`props.rs`): a restore of one plan that worked ends `retry` when the model applies and the mirror names no entry
  on a plan other than the active one (`Model::nothing_left_to_retry`, from architecture "A failed `Restore`"); the
  `retry` doc says so. Core (`core/lid.rs`): `done_sleep_check` turns a pass (or an unknown lid) into
  `InputSeen` when the last input reading, clamped at a resume or an unlock as the countdown counts, is under 5
  minutes (`Core::input_since_due`); one log line says so. Decided by the worker, for Haik's review: read the last
  reading rather than keep a flag for input seen while the check is out; no new state, and a resume or an unlock
  after the check counts the same. The three cases are named tests in "Shrunk cases kept as named tests"; one
  scenario in `scenarios/lid.rs`. Core lib tests (debug, 4 threads): 675 passed, 1 ignored.
- About 15:33: the scenario also covers an unlock while the check is out (no reading in between; the clamp makes it
  input, so no 1-minute timeouts right after a sign-in) and an unknown lid. Hand mutants (scratch script, one at a
  time, each file put back and its hash checked): 7 of 7 killed (15:33:20 to 15:34:46 from the script's clock): no
  input guard; no `LidUnknown` in it; `<=` for `<` (an existing unread-journal scenario fails); no clamp;
  `LidOpen` for `InputSeen`; the model without `nothing_left_to_retry`; the model's rule as "mirror empty". Not run:
  the model's rule without its `applies()` filter (no named case reaches a scoped restore while the core does not
  apply).
- 15:36:00 (clock): `cargo fmt --all --check` asked for one line break in `core/lid.rs`; rustfmt on the three changed
  files, then exit 0. Clippy x64 on the workspace, with and without `--all-targets`, `-D warnings`, `-j 8`: exit 0.
- About 15:38: release test binary of the core only (`-j 8`); long runs started by a scratch script: the whole core
  test binary, `PROPTEST_CASES=4096`, one test thread, a random `PROPTEST_RNG_SEED` per run written to the log (so a
  failure replays), free memory checked before each run. `docs/architecture.md` "The sleep route" item 1: one sentence
  on a pass that comes after input.
- 15:37:54 to 15:50:20 (script clock): the whole core test binary at 4096 cases, 10 runs, random seeds 1527431071,
  618099157, 1376596991, 313561811, 1117139602, 947922590, 374311577, 731538138, 316918203, 830391566: 10 of 10 passed
  (675 passed, 1 ignored each; 52 to 88 s each, one test thread); free memory 35 to 36.6 GB. 15:50: the same binary at
  256 cases, 50 runs, started.
- 15:50:27 to 15:53:59 (script clock): 256 cases, 50 runs, random seeds in the log (first 285872043, last
  1197684361): 50 of 50 passed (675 passed, 1 ignored each; 4 to 5 s each, one test thread); free memory 36.2 to
  36.6 GB. No other property failed, so no further case to catch.
- 15:54 to 15:56:05 (clock): Haik's plan read before and after (read only: `powercfg /getactivescheme` and `/qh`):
  Balanced active, lid Sleep (1) AC and DC, sleep after 18000 s AC and DC, screen off 300 s AC and 180 s DC, the
  same both times. `cargo test --workspace --locked` (debug, `-j 8`): exit 0, core 675 passed and 1 ignored, every
  other binary passed (the e2e and smoke tests ignored as always, so no tray started). `cargo fmt --all --check` exit 0
  again. `docs/research.md` "Three rare core property failures" added. 15:56:35: build folder and every scratch file
  of this card deleted; no process of mine left.

## Result

**Deliverable 1, `a_closed_laptop_never_runs_on_battery_on_our_do_nothing_unlocked_past_30_s`: the model was wrong.**
The core ends the restore retry when a scoped restore works while it applies and no entry is left on another plan
(architecture "A failed `Restore`": the hold's apply takes over the active plan's entries). The model kept it, so with
an empty mirror it thought the journal unknown at a lid close on the user's own Sleep and owed a window lock the core
rightly never owed. Fix: the model ends the retry the same way (`Model::nothing_left_to_retry`).

**Deliverable 2, `a_closed_laptop_is_locked_also_before_the_first_plan_read`: the same model gap, no core change.** A
plan switch's restore that worked and left nothing ended the core's retry; the model's did not. By product rule 7 the
window lock is owed only for a close while the journal is unknown; a hold that starts with the lid already closed is
no lock trigger (a user on an external screen). Decided by the worker, for Haik's review.

**Deliverable 3, `no_hibernate_within_5_minutes_of_a_resume_or_of_input`: a core gap, fixed in the core.** Input read
while the sleep check was out was used up; the check's older pass then wrote the 1-minute timeouts right after input,
or, where they cannot be written, made the fallback due at once (`Hibernate` 10 s after input). Now a pass that comes
when the last input reading, clamped at an unlock or a resume, is under 5 minutes counts as a skip for input
(`Core::input_since_due`). Decided by the worker, for Haik's review: read the last reading, not a new flag. Change it
when input stops coming only through readings.

**Files:** `crates/wakeshot-core/src/core/lid.rs` (`done_sleep_check`, `input_since_due`),
`crates/wakeshot-core/src/scenarios/props.rs` (model rule and its doc, three named tests),
`crates/wakeshot-core/src/scenarios/lid.rs` (one scenario: input, unlock, unknown lid), `docs/architecture.md` (one
sentence, "The sleep route" 1), `docs/research.md` (one entry), this card.

**Checks:** `cargo fmt --all --check` exit 0. Clippy x64, workspace, with and without `--all-targets`, `-D warnings`:
exit 0. `cargo test --workspace --locked` (debug): exit 0, core 675 passed, 1 ignored. Core at 4096 cases: 10 of 10
runs passed; at 256 cases: 50 of 50 passed (random seeds, logged). Hand mutants: 7 of 7 killed. Haik's plan never
written, read back unchanged; no tray started; build folder deleted.

**Open:** "Found outside scope" 1 (inferred, not verified): a short window where the core counts a known, empty
journal as unknown.
