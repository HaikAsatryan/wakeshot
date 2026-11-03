# Core state machine, part 1: start, holds, requests, view, Sim harness

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens. Split trigger (review
m13: no margin): if the worker passes about 120K before the `Sim` harness is done, it stops at a clean point and the
`Sim` harness (`sim.rs`) becomes its own card, run before `p2-core-settings`.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-holds\target`.
Runs after: `p2-policy-holds`, `p2-policy-power`, `p2-present`, and the sources track's `p4-hold-files`
(`holds::file::format_expires`). Parallel with: `p2-ownership-apply`, and any card of another track.

## Goal

The first part of `Core::handle`: the state, startup, holds from agents and hold files, Automatic, Keep awake, power
requests, the tray view and `state.json`, deadlines, and the `Sim` harness the later core cards test with. Settings
(apply, restore) and the lid rules come in the next two cards; leave clear seams for them.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (all track notes).
- `docs/product.md` "What keeps the PC awake", UX.
- `docs/architecture.md` section 3 (all; this card builds the parts named below), section 4 "Tray threads" (what
  runs where), section 6 (`config.json`, `state.json`, holds), section 8 (rows for requests, liveness, format), section
  9 core.
- `crates/wakeshot-core/src/` (all contract files); Results of `p2-policy-holds`, `p2-policy-power`, `p2-present`.

## Owns

`crates/wakeshot-core/src/core.rs` (bodies and private fields; the three public signatures are contract),
`state.rs`, `sim.rs`, `scenarios/holds.rs`; private submodules under `src/core/` if the card makes them. Nothing else.

## Deliverables

1. `state.rs`: the state of architecture section 3 ("State is one struct"), every field declared now, including those
   the next cards fill (journal mirror, released keys, lid, floor, countdown), so they do not reshape it.
2. `core.rs`, events handled here:
   - `Started`: config (its `config` wins over `Core::new`'s), journal mirror, owner, console, tray; `first_run` gives
     `Notify(FirstRun)` and `WriteConfig`.
   - `ConfigChanged`; `Menu(Automatic)` (gives `WriteConfig`); `Menu(KeepAwake)` and `Menu(KeepScreenOn)` (give
     `WriteHold` for `keep-awake.json` with the tray's pid and start from `Started.tray`, `expires` from
     `holds::file::format_expires`, `screen`; or `DeleteHold(keep-awake.json)`); `Menu(OpenLog)`.
   - `Sources` and `Holds` through `policy::sessions` and `policy::hold_files`: per-session memory (last liveness, first
     seen waiting); `DeleteHold` for a Gone hold file; Unknown keeps the last answer; "update Wakeshot" once per start
     per agent, a `Log` line at most once a minute per source.
   - Holding starts and ends: `Requests` through `policy::requests`, a `Log` line with the labels. A seam (one
     function each) where `p2-core-settings` adds the apply and the restore.
   - `Resumed`: `Retake` if any request is on. `Done(Requests(Err))` (also the answer to `Retake`): `Notify(CannotHold)`
     once per episode.
   - `Tick` and `next_deadline`: the nearest hold deadline (timer windows, waiting hour, `expires`, Keep awake end) and
     the 1 s `state.json` debounce.
   - On every change the user can see: `Tray(view)` and, debounced, `WriteState`, through `present`.
3. `sim.rs` (behind `test-support`): `Sim` with a `FakeClock`; send an event at the current time, get the commands;
   `run(ms)` delivers `Tick` at each `next_deadline` and every 30 s (the safety tick); `suspend(ms)` and `set_wall`;
   builders for common events (a Claude session, a hold file, `Started`); a record of every step for failure messages.
   Later cards extend it.
4. `scenarios/holds.rs`, written before the code, one named test per case: start with and without a config file; a
   busy session holds and ends; a timer window opens and closes on time across a suspend and a wall clock jump; a
   waiting session with Remote Control ends after 1 hour; Unknown liveness keeps a hold; a Gone hold file is deleted;
   Automatic off keeps only Keep awake; Keep awake 30 minutes ends on time; Keep screen on asks for the display;
   "update Wakeshot" once per agent per start; `Retake` after resume; a failed request shows `CannotHold` once; the
   tooltip names what holds; `state.json` writes at most once a second.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::holds` and `cargo test --locked -p wakeshot-core`: all pass; list the
  test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- `Core::handle` never panics on any event in any order: a short `proptest` in `scenarios/holds.rs` over random
  events of this card's kinds (the full properties are `p2-core-props`).
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the owned
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- Apply, restore, plan switch, user changes, session end, console, settings owner, Quit, Restore my power settings
  (`p2-core-settings`). Lid, lock, floor, countdown, closed-lid report (`p2-core-lid`).
