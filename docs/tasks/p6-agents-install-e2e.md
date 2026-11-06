# The bigger v1 installed: checks, a fresh install over Haik's, and a hold per agent

Phase: 6. Status: ready once every `p4-live-*` card of the bigger v1 and its fixes are done (new 2026-11-06). Needs
Haik: no (he is at the laptop for normal work; the tray quit and start take seconds).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p4-live-cursor`, `p4-live-antigravity`, `p4-live-opencode` (with its poll fix), and the hooks code
review verify. Runs alone: no other card builds, runs an agent or a dry-run tray while it runs.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-agents-install-e2e\target`; packaging output under the same folder.
Real laptop: yes. Quits and restarts Haik's installed tray; the new tray holds for real (real power requests and the
lid and battery settings it owns), then restores them.

## Goal

Prove the bigger v1 works as users get it: the whole workspace is green, Haik's install is replaced by the new build
the way an update would, and the installed app holds the PC for each agent and lets go when the agent stops.

## Read first

- `CLAUDE.md` (Checks, ARM64 note, Test laptop), `docs/phases/06-ship.md` work log (2026-11-05 20:48 and 2026-11-06
  ~05:00: how Haik's install was updated before), `packaging/` (the pack script and `e2e.ps1`).
- Each `p4-live-*` card's Result of the bigger v1 (how to start a cheap turn in each agent).

## Deliverables

1. Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (fmt; clippy
   with and without `--all-targets`; `cargo test --workspace --locked`; release builds). Any failure: stop and tell
   the orchestrator with the failing test and file; never fix another card's file.
2. Pack the x64 release the way `packaging/` does (Setup.exe and the package), and record its size and hash.
3. Replace Haik's install: quit his tray only by `WM_CLOSE` to its `Wakeshot.Loop` window (it restores its own
   settings; wait for its exit and check the code), run `Setup.exe --silent` at Medium through `explorer.exe`, check
   the installed exe hash equals the build and `config.json` is kept, start the tray through `explorer.exe`, and check
   tray and guard run at Medium.
4. One cheap turn per agent through the installed app, one at a time, each prompt one line asking for one word or
   `0`: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code (a chat on the Copilot harness; never touch Haik's
   own windows: open your own with `code -n` as `p4-live-vscode` did), OpenCode 2.x, Cursor (editor and, if it
   fires, the CLI), Antigravity (CLI and IDE). For each: the tray's log shows the hold start within the time the live
   card measured and its end after the turn; `powercfg /requests` shows Wakeshot's request during the turn and none
   after; the plugin folders for Cursor and Antigravity were added by the tray with one notice.
5. After the last turn: no Wakeshot request left; the lid action and battery sleep timer equal Haik's values from
   `CLAUDE.md` (Test laptop); the tray still runs.
6. Result: a table (agent, hold start delay, hold end delay, pass or fail), the build hash, anything that failed.
   A failing agent is listed, not fixed here.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never kill Haik's tray: only `WM_CLOSE`. Never run another keep-awake app. Kill only processes you started, by PID.
- Never read token files. Start agents and the driver at Medium through `explorer.exe`.
- Never set `RUSTFLAGS`. Cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
