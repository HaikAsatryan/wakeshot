# Contracts round 3: what the agent hooks build shares

Phase: 4. Status: ready once `p4-contracts-r2` and the hooks security review are done (new 2026-11-06). Needs Haik:
no.
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p4-contracts-r2`, `p4-hooks-design`, the security review `docs/reviews/p4-hooks-design-security.md`
(its fixes to the design applied first). Parallel with: docs-only and agent-session cards. It is the only card that
builds inside the Rust workspace while it runs.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-contracts-r3\target`; delete it when done.
Real laptop: no installs, no power settings, no screen.

## Goal

The shared changes the Cursor and Antigravity hooks build needs, in one card run alone, so `p4-cursor-source`,
`p4-agent-hooks-install` and `p4-antigravity-source` touch only their own files and build with `--locked`.

## Read first

- `CLAUDE.md` (Checks, cargo path, ARM64 note), `docs/architecture.md` sections 2, 6 and 12.
- `docs/agent-hooks.md`, all of it; "Contract changes the build needs" is your list.
- `docs/reviews/p4-hooks-design-security.md`: every finding that changes a contract.
- `docs/tasks/p4-contracts-r2.md` Result (what round 2 added).

## Deliverables

Every item of "Contract changes the build needs" in `docs/agent-hooks.md`, as the design says after the review's
fixes (six items on 2026-11-06 ~16:40: `values` in `Command::Hook`, `current_above_medium()` and
`harden_dll_search()`, and `write_atomic` with `CREATE_NEW` for every caller were added). The list below is the
first version; where it differs, the design wins:

1. `wakeshot-cli`: `Command::Hook { agent, event }` in `command.rs`, its line in `main.rs`, the parse in `args.rs`
   (hidden from help), a stub `hook.rs` (`Owner: p4-cursor-source`) that prints `{}` and exits 0, so the command is
   safe even before the build.
2. `wakeshot-windows::process`: `ProcessEntry.parent: u32` and `image_path(pid)`.
3. `wakeshot-core`: `Event::HooksAdded`, `Notice::HooksAdded`, the core's answer, the words in `text.rs` (the design
   gives them; house tone).
4. `wakeshot-windows::hooks`: `HookEnv::remove_agent_hooks`, called in `on_uninstall` after `remove_notices`, with a
   stub body and the fake updated.
5. A stub `wakeshot-core/src/sources/hooked.rs` (`Owner: p4-cursor-source`); the `Owner:` line of
   `crates/wakeshot-windows/src/agent_hooks.rs` set to `p4-agent-hooks-install`.

6. **OpenCode review major 1** (`docs/reviews/p4-opencode-db-read.md`; added by the orchestrator 2026-11-06 ~18:10,
   since it is watch thread plumbing from `p4-contracts-r2` and no other card may edit `watch/*` now): a `Query` want
   re-runs only on change notices for `<db>-wal` or `<db>`, never `-shm`, so a crash-left `-shm` cannot loop reads.
   A test that fails before the fix (the reviewer's probe scripts are in
   `<scratchpad>\p4-opencode-review\scripts\`, data only).

7. `docs/architecture.md` section 7: the Codex "can outlive its session" mark now applies only to the fallback when
   Restart Manager fails (`docs/tasks/p4-live-codex.md` Result, Restart Manager named the session's own `codex.exe`).

Tests for each new piece that has behavior (the parse, the stub command's output and exit code, the parent field,
the core's answer to `HooksAdded`). `docs/architecture.md`: lines in sections 6 and 12 for what you added (re-read
right before each edit).

## Acceptance checks

- Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, every one with
  `--locked`. The ARM64 run needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that command's PATH only. Record
  each result.
- No `Cargo` change, unless the review's fixes need one: then say why in the Log.

## Out of scope

- The hook command's real body, the state files, the sources, the plugin files and the install code (the build
  cards).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Prompt text is never logged.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
