# VS Code Local chats out of v1

Phase: 4. Status: ready once `p4-antigravity-exact-end` Part B is done (it owns `tray_mode.rs` now). New 2026-11-06
21:29. Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 50K tokens.
Runs after: `p4-live-vscode-stop`, `p4-antigravity-exact-end` Part B. Parallel with: `p6-agents-video-readme`.
Not with `p6-agents-install-e2e` Part 2.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-vscode-local-off\target`; delete it when done.
Real laptop: no live run needed; a `--dry-run` tray check only.

## Goal

Haik, 2026-11-06 21:28, on `p4-live-vscode-stop`'s question: "your default answer is good": Wakeshot does not hold
for VS Code's own Local chat files in v1, because a Local chat whose window closes mid-turn stays "working" until
VS Code exits, with nothing on disk to show the end ("No guessing", `docs/product.md`). Copilot harness chats in VS
Code stay supported (held by the Copilot source; every case exact, `p4-live-vscode-stop` Result).

## Read first

- `docs/product.md` ("No guessing", the agents line), `docs/tasks/p4-live-vscode-stop.md` Result,
  `docs/tasks/p4-live-vscode.md` Result.
- `crates/wakeshot-windows/src/app/tray_mode.rs` (the VS Code line), `crates/wakeshot-core/src/sources/vscode.rs`,
  the watch folders the VS Code source adds (`crates/wakeshot-windows/src/watch/*`).

## Deliverables

1. The tray no longer watches or holds for Local chat files: unwire the VS Code source (and any watch folder only it
   needs). Keep the source code and its tests for a later version. A test that fails if a Local chat file would hold
   through the app again.
2. Copilot harness chats in VS Code still hold: a test or a `--dry-run` tray check that shows it.
3. Docs: `docs/product.md` (the agents line: Copilot in VS Code means its Copilot harness chats; Local chats not in
   v1, with the reason), `docs/research/copilot-vscode-signals.md` and `docs/research.md` short lines,
   `docs/architecture.md` if it names the VS Code source as wired.
4. No contract or `Cargo` change; if one seems needed, stop and tell the orchestrator.

## Acceptance checks

- `cargo fmt` on your files; clippy `-p` and test `-p` for the packages you touched, with and without
  `--all-targets`, x64 and ARM64 (ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo command's
  PATH only).
- The dry-run tray check, if run, takes the lock
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\dry-run.lock`.

## Out of scope

- `README.md`, `site/`, `media/` (the video card updates the lists).
- Haik's VS Code windows: never touch them.

## Rules

- Never run git, not even read-only. Kill only processes you started, by PID. Never read token files. Never set
  `RUSTFLAGS`; cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06 21:29: card written by the orchestrator from Haik's answer.

## Result
