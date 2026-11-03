# Claude Code live shapes: raw transcript fixtures and the `--resume` test

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
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

None.

## Design choices

- Two files per case, `NN-name.transcript.jsonl` and `NN-name.status.jsonl`, not one mixed file. Reason: the
  transcript parser and the status file parser take different inputs, and a transcript fixture should look like a
  real transcript. Change it when the parser card wants one replay file with both streams.
- Only the lines that matter are kept (plus the metadata lines in the resume cases), not whole transcripts. Reason:
  the whole files are 350 to 700 KB of tool and prompt listings that hold nothing the parser needs. Change it when
  the parser needs a test that skips every line kind of a real file: then add one full redacted transcript.
- The poller and the redaction script lived in the scratch folder and were deleted. Reason: the card owns only the
  fixture folder. Change it when the fixtures must be made again for a new Claude Code version: then add both to
  `spikes/claude-watch/`.
- `bridgeSessionId` placeholder `BRIDGE1` (7 characters). Reason: the secret scan pattern for that key matches any
  value of 8 or more characters.

## Found outside scope

- `spikes/claude-watch/fixtures/live-tests.log` holds `"bridgeSessionId":"session_BRIDGE01"` (a placeholder, 16
  characters). It matches the secret scan pattern `"(accessToken|refreshToken|bridgeSessionId)"...{8,}`, so every
  scan flags it unless it is allowed.
- `docs/research/claude-code-signals.md`, older parts: "Short answer" (third bullet) and "Claude Code itself uses two
  outside methods" (item 1) say Claude Code rebuilds pending jobs on `--resume`: live, true for `CronCreate` jobs,
  not for `ScheduleWakeup` wakeups. The "Wakeup armed" row says "resume re-arms from `scheduledFor`": not seen live
  for a resume in a new process. Rule 3 closes a task only on `completed`, `failed` or `killed`; live also gave
  `stopped`. The new section names all of these; the old text is unchanged.
