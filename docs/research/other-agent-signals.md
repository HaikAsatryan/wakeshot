# Other agents: how to tell they are working

Research for Wakeshot, 2026-11-03. Claude Code is covered in `docs/research.md`.

Marks: **verified** = seen in source code or official docs (path given). **unverified** = read elsewhere, guessed,
or not checked. The research was done without running any agent; only the two live test sections (Codex and
Copilot CLI, 2026-11-06) report runs.

## Summary

| Agent | Best signal | Path (Windows) | Confidence | Testable without paying |
|---|---|---|---|---|
| Codex CLI | Last turn event in the rollout: `task_started` = working, `task_complete` / `turn_aborted` = idle. Plus `codex.exe` alive | `%USERPROFILE%\.codex\sessions\YYYY\MM\DD\rollout-*.jsonl` | High (source read). Approval wait looks like working | Yes (`--oss` + Ollama) |
| GitHub Copilot CLI | Last persisted event in `events.jsonl` (no idle event, must infer). `permission.requested` = waiting. `session.schedule_created` = future work | `%USERPROFILE%\.copilot\session-state\<id>\events.jsonl` | Medium (schema read, rule inferred, core is native code) | Unverified (needs Copilot plan; Free unclear) |
| Gemini CLI | None reliable. Transcript growth only, or hooks | `%USERPROFILE%\.gemini\tmp\<id>\chats\session-*.jsonl` | Low | No (enterprise or paid API only now) |
| OpenCode (2.x) | SQLite `session_v2`: `time_busy > time_idle` and not suspended. `schedule.next_due_at` for future work | `%USERPROFILE%\.local\share\opencode\opencode.db` | High (source read) | Yes (Ollama / LM Studio) |
| Cursor CLI | Only "is running": `.running\<pid>` markers. Working only by file activity, or hooks | `%LOCALAPPDATA%\cursor-agent\versions\<ver>\.running\<pid>` | High for alive, low for working | Unverified |
| Qwen Code | Already blocks sleep itself by default (`SystemRequired` while streaming or running tools). Turn markers in transcript, maybe only in ACP mode | `%USERPROFILE%\.qwen\projects\<cwd>\chats\<id>.jsonl` | Medium | Yes (local model) |
| Aider | None. `--notifications-command` fires when idle | `.aider.chat.history.md` in the repo | Low | Yes (local model, unverified) |
| Amp | None found (binary not readable). Much work runs in the cloud | n/a | None | Unverified |
| Kiro CLI | Hooks only, Windows support unclear | `~/.kiro/hooks/` | None | Unverified |

Who blocks sleep on its own (Windows): Codex (opt-in, `PowerRequestSystemRequired`, TUI only), Copilot CLI (opt-in
`/keep-alive`, `PowerRequestSystemRequired`), Qwen Code (on by default, `ES_SYSTEM_REQUIRED`). Gemini CLI, OpenCode
CLI, Cursor CLI, Aider: no. None of them touches the lid action or the battery sleep timer.

## OpenAI Codex CLI

Source: `openai/codex` main at commit `ba0dfd3` (2026-11-02). Paths below are under `codex-rs/` unless they start
with `codex-cli/`. License Apache-2.0 (`LICENSE`), verified.

### State on Windows

