# Live test: Cursor editor and CLI through Wakeshot's hooks

Phase: 4. Status: ready once `p4-cursor-source`, `p4-agent-hooks-install`, the hooks code review fixes and the live
card before it in the lane are done (new 2026-11-06). Needs Haik: no (he signed in to Cursor and its CLI,
2026-11-06 ~17:30); only if the payload check finds the command line (design Question 4).
Agent: `wakeshot-worker`. Size: about 130K tokens.
Runs after: `p4-cursor-source`, `p4-agent-hooks-install`, the hooks review in Round 4 (fixes of blockers and
majors), `p4-live-harness`; one live card at a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-cursor\target`; delete it when done.
Real laptop: Cursor 3.27.12 and its CLI for Haik's user, signed in on Free. Writes Wakeshot's real plugin folder
`%USERPROFILE%\.cursor\plugins\local\wakeshot\` through the product's own install code. No power settings.

## Goal

The live test of the Cursor source and the hooks install, as `docs/agent-hooks.md` "Live test" lists it (items 0
to 6), redacted fixtures, fixes with tests, and the source wired into the tray.

## Read first

- `docs/agent-hooks.md` (all; "Live test" and the privacy Gate), `docs/tasks/p4-cursor-source.md` and
  `docs/tasks/p4-agent-hooks-install.md` Results, the hooks reviews in `docs/reviews/`, `docs/tasks/p4-live-harness.md`
  Result (runbook, driver).

## Deliverables

1. Live test 0 first: the payload form on this Cursor build, at each hook run. If any hook input comes on the
   command line, remove the plugin through the product's remove path, stop, and write it under "Questions for Haik"
   (design Question 4, default: no Cursor plugin at all).
2. The plugin added by the product's own install code (the tray path, `--dry-run` if it allows, else the call the
   install card names), then live tests 1 to 6 of the design: the editor loads the plugin with no prompt; each event
   runs the exe; the process found lives as long as the window; one turn id from prompt to stop; which CLI modes fire
   (interactive and `-p`); the delay per prompt with and without the hook; the exe renamed away (prompts still go);
   uninstall removes the folder and a folder the user changed stays.
3. Each timed from the event to the post, with `agent-watch` and a new `--source cursor` row.
4. Fixtures: redacted hook inputs and state files into `crates/wakeshot-core/tests/fixtures/cursor/live/` (no prompt
   text, no user paths, fake ids), a replay test, fixes each with a test that failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`.
6. A section "Phase 4 live test (2026-11-..)" in `docs/agent-hooks.md` and short lines in `docs/research.md`; the
   Cursor CLI's status per design Question 1 (covered only if its hooks fired).
7. Clean-up: the plugin folder removed through the product's remove path at the end (the fresh install adds it again),
   every process you started ended by PID.

## Orchestrator changes (2026-11-06 ~18:50, for speed; Haik asked for more parallel work)

- Runs after `p4-antigravity-source` (it adds the Cursor tray and harness lines), not after the hooks code review:
  the review runs at the same time. Runs in parallel with `p4-live-antigravity`; both may fix `sources/hooked.rs`,
  `hook.rs` or `agent_hooks.rs`, so this card owns those fixes and `p4-live-antigravity` reports its own to the
  orchestrator.
- Before the first write: copy `%USERPROFILE%\.cursor\plugins\` whole to `<scratchpad>\p4-live-cursor\backup\`, so a
  bug cannot lose anything; at the end compare the folder with the backup (only Wakeshot's folder may differ, and it
  is removed). Never restore over the user's files without telling the orchestrator.
- If the hooks code review reports a blocker while you run, the orchestrator tells you; stop writing real folders.
- First step (orchestrator, 2026-11-06 ~21:10): fix the hooks code review (`docs/reviews/p4-hooks-code.md`, 0
  blockers, 1 major, 7 minors) in the files you own this round (`sources/hooked.rs`, `sources/cursor.rs`,
  `wakeshot-cli/src/hook.rs`, `agent_hooks.rs`, `hooks.rs`, `app/startup.rs`, and the stale lines in
  `docs/agent-hooks.md` and `docs/tasks/p4-agent-hooks-install.md`): M1 (run the command-line privacy check before
  the parent walk gate, and also when the process list fails) and each minor that is right, with a test that fails
  before each fix. Write a "Review fixes" table in your Result (Fixed, or Not fixed with the reason). Then the live
  steps.

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never edit
`%USERPROFILE%\.cursor\hooks.json` or any Cursor settings file of Haik's; never close or kill a Cursor window you did
not start.

## Owns

`crates/wakeshot-core/tests/fixtures/cursor/live/`; fixes in `sources/hooked.rs`, `sources/cursor.rs`,
`wakeshot-cli/src/hook.rs`, `agent_hooks.rs` and their tests; the Cursor line in `tray_mode.rs` and in
`agent-watch.rs`; the sections above.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
