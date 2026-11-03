# Claude Code source: transcripts (timers and background tasks)

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: `p4-claude-status` (this card takes over `sources/claude/mod.rs` and `tests/claude_status.rs`), `p4-cron`,
`p4-claude-live-shapes`.
Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-claude-transcript\target`. Uses the real laptop: no.

## Goal

Read each live session's transcript and report pending timers (as due times with jitter) and open background tasks
of non-terminal sessions, by rules 3, 5 and 6 of `research/claude-code-signals.md` and the resume rule found by
`p4-claude-live-shapes`.

## Read first

- `docs/phases/04-sources.md`: track notes 2 to 6.
- `docs/architecture.md` section 7 (Claude rulings: transcripts read on status change and tick, not watched; timer
  margin is the core's).
- `docs/research/claude-code-signals.md`: "Timers", "Scheduler facts", "Transcript lines that matter", rules 3, 5, 6,
  9, "Proposed rule changes", and the "Phase 4 live shapes" section.
- `crates/wakeshot-core/src/sources/claude/mod.rs`, `status.rs` (from `p4-claude-status`), `sources/cron.rs` (from
  `p4-cron`), `sources.rs` (`WantedFile::FindTail`, `Feed::Appended`).
- `crates/wakeshot-core/tests/fixtures/claude/live/` and its README; `spikes/claude-watch/src/summary.rs`, `tail.rs`.

## Deliverables

1. `crates/wakeshot-core/src/sources/claude/transcript.rs`: per pid, a replay of appended bytes (complete lines only;
   the watch thread already holds back a partial last line). Kept per job: id, cron, recurring, created time, last
   fire, exact next time (`nextFireAt`, `scheduledFor` as floats). Handles: `CronCreate` and its result by
   `tool_use_id` (skip `is_error`), `CronDelete`, `ScheduleWakeup` (armed, `stop: true`, `cancelledWakeups`),
   `scheduled_task_fire` (with and without `taskKind: "loop"`), `session_cron_carry` (`otherArmed`, `tasks`),
   recurring jobs older than 7 days (fire once more, then gone), fired one-shots, the 50-job limit as a sanity cap.
   For sessions whose `entrypoint` is not `cli`: a result with `backgroundTaskId` or a Monitor `taskId` opens a task;
   a user line that starts with `<task-notification>` for it with `completed`, `failed` or `killed` closes it. Reads
   only the listed fields; prompts and text are never stored.
2. Timers out: for each armed job the next due time in wall ms, jitter included (track note 4): the exact time when
   the transcript has it; else `Cron::next_after(last fire)` plus `claude_jitter_ms`; a cron that does not parse
   follows track note 5. Rule 6 ("a period of 1 hour or less holds") is the core's call; the parser reports the due
   time. No timer for the loop fallback wakeup (product).
3. `/clear` and resume: job state is keyed by pid; a new `sessionId` for the same pid reads the new transcript from
   its start and takes `session_cron_carry` as the armed set; a new pid follows the rule `p4-claude-live-shapes`
   wrote.
4. `claude/mod.rs`: `ClaudeSource::new(home, zone: Box<dyn LocalOffset + Send>)` (the zone for cron times);
   `wanted()` adds `FindTail { root: <home>\.claude\projects, depth: 1, name_suffix:
   "<sessionId>.jsonl" }` for each live session (glob, never rebuild the folder name); `feed` routes `Appended` bytes
   by the found path to that pid's replay; `sessions()` fills `timers` and `open_tasks`. A known line kind with a
   missing field: skip it, `UnknownValue("transcript", <kind>)` (track note 6). The `new(home, zone)` change breaks
   `crates/wakeshot-core/tests/claude_status.rs`; this card owns that file after `p4-claude-status` and updates its
   constructor calls only (review m4).
5. Fixtures in `crates/wakeshot-core/tests/fixtures/claude/transcript/` for cases the live fixtures do not cover:
   broken line, unknown kind, a known kind with a missing field, a 7-day-old recurring job, a deleted job, a carry
   line after `/clear`, a resume. Built by hand from the live shapes, same redaction.
6. Tests first: `crates/wakeshot-core/tests/claude_transcript.rs` replays every live fixture in pieces (split at
   random byte points as the tail would deliver them, a `proptest` property: same facts as a whole-file feed) and
   checks timers and open tasks against what the live session did; unit tests per line kind.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- The `*/1` loop fixture gives due times within 1 s of each next logged fire; the dynamic loop gives its
  `scheduledFor`; after `stop: true`, `CronDelete` or a fired one-shot, no timer.
- A grep of the module shows no field kept from `prompt`, `content` or other free text (say so in the Result).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Status files (`p4-claude-status`, done). Durable jobs (rule 7, dropped for v1). `claude agents --json` (rule 10,
  dropped). What holds and for how long (core track). File reads and tails (watch cards). Any file not listed above.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. **A transcript problem shows "update Wakeshot" only once the session works.** The contract has no notice-only
   health: `UnknownValue` holds while the process lives and `Unreadable` drops the status signal. So this card names a
   transcript problem only while the status says `Working` or a task is open, where `UnknownValue` changes no hold
   (orchestrator's ruling kept). A session that stays idle for the rest of its life never shows it. No contract change
   is needed for v1. If the orchestrator wants the notice at once: add `FormatHealth::Notice(&'static str, String)`
   ("wants the notice, changes no hold") to `sources.rs`, map it in `policy::sessions::steady` like `Ok`, and in
   `claude/mod.rs` send it whenever `Replay::problem()` is set (one line in `Slot::facts`).
2. **The watch thread must restart a `FindTail` at offset 0 when it enters `wanted()`** (for `p4-watch-agents`). The
   source expects: read from the start when a want appears, keep the place while it stays, forget it when it leaves.
   The card for the watch thread says `wanted()` is asked after each feed, which gives exactly that. If it diffed
   wants only once per batch, a resume whose leftover `Gone` comes before the new status file would miss the start of
   the transcript. Lines replayed twice are harmless (the replay is keyed by ids), lines skipped are not.
3. **An SDK session's task can hold until the session ends.** Claude Code 2.1.317's own text: a task "stopped (via
   the UI, Monitor timeout, or agent teardown) leaves no transcript marker". Rule 3 then keeps it open while the
   process lives. A Monitor result has `timeoutMs` and `persistent`, so a non-persistent Monitor could end at its
   timeout, but `open_tasks` is a count with no end time (a contract question). Not verified live.
