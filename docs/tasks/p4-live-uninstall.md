# Remove everything installed for the live tests

Phase: 4. Status: later (new 2026-11-06 ~21:20, Haik). Runs last: only when the whole bigger v1 is finalized
(every live card, the fresh install over Haik's, the end-to-end check per agent, the README and video final).
Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 50K tokens.
Runs after: every other card of the bigger v1. Parallel with: nothing that runs an agent.
Real laptop: uninstalls for Haik's user. No power settings.

## Goal

Haik, 2026-11-06 ~21:20: "Whenever you test everything and make sure that everything is working delete everything
(cursor, antigravity, codex, olama, opencode and anything else that you installed for this job). But only if you
fully finalize the work. It's part of cleanup."

## Deliverables

1. From `docs/tasks/p4-live-harness.md` Result (the install list and remove commands) and every live card's Log,
   the full list of what agents installed or created for this job. Remove each one the way its maker documents:
   Copilot CLI, OpenCode 2.x and Codex CLI (npm globals), Ollama with its model and its `Startup\Ollama.lnk`, Cursor
   and its CLI, Antigravity CLI (and its user PATH entry) and IDE, the driver's and harness's scratch folders, test
   folders such as `target\vscode-live`, any WSL change.
2. Keep what Haik installed himself: the Codex desktop app and its login (`~\.codex`), VS Code and its Copilot
   sign-in, `gh`. Keep the shared data folders they use (`~\.codex`, `~\.copilot`, `%APPDATA%\Code`). Wakeshot's own
   plugin folders are removed by Wakeshot's own remove path, never by hand.
3. For each removed app's own data folder (for example `~\.cursor`, `~\.gemini`, OpenCode's data and state folders,
   `~\.ollama`): remove it only if this job created it (check the creation time against the install time in the
   harness Log); else list it for Haik.
4. After: each removed command is gone from PATH at Medium; the Startup folder holds no entry this job added; Haik's
   installed Wakeshot still runs.
5. Result: what was removed, what was kept and why, anything left for Haik.

## Rules

- Never delete a file or folder this job did not create; when unsure, list it for Haik instead.
- Never read token files. Never run git. Kill only processes you started, by PID; ask the orchestrator before
  ending an app process you did not start (for example an IDE Haik opened).
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Log

- 2026-11-06: card written by the orchestrator from Haik's words.

## Result
