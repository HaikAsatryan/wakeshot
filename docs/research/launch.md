# Launch research: rivals, winget, Microsoft Store, media

Card `docs/tasks/p6-launch-research.md`, 2026-11-04. Facts for the README, winget, Store and media cards. Marks:
**verified** (read in source, docs, an API answer or a release file list at the link given, on the date given),
**reported** (a third party says so; not checked by us), **not verified** (a guess or still open; says what would settle
it). Times are Yerevan time (UTC+4). Nothing was installed or run; release sizes come from the GitHub and NuGet APIs.

## 1. Rivals for the README table

### Candidates

Stars and dates read with `gh api repos/<owner>/<repo>` on 2026-11-04 at 00:45 to 00:55. New candidates came from
`gh search repos` (queries: keep awake, stay awake, prevent sleep, caffeinate windows, awake tray, keep awake
claude, keep awake codex, lid closed, ai agents awake windows, lid windows agents; topics `keep-awake`,
`prevent-sleep`, `caffeine`). Only tools that run on Windows are listed; the many macOS tools are left out.

| Tool | Kind | Stars | Created | Last push | Release on Windows | Notes |
|---|---|---|---|---|---|---|
| PowerToys Awake (`microsoft/PowerToys`) | general keep awake | 140,491 (all of PowerToys) | 2019-05-01 | 2026-11-03 | v0.102.3001, 2026-10-27 | Microsoft, the one most Windows users have |
| `arkane-systems/mousejiggler` | fakes mouse input | 1,459 | 2014-04-24 | 2026-10-26 | yes | a jiggler, not a power request: defeats screen lock. Different class, left out |
| `CHerSun/NoSleep` | prevents screen lock | 276 | 2016-07-13 | 2026-10-28 | | screen-lock tool, left out |
| `dmnd/Caffeinated` | general keep awake | 153 | 2011-09-06 | 2020-11-21 | | not maintained since 2020 |
| `airtaxi/LidGuard` | agent aware | 13 | 2026-04-29 | 2026-09-28 | NuGet `lidguard` 2.1.7, 2026-09-28 | Windows, Linux, macOS |
| `itsmbaqer/agents-dont-sleep` (Agents Don't Sleep) | agent aware | 10 | 2026-09-23 | 2026-09-28 | v0.4.0, 2026-09-29 | macOS, Windows, Linux; new since the 2026-11-03 reports |
| `Luchioxy/LidKeeper` | agent aware | 10 | 2026-06-01 | 2026-06-01 | none (a PowerShell script piped from the web) | one day of commits, process-name polling; left out |
| `leikauskas/nightcrew` | agent aware | 9 | 2026-10-13 | 2026-10-23 | v0.3.1, 2026-10-18 | Windows only |
| `stanley-projects/Insomnia` | agent aware | 7 | 2026-03-24 | 2026-09-07 | v1.4.3, 2026-05-28 | Electron |
| `yansslo/nosleep`, `peterintech/nosleepp`, `KujalaLucien/vibe-coding-always-on` | agent aware | 4, 3, 3 | 2026 | 2026 | | CLIs or small trays, left out |

All verified (gh, 2026-11-04). Stars move daily; `p6-readme` reads them again on the day it writes the table.

### Pick: three rivals

1. **PowerToys Awake**: the most-starred general keep-awake tool on Windows, and the one readers already have. Its
   140,491 stars are for all of PowerToys, and the README must say so.
2. **LidGuard** (13 stars): the most-starred agent-aware tool that runs on Windows.
3. **Agents Don't Sleep** (10 stars): the next agent-aware tool with a real Windows release, and the one closest to
   Wakeshot's own pitch ("awake only while your agents are working", lid closed, settings put back). LidKeeper has the
   same 10 stars but no release and no tray; it is a script.

Reserve: **Nightcrew** (9 stars). It is the closest in design (Windows only, no hooks, puts settings back) and the one
Haik used, but it trails by one star. Rule for `p6-readme`: PowerToys Awake plus the two most-starred agent-aware tools
that run on Windows and ship a release, read on the day the README is written; on a tie, the one with the newer
release. Decided by the worker, for Haik's review.

### Facts per rival

Each line is verified at the link on 2026-11-04 unless marked.

**PowerToys Awake**

- Stack: C#, .NET 10, self-contained (`CoreTargetFramework` `net10.0` in
  https://github.com/microsoft/PowerToys/blob/main/src/Common.Dotnet.props; `SelfContained` true in
  `src/Common.SelfContained.props`; Awake imports both in `src/modules/awake/Awake/Awake.csproj`). PowerToys needs
  the WebView2 runtime, which its setup installs ("Latest stable version of Microsoft Edge WebView2 Runtime is
  installed via the bootstrapper during setup", https://learn.microsoft.com/en-us/windows/powertoys/install).
- Download: only as part of PowerToys. v0.102.3001: `PowerToysUserSetup-0.102.3001.0-x64.exe` 323,884,640 bytes
  (323.9 MB), arm64 311.6 MB; machine-wide setups about the same (`gh api repos/microsoft/PowerToys/releases/latest`).
  Install size: not measured (no install).
- Idle RAM and CPU: not published for the shipped app. A maintainer's Rust rewrite PR (closed, not merged) measured
  the C# Awake at 14.3 MB private memory, 130 ms startup, 15 threads, 114.9 M CPU cycles per 10 s during a timed
  countdown ("Release x64, same harness", https://github.com/microsoft/PowerToys/pull/51181). That is a published
  number, but for a test harness, not a user's PC.
- How it works: "doesn't modify any of the Windows power plan settings"; "spawns background threads that tell
  Windows that they require a specific state of the machine" (https://learn.microsoft.com/en-us/windows/powertoys/awake,
  updated 2026-10-21).
- Agents: none. Modes off, indefinite, for a time, until a time; Keep screen on; CLI `--time-limit`, `--expire-at`,
  `--display-on`, `--pid` and `--use-parent-pid` (ends with a process). Same page.
- Lid closed: no. Settings written: none. Crash safety: nothing to put back (the requests end with the process).
  Battery safety: none on the page. Same page.
- Updates: "PowerToys uses an automatic update checker ... a toast notification will appear when an update is
  available" (install page). Install: "Per User" to `%userprofile%\AppData\Local\Programs` or "Machine wide" to
  `%ProgramFiles%` (install page). Also in the Store and winget (`Microsoft.PowerToys`).
- License MIT, free (`gh api`).

**LidGuard** (`airtaxi/LidGuard`, HEAD `3ff7893`, 2026-09-28, the same commit `rivals-nightcrew-lidguard.md` read)

- Stack: C#, ".NET 10 NativeAOT global tool distribution for Windows, systemd/logind Linux, and macOS" (README,
  https://github.com/airtaxi/LidGuard). Installed with `dotnet tool install --global lidguard` (README), and .NET
  tools need the SDK ("This article applies to: .NET Core 2.1 SDK and later versions",
  https://learn.microsoft.com/en-us/dotnet/core/tools/global-tools). Per user: "Tool access is user-specific" (same
  page).
- Download: NuGet `lidguard` 2.1.7 is a 20 KB pointer package; the native packages are `lidguard.win-x64` 25,272,341
  bytes (25.3 MB) and `lidguard.win-arm64` 24,507,348 bytes (`curl` of `api.nuget.org/v3-flatcontainer/...`). Install
  size: not measured.
- Idle RAM and CPU: not published (README read in full).
- Agents: "Codex, Claude Code, GitHub Copilot CLI, and OpenCode" through hooks and plugins it installs in each agent's
  config, plus an MCP route for others. "Currently officially tested with Codex on Windows" (README).
- Lid closed: yes, optional; it sets the lid action to Do nothing on AC and DC while a session holds and puts it back
  (README "Lid-close handling"; code in `rivals-nightcrew-lidguard.md` 3.1). It does not touch the sleep timer (same
  report).
- Crash safety: restores only at the next `lidguard` command or hook (same report, 3.2 "Killed process").
- Battery safety: no battery floor (same report, 4). Optional "Emergency Hibernation" on high temperature (README).
  Never locks the PC (same report, 4).
- Updates: by hand, `dotnet tool update --global lidguard` (the .NET tools page). License MIT, free.
- Its README opens with a prompt for the user's AI agent to install it ("Read https://raw.githubusercontent.com/...
  agent-install.md and install LidGuard for this machine"). That is text for its users' agents; it was read as data
  and nothing was run.

**Agents Don't Sleep** (`itsmbaqer/agents-dont-sleep`, HEAD `f5fd5aa`, 2026-09-28)

- Stack: Rust with Tauri 2 (`tauri = { version = "2", ... }` in `src-tauri/Cargo.toml`); the UI is web code
  (`index.html`, `vite.config.ts`, `package.json` at the root), shown by WebView2 on Windows (Tauri's runtime; not
  checked on a PC). NSIS installer with `"installMode": "currentUser"` (`src-tauri/tauri.conf.json`).
- Download: v0.4.0 `Agents.Dont.Sleep_0.4.0_x64-setup.exe` 1,897,125 bytes (1.9 MB). No Windows ARM64 build in the
  release. Install size: not measured.
- Idle RAM and CPU: not published (README read in full).
- Agents: Claude Code, Codex, Gemini CLI, Cursor, Copilot CLI, OpenCode, Pi, Hermes through hooks written into each
  agent's config ("Connect adds a small hook to that agent's own config"); aider, goose and others by process name.
  "Only Claude Code has been tested end to end so far" (README,
  https://github.com/itsmbaqer/agents-dont-sleep).
- Lid closed: yes. Windows: "The power plan's lid action is set to "Do nothing" and battery idle sleep is turned off,
  then restored"; originals saved to `%USERPROFILE%\.agents-dont-sleep\restore.json` (README "How it works";
  `SECURITY.md`).
- Crash safety: "A separate watchdog process restores the settings if the app is killed. The app itself also
  restores them the next time it starts." (`SECURITY.md`).
- Battery safety: "Stop below N% battery", "it stops at your battery limit (15% by default)"; heat limit "Not
  available" on Windows; lock on lid close: `LockWorkStation`, on by default ("Safe by default. Battery cut-off, heat
  limit, and screen lock when you close the lid."). README.
- Updates: no updater found (`src-tauri/Cargo.toml` has no `tauri-plugin-updater`; the README says to download from
  Releases). Not verified that no other update path exists.
- Network: "no telemetry. The only requests are the usage-limit reads, every 5 minutes, to Anthropic's and OpenAI's
  usage endpoints." (README). License MIT, free; unsigned ("The builds aren't code-signed yet").

**Nightcrew** (reserve; `leikauskas/nightcrew`, HEAD `9b4fd7d`, 2026-10-23)

- C#, .NET Framework 4.8, WPF (`docs/research.md`, "Nightcrew review"). v0.3.1 `nightcrew.exe` 1,758,720 bytes (1.8 MB),
  portable, no installer. README: "It is one 1.7 MB exe in the tray. It uses less than 1% of one CPU core." (a
  published claim, method not given).
- Claude Code and Codex, no hooks; lid closed yes; writes lid action and sleep timers and puts them back; watchdog task;
  battery floor 20%; locks at lid close (`rivals-nightcrew-lidguard.md` sections 1 to 4). Update by hand: quit, replace
  the exe, start (README "Update"). MIT, free.

### Wakeshot's row

From `docs/product.md` and `docs/architecture.md` (decisions, not yet measured on the shipped app):

- Stack: Rust with `windows-rs`, no runtime; native x64 and ARM64; Windows 11 only (product.md, "Platform and
  stack").
- Size: **not measured (p6-media measures them)** (orchestrator, 2026-11-04 00:41: the release build was skipped
  here). An early hint only: a Velopack `Setup.exe` was 10.6 MB (x64) and 8.6 MB (ARM64) around a small test build,
  most of it Velopack's own `Update.exe` (`docs/research.md`, "Velopack checks for Wakeshot", point 9). So Wakeshot's
  download will likely be larger than Agents Don't Sleep's 1.9 MB setup and far smaller than PowerToys.
- Agents: Claude Code in v1, from its status files and transcripts, with no hooks and no edits to Claude's settings;
  `wakeshot run -- <command>`; hold files any script can write; Keep awake from the menu or CLI (product.md, "What
  keeps the PC awake").
- Lid closed: yes, while something holds; writes the lid action and battery sleep (Never) at hold start and puts them
  back at the end; the user's own later change wins (product.md, "Power behavior", "User settings safety").
- Crash safety: a guard process restores if the tray dies; the next start and the next sign-in restore; the gap after
  an OS crash or power cut is accepted and the README says so (product.md, "User settings safety").
- Battery: steps back at Windows' energy saver level (20% when that is Never or Always) and hibernates a closed laptop
  after 5 minutes with no input. Locks at lid close while holding. No heat guard (product.md, "Power behavior").
- Updates: automatic from GitHub Releases, applied only while nothing holds; the network is two GitHub API calls a day;
  no telemetry (product.md, "Distribution", "UX"; `phases/06-ship.md`, "Updates").
- Install: per user, no admin (Velopack). MIT, free.

### Speed numbers: how `p6-media` measures each

All on the installed release build, x64, on the test laptop, with the build version and date written next to each
number. ARM64: "not measured" unless a device is at hand.

| Number | Method |
|---|---|
| Idle RAM | `Get-Process -Id <pid>` every 10 s for 10 minutes with nothing holding: `WorkingSet64` and `PrivateMemorySize64`, median and max, for `wakeshot-tray.exe`. The guard runs only while settings are overridden, so its numbers come from the hold run. |
| Idle CPU | `TotalProcessorTime` of the tray at the start and end of the same 10 minutes; report CPU milliseconds per 10 minutes and the same as a percent of one core. |
| CPU and RAM during a hold | the same two methods for 10 minutes while a hold runs (`wakeshot run -- ping -n 600 127.0.0.1`, so the load is the same every run), for the tray and the guard. |
| Exe and install size | file length of `wakeshot-tray.exe` and `wakeshot.exe` in the release, of `Setup.exe`, and the sum of all files under `%LocalAppData%\HaikAsatryan.Wakeshot` after a silent install. |
| Start delay | Claude Code start of work: `statusUpdatedAt` (ms since epoch) of the session file write that sets `busy`; Wakeshot's log line for the hold start. Report the median and max of 5 runs. |
| End delay | `statusUpdatedAt` of the write that sets `idle`, against the log line for the hold end. Same 5 runs. |

The rivals publish almost none of these. The README may set Wakeshot's numbers next to the PowerToys PR's 14.3 MB
private memory only with the note that it was a different harness, and next to Nightcrew's "less than 1% of one CPU
core" only as their claim.

### Draft table for `p6-readme`

`p6-readme` sets the final words; every rival cell keeps its link and the date read.

| | Wakeshot | PowerToys Awake | LidGuard | Agents Don't Sleep |
|---|---|---|---|---|
| Stars (2026-11-04) | new | 140,491, for all of PowerToys | 13 | 10 |
| Built with | Rust, no runtime | C# .NET 10, part of PowerToys | C# .NET 10 NativeAOT, a .NET tool | Rust and Tauri (web UI in WebView2) |
| Download (x64) | from `p6-media` | 323.9 MB (all of PowerToys) | 25.3 MB, needs the .NET SDK | 1.9 MB |
| ARM64 build | yes | yes | yes | no |
| Idle RAM and CPU | from `p6-media` | not published (14.3 MB private memory in a test harness) | not published | not published |
| Knows when an agent works | Claude Code; any command with `wakeshot run` | no (timer or a process id) | Codex, Claude Code, Copilot CLI, OpenCode (tested: Codex) | 8 agents (tested: Claude Code) |
| Changes your agent's config | no | no | yes, adds hooks | yes, adds hooks |
| Keeps working with the lid closed | yes | no | yes | yes |
| Puts your power settings back | yes, after each hold, at quit and after a crash | changes none | yes, after a crash only at its next run | yes, also after a crash |
| Battery floor | Windows' energy saver level | no | no | 15% by default |
| Locks a closed laptop | yes | no | no | yes, by default |
| Updates itself | yes, only while nothing holds | update checker | no | no |
| Install | per user, no admin | per user or machine | per user | per user, no admin |
| License, price | MIT, free | MIT, free | MIT, free | MIT, free |

### What this means for Wakeshot

- The agent-aware field on Windows is tiny (10 to 13 stars) and young. The comparison readers care about is
  PowerToys Awake, which they likely have; the agent-aware rivals show that Wakeshot is not alone, and where it is
  safer.
- Wakeshot's honest lead: no hooks or config edits in the agent; settings put back with the user's change winning; a
  battery floor and a lock; it updates itself only between holds; native and (to be measured) very light.
- Wakeshot's honest gap: v1 knows only Claude Code, while the rivals list 4 to 8 agents (each tested end to end on
  only one). The README should say "Claude Code today, Codex and Copilot CLI next" and not hide it.
- Download size is not where Wakeshot wins: Velopack's `Update.exe` alone is about 5 MB. Lead with RAM, CPU and the
  start and end delays, not with the download.
- Agents Don't Sleep is the rival to watch: same pitch, cross-platform, battery floor and lock. Its README is a good
  model (see part 4).

## 2. winget with an app that updates itself

### How winget decides an upgrade is available

- winget reads the installed version from Apps and Features (ARP `DisplayVersion`) and offers an upgrade when a
  manifest version in the source is higher. A winget PM, 2026-07-28: "WinGet reads the installed version from ARP, so
  it still sees the old version (84.0.2) and keeps offering the upgrade even though the running app is newer." and
  "the application's own updater needs to write the correct `DisplayVersion` to ARP when it self-updates"
  (https://github.com/microsoft/winget-cli/issues/1239#issuecomment-5099257559). Verified.
- In the client code, each available version is checked with `IsUpdateVersionAvailable(installedVersion, ...)`; a
  package whose installed version is unknown is skipped unless `--include-unknown`
  (`src/AppInstallerCLICore/Workflows/UpdateFlow.cpp`, main branch, read 2026-11-04). Verified.
- Matching an installed program to a package uses `ProductCode` and `AppsAndFeaturesEntries` (`DisplayName`,
  `Publisher`, `DisplayVersion`, `ProductCode`): "used to match installed packages with manifests in configured
  sources" (`doc/manifest/schema/1.28.0/installer.md` in `microsoft/winget-pkgs`). Verified.
- Velopack rewrites `DisplayVersion` at every self-update and every Setup run (`docs/research.md`, "Velopack checks
  for Wakeshot", point 1, tested). So the Firefox and Discord problem above, an updater that leaves ARP stale, does not
  happen to Wakeshot.

### The VS Code and Docker Desktop cases

- **VS Code, upgrade loop** (2026-10-15, open): the winget-pkgs manifest 1.142.0 pointed at the 1.141.2 installers.
  "Because the installed product reports 1.141.2 while the index advertises 1.142.0, winget upgrade never considers
  the package up to date and re-installs it on every run" (https://github.com/microsoft/winget-pkgs/issues/453148).
  The volunteer who submitted it: "The links weren't updated on time in the vscode server so, I got the previous
  version's links." Still open after 3 weeks. Verified.
- **VS Code, stale catalog** (2026-07-22, closed 07-27): "the public WinGet source only advertises version 1.126.0"
  while VS Code was at 1.129.1, so `winget upgrade` could not reach the latest
  (https://github.com/microsoft/winget-pkgs/issues/406071). Verified.
- **Docker Desktop, same version twice** (2026-06-17, closed): 4.78.0 was "published twice", as `4.78.0` and as
  `4.78.0.229452`; the installed app reported 4.78.0, so winget kept offering 4.78.0.229452 and the installer failed
  with exit code 3 (https://github.com/microsoft/winget-pkgs/issues/389278). Verified.
- **Firefox and Discord, stale ARP after a self-update**: issue #1239 above. Verified.
- None of these are winget bugs: each is manifest data or an app's ARP data. All three are in our hands for Wakeshot.

### Manifest fields that help

Read in the 1.28.0 schema docs (`microsoft/winget-pkgs/doc/manifest/schema/1.28.0/installer.md`, the newest folder
there) and the client's `latest` schema, which is `manifest.installer.1.31.0` (`microsoft/winget-cli`,
`schemas/JSON/manifests/latest/`). The current PowerToys manifest uses `ManifestVersion: 1.12.0`. Which version
winget-pkgs validation accepts today: `p6-winget-manifest` checks it. All read 2026-11-04.

| Field | What the docs say | Use for Wakeshot |
|---|---|---|
| `RequireExplicitUpgrade` | "This key identifies packages that upgrade themselves. By default, they are excluded from `winget upgrade --all`." | `true`. Users then see "The following packages have an upgrade available, but require explicit targeting for upgrade" (winget-cli issue #5879); `winget upgrade HaikAsatryan.Wakeshot` still works |
| `UpgradeBehavior` | `install`, `uninstallPrevious`, `deny`; "If the package should not be upgraded through WinGet, the "deny" value should be specified." | `install` (Setup over an install is Velopack's own path). Not `deny`: it blocks an explicit upgrade too |
| `ProductCode` | "used to help correlate installed packages with manifests" | `HaikAsatryan.Wakeshot`, the uninstall key name |
| `AppsAndFeaturesEntries` | `DisplayName`, `Publisher`, `DisplayVersion`, `ProductCode`; "When the PackageVersion and DisplayVersion are the same, the DisplayVersion should be omitted" | `Wakeshot`, `Haik Asatryan`, `HaikAsatryan.Wakeshot`; no `DisplayVersion` |
| `DisplayInstallWarnings` | "not implemented in the Windows Package Manager 1.28 client" | leave out |
| `ReleaseDate` | the release date, `YYYY-MM-DD` | from the GitHub release |
| `ExpectedReturnCodes` | maps installer exit codes to `packageInUse`, `installInProgress` and others | none known for Velopack's Setup yet; leave out |

A catch in the client: `RequireExplicitUpgrade` is stored when winget itself installs the package
(`PackageTrackingCatalog.cpp`: `if (installer.RequireExplicitUpgrade) ... PinnedByManifest`), and the code comment in
`UpdateFlow.cpp` says it has "the downside of not working with packages installed from another source". So a user who
installed `Setup.exe` from GitHub and later runs `winget upgrade --all` gets Wakeshot upgraded by winget. Verified in
source; not tested. Whether the mark survives a Velopack self-update that changes `DisplayVersion`: not verified
(Phase 8, with a local manifest).

### Setup.exe while Velopack updates, or while the app runs

Read in Velopack at tag `1.2.161` (the pinned version), 2026-11-04:

- Velopack has one lock: a file `packages\.velopack_lock` in the install folder, opened with no sharing
  (`share_mode(0)`), non-blocking (`src/lib-rust/src/lockfile.rs`, `locator.rs` `try_get_exclusive_lock`). It is taken
  by `Update.exe apply` (after its elevation check), by the app's `download_updates`, by `Update.exe start` for its
  rename step, and by Setup only after it has prepared the new install folder (`apply_windows_impl.rs`,
  `manager.rs`, `start_windows_impl.rs`, `install.rs`). A second taker retries 4 times, 1 s apart, then fails
  (`retry_io` in `shared/util_common.rs`). There is no named mutex. Verified (source).
- Setup over an install first calls `force_stop_package` on the install folder: it kills every process whose exe is in
  that folder, except itself (`shared/util_windows.rs`). Setup runs from the download folder, so this kills the tray,
  the guard and a running `Update.exe` too. Then it renames the old folder for rollback, empties and refills it, and
  only then takes the lock (`install.rs`). Verified (source); the kill of a running tray is also tested
  (`docs/research.md`, "Velopack checks for Wakeshot", point 7).
- So **Setup always wins**. If `winget upgrade` runs while `Update.exe` applies an update, Setup kills `Update.exe`
  mid-apply and replaces the whole folder, so a half-done swap of `current` is thrown away with it. The result is one
  clean install of the winget version. If that version is older than the one Velopack was applying, the app updates
  again at its next check. Inferred from the source; not tested.
- What can still break: a Setup that is itself killed midway leaves `%LocalAppData%\HaikAsatryan.<16 letters>` behind
  (point 1 in the same research section; not tested), and a Setup that cannot kill an elevated `Update.exe` stops with
  "StopApplicationFailed" (`install.rs`; Wakeshot never runs elevated in normal use). Not tested.
- Power settings stay safe in every order: Setup runs no uninstall or obsolete hook, only the install hook, and that
  restores when the journal's owner is gone (`phases/06-ship.md`, "Hooks").
- Two Setups at once (two winget runs, or winget and a Store install): not verified. What settles it: a Phase 8 test.

### Lag from our release to winget, and opening the PR from CI

- Merge time of winget-pkgs PRs, measured 2026-11-04 with `gh api search/issues` (up to 100 merged PRs per day on 7
  days from 2026-10-14 to 2026-10-26, created to `merged_at` in hours; a throwaway script): "New version" PRs, n=700,
  median 3.9 h, 75th percentile 20.1 h, 90th 51.8 h, max 18 days; "New package" PRs, n=534, median 1.8 h, 90th 33.3 h.
  Only merged PRs count, so the real wait is longer. The time from merge until clients see the new index: not
  verified.
- Komac 2.17.1 (2026-10-15) and `wingetcreate` 1.12.13.0 (2026-07-23) are both active; `vedantmgoyal9/winget-releaser`
  (an Action that runs Komac) last released v2 on 2025-01-27 but was pushed 2026-10-13 (`gh api`).
- Both need a classic personal access token with `public_repo`: Komac "needs a classic GitHub token ... with the
  `public_repo` scope" and fine-grained tokens fail to open the PR (Komac README); `wingetcreate`: "Fine-grained tokens
  are not supported" (`doc/token.md`). winget-releaser also wants `workflow` scope for fork syncing and needs a fork of
  winget-pkgs and at least one version already in winget-pkgs (its README). Verified.
- `wingetcreate submit --prtitle <title> --token <token> <PathToManifest>` submits a manifest folder we wrote
  ourselves (`doc/submit.md`). Verified (docs).

### Rules for `p6-winget-manifest` and `release.yml`

One updater at a time: Velopack updates Wakeshot; winget installs it and upgrades it only when the user names it.

1. `RequireExplicitUpgrade: true`, `UpgradeBehavior: install`, `Scope: user`, `InstallerType: exe`, silent switch
   `--silent`, one installer per CPU.
2. `PackageVersion` = the Velopack pack version = the tag without `v` = the `DisplayVersion` Velopack writes. Three
   numbers only, never a second spelling of the same version (Docker case). No `DisplayVersion` in
   `AppsAndFeaturesEntries`.
3. `InstallerUrl` and `InstallerSha256` come from the same tag's published release files, never from a guess (VS Code
   case). `fill.ps1` downloads the files of that tag and hashes them.
4. `ProductCode: HaikAsatryan.Wakeshot` at the installer level and in `AppsAndFeaturesEntries` with `DisplayName:
   Wakeshot`, `Publisher: Haik Asatryan`, so a GitHub install is matched too.
5. `release.yml`: the smoke test reads `DisplayVersion` after the silent install and fails the release when it is not
   the tag's version.
6. Only stable releases go to winget; never a pre-release.
7. The winget step runs as its own job after the GitHub release is published (assets are public only then), with
   `wingetcreate submit` on the folder `fill.ps1` wrote, so every field is ours. The token is a secret used only by
   that job. Runner-up: winget-releaser (Komac); change to it when keeping `fill.ps1` costs more than it saves.
8. The first submission (a new package) stays a Phase 8 hand step; CI opens PRs only for later versions.
9. README: winget users get updates from Wakeshot itself; `winget upgrade --all` skips Wakeshot; `winget upgrade
   HaikAsatryan.Wakeshot` works but ends a running `wakeshot run` wrapper and leaves Wakeshot off until the next
   sign-in (already decided). Users who installed from GitHub and want `--all` to skip it can run `winget pin add
   HaikAsatryan.Wakeshot` (pinning docs; not tested on Wakeshot).

### What this means for Wakeshot

- Velopack keeps ARP right, so the stale-version loop that hits Firefox cannot happen; the loops seen in 2026 came from
  manifest data, which rules 2, 3 and 5 close.
- `RequireExplicitUpgrade` keeps winget out of routine `--all` runs, which matters because a winget upgrade kills the
  tray during a hold and the PC may then sleep while the agent works. It does not cover GitHub installs; the README
  says how to pin.
- The winget catalog lags our release by hours to days. That does not matter for updates, because Velopack updates
  every install within about 12 hours, whatever installed it.

## 3. Microsoft Store

Read 2026-11-04 by a research helper of this card (its notes were temporary; the quotes that matter are below); the
four facts the answer turns on were read again by the worker the same day: the account fee, policy 10.2.9, the
Artifact Signing countries and winget-cli #4347. One read-only command ran on this PC: `winget show --id XP89DCGQ3K6VLD
-s msstore` (the Store's PowerToys listing; nothing installed).

### Two paths for a Win32 app

- "MSIX is the recommended format, but traditional EXE/MSI installers are also accepted."
  (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements). Verified.
- **EXE path** (policy 10.2.9, Store Policies 7.20, updated 2026-09-15,
  https://learn.microsoft.com/en-us/windows/apps/publish/store-policies): "The installer binary may only be an .msi or
  .exe"; "The binary and all of its Portable Executable (PE) files must be digitally signed with a code signing
  certificate that chains up to a certificate issued by a Certificate Authority (CA) that is part of the Microsoft
  Trusted Root Program"; "You must submit a versioned download URL ... The binary associated with that URL must not
  change after submission"; "silent install is required, however a User Account Control (UAC) dialog is allowed"; "a
  standalone installer and is not a downloader stub/web installer". Verified (worker read).
  - The Store runs our installer with our switch: "The Store will need to run your installer in silent mode ... you
    need to provide the required switches"
    (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/upload-app-packages). And it keeps
    its own copy: "The Store will retain copies of your most recent app packages" (same page). `winget show` of the
    PowerToys Store listing gave an installer URL on `cdn.storeedgefd.dsx.mp.microsoft.com/.../cachedpackages/`.
    Verified.
  - Self-update is allowed: "The Store allows existing users to install in-app updates through your app, if your
    installer supports it. The Store does not provide these updates automatically or manually to existing users."
    (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/publish-update-to-your-app-on-store).
    The "installed and updated only through the Store" rule (10.2.5) "does not apply to products that are subject to
    the requirements in 10.2.9". Verified.
  - Uninstall: certification requires an Add or Remove Programs entry and a clean uninstall
    (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-certification-process); no doc
    says who runs the uninstaller, and there is no uninstall-switch field. Settings and winget run the app's own
    `UninstallString` from that entry (winget-cli `ARPHelper.cpp`: "Pick up UninstallString and QuietUninstallString
    for uninstall"), which for Wakeshot is Velopack's `Update.exe --uninstall`, so the uninstall hook runs. Not
    verified for a Store install; settles with one test install from a hidden ("available through link") listing.
  - Changing settings (10.2.8): "You are required to use supported methods and must obtain user consent to change any
    user's Windows settings". Power writes through the documented `PowerWrite*` API are a supported method; consent is
    the user turning on Automatic or Keep awake (worker's reading, not checked with Microsoft).
  - x64 and ARM64 can sit in one listing (one package entry per CPU). A GitHub Releases URL redirects to
    `release-assets.githubusercontent.com`; whether the Store accepts that as a "direct link": not verified (settles
    at the first package validation).
- **MSIX path**: "Microsoft re-signs the package after certification and you don't need to purchase or manage a
  certificate" (https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options). But at
  uninstall "all files and folders ... are removed, as well as any redirected writes to AppData or the registry"
  (https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes), and no doc offers code
  at uninstall; HKCU writes are virtualized into a private hive by default
  (https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization), so the Run, RunOnce and Path
  entries would have to become a StartupTask and an AppExecutionAlias, and Velopack would be gone. Verified (docs);
  that no uninstall hook exists is the absence of one in the docs, not a sentence that says so.

### Haik's three conditions

| Condition | EXE path | MSIX path |
|---|---|---|
| Account free | Yes. "With the new onboarding experience, there are **no registration fees** for either account type" (https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account, updated 2026-10-29). Individual accounts verify a government ID and a selfie. Armenia was on the last published country list (2025-10-01); today's pages give no list (not verified for 2026) | Same |
| Signing free | **No, not today.** Every PE file must be signed by a Trusted Root CA; Microsoft does not sign EXE installers. Azure Artifact Signing: "Individual developers must be located in the United States or Canada", organizations only in listed countries without Armenia (https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart, updated 2026-10-08), and $9.99 a month. SignPath Foundation is free but declines projects without users (`distribution.md` section 3); whether the Store accepts its certificate (publisher "SignPath Foundation") is not verified, and no rule asks the signer to match the account name. Certum Open Source costs about EUR 25 to 69 a year (shop.certum.eu, reported by the helper; checkout price not verified) | Yes: the Store signs |
| Submission without hands | Mostly. The first submission is by hand ("You cannot use the Microsoft Store submission API to create an app in Partner Center", https://learn.microsoft.com/en-us/windows/apps/publish/store-submission-api). After that the MSI/EXE submission API or `msstore` CLI (GitHub Action `microsoft/microsoft-store-apppublisher` v1.4, 2026-07-21; CLI v0.4.4, 2026-11-02) can set the new package URLs and submit, with four repo secrets (`AZURE_AD_TENANT_ID`, `AZURE_AD_APPLICATION_CLIENT_ID`, `AZURE_AD_APPLICATION_SECRET`, `SELLER_ID`; the secret lasts at most 24 months). Every version goes through certification, "typically ... up to three business days", and a submit "fails ... if an active submission exists" | Yes, the same tools |
| Uninstall restores | Very likely yes (the uninstall runs Velopack's hook through the ARP entry); not tested | **No.** No code runs at uninstall, and power plan values written through the power API stay (not tested) |

### winget and the Store side by side

- A Store EXE app shows in winget's `msstore` source with an id that starts with `XP`, type `exe`, and Microsoft's
  cached copy as the URL (the `winget show` above). Verified.
- The same install can match both sources, and winget may pick the Store one: "When upgrade and uninstall commands
  are used with <query> parameter which ambiguously matches the package from both winget and msstore package sources,
  msstore takes precedence, even if the package was installed from winget source." The reporter's Todoist went to the
  Store build 8.18.0 while winget had 9.1.1, and then saw "No available upgrade found". A winget PM: "we've still got
  some work to do on the matching logic" (https://github.com/microsoft/winget-cli/issues/4347, open since
  2024-04-04). Verified. Related open: #5345, #6435.
- Because each Store version waits up to three business days for certification, the Store copy will often be older
  than the GitHub release and the winget manifest. So a Store listing could make `winget upgrade` move a Wakeshot
  user to an older build, which Velopack then updates again: two updaters fighting, the thing Haik wants to avoid.
  Inferred from #4347; not tested.
- The wait Haik saw: "Waiting for another install/uninstall to complete..." is winget's own lock, a named mutex
  `WinGetCrossProcessInstallLock`, "to prevent multiple winget related processes from attempting to install (or
  uninstall) at the same time"; MSIX installs skip it ("MSIX installs are safe to run in parallel"), EXE installs take
  it (winget-cli source, read by the helper). So when the Store app and `winget upgrade --all` both install through
  winget's code, one waits for the other; it is not MSI's `_MSIExecute` mutex. That the Store's own Win32 installs go
  through winget's code is reported (a Microsoft manager quoted by Windows Latest, 2024), not verified.

### Listing needs (for later)

- Screenshots: 1 to 10 (4 or more recommended), PNG up to 50 MB, desktop 1366 x 768 or larger, up to 3840 x 2160.
  Logos: 1:1 box art required (1080 x 1080 or 2160 x 2160), 2:3 poster recommended (720 x 1080 or 1440 x 2160).
  Sizes from the MSIX listing page
  (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images); the MSI/EXE
  page gives none. Read by the helper; not read again by the worker.
- Trailer: MP4 or MOV, 1920 x 1080, "60 seconds or less" recommended, with a 1920 x 1080 PNG thumbnail; the MSI/EXE
  docs and API say nothing of trailers, so whether an EXE listing can show one is not verified.
- Text: description up to 10,000 characters (required); short description 1,000 (first 270 shown in some views);
  what's new 1,500; up to 20 features of 200 characters; up to 7 keywords
  (https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/create-app-submission).
- Age rating: the IARC questionnaire, on the first (hand) submission.
- Privacy policy: policy 10.5.1 says Win32 products "must always have privacy policies"; the form takes a URL or the
  text itself, so a Pages page or pasted text both work.

### Recommendation

**No Store at launch. Revisit once a free signing certificate is in hand (SignPath Foundation after Wakeshot has
users), then on the EXE path only, never MSIX.** Reasons, in order:

1. Signing is not free for Haik today, and an unsigned EXE is never accepted (10.2.9). This fails his first condition
   outright.
2. MSIX would be free and automated but fails "uninstall must still restore", and would also drop Velopack and change
   how Wakeshot starts at login and puts itself on the Path.
3. Even when signing is free, a Store EXE listing adds a second, slower copy of every version that winget can match
   (#4347). Before going in, a test must show that `winget upgrade --all` on a PC with both entries keeps the newest
   build.

If Haik wants the Store sooner, the cheapest real path is a Certum Open Source certificate (about EUR 25 to 69 a year,
not verified at checkout; Store acceptance not verified): it would also sign the GitHub release, though SmartScreen
reputation for a new certificate still has to build up (not verified how fast). That is money, so it is his call.
Haik's answer (2026-11-04 01:14): no; free paths only for signing and distribution. So the Store waits for SignPath
Foundation.

### What this means for Wakeshot

- `p8-store` stays blocked, with the reason "signing is not free yet"; it re-opens when SignPath Foundation approves
  Wakeshot (no paid certificate: Haik, 2026-11-04 01:14).
- When it re-opens: EXE path; the hidden-listing test of uninstall during a hold; the two-source winget test; the
  first submission by hand (Haik: account, ID check, age rating); then the release workflow sets the package URLs
  and submits with the four secrets.
- Nothing in v1 needs to change for the Store later: the EXE path takes the same `Setup.exe`, as long as it and
  `Update.exe` and both exes are signed.

## 4. Pictures, animation and video

Read 2026-11-04 by a research helper of this card (its notes were temporary; the quotes that matter are below). GitHub docs
were read from `github/docs` through `gh api`; "observed" means seen in GitHub's REST-rendered README HTML or in HTTP
headers, not in a browser. The worker read again the Remotion license, the attachment limits and `gh issue comment
--attach` the same day.

### What the rivals and well-liked tools show

- **Nightcrew**: a full-width banner (alt text is the tagline), 2 badges, two short paragraphs with numbers ("one 1.7
  MB exe", "less than 1% of one CPU core"), then a bare `https://github.com/user-attachments/assets/...` line that
  plays as a 48 s video. Then Install, a wide "night in four steps" picture, a panel screenshot floated right, one
  `<details>`, Update, Uninstall. Its pictures are built from HTML sources in `docs/assets/src/`, so they can be
  rebuilt. Verified (README).
- **LidGuard**: tagline as H1, 4 badges, "Demo" with a 1.76 MB GIF; the rest is a long wall of text with no folding.
- **Agents Don't Sleep**: banner, 4 badges, a story in three sentences, a "Without it / With it" table, a tour with
  the four tray states in one picture and the menu, then settings in `<details open>` blocks, each with a screenshot
  and a "Why" table. Verified (README, read for part 1).
- **PowerToys Awake docs**: one settings picture, a GIF of the tray menu, and a State, Icon, Description table with
  the real icon of each state. A good pattern for Wakeshot's idle, working and off icons.
- Well-liked small tools: EarTrumpet (11.5k stars) opens with a 5.1 MB GIF kept in the repo; TranslucentTB (20.7k) a
  one-line pitch with a number ("uses a few MB of RAM and almost no CPU") and a row of screenshots; KeepingYouAwake
  (7k) 45 lines with questions as headings; `amnweb/yasb` (5.7k) a `<picture>` hero in light and dark and one
  `<details>` per install method, the main one `<details open>`; `zoxide` (40k, 553 lines) folds most of its length.
- GitHub truncates a README "beyond 500 KiB" and builds its own outline from the headings, so a hand-written contents
  list is optional (GitHub docs, about READMEs). Verified (docs source).

### What GitHub renders in a README

- Attachments: "10MB for images and gifs", "10MB for videos uploaded to a repository owned by a user or organization
  on a free GitHub plan", "100MB for videos ... on a paid GitHub plan"; video formats `.mp4`, `.mov`, `.webm`; "we
  recommend using H.264 for greatest compatibility"
  (https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files). Verified
  (worker read).
- The upload can be scripted: `gh issue comment <n> --attach <file>` uploads and writes the URL into the comment
  (gh 2.103.3 on this laptop has it; "You need push access to the repository", GitHub CLI docs). Verified (worker ran
  `gh issue comment --help`). Removing the link "does not necessarily delete the uploaded file"; whether deleting the
  comment breaks the URL: not verified, so the comment stays as the video's home.
- A video plays inline only as an uploaded attachment: the bare URL line renders as `<video ... controls muted>`; no
  autoplay, no loop (GitHub strips both); sound plays when the viewer unmutes. An MP4 or WebM kept in the repo does
  not play (github.com's `media-src` does not list `raw.githubusercontent.com`, and raw serves `.mp4` as
  `application/octet-stream`). Observed, not checked in a browser.
- GIF animates and honors the viewer's reduced-motion setting (GitHub changelog 2022-05-19). Animated WebP is served
  as `image/webp` and browsers animate it, but no GitHub doc names WebP or APNG. Verified (GIF), observed (WebP).
- Light and dark: "The `<picture>` HTML element is supported", with `<source media="(prefers-color-scheme: dark)">`
  (GitHub docs). `#gh-dark-mode-only` is gone from the current docs and was marked deprecated in 2022. Use
  `<picture>`.
- Repo files: Git warns above 50 MiB and blocks above 100 MiB; a repo should stay under 1 GB. Pages: a site "no larger
  than 1 GB", "soft bandwidth limit of 100 GB per month"; nothing against video, and a Pages site serves `.mp4` as
  `video/mp4` with range requests (observed on another Pages site), so `<video autoplay muted loop playsinline>` with
  an MP4 in `site/` should play. Not tested on our page.

### The video tool: Remotion

| | Remotion | HyperFrames | Motion Canvas | Revideo | Manim |
|---|---|---|---|---|---|
| License | own license: free for "an individual" and companies "with up to 3 employees"; "source-available ... not open source" | Apache-2.0 | MIT | MIT | MIT |
| State (2026-11-04) | v4.0.540, 2026-11-02; since 2020 | v0.9.9, 2026-11-03; created 2026-03-10 | last release 2024-12-14, dormant | last push 2026-07-15 | v0.22.1, 2026-10-27 |
| Headless CLI render | yes, `npx remotion render`, `npx remotion still` | yes, `npx hyperframes render` | no (editor button only; issue #415 open since 2023) | yes, `renderVideo()` | yes |
| MP4 with audio, GIF | yes; GIF without audio | yes | yes, from the editor | yes | yes |
| Windows | "Windows (x64)"; FFmpeg bundled | yes; needs a system FFmpeg (not on this laptop) | yes | yes | yes |
| For an agent | official agent skills (`remotion-dev/skills`), `llms.txt` | built for agents, Claude Code plugin | | | built for math animation |
| Telemetry | not checked | on by default | not checked | on by default | not checked |

All from the helper's reads with links in its notes; the Remotion license lines verified by the worker
(`LICENSE.md` in `remotion-dev/remotion`). Node v26.13.0 is on this laptop; `ffmpeg` is not on the PATH (worker
check, 2026-11-04 01:07).

**Pick: Remotion.** Mature, one command for the MP4 with sound, the GIF loop and the stills, FFmpeg bundled, official
agent skills, and free for Haik as an individual. The cost: it is not open source, so someone at a company of 4 or
more who wants to rebuild the video needs a Company License; Remotion 5.0 plans "an organization or team of
individuals with up to 3 people" (PR #3750, open), which still covers Haik. The video's code lives in `media/` and is
not part of the MIT app. **Runner-up: HyperFrames** (Apache-2.0, plain HTML). Change to it when the media must be
rebuildable by anyone with no license question, or when it ships a stable 1.0 and keeps it for a few months.
Decided by the worker, for Haik's review.

### Sound

| Source | License | Credit | Catch |
|---|---|---|---|
| Wakeshot's own chimes (`p5-sounds`, made in code) | MIT, ours | none | none |
| Kenney audio packs (Interface Sounds, UI Audio, Impact Sounds, Music Jingles) | CC0 ("Attribution is not required") | none | "Do not use our logo." No full music tracks |
| Freesound | per sound: CC0, CC BY 4.0, CC BY-NC 4.0 | CC0 none; BY needs `"sound" by user ( url ) licensed under ...` | use CC0 only |
| Pixabay | Pixabay Content License | none | "You cannot sell or distribute the Content ... on a Standalone basis": a raw track in a public repo likely breaks that (helper's reading, not legal advice); YouTube Content ID claims possible |
| YouTube Audio Library | YouTube terms | artist credit for CC tracks | terms outside YouTube not verified; avoid |
| Sonniss GDC bundle | own license | none | may not be redistributed "as sound effects": raw files cannot go in the repo |

**Pick:** Wakeshot's own chimes for the lid beat, Kenney CC0 sounds for clicks and the rise and fall, and one CC0 music
loop from Freesound for the bed, all committed under `media/` with their source URLs in `media/CREDITS.md` (credit is
not required for CC0; we list it anyway). Fallback for the bed: a soft pad made in code. Decided by the worker, for
Haik's review.

### Specs

| Output | Spec |
|---|---|
| README video | MP4, H.264 with AAC audio, 1920 x 1080, 30 fps, 30 s (plus or minus 2 s), under 10 MB (free-plan attachment limit; flat graphics compress well, else 1280 x 720). Uploaded with `gh issue comment <n> --attach` on a pinned "Media" issue; the README holds the bare URL on its own line. Plays muted with controls |
| README icon loop | GIF in the repo, about 4 to 6 s, light and dark versions in `<picture>`, under 1 MB (no documented cap for repo images; small keeps the top fast) |
| README stills | PNG, light and dark in `<picture>` |
| Pages | the same MP4 committed in `site/` with `<video autoplay muted loop playsinline>` and a poster PNG; the loop GIF; no other host (`p6-pages` rule) |
| Store (later) | screenshots PNG 1366 x 768 to 3840 x 2160, 1 to 10; box art 1080 x 1080 or 2160 x 2160; trailer MP4 1920 x 1080 with a 1920 x 1080 PNG thumbnail, 60 s or less (part 3) |

### Capturing the real tray area by script

- Stills: the spike's `BitBlt` with `CAPTUREBLT` (`spikes/tray/src/capture.rs`, per-monitor v2 aware through its
  manifest) is enough for the icon and the menu. Gaps: the tooltip is Explorer's window, so the spike's menu-window
  search does not find it (capture a screen rect instead); pixels inside the menu's rect but outside its round corners
  show what is behind (use a plain wallpaper or mask); no cursor. pwsh 7 runs DPI-unaware here and sees the 1920 x 1200
  screen as 1536 x 960, so `CopyFromScreen` needs `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` first. Helper
  read; not run.
- Video capture is not needed: the promo is animated in code from stills. If one is ever wanted, ffmpeg's `ddagrab`
  (Desktop Duplication, ffmpeg 6.0+; `output_idx`, `offset_x`, `offset_y`, `video_size`, `framerate`, `draw_mouse`,
  then `hwdownload`) handles mixed scales better than `gdigrab`, which scales all screens by one ratio. ffmpeg 8.1 adds
  `gfxcapture` (Windows.Graphics.Capture of one window). From the ffmpeg docs; not run.
- Scale: no documented API changes the display scale at run time (only community code with an undocumented
  `DisplayConfigSetDeviceInfo` type). The test laptop already has two real scales: the DELL screen at 150 % (it holds
  the tray when docked) and the laptop screen at 125 %. **Decision: capture at 150 % and 125 %, the two real scales;
  100 % only if Haik switches a screen to 100 % in Settings by hand.** Decided by the worker, for Haik's review.
- Light and dark: Microsoft documents only reading the mode; writing `SystemUsesLightTheme` and broadcasting
  `ImmersiveColorSet` is community practice. The repo has already switched the mode through Settings with UI
  Automation (`docs/research.md`, "Tray, menu and notifications"), so `p6-media` reuses that.

### What this means for Wakeshot

- The README top: one picture or motion in the first screen (the GIF loop in `<picture>`), a one or two sentence
  pitch with a number, then install. The video goes right below as an uploaded attachment; it starts muted, so it
  must make sense without sound, with sound as the bonus.
- A State, Icon, Description table (PowerToys' pattern) shows the three icons clearly, light and dark.
- Agents Don't Sleep's "Without it / With it" table and "Why" lines are a strong pattern the README can borrow in its
  own words.
- Every media file rebuilds from `media/` with one command each: Remotion for the video, the GIF loop and the
  composed stills; the real captures are the only inputs that need the laptop.

### Storyboard for the 30 second video

Built in code from the real icon art, the real screenshots and Wakeshot's own chimes. Every claim on screen is
something Wakeshot does in v1. 1920 x 1080, 30 fps. Words on screen are drafts; `p5-text` and `p6-readme` own the final
wording. Decided by the worker, for Haik's review.

| Time | Scene | On screen | Sound |
|---|---|---|---|
| 0 to 3 s | Title | The idle icon, large, centered, wakes to the working state once; "Wakeshot"; "Your PC stays awake while your AI agent works." | music bed fades in |
| 3 to 8 s | Work starts | A terminal card drawn in code: `claude` and a short prompt typing in. Cut to the real tray crop; the icon morphs from idle to working; the real holding tooltip slides up. Caption: "Claude Code starts work. Wakeshot holds the PC awake." | key clicks, a soft rise on the morph |
| 8 to 12 s | Lid closed (optional beat) | A laptop outline closes; a lock badge. Caption: "Close the lid: it keeps working, and locks the PC." | Wakeshot's own lid chime (`LidClosedHolding`) |
| 12 to 17 s | Work ends | The terminal card shows the turn done; the icon goes back to idle; the idle tooltip. Caption: "Work done. It lets go." | a soft fall |
| 17 to 21 s | Settings back | A card drawn in code with two rows, "Lid close: Sleep" and "Battery sleep: 4 hours", each ticking from Wakeshot's value back to the user's. Caption: "Your own power settings are back." | two light ticks |
| 21 to 25 s | Keep awake | The real menu screenshot (light, then dark in a split), the highlight moves to Keep awake, then "1 hour"; the icon goes to working. Caption: "Keep awake on demand. No second tool." | menu clicks |
| 25 to 28 s | Speed | Three numbers count up from `p6-media`'s measurements: idle RAM, CPU per 10 minutes, delay from Claude starting to the hold. Small print: "measured on <build>, <date>". | soft ticks |
| 28 to 30 s | End card | Icon, "Wakeshot", "Free and open source. Windows 11, x64 and ARM64.", the install line (winget once it is live, else the GitHub URL). | music resolves |

Rules: no fake numbers (the speed scene waits for `p6-media`'s measurements); no agent shown that v1 does not detect;
the terminal card is an illustration of Claude Code, not a copy of its screen; nothing private in any capture.

### Shot list for `p6-media`

| Id | Capture | Variants | Used in |
|---|---|---|---|
| S1 | Tray area crop with the idle icon (taskbar right side, about 400 x 48 px at 100 %, scaled) | light, dark; 150 % and 125 % | video 3 to 17 s, README, Store |
| S2 | The same with the working icon | same | video, README loop source |
| S3 | Tooltip while holding (Claude Code, 1 session) | light, dark; 150 % and 125 % | video 3 to 8 s, README |
| S4 | Tooltip while idle | light, dark | video 12 to 17 s |
| S5 | Menu open, Keep awake submenu open | light, dark; 150 % and 125 % | video 21 to 25 s, README, Store |
| S6 | One notice: the closed-lid summary after a hold | light, dark | README, Store |
| S7 | The icon art files of every state (from `p5-icon-art`) | as delivered | title, end card, loop |
| S8 | Wakeshot's chimes rendered to WAV from the same code that plays them (`p5-sounds`) | 48 kHz | lid beat |
| S9 | The before and after values of lid action and battery sleep from the capture session's log, as text | | settings scene |
| S10 | Store screenshots: S1, S3, S5 and S6 placed on a plain background at the Store sizes (part 3) | light, dark | Store |

The silent icon loop for the README top: the working and idle states of S7, idle to working to idle, 4 to 6 s, made
from the art (not a screen capture), in the format the facts below pick.
