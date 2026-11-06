# Cursor, Gemini CLI, and hooks as a signal

Research for Wakeshot, 2026-11-06 (card `p4-cursor-gemini-research`). Builds on `other-agent-signals.md`.

Marks: **verified** = seen in source, a shipped bundle or official docs (path or URL given). **claim** = a number
the company states about itself. **not verified** = read elsewhere, inferred, or not checked. Nothing was installed
or run. Downloads were unpacked as data only, in the card's scratch folder, and deleted at the end.

## Summary

| Agent | Real usage (source, date) | Best signal | Confidence | Needs hooks |
|---|---|---|---|---|
| Cursor editor 3.29.9 | 18% of developers use it at work (JetBrains, Jan 2026). Revenue $608M in 48 days (SpaceX 10-Q, Q3 2026). "Millions of developers" (claim) | Cursor's own wakelock lines in `main.log`, or hooks from a Wakeshot plugin folder | Medium for the log (undocumented text); medium-high for hooks (documented, not run) | No (log), or yes (hooks) |
| Cursor CLI 2026.11.05 | No numbers found. Homebrew cask 1,537 installs in 30 days (macOS only) | Alive: `.running\<pid>` marker. Working: hooks only | High for alive; medium for hooks (not run) | Yes |
| Gemini CLI 0.67.0 | npm 1.49M downloads in Oct 2026, down from 3.47M in Apr. Enterprise only since 2026-06-18 | Hooks from a Wakeshot extension folder | Medium (documented, not run; a cancel may skip the end event) | Yes |

Short answer to the goal: Cursor is the bigger audience by far and its editor has a passive signal; Gemini CLI's
users moved to Antigravity CLI and Gemini CLI has no file signal at all.

## 1. Usage

### Gemini CLI

| Number | Source | Mark |
|---|---|---|
| npm downloads per month: Apr 3,471,911; May 3,185,224; Jun 2,651,187; Jul 2,077,435; Aug 1,688,011; Sep 1,701,355; Oct 1,486,041 | `api.npmjs.org/downloads/range/2026-04-01:2026-10-31/@google/gemini-cli`, read 2026-11-06 | verified |
| Oct 2026 npm for comparison: Codex 89.4M, Claude Code 44.0M, OpenCode (`opencode-ai`) 13.7M, Copilot CLI 6.16M, Qwen Code 0.34M | same API, `point/2026-10-01:2026-10-31` | verified |
| Homebrew `gemini-cli` installs: 30 days 14,691; 90 days 25,698; 365 days 756,661 | `formulae.brew.sh/api/formula/gemini-cli.json`, read 2026-11-06 | verified (macOS and Linux only, opt-out analytics) |
| 108,745 GitHub stars | `gh api repos/google-gemini/gemini-cli`, 2026-11-06 | verified |
| "more than one million developers are building with Gemini CLI" three months after launch | Google blog, 2025-10-08, `blog.google/technology/developers/gemini-cli-extensions/` | claim |
| "over 100,000 GitHub stars ... millions of users" | Google developers blog, 2026-05-19, transition post | claim |

Why the npm numbers fall:

- **Free tier gone.** From 2026-06-18 Gemini CLI stops serving Google AI Pro and Ultra subscribers and free Gemini Code
  Assist for individuals. It stays for Gemini Code Assist Standard and Enterprise, Code Assist for GitHub on Google
  Cloud, and paid Gemini API or Vertex keys. Verified: Google developers blog "An important update: Transitioning
  Gemini CLI to Antigravity CLI" (2026-05-19), and the repo `README.md` lines 11-24 at commit `3fd7d26`.
- **Google moves people to another product.** Free and Pro or Ultra users go to Antigravity CLI (`agy`), which Google
  describes as built in Go and sharing its harness with Antigravity 2.0. The README now says Gemini CLI is "Not
  optimized for Gemini 4+". Verified: same two sources. Antigravity CLI is a native binary (Homebrew cask
  `antigravity-cli` 1.5.1 downloads a tarball from `storage.googleapis.com`), so it does not show up in npm at all.
