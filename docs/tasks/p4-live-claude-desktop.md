# Live test: Claude Desktop

Phase: 4. Status: blocked (2026-11-07 04:10): the no-input part is done; the live part needs Haik's "Morning steps"
(about 8 minutes, 5 short prompts), then a worker finishes it ("Morning plan"). Haik signed in about 02:35.
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-idle-cpu` (it updates Haik's install). Parallel with: non-Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-claude-desktop\target`; delete it when done.
Real laptop: Claude Desktop 2.42304.2, installed 2026-11-07 for Haik's user at Medium from Anthropic's signed
installer (winget `Anthropic.Claude`, `%LocalAppData%\AnthropicClaude`); it updated itself to 4.23118.0 within a
minute. No power settings.

## Goal

Haik, 2026-11-07: "I did not like that we did not check claude desktop." The README lists the Claude desktop app as
"not tested live yet". Test, live, whether Wakeshot holds exactly for work done in Claude Desktop, by the "No
guessing" rule (`docs/product.md`): every start and end of work seen exactly, or that case is not supported.

## Read first

- `docs/product.md` (Claude Code rules, "No guessing"), `docs/research/claude-code-signals.md`, `docs/tasks/p4-live-claude.md`
  Result (how a Claude live test ran), README "When each hold counts" and "Not tested live yet".
- `docs/tasks/p4-live-vscode-stop.md` (how a desktop app was driven without synthetic keys or clicks).

## Deliverables

1. What Claude Desktop runs that is work: its Code sessions (Claude Code inside the app), its own scheduled tasks
   and "Keep computer awake" setting, and plain chats. For each: does it write the files the Claude Code source
   reads (`~/.claude/sessions`, transcripts), and does Wakeshot see start, end, Stop, approval wait, app close and
   crash exactly. Plain chats run in the cloud: say whether a local signal exists at all.
2. A table in the Result: case, what Wakeshot showed, exact or not.
3. Fixes with tests for what can be made exact (Claude source files only); redacted fixtures (no prompt text, no
   user paths, fake ids).
4. Docs: `docs/research/claude-code-signals.md` section, `docs/research.md` short lines; the README lines the video
   card should use (supported, or "Not supported, and why").
5. If a step needs Haik (a click, a prompt typed in the app), stop early with exact short steps and reply.

## Rules

- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply.
- Never run git, not even read-only. Never read Claude's `.key` files or any token file. Prompt text is never logged.
  No synthetic key presses or clicks in Haik's apps. Kill only processes you started, by PID. Start programs at Medium
  through `explorer.exe`. Never set `RUSTFLAGS`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Morning plan (for the orchestrator)

1. After `p6-logs-diagnostics` is installed (its tray log has one line per hold start and end per session, a pid for
   Claude), start a worker on this card. It builds `agent-watch` into this card's build folder, takes the dry-run
   lock, starts `agent-watch --source claude --raw <scratch>\p4-live-claude-desktop\raw.txt` at Medium through
   `explorer.exe` (as `p4-live-claude` did), and a loop of `powercfg /requests` every 5 s that keeps only process
   names and request types. It checks that `%TEMP%\wakeshot-desktop-test` exists and is empty (made tonight; make it
   again if gone), writes "ready HH:MM" in the Log, then waits up to 45 minutes for your SendMessage "Haik done".
