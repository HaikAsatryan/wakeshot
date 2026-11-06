# VS Code chat source (Copilot agent mode in VS Code)

Phase: 4. Status: ready once `p4-contracts-r2` is done (new 2026-11-06, Haik: Copilot agent mode in VS Code ships in
v1). Needs Haik: no (a live card follows).
Agent: `wakeshot-worker`. Size: about 130K tokens.
Runs after: `p4-contracts-r2`, `p4-copilot-vscode-research` (done). Parallel with: the other Round 2 source cards
(`p4-opencode-source` Part B, `p4-cursor-source`, `p4-antigravity-source`), docs cards; at most four Rust cards at
once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-vscode-source\target`; delete it when done.
Real laptop: you may list VS Code's folder names and read your own test copies; never open `state.vscdb` or any file
rule 9 names. No power settings, no screen.

## Goal

`VsCodeSource`, an `AgentSource` like Claude's, Codex's and Copilot's: working, waiting and idle for each chat in VS
Code's chat view (Copilot agent mode and other chat participants), from the chat session files, built from
`docs/research/copilot-vscode-signals.md`.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6 and 7.
- `docs/research/copilot-vscode-signals.md`, all of it (rules 1 to 9, "For the source card", the fixture sketch,
  "Open questions only a live test answers").
- `docs/tasks/p4-contracts-r2.md` Result: the read mode for whole files of recently changed files and
  `Liveness::ImageSince`. Its shapes win over the research file's wording.
- One existing source as the house model: `crates/wakeshot-core/src/sources/copilot.rs` and its tests.

## Deliverables

1. `crates/wakeshot-core/src/sources/vscode.rs` (the stub `p4-contracts-r2` made): `VsCodeSource` per rules 1 to 5.
   `wanted()` uses the new read mode on both chat folders under `%APPDATA%\Code\User` (honor `VSCODE_APPDATA` from
   the tray's own environment only, like `CLAUDE_CONFIG_DIR`; Insiders and portable installs: say what you did and
   why). Parser: bytes of one `.jsonl` in, one `SessionFacts` out, replaying `kind` 0 to 3 on `requests`.
2. **Liveness**: rule 7 through `Liveness::ImageSince { images: &["Code.exe"], since_wall_ms: <request timestamp> }`
   unless the contracts Result says otherwise, so a crash leftover stops counting once VS Code restarts. Rule 6's
   `AnyProcess` fallback holds while any `Code.exe` runs and `state.vscdb` always exists, so do not use it alone. Your
   pick, with the reason, is a decision for Haik's review.
3. `label`: "Copilot in VS Code" (the research's name for nearly every user of that view; "VS Code chat" is the other
   option, for Haik's review).
4. Tests in `crates/wakeshot-core/tests/vscode_source.rs` with fixtures in `crates/wakeshot-core/tests/fixtures/
   vscode/`, written from the schema (never copied from a real chat file): working, waiting, idle, working again
   after waiting, a compacted file (one `kind: 0` line), a partial last line (`RetryOnce` then `Unreadable`), an
   unknown `kind` and an unknown `modelState.value` (`UnknownValue`), several requests, `Gone`, an empty window
   session, and that no prompt text reaches any `SessionFacts` field or log text.
5. A "Built" note in `docs/research/copilot-vscode-signals.md` and lines in `docs/architecture.md` section 7 (re-read
   right before each edit; other source cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows --all-targets --locked -- -D
  warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows --locked`.
  Record each result. A failure in another card's file is reported, never fixed. The orchestrator runs the
  whole-workspace checks.
- Each rule in the research table that you built has a test that fails when the rule is broken (break two on
  purpose and say which).

## Out of scope

- Contracts, module lists and `Cargo` files (stop and ask the orchestrator if you need a change).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` or the harness (the live card).
- The Copilot CLI source's `copilot-runtime.exe` and `Code.exe` owners (rule 8: `p4-live-copilot`).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files, `state.vscdb`, `agent-host.db` or `session.db`. Prompt text is never logged or stored.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
