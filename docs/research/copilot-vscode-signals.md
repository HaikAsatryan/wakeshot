# Copilot in VS Code: how to tell it is working

Research for Wakeshot, 2026-11-06 (card `p4-copilot-vscode-research`). Same marks as `other-agent-signals.md`:
**verified** = seen in source code or official docs (path given), **unverified** = guessed, remembered or not checked.
Nothing was run: no chat was sent, VS Code was not started by me, no setting was changed. On the laptop only file
names, sizes and times were listed, plus the process list (image, pid, parent, `--type` flags only).

## Summary

| Harness (picker name) | Who runs the agent | Best signal | Path (Windows) | Confidence |
|---|---|---|---|---|
| Local (default) | Copilot extension in the extension host; chat model in the window | Last write of each request's `modelState` in the session's operation log: `0` Pending = working, `4` NeedsInput = waiting, `1`/`2`/`3` = done | `%APPDATA%\Code\User\workspaceStorage\<workspace id>\chatSessions\<session id>.jsonl`; empty window: `...\User\globalStorage\emptyWindowChatSessions\<session id>.jsonl` | Medium. Source read; written about once a minute, not live; no pid in the file |
| Copilot (agent host) | `copilot-runtime.exe --stdio`, child of the agent host process | The Copilot CLI rule on `events.jsonl` | `%USERPROFILE%\.copilot\session-state\<id>\events.jsonl` (or `COPILOT_HOME`) | Medium, same as the CLI rule; the runtime is not `copilot.exe` |
| Background (Copilot CLI in VS Code) | `@github/copilot/sdk` inside the extension host | The Copilot CLI rule | same `session-state` folder | Medium; process image not checked |
| Claude, Codex (agent host) | Claude Agent SDK under `Code.exe`; `codex.exe app-server` | The Claude and Codex sources, if these engines write their usual files | `~\.claude`, `~\.codex` | Low, not verified |
| Cloud agent | GitHub cloud | None local | n/a | n/a |

Answer for the card: **yes, partly.** The default Local harness leaves a reliable state field on disk, but with a delay
of about a minute and with no process id, so liveness is weaker than for the CLIs. The opt-in or experiment-driven
Copilot harness writes the same files as Copilot CLI and needs only a liveness fix in the CLI source.

Who blocks sleep: the Copilot extension takes an Electron `prevent-app-suspension` blocker for each model call and lets
it go 2 minutes after the last call ends. On Windows that is `PowerRequestExecutionRequired`, which does not block idle
sleep on Modern Standby. The agent host harnesses take nothing. Nothing touches the lid action or the battery timer.

## Sources

- `microsoft/vscode` tag `1.145.1` (release published 2026-11-06 00:11 UTC), source tarball from GitHub, read as data
  in the card's build folder. License MIT (`LICENSE.txt`), verified. Paths below are relative to the repo root.
- The laptop runs the same build: `Code.exe` 1.145.1, `product.json` commit `6bab64e2d12b`, date 2026-11-05.
  Verified.
- The Copilot extension now lives in that repo as `extensions/copilot` (version 0.73.0 in its `package.json`, MIT)
  and ships built in (`C:\Program Files\Microsoft VS Code\6bab64e2d1\resources\app\extensions\copilot`; no
  `github.copilot-chat` folder under `%USERPROFILE%\.vscode\extensions`). Verified.
- `microsoft/vscode-copilot-chat` is archived: last push 2026-05-20, last release `v0.43.0` (2026-04-07). Checked with
  `gh api`; its code was not read, since the current code is in the vscode repo.
- Chromium `services/device/wake_lock/power_save_blocker/power_save_blocker_win.cc` at `main`, read 2026-11-06.
  VS Code 1.145.1 targets Electron 43.8.3 (`.npmrc`), whose Chromium may be older than `main` (not checked).
- GitHub Docs, "GitHub Copilot plans" (`docs.github.com/en/copilot/get-started/plans`), read 2026-11-06.

## How Copilot runs in VS Code 1.145

- **Two harnesses for the Copilot agent.** "Local" is the old path: the chat model lives in the window (renderer) and
  the Copilot extension answers in the extension host. "Copilot" is the agent host: a separate utility process
  (`src/vs/platform/agentHost/electron-main/electronAgentHostStarter.ts:201-209`, entry
  `vs/platform/agentHost/node/agentHostMain`) that drives the Copilot SDK runtime. Verified.