- Policies and the view (done; a bug found goes under "Found outside scope" with a failing test name).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`: proposals go to the Result.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

1. `ownership/apply.rs` (`p2-ownership-apply`, running beside this card): at 21:00 `cargo clippy -p wakeshot-core
   --all-targets -- -D warnings` failed on two lints there (`unnecessary use of clone`, line 676; `very complex type`,
   line 864). Gone at the 21:20 rerun and at the final run. Not a standing problem.
2. `docs/phases/02-core.md` card table: `p2-policy-holds` still reads "running (19:55)" though its card is done.
   For the orchestrator.
3. Not this card's, seen in `docs/research.md` ("Power rules checks"): the dock's two AC drops lasted 5.3 s and
   6.3 s, longer than the 5 s settle in `policy::lid`. Already `p2-policy-power`'s question to Haik.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:12, orchestrator, from `p2-present`'s Result: keep the Keep awake length the user picked. The
  policy's `keep_awake().choice` is the minutes left; passed to the present snapshot as is, the menu checks no length.
  Read the Results of `p2-policy-holds`, `p2-policy-power`, `p2-present` and `p4-hold-files` for their APIs.
- 2026-11-03 20:16, worker: started. Reading the contracts, the policy and present Results, the design sections.
- 2026-11-03 20:40: design set. One settle pass after every event (facts in, then holds, requests, episode edges,
  view, deadline); state in `state.rs`; private submodules `core/holds.rs` and `core/view.rs`. Keep awake: a memo
  "file with this `expires` stands for this length", set by the menu pick and at first sight of a file the tray did
  not write. `cargo tree` baseline taken (serde, serde_json). Next: scenario tests first.
- 2026-11-03 21:05: `state.rs`, `sim.rs`, `core.rs`, `core/holds.rs`, `core/view.rs` and 35 tests in
  `scenarios/holds.rs` written; all pass. One test bug found by the trace (it read `state.json` inside the 1 s
  throttle). Lib clippy clean; `--all-targets` clippy stops on two lints in `ownership/apply.rs`
  (`p2-ownership-apply`, still running): wait and rerun.
- 2026-11-03 21:25: 5 more tests (journal at start, console, first sight of a wait, the Automatic gate on the
  notice, Keep awake Off by the file's own name): 40 in `scenarios::holds`, 332 in the crate, all pass. Rerun of
  clippy (four runs) clean: the `apply.rs` lints are gone. Hand-planted mutants running in a scratch copy.
- 2026-11-03 21:40: mutant round 1: 11 of 13 caught. "No deadline at all" survived: every checked end sat on the
  30 s safety-tick grid, so the safety tick alone hit it. Timing tests moved 7 s off the grid (`OFF_THE_TICK`); one
  mutant pattern fixed after rustfmt. Round 2 running.
- 2026-11-03 21:55: round 2: 13 of 13 caught. Checked in the scratch copy that a read of an `expect(dead_code)`
  field fails `-D warnings`. Copy and its build folder deleted. Long property run passed. Final checks all green.

## Result

What was done, files changed, checks and outcomes, open items. Describe the seams and the `Sim` API for the next cards.

Status: done 2026-11-03. The first part of `Core::handle` is built, with the `Sim` harness and 40 hold scenarios.

### Files changed

- `crates/wakeshot-core/src/core.rs` (bodies and one private field; the three public signatures unchanged).
- `crates/wakeshot-core/src/core/holds.rs`, `core/view.rs`: new private submodules (the card allows them; the next
  core cards own them too).
- `crates/wakeshot-core/src/state.rs`, `sim.rs`, `scenarios/holds.rs`.
- `docs/research.md`: "Core state machine checks" added at the end. This card.

### How the core works

Each `handle` takes the event's facts into `State`, then runs one settle pass: holds (`policy::sessions`,
`policy::hold_files`), "update Wakeshot", requests (`policy::requests`), `Retake` after `Resumed`, episode edges
(log lines and the two seams), the view and `state.json` (`present::build`, sent only on a change), and the next
deadline. The pass reads only the state, so a `Tick`, a late event or a repeated snapshot give the same answer.
Events before `Started` are dropped; a second `Started` is ignored.

### Seams for `p2-core-settings` and `p2-core-lid`

- `Core::holding_starts` / `Core::holding_ends`: one function each, called after the `Requests` change. Add the
  apply and the restore there.
- `Core::notify`: every notice goes through it, so `p2-core-lid` can hold notices while locked (note 18).
- `Core::take`, `Core::menu`, `Core::done`: the arms not built here are listed by name with their card in a comment
  (`SettingsOwner`, `Plan`, `ActivePlan`, `UserChanged`, `JournalCorrupt`, `Lid`, `Power`, `InputIdle`, `Suspending`,
  `Session`, `UpdateReady`, `Updated`, `Menu(Restore)`, `Menu(Quit)`, `Applied`, `Restored`, `Locked`, `Hibernate`,
  `Update`, `Written(RestoreRequest)`). No wildcard arm, so a new event fails the build.
- `settle` already reads `stepped_back` and `on_console` (requests), `requests_only_run`, `episode.requests_only`
  and the journal mirror (view, `state.json` `overridden`). Fill those and the view follows.
- `state.rs` declares every field of architecture section 3 plus the next cards' (owner, plan, leftover, session end,
  lid, AC, battery, floor, source lock, countdown, last resume, locked, idle lock, held notices, `released`,
  `lid_closed_since_mono`). Each unused one has `#[expect(dead_code, reason = "<card> fills it")]`: remove it when the
  field is read, or `-D warnings` fails (checked, `docs/research.md`). `Episode.held_by` (kind plus label of all
  that held) is ready for the closed-lid report.

