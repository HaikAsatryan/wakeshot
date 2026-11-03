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

## Claude parsing review fixes (2026-11-04)

Status: done. Agent: `wakeshot-worker` (work name `p4-claude-fixes`). Findings from
`docs/reviews/p4-claude-parsing.md`: B1, M1, m1 to m5, m7, m8 (third bullet); rules 3 and 5 of
`docs/research/claude-code-signals.md` (rule 5 added by the orchestrator, 2026-11-04). Not this round: m6.

### Log

- 2026-11-04 00:11: started. Read the review, this card, the Results of `p4-claude-status` and `p4-cron`, the
  signals doc. Read Claude Code 2.1.317 (`claude.exe`, read only): `TaskStop` returns `{message, task_id, task_type,
  command}` and takes `task_id` or `shell_id`; its own resume scanner (`hrn`) takes the result of a `TaskStop`,
  `KillShell` or `KillBash` call with string `task_id` and `task_type` as the stop. The shell Monitor arms its
  timeout before it returns `{taskId, timeoutMs: persistent ? 0 : timeout, persistent}`. A stop from the UI queues a
  `passive` notification with `<status>killed</status>` (`tUr`). The notification header (`Dwe`) is a fixed text that
  starts `[SYSTEM NOTIFICATION - NOT USER INPUT]` and ends with a blank line; joined notices follow it, split by a
  blank line (`qi`, `Xi`). A fire line has no field that marks a job's last fire.
- 2026-11-04 00:24: B1, M1, m1, m2, m4, m5 built in `transcript.rs`, `status.rs`, `mod.rs`, `cron.rs`, with unit
  tests and 4 hand fixtures (`task-stop`, `monitor-timeout`, `header-notice`, `joined-notices`). A mutation back to
  the old behavior (TaskStop ignored, Monitor as an open task, header not read) fails 4 new unit tests. Monitor
  timers are added only for sessions whose `entrypoint` is not `cli`: a terminal session's status says `shell` for
  it, so a timer there would hold up to an hour after a UI stop with the status already `idle`.