4. **A terminal session's MCP or WebSocket Monitor does not hold.** Its status is `idle` (signals doc, "Background
   work"), and rule 3 counts tasks only for sessions whose `entrypoint` is not `cli`, as the card says. A product or
   rule question for the parsing review.
5. **Background sessions get their pending wakeup back at a resume** (`Vi`, sessions with a job folder, `kind: bg`).
   Wakeshot drops every wakeup older than the process. Not verified live; `bg` sessions are not tested in v1.
6. **A clock set back after a process starts** makes its own lines look older than `procStart`, so its wakeups and
   tasks would be dropped and its recurring jobs counted from the start. Not handled.
7. **Transcript times are read with `holds::file::parse_expires`**, the crate's only RFC 3339 reader. A change there
   for hold files would change transcript times too. A shared reader in `time` would remove the coupling (contracts).
8. **Older text in `docs/research/claude-code-signals.md`** (rule 3's `killed`, rule 5's `startedAt`) is still the
   pre-Phase 4 wording, as `p4-claude-live-shapes` noted; the new facts are in `docs/research.md`, "Claude transcript
   checks". Not this card's file.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:39, orchestrator, from `p4-claude-status`' Result (Found outside scope 1): track note 6 sends
  `UnknownValue` from transcripts, but the core counts any `UnknownValue` as working while the process lives, so one
  dropped transcript field would keep every idle Claude session holding for its whole life. Orchestrator decision: an
  unknown transcript shape must never make a session hold; the status file alone decides, and the user still sees
  "update Wakeshot". Build it without a contract change if you can (for example: a transcript you cannot read adds no
  open task and raises the notice). If it needs a contract change, stop at that point, write the proposal in your
  Result and say so in your final reply; the orchestrator runs contract changes alone. Also (item 2): the resume rule
  uses `startedAt`, which `p4-claude-status` does not keep; `procStart` is a better divider (0.4 to 21 s earlier).
