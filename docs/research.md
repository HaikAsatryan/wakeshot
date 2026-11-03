# Research

Collected on 2026-11-02 on the Windows laptop. Each fact says how it was checked. "Not verified" means read
somewhere or remembered, not tested.

## The problem

A laptop sleeps in the middle of long work: an AI agent run, a build, a download. People either change power
settings by hand (and forget to change them back) or use a keep-awake tool that stays on all the time.

### Claude Code today

- **Windows CLI holds nothing.** In the installed `claude.exe` 2.1.316 the sleep inhibitor only returns spawn args on
  macOS (`caffeinate -i -t 300`) and `null` everywhere else. Checked in the binary. Open issue: anthropics/claude-code#96679.
- **macOS CLI** spawns `caffeinate -i -t 300`, restarts it every 240 s while work runs, and releases 30 s after work
  ends. Checked in the binary (constants 300, 240000, 30000). Users complain it sometimes holds while idle (#21432).
- **Desktop app** has "Keep computer awake while Claude works". On Windows it takes only an EXECUTION power request,
  so Modern Standby laptops still sleep mid-task (#103669, open). Not verified by us.
- **Docs** (code.claude.com, desktop scheduled tasks): "Closing the laptop lid still puts it to sleep." Remote Control
  has a desktop setting "Keep this computer awake for Remote Control"; #100743 says it holds only in 5 minute windows.

### Windows power facts

- **Modern Standby.** This laptop has only "Standby (S0 Low Power Idle) Network Connected", no S3. Checked with
  `powercfg /a`. Most laptops from the last years are the same.
- **Request types.** `PowerRequestSystemRequired` blocks idle sleep. `PowerRequestExecutionRequired` alone does not
  block idle sleep on Modern Standby. Source: the PowerSetRequest docs (re-read 2026-11-03): SystemRequired "The
  system continues to run instead of entering sleep after a period of user inactivity"; ExecutionRequired "On
  Traditional Sleep (S3) systems, an active PowerRequestExecutionRequired request implies
  PowerRequestSystemRequired", so not on Modern Standby. Not tested by us.
- **Battery.** PowerSetRequest docs, Remarks (re-read 2026-11-03): "On Modern Standby systems on DC power, system and
  execution required power requests are terminated 5 minutes after the system sleep timeout has expired." Tested on
  this laptop: it holds (slept 380 s after the last input with a 60 s battery timeout and the requests held; "Power
  settings spike" below). So a tool must also change the sleep timer on battery. Battery sleep Never stopped the
  rule in a 16 minute test, lid open (same section).
- **Lid close.** It counts as the user asking for sleep. No power request can block it. The only way is to set the lid
  action to "Do nothing". PowerSetRequest docs (re-read 2026-11-03): "power requests are terminated upon
  user-initiated system sleep entry (power button, lid close or selecting Sleep from the Start menu)", except Away
  mode on S3. SetThreadExecutionState docs: it "cannot be used to prevent the user from putting the computer to
  sleep". Tested here for Start > Sleep with a request held: the request was not honored after resume ("Lid and sleep
  spike", morning results, "Resume"). Not tested here with a request held: lid close with action Sleep, power button.
- **Electron.** `powerSaveBlocker` "prevent-app-suspension" maps to `PowerRequestExecutionRequired` only, so Electron
  apps that use it do not block idle sleep on Modern Standby. Checked in Chromium
  `services/device/wake_lock/power_save_blocker/power_save_blocker_win.cc`. "prevent-display-sleep" adds
  `PowerRequestSystemRequired`, but keeps the screen on.
- **Power setting IDs.**
  - Lid action: subgroup `4f971e89-eebd-4455-a8de-9e59040e7347`, setting `5ca83367-6e45-459f-a27b-476b1d01c936`.
    Values: 0 Do nothing, 1 Sleep, 2 Hibernate, 3 Shut down. It is a hidden setting: read it with `powercfg /qh`.
  - Sleep after: subgroup `238c9fa8-0aad-41ed-83f4-97be242c8f20`, setting `29f6c1db-86da-48c5-9fdb-f2b67b1f44da`.
    Seconds, 0 = never.
- **No admin needed.** A normal user can write both settings on the active plan with `PowerWriteACValueIndex` /
  `PowerWriteDCValueIndex` and apply them with `PowerSetActiveScheme`. Tested with an unelevated Medium token: see
  "Power settings spike" below. `PowerSettingAccessCheckEx` reports a plain access denial too, not only a policy.
- `powercfg /requests` needs an admin prompt.

### How to know Claude Code is working

- **Status files.** Claude Code writes `~/.claude/sessions/<pid>.json` per running session. Seen fields: `pid`,
  `procStart` (Windows FILETIME as a string), `status` (`busy`, `idle`, `waiting`, `shell`), `cwd`,
  `kind`, `entrypoint` (`cli`, `claude-vscode`, `sdk-cli`), `statusUpdatedAt`, `bridgeSessionId` (Remote Control
  session id; `null` after a disconnect, seen live). Checked on this machine. The format is undocumented and can change in
  any release.
  - A file can be left behind by a crash: count it only if the pid is alive and its start time equals `procStart`.
  - Never read the `<pid>.<hash>.key` files next to them. They hold a peer token.
  - Not verified: whether desktop app sessions write these files (app not installed).
  - Live tests 2026-11-03 (Claude Code 2.1.316, test sessions driven by `spikes/claude-watch`; details and the
    results table in `docs/research/claude-code-signals.md`):
    - Terminal sessions: `shell` while a background shell or shell Monitor runs and the turn is over; `busy` stays on
      while background subagents run. `waiting` comes with `permission prompt`, `input needed` or `dialog open` (a
      slash command dialog, not a real block).
    - SDK path (VS Code, `claude -p` stream-json): no `shell`. A background shell shows `idle`; only the transcript
      shows it. A background subagent keeps `busy`.
    - Timers leave the status `idle`. The transcript shows them: `CronCreate` (`nextFireAt`, a float in ms) or
      `ScheduleWakeup` (`scheduledFor`, rounded up to a whole minute, no job id), then `scheduled_task_fire` lines. An
      Esc cancel of a wakeup writes nothing. `/clear` changes `sessionId` but jobs keep firing; the new transcript
      starts with a `session_cron_carry` line listing them.
    - Durable cron jobs are off in this build ("durable persistence is not available"), so no
      `scheduled_tasks.json`.
    - A hard kill leaves the `.json`; the pid plus start time check drops it at once, and the next Claude Code start
      deletes it. Remote Control off writes `bridgeSessionId: null`.
  - Live shapes and the resume test 2026-11-03 (Claude Code 2.1.317, card `p4-claude-live-shapes`; details, marks and
    rule changes in `docs/research/claude-code-signals.md`, "Phase 4 live shapes"; redacted raw lines in
    `crates/wakeshot-core/tests/fixtures/claude/live/`):
    - `claude --resume` in a new process: new pid, same `sessionId`, the same transcript grows, and no line marks the
      resume. `CronCreate` jobs come back and fire (a `*/5` job at resume plus 285 s, not its old `nextFireAt`);
      `ScheduleWakeup` wakeups do not. Same after a clean exit and a hard kill. Seen live.
    - The SDK path resumed after a hard kill writes a `<task-notification>` with `<status>stopped</status>` for a
      background task left open. A subagent's notification can come as an `attachment` `queued_command` instead of a
      user line. Monitor event notifications have no `<status>`. Seen live, one case each.
    - On the SDK path, closing stdin does not end the process while a background shell runs. Seen live once.
- **Hooks.** `UserPromptSubmit`, `PreToolUse`, `Stop`, `SessionEnd` in `~/.claude/settings.json`. Exact, but means
  editing the user's settings. Insomnia does this.
- **Transcripts.** `~/.claude/projects/**/*.jsonl` grow while a session works. Nightcrew watches their size.

### Reading elevated Claude Code sessions from an unelevated process

Checked 2026-11-03 (card `p0-unelevated-liveness`, Claude Code 2.1.316) with `spikes/claude-watch --probe` and
`--wait-exit`. Logs in `spikes/claude-watch/out/unelevated-liveness/` (user SID masked).

- **How the levels were checked.** Reader: the probe started by Explorer from a `.cmd` (opened with `explorer.exe`, no
  parent spoofing). Its own token (`GetTokenInformation`): Medium `S-1-16-8192`, elevation type Limited; `whoami
  /groups` in the same `.cmd`: Medium Mandatory Level. Targets: the token of each `claude.exe`, read with
  `OpenProcessToken`: High `S-1-16-12288`, elevation type Full. Targets were Haik's sessions 20636 and 32332 (CLI,
  read only), my own test sessions over stream-json (one clean exit, one hard kill), and an elevated `pwsh` as a
  stand-in for an elevated `wakeshot run`. Both sides had the same logon SID (`S-1-5-5-0-930782`).
- **Medium reader, High `claude.exe`: everything the design needs works.**
  - Status files read fine. Files written by the High process carry only inherited ACEs (SYSTEM, Administrators,
    haika: full) and no mandatory label (`icacls`). Same for transcripts.
  - `OpenProcess` with `PROCESS_QUERY_LIMITED_INFORMATION`, with `SYNCHRONIZE`, and with both: ok.
    `PROCESS_QUERY_INFORMATION`, `PROCESS_VM_READ` and `READ_CONTROL`: access denied (`0x80070005`). So ask for
    exactly `QUERY_LIMITED_INFORMATION | SYNCHRONIZE`: one extra right fails the whole open.
  - On that handle: `GetProcessTimes` creation time equals `procStart` for every session; `QueryFullProcessImageNameW`
    gives `C:\Users\haika\.local\bin\claude.exe`; `GetExitCodeProcess` gives `STILL_ACTIVE`.
  - Wait on the handle: a 1 s `WaitForSingleObject` times out while the session runs. Clean exit of a High test
    session (its stdin closed): the Medium wait signaled 0.4 s after the driver stopped, exit code 0. Hard kill
    (`Stop-Process -Force` on the High `claude.exe`): signaled 24 ms after, exit code `0xFFFFFFFF`.
  - `OpenProcessToken(TOKEN_QUERY)` on the High process also works from Medium (shows High, Full).
  - The system process list (`NtQuerySystemInformation(SystemProcessInformation)`) works at Medium and its create
    time equals `procStart`. No handle needed. One call took about 5 to 9 ms here (log timestamps).
  - Transcript `projects\<folder>\<sessionId>.jsonl` read fine (the 2 MB orchestrator transcript).
  - Change notifications: `ReadDirectoryChangesW` on `~/.claude/sessions` (opened with `FILE_LIST_DIRECTORY`) works at
    Medium and reports writes by the High process: `<pid>.json` added, modified, removed. A status write with
    `updatedAt` 03:44:36.544 was reported at 03:44:36.549; the removal at exit 8 ms after the driver stopped. The
    `.key` file shows up too (added, modified, renamed from a temp name, removed); its name was not logged.
  - The elevated `pwsh` stand-in gave the same results, so an elevated `wakeshot run` can be watched the same way.
- **Why the open works** (inference from the security descriptor, not checked in the docs). Read from the High side:
  `O:BA D:(A;;0x1fffff;;;BA)(A;;0x1fffff;;;SY)(A;;0x121411;;;<logon SID>) S:(ML;;NWNR;;;HI)`. The logon SID ACE grants
  `0x121411` (SYNCHRONIZE, READ_CONTROL, QUERY_LIMITED_INFORMATION, QUERY_INFORMATION, VM_READ, TERMINATE). The High
  label with no-read-up seems to remove the read rights (QUERY_INFORMATION, VM_READ, READ_CONTROL) for a lower
  caller, which matches the denied set exactly.
- **Gone vs no access.** After the hard kill the `<pid>.json` stayed. At Medium every `OpenProcess` on that pid
  failed with `0x80070057` (invalid parameter: no such process) and the pid was not in the system process list. A
  denial is `0x80070005`, so a reader can tell "gone" from "no access". The next Claude Code start deleted the file.
- **Medium reader, Medium `claude.exe`** (the normal case for other users): all six rights open and every check
  works. Security descriptor `O:<user SID> D:(A;;0x1fffff;;;<user SID>)(A;;0x1fffff;;;SY)(A;;0x121411;;;<logon SID>)
  S:(ML;;NWNR;;;ME)`. `ReadDirectoryChangesW` reported the new file and each status write (the `busy` write came
  0.26 s after the prompt was handed to the driver); the wait signaled 0.6 s after the driver stopped. A High reader (an elevated tool) also sees a Medium session fully.
- **Not verified.** Elevation with another account's password (a standard user typing an admin's credentials): the
  elevated process then runs as another user with another logon SID, and the open would likely fail with
  `0x80070005`. Also not checked: sessions in another Windows session (RDP, fast user switching), the Desktop app.
- **Workaround if the open is ever denied.** Use the system process list: create time equals `procStart` means
  alive; pid missing or another create time means dead. Cost: about 5 to 9 ms per call, and no handle to wait on, so
  an exit is seen only at the next poll (the safety tick), not at once.

## Competitors

| Tool | Platform | Tech | Agent aware | Blocks idle sleep on Modern Standby | Lid close | Stars |
|---|---|---|---|---|---|---|
| PowerToys Awake (microsoft/PowerToys) | Windows | C#, .NET 10, part of PowerToys | No: manual, timer or process id | Not checked | No | 140,491 (all of PowerToys) |
| LidGuard (airtaxi/LidGuard) | Windows, Linux, macOS | C#, .NET 10 NativeAOT, a `dotnet tool` | Codex, Claude Code, Copilot CLI, OpenCode (hooks; officially tested: Codex on Windows) | Not checked | Yes, optional | 13 |
| Agents Don't Sleep (itsmbaqer/agents-dont-sleep) | macOS, Windows, Linux | Rust, Tauri 2, web UI | 8 agents through hooks, others by process name (tested: Claude Code) | Not checked | Yes | 10 |
| Nightcrew (leikauskas/nightcrew) | Windows | C#, .NET Framework 4.8, WPF | Claude Code, Codex | Yes | Yes | 9 |
| Insomnia (stanley-projects/Insomnia) | Windows | Electron | Claude Code, Codex (hooks); apps by process | No (see Electron above) | No | 7 |
| Insomnia (krishhgg/Insomnia) | macOS | | | | Yes | 1,894 (2026-11-02) |

Stars read with `gh api` on 2026-11-04 (`docs/research/launch.md`, part 1, which has the facts and links per tool),
except the macOS Insomnia (2026-11-02). Rivals for the README: PowerToys Awake, LidGuard and Agents Don't Sleep;
Nightcrew is the reserve, one star behind (same file, "Pick: three rivals"; decided by the worker, for Haik's review).
Many more tiny tools exist (0 to 30 stars): LidKeeper (a PowerShell script, no release), LidAwake, lidwake, LidGuard
(macOS), wakekeeper (Windows and macOS), StayLit, WideAwake, Caffeinator, DozeGuard, a claude-kit keep-awake plugin,
the claudfeine wrapper.

**Demand:** real on macOS (1.9k stars for a lid keep-awake app). On Windows every agent-aware tool found is small (7 to
13 stars) and young. The space is busy with small projects. Simple, native and correct on Modern Standby is the open
spot.

### Nightcrew review (code read 2026-11-02, v0.3.1 installed and tested on this laptop)

What it does: every 10 s it checks Claude Code status files, transcript growth, CPU of processes an agent started,
and GPU use. While work runs it holds SYSTEM and EXECUTION requests, sets lid action to Do nothing on AC and DC, sets
sleep after to Never on AC and to "lid-closed limit + 1 h + 7 s" on DC. After 5 minutes with no work it puts the
user's values back. Tested here: it armed within a second of start and wrote exactly these values. Uninstalled 2026-11-03; nothing of it is left (checked 2026-11-03: `Test-Path` false for `%LOCALAPPDATA%\Programs\Nightcrew`, `%LOCALAPPDATA%\Nightcrew` and `HKCU:\Software\Nightcrew`; no Nightcrew entry in the HKCU Run key or in `Get-ScheduledTask`).

Worth copying:

- A pure state machine (`Controller`: inputs in, commands out) and a separate `Engine` that runs the commands. All
  Windows calls behind small interfaces, so it has 404 tests. Dry run by swapping implementations.
- Restore safety: atomic file plus registry copy; a field the user changes while armed is kept as theirs; follows a
  power plan switch; checks policy locks; restores on quit, sign-out and next start.
- A watchdog scheduled task (`schtasks` XML, logon trigger plus every 5 minutes, runs `exe --watchdog`) that restores
  settings if the app was killed.
- Marks its own battery timer value with "+7 seconds" so it can tell its values from the user's after both saved
  copies are lost. Windows offers only whole minutes.
- With the lid closed: locks the PC (unless the user's own lid action is Do nothing, which means docked use);
  hibernates at 20% battery or after 8 hours with no input.
- When work ends with the lid closed: waits for 5 minutes with no input, turns the screen off, and hibernates if the
  PC is still awake 5 minutes later.
- Clock that ignores sleep time (`QueryUnbiasedInterruptTime`). Handles `PBT_APMSUSPEND` so a late tick after wake
  cannot hibernate the PC in front of the user.
- WPF set to software rendering, which saves about 75 MB (their comment).

Weak spots:

- Edits the user's real power plan in place. The safety net for that is complex (368 line journal, registry copy,
  the +7 s trick, which does not work when the lid-closed limit is off).
- Scope creep: about 800 lines of "Efficiency mode" that slows other apps, a GPU signal, a loopback connection scan,
  a custom drawn panel with bundled fonts.
- CPU based detection: a background dev server started by an agent and using over 10% CPU likely keeps the PC awake
  as long as it runs. Not tested.
- Rules for any process exist only as a JSON hack (add the name as an "agent"), and need a restart.
- One author, created 2026-10-13, about 100 commits in 10 days, unsigned. About 80 MB RAM with the panel open.
- UI: tray icon is WinForms `NotifyIcon`, panel is a borderless WPF window. Not WinUI 3 (cannot run on .NET
  Framework 4.8).

UX problems (product owner view, agreed with Haik):

- Too many modes: "Hold awake" and "Pause". Pause is kept in memory only, so a reboot turns it back on.
- Tray icon style choice (removed upstream after v0.3.1).
- "Lock when lid closes" is optional. It should not be: closing the lid normally means sleep and a password on wake.
- "Stay awake for approvals" and "Battery floor" are settings that should be fixed behavior. "Lid-closed limit" should not exist at all (decision 3, 2026-11-03).
- Look is borrowed from Wispr Flow's design language (their credits) with serif fonts. Does not look like Windows.

### Insomnia review (code read 2026-11-02)

Electron app. Polls `tasklist` every 10 s for watched apps. For Claude Code it adds hooks to `~/.claude/settings.json`
and keeps awake for 15 minutes after the last hook. Uses `prevent-app-suspension`, so on Modern Standby it does not
stop idle sleep. No lid handling, no battery rules.

## Distribution

Checked 2026-11-03 (card `p0-distribution-facts`). Details, sources and marks: `docs/research/distribution.md`.

- **Velopack 1.2.161 works with Rust.** Tested on this laptop with a throwaway crate: `vpk pack`, silent install
  (`Setup.exe --silent`) to `%LocalAppData%\{packId}` with no admin, update with all hooks, silent uninstall. Every
  hook reaches Rust through `VelopackApp` callbacks. `vpk` 1.2.161 installs as a .NET tool and runs on the .NET 10
  SDK (it ships net8, net9 and net10 builds).
- **Velopack kills the app without warning** before uninstall, before an update swap, when Setup.exe runs over an
  existing install, and after every hook. Setup.exe over an existing install runs no uninstall hook and wipes the
  install folder. Uninstall runs `--veloapp-uninstall` after the kill (30 s limit in the docs). Silent install does
  not start the app. Seen in source and in the trial.
- **Size.** `Setup.exe` was 10.7 MB for a 1.5 MB app, mostly Velopack's own `Update.exe` (5.2 MB). Tested.
- **winget** accepts unsigned installers: an unsigned Velopack `Setup.exe` (Flowersauce.FSClicker 1.4.0) is merged.
  Manifest: `InstallerType: exe`, `Scope: user`, silent switch `--silent`, `ProductCode` = pack id. Validation runs
  installed exes with no arguments and flags non-zero exit codes. Checked in winget-pkgs.
- **SignPath Foundation** certificates are issued to "SignPath Foundation", so users see that name as publisher.
  It requires an existing release, MFA, CI builds on GitHub-hosted runners, a code signing policy page, manual
  approval per release, and "a certain verifiable reputation". Several new projects were declined in 2026 for lack
  of users (reported). Read on signpath.org and docs.signpath.io.
- **GitHub Actions**: standard runners, including Windows ARM64 (`windows-11-arm`), are free and unlimited for public
  repos. 6 h per job, 20 concurrent jobs. Read on docs.github.com.
- **scoop**: Extras needs about 100 stars or 50 forks, so only an own bucket at launch. A Scoop uninstall does not run
  the Velopack hook. Read in ScoopInstaller repos.

## Name research

Picked: **Wakeshot**. On 2026-11-02 there was no GitHub repo with that name, no winget package and no NuGet package.
Domains and trademarks not checked.

Rejected:

- Taken by big or same-space projects: Insomnia (Kong API client 40k stars, plus two keep-awake apps), Adrenaline
  (several large repos; AMD's driver software is "AMD Software: Adrenalin Edition", from memory), Caffeinator,
  Unblink, Allnighter, OpenEyes, Wakekeeper, LidAwake, Lidwake, LidGuard, StayLit, WideAwake, Shutlid.
- Common coffee words, all used: Espresso, Doppio, Cortado, Barista, Jolt, Taurine, Redeye, Doubleshot.
- Free but weaker: EyesOpen (clear, great eye icon, but a search finds a song and eye clinics), EyesAwake, WideEyed
  (also means naive), NoBlink, Blinkless, Wakebrew, Wakejolt, Wakedose, Holdwake.

## Toolchain

Set up on 2026-11-03 on the Windows laptop (card `docs/tasks/p0-toolchain.md`).

- **Build Tools.** Visual Studio Build Tools 2026, version 18.11.2 (installationVersion 18.11.12324.101), in
  `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`. Installed by Haik with workload VCTools, component
  VC.Tools.ARM64 and `--includeRecommended`. Checked with `vswhere -products *`.
- **MSVC and SDK.** MSVC 14.51.36231 with `bin\Hostx64\x64`, `arm64` and `x86`. Windows SDK 10.0.26100.0 with `um`
  libs for x64, arm64 and x86. Checked by listing the folders.
- **rustup.** 1.29.2 (2026-10-21), the current release per `static.rust-lang.org/rustup/release-stable.toml`.
  `winget install --id Rustlang.Rustup` installs it per user with no admin prompt, into `%USERPROFILE%\.cargo` and
  `%USERPROFILE%\.rustup`, and adds `.cargo\bin` to the user PATH. Adding `--scope user` fails with "No applicable
  installer found": the manifest declares no scope. Shells and Claude Code sessions started before the install do not
  see the new PATH.
- **Rust.** Stable 1.99.1 (2026-10-13), LLVM 23.1.1, host `x86_64-pc-windows-msvc`, profile default. The Rust blog's
  latest release page points to 1.99.1. Other versions: cargo 1.99.1, clippy 0.1.99, rustfmt 1.10.0, rust-analyzer
  1.99.1 (rustup component). Targets `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`. Checked with `rustc -vV`,
  `rustup show` and each tool's `--version`.
- **`cargo new`** uses edition 2024 by default.
- **Linker.** rustc finds the Build Tools 2026 linker by itself, with no Developer prompt or env setup:
  `MSVC\14.51.36231\bin\HostX64\x64\link.exe` for x64 and `HostX64\arm64\link.exe` for ARM64. Checked with
  `cargo rustc -- --print link-args`.
- **Cross build.** A hello-world builds in release for both targets. The PE machine field is `0x8664` (x64) and
  `0xAA64` (ARM64); each exe is about 130 KB. The x64 exe runs and prints. The ARM64 exe was not run: there is no ARM64
  device here.
- **Lint and format.** `cargo clippy -- -D warnings` passes for both targets. `cargo fmt --check` passes on clean code
  and prints a diff on badly formatted code.
- **Claude Code LSP.** The official plugin is `rust-analyzer-lsp@claude-plugins-official` 1.0.0 (marketplace
  `anthropics/claude-plugins-official`). It runs the `rust-analyzer` command from PATH, which the rustup proxy
  provides. Installed at user scope and listed as enabled by `claude plugin list`. Not verified: that it serves a
  session; it loads only in a session started after the install.

## Tray, menu and notifications

Checked on 2026-11-03 on the Windows laptop with the spike in `spikes/tray/` (card `docs/tasks/p0-spike-tray.md`).
Windows 11 26H2, build 26300.9711. Two displays: the primary 3840x2160 at 150% holds the taskbar and tray, a second
1920x1200 at 125%. Theme during the checks: apps dark, taskbar dark. Raw logs: `spikes/tray/out/`. Screenshots:
`docs/research/img/tray-*.png`. How our menu compares with current Windows 11 menus, and the pick for the menu look:
`docs/research/tray-menu-look.md`.

- **Crates.** `windows` 0.62.2, `embed-resource` 3.0.12, `png` 0.18.1 (spike screenshots only). Current per
  `cargo search` and `cargo info`. `embed-resource` found the SDK `rc.exe` by itself for both targets.
- **Manifest needed.** `LoadIconMetric` is exported only by Common Controls 6 (WinSxS
  `6.0.26100.9711`), not by `System32\comctl32.dll` 5.82. Checked with `dumpbin /exports`. So the exe needs a manifest
  that asks for Common-Controls 6. The spike embeds one as `RT_MANIFEST` 1 through the `.rc` file; the log confirms
  per-monitor v2 DPI awareness came from it.
- **Add and update.** From a hidden top-level window: `NIM_ADD`, `NIM_SETVERSION` (version 4) and `NIM_MODIFY` (new
  icon and two-line tooltip) all return TRUE. With version 4, a right-click arrives as `WM_CONTEXTMENU` in
  `LOWORD(lParam)` with the point in `wParam`, after `WM_RBUTTONDOWN` and `WM_RBUTTONUP`. Checked in the logs.
- **First add lands in the overflow.** On the first `NIM_ADD` from a new exe path, Windows writes
  `HKCU\Control Panel\NotifyIconSettings\<id>` with `ExecutablePath`, `UID`, `InitialTooltip`, `IconSnapshot` and no
  `IsPromoted`. Icons the user pinned have `IsPromoted=1` (OneDrive, Telegram here). `Shell_NotifyIconGetRect` then
  returns the rect of the `^` button: `tray-icon-first-add-overflow.png` shows only the chevron. Haik's right-click
  during a run came from above the taskbar, so from the flyout. Checked with the registry, the screenshot and the log.
- **Explorer applies `IsPromoted` live.** 2026-11-03 10:20, from the elevated agent session (card
  `p0-spike-tray`, T2): with the spike running (Medium token) and its icon in the overflow, writing `IsPromoted` = 1
  (DWORD) on its own `NotifyIconSettings\<id>` key moved the icon onto the taskbar within 3 s, with no Explorer
  restart and no input. Checked with taskbar screenshots before and after (`tray-taskbar-before-promote.png`,
  `tray-taskbar-after-promote.png`). `Shell_NotifyIconGetRect` gave the same
  RECT both times: the chevron moved one slot left and the icon took its place, so the rect alone cannot tell hidden
  from pinned. Whether Windows also does this for an unpinned icon that the user had moved back to the overflow: not
  checked.
- **`NIF_GUID` and the exe path.** An icon with a fixed `NIF_GUID` was added from the build folder, then from a copy
  in another folder: `NIM_ADD` returned TRUE both times. Windows kept one settings entry for the GUID and pointed it at
  the second path. So on this build a moved exe does not break a GUID icon. Not checked: whether a pinned state
  survives the move.
- **Dark menus.** uxtheme ordinals 135 (`SetPreferredAppMode`), 136 (`FlushMenuThemes`), 104
  (`RefreshImmersiveColorPolicyState`) and 132 (`ShouldAppsUseDarkMode`) all resolve by ordinal on 26300.
  `SetPreferredAppMode` returned the previous mode each time (0, then 1, 3, 2), which matches the 1903+ meaning, not
  the 1809 `AllowDarkModeForApp`. Ordinals and the mode enum from `ysc3839/win32-darkmode` `DarkMode.h`.
  - `AllowDark` set before the first menu: the menu follows the app theme (dark here). Tested apart on 2026-11-03 in
    Custom mode (Windows mode dark, app mode light): the `AllowDark` menu was light, while Windows' own tray menus were
    dark (`docs/research/tray-menu-look.md`, "Which mode the tray menus follow").
  - `ForceLight` or `ForceDark` plus `FlushMenuThemes` switch the next menu of the running process at once.
  - Submenu arrow, check mark, radio bullet, disabled line, separators and Windows 11 rounded corners render correctly
    in both. Screenshots `tray-menu-forcelight*.png`, `tray-menu-forcedark*.png`, `tray-menu-allowdark-system-dark.png`,
    and `tray-menu-live-dark-*.png` from Haik's own right-click.
  - The disabled status line is grey and low contrast in both themes.
- **Menu details.** `SetForegroundWindow` before `TrackPopupMenuEx` returned TRUE after every right-click on the icon.
  For menus opened by code it returned FALSE at 01:33, 01:35, 11:37 and 12:04 and TRUE at 13:01 and 13:02: Windows'
  foreground rules decide, not who opened the menu. Menu windows (`#32768`) belong to the thread that calls
  `TrackPopupMenuEx`. Posting `WM_CHAR 'k'` to the menu window opened the `&Keep awake` submenu. Checked in the logs
  and screenshots. Microsoft's `TrackPopupMenuEx` Remarks (read 2026-11-03): for a notification icon's menu the window
  must be the foreground window before the call; "Otherwise, the menu will not disappear when the user clicks outside
  of the menu" (the old KB135788 problem). So a menu opened by code (a hotkey, a notice click) may not close on an
  outside click when `SetForegroundWindow` fails; not tested (tray review, `reviews/p0-tray.md` finding 6).
- **Theme and contrast reads.** `AppsUseLightTheme=0` and `SystemUsesLightTheme=0` under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`; `ShouldAppsUseDarkMode()` agreed.
  `SPI_GETHIGHCONTRAST`: off. Checked in the log.
- **DPI.** `GetDpiForSystem` 144. `LoadIconMetric(LIM_SMALL)` returned a 24 px icon, the `SM_CXSMICON` for 144 dpi, so
  the resource frame was used without scaling. `GetSystemMetricsForDpi(SM_CXSMICON, 120)` is 20 for the second
  display. Not verified: that Windows 11 shows tray icons only on the primary taskbar.
- **Balloon.** `NIM_MODIFY` with `NIF_INFO` from the unregistered exe returns TRUE; `NIN_BALLOONSHOW` comes at once and
  `NIN_BALLOONTIMEOUT` about 6.5 s later. So Windows shows it, with no registration. Checked in the log.
  `NIN_BALLOONTIMEOUT` came twice in the same millisecond in both runs (`log-auto.txt` 15696 ms,
  `log-interactive.txt` 9522219 ms; grep 2026-11-03), so balloon events must not be counted. Windows files a balloon
  under `NotifyIconGeneratedAumid_<NotifyIconSettings id>`: the tray review found that handler in the notification
  database with 0 stored rows, and a `Notifications\Settings` key of that name (registry and database reads,
  `reviews/p0-tray.md` finding 2). So balloons are not stored, and a balloon's on/off switch most likely follows the
  icon's settings entry (inferred from the name; not tested).
- **Toast.** `ToastNotificationManager::CreateToastNotifierWithId(aumid).Show()` after `RoInitialize`:
  - Unregistered app id: `Show()` succeeds and `History.GetHistoryWithId(aumid)` counts 1, so Windows stored it (the
    notification database holds it), but the notification centre does not show it (Win+N, below). No `Dismissed`
    event within 10 s, and the 12:58 screenshots below show no banner.
  - App id registered per user, no admin: values `DisplayName` and `IconUri` (path to a PNG) under
    `HKCU\Software\Classes\AppUserModelId\<aumid>`. `Show()` succeeds, history counts 1, and `Dismissed` fires with
    reason `TimedOut` after 6.5 s, which means a banner showed and timed out.
  - `ToastNotifier.Setting` throws `0x80070490` (element not found) in both cases in that run; in the later Medium
    run it returned `Enabled` (see below).
  - Windows adds `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\<aumid>` after the first toast.
  - Microsoft Learn ("Send a local toast notification from other types of unpackaged apps", found by search; the page
    URL now returns 404) shows the same values under HKLM plus a `CustomActivator` COM class for clicks. Click
    handling was not tested.
- **Sound.** `PlaySoundW("SystemNotification", SND_ALIAS | SND_ASYNC | SND_NODEFAULT | SND_SYSTEM)` and
  `PlaySoundW("CHIME", module, SND_RESOURCE | ...)` with a 13 KB embedded WAVE both return TRUE. The user's scheme maps
  `SystemNotification` to `C:\WINDOWS\media\Windows Background.wav`; a user who picked "No sounds" gets silence with
  the alias. `SND_SYSTEM` puts the sound under the "System Sounds" volume slider (docs, not checked by ear).
- **Size.** x64 release with LTO, one codegen unit, `panic = "abort"`, `strip`: 504 KB at first, 519,168 bytes after
  the token code (built 03:41); 420 KB without the PNG encoder (first build). About 198 KB of that is resources: four uncompressed
  32-bit icons of 45 KB each, the WAV and the manifest. So the code (Win32 tray, menu, WinRT toast, registry) is about
  220 KB. ARM64 release builds (486 KB, the 01:43 build from before the token code, PE machine `0xAA64`) and passes
  clippy. Not run on ARM64: no device.
- **Memory.** In-process `GetProcessMemoryInfo` and `Get-Process` from outside agree. Icon only, idle: working set
  10.0 to 10.4 MB, private 1.6 to 1.7 MB in the logs (9.7 MB, 1.5 MB, 133 handles and threads dropping from 4 to 1
  after about 30 s are from outside reads that were not saved). After opening menus: working set 16 to 19 MB, private
  2.0 to 3.3 MB. After WinRT toasts: working set 25 to 28 MB, private 3.2 to 3.9 MB. The working set counts shared system DLL
  pages; private bytes is the app's own cost.

Live checks on 2026-11-03 10:27 (Haik's Explorer restart) and 12:53 to 13:04 (agent steps T3 to T12), with the
interactive spike running at Medium since 10:19 and its icon pinned by `IsPromoted`. Only the laptop screen was active
at 12:53 (1920x1200 at 125%). Evidence: `spikes/tray/out/log-interactive.txt`, the card's Log, and the agent's
scripts and shots in the scratchpad named there.

- **Explorer restart.** `TaskbarCreated` reached the hidden top-level window; the message-only window logged nothing.
  `NIM_ADD` right after it returned TRUE, and `tray-icon-add2-dark-taskbar.png` shows the icon on the taskbar, not the
  chevron: a pinned icon stays pinned across an Explorer restart.
- **Live theme switch through Settings.** Settings > Personalization > Colors > "Choose your mode", Light then Dark,
  driven by UI Automation. Each switch sent two `WM_SETTINGCHANGE` "ImmersiveColorSet" about 0.1 s apart. In the first
  one the values were not all updated yet (`SystemUsesLightTheme` still 0 in the Light switch; `ShouldAppsUseDarkMode`
  still old in both, but the spike reads it without calling `RefreshImmersiveColorPolicyState` first); in the second
  they agreed. So act once the broadcasts settle (the spike waits 2.5 s). The icon switched to the glyph for the new
  taskbar colour (`tray-icon-switch3-light-taskbar.png`, `...switch4-dark...`). With `SetPreferredAppMode(AllowDark)`,
  `RefreshImmersiveColorPolicyState` and `FlushMenuThemes` once at start (and after each earlier settle, after its no-flush
  capture) and a new menu built for each open, the next
  menu already had the new theme without another `FlushMenuThemes` (`tray-menu-switch3-light-noflush.png` light,
  `tray-menu-switch4-dark-noflush.png` dark; submenus too). Every other theme value (Themes, Personalize, DWM, Accent,
  `Custom.theme` hash) was the same after Light and back to Dark.
- **"ImmersiveColorSet" also comes without a theme change.** Two arrived at about 11:37 and 12:04 with both values
  still 0, during or right after Haik's battery steps; the `switch1` and `switch2` pictures are from those. Cause not
  checked. A tray app must compare the values, not trust the broadcast.
- **Tooltip.** Hovering the icon shows a native dark tooltip with both `szTip` lines (`tray-tooltip-dark.png`). It
  showed the new text 3 s after a `NIM_MODIFY`. It also stayed on top of the open menu while the cursor rested on the
  icon.
- **Synthetic right-click.** A right-click sent with `mouse_event` inside the icon rect gave the same `WM_CONTEXTMENU`
  as a real one, and `SetForegroundWindow` before the menu returned TRUE.
- **Balloon on Windows 11** (`tray-balloon-dark.png`): shown as a toast banner. Header "Wakeshot tray spike" with the
  tray icon (the exe's version info has that string as both `FileDescription` and `ProductName`, so which one Windows
  reads is not known), a large blue "i" for `NIIF_INFO`, then title and text. `NIN_BALLOONTIMEOUT` 6.6 s later.
- **Registered toast** (`tray-toast-registered-dark.png`): banner with the registered `DisplayName` and the `IconUri`
  picture, `Dismissed` reason TimedOut after 6.5 s. The picture showed orange, while the spike draws RGB 2b7cd3
  (blue). The PNG on disk is blue (43, 124, 211) and the spike writes straight RGBA, so Windows or a cached copy
  swapped R and B; which one was not checked. Phase 5 checks its real toast icon on screen: show a registered toast
  with an asymmetric test icon (pure red), compare on screen, and try a fresh `IconUri` path to rule out a cache.
  **Unregistered toast:** no banner at 2 s or 4 s, no `Dismissed` event (`tray-toast-unregistered-none.png` keeps only the
  taskbar and the spike tooltip; the area above it, where a banner would show, held other apps' content and is greyed).
- **`ToastNotifier.Setting`** returned `Enabled` (0) for both app ids in this Medium process; in the 01:33 `--auto`
  run, which was probably elevated, it threw `0x80070490`. The cause of the difference was not checked.
- **Notification centre** (Win+N): a "Wakeshot tray spike" group with the registered toast and "+1 notifications";
  no separate entry for the balloon or the unregistered toast. The notification database holds two toasts for the
  registered id (01:33:39 and 12:58:07), so the "+1" is most likely the 01:33 toast, which survived that run's key
  deletion.
- **Windows writes `Notifications\Settings\<aumid>` only for the registered app id**; quit removed it and the
  `AppUserModelId` key (`unregister` log: 0 for the registered id, 2 = not found for the unregistered one). Not
  removed by quit: the balloon's `Notifications\Settings\NotifyIconGeneratedAumid_<NotifyIconSettings id>` key and the
  stored toasts in the notification database (they expire after 3 days). So an uninstall must clear the toast
  history, not only delete the keys.
- **`LoadIconMetric` follows the system DPI, which is fixed at sign-in.** With the DELL (144 dpi) primary at sign-in
  and only the 120 dpi laptop screen left later, the spike still got 24 px icons while the taskbar slot was 40x60
  physical pixels; Windows scaled the icon. How sharp it looks was not judged. The hidden window got `WM_DISPLAYCHANGE`
  nine times and never `WM_DPICHANGED`.
- **The spike's log clock drifted 16 s.** The spike stamps each line with milliseconds since start (`Instant`). By
  11:37 that clock ran 16 s ahead of the wall clock (picture file times and the notification database agree); at 10:27
  it was still in step. So "start plus ms" times after 10:27 read up to about 16 s later than the real time (13:02:04
  for the Light switch, really 13:01:48). Cause not checked (likely the sleep
  during Haik's battery steps, or a clock sync). A log must read wall time for every line (tray review,
  `reviews/p0-tray.md` finding 13).

## Power settings spike

Checked on 2026-11-03 (night) on the Windows laptop with `spikes/power-settings/` (card
`docs/tasks/p0-spike-power-settings.md`). The laptop was on AC the whole time. The Claude Code session runs elevated
(High integrity), so every write test ran through the spike's `limited` mode: a child of Explorer with an unelevated
Medium integrity token, which the spike logs at start. Raw logs: `spikes/power-settings/out/`. Results marked
"morning" wait for Haik's steps in the card.

- **Writes as a normal user.** With the Medium token, `PowerWriteACValueIndex` and `PowerWriteDCValueIndex` on the
  active built-in Balanced plan return 0 for the lid action and sleep after, AC and DC; `PowerSetActiveScheme`
  returns 0. `powercfg` in the same run showed the new values, and the restore put the old ones back
  (`write-test-limited-20261103-021636.log`).
- **`PowerSettingAccessCheckEx`.** Returns 0 (`ERROR_SUCCESS`) for read and write of the lid action, sleep after,
  energy saver threshold and display off, AC and DC. Also 0 for `ACCESS_ACTIVE_SCHEME`, `ACCESS_CREATE_SCHEME` and
  `ACCESS_ACTIVE_OVERLAY_SCHEME` with a null GUID. For a write of CONSOLELOCK (require sign-in) it returns 5
  (`ERROR_ACCESS_DENIED`) with the Medium token and 0 elevated. So it reports a plain access denial, not only a
  policy. `ACCESS_SCHEME` with a null GUID returns 87. No group policy here, so 1260
  (`ERROR_ACCESS_DISABLED_BY_POLICY`) was not seen. Checked with `read` and `limited read`.
- **Stored at once, applied only on `PowerSetActiveScheme`.** Right after a write call the registry and a read show
  the new value, but the setting notification (the live value) changes only after `PowerSetActiveScheme`,
  `powercfg /setactive`, or `powercfg /change` (which applies by itself). `powercfg /setacvalueindex` alone stores
  without applying.
- **Detection latency.** From the start of the call (or of the `powercfg` start, about 25 ms of which is process
  start). `PowerSetActiveScheme` itself takes 37 to 76 ms. From `write-test` and `copy-test`:

  | Change | Setting notification | Registry notification | Poll every 250 ms |
  |---|---|---|---|
  | API write, no SetActive | none | 0 to 1 ms | 5 to 15 ms |
  | `PowerSetActiveScheme` after that write | 10 to 13 ms | none | none |
  | `powercfg /setacvalueindex`, no `/setactive` | none | 28 ms | 267 ms |
  | then `powercfg /setactive` | 29 ms | none | none |
  | `powercfg /change standby-timeout-ac` | 34 ms | 62 ms | 208 ms |
  | DC value changed while on AC | none | 27 ms | 159 ms |
  | Plan switch | 4 ms (active plan GUID) | 57 to 78 ms | 57 to 78 ms |

- **What each detector sees.**
  - Registry: `RegNotifyChangeKeyValue` on `HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes`, subtree,
    opened with `KEY_NOTIFY | KEY_READ` by the Medium token. Fires for every stored change on any plan, AC and DC, a
    plan switch (`ActivePowerScheme`), and a plan create, rename or delete. It says only "something changed", so the
    code must read again. The key path is not a documented API.
  - Setting notification: `PowerSettingRegisterNotification` with `DEVICE_NOTIFY_CALLBACK`. Fires once at
    registration with the current value (as the docs say), then only when the live value of the active plan changes
    for the current power source. A DC change on AC, or a write without SetActive, gives nothing.
  - Poll: sees everything at the poll interval. A read of five values takes under 1 ms.
- **The lid value in the notification is not the index.** The setting notification for the lid action gave 2 while
  the stored index was 1 (Sleep), and 0 for Do nothing. 2 matches `PowerActionSleep` in the `POWER_ACTION` enum
  (winnt.h); that link is an inference, Hibernate and Shut down were not tried. Read the index with
  `PowerRead*ValueIndex`, not from the payload.
- **Our writes and the user's look the same.** No detector says who wrote. Our API writes and `powercfg` writes give
  the same events. The only way to tell is to compare the new value with the one we wrote.
- **Plan copy.** With the Medium token `PowerDuplicateScheme` (2 ms), `PowerWriteFriendlyName`,
  `PowerSetActiveScheme` to the copy and back, and `PowerDeleteScheme` all return 0. The copy has every value of the
  source (lid, sleep, energy saver threshold, CONSOLELOCK, display off). Deleting the copy while it is active returns
  50 (`ERROR_NOT_SUPPORTED`) and keeps it. After switching back and deleting: gone from `PowerEnumerate`, its registry
  key is gone, `powercfg /list` shows only Balanced (`copy-test-limited-20261103-021755.log`).
  - Power mode: the overlay stayed `ded574b5-...` (registry `ActiveOverlayAcPowerScheme`,
    `PowerGetActualOverlayScheme`, `PowerGetEffectiveOverlayScheme`) and the effective power mode stayed
    MaxPerformance before, during and after; no effective power mode event fired at either switch. The two overlay
    getters are undocumented powrprof exports.
  - Settings shows and enables Power mode while a copy is active, and Lenovo Vantage's Power page does not change
    ("What Windows Settings writes" below). Not done: whether the copy stays active after a restart (B5, dropped
    2026-11-03: in place stays, so nothing depends on it).
- **Reads without admin** (Medium token).
  - Energy saver threshold: subgroup `de830923-a562-41af-a086-e3a2c6bad2da`, setting
    `e69653ca-cf7f-4f05-aa73-cb833fa90ad4`, a per-plan percent. Balanced here: AC 0, DC 30. Its setting notification
    gives the value for the current power source (0 on AC). Settings stores Never as 0 and "On Battery" as 100 (below).
  - Require sign-in: CONSOLELOCK (subgroup `fea3413e-7e05-4911-9a71-700331f1c294`, setting
    `0e796bdb-100d-47d6-a2d5-f7d2daa51f51`) AC 1, DC 1. `HKCU\Control Panel\Desktop\DelayLockInterval` = 0. Which of
    them the Settings sign-in choice changes: not tested, Windows Hello greys the choice out here (below).
  - Idle: `GetLastInputInfo` works. The overnight `ES_SYSTEM_REQUIRED` hold does not reset it: idle grew from 1182 s
    to 1430 s between runs.
  - Active plan change: the setting notification for `GUID_ACTIVE_POWERSCHEME` fires 4 ms after the
    `PowerSetActiveScheme` call starts, with the new plan GUID.
  - Effective power mode (`PowerRegisterForEffectivePowerModeNotifications`, version 2): MaxPerformance on AC here.
- **Screen off starts Modern Standby here.** Kernel-Power 506 "entering Modern Standby, Reason: Idle Timeout" came
  at 02:01:26, about 5 minutes after the last input, which is the AC display timeout (300 s); sleep after is 3 h and
  an `ES_SYSTEM_REQUIRED` hold was active. Shells kept running after it. Checked with `Get-WinEvent` and
  `GetLastInputInfo`; the link to the display timeout is an inference from the timing.
- **Battery rule: it holds on this laptop.** Microsoft's text is in "Windows power facts" above. Tested in Haik's
  block C, 2026-11-03 11:21 to 11:37 (`hold-20261103-112124.log`): on battery (98%), Medium token, a plan copy with
  battery sleep after 60 s, SYSTEM and EXECUTION requests set at 11:21:24.7, no input after (idle 0 at start). The 5 s
  ticks ran with no gap. The screen dimmed at 165 s and turned off at 180 s (Kernel-Power 506 "Idle Timeout"), and the
  ticks went on. At 379.6 s after the last input (`PBT_APMSUSPEND`, 11:27:44.1) the system slept while the requests
  were still set. The next tick came at 11:37:10 after the power button (507): wall step 571 s, unbiased step 4.9 s.
  The rule predicts 60 s + 5 minutes = 360 s; seen about 20 s later. The orchestrator's `powercfg /requests` log (every
  30 s) shows only the spike's two requests from 11:21:29 to 11:27:30, no other app's request during C, and no sample
  from 11:27:30 to 11:37:26 (the logger slept too). The log's "GAP at t 945 s" line names the gap by its end; it
  started at 379.6 s. A 600 s timeout was tried later in the lid spike's block I, with the same result ("Lid and sleep
  spike", morning results, "Battery rule").
- **Battery sleep Never stops the battery rule** (Haik's check M1 from the delta review, 2026-11-03 13:13 to 13:29,
  `hold-20261103-131303.log`, read by the `p0-close-results` worker). On battery (94% to 93%), Medium token through
  Explorer, lid open, a plan copy with battery sleep after 0 (Never), SYSTEM and EXECUTION requests set at 13:13:04.1,
  idle 29 s at the start and no input after. The display turned off at 13:16:04 (Kernel-Power 506 "Idle Timeout")
  and the 5 s ticks went on (largest step 5105 ms, wall and unbiased alike) until the end at 13:29:05, idle 990 s:
  past the 916 s where block I (Haik's 600 s timeout) suspended. `gaps: none`. Restored: Balanced active, copy
  deleted, sleep after AC 10800 and DC 600, lid 1/1. Only Never was tried; a raised finite value was not. The log's
  start lines read the 0 as a 0 s timeout ("requests end about 300 s after the last input"); 0 means Never.
- **What Windows Settings writes** (2026-11-03 12:29 to 12:33, an agent step with UI Automation on Settings, Windows
  build 26300, on AC, plan copy `42ffeeaa-...` of Balanced active, `watch-20261103-122958.log`). Each test value was
  set, read 5 s later with `powercfg /qh <guid>` on the copy and on Balanced, then put back and read again:
  - Sleep after, on battery (15 minutes): DC 900 on the copy and on Balanced. Lid on battery (Do Nothing): DC 0 on the
    copy and on Balanced. So for these two Settings writes the active plan and also the inactive Balanced. Why, and
    whether it writes other plans too: not checked. The watch saw two registry events for each such change.
  - Energy saver threshold: only the active plan (the copy). Never is stored as DC 0, "On Battery" (the list's last
    item) as DC 100. AC stays 0. The list has Never, 10% to 50% and On Battery. "Always use energy saver" is a
    separate toggle; not touched.
  - Power mode, "Plugged in" to Balanced: `ActiveOverlayAcPowerScheme` becomes
    `00000000-0000-0000-0000-000000000000` and the effective power mode event says Balanced (2); back to Best
    Performance: `ded574b5-...` and MaxPerformance (4). The page has separate "Plugged in" and "On battery" choices;
    "On battery" shows Balanced while `ActiveOverlayDcPowerScheme` is absent. "On battery" was not touched.
  - Power mode is shown and enabled while a copy of Balanced is active (both choices).
  - Sign-in options, "If you've been away": shows "Every Time", greyed out, with "Windows Hello is preventing some
    options from being shown" (UI Automation: `SystemSettings_Users_DelayLock_ComboBox`, IsEnabled False). So on this
    laptop it cannot be changed in Settings, and which value it maps to was not tested. CONSOLELOCK 1/1 and
    `DelayLockInterval` 0 at every read.
  - With "Every Time", the session locks when the screen turns off on AC: LogonUI started at 12:33:48, the same second
    as Kernel-Power 506 "Idle Timeout".
  - Lenovo Vantage (10.2608.62.0) with a copy active (12:43 to 12:45, a second copy `fa098892-...`): Device settings
    > Power read the same with and without the copy (whole UI Automation tree compared as text, no line differs):
    Smart standby Off (Auto, greyed), Battery charge threshold Off (75% to 80%, default), Always on USB On, "even when
    device is shut off" Off, no warning. This build has no thermal or performance page (Device settings has Power,
    Display, Sound, Input, Device details). Its pages are web content: UI Automation `FindAll` did not reach the
    items, a `TreeWalker` walk did. A feedback survey covered the page once; its Close button dismissed it.
  - Not done: a restart with a copy active (B5, dropped 2026-11-03: in place stays).
- **Lenovo software here.** Lenovo Vantage `E046963F.LenovoCompanion` 10.2608.62.0. Running services: Lenovo Smart
  Standby, Lenovo Intelligent Thermal Solution, Lenovo PM Service, Critical Service for Lenovo Vantage. Checked with
  `Get-AppxPackage` and `Get-Service`. What Smart Standby does to standby: not checked.
- **`wevtutil /f:text`** prints local time with a `Z` suffix on this machine (the same events show local time in
  `Get-WinEvent`).

## Lid and sleep spike

Checked on 2026-11-03 (night) on the Windows laptop with `spikes/lid-sleep/` (card
`docs/tasks/p0-spike-lid-sleep.md`). On AC, lid open, docked to a DELL U2723QE. The orchestrator held
`ES_SYSTEM_REQUIRED` all night, so nothing that needs a real sleep, an idle sleep or a lid close ran; those are
Haik's morning steps in the card. Tests ran with an unelevated Medium token (the spike's `limited` mode, or a process
started by Explorer) unless marked. Raw logs: `spikes/lid-sleep/out/`. Haik's morning runs (10:30 to 12:19) are in
"Morning results" at the end of this section.

- **Capabilities.** `GetPwrCapabilities` (Medium token): LidPresent, AoAc and AoAcConnectivitySupported true; S3
  false; S4 and HiberFilePresent true; Hiberboot (fast startup) true; WakeAlarmPresent false; RtcWake S4;
  ThermalControl true. `IsPwrHibernateAllowed` true, `IsPwrSuspendAllowed` false. Registry
  `HKLM\SYSTEM\CurrentControlSet\Control\Power`: `HibernateEnabled` absent, `HibernateEnabledDefault` 1.
  `powercfg /a` runs without admin: S0 Low Power Idle Network Connected, Hibernate, Fast Startup.
- **Hibernate works here, and Windows uses it by itself.** System log: Kernel-Power 187 at 2026-11-01 23:46:42 (caller
  `winlogon.exe`, SystemAction 3 = hibernate), then 42 "Sleep Reason: Application API", 107, and Kernel-Boot boot type
  0x1 at 23:47:58: a hibernate and resume. (The morning's own hibernates logged boot type 0x2. 0x1 is often read as
  fast startup, so this one may have been a shutdown with fast startup; the meaning of the values is not verified.)
  On 2026-11-02 08:48 Kernel-Power 42 "Hibernate from Sleep - Standby
  Battery Budget Exceeded": Windows hibernated a sleeping laptop on its own. 506/507 "Austerity Battery Drain Budget
  Exceeded" also appear (2026-11-02 02:29 and 13:59). Microsoft's Modern Standby wake sources page (read
  2026-11-03): "Starting in Windows 11, version 24H2, ... If excessive battery drain is detected, most wake sources
  will be disabled. In this state, the device can be woken from Modern Standby by pressing the power button or
  opening the lid."
- **"Hibernate after"** (`9d7815a6-...`) on Balanced: AC 0, DC 2147483647. What the DC value means: not verified.
- **Entering Modern Standby.** Lid close with action Sleep: 506 "Reason: Lid" (2026-11-02 22:57). Idle: 506 "Idle
  Timeout" at 02:01:26. At 02:44 the session was locked (`WTSQuerySessionInformation` SessionFlags) with the display
  off and 47 minutes idle; CONSOLELOCK is 1/1 and `DelayLockInterval` 0. The morning runs logged it: Windows locks
  within 1 s of the display-off standby entry (morning results).
- **SetSuspendState.** Doc text (re-read 2026-11-03): "The calling process must have the SE_SHUTDOWN_NAME
  privilege"; `bForce` "has no effect". The page says nothing about Modern Standby. Not called tonight. A process
  started by Explorer (Medium token) enables SeShutdownPrivilege (`AdjustTokenPrivileges` ok, last error 0;
  `out/read-explorer.log`). Under `limited` (parent process spoofing) `OpenProcessToken(TOKEN_ADJUST_PRIVILEGES)`
  fails with 5, so `limited` cannot test `SetSuspendState`; that failure is an artifact of the launch. Both
  `SetSuspendState(FALSE)` and `(TRUE)` hibernate this laptop (morning results).
- **Hibernate off.** Not tested: the card forbids turning hibernate off. The rivals check `SystemS4` and
  `HiberFilePresent` first (`rivals-nightcrew-lidguard.md`); `IsPwrHibernateAllowed` is the documented single call.
- **Lid notification.** `PowerSettingRegisterNotification(GUID_LIDSWITCH_STATE_CHANGE, DEVICE_NOTIFY_CALLBACK)` works
  with the Medium token, no window. The first callback came within 1 ms of the call with the current state (1, open).
  The same holds for `GUID_CONSOLE_DISPLAY_STATE`, `GUID_SESSION_DISPLAY_STATUS`, power source, battery percent and
  active plan. Doc (Power Setting GUIDs page, read 2026-11-03): lid 0 closed, 1 opened; "The callback won't be called
  until a lid device is found and its current state is known". It also says interactive user-mode apps should use
  `GUID_SESSION_DISPLAY_STATUS`. Timing at a real close, the sound and the lock: morning results.
- **Sound at lid close: Microsoft's rule.** Wake sources page, Audio table: playback during screen off is enabled for
  "Idle to Standby" and not for "Explicit entry to Standby (Power button press, closing the lid or through Start
  menu)". A close with action Do nothing is no standby entry at all: no 506 at the close in D, E or I (morning
  results).
- **Input with the lid closed.** Same page: from Windows 11 24H2, input suppression keeps the display off when the lid
  is closed, unless an external display is connected.
- **Counting screens without admin.** `QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS)` with the Medium token returned two
  active paths: `INTERNAL` (0x80000000) "N160JCA-GT1" and `DISPLAYPORT_EXTERNAL` (10) "DELL U2723QE";
  `SM_CMONITORS` 2. WMI `root\wmi WmiMonitorConnectionParams` also works unelevated and agrees
  (`VideoOutputTechnology` 2147483648 and 10), but through PowerShell it takes about 1 s; the API takes under 1 ms.
  Kinds used by the spike: internal = INTERNAL, LVDS (6), DISPLAYPORT_EMBEDDED (11), UDI_EMBEDDED (13); Miracast and
  indirect (15 to 17) are not a docked screen. What the count shows with the lid closed: morning results.
- **Temperature without admin.** PDH counters `\Thermal Zone Information(*)\Temperature`, `High Precision
  Temperature`, `% Passive Limit` and `Throttle Reasons` read with the Medium token. One zone, `\_TZ.THM0`. Temperature
  is whole Kelvin; the high precision value is tenths of Kelvin but always ends in 2 (3382 = 338.2 K), so 1 K steps
  in practice. WMI `Win32_PerfFormattedData_Counters_ThermalZoneInformation` gives the same data.
  `MSAcpi_ThermalZoneTemperature` (root\wmi) returns Access denied to a normal user.
  - It is meaningful: it follows CPU load within seconds. Idle 41 to 47 C. With 8 busy threads: 47 C after 2 s, 54 C
    after 4 s, 67 C after 60 s; 51 C 8 s after the load stopped (`out/heat-limited-20261103-024517.log`).
  - Limits: Kernel-Power 125 says `_CRT` = `_HOT` = 386 K (112.85 C) and `_PSV` = 0 K (no passive trip point).
    Throttle reasons stayed 0 and the passive limit 100% at 67 C. Closed on a desk in the morning runs: 35 to 60 C.
    No bag test.
- **Wake timers.** "Allow wake timers" (RTCWAKE `bd3b718a-...`) on Haik's Balanced plan: AC 0, DC 0 (Disable).
  Values: 0 Disable, 1 Enable, 2 Important Wake Timers Only (`powercfg /qh`).
  - `SetWaitableTimer(..., fResume TRUE)` with the Medium token succeeds with last error 0, not `ERROR_NOT_SUPPORTED`
    (the doc says that error means "the system does not support a restore"), even with RTCWAKE Disable. So the call
    does not tell whether the timer can wake the PC. `SetWaitableTimerEx` with a reason context behaves the same.
  - `powercfg /waketimers` needs admin. From the elevated session it listed the spike's timer ("Timer set by
    [PROCESS] ...lid-sleep-spike.exe", with our reason text when set through `SetWaitableTimerEx`), and a Lenovo task
    `LvfGuiRemoval` that asks to wake the PC at 17:17.
  - Doc text (SetWaitableTimer, read 2026-11-03): "Windows 8 and newer ... If relative time is specified, the timer
    does not include time spent in low-power states. For example, the timer does not continue counting down while
    the computer is asleep." So a wake timer must use an absolute UTC due time. The spike does.
  - The wake sources page says the always-on timer "wakes SoC but cannot turn on display". No Microsoft page found
    on how a desktop app's wake timer behaves in Modern Standby (search 2026-11-03). Whether it wakes this laptop, on
    AC and on battery, and whether RTCWAKE matters: could not test. The morning trim dropped G and I3 because no
    design item uses wake timers.
- **`CallNtPowerInformation` without admin.**
  - `SystemExecutionState` returned 0x1 (`ES_SYSTEM_REQUIRED`) while only the orchestrator held a request. So a normal
    user may see "someone holds a system request" without `powercfg /requests`. Morning: it reads 0x0 with no
    request, and 0x1 for up to 30 s after some resumes because of Windows' own request (morning results).
  - `LastSleepTime` and `LastWakeTime` are 0 since boot (2026-11-02 17:17), although the System log shows Modern
    Standby sessions since then. They do not record Modern Standby.
  - Interrupt time and unbiased interrupt time were equal (34028 s) after 9.5 hours with several Modern Standby
    sessions. So `QueryUnbiasedInterruptTime` did not skip those sessions. The morning gaps explain it: unbiased time
    skips the time the PC is suspended, not the screen-off part of Modern Standby where apps still run. The night
    hold kept the PC from suspending (morning results).
  - `SystemPowerInformation.TimeRemaining` (idle countdown) read 0 while the request was held.
- **Plan copy with lid Do nothing.** Made, applied and removed with the Medium token. A run killed by PID left the copy
  active with LIDACTION 0/0 until `restore` put Balanced back and deleted it. Ctrl+C runs the restore within 100 ms
  (console handler). Closing a classic console window runs it too (safety review, 03:18, and "Spike launch and stop
  checks" below); closing a Windows Terminal tab was not tested.
- **Lenovo Smart Standby** service is running (Auto); its description says only "Provides the functionalities of
  Lenovo Smart Standby". What it changes: not checked.

**Morning results** (2026-11-03, 10:30 to 12:19). Haik ran blocks D, F, H, E and I of
`docs/phases/00-morning-steps.md`; C belongs to the power settings spike. Medium token; every run restored the plan
(`after` equals `before` in each log). A read-only `powercfg /requests` log every 30 s showed no other app's DISPLAY,
SYSTEM or EXECUTION request in any block. System log entries read with `Get-WinEvent`. Logs: D
`out/lid-20261103-103006.log`, F `out/suspend-20261103-103649.log` and `-103853.log`, H
`out/resume-20261103-104117.log`, `-105721.log` and `-120914.log`, E `out/lid-20261103-111112.log`, I
`out/lid-20261103-114033.log`. Some spike `RESULT` lines misread their own data; each is noted at its result below,
and the readings here come from the events (delta review `docs/reviews/p0-delta.md`, m1; each line re-read by the
`p0-close-results` worker). If Phase 3 reuses a spike: judge a suspend by `PBT_APMSUSPEND`, the S-state by the System
log, and name a gap by its start.

- **Lid callback at a real close.** It arrives at once; the spike reacted 370 to 561 ms later only because it acts on
  its next loop step. A close with action Do nothing logs no 506 and does not turn the display off. In E (no external
  screen) the display state stayed on until the DC display timeout. In D (DELL in) the internal path dropped and the
  DELL stayed on.
- **Lock.** With `--no-lock` (E) the session stayed unlocked for 176 s with the lid closed. `LockWorkStation` right at
  the close (I) returned Ok in 0.5 ms and the session read locked 1 s later; one `QueryDisplayConfig` call during the
  lock returned 31. Windows itself locks within 1 s of every display-off standby entry (506 Idle Timeout), with or
  without a held request, on AC (H) and on battery (E). Haik's CONSOLELOCK is 1/1 and `DelayLockInterval` 0; other
  values not tested.
- **Display off on the lock screen.** 31 s after `LockWorkStation` in I, and 32 s after a key press on the lock screen
  in H4. Which setting gives this: not verified.
- **Sound.** `PlaySound("SystemNotification", SND_ALIAS | SND_ASYNC | SND_NODEFAULT | SND_SYSTEM)` returned true in D,
  E and I. With `SND_ASYNC`, true only means the sound started. Haik heard: D "sound from laptop", E "the laptop sound
  again (not windows one)", I nothing. In D the default device was the DELL, which has no built-in speakers (Macworld
  review and a Dell community staff post, web, 2026-11-03), so our sound could not come from the laptop. The DELL
  cable came out 33 s after the close in D and 4.3 s before it in E, and the sound scheme plays `Windows Hardware
  Remove.wav` on device removal. Most likely reading (inference): what Haik heard in D and E was that device sound,
  and our sound was heard in no block. Our alias plays `Windows Background.wav`, not the toast sound `Windows Notify
  System Generic.wav`, so the E sound may also have been ours. The System Sounds level, the closed lid and the lock
  were checked later: see "Lid-close sound" (System Sounds 100%, not muted; at two later closes neither the lock nor
  the closed lid stopped a sound, and Haik heard it).
- **Hold with the lid closed.** The held request kept the closed laptop running: D 163 s through undock, power unplug
  and replug (only Kernel-Power 105, no 506, no gap); E 60 s; I 912 s. Trap 12 of `research/rivals-others.md` (power
  switch with the lid closed) did not break the hold here.
- **Battery rule.** In I (battery, lid closed, request held, Haik's 600 s DC sleep) the PC suspended
  (`PBT_APMSUSPEND`, gap 514 s) 916 s after the last input. The power spike's C run (copy with sleep.dc 60 s, lid open)
  suspended about 380 s after the last input (`spikes/power-settings/out/hold-20261103-112124.log`, read only). Both
  are the DC sleep timeout plus 316 to 320 s, as the "5 minutes after the sleep timeout" rule quoted under "Windows
  power facts" says. In I `SystemExecutionState` read 0x1 up to the last tick before the suspend (11:55:54.8), so it
  gives no warning that the rule is about to end the request. Battery sleep Never stopped the rule in a 16 minute
  test, lid open ("Power settings spike", "Battery sleep Never stops the battery rule"). I's RESULT line "the lid was
  open when work ended, so no end method ran" misreads the run: the work end (1200 s) was due inside the suspend
  (11:55:56 to 12:04:28), and the tool saw the lid open first.
- **Screen off is not suspend.** Display off at the display timeout logs 506 Idle Timeout, but desktop apps keep
  running; the spike's 1 s tick then fires about every 1.5 s. Apps pause only at `PBT_APMSUSPEND`. With no request it
  came at the sleep timeout: 67 to 70 s after the last input on a 60 s copy (H control phases, AC, lid open). In E
  (lid closed, battery) the spike released its request and put Balanced back (lid action Sleep) at 11:12:52 with the
  lid still closed (`lid-20261103-111112.log:143-149`). The PC did not sleep: putting the lid action back does not
  sleep a laptop whose lid is already closed. It ran for 263 s after the 506, until the lid opened at 455 s idle,
  below the 600 s sleep timeout; the spike's "no standby" RESULT means only "no gap". So after a release, a closed
  laptop most likely stays awake until the user's sleep timeout (inferred from H; Haik's AC value is 3 h; not tested
  with the lid closed).
- **SetSuspendState.** `(FALSE)` (sleep) and `(TRUE)` (hibernate), Medium token, lid open, AC: both hibernated.
  Kernel-Power 187 SystemAction 2 and 3, then 42 and 107 with TargetState and EffectiveState 5 (S4);
  Power-Troubleshooter 1 HiberWriteDuration 7950 and 8156 ms (about 1.3 M pages), BiosInitDuration 8584 and 8566 ms,
  HiberReadDuration 10196 and 9581 ms; Kernel-Boot 27 boot type 0x2;
  Kernel-General 1 set the clock forward 59 and 64 s. The call returns true only after resume (74 and 78 s). After
  resume: 506 "Sleep, Hibernate, or Shutdown", 507 Power Button 1 s later, session locked. The thermal zone read 73 to
  87 C for about 30 s after each resume. F3 (screen off) did not run because F1 slept. Hibernate off: not tested, by
  rule. Kernel-Power 40 "The driver \Driver\ACPI for device stopped the power transition" came about 0.1 s before the
  42 in both runs (10:37:02.2 and 10:39:06.4, `Get-WinEvent`); both hibernates still completed. Meaning not checked.
  The spike's RESULT lines say "the PC stopped (Slept)" for both: its gap kind cannot tell standby from hibernate.
- **Start > Power > Sleep with a request held** (H run 2, H4): 506 "Screen Off Request" and `PBT_APMSUSPEND` 0.13 s
  later. A held request does not block the user's own sleep.
- **Resume.** H run 2: a request held through Start > Sleep was not honored after resume. At the next display off
  (61 s after resume) the PC suspended within 17 ms, gap 319 s; `SystemExecutionState` read 0x0 in every hold tick
  (0x1 before the sleep). H4, re-take (clear, then set again) right after resume: display off, then 3 minutes with no
  suspend until the release; `SystemExecutionState` 0x1. The control phases slept both times, so both runs are valid.
  One run each, both on AC. Only clear then set on the same handle was tested; taking a new request before dropping
  the old one was not. In H run 2's hold phase the requests log still listed the spike's SYSTEM and EXECUTION requests
  at 10:58:45 and 10:59:15 (`requests.log` in the orchestrator's scratchpad), while `SystemExecutionState` read 0x0
  and the PC suspended at 10:59:25.9. So `powercfg /requests` lists a request that Windows no longer honors.
  `SystemExecutionState` showed this loss, but gave no warning before the battery rule ("Battery rule" above). Design
  effect (delta review, m2): re-take always and raise the battery timer; do not rely on detecting the loss. H run 1 had
  no sleep (the manual sleep did not register); it shows that a request held from before any sleep kept the PC running
  through three display-off standby entries. Its RESULT line "hold-phase gaps 0 (request honored)" is a tool misread:
  no hold phase ran (all 744 ticks are `[before-sleep]`).
- **Clock.** In every gap unbiased interrupt time skipped the suspended time: wall 40 to 514 s against unbiased 0.15
  to 1.35 s for Modern Standby suspends (the power spike's C gap: 571 s against 4.9 s), 7.4 and 7.8 s for the
  hibernates. While the screen was off and apps still
  ran, unbiased time and wall time moved together. Idle time from `GetLastInputInfo` counts suspended time: at the
  first tick after the I suspend it read 1430 s (514 s of it suspended), after C 945 s (571 s suspended). So never
  compare it with unbiased time.
- **SystemExecutionState.** 0x0 with no request (step 0 read at 10:22 after the keep-awake stopped,
  `out/read-limited-20261103-102214.log`; F; D and E after the release). 0x1 for up to 30 s right after
  some resumes with no request held; the requests log showed `[DRIVER] Legacy Kernel Caller` under SYSTEM at
  12:15:09, right after such a resume.
- **Screen count with the lid closed.** Docked (D): the internal path dropped at the close (int 0, ext 1). After the
  cable came out with the lid closed, ext stayed 1 until the lid opened, so the count was stale. Not docked (E): int
  stayed 1 with the lid closed. Lid open (E): unplugging dropped ext to 0 within 1 s.
- **Heat** (whole runs): D 40 to 52 C, E 39 to 60 C, I 35 to 53 C. One tick right after a resume read 7.1 C, a bad
  first read.

## Lid-close sound

Card `docs/tasks/p1-lid-sound.md`, 2026-11-03 from 14:37. Agent tests with the lid open, on battery, DELL not
connected, Speakers (Realtek) the default for all roles at 67.3%. Code: `spikes/lid-sleep/src/sound.rs`, modes
`audio-read` and `sound-test`, `lid --listen`. Logs in `spikes/lid-sleep/out/`.

- **Loopback works as an agent check.** WASAPI loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK`, shared mode, the endpoint's
  mix format: float32, 2 ch, 48 kHz) on the Speakers saw every sound the tests played, with its length and peak, from
  the elevated agent and from a Medium process (`out/sound-test-20261103-145607.log`,
  `out/lid-limited-20261103-145749.log`). It shows what the audio engine sent to the endpoint, not what the speaker
  made: a closed lid, a muted amplifier or a codec that wakes late are not seen by it. No packets arrive while nothing
  plays (the "alive" lines stop counting), so silence and a stalled engine look the same to it.
- **Peaks in loopback are higher than in the files.** `Windows Background.wav` peaks at -9.2 dBFS in the file
  (`wavstat.py`, below) and at -1.2 dBFS in loopback; our chime peaks at -6 dBFS in the PCM and -0.4 dBFS in
  loopback. Something in the endpoint path adds about 6 to 8 dB; the Realtek driver has Dolby settings
  (`DolbySlidingBass` under its Class key), so a Dolby effect is the likely cause (not verified).
- **System Sounds level.** The System Sounds session on the Speakers reads 100%, not muted
  (`IAudioSessionControl2::IsSystemSoundsSession`, `ISimpleAudioVolume`; `out/audio-read-20261103-145551.log`). So
  the mixer slider did not silence run I (assuming it was the same then; it was not read during I).
- **`SND_SYSTEM` routing.** PlaySound doc (archived page dd743680, read 2026-11-03): with `SND_SYSTEM` "the sound is
  assigned to the audio session for system notification sounds", under the System Sounds slider; it names no device.
  Loopback: the alias sound reached the default device (Speakers), 1210 ms long. With the DELL as default (run D) it
  goes to the DELL, which has no speakers. That explains D, not I.
- **Start delay.** `PlaySound(SND_ASYNC)` returned in 5 to 8 ms, but the sound reached the endpoint 390 to 760 ms
  later. Our chime: from the call to a running stream took 290 to 570 ms in three runs, almost all of it in
  `IAudioClient::Initialize` (device pick 5 ms, stream ready at 482 ms in the timed run). Loading the audio stack
  first (warm-up: enumerate, activate, mix format, 6 ms) did not help in that run (one run). So any sound starts
  about 0.3 to 0.8 s after the call on this laptop. In run I the lock came 2 ms after `PlaySound` returned, so it came
  well before the sound reached the endpoint. Whether that matters: lid-close results below.
- **Sound file levels** (`wavstat.py` in the card worker's scratchpad, reads the PCM): `Windows Background.wav` (our
  alias) 1.29 s, peak -9.2 dBFS, loudest 50 ms -17.1 dBFS; the toast sound `Windows Notify System Generic.wav`
  -9.8 / -18.3; `Windows Hardware Remove.wav` -12.9 / -19.4. Our alias sound is not quieter than the device-remove
  sound Haik most likely heard in D (with the lid closed, 33 s after the close). So the file is not the reason for I.
- **Device form factors** (registry `MMDevices\Audio\Render\<id>\Properties`, `{1da5d803-...},0`, and
  `PKEY_AudioEndpoint_FormFactor` through Core Audio): both DELL U2723QE endpoints 9 (`DigitalAudioDisplayDevice`),
  Speakers 1 (`Speakers`), "Realtek HD Audio 2nd output" 3 (`Headphones`). So "default is a screen's audio" is
  readable without names.
- **No lid setting in the audio driver.** No value or key with "lid" in its name under the audio Class key
  `{4d36e96c-...}` (Realtek 6.0.10007.1, AMD HD Audio, AMD USB and Bluetooth audio); the two matches were
  `ValidateCodecWrites` and `DolbySlidingBass`. Lenovo Vantage settings were not looked at. Whether the closed lid
  muffles the speakers: only an ear can tell (lid-close results below).
- **Lid-close results** (Haik, 15:16 to 15:20, Medium token through `sound-close-1.cmd` and `sound-close-2.cmd`;
  logs `out/lid-20261103-151653.log` and `out/lid-20261103-151907.log`; plan copy restored in both, 13 `after`
  lines equal the `before` lines). Haik's words: "Both close 1 and close 2 tests work", in close 1 "first was windows
  sound then your custom", in close 2 the reverse.
  - Close 1, laptop alone on battery, exactly as run I: `PlaySound` at the close, `LockWorkStation` 0.2 ms after it
    returned. The alias reached the Speakers 504 ms after the call, 1202 ms long, peak -1.2 dBFS (the same as with
    the lid open), and Haik heard it. Our chime 3.05 s after the close (session locked, lid shut): stream running in
    125 ms, all 750 ms taken by the engine, 503 ms in loopback, heard. So neither the lock nor the closed lid stops
    or mutes a sound on this laptop, and run I's silence did not repeat. Its cause stays unknown; most likely it was
    not noticed (inference).
  - Close 2, DELL connected on AC; Windows had made the DELL (`{...ca5b6419...}`, form factor 9, 26.0%) the default
    for all roles. The chime picked the Speakers (`SpeakersInsteadOfDisplay`), ran 427 ms after the call, then the
    lock came; 498 ms in loopback on the Speakers, all taken by the engine, heard. The alias control 3.4 s after the
    close went to the DELL: 944 ms in the DELL's loopback, nothing on the Speakers. The Windows sound Haik heard after
    the chime was most likely the device-remove sound when he unplugged the DELL: power went DC at 15:19:37.9, the
    DELL endpoint was invalidated at 15:19:39.7 (`0x88890004`), and the Speakers' loopback logged a 351 ms sound at
    15:19:40.4, about 20 s after the lid opened (inference; the DELL has no speakers, see run D).
  - The alias in the DELL's loopback peaked at -31.7 dBFS against -1.2 dBFS on the Speakers. -9.2 dBFS (file) at the
    DELL's 26% volume is about -31 dB, so the DELL's loopback looks like it is after the volume and the Speakers' is
    before it (inference, not verified).
  - So: our chime on the picked device was in loopback and heard at both closes, with the lock on, once before the
    lock and once after it.

## Spike launch and stop checks

Checked on 2026-11-03 between 03:30 and 03:57 (round 2: 04:20 to 04:30) for card
`docs/tasks/p0-spike-safety-fixes.md`, on the Windows laptop, from an elevated agent session. Logs and scripts are in
that card's Log.

- **Windows Terminal opens elevated here.** Its `settings.json` has `profiles.defaults.elevate: true`; UAC is on
  (`EnableLUA` 1) with `ConsentPromptBehaviorAdmin` 0 (no prompt). The running `WindowsTerminal.exe` had a High,
  elevated token; the shell `explorer.exe` (the one that owns the desktop window) had Medium, not elevated (token read
  with `OpenProcessToken` from the elevated session). `HKCU\Console\%%Startup` names Windows Terminal as the default
  terminal. `SeparateProcess` is 0, so File Explorer windows run in the shell Explorer.
- **A `.cmd` opened through Explorer runs at Medium.** `explorer.exe <file>.cmd` from the elevated session was served
  by a new `explorer.exe /factory,{75dff2b7-6936-4c06-a8bb-676a7b00b24b} -Embedding` started by `svchost -k
  DcomLaunch`; the chain was pwsh <- cmd <- that Explorer. `whoami /groups`: Medium Mandatory Level; Administrators
  not enabled. Every exe started from that pwsh (both spikes and the tray spike) reported "elevated false, integrity
  Medium (0x2000)", and the lid spike enabled SeShutdownPrivilege (last error 0). A real mouse double-click goes
  through the shell Explorer instead of a factory instance; that path was not run by an agent (the morning shell
  prints the token, so Haik sees it).
- **That `.cmd` runs in a new Windows Terminal window, not a classic console** (checked 04:26 for round 2 of the same
  card, `<scratchpad>\r2-test.cmd` opened with `explorer.exe`). From inside the shell, `GetConsoleWindow` gave a
  window of class `PseudoConsoleWindow` whose owner window belonged to a new `WindowsTerminal.exe` (not Haik's
  elevated one, PID 13456), while `WT_SESSION` was empty. So a missing `WT_SESSION` does not mean a classic console
  here: Windows Terminal takes over the console after the shell has started (it is the default terminal), too late to
  add the variable. The round 1 line "the console host hosted it" was wrong. The shell token was still Medium, not
  elevated. `morning-shell.ps1` now logs the window class and owner instead.
- **powercfg as a normal user.** At Medium: `/duplicatescheme`, `/changename`, `/setacvalueindex` and
  `/setdcvalueindex` (LIDACTION, STANDBYIDLE, VIDEOIDLE, RTCWAKE, ESBATTTHRESHOLD) on a plan copy, `/setactive`,
  `/list` and `/delete` of an inactive copy all exit 0, and the values read back. CONSOLELOCK writes fail with exit 1,
  "You do not have the permissions required to perform this action."; elevated they work. Same as the spike's
  `PowerSettingAccessCheckEx` result (5 for a CONSOLELOCK write).
- **Ctrl+C and window close through pwsh.** A spike exe started by `pwsh -Command` in a new console host window gets
  Ctrl+C (`GenerateConsoleCtrlEvent` to the console group) in its own handler (event 0); pwsh waits for it and then
  exits. `WM_CLOSE` to the console window gives the exe event 2 (close); the power spike switched back and deleted its
  copy 130 ms later, before Windows ended the process. (That test used an explicit `conhost.exe` window.)
- **Closing the morning shell's Windows Terminal window** (04:28 and 04:29, round 2; `<scratchpad>\r2-wtclose-run.ps1`
  and `r2-wtclose-lid-run.ps1`: the exe run through a `function spike { & exe @args }` wrapper in a window started like
  `morning-shell.cmd`, then `WM_CLOSE` posted to that Terminal window). `spike hold --allow-ac`: event 2 arrived 0.33 s
  after the close, the plan was back and the copy deleted 0.47 s after it, the journal was gone, and the exe, pwsh and
  that Terminal process all ended. `lidspike lid` (closed while it waited for the lid): restored 0.1 s after event 2,
  and every log line through `DONE` and `[end]` was written before the process ended, 1.1 s after the close. Ctrl+C
  typed into a Windows Terminal window was not tested by an agent.
- **After a close event, Windows ends the process as soon as the handler returns.** Microsoft's `HandlerRoutine` page
  (read 2026-11-03 04:20): for `CTRL_CLOSE_EVENT`, returning TRUE means "the system terminates the process"; the
  time-out is `SPI_GETHUNGAPPTIMEOUT`, 5000 ms. Seen above: `hold` ended right after its handler returned (it marks
  itself clean right after the restore); `lid` waits in its handler until its log is done (up to 4.5 s), so its log
  survived. Marking the lid tool clean right after the restore would cut its RESULT lines.
- **Logoff and shutdown never reach the spikes' console handlers.** Both spike exes import `user32.dll` (`dumpbin
  /imports`). The rule that a console app which loads user32 gets no `CTRL_LOGOFF_EVENT` or `CTRL_SHUTDOWN_EVENT` is
  from Microsoft's docs as the safety review quotes it; not re-read. The `HandlerRoutine` page (read 2026-11-03) says
  more broadly that interactive apps are ended at logoff and shutdown before those two signals are sent, so only
  services receive them.
- **PowerShell pipelines can stop a native exe early.** `lid-sleep-spike.exe read | Select-String ... |
  Select-Object -First 1` in pwsh 7.6.9 left a log with 2 lines; the same run with the output collected first logged
  everything. So step lists must not pipe a spike into `Select-Object -First`. Whether pwsh ended the process or its
  output pipe broke: not checked.
- **Thermal zone with the lid open on AC** read 55.1 C and 53.1 C at 03:47 and 03:51 during the dry runs (idle 41 to
  47 C at 02:45). The spikes' new default heat limit of 85 C sits above both and the 67 C full-load reading, and below
  `_CRT` 112.85 C.

## Agent reach on this laptop

Checked on 2026-11-03 at about 09:37 to 09:45 for card `docs/tasks/p0-morning-trim.md`, from the elevated agent
session, reads only. Scripts `probe.ps1` and `volume-read.ps1` are in that worker's scratchpad
(`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`).

- **Screenshots work while the session is unlocked.** `Graphics.CopyFromScreen` in pwsh 7 saved the 3840x2160
  primary screen to a PNG, and the agent read it back as an image (it showed the orchestrator chat). `LogonUI` was not
  running, so the session was unlocked. Not checked: a capture while locked or with the screen off (expected useless),
  and whether a capture shows toasts.
- **UI Automation loads.** `Add-Type -AssemblyName UIAutomationClient` worked in pwsh 7.6.9, and the root element
  found the taskbar (`Shell_TrayWnd`). Not tried: driving Settings, Lenovo Vantage or a tray menu with it.
- **Volume reads through Core Audio.** `IAudioEndpointVolume` on the default playback device: 24%, not muted. A write
  was not tried. The safety review of this card (`docs/reviews/p0-morning-trim-safety.md`, finding 6, read-only Core
  Audio scripts) found that the default device then was the DELL U2723QE (DisplayPort audio, 24%), and the laptop
  Speakers (Realtek) were at 67%, not muted. So volume steps must name the device by endpoint ID. The IDs (read
  2026-11-03 10:21 with `IMMDeviceEnumerator::GetDevice` and the `MMDevices\Audio\Render` registry names): Speakers
  `{0.0.0.00000000}.{fb152119-7d6e-4352-a9bf-882ae54afa03}` (67.3%), DELL
  `{0.0.0.00000000}.{ca5b6419-0bd3-4065-b7b7-0b7fc5450811}` (24.0%, default for all three roles while connected).
- **Power mode GUID.** Microsoft Learn, "Customize the Windows performance power slider" (read 2026-11-03): overlay
  `ded574b5-45a0-4f42-8737-46345c09c238` is "Best Performance", `961cc777-2547-4f9d-8174-7d86181b8a7a` "Better
  Battery", `3af9B8d9-7c97-431d-ad78-34a8bfea439f` "Better Performance". The same page says the slider appears only
  when Balanced or a plan derived from Balanced is active. Haik's AC overlay is `ded574b5-...`, so his Power mode is
  Best performance (the label in Settings was not looked at).
- **State at 09:37.** Theme Dark (`AppsUseLightTheme` 0, `SystemUsesLightTheme` 0). On AC, battery 100%.
  `powercfg /requests`: only SYSTEM, the orchestrator's keep-awake (`pwsh`, PID 11284, command line
  `keep-awake.ps1`). The tray spike's `HKCU\Control Panel\NotifyIconSettings\17618738229921704208` (its release exe)
  has no `IsPromoted` value. `HKCU\AppEvents\Schemes\Apps\.Default\SystemNotification\.Current` names
  `C:\WINDOWS\media\Windows Background.wav`, so the spikes' alias sound is that file. Lenovo Vantage is installed
  (`E046963F.LenovoCompanion` 10.2608.62.0, app id `E046963F.LenovoCompanion_k1h2ywk1493x8!App`).
- **The tray spike takes no power request.** `spikes/tray/src` has no `PowerSetRequest`, `PowerCreateRequest` or
  `SetThreadExecutionState` (grep), so it can sit idle in the tray during power tests. Its logs append, so a second
  run does not erase the first.
- **Opening the morning shell from an agent.** `morning-shell.log` lines at 03:41, 03:56, 04:26 and 04:34 come from
  agents that ran `explorer.exe ...morning-shell.cmd`: Medium, not admin, in a new Windows Terminal window since 04:26.
  So an agent can open it for Haik.
- **Not tried, by rule or choice:** a theme switch (registry or Settings), synthetic mouse or keyboard input, an
  Explorer restart (Haik's rule: agents end only processes they started), and any power setting change. (The theme
  switch and synthetic input were tried later the same day: see "Tray menus, synthetic input and the theme switch".)
- **UI Automation drives Settings** (2026-11-03 12:28 to 12:33, power card agent steps, from the elevated session,
  scripts `ab-uia.ps1` and `ab-tour.ps1` in the scratchpad above). `ms-settings:powersleep` and
  `ms-settings:signinoptions` open the pages; "Show more settings" buttons expand with ExpandCollapse; a combo box
  changes with Expand, SelectionItem.Select on the named list item, Collapse. The AC and DC combo boxes share a name
  ("Make my device sleep after"), so find them by AutomationId (`..._AC2_ComboBox`, `..._DC2_ComboBox`).
- **Tray menus, synthetic input and the theme switch** (2026-11-03 12:53 to 13:04, tray card agent steps, from the
  elevated session, scripts `tray-*.ps1` in the scratchpad above). pwsh 7's managed UI Automation shows a Win32 popup
  menu (`#32768`) as an empty Pane, and registering `UIAutomationClientSideProviders` fails (type not found). MSAA
  works: `AccessibleObjectFromWindow(menu, OBJID_CLIENT)` lists every item with name, role and state, and
  `accDoDefaultAction` runs an item or opens a submenu. In PowerShell the `IAccessible` object must be returned with
  `return , $o`, or the pipeline unrolls its children. `mouse_event` (absolute move, right button) inside the icon
  rect and `keybd_event` (Esc, Win+N) reach the Medium taskbar and spike. Esc goes to the foreground window, so send
  it only after checking that the foreground belongs to the target. Settings > Colors "Choose your mode" changes
  with Expand, SelectionItem.Select, Collapse on AutomationId `SystemSettings_Personalize_Color_ColorMode_ComboBox`.
  The notification centre (foreground `ShellExperienceHost`) gave no UIA element from this session.
- **AC can drop for a few seconds while docked.** Kernel-Power 105 (power source change) at 12:27:54 and 12:28:00 on
  2026-11-03, while the laptop sat on the DELL USB-C cable with no one touching it; a screen holder that checks
  `PowerLineStatus` every 5 s saw it as battery and ended. The cause was not checked. One more came in the
  night (setup then not checked): AC off at 01:50:47.1 and back at 01:50:52.4 (Kernel-Power 105 `AcOnline` false,
  then true; `Get-WinEvent`, read 2026-11-03).

## Phase 1 design checks

Checked 2026-11-03 between 14:37 and 15:06 for card `docs/tasks/p1-detailed-design.md`. Reads only; nothing installed
or changed.

- **Crate versions.** crates.io API `https://crates.io/api/v1/crates/<name>/versions` (newest first) and `cargo info`:
  `windows` 0.62.2 (2025-10-06, newest; MSRV 1.82), `windows-sys` 0.61.2, `serde` 1.0.229, `serde_json` 1.0.154,
  `embed-resource` 3.0.12, `proptest` 1.12.0, `jiff` 0.2.39, `croner` 4.0.2. `velopack`: newest stable 1.2.161
  (2026-09-29); 1.3.39-g8d2844f (2026-10-17) carries a semver pre-release tag.
- **windows-rs 0.100.** `windows-core`, `windows-result` and `windows-strings` are at 0.100.0 (2026-09-03), but
  `windows` and `windows-sys` are not (crates.io lists above). GitHub release 74 (2026-09-03) points to issue #4867,
  the August 2026 newsletter (read with `gh api`): 0.100 moves the crates to new metadata parsed from the SDK headers,
  MSRV 1.95; "We may initially publish the release without `windows` and `windows-sys`"; API shapes change (raw
  `HRESULT` values, header-named modules). The same release ships `windows-reactor` and `windows-window` as first real
  versions.
- **GitHub Actions** (`gh api repos/<repo>/releases/latest`): `actions/checkout` v7.0.1, `actions/upload-artifact`
  v7.0.2, `actions/cache` v6.1.0, `Swatinem/rust-cache` v2.9.3.
- **ARM64 runner image.** `actions/partner-runner-images`, `images/arm-windows-11-image.md` (image version
  20260105.41.1): Rust 1.92.0, Rustup 1.28.2, Rustfmt, Visual Studio Enterprise 2022. The live image may be newer, and
  whether it has the MSVC ARM64 tools the workspace needs: not verified.
- **Posted thread messages and modal loops.** Microsoft, `PostThreadMessageW` Remarks (page updated 2025-07-01): "if
  the recipient thread is in a modal loop (as used by MessageBox or DialogBox), the messages will be lost"; limit of
  10,000 posted messages per queue.
- **RunOnce.** Microsoft, "Run and RunOnce Registry Keys" (updated 2026-02-21): "By default, the value of a `RunOnce`
  key is deleted before the command line is run"; a `!` prefix defers the delete until after the command; command
  line at most 260 characters; a program run from these keys "should not write to the key during its execution";
  ignored in Safe Mode. HKLM RunOnce runs only for administrators; the HKCU key has no such note. Not tested here.
- **`Global\` named objects.** Microsoft, "Kernel object namespaces" (updated 2025-04-15): the namespaces cover
  events, semaphores, mutexes, waitable timers, file mappings, jobs and symbolic links. Only creating a file mapping or
  a symbolic link in the global namespace from a non-zero session needs SeCreateGlobalPrivilege. So a normal user may
  create a `Global\` mutex. Whether another user's process can open it (default security): not verified.
- **Secret scan patterns, dry run** (2026-11-03 15:48, card `p1-design-review-fixes`). The patterns in
  `phases/01-design-skeleton.md`, "Secret scan", parse as ERE: GNU `grep -rnIE -f` (Git Bash) over the working tree
  without `target/`. Hits: only redacted values in `spikes/claude-watch/fixtures/live-tests.log`, a test value in
  `spikes/claude-watch/src/watch.rs`, and the public Common Controls `publicKeyToken` in the tray spike manifest. No
  key, certificate, `.env`, credentials or exe file in the tree. Not run through `git grep` (no repo yet); git's own
  regex engine may differ.

## Repo files

Checked 2026-11-03 between 15:51 and 15:57 for card `docs/tasks/p1-repo-files.md`. Reads only.

- **MIT text.** SPDX `https://spdx.org/licenses/MIT.html` and `spdx/license-list-data` `text/MIT.txt` (downloaded
  with `Invoke-WebRequest`) give the same text once the page's optional words ("on", "(including the next
  paragraph)") are left out. `LICENSE` is `MIT.txt` byte for byte, except the copyright line (script compare).
- **Claude Code and git.** Claude Code docs, "Run parallel sessions with worktrees": worktrees go to
  `.claude/worktrees/<name>/` at the repo root, and "Add `.claude/worktrees/` to your `.gitignore` so worktree contents
  don't appear as untracked files". "Settings files and precedence": Claude Code adds
  `**/.claude/settings.local.json` to the global git excludes the first time it writes the file; a file made by hand
  needs its own `.gitignore` entry. On Windows the local file stays next to `.claude/settings.json`.
- **When git sees a text file as binary.** git source, `master` on GitHub: `convert.c` `convert_is_binary` treats a
  file with a lone CR, a NUL, or too many control bytes as binary, so `text=auto` stores it unchanged. Diff and
  `git grep -I` use another test, `buffer_is_binary` in `xdiff-interface.c`: a NUL in the first 8000 bytes.
- **Odd files in this repo** (a script with the same tests over every file, 15:56). Six spike logs are binary to
  `text=auto` and are stored unchanged: three `power-settings/out/copy-test-*` logs with lone CRs (`powercfg` output)
  and three `power-settings/out/hold-*` logs with NUL bytes (inside Windows event messages). One of them,
  `hold-limited-20261103-021916.log`, has a NUL in its first 8000 bytes, so `git grep -I` skips it. Not run through
  git (no repo yet).

## Workspace build

Checked 2026-11-03 between 16:02 and 16:12 for card `docs/tasks/p1-cargo-workspace.md`, on the Windows laptop.

- **Pinned toolchain.** A `rust-toolchain.toml` with `channel = "1.99.1"` needs a toolchain named
  `1.99.1-x86_64-pc-windows-msvc`; the existing `stable` (also 1.99.1) does not count. `rustup toolchain install 1.99.1
  --profile minimal` with rustfmt, clippy and both targets took 17 s. Under the pin, the rustup proxy runs the pinned
  toolchain's `rust-analyzer`, so the Claude Code LSP plugin needs `rustup component add rust-analyzer --toolchain
  1.99.1` (done here). `rustup show active-toolchain` in `spikes\tray` names the root file as the override.
- **Static CRT.** With `-C target-feature=+crt-static` in `.cargo/config.toml`, all four release exes (x64 and ARM64)
  import only `KERNEL32.dll`, `ntdll.dll` and `api-ms-win-core-synch-l1-2-0.dll` (`dumpbin /dependents`). A control
  build of `wakeshot.exe` with `RUSTFLAGS=-C debuginfo=0` imports `VCRUNTIME140.dll` and six `api-ms-win-crt-*` DLLs and
  is 120,320 bytes against 217,600. So Rust links the CRT DLL by default and the flag removes it, for about 97 KB.
- **RUSTFLAGS.** Cargo reference, "Configuration", `build.rustflags`: four sources, "mutually exclusive", first match
  wins: `CARGO_ENCODED_RUSTFLAGS`, `RUSTFLAGS`, then `target.<triple>.rustflags`, then `build.rustflags`. The control
  build above confirms it: any `RUSTFLAGS` drops the static CRT. With `--target`, config rustflags skip build scripts
  and proc macros.
- **Skeleton sizes.** Release, static CRT: x64 `wakeshot-tray.exe` 212,992 bytes, `wakeshot.exe` 217,600; ARM64
  210,944 and 216,064. PE machine `0x8664` and `0xAA64` (script reads the PE header).
- **`embed-resource` 3.0.12.** docs.rs page for `compile` and the crate source: since rustc 1.50 the resource links
  only to the bins (`cargo:rustc-link-arg-bins`), or to the lib when there are none; `manifest_required()` turns "not
  attempted" into an error. The SDK `rc.exe` was found for both targets with no setup. `mt.exe -inputresource:<exe>;#1`
  reads the embedded manifest back.
- **Manifest.** Microsoft, "Application manifests" (updated 2026-04-15): `assemblyIdentity` with `type`, `name` and
  `version` is required and must be the first child; one supportedOS Id, `{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`,
  covers Windows 10 and Windows 11; "Application manifests without a compatibility element default to Windows Vista
  compatibility on Windows 7".
- **Clock calls.** `windows` 0.62.2 source: `GetSystemTimePreciseAsFileTime` is under feature
  `Win32_System_SystemInformation` and returns `FILETIME` (`Win32_Foundation`); `QueryUnbiasedInterruptTime` is under
  `Win32_System_WindowsProgramming`. Microsoft (updated 2025-07-01): it counts 100 ns units without sleep or
  hibernation and fails only when called with a null pointer. The test reads wall time after 2026-01-01 and mono time
  above 0.
- **Spikes outside the workspace.** `cargo metadata --no-deps --manifest-path spikes\<name>\Cargo.toml` gives each
  spike's own folder as workspace root, for all four. The tray spike builds in release from its folder with the root
  toolchain and static CRT (676,864 bytes; Phase 0's 519,168-byte build had no static CRT and older code), and
  `--locked` leaves its `Cargo.lock` unchanged.

## Design verify checks

Checked 2026-11-03 between 16:07 and 16:10 for card `docs/tasks/p1-design-verify-fixes.md`. Reads only; no power
setting written.

- **End of the plan list.** A read-only script (`PowerEnumerate` with `ACCESS_SCHEME`, index 0 up, through PowerShell
  `Add-Type`) listed two plans (Balanced and the orchestrator's spike copy `d4adc592-...`), then returned 259
  (`ERROR_NO_MORE_ITEMS`) at index 2. Microsoft's `PowerEnumerate` page (updated 2024-02-22) names no end code: only
  `ERROR_SUCCESS`, `ERROR_MORE_DATA` and "a nonzero value if the call failed". The power spike's `schemes` loop
  (`spikes/power-settings/src/power.rs`) breaks on 259 and on any other error alike, so a failed call there looks like
  a shorter list.
- **Console notices.** Microsoft, `WM_WTSSESSION_CHANGE` (updated 2025-10-22): `WTS_CONSOLE_CONNECT` "The session
  identified by lParam was connected to the console terminal or RemoteFX session", `WTS_CONSOLE_DISCONNECT` the same
  for disconnected; sent only to windows registered with `WTSRegisterSessionNotification`. That fast user switching
  and Remote Desktop send them to the session that leaves: not verified (no test).
- **Abandoned mutex.** Microsoft, `WaitForSingleObject` (updated 2024-11-20): `WAIT_ABANDONED` means the owner thread
  ended without releasing the mutex; "Ownership of the mutex object is granted to the calling thread", and "If the
  mutex was protecting persistent state information, you should check it for consistency".
- **Panic hook.** Rust std docs, `std::panic::set_hook` (read 2026-11-03): "The panic hook is invoked when a thread
  panics, but before the panic runtime is invoked. As such, the hook will run with both the aborting and unwinding
  runtimes." So `std::process::abort()` in the hook ends the process before any unwinding. Whether proptest needs
  unwinding to shrink: not verified.
- **commit-msg pattern.**
  `'^[[:space:]]*(co-authored-by|claude-session):|generated with|anthropic\.com|claude\.ai/code'` with `grep -qiE`
  (GNU grep 3.0, Git Bash) on nine sample messages: rejects a co-author trailer, a `Claude-Session:` trailer with and
  without leading spaces, a tab before `co-authored-by:`, a bare `https://claude.ai/code/session_...` line and
  "Generated with Claude Code"; passes "Add Claude Code status file parser", "Read ~/.claude/sessions files"
  and "Initial commit". Not run as a real git hook (no repo yet).

## CI workflow

Checked 2026-11-03 between 16:14 and 16:25 for card `docs/tasks/p1-ci.md`: `gh api`, docs.github.com, and local runs
on the Windows laptop.

- **Action versions.** `gh api repos/<repo>/releases/latest`: `actions/checkout` v7.0.1, `actions/upload-artifact`
  v7.0.2, `actions/cache` v6.1.0 (same as "Phase 1 design checks"). The major tags `v7`, `v7` and `v6` point at those
  release commits (`gh api repos/<repo>/git/ref/tags/<tag>`). All three run on `node24` (their `action.yml`).
- **Runner labels.** docs.github.com, "GitHub-hosted runners" reference: Windows x64 `windows-latest`, `windows-2025`,
  `windows-2025-vs2026`, `windows-2022`; Windows arm64 `windows-11-arm`, `windows-11-vs2026-arm`; Ubuntu x64
  `ubuntu-latest`, `ubuntu-24.04`, `ubuntu-22.04`, `ubuntu-26.04`. `windows-latest` is Windows Server 2025 and
  `ubuntu-latest` is Ubuntu 24.04. No preview mark on `windows-11-arm`.
- **ARM64 image.** The current Windows 11 ARM64 readmes are in `actions/runner-images`, `images/windows/`
  (`Windows11-Arm64-Readme.md`, image 20261006.188.1, VS 2022; `Windows11-VS2026-Arm64-Readme.md`, image
  20261026.197.1, VS 2026 18.11); the copy in `actions/partner-runner-images` is older (20260105.41.1). Both current
  images list rustup 1.29.x, Rust 1.99.x, `VC.Tools.ARM64`, `VC.Tools.x86.x64` and Windows SDK 10.0.26100.0, which
  answers the open point in "Phase 1 design checks". Issue `actions/runner-images#14602`: `windows-11-arm` moves to
  the VS 2026 image between 2026-09-21 and 2026-09-30; the issue is still open, so which image the label gives today:
  not verified. The image script `Install-Rust.ps1` downloads the `aarch64-pc-windows-msvc` rustup-init on ARM64, so
  the toolchain there is native ARM64.
- **x64 and Linux images.** Windows Server 2025 image 20261021.299.1: rustup 1.29.1, Rust 1.99.1, VS 2022 17.14 with
  `VC.Tools.x86.x64` and `VC.Tools.ARM64`, SDK 10.0.26100.0, PowerShell 7.6. Ubuntu 24.04 image 20261026.354.1:
  rustup 1.29.2, Rust 1.99.1. Ubuntu installs rustup into `/etc/skel`, which becomes the runner's home; that `~/.cargo`
  is the cargo home at run time: not verified (the `actions/cache` Rust example assumes it).
- **Toolchain step.** `rustup toolchain install` with no toolchain name installs "by default the active toolchain"
  (rustup 1.29.2 help). `rustup toolchain install --no-self-update`, run from the repo root on an empty scratch
  `RUSTUP_HOME`, installed `1.99.1-x86_64-pc-windows-msvc` with 6 components (cargo, clippy, rustc, rustfmt, rust-std
  for both Windows targets) in 10 s. So it honors the profile, components and targets in `rust-toolchain.toml`.
- **pwsh steps.** docs.github.com, "Workflow syntax", "Exit codes and error action preference": the `pwsh` shell
  prepends `$ErrorActionPreference = 'stop'` and appends `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit
  $LASTEXITCODE }`, so only the last native exit code of a step counts. pwsh is the default shell on Windows runners.
- **CRT check.** `vswhere -latest -products * -find 'VC\Tools\MSVC\**\dumpbin.exe'` lists six `dumpbin.exe` here
  (Build Tools 2026). The CI step, run on this laptop's release builds: x64 and ARM64 `wakeshot.exe` and
  `wakeshot-tray.exe` import only `KERNEL32.dll`, `ntdll.dll` and `api-ms-win-core-synch-l1-2-0.dll`, and it passes. On
  a control build with `RUSTFLAGS=-C debuginfo=0`, which imports `VCRUNTIME140.dll` and six `api-ms-win-crt-*` DLLs,
  it fails. That the same lookup works on the runners: not verified until the first run.
- **actionlint** 1.7.12 (release exe, SHA-256 matches the release checksum list): 0 errors on `ci.yml`. It knows
  `windows-11-arm` and the `actions/upload-artifact@v7` inputs: a copy with a bad label and a bad input is flagged. Its
  label list lacks `windows-11-vs2026-arm` and `ubuntu-26.04` (neither is used). It does not check pwsh scripts.

## Contracts build

Checked 2026-11-03 between 16:24 and 16:34 for card `docs/tasks/p1-contracts.md`, on the Windows laptop, each build
in the worker's own target folder.

- **`velopack` 1.2.161 weight.** A scratch project outside the repo, `cargo add velopack@=1.2.161`, then
  `cargo tree -e normal --target x86_64-pc-windows-msvc`: 113 packages besides the project itself (`cargo add` locked
  148, for all targets). Among them `ring`, `rustls`, `rustls-webpki`, `webpki-roots`, `ureq`, `zip`, `zopfli`,
  `flate2`, `regex`, `uuid` and `xml`. Default features only; it also offers `file-logging`, `log-panics`,
  `public-utils`, `simplelog` and `time`. Whether `ring` builds for `aarch64-pc-windows-msvc` with the tools on this
  laptop: not verified. Not built here.
- **Fakes stay out of normal builds.** Both libs have a `test-support` feature that only the bins' dev-dependencies
  turn on (resolver 3). `cargo tree -p wakeshot-tray -p wakeshot-cli -e normal,features` lists only `default` for
  both libs; `-e dev,features` lists `test-support`. The release exes are byte-count identical to the workspace card's
  (x64 212,992 and 217,600; ARM64 210,944 and 216,064) after both bins gained both libs and `windows`, so an unused
  dependency adds nothing to an exe (sizes compared, contents not compared).
- **The core stays free of Windows crates.** `cargo tree -p wakeshot-core -e normal`: only `serde`, `serde_json` and
  their own dependencies. With dev-dependencies on the Windows host, `windows-sys` 0.61.2 and `windows-link` appear,
  through `proptest`'s dependencies (test builds only). `cargo tree -p wakeshot-core --target
  x86_64-unknown-linux-gnu` lists no `windows` crate. The core was not compiled for Linux here (no Linux target
  installed); CI does that.
- **A module named `core`.** `pub mod core;` in `wakeshot-core` builds and passes `clippy -D warnings` with serde and
  std derives in other modules. Not tried: a `use core::...` at the crate root, where Rust 2018 and later would see two
  `core` names.
- **Unknown journal keys.** serde `#[serde(untagged)]` over `Known(SettingKey)` and `Other(String)` reads `"sleepAc"`
  as `Other` and `"sleepDc"` as `Known`, and writes both back unchanged (`files::tests`).

## Sources track plan checks

Checked 2026-11-03 between 16:40 and 16:50 for card `docs/tasks/p4-plan-sources.md`. Reads only.

- **Claude Code cron syntax.** Docs "Run prompts on a schedule" (`code.claude.com/docs/en/scheduled-tasks`, read
  2026-11-03): `CronCreate` takes a standard 5-field expression `minute hour day-of-month month day-of-week`; every
  field takes `*`, single values, steps (`*/15`), ranges (`1-5`) and lists (`1,15,30`); day of week `0` or `7` is
  Sunday; `L`, `W`, `?` and names such as `MON` or `JAN` are "not supported". When both day fields are constrained, a
  date matches if either matches (vixie-cron). "All times are interpreted in your local timezone." Jitter as in
  `research/claude-code-signals.md`, plus: recurring tasks run late by up to half the gap to the next run, at most 30
  minutes; `*/5 * * * *` runs 15 s early; one-shots on `:00` or `:30` up to 90 s early. A task restored by `--resume`
  "doesn't fire for the runs it missed"; a self-paced `/loop` is not restored. Whether a range takes a step
  (`1-30/5`): not stated.
- **Cron and date crates.** `cargo info` (crates.io index, 2026-11-03): `croner` 4.0.2 has features `chrono` (default)
  and `jiff`; `jiff` 0.2.39 default features are `std`, `tz-system` (pulls `windows-link`), `tz-fat`,
  `tzdb-bundle-platform`, `tzdb-zoneinfo`, `tzdb-concatenated`, `perf-inline`. Exe size cost of either: not measured.
- **`windows` 0.62.2 features for the sources track.** Read in the crate source in the cargo registry: features
  `Win32_System_RestartManager` and `Win32_System_Time` exist. `RmStartSession`, `RmRegisterResources`, `RmGetList`,
  `RmEndSession` are in the first; `RM_UNIQUE_PROCESS` has `dwProcessId` and `ProcessStartTime` (a `FILETIME`), so a
  lock file holder comes back as pid plus start time. `GetDynamicTimeZoneInformation`,
  `SystemTimeToTzSpecificLocalTimeEx` and `TzSpecificLocalTimeToSystemTimeEx` are in the second. Whether Restart
  Manager lists Codex as the holder of its thread lock file: not verified (Phase 4 live test).
- **Rust `Command` on Windows does not use `PATHEXT`.** std docs, `Command::new` (read 2026-11-03): the program is
  searched in the child's `PATH` if set, the current exe's folder, the system folder, the Windows folder, then the
  parent's `PATH`; "the `.exe` extension may be omitted. Files with other extensions must include the extension".
  So `Command::new("npm")` does not find `npm.cmd`. The docs warn that `cmd.exe` and `.bat` files decode arguments
  their own way.
- **Ctrl+C ignore is inherited.** Microsoft, `SetConsoleCtrlHandler` (page updated 2021-12-30): with a NULL handler
  and TRUE, "the calling process [ignores] CTRL+C input ... This attribute of ignoring or processing CTRL+C is
  inherited by child processes". Handler routines are per process ("does not affect the list of handler functions
  for other processes"). So a wrapper that must leave Ctrl+C to its child installs a handler that returns TRUE
  instead of the NULL form. Not tested here.

## Power track plan checks

Checked 2026-11-03 between 16:45 and 16:50 for card `docs/tasks/p3-plan-windows-power.md`. Reads only: the
`windows` 0.62.2 crate source in the local cargo registry and `crates/wakeshot-windows/Cargo.toml`; nothing built or
run.

- **The process list needs one more `windows` feature.** `NtQuerySystemInformation` and the class value
  `SystemProcessInformation` are in `Wdk::System::SystemInformation` (feature `Wdk_System_SystemInformation`, which
  pulls in `Wdk_System`, `Wdk` and `Win32_Foundation` by the crate's own feature table). The record type
  `SYSTEM_PROCESS_INFORMATION` is in `Win32::System::WindowsProgramming`. At 16:45 `wakeshot-windows` declared
  `Win32_System_WindowsProgramming` but no `Wdk_*` feature, so the liveness fallback (the system process list, research
  "Reading elevated Claude Code sessions") did not compile without a contract change. The spike that used it
  (`spikes/claude-watch/Cargo.toml`) declared `Wdk_System_SystemInformation`. Out of date since `p1-contracts-split`
  added that feature ("Contracts split checks"); `p3-process` builds the list with it ("Process module checks").
- **Every other call the power track needs is behind a declared feature.** A script over the crate source found the
  module of 50 functions: power requests, plan reads, writes, copies and access checks, suspend and setting
  notifications, `SetSuspendState`, `IsPwrHibernateAllowed`, `GetSystemPowerStatus` (`Win32_System_Power`); process
  open, times, waits, mutexes, events, `RegisterWaitForSingleObject`, the parent process attribute and `CreateProcessW`
  (`Win32_System_Threading`); `GetShellWindow` (`Win32_UI_WindowsAndMessaging`); WTS session calls and
  `ProcessIdToSessionId` (`Win32_System_RemoteDesktop`); `LockWorkStation` (`Win32_System_Shutdown`); token privilege
  calls (`Win32_Security`); `MoveFileExW`, `FlushFileBuffers`, `FileTimeToLocalFileTime` (`Win32_Storage_FileSystem`);
  registry values (`Win32_System_Registry`); `SHGetKnownFolderPath` (`Win32_UI_Shell`); `GetLastInputInfo`
  (`Win32_UI_Input_KeyboardAndMouse`). That a function compiles with only its own module's feature: not verified (some
  take types from other modules); the spikes built these calls with the same feature list.
- **Local time for log lines.** `FileTimeToLocalFileTime` (declared) gives local time from UTC with the current time
  zone rules, enough for a line stamped "now"; its offset is local minus UTC. `GetDynamicTimeZoneInformation` and
  `SystemTimeToTzSpecificLocalTime` are in `Win32_System_Time` (declared since then; see "Log writer checks"). That
  `FileTimeToLocalFileTime` uses the current time zone and daylight saving settings: not verified (Microsoft's page
  not re-read; the log card checks it).

## Ship track planning checks

Checked 2026-11-03 between 16:40 and 16:50 for card `docs/tasks/p6-plan-ship.md`. Reads only; nothing installed,
built or changed.

- **`ring` needs clang for ARM64.** `ring`'s `BUILDING.md` (GitHub, `main`): for `aarch64-pc-windows-msvc` the Build
  Tools need the C++ ARM64 build tools and clang, with the Visual Studio LLVM folder on the PATH. `ring` comes into
  the tray with `velopack` (research "Contracts build" saw it in the tree). Build scripts run for `cargo clippy` too,
  so once `velopack` is declared, an ARM64 clippy, test or build of the workspace needs clang. Not tried here.
- **This laptop has no clang.** `Get-Command clang` finds nothing, and `vswhere -all -products * -find
  'VC\Tools\Llvm\**\clang.exe'` finds nothing in Build Tools 2026 18.11. So local ARM64 checks with `velopack` fail
  until the clang component is added (inferred from the two facts above; not tried).
- **Runner images have LLVM.** `actions/runner-images` readmes: `Windows11-Arm64-Readme.md` (image 20261006.188.1)
  lists LLVM 22.1.8 and the VS components `VC.Llvm.Clang` and `VC.Llvm.ClangToolset`, and .NET SDKs up to 10.0.401;
  `Windows2025-Readme.md` (image 20261021.299.1) lists LLVM 20.1.8, `VC.Llvm.Clang` and .NET SDKs up to 10.0.402.
  Whether `clang` is on the runner's PATH and `ring` builds there: not verified (the first CI run with `velopack`
  shows it).
- **`velopack` 1.2.161 manifest** (docs.rs, `Cargo.toml.orig`): features `default = []`, `file-logging`,
  `public-utils`; `ureq` 3.4 with no features named, so its TLS stack comes from `ureq`'s defaults and cannot be
  switched off from our side (features only add). That `ureq`'s defaults pick `rustls` with `ring`: inferred from the
  tree in "Contracts build", not read in `ureq`'s manifest.
- **`UpdateManager` API** (docs.rs 1.2.161): `new(source, Option<UpdateOptions>, Option<VelopackLocatorConfig>)`,
  `check_for_updates() -> Result<UpdateCheck, Error>`, `download_updates(&UpdateInfo, Option<Sender<i16>>)`,
  `apply_updates_and_restart`, `apply_updates_and_exit` (both "exit your app immediately"),
  `wait_exit_then_apply_updates(asset, silent, restart, restart_args)` ("You clean up any state and exit your app
  after calling this method"; the updater waits only 60 s for the exit), `get_update_pending_restart()`,
  `get_current_version()`. `UpdateManager` is `Clone`; whether it is `Send`: not shown. `UpdateOptions` fields:
  `AllowVersionDowngrade`, `ExplicitChannel` ("should usually be left None"), `MaximumDeltasBeforeFallback`.
- **Tools on this laptop.** `vpk` 1.2.161 is still installed (`~\.dotnet\tools\vpk.exe`, `vpk --help`). `winget
  --version`: v1.29.380, the release that fixed the Velopack install bug (`distribution.md`, section 2).

## Tray UX track plan checks

Card `docs/tasks/p5-plan-tray-ux.md`, 2026-11-03 16:50. A scratch crate outside the repo (`featcheck` in the card's
scratchpad, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`, toolchain 1.99.1, `--offline`) with `windows` 0.62.2 and exactly
the 32 features `crates/wakeshot-windows/Cargo.toml` declares, importing each API the tray track plans to call.

- **The declared features cover the tray track.** The crate built on `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc` with these imports: `Shell_NotifyIconW`, `Shell_NotifyIconGetRect`, `NOTIFYICONDATAW`,
  `NOTIFYICONIDENTIFIER`, `NIF_GUID`, `NIF_INFO`, `NIIF_USER`, `NIIF_LARGE_ICON`, `NIIF_NOSOUND`, `NIN_SELECT`,
  `NINF_KEY`, `NIN_BALLOONUSERCLICK`, `ShellExecuteExW`, `SHELLEXECUTEINFOW`, `AssocQueryStringW`,
  `SHQueryUserNotificationState` (`Win32_UI_Shell`); `CreateIconIndirect`, `DestroyIcon`, `ICONINFO`,
  `TrackPopupMenuEx`, `MENUITEMINFOW`, `WM_MENUCHAR`, `MNC_EXECUTE`, `ChangeWindowMessageFilterEx`,
  `AllowSetForegroundWindow`, `RegisterWindowMessageW`, `SystemParametersInfoW`, `SPI_GETHIGHCONTRAST`
  (`Win32_UI_WindowsAndMessaging`); `DRAWITEMSTRUCT`, `MEASUREITEMSTRUCT`, `LoadIconMetric` (`Win32_UI_Controls`);
  `MSAAMENUINFO`, `HIGHCONTRASTW`, `HCF_HIGHCONTRASTON`, `AccessibleObjectFromWindow`, `IAccessible`
  (`Win32_UI_Accessibility`); `GetDpiForMonitor`, `GetDpiForWindow`, `GetSystemMetricsForDpi`,
  `SetProcessDpiAwarenessContext` (`Win32_UI_HiDpi`); `DwmSetWindowAttribute`, `DWMWA_WINDOW_CORNER_PREFERENCE`,
  `DWMWA_BORDER_COLOR`, `DWMWCP_ROUND`; `CreateDIBSection`, `CreateCompatibleDC`, `CreateFontIndirectW`, `DrawTextW`,
  `RoundRect`, `ExcludeClipRect`, `GetSysColor`, `MonitorFromPoint`; `RegGetValueW`; `IMMDeviceEnumerator`,
  `MMDeviceEnumerator`, `IAudioClient`, `IAudioRenderClient`, `IAudioSessionControl`, `PKEY_AudioEndpoint_FormFactor`,
  `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM`, `WAVEFORMATEXTENSIBLE`; `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`
  (`Win32_Media_Multimedia`); `PKEY_Device_FriendlyName`; `IPropertyStore`; `CoInitializeEx`, `CoCreateInstance`;
  `GetProcAddress`, `LoadLibraryW`. So no tray card needs a new `windows` feature.
- **Not in `windows` 0.62.2:** `NIN_KEYSELECT` (Win32 defines it as `NIN_SELECT | NINF_KEY`; the crate has both
  parts). `DRAWITEMSTRUCT` and `MEASUREITEMSTRUCT` live in `Win32::UI::Controls`, not `WindowsAndMessaging`. Both seen
  as compile errors on the first try.
- **A file module can have child modules without moving to `mod.rs`.** `tray.rs` with `pub mod art;` resolved to
  `tray/art.rs` (the build ran and printed the child's constant). So the contracts card can declare `tray`'s children
  in `tray.rs` and keep the file name.

## Contracts split checks

Card `p1-contracts-split`, 2026-11-03. Pinned toolchain 1.99.1, own `CARGO_TARGET_DIR`, no `RUSTFLAGS`.

- **A folder under `crates/` without a `Cargo.toml` breaks the workspace; a plain file does not.** Checked in a
  scratch workspace with `members = ["crates/*"]` and `cargo metadata --no-deps --offline`: a stray `crates/shared.rs`
  is ignored (exit 0, one member); a `crates/build/` folder fails with "failed to load manifest for workspace member".
  So the shared build file is `crates/build-resources.rs`, not the `crates/build/resources.rs` the general review
  suggested.
- **The new features exist in `windows` 0.62.2 and change nothing in `Cargo.lock`.** `Wdk_System_SystemInformation`,
  `Win32_System_RestartManager` and `Win32_System_Time` added; `cargo clippy` and `cargo build --release` with
  `--locked` pass on x64 and ARM64.
- **The clippy run without `--all-targets` catches what the review predicted.** Its first run failed on an import used
  only by a fake (`std::sync::Mutex` in `sink.rs`), while the `--all-targets` run passed. Fixed with a `cfg`.
- **The core's tree on every target has no `windows` crate.** `cargo tree -p wakeshot-core -e normal --target all
  --prefix none --locked` lists only `serde`, `serde_core`, `serde_derive`, `serde_json`, `itoa`, `memchr`, `zmij`,
  `proc-macro2`, `quote`, `syn`, `unicode-ident`. The CI step's `grep -E '^windows'` was run in Git Bash on that output
  (no hit) and on a made-up line `windows-sys v0.61` (hit).
- **The app icon is in both exes.** A placeholder `assets/wakeshot.ico` (a ring, 16, 32 and 48 px, 32-bit frames) is
  embedded through `1 ICON` in the generated `.rc`; a script found all three frames byte for byte in all four release
  exes (x64 and ARM64).
- **The CI `dumpbin` pick.** On this laptop `vswhere -find` lists six `dumpbin.exe` (`Hostx64` and `Hostx86`, each for
  `arm64`, `x64`, `x86`), the first being `Hostx64\arm64`; the new rule picks the first under the runner's own host
  folder (`Hostx64` here). Whether the `windows-11-arm` image has a `HostARM64` folder: not verified.
- **A `std::sync::Mutex` guard as a temporary blocks a second lock in the same statement.** The first `FakePlans` test
  hung on `fake.script(fake.state().calls.len(), ..)`: the guard from `state()` lives to the end of the statement.
  Its doc now says so.

## Track plan fixes checks

Checked 2026-11-03 by `p1-track-plan-fixes`, for the per-card check rule (cross-track review B1). Tools: rustfmt
1.10.0 and cargo 1.99.1 (the pinned toolchain), on throwaway files in the session scratchpad, not on `crates/`.

- **A bare `rustfmt` uses the 2015 edition.** With no `rustfmt.toml` up the tree, `rustfmt --check` on a file with
  `pub async fn` failed with "`async fn` is not permitted in Rust 2015"; `--edition 2024` parsed it (and rejected
  `let gen`, a 2024 keyword). The repo has no `rustfmt.toml`, so a card that checks its own files passes
  `--edition 2024`.
- **`rustfmt` on a file also checks the child modules it declares.** `rustfmt --check lib.rs`, where `lib.rs` says
  `mod child;`, printed the diff of an unformatted `child.rs` and exited 1. So `--check` on a module-list file reports
  other cards' files, and `rustfmt` without `--check` on it would rewrite them.
- **`cargo test` takes one test-name filter before `--`.** `cargo test alpha:: beta::` failed: "unexpected argument
  'beta::' found". After `--` several filters work and run tests matching any of them (`cargo test --lib -- alpha::
  beta::` ran the tests of both modules).
- **`real_` with `--ignored` runs every ignored test whose name holds `real_`, in any module.** `cargo test --lib
  real_ -- --ignored` ran `alpha::tests::real_alpha` and `beta::tests::real_beta`; `cargo test --lib -- --ignored
  alpha::` ran only the first. So a real-laptop card filters by its own module (review m15).

## Close contracts checks

Checked 2026-11-03 by `p1-close-contracts`, with the pinned toolchain (cargo 1.99.1), its own build folder in the
session scratchpad and no `RUSTFLAGS`.

- **`GetCommandLineW` needs the feature `Win32_System_Environment`.** In the cached `windows-0.62.2` source,
  `src/Windows/Win32/System/mod.rs` declares `pub mod Environment;` under `#[cfg(feature =
  "Win32_System_Environment")]`, and `GetCommandLineW` in `Environment/mod.rs` has no other `cfg`. Microsoft's
  `windows-docs-rs` page for 0.62.2 puts it in `windows::Win32::System::Environment`. docs.rs has no page for that
  version of the crate (404). A feature of `windows` does not change `Cargo.lock` (checked: the lock gained no line for
  it).
- **`proptest` 1.12.0 is the newest and not yanked.** crates.io API (`/api/v1/crates/proptest`): max version 1.12.0,
  `yanked: false`. It was already locked for the core, so adding `proptest.workspace = true` to the CLI's
  dev-dependencies downloaded nothing: `cargo metadata --offline` wrote one line to `Cargo.lock`, ` "proptest",` in
  the `wakeshot-cli` entry, and nothing else (diff against a copy taken first).
- **No `match` in `crates/` lists every `Event` or `Notice` variant** (grep for `Event::` and `Notice::`), so the two
  new variants compile everywhere without other edits. `text::notice` takes any `Notice` and formats it with
  `{notice:?}` until `p5-text` words it.
- **A scratch copy of the workspace builds and tests alone.** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
  `.cargo\`, `assets\` and `crates\` copied to the scratchpad: `cargo test --locked -p wakeshot-core` there passed
  (24 tests) with its own `CARGO_TARGET_DIR`. A mutant planted in the copy's `files.rs` (the newer-version check
  moved by 100) failed `files::tests::read_ignores_newer_files`, and the repo's `files.rs` hash stayed the same. The
  exes' build scripts read `assets\wakeshot.ico` through `..\..\assets`, so a copy without `assets\` cannot build
  the bins. This is the rule in `docs/phases/02-core.md` note 20 (verify review Major 2).
- **Whole-workspace checks after the changes** (x64 and ARM64): `cargo fmt --all --check`, both clippy runs per
  target, `cargo test --workspace --locked` (36 tests: core 24, windows 10, cli 1, tray 1) and both release builds:
  all exit 0.

## Log writer checks

Checked 2026-11-03 by `p3-log` on this laptop (local NTFS SSD), with the tests in `wakeshot-windows/src/log.rs` and
Microsoft's pages read the same day.

- **Appends from several processes never mix.** This answers architecture section 8's "not verified (Phase 3)".
  `log::tests::appends_from_many_processes_never_mix` starts its own test exe as 4 children with one start time. Each
  opens `wakeshot.log` with `FILE_APPEND_DATA` only (share read, write and delete) and appends 2000 lines, one
  `WriteFile` each: payloads of 0 to 179 bytes, every 50th one 3000 to 5199 bytes, so some lines cross a 4 KiB page.
  Six runs: 2,083,667 bytes each, every line whole, each child's lines all there and in order, 7826 to 7936 switches
  between children (of 8000 lines), so the writes really overlapped. Network shares, FAT, exFAT or ReFS: not verified.
- **`FILE_APPEND_DATA` without `FILE_WRITE_DATA`** (Microsoft, "File Access Rights Constants"): "For local files,
  write operations will not overwrite existing data if this flag is specified without FILE_WRITE_DATA." The page says
  nothing about one process's write against another's; the test above is the evidence.
- **Roll with the file open elsewhere** (`log::tests::roll_with_file_open_elsewhere`). `MoveFileExW` with
  `MOVEFILE_REPLACE_EXISTING` on `wakeshot.log` while another writer (delete sharing) has it open: works, and that
  writer's next lines land in the moved file. A reader without delete sharing on `wakeshot.log`: fails with 32
  (`ERROR_SHARING_VIOLATION`). A reader that shares delete (std `File::open`) on the target `wakeshot.2.log`: fails
  with 5 (`ERROR_ACCESS_DENIED`), so no roll happens while an app holds the oldest file open. No line was lost in any
  case.
- **The log's UTC offset matches Windows.** `FileTimeToLocalFileTime` on the current time gives local minus UTC of
  +240 minutes; `GetDynamicTimeZoneInformation` (`Bias` plus the standard or daylight bias its return value names)
  gives the same (`log::tests::local_offset_matches_windows`); `Get-Date -Format zzz` printed `+04:00`. Yerevan has no
  daylight saving time, so a switch was not tested.
- **`FileTimeToLocalFileTime` page** (Microsoft Learn, updated 2025-07-01). Remarks, in full: to account for daylight
  saving time when converting a file time to a local time, use `FileTimeToSystemTime`,
  `SystemTimeToTzSpecificLocalTime` and `SystemTimeToFileTime` instead. The page no longer says which time zone
  settings the call uses. So "it applies the bias in force now" (asked in "Power track plan checks") is inferred from
  that remark, not verified. The log writes the offset it applied next to each time, so local time minus the offset is
  the exact UTC instant either way.
- **A directory listing showed the true size of a file still open for append.** A scratch PowerShell check appended
  2 MiB through an open .NET `FileStream` (append, share read, write and delete): `FileInfo.Length`, `dir` and
  `DirectoryInfo.EnumerateFiles` all showed 2,097,157 bytes before the close. Same process only; another process's
  open handle: not verified. The roll reads the size from its own handle anyway (`GetFileSizeEx`).
- **`Win32_System_Time` is declared now.** `crates/wakeshot-windows/Cargo.toml` lists it, and the log test's call to
  `GetDynamicTimeZoneInformation` compiles. So the line "which is not declared" in "Power track plan checks" is out of
  date.

## Restore routine checks

Checked 2026-11-03 by `p2-ownership-restore`, with the core's tests against `ownership::fake::FakePlans` (a model of
Windows, not Windows itself).

- **Restore step 5 as written can drop a value still in force.** Step 5 says entries on an inactive plan need no
  apply. If "inactive" is judged by step 2's plan read, a switch to that plan after step 2 and before step 4 writes its
  `original` brings our stored value into force; the write then changes only the stored value, the entry is dropped,
  and our value stays in force with no entry. Shown by
  `restore::tests::plan_switch_before_the_write_applies_the_plan_now_active`: the routine judges "inactive" by step 5's
  second read instead and calls `set_active` on the plan now active. It rests on two points the design already marks
  as not tested: a switch applies the new plan's stored values, and a write is applied only by `PowerSetActiveScheme`.
- **The stop-point walk holds on the fake.** `restore::tests::a_stop_at_any_call_keeps_every_held_value_journaled`
  stops at each call of a run over two plans and three keys, from two starting states (all ours, 19 calls; mixed
  marks plus an entry with an unknown key, 17 calls). After each stop, every value still ours (stored, or in force on the active
  plan) has a known entry, and RunOnce is set while one exists; the next run puts every value back. A stop between
  the file delete and the RunOnce clear leaves the RunOnce value (and a stop before the guard's stop event, the guard)
  with no journal, as walk R6 in `p1-design-verify-fixes` says. So the power track's note 4 check "RunOnce is gone
  after the next restore" holds only for stops before the delete.
- **Mutants.** In a scratch copy of the workspace, each failed at least one test: a failed plan list read as an empty
  list; `original` written before the mark; marked entries dropped when `set_active` fails; an entry dropped after a
  failed read (the clean-up then runs); RunOnce and the guard released after any failure.
- **`cargo` on this shell.** In this session's PowerShell and Git Bash, `cargo` was not on `PATH`;
  `%USERPROFILE%\.cargo\bin\cargo.exe` exists and works when called by path.

## Process module checks

Checked 2026-11-03 for card `docs/tasks/p3-process.md` on this laptop (Windows 11 Pro 10.0.26300), with the tests in
`crates/wakeshot-windows/src/process.rs`: `cargo test --locked -p wakeshot-windows process:: -- --nocapture` and
`cargo test --locked -p wakeshot-windows -- --ignored process:: --nocapture`. The tests print what each call returned.

- **The test runner here is elevated, with SeDebugPrivilege on.** `whoami /groups` in the agent's shell: High
  Mandatory Level; `whoami /priv`: SeDebugPrivilege Enabled. The test's own token read: High (S-1-16-12288),
  elevation type Full. So Haik's agent sessions on this laptop run elevated, and their children inherit an enabled
  SeDebugPrivilege.
- **SeDebugPrivilege passes the DACL, so a denied open needs a token without it.** With the runner's token, an open
  with exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION` worked on both `csrss.exe` processes and on a child
  made with an empty DACL (`D:`). With the same token minus every privilege but SeChangeNotify
  (`CreateRestrictedToken(DISABLE_MAX_PRIVILEGE)`, impersonated on the test thread), all three opens failed with
  `0x80070005`, and the system process list still had each pid with the same creation time, so `liveness` said Yes.
  Not checked: a real Medium caller without the impersonation (the restricted token keeps the High label).
- **An ended process leaves the system list at once, even while a handle to it is open.** A child was killed and
  waited on, with its handle still open: `NtQuerySystemInformation(SystemProcessInformation)` had no record with its
  pid, while `OpenProcess` by pid still worked and a 0 ms wait on that handle was signaled. One check per test run, in
  three runs. So the list needs no thread count check.
- **The list's parent pid and image name.** `Reserved2` in the `windows` crate's `SYSTEM_PROCESS_INFORMATION` (in the
  NT layout `InheritedFromUniqueProcessId`; winternl.h does not name it) held the real parent pid: this test process
  for a child from `std::process::Command` and for one from `CreateProcessW`, Explorer for the outside-tree child.
  `ImageName` for this process was the exe file name only, `wakeshot_windows-a7dbc494c7637936.exe` (37 characters,
  not cut to 15), no path. The creation time at `Reserved1` plus 24 bytes equalled `GetProcessTimes` for this process
  and for each child.
- **Explorer as the parent works, from High and from Medium.** `spawn_outside_tree` opened Explorer (the process of
  `GetShellWindow`, pid 12528) with only `PROCESS_CREATE_PROCESS` and passed it as
  `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`. From the elevated runner: the child's parent pid in the list was Explorer's,
  and its token was Medium (S-1-16-8192), elevation type Limited: Explorer's token, not the caller's. Then the test exe
  was started that way (so with Medium, Limited) and ran the same test, writing a report file: it passed, so a Medium
  process may use Explorer as the parent. Not tested: how Task Manager groups such a child (checklist 4), and that it
  also takes Explorer's job (Microsoft's UpdateProcThreadAttribute page says it does; not re-read this session).
- **Exit waits.** `RegisterWaitForSingleObject` with `WT_EXECUTEONLYONCE`, on a duplicate of the process handle:
  fired once after the child was killed, though the caller had closed its own handle first; no second call within
  300 ms. `UnregisterWaitEx(wait, INVALID_HANDLE_VALUE)` before the exit: the callback never ran (checked 300 ms
  after the exit). The same call while a callback ran (it slept 300 ms) returned only after the callback finished.
- **Command line quoting.** `CommandLineToArgvW` split the line `command_line` builds back into the exact arguments,
  among them an empty one, one with spaces, one with quotes, `C:\dir\`, `C:\my dir\` and `back\\"quote`.

## Store checks

Checked 2026-11-03 for card `docs/tasks/p3-store.md`, by the tests in `store::tests` (Windows 11 Pro 10.0.26300,
NTFS, files in `%TEMP%`, test keys under `HKCU\Software\Wakeshot-test`), unless a line names another source.

- **`MoveFileExW` cannot replace a file a reader has open.** `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` over a
  target that another handle had open returned 5 (`ERROR_ACCESS_DENIED`), even when that handle was opened with
  read, write and delete sharing (first run of `atomic_write_replaces_a_file_a_reader_has_open`). The same 5 came
  back when the holder had no delete sharing.
- **A rename by handle with POSIX semantics can.** `SetFileInformationByHandle(FileRenameInfoEx)` with
  `FILE_RENAME_FLAG_REPLACE_IF_EXISTS | FILE_RENAME_FLAG_POSIX_SEMANTICS`, on a `DELETE` handle of the temp file,
  replaced the target while a reader with delete sharing held it; the reader then still read the old bytes, whole,
  and a new open read the new bytes. When the holder had no delete sharing it failed too, and the old file stayed
  whole with no temp file left (`atomic_write_keeps_old_file_on_failure`). So `store::write_atomic` tries
  `MoveFileExW` first and this rename on 5. Rust's `std::fs::rename` does the same: `MoveFileExW`, then on
  `ERROR_ACCESS_DENIED` the same `FileRenameInfoEx` call with the same two flags, returning the first error if it
  fails (source read 2026-11-03, `library/std/src/sys/fs/windows.rs`, `rename_inner`, rust-lang/rust master).
- **Delete with a reader.** `DeleteFileW` of a file a reader held with delete sharing succeeded, the name was free at
  once (a read of it said missing, a new write of the same name worked), and the reader still read the old bytes
  (`remove_and_rewrite_while_a_reader_has_the_file_open`).
- **Write-through.** Microsoft, `MoveFileExW` (updated 2025-07-01): `MOVEFILE_WRITE_THROUGH` "The function does not
  return until the file is actually moved on the disk. Setting this value guarantees that a move performed as a copy
  and delete operation is flushed to disk before the function returns." The temp file sits beside the target, so the
  move is never a copy and delete. What a power cut right after the move leaves: not tested by us. Nightcrew writes
  the same way (temp file, flush, `MoveFileEx(REPLACE | WRITE_THROUGH)`; `docs/research/rivals-nightcrew-lidguard.md`).
- **Missing versus error on read.** `CreateFileW(OPEN_EXISTING)` gave "missing" (2 or 3) for a missing file and for a
  file in a missing folder; a folder path (no `FILE_FLAG_BACKUP_SEMANTICS`) failed with another error
  (`read_whole_tells_missing_from_error`; the code was not printed).
- **Abandoned mutex.** A thread that took a named mutex and ended without a release: the next wait returned
  `WAIT_ABANDONED` with ownership, and a wait after the release returned `WAIT_OBJECT_0`
  (`abandoned_mutex_counts_as_taken`). A `Global\` mutex was created and taken by the test runner, whose integrity
  level was High (0x3000): this shell runs elevated, so worker test runs here do not cover Medium (that check is
  `p3-real-checks`). Microsoft, `CreateMutexExW` (updated 2024-11-20): for a named mutex that existed, "the return value
  is a handle to the existing object, and the GetLastError function returns ERROR_ALREADY_EXISTS".
- **Registry.** `RegGetValueW` with `RRF_RT_REG_SZ` on a `REG_DWORD` value returned 1630 (`ERROR_UNSUPPORTED_TYPE`)
  (`auto_start_refuses_other_types`). A get or a delete of a value under a `Run` key that did not exist yet came back
  as missing (2 or 3), and a `REG_SZ` with Armenian letters came back unchanged (`auto_start_round_trip`).
- **Pid check for the key sweep.** `OpenProcess(SYNCHRONIZE)` by pid of an ended child whose handle the test still
  held worked, and a 0 ms wait was signaled, so the sweep deleted that pid's key; the live test process's key stayed
  (`sweep_skips_live_pids`). Not tested: an open denied for a pid of another user's process (the sweep keeps such a
  key).

## Hold file checks

Checked 2026-11-03 for card `docs/tasks/p4-hold-files.md`.

- **`procStart` from PowerShell.** `(Get-Process).StartTime.ToFileTimeUtc()` compared with the raw creation time from
  `GetProcessTimes` (P/Invoke through `Add-Type`, `PROCESS_QUERY_LIMITED_INFORMATION`) for every process the shell
  could open: equal for 337 of 337 in PowerShell 7.6.9 and 332 of 332 in Windows PowerShell 5.1 (1 skipped in each).
  Run elevated. Not tested: a process that started inside the hour a daylight saving change repeats.
- **Wall ms for the `expires` tests.** Every expected value in `holds::file` tests came from .NET
  `[DateTimeOffset]::Parse(text, InvariantCulture).ToUnixTimeMilliseconds()` in pwsh 7.6.9, for example
  `2026-11-03T18:00:00Z` = 1793728800000, `9999-12-31T23:59:59Z` = 253402300799000,
  `2026-11-03T18:00:00.1234567-05:30` = 1793748600123 (fraction cut, not rounded, as ours).
- **Date algorithms.** Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms"
  (`howardhinnant.github.io/date_algorithms.html`, read 2026-11-03): "Consider these donated to the public domain."
  `holds::file` uses `days_from_civil` and `civil_from_days` from there, rewritten in Rust; no credit line is needed.
- **The user doc's examples.** The PowerShell and cmd blocks of `docs/hold-files.md` were cut from the doc and run
  in a scratch folder (only the folder line and the work line swapped) under pwsh 7.6.9, Windows PowerShell 5.1 and
  cmd. Each wrote its file with the pid of the process that ran it (the cmd one through PowerShell's
  `ParentProcessId`), UTF-8 with no byte order mark (`[IO.File]::WriteAllText`) or ASCII plus CRLF (cmd `echo`), and
  deleted it at the end. The bytes parse with `holds::file::parse` (test `files_the_doc_examples_wrote_parse`).
  `ToString('o')` on a UTC time gives `2026-11-03T20:05:00.2920348Z`, seven fraction digits, in both PowerShells.
- **RFC 3339 grammar.** `rfc-editor.org/rfc/rfc3339.html`, read 2026-11-03. Section 5.6: `time-secfrac = "." 1*DIGIT`,
  `time-numoffset = ("+" / "-") time-hour ":" time-minute`, `time-offset = "Z" / time-numoffset`,
  `date-time = full-date "T" full-time`; a note says "T" and "Z" "may alternatively be lower case"; another says
  applications "may choose" a space instead of `T` (an option, not the grammar, so `parse_expires` refuses it).
  Section 5.7: `time-second` "may have the value "60" at the end of months in which a leap second occurs". Section
  4.3: `-00:00` means the UTC time is known and the local offset is not.
- **The examples' rename over an existing file.** In a scratch folder, `Move-Item -Force temp final` (pwsh 7.6.9 and
  Windows PowerShell 5.1) and `move /y temp final` (cmd) each replaced an existing `final` with the new content and
  left no temp file. Whether each does it in one rename or as a delete plus a move: not verified.

## Tray view checks

Checked 2026-11-03 for card `docs/tasks/p2-present.md`.

- **Tooltip and balloon sizes.** Microsoft Learn, `NOTIFYICONDATAW` (`learn.microsoft.com/en-us/windows/win32/api/
  shellapi/ns-shellapi-notifyicondataw`, page dated 2024-11-20, read 2026-11-03): `WCHAR szTip[128]`; "For Windows
  2000 and later, szTip can have a maximum of 128 characters, including the terminating null character". In the `W`
  struct a character is one `WCHAR`, one UTF-16 unit, so `text::TOOLTIP_MAX_UTF16 = 127` matches. Same page:
  `szInfo[256]` (256 with the null, "should be restricted to 200 characters in English") and `szInfoTitle[64]` (64
  with the null, "should be restricted to 48 characters in English"). Not tested here: what Windows 11 shows for a
  tooltip of exactly 127 units.

## Power rules checks

Checked 2026-11-03 for card `docs/tasks/p2-policy-power.md`.

- **The dock's short AC drops last longer than 5 s.** All Kernel-Power 105 events in the System log
  (`Get-WinEvent -FilterHashtable @{ LogName = 'System'; ProviderName = 'Microsoft-Windows-Kernel-Power'; Id = 105 }`,
  28 events from 2026-11-02 00:35 to 2026-11-03 18:38, read 2026-11-03 20:10), paired `AcOnline` false then true:
  two short drops, 5.3 s (01:50:47.050 to 01:50:52.369) and 6.3 s (12:27:54.032 to 12:28:00.285). Every other
  battery period lasted 86 s or more. So the 5 s settle in `policy::lid` (product, "Never lower security") does not
  cover the drops seen. Not verified: whether Wakeshot's `GUID_ACDC_POWER_SOURCE` callbacks come at the same times
  as these events, and what caused the drops.

## Journal seam checks

Checked 2026-11-03 for card `docs/tasks/p3-journal.md`, by the tests in `power::journal::tests` (Windows 11 Pro
10.0.26300, NTFS, files in `%TEMP%`, test keys under `HKCU\Software\Wakeshot-test`, test runner elevated, High),
unless a line names another source.

- **A folder in place of `journal.json` is a failed read, not a missing file.** `store::read_whole` on it failed in
  `CreateFileW` with 5 (`ERROR_ACCESS_DENIED`), so `FileNet::load_journal` gives `Failed` and the restore routine
  fails the run rather than taking the journal as gone (`load_tells_missing_corrupt_failed`, printed). A folder open
  needs `FILE_FLAG_BACKUP_SEMANTICS`, which `read_whole` does not pass; that this 5 is the same under a Medium token:
  not tested.
- **`std::fs::rename` replaces a target a reader has open.** `journal.json` renamed over an older
  `journal.json.corrupt` that another handle held open with read, write and delete sharing: `Ok`, and the target then
  held the new bytes (`set_aside_keeps_bytes`). This matches the std source read for `p3-store` ("Store checks":
  `MoveFileExW`, then on 5 a rename by handle with POSIX semantics). A rename with no source gave 2
  (`ERROR_FILE_NOT_FOUND`).
- **The journal mutex excludes a second caller of the entry point.** A first `restore_locked` (inner form with the
  wait as an argument) held `Local\Wakeshot.Test...Journal` inside the routine; a second caller on another thread with
  a 100 ms wait gave up after 112 ms with `WaitForSingleObject` 258 and ran no step of the routine; once the first
  finished, a third caller with a 0 ms wait took it (`lock_excludes_second_caller`).
- **An abandoned journal mutex is taken through the entry point.** A thread took the test mutex and ended without a
  release; the next `restore_locked` got it (`WAIT_ABANDONED`), ran the routine and logged the abandon; the run after
  that took it plainly (`lock_abandoned_counts_as_taken`). The same Windows behavior through `store` alone is in
  "Store checks".
- **Not tested: an elevated and a Medium process on one `Local\` mutex.** Every test here ran High. A mutex created
  by an elevated process (a `wakeshot restore` from an admin terminal) may refuse a Medium tray's open while that
  process holds a handle; the tray's apply then fails at its safety net step for that one try. Belongs to
  `p3-real-checks`.

## Power plan checks

Checked 2026-11-03 for card `docs/tasks/p3-power-plan.md`. The real run was 20:27:02 to 20:27:06, on AC with the
dock, from an elevated (High) agent shell, with `cargo test --locked -p wakeshot-windows -- --ignored
--test-threads=1 --nocapture power::plan::` (four `real_` tests in `crates/wakeshot-windows/src/power/plan.rs`).

- **A write to an inactive plan copy changes only that copy** (architecture section 9, the first Phase 3 test).
  `real_write_to_inactive_copy_changes_only_the_copy`: a `PowerDuplicateScheme` copy of the active Balanced plan,
  named "Wakeshot test <pid>", never made active. `PowerWriteACValueIndex` lid 1 to 0, `PowerWriteDCValueIndex` lid
  1 to 0 and sleep after 600 to 0, each on the copy, each returned 0. Our read and `powercfg /qh <copy>` showed each
  new value on the copy. Balanced stayed active (`PowerGetActiveScheme`) and its five values (lid AC 1, DC 1, sleep
  AC 10800, DC 600, energy saver DC 30) were the same before and after, by our read and by `powercfg /qh` in the
  test and from a shell after it. This covers stored values. That such a write leaves the values in force alone
  follows from "Stored at once, applied only on `PowerSetActiveScheme`" above; not measured here. Still not tested:
  that activating a plan later applies its stored values (the second point under architecture "Plan switch").
- **The plan list ends with 259 here.** `PowerEnumerate` with `ACCESS_SCHEME` and a 16-byte buffer: index 0 gave
  Balanced, index 1 returned 259 (`ERROR_NO_MORE_ITEMS`); with a copy present the list had two plans and ended at
  index 2; after the copy's delete, one plan again. Each successful call left the size at 16. Microsoft's page
  (updated 2024-02-22, read 2026-11-03) says what `BufferSize` holds only for `ERROR_MORE_DATA`, not after success.
- **`PowerSettingAccessCheckEx` with `KEY_WRITE`** returned 0 for lid AC, lid DC (setting
  `5ca83367-...`) and sleep after DC (`29f6c1db-...`). Elevated only; the Medium result for the same keys is in
  "Power settings spike" above.
- **A copy's delete leaves nothing.** Each of the three copies (`50df20c5-...`, `15920ad3-...`, `09f0c480-...`) was
  gone from `PowerEnumerate` right after `PowerDeleteScheme`, gone from `powercfg /list`, and its key under
  `HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes` did not exist (`Test-Path`).
- **Memory the calls return.** Microsoft Learn: `PowerGetActiveScheme` (updated 2025-07-01) "Use the LocalFree
  function to free this memory"; `PowerDuplicateScheme` (updated 2025-11-25) "If the pointer contains NULL, the
  function allocates memory for a new GUID ... The caller can free this memory using LocalFree", and lists 183
  `ERROR_ALREADY_EXISTS` when the destination names an existing plan; `PowerDeleteScheme` (updated 2024-02-22) names
  no code for the active plan (the spike saw 50, "Plan copy" above). All read 2026-11-03.
- **GUID names in the `windows` crate 0.62.2** (`Win32::System::SystemServices`, read in the local registry source):
  the lid action is `GUID_LIDCLOSE_ACTION` (`5ca83367-...`), sleep after is `GUID_STANDBY_TIMEOUT`, their subgroups
  `GUID_SYSTEM_BUTTON_SUBGROUP` and `GUID_SLEEP_SUBGROUP`, the energy saver threshold
  `GUID_ENERGY_SAVER_BATTERY_THRESHOLD` in `GUID_ENERGY_SAVER_SUBGROUP`. Each equals the spike's literal
  (`setting_guids_match_the_spike`).
- **`powercfg /q` with the aliases LIDACTION and ESBATTTHRESHOLD prints no value here** (only the plan line);
  `powercfg /qh` with the subgroup and setting GUIDs prints both indexes. Why: not checked (likely hidden settings,
  not verified).
- **`cargo clippy -p wakeshot-windows -- -D warnings` also fails on warnings in `wakeshot-core`.** At 20:26 another
  card's `wakeshot-core/src/state.rs` had unfulfilled `#[expect(dead_code)]` attributes: `cargo test -p
  wakeshot-windows` printed them as warnings and passed, while the clippy run stopped with "could not compile
  `wakeshot-core` (lib) due to 25 previous errors". So `-D warnings` reaches every workspace crate the build lints,
  not only the `-p` package (cargo's clippy wrapper is a workspace wrapper; that reason is from memory, not
  verified in the cargo docs). A card's own clippy check can therefore fail on a half-done file in another crate.

## Apply and ownership check checks

Checked 2026-11-03 by `p2-ownership-apply`, with the core's tests against `ownership::fake::FakePlans` (a model of
Windows, not Windows itself).

- **Two Win32 codes.** `ERROR_FILE_CORRUPT` is 1392 and `ERROR_INVALID_DATA` is 13: read in the `windows` 0.62.2
  source in the local cargo registry (`src/Windows/Win32/Foundation/mod.rs`; `windows-sys` 0.61.2 says the same).
  The apply and the check use them for two failures Windows gives no code for: a journal that does not parse, and a
  read-back that returns another value.
- **Apply step 3's takeover text, read literally, puts back an old value.** "A key that already has an entry for
  this plan keeps its `original` and its `restored` mark". For an entry the user changed while a restore waited (the
  check skips marked entries, so no `UserChanged` came), keeping that `original` makes the next restore write the
  value from before the user's change. The apply takes an entry over only while it is ours (live equals `written`, or
  marked with live equal to `original`, restore step 3's two "ours" cases); any other entry becomes a new one with the
  live value as `original`. Shown by `apply::tests::an_entry_no_longer_ours_gets_the_live_value_as_original`.
- **A stop walk over all three taken-over entries hides a dropped E1.** With a mutant that clears the mark on
  takeover, a stop between step 3's save and the write leaves E1 unmarked with `original` stored and ours in force;
  the next restore drops it. Beside E2 or E3 on the same plan the restore still calls `set_active` for them, which
  brings E1's stored `original` into force, so the walk passed. Walking each entry alone caught it
  (`apply::tests::takeover_then_a_stop_at_any_call_drops_no_held_value`). Stop-point tests of other tracks
  (`p3-stop-points`) should keep one starting state with E1 as the only entry on its plan.
- **The stop walk holds on the fake for the apply.** A stop at each call of an apply from six starting states
  (fresh; E1, E2 and E3 together with an unknown-key entry; each of the three alone; a released key's entry with an
  unknown-key entry), then a fault-free restore: at the stop every held value has a known entry with RunOnce set and the guard running, and the restore
  puts every value back.
- **The mutant "call `set_active` with the step 1 id" cannot fail a test.** The call runs only when the second read
  equals the command's plan, which step 1 also checked, so both ids are equal there (an equivalent mutant). The
  variant that makes the call with the step 1 id whatever the second read returns failed two tests.
- **The scratch copy for mutants needs `docs\` now.** `holds/file.rs` tests read `docs/hold-files.md` through
  `include_str!`, so a copy without `docs\` (`docs/phases/02-core.md` note 20's list) does not compile.

## Core state machine checks

Checked 2026-11-03 for card `docs/tasks/p2-core-holds.md`.

- **proptest writes no regression file when `failure_persistence` is `None`.** proptest 1.12.0 source in the cargo
  registry (`src/test_runner/runner.rs`, near line 637): a failing case is saved only inside
  `if let Some(ref mut failure_persistence) = self.config.failure_persistence`. `scenarios/holds.rs` sets `None`, so
  a failure there leaves no file in `proptest-regressions/`. Not tested with a real failure.
- **`#[expect(dead_code)]` works field by field, and fails the build once the field is read.** rustc 1.99.1:
  `state.rs` puts one on each field a later card fills; `cargo clippy -p wakeshot-core` with and without
  `--all-targets`, both targets, `-D warnings`: clean. In a scratch copy, one read of such a field (`owner`) gave
  `error: this lint expectation is unfulfilled` (`-D unfulfilled-lint-expectations` implied by `-D warnings`), so a
  later card must drop the attribute when it fills the field.
- **Checked ends must lie off the 30 s safety tick in `Sim` tests.** A mutant that drops every core deadline
  survived all hold scenarios while each checked end (30 minutes, 1 hour, 90 s) was a multiple of 30 s from the
  start: the safety tick alone hit it to the millisecond. With the ends moved 7 s off that grid, six tests caught it.

## Power request checks

Checked 2026-11-03 by `p3-requests`.

- **A set adds to a count.** `PowerSetRequest` "increments the count of power requests of the specified type", and
  `PowerClearRequest` decrements it (Microsoft docs, `PowerSetRequest` page, read 2026-11-03). So `PowerRequests`
  sets each type at most once and a re-take clears before it sets; a second set would need a second clear before the
  PC may sleep. The same page: a DISPLAY request needs a SYSTEM request beside it to keep the PC from sleeping, and
  on Modern Standby on DC, SYSTEM and EXECUTION requests end 5 minutes after the sleep timeout.
- **Closing the handle.** `PowerCreateRequest`'s page says to use `CloseHandle` "to free the handle and clean up the
  object"; on failure it returns `INVALID_HANDLE_VALUE` with the error in `GetLastError` (read 2026-11-03). Whether a
  close alone ends outstanding requests: not verified, so the drop clears every type first. Whether the call copies
  the reason text: not documented, not verified; the text is kept as long as the handle.
- **Real calls work here.** `power::requests::tests::request_sets_and_clears` (create, set SYSTEM and EXECUTION,
  re-take, clear, close) returned `Ok` for every call, elevated, on this laptop, in well under a second. Judged by the
  calls' results only, never by `powercfg /requests` (architecture section 5).

## System events checks

Checked 2026-11-03 by `p3-system-events`. Microsoft pages read that day (date is each page's `updated_at`); the runs
were on this laptop, elevated (High), on AC through the dock, lid open.

- **Nothing documents a callback after unregister.** `PowerSettingUnregisterNotification` (2025-07-01) and
  `PowerUnregisterSuspendResumeNotification` (2024-02-22) give only the parameter and "ERROR_SUCCESS (zero) if the
  call was successful". `DEVICE_NOTIFY_CALLBACK_ROUTINE` (2024-02-22) says nothing about threads or unregister. So
  `power::system` routes each callback through a table by id: a late callback finds no entry and posts nothing.
- **Unregister waited for a running callback.** A probe test (removed after the run) registered for the lid with a
  receiver that slept 300 ms: `PowerSettingUnregisterNotification`, called while it slept, returned after 300.4 ms;
  with no sleep, 19 us. One run, lid setting only; the suspend and resume unregister was not tried. Not documented,
  so nothing relies on it, but it is why a drop must not hold a lock the callback takes.
- **The first callback comes on another thread, after register returns.** Same probe: register returned in 14 to
  114 us with no callback yet; the first callback came 53 to 373 us after the register call began, on a thread other
  than the registering one. Two registrations in one run. Not documented; the code also works when the callback comes
  inside the call.
- **Whether Windows copies `DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS`** at register or reads it later: not documented, not
  verified. The parameters are leaked (16 bytes per registration).
- **First values.** `real_registration_gives_first_values` got lid open, `Event::Power` equal to
  `GetSystemPowerStatus` (on AC, a battery percent), and the active plan equal to `PowerGetActiveScheme`, all well
  inside 2 s (the whole test took under 0.1 s). `real_lid_once_returns_open_here`: `Some(false)`, open.
- **`GUID_ACTIVE_POWERSCHEME` is not on the Power Setting GUIDs page** (2025-07-16); the page lists
  `GUID_POWERSCHEME_PERSONALITY` instead. The GUID is in the `windows` crate (`SystemServices`, from winnt.h), and its
  16-byte GUID payload is what the power settings spike saw.
- **Payload values** (Power Setting GUIDs page): lid 0 closed, 1 opened; power source 0 AC, 1 battery, 2 short-term
  (UPS); battery percent a DWORD 0 to 100. `POWERBROADCAST_SETTING` (2024-02-22): `DataLength` is the size of `Data`.
- **`PBT_APMRESUMESUSPEND` follows `PBT_APMRESUMEAUTOMATIC`** "if the system detects any user activity after"
  it, and `PBT_APMRESUMEAUTOMATIC` "is delivered every time the system resumes" (its page, 2025-07-16). So only the
  second gives `Event::Resumed`.
- **`SYSTEM_POWER_STATUS`** (2024-02-22): `ACLineStatus` 0 offline, 1 online, 255 unknown; `BatteryFlag` 128 no
  system battery, 255 unknown; `BatteryLifePercent` 0 to 100 or 255 unknown. `power_status` counts 255 as AC, so an
  unknown source never lets the battery floor hibernate.
- **`GetLastInputInfo`** (2025-07-01): the failure return names no `GetLastError`; the input tick "is not guaranteed
  to be incremental", and `SendInput` "supplies its own tick count". So `input_idle_ms` reads a tick just after now
  as 0 idle (fresh input) instead of 49.7 days. It is per session.

## Session checks

Checked 2026-11-03 by `p3-session`. Reads only: no lock, sign-out, sleep or setting change by this card.

- **Session messages** (Microsoft docs, read 2026-11-03). `WM_WTSSESSION_CHANGE`: wParam is the code
  (`WTS_CONSOLE_CONNECT` 1, `_DISCONNECT` 2, `WTS_REMOTE_CONNECT` 3, `_DISCONNECT` 4, `WTS_SESSION_LOGON` 5, `_LOGOFF`
  6, `_LOCK` 7, `_UNLOCK` 8, `_REMOTE_CONTROL` 9, `_CREATE` and `_TERMINATE` reserved, `_DESKTOP_READY` 0xF), lParam
  the session id. `WTSRegisterSessionNotification` with `NOTIFY_FOR_THIS_SESSION` gets only this session's
  notifications; every register needs an unregister, made before the window is destroyed; a call made before the
  Remote Desktop services have started may fail with `RPC_S_INVALID_BINDING` (they are ready once
  `Global\TermSrvReadyEvent` is set). Whether a tray started at sign-in can ever hit that error: not verified.
- **End of session** (Microsoft docs, `WM_ENDSESSION`, read 2026-11-03). wParam TRUE: the session ends any time after
  the handlers return; FALSE: it does not end. lParam is a bit mask: `ENDSESSION_CLOSEAPP` 0x1 (the Restart Manager
  closes the app: a file in use must be replaced, servicing, or resources exhausted), `ENDSESSION_CRITICAL`
  0x40000000, `ENDSESSION_LOGOFF` 0x80000000; 0 means shutdown or restart. `session::decode` maps them all the same,
  since in each the tray is about to end.
- **Console session id** (Microsoft docs, `WTSGetActiveConsoleSessionId`, read 2026-11-03): 0xFFFFFFFF while no
  session is attached to the console, "for example, if the physical console session is in the process of being
  attached or detached". So `on_console()` reads `false` at such a moment. Whether a tray started at sign-in can see
  it: not verified.
- **Lock state flags** (Microsoft docs, `WTSINFOEX_LEVEL1_W`, read 2026-11-03): `SessionFlags` is a `LONG`;
  `WTS_SESSIONSTATE_LOCK` 0, `_UNLOCK` 1, `_UNKNOWN` 0xFFFFFFFF; the lock and unlock values are swapped on Windows 7
  and Server 2008 R2 only.
- **`LockWorkStation`** (Microsoft docs, read 2026-11-03): asynchronous; nonzero means the lock has started, not that
  the session is locked. It works only from a process on the interactive desktop.
- **Real reads here** (elevated agent shell, desktop session). At 20:44:20 `on_console()` gave `Ok(true)` and
  `locked_now()` gave `Some(false)`. At 20:45:29.8 Windows entered Modern Standby with reason "Idle Timeout" (System
  log, Kernel-Power 506, with 566 "SessionUnlock" at the same time) and `LogonUI.exe` started at 20:45:29: Windows
  locked the session at screen off, as in the lid spike ("Lock"). Nothing in this card locks. At 20:45:33
  `locked_now()` gave `Some(true)` and `on_console()` still `Ok(true)`; at 20:45:40 `locked_now()` again gave
  `Some(true)`. So the lock read follows the real state both ways. The `Some(false)` read ran on the code before a
  clippy-only change (the union field is now read before the `Level == 1` test instead of inside a closure), so on the
  final code only `Some(true)` was seen; `p3-real-checks` step 2 runs `real_locked_now_here` again.
- **Registration on a hidden window.** `session::tests::register_on_hidden_window`: a never-shown top-level window of
  the system `STATIC` class; register, drop, register again and `unregister()` all returned without error.

## Power IO checks

Checked 2026-11-03 for card `docs/tasks/p3-power-io.md`, by the tests in `power::executor::tests` and
`io_queue::tests` (Windows 11 Pro 10.0.26300, test build, which unwinds on panic; temp folders and `Names::test`
mutexes; test runner elevated, High).

- **A stop inside `restore_locked` releases the journal mutex.** In `startup_stop_points`, `FakePlans` panicked at
  every call of the startup restore of three starting states, also while `restore_locked` held its test journal
  mutex. After each stop another thread took that mutex with a 0 ms wait. The other thread matters: a Windows mutex
  counts per owning thread, so the same thread would take it again even if it were never released. A release build
  aborts on panic instead, and the next taker then gets `WAIT_ABANDONED` ("Journal seam checks").
- **A set-aside over an older `.seen` keeps the newer bytes.** `corrupt_journal_gives_event_once` set two corrupt
  journals aside through `FileNet::dry_run` and marked each seen; `journal.json.corrupt.seen` then held the second
  file's bytes, as "Journal seam checks" found for `std::fs::rename` over an existing target.
- **Not tested: the executor on real plans.** Every test here ran on `FakePlans`; `p3-harness` and `p3-real-checks`
  drive it over a plan copy.

## Watch reads checks

Checked 2026-11-03 for card `docs/tasks/p4-watch-reads.md`, by the tests in `watch::read::tests` (Windows 11 Pro
10.0.26300, NTFS, files in `%TEMP%`; test runner elevated, High) unless a line names another source. No check here
depends on elevation: share modes and byte range locks apply to every token.

- **A reader with full sharing blocks no writer.** While a handle with read, write and delete sharing was open, a
  second handle wrote the file in place, `std::fs::rename` replaced it, and `std::fs::remove_file` deleted it; each
  returned `Ok` (`whole_read_shares_with_a_writer_that_writes_renames_and_deletes`). A writer that shared no read
  made the read fail at once with 32 (`ERROR_SHARING_VIOLATION`), not wait
  (`whole_read_fails_at_once_while_a_writer_shares_no_read`).
- **std's defaults on Windows.** Read in the toolchain's std source (Rust 1.99.1, `library/std/src/sys/fs/windows.rs`):
  `OpenOptions::new()` sets `share_mode` to `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`;
  `DirEntry::file_type` and `DirEntry::metadata` come from the `WIN32_FIND_DATAW` of the listing (no open);
  `fs::metadata` opens the path with access 0 and `FILE_FLAG_BACKUP_SEMANTICS`, falling back to `FindFirstFileExW` on
  32 or 5. `FileType::is_dir` and `is_file` are false for a reparse point whose tag has the name surrogate bit
  (`0x20000000`: symlinks and junctions), so a walk that follows only `is_dir` never enters a folder link.
- **Win32 name trimming.** With a file `x.key` in a scratch folder, `cmd /d /c type` of `x.key.`, `x.key ` (trailing
  space), `x.key. . ` and `x.key\..\x.key` each printed its content, and .NET `File.ReadAllText` of `x.key::$DATA`
  did too (cmd refused that name). So a name filter that sees `x.key.` or `x.key::$DATA` but not `x.key` can still
  open `x.key`; `read_whole` refuses names that end in a dot or a space or hold a `:`
  (`name_guard_refuses_a_key_file_even_with_a_crafted_path`, with the key file held open with no sharing, so any open
  of it would fail with 32 instead of the guard's `InvalidInput`).
- **A rename over a file changes its file id.** `GetFileInformationByHandleEx(FileIdInfo)` gave a new id for the file
  `std::fs::rename` put under the old name, and the same id after an in-place truncate
  (`tail_restarts_when_the_file_is_replaced`, `tail_restarts_when_the_file_gets_shorter`). The tail compares ids, not
  creation times, because NTFS tunneling can give a new file under a recently freed name the old creation time (from
  memory of Microsoft's docs: not verified). FAT, which has no stable ids: not tested.
- **A listing does not disturb a lock.** With one file opened with no sharing and another locked whole by
  `LockFileEx(LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)`, `names` listed both; afterwards a second handle's
  `LockFileEx` failed with 33 (`ERROR_LOCK_VIOLATION`), an open of the no-sharing file failed with 32, and the holder's
  `UnlockFileEx` returned `Ok` (`names_lists_locked_files_without_disturbing_the_lock`).
- **Folder times for "newest first".** `File::set_modified` on a folder opened with `FILE_WRITE_ATTRIBUTES` and
  `FILE_FLAG_BACKUP_SEMANTICS` set the time the listing then reported (`find_takes_exactly_one_level_with_decoys`,
  `find_takes_exactly_three_levels_with_decoys`). When Codex's date folders and Claude's project folders get a new
  write time in practice (an entry added or removed, not a file in it growing): not verified.

## Core settings checks

Checked 2026-11-03 for card `docs/tasks/p2-core-settings.md`.

- **PowerShell drops a bare `--` passed to a `.ps1` script.** `& cargo.ps1 clippy ... -- -D warnings` reached cargo
  without the `--` (cargo: `unexpected argument '-D' found`); the same call with `'--'` quoted ran clippy. A helper
  script that forwards `@args` to cargo needs the quoted form.
- **The settings scenarios catch the card's mutants.** Twelve mutants planted by hand in a scratch copy, each run
  against `cargo test --locked -p wakeshot-core scenarios::` and reverted: all twelve failed at least one test (list in
  the card's Result). A mutant that clears the released keys at a plan switch (the effect of keying them by plan)
  failed two tests; one that lets the retry take the active plan's entries failed one; one that applies after
  `Session(Ending)` failed three, the property among them.
- **`settings_events_in_any_order` runs 10,000 cases in about 10 s.** `PROPTEST_CASES=10000`, debug build, this laptop:
  9.9 s, no failure.

## Watch data checks

Checked 2026-11-03 for card `docs/tasks/p4-watch-data.md`, by the tests in `watch::data::tests` (Windows 11 Pro
10.0.26300, NTFS, data folders on `%TEMP%` through `DataDir::at`; test runner elevated, High).

- **A write time set to the 100 ns comes back exact.** `File::set_modified` with a `SystemTime` built from a FILETIME
  (2026-11-03 12:00:00 UTC, and that minus or plus 100 ns as process starts) gave the same FILETIME from
  `MetadataExt::last_write_time` on a later handle, so the "started at or before the write time" rule was tested at
  its edge (`missing_proc_start_counts_a_process_started_by_the_write_time`).
- **A hold file held with no sharing reads as error 32**, not as missing, so it is retried and then logged, never
  read as a deleted hold (`an_unreadable_file_is_retried_then_logged`).

## Hibernate thread checks

Checked 2026-11-03 for card `docs/tasks/p3-hibernate.md`: Microsoft's pages read that day, and the real tests in
`power::hibernate::tests` (Windows 11 Pro 10.0.26300, this laptop, lid open, AC; test runner elevated, High). Nothing
called `SetSuspendState`.

- **`SetSuspendState` page** (updated 2024-02-22): nonzero on success, zero on failure with `GetLastError`; "The
  calling process must have the SE_SHUTDOWN_NAME privilege"; `bForce` "has no effect"; `bWakeupEventsDisabled` TRUE
  "disables all wake events". The page does not say the call returns only after resume; that comes from the morning
  runs ("Lid and sleep spike"). Whether it can return TRUE without a hibernate (a driver veto, say): not verified.
- **`IsPwrHibernateAllowed` page** (updated 2024-02-22): TRUE "if the computer supports hibernation (power state S4)
  and the file Hiberfil.sys is present". The page also says the function "may be altered or unavailable in subsequent
  versions" and that apps for Vista and later "should use GetPwrCapabilities instead"; its Remarks say the value is
  in `SystemS4`.
- **Read here:** `IsPwrHibernateAllowed` true; `GetPwrCapabilities` `SystemS4` true, `HiberFilePresent` true
  (`hibernate_allowed_reads`, printed with `--nocapture`). Same values as the lid spike.
- **`AdjustTokenPrivileges` page** (updated 2025-07-01): a success can still carry `ERROR_NOT_ALL_ASSIGNED` (1300),
  "the token does not have one or more of the privileges"; `TOKEN_QUERY` is needed only with a previous state, so
  the enable opens the token with `TOKEN_ADJUST_PRIVILEGES` alone.
- **SeShutdownPrivilege off and on in the test process** (`privilege_enables`): turned off, read back off with
  `GetTokenInformation(TokenPrivileges)`; then the enable returned `Ok` and the read showed it enabled. Elevated
  token only; the Medium token case is the lid spike's (`AdjustTokenPrivileges` ok, last error 0), not re-run here.

## Recovery checks

Checked 2026-11-03 for card `docs/tasks/p3-recovery.md` on this laptop (Windows 11 Pro 10.0.26300, test runner
elevated, High), by the tests in `crates/wakeshot-windows/src/recovery.rs` and `crates/wakeshot-tray/tests/recovery.rs`
(`cargo test --locked -p wakeshot-windows recovery::`, `cargo test --locked -p wakeshot-tray`, with `--nocapture`),
unless a line names another source.

- **A guard under Explorer opens a stop event an elevated process made with default security.** A one-off test
  (removed after the run) made a named manual-reset event with `CreateEventW(NULL, ...)` in the High test process,
  then started this test exe through `process::spawn_outside_tree` (`outside_tree: true`, so Explorer's token, Medium
  per "Process module checks"; the child's own level was not read in this run). The child opened the event with
  `SYNCHRONIZE` only, was still waiting 2 s later, and left when the test set it (exit 10). So `StopEvent` uses the
  default security. Why the default DACL of an elevated token admits the Medium child: not verified.
  `tray_guard_stop_sets_event` repeats the same path on every run with the shipped `StopEvent` and `TrayGuard`.
- **A `SYNCHRONIZE`-only event handle cannot set the event, and the event goes with its last handle.** `SetEvent`
  on the guard's handle failed with 5; after the last handle closed, `OpenEventW` failed with 2; a second
  `CreateEventExW` of a live name returned a handle and `GetLastError` 183 (`ERROR_ALREADY_EXISTS`), which `create`
  reports as `None` (`stop_event_create_open_set`).
- **libtest takes `--` as the end of its options.** A child started as `<test exe> --exact --ignored
  --test-threads=1 --quiet -- <role test> --guard <pid> <start> <event>` ran only the role test and got the guard
  arguments through untouched (the `TrayGuard` tests: the role child exited with the codes it computes from them).
- **The real guard's parent is Explorer.** `TrayGuard` started `wakeshot-tray.exe --guard ...` (debug build) from the
  elevated test process; `Get-CimInstance Win32_Process` gave its `ParentProcessId`, and `Get-Process` named it
  `explorer`, pid 12528 (`real_guard_parent_is_explorer`, run once with `--ignored`). How Task Manager groups it:
  not verified (checklist 4).
- **A guard under Explorer writes its log into a data folder an elevated process made.** In
  `tray_guard_runs_the_real_guard_until_stopped` the High test made the temp data folder; the guard `TrayGuard`
  started (`outside_tree: true`) created `logs\wakeshot.log` in it and wrote its start and "stopped" lines, which the
  test read back. The folder was under `%TEMP%`; the real `%LocalAppData%\Wakeshot`: not tested.
- **How fast the exe modes end.** The guard exe left 10 to 13 ms after its tray was ended through its handle or its
  stop event was set (two runs, debug build). `--restore` with no journal took 26 ms from spawn to exit; the first
  run of a freshly built exe took 0.95 s (why: not verified).
- **`std::env::set_var` is sound on Windows in a threaded program.** Its doc in the pinned toolchain's
  `library/std/src/env.rs` (1.99.1) says it "is also always sound to call on Windows, in single-threaded and
  multi-threaded programs". The tray's exe tests set `WAKESHOT_DATA_DIR` that way, under a lock, for the guards
  `TrayGuard` starts with the test's own environment.
- **Not tested here:** the guard's fallback wait when its tray cannot be opened (a denied open needs a token without
  SeDebugPrivilege, "Process module checks"); that `WaitForMultipleObjects` returns the lowest index when the tray
  and the event are both signaled (Microsoft's documented rule, which the guard relies on to check the journal first).

## Claude status file checks

Card `docs/tasks/p4-claude-status.md`, 2026-11-03.

- **Every status file write of the 2.1.317 live sessions reads as the card's rules say.** The 13 cases in
  `crates/wakeshot-core/tests/fixtures/claude/live/*.status.jsonl` hold 150 writes and 17 deletes of
  `sessions/<pid>.json`: 20 with no `status` yet, 52 `busy`, 67 `idle`, 6 `shell`, 5 `waiting` / `dialog open`
  (counted with `ConvertFrom-Json`). Test `every_live_status_write_reads` feeds each one to `ClaudeSource`: every
  write gives `FormatHealth::Ok`, the pid and `procStart` of the file, and the activity the card's mapping gives;
  every delete drops the session. No `permission prompt`, `input needed` or Remote Control off write is in that set;
  those shapes come from the 2.1.316 log (`spikes/claude-watch/fixtures/live-tests.log`), rebuilt as whole files in
  `tests/fixtures/claude/status/`.
- **serde fills a struct from a JSON array.** A one-off test (serde 1.0.229, serde_json 1.0.154; added and removed)
  read `[4242, "busy"]` into a derived struct with two `Option<Value>` fields as `pid: 4242, status: "busy"`, and
  `[4242]` failed with "invalid length 1, expected struct P with 2 elements". So the status parser checks that the
  first value opens with `{`; without it, an array with one value per read key would pass as a status file.
- **`procStart` comes before `startedAt`, by 0.4 s to 21 s.** In the same live fixtures, the first write of each of
  the 17 sessions (16 pids; 8784 ran twice): `startedAt` minus `procStart` (FILETIME / 10,000 minus
  11,644,473,600,000 for Unix ms) was 8.1 to 20.9 s for the 10 fresh terminal sessions and 0.4 to 0.9 s for the 7
  resumed or SDK ones. The fresh terminal ones
  waited on the folder trust dialog, which comes before the status file (signals doc, "Other facts seen live"); that
  the gap is that dialog: not verified. So the process start, not `startedAt`, is the earliest time a line can come
  from this process (for the resume rule of `p4-claude-live-shapes`).
- **`CLAUDE_CONFIG_DIR` moves the whole Claude folder.** Claude Code docs, `code.claude.com/docs/en/env-vars` (read
  2026-11-03): "Override the configuration directory (default: `~/.claude`). All settings, session history, and
  plugins are stored under this path", suggested for running several accounts side by side. That `sessions\` and
  `projects\` follow it: not verified (no live test). `ClaudeSource` reads only `<home>\.claude\sessions`.
- **Not verified:** whether the System process (pid 4) reports a creation time of 0; the parser refuses a
  `procStart` of 0 and a pid of 0 either way, so no file can tie a session to a process that never ends by those
  values.

## Core lid checks

Checked 2026-11-03 for card `docs/tasks/p2-core-lid.md`, by the scenarios in `scenarios::lid` and hand-planted
mutants in a scratch copy of the workspace.

- **The plan copy lags an apply, so the lid rules need what is in force.** The core keeps the last plan read; after an
  apply it shows the user's values (lid 1, Sleep) until the next read, up to 10 s later, while the journal copy says
  Wakeshot wrote 0. `policy::lid::ours` on those values finds no entry whose `written` equals the live value, so a lid
  close in the first seconds of a hold would get no lock. The core now passes the policy a view where each journal
  entry on the active plan sets its key to `written`, and an apply not answered yet counts as written. A mutant that
  drops the view failed 14 of the lid scenarios, among them `lid_closed_in_the_first_seconds_of_a_hold_locks`.
- **The lid scenarios catch the card's mutants.** 19 mutants in two batches (`cargo test --locked -p wakeshot-core`,
  the second batch `-- scenarios::`, since another card's `sources::cron` test failed for a while), each reverted: all
  19 failed at least one test (list in the card's Result). The card's four: a lock at a lid close on the user's own
  Do nothing, no lock at AC to battery with the lid closed, a hibernate without the resume clamp, `Play(GoingToSleep)`
  with `Hibernate`.
- **`lid_events_in_any_order` runs 10,000 cases in about 27 s.** `PROPTEST_CASES=10000`, debug build, this laptop:
  27.4 s, no failure.
- **Not verified: what `LockWorkStation` returns for a session that is already locked.** Microsoft's page could not be
  fetched from the agent shell (the fetch tool refused `learn.microsoft.com`). The core sends `Lock` into a locked
  session too, so a lost `Unlocked` never skips a lock, and a failure while locked is only logged, so either answer is
  safe.

## Stop point checks

Checked 2026-11-03 for card `docs/tasks/p3-stop-points.md`, on AC with the dock, from an elevated (High) agent shell,
by the four `real_stop_points_*` tests in `crates/wakeshot-windows/src/power/stop_points.rs` (final run 21:46:48 to
21:47:46, `cargo test --locked -p wakeshot-windows real_stop_points -- --ignored --test-threads=1 --nocapture`).

- **The real parts keep every held value journaled at every stop.** The core's apply, restore and ownership check ran
  through `apply_locked`, `restore_locked` and `check_locked` over `CopyAsActive` on a "Wakeshot test <pid>" copy of
  Balanced and `FileNet::tray` in a temp folder, with RunOnce under a `ScratchKey` and a `RecordingGuard`. A wrapper
  stopped each run at call k (a panic before the call), for every k of 10 starts, then the next start's restore ran
  with a fresh `FileNet` and guard; then the same stop with that restore failing at each of its calls in turn, and a
  retry. 3142 cases, all passed: no key at the target, stored or in force on the copy, was without a known entry
  holding the user's value; RunOnce and the guard stayed while a known entry did; after the next restore every key
  on the copy had the user's value (lid 1 and 1, battery sleep 14400) stored and in force, and sleep AC and the energy
  saver were unchanged. Calls per run (N): fresh apply 19, apply over E1 alone 12, over E1, E2, E3 16, with lid AC
  released 14; restore after an apply 15, E1 alone 9, E1, E2, E3 and a deleted plan's entry 14 (all plans) and 12
  (one plan); check with one user change 5, with the last entry changed 6.
- **Where the next restore finds no journal, it does nothing, and what the stopped run left stays.** Two cases: a stop
  between the clean-up's journal delete and its RunOnce clear (walk R6: the value stays; a stop at the guard's stop
  leaves the guard), and a fresh apply stopped after `set_run_once` or `ensure_guard` but before its first journal
  save (the value and the guard stay with no journal). Both are the harmless kind the design accepts: RunOnce and the
  guard find no journal and do nothing.
- **An unknown key's entry never blocks the clean-up.** With one in the file (on the copy, key `sleepAc`), the
  clean-up saves the file with only that entry instead of deleting it, so the next restore always finds a file and
  releases RunOnce and the guard: no R6 window. The entry, read back and parsed, had the same five fields after every
  stop, failure and restore.
- **The walk catches an order break in `FileNet`.** In a scratch copy, `FileNet::delete_journal` and
  `FileNet::clear_run_once` swapped by name (RunOnce cleared before the journal delete): all four tests failed. With
  only the stop walk on, the first failure was "restore all plans after an apply, stop at call 13 (clear_run_once)":
  a known entry without RunOnce. The unknown-key walk lost the file with the unknown entry.
- **Time.** 57 s for the 3142 cases, debug build, this laptop: about 18 ms a case, each with a dozen plan writes to the
  copy, two or three write-through journal saves and RunOnce writes under the test key.
- **Model, not verified.** A user's change in Settings is modelled as the value stored and the whole plan in force at
  once (as `CopyAsActive` notes say a test must model it). That Settings applies every stored value of the plan at
  once is inferred, not tested.

## Cron checks

Checked 2026-11-03 between 21:20 and 21:55 for card `docs/tasks/p4-cron.md`. Reads only: Claude Code's binary and
Windows zone data; no setting, zone or clock changed.

- **Claude Code's own cron code.** Read in `C:\Users\haika\.local\bin\claude.exe` (2.1.317, 338,187,936 bytes) by a
  byte search for `recurringFrac` (bundled code near offset 292,466,000): field parser `F`, parser `d5`, search
  `FYt`, next fire `Zkn` (recurring) and `W7o` (one-shot); `wRt` calls `Zkn` from `lastFiredAt ?? createdAt` or
  `W7o` from `createdAt`. Built-in constants `jJ`: `recurringFrac` 0.5, `recurringCapMs` 1,800,000, `oneShotMaxMs`
  90,000, `oneShotFloorMs` 0, `oneShotMinuteMod` 30, `recurringMaxAgeMs` 604,800,000, `cacheLeadMs` 15,000.
  **Verified** (code read).
- **Parser.** Five fields after a JS `trim` and a split on JS white space; each field a comma list matching
  `^\*(?:\/(\d+))?$`, `^(\d+)-(\d+)(?:\/(\d+))?$` or `^\d+$`. Step 0 and a range with a > b are errors (no wrap);
  day of week takes 7, as 0, in single values and ranges; a step on a range (`1-30/5`, open in "Sources track plan
  checks") works. For the both-day-fields rule a field counts as restricted when its value set is not full, so `1-31`
  counts as `*` and `*/2` as restricted; vixie cron looks at a literal `*` instead. **Verified** (code, and the
  differential check below).
- **Search (`FYt`).** Starts at the next UTC minute after `from`; walks with JS `Date` setters on local time (next
  month, next day, next hour, next minute); at most 527,040 steps (366 days of minutes), no time bound. Recurring jobs
  skip the second pass of a repeated local time (its test: `setMinutes(getMinutes())` moves the time); one-shots take
  it and walk a matching day minute by minute. A local time skipped by a change never matches. **Verified** (code,
  differential check).
- **Next fire.** Recurring (`Zkn`): `i` = the next run after `from`, `n` = the gap from `i` to the run after it. When
  the raw text matches `^\*\/\d+ \* \* \* \*$` and `15,000 < n`, `n >= 300,000`, `n - 15,000 < 300,000` (only a 5
  minute gap), the fire is `from + n - 15,000`: 285 s after the last fire or the creation, wherever that falls on the
  clock. So "`*/5` runs 15 s early" holds only when `from` is on a 5 minute mark. Otherwise the fire is `i + min(frac
  * 0.5 * n, 30 min)`, with `frac = parseInt(id.slice(0, 8), 16) / 2^32` (0 when NaN). One-shot (`W7o`): the run, or
  when its local minute is divisible by 30, `max(run - frac * 90,000, createdAt)`. The tick fires when `Date.now()`
  (whole ms) reaches the float time, and after a fire computes the next one from that tick's `Date.now()`. **Verified**
  (code; the live fixtures 03, 06, 09, 10, 11 and Phase 0 tests 3, 5, 14 match to the ms: tests in `sources/cron.rs`).
- **The constants can change without a release.** The scheduler takes them from `getJitterConfig`, which reads the
  remote flag `tengu_kairos_cron_config` (`AY(name, jJ, 60000)`) and keeps `jJ` only when the flag value fails its
  schema: `recurringFrac` 0 to 1; `recurringCapMs`, `oneShotMaxMs`, `oneShotFloorMs` 0 to 30 min; `oneShotMinuteMod` 1
  to 60; `recurringMaxAgeMs` 0 to 30 days; `cacheLeadMs` 0 to 60 s. So a recurring job is never more than 30 minutes
  late, whatever the flag holds. What the flag holds today: not verified (the live fires match `jJ`). **Verified**
  (code).
- **After a resume (inference).** The live `*/5` jobs fired first at resume start plus 285 s, and the daily and
  one-shot jobs kept their times. That fits a restored job whose `from` is the resume time. The restore code was not
  read: not verified.
- **Differential check.** Those functions copied byte for byte out of `claude.exe` into a scratch `claude.js`, run in
  Node 26.13 with `TZ` set to 10 zones (Los Angeles, Sydney, Lord Howe, Santiago, Yerevan, London, Havana, Kathmandu,
  Casablanca, Sao Paulo), against `sources/cron.rs` built into a scratch program with the same offsets (each zone's
  changes to the minute, taken from Node). Per zone 20,000 random parse texts and 20,000 random searches (half within
  2 days of a change; each checks the recurring next run, the recurring fire and the one-shot fire): 0 mismatches in
  400,000. Claude Code runs on Bun (JavaScriptCore), not Node (V8); that both engines resolve skipped and repeated
  local times the same way (ECMAScript `UTC(t)`): not verified. The scratch files were deleted.
- **Windows zone data.** `GetTimeZoneInformationForYear(2026)` for "Pacific Standard Time" (found with
  `EnumDynamicTimeZoneInformation`): the rules give 2026-03-08 02:00 and 2026-11-01 02:00 local, so 10:00 UTC and
  09:00 UTC. `SystemTimeToTzSpecificLocalTimeEx` gives -480 one ms before and -420 at the spring change, and -420 then
  -480 at the fall change; 2006 still uses the old rule (first Sunday of April). **Verified** (tests
  `localtime::tests`, 2026-11-03). This PC's zone: "Caucasus Standard Time", +240, no daylight saving.
- **Cost of the zone calls.** PowerShell P/Invoke loops of 20,000 calls: `GetDynamicTimeZoneInformation` 0.3 us;
  `SystemTimeToTzSpecificLocalTimeEx` 0.24 to 0.86 us within one year, about 23 us when each call names another year
  (Windows seems to keep one year's rules: inferred). A daily job's next fire through `SystemZone::current()` took 3.8
  ms in a debug build. **Verified** (measured 2026-11-03).
- **Not verified.** What `GetDynamicTimeZoneInformation` returns with "Adjust for daylight saving time automatically"
  off (expected: `TIME_ZONE_ID_UNKNOWN` and no change dates); whether a running Claude Code follows a PC zone change;
  where Windows zone data and the IANA data Bun uses differ for a zone.

## Velopack checks for Wakeshot

Checked 2026-11-03 between 21:50 and 22:15 for card `docs/tasks/p6-velopack-facts.md`, on this laptop. A throwaway
crate outside the repo (`velopack = "=1.2.161"`, Rust 1.99.1, x64, static CRT, the repo's release profile with
`panic = "abort"`) with two exes named like Wakeshot's: GUI `wakeshot-tray.exe` (`VelopackApp::build()`, the four fast
callbacks, `on_first_run`, `on_restarted`) and console `wakeshot.exe` (no `VelopackApp`). Every start and hook logged
its args, `VELOPACK_*` variables, working folder, exe path and token elevation to a file outside the install folder,
and the crate's own `log` lines went to the same file. Packed with `vpk` 1.2.161: `vpk pack -u HaikAsatryan.Wakeshot
-v <v> -r win-x64 --channel win-x64 -p <dir> -e wakeshot-tray.exe --packTitle Wakeshot --packAuthors "Haik Asatryan"
--shortcuts StartMenuRoot -o <out>`, versions 0.1.0 to 0.6.0 into one feed folder. No power settings touched.
Everything uninstalled; the AFTER check is in the card's Log. "Test" means seen in the logs on this laptop; "source"
means read in velopack 1.2.161 (the crate in the cargo registry, or tag `1.2.161` on GitHub through `gh api`). All
runs were elevated (High), as agents here are; see "Elevation" at the end.

- **1. The dotted pack id works.** With `HaikAsatryan.Wakeshot`, pack, silent install, four updates, Setup over an
  install (three times) and silent uninstall all worked, exit 0. Install folder `%LocalAppData%\HaikAsatryan.Wakeshot`.
  Uninstall key `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\HaikAsatryan.Wakeshot`, so winget's
  `ProductCode` is `HaikAsatryan.Wakeshot`; `DisplayName` `Wakeshot`, `Publisher` `Haik Asatryan`, `DisplayVersion`
  follows each update and each Setup run. File names: `HaikAsatryan.Wakeshot-win-x64-Setup.exe` and
  `-win-x64-Portable.zip` (no version in them), `HaikAsatryan.Wakeshot-<v>-win-x64-full.nupkg` and `-delta.nupkg`,
  `releases.win-x64.json`, `assets.win-x64.json`, `RELEASES-win-x64`. One quirk of the dot: Setup over an install
  renames the old folder for rollback to `%LocalAppData%\HaikAsatryan.<16 random letters>` (the part after the dot is
  replaced, like a file extension) and deletes it at the end, so a Setup killed midway would leave such a folder (not
  tested). **Verified** (test; key read with `Get-ItemProperty`).
- **2. Root stub and shortcut.** Layout after install: `Update.exe`, `Wakeshot.exe` (the stub, named after
  `--packTitle`), `current\wakeshot-tray.exe`, `current\wakeshot.exe`, `current\sq.version`, `packages\*.nupkg`. The
  stub runs `Update.exe start -- <its own args>`, which starts the main exe `current\wakeshot-tray.exe` with the same
  args and working folder `current`; the stub exits 0 at once. Root `Wakeshot.exe` and `current\wakeshot.exe` do not
  clash: different folders, no vpk warning, and every install, update and Setup run kept both. `--shortcuts
  StartMenuRoot` made only `%AppData%\Microsoft\Windows\Start Menu\Programs\Wakeshot.lnk`, no desktop shortcut. Its
  target is `current\wakeshot-tray.exe` (not the stub), working folder `current`, AUMID
  `velopack.HaikAsatryan.Wakeshot`. Updates rewrite it in place; uninstall removes it. **Verified** (test; `.lnk` read
  with `WScript.Shell`).
- **3. `current` keeps its path.** Before and after each update (local `FileSource`, delta and full):
  `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current`. The folder itself is new each time (Velopack backs up
  the old one and moves a new one in; its creation time changes). So the Run value and the Path entry can point at
  `current\wakeshot-tray.exe` and `current`. **Verified** (test; update log "Replacing current dir").
- **4. `wait_exit_then_apply_updates(asset, true, true, [])`.** It starts `Update.exe apply --package <nupkg> --waitPid
  <own pid> --silent --root <root> --packageDir <packages>` and returns `Ok(())` in about 0.1 s.
  - App exits 5 s after the call: Update.exe goes on at once (the obsolete hook started 30 ms after the exit). Order:
    `--veloapp-obsolete <old>` run by the old exe; kill of any process left in the install folder; swap of `current`;
    uninstall key rewritten; `--veloapp-updated <new>` run by the new exe; shortcut updated; then the new
    `wakeshot-tray.exe` starts with no args, working folder `current`, env `VELOPACK_RESTART=true`, and
    `on_restarted(<new version>)` fires. `on_first_run` fired in no run (all installs were silent). Hooks run with
    working folder `current` and get no `VELOPACK_*` variable.
  - App does not exit for 75 s: Update.exe waits 60 s, then does **not** give up. It runs the obsolete hook while the
    old app still runs, kills the app (`TerminateProcess`, exit code 1), applies, runs the updated hook and starts the
    new version as above. The doc comment "the updater will only wait for 60 seconds before giving up" means it stops
    waiting, not that it drops the update.
  - Each wait logs `WARN Failed to wait for process (<pid>) to exit (Access is denied...). Continuing...`, also on a
    clean exit. Harmless: `wait_for_pid_to_exit` opens the pid with `PROCESS_SYNCHRONIZE` only, then calls
    `GetExitCodeProcess`, which needs a query right (source `process_win.rs`). The wait itself works.
  - Restart args, when given, reach the new app after `--` (seen in the auto apply below).
  **Verified** (test, 21:57 and 22:01 to 22:02).
- **5. `UpdateManager::new` when not installed.** From the build folder it returns
  `Err(Error::NotInstalled("Could not auto-locate app manifest"))`, shown as "This application is not properly
  installed: Could not auto-locate app manifest". `VelopackApp::run()` logs it as an error and returns normally. The
  locator also gives `NotInstalled` when `Update.exe` or the manifest is missing (source `locator.rs`), so the tray can
  treat `Error::NotInstalled(_)` as "no install" and every other error as a real failure. **Verified** (test and
  source).
- **6. Hooks that fail.**
  - A panic (with `panic = "abort"`): the hook exits with 0xC0000409 (3221226505) after about 3 s, because Windows
    Error Reporting takes the crash: Application log events 1000 and 1001, and a report folder
    `C:\ProgramData\Microsoft\Windows\WER\ReportArchive\AppCrash_wakeshot-tray...`. Velopack logs `WARN Hook exited with
    non-zero exit code: 3221226505` and goes on: the update and the install both finished. Seen for obsolete (Update.exe
    apply) and install (Setup over an install).
  - Past its limit: Velopack logs `ERROR Process timed out after <n>s and was killed.` and goes on. Measured: install
    30 s (Setup), updated 15 s (apply), uninstall **60 s** (the docs say 30 s; the source and the test say 60 s).
    Obsolete waits 15 s (log "Waiting 15000ms"); not run past it.
  - stdout and stderr go nowhere: hooks start through `CreateProcessW` with no inherited handles, no std handles and
    `CREATE_NO_WINDOW` (source `process_win.rs` `run_process`; the logs show the same "About to launch ... flags"
    lines). The lines the hooks printed, and the panic message, appear in no log. A hook must write its own log.
  - The limit counts from process start. A hook run by a just-extracted exe took 0.9 to 1.7 s in all, one already run
    17 to 98 ms; the extra second is likely the antivirus scan of a new file (not verified). The updated hook always
    runs a just-extracted exe, and so does the install hook. A 25 s inner limit in a 30 s hook leaves about 3 s; a
    12 s inner limit in the 15 s updated hook leaves about 1.3 s.
  **Verified** (test).
- **7. `Setup.exe --silent` over a running install** (what `winget upgrade` runs), with the dotted id: the running
  tray was killed (exit code 1), no obsolete or uninstall hook ran, the old folder was renamed and deleted (marker
  files in the root and in `current` were gone), only `--veloapp-install 0.6.0` ran, the key was rewritten, and the
  app was **not** started. Exit 0 in 2.4 s. **Verified** (test).
- **8. `Update.exe apply` alone.** `Update.exe apply --silent --norestart`, with no `--package` and no `--waitPid`,
  found the newest full package in `packages\` (staged by the app with `check_for_updates` and `download_updates`
  only), ran the obsolete and updated hooks, swapped `current`, rewrote the key and started nothing; exit 0. So
  `p6-install-e2e` can stage with the app and apply with Update.exe, no running tray needed. `Update.exe apply -h`:
  `--norestart`, `--waitPid <PID>`, `-p, --package <FILE>`, `--verbose`, `-s, --silent`, `--rootDir`,
  `--packageDir`, `-l, --log`, `[-- EXE_ARGS]`. **Verified** (test).
- **Auto apply at startup.** With the default `set_auto_apply_on_startup(true)`, when `packages\` holds a full package
  newer than the running version and the start is not a Velopack restart, `VelopackApp::run()` starts `Update.exe
  apply` (not silent, `--waitPid` itself, its own args after `--`) and calls `exit(0)`, so `main` never gets past
  `run()`. Seen: a plain start of 0.4.0 with 0.5.0 staged ran both hooks, applied 0.5.0 and restarted it with
  `VELOPACK_RESTART=true` and the original arg. That apply kills every process from the install folder, like any
  apply. **Verified** (test and source `app.rs`).
- **Package housekeeping.** `download_updates` deletes every other full package in `packages\` ("Deleting up old
  package"); a delta is turned into a full package by a separate `Update.exe patch` run. **Verified** (test).
- **9. ARM64 packing on this laptop.** The repo's own release build for `aarch64-pc-windows-msvc` (no `velopack` in it
  yet) built in 46 s, and `vpk pack -r win-arm64 --channel win-arm64` packed it with no warning. PE machine field (read
  from the header): `HaikAsatryan.Wakeshot-win-arm64-Setup.exe` 0xAA64 (ARM64); in the nupkg, `lib/app/Squirrel.exe`
  (the package's Update.exe, 4.7 MB) and `lib/app/Wakeshot_ExecutionStub.exe` 0xAA64; in the portable zip,
  `Update.exe` and `Wakeshot.exe` 0xAA64. The x64 package's are 0x8664. So vpk ships native ARM64 Setup and Update.
  ARM64 Setup 8.6 MB, x64 Setup 10.6 MB. Not installed (x64 PC). **Verified** (test).
- **10. A main exe that never calls `VelopackApp`.** `vpk pack` says nothing: no warning, no error, also with
  `--verbose` (the repo's skeleton tray for ARM64, and the trial's `wakeshot.exe` as main exe for x64). The check
  (`CompatUtil.VerifyVelopackApp`, skipped by `--skipVeloAppCheck`) loads the main exe as a .NET assembly and returns at
  once when it is not one. So vpk never catches a Rust tray that forgot `VelopackApp`; only a smoke test that looks
  for the install hook's log line does. **Verified** (test, and vpk source
  `src/vpk/Velopack.Packaging.Windows/CompatUtil.cs` at tag `1.2.161`).
- **11. vpk upload, download and pack options.**
  - `vpk upload github -H`: `-o, --outputDir` (default `Releases`), `-c, --channel` (default `win`), `--repoUrl`,
    `--token` (both required), `--timeout` (minutes, default 30), `--publish` (else the release stays a draft), `--pre`
    (pre-release), `--merge`, `--releaseName`, `--tag`, `--targetCommitish`. `vpk download github -H`: `-o`, `-c`,
    `--repoUrl`, `--token` (optional), `--timeout`, `--pre`, each also from an env var (`VPK_REPO_URL`, `VPK_TOKEN`,
    ...). **Verified** (this laptop).
  - Upload logic (source `src/vpk/Velopack.Deployment/_GitRelease.cs`, tag `1.2.161`): the tag defaults to the newest
    full package's version (`0.1.0`, no `v`), so pass `--tag v<version>` to match the `v*` tag; the name defaults to
    the tag. Without `--merge`, a release with that tag or name already there is an error. With `--merge`, the upload
    adds its files to that release; the same channel twice still fails (`releases.<channel>.json` is already there).
    `--pre` counts only when the release is created; `--publish` turns a draft into a published release. It uploads
    the files the last pack of that channel listed in `assets.<channel>.json` (Setup, portable zip, full and delta
    nupkg; inferred from `BuildAssets.Read` and the local file) plus a new `releases.<channel>.json`; `RELEASES` only
    for the default channel `win`. So both channels go into one release with: upload `win-x64 --tag v<version>`
    (draft), then `win-arm64 --tag v<version> --merge --publish`. **Verified** (source).
  - Feed side (crate `sources/github.rs`, `sources/mod.rs`): `GithubSource` lists the 10 newest releases in one API
    call, drops pre-releases unless asked, and joins the `releases.<channel>.json` of each release that has one,
    skipping the rest; each file download lists the releases again. So a release with only one channel does not break
    the other channel. Drafts are listed only to users with push access (GitHub REST docs, "List releases"), so an app
    never sees a draft. **Verified** (source and docs).
  - `vpk pack`: `--icon` sets the icon of Update.exe and Setup.exe and adds `setup.ico` to the package (source
    `WindowsPackCommandRunner.cs`). `--releaseNotes <file>` (markdown) goes into the package manifest, comes back as
    `VelopackAsset.NotesMarkdown` and `NotesHtml` (crate `manager.rs`), and becomes the GitHub release body
    (`_GitRelease.cs`). `--delta None|BestSpeed|BestSize`, default `BestSpeed` (docs.velopack.io/packaging/deltas),
    diffs against the newest full package in `-o`, which is why CI runs `vpk download github` into the same `-o`
    first. `vpk pack` works in `%TEMP%\velopack\` and leaves that folder behind, empty. **Verified** (this laptop,
    source, docs).
- **Velopack's own logs.** Setup writes `%LocalAppData%\velopack\velopack.log`; Update.exe and the stub write
  `%LocalAppData%\velopack\velopack_HaikAsatryan.Wakeshot.log`. Uninstall removes neither. `-l <file>` on Setup.exe or
  Update.exe moves them. **Verified** (test).
- **Uninstall.** `Update.exe --uninstall --silent` while the app runs: kill, uninstall hook, shortcut, install folder,
  `%TEMP%\velopack_HaikAsatryan.Wakeshot`, uninstall key; exit 0. Update.exe cannot delete itself, so the install folder
  goes 3 s after it exits, through `cmd.exe /C choice /C Y /N /D Y /T 3 & rmdir /s /q "<folder>"`. A check right after
  the exit still finds the folder and must wait. **Verified** (test).
- **AUMID.** A normal start (not a hook: fast hooks exit before this) calls
  `SetCurrentProcessExplicitAppUserModelID("velopack.HaikAsatryan.Wakeshot")` inside `VelopackApp::run()`, the same id
  as the shortcut. A build run from `target\` is not installed, so `run()` returns before this and the process keeps
  no explicit AUMID. `vpk pack --aumid` and `VelopackApp::set_app_user_model_id` change it. **Verified** (test log line
  and source `app.rs`).
- **Elevation.** Every process here ran elevated: Setup and Update.exe start hooks and restarts with their own token,
  so all logged `elevated=yes`. A normal user install runs them all at Medium. Not tested unelevated: whether a Medium
  Update.exe can kill a tray started elevated (a kill across integrity levels is normally denied), and the working
  folder of a start from the Run value. The Access-denied warning in point 4 comes from the access mask, not from
  elevation (source).

## Dispatcher checks

Checked by `p6-int-dispatch`, 2026-11-03.

- **A call that re-enters through `&mut self` is undefined behavior in Rust.** The Rust Reference, "Behavior
  considered undefined" (doc.rust-lang.org/reference, read 2026-11-03): a live `&mut T` "must point to memory that is
  not read or written by any pointer not derived from the reference", and "When a reference (but not a `Box`!) is
  passed to a function, it is live at least as long as that function call, again except if the `&T` contains an
  `UnsafeCell<U>`." So a window that calls `LoopHandler::event(&mut self)` again from a nested pump breaks the rule
  whatever the inner call does, while `&self` methods over `Cell` and `RefCell` (both hold an `UnsafeCell`) do not.
  **Verified** (the Reference); not checked with Miri.
- **The end-of-session `Ack` comes for every session end restore.** `p6-int-io-thread` deliverable 3 sets it after
  any `Restore { why: SessionEnd }`, and the core sends one at `Session(Ending)` (`core/settings.rs`). So at
  `WM_ENDSESSION` the `Ack` is often set already; the dispatcher resets it before `session_ending()`, so its wait
  covers the end's own restore. Read from the card and the code; the IO thread is not built yet (not verified).
- **The core gives one next deadline.** Right after a Keep awake pick it is the 1 s `state.json` wait (`Mono`); the
  Keep awake end (`Wall`) shows only after the next `Tick`. Seen in `the_timer_never_asks_for_more_than_30_s`: the
  timer asked 1,000 ms. Fine, since the loop reads `next_deadline` after every call. **Verified** (test).
- **The end-of-session wait, measured** (tests on this laptop, three runs, beside the other tests): an `Ack` set by
  another thread after 200 ms ended the wait after at least 200 ms and under 2 s (the test's bounds; exact times not
  printed); with no `Ack` the call took at least 4.00 s and at most 4.02 s (the whole test run's time). **Verified**
  (test).
- **Mutants**: 22 hand-planted mutants of `app/dispatch.rs`, run in a scratch copy of the workspace, all killed after
  one test gained an assertion (the safety tick must win over a farther deadline; the 30 s clamp alone had hidden a
  timer set from the deadline only). **Verified** (test).

## Journal property test checks

Checked 2026-11-03 by `p2-ownership-props`, with `ownership::props` against `ownership::fake::FakePlans` (a model of
Windows, not Windows itself).

- **A failed apply whose restore routine fails too leaves a false user change.** A failure in Apply step 5 or 6 runs
  the restore routine on the plan. When that routine fails as well (its journal load, a plan read, its mark save or
  its drop save), an entry the apply saved in step 3 but never wrote stays in the file, unmarked, with live equal to
  `original`. The next ownership check sees "differs from `written`" and reports it as the user's change. A read-back
  that differs (the user changing the value in that moment, or anything else that resets it) counts as the first
  failure. No held value is lost. Shown by the ignored test
  `ownership::props::a_failed_apply_whose_restore_fails_reports_no_false_user_change` (the user changes `lidAc` just
  before its read-back, then the routine's journal delete fails: the check reports `lidAc` and `lidDc`, and `lidDc`
  was never written), and by the user-change property in a run with a fault and a user action in every routine,
  once its excuse for this case was taken out (shrunk to 4 operations). **Verified** (test, on the fake).
- **The restore routine overwrites a user change between its steps 3 and 4.** A change after step 3 reads an entry as
  `written` and before step 4 writes `original` (the mark save sits between) is replaced by `original`. With the
  test harness, a change at the restore's call 4 or 5 ended at `original`; at call 3 (before the read) or call 6
  (after the write) the user's value stayed. Section 5 names only the apply's window (steps 2 to 5). **Verified**
  (test, on the fake).
- **`Applied.journal` is empty when the apply's own restore routine cannot load the journal.** After a failed write
  the apply reports what that routine reports, which is empty when its load fails, while the file still holds the
  entries the apply saved in step 3 (a run's trace: `Write(..), wrote [lidAc], 0 entries`, two entries in the file).
  The same case as `p2-ownership-apply` Found outside scope 3. **Verified** (test, on the fake).
- **At 256 cases, a fault on one exact call is often missed.** With fault points spread evenly over 32 calls, the
  mutant "a takeover clears the mark" (it needs E1 alone, then a stop or a failure between Apply step 3's save and
  E1's write) survived a full run; with points biased to the first 18 calls it was caught in 2 of 3 runs, and a
  property whose every case starts that way caught it in 3 of 3. **Verified** (runs).
- **Not verified: Windows refuses to delete the active plan.** The test world deletes only an inactive plan.

## Claude transcript checks

Card `docs/tasks/p4-claude-transcript.md`, 2026-11-03. Claude Code 2.1.317's own code was read in
`C:\Users\haika\.local\bin\claude.exe` (read only, a byte search for the strings named; the short names are this
build's minified ones). The live checks use `crates/wakeshot-core/tests/fixtures/claude/live/`.

- **The carry line can hold more than the live one showed.** The builder `OZn` (with `Tdn`) writes
  `{type: "session_cron_carry", tasks, otherArmed?, wakeup?, ended?}`. `tasks` and `otherArmed` together are the
  process's armed session jobs, each `{id, cron, prompt, createdAt, recurring, plugin?}` (schema `$cr`); `tasks` holds
  the ones made or carried in the conversation's kept lines. `wakeup` is the pending dynamic-loop wakeup
  `{prompt, scheduledFor, reason?, keepalive?}` (schema `Hcr`); `ended` lists ids of jobs that ended. Jobs owned by
  a subagent (`agentId`) are left out. The live `/clear` (fixture 06) showed only `tasks: []` and `otherArmed`.
- **A resume restores jobs from the transcript (`zi`).** For each `CronCreate` call whose result is not an error, not
  durable and not deleted: a recurring job is restored with `lastFiredAt` set to the restore time (`Date.now()`),
  unless it is `recurringMaxAgeMs` (7 days) old; a one-shot only if its time (`W7o` from `createdAt`) has not passed.
  That is why the live `*/5` jobs fired at resume plus 285 s. Wakeups are re-armed (`Vi`) only for a session with a
  job folder (`SN()`: `jobDir` or `CLAUDE_JOB_DIR`, so `bg` sessions); for terminal and SDK sessions they are not,
  as seen live. The `bg` case: not verified live.
- **The restore comes about 1 s after the process start.** In fixtures 09 and 10 the first fire after the resume
  came 1104 and 1151 ms after `procStart` plus 285 s; every other live fire came 54 to 951 ms after the due time the
  source gave (printed once from test `every_live_fire_was_due_and_every_live_line_reads`). A due time counted from
  `procStart` is so about 1 s early, well inside the core's 2 minute margin.
- **The scheduler tick (`de`).** A job's next fire is cached at first sight with `wRt` (`lastFiredAt ?? createdAt`)
  and, after a recurring fire, recomputed with `Zkn(cron, now)`. A recurring job past `recurringMaxAgeMs` that is not
  `permanent` (`Ze`) is deleted after that fire.
- **`ScheduleWakeup`.** `stop: true` cancels every pending loop wakeup (`Uzs`) and returns `scheduledFor: 0,
  stopped: true, cancelledWakeups: N`. A call that arms nothing (a loop 7 days old) returns `scheduledFor: 0` without
  `stopped`. Each wakeup is a store job with `kind: "loop"`, a random id and a `cron` of the minute and hour of
  `scheduledFor` (`M`), fired by the one-shot rule, so it fires up to 90 s early on :00 and :30 and not after
  `scheduledFor` (plus the 1 s tick, or the end of a busy turn). No code was seen that drops an earlier pending
  wakeup when a new one is armed.
- **Task notifications (`Ai`).** Header fields come in the order `task-id`, `tool-use-id`, `task-type`,
  `output-file`, `status`, `summary`, one per line, then the body. Tasks a dead process left get one notification
  each, or past a count one batch notification with several `<task-id>` lines and one `<status>` (`stopped` or
  `failed`). Status values written in the code: `completed`, `failed`, `killed`, `stopped`. Claude Code's own text
  about such a task: "It may have been stopped (via the UI, Monitor timeout, or agent teardown: these leave no
  transcript marker)". That a Monitor timeout writes no notification: not verified live.
- **The live transcripts read cleanly.** All 14 give no format problem (test `no_live_transcript_line_is_a_problem`).
  The 6 `CronList` answers in them name next fires within 2 ms of the source's due times (test
  `every_live_cron_list_agrees`).
- **A carried job's next fire follows from its creation alone.** A recurring fire comes `min(frac * 0.5 * gap,
  30 min)` after its cron run, so the first fire after a time t is on the chain of fires that starts 30 minutes
  before t. Fixture 06 `.2` read alone gives 14:22:28.955 UTC; the job fired 282 ms later. A `*/5` job's chain hangs
  on its real fire times, so for it the guess is off by up to one 285 s step (not tested live).

## Harness checks

Checked 2026-11-03 22:47 to 22:52 for card `docs/tasks/p3-harness.md` on this laptop (Windows 11 Pro 10.0.26300, on
AC, lid open, session locked by Windows at screen off, agent elevated: High), with the debug build of
`crates/wakeshot-windows/examples/power_check.rs` and its own harness folder in the card's scratch folder. Haik's
values read by `powercfg` before (22:47:36, 22:49:42) and after (22:49:43): Balanced active and the only plan, lid
AC 1 and DC 1, sleep AC 10800 and DC 14400, energy saver DC 30; no `WakeshotRestore` RunOnce value; no
`%LocalAppData%\Wakeshot` and no `%LocalAppData%\Wakeshot-harness` folder.

- **Windows shows our request reason.** While `power_check hold --for 10` held its requests, `powercfg /requests`
  (elevated) listed `power_check.exe` under SYSTEM and EXECUTION with the text "Wakeshot: keeping the PC awake for
  running work"; 6 s after the run's clear both were gone, and the orchestrator's own SYSTEM request stayed.
- **First values of the notifications, in order.** `Subscriptions::start` gave, within 1 ms of the call: power AC
  with no battery reading, lid open, power AC with battery 100%, then the active plan. Two runs, the same order.
- **Ticks through `MsgWaitForMultipleObjects`.** 5 s ticks on the wall clock came 4996 to 5018 ms apart, wall and
  unbiased steps within 11 ms of each other, over 6 ticks in three runs; no gap.
- **A console window names its client as owner.** For a window started as `conhost.exe "<exe>" ...` from the
  elevated shell, `GetWindowThreadProcessId` on the `ConsoleWindowClass` window gave the harness pid, not the
  conhost pid; conhost's own windows were of class `IME` and `MSCTFIME UI`. `Process.MainWindowHandle` of that
  conhost read 0 (the session was locked; why: not verified).
- **The close of a console window reaches the harness's handler, which finishes first.** `WM_CLOSE` posted to that
  window: the run logged "stopping: the console window closed" 1.3 s after its last tick, then every line through
  "power_check hold ended" within 2 ms, and the process was gone 10 ms after the post (`CTRL_CLOSE_EVENT`, handler
  waiting on the clean exit, as in "Spike launch and stop checks").
- **Session notices register from an elevated console process.** A hidden top-level window (class registered by
  the harness, `WS_OVERLAPPED`, never shown) took `WTSRegisterSessionNotification` with no error and unregistered
  with no error after 5 s; no notice came in those 5 s. `locked_now()` read locked, `on_console()` true.
- **`Global\` mutex from High.** `NamedMutex::open("Global\Wakeshot.Probe.<pid>")` and `take(0)` worked at High
  integrity. At Medium: not tested here (`p3-real-checks`).
- **pwsh and exit codes.** `& exe bad-args 2>&1 | Select-Object -First 2` then `$LASTEXITCODE` read 0 though the
  exe exits 2; without the pipe it read 2. Read exit codes without a pipe.

## Core property test checks

Card `p2-core-props`, 2026-11-03, on this laptop, each with how it was checked.

- **proptest prints the shrunk input with `{:#?}`.** proptest 1.12.0, `src/test_runner/errors.rs` line 119:
  `write!(f, "minimal failing input: {:#?}", what)`. So a case type with its own `Debug` (one line per world step)
  prints as written, each line indented by the pretty printer.
- **Cost of one core step in the world.** The coverage tally (256 cases of the general mix, debug build): 15,759
  core steps and 25,670 log entries in 0.58 s, about 37 us per core step, IO runs included.
- **How often the general mix builds a property's chain** (tally, 256 cases, after the first rebalance): 11 changes
  that leave a closed laptop on battery, unlocked, on our Do nothing (the AC to battery lock); 70 user changes inside
  an episode; 2 restore retries while the core applies. With the first weights, sticky states (another session on
  the console, a session end, a step back) covered most steps. The AC to battery lock mutant survived the general mix
  at 256 cases and was killed in 3 of 3 runs once the lid properties also drew from a lid-weighted mix.
- **Finding 1, shown with the world.** A hold on Balanced; the user sets Balanced's battery sleep, then switches
  plan before the next read. The read posts `Plan(other)` before `UserChanged(Balanced, SleepDc)`; the core's switch
  apply carries all three keys, the release comes after, and our battery sleep Never stays in force on the other plan
  for the rest of the episode. Shrunk case in `p2-core-props`, "Found outside scope" 1.
- **Finding 2, shown with the world.** With a crashed tray's journal in `Started` (the start-up restore failed), a
  lid close before the first `Plan(Ok)` gets no lock: `Core::in_force` gives `None` with no plan read. Shrunk case in
  `p2-core-props`, "Found outside scope" 2.
- **pwsh splat of a one-flag array.** `$a = if ($all) { @("--all-targets") } else { @() }` then `& cargo clippy @a`
  failed with "unexpected argument '-' found"; written out literally, the flag worked. Why the splat broke it: not
  verified (most likely the `if` unrolled the array to a string). Pass such flags literally.

## Tray words checks

Checked 2026-11-03 for card `docs/tasks/p5-text.md`.

- **Balloon text limits and the empty body.** Microsoft Learn, `NOTIFYICONDATAW`
  (`learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw`, updated 2024-11-20, read
  2026-11-03): `szInfo` "can have a maximum of 256 characters, including the terminating null character, but should
  be restricted to 200 characters in English"; `szInfoTitle` "a maximum of 64 characters, including the terminating
  null character, but should be restricted to 48 characters in English". So `text::notice` cuts at 63 and 255 UTF-16
  units, and its tests hold the fixed words to 48 and 200. Same page, `NIF_INFO`: "To remove a balloon notification,
  specify NIF_INFO and provide an empty string through szInfo", so no notice has an empty body (`Updated` gets one).
  `NIIF_NOSOUND`: "Do not play the associated sound".
- **The `&` prefix rule.** Microsoft Learn, `DrawText`
  (`learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-drawtext`, updated 2024-02-22, read 2026-11-03),
  `DT_NOPREFIX`: "Normally, DrawText interprets the mnemonic-prefix character & as a directive to underscore the
  character that follows, and the mnemonic-prefix characters && as a directive to print a single &." So
  `text::status_line`, a menu `Label`, doubles `&` in hold reasons. Whether `p5-menu-paint` draws with this rule is
  that card's; the `MenuItem` doc in `view.rs` asks for it.
- **Two-line tooltip.** Not checked again here: the Phase 0 tray spike showed both lines of
  `"Wakeshot spike\nHolding awake: Claude Code, 2 sessions"` in the native tooltip (this file, "Tooltip";
  `img/tray-tooltip-dark.png`). `text::tooltip` uses the same `\n` form.
- **"Other system tray icons".** The first-run notice names this section of Settings > Personalization > Taskbar, the
  page `ms-settings:taskbar` opens. The label on Windows 11 build 26300: not verified (no screen use in this card);
  `p5-haik-check` and `p6-int-haik-checks` look at it.

## Tray startup checks

Checked 2026-11-03 for card `docs/tasks/p6-int-startup.md`, with `app::startup::tests` (22 tests, all pass).

- **The panic hook's exit code.** A child run of the test binary that installs the hook and panics ends with
  0xC0000409 (`STATUS_STACK_BUFFER_OVERRUN`, the fast fail of `std::process::abort()`), not 101 (a test that panics
  with the default hook), and its log holds one `ERROR panic:` line with the place and the message cut to 200
  characters (`panic_hook_logs_then_aborts`). The whole test run took 0.18 s, so this console child had no 3 s WER
  wait. After the run, no new entry in `ReportArchive` or `ReportQueue` (machine and user WER folders) and no
  Application event 1000 or 1001 since the run (`Get-ChildItem`, `Get-WinEvent`). Why the Velopack hook's panic
  above went through WER and this child did not: not verified (a GUI exe started by Velopack against a console test
  exe started by the test runner).
- **Mutex tests need a second thread.** The thread that holds a mutex takes it again at once (`store::NamedMutex`
  doc), so the second tray and the second settings owner in the tests are other threads; then `take(0)` gives `None`
  while the first holds it, and `Some` after it is dropped. A thread that ends while holding the tray mutex leaves
  it abandoned, and the next take reports `abandoned()` (`a_tray_lock_left_by_an_ended_tray_counts_as_taken`).
- **A failed journal read, for the tests.** A folder named `journal.json` makes `store::read_whole` fail, so
  `FileNet::load_journal` gives `JournalRead::Failed` (code 5 in `p3-journal`'s test; the code is not checked here).
  The startup retry tests swap the folder for the file between the two tries.
- **An unknown lock state at start.** Read in the code, not tested: `core/lid.rs` `lock_and_chime` watches a lock
  only when the core thinks the session is unlocked. So `None` from `session::locked_now()` counted as unlocked makes
  a lid close on a session that was locked after all look like a failed lock (lock once more, then the step back
  toward sleep); counted as locked, a lock that really failed would go unseen.

## Loop window checks

Checked 2026-11-04 for card `docs/tasks/p6-int-window.md`, by the tests in `app::window::tests` (Windows 11 Pro
10.0.26300, debug test build, elevated agent shell with a desktop session; every test window hidden, of a test class
`Wakeshot.Loop.Test*`). Ten runs in a row, all passed.

- **The posted-message limit.** A loop window whose thread did not pump took 9,999 `PostMessageW` calls; the
  10,000th failed with 1816 (`ERROR_NOT_ENOUGH_QUOTA`), the same count in all ten runs. Microsoft documents 10,000
  per queue; why one fewer here (one other message in the queue, or the limit counted as "fewer than"): not verified.
- **`FindWindowW` when nothing matches.** It returned NULL and left the thread's last error at 0, for a class never
  registered and for a registered class with no window left. It found the hidden (never shown) top-level window by
  class name.
- **`PostMessageW` to a destroyed window** fails with 1400 (`ERROR_INVALID_WINDOW_HANDLE`).
- **`SetTimer` accuracy** (ten runs, under the load of other builds): asked 150 ms, fired after 138 to 166 ms; 300 ms
  after 300 to 316 ms; 80 ms after 93 to 97 ms; a timer re-armed to 200 ms by `SetTimer` with the same id fired 190 to
  204 ms after the re-arm. So a fire can come up to about 12 ms early.
- **A sent `WM_ENDSESSION` waits for the window procedure.** `SendMessageTimeoutW` from another thread, with a handler
  that slept 300 ms, came back after 300 to 302 ms, and never before the handler returned. `WM_QUERYENDSESSION`
  answered 1. Whether Windows itself sends these to this hidden window at a real sign-out: not verified here (an
  end-to-end check).
- **A `WM_QUIT` from `PostThreadMessageW`** ends `GetMessageW` on the loop thread as the window's own would, so the
  window treats one it did not post as Quit from the menu.

## Power review fix checks

Card `p2-core-lid`, section "Power review fixes (2026-11-03)". Pure core code; nothing here touched the laptop.

- **Tests written first fail first.** 15 new scenarios against the unchanged core: 12 failed, 3 passed by design (a
  failed lock with nothing to retry, the finding 4 pin, and finding 7's second path, which the lid rule already
  covered). After the fix all pass; one more scenario came from a surviving mutant.
- **The strict property forms hold now.** `a_released_key_is_never_ours_in_force_again_in_its_episode` and
  `a_closed_laptop_is_locked_also_before_the_first_plan_read`, ignored by `p2-core-props`, pass with the default case
  count and in three release runs of `PROPTEST_CASES=10000`, the last two on the final code (2026-11-04, about 16 to
  19 s each for the test run).
- **Mutants by hand, 21 of 21 killed** (scratch copy of the workspace, own build folder, each reverted): the four of
  the card and 17 for the lock watch, the lock step back, the floor rule, the guess before the first plan read, the
  release that asks for a new apply, and the restore after a failed write. One, "the core applies during the lock
  step back", survived the first run and is killed by a test added for it.
- **The 5 s lock check rests on one reading, not verified beyond it:** in the lid spike the session read locked 1 s
  after `LockWorkStation` ("Lock" above). How long `WTS_SESSION_LOCK` takes on a slow or busy PC: not verified.

## Folder watch checks

Checked 2026-11-04 for card `docs/tasks/p4-watch-folders.md`, by the tests in `watch::dir::tests` (Windows 11 Pro
10.0.26300, NTFS, folders in `%TEMP%`; test runner elevated, High) unless a line names another source. No check here
depends on elevation (the Medium case is in "Reading elevated Claude Code sessions").

- **Microsoft docs, read 2026-11-04** (`ReadDirectoryChangesW`, "Synchronization and Overlapped Input and Output"):
  the system keeps a buffer per directory handle from the first call, so changes between two calls come with the
  next one; an overflow gives a zero-byte result or `ERROR_NOTIFY_ENUM_DIR`, and the caller should list the folder;
  the buffer must be DWORD aligned; over the network a buffer above 64 KB fails with `ERROR_INVALID_PARAMETER`; the
  folder's own changes are not reported; a name can come back in its short (8.3) form when the writer opened the file
  by it. Each `OVERLAPPED` of several reads waited on together needs its own manual-reset event; with an auto-reset
  one, a waiting `GetOverlappedResult` can block for good.
- **Latency.** A create, an append, a delete and a rename into place came back in 0.3 to 2.6 ms (`latency` lines of
  the test output, one run, several tests at once); a file one level down in a subtree watch in 0.7 to 1 ms.
- **Names.** A rename into place gives the temp name and the final name. A subtree watch gives `sess-1` (the new
  folder) and `sess-1\events.jsonl`; a one-level watch never gave the deeper name. An Armenian, a Russian and an
  unpaired-surrogate name came back exactly (`names_come_back_as_given`).
- **Overflow.** 5,000 new files (written in about 0.8 s) against a 4 KB buffer: the first read gave 1 name, the next
  an overflow, so `Rescan`; the watch went on after it (`a_burst_that_overflows_the_buffer_gives_rescan`).
- **The watched folder deleted.** After `DeleteFileW` of its file and `RemoveDirectoryW` of the folder, both `Ok`
  while the watch held it, the queued read ended with `ERROR_ACCESS_DENIED` (`0x80070005`). The same name could be
  made again at once, even before the watch closed its handle, and `RemoveDirectoryW` of the parent right after the
  watched folder worked too: on this build a deleted folder leaves its name at once (POSIX-style delete; which Windows
  build made that the default for `RemoveDirectoryW`: not verified).
- **Renames.** With a subtree watch open, a file rename and delete, a subfolder rename and delete, and a rename of the
  watched folder itself and back all worked. A rename of the folder above a watched one failed with 5 (Access
  denied). A probe with an ordinary open file instead (`.NET File.Open` with read, write and delete sharing, two
  levels down, pwsh) failed the same way and worked once the file was closed: Windows blocks renaming a folder that
  holds any open handle below it, whatever the sharing. Deleting that folder with `remove_dir_all` worked.
- **A read belongs to its thread.** A read queued by a thread that then ended completed with
  `ERROR_OPERATION_ABORTED` (the watch reports `Rescan` and queues a new one;
  `a_read_cancelled_by_its_thread_ending_gives_rescan_and_goes_on`).
- **A path that is a file.** `CreateFileW` with `FILE_FLAG_BACKUP_SEMANTICS` opens it; `ReadDirectoryChangesW` then
  fails with `ERROR_INVALID_PARAMETER` (`0x80070057`).
- **No handle leak.** 1,000 cycles of add and remove (each with a missing folder too; every 10th with a completed
  read nobody took, every 20th with one taken and queued again), in a child test process of its own: 122 handles
  before and after (`GetProcessHandleCount`).

## Tray UX: theme reads

Card `p5-theme`, 2026-11-04, on this laptop (Windows 11 build 26300). Reads only; no setting was switched.

- **This PC now.** `tray::theme::read()` gave `Theme { light: false, contrast: None }` (test `reads_this_pc`,
  00:21). PowerShell `Get-ItemProperty` of `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` agrees:
  `SystemUsesLightTheme` 0, `AppsUseLightTheme` 0.
- **The mode Windows 11 installs with is light.** `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes` holds
  `InstallTheme` = `C:\Windows\resources\Themes\aero.theme`, `InstallThemeLight` = the same file and
  `InstallThemeDark` = `dark.theme`. `aero.theme` has `SystemMode=Light` and `AppMode=Light`; `dark.theme` has both
  `Dark` (registry and file reads, 2026-11-04). So Wakeshot takes light when `SystemUsesLightTheme` or its key is
  missing (`ERROR_FILE_NOT_FOUND` or `ERROR_PATH_NOT_FOUND`); any other read failure, a value of another type
  included, is an error.
- **What Explorer does with a missing value: not verified.** No Microsoft page says (web search 2026-11-04). A Wine
  bug (55771, `test_ShouldSystemUseDarkMode`) reports the value present on Windows 11 21H2 and absent on older
  Windows 10 builds; a Stack Overflow answer ("detect dark windows taskbar") treats a missing value as dark, the
  pre-1903 Windows 10 taskbar. Neither checked here: deleting the value would change Haik's settings.
- **`HKU\.DEFAULT`** (the LocalSystem profile, sign-in screen) has no `Personalize` values on this PC.
- **High contrast colours** come from `GetSysColor` only while `SPI_GETHIGHCONTRAST` has `HCF_HIGHCONTRASTON`:
  `COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_HIGHLIGHT`, `COLOR_HIGHLIGHTTEXT`, `COLOR_GRAYTEXT`, `COLOR_BTNFACE`,
  `COLOR_BTNTEXT`, `COLOR_HOTLIGHT`. Outside high contrast they are the classic theme's colours and are not used. A
  real high contrast switch was not tested (Haik's theme stays; track note 3.10).

## CLI command checks

Checked 2026-11-04 for card `docs/tasks/p4-cli-commands.md` (Windows 11 Pro 10.0.26300, elevated agent shell).

- **`ShellExecuteExW` needs COM and `SEE_MASK_NOASYNC` here** (Microsoft docs, read 2026-11-04): the ShellExecuteEx
  Remarks ask for `CoInitializeEx(NULL, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)` first, since shell
  extensions may need a single-threaded apartment. The SHELLEXECUTEINFO page says `SEE_MASK_NOASYNC` "must be
  specified if the thread calling ShellExecuteEx does not have a message loop or if the thread or process will
  terminate soon after", and that it is "only respected when launching files, does not apply to uris".
  `SEE_MASK_FLAG_NO_UI` drops error dialogs; security prompts still show. On failure `GetLastError` holds the Win32
  code (2, 3, 1155 no association, 1223 cancelled, among others).
- **`AllowSetForegroundWindow(ASFW_ANY)` fails unless this process may set the foreground itself** (Microsoft docs):
  for example right after the user clicked its menu. From a terminal it may fail; the open still works, the app may
  then open behind.
- **The real open** (`shell::tests::real_open_file`, run once by hand): a temp `.cmd` opened through
  `open_with_default_app` returned after 119 ms, and the script had run (its mark file was there) within 622 ms. Its
  window closed by itself: Windows Terminal is the default terminal on this PC, and its `closeOnExit` default
  `"automatic"` closes a handed-off process's tab whatever its exit code (Windows Terminal docs, "Profile termination
  behavior"). Windows started `smartscreen.exe` for the launch; it is Windows' own and was left to end on its own.
- **A missing file opens nothing.** A path in a folder that does not exist, with an extension no app has, gave a
  `ShellExecuteExW` error with `SEE_MASK_FLAG_NO_UI` (the test takes 2 or 3; which of the two: not recorded), and no
  new process or window (`a_missing_file_is_an_error_without_a_dialog`, process list compared before and after).
- **Why the real open was not a `.log`.** On this PC `.log` and `.txt` open in Windows Notepad
  (`HKCU\...\FileExts\.log\UserChoice` ProgId `AppX4ztfk9wxr86nxmzzq47px0nh0e58b8fw`, Microsoft.WindowsNotepad
  11.2607), and Haik's Notepad was open with an unsaved tab. Windows 11 Notepad opens a file as a new tab of an open
  window by default (not verified here; not tried, so nothing touched that window). `open_log::tests::real_open_log`
  does that open, for a run with a person at the screen.
- **`wakeshot status`, `keep-awake 1h` and `automatic off` with `--dry-run`** (through each command's `main`,
  `status::tests::dry_run_folder_checks`; `args::parse` routes no command yet): "Wakeshot is not running.", exit 0;
  "Wakeshot is not running. Start it, then try again.", exit 1, no `keep-awake.json`; "Automatic is off.", exit 0,
  `config.json` `{"version":1,"automatic":false}`, then removed (it did not exist before). The log got one `cli` line
  per command. `%LocalAppData%\Wakeshot` did not exist before the run; the run made only `dry-run\logs\wakeshot.log`,
  and the folder was removed after.
- **`wakeshot restore` with `--dry-run` and no dry-run journal** (card `p4-cli-restore`, through `main`,
  `restore::tests::dry_run_folder_check`): "Nothing to restore.", exit 0; one log line `cli: restore: no journal,
  nothing to do`. The folder it made was removed after, as above.
- **`restore_locked` logs a line of its own per run** in area `journal` (`restore of every plan: found journal
  entries; 0 entries left`, or `... failed: <call> failed with <code>; ...`), beside the CLI's one `cli` line (seen in
  `MemoryLog` in the `restore::tests` runs).
- **`FileNet::restore_only` over a RunOnce key that does not exist writes nothing to the registry**: its clean-up's
  `AutoStart::remove` finds no value and creates no key. After the `restore::tests` runs, `HKCU\Software` had no
  `Wakeshot*` key (`Get-ChildItem HKCU:\Software`).

## Claude parsing review fix checks

Card `docs/tasks/p4-claude-transcript.md`, section "Claude parsing review fixes (2026-11-04)"; review
`docs/reviews/p4-claude-parsing.md`. Read in Claude Code 2.1.317's bundled code (`C:\Users\haika\.local\bin\claude.exe`,
read only, a byte search with context; minified names from that build only). None of it was run live; `p4-live-claude`
runs the stops.

- **`TaskStop`** (aliases `KillShell`, `KillBash`) takes `task_id`, else `shell_id`, and returns `{message, task_id,
  task_type, command}` (`command` optional); a failed check is a tool error. Its stop path (`ILe`) marks a shell
  `notified` and sends only an SDK stream event, so no `<task-notification>` reaches the transcript. Claude Code's own
  resume reader (`hrn`) takes a `TaskStop`/`KillShell`/`KillBash` result with string `task_id` and `task_type` as the
  stop. Verified in code.
- **Monitor timeout**: the shell Monitor arms `setTimeout(timeoutMs)` after its task starts and before it returns
  `{taskId, timeoutMs: persistent ? 0 : timeoutMs, persistent}`. At the timeout it queues one event notification (body
  `<event>...</event>`, no `<status>`) and kills the task. Verified in code. The timeout fires a few ms before the
  result line's time plus `timeoutMs`, so that sum is never early.
- **Stop from the UI** (`tUr`): queues `<status>killed</status>` with `passive: true` and priority `next`. When it is
  written to the transcript: not verified.
- **Notification header**: `Dwe` is a first line `[SYSTEM NOTIFICATION - NOT USER INPUT]`, three more lines and a blank
  line; `RJr` is a variant with the same first line, and one path wraps `Dwe` in `<system-reminder>`. Claude Code's own
  reader (`qi`, `Xi`) strips `Dwe` and splits joined notices by `joinedNoticeLengths` (UTF-16 lengths), each after a
  blank line; without the lengths it reads the rest as one notice. Verified in code; never seen in a transcript.
- **Fire line** (`Dnr`): `{type, subtype, content, isMeta, timestamp, uuid, taskId, cron, prompt}` plus `taskKind`,
  `cronKind`, `noOpStreak`, `streakStartedAt`, `foldedUuids` when set. Nothing marks a job's last fire at 7 days.
  Verified in code.
- **Latest fire under any flag value**: `ClaudeScheduler::latest_recurring_ms` (frac 1, cap 30 minutes, lead 0, and
  `from + 5 min` for a `*/5` job). The unit test `a_computed_recurring_time_covers_every_flag_value` checks five crons
  against 60 flag settings (frac 0 to 1, cap 0 to 30 minutes, lead 0 to 60 s): every real fire falls in the core's
  window of one of the two reported times.

## Tray UX: menu painting

Card `p5-menu-paint`, 2026-11-04, on this laptop (Windows 11 build 26300). Offscreen drawing only: no window, no
screen. Code: `crates/wakeshot-windows/src/tray/menu_paint.rs`.

- **WinUI template, read again** (`gh api`, microsoft-ui-xaml
  `controls/dev/CommonStyles/MenuFlyout_themeresources.xaml`, last commit 2026-05-30;
  `RadioMenuFlyoutItem_themeresources.xaml`): an item icon is a 16 x 16 epx `Viewbox`; a mark or an icon each take
  one 28 epx column (`MenuFlyoutItemPlaceholderThemeThickness`), both take 56 (`...DoublePlaceholder...`), and a menu
  that has any toggle or icon item reserves the columns for every item (inferred from the visual state names
  `CheckPlaceholder`, `IconPlaceholder`, `CheckAndIconPlaceholder`; the presenter code that picks them was not read).
  Check `E73E` and radio bullet `E915` are
  12 epx with 16 epx after them (15 in the radio sub-item template); chevron `E974` 12 epx, 24 epx after the text.
  Marks at rest and the chevron use `TextFillColorSecondary` (light `#9E000000`, dark `#C5FFFFFF`); a hot mark turns
  `TextFillColorPrimary`. High contrast: fill `Window`, text, marks and chevron `ButtonText`, hover `Highlight` with
  `HighlightText`, disabled `GrayText`, separator and border `WindowText` (the border is 2 px there). No maximum width.
- **Icons, Microsoft's rule** (learn.microsoft.com/windows/apps/design/controls/menus, updated 2026-10-14): icons for
  the most used items, standard pictures and pictures that show the command; "Don't feel obligated to provide icons
  for commands that don't have a standard visualization." The menu icon is 16 x 16 px.
- **Fonts on this PC** (WPF `GlyphTypeface` reads, 2026-11-04): Segoe UI Variable (`SegUIVar.ttf` 2.03) has Latin,
  Cyrillic (U+0410, U+0436, U+0491, U+04D9) and U+2014, but no Armenian (U+0531, U+0561, U+0570), Hebrew or Arabic;
  Segoe UI has all of those; neither has U+4E2D. Segoe Fluent Icons (`SegoeIcons.ttf` 1.54) has every glyph used here;
  Segoe MDL2 Assets has them too. In GDI, `GetGlyphIndicesW` with `GGI_MARK_NONEXISTING_GLYPHS` on Segoe UI Variable
  Text marked all 13 Armenian letters of a test label missing, and Segoe UI none (test
  `non_latin_labels_get_a_font_that_has_them`).
- **What Windows adds to an owner-drawn item's width.** Wine's `dlls/win32u/menu.c` (`calc_menu_item_size`): "Tests
  reveal that Windows ( Win95 through WinXP) adds twice the average width of a menufont character", the average being
  `(width of a..zA..Z / 26 + 1) / 2`. With the menu font from `SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS)`
  this gives 14 px at 96 dpi, 16 at 120 and 20 at 144 here (test `measure_asks_less_what_windows_adds`); Phase 1
  measured 16 px at 120 dpi on screen. 96 and 144 dpi not checked on screen (`p5-menu-view` Part B).
- **Renders match the Phase 1 pick.** `docs/research/img/ux/menu-render-<96|120|144>-<light|dark|contrast>.png`
  (from the ignored test `renders_for_review` as BMP, made PNG by a stdlib Python script; `art::png` was not there
  yet). Rows 33, 41 and 50 px, text 14, 18 and 21 px, marks and chevron 12, 15 and 18 px. The fill, hover and
  separator colours are the same pixels as `tray-look-render-96dpi-dark.png` and `-light.png` (`2c2c2c`, `383838`,
  `3d3d3d`; `f9f9f9`, `f0f0f0`, `eaeaea`), and the hover band is 27 rows at 96 dpi and 42 at 144 in both (pixel
  counts with a stdlib script). The contrast render uses this PC's system colours, as Phase 1's did (`ffffff`,
  `0078d7`, `000000`).
- **What changed from the spike, and why:**
  - Icons on Automatic (eye `E890`), Keep awake (cup `EC32`), Restore (`E777`), Open log (`E8A5`) and Quit (cross
    `E711`), Haik's ask (2026-11-03 17:32). The Keep awake lengths and Keep screen on get none (Microsoft's rule
    above); the status line gets none. With a mark and an icon column, the main menu's text starts at 71 epx instead
    of 43.
  - Marks and the chevron in the secondary text colour, as WinUI draws them; the spike drew them in the text colour.
  - The status line in the secondary text colour (about 6:1 in light, 9:1 in dark) instead of the disabled grey
    (2.5 to 2.7:1, research "Dark menus"), and never highlighted; a disabled item is never highlighted either.
  - A label with characters the text font lacks is measured and drawn with Segoe UI (then the Windows UI font of
    each script group); the spike had one font.
  - Text never runs past its column: an item asks for at most 480 epx of text, and a longer status line ends in an
    ellipsis. The status line of two hold groups fits.
  - The width asked is the width wanted less what Windows adds (above), so the menu is as wide as designed.
  - Sizes are scaled from their epx sums per edge, so rounding never adds up across columns.

## Watch thread checks

Checked 2026-11-04 for card `docs/tasks/p4-watch-thread.md`, by the tests in `watch::thread::tests` and
`watch::agents::tests` (Windows 11 Pro 10.0.26300, NTFS, folders in `%TEMP%`, debug build; test runner elevated,
High). Times are from one run with the crate's other tests running beside them.

- **Latency through the whole thread** (file call or process end to the event in the fake sink): first round 11 ms
  after start; a hold file written 12 ms, deleted 2 ms; `config.json` 3 to 7 ms; `restore-request.json` 17 ms; a
  `holds\` folder made after start, then a hold in it, 15 ms (the data folder notice reopens it at once); a file in a
  folder a source asked for later 3 ms. A process ended with `TerminateProcess`: its session and its hold posted as
  `Gone` 4 ms later, through the exit wait alone (the fake liveness had changed 300 ms before and nothing was posted
  until the process ended). A 200 ms retry fed 213 ms after the first feed. Stop took under 1 ms.
- **No handle leak.** 50 start and stop cycles (each with two folders, one of them missing, a hold file, and a live
  session with its exit wait on the test's own process), after 3 warm-up cycles, in a child test process of its own:
  122 handles before and after (`GetProcessHandleCount`).
- **A folder whose listing is denied, its files not.** A deny ACE for `FILE_LIST_DIRECTORY` (0x1) for Everyone on the
  folder only (SDDL `D:(D;;0x1;;;WD)(A;OICI;FA;;;WD)`, set with `SetFileSecurityW`, which changes the folder and not
  the files in it): `fs::read_dir` fails with error 5 even in this elevated shell, while the files in it can still be
  read, written and deleted and a new file can be made. The test asserts the denial first, so it cannot pass for the
  wrong reason under High.
- **A tool fact for mutation checks.** `Copy-Item` keeps the source's `LastWriteTime`. A file restored from a backup
  copy after a mutated build therefore looks older than the build, and cargo ran the mutated test binary again until
  the file was touched (seen 2026-11-04: the restored tests failed exactly as under the mutation). Touch the file, or
  restore it with a write, after such a check.

## Power track real checks

Checked 2026-11-04 00:32 to 00:52 for card `docs/tasks/p3-real-checks.md` on this laptop (Windows 11 Pro
10.0.26300), with the x64 debug build of `power_check` and `wakeshot-tray` (tree of 00:36), from an elevated (High)
agent shell. The laptop was on battery (93%) and undocked, lid open; the screen was off from 00:42:37 (idle timer)
until a touchpad touch at 00:48:56. The orchestrator's own SYSTEM request (a pwsh helper) was held all through. Haik's
values before (00:32:57) and after (00:52:26), by `powercfg /qh` and our read: Balanced active and the only plan, lid
AC 1 and DC 1, sleep AC 18000 and DC 18000, energy saver DC 30; no journal, no `WakeshotRestore` value. Logs and
scripts: the card's Log.

- **The track's real tests pass on the finished power code.** `cargo test --locked -p wakeshot-windows -- --ignored
  --test-threads=1 --skip real_lock_locks power:: process:: session:: recovery::`: 16 passed (plan 4, stop points 4,
  system 2, process 2 plus a helper role, recovery helper role, session 2), and `-p wakeshot-tray --test recovery --
  --ignored`: 2 passed. `real_lock_locks` was not run (it locks the screen; `p3-haik-checks`).
- **A Medium process started through Explorer creates a `Global\` mutex** (checklist 11, first part).
  `explorer.exe <file>.cmd` from the elevated shell ran the file at Medium (cmd <- `explorer.exe
  /factory,{75dff2b7-...} -Embedding` <- svchost); `power_check medium-probe` read "integrity Medium (0x2000),
  elevation type Limited" and created and took `Global\Wakeshot.Probe.<pid>`. A denied open across users
  (architecture section 4): not tested (no second account). The factory Explorer instance ended by itself within
  minutes.
- **The journal lock works across integrity levels, both ways.** A pwsh script made the same call as
  `NamedMutex::open` (`CreateMutexExW`, default security, flags 0, `SYNCHRONIZE | MUTEX_MODIFY_STATE`; take by
  `WaitForSingleObject`) on probe names in `Local\`. High holder, Medium opener: the open gave a handle with error 183,
  `take(0)` gave 258 while held and 0 after the holder's release. Medium holder, High opener: the same. A High creator
  gives the object owner BA and full access (0x1f0001) to SYSTEM, BA and the logon session SID (`S-1-5-5-0-...`); a
  Medium creator gives owner the user and full access to SYSTEM, the user and the same logon SID. Both tokens carry
  that logon SID, so each side opens the other's mutex. So an elevated `wakeshot restore` holding the lock makes the
  Medium tray wait, not fail with 5 (the case in `docs/reviews/p2-6-power-safety.md` finding 8). Not run with the Rust
  exes and not under the real name `Local\Wakeshot.Journal`; the object type, namespace, call and access are the same.
- **The real guard restores after a crash of a Medium tray.** `power_check fake-tray` at Medium in
  `%LocalAppData%\Wakeshot`: copy written (lid AC 0, lid DC 0, sleep DC 0), journal with three entries, RunOnce value,
  guard `wakeshot-tray.exe --guard ...` with the shell Explorer as parent. `taskkill /PID <tray> /F`: the guard logged
  "tray pid ... is gone" 270 ms after the kill, wrote the three originals back, deleted the journal and removed the
  RunOnce value within 10 ms more, and exited; journal and guard process gone within 300 ms of the kill (polled every
  100 ms). The copy read its original values by `powercfg /qh`. `power_check cleanup` then deleted the copy.
- **Activating a plan applies its stored values; a write to an inactive plan does not change the values in force.**
  Read with `CallNtPowerInformation` `SystemPowerPolicyAc` (0) and `SystemPowerPolicyDc` (1) (`SYSTEM_POWER_POLICY`,
  232 bytes; `LidClose.Action` at offset 28, `IdleTimeout` at 60). Balanced: lid action 2 (Sleep) on AC and DC, DC idle
  Sleep after 18000 s. A copy given lid AC 0, lid DC 0 and sleep DC 0 by `powercfg /set*valueindex` while inactive: the
  same 0.5 s later. `powercfg /setactive <copy>`: lid 0 on AC and DC, DC idle after 2147483647 s (Never). Balanced
  again: back to 2 and 18000 s. This covers both points architecture section 5 "Plan switch" lists as not tested.
  `SystemPowerPolicyCurrent` (8) read lid 3 and idle 0 for Balanced on battery, unlike the DC policy; why: not
  verified, so it is not used as a measure.
- **50 apply and restore cycles on an active copy** (checklist 15). `power_check cycles 50`, 1000 ms pauses, 108 s:
  100 `PowerSetActiveScheme` calls, min 33.2, median 51.5, max 121.8 ms; 11 cycles had one call over 100 ms. Each
  apply took effect: the DC policy in force flipped between lid 2 with 18000 s and lid 0 with Never, 103 times.
  Display brightness (`WmiMonitorBrightness`, sampled about every 220 ms, 519 samples): 100 in every sample, before,
  during and after each cycle. System and Application logs for the run: no error or warning. A flicker someone could
  see: not verified (nobody watched; the screen was off for cycles 1 to 24, on at the lock screen from cycle 25). Other
  apps' setting callbacks: no app logged an error; nothing more checked. On AC: not run.
- **Each `PowerSetActiveScheme` call writes one System log event.** `Microsoft-Windows-UserModePowerService` event 12
  (Information), "Process <exe> (process ID:<pid>) reset policy scheme from {<old>} to {<new>}": 102 in the cycles
  run, one per call. So the tray adds about two per hold (one at its apply, one at its restore; inferred from
  architecture section 5, not counted on a tray).
- **A change in Settings can be dated.** Settings writes show as the same event 12 with
  `C:\Windows\ImmersiveControlPanel\SystemSettings.exe` as the process, and the setting's key under
  `HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\<plan>\<subgroup>\<setting>` gets a new last write
  time (`RegQueryInfoKeyW`). Here: both sleep values went from 10800 and 14400 to 18000 at 00:31:08, before the card
  started.
- **Wake reason in the System log.** Kernel-Power 507 "exiting Modern Standby" carries the reason ("Input
  Touchpad"), and 566 the session reason ("InputPoUserPresent"); 506 "entering Modern Standby" said "Idle Timeout"
  for the screen-off timer. Useful for the battery runs of `p3-haik-checks`.

## CLI argument checks

Checked 2026-11-04 for card `docs/tasks/p4-cli-args.md` (Windows 11 Pro 10.0.26300, Rust 1.99.1).

- **How `std::env::args_os` splits the command line on Windows.** Read in the toolchain's own source
  (`library/std/src/sys/args/windows.rs`, `parse_lp_cmd_line`, Rust 1.99.1). The program name: a quote only toggles
  quoting (no escapes), and a space or tab outside quotes ends it. The arguments: space and tab separate, any run of
  them counts as one; backslashes are literal unless a quote follows; 2n backslashes before a quote give n and the
  quote toggles quoting, 2n+1 give n and a literal quote; inside quotes `""` is one literal quote and quoting goes
  on. The last argument is kept even when empty if a quote opened it. Microsoft's "Parsing C command-line
  arguments" (Learn, updated 2021-12-09) gives the same rules for the C runtime; its six example rows split the same
  way in `args::next_arg` (test `microsofts_examples_split_the_documented_way`). `args::run_tail` and
  `args::split_program` are a port of that loop, one argument at a time; a property test checks them against
  arguments quoted the MSVC way (std's `\"` form and the `""` form).
- **The built `wakeshot.exe`** (debug build of this card): `wakeshot --help` printed the help to stdout, exit 0;
  `wakeshot nonsense` printed `wakeshot: unknown command 'nonsense'. Run 'wakeshot --help'.` to stderr, exit 2;
  `wakeshot run` and `wakeshot keep-awake off --screen` printed their one-line errors, exit 2. The same checks are in
  `crates/wakeshot-cli/tests/exe.rs`.
- **The `--dry-run` checks of `p4-cli-commands` and `p4-cli-restore`, now through the exe** (2026-11-04 00:54;
  `%LocalAppData%\Wakeshot` did not exist before, `WAKESHOT_DATA_DIR` unset): `--dry-run log` "No log yet. Wakeshot
  writes one once it runs." exit 0 (run first, so no window opened); `--dry-run status` "Wakeshot is not running."
  exit 0; `--dry-run keep-awake 1h` "Wakeshot is not running. Start it, then try again." exit 1, no hold file;
  `--dry-run automatic off` "Automatic is off." exit 0, `dry-run\config.json` `{"version":1,"automatic":false}`;
  `--dry-run restore` "Nothing to restore." exit 0. The log got one `cli` line each (status, keep-awake, automatic,
  restore). Only `dry-run\config.json` and `dry-run\logs\wakeshot.log` were made; both and the empty folders were
  removed after.

## Claude UI stop notice checks

Card `docs/tasks/p4-claude-transcript.md`, section "Claude parsing review fixes, round 2 (2026-11-04)"; verify
`docs/reviews/p4-claude-parsing-verify.md`, "Left from M1 part b". Read in Claude Code 2.1.317's bundled code
(`C:\Users\haika\.local\bin\claude.exe`, read only, a byte search with context; minified names and byte offsets from
that build only). Not run live; `p4-live-claude` runs the stop from the VS Code panel.

- **The UI stop queues its notice through the logged enqueue.** The stop path (`ILe`, near offset 302,206,558) calls
  `tUr` for a shell or shell Monitor (`Td(w)`) owned by the main agent when the stop has a `source` (a stop from the
  user or the connected app) and `Ube()` is on. `Ube()` is `k("tengu_rosy_donut", true)` (offset 293,636,218): a
  remote flag, on by default. `tUr` (offset 301,313,178) builds `Ai({taskId, toolUseId, outputFile, status: "killed",
  summary})` and calls `dl` with `passive: true`, priority `next`. In that build `dl` is
  `(...e) => Pg().enqueuePendingNotification(...e)` (offset 293,970,042), and the queue's
  `enqueuePendingNotification` is `vr` (export list near offset 293,967,544). Verified in code.
- **`vr` writes the line when it queues.** After its admission check (`rr`, which refuses only peer messages) it pushes
  the item and calls `s("enqueue", $fe(pi))` unless the item is `unlogged`; `tUr` does not set `unlogged`. `passive`
  changes only delivery. `$fe` gives the value text for a string value that is not a prompt under screening. `s`
  (inside `I3o`, offset 293,948,812) builds `{type: "queue-operation", operation, timestamp, sessionId, content}` and
  hands it to the default recorder (`Pgo`), which records it unless the session is diskless. Verified in code.
- **Two ways the line may not come at once** (not verified): `$r` lets a hook set in `lSn` take the enqueue over and run
  it later; what sets that hook was not traced. With `tengu_rosy_donut` off, `tUr` is not called. How soon the
  recorder's write reaches the disk was not timed.
- **The same line shape is live for other ends.** Every task notification in the live fixtures has its `enqueue` line
  first, with the whole notice as `content`: `01-bg-shell` line 5 (`completed`, user line 7), `02-monitor` lines 6, 12,
  29 (`failed`, `failed`, `completed`) and the events at 21 and 25 (no `<status>`), `08-sdk-bg-shell` line 6,
  `12-sdk-close-during-shell` line 6, `13-sdk-resume-kill` line 16 (`stopped`). Verified (fixtures, read by
  `the_live_open_tasks` and `the_live_monitors_hold_until_their_timeout`). In `07-subagent` a notice had an `enqueue`
  (21), then a `remove` (27) and an `attachment` (28).
- **Every `<status>` notice found is queued after its task ended**: the UI stop (`tUr`, after the kill), the subagent
  stop (`eLo`), the failed-task helpers (`Y`, near offset 304,059,851; `iqe`, near 319,923,901). The check-in notice
  for a quiet shell (near offset 298,210,086) and Monitor events (`Aq`) have no `<status>`. Code-read, not a full
  search of every caller.

## Launch research checks

Checked 2026-11-04 for card `docs/tasks/p6-launch-research.md`; details, quotes and links in
`docs/research/launch.md`. Nothing installed or run.

- **Rival stars** (`gh api repos/<o>/<r>`, 2026-11-04 00:45): PowerToys 140,491 (all of PowerToys), LidGuard 13,
  Agents Don't Sleep (`itsmbaqer/agents-dont-sleep`, new) 10, LidKeeper 10 (no release), Nightcrew 9, Insomnia 7.
  Picked for the README table: PowerToys Awake, LidGuard, Agents Don't Sleep; Nightcrew in reserve.
- **Rival download sizes** (release and NuGet APIs): PowerToys user setup x64 323.9 MB (v0.102.3001); LidGuard
  `lidguard.win-x64` 2.1.7 25.3 MB (a .NET tool, needs the .NET SDK to install); Agents Don't Sleep v0.4.0 x64 setup
  1.9 MB (Tauri, no ARM64); Nightcrew v0.3.1 exe 1.8 MB. None publishes idle RAM or CPU, except a PowerToys PR
  harness (C# Awake 14.3 MB private memory, PR #51181, closed) and Nightcrew's "less than 1% of one CPU core".
- **winget and self-updating apps**: winget compares the ARP `DisplayVersion` with the manifest version
  (winget-cli #1239, PM comment 2026-07-28). `RequireExplicitUpgrade: true` keeps a package out of `winget upgrade
  --all`, but only when winget installed it (`PackageTrackingCatalog.cpp`, `UpdateFlow.cpp`). The 2026 loops came from
  manifest data: VS Code 1.142.0 pointing at 1.141.2 installers (winget-pkgs #453148), Docker 4.78.0 published twice
  (#389278).
- **Velopack has no lock that stops Setup.exe** (source at tag 1.2.161): its only lock is `packages\.velopack_lock`,
  taken by apply, download and start, and by Setup only after it has killed every process in the install folder
  (`force_stop_package`, which also kills a running `Update.exe`) and renamed the old folder. Setup over an install
  always wins. Source read; not tested.
- **winget-pkgs merge time** (`gh api search/issues`, 7 days 2026-10-14 to 10-26, merged PRs only): new version
  median 3.9 h, 90th percentile 51.8 h; new package median 1.8 h, 90th 33.3 h.
- **Microsoft Store**: no registration fee for either account type (Learn, updated 2026-10-29); every PE file of an
  EXE or MSI submission must be signed by a Microsoft Trusted Root CA (Store Policies 7.20, 10.2.9); Azure Artifact
  Signing takes individual developers only in the US and Canada (Learn, updated 2026-10-08); winget may pick the
  `msstore` copy of an app installed from the winget source (winget-cli #4347, open). Recommendation: no Store at
  launch.
- **GitHub README media**: videos up to 10 MB on a free plan (GitHub docs, attaching files); an uploaded video plays
  muted with controls, an MP4 kept in the repo does not play (observed in rendered HTML, not in a browser);
  `gh issue comment <n> --attach` uploads by script (gh 2.103.3 here). `<picture>` with `prefers-color-scheme` works.
- **Video tool**: Remotion 4.0.540; its license is free for "an individual" and companies "with up to 3 employees"
  (`LICENSE.md`). Node v26.13.0 is on this laptop; `ffmpeg` is not on the PATH (Remotion bundles its own).

## CLI run checks

Checked 2026-11-04 between 01:00 and 01:11 for card `docs/tasks/p4-cli-run.md` (Windows 11 Pro 10.0.26300, Rust
1.99.1, elevated agent shell).

- **How std's `Command` finds and quotes a program** (toolchain source, `library/std/src/sys/process/windows.rs`):
  `resolve_exe` passes a path that has a folder part and ends in `.exe` to `CreateProcessW` as it is; another path is
  tried as `<path>.exe`, then as given; only a bare file name (no `\` or `/`, `path::is_file_name`) is searched, with
  `.exe` added when the name has no dot. `program_exists` reads `GetFileAttributesW`, which does not follow links.
  `make_command_line` always quotes the program and adds each argument after one space; `raw_arg` text is added
  unchanged (`Arg::Raw`), and `get_args` lists it. std starts a batch file through `GetSystemDirectoryW` plus
  `\cmd.exe`, not `%ComSpec%`. So `child::command` passes the resolved full path, and a batch file goes to
  `%ComSpec%` (a full path; else the system folder's `cmd.exe`) with `raw_arg("/d /s /c \"\"<path>\" <rest>\"")`.
- **The text reaches the child exactly** (`child::tests::an_exe_child_gets_the_text_exactly`): `cmd /u /d /c echo`
  given through `raw_arg` printed back, byte for byte, a text with doubled spaces, `\"`, `""`, trailing backslashes
  before a quote, a tab, Armenian and an emoji. A `.cmd` file in a folder with a space, named with a space, got `%*`
  exactly as typed and `%1` as its first quoted argument, and its `exit /b 3` came back as 3
  (`a_batch_child_sees_its_arguments_as_typed`, and `run::tests::a_cmd_file_sees_its_arguments_as_typed`).
- **Exit codes pass through whole**: `cmd /d /c exit -1073741510` gave 0xC000013A through `run` and through the
  built exe (`std::process::exit`); 300 gave 300 (`tests/run.rs`, `the_exe_exits_with_the_childs_whole_code`).
- **Ctrl+C, tested in a console with no window** (`tests/run.rs`, `ctrl_c_ends_the_child_and_wakeshot_exits_with_its_code`).
  The test starts a helper (the test exe itself) with `CREATE_NO_WINDOW`; it got a console of its own:
  `GetConsoleProcessList` there listed only the helper, `wakeshot.exe` and its child (pids 21476, 35068, 37348 in
  one run), never the test runner. The helper resets the inherited Ctrl+C flag (`SetConsoleCtrlHandler(NULL,
  FALSE)`), starts `wakeshot run -- <test exe> sleeper` (a child with Rust's default handling), waits for the hold
  file, ignores Ctrl+C itself (NULL form, set after the start so nothing inherits it) and calls
  `GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0)`. The child ended, `wakeshot` exited with 0xC000013A, deleted the hold
  file and logged `exited with -1073741510 (0xC000013A)`; the last two show that it outlived the Ctrl+C (the default
  handler would have ended it at once). The helper refuses to send when its console holds any other process. Not
  tested: a real key press in Windows Terminal or conhost, Ctrl+Break, closing the console window.
- **App execution aliases are found.** `%LocalAppData%\Microsoft\WindowsApps\winget.exe` is a 0-byte file with the
  `ReparsePoint` attribute here; `wakeshot run -- winget --version` (debug exe, temp `WAKESHOT_DATA_DIR`) resolved it
  through `PATH` and printed `v1.29.380`, exit 0. Here `Path::is_file`, `fs::metadata` (length 0) and
  `fs::symlink_metadata` all succeed on it too (a throwaway probe test, removed); `child` reads the attributes
  without following, as std does.
- **The card's built-exe check** (2026-11-04 01:10, debug exe, `WAKESHOT_DATA_DIR` unset, `%LocalAppData%\Wakeshot`
  missing before): `wakeshot --dry-run run -- cmd /c echo hi` printed `hi`, exit 0, and on stderr `wakeshot: Wakeshot
  is not running, so nothing keeps the PC awake for this command until it starts.` It made `dry-run\holds` (empty
  after: the hold file was deleted) and `dry-run\logs\wakeshot.log` with `cli: run: cmd started, pid 32640` and `cli:
  run: cmd exited with 0`. Both and the empty folders were removed after.

## Tray UX: icon art

Checked 2026-11-04 by `p5-icon-art` (code `crates/wakeshot-windows/src/tray/art.rs`, sheets
`docs/research/img/ux/icon-*.png`). Colours were read from Phase 0's pictures of this laptop (build 26300) with a
stdlib Python PNG reader in the card's scratchpad (`p5-icon-art\pngread.py`, `measure.py` to `measure3.py`).

- **Taskbar colours.** Dark: 202021, and 1f1f20 to 222223 along the bar (`tray-taskbar-after-promote.png`,
  `tray-icon-add2-dark-taskbar.png`); the bar is translucent, so it follows the wallpaper a little. The plate under an
  icon the mouse rests on, dark: 2d2d2e (`tray-icon-switch4-dark-taskbar.png`). Light: d8dadc, from d4d7d8 to dadcde,
  with a hover plate of ecedee (`tray-icon-switch3-light-taskbar.png`).
- **Windows' own glyph colour.** On the dark taskbar: ffffff (887 pure white pixels in the chevron, wifi, volume and
  battery glyphs of `tray-taskbar-after-promote.png`). No picture shows Windows' tray glyphs on a light taskbar. In the
  light Quick Settings picture (`tray-look-light-quick-settings.png`) the darkest glyph pixels are 1a1a1a on f5f5f5,
  1b1b1b on fcfcfc and 191919 on f1f1f1; each fits black at alpha e4 over its plate (worked out from the numbers;
  Microsoft's token value not checked). Wakeshot uses solid 1a1a1a on light (the spike used 1c1c1c) and white on dark:
  `GLYPH_ON_LIGHT`, `GLYPH_ON_DARK`.
- **Contrast** (WCAG 2, test `glyph_contrast_on_taskbars` and a Python check): white on 202021 16.3:1, on 2d2d2e
  13.8:1; 1a1a1a on d8dadc 12.4:1, on d4d7d8 12.0:1, on ecedee 14.9:1. The sample high contrast pair, yellow on black:
  19.6:1. App icon disc: white mark on e06a22 3.4:1 and on c2501a 4.7:1; the disc on f3f3f3 3.0:1 and 4.3:1, on
  202020 4.9:1 and 3.5:1. All at or above Microsoft's 3:1.
- **Windows' own glyph shapes at 150% (24 px slot).** Strokes about 1.5 px, antialiased, not snapped to pixels. Wifi
  24 px wide and 17 tall, volume 18 tall, battery 30 wide (coverage grids printed by `measure3.py`). So Windows' own
  glyphs fill the slot's width.
- **Microsoft's guidance used** (read 2026-11-04):
  - "Construct your Windows app's icon" (learn.microsoft.com/windows/apps/design/iconography/app-icon-construction,
    dated 2026-07-21): "Context menu, title bar, system tray: 16px, 20px, 24px, 32px, 40px, 48px, 64px" for 100% to
    400%; Windows looks for an exact size, else "the next size above and scale down"; the bare minimum is 16, 24, 32,
    48 and 256. Wakeshot ships 16, 20, 24, 32, 40, 48 and 64, no 256 (track note 3.2).
  - "Design guidelines for Windows app icons" (app-icon-design, updated 2026-07-14): do not rely on colour alone; make
    "at least half of your icon" pass 3:1 on light and dark; subtle gradients, default angle 120 degrees; literal
    metaphors ("an envelope for mail"); no letters in the icon. The construction page allows a plate behind the icon.
  - "Segoe Fluent Icons font" (dated 2026-10-13): glyphs look best at 16, 20, 24, 32, 40, 48 and 64.
  - No Microsoft page on notification area glyph style or padding: `.../iconography/system-tray-icons` returns 404.
    Fluent's icon keyline: 2 px padding on a 20 px icon, doubled at 48 px, and a 2 px stroke at 32 px (Fluent icon
    designer Jason Custer, github.com/microsoft/fluentui-system-icons issue 44, 2020 and 2021).
- **Padding rule taken:** size / 16, at least 1 px (1 px at 16 to 28, 2 at 32 to 40, 3 at 48 and 56, 4 at 64). Between
  Fluent's keyline and Windows' tray glyphs, which use the whole slot width: the glyph reads as large as its
  neighbours and never touches the hover plate's edge. Test `glyph_stays_inside_its_box`.
- **Stroke:** whole pixels, size / 16 rounded (1 px at 16 and 20, 2 at 24 to 36, 3 at 40 and 48, 4 at 56 and 64), with
  straight edges snapped to pixel edges. At 16 to 32 px the cup has at most 13% half-covered pixels (its round bottom
  and the off bar); a stroke off the grid would give 50% or more. At 24 px the cup's 2 px strokes are a little heavier
  than Windows' 1.5 px.
- **Concepts and pick.** Cup (picked): a full cup with steam (holding), an empty cup (idle), the empty cup struck
  through (off). Eye: open eye, closed eye with lashes, struck through. Sky: sun, moon, struck-through moon. States
  differ in shape: test `states_differ_in_shape` asks that at least 25% of two states' ink sits in different pixels at
  16 and 24 px; measured 34% to 90% (the cup: 38% idle to off, over 50% against holding; read before the last steam
  change, which only adds ink to holding). App icon: the holding glyph in white on a disc from e06a22 to c2501a.
- **ICO and PNG.** `ico` writes 32-bit BGRA frames with an AND mask (bit set where alpha is 0; those pixels carry no
  colour). .NET `System.Drawing.Icon` loaded each of the seven frames of `assets/wakeshot.ico` at its size, with a
  transparent corner and the disc's orange inside (2026-11-04, between 01:20 and 01:26); both x64 release exes carry
  it (`ExtractAssociatedIcon`). `png` deflates with fixed codes and runs only,
  and writes a palette when an image has at most 256 colours: the sheets are 1892 by 3712 px and 107 KB each (a stored
  PNG would be about 28 MB); the app sheet (1582 colours, RGBA) is 333 KB. Python's `zlib` inflated every IDAT stream
  and every chunk CRC matched (`colours.py`).
- **Same bytes on x64 and ARM64:** the shapes use `+`, `-`, `*`, `/`, `sqrt` and `round` only, all exact in IEEE 754,
  and Rust does not fuse a multiply and an add on its own. So the golden test compares exactly. Tests ran on x64 only;
  ARM64 got clippy and builds, not a test run.

## Path value checks

Checked 2026-11-04 between 01:15 and 01:28 for card `docs/tasks/p6-login-and-path.md` (Windows 11 Pro 10.0.26300,
`windows` 0.62.2, elevated agent shell). Tests wrote only scratch keys under `HKCU\Software\Wakeshot-test\<pid>`.

- **`RegSetValueExW` stores a NUL it was not given.** For string types, when the data does not end in a NUL and the
  UTF-16 unit just past the data is a NUL, Windows stores that NUL too. Seen first as a test that failed 3 runs in 5
  (the unit past a `Vec` is whatever the heap holds there), then pinned by
  `install::tests::windows_stores_a_nul_just_past_string_data`: a buffer `C:\A` plus NUL, passed without the NUL,
  was stored with it, 3 runs of 3. Wine's `RegSetValueExW` has the same rule, with the comment "if user forgot to
  count terminating null, add it (yes NT does this)" (`dlls/kernelbase/registry.c`, GitHub mirror `wine-mirror/wine`,
  `master`, read 2026-11-04: `if (str[count / sizeof(WCHAR) - 1] && !str[count / sizeof(WCHAR)]) count +=
  sizeof(WCHAR);`). Microsoft's page for
  the function does not mention it (not verified). So Windows reads one unit past the buffer it is given; `install`
  puts a non-zero guard unit there, and a value stored without a terminator then comes back byte for byte.
- **A value's name keeps its case.** A value `PATH` updated and then restored through the name `Path` was still
  listed as `PATH` by `RegEnumValueW` (`install::tests::the_value_keeps_its_name_case`).
- **Raw reads and writes keep type and bytes.** `RegQueryValueExW` gives the bytes as stored (no terminator added, no
  `%VAR%` expanded); `RegSetValueExW` with the old type stores them back. Round trips on the scratch key gave back the
  exact old bytes and type for `REG_SZ`, `REG_EXPAND_SZ` with `%USERPROFILE%`, a trailing `;`, an empty value of 0
  and of 2 bytes, a value with no terminator, and bytes after the first NUL (the `install::tests::round_trip_*`
  tests).
- **`SendMessageTimeoutW` with `HWND_BROADCAST`** (Microsoft Learn, read 2026-11-04): the timeout is per window ("each
  window can use the full time-out period ... up to a 15 second delay" for three windows that do not answer);
  `SMTO_ABORTIFHUNG` returns at once for a thread that "appears to not respond", which means it has not called
  `GetMessage` or the like within five seconds; the result gives no per-window outcome for a broadcast; a zero result
  with no last error is "a generic failure", and the function does not always set the last error, so set it to 0
  first. `install` uses 1 s per window. Never run here: tests inject the broadcast, so no test sent it.
- **Environment variable limit** (Microsoft Learn, "Environment Variables", read 2026-11-04): "The maximum size of a
  user-defined environment variable is 32,767 characters." The same page says to broadcast `WM_SETTINGCHANGE` with
  `lParam` "Environment" after changing the system variables in the registry; it names no flags or timeout. What
  Windows does with a longer user Path: not verified, so `install` refuses an add that would pass the limit.
- **The real keys were not touched.** A read-only script hashed the user `Path` (type and text) and every value under
  the real `Run` key, and `reg export` of both keys, before and after the test runs: the same hashes each time (card
  Log).

## IO thread checks

Checked 2026-11-04 01:26 for card `docs/tasks/p6-int-io-thread.md` (Windows 11 Pro 10.0.26300, Rust 1.99.1, elevated
agent shell). Fakes and temp folders only: no power plan, notice or window was touched.

- **A `WaitForSingleObject` timeout can end a few ms early by Rust's clock.** `IoThread::finish(200)` waits on the
  thread's handle with a 200 ms timeout. With the thread blocked, `Instant` measured 196.5 ms and 198.6 ms in 2 of 5
  runs of `app::io_thread::tests::finish_gives_up_when_the_executor_blocks`; the other 3 were at or above 200 ms. Why
  (the wait and `Instant` use different clocks, or the timer tick): not verified. So a test that times a Windows wait
  checks a looser lower bound (150 ms for 200); 10 of 10 runs passed with it.

## Update thread checks

Checked 2026-11-04 between 01:30 and 01:40 for card `docs/tasks/p6-update-schedule.md` (Rust 1.99.1).

- **The schedule, on the fake clock** (`update::tests`): the first check after 5 minutes of run time, then every 12
  hours; a sleep of 20 hours mid-wait, then the check within one 60 s wait of the wake-up; a wall clock set back 2
  days, the check still 12 hours of run time after the last; a wall clock set forward 1 hour, no check; set forward a
  day, a check within one wait. The fake wait runs the fake clock, so no test sleeps.
- **The real thread** (system clock, `std::sync::mpsc`): an apply request reached the thread and its answer came back
  through the sink; `UpdateHandle::stop` returned in under 1 s while the thread waited; dropping the handle ended the
  thread (the test's channel closed). `UpdateHandle` is `Send` and `Sync` (a compile-time check), so `mpsc::Sender`
  is `Sync` on this toolchain.
- **Not verified:** whether `Receiver::recv_timeout` counts time the PC sleeps. The thread does not rely on it: it
  reads both clocks again at least every 60 s of waiting.

## Tray UX: tray icon

Card `p5-tray-icon`, 2026-11-04 (code `crates/wakeshot-windows/src/tray/icon.rs`, harness
`crates/wakeshot-windows/examples/ux_icon.rs`). Part A facts first (no test calls `Shell_NotifyIconW`; the tests use a
fake shell), then the Part B checks that ran on the real taskbar. The picture and click checks are still to come.

- **Icon GUIDs** (made with PowerShell `[guid]::NewGuid()`, 01:37): Product `7db9403e-d4b9-4165-a99e-a69b8ea56744`,
  DryRun `c0d07dcd-dfd4-4a9d-b139-ad1e2ce9f987`, Demo `1e8c66d4-0fa3-4a54-95a1-61bb89b171e9` (`IconKind::guid`).
- **High contrast glyph colour: `COLOR_WINDOWTEXT`.** Microsoft's "Contrast themes" page
  (learn.microsoft.com/windows/apps/design/accessibility/high-contrast-themes, dated 2026-08-29, read 2026-11-04)
  says "Pages, panes, popups, and bars should generally use SystemColorWindowColor as the background baseline" and
  pairs `SystemColorWindowColor` with `SystemColorWindowTextColor`. No Microsoft page names the colour Windows' own
  taskbar uses in a contrast theme: not verified.
- **`HICON` from the art reads back** (test `hicon_from_art_reads_back`, 2026-11-04): a 24 px glyph through a 32-bit
  top-down DIB section (BGRA) and `CreateIconIndirect` gives a 24 by 24, 32 bpp colour bitmap from `GetIconInfo`;
  `GetDIBits` returns the pixel as B, G, R with alpha 0xff kept, and 0 alpha where the art is clear.
- **`GetSystemMetricsForDpi(SM_CXSMICON, dpi)`** on this PC gives 16, 20, 24 and 32 at 96, 120, 144 and 192 dpi, and
  `16 * dpi / 96` at every 25% step from 100% to 400% (test `glyph_size_per_dpi`).
- **Not verified, for Part B:** the high word of the callback `lParam` for a GUID icon (the code sets `uID` 0, so an
  echo of `uID` and a 0 both pass); whether a `NIM_ADD` of a GUID left by a run that ended without `NIM_DELETE` fails
  (the code deletes the GUID and adds once more when an add fails); whether `Shell_TrayWnd`'s DPI is the tray slot's
  size and the shell shows the drawn icon 1:1.

Part B, first run 2026-11-04 02:16 to 02:18, with the session locked (so no pictures, clicks or theme switch; card
Log). The DELL 3840x2160 at 150% was the primary and held the taskbar.

- **The demo at Medium adds its GUID icon.** `ux_icon` started with `explorer.exe <launch .cmd>`: `whoami /groups`
  in the `.cmd` gave "Medium Mandatory Level" (S-1-16-8192); the demo logged `Added { attempt: 1 }` (`NIM_ADD` and
  `NIM_SETVERSION` 4 both TRUE) while the session was locked.
- **A GUID icon's settings entry.** Windows wrote `HKCU\Control Panel\NotifyIconSettings\<id>` with only `IconGuid`
  (the demo GUID), `ExecutablePath` and `IconSnapshot`: no `UID` and no `IsPromoted`. Read with `Get-ItemProperty`.
- **Tray slot and taskbar DPI.** `Shell_NotifyIconGetRect` by GUID from a per-monitor v2 caller: 48x72 physical
  pixels. `GetDpiForWindow(Shell_TrayWnd)` 144, so the demo drew 24 px (`SM_CXSMICON` at 144), half the slot's width,
  which is Windows 11's 16 px icon in a 32 px slot at 150% (inferred; whether the shell shows it 1:1 needs the
  `--pattern` picture).
- **A posted `TaskbarCreated` re-adds.** `RegisterWindowMessage("TaskbarCreated")` was `0xc07f`; posted to the demo's
  window, the demo logged `TaskbarCreated` and `Added { attempt: 1 }` 14 ms later, and the rect stayed the same. Not in
  the log: whether the first `NIM_ADD` of the GUID that was still there failed and the delete-and-add ran.
- **`taskkill /PID` without `/F` quits the demo.** taskkill's `WM_CLOSE` reached the hidden window's thread: the demo
  logged `WM_CLOSE: quitting` 0.2 s later and `icon removed`, exit 0; `Shell_NotifyIconGetRect` then failed with
  `0x80004005` (no such icon).
- **The session locks 5 minutes after the last input on AC.** `LogonUI` started 02:00:38, 301 s after the last input
  (`GetLastInputInfo`); `VIDEOIDLE` AC is 300 s and `DelayLockInterval` 0 (lock when the display turns off);
  `WTSSessionInfoEx` gave `SessionFlags` 0 (locked). So "Haik away 15 minutes" and "screen usable" cannot both hold
  unless a display request keeps the display on from before the 5 minutes run out.

## Core lid round A checks

Card `p2-core-lid`, section "Core round A, lid fixes (2026-11-04)": power safety review finding 5 and verify minors 1 to
5. Pure core code; nothing here touched the laptop or its power settings.

- **Tests written first fail first.** 12 new scenarios against the unchanged core: 8 failed, 4 passed by design (two
  pin code that was right already, two guard the new plan copy rule). Three more came from the mutant plan.
- **Only the lid side reads plan values.** A grep of `crates/wakeshot-core/src` for `state.plan` (2026-11-04): the
  settings side (`core/settings.rs`) reads only `.active`; `core/update.rs`, `core/view.rs`, `core/holds.rs` and
  `present.rs` do not read the plan copy. So putting a restored `original` back in the copy changes only the lid,
  lock and countdown rules.
- **Decision 0 rules out a silent lock.** Every `Lock` must come with a notice or a sound in its step
  (`docs/architecture.md`, decisions table, row 0); the property `every_lock_step_back_and_user_change_is_explained`
  and the lid proptest check it. So the lock after a hibernate that did not happen carries the lid-close chime.
- **Long property runs** (`PROPTEST_CASES=10000`, release, `scenarios::props`, the lid proptest,
  `scenarios::settings`) on the final core code: `a_released_key_is_never_ours_in_force_again_in_its_episode` failed
  in three of seven runs, each a plan switch with failed restores and no lid command in the trace (the property
  clears its "restore failed" flag at a restore of another plan that worked, while the core still retries); every
  other property passed in each of the four runs of the whole set.
- **Mutants by hand, 27 of 27 killed** (scratch copy of the workspace with `docs/`, since `holds/file.rs` tests read
  `docs/hold-files.md`; own build folder; each reverted): the card's four, and 23 on the lines of this round.
- **Not verified:** what a closed laptop does after a hibernate that is not allowed, with the user's own settings back
  and no request held. The product text infers it runs until the user's own sleep timeout (not tested with the lid
  closed); the lock every 5 minutes rests on that inference. Also not verified: what `SetSuspendState(FALSE)` does on a
  Modern Standby PC with hibernation off, the case a sleep fallback would be for.

## Live Claude end to end checks

Card `p4-live-claude`, 2026-11-04 01:27 to 01:53, Claude Code 2.1.317, Haiku test sessions in scratch folders, the
real watch thread in the harness `crates/wakeshot-windows/examples/agent-watch.rs` on the dry-run data folder. Full
table: `docs/research/claude-code-signals.md`, "Phase 4 end to end".

- **Every Claude case of the live tests table matched the rules** (1 to 12, 14, resume, `/clear` during a loop, the
  SDK path): seen live, from the harness lines and the `--raw` line kinds. Resume: a `*/5` job came back at
  `procStart` plus 285 s, its fired wakeup did not; `procStart` is the right divider.
- **`TaskStop`, a Monitor timeout and a stop from the UI end tasks as rule 3 says** (seen live on the SDK path).
  `TaskStop` writes no notification. A Monitor timeout writes one `<event>` notice with no `<status>`. A stop from
  the terminal tasks panel, or an SDK `stop_task` control request, writes a `queue-operation` `enqueue` with
  `<status>killed</status>` at the stop, before any user line.
- **`CLAUDE_CONFIG_DIR` moves `sessions\`** (live: a session started with it wrote under that folder only; code:
  `CLAUDE_CONFIG_DIR ?? join(homedir(), ".claude")`). `projects\` moves with it by the code (`join($e(),
  "projects")`); not seen live, the test session stopped at onboarding. `sources::claude::claude_folder` picks the
  folder the same way; the tray must use it (integration). A value set only in one shell is not visible to the tray.
- **Timing**: a status change was read 7 to 189 ms after its `statusUpdatedAt` (median 29 ms, 170 writes) and posted
  at once. A transcript line with no status write is read at the next sessions folder change or the 30 s tick (seen:
  6.8 s, 12 s, 22 s).
- **Hold files**: `wakeshot --dry-run run -- ping` showed `Run alive=Yes` 115 ms after start and went at the child's
  end; a script hold file with a dead pid showed `Gone` in 24 ms, its reason never printed. With the whole data
  folder missing when the watch thread started, the first hold showed only at the next tick (19.6 s): the thread
  retries a missing root only on the tick (a missing `holds\` alone is picked up at once). The tray makes its data
  folder before the thread starts, so only a harness sees this (not verified in the tray).
- **`wakeshot run` and the console, in a pseudo console** (ConPTY, the host Windows Terminal uses; not a classic
  conhost window): Ctrl+C reached the child (`ping` ended with `0xC000013A`), `wakeshot` returned the same code and
  removed its hold 0.34 s after the key. Ctrl+Break (`GenerateConsoleCtrlEvent`): `ping` printed its statistics and
  went on, `wakeshot` stayed and kept the hold until the child ended, then exited 0 and removed it. Closing the
  pseudo console while it waited: `wakeshot` and `ping` ended and the hold file was gone 12 ms later. The CLI log has
  no exit line for that run, so `wakeshot` likely ended right after it removed the file (inferred, not verified); had
  it ended first, the hold would show `Gone`. Not run: a key press by a person, a classic conhost window.
- **Harness Ctrl+C**: stopped the thread cleanly (`stop` returned `true`, exit 0).

## Tray UX: chimes

Card `docs/tasks/p5-sounds.md`, 2026-11-04. Part A: code and reads only, no sound played. Part B (02:31 to 02:33):
each chime played through `examples/ux_sounds.rs` with a loopback capture running, reads before and after.

- **The "No Sounds" scheme's key name is `.None`** (read on this PC, 2026-11-04 before 01:54, registry reads only):
  `HKCU\AppEvents\Schemes` default value `.Default`; `Schemes\Names` holds `.Default` = `@mmres.dll,-800` and
  `.None` = `@mmres.dll,-801`; `SHLoadIndirectString` loads them as "Windows Default" and "No Sounds".
- **What Windows writes when the user picks "No Sounds"**: no Microsoft Learn page found (web search 2026-11-04).
  Third-party pages (ITPro Today "JSI Tip 6827", Petri) select the scheme by setting the `Schemes` default value to
  `.None` (one writes `.NONE`). That the Sound dialog writes `.None` there: not verified, since no card may switch
  Haik's scheme. A forum post says a scheme changed after picking shows as "(modified)"; that this is stored as
  `.Current`: not verified.
- **Rule built** (`sound::player`): the chimes are skipped only when the value is `.None`, case ignored. `.Default`,
  `.Current`, a saved scheme's name and a value that cannot be read all play, so only a clear "No Sounds" keeps
  Wakeshot quiet.
- **windows-rs 0.62.2** (read in the crate source): `PROPVARIANT` has no `Drop`, so each `IPropertyStore::GetValue`
  result needs `PropVariantClear` (the lid spike's name read never cleared it; spike only); `WAVEFORMATEX` is
  `repr(C, packed(1))`, so the mix format is read with `read_unaligned`.
- **Shapes** (`sound/chime.rs`; each has 150 ms lead and 100 ms tail silence, raised-cosine fades): lid closed
  1046.5 Hz 160 ms then 1568 Hz 340 ms, rising, -6 dBFS, 750 ms in all; going to sleep the same two notes falling
  (170 ms, 380 ms), -9 dBFS, 800 ms; battery floor two 130 ms pulses on 659.25 Hz, 100 ms apart, -6 dBFS, 610 ms.
- **Part B setup** (2026-11-04 02:31 to 02:33): session locked since 02:00:38 (screen off, then the lock), on AC, lid
  open. The DELL's two endpoints read unplugged (state 8), so Speakers (Realtek) was the only active endpoint and the
  default for all three roles (67.33%, not muted, System Sounds session 100%), scheme `.Default`. Harness
  `ux_sounds` (debug build). Loopback: a scratch tool outside the repo, ported from the lid spike's listener, sample
  by sample: -50 dBFS threshold, a sound ends after 300 ms of quiet, notes split at 40 ms of quiet, pitch from rising
  zero crossings of channel 0 per 50 ms, times from each packet's QPC position. Logs stayed in the worker's
  scratchpad.
- **A chime plays and reaches loopback in a locked session.** Every one of the nine chimes below did. Agrees with
  Phase 1 close 1 (the chime after the lock was heard).
- **Per cue, seen in the Speakers' loopback** (three runs each: `play` alone, `--all`, `--wait` alone; "warm" is the
  2nd or 3rd stream of one `--all` process):

  | Cue | Stream running after the call | Call to first sound in loopback | Length (tone) | Peak, loopback (PCM) | Pitch seen |
  |---|---|---|---|---|---|
  | lid closed | 416, 369, 384 ms | 614, 567, 582 ms | 501, 502, 502 ms (500) | -0.2 dBFS (-6) | 1020 to 1060 Hz, then 1540 to 1600 Hz: rising |
  | going to sleep | 382, 376 ms; warm 103 | 586, 577 ms; warm 303 | 546 ms three times (550) | -2.6 dBFS (-9) | 1560 to 1580 Hz, then 1020 to 1060 Hz: falling |
  | battery floor | 374, 377 ms; warm 102 | 573, 575 ms; warm 298 | 361, 362, 362 ms (360) | -0.1 dBFS (-6) | two pulses of 131 to 132 ms, 660 to 663 Hz, the 2nd at +229 to 230 ms |

  The first sound comes 46 to 54 ms after the stream runs plus the 150 ms lead silence, in every run. The sleep
  chime reads 4 ms short: its 30 ms fade ends below the threshold.
- **The slow stream start is paid once per process.** The first stream of each process ran 369 to 416 ms after the
  call (research "Lid-close sound": 290 to 570 ms, almost all in `IAudioClient::Initialize`); the 2nd and 3rd of the
  same process, 3 s apart, ran 102 to 103 ms after it, though each sound has its own thread and COM. Why, and how
  long it stays fast: not verified.
- **`play_and_wait` returns after the chime**: `Ok(Played)` 1135 ms (lid), 1177 ms (sleep) and 986 ms (floor) after
  the call, each 52 to 55 ms after the tone's last sample in loopback, so about 1.8 s under the 3 s limit. It returns
  once the engine has taken every frame, so only part of the 100 ms tail silence is still to play then (inference
  from the code and these times).
- **The endpoint path adds about 6 dB**: -6 dBFS chimes peak at -0.2 and -0.1 dBFS in loopback, the -9 dBFS one at
  -2.6 (as "Lid-close sound": -6 to -0.4). So the two -6 dBFS chimes sit within 0.2 dB of full scale there, and a
  louder chime would gain little. Whether a limiter or the Dolby effect caps it: not verified.
- **Not run live in Part B**: the display-audio branch of the device rule (no DELL endpoint was active; Phase 1
  close 2 ran the same rule live) and the "No Sounds" skip (Haik's scheme stays `.Default`; unit test only).
- **Reads before (02:31:21) and after (02:33:40) were equal**: scheme, default per role, all 14 playback endpoints
  with state and form factor, Speakers master and channel volumes and mute, System Sounds session level and mute; 20
  lines, compared line by line. Nothing was written.

## Hook checks

Card `docs/tasks/p6-hooks.md`, 2026-11-04. Tests in `crates/wakeshot-windows/src/hooks.rs` on this laptop (Windows
11 Pro 26300, `%TEMP%` on NTFS), against a temp data folder and scratch registry keys only.

- **A file deleted while another handle has it open with delete sharing loses its name at once**: the test opens
  `logs\wakeshot.log` with `LogWriter` (all three share flags), then deletes it with `std::fs::remove_file`
  (`DeleteFileW`); `RemoveDirectoryW` of `logs` and of the data folder right after both succeed, and later writes
  through the open handle bring nothing back (`real_delete_all_takes_the_open_log_too`, passed on every run). So the
  uninstall hook can delete the data folder while its own log is open. That this comes from `DeleteFileW`'s POSIX
  delete on NTFS (Microsoft, `FILE_DISPOSITION_INFORMATION_EX`) is inferred, not verified; FAT and network folders:
  not verified.
- **A file open without delete sharing stops `std::fs::remove_dir_all` of its folder** with code 32
  (`ERROR_SHARING_VIOLATION`); the folder stays (`real_folder_that_cannot_go_whole_keeps_its_log`, a file in
  `holds\` opened with `FILE_SHARE_READ` only). Which of the other files in that folder were already gone: not
  checked.
- **Updated hook limit: 10 s, not 12 s.** `p6-velopack-facts` (Result, "Design calls") and `architecture.md`
  section 5, restore step 7, give 10 s; `06-ship.md` "Hooks" still said 12 s (read 2026-11-04 01:50). The code uses
  10 s.

## Tray UX: notices

Card `p5-notices`, 2026-11-04 (code `crates/wakeshot-windows/src/notify.rs`, harness
`crates/wakeshot-windows/examples/ux_notices.rs`). Part A only so far: no balloon was shown yet (no test calls
`Shell_NotifyIconW`; the tests use a fake shell). The screen results (icon choice, click, pair, Do not disturb) come
with Part B.

- **Microsoft's `NOTIFYICONDATAW` page** (learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-notifyicondataw,
  updated 2024-11-20, read 2026-11-04): with `NIIF_USER`, Vista and later show `hBalloonIcon`; `NIIF_LARGE_ICON` asks
  for the `SM_CXICON` size, "can be used with all stock icons", and with `hBalloonIcon` the app "must supply an
  SM_CXICON x SM_CYICON version". An empty `szInfoTitle` hides the icon; an empty `szInfo` removes a balloon.
  `NIF_REALTIME` discards a balloon that cannot show at once. `NIIF_RESPECT_QUIET_TIME` drops a balloon, without
  queueing it, in the user's first hour after the first sign-in or after an OS upgrade or clean install. `uTimeout`
  is ignored since Vista. One balloon at a time: a new one "is queued and displayed when the older notification goes
  away". The page does not say whether the shell copies `hBalloonIcon`.
- **Microsoft's `Shell_NotifyIconW` page** (updated 2025-07-01, read 2026-11-04): the return value section names no
  last error for FALSE. `NIN_BALLOONUSERCLICK` is "Sent when the balloon is dismissed because the user clicked the
  mouse", with no word of which balloon. "On Windows 11 ... When a banner notification times out or is otherwise
  dismissed, it will not be displayed in the Notification Center." This agrees with "Notification centre" above (no
  entry for the balloon). Whether a banner held back by Do not disturb shows later or is lost: not verified (Part B).
- **`CreateIconFromResourceEx` keeps 32-bit alpha** (test `large_icon_reads_back`, 2026-11-04): one frame of
  `art::ico` (a BITMAPINFOHEADER of twice the height, bottom-up BGRA, then the AND mask) at 48 px, format version
  0x00030000 and `LR_DEFAULTCOLOR`, gives a 48 by 48, 32 bpp colour bitmap; `GetDIBits` returns the disc's centre
  pixel as B, G, R with its alpha, and alpha 0 at the clear corner.

## Release workflow checks

Checked 2026-11-04 between 02:05 and 02:18 for card `docs/tasks/p6-pack-and-release.md`: `vpk` 1.2.161 on this
laptop, Velopack and winget-create source at their release tags through `gh api`, and `gh api` release data. Nothing
installed; packs went to the scratch folder only.

- **`vpk download github` with no release yet** (this repo, 2026-11-04 02:09): "No releases found", "No full /
  applicable release was found to download. Aborting.", exit 0, and the out folder is not created. So the first
  tagged run packs with no delta. Source (`src/vpk/Velopack.Deployment/_DownloadCommandRunner.cs`, tag `1.2.161`): it
  downloads only the newest full package of the channel (no index file), and a checksum mismatch logs an error and
  returns without failing the command, so a bad download only costs the delta. **Verified** (run and source).
- **vpk finds earlier releases by `*.nupkg` in the out folder** (`ReleaseEntryHelper.GetReleasesFromDirAsync`), each
  package's channel read from the package, and refuses a version not above one there ("There is a release in channel
  win-x64 which is equal or greater to the current version", exit -1, seen). A 0.1.1 pack with the 0.1.0 full package
  copied in made a 8,975 byte delta (one of 5 files patched). **Verified** (source and run).
- **`vpk upload github` uploads only this pack's files**: the files in `assets.<channel>.json` plus a new
  `releases.<channel>.json` built from them (`_GitRelease.cs`, `GitReleaseUploadCommandRunner.RunCoreAsync`), so an
  earlier full package left in the out folder for the delta is not uploaded again. The release body is the newest
  full package's release notes (none unless `--releaseNotes` was packed). With `--merge` it finds the release by tag
  or name, drafts included as listed by the token; it refuses a channel whose `releases.<channel>.json` is already
  there. **Verified** (source); not run (uploads are Haik's).
- **Every vpk option also reads a `VPK_*` variable** (`src/vpk/Velopack.Vpk/Commands/BaseCommand.cs`): a value not
  given on the command line comes from `VPK_<NAME>`; "required" is checked after that, so `VPK_TOKEN` satisfies
  `--token`. A stray `VPK_DELTA` or `VPK_SIGN_PARAMS` would change a pack silently, which is why `pack.ps1` refuses to
  run while any is set and `release.yml` gives `VPK_TOKEN` only to the download and upload steps. **Verified**
  (source; `vpk pack -H` lists an `ENV=VPK_...` name for each option).
- **DisplayVersion is three numbers only.** Velopack writes `get_manifest_version_short_string()`, which is
  `major.minor.patch` (`src/lib-rust/src/locator.rs`, used by `src/bins/src/windows/registry.rs` for install and each
  update). Hooks get the whole version, `get_manifest_version_full_string()` (`src/bins/src/windows/util.rs`,
  `run_hook`). So `0.1.0-rc.1` shows as `0.1.0` in Apps and Features while the install hook logs `0.1.0-rc.1`; the
  smoke test checks each against its own form. **Verified** (source); not seen in a real install.
- **SemVer pre-release versions pack.** `pack.ps1 -Version 0.1.2-rc.1` gave
  `HaikAsatryan.Wakeshot-0.1.2-rc.1-win-x64-full.nupkg`; Setup and portable zip names carry no version. **Verified**
  (run).
- **Pack sizes, repo exes without `velopack` yet** (`wakeshot-tray.exe` 426,496 and `wakeshot.exe` 667,648 bytes
  x64; 406,016 and 615,936 ARM64): x64 Setup.exe 10,539,825, full package 2,982,193, portable zip 2,974,717; ARM64
  Setup.exe 8,821,095 (PE machine 0xAA64), full package 2,869,607, portable zip 2,862,130. A pack takes about 3 s.
  The `velopack` crate adds about 1.4 MB per tray exe (trial in `research/distribution.md`, section 1). **Verified**
  (run).
- **Action and tool versions** (`gh api .../releases/latest`, 2026-11-04): `actions/download-artifact` v8.0.2; its
  `v8` tag exists, it runs on `node24`, and v8 fails a download whose digest does not match by default
  (`digest-mismatch: error`). `actions/checkout` v7.0.1 and `actions/upload-artifact` v7.0.2 as in "CI workflow".
  `microsoft/winget-create` v1.12.13.0: asset `wingetcreate.exe`, 39,486,456 bytes, SHA-256
  `24042bd37915805615e6cf969ac57c6439124c3fe85823327f5f3fb24bd9ffea` (the API's asset digest; the exe was not
  downloaded). **Verified** (API).
- **`wingetcreate submit`** (source at tag `v1.12.13.0`): takes a manifest file or a folder; for a folder it reads
  every file in it (`SubmitCommand.cs`, `Directory.GetFiles`), so the folder must hold only the manifests. It validates
  YAML manifests before submitting (`BaseCommand.ValidateManifest`, `WinGetUtil.ValidateManifest`). The token comes
  from `--token` or the variable `WINGET_CREATE_GITHUB_TOKEN` (`TokenHelper.cs`), which `doc/token.md` recommends for
  CI; `--no-open` keeps it from opening a browser. It has telemetry (`Telemetry/` in the source); how to turn it off on
  a runner: not verified. **Verified** (source and docs).
- **actionlint was not run on `release.yml`**: downloading the actionlint release exe was refused by this session's
  auto mode classifier, and no YAML parser is on this laptop (no PyYAML, no js-yaml). The workflow got a careful read,
  and its version step ran locally (four refs and two versions, all as expected). **Not verified** until actionlint or
  the first GitHub run.

## winget manifest checks

Checked 2026-11-04 between 02:20 and 02:23 for card `docs/tasks/p6-winget-manifest.md`: `gh api` reads of
`microsoft/winget-pkgs`, `microsoft/winget-cli` and `microsoft/winget-create`, and `winget` v1.29.380 on this laptop.
Tool facts used by the release workflow are in "Release workflow checks".

- **Schema 1.28.0 is the newest winget-pkgs accepts.** `doc/manifest/schema/` in winget-pkgs holds 1.0.0 to 1.12.0
  and 1.28.0. In the 40 newest commits on its master (merged 2026-11-03), the installer manifests listed were 21 at
  `ManifestVersion: 1.28.0`, 13 at 1.12.0 and 6 older, so the pipeline merges 1.28.0 today. The client repo has JSON
  schemas for 1.28.0 (`schemas/JSON/manifests/v1.28.0/`). **Verified** (API).
- **Fields in the 1.28.0 schemas** (winget-cli JSON): installer root has `RequireExplicitUpgrade`, `Commands`,
  `InstallModes`, `ReleaseDate` (format `date`), `UpgradeBehavior`, `ProductCode`, `AppsAndFeaturesEntries`; it
  requires only `PackageIdentifier`, `PackageVersion`, `Installers`, `ManifestType`, `ManifestVersion`. Default
  locale requires `PackageLocale`, `Publisher`, `PackageName`, `License`, `ShortDescription` (3 to 256 characters);
  `Tags` at most 16, each 1 to 40 characters; `InstallationNotes` (up to 10,000 characters) is "displayed to the user
  upon completion of a package installation". **Verified** (schema files).
- **`winget validate --manifest <folder>` passes** on the folder `fill.ps1` wrote from this card's local packs (x64
  and ARM64 `Setup.exe` 0.1.0), with and without `ReleaseDate`: "Manifest validation succeeded.", exit 0. The same
  folder with `InstallerType: bogus` fails ("Schema Error ... Failed to match against any enum values", exit
  -1978335191), so the check is real. It ran in this elevated agent shell; whether it needs an administrator: not
  checked under a Medium token (it reads files only). **Verified** (run).
- **`wingetcreate submit` forks for you**: when the token's account has no fork of `microsoft/winget-pkgs`, it creates
  one (`Repository.Forks.Create`) and syncs it with upstream before each PR (`src/WingetCreateCore/Common/GitHub.cs`,
  tag `v1.12.13.0`). **Verified** (source).
- **No release yet**: `fill.ps1 -Version 0.1.0` without `-FromDir` gets 404 from
  `api.github.com/repos/HaikAsatryan/wakeshot/releases/tags/v0.1.0` and stops. Its download path (hash of the public
  asset URL, checked against the API's asset `digest`) runs first on a real release. **Not verified** until then.

## Core settings round B checks

Card `p2-core-settings`, section "Core round B (2026-11-04)": power safety review findings 8, 9 and 10, the config
version rule, and `p2-core-lid` round A's Needs another card items 1, 2, 3 and 5. Pure core code; nothing here touched
the laptop or its power settings.

- **The power track names the settings calls exactly.** `crates/wakeshot-windows/src/power/plan.rs` (read 2026-11-04)
  builds each failure with `win("PowerSettingAccessCheckEx", ..)`, `win("PowerWriteACValueIndex", ..)` and
  `win("PowerWriteDCValueIndex", ..)`, so the core can tell a policy block from a 5 elsewhere by `Failure::call`. A
  journal mutex open error is passed on as is (`power/journal.rs`, `store.rs`, per the review; not re-read here).
  Whether an elevated `wakeshot restore`'s mutex refuses a Medium open: **not verified**.
- **The core cannot see a newer `config.json`.** `app/startup.rs::read_config` gives `Config::default()` for
  `Read::Newer` with `first_run: false`, and `watch/data.rs::config` posts no event for it (read 2026-11-04), so
  `Event::Started` and `ConfigChanged` look the same as for a version 1 file with Automatic on. The CLI's
  `wakeshot automatic` already refuses to write over a newer file (`wakeshot-cli/src/automatic.rs`). So the write
  check belongs to the tray's `WriteConfig` runner (`app/io_thread.rs`), which writes with no check today.
- **`Applied` is built outside the core.** `power/journal.rs` builds `Applied { .. }` with a struct literal and
  matches `ApplyError` with no wildcard (`log_applied`), so a new field or variant would break `crates/wakeshot-windows`.
  Finding 10 therefore keeps the user-changed entry for the next ownership check rather than adding
  `Applied.user_changed`.
- **Tests written first fail first.** On the old code 9 new or changed tests failed (finding 8: 1 scenario; findings 9
  and 10: 6 apply tests, 1 check test, and the strict `ownership::props` test no longer ignored). The two minimal cases
  of round A item 2, kept as named tests, failed on the old model rule and pass on the new one.
- **Long property runs** (`PROPTEST_CASES=10000`, release, 2026-11-04 02:15 to 02:23): `scenarios::props` passed in
  10 of 10 runs (19 tests each, 16 to 20 s); `ownership::props` in 3 of 3; `scenarios::settings` with the lid
  proptest in 1 of 1.
- **Mutants by hand, 11 of 11 killed** (scratch copy of the workspace with `docs/`, own build folder, each reverted).
  One survived the first round (any code from a settings call counted for the run); the finding 8 scenario now also
  feeds a settings call's non-denied code and a `PowerWriteDCValueIndex` 31, and kills it. The two lock step back
  mutants (requests kept; no restore) are killed by the named World cases, not by the random mix at 256 cases: two lock
  failures around a lid close are rare in it.
- **Windows callers unchanged.** `cargo check -p wakeshot-windows --all-targets` exit 0, and its `power::` and `app::`
  tests (182 passed, 10 ignored real-laptop tests) pass with the changed apply and check routines.

## Tray mode checks

Card `p6-int-tray-mode` Part A, 2026-11-04. No tray started, no screen, no power setting.

- **The core gives the first-run notice before the first icon view.** In `core.rs`, `take` handles `Started` and
  pushes `Notify(FirstRun)` (and `WriteConfig`); `settle` runs after it and ends with `view::show`, which pushes the
  first `Tray`. So in the `Started` batch `Notify` comes before `Tray`. The dispatcher pushes `Notify` onto the IO
  queue at its place and runs `Tray` on the loop later in the same batch, so an IO thread that already runs could call
  the shell for the balloon before `NIM_ADD`, and `BalloonNotifier` does not retry (`p5-notices`). Read in the code;
  `tray_mode` therefore starts the IO thread right after `Dispatcher::new`.
- **`LoopWindow::run` destroys its window before it returns** (`window.rs`: `run(self, ..)`, `Drop` calls
  `DestroyWindow`). Microsoft, `WTSRegisterSessionNotification` Remarks (fetched 2026-11-04): "When a window no
  longer requires these notifications, it must call WTSUnRegisterSessionNotification before being destroyed." So the
  unregister cannot wait for step 9 of the shutdown; `tray_mode` does it when the dispatcher returns Quit. The same
  page: `RPC_S_INVALID_BINDING` may come before the Remote Desktop services have started, and they have all started
  once the event `Global\TermSrvReadyEvent` is set. `tray_mode` retries on each safety tick instead of waiting on
  that event (not tested: no early sign-in here).
- **The profile folder is found the way Node finds it.** Rust's `std::env::home_dir` on Windows returns
  `USERPROFILE` when it is set and not empty, else `GetUserProfileDirectory`; before Rust 1.85 it read `HOME` (std
  docs, fetched 2026-11-04; the repo pins 1.99.1, where it is not deprecated: clippy `-D warnings` is clean). Node's
  `os.homedir()` on Windows "uses the USERPROFILE environment variable if defined. Otherwise it uses the path to the
  profile directory of the current user" (Node docs, fetched 2026-11-04). Claude Code 2.1.317 builds its folder from
  `homedir()` (`p4-live-claude`, Log 01:41). Whether the runtime the installed Claude Code runs on follows Node's rule
  here: not verified.

Part B, 2026-11-04 02:58 to 03:01, without the picture: the session was locked (`LogonUI.exe` ran, read 03:01), so
nothing on screen was seen. One run of the smoke test (x64 debug, `--ignored --test-threads=1 --nocapture`), the
dry-run tray's own log printed by the test.

- **The whole dry-run tray runs end to end in a locked session.** The smoke test passed in 4.1 s: the `.cmd` started
  through Explorer ran at Medium (`S-1-16-8192` in its `whoami /groups`), `state.json` came 1.9 s after the launch, a
  script hold with the elevated test's pid gave holding within 0.9 s, its delete gave idle within 1.0 s, `ask_quit`
  ended the tray with exit code 0, and the dry-run journal was gone. So a Medium tray takes an elevated (High) holder as
  alive; whether through its own open or the process list: not verified.
- **`state.json` follows the core about 0.9 s late here, by design.** The log has "Holding starts" 156 ms after the
  start line and "Holding ends" 1 s before the quit, while the test saw each in `state.json` 0.9 to 1.0 s after its
  own write. `core/view.rs` writes `state.json` at most once a second and holds back a change that comes within a
  second of the last write (read in the code).
- **The dry run reads the real plan and writes nothing.** Its log: `would write` lid AC and DC 1 to 0 and sleep DC
  18000 to 0 on Balanced at the hold, the reverse at its end, "RunOnce value not set", and `PowerSetRequest SYSTEM` and
  `EXECUTION` only as dry-run lines. `powercfg /q` read 03:00 after the run: sleep AC and DC `0x4650` (18000), as the
  dry run read. `%LocalAppData%\Wakeshot` does not exist; `Run` and `RunOnce` name no Wakeshot value (read 03:01).
- **The shell keeps a `NotifyIconSettings` entry for a run in a locked session, and no AUMID key without a notice.**
  After the run there was one entry with the dry-run GUID, `ExecutablePath` the build folder's exe, and an
  `IconSnapshot` PNG of 24 by 24 px (its `IHDR`), which fits the 150% taskbar. No
  `NotifyIconGeneratedAumid_<id>` key existed: no balloon was shown (the test writes `config.json`, so no first-run
  notice). Exported, deleted at 03:00:17, still gone at 03:01:26.
- **The quit is quick and the 1400 line comes once.** From the quit's restore to "Wakeshot ends with code 0" took
  3 ms; the IO thread's one `PostMessageW` 1400 line ("as after a quit ... dropped") is Info, once, as the
  orchestrator asked.

## Velopack dependency checks

Checked 2026-11-04 between 03:08 and 03:31 for card `docs/tasks/p6-velopack-dependency.md`, on this laptop, in the
card's own build folder, with no other cargo command running. Rust 1.99.1, the repo's static CRT config.

- **clang is still missing here.** `Get-Command clang`: not found. `vswhere -all -products * -find
  'VC\Tools\Llvm\**\clang.exe'`: no hit (03:09). The one Visual Studio instance is Build Tools 2026 18.11.12324.101 in
  `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`; it already has
  `Microsoft.VisualStudio.Component.VC.Tools.ARM64` (`vswhere -include packages`). Not installed: it needs Haik's
  elevation (steps in the card).
- **Component ids and the command.** Microsoft Learn, "Visual Studio Build Tools component directory", the Visual
  Studio 2026 part, workload "Desktop development with C++" (`Microsoft.VisualStudio.Workload.VCTools`), all optional:
  `Microsoft.VisualStudio.Component.VC.Llvm.Clang` ("C++ Clang Compiler for Windows (20.1.8)"),
  `Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset` ("MSBuild support for LLVM (clang-cl) toolset"),
  `Microsoft.VisualStudio.ComponentGroup.NativeDesktop.Llvm.Clang` ("C++ Clang tools for Windows (20.1.8 - x64/x86)").
  "Use command-line parameters to install Visual Studio": `setup.exe modify --installPath <dir> --add <id>`,
  `--passive` (a progress window, no questions), `--norestart`; the installer must be started from another folder than
  its own; every command needs elevation; exit 0 is done, 3010 done with a reboot wanted. `ring`'s `BUILDING.md`
  (`main`): for `aarch64-pc-windows-msvc`, the Build Tools "C++ ARM64 build tools" and "clang" components, and the
  Visual Studio LLVM folder on the PATH. Read 03:10 to 03:12.
- **`ring` calls clang by its bare name.** `ring` 0.17.14 (the version `velopack` locks) `build.rs` lines 562 to 564:
  on Windows AArch64, when the compiler cc-rs found is not clang-like, it sets the compiler to `clang`. So the PATH of
  the cargo process decides; no user Path change is needed if the cargo command's own PATH has the LLVM folder. Read in
  the cargo registry copy. That `CC_aarch64_pc_windows_msvc` set to a full clang path works too: inferred from the same
  lines (a clang there is clang-like), not tried.
- **The ARM64 error without clang.** Workspace clippy (with and without `--all-targets`) and the release build for
  `aarch64-pc-windows-msvc` stop in 1 to 3 s: `error: failed to run custom build command for ring v0.17.14` ...
  `error occurred in cc-rs: failed to find tool "clang": program not found`. Build scripts run for clippy too, so every
  ARM64 command that compiles `wakeshot-tray` fails the same way until clang is reachable.
- **The CLI and the libs stay free of it.** `cargo tree --locked -p wakeshot-cli -e normal`: 50 lines, no `velopack`.
  `cargo tree -e all --target aarch64-pc-windows-msvc` for `wakeshot-cli`, and for `wakeshot-core` with
  `wakeshot-windows`: no `velopack` and no `ring`. So for ARM64, `cargo clippy -p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli` with and without `--all-targets` (`-D warnings`) and their release build pass without clang (03:24).
- **Lock and package counts.** The one build without `--locked` took `Cargo.lock` from 80 to 181 packages (+101, all
  targets); none removed, no locked version changed, six existing entries only gained a version suffix (now two
  `windows-sys` and two `getrandom`) or an optional dependency (`rand` gains `chacha20`). Compared with a Python
  `tomllib` diff of the two files. `cargo tree -p velopack -e normal --prefix none`, unique lines: 115 for x64 (114
  packages besides `velopack`), 118 for ARM64 (plus `windows-sys` 0.52, `windows-targets` and `windows_aarch64_msvc`
  0.52.6 for `ring`). Research "Contracts build" counted 113 in a scratch project on 2026-11-03; the newer resolve
  (`ureq` 3.5.1 now) likely explains the difference (not verified). The x64 tray build gains 92 packages; 23 of
  velopack's are already in the libs' tree (`windows` 0.62.2 and its crates, `serde`, `serde_json`, `syn`).
- **Sizes and build time.** Release exes in bytes, before and after `velopack`:

  | Exe | x64 before | x64 after | ARM64 before | ARM64 after |
  |---|---|---|---|---|
  | `wakeshot-tray.exe` | 1,640,960 | 2,026,496 (+385,536) | 1,318,400 | not built (no clang) |
  | `wakeshot.exe` | 667,648 | 667,648 | 615,936 | 615,936 |

  Clean release build in a fresh folder: before, the whole workspace for x64 took 55 s (ARM64 43 s); after, the tray
  alone for x64 took 70.4 s with 133 crates compiled.
- **Static CRT holds with `ring`'s C code.** `ci.yml`'s dumpbin step, run on the new x64 release exes: no `VCRUNTIME`,
  `MSVCP`, `ucrtbase` or `api-ms-win-crt-*`. The tray imports `kernel32`, `ntdll`, `oleaut32`, `bcryptprimitives`,
  `advapi32`, `api-ms-win-core-synch-l1-2-0`, `shell32`, `ole32`, `user32`, `combase`, `gdi32`, `dwmapi`, `powrprof`,
  `propsys`, `wtsapi32`, `userenv` and `api-ms-win-shcore-scaling-l1-1-1`. Which of these are new: not compared (the
  before exe was overwritten).
- **velopack 1.2.161 behavior `velo.rs` relies on** (source in the cargo registry: `app.rs`, `manager.rs`,
  `locator.rs`, `sources/github.rs`).
  - `VelopackApp::run()`: when the first argument is `--veloapp-install`, `-updated`, `-obsolete` or `-uninstall`, it
    calls that callback only if the second argument parses as a semver version, then `exit(0)` unless
    `VELOPACK_DEBUG` is set. Otherwise it locates the install; not installed, it logs and returns.
  - `set_auto_apply_on_startup(false)` skips the apply of a staged package in `run()`; `run()` still deletes local
    full packages other than the running version and the newest staged one.
  - `download_updates` returns at once when the target package is already in `packages\` ("Package already exists on
    disk, skipping download"), so a package staged by an earlier run is offered again after the next good check,
    with no second download.
  - `wait_exit_then_apply_updates` uses only the target asset (`AsRef<VelopackAsset>`); a package missing on disk is
    `Error::FileNotFound`.
  - `GithubSource::new` panics only on a URL that does not parse (`expect`); `UpdateManager::new` auto-locates from
    the exe: `Update.exe` beside it, or `<root>\current\` with `Update.exe` in `<root>`. A test exe in `target\` gets
    `NotInstalled` (unit test `velo::tests::a_build_folder_exe_is_not_installed`).
- **The dry-run tray still starts with `velopack` in.** The smoke test (`--test smoke -- --ignored
  --test-threads=1`) passed after the change: 1 passed, test 4.5 s, 35 s with its debug build. `VelopackApp::run()`
  in the not-installed build returned and the tray ran on, held, went idle and quit with code 0.
- **The shell's entry for that run had no GUID.** One new `HKCU\Control Panel\NotifyIconSettings\<id>` key came with
  the run (ids listed before and after), holding only `IconSnapshot`, a 24 px PNG of the idle cup; no `IconGuid` and
  no `ExecutablePath`, still so 1.5 minutes after the run (session locked). The tray mode card's run had both. So a
  clean-up that looks only for the dry-run GUID can miss the entry; comparing the ids before and after finds it. Why
  the values were missing: not verified.

## Update wiring checks

Checked 2026-11-04 03:36 to 03:50 for card `docs/tasks/p6-int-update-wiring.md`. "Source" means read in velopack
1.2.161 in the cargo registry (`~/.cargo/registry/src/index.crates.io-*/velopack-1.2.161/src`), the version
`Cargo.lock` pins; "docs.rs" means the 1.2.161 pages on docs.rs; "test" means run on this laptop.

- **A local folder source exists.** `velopack::sources::FileSource::new<P: AsRef<Path>>(path: P)`: "Retrieves
  available updates from a local or network-attached disk. The directory must contain one or more valid packages, as
  well as a 'releases.{channel}.json' index file." `get_release_feed` reads `<folder>\releases.<channel>.json`;
  `download_release_entry` copies `<folder>\<asset FileName>` to the local file. The channel is the app's own (from
  its manifest) unless `UpdateOptions` names one, as for `GithubSource`. **Verified** (source `sources/file.rs` and
  docs.rs `struct.FileSource.html`). The `p6-velopack-facts` runs already updated through a local `FileSource` (point
  3 of "Velopack checks for Wakeshot").
- **`UpdateManager::new_boxed(Box<dyn UpdateSource>, ..)`** takes a source chosen at run time; `new` boxes its
  argument and calls it. `UpdateSource: Send + Sync`. **Verified** (source `manager.rs`, `sources/mod.rs`).
- **The restarted callback.** `VelopackApp::on_restarted<F: FnOnce(Version) + 'a>(self, hook: F)`: "This hook is
  triggered when the application is restarted by Velopack after installing updates." Inside `run()`: the manager is
  made with `NoneSource` (no install: an error line and `run()` returns before any hook); then
  `env::var(VELOPACK_RESTART).is_ok()`, then `env::remove_var(VELOPACK_RESTART)`; at the end, when restarted, the
  hook gets `manager.get_current_version()`, the manifest version of the running (new) install. So the version must be
  kept inside the callback: after `run()` the variable is gone. Wakeshot keeps it in a `OnceLock<String>`
  (`velo::RESTARTED`). **Verified** (source `app.rs` lines 125 to 203; docs.rs `struct.VelopackApp.html`); that
  Update.exe sets `VELOPACK_RESTART=true` and the hook gets the new version: **verified** earlier by test, point 4 of
  "Velopack checks for Wakeshot".
- **Install check before the thread.** `velopack::locator::auto_locate_app_manifest(LocationContext::FromCurrentExe)`
  is public. On Windows it looks for `Update.exe` beside the exe, then in `<root>` for an exe under `<root>\current\`,
  and `VelopackLocator::new` also needs the manifest; every miss is `Error::NotInstalled`, a manifest that does not
  read is another error. `velo::installed()` treats only `NotInstalled` as no install, so a broken install still runs
  the update thread, whose checks then log the error. A test exe in `target\` is not installed (unit test
  `velo::tests::a_build_folder_exe_gets_no_update_thread_and_no_updated_notice`). **Verified** (source `locator.rs`,
  test).
- **The debug-only feed name is not in a release exe.** `findstr /c:"WAKESHOT_UPDATE_FEED"` on the release
  `wakeshot-tray.exe` (x64): exit 1, not found; on the debug test exe of the tray: found. The name and the code that
  reads it sit under `#[cfg(debug_assertions)]`. **Verified** (test).
- **The release tray grew by the HTTPS stack.** x64 release `wakeshot-tray.exe`: 3,947,520 bytes, against 2,026,496
  measured by `p6-velopack-dependency`. Before this card `velo::feed()` was dead code in the exe, so LTO dropped
  `UpdateManager`, `GithubSource` and the `ureq`, `rustls` and `ring` code under them; now the update thread calls it.
  `findstr` finds `rustls`, `ureq`, `webpki` and `api.github.com` in the exe now. That the whole 1.9 MB is that stack:
  not verified (no size breakdown run).
- **Not verified:** that the tray Update.exe restarts gets the old tray's environment, so a `WAKESHOT_UPDATE_FEED` or
  `WAKESHOT_DATA_DIR` set for the old debug tray reaches the new one. Update.exe is a child of the tray
  (`wait_exit_then_apply_updates` spawns it), so it likely inherits it; `p6-int-update-e2e` should check.
