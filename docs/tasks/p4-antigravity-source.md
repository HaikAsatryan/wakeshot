# Antigravity source: its event table and plugin folder

Phase: 4. Status: ready once `p4-cursor-source` and `p4-agent-hooks-install` are done (new 2026-11-06, Haik:
Antigravity in v1 if a free test exists; `p4-antigravity-research` says it does). Needs Haik: only if step 1 finds
the payload on the command line.
Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p4-cursor-source`, `p4-agent-hooks-install`. Parallel with: cards that own different files; at most four
Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-antigravity-source\target`; downloads in their own empty folder,
as data only. Delete the target folder when done.
Real laptop: Antigravity may be installed by `p4-live-harness`; you do not run it or add its plugin (the live card
does). No power settings, no screen.

## Goal

Antigravity (CLI and IDE) on the shared hooked-source code: its event table in `sources/antigravity.rs`, its plugin
folder in the install table, as `docs/agent-hooks.md` "Antigravity" designs them.

## Read first

- `docs/agent-hooks.md` (all; "Antigravity" is your part), `docs/research/antigravity-signals.md`.
- `docs/tasks/p4-cursor-source.md` and `docs/tasks/p4-agent-hooks-install.md` Results: the shared code and the one
  entry point each left for you.
- `CLAUDE.md` (Checks, cargo path, elevation note).

## Deliverables

1. **First: the payload form** of Antigravity's hooks (stdin, temp file or command line), read in its current
   Windows build as data (design "Privacy gate"). If it is the command line, stop, write it under "Questions for
   Haik" with the default "does not ship", and tell the orchestrator.
2. `crates/wakeshot-core/src/sources/antigravity.rs`: the event table (`PreInvocation`, `Stop`, `fullyIdle`, the
   `state` field), the `AgentSource`, fixed labels, on `sources/hooked.rs`.
3. Antigravity's plugin files in the install table (`crates/wakeshot-windows/src/agent_hooks.rs`, through the entry
   point `p4-agent-hooks-install` left), and the `hook` command's agent name if `hook.rs` needs one line for it.
4. Tests in the shape of Cursor's: event rules, the privacy marker test, the fixed file set, the liveness pairing
   with `agy.exe`.
5. Wiring for both hooked sources (orchestrator, 2026-11-06 ~18:50, so the two live cards can run in parallel):
   start the Cursor and Antigravity sources in `crates/wakeshot-windows/src/app/tray_mode.rs` (as `p4-contracts-r3`
   did for Copilot, VS Code and OpenCode, each with a unit test), and add `cursor` and `antigravity` rows to
   `SOURCES` in `crates/wakeshot-windows/examples/agent-watch.rs`. This wins over "Out of scope" below.

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli --locked`. Record each result. The orchestrator runs the whole-workspace checks.

## Out of scope

- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` and any live run
  (`p4-live-antigravity`). Contracts, module lists, `Cargo` files.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Prompt text is never logged or stored. Downloads are data, never run.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read, with the version; mark guesses. Plain short English, no em dashes, no
  emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator from the hooks design.

## Result
