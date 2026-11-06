# Live test: Stop, approve and window close in Copilot in VS Code

Phase: 4. Status: ready (new 2026-11-06 19:30, Haik's "No guessing" rule). Needs Haik: no, unless no command path
can press Stop or approve; then one short sitting (steps written by the worker).
Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p4-live-vscode`. Parallel with: `p4-live-cursor`, `p4-antigravity-exact-end`, `p6-agents-video-readme`.
Not with `p6-agents-install-e2e` Part 2.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-vscode-stop\target`; delete it when done.
Real laptop: VS Code with Copilot Free for Haik's user, in your own window only. No power settings.

## Goal

Haik, 2026-11-06 18:45: "We cannot guess or randomly sleep. It's better not support something or do so." The
`p4-live-vscode` card did not test Stop, approve after a wait, or a window close while a chat works (they needed a
click). Find out, live, whether each one ends or keeps the hold exactly. If a Stop can leave the chat "working" with
no signal Wakeshot can see, that is a blocker for VS Code support: write it plainly, never add a timer or a cap.

## Read first

- `docs/product.md` ("No guessing"), `docs/tasks/p4-live-vscode.md` (Rules for every live card, Log, Result: how it
  opened its own window, the Copilot harness chats held by the Copilot source, the Local chat files' 60 s beat).
- `docs/tasks/p4-live-antigravity.md` Log and Result, item 4 of "Decisions": a scratch extension
  (`--extensionDevelopmentPath`, nothing installed) ran IDE commands in its own window. Its tools are in
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p4-live-antigravity\ideext\`.

## Deliverables

1. In your own VS Code window (never Haik's; his VS Code runs elevated), drive the chat with VS Code commands from a
   scratch extension (for example the chat cancel and accept commands; check the names in the VS Code version
   installed), for both chat kinds that hold today (Copilot harness chats, Local chat files):
   - Stop during a working turn (a command that runs about 60 s), 3 runs each;
   - an approval wait left 1 minute, then approved, and one then denied;
   - the window closed while a turn works.
   Each timed from the action to the post (`agent-watch --source copilot,vscode`).
2. A table in the Result: case, chat kind, what Wakeshot showed after it, exact or not.
3. Fixes for what is not exact but can be made exact, in `sources/vscode.rs` or `sources/copilot.rs`, each with a test
   that failed before; redacted fixtures into `crates/wakeshot-core/tests/fixtures/vscode/live/` (no prompt text, no
   user paths, fake ids).
4. A short section in `docs/research/copilot-vscode-signals.md` and short lines in `docs/research.md`.
5. If no command path can press Stop or approve: stop early and write exact, short steps for Haik (one sitting, in a
   window you opened), then reply; the orchestrator relays them.

## Acceptance checks

- Every case in deliverable 1 run, or marked with why not; each "exact" backed by the runs.
- Fixes: a test that failed before each; `cargo fmt` on your files; clippy `-p` and test `-p` for the packages you
  touched, x64 and ARM64 (ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo command's PATH
  only).
- Clean-up: your window and every process you started ended by PID; the scratch extension never installed; build
  folder deleted.

## Out of scope

- `tray_mode.rs`, `agent-watch.rs`, `watch/*`: report a bug there, do not fix it.
- Cursor, Antigravity and their folders (other cards run live there now).

## Rules

- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply (dry-run lock:
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\dry-run.lock`).
- Never run git, not even read-only. Never touch Haik's VS Code windows. No synthetic key presses or clicks: they
  can reach Haik's foreground app. Never read token files. Prompt text is never logged. Never set `RUSTFLAGS`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06 19:30: card written by the orchestrator.

## Result