- 2026-11-03 21:55: started by `wakeshot-worker`. Status: running. Reading the card's sources.
- 2026-11-03 22:18: read the card's sources, the three Results, the core's use of `FormatHealth` and the live
  fixtures. Read Claude Code 2.1.317's own code in `claude.exe` (read only): the carry builder (`OZn`, `Tdn`), its
  scanner (`RU`), the resume restore (`zi`, `Vi`), the scheduler tick (`de`), `ScheduleWakeup` (`Uzs`, `M`) and the
  notification builder (`Ai`). Findings: the carry line also has `wakeup` and `ended`; a resume restores recurring
  jobs with their last fire set to the resume time and drops missed one-shots and 7-day-old jobs; a batch
  notification lists several `<task-id>`s. Design: no contract change. A transcript problem is sent as
  `UnknownValue` only while the session already holds (status `Working` or an open task), where it changes no
  hold; otherwise `Ok`. Resume divider `procStart`, applied when facts are made, so one replay serves any process.
- 2026-11-03 about 22:25 (the clock was not read for this line; the 22:40 first written here was a guess, corrected
  at 22:39): `transcript.rs` (replay, lenient visitors that keep no text) and `mod.rs` (`new(home, zone)`,
  `FindTail` per session, routing, facts) written; `tests/claude_status.rs` constructor updated (10 pass). 27 unit
  tests pass. Next: hand-made fixtures, then `tests/claude_transcript.rs`.
- 2026-11-03 22:39: 7 hand-made fixtures (scratch script from the live lines) and `tests/claude_transcript.rs` (19
  tests) written; all pass. A mutation (no replay hand-over at a resume) was caught by
  `a_resume_while_the_leftover_is_listed_goes_on_from_its_replay`. `Replay`'s `Debug` shows only the length of a held
  partial line. Research notes added (`docs/research.md`, "Claude transcript checks"). Every acceptance check run
  (Result). Status done.

## Result

Status: done. Needs Haik: no. **No contract change needed** (an optional one in "Found outside scope" 1).

**What was done.**

- `crates/wakeshot-core/src/sources/claude/transcript.rs`: `Replay`, one session process's transcript, fed appended
  bytes (`feed(bytes, zone)`; complete lines only, a partial line waits for its newline, so any split gives the same
  state), `start_file()` (after `/clear`), and three reports: `timers(since_ms, zone) -> Vec<i64>` (sorted due times,
  jitter in, no margin), `open_tasks(since_ms) -> u32`, `problem() -> Option<(&'static str, &str)>` (`TRANSCRIPT` with
  a line kind, or `CRON` with the cron text cut to 32 ASCII characters; the first one seen stays). Handles every line
  of deliverable 1: `CronCreate` and its result by `tool_use_id` (`is_error` and durable skipped), `CronDelete`,
  `ScheduleWakeup` (armed, `stop: true`, `scheduledFor: 0`), `scheduled_task_fire` with and without `taskKind: "loop"`
  (a loop fire ends every wakeup due within 90 s of it), `session_cron_carry` (`tasks`, `otherArmed`, and also
  `wakeup` and `ended`, which Claude Code 2.1.317 writes), recurring jobs 7 days old (fire once more, then gone),
  fired one-shots, 50 jobs at most, a wakeup forgotten 1 hour after its time (Esc cancels write nothing). Tasks: a
  result with `backgroundTaskId`, a `Monitor` result's `taskId`; closed by a notification at the start of a user
  line's text or of a `queued_command` attachment's `prompt`, for every `<task-id>` before `<summary>`, when it has a
  `<status>` of any value (the live shapes rule). Lines are read through visitors that look at text in place: only
  ids, cron texts, times and flags are kept; a field of an unrelated line kind never spoils a line.