- **The fall started before June.** April to May already lost 8%. Why: not verified (competition is a guess).
- **Other install paths are small on Windows.** The README lists npx, npm, Homebrew, MacPorts and conda (conda runs npm
  inside). On Windows only npm and npx apply, and both count as npm downloads. So npm is the best proxy for
  Windows users. npm downloads are not users: CI runs, updates (a weekly release train plus nightlies) and npx runs
  all count.
- **Google reports no current user number for Gemini CLI.** The two numbers above are company claims from before the
  change. Google's I/O figures are for all developers on its models, not for the CLI (secondary sources, not
  verified).

Reading: Gemini CLI's Windows audience today is enterprise Code Assist users plus a few paid-key users. By npm it is
a quarter of Copilot CLI and a sixtieth of Codex. Enterprise users also sit behind admin settings that can turn off
extensions or hooks (see section 4).

### Cursor

| Number | Source | Mark |
|---|---|---|
| $608M revenue "attributable to Cursor from the Closing Date through September 30, 2026"; closing 2026-08-14 | SpaceX Form 10-Q for Q3 2026, filed 2026-10-22, `sec.gov/Archives/edgar/data/1181412/000162828026066880/spcx-20260930.htm` | verified (a filed, unaudited quarterly report) |
| About $4.6B a year | my arithmetic: $608M / 48 days x 365. A partial period does not annualize cleanly | not verified |
| "$1B in annualized revenue", "millions of developers", over 300 staff | Cursor blog "Series D", 2025-11-13 | claim |
| "2x as many agent users as Tab users" (it was 2.5x the other way in March 2025); "Agent usage in Cursor has grown over 15x in the last year" | Cursor blog "The third era", Michael Truell, 2026-02-26 | claim |
| 64% of the Fortune 500, 50,000+ enterprises | `cursor.com/enterprise`, read 2026-11-06 | claim |
| More than 1 million daily active users (2025) | Bloomberg and others quoting the company | claim, seen only in secondary sources |
| 18% of developers use Cursor at work, same as Claude Code; Copilot 29%; Antigravity 6%; Codex 3% | JetBrains Research blog, Apr 2026, AI Pulse survey wave of January 2026, over 10,000 professional developers | verified (survey, not a company claim) |
| Homebrew 30-day installs: `cursor` cask 7,201; `cursor-cli` cask 1,537; `visual-studio-code` 35,542 | `formulae.brew.sh`, 2026-11-06 | verified (macOS only) |

How many use its agent: Cursor's own claim is that agent users outnumber Tab users two to one (Feb 2026). No number
for the CLI was found; the Homebrew ratio (CLI about a fifth of the editor on macOS) is the only hint and is weak.
The 10-Q gives no user or seat counts (searched for user, subscriber, seat, daily, monthly).

Reading: Cursor editor users are in the millions (claim) and about a sixth of working developers (survey). Most of them
use the agent (claim). That is far more Windows desktops than Gemini CLI reaches now.

## 2. Cursor editor

Source: Cursor 3.29.9 (commit `306d153d51d7`, VS Code base 1.128.0), the Linux `.deb` from the official download API
(`cursor.com/api/download?platform=linux-x64&releaseTrack=stable`), unpacked as data. The Windows build of the same
version is `CursorUserSetup-x64-3.29.9.exe` (same commit). The app JS is shared across platforms, so the code below
should match Windows; the Windows paths are inferred from VS Code conventions and **not verified** on a real install.
Paths below are under `resources/app/`.

### Cursor blocks sleep itself, with the weak request type

- For every local agent run, Cursor makes a `ComposerWakelockManager`, which calls `powerMainService.startWakelock`
  with reason `agent-loop`. It releases it with reason `user-approval-requested` while the agent waits for the user,
  takes it again with `agent-loop-resumed`, and releases it with `generation-ended`. Not for runs created from a
  cloud agent. No setting or feature gate guards it (only background throttling has a gate). Verified:
  `out/vs/workbench/workbench.desktop.main.js` (module `composerWakelockManager.js`, and the `new jfp(` call in the
  submit path); the same in `workbench.glass.main.js` (the Agents window).
- The main process turns that into Electron `powerSaveBlocker.start("prevent-app-suspension")`. Verified:
  `out/main.js`, module `powerMainService.js`.
