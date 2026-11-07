# Rivals deep dive: LidGuard

Collected on 2026-11-03 on the test laptop.

Marks: **V** = verified (seen in code, docs, git history or on the test laptop). **U** = unverified (claim, inference or
guess). Code references are `repo/path:line`.

## Sources

| Repo | Commit read | Date | Commits | Notes |
|---|---|---|---|---|
| `lidguard` = github.com/airtaxi/LidGuard | `3ff7893` | 2026-09-28 | 222 | NuGet `lidguard` 2.1.7. No GitHub releases. Extra branch `codex/disable-codex-fallback-watchdog`. V |

Read: all source for detection, power, lid and lifecycle; the README; LidGuard's `.agents/skills/*/SKILL.md`
(LLM-written design maps, very detailed); every issue and PR (4 issues, 1 PR); every commit subject and every fix
commit body. Also read (read only) the Claude Code status files on the test laptop.

The repo was cloned to a scratch folder. Nothing was built or run.

## 1. Agent detection

### 1.1 Approach in one line

- **LidGuard: hooks per agent, plus helpers.** Each agent calls `lidguard <provider>-hook` on its events; the hook
  talks over a named pipe to a `lidguard run-server` process it starts on demand. The server also watches the
  agent's parent process and its transcript file. `lidguard/LidGuard/Commands/LidGuardCommandLineApplication.cs:35-38`,
  `Ipc/LidGuardRuntimeClient.cs:12-15`. V

### 1.2 LidGuard, per agent

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

### 1.3 Claude Code status file today (the test laptop, Claude Code 2.1.314 and 2.1.316)

Fields seen (V): `pid`, `sessionId`, `cwd`, `startedAt`, `procStart`, `version`, `peerProtocol`, `peerFeatures`,
`kind` (`interactive`), `entrypoint` (`cli`, `claude-vscode`), `pidDomain` (`win32:<host>`), `asksForConvertedSkills`,
`messagingSocketPath`, `name`, `nameSource`, `nameSince`, `status` (`busy`, `idle`), `updatedAt`, `statusUpdatedAt`
(ms epoch), `bridgeSessionId`.

**`busy` stays set after the turn ends while background subagents run.** V. This session's main transcript ended its
turn at 00:17:39 (`end_turn`, then `system/turn_duration`), and its status file still said `busy` with
`statusUpdatedAt` 00:03:57 at 00:23:36 (6 min after the turn ended), while this background subagent was still
working. This answers the
open question in `docs/research.md`. Not checked: background shell tasks, `/loop` timers.

## 2. Idle Claude Code sessions that will work soon

