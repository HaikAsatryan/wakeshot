# Antigravity: see the end of work exactly (Esc, background commands), or leave v1

Phase: 4. Status: ready (new 2026-11-06 18:50, Haik's "No guessing" rule in `docs/product.md`). Needs Haik: no for
Part A; the IDE first-run pass only if Part A finds Antigravity can stay (`p4-live-antigravity` Result, Open items 1).
Agent: `wakeshot-worker`. Size: about 150K tokens (Part A about 70K, Part B about 80K).
Runs after: `p4-live-antigravity` (CLI part done). Part A: now, beside `p4-live-cursor`. Part B (code): after
`p4-live-cursor` is done, since that card owns the fixes in `sources/hooked.rs`, `wakeshot-cli/src/hook.rs` and
`agent_hooks.rs` this round.
Parallel with: `p4-live-cursor`, `p6-agents-video-readme` Part C. Not with `p6-agents-install-e2e` Part 2.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-antigravity-exact-end\target`; delete it when done.
Real laptop: Antigravity CLI 1.5.2 for Haik's user (signed in, Free). A scratch capture plugin in
`%USERPROFILE%\.gemini\config\plugins\` while Part A runs, removed at the end. No power settings.

## Goal

Haik, 2026-11-06 18:45: "I do not accept answer that we cannot track whether antigravity works or not. We cannot
guess or randomly sleep. It's better not support something or do so." Today, after an Esc, Antigravity fires no
`Stop`, so Wakeshot shows "working" until the next prompt ends or `agy` exits; and a background command holds only
when `agy` says `fullyIdle: false` (2 of 4 runs). Find an exact signal for both. Build what is exact. What is not
exact is not supported; if the end of a turn after Esc cannot be seen exactly, Antigravity leaves v1.

## Read first

- `docs/product.md` ("No guessing", the hooks line), `docs/agent-hooks.md` (privacy Gate, events, "Live test"),
  `docs/research/antigravity-signals.md` (events table: `PostInvocation`, `PreToolUse`, `PostToolUse` were never
  tested on Esc).
- `docs/tasks/p4-live-antigravity.md` Log (17:33 to 17:50: the Esc and background runs) and Result. Its scratch tools
  are in `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p4-live-antigravity\`
  (`run\`, `ideext\`, `addhooks\`, the capture plugin's source): reuse them.

## Part A: the facts (live, no product code)

1. A scratch capture plugin that logs every hook event `agy` offers (`SessionStart`, `PreInvocation`,
   `PostInvocation`, `PreToolUse`, `PostToolUse`, `Stop`) with times and the non-text fields only (event name,
   `conversationId`, `invocationNum`, `stepIdx`, tool name, `error`, `terminationReason`, `fullyIdle`). Never the
   prompt, `lastUserInput`, tool args or the transcript.
2. Esc in each place a turn can be: during a model call, during a tool run, during an approval wait, and during a
   long streamed answer. For each: which events fire after the Esc and with which fields (an `error` or a cancel
   marker on `PostInvocation` or `PostToolUse` would be exact). At least 3 runs each.
3. Background commands: a dev-server-like command (`ping -n 60`) started in the background, 4 runs or more: which
   events show its start and its end, and whether anything exact tells "a background command still runs".
4. Ctrl+C in the TUI, and a second prompt typed while one runs, if `agy` allows it.
5. Cost: the added time per model call and per tool call with the extra hooks (release `wakeshot.exe`, 10 runs).
6. Write the facts in the Log as you go, then a short table in the Result: case, events seen, exact or not.

Exact means: every state Wakeshot shows maps to an event or to the `agy` process's life, never to a timer or to the
absence of an event. A state that can stay "working" after the work ended is not exact.

## Part B: build or remove (after `p4-live-cursor` is done)

- **If Esc can be seen exactly:** the source follows the exact rule (`sources/antigravity.rs`, the shared hooked
  code, the plugin's event list in `agent_hooks.rs`, the hook helper), each case with a test that fails before; the
  privacy Gate still holds (a test). Background commands: hold only if Part A found an exact start and end; else no
  hold for them at all (ignore `fullyIdle`), with a test.
- **If not:** Antigravity is not supported in v1. Unwire it: the tray no longer watches it and `at_start` no longer
  adds its plugin folder (keep the remove path, so `wakeshot hook antigravity remove` and uninstall still clean an old
  folder). Keep the source code and its tests for later. No contract or `Cargo` change; if one seems needed, stop and
  tell the orchestrator.
- Either way: `docs/agent-hooks.md`, `docs/research/antigravity-signals.md`, `docs/research.md` short lines,
  `docs/product.md` (the agent list line and the hooks line). Checks: `cargo fmt` on your files, `cargo clippy -p`
  and `cargo test -p` for the packages you touched, x64 and ARM64 (ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin`
  folder on that cargo command's PATH only).
- The video and README list is not yours: write the outcome in the Result; the orchestrator passes it on.

## Acceptance checks

- Part A table complete for every case in steps 2 to 4; each "exact" backed by at least 3 runs.
- Part B: every new rule has a test that failed before; the privacy test passes; clippy and tests green on x64 and
  ARM64 for the touched packages.
- Clean-up: the capture plugin removed; every process you started ended by PID; build folder deleted.

## Out of scope

- The Antigravity IDE turn (only if Part A says Antigravity stays; then the orchestrator asks Haik for the IDE
  first-run pass and a worker runs it).
- Any timer, cap or "no event for N seconds" rule: not allowed by Haik's rule.
- `%USERPROFILE%\.cursor` and anything of `p4-live-cursor` (its monitors run until about 21:40).

## Rules

- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply.
- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never run `agent_hooks::at_start`, or a tray that runs it, on the real profile (`p4-live-cursor` tests hooks there).
  A dry-run tray takes turns through
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\dry-run.lock`.
- Start `agy` and drivers at Medium through `explorer.exe`. Kill only processes you started, by PID. Never read token
  files. Prompt text is never logged. Never set `RUSTFLAGS`; cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06 18:50: card written by the orchestrator from Haik's rule.

## Result
