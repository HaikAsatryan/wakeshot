# Rivals deep dive: Nightcrew and LidGuard

Collected on 2026-11-03 on the Windows laptop. Builds on `docs/research.md` (first Nightcrew review); this file does
not repeat it.

Marks: **V** = verified (seen in code, docs, git history or on this laptop). **U** = unverified (claim, inference or
guess). Code references are `repo/path:line`.

## Sources

| Repo | Commit read | Date | Commits | Notes |
|---|---|---|---|---|
| `nightcrew` = github.com/leikauskas/nightcrew (was Mokzy/nightcrew) | `9b4fd7d` | 2026-10-23 | 100 | v0.3.1 latest release (2026-10-18). **No commits after 2026-10-23**, so nothing changed since 2026-11-02. V |
| `lidguard` = github.com/airtaxi/LidGuard | `3ff7893` | 2026-09-28 | 222 | NuGet `lidguard` 2.1.7. No GitHub releases. Extra branch `codex/disable-codex-fallback-watchdog`. V |

Read: all source for detection, power, lid and lifecycle; both READMEs; LidGuard's `.agents/skills/*/SKILL.md`
(LLM-written design maps, very detailed); Nightcrew `docs/hardware-checklist.md`; every issue and PR (Nightcrew: 1
issue, 13 PRs; LidGuard: 4 issues, 1 PR); all Nightcrew release notes; every commit subject and every fix commit
body. Also read (read only) the installed Nightcrew 0.3.1 log and the Claude Code status files on this laptop.

Both repos were cloned to the scratchpad. Nothing was built or run.

## 1. Agent detection

### 1.1 Approach in one line each

- **Nightcrew: polls, no hooks.** Every 10 s it samples 4 signals; any signal busy within the last 5 min keeps it
  armed. `nightcrew/src/Nightcrew/Program.cs:147-158`, `Core/ActivityMonitor.cs:26-44`. V
- **LidGuard: hooks per agent, plus helpers.** Each agent calls `lidguard <provider>-hook` on its events; the hook
  talks over a named pipe to a `lidguard run-server` process it starts on demand. The server also watches the
  agent's parent process and its transcript file. `lidguard/LidGuard/Commands/LidGuardCommandLineApplication.cs:35-38`,
  `Ipc/LidGuardRuntimeClient.cs:12-15`. V

### 1.2 Nightcrew, per agent