### `Sim` API (`crate::sim`, behind `test-support`)

- `Sim::new()` (at `START`, `Core::new(Config::default())`), `Sim::with(config, start)`; `start()` sends
  `Start::default()`.
- `send(event) -> Vec<Command>` at the current time. After a `Tick` it panics with the trace if `next_deadline` is
  not ahead of now, so every scenario checks that.
- `run(ms) -> Vec<Command>`: a `Tick` at each loop timer, each at its moment: the core's deadline, armed after every
  event as a wait in run time (so a `Wall` deadline comes early or late after a clock change, as on the real loop),
  and the 30 s safety tick (phase from the start).
- `suspend(ms)`: `Suspending`, only the wall clock moves, `Resumed`. `set_wall(wall_ms)`: no event.
- Hold folder world: `put_hold(facts)`, `remove_hold(name)` (a script changes it, then `Holds` is posted);
  `watch()` runs every `WriteHold` / `DeleteHold` the core sent, answers each with `Done(Written Ok)`, posts `Holds`.
- Reads: `now()`, `deadline()`, `steps()` (event, time, commands), `commands()`, `sent_at(&command)`, `view()`,
  `requests()`, `notices()`, `state_writes()`, `state_file()`, `trace()` (one short line per step and clock move).
- Builders: `Start { .. }.event()`, `claude(pid, activity)`, `sources(vec)`, `hold(name, alive)`,
  `watched(id, &HoldFile, alive)` (through JSON and the real parser). Constants `START` (a whole second), `TRAY`,
  `SAFETY_TICK_MS`, `KEEP_AWAKE`.
- Lesson for timing tests (`docs/research.md`): check ends off the 30 s grid (`OFF_THE_TICK` in
  `scenarios/holds.rs`), or the safety tick hides a missing deadline.

### Design choices (mine), each with when to change it

1. Keep awake menu check: a memo "the file whose `expires` is X stands for length L", set by the menu pick (to the
   `expires` the parser will read back) and at first sight of a file the tray did not write
   (`present::keep_awake_choice`). The check follows the file that holds, so it shows the pick once the watch thread
   posts the file (milliseconds). Change it when that delay is visible.
2. Keep screen on rewrites `keep-awake.json` with the same `expires`; ignored while Keep awake is off. A new length
   keeps the screen as it is; Off deletes the file the snapshot names (any case), else `keep-awake.json`.
3. A Gone `keep-awake.json` is not deleted after this tray wrote one in this run: such a snapshot can only predate
   the write, and the delete would remove the new file. Change it when `HoldFacts` carries the pid.
4. "Update Wakeshot" waits while Automatic is off, and comes when it turns on. The format log line does not wait.
   Change it when Haik wants the notice whatever Automatic says.
5. `CannotHold` only while the system request is wanted; a failed release gives none. Known limit: a failed release
   whose answer lands after a new hold started reads as a failed hold. Change it when a log shows that: keep a queue
   of in-flight request states.