| Case | LidGuard |
|---|---|
| Scheduled timers (`/loop`, cron, wakeups) | Not handled. `Stop` ends the session; with the lid closed it suspends 10 s later. V |
| Background shell task (`run_in_background`) | Tracked: `PostToolUse` of `Bash`, `PowerShell`, `Agent`, legacy `Task` with `run_in_background: true`, and every `Monitor` call, are stored per session in `%LOCALAPPDATA%\LidGuard\claude-hook-work-state\`. `Stop` is deferred while any remain. Cleared by `TaskCompleted`, `TaskStop` tool, `<task-notification>` with status `completed`/`failed`/`stopped`, or transcript `system`/`task_notification` records (queued `queue-operation` records do not count). Scan of last 4096 lines / 4 MB. `Hooks/ClaudeHookWorkTracker.cs:13-14,84-92,298-306,332-374,571-611` V |
| Background subagents | `SubagentStart`/`SubagentStop` tracked; `Stop` deferred. V |
| Lost completion event | Issue #5: one lost event pinned lid Do nothing for 59 h. Fix `1efcd3e`: pending work also expires after 12 min with no activity. `Runtime/LidGuardRuntimeCoordinator.cs:575-578,696-733` V |
| Waiting for permission | Soft-lock: stops holding awake. With lid closed and no external screen, `PermissionRequest` is answered **Deny by default** (`interrupt: true`); options Allow, Ask. Elicitation is answered `cancel`. `Settings/LidGuardSettings.cs:62`, SKILL `provider-integrations:84,92,95` V |
| Long quiet foreground command | No hook fires during the command, so the 12 min timeout soft-locks it; with the lid closed it may suspend mid-command. U (inferred from code, not tested) |
| Usage limit wait | Not handled. |
| Remote Control (`bridgeSessionId`) | Not handled. V |
| Reply from phone | Optional "ask before sleep": Stop hook blocks, webhook to their notification server, reply is fed back as a continuation; window 180 s, +300 s extension. Out of scope for us. V |

## 3. Power settings safety

### 3.1 What it writes

| | LidGuard |
|---|---|
| Power request | `PowerRequestSystemRequired` + `PowerRequestAwayModeRequired` by default, `DisplayRequired` optional (`Power/PowerRequestOptions.cs:9-18`, `Power/PowerRequestService.windows.cs:45-64`) V |
| Lid action | Do nothing, AC and DC, active plan (`Power/LidActionPolicyController.cs:9-45,60-75`) V |
| Sleep after | **Never touched**: "Runtime crashes could leave the user's system policy in a dangerous state" (`.agents/skills/lidguard-power-runtime/SKILL.md:17`) V |
| When | Any active session, lid open or closed (default `ChangeLidAction = true`, `Settings/LidGuardSettings.cs:24-28`, `Settings/LidGuardSettingsStore.cs:25`) V |
| Apply | `PowerSetActiveScheme` on the **saved** plan, even if another is active (`Power/LidActionPolicyController.cs:74,91`, `Power/LidActionService.windows.cs:54-58`) V |

### 3.2 Edge case checklist

| Edge case | LidGuard |
|---|---|
| Save the user's values before the first write | Yes (`Runtime/LidGuardPendingLidActionBackupManager.cs:14-21`) V |
| Never re-capture over an existing saved copy | Yes since `75f2aca` (2026-05-22); before that each arm overwrote the backup. V (diff); real-world impact U |
| Atomic write | Temp file + `File.Move`, no flush (`Runtime/LidGuardPendingLidActionBackupStore.cs:70-71`) V |
| Second copy | None V |
| Damaged saved file (power cut leaves zeros) | `TryLoad` fails, so every arm fails **and every `lidguard` command fails**, hooks included (`...BackupStore.cs:42-46`, `Commands/LidGuardCommandLineApplication.cs:27-31`) V (code); not run |
| Recognise own values when all copies are lost | No V |
| Write verified | No V |
| User changes a value while overridden | Not detected; restore overwrites the user's change with the old value (`Power/LidActionPolicyController.cs:77-92`) V |
| Restore keeps fields the user changed | Writes all saved fields. V |
| Power plan switch while overridden | Not handled: new plan sleeps on lid close; restore writes the old plan **and makes it active again**, undoing the user's switch (`Power/LidActionPolicyController.cs:91`) V (code); not run |
| Saved plan deleted | Restore write fails, backup stays. U |
| Policy lock (GPO) | No pre-check; write fails and protection fails. V |
| Arm fails half way | Roll back from backup (`...BackupManager.cs:24-27,68-81`) V |
| Restore fails | Logged; retried only by the next CLI call. V |
| Unhandled exception | Logged to `log/exceptions.log`; no restore. V |
| Hung check loop | None. V |
| Killed process | Restored only on the **next `lidguard` command or hook** (`Commands/LidGuardCommandLineApplication.cs:27,274-337`). No agent use = lid stays Do nothing. V |
| Sign-out, shutdown | Not handled (only Ctrl+C) (`Commands/LidGuardCommandLineApplication.cs:241`). V |
| Power loss, BSOD | Next CLI call. V |
| Uninstall | `dotnet tool uninstall` has no hook; nothing restores. U (no code found) |
| Lost "work ended" event | Was a 59 h leak (issue #5), now bounded by 12 min. V |
| Battery: Windows drops requests after sleep timeout + 5 min on Modern Standby | Not handled (does not touch the timer). Effect on battery U |
| Dry run | No V |

## 4. Lid closed behavior

| | LidGuard |
|---|---|
| Lid state | `PowerSettingRegisterNotification` with a callback, no window needed (`Power/LidStateSource.windows.cs:40,65-85`); CLI waits up to 500 ms for the first value (`Commands/LidGuardCommandLineApplication.cs:482`) V |
| Lock | **Never locks.** No `LockWorkStation` in the code. With lid action Do nothing, a closed laptop stays unlocked. V |
| Docked detection | Count active external monitors: `SM_CMONITORS` cross-checked with WMI `root\wmi WmiMonitorConnectionParams` (`Active`, `VideoOutputTechnology`); internal = 6 LVDS, 11 eDP, 13 UDI embedded, 0x80000000; any external screen = no suspend (`Power/VisibleDisplayMonitorCountProvider.windows.cs:13-36,111`) V. Also skip when the saved lid action is Do nothing (`Runtime/LidGuardRuntimeCoordinator.cs:1931-1960`) V |
| Battery floor | **None.** V |
| Time limit | None. V |
| Heat | Hibernate at 93 C (average of WMI `Win32_PerfFormattedData_Counters_ThermalZoneInformation`), polled every 10 s while lid closed with no external screen; Sleep if hibernate fails (`Settings/LidGuardSettings.cs:10`, `Power/SystemThermalInformation.windows.cs:9-10,64-67`, `Runtime/EmergencyHibernationThermalMonitor.cs:8`) V. On the test laptop the class reads without admin: 1 zone, 323 K = 50 C. V |
| Work ends, lid closed | After 10 s: re-check sessions, lid and screens, release protection right before the request, then Sleep (default) or Hibernate (`Runtime/LidGuardRuntimeCoordinator.cs:1225-1304`). Removed the "suspend when stopped" option in `1516604`: always suspend. V |
| Sleep on Modern Standby | `SetSuspendState(false)` returns `ERROR_NOT_SUPPORTED` (50) on some Modern Standby PCs; then if `AoAc` it broadcasts `SC_MONITORPOWER` off (`Power/SystemSuspendService.windows.cs:18,40-67`, SKILL `power-runtime:69`) V |
| Hibernate | `SetSuspendState(true,...)`; emergency path falls back to Sleep (`ad9327b`) V |
| Resume | No suspend or resume handling. Timeouts use wall clock (`DateTimeOffset.UtcNow`), so sessions idle over 12 min soft-lock right after wake. V (code) |
| Re-check before suspend | Re-check after the reply wait (`7b015bc`) and before the request V |

## 5. Lessons from real use

All 222 commit subjects read; 4 issues, 1 PR; commit bodies are mostly empty.

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
| LidGuard | MIT (`LICENSE.txt`, `LidGuard/LidGuard.csproj:56`) | `LICENSE.txt` still has the placeholder "Copyright (c) [year] [fullname]"; README names Howon Lee (airtaxi) | Yes. MIT allows copy and change in an MIT project. If we translate code closely, keep their copyright and license text in our `THIRD-PARTY-NOTICES`. Ideas and constants need no notice. Credit "Howon Lee (airtaxi)" since the file names nobody. | V (text); legal reading U (not legal advice) |

LidGuard says it was built with AI help (README: "Built with help from OpenAI Codex"). No effect on the license. V

## Port to Wakeshot

1. **`busy` already covers background subagents** (seen here). Before adding transcript or CPU signals for Claude,
   test background shells and `/loop`; maybe status files alone are enough for Claude.
2. **Bound every "still working" signal by time.** LidGuard's 59 h leak came from one signal with no expiry. Give
   "busy" with no transcript growth a long cap, and drop Remote Control waiting after the planned limit. Not adopted
   for `busy` (decision 3: signals are checked against facts). Kept: Remote Control waiting ends after 1 hour
   (`product.md`).
3. **Capture once; never overwrite a pending saved copy** (LidGuard learned this the hard way, `75f2aca`).
4. **Damaged saved file must never block start, arm or restore**: set it aside and fall back, unlike LidGuard where it
   breaks every command.
5. **Lid state without a window**: `PowerSettingRegisterNotification` with `DEVICE_NOTIFY_CALLBACK` (LidGuard). The
   first callback gives the current state.
6. **Docked check by external screen** (LidGuard's WMI `WmiMonitorConnectionParams` rule) in addition to the user's
   lid action Do nothing. Do not trust `SM_CMONITORS` alone with the lid closed.
7. **Never call Sleep on Modern Standby**; expect `ERROR_NOT_SUPPORTED`; use screen off (`SC_MONITORPOWER` 2) to
   enter standby.
8. **Heat guard** (LidGuard): read `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (works without admin on
   the test laptop) and hibernate above a fixed threshold while lid-closed. Cheap, and it addresses the "hot in a bag"
   report. Threshold needs testing; ACPI zones may lag CPU heat (U). Not adopted (decision 2: no heat guard).
9. **Re-check lid, screens and work right before any suspend**, and release our hold just before the request.
10. **Tolerate odd input**: JSON `null` where a string is expected, unknown fields, unknown `status` values.
11. **Codex later**: transcript growth in `~\.codex\sessions`, `turn_aborted` = stop, pending `request_user_input` =
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
   min after the DC timeout" rule holds (U; test on the test laptop).
8. **Settings for safety rules** (LidGuard has about 30 settings, and later removed options, `1516604`).
9. **A named mutex held across `await`** or any thread-affine lock with async code (LidGuard #3).
