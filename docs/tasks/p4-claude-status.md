# Claude Code source: status files

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: the contracts card (it creates `sources/claude/mod.rs`, `status.rs`, `transcript.rs` and changes
`AgentSource::feed` to return `Fed`). Parallel with: every other card of this track except `p4-claude-transcript`,
which takes over `claude/mod.rs` after this card; every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-claude-status\target`. Uses the real laptop: no.

## Goal

`ClaudeSource`, an `AgentSource` that reads `~/.claude/sessions/<pid>.json` and reports each session's activity,
liveness and format health, by rules 1 to 4 of `research/claude-code-signals.md`. Transcripts come in the next card.

## Read first

- `CLAUDE.md` "Facts to keep in mind" (status files, `.key` files).
- `docs/phases/04-sources.md`: track notes 3 and 6.
- `docs/architecture.md` section 7.
- `docs/research/claude-code-signals.md`: "Status file fields", "Status values and transitions", "Recommended
  detection rules" 1 to 4, live tests 7, 8, 10, 11, and "Other facts seen live".
- `crates/wakeshot-core/src/sources.rs` (after the contracts card), `testing.rs` (`FakeSource`).
- `spikes/claude-watch/src/names.rs`, `watch.rs` (`first_object`), `fixtures/README.md`, `fixtures/live-tests.log`.

## Deliverables

1. `crates/wakeshot-core/src/sources/claude/status.rs`: the status file parser. First JSON value only (stray bytes
   after it are seen in the wild); fields read: `pid`, `sessionId`, `procStart` (or `procStartFt`), `status`,
   `waitingFor`, `statusUpdatedAt`, `bridgeSessionId`, `entrypoint`, `kind`; nothing else is kept. Mapping:
   `busy`, `shell` give `Working`; `waiting` gives `Waiting { since_wall_ms: statusUpdatedAt, remote:
   bridgeSessionId is a string }`, except `waitingFor: "dialog open"`, which is `Idle`; `idle` and a missing status
   give `Idle`; any other value gives `Working` plus `FormatHealth::UnknownValue("status", value)` (the value cut to
   32 ASCII characters). `pid` in the file must equal the pid in the name.
2. `crates/wakeshot-core/src/sources/claude/mod.rs`: `ClaudeSource::new(home: PathBuf)` (`p4-claude-transcript` adds
   the zone parameter once `sources::cron` exists; `p4-cron` runs at the same time as this card). `wanted()`: `Folder { dir: <home>\.claude\sessions, accept }`
   where `accept` takes `<digits>.json` only (port `status_file_pid` and its tests: `.key` files, `.fleetview-heartbeat`
   and temp names never pass). `feed`: `Whole` parses; a parse failure answers `Fed::RetryOnce` the first time and
   gives `Unreadable("status file")` after the retry; `Gone` drops the session. `sessions()`: one `SessionFacts` per
   status file, `key.id` the pid as text (track note 3), `liveness: Process { pid, start: procStart }`, `label: "Claude
   Code"`, `timers: []`, `open_tasks: 0`. A file with no `procStart` cannot be tied to its process: `Unreadable`.
3. Fixtures: `crates/wakeshot-core/tests/fixtures/claude/status/`, one status file per shape in `live-tests.log`
   ("Where each shape is": before the first status, `idle` with Remote Control, SDK path, each `waitingFor`, Remote
   Control off), plus broken ones (empty, half written, trailing bytes, `pid` mismatch, unknown status, no `procStart`).
   Built from the redacted log; keep its redaction rules (`fixtures/README.md`).
4. Tests first: `crates/wakeshot-core/tests/claude_status.rs` runs every fixture through `ClaudeSource` and checks the
   facts; unit tests in `status.rs` for each mapping rule; a test that `accept` refuses every `.key` name form.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- No fixture holds a real user name, session id, bridge id or prompt (the secret scan patterns in
  `docs/phases/01-design-skeleton.md`, "Secret scan", find nothing new in the fixtures folder).
- The parser keeps no field beyond the list in deliverable 1 (review the struct; say so in the Result).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Transcripts, timers, background tasks of non-terminal sessions (`p4-claude-transcript`). Liveness checks and file
  reads (watch cards). Policy: the waiting hour, Remote Control, timer margins (core track). Any file not listed above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
