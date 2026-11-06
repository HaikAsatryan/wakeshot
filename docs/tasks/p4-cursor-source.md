# Cursor source: the hook command and the state files it writes

Phase: 4. Status: ready once `p4-contracts-r3` is done (new 2026-11-06, Haik: Cursor in v1). Needs Haik: only if
step 1 finds the editor puts the payload on the command line (design Question 4).
Agent: `wakeshot-worker`. Size: about 210K tokens. If the card runs past about 230K, stop after a finished part with
its tests, and list the rest in the Result.
Runs after: `p4-contracts-r3`, the hooks security review verify (`docs/reviews/p4-hooks-design-security-verify.md`).
Parallel with: `p4-agent-hooks-install` (owns `agent_hooks.rs`, `hooks.rs`, `app/startup.rs`, `text.rs`), other
source and live cards that own different files; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cursor-source\target`; downloads in their own empty folder under
`<scratchpad>\p4-cursor-source\`, as data only. Delete the target folder when done.
Real laptop: Cursor may be installed by `p4-live-harness`; you do not run it or add any plugin (the live card does).
No power settings, no screen.

## Goal

The shared hooked-source code, Cursor's source and the `wakeshot.exe hook` command, as `docs/agent-hooks.md`
designs them after the security review, so a Cursor turn holds the PC while it runs and nothing of the prompt is
kept.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6, 7 and 12.
- `docs/agent-hooks.md`, all of it (it is the spec), and its "Live test" list.
- `docs/reviews/p4-hooks-design-security.md` and `docs/reviews/p4-hooks-design-security-verify.md`.
- `docs/tasks/p4-hooks-design.md` Result ("Build cards" row for this card, "Review fixes").
- `docs/tasks/p4-contracts-r3.md` Result (what the contracts give you; its shapes win).

## Deliverables

1. **First, before any code: the editor's payload form** (review major 3, design live test 0). Read the Cursor
   editor's hook spawn code (the current Windows build, as data) and write down how the payload reaches the hook:
   stdin, a temp file, or the command line. The editor and the CLI load one plugin folder, and the transport is a
   runtime option in Cursor's code (verify review), so follow the design's privacy Gate as it reads after the verify:
   if the editor can use the command line, Wakeshot adds no Cursor plugin at all. Then write it under "Questions for
   Haik" (design Question 4), tell the orchestrator, and go on with the shared code only.
2. `crates/wakeshot-core/src/sources/hooked.rs`: the state file format, the id rule, the event rules, the parent walk
   over a process list, as the design says.
3. `crates/wakeshot-core/src/sources/cursor.rs`: Cursor's event table, the `AgentSource`, fixed labels.
4. `crates/wakeshot-cli/src/hook.rs`: the command (DLL search and panic hook first, the Medium hand-off when
   elevated, the prune rule), always `{}` and exit 0.
5. Tests and input fixtures as the design's build row lists: a marker-word privacy test (the word in every input
   field never reaches a state file, a log line or stdout), `{}` and exit 0 for every input and for a panic, a
   crafted chat id (traversal, reserved names, long names), a junction test under an elevated token (and the same
   at Medium; `CLAUDE.md` elevation note: run the deny case unelevated or it passes for the wrong reason).
6. Lines in `docs/agent-hooks.md` ("Built" notes) and `docs/architecture.md` section 7 (re-read right before each
   edit; other cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli --locked`. Record each result. A failure in another card's file is reported, never fixed. The
  orchestrator runs the whole-workspace checks.
- Break two rules on purpose (the privacy rule and the exit code rule) and show the tests catch them.

## Out of scope

- Writing or removing plugin folders, the uninstall step, the notice (`p4-agent-hooks-install`).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` and any live run (`p4-live-cursor`).
- Antigravity's table (`p4-antigravity-source`, after this card).
- Contracts, module lists, `Cargo` files: stop and ask the orchestrator if you need a change.

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
