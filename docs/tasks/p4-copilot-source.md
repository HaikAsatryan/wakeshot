# Copilot CLI source

Phase: 4. Status: later (after v1, Haik 2026-11-03; `docs/phases/02-06-lighter.md`). Needs Haik: no (his live test is `p4-live-copilot`). Agent: `wakeshot-worker`. Size: about
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
