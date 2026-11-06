# VS Code Local chats out of v1

Phase: 4. Status: done, 2026-11-06. New 2026-11-06 21:29. Needs Haik: no.
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

## Antigravity out of v1

Added by the orchestrator, 2026-11-06 about 21:50, from Haik: "Yes remove antigravity. Too much problematic." Why:
the Antigravity CLI guard pins `agy` 1.5.2, and `agy` releases almost daily (its `CHANGELOG.md` was updated on about
25 days since 2026-10-02), so it would hold nothing most of the time; no other exact signal exists for Esc
(`p4-antigravity-exact-end` Result).

1. Unwire Antigravity: the tray no longer watches it (`tray_mode.rs`) and at start no longer adds its plugin folder
   (the `add_agent_hooks` path; Cursor's add stays). Keep the remove path, so `wakeshot hook antigravity remove` and
   uninstall still clean an old folder. Keep the source code, its log reader and their tests for later; they must not
   run in the app. A test that fails if the app would watch Antigravity or add its folder again.
2. Docs: `docs/product.md` (the agents line and the Antigravity line: not in v1, with the reason and Haik's words),
   `docs/agent-hooks.md` (README lines: no Antigravity in v1), `docs/research/antigravity-signals.md` and
   `docs/research.md` short lines.

Files for this part: `crates/wakeshot-windows/src/agent_hooks.rs`, `crates/wakeshot-windows/src/app/startup.rs`, the
Antigravity parts of `examples/agent-watch.rs` only if they break, and those docs. No contract or `Cargo` change. The
final reply also gives the README and video line for the agent list.

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

1. `crates/wakeshot-cli/src/hook.rs` (not this card's file) still answers `wakeshot hook antigravity <event>`. No v1
   start adds a folder that runs it, but a folder an earlier build added (the test laptop may have one) still runs it,
   and it writes chat files in `agents\antigravity\` that nothing reads now (a `Stop` deletes them). Harmless; for the
   card that owns `hook.rs`: refuse the Antigravity events in v1, or leave it while no user has such a folder.
2. `tray_mode::watched_sources` keeps the old line "the profile folder is unknown, so no agent is watched", which was
   already wrong before this card (Cursor is watched then too). Left as it was (not this card's change).
3. `docs/plan.md` (line 301, v1 agents) and `docs/phases/04-sources.md`, `docs/phases/06-ship.md` still name
   Antigravity and Copilot in VS Code without the two v1 cuts; the orchestrator's files.

## Log

- 2026-11-06 21:29: card written by the orchestrator from Haik's answer.
- 2026-11-06: worker started; `p4-antigravity-exact-end` is done, so `tray_mode.rs` is free (orchestrator).
- 2026-11-06: read. The tray adds `VsCodeSource` in `tray_mode::sources_in`; its only watch folders come from its
  two `Recent` wants, so unwiring the source drops them. No `watch/*` file has a VS Code folder of its own; the
  `Recent` reader there is a contract variant (`sources.rs`), kept for the later version (removing it is a contract
  change). Plan: drop the source and its two env vars from `tray_mode.rs`, one `watched_sources` that `serve` calls
  and the tests call, and two tests that run real files through the watch's `Agents` and the core's `Sim`.
- 2026-11-06 21:52: scope added by the orchestrator: Antigravity out of v1 (section above). Plan: `hooked_sources`
  keeps Cursor only; `agent_hooks::at_start` adds only the agents of a new start list (Cursor), while `AGENTS` (remove
  and uninstall) keeps Antigravity; its add tests run through an inner start that takes the list.
- 2026-11-06: VS Code unwired in `tray_mode.rs` (`watched_sources`, `AgentEnv` without `APPDATA` and
  `VSCODE_APPDATA`); the old Copilot, VS Code and OpenCode place test now covers Copilot and OpenCode.
- 2026-11-06: Antigravity unwired: `hooked_sources` gives Cursor only; `agent_hooks::ADDED_AT_START` (Cursor) drives
  `at_start` through a new `start_for`; `AGENTS` keeps both for remove and uninstall; the Antigravity add tests run
  through `World::start_all`. Four new tests pass; clippy x64 `--all-targets` clean. Mutation check: with VS Code put
  back in `sources_in` and Antigravity in the start list, five tests fail; with `at_start` over all `AGENTS`, the new
  hooks test fails on `["Cursor", "Antigravity CLI"]`. Both files restored from copies, checked with `cmp`.
- 2026-11-06: docs: `product.md`, `agent-hooks.md`, `architecture.md`, `research/copilot-vscode-signals.md`,
  `research/antigravity-signals.md`, `research.md`; `sources/vscode.rs` module doc says it is not started in v1.
- 2026-11-06 21:50: checks: rustfmt clean; clippy `-p wakeshot-core -p wakeshot-windows` x64 and ARM64, with and
  without `--all-targets`: clean (4 runs); tests x64: all pass (core 737 + integration, windows 720, 0 failed).
- 2026-11-06 21:54:05 to 21:54:56: dry-run tray check. Lock taken 21:54:05; debug tray at Medium through Explorer
  (pid 48996, its `cmd` 49964), own data folder. Its log names five sources (Claude Code, Codex CLI, Copilot CLI,
  OpenCode, Cursor), no VS Code or Antigravity line; it held for Claude Code (this agent's own session). Quit by
  `taskkill /PID 48996` (no force), code 0; lock released 21:54:56; its data, exe and `.cmd` deleted.
- 2026-11-06 21:56: build folder and my scratch files deleted; no process of mine runs. Done.

## Result

Status: done, 2026-11-06. Both parts: VS Code Local chats out of v1, and Antigravity out of v1.

**What changed**

- Tray (`app/tray_mode.rs`): `VsCodeSource` is no longer started, and `APPDATA` and `VSCODE_APPDATA` are no longer
  read; `hooked_sources` gives Cursor only. One `watched_sources` builds the whole list; `serve` and the tests both
  call it. The v1 tray starts five sources: Claude Code, Codex CLI, Copilot CLI, OpenCode, Cursor.
- Agent hooks (`agent_hooks.rs`): a start adds and keeps only the agents of the new `ADDED_AT_START` (Cursor), through
  a new `start_for`. `AGENTS` keeps Antigravity, so `wakeshot hook antigravity remove` and the uninstall still remove
  a folder an earlier build added. A start leaves such a folder alone. `startup.rs` needed no change.
- Kept for later, not run in the app: `sources::vscode`, `sources::antigravity` (with its log reader), the
  Antigravity add code, and all their tests. The `Recent` reader in `watch/` stays: it is a contract variant, and
  removing it would be a contract change. No contract or `Cargo` change.

**Tests** (each fails when VS Code or Antigravity is put back: mutation check in the Log)

- `the_tray_starts_the_v1_sources_only`: the exact source list, with and without a profile folder.
- `a_local_vs_code_chat_holds_nothing_through_the_tray`: the live "window closed mid-turn" chat file, plus a
  `Code.exe` started before its send, holds through `VsCodeSource` (the control), and holds nothing through the tray's
  sources. No folder under `AppData\Roaming` is watched.
- `a_copilot_harness_chat_in_vs_code_holds_through_the_tray_until_its_stop`: the live harness log, held by
  `copilot-runtime.exe`, holds through the tray, and the hold ends at its `abort` while the Local chat on disk still
  says working.
- `a_start_adds_no_antigravity_folder_and_the_uninstall_still_removes_an_old_one`.
- Each of the last three runs real files in a scratch profile through the watch's own `Agents` and feeds the events to
  the core's `Sim`. The old Antigravity add tests now run through `World::start_all`.

**Checks**: rustfmt clean on my three Rust files. Clippy `-p wakeshot-core -p wakeshot-windows`, x64 and ARM64, with
and without `--all-targets`: clean. Tests x64: core 737 unit (1 ignored) and every integration test, windows 720 (28
ignored), 0 failed. Dry-run tray (Medium): five source lines, no VS Code or Antigravity line, quit with code 0.

**Files changed**: `crates/wakeshot-windows/src/app/tray_mode.rs`, `crates/wakeshot-windows/src/agent_hooks.rs`,
`crates/wakeshot-core/src/sources/vscode.rs` (module doc only), `docs/product.md`, `docs/agent-hooks.md`,
`docs/architecture.md`, `docs/research/copilot-vscode-signals.md`, `docs/research/antigravity-signals.md`,
`docs/research.md` (new last section), this card.

**README and video lines** (for `p6-agents-video-readme`):

- Agent list: "Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code, OpenCode 2.x, Cursor".
- VS Code line: "Copilot in VS Code: chats that run on the Copilot harness. VS Code's own Local chats are not
  supported: when you close a window mid-turn, VS Code writes nothing that shows the work ended." (New chats went to
  the harness by default on the test laptop; whether that holds for every user is not verified, so the line does not
  claim it.)
- Antigravity: not named as supported anywhere. If the README lists what is not supported: "Antigravity: not
  supported. It leaves no reliable sign when you stop it with Esc." (`agent-hooks.md`, "README lines", has the Cursor
  and uninstall lines without Antigravity.)

**Decided by the worker**, each with when to change it:

1. A start leaves an old Antigravity folder alone instead of removing it: no user has one (v1 is not out), and a
   start that deletes in another app's folder is a new behavior. Change it when a released build ever added that
   folder: then remove it at start, by the fixed names.
2. One `watched_sources` for `serve` and the tests, so the "never again" tests see exactly what the tray starts.
   Change it never; a new source goes in there.
3. No log line says VS Code Local chats or Antigravity are not watched: the README says it. Change it when users
   report confusion.

**Open items**

1. "Found outside scope" 1 to 3 (the CLI still answers `hook antigravity <event>`; an old log text; plan and phase
   files still list both).
2. The VS Code harness hold through the real dry-run tray was not run live (it needs a VS Code chat; Haik's windows
   are never touched). The test with the live harness log covers it.
