# Live test: Claude Desktop

Phase: 4. Status: blocked until Haik signs in to Claude Desktop (new 2026-11-07). Needs Haik: his sign-in; maybe
clicks the app needs (steps written by the worker).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-idle-cpu` (it updates Haik's install). Parallel with: non-Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-claude-desktop\target`; delete it when done.
Real laptop: Claude Desktop 2.42304.2, installed 2026-11-07 for Haik's user at Medium from Anthropic's signed
installer (winget `Anthropic.Claude`, `%LocalAppData%\AnthropicClaude`). No power settings.

## Goal

Haik, 2026-11-07: "I did not like that we did not check claude desktop." The README lists the Claude desktop app as
"not tested live yet". Test, live, whether Wakeshot holds exactly for work done in Claude Desktop, by the "No
guessing" rule (`docs/product.md`): every start and end of work seen exactly, or that case is not supported.

## Read first

- `docs/product.md` (Claude Code rules, "No guessing"), `docs/research/claude-code-signals.md`, `docs/tasks/p4-live-claude.md`
  Result (how a Claude live test ran), README "When each hold counts" and "Not tested live yet".
- `docs/tasks/p4-live-vscode-stop.md` (how a desktop app was driven without synthetic keys or clicks).

## Deliverables

1. What Claude Desktop runs that is work: its Code sessions (Claude Code inside the app), its own scheduled tasks
   and "Keep computer awake" setting, and plain chats. For each: does it write the files the Claude Code source
   reads (`~/.claude/sessions`, transcripts), and does Wakeshot see start, end, Stop, approval wait, app close and
   crash exactly. Plain chats run in the cloud: say whether a local signal exists at all.
2. A table in the Result: case, what Wakeshot showed, exact or not.
3. Fixes with tests for what can be made exact (Claude source files only); redacted fixtures (no prompt text, no
   user paths, fake ids).
4. Docs: `docs/research/claude-code-signals.md` section, `docs/research.md` short lines; the README lines the video
   card should use (supported, or "Not supported, and why").
5. If a step needs Haik (a click, a prompt typed in the app), stop early with exact short steps and reply.

## Rules

- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply.
- Never run git, not even read-only. Never read Claude's `.key` files or any token file. Prompt text is never logged.
  No synthetic key presses or clicks in Haik's apps. Kill only processes you started, by PID. Start programs at Medium
  through `explorer.exe`. Never set `RUSTFLAGS`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-07 02:29: card written by the orchestrator; Claude Desktop installed by the orchestrator.

## Result