- **Which one a new chat gets.** `chat.defaultToCopilotHarness` ("new editor and panel chat sessions default to the
  Agent Host Copilot SDK instead of the local harness") is `false` by default but marked `experiment: { mode:
  'startup' }`, so Microsoft's experiment service can turn it on per user; `chat.editor.preferCopilotHarness` is the
  same for editor chats (`src/vs/platform/agentHost/common/agentHostEnablementService.ts:71-95`). Managed settings
  (a sandbox floor or permission deny rules) force the Copilot harness unless `chat.temporaryDefaultToLocalHarness` is
  set (`src/vs/platform/policy/common/copilotManagedSettings.ts:342-352`). The user can also pick a harness per chat.
  Verified from the source. So Wakeshot must read both; which one a given user gets: unverified.
- **Session types** (`src/vs/workbench/contrib/chat/common/chatSessionsService.ts:428-436`): `local`, `copilotcli`
  (Background), `agent-host-copilotcli`, `agent-host-claude`, `copilot-cloud-agent`, `openai-codex` (contributed by
  the OpenAI extension, not in this repo). Verified as strings in the source.

## State on Windows: Local harness

- **User data folder.** `%APPDATA%\<product nameShort>`, so `%APPDATA%\Code` for Stable. Overrides, in order:
  `VSCODE_PORTABLE` (then `<that>\user-data`), `VSCODE_APPDATA` (then `<that>\<nameShort>`), `--user-data-dir`.
  Verified: `src/vs/platform/environment/node/userDataPath.ts:44-105`, `.../environment/node/environmentService.ts:20`.
  `User` is `<user data>\User`, `workspaceStorage` is `<User>\workspaceStorage` for every profile
  (`src/vs/platform/environment/common/environmentService.ts:56,86`). Verified.
- **Session file.** One file per chat: `<User>\workspaceStorage\<workspace id>\chatSessions\<session id>.jsonl`. A
  window with no folder uses `<User>\globalStorage\emptyWindowChatSessions\<session id>.jsonl` (default profile).
  Verified: `src/vs/workbench/contrib/chat/common/model/chatSessionStore.ts:68-79,731-742`. Before 1.109 the file was
  a flat `<session id>.json`; a hidden setting `chat.useLogSessionStorage: false` still selects it (line 740, not
  registered anywhere, so default on). Seen on this laptop: one `chatSessions\099c11b9-...jsonl` (1936 bytes,
  2026-11-02) under workspace `55992610...`. Not opened.
- **Only chat-panel and chat-editor sessions** (`initialLocation === Chat`, not imported) are stored this way. External
  sessions (Background, Copilot harness, cloud, Claude, Codex) store only metadata in the index, never a `.jsonl`.
  Verified: `chatServiceImpl.ts:464-475` (`shouldStoreSession`), `chatSessionStore.ts:405-420`.
- **Index.** The list of sessions is a key `chat.ChatSessionStore.index` in the workspace's `state.vscdb` (SQLite; the
  global one for empty windows). Its `lastResponseState` turns Pending and NeedsInput into Cancelled on purpose
  (`chatSessionStore.ts:867-873`), so it can never show work. Not useful. Verified.
- **Line format: an operation log** (`chat/common/model/objectMutationLog.ts:195-215`). Each line is one JSON object:
  - `{"kind":0,"v":{...}}` Initial: the whole session. Always the first line.
  - `{"kind":1,"k":[path],"v":...}` Set: replace the value at a path such as `["requests",3,"modelState"]`.
  - `{"kind":2,"k":[path],"v":[...],"i":n}` Push: drop items from index `i` (if set), then append `v` (if set).
  - `{"kind":3,"k":[path]}` Delete.
  Paths are property names and array indexes. A request whose key (`requestId`) changed is replaced whole by a Set on
  `["requests",n]` or a Push with `i` (`objectMutationLog.ts:534-645`). Verified.
- **Fields that matter** (`chat/common/model/chatSessionOperationLog.ts:132-220`, verified): top level `version` (3),
  `sessionId`, `creationDate`, `requests[]`, `pendingRequests[]` (queued messages), `inputState` (the draft in the
  input box). Per request: `requestId` (key), `timestamp` (ms, when sent), `modelState`, `isSystemInitiated`,
  `terminalExecutionId`, `agent`, `modelId`, and the full `message` and `response` (user text, model text, tool data).
- **`modelState`** is the response's raw state, written as it is at write time (`chatSessionOperationLog.ts:159`):
  `{"value":0}` Pending, `{"value":4}` NeedsInput, `{"value":1|2|3,"completedAt":ms}` Complete, Cancelled, Failed
  (`chat/common/chatService/chatService.ts:1904-1910`, `chat/common/model/chatModel.ts:1418-1421`). A new request's
  response starts Pending (`chatModel.ts:1442`, created in `addRequest`, `chatModel.ts:3676`). It can go from Complete
  back to Pending (`chatModel.ts:1960`). Verified.
- **NeedsInput** is set while the response waits on the user: a tool waiting for approval (before or after it runs),
  tool sign-in, a confirmation, a question carousel, a plan review, an elicitation (`chatModel.ts:1704-1773`).
  Verified. No "waiting since" time is written; `timeSpentWaiting` is not a timestamp in this format.
- **When the file is written.** Not on every change. The chat service saves all live local sessions on the storage
  service's `onWillSaveState` (`chatServiceImpl.ts:397,440-462`) and once more when a session model is disposed
  (`chatServiceImpl.ts:357-375`). `onWillSaveState` fires on a flush: every 60 s when the window is idle
  (`src/vs/platform/storage/common/storage.ts:336,376-384`; only the web and profile storage use other intervals), on
  window blur (`src/vs/workbench/browser/workbench.ts:239-243`), and at shutdown. While any request runs, the window
  turns background throttling off (`chat/electron-browser/chat.contribution.ts:150-157`), so the minute timer keeps
  running when VS Code is not in front. A write with no change appends nothing (`objectMutationLog.ts:435-439`).
  Verified from the source; real timing not seen.
- **Compaction rewrites the file.** After 1024 lines the next write replaces the whole file with one new Initial line
  (`chatSessionOperationLog.ts:222-226`, `objectMutationLog.ts:413-420`, `chatSessionStore.ts:385-386`: `writeFile`
  with `append: false`). So the file can shrink or change in place; a byte-offset tail breaks. Verified.
- **Restart heals old state.** When a session is loaded again, a Pending or NeedsInput request comes back as Cancelled
  (`chatModel.ts:3432-3435`), and the next save appends that change, because the reader keeps the log state it read
  (`chatSessionStore.ts:653-707`). A session that is never opened again keeps its last on-disk state. Verified.

## Processes

Seen on the laptop (VS Code 1.145.1 running, started 11:05 today):

- `Code.exe` main (no `--type`), its `--type=renderer` windows, `--type=gpu-process`, `crashpad-handler`, and several
  `--type=utility --utility-sub-type=node.mojom.NodeService`: the extension host (it had `--inspect-port=0`), the agent
  host, and others (pty host, shared process; not told apart).
- `copilot-runtime.exe --stdio`, parent the agent host. It was running with no chat in progress, so "runtime alive" is
  not "working". Image path `...\resources\app\node_modules.asar.unpacked\@github\copilot-sdk-win32-x64\prebuilds\
  win32-x64\copilot-runtime.exe` (780 KB). Path rule verified: `src/vs/platform/agentHost/node/copilot/
  copilotRuntimePaths.ts:47-53`, started over stdio: `copilotAgent.ts:4484`.
- Two `Code.exe` processes with no `--type` under the extension host: extension child processes run with Electron as
  node (unverified which; one is likely the Claude Code extension's).
- `%APPDATA%\Code\code.lock` holds the main process pid as text (5 bytes here), written at start and deleted at a clean
  exit (`src/vs/code/electron-main/main.ts:143,154`). A crash leaves it. Verified.
- The workspace's `state.vscdb` is opened by the main process when a window loads that workspace and closed only at
  shutdown (`src/vs/platform/storage/electron-main/storageMainService.ts:123-158,273-291`). So while VS Code runs it
  stays open even after that window closes. Verified from the source; whether Restart Manager reports it: unverified.

## Working, waiting, idle: Local harness

Rule designed from the source, not tested on real files:

- Replay the log: take `requests[]` from the Initial line, then apply Push, Set and Delete on `["requests", ...]`
  paths. Keep only `requestId`, `timestamp` and `modelState.value` per request.
- **Working:** some request's last `modelState.value` is `0`.
- **Waiting:** none is `0` and some is `4`.
- **Idle:** all are `1`, `2` or `3`, or there are no requests. A request with no `modelState` is old data: idle.
- **Unknown value** (another number, or a `kind` other than 0 to 3): `FormatHealth::UnknownValue`, which counts as
  working while VS Code lives (same rule as Claude's unknown `status`).
- **Delay:** the file says Working up to about a minute after a request starts, and Idle up to about a minute after it
  ends (sooner if the window loses focus). For a keep-awake tool that only shifts the hold by a minute.

## Copilot harness and Background sessions

- Both use the Copilot home: `COPILOT_HOME` or `%USERPROFILE%\.copilot`. Agent host: `src/vs/platform/environment/
  common/copilotHome.ts:20`, passed to the runtime as `COPILOT_HOME` (`copilotAgent.ts:4355`). Extension Background
  sessions: `extensions/copilot/src/extension/chatSessions/copilotcli/node/cliHelpers.ts:11-23`, and "both surfaces
  share `~/.copilot/session-state/`" (`copilotcliSessionService.ts:529`). Verified.
- The agent host reads `session-state\<id>\events.jsonl` and `plan.md` for its sessions (`copilotAgent.ts:4903,4914`),
  so the runtime writes the CLI's event log there. Verified that the paths are used; the event schema is the runtime's
  and was not re-read here (see "GitHub Copilot CLI" in `other-agent-signals.md`).
- **Lock files seen on this laptop** (names only): `.copilot\session-state\92ed0e2b-...\inuse.9260.lock` (5 bytes) and
  `inuse.9260.hold` (0 bytes), from 2026-11-03 09:10, pid 9260 no longer running, and no `events.jsonl` in that
  folder; plus `session-state\.session-operation-locks\<id>.lock` (0 bytes) for nine sessions, one of them the id of
  an agent host session folder `%APPDATA%\Code\agentSessionData\681372ab-...`. So the CLI research's guess
  `inuse.<pid>.lock` is right, there is also an `inuse.<pid>.hold`, and a folder can hold a lock with no events (an
  unused or pre-warmed session; pre-warming: `src/vs/platform/agentHost/AGENTS.md`, "Session pre-warming"). Which of
  the two files is held open: unverified.
- **Process.** Agent host sessions run in `copilot-runtime.exe`; Background sessions run the SDK inside the extension
  host (`Code.exe`; whether the SDK starts a child process: unverified). Neither is `copilot.exe`.
- Waiting: the CLI rule uses `permission.requested`. In the agent host VS Code answers permissions through its client
  (`copilotApprovalPolicy.ts`); whether the runtime still persists `permission.requested`: unverified.

## Background and scheduled work

- **Long tool calls** (a build in the terminal) keep the request Pending, so they count as working. Verified by the
  state model: the response stays Pending until `_complete` or `cancel` (`chatModel.ts:1964-2004`).
- **Background terminals.** A command that keeps running after the tool returns does not keep the request open.
  When it ends or asks for input, VS Code starts a new system-initiated request (`isSystemInitiated`,
  `terminalExecutionId`; "Terminal completion and input-needed notifications are now always enabled for any command
  that continues running after the tool returns", `src/vs/workbench/contrib/terminalContrib/chatAgentTools/common/
  terminalChatAgentToolsConfiguration.ts:652-659`). Between the two, nothing on disk says the command still runs
  (unverified whether the serialized tool data shows it). Same gap as Claude's SDK path.
- **Queued messages** (`pendingRequests`) are sent after the running one; they add nothing while it runs.
- **Agent host automations** (scheduled work, `src/vs/platform/agentHost/node/agentHostAutomationService.ts`, held
  after wake by `agentHostSystemWakeService.ts`) exist. Where they are stored and whether a next-run time is readable:
  not researched.

## Prevents sleep itself?

- **Copilot extension (Local harness): partly, and not on Modern Standby.** Each model call (`ChatLocation` other than
  `Other`) acquires a blocker; the last release starts a 2 minute timer before it stops
  (`extensions/copilot/src/extension/prompt/node/chatMLFetcher.ts:963-964`,
  `extensions/copilot/src/extension/power/vscode-node/powerService.ts:12,38-78`). The switch is the team-internal,
  experiment-based setting `github.copilot.chat.advanced.chatRequestPowerSaveBlocker`, default `true`
  (`extensions/copilot/src/platform/configuration/common/configurationService.ts:1041`). The blocker is
  `vscode.env.power.startPowerSaveBlocker('prevent-app-suspension')`, which goes to Electron's
  `powerSaveBlocker.start` in the main process (`src/vs/platform/native/electron-main/nativeHostMainService.ts:
  1684-1685`). Chromium maps that type to `PowerRequestExecutionRequired` only (`power_save_blocker_win.cc:106-110`,
  read 2026-11-06; matches `docs/research.md`, "Electron"). So it does not stop idle sleep on Modern Standby, and a
  tool call longer than 2 minutes is not covered even on S3. Verified from source; not tested.
- **Agent host harnesses: no.** No power blocker call in `src/vs/platform/agentHost`; in the core only the extension
  API starts one (`cloudSandboxAgentHostService.ts` uses the power service for suspend and resume events only).
  Whether `copilot-runtime.exe` honors the CLI's `keepAlive` setting in `~\.copilot\settings.json`: unverified.
- No lid handling and no battery timer anywhere.

## Hooks and API (fallback)

- **Hooks exist.** `chat.useHooks` defaults to `true`; `chat.useClaudeHooks` (read Claude's hook files) to `false`
  (`src/vs/workbench/contrib/chat/browser/chat.shared.contribution.ts:2295-2318`). Events: `SessionStart`,
  `SessionEnd`, `UserPromptSubmit`, `UserPromptTransformed`, `PreToolUse`, `PostToolUse`, `PreCompact`,
  `SubagentStart`, `SubagentStop`, `Stop`, `ErrorOccurred` (`chat/common/promptSyntax/hookTypes.ts:12-24`). No
  permission or notification event, so hooks cannot show waiting. Verified. Not usable anyway: product rule "No hooks"
  (`docs/product.md:33`).
- **Agent Host Protocol endpoint.** The agent host serves a WebSocket on a named pipe to same-user processes and
  publishes `<user data>\agent-host\local-endpoint\entries\<sha256>.json` with `pid`, `endpoint` and a
  `connectionToken` (`src/vs/platform/agentHost/LOCAL_ENDPOINT.md`). One entry exists on this laptop (332 bytes, not
  opened). It would give live session state for agent host sessions only, and needs the bearer token. Not
  recommended; treat the entry files like Claude's `.key` files.
- The extension's own transcript (`<workspace storage>\GitHub.copilot-chat\transcripts\<session id>.jsonl`, CLI-like
  events) is started only when hooks are configured (`extensions/copilot/src/extension/intents/node/
  toolCallingLoop.ts:1195-1219`) and flushed only before hooks and at summarization. Not a general signal.

## Free testing

- GitHub Docs "Copilot plans" (read 2026-11-06): Copilot Free includes agent mode and MCP; "Limited to 2000 completions
  per month"; "An allowance of GitHub AI Credits" with no number given; model choice is auto only. So the Local
  harness can be tested free. Whether Free can use the Copilot harness or Background sessions: unverified.

## Recommended detection rules

Scan on a change event in the watched folders plus the safety tick. Liveness is checked by the watch thread.

| # | Rule | Signal | Confidence |
|---|---|---|---|
| 1 | Find the user data folder: `%APPDATA%\Code` (Stable). Insiders is `%APPDATA%\Code - Insiders` (from memory, unverified). `VSCODE_APPDATA` or a portable install moves it; a value set only in one shell is not seen by the tray | Where to look | High for Stable |
| 2 | Watch `<User>\workspaceStorage\*\chatSessions\` and `<User>\globalStorage\emptyWindowChatSessions\`. Accept `*.jsonl` only; skip `*.json` (pre-1.109 or the hidden setting) | Session files | High |
| 3 | Read a session file whole, not as a tail (compaction rewrites it in place). Only read files changed since Wakeshot started or in the last 24 h, so old sessions (up to 400 per workspace, `chatSessionStore.ts:37`) are not parsed at start. A last line that does not parse: retry once (a write may be in progress), then `Unreadable` | Parse cost and safety | High for the need; the mtime filter is a contract change |
| 4 | Replay `kind` 0 to 3 on `requests`; per request keep `requestId`, `timestamp`, `modelState.value`. Any `0`: Working. Else any `4`: Waiting (`since_wall_ms: None`, `remote: false`). Else Idle | Working / waiting / done | Medium (source read; never seen live) |
| 5 | Unknown `kind` or `modelState.value`: `UnknownValue`, counts as working while VS Code lives, shows "update Wakeshot" | Format change | High as a rule |
| 6 | Liveness: `AnyProcess { image: "Code.exe", lock_file: <that workspace>\state.vscdb }` (empty window: `<User>\globalStorage\state.vscdb`). Restart Manager finds the main process while VS Code runs | VS Code alive | Low to medium: RM on SQLite files untested; the fallback (any `Code.exe` while the file exists) is weak because the file always exists |
| 7 | Stale Pending after a crash: drop a Working or Waiting request whose `timestamp` is older than the current VS Code main process start (`code.lock` pid's creation time). Needs a contract change (see below) | Crash leftovers | Medium as a rule; no source today |
| 8 | Copilot harness and Background: no new source. The Copilot CLI source covers them if its liveness accepts the process that holds the session's `inuse.*` file (`copilot-runtime.exe`, `Code.exe`), not only `copilot.exe` | Copilot SDK sessions in VS Code | Medium, as the CLI rule |
| 9 | Do not read the `state.vscdb` index, `agent-host.db`, `agentSessionData\*\session.db` or the local-endpoint entries | Nothing useful, or secrets | High |

## What else the same signal covers

Only what the source shows:

- **Any chat participant in a Local session**, not only Copilot: the log stores every request of a panel or editor
  chat with its `agent`, and the state machine is the core's, not the extension's. So third-party chat participants
  are covered by the same rule. Verified from the schema; not tested.
- **VS Code Insiders** builds from the same repo, so the same code path applies, under its own user data folder
  (`%APPDATA%\<nameShort>`). Insiders often runs a newer format first; the unknown-value rule handles that. The
  Insiders `nameShort` was not checked (no Insiders product.json read).
- **Forks** that keep this chat code (the repo's own `product.json` names `Code - OSS`, data folder `%APPDATA%\Code -
  OSS`) write the same files under `%APPDATA%\<their nameShort>`. Whether VSCodium, Cursor, Windsurf or others ship the
  chat panel and the built-in Copilot extension: not checked. Cursor and Windsurf have their own agents; nothing here
  covers them.
- **Copilot harness and Background (Copilot CLI) sessions**: already in the Copilot CLI's files (see above).
- **Claude in the chat panel (agent host):** runs the Claude Agent SDK with `executable: process.execPath`, so the
  engine is `Code.exe` as node (`src/vs/platform/agentHost/node/claude/claudeSdkOptions.ts:177`). Whether it writes
  `~\.claude\sessions\<pid>.json` like other SDK sessions: unverified. If it does, the Claude source covers it by pid
  and start time.
- **Codex in the chat panel (agent host):** `codex.exe app-server`, with `CODEX_HOME` inherited unless a VS Code
  variable overrides it (`src/vs/platform/agentHost/node/codex/codexAgent.ts:2661,2695-2698`,
  `codexLaunchConfig.ts:162`). It claims Codex's thread writer lock (`AGENTS.md`, Codex notes). So the Codex source
  likely covers it (image `codex.exe`); not verified live.
- **The Claude Code extension** (`anthropic.claude-code`, its own panel) is not this chat; the Claude source already
  covers it (`entrypoint: claude-vscode`).
- **Cloud agent** and **OpenAI's Codex extension** (`openai-codex` session type): nothing local, or not in this source.

## Privacy

Files that hold prompt or code text, so a source never stores or logs their content:

- `chatSessions\*.jsonl` (and old `*.json`), `emptyWindowChatSessions\*`, `globalStorage\transferredChatSessions\*`:
  full prompts, answers, tool inputs and outputs, attachments, and the unsent draft (`inputState.inputText`). Parse
  only `kind`, `k`, and `requestId`, `timestamp`, `modelState` under `requests`; skip every other value without
  keeping it (serde `IgnoredAny`).
- `workspaceStorage\<id>\chatEditingSessions\<id>\state.json` and `contents\`: file snapshots (code). Never open.
- `state.vscdb` (workspace and global): chat titles, external drafts. Never open.
- `globalStorage\github.copilot-chat\session-store.db`, `workspaceStorage\<id>\GitHub.copilot-chat\transcripts\`,
  `debug-logs`, `chat-session-resources`: extension data with prompts (`extensions/copilot/src/extension/extension/
  vscode-node/services.ts:280`, `sessionTranscriptService.ts:66`). Never open.
- `globalStorage\agent-host.db`, `<user data>\agentSessionData\<id>\session.db`: agent host session data. Never open.
- `<user data>\agent-host\local-endpoint\entries\*.json`: holds a bearer `connectionToken`. Never open (like `.key`).
- `~\.copilot\session-state\<id>\events.jsonl`, `plan.md`, `workspace.yaml`: the CLI rule applies; lock and hold
  files are listed, never opened.

## For the source card

- **Files to watch:** `<User>\workspaceStorage\*\chatSessions\*.jsonl` and `<User>\globalStorage\
  emptyWindowChatSessions\*.jsonl`; whole reads of changed files only. Liveness files: the matching `state.vscdb`
  (never opened, only given to Restart Manager) and `<user data>\code.lock` (5 bytes, the pid; safe to read).
- **Parser:** bytes of one `.jsonl` in, one `SessionFacts` out. `key`: a new `SourceId` (for example `VsCode`) and the
  file's session id (a UUID, also `sessionId` in the Initial line; sessions can move between workspace folders, the id
  stays). `activity` per rule 4. `timers`: none. `open_tasks`: 0 (background terminals are not visible).
  `label`: "Copilot in VS Code" (it also covers other chat participants; "VS Code chat" is the honest name).
  `format`: `Unreadable` / `UnknownValue` per rules 3 and 5.
- **Contract needs** (the contracts change only through a card the orchestrator runs alone):
  - A read mode that gives whole files of a folder tree but only files changed recently (mtime filter), because
    `Folder` reads every file at start and `Tail` breaks on compaction.
  - For rule 7, a liveness the parser can state without I/O, such as "pid from `code.lock`, alive, image `Code.exe`,
    created before `timestamp`". Today `Liveness::Process` needs the exact start time, which the file does not have.
    Default without it: rule 6 only, and accept that a crash leftover holds until VS Code exits or reopens that chat.
- **Liveness default:** rule 6. Change it when the live test shows Restart Manager does not report `state.vscdb`, or
  when rule 7's contract change lands.
- **Copilot CLI source** (`p4-copilot-source`): accept `copilot-runtime.exe` and `Code.exe` as owners, and expect a
  lock with no `events.jsonl` (an unused session). Its card owns that change.

### Fixture sketch

Built from the schema, not copied from a real file. Real lines carry many more fields, mostly text.

```jsonl
{"kind":0,"v":{"version":3,"creationDate":1793700000000,"sessionId":"0b1c2d3e-0000-4000-8000-000000000001","requests":[],"pendingRequests":[]}}
{"kind":2,"k":["requests"],"v":[{"requestId":"request_1","timestamp":1793700005000,"message":{"text":"fix the build","parts":[]},"response":[],"modelState":{"value":0}}]}
{"kind":1,"k":["requests",0,"modelState"],"v":{"value":4}}
{"kind":1,"k":["requests",0,"modelState"],"v":{"value":0}}
{"kind":2,"k":["requests",0,"response"],"v":[{"value":"Done."}]}
{"kind":1,"k":["requests",0,"modelState"],"v":{"value":1,"completedAt":1793700090000}}
```

Line 2 is Working, line 3 Waiting, line 4 Working again, line 6 Idle. A compacted file is a single `kind: 0` line whose
`requests[0].modelState` carries the state. The `requestId` format is a guess.

## Open questions only a live test answers

1. How soon the `.jsonl` shows `{"value":0}` after a prompt is sent and `completedAt` after it ends, with the window in
   front, behind another window, minimized, and with the screen locked.
2. Whether a tool approval writes `{"value":4}` within the minute, and back to `0` after approval.
3. Whether Restart Manager reports `Code.exe` for the workspace `state.vscdb` (and for the global one), from an
   unelevated (Medium) reader.
4. Closing the window (other windows open) or Reload Window while a request runs: is the last write Pending or
   Cancelled? A Pending one would hold until VS Code exits.
5. A VS Code crash mid-request, then a restart that does not open that chat: the leftover stays Pending (from the
   source); confirm, and size rule 7.
6. Copilot harness: the runtime's `inuse.<pid>.lock` and `.hold` names and pid for agent host sessions, which one is
   held open, and whether `permission.requested` is persisted when VS Code shows the approval.
7. Background (`copilotcli`) sessions: which process owns the lock (extension host or a child).
8. Whether Claude in the chat panel writes a `~\.claude\sessions\<pid>.json`, and Codex a rollout plus writer lock.
9. Copilot Free: whether the Copilot harness works on Free.

## Built (`p4-vscode-source`, 2026-11-06)

`crates/wakeshot-core/src/sources/vscode.rs`, tests in `crates/wakeshot-core/tests/vscode_source.rs`, fixtures (from
the schema) in `crates/wakeshot-core/tests/fixtures/vscode/`. Not registered in the tray yet and not tested on a real
chat: the live card does both. What differs from the rules above, and what was checked again in the 1.145.1 source:

- **Rules built:** 1 (Stable and Insiders, `VSCODE_APPDATA`), 2, 3, 4, 5 and 7. Rule 6 is not used: rule 7 needs no
  Restart Manager and drops a crash leftover once VS Code starts anew. Rules 8 and 9: rule 8 is the Copilot CLI live
  card's; rule 9 holds because the name filter passes only `<id>.jsonl`.
- **Insiders**, rule 1's guess, is now verified: its folder is `Code - Insiders` (VS Code docs,
  `docs/configure/profiles.md` in `microsoft/vscode-docs`) and its image `Code - Insiders.exe`
  (`test/sanity/src/context.ts:1005,1242`). Its chats live by its own image.
- **Portable installs are not read.** VS Code sets `VSCODE_PORTABLE` inside its own process when a `data` folder sits
  next to the install (`src/bootstrap-node.ts:205-244`), so the tray never sees it. `--user-data-dir` is not seen
  either. Empty-window chats use the default profile's `globalStorage` for every profile
  (`chatSessionStore.ts:68-73`), and the older empty-window folder `workspaceStorage\no-workspace\chatSessions`
  (line 75) is inside the workspace want.
- **Line rules** (`objectMutationLog.ts:195-215,334-501`): every entry is written with a closing `\n`; Delete sets
  the value to undefined; a Set to undefined has no `v`; a write past the end of a list leaves holes; Push cuts at
  `i`, then appends; a Set or Delete on the root does nothing; the reader throws on a line before the Initial line and
  on an unknown `kind` (`assertNever`). `modelState` is a primitive (`chatSessionOperationLog.ts:159`), so it is always
  set whole at `["requests",n,"modelState"]`; the source also takes `["requests",n,"modelState","value"]` in case that
  changes. `requestId` is the key: a changed request is replaced from its index by a Push with `i`.
- **A retry keeps the old time.** `reopen()` (`chatModel.ts:1945-1961`) turns a done request back to Pending with its
  first `timestamp`; `resendRequest` does it for a retry that keeps the request id (`chatServiceImpl.ts:1471,2029`,
  used by the error card, `chatErrorConfirmationPart.ts`). A request seen going from done back to Pending or
  NeedsInput therefore counts while any `Code.exe` runs. Compaction drops that history, so a compacted reopened
  request falls back to its first time (not verified how often this happens).
- **Unknown kinds count only on a path the source reads** (`requests`, one request, its `modelState` or `timestamp`, or
  no path). A new kind of write elsewhere, for example in the answer text, is skipped, so it cannot hold the PC. An
  unknown `modelState.value` counts only on a chat's newest request: requests run one after the other, so an older
  one's value is history.
- **Not verified:** serde_json stops at 128 levels of JSON nesting, so a chat whose saved tool data nests deeper would
  be unreadable; whether real chats nest that deep was not checked.

## Phase 4 live test (2026-11-06)

Card `p4-live-vscode`. VS Code 1.145.1 (commit `6bab64e2d12b`), built-in Copilot extension 0.73.0, Copilot Free
(model `copilot/auto`), the test laptop, 15:58 to 16:20 local time (UTC+4). Readers at Medium: `agent-watch --source
vscode,copilot` (through the pseudo console driver) and the dry-run tray, both built from the tree of that day. A
structure-only monitor (kinds, paths, send times, `modelState`; no text) timed the writes. Five one-line prompts.

**How it ran.** Haik's own VS Code was running, elevated (title "[Administrator]", started 11:05 from a shell that has
since ended). A Medium `code` cannot join an elevated instance: VS Code shows "Another instance of Code is already
running as administrator" (`src/vs/code/electron-main/main.ts:396-400`). So the test windows were opened in his
instance from an elevated shell (`code -n <test folder>`), and the chats ran at High; real users mostly run VS Code at
Medium (not tested). Prompts went in with `code chat -m <mode> "<prompt>"` from the test folder: VS Code sends it to
the window of the current folder (`windowsMainService.ts:520-538`), runs a trust check, starts a new chat, then sends
(`electron-browser/chat.contribution.ts`, `prompt()`). The test folder sat inside the repo folder he had open, so it
was trusted. The screen was locked from 15:57:50 to 16:08:20, so no window lost focus during prompts 1 to 4.

| Step | What happened |
|---|---|
| Prompts 1, 2 (agent mode, `sleep 60`) | Both went to the **Copilot harness**: `~\.copilot\session-state\<id>\` made 0.8 s after the send, with `inuse.2728.lock` (5 bytes), `inuse.2728.hold` (0 bytes), `.workspace-fork.lock`, `workspace.yaml`, `events.jsonl`; 2728 is `copilot-runtime.exe`, the agent host's child, started with VS Code at 11:05. The turns ran 61 s and answered. The Copilot CLI source posted them working 1.4 s after the folder appeared, but with `any copilot.exe alive=Gone`, so they would not hold (rule 8 is not built; sent to `p4-live-copilot`) |
| Harness switch | A workspace setting `chat.defaultToCopilotHarness: false` written while the window was open did not change the next chat: New Chat keeps the current chat's harness (`clearChatSessionPreservingType`; `constants.ts`, `currentSession`). In a window opened after the setting, the next chat was Local. So here the computed default is the Copilot harness (likely the `experiment: startup` rollout; not verified which) |
| Prompt 3 (agent mode, `sleep 60`), Local | File first written 44 s after the send, one Initial line, `modelState` 4 (NeedsInput). The terminal call says "Auto approved by rule `sleep (default)`", but no shell started and the chat waited for a click (what it showed is not saved in the file and was not seen; the renderer log shows a `toolRiskAssessment` at the call; one candidate is the one-time auto approve warning, `TerminalAutoApproveWarningAccepted` in application storage, not verified). `agent-watch` posted `waiting(remote=false)`, `since Code.exe alive=Yes`, 25 ms after the write |
| Prompt 4 (ask mode, one word), Local | Sent 16:07:53.0, done 16:07:55.9, first written 16:08:29.8, already done: Pending never reached the disk. `idle` posted 13 ms before the monitor saw the file |
| Prompt 5 (`code chat -n`, empty window) | Copilot harness again (`session-state\<id>`, 6 s turn); no `emptyWindowChatSessions` folder exists on this PC |
| Dry-run tray (Medium, only VS Code can hold) | "VS Code chat is watched in the folder APPDATA names"; the waiting and the idle chat gave no hold, as the rules say (waiting holds only when remote). CPU 0.95 s in 352 s (debug build, all five sources, four on empty folders) |

**Answers to "Open questions only a live test answers":**

1. Delay: each window saves on its own 60 s beat (16:03:29.7, then 16:08:29.8, nothing in between since nothing
   changed), plus a save on blur. First write 44 s after a send, and 34 s after a turn ended, with the screen locked.
   A turn that ends before the next beat never shows Pending on disk, so Wakeshot never holds for it (harmless: it
   was over). In front, behind, minimized as separate cases: not tested (Haik used the laptop meanwhile).
2. NeedsInput is written at the first beat after the wait starts (seen live). Back to 0 after the approval: not
   tested live (needs a click; no computer use).
3. Restart Manager on `state.vscdb`: not needed, the source does not use rule 6; not tested.
4. Window closed or reloaded mid-request: not tested live. A close while a Local chat runs or waits asks "A session is
   in progress. Are you sure you want to close the window?" (`chatLifecycle.ts`; `isSessionInProgressStatus` takes
   InProgress and NeedsInput), so it needs a click.
5. Crash: not run (only Haik's VS Code could be ended; a new `--user-data-dir` has no sign-in).
6. Copilot harness: the lock and hold are `inuse.<pid>.lock` and `inuse.<pid>.hold` with the pid of
   `copilot-runtime.exe`, one runtime for all sessions of the VS Code instance; which file is held open and whether
   `permission.requested` is written: not tested.
7. and 8. Background sessions, Claude and Codex in the chat panel: not tested.
9. The Copilot harness works on Copilot Free (prompts 1, 2, 5 answered).

**Also seen:**

- The Medium readers see the files and the processes of an elevated VS Code: `ImageSince` gave `alive=Yes`.
- A read at the moment VS Code made a file got 0 bytes; the source asked for its one retry, and the read 14 ms later
  parsed (rule 3 works).
- Watch cost: in 39 minutes the VS Code source posted three times, each on a real chat change, although VS Code
  writes other files under `workspaceStorage` all the time (contracts round 3, "Found outside scope" 3).
- Every saved line read with no format problem. The real Initial line has ten top-level keys, among them `version`
  3, `requests`, `inputState`, `customTitle`, `responderUsername`, `hasPendingEdits`; a request carries `modelId`,
  `modeInfo`, `copilotCredits` (0.26 for prompt 3) and `timeSpentWaiting`. No change to the parser was needed.
- Compaction (1024 lines): not reached; these chats had one line each.
- Each `code chat` brings its window to the front, and a file opened from Explorer goes to the last active VS Code
  window: one of Haik's files opened in the test window. A live test beside a person must expect that.
- Redacted copies: `crates/wakeshot-core/tests/fixtures/vscode/live/`.

## Stop, approve and window close live test (2026-11-06)

Card `p4-live-vscode-stop`, for Haik's "No guessing" rule (`docs/product.md`): is the end of each case seen exactly?
VS Code 1.145.1, built-in Copilot extension 0.73.0, Copilot Free, the test laptop, 19:27 to 21:03 local time (UTC+4).

**How it ran.** My own window inside Haik's elevated VS Code (a Medium `code` cannot join it): `code
--extensionDevelopmentPath=<scratch extension> <test folder>`, an Extension Development Host window, so nothing was
installed and the extension ran only there. It ran, from step files, the same commands the buttons run (read in the
installed bundle `workbench.desktop.main.js`): `workbench.action.chat.cancel` is the Stop button (menu
`ChatExecute`), calling `chatService.cancelCurrentRequestForSession`; `workbench.action.chat.acceptTool` presses the
confirmation's primary button (Allow, one call); `workbench.action.chat.skipTool` is Skip;
`workbench.action.closeWindow` is the window's close. Chats were opened with `workbench.action.chat.newLocalChat` and
`workbench.action.chat.openNewSessionSidebar.agent-host-copilotcli`, prompts sent with `workbench.action.chat.open`
and a query. No key, click or UI automation from my side. Readers: `agent-watch --source copilot,vscode` at Medium
and a structure-only monitor (chat file kinds and `modelState`, Copilot event types, in-use file names, process
images). Times below run from the command to the `agent-watch` post. 19 one-line prompts. The close of a window
with a Local chat in progress asks "A session is in progress. Are you sure you want to close the window?"; Haik
clicked Yes there (the dialog skips only under `--extensionTestsPath`, a mode that kills the whole VS Code process
when its test ends, so not used).

| Case | Copilot harness | Local |
|---|---|---|
| Stop during `sleep 60` (3 runs each) | `abort` (`reason` `user_initiated`), then `assistant.turn_end`, 34 to 58 ms after the command; idle posted 0.10 to 0.19 s after it. The command (`pwsh` under `copilot-runtime.exe`) is ended. **Exact** | The request is saved as Cancelled (`modelState` 2 with `completedAt`) at the window's next save, 39.5 to 51.6 s after the Stop; idle posted 28 to 54 ms after that save. The terminal is closed, so the command ends. **Exact, up to a minute late** |
| Approval left 1 minute, then Allow | `permission.requested` posts waiting; `permission.completed` 10 ms after Allow posts working; idle at `assistant.turn_end`. **Exact** | NeedsInput (4) at a save, then Pending (0), then Complete (1), each at a save. **Exact, up to a minute late** |
| Approval left 1 minute, then Skip | `permission.completed`, `tool.execution_complete` with `success` false, working until the model's short reply ends the turn. **Exact** | NeedsInput, then Complete at the next save. **Exact, up to a minute late** |
| Window closed while a turn works | No dialog (the close veto asks only at quit for agent host chats). The agent host keeps the turn: the command ran to its end and `assistant.turn_end` came 51 s after the close; idle posted 79 ms later. **Exact** (the agent really worked on) | The last save before the close said Pending (20:51:34.856, request sent 20:47:37.119); Yes at 20:51:57.6, the window and its terminal ended at 57.8, and **nothing was written to the chat log** (only `chatEditingSessions\<id>\state.json` and the workspace `state.vscdb`, which rule 9 does not read). Opening the chat again in a new window and closing it wrote nothing either. `agent-watch` kept it working (`since Code.exe alive=Yes`): it holds until every VS Code started before the send has ended. **Not exact: a blocker** |

**Also seen:**

- **A Local turn does not stay working for a long command.** The terminal tool stops waiting after 5 minutes with
  no output (`chat.tools.terminal.idleSilenceTimeoutMs`, restarted by any output; seen with `sleep 1800`: the tool
  returned exactly 5 min after the command started) and, here, after about 2 minutes with output (`ping -n 1800`;
  `elapsedMs` 146767; most likely the `timeout` the model passes, enforced by
  `chat.tools.terminal.enforceTimeoutFromModel`;
  which one was not seen). The model then answers, the turn is saved done, and the command runs on in the terminal
  with nothing on disk saying so. So Wakeshot ends the hold while that command still runs: the same gap as the
  background terminals above, and not a guess (the agent's turn did end). A later tool call in the same terminal
  stops the old command (exit 1). The harness ran `sleep 60` to its end; longer harness commands were not tried.
- **Local terminal calls ask every time on this PC.** VS Code's default auto approve rules (`sleep` among them) apply
  only after their one-time warning is accepted (`chat.tools.terminal.autoApprove.warningAccepted`, set only by the
  "enable" choice and its dialog). The primary Allow approves the one call and stores nothing. In the harness VS
  Code approved `sleep` itself 5 to 12 ms after the runtime asked, and `ping` waited.
- **No save at a Stop.** The Local file is not written when Stop is pressed; the Cancelled state waits for the next
  60 s save (or a blur: one Pending reached the disk at a blur).
- **Pre-warmed harness sessions.** Opening a window made two `session-state` folders with in-use files of
  `copilot-runtime.exe` and no turn; one was released when the first harness chat began, the other 20 minutes
  later. They read idle and hold nothing.
- **Event times are UTC** in the harness `events.jsonl`, as in the CLI's.
- **A minimized window can keep the foreground.** My window, minimized right after it opened, stayed the foreground
  window until Haik clicked elsewhere, so a key typed meanwhile would have gone to it. A live test beside a person
  should expect that.
- Redacted copies: `crates/wakeshot-core/tests/fixtures/vscode/live/` (`stop-three-then-approve.jsonl`,
  `deny-after-wait.jsonl`, `idle-silence-end.jsonl`, `close-while-working.jsonl` and
  `harness/stop-approve-deny-close.jsonl`).

**Local close: no exact signal found.** VS Code saves the chat log on its 60 s beat and when the model is disposed,
but it does not cancel the running request first, and a log write with no change appends nothing; so the last state
on disk stays Pending. The one file written at the close, `chatEditingSessions\<id>\state.json`, is the edit
snapshot store: `ChatEditingSession.storeState` runs at the chat's dispose (`chatEditingServiceImpl.ts:91-95`) and
at every storage flush for sessions marked global (lines 111-126, `startOrContinueGlobalEditingSession`). Here it was
first written at the chat's first save (20:25:18.626, the log 5 ms before), so it is written while a turn works too
(which chats are global was not checked); its time cannot tell a close from a live turn. The research claim that a
reload heals the request to Cancelled on disk (`chatModel.ts:3432-3435`) was not seen: the reopened chat wrote
nothing in 4.5 minutes nor at its window's close. Rule 7 still drops the leftover once VS Code starts anew. The same
is expected for a Reload Window or a folder change mid-turn (same dispose path; not run). The leftover test chat was
deleted after the fixture was taken, so no later dry-run tray on this PC holds on it.

## Local chats out of v1 (2026-11-06)

Haik, 21:28, on the default the Stop live test proposed: "your default answer is good" (card
`p4-vscode-local-off`). Wakeshot v1 does not read VS Code's own Local chat files: the tray no longer starts
`VsCodeSource` and watches no folder under the VS Code user folder, so the window close case above cannot hold the
PC. Copilot harness chats in VS Code stay supported: they are Copilot CLI sessions of `copilot-runtime.exe`, held by
the Copilot source (rule 8). The source, its fixtures and its tests stay for a later version; revisit when VS Code
writes the end of a Local turn at a window close.

Checked by tests in `app/tray_mode.rs`, each on real files in a scratch profile, read by the watch's own `Agents` and
fed to the core: `a_local_vs_code_chat_holds_nothing_through_the_tray` (the live `close-while-working.jsonl` with a
`Code.exe` started before its send holds through `VsCodeSource`, and holds nothing through the tray's sources) and
`a_copilot_harness_chat_in_vs_code_holds_through_the_tray_until_its_stop` (the live harness log, cut at its first
`sleep 60`, holds through the tray with the runtime holding `inuse.2728.hold`, and the hold ends at its `abort` and
`assistant.turn_end` while the Local chat on disk still says working). Both fail when `VsCodeSource` is put back.