| Agent | What it reads | Rule | Ref | Mark |
|---|---|---|---|---|
| Claude Code | `%USERPROFILE%\.claude\sessions\` files matching `^\d+\.json$`; max 64 files, max 64 KB each; opened with `FileShare.ReadWrite \| Delete`; read errors skipped silently | | `Platform/SessionFileReader.cs:14-40` | V |
| Claude Code | Fields `pid`, `procStart`, `status`, `cwd` only | Counts when `status == "busy"`, or `"waiting"` when the lid is closed and "Stay awake for approvals" is on (default on) | `Core/ClaudeSessionSignal.cs:32,43,98-105` | V |
| Claude Code | Liveness | `procStart` parsed as a decimal FILETIME; must **exactly equal** `GetProcessTimes` creation time of `pid`; a process with an exit time counts as dead; unparseable start = not running | `Core/ClaudeSessionSignal.cs:69-82`, `Platform/ToolhelpProcessTable.cs:64-82` | V |
| Claude Code | Bad JSON | File skipped; one warning per run | `Core/ClaudeSessionSignal.cs:37-50` | V |
| Claude Code | Ignore filter | `cwd` containing `claude-mem-observer`, also matched against the project-folder form (every non `[A-Za-z0-9]` char turned into `-`) | `Core/ClaudeSessionSignal.cs:89-96`, `Core/Settings.cs:88` | V |
| Claude Code | Transcripts: `%USERPROFILE%\.claude\projects\**\*.jsonl` (recursive, so subagent files too) | Busy when any file grew or appeared since the last sample; first sample is only a baseline | `Core/TranscriptSignal.cs:25-46` | V |
| Claude Code | Transcript listing | `FileSystemWatcher` on `*.jsonl` (FileName, Size, LastWrite; 64 KB buffer), full relist every 10 min and after a watcher error | `Platform/WatchingJsonlScanner.cs:14,43-50,89-104` | V |
| Claude Code, Codex | Process tree from `claude.exe` / `codex.exe` (Toolhelp snapshot) | A descendant (not an agent, not `conhost`) is busy at >= 10% of one core between samples. Pid reuse guard: a child whose start time is before its parent's is skipped. CPU baseline keyed by (pid, start time) | `Core/AgentTreeSignal.cs:46-83`, `Core/Settings.cs:89-92` | V |
| Codex | Own CPU of `codex.exe` | Busy at >= 2% of one core | `Core/AgentTreeSignal.cs:47-48`, `Core/Settings.cs:90-91` | V |
| Codex | Transcripts `%USERPROFILE%\.codex\sessions\**\*.jsonl` | Growth, same as Claude | `Core/Settings.cs:87` | V |
| Any agent tree | GPU per process (PDH, busiest engine) | >= 30%, only for pids in this tick's agent tree (fixed in `ed7680f`: games and video exports kept the laptop awake) | `Core/GpuSignal.cs:31-45` | V |
| Copilot CLI, OpenCode, others | Nothing built in | User edits `settings.json` (`AgentProcessNames`, `SelfCpuAgentNames`, `TranscriptRoots`) and restarts | `README.md:108-119` | V |

Timing: tick every `SampleSeconds` = 10 s (`App/TrayApp.cs:75`); grace 5 min (`Core/Settings.cs:80`,
`Core/ActivityMonitor.cs:37`); holder list window 5 min (`Core/TranscriptSignal.cs:13`). V

Observed on this laptop (V): the installed Nightcrew logged `Live: Claude Code · 5 sessions` while one Claude session
ran 4 subagents. Subagent transcripts live in `~\.claude\projects\<project>\<session-uuid>\subagents\agent-*.jsonl`,
and Nightcrew counts each file as a "session". Small UX bug.

### 1.3 LidGuard, per agent

All hooks are installed into the agent's user config; LidGuard edits those files itself.

| Agent | Hooks / files | Start | Activity | "Waiting" (soft-lock) | Stop | Mark |
|---|---|---|---|---|---|---|
| Claude Code | 14 hooks in `CLAUDE_CONFIG_DIR\settings.json` or `~\.claude\settings.json`, shell `powershell`, timeout 30 s (longer if reply-by-phone is on) | `UserPromptSubmit`, except a prompt holding `<task-notification>` | `PreToolUse`, `PostToolUse`, `PostToolUseFailure` (not `AskUserQuestion`), `SubagentStart/Stop`, `TaskCreated/Completed`, transcript growth or LastWriteTime change | `Notification` with `permission_prompt` or `elicitation_dialog`; cleared by `elicitation_complete`, `elicitation_response` or any activity | `Stop`, `StopFailure`, `SessionEnd`; `PostToolUseFailure` with `is_interrupt`; last transcript record is a user text `[Request interrupted by user]` or `[Request interrupted by user for tool use]` | V |
| Codex CLI | `UserPromptSubmit`, `PermissionRequest`, `Stop`, optional `SessionEnd` in Codex config TOML; may need `features.hooks = true` | `UserPromptSubmit` | Transcript growth only (no tool hooks) | Transcript: a `response_item` `function_call` named `request_user_input` with no matching `function_call_output` (same `call_id`) in the last 1024 lines / 1 MB | `Stop`, `SessionEnd`; last record `event_msg` with payload type `turn_aborted` | V |
| Copilot CLI | `~\.copilot\hooks\lidguard-copilot-cli.json` (or `COPILOT_HOME`); 11 hooks | `userPromptSubmitted` | `postToolUse`, `subagentStart/Stop`, `session-state\<id>\events.jsonl` growth | `notification` `permission_prompt`, `elicitation_dialog` | `agentStop`, `sessionEnd`, last JSONL record `type == "abort"` | V |
| OpenCode 1 and 2 | Plugin `~\.config\opencode\plugins\lidguard.js` that spawns `lidguard opencode-hook --event <name>` | `chat.message` / `ctx.session.hook("prompt")` | `tool.execute.before/after`, replies | `permission.asked`, `question.asked`, `form.created` | v1 `session.idle`, `session.deleted`, `session.error`; v2 `session.execution.succeeded/failed/interrupted`, `session.deleted` | V |
| Others | "Provider MCP": the model itself is asked to call `provider_start_session` / `provider_stop_session` tools. Their docs say behavior is not guaranteed | | | | | V |

Refs: `Hooks/ClaudeHookCommand.cs:62-114`, `Hooks/ClaudeSoftLockSignalSource.cs:5-38`,
`Hooks/ClaudeHookEventNames.cs:30`, `Hooks/CodexHookCommand.cs:62-64`, `Hooks/CodexHookEventNames.cs:10`,
`Runtime/AgentTranscriptMonitor.cs:360-391,481,517,541`, `Hooks/GitHubCopilotHookEventNames.cs:6-38`,
`Assets/OpenCode/lidguard.js:9-41,66`, `Settings/ClosedLidStopFollowUpConfiguration.cs:8,28`,
`.agents/skills/lidguard-provider-integrations/SKILL.md:38-189`. V

Transcript monitor: `FileSystemWatcher` on the one transcript file plus a 1 s metadata poll; activity = length grew
or `LastWriteTimeUtc` advanced; a shorter file resets the baseline. `Runtime/AgentTranscriptMonitor.cs:9,36-38,100-114`. V
Path fallback when the hook gives none: scan the provider root for `*.jsonl` whose path contains the session id, use
it only if exactly one matches. `Runtime/AgentTranscriptMonitor.cs:195-238`. V

Liveness and timing:

| Item | Value | Ref | Mark |
|---|---|---|---|
| Watched process | Hook walks its own parent chain (max 16 levels) until a name contains `claude` / `copilot` / `opencode`, or `codex` CLI or app-server, or `bun`/`node`/`npm`/`npx` whose command line contains the name | `Processes/HookParentProcessResolver.cs:7,17-83` | V |
| Exit watch | `OpenProcess(SYNCHRONIZE)` + `WaitForSingleObject` every 1 s. **No start-time check** at open, so pid reuse between hook and server is not guarded | `Processes/ProcessExitWatcher.windows.cs:20-39`, `Runtime/LidGuardRuntimeCoordinator.cs:21,754-766` | V |
| Process exit | Treated as "cancel", not as a normal end: no webhooks, but the session is removed | `.agents/skills/lidguard-session-runtime/SKILL.md:23` | V |
| Inactive session timeout | 12 min with no activity -> session soft-locked (stops holding awake) | `Settings/LidGuardSettings.cs:16`, `Runtime/LidGuardRuntimeCoordinator.cs:565-591,684-713` | V |
| Post-stop suspend delay | 10 s | `Settings/LidGuardSettings.cs:38` | V |
| Server exits | 10 min after the last session | `Settings/LidGuardSettings.cs:18` | V |
| Hook to server | Probe 50 ms, connect attempt 250 ms, total 750 ms, server start wait 5 s | `Ipc/LidGuardRuntimeClient.cs:12-15` | V |
| Earlier approach removed | A resolver that found the agent process by command line and working directory (869 lines deleted in `d8a49f5`). Rule now: "Working directory must not be used to auto-resolve watched processes" | `.agents/skills/lidguard-session-runtime/SKILL.md:14` | V (why: U) |

### 1.4 Claude Code status file today (this laptop, Claude Code 2.1.314 and 2.1.316)

Fields seen (V): `pid`, `sessionId`, `cwd`, `startedAt`, `procStart`, `version`, `peerProtocol`, `peerFeatures`,
`kind` (`interactive`), `entrypoint` (`cli`, `claude-vscode`), `pidDomain` (`win32:<host>`), `asksForConvertedSkills`,
`messagingSocketPath`, `name`, `nameSource`, `nameSince`, `status` (`busy`, `idle`), `updatedAt`, `statusUpdatedAt`
(ms epoch), `bridgeSessionId`. New since the first review: `version`, `pidDomain`, `messagingSocketPath`, `name*`.

**`busy` stays set after the turn ends while background subagents run.** V. This session's main transcript ended its
turn at 00:17:39 (`end_turn`, then `system/turn_duration`), and its status file still said `busy` with
`statusUpdatedAt` 00:03:57 at 00:23:36 (6 min after the turn ended), while this background subagent was still
working. This answers the
open question in `docs/research.md`. Not checked: background shell tasks, `/loop` timers.

## 2. Idle Claude Code sessions that will work soon

| Case | Nightcrew | LidGuard |
|---|---|---|
| Scheduled timers (`/loop`, cron, wakeups) | Not handled. Status goes `idle`, so it disarms 5 min later. Search for cron/loop/wakeup found nothing. V | Not handled. `Stop` ends the session; with the lid closed it suspends 10 s later. V |
| Background shell task (`run_in_background`) | Only indirect: a child of `claude.exe` above 10% CPU, or status still `busy` (U for shells), or transcript growth. A quiet download or `sleep` is missed unless status stays busy. V/U | Tracked: `PostToolUse` of `Bash`, `PowerShell`, `Agent`, legacy `Task` with `run_in_background: true`, and every `Monitor` call, are stored per session in `%LOCALAPPDATA%\LidGuard\claude-hook-work-state\`. `Stop` is deferred while any remain. Cleared by `TaskCompleted`, `TaskStop` tool, `<task-notification>` with status `completed`/`failed`/`stopped`, or transcript `system`/`task_notification` records (queued `queue-operation` records do not count). Scan of last 4096 lines / 4 MB. `Hooks/ClaudeHookWorkTracker.cs:13-14,84-92,298-306,332-374,571-611` V |
| Background subagents | Their transcripts grow (recursive scan) and status stays `busy` (seen here). V | `SubagentStart`/`SubagentStop` tracked; `Stop` deferred. V |
| Lost completion event | n/a (polls) | Issue #5: one lost event pinned lid Do nothing for 59 h. Fix `1efcd3e`: pending work also expires after 12 min with no activity. `Runtime/LidGuardRuntimeCoordinator.cs:575-578,696-733` V |
| Waiting for permission | `waiting` counts only with lid closed and setting on (default). **No time limit** except the 8 h lid-closed cap or 20% battery. V | Soft-lock: stops holding awake. With lid closed and no external screen, `PermissionRequest` is answered **Deny by default** (`interrupt: true`); options Allow, Ask. Elicitation is answered `cancel`. `Settings/LidGuardSettings.cs:62`, SKILL `provider-integrations:84,92,95` V |
| Long quiet foreground command | Covered by `busy` status (hardware checklist step 14). V | No hook fires during the command, so the 12 min timeout soft-locks it; with the lid closed it may suspend mid-command. U (inferred from code, not tested) |
| Usage limit wait | Expected `idle`, so not work (checklist step 16, a test plan, not a result). U | Not handled. |
| Remote Control (`bridgeSessionId`) | Not read. V | Not handled. V |
| Reply from phone | No | Optional "ask before sleep": Stop hook blocks, webhook to their notification server, reply is fed back as a continuation; window 180 s, +300 s extension. Out of scope for us. V |

## 3. Power settings safety

### 3.1 What each writes

| | Nightcrew | LidGuard |
|---|---|---|
| Power request | `PowerRequestSystemRequired` + `PowerRequestExecutionRequired`, only while armed (`Platform/KeepAwake.cs:39-70`, `Core/Engine.cs:157-160`) V | `PowerRequestSystemRequired` + `PowerRequestAwayModeRequired` by default, `DisplayRequired` optional (`Power/PowerRequestOptions.cs:9-18`, `Power/PowerRequestService.windows.cs:45-64`) V |
| Lid action | Do nothing, AC and DC, active plan | Do nothing, AC and DC, active plan (`Power/LidActionPolicyController.cs:9-45,60-75`) V |
| Sleep after | AC Never; DC = lid-closed limit + 1 h + 7 s (32407 s with the 8 h default) (`Core/RestoreJournal.cs:61-62`) V | **Never touched**: "Runtime crashes could leave the user's system policy in a dangerous state" (`.agents/skills/lidguard-power-runtime/SKILL.md:17`) V |
| When | Any detected work, lid open or closed | Any active session, lid open or closed (default `ChangeLidAction = true`, `Settings/LidGuardSettings.cs:24-28`, `Settings/LidGuardSettingsStore.cs:25`) V |
| Apply | `PowerSetActiveScheme` only if the plan is active, then read back and compare (`Platform/PowrProfPolicy.cs:25-36`) V | `PowerSetActiveScheme` on the **saved** plan, even if another is active (`Power/LidActionPolicyController.cs:74,91`, `Power/LidActionService.windows.cs:54-58`) V |

On this laptop right now (V): Nightcrew is armed because of this research session. `restore.json` holds owner values
lid 1/1, sleep 10800/600 and written values 0/0/0/32407 on plan `381b4222-...`.

### 3.2 Edge case checklist

| Edge case | Nightcrew | LidGuard |
|---|---|---|
| Save the user's values before the first write | Yes, record then write (`Core/RestoreJournal.cs:146-179`) V | Yes (`Runtime/LidGuardPendingLidActionBackupManager.cs:14-21`) V |
| Never re-capture over an existing saved copy | Yes (`Core/RestoreJournal.cs:150-177`) V | Yes since `75f2aca` (2026-05-22); before that each arm overwrote the backup. V (diff); real-world impact U |
| Atomic write | Temp file, `Flush(true)`, `MoveFileEx(REPLACE \| WRITE_THROUGH)` (`Platform/AtomicFile.cs:15-27`) V | Temp file + `File.Move`, no flush (`Runtime/LidGuardPendingLidActionBackupStore.cs:70-71`) V |
| Second copy | Registry `HKCU\Software\Nightcrew\Restore` (`Platform/RegistryRestoreBackup.cs:9`) V | None V |
| Damaged saved file (power cut leaves zeros) | Moved to `.corrupt`, then registry copy, then "Windows defaults if values look like ours" (`Core/RestoreJournal.cs:269-305,349-367`) V | `TryLoad` fails, so every arm fails **and every `lidguard` command fails**, hooks included (`...BackupStore.cs:42-46`, `Commands/LidGuardCommandLineApplication.cs:27-31`) V (code); not run |
| Recognise own values when all copies are lost | "+7 s" marker on DC timer (`Core/RestoreJournal.cs:65-69`); fails when the limit is off (README:72-76) V | No V |
| Write verified | Read-back compare, throws on mismatch (`Platform/PowrProfPolicy.cs:33-35`) V | No V |
| User changes a value while overridden | Each tick: if the plan no longer holds what was written, save the new value as the user's and write Do nothing again (`Core/Engine.cs:112-123`, `Core/RestoreJournal.cs:112-139`). Added in PR #14 after a real bug (see 5.1) V | Not detected; restore overwrites the user's change with the old value (`Power/LidActionPolicyController.cs:77-92`) V |
| Restore keeps fields the user changed | Per field: restore only fields that still hold what was written (`Core/RestoreJournal.cs:325-337`) V. Miss: if the user sets exactly the written value (for example Do nothing), it is reverted. V (code) | Writes all saved fields. V |
| Power plan switch while overridden | Detected each tick; old plan put back, new plan armed (`Core/Engine.cs:94-106`) V | Not handled: new plan sleeps on lid close; restore writes the old plan **and makes it active again**, undoing the user's switch (`Power/LidActionPolicyController.cs:91`) V (code); not run |
| Saved plan deleted | Record dropped (`Core/RestoreJournal.cs:311-317`) V | Restore write fails, backup stays. U |
| Policy lock (GPO) | `PowerSettingAccessCheckEx` for both settings, AC and DC, every 5 min; no arm when blocked; panel warns (`Platform/PowrProfPolicy.cs:58-75`, `Core/Engine.cs:194-209`) V | No pre-check; write fails and protection fails. V |
| Arm fails half way | Undo, back off 5 min (`Core/Engine.cs:181-190`) V | Roll back from backup (`...BackupManager.cs:24-27,68-81`) V |
| Restore fails | Record kept, retried every 60 s while not armed (`Core/Engine.cs:22,142-148`) V | Logged; retried only by the next CLI call. V |
| Quit | Restore with 10 s lock wait and 20 s deadline, then exit so the watchdog can finish (`App/TrayApp.cs:443-495`) V | n/a |
| Unhandled exception | Restore + restart (only after 10 min uptime) (`App/Recovery.cs:14-28`, `Program.cs:106-108,163-165`) V | Logged to `log/exceptions.log`; no restore. V |
| Hung check loop | Freeze guard: no finished tick for 3 min, two checks in a row, clock that stops in sleep (`Core/FreezeGuard.cs:9-38`, `App/TrayApp.cs:245-248`) V | None. V |
| Killed process | Scheduled task `--watchdog` at logon and every 5 min, 2 min limit, ignore new instances, runs on battery (`Core/WatchdogTaskXml.cs:19-37`, `App/Watchdog.cs:9-29`) V | Restored only on the **next `lidguard` command or hook** (`Commands/LidGuardCommandLineApplication.cs:27,274-337`). No agent use = lid stays Do nothing. V |
| Sign-out, shutdown | `WM_QUERYENDSESSION`: restore, block re-arm; `WM_ENDSESSION`: exit (`Platform/PowerEventsWindow.cs:57-69`, `App/TrayApp.cs:130-144`) V | Not handled (only Ctrl+C) (`Commands/LidGuardCommandLineApplication.cs:241`). V |
| Power loss, BSOD | Next start restores first (`Program.cs:116-127`); watchdog at logon V | Next CLI call. V |
| Uninstall | `--uninstall`: restore, remove Run key, task, registry key (`Program.cs:71-100`) V | `dotnet tool uninstall` has no hook; nothing restores. U (no code found) |
| Exe deleted without uninstall | Task stays and fails every 5 min (`README.md:173`) V | n/a |
| Lost "work ended" event | n/a (polls) | Was a 59 h leak (issue #5), now bounded by 12 min. V |
| Battery: Windows drops requests after sleep timeout + 5 min on Modern Standby | Handled by writing DC sleep-after (hardware checklist step 4) V (design); the Microsoft rule itself U | Not handled (does not touch the timer). Effect on battery U |
| Dry run | Yes, refuses if a real record is pending (`Program.cs:121-127`) V | No V |

## 4. Lid closed behavior

| | Nightcrew | LidGuard |
|---|---|---|
| Lid state | Hidden window, `RegisterPowerSettingNotification(GUID_LIDSWITCH_STATE_CHANGE)`; 0 closed, 1 open (`Platform/PowerEventsWindow.cs:34-40`, `Platform/PowerBroadcast.cs:8-22`) V | `PowerSettingRegisterNotification` with a callback, no window needed (`Power/LidStateSource.windows.cs:40,65-85`); CLI waits up to 500 ms for the first value (`Commands/LidGuardCommandLineApplication.cs:482`) V |
| Lock | On lid close while armed, and on arm if already closed and idle 5 min. Skipped when the user's own lid action is Do nothing (docked) (`Core/Controller.cs:83-86,100,107`) V | **Never locks.** No `LockWorkStation` in the code. With lid action Do nothing, a closed laptop stays unlocked. V |
| Docked detection | User's own lid action is Do nothing; "unattended" = lid closed and no input for 5 min (`Core/Controller.cs:79-80`) V | Count active external monitors: `SM_CMONITORS` cross-checked with WMI `root\wmi WmiMonitorConnectionParams` (`Active`, `VideoOutputTechnology`); internal = 6 LVDS, 11 eDP, 13 UDI embedded, 0x80000000; any external screen = no suspend (`Power/VisibleDisplayMonitorCountProvider.windows.cs:13-36,111`) V. Also skip when the saved lid action is Do nothing (`Runtime/LidGuardRuntimeCoordinator.cs:1931-1960`) V |
| Battery floor | Fixed setting, default 20%, from `GetSystemPowerStatus` (not Windows' low or critical levels); only lid closed and on battery; hibernate (`Core/Controller.cs:111-112`, `Core/Settings.cs:81`, `Platform/SystemPower.cs:104-112`) V | **None.** V |
| Time limit | 8 h of *unattended* time (lid closed + idle), per episode; hibernate (`Core/Controller.cs:108-113`) V | None. V |
| Heat | None V | Hibernate at 93 C (average of WMI `Win32_PerfFormattedData_Counters_ThermalZoneInformation`), polled every 10 s while lid closed with no external screen; Sleep if hibernate fails (`Settings/LidGuardSettings.cs:10`, `Power/SystemThermalInformation.windows.cs:9-10,64-67`, `Runtime/EmergencyHibernationThermalMonitor.cs:8`) V. On this laptop the class reads without admin: 1 zone, 323 K = 50 C. V |
| Work ends, lid closed | Disarm after 5 min grace. Then wait for 5 min with no input, then by the user's lid action: Sleep = screen off, and hibernate 5 min later if still awake and idle; Hibernate or Shut down = hibernate; Do nothing = nothing (`Core/Controller.cs:137-176`) V | After 10 s: re-check sessions, lid and screens, release protection right before the request, then Sleep (default) or Hibernate (`Runtime/LidGuardRuntimeCoordinator.cs:1225-1304`). Removed the "suspend when stopped" option in `1516604`: always suspend. V |
| Sleep on Modern Standby | Never calls Sleep; uses screen off or hibernate V | `SetSuspendState(false)` returns `ERROR_NOT_SUPPORTED` (50) on some Modern Standby PCs; then if `AoAc` it broadcasts `SC_MONITORPOWER` off (`Power/SystemSuspendService.windows.cs:18,40-67`, SKILL `power-runtime:69`) V |
| Hibernate | Checks `SystemS4` and `HiberFilePresent`; on failure turns the screen off (`Platform/SystemPower.cs:125-132`, `Core/Engine.cs:172-177`) V | `SetSuspendState(true,...)`; emergency path falls back to Sleep (`ad9327b`) V |
| Resume | `PBT_APMSUSPEND` sets a flag; next tick ends the "finishing" run so a late tick cannot hibernate a PC the user just opened (`Core/Engine.cs:81-85`, fix `b7d2377`) V. Clock `QueryUnbiasedInterruptTime` (`Platform/SystemClock.cs:8-18`) V | No suspend or resume handling. Timeouts use wall clock (`DateTimeOffset.UtcNow`), so sessions idle over 12 min soft-lock right after wake. V (code) |
| Re-check before suspend | Lid open ends finishing (`Core/Controller.cs:139-142`) V | Re-check after the reply wait (`7b015bc`) and before the request V |

## 5. Lessons from real use

### 5.1 Nightcrew (all 100 commits read; 1 issue, 13 PRs, 7 releases)

| What broke | Why | Fix | Ref |
|---|---|---|---|
| A late tick after wake could hibernate the laptop in front of the user | Timer tick fired after resume before the lid event was handled | `PBT_APMSUSPEND` flag; next tick ends "finishing" | `b7d2377` V |
| An error in the panel killed the app with settings still changed | WinForms loop does not catch WPF dispatcher exceptions | Log panel errors | `7c637a4` V |
| Every tick failed after a power cut | Zero-filled `restore.json` threw `XmlException`, only `SerializationException` was caught | Any parse error = corrupt, set aside | `b931310`, `68ba183` V |
| Plan switch while armed: new plan slept on lid close, old plan stuck on Do nothing | Record had no plan id | Record stores plan GUID; follow switches | `cd8fcb0` V |
| A lost record made Do nothing the user's "own" value | Re-capture read Nightcrew's own values | Detect own values, use Windows defaults | `787bee4` V |
| After crash + restart the logon screen could stay awake forever on battery | DC sleep-after was Never | DC = limit + 1 h | `787bee4` V |
| Docked users got locked | Lock ignored the user's lid action | No lock when the user's lid action is Do nothing | `787bee4` V |
| A frozen tick kept settings armed; battery floor never checked; watchdog saw a live process | One stuck tick blocked all later ticks | Freeze guard, recovery, restart | `7ccb678` V |
| Quit or recovery could hang | Waiting for a lock held by the stuck tick | Deadlines 10/20/30 s, then exit | `a7528ce` V |
| Own value hard to tell from the user's | Whole-minute values can match | "+7 s" marker | `a7528ce` V |
| `--exit` failed in a test; panel opened 5 s late | Second start waited on the mutex and could take over a quitting copy | Signal first, wait only if nobody answers | `ba16fc5` V |
| Games and video exports kept the laptop awake | Any process' GPU counted | Only the agent tree | `ed7680f` V |
| Registry copy thrown away after the user changed one value | Copy accepted only on an exact match | Accept when the +7 s marker is present | `f25e6c5` V |
| Lid action changed in Windows Settings during work never showed, and the plan kept it, so closing the lid slept mid-work | Panel read the arm-time record; nothing re-checked the plan | Re-check each tick, adopt as the user's, re-apply Do nothing | PR #14, `c387cb1` V |
| Tray looked "working" when it could not arm (policy block) | Icon followed detection, not the armed state | Icon lit only while armed; "keep the lid open" warning | `8f60eaa` V |
| Efficiency mode risk | Slowing other apps | Off by default in 0.3.1 | `5bc1451` V |

Other: issue #13 (Linux port offer); owner warns porters to read only `*.json` in `~/.claude/sessions`, never `.key`. V.
Hardware checklist gives a good test method: a 30 s PowerShell heartbeat file to find sleep gaps, and Kernel-Power
events 506/507 (Modern Standby in/out) and 42 (`TargetState` 5 = hibernate) (`docs/hardware-checklist.md:33-55`). V

### 5.2 LidGuard (all 222 commit subjects; 4 issues, 1 PR; commit bodies are mostly empty)

| What broke | Why | Fix | Ref |
|---|---|---|---|
| **Lid stayed Do nothing for 59 h; laptop got hot in a bag** | One lost Stop event left "pending work" set, which was exempt from the inactivity timeout | Pending work expires after 12 min with no activity; status shows it | issue #5, `1efcd3e` V |
| Hook process crashed 18 times, losing subagent events | Explicit JSON `null` in `agent_type` overwrote the C# default; `.Trim()` threw | Coerce null to empty in every field | issue #4, `b20fa40` V |
| Server fail-fast (crash dump) on every clean exit | Named `Mutex` released on another thread after `await` | Hold and release on one thread | issue #3, `5285e0d` V |
| WSL install failed on NixOS | `wsl.exe --exec true` needs an external `true` | Run through `sh -lc` | issue #1, PR #2 V |
| OpenCode 2 sessions never stopped | Plugin dropped events without a location | Track session ids per plugin | `9fff5c5` V |
| Closed laptop never suspended | `SM_CMONITORS` still counted the internal panel with the lid closed | WMI connection check, exclude internal | `9c5f2c2` V (diff); symptom U |
| Suspended a docked laptop | Did not consider the user's own Do nothing | Skip suspend when the saved lid action is Do nothing | `f3bac30`, `66fb2de` V |
| Saved values could be overwritten | Each arm re-captured | Capture once, reuse | `75f2aca` V (diff) |
| Slow hooks on every tool call | First-chance exception logging; 750 ms pipe waits | Logging opt-in; 50 ms probe | `609be16` V (diff) |
| Sessions stuck after the user pressed Esc | No Stop event on interrupt | Detect interrupt markers in transcripts | `54d8893`, Claude rules in `AgentTranscriptMonitor.cs:541` V (code); cause U |
| Suspend after the user opened the lid during a wait | No re-check | Re-check before suspend | `7b015bc` V |
| Process found by working directory was unreliable | Matching by cwd and command line | Walk the hook's own parent chain | `d8a49f5` V (code); cause U |

## 6. Licenses

| Repo | License | Holder | Port with attribution? | Mark |
|---|---|---|---|---|
| Nightcrew | MIT (`LICENSE`) | "Vilius (William) Leikauskas" (`f501e40`: "license under my own name"). Bundled fonts are SIL OFL 1.1 (`THIRD-PARTY-NOTICES.md:3`) | Yes. MIT allows copy and change in an MIT project. If we translate code closely, keep their copyright and license text in our `THIRD-PARTY-NOTICES`. Ideas and constants need no notice. Do not take the fonts or Wispr-style art. | V (license text); legal reading U (not legal advice) |
| LidGuard | MIT (`LICENSE.txt`, `LidGuard/LidGuard.csproj:56`) | `LICENSE.txt` still has the placeholder "Copyright (c) [year] [fullname]"; README names Howon Lee (airtaxi) | Yes, same as above. Credit "Howon Lee (airtaxi)" since the file names nobody. | V (text); legal reading U |

Both say they were built with AI help (Nightcrew commits carry Claude co-author lines; LidGuard README: "Built with
help from OpenAI Codex"). No effect on the license. V

## Port to Wakeshot

1. **Status file parsing rules** (Nightcrew): only `^\d+\.json$`, max 64 files, max 64 KB, share read/write/delete,
   skip unreadable files, warn once; `procStart` must exactly equal the process creation FILETIME and the process
   must have no exit time. Treat `busy` as work. (`ClaudeSessionSignal.cs:69-82`, `SessionFileReader.cs:14-40`)
2. **`busy` already covers background subagents** (seen here). Before adding transcript or CPU signals for Claude,
   test background shells and `/loop`; maybe status files alone are enough for Claude.
3. **Bound every "still working" signal by time.** LidGuard's 59 h leak came from one signal with no expiry. Give
   "busy" with no transcript growth a long cap, and drop Remote Control waiting after the planned limit. Not adopted
   for `busy` (decision 3, 2026-11-03: signals are checked against facts). Kept: Remote Control waiting ends after
   1 hour (`product.md:33`).
4. **Re-check the plan every tick while overriding** (Nightcrew PR #14): a changed value is the user's new value;
   re-apply ours; a plan switch moves the override. If we use a temporary plan instead, the same check must notice the
   user picking another plan.
5. **Per-field restore**: restore a field only if it still holds what we wrote (`RestoreJournal.cs:325-337`).
6. **Capture once; never overwrite a pending saved copy** (both learned this the hard way).
7. **Damaged saved file must never block start, arm or restore**: set it aside and fall back (Nightcrew), unlike
   LidGuard where it breaks every command.
8. **Atomic write with flush** (`AtomicFile.cs`), read-back after each power write (`PowrProfPolicy.cs:33-35`).
9. **Policy lock check** with `PowerSettingAccessCheckEx` for both settings, AC and DC, before arming.
10. **Recovery paths**: restore on `WM_QUERYENDSESSION`; restore on next start; a logon + 5 min scheduled task with
    `--watchdog` that restores only if the app's mutex is free; freeze guard on an unbiased clock.
11. **Lid state without a window**: `PowerSettingRegisterNotification` with `DEVICE_NOTIFY_CALLBACK` (LidGuard). The
    first callback gives the current state.
12. **Docked check by external screen** (LidGuard's WMI `WmiMonitorConnectionParams` rule) in addition to the user's
    lid action Do nothing. Do not trust `SM_CMONITORS` alone with the lid closed.
13. **Never call Sleep on Modern Standby**; expect `ERROR_NOT_SUPPORTED`; use screen off (`SC_MONITORPOWER` 2) to
    enter standby, or hibernate after checking `SystemS4` and `HiberFilePresent`.
14. **Handle `PBT_APMSUSPEND`** so a late timer after wake cannot hibernate a PC the user just opened.
15. **Heat guard** (LidGuard): read `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (works without admin on
    this laptop) and hibernate above a fixed threshold while lid-closed. Cheap, and it addresses the "hot in a bag"
    report. Threshold needs testing; ACPI zones may lag CPU heat (U). Not adopted (decision 2, 2026-11-03: no heat
    guard).
