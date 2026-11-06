# Antigravity CLI and IDE: how to tell they are working

Research for Wakeshot, 2026-11-06 (card `p4-antigravity-research`). Builds on `cursor-gemini-signals.md` and the
shape of `copilot-vscode-signals.md`.

Marks: **verified** = seen in official docs, a shipped bundle or a shipped binary (URL or path given). **claim** = what
Google says about its own product. **not verified** = inferred, guessed or not checked. Nothing was installed, run or
signed in. Downloads were kept as data in the card's scratch folder (the CLI binary renamed to `.exe.bin`) and never
run. On the laptop only folder names were checked: Antigravity is not installed there. All pages read 2026-11-06.

## Summary

| Surface | Agent process | Best signal | Confidence | Needs hooks | Free test |
|---|---|---|---|---|---|
| Antigravity CLI 1.5.2 (`agy`) | `agy.exe` in a terminal | Hooks from a Wakeshot plugin folder: `PreInvocation` = working, `Stop` = turn over; Esc and Ctrl+C fire no hook, `agy`'s own log has a cancel line for them ("Exact end" below). `fullyIdle` is not reliable | High (live, 2026-11-06) | Yes | Yes (Free plan) |
| Antigravity IDE 2.6.0 | `agy.exe --hub`, child of the IDE (the same binary) | The same hooks, from the same plugin folder | Medium (same engine; hub mode not run) | Yes | Yes (Free plan) |
| Without hooks | | `transcript.jsonl` per conversation, one line per step | Low (write timing not known) | No | |

Short answer to the goal: **yes, with hooks.** The CLI and the IDE run one agent engine (the IDE starts the CLI binary
in hub mode), and both load hooks from `~/.gemini/config/plugins/<plugin>/hooks.json`, a folder Wakeshot can own. No
passive file is known to show a running turn reliably. Waiting for approval cannot be told apart from a running tool.

A free live test exists: the Free plan for a personal Google account (claim), country Armenia is listed, age 18+.

Who blocks sleep: the CLI takes no power request (no power API name in the binary). The IDE's main process takes an
Electron `prevent-app-suspension` blocker while any conversation runs, which is `PowerRequestExecutionRequired` on
Windows and does not stop idle sleep on Modern Standby. Nothing touches the lid action or the battery timer.

## What it is and who can use it

### Surfaces and versions

Google ships four local surfaces on one engine: the Antigravity App (desktop, "Antigravity 2.0", the "Hub"), the
Antigravity CLI (`agy`), the standalone Antigravity IDE, and IDE extensions (VS Code, JetBrains, Visual Studio, Zed,
Xcode, Eclipse); plus a Python SDK. Verified: `antigravity.google/docs/getting-started`. "Both interfaces run on the
same core agent engine" (claim, `github.com/google-antigravity/antigravity-cli` `README.md`).

