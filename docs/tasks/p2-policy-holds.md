# Hold rules: agent sessions and hold files

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-policy-holds\target`.
Runs after: the contracts card. Parallel with: `p2-ownership-restore`, `p2-policy-power`, `p2-present`, and any card
of another track.

## Goal

The pure rules that turn agent session facts and hold file facts into holds, with the time each answer next changes,
so the state machine (`p2-core-holds`) only stores and combines them. Parsers never decide policy (section 7); these
rules do.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 12, 14, 17).
- `docs/product.md` "What keeps the PC awake" and UX "Automatic", "Keep awake".
- `docs/architecture.md` section 3 "Clocks"; section 6 hold format and the bullets under it; section 7 (all);
  section 5 "Liveness" (only Yes, Gone, Unknown).
- `docs/research/claude-code-signals.md` "Recommended detection rules" (only to understand the facts the parser
  gives; parsing is the sources track's).
- `crates/wakeshot-core/src/`: `sources.rs`, `holds.rs`, `process.rs`, `time.rs`, `policy/` stubs.

## Owns

`crates/wakeshot-core/src/policy/sessions.rs`, `policy/hold_files.rs`. Nothing else.

## Deliverables

1. `policy/sessions.rs`: for one session (`SessionFacts`, the adapter's `Alive`, and what the core remembers about it:
   last liveness answer, the wall time it first saw the session waiting) and `Now`: does it hold, its label, whether
   it needs the "update Wakeshot" notice, and the next `Deadline` at which the answer changes. Rules:
   - `Alive::Gone`: no hold. `Unknown`: the last answer stands (the caller passes it).
   - `Working` holds while the process lives; `open_tasks > 0` holds while the process lives (decision 8).
   - A timer holds from its due time minus 1 hour to its due time plus 2 minutes (decision 6, margin from section 7).
   - `Waiting` holds only with `remote`, until `since_wall_ms` (or the first time the core saw it waiting) plus 1 hour
     (decision 7).
   - `FormatHealth::Unreadable`: no signal for that session, notice wanted. `UnknownValue`: counts as working while
     the process lives, notice wanted (decision 37).
   - Wall time can jump either way (`FakeClock::set_wall`): a deadline in the past is due now; a rule never holds
     forever because of a jump.
2. `policy/hold_files.rs`: for one `HoldFacts` and `Now` and Automatic: does it count, its label, its screen wish,
   whether to delete the file (`Alive::Gone`), and its next `Deadline` (`expires`). Rules (section 6, decision 11,
   32): counts while alive and before `expires`; Unknown keeps the last answer; Automatic off stops `Run` and
   `Script` but not `KeepAwake`; `screen` counts only for `KeepAwake`. Also the current Keep awake choice for the menu
   from the `keep-awake.json` facts (Off, a number of minutes left, Until I turn it off).
3. Unit tests, written first, one named test per rule and edge: each activity; each liveness answer; a timer just
   before its window, inside it, at its end, past it; two timers; waiting with and without `remote`, with and without
   `since_wall_ms`, at the hour; a wall clock jump back and forward; each `FormatHealth`; each hold kind with
   Automatic on and off; expiry exactly at `expires`; Gone gives a delete; Unknown keeps the last answer.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core policy`: all pass; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the two
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the two owned files changed.

## Out of scope

- Parsing status files, transcripts, hold files and `expires` (sources track). Building the Keep awake file
  (`p2-core-holds`). Labels as words and their grouping in the tooltip (`p2-present`, `text.rs`).
- State, events and commands (`p2-core-holds`). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

1. Track note 20 (`docs/phases/02-core.md`, mutants): the list of folders to copy lacks `docs\`. `holds/file.rs`
   (`p4-hold-files`) now reads `docs/hold-files.md` at compile time, so a copy without `docs\` fails to build
   (`couldn't read ...docs/hold-files.md`, seen 2026-11-03 20:50). Proposal: add `docs\` to the list.
2. `docs/architecture.md` section 7, "Waiting limit": says wall time only. This card adds a run-time cap (decision 3
   in the Result). Proposal: add "and at most 1 hour of run time after Wakeshot first saw the wait, so a clock set
   back cannot stretch it".