2. When the Log says ready, give Haik the "Morning steps" below.
3. When Haik says done, SendMessage the worker. It stops its own processes by PID, removes the lock, and reads
   (structure only, never text): the `--raw` status writes of every `entrypoint: claude-desktop` file, the test
   transcript (glob `projects\*\<sessionId>.jsonl`; expected under
   `~/.claude/projects\C--Users-haika-AppData-Local-Temp-wakeshot-desktop-test\`), the app's
   `%LOCALAPPDATA%\Claude\logs\main.log` (`Using Claude Code binary at`, `[keep-awake] code session hold`,
   `Stopping`), and the tray log's per-session lines. Then it fills the table in Result, writes redacted fixtures
   (fake ids, every string `"x"`), adds tests if a fix is needed, and checks as the live card rules say.

Fallback if no worker can watch during the steps: the tray log's per-session lines plus the app's `main.log` give
each start, end, Stop and quit; the approval wait is then judged from the tray line's reason only (weaker).

What the worker should expect, by the code: step 2 writes `waiting` / `permission prompt` for about 30 s (no hold:
`waiting` holds only with Remote Control on), then `busy`, then `idle` at Stop. Step 3 ends its turn `idle` with one
open task (holds by rule 3) until the `<task-notification>` about 61 s later. Step 4 stays `busy` through the window
close. Step 5 ends the session process at quit (pid gone, or a clean exit). Step 6 writes nothing under
`~/.claude/sessions` or `projects`.

## Morning steps (for Haik, about 8 minutes)

Start only when the orchestrator says the watcher is ready. Each prompt is a short app test and costs very little.

1. In Claude Desktop, click Code, start a new local session, and pick the folder `%TEMP%\wakeshot-desktop-test`
   (paste it into the folder dialog's address bar; trust the folder if asked). Pick Haiku in the model menu if it is
   there, and leave the mode on Ask. (1 minute)
2. Send `App test for Wakeshot. Run this exact command: ping -n 40 127.0.0.1 . Then reply with one word.` A permission
   card appears: wait 30 seconds, then click Allow (once, not always). About 10 seconds after the command starts,
   click Stop. You see the turn stop. (1 minute)
3. Send `App test for Wakeshot. Run this exact command in the background (run_in_background true): ping -n 61
   127.0.0.1 . Then end your turn with one word.` Click Allow. Wait for a second short reply, about 70 seconds
   later. (1.5 minutes)
4. Send `App test for Wakeshot. Run this exact command: ping -n 45 127.0.0.1 . Then reply with one word.` Click
   Allow. Right after the command starts, close the Claude window with its X. Wait 60 seconds, then open Claude again
   from its tray icon or the Start menu. You see the one-word reply. (1.5 minutes)
5. Send the same prompt as in step 4 and click Allow. Right after the command starts, right-click Claude's tray icon
   (it may sit under the tray's up arrow) and click Quit. Wait 15 seconds, then start Claude from the Start menu.
   Send nothing more. (1 minute)
6. Click Chat, start a new chat and send `App test. Reply with one word.` Wait for the reply. (30 seconds)
7. Tell the orchestrator "done" and the time.

Optional, 3 more minutes: in the step 1 session send `/loop 1m App test. Reply with one word.`, wait for two
replies (about 2 minutes), then send `Stop the loop.`

## Questions for Haik

None. The calls under "Decided by the worker" in Result are for Haik's review.

## Found outside scope

1. `docs/research/claude-code-signals.md`: the old "Desktop app" section ("Not installed on this laptop"), the
   "Desktop app scheduled tasks" row of "Timers" ("schedule ... not in that file", "app not installed here") and the
   Desktop bullet of "Live tests still open" are stale. My new section says it supersedes them where they differ; a
   docs pass can shorten them to a pointer.
2. README, the paragraph under the comparison table ("The Claude desktop app has a Keep computer awake setting
   too ..."): it misses that the app holds by itself while its Code sessions work (on by default), and that all its
   holds are an execution request only (live `powercfg`), which on Modern Standby does not stop idle sleep
   (Microsoft docs; not tested by us). Proposed text in Result.
3. The tray log writes one line per source ("Holding starts: Claude Code"), so while one CLI session holds, another
   session's start and end are invisible in it. `p6-logs-diagnostics` adds per-session lines; until then a live test
   cannot be judged from the tray log.
4. The Claude app takes an execution request by itself for 5 minutes at each credential rotation of its remote
   device link (03:04 and 04:00 here, about hourly). Harmless for Wakeshot (it reads no other app's requests), but a
   later idle-sleep test on this laptop will see it in `powercfg /requests`.

## Log

- 2026-11-07 02:29: card written by the orchestrator; Claude Desktop installed by the orchestrator.
- 2026-11-07 03:47: worker started (orchestrator wakeshot-4e). Tonight: no input in Haik's apps, no cargo build, no
  code change (`p6-logs-diagnostics` builds and replaces the installed tray). Holds are not judged by Haik's tray log
  tonight. Calls are logged "for Haik's review".
- 03:55: Claude Desktop runs since 02:33:46 (main pid 27612, Medium start by Haik's sign-in), now version 4.23118.0:
  Squirrel updated it from 2.42304.2 at 02:28, right after the install. At 02:33:53 it downloaded its own Claude
  Code 2.1.317 to `%APPDATA%\Claude\claude-code\2.1.317\<hash>\claude.exe`. Folders seen (names only):
  `claude-code-sessions\<id>\<id>\scheduled-tasks.json`, `local-agent-mode-sessions\...`, logs in
  `%LOCALAPPDATA%\Claude\logs` (`main.log`, `remote-control.log`, `cowork_vm_node.log`, ...).
- 03:59: read the app's main process code (`app.asar` unpacked to my scratch folder, read only, nothing run). Code
  sessions: env `CLAUDE_CODE_ENTRYPOINT=claude-desktop` (`claude-desktop-3p` for third-party sign-in), drops
  `CLAUDECODE` and `CLAUDE_CODE_CHILD_SESSION`, sets no `CLAUDE_CONFIG_DIR` of its own (only one the user puts in
  Desktop Settings), started through the Agent SDK (stream-json path), cron tools on unless the org turns scheduled
  tasks off. Cowork (`local-agent`): its own `CLAUDE_CONFIG_DIR` (`vmClaudeDir`); here "workspace VM not supported".
  Helper runs (usage probe, one-shot sampling, sign-in) use `persistSession: false`. The app's own keep-awake:
  `powerSaveBlocker` "prevent-app-suspension" with claims; settings in `claude_desktop_config.json` `preferences`:
  `keepAwakeEnabled` (default false), `ccKeepAwakeWhileWorking` (default true, "Keep computer awake while Claude
  works"), `ccKeepAwakeOnBattery` (default true). None is set in Haik's file, so the defaults hold. Not touched.
  `main.log` 03:04:37 to 03:09:37: the app held keep-awake 300 s for its `remote_tools` claim by itself.
  Desktop scheduled tasks: `claude-code-sessions\<account>\<org>\scheduled-tasks.json` (`cronExpression`, `fireAt`,
  `enabled`, ...), empty here. One file opened by mistake: `ssh-remote-server-state.json` (shape only; its `tokens`
  object was empty); no other token file opened.
- 04:05: `powercfg /requests` (read only): the app's `claude.exe` holds an EXECUTION request ("Electron") and no
  SYSTEM request, while `main.log` shows `[keep-awake] started (id=1, claims=remote_tools)` at 04:00:01 (a credential
  rotation of its remote device link; the 03:04 one was let go after 300 s). So the app's own keep-awake is
  execution only. No Desktop Code session ran tonight: `~/.claude/sessions` holds only the two CLI sessions
  (`entrypoint: cli`). Docs read: desktop-scheduled-tasks (each local run is a fresh session; a fixed few-minute
  delay per task; runs skipped while the PC sleeps, one catch-up run at wake) and desktop (nothing on local sessions
  at quit). In the code, quit stops every active local session ("Stopping N active session(s) on quit") and saves
  armed work for the next launch.
- 04:06: the tray log shows only "Holding starts: Claude Code" per source, and the orchestrator's CLI session holds
  the whole time, so tonight's tray log cannot show a Desktop session. The per-session lines of
  `p6-logs-diagnostics` or `agent-watch` can; the morning plan below uses both.
- 04:06: made the empty test folder `%TEMP%\wakeshot-desktop-test` for the morning session.
- 04:08: research written (`docs/research/claude-code-signals.md` new last section "Claude Desktop (2026-11-07)",
  `docs/research.md` new last section). Morning plan and steps written. Scratch files deleted (the unpacked
  `app.asar` and two read-only Python helpers). No process of mine left; no build folder made; no cargo run.
- 04:10: Status blocked on Haik's morning steps.

## Result

Status: blocked, 2026-11-07 04:10. The part that needs no input is done; the live part waits for Haik's "Morning
steps" (about 8 minutes, 5 prompts), then a worker finishes it as in "Morning plan".

**What was done tonight.** Read Claude Desktop 4.23118.0 without any input: its folders (names and JSON shapes only),
its logs, `powercfg /requests`, and its main process code (unpacked `app.asar`, read only). No click, key, prompt,
build or code change. Details: `docs/research/claude-code-signals.md`, "Claude Desktop (2026-11-07)".

| Case | What Wakeshot showed | Exact |
|---|---|---|
| Code session: start, work, end | Not run. By the code, the app's own Claude Code 2.1.317 writes `~/.claude/sessions/<pid>.json` with `entrypoint: claude-desktop` and the usual transcript, on the SDK path the Claude source already reads (tested live for `sdk-cli`) | needs Haik (steps 1 to 3) |
| Stop | Not run | needs Haik (step 2) |
| Approval wait | Not run. By rule, `waiting` holds only with Remote Control on | needs Haik (step 2) |
| Background task | Not run. Rule 3 transcript tasks apply to `claude-desktop` (code path checked) | needs Haik (step 3) |
| Window closed | Not run | needs Haik (step 4) |
| App quit | Not run. The app stops every active local session at quit (code) | needs Haik (step 5) |
| App crash | Not run: only by killing Haik's app. The pid wait and the SDK input-close exit are tested for CLI and SDK | not run with the app |
| `/loop` in a Code session | Not run. Cron tools are on in Code sessions (code); same transcript rule | needs Haik (optional step) |
| Desktop scheduled task: a run | Not run. Docs: each run is a fresh session, so it should hold as a Code session | not verified |
| Desktop scheduled task: wait before a run | No hold: Wakeshot does not read the app's `scheduled-tasks.json` | not supported (decision 3) |
| Cowork | Nothing on Windows: runs in the app's Linux VM with its own Claude folder; the VM is "not supported" here | not supported (decision 2) |
| Plain chats | No local file, process or keep-awake claim for chats (code) | not supported, no local signal (decision 1); step 6 confirms |
| The app's own keep-awake | Not touched. Execution request only (live); "Keep computer awake" off, "Keep computer awake while Claude works" on (defaults) | another holder, not a Wakeshot case |

**Fixes:** none proposed tonight. The Claude source already treats any entrypoint other than `cli` as a non-terminal
session (`crates/wakeshot-core/src/sources/claude/mod.rs`, the `cli` check before `open_tasks`), which covers
`claude-desktop`. A fix waits for the morning data.

**README lines for the video card** (until the morning run, only the "not supported" ones are final):

- Under "Claude Code" in "When each hold counts", after the morning run passes: "Code sessions in the Claude desktop
  app are Claude Code sessions too: the app runs its own copy of Claude Code, which writes the same files, so the
  same rules apply. Tested live with Claude Desktop 4.23118.0 and its Claude Code 2.1.317." Then drop "and the
  Desktop app" from "Not tested live yet".
- Limits: "The Claude desktop app's plain chats and Cowork tasks: not supported. Chats run on Anthropic's servers and
  leave nothing on the PC that shows a reply has ended; Cowork runs inside the app's own virtual machine, with its own
  Claude folder. The app's scheduled tasks hold while they run, but Wakeshot does not keep the PC awake for the next
  run: the app skips a run the PC sleeps through and runs it once when the PC wakes."
- Replace the paragraph under the comparison table: "The Claude desktop app keeps the PC awake by itself while its
  Code sessions work (its setting Keep computer awake while Claude works, on by default), and has a Keep computer
  awake switch for as long as it runs. Both use only Windows' execution request, which on a Modern Standby laptop
  does not stop idle sleep, and its docs say closing the lid still puts the laptop to sleep. They hold for that app
  only, not for Claude Code in a terminal, other agents or other programs."

**Decided by the worker, for Haik's review** (each with when to change it):

1. Plain chats are not supported: nothing local shows a reply's end. Change it when the app writes a turn state on
   disk.
2. Cowork is not supported: it runs in the app's VM with its own Claude folder, and the VM does not run on this
   laptop, so it cannot be tested here. Change it when Cowork runs on the Windows side with a known Claude folder.
3. Desktop scheduled tasks: a run holds as a Code session; the wait before a run is not held in v1 (it would need a
   new reader of the app's `scheduled-tasks.json` and the app's undocumented few-minute delay). Change it when users
   ask, or the delay rule is documented.
4. The app's own "Keep computer awake while Claude works" stays on (Haik's default). Wakeshot never edits another
   app's settings, and that hold adds only an execution request, which changes nothing Wakeshot reads. Change it if a
   later test measures idle sleep.
5. A `CLAUDE_CONFIG_DIR` set only inside the app is not followed, as with a per-shell value (`p4-live-claude`
   decision 2). Change it when users ask.
6. The app crash is not run live (it means killing Haik's app); the two possible ends are checks already tested. Change
   it when a PC nobody uses is available, or Haik ends the app from Task Manager once.
7. Morning test folder `%TEMP%\wakeshot-desktop-test`, so no repo or home folder gets test files. Change it never.

**Files:** changed `docs/tasks/p4-live-claude-desktop.md` (this card), `docs/research/claude-code-signals.md` (new
last section), `docs/research.md` (new last section). Created, outside the repo: the empty folder
`%TEMP%\wakeshot-desktop-test` (for the morning; the morning worker deletes it at the end). Created and deleted: my
scratch folder `p4-live-claude-desktop` (unpacked `app.asar`, two Python helpers). Nothing else deleted.

**Checks:** no Rust change, so no cargo check ran (none allowed tonight). No process of mine is left. Only read
access to Claude Desktop's files, except one shape read of `ssh-remote-server-state.json` by mistake (its `tokens`
object was empty, no value printed). No `.key`, credential, `config.json`, cookie or web storage file opened.

**Open items:** the morning run (steps above); then the table, redacted fixtures, any fix with tests, and the final
README lines.
