# Claude Code source: transcripts (timers and background tasks)

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

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

## Result

What was done, files changed, checks and outcomes, open items.
