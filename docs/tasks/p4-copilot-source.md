# Copilot CLI source

Phase: 4. Status: done (2026-11-06; back in v1, Haik 2026-11-06; see "Round 2" at the end). Needs Haik: no (his live test is `p4-live-copilot`). Agent: `wakeshot-worker`. Size: about
90K tokens.
Runs after: `p4-cron` (for cron schedules). Parallel with: every other card of this track and every card of tracks
2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-copilot-source\target`. Uses the real laptop: no (an npm package may be
downloaded as data into the scratchpad, never installed or run).

## Goal

`CopilotSource`, an `AgentSource` built from the Copilot schema reading in `research/other-agent-signals.md`:
sessions from `session-state\<id>\`, activity inferred from the last persisted event, scheduled prompts as timers.
Not tested live until `p4-live-copilot`; the docs mark it so.

## Read first

- `docs/phases/04-sources.md`: track notes 2, 7, 9, and Question 2 (answered with the default, Haik 2026-11-03
  18:09: waiting does not hold).
- `docs/architecture.md` section 7.
- `docs/research/other-agent-signals.md`: "GitHub Copilot CLI" (all of it), "Design notes", "Fixture sketches".
- `crates/wakeshot-core/src/sources.rs` (after the contracts card), `sources/cron.rs` (from `p4-cron`).

## Deliverables

1. Schema check first: download `@github/copilot` (current version) as data into a fresh folder in the scratchpad
   (`npm pack` or the registry tarball; never install or run it) and re-read `session-events.d.ts` for the events in
   deliverable 3 and the `session.schedule_*` fields (`at`, `intervalMs`, `cron`, `tz`, `recurring`, and how a rearm
   or a fire shows). Note the version and any change in `docs/research.md` (additions only).
2. `crates/wakeshot-core/src/sources/copilot.rs`: `CopilotSource::new(home: PathBuf, zone: Box<dyn LocalOffset +
   Send>)` (`COPILOT_HOME` or `%USERPROFILE%\.copilot`, resolved by the caller). `wanted()`: `Names { dir:
   <home>\session-state, depth: 1, accept }` for the in-use lock files (name very likely `inuse.<pid>.lock`, not
   verified: accept `inuse.*.lock`; never opened); for each session folder with a lock, `Tail(<folder>\events.jsonl)`.
3. Activity from the last relevant event (the research rule): `Working` for `user.message`, `assistant.turn_start`,
   `tool.execution_start`, `tool.execution_complete`, `permission.completed`, `subagent.started`, or an
   `assistant.message` with non-empty `toolRequests`; `Waiting { since_wall_ms: event timestamp, remote: false }` for
   an unanswered `permission.requested` (Question 2: the core then does not hold); `Idle` for `assistant.turn_end`
   after a message with no tool requests, `abort`, `session.error`, `session.shutdown`. An open `subagent.started`
   with no `subagent.completed` or `subagent.failed` keeps `Working`. Ephemeral events never reach the file.
4. Timers from `session.schedule_created` / `_cancelled` / `_rearmed`: `at` exact; `intervalMs` from the last fire or
   creation; `cron` through `Cron::next_after` in the PC's zone, whatever `tz` says (track note 9). Due times carry no
   margin (the core adds it).
5. `SessionFacts`: `key.id` the session id, `liveness: AnyProcess { image: "copilot.exe", lock_file }`, `label:
   "Copilot"`. A lock file whose name holds a pid: note in the module doc that the live test decides whether to use
   it. Lock gone: the session is gone.
6. Fixtures `crates/wakeshot-core/tests/fixtures/copilot/` from the sketches and the schema (working, waiting,
   idle, aborted, subagent open, each schedule form, junk lines), marked "built from the schema" in a README. Tests
   first: `crates/wakeshot-core/tests/copilot_source.rs`, whole and in pieces.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- No code path opens a lock or handshake file. Prompt text (`content`, `prompt`) is never stored.
- The downloaded package stays in the scratchpad and is deleted at the end.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Restart Manager and process checks (watch cards). The live test (`p4-live-copilot`). `/keep-alive`. Any file not
  listed above.

## Questions for Haik

Each with the worker's default. None blocks this card; the default is what is built.

1. **Copilot asking you a question.** When Copilot asks you something (its `ask_user` tool), nothing about the
   question is saved to disk except that the tool started. I read that as waiting, the same as an approval prompt, so
   it does not keep the PC awake (your answer to sign-off item 2: "the same as Claude waiting", and Claude's waiting
   covers questions too). Default: no hold. The other option: count it as working, which holds until you answer.

## Found outside scope

Problems seen but not fixed.

- Stale docs after Round 2: `docs/architecture.md` section 7 still says Codex and Copilot come "after v1" with cards
  `later`; `docs/phases/04-sources.md` says the same at "Notes 7, 8 and 9 apply after v1", under the card table
  ("Codex and Copilot moved to after v1") and in "Order inside the track" item 5. Not this card's files.
- `docs/research/other-agent-signals.md`, "GitHub Copilot CLI", describes 1.0.105; the 1.0.106 changes in Result are
  for `p4-live-copilot` to carry there (Round 2).
- Liveness for SDK sessions: 1.0.106 ships `prebuilds/win32-x64/copilot-runtime.exe`, an SDK server host
  (`--server`/`--headless`) that the bundled `copilot-sdk/index.js` starts. A session an SDK app hosts there runs in
  that process, not `copilot.exe`, so the weaker `AnyProcess` rule ("any live `copilot.exe` while the lock exists")
  would miss it; a Restart Manager holder check would not. Whether such sessions write the same lock files is not
  verified. The `AnyProcess` check is `p4-codex-source`'s; the live test can decide.
- The caller must resolve Copilot's folder: `--config-dir`, then `COPILOT_HOME`, then `%USERPROFILE%\.copilot`
  (1.0.106 `cli-main.js`: `[e?.configDir, process.env.COPILOT_HOME, osHomeDir]`). A session started with
  `--config-dir` writes elsewhere, like Claude's `CLAUDE_CONFIG_DIR`. For whoever wires the source into the tray
  (`p4-live-copilot`, Round 2).
- Archived log segments (`events.<n>.jsonl`, 1.0.106) are not read. If a rotation's checkpoint prefix does not carry
  every live schedule, a Wakeshot that starts after the rotation misses those schedules until their next line. For
  `p4-live-copilot` to check (set `COPILOT_SESSION_EVENT_LOG_ROTATION_BYTES` low to force a rotation).

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-06: started by `wakeshot-worker` (Round 2). Read the card, plan "How we work", track notes, architecture
  section 7, the Copilot research, `sources.rs`, `cron.rs`, the Claude source and the watch side of `Names` and
  `Tail`.
- 2026-11-06: schema check. npm `@github/copilot` is now 1.0.106 (research read 1.0.105). Got the registry tarballs
  of `@github/copilot` and `@github/copilot-win32-x64` 1.0.106 into the scratchpad (sha512 equal to the registry's
  `dist.integrity`), cut the `copilot.tgz` bundle out of `copilot.exe` with a byte scan (nothing run), and read
  `copilot-sdk/generated/session-events.d.ts`, `schemas/session-events.schema.json`, `rpc.d.ts`, `cli-main.js`,
  `changelog.json` and strings of `prebuilds/win32-x64/runtime.node`. Changes from the research, details in Result.
- 2026-11-06: design set (reasons in Result): last relevant event as the card says, plus `session.start`/`resume`
  as idle, sub-agent idle kinds ignored, `ask_user` start as waiting; journal rotation kept across a `Gone` when the
  new head starts with `session.journal_link`; schedule fires are `user.message` with `source` `schedule-*`;
  interval and cron due times also move past the newest event time in the file. Writing `copilot.rs` next.
- 2026-11-06: `copilot.rs`, 19 fixtures with a README (made by a script in the scratch folder) and
  `tests/copilot_source.rs` written; 42 tests pass. Next: break each rule once to see a test catch it, then clippy.
- 2026-11-06: mutation check (17 rules broken one at a time by a script, file restored after): first run missed
  "a sub-agent's turn end is ignored" (the test's main turn still had tools to run); the test now starts a fresh main
  turn first, and all 17 are caught. Clippy and tests clean (Result). Deleted the downloaded package and the build
  folder.

## Result

Status: done (2026-11-06). Built from Copilot CLI 1.0.106's schema; not tested live (`p4-live-copilot`).

**Version read and changes from the research (1.0.105).** npm `@github/copilot` 1.0.106 (registry `time.modified`
2026-11-06T01:46Z), with `@github/copilot-win32-x64` 1.0.106. The main package is only `npm-loader.js`; the bundle is
the `copilot.tgz` asset inside `copilot.exe` (found by a byte scan for a gzip stream that holds a tar; nothing run).
Read: `copilot-sdk/generated/session-events.d.ts` (16,818 lines; well over 100 event types),
`schemas/session-events.schema.json`, `copilot-sdk/generated/rpc.d.ts`, `cli-main.js`, `changelog.json`, strings of
`prebuilds/win32-x64/runtime.node`. Every point is from that read; "strings only" means seen as text in the native
addon, not as code.

- Unchanged: home `.copilot` or `COPILOT_HOME` (now resolved in native code, `--config-dir` first), folder
  `session-state\<id>\`, file `events.jsonl`, the envelope `id`, `timestamp`, `parentId`, `type`, `data`,
  `ephemeral`. Still ephemeral: `session.idle`, `assistant.idle`, `session.background_tasks_changed`,
  `user_input.requested`, `elicitation.requested`, `exit_plan_mode.requested`. Still persisted: every event of
  deliverable 3. Lock name: strings `inuse` and `lock` sit together, plus "Failed to .. in-use lock file" and "Failed
  to clean up dead lock file"; the exact name is still not verified.
- New envelope fields: `agentId` (on a sub-agent's own events) and `checkpoint` (a replay record).
- New: **log rotation.** `session.journal_link` ("Links a standalone replay head to its complete preceding numbered
  journal archive", data `previousFile`); strings: `events.` + `.jsonl` segment names, "predecessor link is not the
  first record", "missing standalone checkpoint prefix", rotation by size with `COPILOT_SESSION_EVENT_LOG_ROTATION_BYTES`
  (default not found). A rewind can also truncate the file (strings "Failed to truncate JSONL file").
- New: events are written in batches, with a 100 ms default flush debounce (strings only:
  `COPILOT_SESSIONFS_FLUSH_DEBOUNCE_MS`, `sessionEventWriterSetFlushDebounce`).
- Schedules: `data.id` is an integer, sequential in the session and "Stable across resumes (rebuilt from the event
  log)" (`rpc.d.ts`). New `selfPaced` (the model arms each run) and `origin` (`user`/`model`). `at` is epoch ms and
  never recurring ("A schedule for a fixed time cannot be recurring."). `intervalMs` and `cron` default to recurring
  (`rpc.d.ts` request defaults). `cron` is "evaluated in `tz`" (IANA). **A rearm** is `session.schedule_rearmed`
  `{id, nextRunAt}` (epoch ms), for self-paced schedules only ("is not self-paced and cannot be re-armed").
  **A fire** writes no schedule event: it is a `user.message` whose `source` starts with `schedule-` (`cli-main.js`
  checks `source?.startsWith("schedule-")`); what follows the prefix is not verified. `schedule_cancelled` is `{id}`.
  No persisted "fired" or "last run" field; how Copilot itself times the next interval run is in native code, not
  verified.
- `permission.requested` also requires `requestId`; `permission.completed` has `requestId` and `result`. New
  `permission.carriedForward` (an earlier approval answers the request). `abort.reason` is `user_initiated`,
  `remote_command`, `user_abort` or `autopilot_credit_limit`. `subagent.completed` can carry `cancelled: true`.
  `session.resume` has `sessionWasActive` and `continuePendingWork`.
- New: `prebuilds/win32-x64/copilot-runtime.exe`, the SDK server host (Found outside scope). One process can hold
  several sessions (1.0.106 changelog: the Sessions sidebar).
- The built-in "ask the user" tool is `ask_user` (`cli-main.js`).

**Built.** `crates/wakeshot-core/src/sources/copilot.rs`: `CopilotSource::new(home, zone)`, `is_lock_file_name`,
and constants `LABEL` ("Copilot"), `IMAGE` ("copilot.exe"), `EVENTS_FILE` and the `FormatHealth` names. `wanted()`
is `Names { <home>\session-state, depth 1, inuse.*.lock }` (case ignored), plus a `Tail` of `events.jsonl` for each
session folder with a lock. Facts: key the folder name (UUID-like names only; dot folders skipped), `AnyProcess {
"copilot.exe", <first lock> }`, `open_tasks` 0. Choices, each with its reason and "change it when":

- Activity is the last relevant event, as deliverable 3 says. Added, since the card's list leaves these open:
  `session.start` and `session.resume` read as idle and clear open sub-agents (a crash can leave a turn open; the
  resumed process starts at the prompt); a sub-agent's own `assistant.turn_end`, `abort` and `session.error` are
  ignored (they end its work, not the session's), while its work events and approvals count; a `tool.execution_start`
  of `ask_user` reads as waiting since that event (Question 1); `permission.carriedForward` answers like
  `permission.completed`. An open sub-agent turns idle into working, never waiting into working (the card's rule; a
  prompt on screen is the newest fact). Change it when the live logs show another order.
- Schedules: `at` exact; `/after` (interval, not recurring) creation plus interval; a recurring interval counts from
  its last fire, else its creation; cron through `Cron::next_after` in the PC's zone, `tz` ignored (track note 9);
  self-paced takes its last `nextRunAt`. A fire whose `source` tail is a known id moves that schedule (a one-shot
  ends); one that names no known id moves every recurring interval due by its time plus 60 s. Recurring interval and
  cron times also move past the newest event time in the file, because any event proves the process ran then; so a
  missed fire costs at most drift, never a due time stuck in the past. `session.shutdown` drops the timers until the
  session starts again. One-shots more than an hour behind the newest event time are dropped (the core holds at most 2
  minutes past a due time). Change it when the live test shows how `source` names a schedule, or that Copilot times
  intervals from creation rather than from each fire.
- Rotation: after a `Gone`, a file whose first line is `session.journal_link` keeps all state (the new head is the
  same session's log); any other first line replays from scratch (a truncation or a new file). Checkpoint records only
  add schedules (by id, so a known one keeps what was learned since); their turns and times are history. Change it
  when the live test shows what the checkpoint prefix holds.
- Format: every unknown kind is skipped without a word (Copilot has well over 100). A line that is not an event, a bad
  `timestamp`, a known kind missing a field it needs, a cron `Cron::parse` refuses or an interval that is not a
  positive number is skipped and kept as the first problem, named only while the session works (as for Claude, so a
  problem never makes a session hold); a log of which no line reads is `Unreadable("events file")`.
- Privacy and bounds: only ids, times, flags and cron texts are kept; `content`, `prompt` and other text are skipped by
  serde, a partial line shows only its length in `Debug`. Lines over 32 MB are skipped; at most 256 schedules and 256
  open sub-agents.

`crates/wakeshot-core/tests/fixtures/copilot/`: 19 logs and a README that marks them "built from the schema", made
by `make_fixtures.py` in the scratch folder.

**Tests** (`crates/wakeshot-core/tests/copilot_source.rs`, 42): `wanted_lists_lock_names_then_tails_each_locked_session`,
`lock_names_are_filtered_before_any_open`, `only_session_folders_with_a_lock_count`, `a_lock_alone_is_an_idle_session`,
`lock_gone_ends_the_session_and_its_log`, `lock_bytes_and_other_files_are_never_parsed`, `two_sessions_are_kept_apart`,
`source_and_labels_name_copilot`, `a_turn_that_asked_for_a_tool_is_working`,
`an_approval_prompt_waits_since_its_time_and_is_not_remote`, `a_turn_that_ends_with_no_tool_is_idle`, `an_abort_is_idle`,
`an_open_subagent_keeps_working_after_the_main_turn_ends`, `a_subagents_own_turn_end_or_abort_does_not_end_the_main_turn`,
`a_subagent_waiting_for_approval_waits`, `a_question_to_the_user_waits_like_an_approval`,
`a_resume_after_a_crash_starts_at_the_prompt`, `a_session_error_is_idle_and_the_next_prompt_works`,
`a_fixed_time_schedule_is_exact`, `an_interval_counts_from_its_last_fire`,
`an_interval_with_no_fire_seen_moves_past_the_newest_event`, `a_fire_that_names_no_schedule_moves_the_intervals_that_were_due`,
`a_one_shot_interval_fires_once`, `a_cron_runs_in_the_pc_zone_whatever_tz_says`, `a_self_paced_schedule_takes_its_last_rearm`,
`a_cancelled_schedule_holds_nothing`, `a_shutdown_drops_timers_until_the_session_resumes`, `spent_one_shots_are_dropped`,
`a_bad_schedule_is_named_only_while_working_and_holds_nothing`, `junk_lines_are_skipped_and_named_only_while_working`,
`a_known_kind_with_a_missing_field_is_named_by_its_type`, `a_log_with_no_readable_line_is_unreadable`,
`ephemeral_lines_are_ignored`, `a_last_line_without_its_newline_waits_for_the_rest`, `crlf_lines_read_the_same`,
`a_rotation_keeps_what_the_old_file_said`, `a_rotated_head_read_alone_takes_its_checkpoint_schedules_only`,
`a_file_that_starts_over_without_a_link_is_replayed_from_scratch`, `every_fixture_gives_the_same_facts_byte_by_byte`,
`every_fixture_gives_the_same_facts_in_any_pieces` (proptest), `any_bytes_never_panic_and_keep_the_bounds` (proptest),
`prompt_text_never_reaches_facts_or_debug`. Each of 17 rules, broken on purpose, fails at least one of them (Log).

**Checks** (2026-11-06, build folder `<scratchpad>\p4-copilot-source\target`):

- `rustfmt --edition 2024 --check` on `copilot.rs` and `tests/copilot_source.rs`: clean.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-core`: all pass (706 unit, 1 ignored as before; `claude_status` 11,
  `claude_transcript` 25, `codex_source` 16, `copilot_source` 42).
