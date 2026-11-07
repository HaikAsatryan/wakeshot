# A silent winget upgrade starts Wakeshot again

Phase: 6. Status: ready once `p6-logs-diagnostics` is done (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: a
fix only if small and clean; orchestrator decision, for his review: build it, about 70 lines of code and 60 of tests).
Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p6-winget-upgrade-restart` (the research, done), `p6-logs-diagnostics` (it also edits the tray's start).
Runs alone among Rust cards if it changes a contract (a file format in `docs/architecture.md`); otherwise beside cards
that own other files.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-winget-upgrade-restart-fix\target`; delete it when done.
Owns (2026-11-07 04:43): `crates/wakeshot-windows/src/hooks.rs`, `store.rs`, `app/` except `app/io_thread.rs`, the tray's Quit code wherever it lives in `wakeshot-windows`, `crates/wakeshot-tray/src/velo.rs` if needed, `packaging/e2e.ps1`, the README winget limit line, hook lines in `docs/architecture.md`. Beside it run `p6-log-silent-decisions`, `p8-polish-core-engine` and `p8-polish-windows-power`: a build failure in a file it does not own is reported, never fixed. If the fix needs a contract change (a type in `wakeshot-core` other cards use, or a file format other code reads), stop and reply to the orchestrator before making it.

## Goal

Build the fix that `docs/tasks/p6-winget-upgrade-restart.md` Result picked: the install hook starts the tray again
after Setup over an install, when the tray ran before (a `state.json` naming a process that is gone, and no Quit
mark) and the hook is not elevated. Then update the README limit line.

## Read first

- `docs/tasks/p6-winget-upgrade-restart.md` Result (whole), `docs/tasks/p6-hooks.md` Result, `CLAUDE.md`.
- `crates/wakeshot-windows/src/hooks.rs`, the store code it names, the tray's Quit and start code,
  `packaging/e2e.ps1`, `docs/architecture.md` (hooks and file formats).

## Deliverables

1. The Quit mark (written on menu Quit, removed at start) and the hook's restart, as the research Result describes.
   A started tray is detached, inherits no handles, runs from `current`, at the hook's integrity, never elevated.
2. Tests: each rule fails with the old behavior (no restart after a Quit; no restart when elevated; restart when the
   named tray is gone and no mark; no restart on a first install with no `state.json`).
3. `packaging/e2e.ps1`: one step that runs `Setup.exe --silent` over a running install and checks the tray runs
   again at Medium; and one after a Quit that checks it does not.
4. README: the winget upgrade limit line says what is true now (Haik approved the README; change only that line and
   any line that repeats the limit). `docs/architecture.md` if a file format or hook rule changed.
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks).

## Rules

- Never run git, not even read-only. Never read token files. Never set `RUSTFLAGS`. Kill only processes you started,
  by PID. Never touch Haik's installed tray; `e2e.ps1` refuses while it runs, so run the e2e step only if the card's
  rules allow it without quitting his tray, else leave it written and untested and say so.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None tonight: decide and log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07 03:54: card written by the orchestrator (session wakeshot-4e) from the research Result.

## Result