3. For `p2-present` and `p2-core-holds`: `keep-awake.json` gives only the time left, not the pick (the CLI writes the
   file too). `hold_files::keep_awake` returns `Minutes(left)`, such as `Minutes(25)`, which matches no menu item
   (30, 60, 120, 240). Which item to check then (none, or the smallest pick at or above the minutes left) is
   `present`'s call; the core may also remember the last pick from `Menu(KeepAwake)`.
4. During this card, `p2-policy-power`'s files were mid-edit: one build failed (`lock.rs:59` imported a missing
   `ending::clamped_idle_ms`), then 59 of its tests failed on `todo!()` and clippy failed on unused variables in
   `lid.rs` and `lock.rs`. At the final run (21:00) everything passed; nothing left to report.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started; reading the card, docs and the contract files.
- 2026-11-03 19:29 to 19:45: first run stopped by Haik before any code.
- 2026-11-03 19:55: second worker started fresh. Owns only the two policy files and this card; no phase file edits.
- 2026-11-03 20:05: read the docs and contracts. `cargo tree -p wakeshot-core -e normal` recorded before any change
  (serde, serde_json only). Design: pure `decide` functions that also return the memory to store; the next deadline
  is found by checking the rule at each time edge, so it is the exact time the answer changes. Writing tests first.
- 2026-11-03 20:20: tests written against `todo!()` bodies; all failed as expected. Both files implemented.
- 2026-11-03 20:25: `cargo test -p wakeshot-core` does not build: `policy/lock.rs:59` (owned by `p2-policy-power`,
  mid-edit) imports `policy::ending::clamped_idle_ms`, which does not exist yet. Not mine; waiting, then retrying.
- 2026-11-03 20:35: the build works again. My 78 tests pass. 59 tests fail, all in `ending`, `floor`, `lid`, `lock`,
  `requests` (`p2-policy-power`, still `todo!()` bodies). Clippy fails only on `lid.rs` and `lock.rs` (unused
  variables in their stubs); nothing in my files. `cargo tree` unchanged; the grep finds no clock, file or env use.
- 2026-11-03 20:40: added a test for a wait whose `since` is ahead of the clock. Long run: 79 pass with
  `PROPTEST_CASES=20000` in 0.84 s. Next: mutants in a scratch copy.
