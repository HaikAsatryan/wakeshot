# Logs that clean themselves and a bug report users can send

Phase: 6. Status: ready (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: follow the latest best practices; no
questions to him).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p4-idle-cpu`. Runs alone among Rust cards if it changes a contract (a menu item or CLI command is in
`architecture.md` section 2); otherwise beside non-Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-logs-diagnostics\target`; delete it when done.

## Goal

Haik, 2026-11-07: "do we autocleanup log at some point or not? Do we have proper logs at appropriate way that real
users can submit logs for any bugs and we confidently find any problems? If any answer is no then seperate card for
that to to follow all the latest best practices."

Today (checked 2026-11-07): the log rolls only when the tray starts (`startup.rs` calls `LogWriter::roll`): over
1 MB it keeps three files. A tray that runs for weeks never rolls, so the file grows until the next sign-in. Users
send logs by hand: "Open log" or `wakeshot log`, then copy lines into the bug form. The install check found no log
line per Claude Code session (its holds were read from `state.json`).

## Deliverables

1. Research first, with current sources: how well-made Windows tray apps and CLIs keep logs and take bug reports
   (size and age caps, rolling while running, a diagnostics bundle, what goes in it, privacy). Short notes in
   `docs/research.md` with sources.
2. Clean-up: the log rolls by size while running too, not only at start; a cap on the total size of the logs folder;
   old files go. Every writer process (tray, guard, CLI, hook) stays safe (the four-writer rule in `architecture.md`
   section 8). Tests for each rule.
3. Enough to find a problem from the log alone: a header at each start (Wakeshot version, Windows version and build,
   CPU, Modern Standby or S3, power plan GUID, the lid and sleep values Wakeshot reads, which sources run); one line
   per hold start and end per agent and session (a short id, never a prompt, title or full path), with the reason;
   every source's format problem; every restore. Check each agent source and the power layer for silent decisions.
4. A way to send it: a menu item or CLI command (for example `wakeshot diagnostics`) that writes one zip or text
   bundle (the logs, the state and config files Wakeshot owns, the header facts) to a place the user picks or the
   Desktop, and opens Explorer on it. It holds no prompt text, no token, no other app's files, and replaces the user
   name in paths. A test proves the bundle never holds a planted prompt or token string.
5. `.github/ISSUE_TEMPLATE/bug-report.yml` and the README "Log" or "Reporting a bug" part: how to make and attach the
   bundle. `docs/architecture.md` section 8 "Log" updated.
6. A contract change (a new menu command or CLI command) only in a run alone; say so in the Log.

## Acceptance checks

- Tests for rolling while running, the folder cap, the header, the per-session lines and the bundle's privacy; each
  fails before its change.
- Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks).

## Rules

- Never run git, not even read-only. Never read token files. Prompt text is never logged. Kill only processes you
  started, by PID. Never set `RUSTFLAGS`; cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64
  needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None: Haik asked for no questions; decide by best practice and log each call "for Haik's review".

## Found outside scope

## Log

- 2026-11-07 02:43: card written by the orchestrator.

## Result