- No code path opens a lock or handshake file: the module has no I/O; locks come through `Names`, which only lists.
  Prompt text is never stored: no field reads `content` or `prompt`; a test checks every fixture's facts and `Debug`.
- The downloaded package (both tarballs and the unpacked bundle) and the build folder are deleted; the scratch folder
  keeps only the scripts.

**Open items.** Question 1. Everything marked "not verified" in the module doc goes to `p4-live-copilot`: the lock
name and its pid, the fire `source` format, interval timing, what a checkpoint prefix holds, write timing, and the
`tz` value. The tray wiring is that card's too (Round 2).

## Round 2: back in v1 (2026-11-06)

Written by the orchestrator. Haik, 2026-11-06: Codex CLI and Copilot CLI ship in v1 after all. Changes to this card:

- The agent may have changed since the research of 2026-11-03. First re-read the current release's own source
  (deliverable 1 already asks for it: `@github/copilot`, the current version, as data) for every fact this card relies on, and note the version and any change in this card's Log and Result
  (not in `docs/research.md` or `other-agent-signals.md`: the live cards carry them there).
- Haik's answers to sign-off items 1 and 2 (`docs/phases/02-06-signoff.md`) apply: Codex waiting for approval reads
  as working and holds with no limit; Copilot waiting for approval does not hold.
- The tray still starts the watch thread with the Claude source only (`crates/wakeshot-windows/src/app/tray_mode.rs`).
  Adding this source there is the live card's job, not this one's.
