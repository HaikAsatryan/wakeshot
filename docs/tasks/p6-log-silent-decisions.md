# Log lines for the silent decisions the logs audit found

Phase: 6. Status: ready (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: logs good enough that a user's report
lets us find any problem; no questions overnight).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p6-logs-diagnostics` (done, 880268e). Parallel with: `p6-winget-upgrade-restart-fix`,
`p8-polish-core-engine`, `p8-polish-windows-power` (they own other files).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-log-silent-decisions\target`; delete it when done.

## Goal

`docs/tasks/p6-logs-diagnostics.md` "Found outside scope" items 1 to 6 and 8 list decisions the watch and source code
make without a log line. Add one clear line for each, so a diagnostics file shows why an agent did not hold. Also take
the token-shaped literals out of the diagnostics privacy test.

## Read first

- `docs/tasks/p6-logs-diagnostics.md`: Result and "Found outside scope" (the file and line of each item).
- `docs/architecture.md` section 8 (log rules, the four writers), `docs/product.md` "No guessing" and the Claude Code
  rules on format problems.

## Deliverables

1. A log line for items 1 to 6 and 8, at the place the decision is made, with the reason and a short id (never a
   prompt, a title, a token or a full path with the user name). Once per session, file or state change, never once per
   line or tick, so a quiet tray stays quiet and the log caps are never the thing that saves it.
2. Log lines only: no change to what holds, when, or what the menu and notices show. If an item looks like a real
   product gap (for example item 5: a renamed Codex event means "never holds"), write it under "Questions for Haik"
   with your default; do not change behavior.
3. Item 7 (power safety paths) is out of scope tonight: it waits for the power safety review.
4. `crates/wakeshot-windows/src/diagnostics.rs` tests: build the planted token strings at run time (for example by
   joining parts), so no token-shaped literal stays in the source. The test must still prove the same thing.
5. Tests: each new line is checked by a test that fails without it. Whole-workspace checks on x64 and ARM64
   (`CLAUDE.md`, Checks) before you finish.

## Files

Owns: `crates/wakeshot-windows/src/watch/` (all), `crates/wakeshot-windows/src/sqlite.rs`,
`crates/wakeshot-windows/src/app/io_thread.rs`, `crates/wakeshot-windows/src/diagnostics.rs`,
`crates/wakeshot-core/src/sources/` (all), additions to `docs/architecture.md` section 8 if a line kind is new, this
card. Other cards own `app/` (the rest), `hooks.rs`, `store.rs`, `power/`, `ownership/` and core's engine files right
now: a build failure in a file you do not own is reported, never fixed; wait a minute and build again.

## Rules

- Never run git, not even read-only. Never read token files. Prompt text is never logged. Never set `RUSTFLAGS`. Kill
  only processes you started, by PID. Never touch Haik's installed tray.
- Follow the Phase 8 polish rules in what you write (`docs/phases/08-launch-cleanup.md`, "Polish rules"): short "why"
  comments only, no card codes or names in code.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-07 04:52: card written by the orchestrator (session wakeshot-4e).

## Result