- Electron maps `prevent-app-suspension` to `WakeLockType::kPreventAppSuspension` (verified:
  `electron/electron` `shell/browser/api/electron_api_power_save_blocker.cc`). Chromium's Windows code turns that
  type into **`PowerRequestExecutionRequired`**; only the display types get `PowerRequestDisplayRequired`. Verified:
  `chromium/src` main, `services/device/wake_lock/power_save_blocker/power_save_blocker_win.cc`, `RequestType()`,
  read 2026-11-06. Cursor's own Chromium version was not checked (not verified that it has the same code).
- What that means: on Modern Standby, `PowerRequestExecutionRequired` alone does not stop idle sleep (Microsoft
  docs; not tested by us; `CLAUDE.md`). On older S3 laptops it implies `SystemRequired` (same docs). So on most new
  laptops Cursor's own lock probably does not keep the PC awake, and it never touches the lid action or the battery
  timer. **Not tested.** Wakeshot adds value for Cursor in all three cases.
- A second user of the same service: `remote-control-keep-awake` in the Agents window (remote control). Verified:
  `workbench.glass.main.js`. Wakeshot should not count it as agent work.

### Signal from outside, no admin

**Best without hooks: Cursor's `main.log`.** The main process logs every wakelock at info level:

```
[PowerMainService] Started wakelock id=<n> owner=<owner> reason="agent-loop" activeCount=<k>
[PowerMainService] Stopping wakelock id=<n> owner=<owner> reason="agent-loop" heldForMs=<ms>
```

Verified: the two `logService.info` strings in `out/main.js`, `powerMainService.js`. The window log (renderer) also
logs `[ComposerWakelockManager] Acquired wakelock id=... reason="..." composerId=...` and `Released wakelock ...
reason="user-approval-requested"` (verified: `workbench.desktop.main.js`), which adds the reason for the release.

- Path: `<userData>\logs\<UTC start time as YYYYMMDDTHHMMSS>\main.log`. The folder name and `main.log` are verified
  in `out/main.js` (`logsHome`, `createLogger("main")`). `<userData>` on Windows is `%APPDATA%\Cursor` by the VS Code
  convention (`nameShort` is `Cursor` in `product.json`): **not verified** on a real install.
- Rule sketch: while `Cursor.exe` lives, take the newest logs folder, read `main.log` forward, and count `Started`
  lines with reason `agent-loop` or `agent-loop-resumed` whose id has no `Stopping` line. Count > 0 = working.
  Approval waits release the lock, so they read as not working. A crash leaves a `Started` line behind, but then
  `Cursor.exe` is gone. Pair with the process, as for every other source.
- Risks: it is log text, not a contract. Cursor ships often and may change the words; then the rule finds nothing and
  the PC sleeps as usual (fails safe), and Wakeshot should show "update Wakeshot" if Cursor runs and the log has no
  `[PowerMainService]` lines at all. A user who lowers the log level below info hides the lines. Log size limits and
  rotation: **not verified**. The default log level being info is the VS Code default: **not checked** in this
  bundle.
- Privacy: the lines carry ids and reasons, no prompt text. Wakeshot would read only lines that start with
  `[PowerMainService]`.

**Not a signal: `state.vscdb`.** Chats live in SQLite (`cursorDiskKV`, keys `composerData:<id>`; global and
per-workspace `state.vscdb`). The chat `status` has the values `none`, `generating`, `completed`, `aborted`, `error`,
but the save function writes `generating` as `aborted` (unless the chat streams from a cloud agent), so the file never
shows a running chat. Verified: `workbench.desktop.main.js`, function `jAo` (`u==="generating"&&!Nn?"aborted":u`).

**Weak: transcripts.** The editor's agent runtime also writes `~/.cursor/projects/<slug>/agent-transcripts/<id>.jsonl`
(or `<id>/<id>.jsonl`, and `subagents/`). Verified: `extensions/cursor-local-agent-runtime/dist/main.js`. A turn end
marker in them: **not checked**. Same weakness as for the CLI.

**Not a signal: `CallNtPowerInformation(SystemExecutionState)`.** It shows only that someone holds a system request
(`research.md`), and Cursor's request is `ExecutionRequired`, which may not show there at all (not checked).

### Cursor hooks (editor and CLI)