- **Home folder.** `%USERPROFILE%\.codex`, or `CODEX_HOME` if set. Verified: `utils/home-dir/src/lib.rs:14,59`.
- **Session transcripts ("rollouts").** `<home>\sessions\YYYY\MM\DD\rollout-YYYY-MM-DDTHH-MM-SS-<thread_id>.jsonl`.
  Archived ones move to `<home>\archived_sessions\`. A forked rollout is `rollout-<ts>-<thread_id>_<rollout_id>.jsonl`.
  Old files may be compressed to `.jsonl.zst`. The date folders are the session start date (time zone not
  checked). Verified: `rollout/src/recorder.rs:1723-1730`, `rollout/src/list.rs:424`,
  `rollout/src/lib.rs:86-87`,
  `rollout/src/rollout_file_name.rs:62-74`, `rollout/src/compression.rs:26`.
- **Line format.** One JSON object per line: `{"timestamp":"...","ordinal":N,"type":"<kind>","payload":{...}}`. Kinds
  are snake_case: `session_meta`, `response_item`, `turn_context`, `event_msg`, `compacted`, and more. Verified:
  `history/src/lib.rs:375-381` (`RolloutLine`), `history/src/rollout_payload.rs:32-73` (`tag = "type"`).
- **Working vs idle.** These `event_msg` payloads are written to the rollout (verified, `rollout/src/policy.rs:126-140`):
  - `{"type":"task_started","turn_id":"..."}` when a turn starts (`protocol/src/protocol.rs:1585`).
  - `{"type":"task_complete","turn_id":"...","last_agent_message":...}` when it ends (`protocol.rs:1594`).
  - `{"type":"turn_aborted","turn_id":...,"reason":"interrupted|replaced|review_ended|budget_limited"}`
    (`protocol.rs:1716,4572-4579`, `rename_all = "snake_case"`).
  - Readers must also accept `turn_started` and `turn_complete` (serde aliases, same lines).
  - Rule: scan the tail of the file. Last turn event is `task_started` = turn running. `task_complete` or
    `turn_aborted` = idle.
- **Waiting for the user.** Approval prompts (`exec_approval_request`, `apply_patch_approval_request`,
  `request_user_input`, `request_permissions`) are NOT written to the rollout. Verified: `rollout/src/policy.rs:161-203`.
  So "waiting for approval" looks the same as "working" (turn started, no end). Wakeshot cannot tell them apart from
  files.
- **Write timing.** Each batch of items is written and flushed right away once the file exists
  (`rollout/src/recorder.rs:1956-1958`, `1904`). A new session's file is created lazily, at the first persisted
  item (`deferred_creation`, `recorder.rs:997`). Verified.
- **File growth is not enough.** The built-in `sleep` tool (stable, on by default) lets the model wait up to 12 hours
  inside one turn, with no file writes. Verified: `core/src/tools/handlers/sleep.rs:28,51`,
  `features/src/lib.rs:1174-1177`. Use the last turn event, not the file size.
- **Thread lock files.** While a process has a thread open for writing it holds
  `<home>\thread-writer-locks\<thread_id>.lock` with an exclusive OS file lock, and deletes the file when done.
  Stale files are cleaned on the next start. Verified: `rollout/src/writer_lock.rs:19,54-80,108-120`;
  used for every local thread by `thread-store/src/local/live_writer.rs:152`. This means "thread loaded", not
  "working". Do not try to take the lock to test it: Codex treats a held lock as "already has an active writer"
  and would fail to open the thread.
- **Goals (auto-continue).** `goals` is stable and on by default. It keeps starting new turns until the goal is done.
  State in `<home>\goals_1.sqlite`, table `thread_goals`, column `status` in `active|paused|blocked|usage_limited|
  budget_limited|complete`. Verified: `features/src/lib.rs:2134-2137`, `state/src/sqlite.rs:49`,
  `state/goals_migrations/0001_thread_goals.sql`. A thread with `status='active'` whose lock file exists will very
  likely start another turn soon. Unverified in practice.
- **Not persisted at all.** `codex exec --ephemeral` writes no rollout. Verified: `exec/src/cli.rs:41`. Only the process
  is visible.

### Processes

- npm install: `node.exe` runs `codex.js`, which spawns the native `codex.exe` from `@openai/codex-win32-x64` (or
  `-arm64`) and waits. Verified: `codex-cli/bin/codex.js:22-23,93,247`. Count `codex.exe`.
- Turns can run in a shared background daemon (`codex app-server daemon start`), also `codex.exe`, used by the desktop
  and mobile apps over remote control. The TUI attaches to it when found. Verified: `app-server-daemon/README.md`.
  So the process that writes the rollout may not be the terminal the user sees.
- No pid is stored in the rollout file. Mapping session to process is not possible from files alone, only "some
  `codex.exe` is alive". Unverified that no other file maps pid to thread.

### Background and scheduled work

- Goals auto-continue (above). Visible through `goals_1.sqlite` plus `task_started` lines.
- App-server daemon runs turns with no terminal open (remote control from phone or desktop). Its rollouts are written
  to the same `sessions` folder. Verified by README; write path not traced.
- Cloud tasks (`cloud-tasks` crates) run on OpenAI servers, not on the PC. No need to keep the PC awake.

### Prevents sleep itself?

- Yes, opt in. Feature `prevent_idle_sleep`, stage Experimental on Windows, **default off**. Toggle in `/experimental`
  or `[features] prevent_idle_sleep = true` in `config.toml`. Verified: `features/src/lib.rs:2259-2276`.
- Uses `PowerCreateRequest` + `PowerSetRequest(PowerRequestSystemRequired)` with reason "Codex is running an active
  turn", held only while a turn runs, released when it ends. This is the correct type for Modern Standby. Verified:
  `utils/sleep-inhibitor/src/windows_inhibitor.rs:17,36,80-83`, `tui/src/chatwidget/turn_lifecycle.rs:35-48`.
- Only the TUI uses it. `codex exec` and the daemon do not. Verified by grep: only `tui/` calls `SleepInhibitor`.
- It does not change the lid action or the battery sleep timeout, so Wakeshot still adds value.

### Hooks (fallback)

- Claude-style hooks from `<home>\hooks.json` or `config.toml`. Events: `SessionStart`, `SessionEnd`,
  `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Stop`, `SubagentStart`, `SubagentStop`,
  `Interrupt`, `PreCompact`, `PostCompact`, `PreModelRequest`, `PostModelRequest`. Verified:
  `hooks/src/engine/dispatcher.rs:296-312`, `hooks/src/engine/discovery.rs:394`. Hooks go through a trust check
  (`discovery.rs`, `bypass_hook_trust`); exact user flow unverified.
- Legacy `notify = [...]` in `config.toml` runs a program with a JSON argument after each turn
  (`hooks/src/legacy_notify.rs`). Verified.

### Free testing

- Yes. `--oss` runs against a local Ollama or LM Studio model, no account. Verified: `utils/cli/src/shared_options.rs:26`,
  crates `ollama/`, `lmstudio/`. The rollout and turn events are the same code path (unverified by a run).

### Codex live test (2026-11-06)

Codex CLI 0.167.0 (npm) on the test laptop, a real ChatGPT login shared with the desktop app, model `gpt-6-luna` at
`low` effort, 7 one-line prompts. Every session ran at Medium in a pseudo console (a throwaway driver, not kept),
with `agent-watch --source codex` at Medium beside it and a change notice logger on
`sessions\` and `thread-writer-locks\`. Redacted rollouts: `crates/wakeshot-core/tests/fixtures/codex/live/`.

What holds (checked live):

- **Lock file.** The TUI makes `thread-writer-locks\<thread id>.lock` at start, before any prompt; `codex exec`
  makes it at start too. `/exit` and the end of `exec` delete it (posted `n=0` within 1 ms of the delete).
- **Holder.** Restart Manager named the TUI's own `codex.exe` (the child of npm's `node.exe`), with the pid and start
  time of that process, in both TUIs and in `exec`. No app-server daemon started in these runs, although
  `daemon_auto_start` is on (an earlier harness run saw two more `codex.exe` at a first TUI start; why is not
  known). The
  desktop app's two `codex.exe` (children of `ChatGPT.exe`) never held a CLI thread's lock.
- **Crash.** The TUI's `codex.exe` killed by pid during a turn: posted working `Gone` 115 ms later, so no hold. The
  lock file stays with no holder until the next Codex start, which deletes it (seen for a TUI start and an `exec`
  start). So the "can outlive its session" mark goes for the Restart Manager rule. It stays for the
  fallback when Restart Manager fails: any `codex.exe` counts then, and the desktop app keeps two running.
- **Turn events.** As the source rules say: `task_started`, `task_complete`, `turn_aborted` with `reason`
  `interrupted` after Esc. Every real line read with no format problem. An approval wait writes nothing, so it reads
  as working (held through a 60 s wait). An `item_completed` can come after `turn_aborted` (the command went on): it
  does not open the turn again.
- **Esc.** One Esc interrupts the turn, but a command run through unified exec goes on as a "background terminal"
  (`/ps`) after the abort; the rollout says idle while it runs. Not tracked by Wakeshot (not checked further).
- **Backtrack.** Esc Esc on an empty composer, left arrow, Enter: "Conversation reverted to this point". Codex
  deleted and made the lock file again twice in 0.8 s and wrote `rollout-<time>-<thread id>_<rollout id>.jsonl`; the
  next prompt's turn went there. The source followed it: the first new lock fed the old file again (the new one did
  not exist yet), the second one the new file. A brief idle `Gone` was posted between the two locks; no harm.
- **Write timing.** Codex appends to an open rollout, and each append gave a change notice in `sessions\` within
  milliseconds. A folder listing showed the open file's creation time as its last write time. Before the fix the
  source watched only the lock folder, so working and idle reached a post only at the 30 s tick: 1.3 to 29 s after
  the line, and a 12 s turn was never seen. After the fix (the source lists `sessions\` so the tree is watched):
  `task_started` to the working post 0.3 s, `task_complete` to the idle post 17 ms.
- **Through the app.** A dry-run tray with the Codex source logged "Holding starts: Codex" and `PowerSetRequest
  SYSTEM` 0.5 s after `task_started` of an `exec` turn, and "Holding ends; held by Codex" 17 ms after
  `task_complete`.

Not tested live: the app-server daemon as a holder; the desktop app's own threads (the app cannot be driven from the
command line, so whether it writes the same lock and rollout files is not known); goals (`goals_1.sqlite`);
`--oss`; a turn longer than a few minutes.

## GitHub Copilot CLI

Source: npm `@github/copilot` 1.0.105 and `@github/copilot-win32-x64` 1.0.105 (build `845c22879d4`). The Windows
package is one `copilot.exe` (Node single executable). Its JS bundle is a `copilot.tgz` asset inside the exe, cut
out with a small byte parser (nothing was run). Below, `pkg/` means that unpacked bundle. Most session logic is in
a Rust native addon `pkg/prebuilds/win32-x64/runtime.node` (source path strings point to
`src/runtime/src/session_base/locks.rs`), so some details come from strings only and are marked so.
`github/copilot-cli` on GitHub holds only README, changelog, license and a container sample.

License: proprietary "GitHub Copilot CLI License" (`LICENSE.md`): free to run, redistribution only inside a larger
product, no modification. Verified.

### State on Windows

- **Home folder.** `%USERPROFILE%\.copilot`, or `COPILOT_HOME`, or a `--config-dir`. Verified: `pkg/cli-main.js`,
  search `function gX(` and `q6n=".copilot"`.
- **Per session folder.** `<home>\session-state\<session-id>\`. Verified: `_w="session-state"` in `cli-main.js`,
  and SDK docs `pkg/copilot-sdk/docs/examples.md:443` ("e.g. ~/.copilot/session-state/<id>"). Holds:
  - `events.jsonl`: the persisted event log. Verified: `pkg/copilot-sdk/generated/rpc.d.ts:12105,12117`.
  - `workspace.yaml`: session metadata. Seen as a string in `runtime.node`; fields not checked.
  - An in-use lock file. Strings `inuse.` and `.lock` sit next to each other in `runtime.node`, so the name is very
    likely `inuse.<pid>.lock`. There is also "in-use handshake" file logic and "Failed to clean up dead lock file",
    so a lock whose process died is treated as dead. Name pattern and content: **unverified**.
  - `<home>\session-state\.session-operation-locks\` folder. Seen on the test laptop (empty) and in strings.
  - `<home>\session-store.db`: a session index database (string in `cli-main.js`, search `session-store.db`).
- **Event line format.** Each line: `{"id":uuid,"timestamp":iso,"parentId":uuid|null,"type":"...","data":{...},
  "agentId"?:..., "ephemeral"?:bool}`. Events with `ephemeral: true` are "transient and not persisted to the session
  event log on disk". Verified: `pkg/copilot-sdk/generated/session-events.d.ts:5526-5556`,
  `pkg/schemas/session-events.schema.json`.
- **Persisted events that matter** (verified from the schema, `ephemeral` not forced to true):
  - `user.message`: user sent a prompt.
  - `assistant.turn_start` / `assistant.turn_end` with `data.turnId`. One "turn" is one model call inside the agent
    loop ("typically a stringified turn number"), so a single prompt makes many start/end pairs.
  - `assistant.message` with `data.toolRequests?: [...]`. Tools follow when it is non-empty.
  - `tool.execution_start` / `tool.execution_complete`.
  - `permission.requested` / `permission.completed`: approval prompt shown and answered.
  - `abort` with `data.reason` (`user_initiated`, ...), `session.error`, `session.shutdown` (`data.shutdownType`
    `routine` ...), `session.start`, `session.resume`.
  - `subagent.started` / `subagent.completed` / `subagent.failed`.
  - `session.schedule_created` / `session.schedule_cancelled` / `session.schedule_rearmed` (see below).
- **Not persisted** (`ephemeral: true` const): `session.idle`, `assistant.idle`, `session.background_tasks_changed`,
  `user_input.requested`, `elicitation.requested`, `exit_plan_mode.requested`. Verified from the schema.
- **Working vs idle vs waiting, from `events.jsonl` alone.** There is no persisted "idle" event, so it must be
  inferred (rule designed from the schema, not tested on real logs):
  - Working: last relevant event is `user.message`, `assistant.turn_start`, `tool.execution_start`,
    `tool.execution_complete`, `permission.completed`, `subagent.started`, or an `assistant.message` with
    non-empty `toolRequests`.
  - Waiting for approval: last relevant event is `permission.requested` with no matching `permission.completed`.
  - Idle: last is `assistant.turn_end` after an `assistant.message` with no `toolRequests`, or `abort`,
    `session.error`, `session.shutdown`.
  - Questions to the user (`user_input.requested`) are ephemeral, so they look like "working".
- **Write timing: unverified.** The close path says it "flushes pending events", so writes may be buffered.
- The test laptop already had `%USERPROFILE%\.copilot\session-state\` (empty, 2026-11-03). VS Code ships a
  `copilot-shim` on PATH.

### Processes

- npm: `node.exe` runs `npm-loader.js`, which `spawnSync`s `copilot.exe` from `@github/copilot-win32-x64` and waits.
  Verified: `npm-loader.js`. WinGet: `winget install GitHub.Copilot` (README). Count `copilot.exe`.
- On first run `copilot.exe` unpacks its JS to `%LOCALAPPDATA%\copilot\pkg\win32-x64\<version>\`. Verified:
  `cli-main.js`/exe loader, search `function Wi(` and `COPILOT_SEA_EXTRACT_ONLY`.
- One process can hold several sessions ("backgrounded sessions" in the changelog), so process count is not session
  count. The lock file name, if it holds the pid, maps session to process (unverified).

### Background and scheduled work

- **Scheduled prompts.** `/every` and `/after` (and cron) inside a running session. Persisted as
  `session.schedule_created` with `data.id`, `at?`, `intervalMs?`, `cron?`, `tz?`, `recurring?`, `prompt`, and
  later `session.schedule_cancelled` / `session.schedule_rearmed`. Verified: `session-events.d.ts:2199-2232`,
  schema. Wakeshot can compute the next fire time from these. They only fire while the process lives.
- **Background agents and attached shells.** The session stays "not idle" while they run (`session.idle` is
  deferred). Only `subagent.*` is persisted; shells are not. Partly visible.
- **Autopilot** (`--autopilot`) keeps going without the user; looks like normal working.
- `/delegate` hands work to the GitHub cloud agent; nothing runs locally.

### Prevents sleep itself?

- Yes, opt in. `/keep-alive` (alias `/caffeinate`) with `on` (optional duration), `off`, `busy`. Setting
  `keepAlive: "on"|"off"|"busy"` in `~/.copilot/settings.json`; unset means off. Verified: `cli-main.js`, search
  `aliases:["/caffeinate"]` and `function DCn(`.
- `busy` mode starts on `user.message` and stops on `session.idle`. Verified: `cli-main.js`, search
  `function b5n(`.
- Windows uses a native addon (`cli-native.node`, `enableKeepAlive`). It imports `PowerCreateRequest`,
  `PowerSetRequest`, `PowerClearRequest`, and the only `PowerSetRequest` call site loads `mov edx, 1` just before
  the call, which is `PowerRequestSystemRequired`. Verified by reading the call site bytes at RVA `0xb217a`.
  Correct type for Modern Standby. No lid handling.

### Hooks (fallback)

- Hook names found in `runtime.node`: `sessionStart`, `sessionEnd`, `userPromptSubmitted`, `preToolUse`,
  `postToolUse`, `agentStop`, `subagentStart`, `subagentStop`, `errorOccurred`, `permissionRequest`, `preCompact`,
  `notification`. User config lives under `<home>\hooks` (string list in `cli-main.js`, search `m8o=[`). Exact file
  format not checked. Strings only: **unverified** shape.

### Free testing

- Needs a Copilot subscription per README ("An active Copilot subscription"). GitHub has a Copilot Free plan;
  whether Free can use the CLI: **unverified**. The changelog mentions local models in `providers.json`, but
  sign-in may still be required (unverified).

### Copilot CLI live test (2026-11-06)

Copilot CLI 1.0.106 (npm `@github/copilot`) on the test laptop, a real `gh` login on Copilot Free (it works: the CLI
takes `gh`'s login itself). Copilot Fusion (the default) picked `gpt-6-luna` each time; 7 one-line prompts in all,
one of them a schedule fire. Every session ran at Medium in a pseudo console (the throwaway driver), with
`agent-watch --source copilot --raw`, a
change notice logger on `session-state\` and a Restart Manager script at Medium beside it. Redacted logs:
`crates/wakeshot-core/tests/fixtures/copilot/live/`.

What holds (checked live):

- **In-use files.** At start, before any prompt, the process that opens a session makes the session folder,
  `inuse.<pid>.hold` (0 bytes) and `inuse.<pid>.lock` (as many bytes as the pid has digits, plus one: very likely
  the pid as text; content not read), named after its own pid. Restart Manager
  names that process as the holder of the `.hold`; the `.lock` has no holder. So the earlier guess (`inuse.<pid>
  .lock`, held) was half right: the name holds the pid, but the held file is the `.hold`. `/exit` writes
  `session.shutdown`, then deletes both files in the same millisecond. A crash leaves both, with no holder; the next
  Copilot start (any session) deletes them. The extracted package folder `%LOCALAPPDATA%\copilot\pkg\win32-x64\
  <version>\` gets the same pair.
- **Process tree.** VS Code's `copilot` shim comes first on PATH: shim `copilot.exe` > `cmd.exe` > `node.exe` (npm's
  loader) > `copilot.exe` (`@github\copilot-win32-x64\copilot.exe`, the CLI). The in-use files name the inner
  `copilot.exe`, and it is the holder. Killing the inner one ended the whole tree within 66 ms (the shim exits with
  it).
- **VS Code's Copilot harness** (sessions another test started through VS Code chat, never touched here): the same
  `session-state` folder, files `inuse.2728.*`, where 2728 is the `copilot-runtime.exe` of the user's VS Code (elevated;
  path `...\resources\app\node_modules.asar.unpacked\@github\copilot-sdk-win32-x64\prebuilds\win32-x64\`). Restart
  Manager asked from a Medium process named it, with its start time; the process list has it too. Their logs follow
  the same rules (`user.message`, a tool with `permission.requested` and `permission.completed` 47 ms apart,
  `assistant.turn_end`). This is research rule 8 of `copilot-vscode-signals.md`, now seen live.
- **Write timing.** Nothing reaches `events.jsonl` until the first prompt; then `session.start` (from the start) and
  the prompt's first lines come in one write. After that each event lands at once: the watch read lines 0.13 to
  0.25 s after their `timestamp`, each write with a change notice. Nothing is written while a command runs or an
  approval waits.
- **Turn events.** `user.message`, `assistant.turn_start`, `assistant.message` with `toolRequests`, then
  `tool.execution_start` **before** `permission.requested`; `permission.completed` after the answer;
  `tool.execution_complete`; an `assistant.turn_end` that tools follow; a last `assistant.message` with no tools and
  its `assistant.turn_end`. `ask_user`: `tool.execution_start` with `toolName` `ask_user`, then on the answer
  `human_response.recorded` and `tool.execution_complete`. Many kinds the source skips came along without a format
  problem (`model.*`, `hook.*`, `session.usage_*`, `instruction.update`, `system.message`, `session.model_change`,
  `session.auto_mode_resolved`).
- **Interrupt.** Esc did nothing while a shell command ran (the footer says "ctrl+c interrupt"). One Ctrl+C:
  `assistant.turn_end`, then `abort` with `reason` `user_initiated`; the TUI stays.
- **Schedules.** `/every` (alias `/loop`) is experimental: it needs `--experimental` (a flag for one run). `/every 2m
  <prompt>` made schedule 1 with no run at creation: `session.schedule_created` with `id` 1, `intervalMs` 120000,
  `recurring` true (no `at`, `cron` or `selfPaced`). It fired 2 min 0.12 s later as a `user.message` with `source`
  `schedule-1`, so the id follows `schedule-`. `/every` alone opens "Scheduled prompts"; `x` deletes one:
  `session.schedule_cancelled` with its `id`.
- **Shutdown.** `session.shutdown` has `shutdownType` `routine` and `totalPremiumRequests` (4 for this TUI: one per
  prompt or fire).

What the source made of it (before and after the fix):

- Activity and timers: right at every line (replay test `the_live_tui_session_moves_at_its_turn_events`); posts
  0.13 to 0.24 s after each line: working, waiting since the request's time (approval and `ask_user`), idle after
  the last turn end and after the abort; the timer at creation, moved to the next run at the fire, dropped at the
  cancel.
- **Liveness was wrong**: the source named the `.lock` file, which no process holds, so every live session was
  `Gone` and never held. Fixed: the source lists `.hold` and `.lock` files and names the `.hold`; the watch counts a
  Restart Manager holder whose pid the file name gives, whatever its image (so `copilot-runtime.exe` counts), and
  when Restart Manager fails, only that pid (not the shim or another session's `copilot.exe`). After the fix, at
  Medium: a new TUI session `alive=Yes` 0.5 s after its files appeared; VS Code's harness sessions `alive=Yes`; the
  stale 11-03 session `Gone`.
- **Crash.** The inner `copilot.exe` killed by pid during a 60 s command: working `Gone` posted 61 ms later (the
  holder's exit wait), so no hold. Its log ends inside the turn.
- **One-shot through npm's own launcher** (`%APPDATA%\npm\copilot.cmd -p ... -s --trust-folder --allow-tool
  "shell(Start-Sleep)"`, no shim; tree `cmd.exe` > `node.exe` > `copilot.exe` > `pwsh.exe` for the command): the same
  files and events as the TUI, with `COPILOT_HOME` on an empty scratch folder (Copilot made `session-state\` and its
  other files there, and `gh`'s login still worked). Exit 0 after `session.shutdown`; the in-use files were gone.
- **Through the app.** A dry-run tray (fixed build, at Medium, `COPILOT_HOME` on that folder, the other agents' homes
  on empty folders): "Holding starts: Copilot" with `dry run: PowerSetRequest SYSTEM` 0.25 s after `user.message`,
  "Holding ends; held by Copilot" 0.24 s after the last `assistant.turn_end`. On the first run the start came 26.7 s
  late, at the tray's 30 s tick: `session-state\` did not exist when the tray started, and the watch thread lists a
  missing folder only at the tick (the timing matches the tick; the cause is inferred, not traced).

Not tested live: rotation (`session.journal_link`; no log grew large enough), sub-agents, `/after`, cron and
self-paced schedules, `--resume`, VS Code Background sessions (`Code.exe` as holder), two clients on one session
(`alreadyInUse` in `session.start`), what Copilot does with an empty `COPILOT_HOME` value, `/keep-alive`.

## Google Gemini CLI

Source: `google-gemini/gemini-cli` main at commit `0ac57c0` (2026-11-02). Paths are under `packages/`. License
Apache-2.0 (`LICENSE`), verified.

**Product status, important.** The README now says Gemini CLI is for Gemini Code Assist Standard / Enterprise only.
"Free individual access and Google AI Pro / Ultra moved to Antigravity CLI (`agy`) on June 18, 2026." Verified:
`README.md:10-24`. So most individual users will run Antigravity CLI, not Gemini CLI. Antigravity CLI was not
researched here (closed source, unverified).

### State on Windows

- **Home.** `%USERPROFILE%\.gemini`, or `GEMINI_CLI_HOME`. Verified: `core/src/utils/paths.ts:13,22-28`,
  `core/src/config/storage.ts:54-60`.
- **Session transcripts.** `<home>\tmp\<project-short-id>\chats\session-<YYYY-MM-DDTHH-MM>-<first 8 chars of
  session id>.jsonl`. Subagents: `chats\<parent-session-id>\<session-id>.jsonl`. The short id comes from
  `<home>\projects.json`. Verified: `core/src/services/chatRecordingService.ts:780-823`,
  `core/src/config/storage.ts:195-198,230-234,286-300`, `core/src/services/chatRecordingTypes.ts:12`.
- **Line format.** Append-only JSONL (`fs.appendFileSync`, so each line is on disk at once; verified
  `chatRecordingService.ts:943-950`). Lines are one of: a message `{id, timestamp, type, content, ...}` with
  `type` in `user|info|error|warning|gemini`; a patch `{"$patch":{...}}`; a metadata update `{"$set":{...}}`; a rewind
  `{"$rewindTo":id}`. `gemini` messages can carry `toolCalls[]` with `status` in
  `validating|scheduled|executing|awaiting_approval|success|error|cancelled`. Verified:
  `chatRecordingTypes.ts:44-170`, `core/src/scheduler/types.ts:26-34`.
- **Working vs idle: no reliable file signal.** The model reply is written when its stream ends
  (`core/src/core/geminiChat.ts:1803-1818`), and tool calls are written only after they complete
  (`recordCompletedToolCalls`, `geminiChat.ts:1842-1871`). So while a tool runs or waits for approval, the last
  line is a `gemini` message that looks the same as a final answer. There is no turn-start or turn-end marker and
  no status file. Only "the file grew in the last N minutes" works, and long tool runs break it.
- **No pid or lock file** per session. Verified by grep for `process.pid` (only temp file names).
- **Window title.** With `ui.dynamicWindowTitle` (default true) the terminal title is `◇  Ready (<folder>)`,
  `✋  Action Required (...)`, `⏲  Working… (...)` or a thinking text. Verified:
  `cli/src/utils/windowTitle.ts`, `cli/src/config/settingsSchema.ts:579-584`. In Windows Terminal only the active
  tab's title reaches the window, so this is weak. Not tested.

### Processes

- `node.exe` running `bundle\gemini.js` (npm `@google/gemini-cli`, bin `gemini`). Verified: `package.json:92-94`.
  The CLI may relaunch itself as a child `node.exe` (`cli/src/utils/relaunch.ts:40-45`, off with
  `GEMINI_CLI_NO_RELAUNCH`). Telling it apart from other Node apps needs the command line of `node.exe`.

### Background and scheduled work

- Background shell commands (`tools.shell.backgroundCompletionBehavior`, `cli/src/config/settingsSchema.ts:1612`).
  Not visible in files.
- Background memory extraction from past sessions (`settingsSchema.ts:2439`). Light work, not a reason to stay awake.
- No scheduled prompts found. Verified by grep for schedule terms in `cli/src` settings (not exhaustive).

### Prevents sleep itself?

- No. No `PowerSetRequest`, `SetThreadExecutionState`, `caffeinate` or `systemd-inhibit` anywhere in `packages/`.
  Verified by grep.

### Hooks (fallback)

- Events: `BeforeAgent`, `AfterAgent`, `BeforeTool`, `AfterTool`, `BeforeModel`, `AfterModel`,
  `BeforeToolSelection`, `Notification`, `SessionStart`, `SessionEnd`, `PreCompress`. Verified:
  `core/src/hooks/types.ts:43-55`. `BeforeAgent` / `AfterAgent` map to busy / idle; `Notification` covers
  permission prompts (`ToolPermission`, `types.ts:565`).
- Config in `settings.json` `hooks` (user or project), or shipped inside an extension at
  `<extension>\hooks\hooks.json` (`cli/src/config/extension-manager.ts:1128`). An extension is a cleaner install
  than editing the user's settings, but it is still a hook.

### Free testing

- No free tier for Gemini CLI any more (README). A paid Gemini API key or Vertex still works ("legacy paid API /
  Vertex keys"). Verified: `README.md:14,24`.

## OpenCode

Newer research, 1.x and 2.x at their 2026-11-06 releases: `research/opencode-signals.md` (in 2.2.5
`time_suspended` is set while a turn runs, so the rule below no longer holds).

Source: `anomalyco/opencode` (the old `sst/opencode` redirects here) main at commit `2e8a861` (2026-11-02), CLI
package `@opencode/cli` 2.2.5. Paths are under `packages/`. License MIT (`LICENSE`), verified. Note: this is the new
2.x architecture. Users on 1.x have a different store (there is a `core/src/database/v1-migration.bun.ts`); 1.x
was not researched.

### Architecture that matters

- The `opencode` TUI is a client. Sessions run in a **background service** process (`opencode.exe serve --service`)
  that the TUI starts if missing and that outlives the TUI. Verified: `cli/src/commands/handlers/default.ts`
  ("Starting background server..."), `cli/src/services/service-config.ts:150`.
- Service registration file: `%USERPROFILE%\.local\state\opencode\service.json` (or `service-<channel>.json`), JSON
  `{id?, version?, url, pid, password?}`. Verified: `client/src/effect/service.ts:140-147`,
  `cli/src/services/service-config.ts:42-43,121`. It holds the service password, so treat it like Claude's `.key`
  files: do not read it. The HTTP API needs that password, so Wakeshot cannot use the API without the secret.

### State on Windows

- **Folders.** XDG style even on Windows: data `%USERPROFILE%\.local\share\opencode`, state
  `%USERPROFILE%\.local\state\opencode`, config `%USERPROFILE%\.config\opencode`. `XDG_*_HOME` override. Verified:
  `util/src/global-roots.ts:4-18`, `util/src/global.ts`.
- **Database.** `<data>\opencode.db` (SQLite, WAL, `busy_timeout` 5000). Name changes with `OPENCODE_DB` or a
  non-standard release channel (`opencode-<channel>.db`). Verified: `cli/src/database-path.ts`,
  `cli/src/server-process.ts:115`, `core/src/database/database.ts:40-42`.
- **Working vs idle: table `session_v2`.** Columns `time_busy`, `time_idle` (epoch ms), `idle_outcome`
  (`succeeded|failed|interrupted`), `time_suspended`, `parent_id`, `time_archived`. Verified:
  `core/src/database/schema.gen.ts:251-291`.
  - `time_busy` is set on event `session.execution.started` (`core/src/session/projector.ts:729-733`).
  - `time_idle` and `idle_outcome` are set on `session.execution.succeeded|failed|interrupted`, always moving
    forward by at least 1 ms (`projector.ts:505-527`).
  - An interrupt with reason `shutdown` does NOT set `time_idle`; the session gets `time_suspended` instead and is
    resumed when the service starts again (`projector.ts:514`, `core/src/session/store.ts:211-233`).
  - Rule: a session is working when `time_busy IS NOT NULL AND (time_idle IS NULL OR time_busy > time_idle) AND
    time_suspended IS NULL`, and the service pid is alive. Query with a read-only SQLite connection.
- **Waiting for the user.** Pending permission prompts live in memory only (`core/src/permission.ts:126-141,242`).
  The `permission` table holds saved rules, not open prompts. So waiting looks like working.
- **Event log.** Table `event` (`aggregate_id`, `seq`, `created`, `type`, `data`) holds the same events
  (`session.execution.started` etc., `core/src/session/message-updater.ts:145-149`). The `session_v2` columns are
  simpler to read.
- **Background jobs.** Table `job` with `status` in `running|completed|error|cancelled` (subagents, background
  tasks). Verified: `schema.gen.ts:72-86`, `core/src/job.ts:35`.

### Processes

- npm `@opencode/cli`: `node.exe` runs `bin/opencode.cjs`, which spawns `opencode.exe` from
  `@opencode/cli-windows-x64`. Verified: `cli/bin/opencode.cjs:12,36-42`. The service is another `opencode.exe`
  with arguments `serve --service`. Its pid is in `service.json` (secret file, see above), so prefer finding the
  process by name and command line.

### Background and scheduled work

- **Schedules, best "will work soon" signal of all agents.** Table `schedule`: `status` in `active|paused|done`,
  `next_due_at` (epoch ms), `ends_at`, `remaining_runs`, `session_id`, `when`, `tz`. Verified:
  `schema.gen.ts:136-160`, `schema/src/schedule.ts:114`, `core/src/schedule/sql.ts:23-43`. Fired by the service
  loop (`core/src/schedule.ts`). Wakeshot can read `min(next_due_at) WHERE status='active'` and stay awake (or not
  sleep) before it.
- **Goals** with status `active|paused|complete` (`schema/src/goal.ts:7`). Storage not traced.
- Suspended sessions resume when the service restarts (above).

### Prevents sleep itself?

- CLI and service: no. Verified by grep (no power APIs in `packages/`).
- The desktop app (Electron) has an opt-in "keep screen active" for remote pairing:
  `powerSaveBlocker.start("prevent-display-sleep")`, default false. Verified:
  `gui-plugins/src/pairing/main.ts:15-35`. That keeps the screen on too, and only while the desktop app runs.

### Hooks (fallback)

- Plugins (JS) can subscribe to bus events. Not researched in detail. The DB signal makes hooks unneeded.

### Free testing

- Yes. Local providers exist: `ollama.ts`, `lmstudio.ts`, `vllm.ts`, `local.ts` in `core/src/plugin/provider/`.
  Verified that the files exist; a full run was not tried.

## Cursor CLI

Source: Windows install script `https://cursor.com/install?win32=true` (version `2026.10.22-27dcf5f`) and its
package `https://downloads.cursor.com/lab/2026.10.22-27dcf5f/windows/x64/agent-cli-package.zip`, unpacked as data
(`dist-package/`, minified webpack JS). Closed source, proprietary license (no license file in the package).
Below, `cursor/` means `dist-package/`.

### State on Windows

- **Install.** `%LOCALAPPDATA%\cursor-agent\` with `cursor-agent.cmd`, `agent.cmd` on PATH and
  `versions\<YYYY.MM.DD-commit>\` holding `node.exe`, `index.js` and native modules. Verified: install script
  lines 283-300, 240-252.
- **Liveness markers.** Every running agent writes `%LOCALAPPDATA%\cursor-agent\versions\<ver>\.running\<pid>`.
  File body `<image name>|<process creation time>` (the WMI `CreationDate` string). A marker counts as live only if
  that pid is alive with the same image and start time; image must be `node.exe` or `cursor-agent.exe`. Verified:
  install script `Test-VersionInUse` (lines 28-49) and `cursor/index.js` module
  `./src/utils/install-in-use-marker.ts`. This shows "a Cursor agent process is running", not "working".
- **Config home.** `%USERPROFILE%\.cursor`, or `%XDG_CONFIG_HOME%\cursor`, or `CURSOR_CONFIG_DIR`. Verified:
  install script lines 146-148.
- **Chats.** `<home>\chats\<md5 of cwd>\<chat-id>\store.db` (SQLite key/blob store, keys like `agentId`, `name`,
  `createdAt`, `latestRootBlobId`) plus a meta sidecar with `schemaVersion`, `title`, `createdAtMs`,
  `updatedAtMs`, `hasConversation`, `isSubagent`. Verified: `cursor/1398.index.js` module `./src/state/index.ts`,
  `cursor/4525.index.js` module `./src/state/chat-session-sidecar.ts`. No busy/idle field.
- **Transcripts and terminals.** `%USERPROFILE%\.cursor\projects\<cwd with non-alphanumerics turned into ->\`
  holds `agent-transcripts\` (primary transcript `.jsonl` per conversation) and `terminals\<shellId>.txt` (output
  of shells the agent runs). Verified: `cursor/index.js` module `../utils/dist/workspace-paths.js`,
  `cursor/1317.index.js`. Transcript writes use `writeFile` (whole file); when during a turn: **unverified**.
- **Working vs idle: no status field found.** Searched for status files, heartbeats and `busy` fields. Only
  activity proxies remain: `store.db` / `store.db-wal` mtime and transcript or terminal file growth. Weak.

### Processes

- `cursor-agent.cmd` -> `powershell.exe` -> `%LOCALAPPDATA%\cursor-agent\versions\<ver>\node.exe index.js`.
  Verified: `cursor/cursor-agent.cmd`, `cursor/cursor-agent.ps1`. The image path is unique, so match `node.exe`
  by full image path, no command line needed. Combine with the `.running\<pid>` markers.

### Background and scheduled work

- Background shells tracked in a `backgroundWorkRegistry` with output in `terminals\` (`cursor/index.js`). No
  scheduler found. Cloud agents run on Cursor servers.

### Prevents sleep itself?

- No. No power API, `caffeinate`, `systemd-inhibit` or `powerSaveBlocker` in the package. Verified by grep.

### Hooks (fallback)

- `%USERPROFILE%\.cursor\hooks.json` (and project `.cursor\hooks.json`), `{"version":1,"hooks":{...}}`. Event
  names in the CLI bundle: `sessionStart`, `sessionEnd`, `beforeSubmitPrompt`, `stop`, `afterAgentResponse`,
  `afterAgentThought`, `preToolUse`, `postToolUse`, `postToolUseFailure`, `beforeShellExecution`,
  `afterShellExecution`, `beforeMCPExecution`, `afterMCPExecution`, `beforeReadFile`, `afterFileEdit`,
  `subagentStart`, `subagentStop`, `preCompact`, `notification`. Verified as strings in `cursor/*.js`;
  which ones the CLI fires (vs only the IDE): **unverified**. `beforeSubmitPrompt` + `stop` would give busy/idle.

### Free testing

- Cursor has a free Hobby plan; whether it covers CLI agent runs: **unverified**.

## Aider

Source: `Aider-AI/aider` main at commit `5dc9490` (2026-05-22, last push in May: slow project now). Python.
License Apache-2.0, verified.

- **State.** Only plain files in the repo root (git root or cwd): `.aider.chat.history.md` (appended per message
  with `open("a")`), `.aider.input.history`, optional `.aider.llm.history`. Verified: `aider/args.py:271-280`,
  `aider/io.py:1117-1136`. No session id, no pid, no status. While a shell command or a long model call runs,
  nothing is written. **No reliable signal**, only "a file grew recently".
- **Waiting for the user.** `--notifications` / `--notifications-command <cmd>` (also settable in
  `.aider.conf.yml`) runs a command when a reply is done and Aider waits for input. Verified:
  `aider/args.py:825-840`, `aider/io.py:1090-1103`. This is the closest thing to an idle hook. No busy hook.
- **Process.** `aider.exe` (pip / uv console-script launcher, entry `aider.main:main`, `pyproject.toml:26-27`)
  with a child `python.exe`. Launcher behavior on Windows: unverified.
- **Sleep prevention.** None (grep for power APIs found nothing). Verified.
- **Background or scheduled work.** None found.
- **Free testing.** Yes with a local model (Aider supports Ollama; from memory, unverified here).

## Amp

Source: npm `@sourcegraph/amp` (now only a pointer: "Renamed to @ampcode/cli") and `@ampcode/cli-win32-x64`
0.0.1793649628, which is one Bun-compiled `amp.exe` (128 MB). License: "© Sourcegraph Inc. All rights reserved",
proprietary. Verified: package files.

- **State.** The JS inside `amp.exe` is not stored as plain text (only Bun's own sources showed up in a string
  search), so local file layout was **not found**. The Amp manual says threads are "the same agent and the same
  threads everywhere" and that cloud machines ("Orbs") keep working "when you close your laptop"
  (ampcode.com/manual, read 2026-11-03). Much Amp work runs in the cloud, where the PC does not need to stay awake.
- **Process.** `amp.exe` (npm bin `bin/amp.exe`). Verified: `@ampcode/cli` `package.json`.
- **Sleep prevention.** No `SetThreadExecutionState`, `PowerSetRequest` or `caffeinate` strings in `amp.exe`.
  Verified by string search (weak evidence for a compiled binary).
- **Hooks.** The manual links a "Plugin API"; not read. Unverified.
- **Free testing.** Unverified.

## Qwen Code

Source: `QwenLM/qwen-code` main at commit `0441faf` (2026-11-03). Started as a Gemini CLI fork, now far apart.
Paths under `packages/`. License Apache-2.0, verified.

### Prevents sleep itself? Yes, on by default

- Setting `general.preventSystemSleep`, **default true**: "Prevent the system from sleeping while Qwen Code is
  streaming a model response or executing tools. Idle prompt time and permission prompts do not inhibit sleep."
  Verified: `cli/src/config/settingsSchema.ts:731-741`.
- On Windows it spawns a hidden `powershell.exe` that calls `SetThreadExecutionState(0x80000001)`
  (`ES_CONTINUOUS | ES_SYSTEM_REQUIRED`) and sleeps in a loop; killing the child releases it. Verified:
  `core/src/services/sleepInhibitor.ts:327-360`. `ES_SYSTEM_REQUIRED` should block idle sleep on Modern Standby
  (same effect as `PowerRequestSystemRequired`; not tested here).
- Taken per tool execution (`core/src/core/coreToolScheduler.ts:6139`) and per ACP prompt
  (`cli/src/acp-integration/session/Session.ts:18388`). No lid handling, no battery timer change.

### State on Windows

- **Home.** `%USERPROFILE%\.qwen`, or `QWEN_HOME`; runtime dir can move with `QWEN_RUNTIME_DIR`. Verified:
  `core/src/utils/paths.ts:14`, `core/src/config/storage.ts:169-200`.
- **Transcripts.** `<runtime>\projects\<sanitized cwd>\chats\<sessionId>.jsonl`, Claude-like records
  `{uuid, parentUuid, sessionId, timestamp, type: user|assistant|tool_result|system, subtype?, ...}`. Verified:
  `core/src/services/chatRecordingService.ts:380-430,1412-1430`, `core/src/config/storage.ts:619-623`.
- **Turn markers.** `system` records with `subtype: 'turn_attempt'` (turn start) and `subtype: 'turn_result'`
  (settled outcome) exist. Verified: `chatRecordingService.ts:4277-4330`. They are written from the ACP session
  path (`cli/src/acp-integration/session/Session.ts:6328,7018,10460`). Whether the normal terminal UI writes
  them too: **unverified**. If it does, "last `turn_attempt` newer than last `turn_result`" means working.
- **Scheduled tasks.** Durable cron jobs in `<runtime>\tmp\<project-hash>\scheduled_tasks.json`, items
  `{id, cron, prompt, recurring, createdAt, lastFiredAt, ...}`. In-memory jobs die with the process; durable ones
  survive restarts. Verified: `core/src/services/cronTasksFile.ts:132-144,345-348`,
  `core/src/services/cronScheduler.ts:1-5`. Wakeshot would have to evaluate the cron expression to get the next
  fire time.

### Processes

- `node.exe` running the `qwen` bin (`dist/index.js` or `scripts/cli-entry.js`). Needs a command line check.
  Verified: `package.json:164-166`. Plus a child `powershell.exe` while it holds the sleep block.

### Hooks (fallback)

- Inherits a hooks system from Gemini CLI (not re-checked for this fork).

### Free testing

- Yes with a local model: README says it supports "Any third-party provider or local model (Ollama / vLLM)".
  Verified: `README.md:28`. A free Qwen OAuth tier: **unverified** (not in the current README).

## Kiro CLI

Closed source. It replaced the open-source Amazon Q Developer CLI (`aws/amazon-q-developer-cli` README: "now
available as Kiro CLI, a closed-source product"). Verified: that README.

- **State.** Not found for Kiro. The old Q CLI kept conversations in SQLite
  `<data dir>\amazon-q\data.sqlite3`, table `conversations`, with no status column
  (`crates/agent/src/agent/util/directories.rs:16,47-48`, `crates/chat-cli/src/database/mod.rs:171`). Kiro may
  have kept this design: **unverified**.
- **Hooks.** Agent config hooks such as `agentSpawn`, `userPromptSubmit`, `preToolUse`, `postToolUse`, `stop`
  (kiro.dev docs via search, not read in full), and hook files in `~/.kiro/hooks/` (global, CLI v3) or
  `.kiro/hooks/` (kiro.dev/docs/hooks, read 2026-11-03). Event list **unverified**.
- **Windows.** Native Windows support not confirmed (one third-party note says hooks go through WSL).
  **Unverified.**
- **Sleep prevention, processes, free tier.** Unverified.

## Not covered: Antigravity CLI

Google moved free and Google AI Pro users from Gemini CLI to Antigravity CLI (`agy`) on 2026-06-18 (Gemini CLI
`README.md:14`). Third-party pages say it is a compiled Go binary installed under `%LOCALAPPDATA%\Antigravity\`
with data in `~/.gemini/antigravity-cli/` (conversations, history, logs). None of this is verified. It probably
deserves its own look before Gemini CLI does. Researched since: `antigravity-signals.md` (2026-11-06).

## Design notes for Wakeshot

Suggestions from the findings above, written before the design. The design is `architecture.md` section 7.

- **One adapter per agent**, each returning `Working | Waiting | Idle | Unknown` and an optional "next scheduled
  run at". Unknown file shape = `Unknown` = no signal, same rule as for Claude Code.
- **Always pair file state with process liveness.** A crash leaves `task_started` (Codex) or `time_busy > time_idle`
  (OpenCode) behind forever. Codex files hold no pid, so the check is "some `codex.exe` is alive and the thread
  lock file `thread-writer-locks\<id>.lock` exists". OpenCode: an `opencode.exe` whose command line has
  `serve --service`. Cursor: the `.running\<pid>` marker plus pid and start time.
- **Process checks without admin.** Image path of a same-user process: `QueryFullProcessImageNameW`. Command line
  (needed for `node.exe`-based agents: Gemini, Qwen, OpenCode service): `NtQueryInformationProcess` with
  `ProcessCommandLineInformation`. From memory, works for same-user processes without elevation: unverified.
- **Waiting counts as working** for Codex, OpenCode and most others, because approval prompts are not on disk.
  Only Copilot persists `permission.requested`. The product rule (`product.md`, "Detailed rules"): Codex's wait holds,
  Copilot's `permission.requested` does not.
- **Long silent turns.** Codex's `sleep` tool can wait up to 12 hours inside one turn with no writes. Any "file
  grew in the last N minutes" rule ends too early there. Prefer turn markers.
- **Scheduled work** is visible for OpenCode (`schedule.next_due_at`, epoch ms), Copilot (`session.schedule_*`
  events, process must stay alive), Qwen (`scheduled_tasks.json`, cron strings) and Codex goals
  (`goals_1.sqlite`). This allows "stay awake until the next run" or "do not sleep 5 minutes before it".
- **Never read secret files.** OpenCode `service.json` holds the service password (and the HTTP API needs it).
  Treat it like Claude's `.key` files. Copilot's lock and handshake file contents are unknown: read names only.
- **SQLite reads** (OpenCode, Codex goals): open read-only, set a busy timeout, never write, expect WAL mode.
- **JSONL tail reads** (Codex, Copilot, Qwen): read the last 64 to 256 KB, drop a partial last line, scan backwards
  for the last relevant event. Skip `.jsonl.zst` (Codex compresses only old files).
- **Agents that block sleep themselves** still need Wakeshot for lid close and the battery timer. Wakeshot cannot
  see their power requests without admin (`powercfg /requests`, see `docs/research.md`).

### Fixture sketches

Built from the schemas above, not copied from real logs. Field order and extra fields will differ in real files.

Codex rollout, turn running:

```jsonl
{"timestamp":"2026-11-03T10:00:00.000Z","type":"session_meta","payload":{"id":"0199aaaa-0000-7000-8000-000000000001","cwd":"C:\\code\\demo"}}
{"timestamp":"2026-11-03T10:00:05.000Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"turn-1"}}
{"timestamp":"2026-11-03T10:00:09.000Z","ordinal":2,"type":"response_item","payload":{"type":"message","role":"assistant","content":[]}}
```

Codex idle: append `{"timestamp":"...","type":"event_msg","payload":{"type":"task_complete","turn_id":"turn-1","last_agent_message":"Done"}}`
or `{"timestamp":"...","type":"event_msg","payload":{"type":"turn_aborted","turn_id":"turn-1","reason":"interrupted"}}`.

Copilot `events.jsonl`, waiting for approval:

```jsonl
{"id":"e1","timestamp":"2026-11-03T10:00:00.000Z","parentId":null,"type":"session.start","data":{}}
{"id":"e2","timestamp":"2026-11-03T10:00:01.000Z","parentId":"e1","type":"user.message","data":{"content":"fix the build"}}
{"id":"e3","timestamp":"2026-11-03T10:00:02.000Z","parentId":"e2","type":"assistant.turn_start","data":{"turnId":"0"}}
{"id":"e4","timestamp":"2026-11-03T10:00:04.000Z","parentId":"e3","type":"assistant.message","data":{"messageId":"m1","content":"","toolRequests":[{"toolCallId":"c1","name":"powershell"}]}}
{"id":"e5","timestamp":"2026-11-03T10:00:04.100Z","parentId":"e4","type":"assistant.turn_end","data":{"turnId":"0"}}
{"id":"e6","timestamp":"2026-11-03T10:00:04.200Z","parentId":"e5","type":"permission.requested","data":{"permissionRequest":{}}}
```

Copilot idle: last lines are an `assistant.message` with no `toolRequests` and its `assistant.turn_end`.
(`toolRequests[]` items have required `toolCallId` and `name`; `permission.requested` has required
`data.permissionRequest`, whose inner shape was not checked. `session-events.d.ts:6519-6555,9481-9498`.)

OpenCode `session_v2` (only the columns Wakeshot reads):

```sql
CREATE TABLE session_v2 (id TEXT PRIMARY KEY, parent_id TEXT, time_busy INTEGER, time_idle INTEGER,
  idle_outcome TEXT, time_suspended INTEGER, time_archived INTEGER);
INSERT INTO session_v2 VALUES ('ses_working', NULL, 1793700000000, 1793690000000, 'succeeded', NULL, NULL);
INSERT INTO session_v2 VALUES ('ses_idle',    NULL, 1793690000000, 1793690005000, 'succeeded', NULL, NULL);
INSERT INTO session_v2 VALUES ('ses_never',   NULL, NULL, NULL, NULL, NULL, NULL);
INSERT INTO session_v2 VALUES ('ses_susp',    NULL, 1793700000000, NULL, NULL, 1793700100000, NULL);
CREATE TABLE schedule (id TEXT PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active', next_due_at INTEGER);
INSERT INTO schedule VALUES ('sch_1', 'active', 1793703600000);
```

### What is left

- Codex and Copilot CLI: checked live (the two live test sections). OpenCode 1.x and 2.x: `opencode-signals.md`.
  Antigravity CLI and IDE: `antigravity-signals.md`. Cursor's hooks: `cursor-gemini-signals.md` and
  `../agent-hooks.md`.
- Not checked: whether Qwen's normal terminal UI writes `turn_attempt` / `turn_result`; when the Cursor CLI writes
  its transcripts; Amp's local state; Kiro CLI on Windows.
