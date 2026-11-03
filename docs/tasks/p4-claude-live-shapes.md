# Claude Code live shapes: raw transcript fixtures and the `--resume` test

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: the contracts card. Parallel with: every code card of every track. Never at the same time as
`p4-live-claude`, `p4-live-codex` or `p4-live-copilot` (they all run agent sessions on the laptop).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-claude-live-shapes\target` (only if a spike is rebuilt).
Uses the real laptop: yes, Claude Code test sessions (Haiku, a few cents of Haik's usage), in throwaway folders in
the scratchpad. No power settings, screen or sound. Lane: agent-session lane, one at a time with the other p4 live
cards; may run beside the global lane (power, screen, sound, installs) (review m19).

## Goal

Two things the transcript parser needs before it is written: raw JSONL lines (redacted) for every transcript case,
since the Phase 0 log kept only summaries; and the rule for `claude --resume` in a new process
(`architecture.md` section 7: "Rule 5 does not cover `claude --resume` in a new process. Phase 4 runs one live test").

## Read first

- `CLAUDE.md` "Facts to keep in mind": never read `.key` files.
- `docs/research/claude-code-signals.md`: "Timers", "Transcript lines that matter", "Live test results", "Other facts
  seen live" (driving tests under ConPTY: kitty Enter, drop the `CLAUDE*` variables).
- `docs/tasks/p0-claude-live-tests.md` (how Phase 0 ran them), `spikes/claude-watch/ccdrive.py`,
  `spikes/claude-watch/fixtures/README.md` (redaction rules).
- `docs/research.md` "Sources track plan checks" (resume: missed runs do not fire; a self-paced loop is not restored).

## Deliverables

1. Live sessions driven with `ccdrive.py` (Haiku), one throwaway folder each, covering: background shell and its
   notification; shell Monitor; `/loop 1m` (`CronCreate`, fires, `CronDelete`); dynamic `/loop` (`ScheduleWakeup`,
   then `stop: true`); a one-shot reminder; `/clear` during a loop (`session_cron_carry`); a background subagent; the
   SDK path (stream-json) with a background shell.
2. The `--resume` test: a session with `/loop 5m` and a `ScheduleWakeup`, clean exit, then `claude --resume <id>` in a
   new process. Record: the new status file (pid, `sessionId` same or new), which transcript file grows (same file or
   a new one), any line that marks the resume or lists restored jobs, whether the jobs fire, and whether the old
   `CronCreate` lines would wrongly count as armed in a replay from the start. Repeat with a hard kill
   (`Stop-Process` on the test session's pid, which this card started) instead of a clean exit.
3. `crates/wakeshot-core/tests/fixtures/claude/live/`: one `.jsonl` per case with the raw lines that matter (the
   kinds in "Transcript lines that matter", plus the resume lines), and the matching status file writes; a
   `README.md` naming the case, the Claude Code version, and the redaction. Redact by script that prints only redacted
   text: user name, session ids (fake UUIDs, consistent per file), bridge ids, pipe names, every string that can hold
   user text as `<str N>`. Keep ids that the parser keys on (`toolu_`, task ids, job ids), times, cron strings, enum
   values.
4. A new section "Phase 4 live shapes (2026-11-..)" at the end of `docs/research/claude-code-signals.md`: what was run,
   the resume findings, and the rule change for rule 5 (exact text), each marked verified or not. Short lines in
   `docs/research.md` pointing to it.
5. Clean-up: end every test session this card started (by pid), delete their `~/.claude/projects/<test folder>`
   transcripts and the scratch folders. Never touch Haik's own sessions or their files.

## Acceptance checks

- Every case in deliverable 1 has a fixture file, or the Result says why not.
- The resume test answers each question in deliverable 2.
- No test session process is left (list `claude.exe` processes started by this card's driver, by pid).
- The fixtures pass the secret scan patterns (`docs/phases/01-design-skeleton.md`, "Secret scan"), and a grep for
  the real Windows user name and for `.key` finds nothing.

## Out of scope

- Writing the parser (`p4-claude-transcript`). The VS Code UI and the Desktop app (`p4-live-claude`, optional).
  Changing any Claude setting. Any file not listed above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
