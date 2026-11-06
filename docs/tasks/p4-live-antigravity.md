# Live test: Antigravity CLI and IDE through Wakeshot's hooks

Phase: 4. Status: ready once `p4-antigravity-source`, `p4-live-cursor` and the hooks code review fixes are done (new
2026-11-06). Needs Haik: no (he signed in to the Antigravity CLI and IDE, 2026-11-06 ~17:30); only if the payload
check finds the command line.
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-antigravity-source`, `p4-live-cursor`, `p4-live-harness`; one live card at a time (agent-session
lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-antigravity\target`; delete it when done.
Real laptop: Antigravity CLI 1.5.2 and IDE 2.6.0 for Haik's user, Free plan. Writes Wakeshot's real plugin folder
under `%USERPROFILE%\.gemini\config\plugins\wakeshot\` through the product's own install code. No power settings.

## Goal

The live test of the Antigravity source and its hooks, as `docs/agent-hooks.md` "Live test" item 7 and
`docs/research/antigravity-signals.md` list it, redacted fixtures, fixes with tests, and the source wired into the
tray.

## Read first

- `docs/agent-hooks.md` ("Antigravity", "Live test", the privacy Gate), `docs/research/antigravity-signals.md`,
  `docs/tasks/p4-antigravity-source.md` and `docs/tasks/p4-live-cursor.md` Results, `docs/tasks/p4-live-harness.md`
  Result (runbook, driver; the IDE starts `agy.exe --hub`).

## Deliverables

1. The payload form first, at each hook run; command line: remove the plugin, stop, ask Haik (default: no plugin).
2. The plugin added by the product's own install code; then in the CLI and the IDE: the folder loads, the shell and
   the hook's parent, the cost per `PreInvocation`, a working turn, a turn that waits for approval, Esc (does `Stop`
   fire?), `fullyIdle` with a background command such as a dev server, a clean exit, a crash (kill by PID an
   `agy.exe` you started; never the IDE's if you did not start it), `{}` lets a stop through.
3. Each timed from the event to the post, with `agent-watch` and a new `--source antigravity` row.
4. Fixtures into `crates/wakeshot-core/tests/fixtures/antigravity/live/` (redacted), a replay test, fixes each with
   a test that failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`.
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/antigravity-signals.md` and short lines in
   `docs/research.md`; anything not run stays "not tested live".
7. Clean-up: the plugin folder removed through the product's remove path (the fresh install adds it again), every
   process you started ended by PID.

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never edit Haik's
own Antigravity or Gemini settings files.

## Owns

`crates/wakeshot-core/tests/fixtures/antigravity/live/`; fixes in `sources/antigravity.rs`, `sources/hooked.rs`,
`wakeshot-cli/src/hook.rs`, `agent_hooks.rs` and their tests; the Antigravity line in `tray_mode.rs` and in
`agent-watch.rs`; the sections above.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