6. The icon shows Holding as soon as `Requests` goes out; a failed answer turns it Idle until a later success.
7. `state.json`: the first change after a quiet second at once, later ones at the end of that second (run time).
   A failed write is sent again at the next step.
8. Log lines: `Holding starts: ...`, `Hold starts: ...`, `Hold ends: ...`, `Holding ends; held by ...`; a format
   problem as `<label>: format problem: ...`, at most once a minute per source (run time), unknown values cut to 40
   characters and quoted, so no control character splits a line.
9. Agent deadlines are dropped while Automatic is off (they change nothing).
10. `Menu(Automatic)` writes `config.json` only on a change; `ConfigChanged` never writes.

### Acceptance checks (final run 2026-11-03 21:55, own build folder, every cargo command `--locked`)

- `cargo test -p wakeshot-core scenarios::holds`: 40 passed. `cargo test -p wakeshot-core`: 332 passed, 0 failed.
  Tests: `a_dead_trays_keep_awake_file_is_deleted`, `a_failed_release_shows_no_notice`,
  `a_failed_state_write_is_sent_again`, `a_hold_file_holds_until_its_process_is_gone`, `a_hold_file_stops_at_expires`,
  `a_journal_at_start_shows_as_overridden`, `a_keep_awake_file_seen_gone_after_this_tray_wrote_it_is_not_deleted`,
  `a_second_started_changes_nothing`, `a_session_that_leaves_the_snapshot_ends_its_hold`,
  `a_sleep_past_the_window_ends_the_hold_at_resume`, `automatic_from_the_config_file_writes_no_config`,
  `automatic_off_keeps_only_keep_awake`, `busy_session_holds_and_ends`, `events_before_started_are_dropped`,
  `failed_request_shows_cannot_hold_once`, `format_problem_logs_at_most_once_a_minute_per_source`,
  `gone_hold_file_is_deleted`, `handle_never_panics_on_any_order` (proptest), `keep_awake_30_minutes_ends_on_time`,
  `keep_awake_from_the_cli_checks_the_length_that_covers_it`, `keep_awake_off_deletes_the_file_by_the_name_it_has`,
  `keep_awake_picked_again_starts_its_time_again`, `keep_screen_on_asks_for_the_display`,
  `off_the_console_keep_screen_on_takes_no_display`, `open_log_opens_the_log`, `retake_after_resume`,
  `starts_with_a_config_file_and_its_config_wins`, `starts_without_a_config_file`,
  `state_json_writes_at_most_once_a_second`, `timer_window_ends_two_minutes_after_due`,
  `timer_window_opens_and_closes_on_time_across_a_suspend_and_a_wall_jump`, `tooltip_names_what_holds`,
  `unknown_liveness_keeps_a_hold`, `unknown_liveness_never_deletes_a_hold_file`,
  `unknown_status_counts_as_working_with_the_notice`, `update_wakeshot_once_per_agent_per_start`,
  `update_wakeshot_waits_while_automatic_is_off`, `waiting_session_with_remote_control_ends_after_one_hour`,
  `waiting_without_a_time_counts_from_first_sight`, `waiting_without_remote_control_does_not_hold`.
- Never panics: `handle_never_panics_on_any_order`, random sequences of up to 40 steps (every event kind of this
  card, outcomes, run, suspend, wall jumps; with and without `Started` first); it also checks the 1 s `state.json`
  rule and, through `Sim`, that no deadline lies behind a `Tick`. Long run: `PROPTEST_CASES=10000`, passed in 8.9 s.
  No regression file: `failure_persistence: None` (`docs/research.md`).
- `rustfmt --edition 2024 --check` on the six owned files: no diff.
- `cargo clippy -p wakeshot-core --target <t> -- -D warnings`, with and without `--all-targets`, for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0. Also with `--features test-support`
  (x64): exit 0.
- `cargo tree -p wakeshot-core -e normal`: identical to the baseline (serde, serde_json). Grep of the owned files
  for `std::time`, `std::fs`, `std::env`: none.
- Only the owned files changed (from my own edit record; git not run), plus this card and the `research.md`
  addition.
- Not asked, done: 13 hand-planted mutants in a scratch copy (note 20), all caught after one test fix (Log 21:40).
  Copy, its build folder and this card's build folder deleted.

### Contract proposals

None needed. Optional, for decision 5's limit: `Outcome::Requests` could carry the state it answers, so a failed
release is told from a failed hold.

### Open items

None for Haik. Next: `p2-core-settings` builds on the seams and `Sim` above.