16. **Re-check lid, screens and work right before any suspend**, and release our hold just before the request.
17. **Tolerate odd input**: JSON `null` where a string is expected, unknown fields, unknown `status` values.
18. **Test method**: heartbeat file + Kernel-Power events 506/507/42 from Nightcrew's hardware checklist.
19. **Codex later**: transcript growth in `~\.codex\sessions`, `turn_aborted` = stop, pending `request_user_input` =
    waiting (LidGuard rules), if we add Codex without hooks.

## Avoid

1. **Answering the agent's permission prompts** (LidGuard denies them by default with the lid closed). Wakeshot must
   never change agent behavior.
2. **Hooks and edits to agent configs** for Claude: 14 hooks, a process start per tool call, hook timeouts, and a
   failure mode where a bad backup file makes every hook fail.
3. **Inactivity timeout alone** for "work ended" (LidGuard 12 min): it can stop a long quiet command.
4. **Restore that activates the saved plan** (LidGuard): undoes the user's plan switch.
5. **Crash recovery that waits for the next agent run** (LidGuard): a closed lid can then do nothing for days.
6. **No lock and no battery floor with lid Do nothing** (LidGuard).
7. **Not touching the battery sleep timer** while relying on the request (LidGuard), if Microsoft's "requests end 5
   min after the DC timeout" rule holds (U; test on this laptop).
8. **"Waiting for approval" with no time limit** (Nightcrew holds up to 8 h).
9. **Counting transcript files as sessions** (Nightcrew shows subagents as sessions).
10. **CPU and GPU heuristics** as main signals (Nightcrew); a busy dev server or a game can hold the PC awake.
11. **Settings for safety rules** (Nightcrew's floor, cap, lock switch; LidGuard has about 30 settings). Both teams
    removed options later (`1516604`, `5bc1451`, tray style `dc1cc78`).
12. **Watchdog task that outlives a deleted exe** (Nightcrew README:173): register it from the installer and remove
    it on uninstall, or make the task check that the exe exists.
13. **A named mutex held across `await`** or any thread-affine lock with async code (LidGuard #3).
