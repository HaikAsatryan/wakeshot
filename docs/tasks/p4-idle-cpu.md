# Idle CPU back near 0.1.0's

Phase: 4. Status: running since 01:59 (new 2026-11-07, orchestrator decision for Haik's review). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p6-agents-video-readme` Part B. Runs alone among Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-idle-cpu\target`; delete it when done.
Real laptop: a `--dry-run` tray for the measurement; at the end Haik's install updated (as `p4-cursor-off` did it).

## Goal

The tray at rest now uses about 1,016 ms of CPU in 10 minutes; 0.1.0 used 109 ms (`p6-agents-video-readme` Result,
`docs/research.md` "Speed numbers"). Likely causes, inferred, not proven: every 30 s tick asks Restart Manager again
about dead Copilot lock files (14 to 22 ms each), and OpenCode's 2 s check costs about 250 ms per 10 minutes; both in
`crates/wakeshot-windows/src/watch/agents.rs`. Find the real costs, then cut them without losing an exact signal
("No guessing", `docs/product.md`): every hold must still start and end as the live cards measured.

## Deliverables

1. Measure first: where the idle CPU goes (a profile or counters per check), with the method of "Speed numbers".
2. Fixes, each with a test that fails before, for example: do not ask Restart Manager again about a lock file whose
   owner was already found gone until the file changes; run OpenCode's 2 s check only while an OpenCode process
   lives. Any other cost you find. No timer that ends or keeps a hold.
3. Memory while holding rose from 4.0 to 7.7 MB: a short look; fix only if the cause is clear and small.
4. Measure again the same way: idle CPU and memory before and after, in the Result and in `docs/research.md`
   "Speed numbers".
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks), then pack x64 and update Haik's install the way
   `p4-cursor-off` did (WM_CLOSE only, `Setup.exe --silent` at Medium through `explorer.exe`, hash, `config.json`
   kept, tray at Medium), and one Claude Code, one Codex and one OpenCode turn still hold and let go.

## Out of scope

- `README.md`, `site/`, `media/` (the video card writes the end card numbers after this card).

## Rules

- Never run git, not even read-only. Never kill Haik's tray: only WM_CLOSE. Start programs at Medium through
  `explorer.exe`. Kill only processes you started, by PID. Never read token files. Prompt text is never logged. Never
  set `RUSTFLAGS`; cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64 needs the Build Tools
  `VC\Tools\Llvm\x64\bin` folder on that cargo command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-07 01:59: card written by the orchestrator.

## Result