- Timers (deliverable 2): `nextFireAt` and `scheduledFor` as given (ceiling of the float); otherwise
  `ClaudeScheduler::DEFAULT.next_fire_ms` from the last fire or the creation (the `p4-cron` replacement of
  `claude_jitter_ms`). A job known only from a carry line counts from the chain of fires that starts 30 minutes
  before the line, which gives its first fire after the line (exact but for `*/5`). Track note 5: a cron that does
  not parse is due only at the exact time it came with. No timer for the loop fallback.
- Resume (deliverable 3): lines older than the session process's `procStart` are an earlier process's (orchestrator:
  `procStart`, not `startedAt`). Applied when facts are made, as Claude Code's own restore (`zi`) does: a recurring
  job counts from the process start and is gone at 7 days old, a one-shot whose time passed is gone, every wakeup
  and open task of the earlier process is gone. The live first fires after a resume came 1.1 s after the due time.
- `crates/wakeshot-core/src/sources/claude/mod.rs`: `ClaudeSource::new(home: PathBuf, zone: Box<dyn LocalOffset +
  Send>)`; `wanted()` adds one `FindTail { root: <home>\.claude\projects, depth: 1, name_suffix: "<sessionId>.jsonl" }`
  per distinct session id; `feed` routes `Appended` bytes of `projects\<folder>\<sessionId>.jsonl` to each session
  with that id (bound to the first path fed; `Whole`, `Present`, `Gone` of a transcript change nothing); a `/clear`
  (new `sessionId`, same process) keeps the replay; a new process starts one, or takes over the replay of a listed
  leftover with the same id (the tail then goes on). `sessions()` fills `timers` and `open_tasks` (0 for `entrypoint`
  `cli`, rule 3). Format: the status file's problem first; a transcript problem becomes
  `UnknownValue(field, value)` only while the session holds anyway (status `Working` or an open task), else `Ok`.
  Reason: the core holds on any `UnknownValue`; there it changes no hold and still shows "update Wakeshot" at the next
  turn. Change it when the contract gets a notice-only health (Found outside scope 1).
- `tests/claude_status.rs`: the `source()` constructor and its import only (review m4).
- Fixtures: `tests/fixtures/claude/transcript/`, 7 files and a `README.md` (built by a scratch script from the live
  lines, same keys, order and redaction): broken line, unknown kind, missing field, 7-day-old job, deleted job, carry
  after `/clear`, resume.

**For `p4-live-claude` and `p6-int-tray-mode`.** The zone is `wakeshot_windows::localtime::SystemZone::current()`
boxed. Facts per session as above; `Replay` is public for direct use. The watch thread expectation is in "Found
outside scope" 2.

**Files changed.** `crates/wakeshot-core/src/sources/claude/transcript.rs`, `crates/wakeshot-core/src/sources/claude/
mod.rs`, `crates/wakeshot-core/tests/claude_status.rs`, `crates/wakeshot-core/tests/claude_transcript.rs` (new),
`crates/wakeshot-core/tests/fixtures/claude/transcript/` (new), `docs/research.md` (new section "Claude transcript
checks", addition only), this card.

**Checks and outcomes.**

- `rustfmt --edition 2024 --check` on `transcript.rs`, both test files and `mod.rs` (with its children): clean.
  `mod.rs` was formatted as a copy with empty child files, since `status.rs` is not this card's.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- `cargo test --locked -p wakeshot-core`: 512 lib tests pass (1 ignored, not this card's), `claude_status` 10 pass,
  `claude_transcript` 19 pass. No failure in another card's file came up.