Verified from `cursor.com/docs/agent/hooks` and `cursor.com/docs/reference/third-party-hooks`, read 2026-11-06:

- File `{"version": 1, "hooks": {...}}`. Sources, highest first: enterprise `C:\ProgramData\Cursor\hooks.json`, team
  (dashboard), project `.cursor\hooks.json`, user `~/.cursor/hooks.json`, then installed plugins. All matching hooks
  run. Cursor watches the files and reloads them.
- Events that matter here: `beforeSubmitPrompt` (after the user hits send), `stop` (agent loop ends; input `status`
  is `completed`, `aborted` or `error`), `sessionEnd`, `subagentStart` / `subagentStop`. Common input:
  `conversation_id`, `generation_id`, `cursor_version`, `workspace_roots`, `user_email`, `transcript_path`; the
  `beforeSubmitPrompt` input also has the `prompt` text.
- Failures: crashes, timeouts and exit codes other than 2 fail open by default. On Windows the desktop app runs hook
  commands through PowerShell.
- Cursor also loads Claude Code hooks from `~/.claude/settings.json` and project `.claude/settings*.json`, on by
  default ("Include Third-Party Plugins, Skills, and Other Configs").
- **Plugin form.** A plugin can ship hooks. A local plugin goes in `~/.cursor/plugins/local/<name>/` with
  `.cursor-plugin/plugin.json`; then restart Cursor or Reload Window. On Enterprise an admin must allow local plugin
  imports. Verified: `cursor.com/docs/plugins`. Removal of a local plugin by deleting its folder: inferred, **not
  verified**.
- Cloud agents run only some hooks and no user-level hooks; they do not need the PC anyway.

## 3. Cursor CLI, re-checked

Newest version on 2026-11-06: `2026.11.05-86047df` (install script `cursor.com/install?win32=true`; package
`downloads.cursor.com/lab/2026.11.05-86047df/windows/x64/agent-cli-package.zip`, unpacked as data). The earlier
research read `2026.10.22-27dcf5f`.

- Still no power API: no `powerSaveBlocker`, `SetThreadExecutionState`, `PowerSetRequest`, `caffeinate`,
  `systemd-inhibit` or keep-awake string in `dist-package/*.js`. Verified by grep.
- Liveness markers unchanged: `install-in-use-marker` module present (`2093.index.js`, `3026.index.js`,
  `5006.index.js`), and the install script still checks `.running` (line 32). Verified.
- Hooks: the bundle holds a hook executor that runs `beforeSubmitPrompt`, `stop`, `subagentStop` and the rest
  through `hookExecutor.executeHookForStep` when the agent loop asks for them. Verified as code. That the CLI loop
  sends each event in a real run: **not verified** (the docs do not list CLI events).
- So the earlier finding stands: the CLI shows "running" only; working needs hooks.

## 4. Gemini CLI, re-checked

- Newest release `v0.67.0`, 2026-11-04 (preview `v0.68.0-preview.0`). Main moved from `0ac57c0` (the earlier
  research) to `3fd7d26` (2026-11-05): 17 commits. None touches `chatRecordingService.ts`; the `geminiChat.ts` change
  is about empty parts, not write timing; no new pid, lock or status file. Verified: GitHub compare
  `0ac57c0...3fd7d26`.
- So there is still no file signal: a running tool or an approval wait looks the same on disk as a finished reply
  (`other-agent-signals.md`, Gemini section).
- Still no power API. Verified earlier by grep; the 17 commits add none.

### Gemini hooks

Verified at `3fd7d26`: `docs/hooks/index.md`, `docs/extensions/reference.md`, `packages/core/src/hooks/types.ts`,
`packages/cli/src/config/extension-manager.ts`, `packages/cli/src/config/settingsSchema.ts`,
`packages/core/src/hooks/hookRunner.ts`, `packages/core/src/core/client.ts`.

- Events: `BeforeAgent` (after the prompt, before planning), `AfterAgent` (agent loop ends, fired only when no tool
  calls are pending), `SessionEnd`, `Notification` (with type `ToolPermission` for approval prompts), and others.
