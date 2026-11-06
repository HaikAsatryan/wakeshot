# Cursor out of v1, and the hook folders earlier builds added removed

Phase: 4. Status: ready (new 2026-11-06 23:22). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p6-agents-install-e2e`. Runs alone among Rust cards; beside the video card (`media/` only).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cursor-off\target`; delete it when done.
Real laptop: yes, at the end: Haik's install updated once more (the way `p6-agents-install-e2e` did it).

## Goal

Haik, 2026-11-06 23:21: "ok le'ts drop cursor as well." Why: a Cursor chat deleted while it runs sends no `stop`,
only the `sessionEnd` that Ctrl+F4 also sends, so the hold lasts until the window closes (`p6-agents-install-e2e`
Result; "No guessing" in `docs/product.md`). v1 agents: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code
(Copilot harness chats), OpenCode 2.x. With Cursor and Antigravity out, v1 adds no hook folder to any agent.

Haik's installed tray (the `p6-agents-install-e2e` build) already added Wakeshot's Cursor plugin folder to his
profile. Left there, its hook runs on every Cursor prompt (about 0.85 s each) for nothing.

## Read first

- `docs/product.md` ("No guessing", the agents and hooks lines), `docs/agent-hooks.md` (adding, removing, the
  notice), `docs/tasks/p4-vscode-local-off.md` (how Antigravity was unwired), `docs/tasks/p6-agents-install-e2e.md`
  (Result, and how Haik's install was updated).

## Deliverables

1. The tray no longer watches Cursor and adds no hook folder at start. Keep the source code, the hook command and
   their tests for later; they must not run in the app. A test that fails if the app would watch Cursor or add a
   folder again.
2. At start, the tray removes a Wakeshot hook folder an earlier build added (Cursor and Antigravity), through the
   product's own remove path: an unchanged folder is deleted, a folder the user changed is moved aside as today.
   Uninstall keeps removing them too. Tests for both, each failing before. If the "hooks added" notice no longer
   has a use, retire it with the least change; say what you did.
3. No contract or `Cargo` change; if one seems needed, stop and tell the orchestrator.
4. Docs: `docs/product.md` (agents line, hooks line: Cursor out of v1 with the reason and Haik's words),
   `docs/agent-hooks.md` (README lines: v1 adds no hooks), `docs/architecture.md` if it names Cursor as wired,
   `docs/research/cursor-gemini-signals.md` or the Cursor research file and `docs/research.md` short lines.
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks), then pack x64 and update Haik's install the way
   `p6-agents-install-e2e` deliverable 3 did (WM_CLOSE to `Wakeshot.Loop`, `Setup.exe --silent` at Medium through
   `explorer.exe`, hash check, `config.json` kept, tray started through `explorer.exe`, Medium). Then check:
   Wakeshot's Cursor folder is gone from `%USERPROFILE%\.cursor\plugins\local\`, no Antigravity folder exists, one
   cheap Claude Code turn and one Codex turn still hold and let go, and Haik's lid and sleep values are his own
   (lid Sleep on AC and DC, 18000 on AC and DC) apart from the orchestrator's own Claude hold.

## Out of scope

- `README.md`, `site/`, `media/` (the video card updates the lists).
- Cursor windows and Cursor settings of Haik's: never touch them; only Wakeshot's own plugin folder, through the
  product's remove path.

## Rules

- Never run git, not even read-only. Never kill Haik's tray: only WM_CLOSE. Never run another keep-awake app. Start
  programs at Medium through `explorer.exe`. Kill only processes you started, by PID. Never read token files.
  Prompt text is never logged. Never set `RUSTFLAGS`; cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo
  command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06 23:22: card written by the orchestrator from Haik's answer.

## Result