- New tests in `sources::claude::transcript::tests` (27): `cron_create_arms_its_exact_next_fire`,
  `a_failed_or_durable_cron_create_arms_nothing`, `a_recurring_fire_moves_the_job_on_and_a_one_shot_fire_ends_it`,
  `a_recurring_job_7_days_old_fires_once_more_then_is_gone`, `cron_delete_ends_the_job_unless_it_failed`,
  `a_wakeup_holds_its_time_until_a_loop_fire_or_stop`, `a_wakeup_is_forgotten_an_hour_after_its_time`,
  `a_carry_line_is_the_armed_set`, `a_job_known_only_from_a_carry_is_due_at_its_first_fire_after_the_line`,
  `background_tasks_open_and_any_status_closes`, `a_monitor_opens_a_task_and_its_events_close_nothing`,
  `a_batch_notification_closes_every_listed_task`, `a_queued_command_notification_closes_its_task`,
  `only_a_notification_at_the_start_of_the_text_counts`, `lines_of_an_earlier_process_follow_the_resume_rules`,
  `a_cron_that_does_not_parse_holds_only_for_its_exact_time`, `a_known_kind_with_a_missing_field_is_skipped_and_named`
  (16 shapes), `other_kinds_and_shapes_are_ignored_silently`, `a_broken_line_that_may_matter_is_named`,
  `the_first_problem_is_kept`, `at_most_50_jobs`, `a_partial_line_waits_for_its_newline`, `a_line_too_long_is_skipped`,
  `the_replay_keeps_no_text`; properties `any_split_gives_the_same_state`, `any_bytes_feed_without_a_panic`,
  `changed_values_never_panic`.
- New tests in `tests/claude_transcript.rs` (19), the live sessions replayed with their status writes as the watch
  thread would feed them: `every_live_fire_was_due_and_every_live_line_reads` (11 fires: the two first fires after a
  resume 1104 and 1151 ms after the due time, the other nine 54 to 951 ms), `every_live_cron_list_agrees` (6 `CronList` answers,
  within 2 ms), `the_live_timers_after_the_lines_that_change_them`, `the_live_open_tasks`,
  `no_live_transcript_line_is_a_problem`, `a_resume_gives_the_same_facts_in_either_feed_order`,
  `a_resume_while_the_leftover_is_listed_goes_on_from_its_replay`, `wakeshot_started_after_a_clear_finds_the_next_fire`,
  `every_hand_fixture_has_a_case`, `a_broken_line_is_named_only_while_the_session_holds_anyway`,
  `unknown_kinds_are_ignored_silently`, `a_known_kind_with_a_missing_field_is_skipped_and_named`,
  `a_recurring_job_ends_after_its_fire_at_7_days`, `a_deleted_job_is_gone_and_a_failed_delete_keeps_it`,
  `a_carry_line_after_clear_is_the_armed_set`, `a_resumed_process_takes_back_only_what_claude_code_restores`,
  `wanted_tails_each_session_transcript_by_name`, `a_transcript_it_did_not_ask_for_is_ignored`; property
  `every_fixture_transcript_in_any_pieces_gives_the_same_facts` (all 21 transcripts split at random byte points).
- Acceptance: the `*/1` loop (fixtures 03 and 06) is due 54 to 282 ms before each next logged fire; the dynamic loop
  gives its `scheduledFor` (1793715660000); after `stop: true`, `CronDelete` and a fired one-shot no timer is left
  (`the_live_timers_after_the_lines_that_change_them`).
- Acceptance, privacy: a grep of `transcript.rs` shows the kept state is `jobs` (id to cron bit sets, `recurring`,
  times), `wakeups` (times), `tasks` (id to time), `calls` (tool id to a cron text or a job id), `problem` (a field
  name and a line kind or cut cron text) and a held partial line, which `Debug` shows only as a length. `prompt`,
  `content` and `text` are read only through `notice_ids`, which keeps task ids; no other text field is read.
  `the_replay_keeps_no_text` checks it with marked text in every free-text field.
- Secret scan (both pattern sets of `docs/phases/01-design-skeleton.md`, any UUID that is not a `5e55` fake,
  `haika`, the host name) over the new fixtures, their README, both test files, `transcript.rs` and `mod.rs`: 0 hits.

**Open items.** Not verified (in research): the `bg` resume path, a Monitor timeout writing no line, the `*/5`
carry guess. The scratch scripts and the build folder are deleted.