- Config: `hooks` in project `.gemini/settings.json`, user `~/.gemini/settings.json`, system settings, or an
  extension's `hooks/hooks.json`. Hooks are on by default (`hooksConfig.enabled` default true).
- **Extension form.** Gemini loads every folder in `~/.gemini/extensions/` that holds a `gemini-extension.json`
  (`loadExtensions`, `readdir`); hooks come from `<extension>/hooks/hooks.json` with `${extensionPath}` filled in. The
  supported way in is `gemini extensions install <path>` (asks for consent; `--consent` skips the prompt) and out is
  `gemini extensions uninstall <name>`. An admin can block it: `admin.extensions.enabled = false`,
  `security.allowedExtensions` (then a folder without install metadata is skipped), `security.blockGitExtensions`.
- Hooks run synchronously; the agent waits for them. Default timeout 60 s. On Windows the command runs through
  PowerShell (`getShellConfiguration`, a `powershell` branch in `hookRunner.ts`). Exit 0 = fine, 2 = block, other =
  "a warning is shown" and the agent goes on.
- **A cancel may skip `AfterAgent`.** In `client.ts` an abort error goes to the `catch`, which yields
  `UserCancelled` and returns before the `AfterAgent` call. So after Esc, a hook signal could stay "working" until the
  next prompt or `SessionEnd`. Read in code, **not run**.

### Antigravity CLI (where Google's individual users went)

Not researched in depth. From `antigravity.google/docs/hooks?tab=cli`, read 2026-11-06: hooks shared by the
Antigravity App, CLI and IDE; events `SessionStart`, `PreToolUse`, `PostToolUse`, `PreInvocation`,
`PostInvocation`, `Stop`; global file `~/.gemini/config/hooks.json`, project `.agents/hooks.json`, or a plugin's
`hooks.json`; hooks in `~/.gemini/antigravity-cli/settings.json` are not loaded; timeout default 30 s. Windows
behavior, file state and usage: **not verified**. JetBrains counts Antigravity (the editor) at 6% of developers at
work in January 2026. If Haik wants a Google agent, Antigravity is the one to research, not Gemini CLI.

## 5. Hooks as a signal, per agent

What Wakeshot would add: one small helper exe (`wakeshot-hook`) and a config entry per agent that runs it on turn
start, turn end and session end. The helper writes a tiny state file in Wakeshot's data folder (like the hold file
of `wakeshot run`) and exits at once; the tray reads those files. So the tray does not need to run for the hook to
work, and a tray that starts later still sees the state.