- m2 decision (Haik's rule: no work, no hold; never hold forever): a computed recurring time is reported twice, as
  the built-in values give it and as the latest any flag value in the schema allows. Not "may miss a fire by the
  jitter": a missed fire is work the PC sleeps through. Not the review's "latest only": that moves the hold's start
  up to 15 minutes later than the decided 1 hour lead. With two times the window runs from the built-in time less 1
  hour to the latest plus 2 minutes; the real fire ends it through its fire line, so with the built-in values the
  hold is exactly as before, and it is longer (at most about 15 minutes) only when a fire never comes. The 7 day end
  stays at the built-in value (dropping a job after its fire at 7 days costs nothing when the flag is the built-in);
  a later fire of that job brings it back and only fire lines end it from then on: one missed fire at most if the
  flag ever raises the age. The drift check is `Replay::scheduler_drift_ms()` for `p4-live-claude`: the core has no
  log of its own and a source's sessions carry no log line, so a log needs a contract change.
- 2026-11-04 00:28: m7, docs (rules 3 and 5, `product.md`, `hold-files.md`, research, fixture README), the m5 source
  test and every check done. m3 left: its fix is in `watch/agents.rs`, not `read.rs` (Result). Status done.

### Result

Status: done. Fixed: B1, M1 (part a; part b as far as the code can), m1, m2, m4, m5, m7, m8's third bullet, and
rule 5. Left: m3 (not this round's file). Not this round: m6. No contract change.

**What was done.**

- B1: `TaskStop`, `KillShell` and `KillBash` calls are kept; a result that is not an error closes its `task_id`
  (else the input's `task_id`, then `shell_id`); when none names a task: `transcript`/`TaskStop result`. `message`
  and `command` are never read.
- M1: a Monitor result with `persistent: false` is a task with an end (its time plus `timeoutMs`), reported by the new
  `Replay::task_timers` and added to `timers` only for sessions whose `entrypoint` is not `cli`; `open_tasks` counts
  only tasks with no end (shells, persistent Monitors). A `<status>` or a stop removes it; it is forgotten an hour
  after its end. A Monitor result of another shape (no `timeoutMs`, or 0 without `persistent`) is named and ends after
  1 hour; a timeout over 1 hour is named and gets a due time every hour from its start, at most 24. Part b (UI stop):
  the `killed` notification already closes in every form the code knows, now also after the header (m4); nothing
  else is safe without a new file kind (decision 3 below).
- m1: a Bash or PowerShell call with `run_in_background: true` whose result has no `backgroundTaskId` is named
  `transcript`/`background shell result`.
- m2: a computed recurring time also comes as `ClaudeScheduler::latest_recurring_ms` (new, `cron.rs`): the latest any
  flag value the schema allows. `Replay::scheduler_drift_ms()` keeps the first gap over 2 s between a `nextFireAt` and
  the built-in time. A job dropped at the built-in 7 days comes back at its next fire line or carry line and is not
  aged again.
- m4: `notice_ids` takes Claude Code's header (also inside `<system-reminder>`) and, after it, every notification
  joined after a blank line that follows a `</task-notification>`.
- m5: `StatusFile::session_id_problem` (`<string N>` or the JSON type); `mod.rs` names it as
  `UnknownValue("sessionId", ...)` only while the session holds anyway, like a transcript problem.
- m7: `holds/file.rs` refuses pid 4 ("pid 4 is the System process"); `docs/hold-files.md` says so.
- Docs: rules 3 and 5 of `docs/research/claude-code-signals.md` (rule 3: any `<status>`, the header and joined forms,
  the ends with no notification; rule 5: the resume and reminder texts the code follows, the two times, the 7 day
  comeback); `docs/product.md`, Claude Code sources: the silent ends. `docs/research.md`: "Claude parsing review fix
  checks".

**Decided by the worker, for Haik's review.**

1. m2, by Haik's rule: two due times (built-in and latest), not "may miss a fire" and not the review's "latest only"
   (Log 00:24). Cost: up to about 15 minutes more hold, only when a fire never comes.
2. m2: the 7 day end stays at the built-in value; a fire after it brings the job back. Cost: one missed fire if the
   flag ever raises the age.
3. M1 part b: the task's output file (`[killed]` trailer) is not read: it is a new wanted-file kind that reads tool
   output. A UI stop may hold until Claude Code writes its `killed` line or the session ends. Said in `product.md`.
4. A Monitor of an unknown shape ends after 1 hour rather than holding for the life of the process.
5. m5: a missing `sessionId` is not a problem (early writes may lack it); only a present one of another shape is.
6. m4: joined notices are split at a blank line after each end tag, not by `joinedNoticeLengths` (that field may
   come after the text on the line, and the text is never stored).

**Tests** (each fails before its fix: the behavior is new, or a mutation back to the old code failed 4 of them).

- `transcript.rs` (8 new, 1 of them replacing `a_monitor_opens_a_task_and_its_events_close_nothing`):
  `a_monitor_holds_until_its_timeout_and_its_events_close_nothing`, `a_monitor_of_another_shape_still_ends`,
  `task_stop_closes_the_task_it_names`, `a_background_shell_result_without_its_task_id_is_named`,
  `a_notification_after_the_header_and_joined_ones_close_their_tasks`,
  `a_computed_recurring_time_covers_every_flag_value` (5 crons, 60 flag settings),
  `a_next_fire_the_built_in_scheduler_does_not_give_is_a_drift`, `a_job_fired_after_its_age_comes_back`; and a
  compile-time check that the core's lead is at least `MONITOR_MAX_MS`. Six older tests now expect the second time.
- `status.rs`: `a_session_id_of_another_shape_is_named`. `holds/file.rs`: pid 4 added next to the pid 0 case.
- `tests/claude_transcript.rs` (5 new): `the_live_monitors_hold_until_their_timeout`,
  `a_task_stop_ends_the_shell_it_names`, `a_monitor_that_times_out_holds_only_until_its_timeout`,
  `notifications_after_the_header_close_their_shells`,
  `a_session_id_of_another_shape_is_named_while_the_session_holds`. Older ones now expect the second time or the
  Monitor as a timer. New hand fixtures: `task-stop.jsonl`, `monitor-timeout.jsonl`, `header-notice.jsonl`,
  `joined-notices.jsonl`, built by a scratch script from the redacted live lines (README rows).

**Checks.** `rustfmt --edition 2024 --check` on every changed Rust file: clean. `cargo clippy -p wakeshot-core
--locked --target <t> -- -D warnings`, with and without `--all-targets`, x86_64 and aarch64: all exit 0. `cargo test
-p wakeshot-core --locked`: 561 lib tests (2 ignored, not mine), `claude_status` 10, `claude_transcript` 24, all
pass. No failure in another card's file. Secret scan of the new fixtures and changed files (`haika`, the host name,
UUIDs that are not `5e55` fakes): 0 hits. `read.rs` was not changed, so no `wakeshot-windows` check was run.

**Files changed.** `crates/wakeshot-core/src/sources/claude/transcript.rs`, `status.rs`, `mod.rs`;
`crates/wakeshot-core/src/sources/cron.rs`; `crates/wakeshot-core/src/holds/file.rs`;
`crates/wakeshot-core/tests/claude_transcript.rs`; `crates/wakeshot-core/tests/fixtures/claude/transcript/` (4 new
files, README rows); `docs/research/claude-code-signals.md` (rules 3 and 5 only); `docs/product.md` (Claude Code
sources); `docs/hold-files.md`; `docs/research.md` (new section at the end); this card.

**Left, and found outside scope.**

1. m3 is not done: the early `return` is in `crates/wakeshot-windows/src/watch/agents.rs` (`fill_folder`, the `Err`
   arm of `names`), not in `read.rs`, which only lists. Fix: on that error take `state.known`'s names as the
   targets. Its test needs a folder whose listing is denied while its files stay readable (a deny ACE for listing on
   the folder only); the current test `a_folder_that_cannot_be_listed_keeps_its_last_files` replaces the folder by a
   file, where the fix would rightly feed `Gone`.
2. The README should say that a task stopped from Claude Code's UI may hold until the next prompt (review M1 b). Not
   this round's file.
3. For `p4-live-claude`: an SDK background shell stopped with `TaskStop` (expect no notification and `open_tasks` 0
   after the result), a Monitor timeout (expect one event line, no `<status>`), a stop from the VS Code panel (when
   the `killed` line lands), whether a header form ever shows in a transcript, and `Replay::scheduler_drift_ms()` on
   live jobs.
4. Logging the drift needs a log path from a source (a contract change): `sessions()` carries no log line.

## Claude parsing review fixes, round 2 (2026-11-04)

Status: done. Agent: `wakeshot-worker` (work name `claude-fix2`). From `docs/reviews/p4-claude-parsing-verify.md`:
"Left from M1 part b" (major) and the minors N1 to N3 where the fix is one line or a doc line.

### Log

- 2026-11-04 00:52: started. Read this card's review-fix section, the review (M1) and the verify.
- 2026-11-04 00:58: checked the verify's claim in Claude Code 2.1.317 (`claude.exe`, read only): the stop path `ILe`
  kills the task, then calls `tUr` for a shell or shell Monitor stopped with a `source` while `tengu_rosy_donut` is on
  (default on); `tUr` calls `dl`, which is `enqueuePendingNotification` (`vr`); `vr` writes a `queue-operation`
  `enqueue` line with the notice as `content` unless the item is `unlogged` (`tUr` does not set it) or a hook takes the
  enqueue over. The claim holds. The live fixtures show the same line before each delivered notice. Details:
  `docs/research.md`, "Claude UI stop notice checks".
- 2026-11-04 01:00: built: `transcript.rs` reads `operation` and the line's `content` (as a notice only) and closes
  the listed tasks on an `enqueue` line. Unit test, hand fixture `ui-stop.jsonl`, integration test, and live checks
  at the enqueue lines of fixtures 02, 08, 12. A mutation (the `queue-operation` arm ignored) fails 4 tests.
- 2026-11-04 01:04: docs (rule 3, fixture README, research), every check run (Result). `wakeshot-windows` clippy with
  `--all-targets` stops at a lint in `child.rs`, a file of the running `p4-cli-run` card; not this round's file.
- 2026-11-04 01:06: that clippy run tried 3 times from 01:03 to 01:06, same lint each time (`child.rs` unchanged
  since 01:02). Build folder and scratch scripts deleted.

### Result

Status: done. The verify's claim holds in the 2.1.317 code, so M1 part b is fixed as it proposed. No contract change.

**What was done.**

- `transcript.rs`: a `queue-operation` line with `operation: "enqueue"` closes every task its `content` lists, read
  with the same `notice_ids` as a user line (any `<status>`, header and joined forms; only the start of the text
  counts). New keys read: `operation`, and the line's own `content`, kept only as task ids. `dequeue`, `remove` and
  other operations change nothing; a missing or non-text `content` is no problem. Effect: a shell or Monitor stopped
  from the VS Code panel ends when Claude Code queues its `killed` notice, not when the passive notice is delivered
  at the next prompt. Every other notice also ends its task a little earlier (live fixtures: the enqueue line comes 0
  to 3.4 s before the line that delivers the same notice, 10 cases).
- Not checked live; `p4-live-claude` checks it (stop a background shell from the panel; expect the `enqueue` line
  within a second and `open_tasks` 0 right after it).
- Docs: rule 3 of `docs/research/claude-code-signals.md` (the enqueue line, the UI stop, N1 and N3's Monitor
  details); fixture README row; `docs/research.md`, "Claude UI stop notice checks" (the code path with offsets, what
  is not verified).

**Verify minors.**

- N1 (failed `TaskStop` closes nothing): no code change; its main case (a UI stop whose notice had not landed) is now
  closed by the enqueue line. Rule 3 now says a failed stop closes nothing.
- N2 (Monitor end on the wall clock across a PC sleep): not a one-line fix. For `p4-live-claude`: sleep the PC for 5
  minutes during a 10 minute Monitor and see when its timeout event comes.
- N3 (docs): rule 3 fixed. Two lines are in files this round does not own (list below).

**Decided by the worker, for Haik's review.**

1. This replaces decision 3 of the first round: a task stopped from the UI now stops holding when Claude Code writes
   its `killed` notice to the queue, which the code does at once; it holds longer only if the remote flag
   `tengu_rosy_donut` is off or a hook holds the enqueue back (not verified). Change it when `p4-live-claude` shows
   the line comes late or not at all.
2. Only `enqueue` is read, not `remove`: in every case seen a `remove` follows the same notice's `enqueue`. Change it
   when a live session shows a `remove` (or other operation) with a notice that had no `enqueue` line.
3. The line's `content` is read (as ids only) on every line kind and used only for `queue-operation`, since key order
   is not fixed. Cost: one prefix check per line that has a marker.
4. `ui-stop.jsonl` has no delivery line after the enqueue: how a passive notice is delivered at the next prompt is
   not known. The unit test covers a later user line.

**Tests.**

- `transcript.rs` (1 new): `a_queued_notice_ends_its_task_at_once` (a shell, a Monitor persistent or not, the header
  form; `remove`, `dequeue`, an event, a quoted notice, a prompt, no content, a non-text content and a non-text
  operation close nothing and name nothing). Changed: `other_kinds_and_shapes_are_ignored_silently` now uses a
  `remove` line (an `enqueue` is read now); `the_replay_keeps_no_text` and the proptest `sample()` gain enqueue lines.
- `tests/claude_transcript.rs` (1 new): `a_stop_from_the_ui_ends_the_shell_at_its_queued_notice` on the new fixture
  `ui-stop.jsonl`. `the_live_open_tasks` checks fixtures 08 and 12 at line 6 (the enqueue, 0 open);
  `the_live_monitors_hold_until_their_timeout` checks 02 at lines 6, 12, 29 (timer gone) and 21, 25 (events: timer
  kept).
- Mutation: with the `queue-operation` arm ignored, 4 tests fail (the two new ones and the two live ones).

**Checks** (2026-11-04 01:02 to 01:04, build folder `<scratchpad>\claude-fix2\target`).

- `cargo fmt --all --check`: my files clean (`rustfmt --edition 2024 --check` on `transcript.rs`,
  `tests/claude_transcript.rs`, `watch/data.rs`: exit 0). The workspace run shows diffs only in
  `crates/wakeshot-windows/src/child.rs`, a file of the running `p4-cli-run` card.
- `cargo clippy -p wakeshot-core --locked --target x86_64-pc-windows-msvc -- -D warnings`, with and without
  `--all-targets`: both exit 0.
- `cargo test -p wakeshot-core --locked`: lib 565 passed, 2 ignored (not mine), `claude_status` 10, `claude_transcript`
  25, all pass.
- Privacy: `content` and `operation` are read only through `read_notice` (task ids) and `Word`; no text is kept
  (`the_replay_keeps_no_text`). No new file is opened; no `.key` path is involved. Secret scan (the patterns of
  `docs/phases/01-design-skeleton.md`, plus `haika`, the host name, and any UUID that is not a `5e55` fake) over the
  new fixture, its README and the three changed Rust files: 0 hits (self-test: 1 hit on a planted token).
- `wakeshot-windows` (for `data.rs`): `cargo test -p wakeshot-windows --lib --locked` 387 passed, 20 ignored; clippy
  without `--all-targets` exit 0; with it, one lint in `child.rs:489`, not this round's file.

**Files changed.** `crates/wakeshot-core/src/sources/claude/transcript.rs`, `crates/wakeshot-core/tests/
claude_transcript.rs`, `crates/wakeshot-core/tests/fixtures/claude/transcript/ui-stop.jsonl` (new) and `README.md`,
`docs/research/claude-code-signals.md` (rule 3 only), `docs/research.md` (new section at the end), this card.

**Left, and found outside scope.**

1. `docs/product.md:41-46` (not this round's file): the sentence "A task stopped from the app's UI may hold until ...
   the only other trace, is a new file kind and is not built" is now wrong. Proposed: "A task stopped from the app's
   UI ends when Claude Code queues its stop notice, which it writes to the transcript at once (2.1.317 code; not
   checked live)."
2. `docs/architecture.md:899-900` (verify N3, not this round's file): `timers` also holds a Monitor's timeout, and
   `open_tasks` counts only tasks with no known end. Proposed: `// next due times, wall ms: jobs, wakeups, Monitor
   timeouts` and `// background tasks with no known end, seen open in the transcript`.
3. N2 and the panel stop go to `p4-live-claude` (its card is not this round's file).
4. The README should say that a task stopped from Claude Code's UI may hold only if its queued notice is late (first
   round's item 2, now weaker).