| Item | Value | Source | Mark |
|---|---|---|---|
| CLI version | 1.5.2 for `windows_amd64` and `windows_arm64` | `antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/windows_amd64.json` | verified |
| CLI binary | One native exe, 222,561,944 bytes, Go (`go1.28-20261102-RC00` in the build strings), x64 console subsystem | the binary from the manifest URL, SHA-512 matches the manifest | verified |
| CLI install path | `%LOCALAPPDATA%\agy\bin\agy.exe`; staging in `%LOCALAPPDATA%\antigravity\staging\`; then the script runs `agy.exe install` (PATH setup) | `antigravity.google/cli/install.ps1` read as text | verified |
| CLI install path, other doc | `C:\Program Files\Google\antigravity-cli` in the PATH fix of the troubleshooting page | `docs/cli/troubleshooting` | verified that the page says it; it contradicts the script, so the script wins |
| CLI updates | Self-updater in the binary; lock `~/.gemini/antigravity-cli/updater/update.lock`, marker `last_check.timestamp`; `AGY_CLI_DISABLE_AUTO_UPDATE=true` turns it off | `docs/cli/troubleshooting` | verified (docs) |
| CLI source | Not open. The public repo holds only `README.md`, `CHANGELOG.md` (86 versions, 1.0.0 to 1.5.2), a demo GIF and status line examples; no license file | `gh api repos/google-antigravity/antigravity-cli` | verified |
| IDE version | 2.6.0, VS Code base 1.107.0, commit `9e3a5da5d673`, built 2026-10-26 | `resources/app/product.json` in the Linux x64 tarball from `antigravity.google/download` | verified |
| IDE names | `nameShort` "Antigravity IDE", `dataFolderName` `.antigravity-ide`, `win32AppUserModelId` `Google.AntigravityIDE`, mutex `antigravity-ide` | same `product.json` | verified |
| IDE on Windows | `Antigravity IDE.exe`, user data `%APPDATA%\Antigravity IDE`, extensions `%USERPROFILE%\.antigravity-ide\extensions` | VS Code conventions applied to `product.json` | not verified (no Windows install read) |
| App version | 2.29.0 (`antigravity-hub` on the download page) | `antigravity.google/download` | verified |
| Usage | 6% of developers use Antigravity (the editor) at work, January 2026 (JetBrains AI Pulse); the CLI repo has 2,740 stars and 859 open issues | `cursor-gemini-signals.md` (survey); `gh api`, 2026-11-06 | verified |

The IDE is a VS Code fork, so some `copilot-vscode-signals.md` rules carry over in form (user data folder, `code.lock`
holding the main pid), but its agent is not VS Code chat: the chat `.jsonl` files of that research do not apply.

### Plans and limits

From `docs/plans`, `docs/models`, `docs/faq` and `docs/install` (the "Authenticate" page), all claims of Google:

- **Free plan exists.** "Personal Google Accounts (such as `@gmail.com` addresses) can use Google Antigravity on any
  plan, including the free tier." Users not on Google AI Pro or Ultra get "a meaningful quota, refreshed weekly" and
  "a weekly rate limit". No number is given; limits "correlate with the amount of work done by the agent".
- **Models on Free (and Google AI Plus):** Gemini 3.8 Flash; also Claude Sonnet 4.6, Claude Opus 4.6 and GPT-OSS 120B
  until they are "removed on November 18, 2026". Gemini 4 Argon needs paid Pro or Ultra.
- **All plans get** "Access to all product features, such as scheduled tasks and the CLI". `/teamwork` is for paid
  plans (`docs/cli/reference`).
- **Who may sign in:** a personal Google account, age 18 or over (age may need verifying), in a listed country.
  Armenia is in the list (Europe section of `docs/faq`). Workspace accounts need their admin.
- **Card or phone:** no page asks for one for Free. Not verified (no sign-in was tried; Google may ask at sign-in).
- **Terms:** Google Terms of Service and Privacy Policy; "By using Antigravity CLI, you agree to help improve the
  product by allowing Google to collect and use your Interactions data", opt-out in settings (`README.md`).
- **Other way in:** a Gemini API key (`modelProvider: "gemini"` in `~/.gemini/antigravity-cli/settings.json` plus
  `GEMINI_API_KEY`), for headless runs. Whether a free AI Studio key covers Gemini 3.8 Flash: not verified.

## Hooks

The one documented, stable way to learn turn state. Verified from `docs/hooks`, `docs/plugins`, `docs/build-plugins`,
`docs/customization-discovery`, `docs/settings` and the CLI `CHANGELOG.md`. Not run.

### Where hooks load from

- Global `~/.gemini/config/hooks.json`; workspace `<repo>/.agents/hooks.json` (loads only after the user trusts the
  folder, changelog "workspace-local hooks ... not loading after trusting a folder"); and each enabled plugin's
  `<plugin-root>/hooks.json`.
- Hooks in `~/.gemini/antigravity-cli/settings.json` are not loaded ("the CLI keeps a `hooks` key there but never
  loads it", `docs/settings`).
- A plugin's `hooks/hooks.json` is **not read**, and a Claude Code style top-level `"hooks"` key loads as one hook
  named `hooks` (changelog, `agy plugin validate` entry). So the file is `<plugin-root>/hooks.json`, not Cursor's or
  Gemini CLI's layout.
- The same files serve the App, the CLI and the IDE (each docs page lists all three; the IDE and the App read
  `~/.gemini/config/` too).

### A folder Wakeshot owns: a local plugin

- Plugins live in `~/.gemini/config/plugins/<plugin-folder>/` (personal) or `.agents/plugins/` (workspace). The scan
  is one level deep. Each plugin needs `plugin.json` in its folder (or in `.antigravity-plugin/`). Verified:
  `docs/customization-discovery` ("Plugins: `plugins/<plugin-folder>/`"), `docs/plugins` ("Manual plugin
  installation: ... placing their directories in ... `~/.gemini/config/plugins/`", stated for the App and the IDE).
- So `%USERPROFILE%\.gemini\config\plugins\wakeshot\` with `plugin.json` and `hooks.json` is a Wakeshot-owned folder,
  and no user file is edited. That the CLI picks up a dropped folder without `agy plugin install`: stated by the
  discovery page for "Antigravity", not for the CLI by name; **not verified**.
- Supported install and removal: `agy plugin install <local path>` and `agy plugin uninstall <name>` ("Remove the
  plugin files and clean up configuration registries"). Where enabled or disabled state is kept: not found. Removal by
  deleting the folder: **not verified**.
- Plugin name rules (for the marketplace, `docs/build-plugins`): lowercase kebab-case, not starting with
  "Antigravity", "Gemini", "Claude" or "Cursor". `wakeshot` fits.
- A disabled plugin does not run its hooks (changelog "Fixed disabled plugins still running their hooks"). The user
  sees the plugin in `/plugin`, `/hooks` and the App's Customizations, and can turn it off there.
- Plugin hook commands run with the plugin root as working directory and "should use relative paths (such as
  `node ./scripts/policy.mjs`)" (`docs/build-plugins`).

### File format

```json
{
  "wakeshot": {
    "PreInvocation": [ { "type": "command", "command": "<helper command>", "timeout": 5 } ],
    "Stop": [ { "type": "command", "command": "<helper command>", "timeout": 5 } ]
  }
}
```

Top-level keys are named hook groups with an optional `"enabled"`. `PreToolUse` and `PostToolUse` take
`{"matcher": "<regex>", "hooks": [...]}`; `SessionStart`, `PreInvocation`, `PostInvocation` and `Stop` take the
handler list directly. `type` is `command` (default) or a prompt hook; `timeout` in seconds, default 30. Verified:
`docs/hooks`, `docs/build-plugins`.

### Events and inputs

| Event | Fires | Event fields on stdin | Use for Wakeshot |
|---|---|---|---|
| `SessionStart` | Once when a conversation starts, before the first model call | common only | Record the conversation and the agent pid |
| `PreInvocation` | Before every model call (`invocationNum` 0-based) | `invocationNum`, `initialNumSteps` | Working. The only "turn started" event; there is no prompt-submit event |
| `PostInvocation` | After every model call | same | Not needed |
| `PreToolUse` / `PostToolUse` | Around each tool | `toolCall` (`name`, `args`), `stepIdx`, `error` | Not needed (cost); a long tool and an approval wait look the same |
| `Stop` | "when the execution loop terminates" | `executionNum`, `terminationReason` (`model_stop`, `max_steps_exceeded`, `error`), `error`, `fullyIdle` | Turn over. `fullyIdle: false` = "active background tasks are still running" |

Common fields on every event: `conversationId` (UUID), `workspacePaths`, `transcriptPath`, `artifactDirectoryPath`,
`modelName`, `lastUserInput`. Verified: `docs/hooks`.

- **Session id:** `conversationId` on stdin. No separate session id. `/fork` makes a new conversation id
  (`docs/cli/conversations`). Whether the hook also gets an environment variable with it: not documented.
- **Agent pid:** not in the input. The helper must find it by walking up its parent processes to `agy.exe`. Whether
  the hook's parent is `agy.exe` or a shell under it: **not verified**.
- **No end event.** There is no `SessionEnd`. Closing the CLI or the IDE fires nothing, so hook state must be paired
  with the agent process's life.
- **No waiting event.** No permission or notification event exists. An approval wait sits between `PreToolUse` and
  `PostToolUse`, exactly like a running tool, so it reads as working.
- **Cancel:** whether Ctrl+C or Esc fires `Stop`: not documented. **Not verified.**
- `/btw` side questions do not run user and workspace hooks (changelog). Whether plugin hooks also skip them: not
  verified.

### Output, failures and cost

- The command reads JSON on stdin and must exit 0 and print a JSON object. "Any non-zero exit code is logged as a hook
  execution error and ignored (failing open)" (`docs/build-plugins`). A failing `PostToolUse` hook no longer fails its
  tool (changelog).
- `Stop` output: `decision` is called "Required"; `"continue"` makes the agent go on, "Any other value allows the stop"
  (`docs/hooks`). `docs/build-plugins` shows `{"decision": "block"}` to prevent completion. The two pages disagree, so
  the helper should print `{}` and a live test must confirm a stop with no decision is allowed.
- Hooks run synchronously. `PreInvocation` fires on every model call, so a turn with 30 tool steps starts the helper
  about 30 times. How the command is started on Windows (directly, `cmd`, or PowerShell) is **not documented**; the
  binary contains `powershell -noprofile -command` but its use was not traced. If it is PowerShell, each call costs a
  PowerShell start. Measure in the live test.
- Command form: because the shell is unknown, use a form that works in `cmd`, PowerShell and a direct start: a
  backslash relative path with no spaces, such as `.\wakeshot-hook.exe antigravity`, with the helper (or a copy)
  inside the plugin folder. Design idea, **not tested**.

### Privacy

`lastUserInput` (the latest prompt), `toolCall.args` (commands, file contents for edits) and `workspacePaths` arrive
on stdin. The helper keeps only the event name, `conversationId`, `fullyIdle`, `terminationReason` and its own pid
walk, and never stores or logs the rest (same rule as Claude's transcripts).

## State on Windows

### Folders

`~` is `%USERPROFILE%`. The `.gemini` folder moves with `ANTIGRAVITY_GEMINI_DIR` (string in the CLI binary; exact
meaning not verified).

| Path | What | Source | Mark |
|---|---|---|---|
| `~\.gemini\config\` | Shared customizations: `hooks.json`, `plugins\`, `skills\`, `rules\`, `agents\`, `mcp_config.json`, `config.json` (user settings, `enableAdc`, remote control names) | `docs/customization-discovery`, `docs/install`, `docs/remote-control` | verified (docs) |
| `~\.gemini\antigravity-cli\` | CLI app data: `settings.json`, `keybindings.json`, `cache\` (`projects.json`), `updater\`, `skills\` | `docs/cli/using`, `docs/cli/troubleshooting`, changelog | verified (docs) |
| `~\.gemini\antigravity\` | App data of the App, and of the IDE since 2.6 (the IDE starts its agent with `--app_data_dir=antigravity` and copies `antigravity-ide\conversations\*` into `antigravity\conversations\`, writing `annotations\` and `.<id>.done` markers) | IDE `extensions/agy/vscode/extension.js` (`spawnAndWaitForPort`), `out/vs/workbench/api/node/extensionHostProcess.js` (`IdeConversationMigration`) | verified (code) |
| `~\.gemini\antigravity-ide\` | The IDE's data folder before 2.6. `docs/hooks` still says the IDE uses it; the 2.6 code says otherwise | as above | verified that they disagree |
| `<app data>\brain\<conversationId>\` | Artifacts; `.system_generated\logs\transcript.jsonl` and `transcript_full.jsonl` | `docs/hooks`; prompt text in the CLI binary | verified (docs and strings) |
| `<app data>\conversations\<id>.db` (+ `-wal`, `-shm`), older `<id>.pb` | One SQLite file per conversation, the format since about 1.0 ("Added SQLite (.db) conversation support and will be CLI's conversation format") | changelog; IDE migration code (`.db` or `.pb` is the primary file) | verified that the files exist in this form; schema not read |
| `<app data>` CLI log | `cli-<...>.log` files and a link to the newest; `--log-file` overrides; the path is shown in `/help` | strings `cli-%s.log`, `cli-%s-*.log`, package `cli/logfile` (`Create`, `CleanStale`, `updateSymlink`); changelog | verified as strings; folder and content not verified |
| `~\.gemini\bin\agy.exe` | The copy of the CLI that the IDE (and the VS Code extension) downloads and runs | `vscode/extension.js` `getInstalledTargetPath` | verified (code) |
| Tokens | Windows Credential Manager, plus "the CLI's private secrets file" for large tokens (`plugin-secrets.json` string seen) | `docs/install`, `docs/cli/troubleshooting`, changelog | verified (docs); never open |

### The transcript

Each line is one step: `step_index`, `source` (`USER_EXPLICIT`, `MODEL`, `SYSTEM`), `type` (`USER_INPUT`,
`PLANNER_RESPONSE`, ...), `status` (`DONE`, `ERROR`, ...), `created_at` (ISO 8601), `content`, `thinking`,
`tool_calls`, `media`, `truncated_fields`. Verified: the format text embedded in the CLI binary. The writer has
`AppendStepToTranscript` and `WriteFullTranscript` (package `cortex/artifacts/trajectorylog`), and the step status
enum also has `RUNNING`, `WAITING`, `GENERATING`, `PENDING`, `QUEUED`, `CANCELED`, `INTERRUPTED`, `HALTED`
(`CORTEX_STEP_STATUS_*` strings). Not known: whether a step is written when it starts or only when it ends, and
whether a waiting step is ever written. Context compaction rewrites the file (changelog: "a background message
appended to it while context compaction was rewriting it"), so a byte-offset tail can break, as for VS Code.

## Processes

- **CLI:** `agy.exe` from `%LOCALAPPDATA%\agy\bin\`, a child of the user's shell. Headless `agy -p` runs exist.
  "Daemon background tasks" (dev servers started by the agent) can outlive a turn and, in `-p`, the run (changelog).
  Their process image: not verified.
- **IDE:** `Antigravity IDE.exe` (main) and its utility processes; the bundled extension (`extensions/agy`, "Google
  Antigravity") spawns `~\.gemini\bin\agy.exe --hub --hub-port=<port> --app_data_dir=antigravity
  --csrf_token=<token> --add-dir=<each workspace folder>` with `shell: false`. Verified: `vscode/extension.js`.
  The parent is the extension host. Not verified on Windows.
- **Remote Control daemon:** `agy remote-control start` registers a Scheduled Task (strings `agy_task.xml`,
  `agy_hide.vbs`, `schtasks`) that keeps an `agy.exe` running at logon or boot (`docs/remote-control`). It idles most
  of the time.
- **App:** `Antigravity.exe` (2.29.0, not read). "Another `agy` session, the Antigravity Hub, or the `remote-control`
  daemon" can each hold plugin files (changelog).
- So "`agy.exe` is alive" is not "working": an idle TUI, an IDE hub and the daemon all look the same by image. A
  command-line check (`--hub`) tells the IDE or an extension from the CLI.

## Working, waiting, idle from outside

| Signal | What it shows | No admin | Confidence |
|---|---|---|---|
| Hooks (`PreInvocation` ... `Stop`) | Working from the first model call to the end of the loop; `fullyIdle` for background tasks | Yes | Medium (docs; not run) |
| `transcript.jsonl` | Steps as they are appended | Yes (file read) | Low (write timing unknown; holds prompt text) |
| Conversation `.db` | Steps with status, likely | Read-only SQLite | Low (schema not read; holds prompt text) |
| IDE state sync (`trajectorySummaries` with `RUNNING`/`BUSY`/`IDLE`) | Live run status per conversation | Needs the hub's CSRF token | High as data, but not usable: treat the token like Claude's `.key` |
| `agy.exe` alive | Only "running" | Yes | High for alive, none for working |
| `CallNtPowerInformation` | Someone holds a system request | Yes | Not useful: the CLI holds none, the IDE's is `ExecutionRequired` |

- **Waiting** is not visible through hooks. It may be visible as a `WAITING` step in the transcript or `.db`: not
  verified. Default: waiting counts as working while the process lives (as for Codex).
- **Scheduled work:** `/schedule` makes one-shot timers and cron jobs inside a conversation; the agent ends its turn
  and the timer wakes it later; `IsDaemon` jobs keep firing after the task ends; they show in `/tasks`
  (`docs/commands/schedule`). Whether a pending timer makes `Stop` report `fullyIdle: false`: not verified. The
  timer's due time is in the `schedule` tool call (`DurationSeconds`, `CronExpression`), so a transcript reader could
  find it; hooks cannot.
- **Background commands:** a command left running after a turn (a dev server) makes `fullyIdle: false` (by the
  field's doc). Hold it like Claude's `shell` state: while the agent process lives. Whether a later `Stop` comes when
  the task ends: not verified.

### Power requests

- **CLI:** none. No `SetThreadExecutionState`, `PowerCreateRequest`, `PowerSetRequest` or keep-awake string in the
  1.5.2 binary (grep of all printable strings; Go stores Windows API names it loads as strings). The one `caffeinate`
  string sits in a command-name list. The changelog has no keep-awake entry. Verified by grep; not run.
- **IDE:** yes, weakly. `streamMonitorMainService` in the main process subscribes to `trajectorySummaries` and calls
  `sleepControl.preventSleep()` while any summary is `RUNNING` or `BUSY`, which is
  `powerSaveBlocker.start("prevent-app-suspension")` (`out/main.js`, module `vs/platform/sleepControl/electron-main`).
  Chromium maps that to `PowerRequestExecutionRequired` (`copilot-vscode-signals.md`, Chromium source), which does
  not stop idle sleep on Modern Standby (Microsoft docs; not tested by us). Approval waits: the summary status during
  a wait is not known. Verified from the bundle; not run.
- Nothing touches the lid action or the battery timer.

## Free live test

What a test needs (all from the docs above; not tried):

1. A personal Google account (not Workspace), age 18 or over (verified with Google if asked), country on the account
   set to a listed one (Armenia is listed).
2. The Free plan; nothing to buy. Model Gemini 3.8 Flash (after 2026-11-18 the free Claude and GPT-OSS models go).
   Weekly quota of unknown size: keep test prompts short.
3. Accept Google's terms at first sign-in. The CLI asks for a color scheme, a rendering mode and workspace trust on
   first start (`docs/getting-started`); trust only the scratch test folder.
4. Installs: the CLI by `irm https://antigravity.google/cli/install.ps1 | iex` (per user, no admin); the IDE by
   `Antigravity IDE.exe` from `antigravity.google/download` (per user or system wide: not checked). The IDE also
   downloads its own `agy.exe` into `~\.gemini\bin\`.
5. Sign-in is shared: "Signing in on one local surface shares your credentials across your local environment"
   (claim, `docs/install`). So one sign-in in the CLI should also serve the IDE. Not verified.

An agent can drive most of the CLI test alone after the sign-in: `agy -p "<prompt>"` runs headless (hooks run in
`-p`: `PreInvocation` errors change the `-p` exit code, `docs/hooks`). Approval waits, Esc and the IDE panel need a
person or UI automation.

### Sign-in steps for Haik's one sign-in sitting

After the harness card has installed the CLI (and the IDE if it tests the IDE):

1. Open a new PowerShell window (not elevated) and run `cd <the scratch test folder the harness names>`.
2. Run `agy`. If `agy` is not found, run `& "$env:LOCALAPPDATA\agy\bin\agy.exe"`.
3. Pick any color scheme and rendering mode. When asked to trust the folder, answer yes (it is the test folder).
4. A browser opens. Sign in with your personal Google account (not a work account) and accept the terms. If Google
   asks to verify your age, do it.
5. Back in the terminal, check that the header shows your account email. Type `/usage` and Enter: it should show a
   Free quota. Type `/exit`.
6. If the harness tests the IDE: start Antigravity IDE once, open the same folder, open the agent panel with Ctrl+L,
   and check it is signed in without a new sign-in. If it asks, sign in the same way.

Tell the harness when step 5 shows the quota; it runs the rest. The token stays in Windows Credential Manager;
`/logout` removes it after the tests.

## Recommended detection rules

| # | Rule | Signal | Confidence |
|---|---|---|---|
| 1 | Find the agent: `%USERPROFILE%\.gemini\` exists, and `%LOCALAPPDATA%\agy\bin\agy.exe` or `%USERPROFILE%\.gemini\bin\agy.exe` exists | Agent installed | High for the paths in the code; the troubleshooting page's `Program Files` path is ignored |
| 2 | Install the Wakeshot plugin: `%USERPROFILE%\.gemini\config\plugins\wakeshot\` with `plugin.json` (`name: "wakeshot"`) and `hooks.json` with `PreInvocation` and `Stop` (and `SessionStart` to record the pid early). No tool events | Hook source | Medium (docs; dropped-folder load not verified for the CLI) |
| 3 | Helper on each event: read stdin, keep only the event, `conversationId`, `fullyIdle`; find the agent pid by walking parents to `agy.exe`; note whether its command line has `--hub` (IDE, App or an IDE extension) or not (CLI); write the state file; print `{}`; exit 0 | Turn state | Medium (pid walk not verified) |
| 4 | `PreInvocation`, `SessionStart`: Working. `Stop` with `fullyIdle: true`: Idle. `Stop` with `fullyIdle: false`: Idle with one open task (holds while the process lives, like Claude's `shell`) | Working / idle | Medium |
| 5 | Every state counts only while that `agy.exe` (pid and start time) lives. No end event exists, so this is the only end for a closed CLI or IDE | Liveness | High as a rule |
| 6 | Waiting: none. An approval wait reads as working until `Stop` or process exit | Waiting | High that hooks cannot see it |
| 7 | A state file with an unknown event or a missing field: `UnknownValue` (counts as working while the process lives, shows "update Wakeshot") | Format change | High as a rule |
| 8 | Do not read the conversation `.db`, `transcript*.jsonl`, `brain\` artifacts, Credential Manager, `plugin-secrets.json`, `~\.gemini\config\config.json` or the hub's port and token | Nothing safe or needed | High |
| 9 | Uninstall: delete `plugins\wakeshot\`; retry later if a file is held | Clean removal | Not verified |

## For the source card

- **Files:** only Wakeshot's own hook state files (the folder and format come from `p4-hooks-design`). The source
  never opens an Antigravity file. The plugin folder is written by the hooks install code (`agent_hooks.rs`, owned by
  `p4-cursor-source`, or the place `p4-hooks-design` names).
- **Parser:** one state file in (`Feed::Whole`), one `SessionFacts` out. `key`: `SourceId::Antigravity` plus
  `conversationId`. `liveness`: `Liveness::Process { pid, start }` from the helper's walk. `activity`: rule 4.
  `timers`: none. `open_tasks`: 1 after `Stop` with `fullyIdle: false`, else 0. `label`: "Antigravity CLI" or
  "Antigravity" (hub: IDE, App or an IDE extension). `format`: rule 7.
- **Contract needs beyond `p4-contracts-r2`:** none found. `SourceId::Antigravity` and the `sources/antigravity.rs`
  stub come from its Deliverable 1; `Liveness::Process` exists; reading a folder of small state files is
  `WantedFile::Folder`. If the helper cannot find a start time, `Liveness::ImageSince` (Deliverable 3) with
  `agy.exe` is the weaker fallback.
- **Without hooks:** no source today. If a live test shows `transcript.jsonl` gets a line when a turn starts and a
  closing line when it ends, a transcript reader could use the recently-changed whole-file mode of Deliverable 5 on
  `brain\*\.system_generated\logs\transcript.jsonl` under both app data folders. Change it when that test passes.
- **What else the same plugin covers:** the Antigravity App and the IDE extensions (VS Code and others start the same
  `agy --hub`, and every surface reads `~\.gemini\config\`). Claim plus code for VS Code; not verified for the others.

### Fixture sketch

Hook stdin as the docs show it (`docs/hooks`), shortened. Real payloads carry the prompt in `lastUserInput`.

```json
{"invocationNum":0,"initialNumSteps":1,"conversationId":"ec33ebf9-0cba-4100-8142-c61503f6c587","workspacePaths":["C:\\code\\demo"],"modelName":"gemini-3.8-flash","lastUserInput":"fix the build"}
{"executionNum":1,"terminationReason":"model_stop","error":"","fullyIdle":true,"conversationId":"ec33ebf9-0cba-4100-8142-c61503f6c587"}
{"executionNum":2,"terminationReason":"model_stop","error":"","fullyIdle":false,"conversationId":"ec33ebf9-0cba-4100-8142-c61503f6c587"}
```

Line 1 is `PreInvocation` (Working), line 2 `Stop` (Idle), line 3 `Stop` with a background task left (Idle, one open
task). The `modelName` value is a guess.

## Open questions only a live test answers

1. Does the CLI load a plugin folder dropped into `~\.gemini\config\plugins\` without `agy plugin install`, and is it
   enabled by default? Same for the IDE.
2. How a hook command starts on Windows (direct, `cmd`, PowerShell), the parent of the helper, and the time each call
   adds per model call.
3. Does Ctrl+C or Esc during a turn fire `Stop`? Does closing the terminal fire anything?
4. Does `Stop` come with `fullyIdle: false` while a `/schedule` timer waits or a dev server runs, and does a later
   `Stop` come when it ends?
5. Is `{}` accepted as `Stop` output (no `decision`) without blocking the stop?
6. In the IDE: is the hub `agy.exe` the hook's ancestor, and does one IDE window per workspace share one hub?
7. Does `transcript.jsonl` get a line when a step starts (`RUNNING`, `WAITING`) or only when it ends? This decides
   whether a no-hooks rule is possible.
8. Does `powercfg /requests` list `Antigravity IDE.exe` under EXECUTION while the IDE agent runs, and nothing for the
   CLI?
9. Is Free sign-in possible from Armenia without a card or phone?

## Phase 4 live test (2026-11-06)

Card `p4-live-antigravity`. Antigravity CLI 1.5.2 and IDE 2.6.0 for Haik's user, Free plan ("Antigravity Starter
Quota"), Gemini 3.8 Flash at low effort; 11 one-line test prompts, 10 of them reached a model. Wakeshot's plugin
folder came from the product's add code (`agent_hooks::at_start`, run on a scratch profile so the real `.cursor` was
not touched, then its three files copied into the real `plugins\` under the staging name and renamed), naming a debug
`wakeshot.exe` that wrote to a scratch data folder. Readers at Medium: `agent-watch --source antigravity` and a
dry-run tray. Times are from a kernel process trace (`logman`, Microsoft-Windows-Kernel-Process: names, pids, times
only), the driver's log and the readers' lines, on one clock. A scratch capture plugin beside Wakeshot's kept the real
inputs in the scratch folder (redacted copies in `crates/wakeshot-core/tests/fixtures/antigravity/live/`).

Answers to "Open questions only a live test answers":

1. **A dropped folder loads, enabled, with no `agy plugin install`** (CLI): `agy plugin list --json` lists it at once,
   `enabled: true`; `/hooks` shows PreInvocation and Stop with 1 hook each; `agy plugin validate --strict` passes (one
   note: no logo). A running TUI also picks up a plugin folder added after its start, from its next model call.
   **Dot-named folders are not skipped**: a staging folder `.wakeshot-new-<pid>` is listed as an enabled plugin under
   its folder name, and a folder renamed `.wakeshot-removed` ran both its hooks in a real turn. Verified live.
2. **Shell and parent**: `agy.exe` starts `cmd.exe`, which starts the hook (`wakeshot.exe` -> `cmd.exe` -> `agy.exe`);
   the input is on stdin (a pipe), `ANTIGRAVITY_CONVERSATION_ID` is set and equals the input's id, the working folder
   is the plugin's folder. The runtime transport check found no payload key in `cmd`'s command line in any run (no
   `blocked.json`, no log line). Hooks of two plugins run one after the other, never at once. Cost of Wakeshot's hook
   per event, from 21 runs: `cmd` 36 to 111 ms (median 42), the debug exe inside it 25 to 88 ms (median 31); a
   `PreInvocation` whose file already says working writes nothing. The release exe was not measured. Verified live.
3. **Esc runs no `Stop`**: Esc during a running command shows "Interrupted · What should Antigravity CLI do instead?",
   ends the command, and runs no hook. The chat's file stays `working`, so the hold lasts until the next prompt's
   `Stop` or the end of that `agy.exe`. `/exit` runs no hook either; it ends the process and its background commands,
   and the watch thread sees the end at once (`alive=Gone` 5 ms before the driver did). A crash (the TUI killed by
   pid mid-command) leaves the file `working` with a dead pid, which holds nothing. Verified live (CLI).
4. **`fullyIdle` is not "no background command runs"**: `Stop` came with `fullyIdle: false` twice (a command `agy`
   moved to the background after waiting about 10 s; a background command after an Esc'd turn) and `fullyIdle: true`
   twice for a command the agent started in the background while it still ran (the footer showed "1 task(s)"). So the
   dry-run tray held during the second case only until the `Stop`. Each time the command ended while `agy` ran, the
   agent woke itself: a new `PreInvocation` (`invocationNum` 0), then `Stop`, and that held. The binary has
   `HasIdleBlockingBackgroundTasks` and a `WaitMsBeforeAsync` run-command argument, so likely only some background
   tasks block "idle" (strings only; **not verified**). A `/schedule` timer: **not tested live**.
5. **`{}` lets a stop through**: every turn ended normally with Wakeshot's `{}` answer. Verified live.
6. **IDE**: the hub is `%USERPROFILE%\.gemini\bin\agy.exe --hub ...`, started by the IDE's extension host for an
   empty (trusted) window; a folder in Restricted Mode loads no Antigravity extension and starts no hub (process
   facts, verified). The IDE's settings text says each window starts its own hub. An IDE chat could not run: the IDE
   showed its first-run "Get Started" screen, which only Haik may finish. So a hook under the hub, the IDE's Esc and
   crash, and a Remote Control chat are **not tested live**.
7. Transcript timing: not looked at (the transcript holds prompt text; Wakeshot does not read it).
8. `powercfg /requests`: not run.

Other facts seen: the real `Stop` input has `artifactDirectoryPath`, `conversationId`, `error`, `executionNum`,
`fullyIdle`, `lastUserInput`, `modelName`, `terminationReason` (seen: `NO_TOOL_CALL`), `transcriptPath`,
`workspacePaths`, and no `finalModelOutput`; `PreInvocation` has `initialNumSteps` and `invocationNum` instead of the
four `Stop` fields. Keys come sorted, compact, paths with `/`, no line end. `agy` runs a shell command through
`powershell.exe`. Through the app: the dry-run tray logged "Holding starts: Antigravity CLI" 0.1 s after Enter and
"Holding ends" at the `Stop`, also for the first file in an `agents\antigravity\` folder the hook made while the tray
ran.

## Exact end (2026-11-06)

Card `p4-antigravity-exact-end`, after Haik's "No guessing" rule. Antigravity CLI 1.5.2, Gemini 3.8 Flash, Free plan,
TUIs at Medium under the harness driver. Part A (facts): a scratch capture plugin on all six hook events (event, times,
ids cut to 8 characters, numbers, booleans, key names; never the prompt, `lastUserInput`, tool args or a path), a
kernel process trace, and `agy`'s own log read with the prompt lines cut out; 42 one-line prompts. Part B (build and
live check through a dry-run tray): 6 one-word prompts. Every time and run is in the card's Log.

| Case | Hook events after the key or the end | Other exact signal | Exact? |
|---|---|---|---|
| Normal turn | `PreInvocation` ... `PostInvocation` (after the call's tools) ... `Stop` | `agy` exit | Yes, by hooks |
| Esc during a model call, a tool run, an approval wait, a streamed answer (18 runs) | None after `PreInvocation` or `PreToolUse`; a foreground command's `powershell.exe` ends at once | `agy`'s log: "Cancelling conversation <chat id> (killBackgroundTasks=false)" | Hooks: no. With the log: yes |
| Ctrl+C in a turn (3 runs; 1 more live in Part B) | Same as Esc; `agy` stays open | The same log line | With the log: yes |
| A second prompt typed while one runs (3 runs) | Queued: the first turn's `Stop`, then the second's `PreInvocation` 0.15 to 0.46 s later | | Yes, by hooks |
| Background command, natural end (4 runs and 2 more) | `PreToolUse` N, `PostInvocation` with no `PostToolUse` N, `PostToolUse` N 50 to 65 ms after its process ends, then the agent wakes itself | The command's `powershell.exe`, a child of `agy.exe` | By hooks or by the process |
| Background command killed from `/tasks` or by the agent's `manage_task` (1 run each) | None for it | `/tasks`: "Cancelling steps [N]"; the agent's kill: no line; the process ends | Only by the process |
| `fullyIdle` on `Stop` while a background command runs | `false` in 5 runs, `true` in 2 (their args had `IsDaemon`; values not read) | | No |

**The log** (verified live): each `agy.exe` writes `%USERPROFILE%\.gemini\antigravity-cli\log\cli-<start time>.log`
(glog lines `Lmmdd hh:mm:ss.uuuuuu <thread> <file>:<line>] <msg>`, local time, line ends `\n`). Line 1 is "Starting
language server process with pid <pid>", the `agy.exe`'s own pid; line 2 is "Language server version: 1.5.2" (9 of 9
logs on this PC). `cli.log` one folder up is a hard link to the newest process's file. Each Esc and Ctrl+C wrote the
cancel line within 1 ms of the "Cancelling in-progress response" line, about 50 ms after the key (22 of 22 in Part A,
5 of 5 in Part B), flushed at once; no normal turn and no Ctrl+C on an idle prompt wrote it; its id is the hook's
full `conversationId` (Part B). All 28 such lines on this PC end in "(killBackgroundTasks=false)". The log also holds
every prompt in plain text ("HandleUserInput called with text: ..."). Not documented anywhere found; a later `agy` may
change any of it, so Wakeshot trusts only tested versions.

**Timing** (verified): in the two fastest Part A runs (Esc 0 to 0.1 s after Enter), the cancel line came 16 and 25
ms after the `PreInvocation` hook had started; in Part B the hook's `cmd /c` started 33 to 42 ms after Enter and the
cancel line came about 1.1 s later. A folder watch (Wakeshot's own filter) sees no change while a process appends to
a file it keeps open (a scratch writer: "Created" at the open, "Changed" only at the close), so the log is read at the
hook's writes and at the 30 s safety tick: in Part B each Esc's or Ctrl+C's hold ended 7.6 to 20.4 s after the cancel
line, and an Esc followed by `/exit` ended at the process's end.

**The rule built** (`agent-hooks.md`, "Exact end"): a model call's hook names the log and the start of its `cmd /c`;
the source ends the chat at a cancel line for it at or after that start (local time, through the PC's zone), at
`Stop`, or at `agy`'s end. A log not as tested, or none found: no hold and "update Wakeshot". Background commands
are not held. The IDE's hooks write nothing (the IDE is out of v1 until its own check passes).

Not tested: `killBackgroundTasks=true` (never seen), `/clear` or `/new` during a turn, a turn that ends on a quota or
network error, `/btw`, a second TUI through the tray in Part B (Part A saw each process keep its own log), the IDE.

## Out of v1 (2026-11-06)

Haik, about 21:50: "Yes remove antigravity. Too much problematic." (card `p4-vscode-local-off`). The reason the
orchestrator gave: the rule above trusts `agy` 1.5.2 only, and `agy` releases almost daily (its `CHANGELOG.md`
changed on about 25 days since 2026-10-02; not verified here), so it would hold nothing most of the time, and
Esc has no other exact signal ("Exact end"). Since then the tray neither watches Antigravity nor adds its plugin
folder at a start; `wakeshot hook antigravity remove` and the uninstall still remove a folder an earlier build added.
Verified by tests in `app/tray_mode.rs` (`the_tray_starts_the_v1_sources_only`) and `agent_hooks.rs`
(`a_start_adds_no_antigravity_folder_and_the_uninstall_still_removes_an_old_one`), each seen failing when Antigravity
was put back. The source, the log reader and their tests stay for a later version.
