# Agent hooks install: add, keep and remove Wakeshot's plugin folders

Phase: 4. Status: ready once `p4-contracts-r3` is done (new 2026-11-06). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 180K tokens. If the card runs past about 210K, stop after a finished part with
its tests, and list the rest in the Result.
Runs after: `p4-contracts-r3`, the hooks security review verify (`docs/reviews/p4-hooks-design-security-verify.md`).
Parallel with: `p4-cursor-source` (owns `sources/hooked.rs`, `sources/cursor.rs`, `wakeshot-cli/src/hook.rs`), other
source and live cards that own different files; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-agent-hooks-install\target`; delete it when done.
Real laptop: tests write only under temp folders. Never write into the real `%USERPROFILE%\.cursor` or `.gemini`
folders (the live card does that). No power settings, no screen.

## Goal

The install side of `docs/agent-hooks.md`: when the tray finds Cursor (and later Antigravity), it adds Wakeshot's own
plugin folder once, says so once, never fights the user, and the uninstall removes exactly what Wakeshot wrote.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6 and 12, the uninstall and
  hooks parts.
- `docs/agent-hooks.md`, all of it (it is the spec): "Adding", "Removing", "Failure cases", the risk list.
- `docs/reviews/p4-hooks-design-security.md` and `docs/reviews/p4-hooks-design-security-verify.md`.
- `docs/tasks/p4-hooks-design.md` Result ("Build cards" row for this card, "Review fixes").
- `docs/tasks/p4-contracts-r3.md` Result; `crates/wakeshot-windows/src/hooks.rs` and `docs/tasks/p6-hooks.md`.

## Deliverables

1. `crates/wakeshot-windows/src/agent_hooks.rs`: the fixed file set and marker keys, the add, rewrite and remove
   table, reparse point checks, the staging folder with fail-if-exists and its cleanup, the `.wakeshot-removed`
   rename, the record (`agent-hooks.json`), the Medium-only rule for the tray, `hook <agent> remove`, as the design
   says. Cursor's plugin files now; leave one table entry point so `p4-antigravity-source` adds its own.
2. The uninstall step and its fake in `crates/wakeshot-windows/src/hooks.rs` (with the Medium copy when elevated).
3. The call in `crates/wakeshot-windows/src/app/startup.rs` that posts `Event::HooksAdded`; the notice words in
   `text.rs` if `p4-contracts-r3` left them to you.
4. Tests on temp folders: a fresh add, a second start adds nothing, a folder the user changed is never touched, a
   folder the user removed is never added back, a junction or link in any path refuses the write, a crafted record
   or marker deletes nothing outside the fixed file set, uninstall removes exactly the fixed set, an elevated run
   writes nothing in user folders (run the deny case under a Medium token: `CLAUDE.md` elevation note).
5. Rows for `agents\` and `agent-hooks.json` in `docs/architecture.md` section 6, and "Built" notes in
   `docs/agent-hooks.md` (re-read right before each edit; other cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-tray --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-tray --locked`. Record each result. A failure in another card's file is reported, never fixed. The
  orchestrator runs the whole-workspace checks.
- Break two rules on purpose (the link check and the fixed file set) and show the tests catch them.

## Out of scope

- The hook command, the state files and the sources (`p4-cursor-source`).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs`, any real plugin folder, any live run
  (`p4-live-cursor`).
- Contracts, module lists, `Cargo` files: stop and ask the orchestrator if you need a change.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Never write outside temp folders in tests.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator from the hooks design.

## Result
