# Live test: OpenCode 2.x

Phase: 4. Status: ready once `p4-opencode-source` Part B, its review fixes, and the live card before it in the lane
are done (new 2026-11-06). Needs Haik: no (OpenCode 2.2.5 runs on Ollama with `qwen3:4b-instruct`, no login).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-opencode-source`, `docs/reviews/p4-opencode-db-read.md` fixes, `p4-live-harness`; one live card at
a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-opencode\target`; delete it when done.
Real laptop: OpenCode and Ollama for Haik's user. No power settings, no screen captures.

## Goal

The live test of the OpenCode 2.x source: the rules of `docs/research/opencode-signals.md` and the build of
`p4-opencode-source` on a real OpenCode, redacted fixtures, fixes with tests, and the source wired into the tray.

## Read first

- `docs/research/opencode-signals.md` ("What is left" and the 2.x rules), `docs/tasks/p4-opencode-source.md` Result
  and Haik's answers, `docs/reviews/p4-opencode-db-read.md`, `docs/tasks/p4-live-harness.md` Result (runbook,
  driver, the OpenCode steps it already ran).

## Deliverables

1. Sessions at Medium, each timed (write to post): a working turn in the TUI and one with `opencode run`, a
   permission prompt left 1 minute then approved, Esc during a turn, a clean exit, a crash (kill by PID an
   `opencode.exe` you started), a schedule due in a few minutes (fires while the server runs; the 1 hour rule), and a
   crash leftover row with a newer OpenCode running (the `ImageSince` limit in the Result).
2. Privacy and safety, live: while Wakeshot reads, OpenCode is never blocked (no `SQLITE_BUSY` error in OpenCode's
   own log), Wakeshot creates no `-wal` or `-shm` file and opens the database read only (Process Monitor is not
   installed: use handle and file time checks you can run at Medium; say what you used), and no row value reaches the
   harness or app log.
3. Run `agent-watch` with a new `--source opencode` row and record what it posted per step.
4. Fixtures: redacted rows into `crates/wakeshot-core/tests/fixtures/opencode/live/` (no prompt text, no user
   paths, fake ids), a test that replays them, fixes in `sources/opencode.rs` or `sqlite.rs` each with a test that
   failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`
   (never beside another dry-run card; never quit Haik's installed tray).
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/opencode-signals.md` and short lines in
   `docs/research.md`; anything not run stays "not tested live".

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never read
OpenCode's `service.json`; never open `opencode.db` yourself except through Wakeshot's own read path.

## Owns

`crates/wakeshot-core/tests/fixtures/opencode/live/`; fixes in `sources/opencode.rs`, `tests/opencode_source.rs` and
`crates/wakeshot-windows/src/sqlite.rs`; the OpenCode line in `tray_mode.rs` and in `agent-watch.rs`; the research
sections above.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