- 2026-11-03 20:55: mutants (22, scratch copy, track note 20): 21 killed. The survivor, the `ms > 0` filter in
  `ahead_of`, was dead code (an edge at `now` never changes the answer), so it is removed. The first copy did not
  build: `holds/file.rs` now reads `docs/hold-files.md` at compile time, and note 20's list has no `docs\`. Copied
  `docs\` too. Mutant folders deleted. Next: final checks.
- 2026-11-03 21:00: final checks all pass (the other card's files are done now). Result written; build folder
  deleted. Status done.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API for `p2-core-holds`.

Done 2026-11-03. Pure hold rules for agent sessions and hold files, with exact next deadlines and 79 tests.

Files changed: `crates/wakeshot-core/src/policy/sessions.rs`, `crates/wakeshot-core/src/policy/hold_files.rs`, and
this card. Nothing else.

### Public API for `p2-core-holds`

`policy::sessions`:

- `TIMER_LEAD_MS` (1 h), `TIMER_MARGIN_MS` (2 min), `WAITING_LIMIT_MS` (1 h).
- `SessionMemory { alive: Alive, waiting: Option<WaitSeen> }`, `Default` (alive `Unknown`, no wait).
  `WaitSeen { since_wall_ms: Option<i64>, wall_ms: i64, mono_ms: u64 }`.
- `decide(facts: &SessionFacts, alive: Alive, memory: &SessionMemory, now: Now) -> SessionAnswer`.
- `SessionAnswer { holds: bool, label: &'static str, update_notice: bool, next: Option<Deadline>, memory }`.

How the core uses it: keep per `SessionKey` the last `(SessionFacts, Alive)` and the `SessionMemory`. On `Sources`
and on each `Tick`, call `decide` and store `answer.memory`. A new session starts from `SessionMemory::default()`;
drop the memory when the session leaves the snapshot. Count it as `answer.holds && kind_counts(HoldKind::Agent,
automatic)`. `update_notice` feeds "update Wakeshot" once per start per `label`. `next` is the exact time `holds`
changes with the same facts, always after `now`; take the nearest over all answers.

`policy::hold_files`:

- `kind_counts(kind: HoldKind, automatic: bool) -> bool`: the one place for the Automatic rule.
- `decide(facts: &HoldFacts, last_alive: Alive, automatic: bool, now: Now) -> HoldAnswer`.
- `HoldAnswer { counts, kind, label: Option<String>, screen, delete, next: Option<Deadline>, alive: Alive }`.
- `keep_awake(facts: Option<&HoldFacts>, last_alive: Alive, now: Now) -> KeepAwakeNow`.
- `KeepAwakeNow { choice: KeepAwake, next: Option<Deadline> }`.

How the core uses it: keep per `HoldId` the last `answer.alive` (start from `Alive::Unknown`) and pass it back as
`last_alive`. `delete` means send `DeleteHold(id)`. `label` is the file's reason, `None` for the kind's fallback in
`present`. `keep_awake` takes the facts of `keep-awake.json` (`None` when absent) and gives the menu choice and
minutes left; its `next` is the next minute boundary, so the tooltip's "25 min left" stays right.

### Design choices (mine, with triggers)

1. Unknown liveness keeps the last `Yes` or `Gone`, and the current facts still apply (a session that goes idle
   under Unknown stops holding). Reason: "Unknown keeps the last state" is about the process; the file was still
   read. Change it when a failed liveness check also makes the facts stale.
2. A session or hold file seen first with Unknown does not hold until its first `Yes`. Reason: decision 12 (every
   signal checked against a fact); the safety tick retries in 30 s. Change it when Unknown on first sight shows up
   in live tests.
3. The waiting hour runs on the wall clock and is also capped at the hour left at first sight, counted in run time
   (`mono_ms`). While the clock runs true both give the same end; after a clock set back, the cap ends it, so no jump
   stretches it. Ties go to the `Wall` deadline, which fires at resume after a sleep. Change it when Haik wants the
   hour to follow the wall clock alone.
4. A new `since_wall_ms` starts a new hour; turning Remote Control on mid-wait does not. Change it when a live test
   shows Claude writes a new `statusUpdatedAt` when Remote Control turns on.
5. Timers hold in any activity, also while waiting without Remote Control (track note 12 gives no activity
   condition). Change it when a live test shows a timer does not fire while the session waits on a prompt.
6. An unreadable file gives no signal, keeps the wait memory, and wants the notice whatever the liveness (an
   unreadable file may give no process to check).
7. Hold files follow the wall clock only (`expires` is a clock time; section 3). A clock set back makes a hold with
   `expires` last longer by the jump; its process still bounds it. Change it when Haik's checks show a wrong time
   left after a clock change: then cap it in run time like the waiting hour.
8. Deadlines are exact: the rule is checked at each time edge and edges that change nothing are skipped, so no Tick
   is wasted.

### Checks (final run 2026-11-03 21:00, build folder above)

- `cargo test --locked -p wakeshot-core policy`: exit 0, 138 passed (79 are this card's).
- `cargo test --locked -p wakeshot-core`: exit 0, 246 passed, 0 failed.
- `rustfmt --edition 2024 --check` on the two files: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- `cargo tree --locked -p wakeshot-core -e normal`: the same as before the card (serde, serde_json).
- Grep of the two files for `std::time`, `std::fs`, `std::env`, `SystemTime`, `Instant`: none.
- Long property run: `PROPTEST_CASES=20000`, 79 passed in 0.84 s.
- Mutants in a scratch copy (track note 20): 22 planted, 21 killed; the survivor was dead code and is removed.
  Copy and its build folder deleted.
- Only the two owned files and this card changed (my own edits; git not run).

Tests in `policy::sessions` (44): `a_new_wait_starts_a_new_hour`, `extreme_times_do_not_overflow`,
`gone_after_yes_stops_holding`, `gone_does_not_hold`, `idle_does_not_hold`, `label_comes_from_the_source`,
`leaving_waiting_forgets_first_seen`, `next_deadline_is_where_the_answer_changes` (property),
`ok_format_wants_no_notice`, `open_task_holds_while_idle`, `open_task_holds_while_waiting_without_remote`,
`timer_holds_inside_its_window`, `timer_holds_just_before_its_window_ends`,
`timer_just_before_its_window_does_not_hold`, `timer_past_its_window_does_not_hold`,
`timer_window_ends_two_minutes_after_due`, `timer_window_opens_one_hour_before_due`,
`timer_windows_that_touch_hold_through`, `turning_on_remote_control_does_not_restart_the_hour`,
`two_overlapping_timers_hold_until_the_later_ends`, `two_timers_apart_hold_in_turn`,
`unknown_before_any_answer_does_not_hold`, `unknown_keeps_the_last_gone`, `unknown_keeps_the_last_yes`,
`unknown_still_follows_new_facts`, `unknown_value_gone_does_not_hold_but_wants_the_notice`,
`unknown_value_holds_while_alive_with_a_notice`, `unreadable_format_gives_no_signal_and_a_notice`,
`unreadable_format_keeps_the_waiting_first_seen`, `unreadable_format_wants_the_notice_whatever_the_liveness`,
`wait_since_ahead_of_the_clock_holds_at_most_an_hour`, `waiting_seen_late_holds_only_the_rest_of_the_hour`,
`waiting_with_remote_ends_at_the_hour`, `waiting_with_remote_holds_just_before_the_hour`,
`waiting_with_remote_holds_until_an_hour_after_since`, `waiting_without_remote_does_not_hold`,
`waiting_without_since_counts_from_first_seen`, `wall_jump_back_before_a_timer_window_waits_for_it`,
`wall_jump_back_does_not_stretch_the_waiting_hour`, `wall_jump_forward_ends_the_waiting_hour_now`,
`wall_jump_forward_past_a_timer_window_ends_it`, `working_holds_with_no_deadline`,
`working_with_a_timer_has_no_deadline`, `yes_is_remembered`.

Tests in `policy::hold_files` (35): `automatic_off_keeps_keep_awake`, `automatic_off_stops_run`,
`automatic_off_stops_script`, `counts_before_expires_with_its_deadline`, `counts_just_before_expires`,
`each_live_kind_counts_with_automatic_on`, `expired_file_is_kept_while_its_process_lives`,
`extreme_times_do_not_overflow`, `gone_after_expires_still_gives_a_delete`, `gone_gives_a_delete_and_no_count`,
`hold_deadline_is_where_the_count_changes` (property), `keep_awake_deadline_is_where_the_choice_changes` (property),
`keep_awake_ignores_other_kinds`, `keep_awake_is_off_at_expires`, `keep_awake_is_off_when_its_tray_is_gone`,
`keep_awake_is_off_without_a_file`, `keep_awake_last_minute_reads_one_until_expires`,
`keep_awake_reads_its_full_minutes_at_the_start`, `keep_awake_rounds_minutes_left_up`,
`keep_awake_unknown_keeps_the_last_answer`, `keep_awake_until_turned_off`, `kind_counts_follows_automatic`,
`label_is_the_reason`, `no_expires_counts_with_no_deadline`, `screen_counts_for_keep_awake`,
`screen_ends_with_the_hold`, `screen_holds_with_automatic_off`, `screen_is_ignored_outside_keep_awake`,
`stops_exactly_at_expires`, `unknown_before_any_answer_counts_nothing_and_deletes_nothing`,
`unknown_keeps_the_last_gone_and_deletes_nothing`, `unknown_keeps_the_last_yes`,
`wall_jump_back_keeps_the_deadline_at_expires`, `wall_jump_forward_past_expires_ends_the_hold`, `yes_is_remembered`.

Open items: the three proposals under "Found outside scope" (note 20 copy list, architecture waiting limit text, the
Keep awake menu check). Nothing for Haik.