- The "Background work" table says task output goes to `%LOCALAPPDATA%\Temp\claude\<folder>\<sessionId>\tasks\`.
  With 2.1.317 and the long test folder paths, `<folder>` was a 16 hex character name, not the readable one. Why
  (path length or a new release): not verified. v1 does not read task files.
- The laptop updated itself from Claude Code 2.1.316 to 2.1.317. The byte offsets and minified names in the signals
  doc are for 2.1.316.
- Side effects on Haik's setup, all written by Claude Code itself: the 13 test folders are now marked trusted in
  `~/.claude.json` (the folders are deleted, so the entries point nowhere); the test sessions showed in the Remote
  Control session list while they ran. Phase 0's three `...scratchpad-cctest*` folders are still under
  `%LOCALAPPDATA%\Temp\claude` (not this card's).
- After a hard kill, a background shell's `pwsh` keeps running with no parent. Claude Code does not end it.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03: worker started; reading the signals doc, the Phase 0 card, `ccdrive.py` and the redaction rules.
- 2026-11-03 18:16: Claude Code is now 2.1.317 (Phase 0 had 2.1.316). Scratch venv with pywinpty 3.1.2 and pyte.
  Plan: one folder per case under `<scratch>\p4-claude-live-shapes\runs`, Haiku, `--permission-mode auto` (as Phase
  0); a small Python poller logs raw status file writes for this card's pids only, plus lock files and transcript
  sizes of the test folders. Fixtures come from the test transcripts by a redaction script.
- 2026-11-03 18:18 to 18:24: ran bgshell, monitor, loop1m, dynloop, reminder, clear, subagent, sdk. Notes: the trust
  dialog now defaults to "No, exit" (send Down first). Haiku made the "remind me in 2 minutes" timer a background
  shell; a second prompt got a `CronCreate` one-shot (`recurring: false`). The PowerShell tool blocks a standalone
  `Start-Sleep 40` in a subagent, so the subagent task writes ten files instead. Monitor commands run through Bash:
  two failed before one worked. `/exit` with armed jobs opens a "Background work is running" dialog (`waiting`):
  "Exit and stop tasks" is the clean exit.
- 2026-11-03 18:23: `*/5` job created 18:23:26.767 got `nextFireAt` 18:28:11.763 (creation plus 285 s), not the next
  grid time 18:25.
- 2026-11-03 18:24: resclean (pid 29144) exited clean; reskill (pid 27980) hard killed. The dead pid's status file
  went away in the same ms as 29144's clean exit. Resumed both at 18:24:55 (pids 8784, 11540): same `sessionId`,
  same transcript file grows, lock file taken by the new pid at start. No fire at 18:28 (wakeup) or 18:28:11 (cron)
  in either resumed session by 18:28:33.
- 2026-11-03 18:29: the restored `*/5` jobs fired at 18:29:41 and 18:29:43: resume start plus 285 s, not the old
  `nextFireAt`. The `ScheduleWakeup` (18:28:00) never fired and CronList did not list it. CronList showed the cron
  job with next run 18:34. Same for the clean exit and the hard kill. Pid 8784 was reused: first the reminder
  session, then the resumed resclean session.
- 2026-11-03 18:31: third resume test `resone` (clean exit, pid 36584, resumed as pid 31376): a one-shot `CronCreate`
  (18:34) and a daily job (`0 9 * * *`, next 09:30 with jitter). After resume CronList shows both with the same next
  runs as before.
- 2026-11-03 18:34: resone's restored one-shot fired at 18:34; resclean's and reskill's restored jobs fired again at
  18:34:26 and 18:34:28 (5 min after the first restored fire). All three resumed sessions closed with /exit and
  "Exit and stop tasks". Poller stopped by pid. No process this card started is alive (58 pids checked).
- 2026-11-03 18:35: draft redaction run in the scratch folder. New shapes seen: Monitor event notifications carry
  `<event>` and no `<status>`; a subagent's notification that arrives while the main thread is busy comes as an
  `attachment` `queued_command` (its `prompt` starts with `<task-notification>`), plus `queue-operation` `remove`.
- 2026-11-03 18:36 to 18:40: two more SDK cases for the resume rule. sdkres (pid 32448): stdin closed while a
  150 s background shell ran; the process stayed alive until the shell ended, ran the notification turn, then exited.
  sdkkill (pid 30812): hard kill while a 300 s shell ran; the shell's pwsh (16316) survived as an orphan and was
  killed by pid. Resumed over stream-json (pid 31708): at start, before any prompt, it wrote a user line
  `<task-notification>` for the old task with `<status>stopped</status>` and `queueTranscriptOnly: true`. The dead
  pid's status file went away at the resumed process's start.
- 2026-11-03 18:45: all sessions and both poller runs ended (76 pids checked, none alive). Fixtures written to
  `crates/wakeshot-core/tests/fixtures/claude/live/` (13 cases, 27 files plus `README.md`); secret scan patterns and
  the user name, `.key`, host and scratch path greps find nothing. Section "Phase 4 live shapes (2026-11-03)" added
  to `docs/research/claude-code-signals.md`.

- 2026-11-03 18:50: clean-up: deleted the 13 test project folders in `~/.claude/projects`, their 13 task folders in
  `%LOCALAPPDATA%\Temp\claude` (each held one of this card's session ids), one `~/.claude/session-env/<id>` folder,
  and the scratch folder. No spike was rebuilt, so no build folder.

## Result

Done.

What was done:

- 13 live cases with Claude Code 2.1.317 and Haiku, 17 processes with the resumed ones: the 8 of deliverable 1, the
  resume test after a clean exit and after a hard kill, a third resume with a one-shot and a daily job, and two SDK
  cases (stdin closed during a shell; hard kill during a shell, then resume).
- Resume answers: new pid, same `sessionId`, same transcript grows, no line marks the resume. `CronCreate` jobs come
  back and fire (a `*/5` job at resume plus 285 s, not its old `nextFireAt`); `ScheduleWakeup` wakeups do not. The
  new process takes the lock file at start. A replay from the start counts the old jobs right but holds the dead
  wakeup until `scheduledFor` plus the margin. Same for clean exit and hard kill. The SDK resume after a hard kill
  writes a `stopped` notification for the open background task.
- Rule changes with exact text (rule 5 resume, rule 3 `stopped` and `queued_command`) in the new section.

Files changed:

- New: `crates/wakeshot-core/tests/fixtures/claude/live/` (`README.md`, 13 `*.status.jsonl`, 14
  `*.transcript.jsonl`).
- `docs/research/claude-code-signals.md`: new section "Phase 4 live shapes (2026-11-03)" at the end.
- `docs/research.md`: four bullets under "How to know Claude Code is working".
- This card.

Acceptance checks:

- Every case of deliverable 1 has a fixture (`01` to `08`): pass.
- The resume test answers each question of deliverable 2 (table "Resume findings"): pass.
- No test session process left: pass. 76 pids from the driver log and the state folders checked, none alive; no
  status file left for a test pid.
- Fixtures: both secret scan pattern sets from `docs/phases/01-design-skeleton.md` find nothing; grep for `haika`
  and `.key` finds nothing; no UUID except the `5e55...` fakes; every `.jsonl` line parses. `cargo metadata` shows no
  new target in `wakeshot-core`. No Rust code changed, so no fmt, clippy or test run.

Open items:

- Apply the rule changes to "Recommended detection rules" (or let `p4-claude-transcript` take them from the new
  section), and fix the older text listed under "Found outside scope".
- Not run: `claude --continue`, `/resume` inside a running session, a resume after a job's time passed while no
  process ran, a terminal resume with a background shell left open, the VS Code extension's own resume.