| Agent | Has another reliable signal | Events Wakeshot needs | Wakeshot-owned form (no edit of the user's file) | Clean removal | Gate or trust step |
|---|---|---|---|---|---|
| Gemini CLI | No | `BeforeAgent`, `AfterAgent`, `SessionEnd`, `Notification` | Extension folder `~/.gemini/extensions/wakeshot/` (`gemini-extension.json` + `hooks/hooks.json`) | `gemini extensions uninstall wakeshot`, or delete the folder | Install consent prompt; admin can block extensions |
| Cursor editor and CLI | Editor: `main.log` (medium). CLI: no | `beforeSubmitPrompt`, `stop`, `sessionEnd` | Local plugin `~/.cursor/plugins/local/wakeshot/` | Delete the folder (not verified) | Enterprise admin must allow local plugins; Reload Window |
| Copilot CLI | Yes (`events.jsonl`) | `userPromptSubmitted`, `agentStop`, `sessionEnd`, `permissionRequest` | Own file in `%USERPROFILE%\.copilot\hooks\` (that every `*.json` there loads, as in `.github/hooks/*.json`: not verified), or a plugin's `hooks.json` | Delete the file | None found |
| Codex CLI | Yes (rollout events) | `UserPromptSubmit`, `Stop`, `SessionEnd`, `PermissionRequest` | A plugin's `hooks/hooks.json`; else `~/.codex/hooks.json` | Delete the plugin or file | Every non-managed hook must be trusted by the user in `/hooks` first; changed hooks are skipped until trusted |
| OpenCode | Yes (SQLite) | `session.status`, `session.idle` | JS file in `~/.config/opencode/plugins/` (auto-loaded) | Delete the file | None found; plugin errors: not documented |
| Kiro CLI (Windows 11 native since 2.0) | Not known | Prompt Submit, Agent Stop, Session End | Own file in `~/.kiro/hooks/` (`"version": "v1"`, CLI V3) | Delete the file | Not known |
| Antigravity CLI | Not known | `PreInvocation`, `Stop` | A plugin's `hooks.json`; else the shared `~/.gemini/config/hooks.json` | Delete the plugin | Not known |
| Claude Code | Yes (session files) | n/a | Plugins can ship hooks | n/a | Product rule: no hooks for Claude |

Sources, all read 2026-11-06: Copilot `docs.github.com/en/copilot/reference/hooks-configuration`; Codex
`learn.chatgpt.com/docs/hooks` (was `developers.openai.com/codex/hooks`); OpenCode `opencode.ai/docs/plugins/`; Kiro
`kiro.dev/docs/hooks/` (updated 2026-10-21), `kiro.dev/docs/hooks/types/` (updated 2026-10-10) and the CLI 2.0
changelog; Antigravity as above. Docs only; no hook was run.

### When Wakeshot is not running, or is gone

- Tray not running: the helper still writes its file; nothing breaks.
- Wakeshot uninstalled with hooks left behind: the command is missing. Gemini then shows a warning on every event;
  Cursor fails open (logs it); Copilot logs and skips; Codex reports a hook failure. The Velopack uninstall hook is
  per-user and can delete Wakeshot's own files. A Store (MSIX) install has no uninstall hook (`product.md`), so it
  would leave them. Cheap guard: both Gemini and Cursor run hook commands through PowerShell on Windows, so the
  command can test for the exe first and exit 0 when it is gone. Design idea, **not tested**.
- Wakeshot updated: the hook must point at a path that survives updates (Velopack's `current` folder is assumed
  stable: **not verified**).

### Risks

- **Prompts reach the helper.** `beforeSubmitPrompt` and `BeforeAgent` send the prompt text (and Cursor sends
  `user_email`) on stdin. The helper must read only the event name and ids and never store or log the rest, the same
  rule as for Claude's transcripts.
- **Missed end events.** A crash or kill never fires the end event, and a Gemini cancel may not either. Pair every hook
  state with process liveness (the helper records the agent's pid, walking up past the PowerShell parent: not
  verified), plus the Gemini cancel case needs a live test.
- **Cost per event.** Each event starts PowerShell and the helper, and Gemini waits for it. Hook only turn start and
  turn end, never tool events.
- **Trust and consent.** Writing into another tool's config folder is a change the user did not make. It must be
  opt-in per agent, shown in Wakeshot's settings, and undone when turned off. For Gemini, dropping a folder skips the
  agent's own consent prompt; running `gemini extensions install` keeps it. Codex makes the user trust each hook.
- **Admin policy.** Gemini CLI's remaining users are enterprise customers; their admins can turn off extensions or
  hooks. Cursor Enterprise admins can block local plugins.
- **Format drift.** Hook schemas are versioned (`version: 1` for Cursor and Copilot, `v1` for Kiro), which is more
  stable than log text or private files.

## 6. Product question for Haik

May Wakeshot install hooks for agents that have no other reliable signal (today: Gemini CLI and Cursor CLI)? My
default: yes, but only opt-in per agent ("Connect Gemini CLI" in settings, off by default), only as a file or folder
Wakeshot owns (a Gemini extension, a Cursor local plugin), never an edit of the user's own settings file, removed when
turned off and at uninstall, and never for Claude Code. For the Cursor editor I would start without hooks, from
`main.log`, and keep the plugin as the fallback if a live test shows the log is not reliable.

## What is left

- Live test of the Cursor `main.log` rule on Windows (needs a Cursor install; the Hobby plan may be enough: not
  verified): the log path, info level by default, the `Started` and `Stopping` lines, approval waits, a cancel.
- Live test that Cursor's own lock does not stop idle sleep on this Modern Standby laptop (`powercfg /requests`
  should list it under EXECUTION, not SYSTEM).
- Gemini: whether `AfterAgent` fires after Esc. Needs a paid key or an enterprise seat.
- Cursor CLI: which hook events a real run fires.
- Antigravity CLI: state on Windows, hooks, usage. Probably more individual users than Gemini CLI now.
