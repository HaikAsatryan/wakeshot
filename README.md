# Wakeshot

<!-- media: icon loop. One centered <picture> block here: a dark and a light GIF (prefers-color-scheme), 4 to 6 s, under 1 MB, the tray icon going idle to working to idle, made from the final icon art, alt text "Wakeshot's tray icon wakes while an agent works and rests when it is done". Files under media/ from p6-media; the art from p5-haik-check. Takes 1 line here. -->

**Your PC stays awake while your AI agent works. Then it sleeps as usual, with your own power settings back.**
Keep awake on demand comes in the same menu, so you never need a second tool.

<!-- media: speed numbers. One line here: idle RAM (working set, median), idle CPU (ms per 10 minutes and % of one core) and the delay from Claude Code starting work to the hold starting (median of 5 runs), each number linked to its method section in docs/research.md, with the build version and the date. Numbers from p6-media, measured as docs/research/launch.md "Speed numbers: how p6-media measures each" says. Takes 1 line here. -->

**Install:** download `HaikAsatryan.Wakeshot-win-x64-Setup.exe` (`-win-arm64-` for Windows on Arm) from [Releases](https://github.com/HaikAsatryan/wakeshot/releases) and run it. <!-- winget: Phase 8 adds "or: winget install HaikAsatryan.Wakeshot" to this line once the package is merged. -->
Windows 11, x64 and ARM64. Per user, no admin. Free and open source (MIT). macOS is planned for later.
Knows Claude Code today, with no hooks and no edits to its settings. Codex CLI and Copilot CLI come next.

> Not released yet: the Releases page is still empty. <!-- release: drop this line at the first release. -->

## How it compares

PowerToys Awake is the most-starred keep-awake tool on Windows. LidGuard and Agents Don't Sleep are the two
most-starred tools that watch AI agents and ship a Windows release. Every rival cell links to its source, all read
2026-11-04.

| | Wakeshot | PowerToys Awake | LidGuard | Agents Don't Sleep |
|---|---|---|---|---|
| Stars | new | [140,491, for all of PowerToys](https://github.com/microsoft/PowerToys) | [13](https://github.com/airtaxi/LidGuard) | [10](https://github.com/itsmbaqer/agents-dont-sleep) |
| Runs on | Windows 11; macOS later | [Windows](https://learn.microsoft.com/en-us/windows/powertoys/install) | [Windows, Linux, macOS](https://github.com/airtaxi/LidGuard) | [Windows, macOS, Linux](https://github.com/itsmbaqer/agents-dont-sleep) |
| Built with | Rust and `windows-rs`, no runtime | [C# on .NET 10, inside PowerToys](https://github.com/microsoft/PowerToys/blob/main/src/Common.Dotnet.props) | [C# .NET 10 NativeAOT, a .NET tool](https://github.com/airtaxi/LidGuard) | [Rust and Tauri, with a web UI](https://github.com/itsmbaqer/agents-dont-sleep/blob/HEAD/src-tauri/Cargo.toml) |
| Download, x64 | <!-- media: Wakeshot's x64 Setup.exe size in MB from p6-media, linked to its method in docs/research.md --> | [323.9 MB, all of PowerToys](https://github.com/microsoft/PowerToys/releases/tag/v0.102.3001) | [25.3 MB](https://www.nuget.org/packages/lidguard.win-x64), plus the [.NET SDK](https://learn.microsoft.com/en-us/dotnet/core/tools/global-tools) | [1.9 MB](https://github.com/itsmbaqer/agents-dont-sleep/releases/tag/v0.4.0) |
| ARM64 build | yes, native | [yes](https://github.com/microsoft/PowerToys/releases/tag/v0.102.3001) | [yes](https://www.nuget.org/packages/lidguard.win-arm64) | [no](https://github.com/itsmbaqer/agents-dont-sleep/releases/tag/v0.4.0) |
| Idle RAM and CPU | <!-- media: Wakeshot's idle RAM and CPU from p6-media, linked to its method in docs/research.md --> | [not published; 14.3 MB private memory in a maintainer's test harness](https://github.com/microsoft/PowerToys/pull/51181) | [not published](https://github.com/airtaxi/LidGuard) | [not published](https://github.com/itsmbaqer/agents-dont-sleep) |
| Knows when an agent works | Claude Code, from its own status files; any command through `wakeshot run`; scripts through hold files | [no: a timer, a time or a process id](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [Codex, Claude Code, Copilot CLI, OpenCode; tested: Codex](https://github.com/airtaxi/LidGuard) | [8 agents; tested end to end: Claude Code](https://github.com/itsmbaqer/agents-dont-sleep) |
| Edits your agent's config | no | [no](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [yes, adds hooks](https://github.com/airtaxi/LidGuard) | [yes, adds hooks](https://github.com/itsmbaqer/agents-dont-sleep) |
| Keeps working with the lid closed | yes, while something holds | [no](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [yes, optional](https://github.com/airtaxi/LidGuard) | [yes](https://github.com/itsmbaqer/agents-dont-sleep) |
| Puts your power settings back | yes: after each hold, at quit, after a crash | [changes none](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [yes; after a crash, only at its next run](docs/research/rivals-nightcrew-lidguard.md#32-edge-case-checklist) | [yes, also after a crash](https://github.com/itsmbaqer/agents-dont-sleep/blob/HEAD/SECURITY.md) |
| Battery floor | Windows' own energy saver level | [none listed](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [no](docs/research/rivals-nightcrew-lidguard.md#4-lid-closed-behavior) | [15% by default](https://github.com/itsmbaqer/agents-dont-sleep) |
| Locks a closed laptop | yes | [n/a: no lid support](https://learn.microsoft.com/en-us/windows/powertoys/awake) | [no](docs/research/rivals-nightcrew-lidguard.md#4-lid-closed-behavior) | [yes, by default](https://github.com/itsmbaqer/agents-dont-sleep) |
| Updates itself | yes, only while nothing holds | [checks, then shows a notice](https://learn.microsoft.com/en-us/windows/powertoys/install) | [no: `dotnet tool update` by hand](https://learn.microsoft.com/en-us/dotnet/core/tools/global-tools) | [no updater found](https://github.com/itsmbaqer/agents-dont-sleep) |
| Install | per user, no admin | [per user or machine-wide](https://learn.microsoft.com/en-us/windows/powertoys/install) | [per user](https://learn.microsoft.com/en-us/dotnet/core/tools/global-tools) | [per user](https://github.com/itsmbaqer/agents-dont-sleep/blob/HEAD/src-tauri/tauri.conf.json) |
| License, price | MIT, free | [MIT, free](https://github.com/microsoft/PowerToys) | [MIT, free](https://github.com/airtaxi/LidGuard) | [MIT, free](https://github.com/itsmbaqer/agents-dont-sleep) |

Where the others lead today: LidGuard and Agents Don't Sleep know more agents and also run on macOS and Linux, and
Agents Don't Sleep's installer is smaller. Where Wakeshot differs: it reads Claude Code's own status files instead of
adding hooks to your agent's config, and it updates itself, only between holds. Next to LidGuard it adds a lock and a
battery floor; next to PowerToys Awake, it knows when your agent works and keeps a closed laptop working.

## See it work

<!-- media: video. The bare https://github.com/user-attachments/assets/<id> URL of the 30 second promo on its own line, so GitHub shows a player (muted, with controls): MP4, H.264 and AAC, 1920 x 1080, under 10 MB, uploaded with `gh issue comment <n> --attach` to the pinned "Media" issue, its URL recorded in media/README.md. From p6-media. -->

<!-- media: menu picture. One <picture> block: a dark and a light PNG (prefers-color-scheme) of the real menu with the Keep awake submenu open (shot S5), cropped to the tray area, alt text "Wakeshot's menu with the Keep awake choices open". Files under media/ from p6-media. -->

<!-- media: tooltip picture. One <picture> block: a dark and a light PNG of the real tooltip while holding for Claude Code, "Wakeshot" over "Holding awake: Claude Code (1 session)" (shot S3), alt text "Wakeshot's tooltip naming what holds the PC awake". Files under media/ from p6-media. -->

The icon tells you what is going on, and the tooltip names what holds the PC awake:

| Icon | State | The tooltip says | What it means |
|---|---|---|---|
| <!-- media: holding icon, light and dark, from p5-haik-check's art --> | Holding | `Holding awake: Claude Code (1 session)` | Something needs the PC. It stays awake, and closing the lid does not stop it. |
| <!-- media: idle icon, light and dark, from p5-haik-check's art --> | Idle | `Not holding, the PC sleeps as usual` | Automatic is on, and nothing needs the PC right now. |
| <!-- media: off icon, light and dark, from p5-haik-check's art --> | Off | `Automatic is off, the PC sleeps as usual` | Wakeshot watches nothing. Keep awake still works. |

### A night with Wakeshot

1. **You give Claude Code a long task and walk away.** The icon turns to holding. Your screen still turns off on its
   usual timer; only sleep waits.
2. **You close the lid.** A short chime says the laptop keeps running, and Wakeshot locks it, so a closed laptop
   never runs unlocked.
3. **Claude finishes in the night.** Wakeshot puts your lid and sleep settings back at once. After 5 minutes with no
   input it plays a soft "going to sleep" sound and hibernates the laptop.
4. **Morning.** You open the lid and sign in. A notice says how long the PC ran with the lid closed, and what kept it
   running.
5. **The battery ran low instead?** Below Windows' own energy saver level, Wakeshot steps back at once, puts your
   settings back and hibernates the closed laptop after 5 minutes with no input.

No window, no settings page, nothing to learn: a tray icon, one menu, a few notices and a small command line.

## Details

- [Install](#install)
- [What keeps the PC awake](#what-keeps-the-pc-awake)
- [What it changes on your PC](#what-it-changes-on-your-pc)
- [Lid closed, lock and battery](#lid-closed-lock-and-battery)
- [The menu, notices and start at login](#the-menu-notices-and-start-at-login)
- [Commands](#commands)
- [Updates and privacy](#updates-and-privacy)
- [Limits](#limits)
- [Uninstall](#uninstall)
- [Credits and license](#credits-and-license)

### Install

<details>
<summary>Setup.exe from GitHub Releases, the SmartScreen warning, winget</summary>

1. Open [Releases](https://github.com/HaikAsatryan/wakeshot/releases) and download the Setup for your PC:
   - `HaikAsatryan.Wakeshot-win-x64-Setup.exe` for most PCs;
   - `HaikAsatryan.Wakeshot-win-arm64-Setup.exe` for Windows on Arm.
2. Run it. It installs for your user only and never asks for admin rights.
3. Wakeshot starts right away, and from then on at each sign-in. A first notice, "Wakeshot is running", says that its
   icon may hide under the `^` arrow on the taskbar. To keep it in view, click the notice, then turn on Wakeshot under
   "Other system tray icons".

What the install sets up:

- the app in `%LocalAppData%\HaikAsatryan.Wakeshot`, and a Start menu shortcut (no desktop shortcut);
- start at login, a `Run` value under your user's registry;
- the `wakeshot` command on your user `Path` (open a new terminal to use it);
- its own data in `%LocalAppData%\Wakeshot`, kept apart from the app, so updates and reinstalls never touch it.

**Not signed yet.** v1 is not code-signed, so Windows SmartScreen may warn you before Setup runs.
<!-- todo: the exact SmartScreen words and the steps to go on are not in docs/research.md yet; add them from a capture of the first public pre-release (v0.1.0-rc.1). -->
Signing comes through SignPath Foundation's free program once Wakeshot has users; signed builds then show "SignPath
Foundation" as the publisher.

**winget.** Coming. Wakeshot will update itself there too: winget installs it, and Wakeshot's own updater keeps it
current.
<!-- winget: Phase 8 adds "winget install HaikAsatryan.Wakeshot" here once the package is merged. -->

</details>

### What keeps the PC awake

<details>
<summary>Claude Code, <code>wakeshot run</code>, hold files and Keep awake</summary>

Wakeshot calls each reason to stay awake a hold. While any hold counts, the PC stays awake. When the last one ends,
the PC sleeps as usual.

**Claude Code.** Wakeshot reads the small status file Claude Code keeps for each session (in `~/.claude/sessions`),
and that session's transcript for timers and background tasks. It reads only the fields it needs, never stores or
logs your prompts, adds no hooks and never edits Claude's settings. A session holds:

- while it works, also while background subagents run after a turn;
- while a background command runs and the session is idle, for as long as the session lives;
- in VS Code, the SDK or the Desktop app, while a background task in the transcript is still open;
- while a scheduled run (`/loop`, a cron job, a wakeup) is due within 1 hour;
- while it waits for your answer, but only with Remote Control on, and at most 1 hour.

A session counts only while its process lives, so a crashed session or a file left behind never holds. If Claude
Code changes its files and Wakeshot cannot read them, a notice says "Update Wakeshot". Tested live with Claude Code
2.1.317, in the terminal and through the SDK ([results](docs/research/claude-code-signals.md#phase-4-end-to-end-2026-11-04)).

**`wakeshot run -- <command>`.** Holds while that command runs: a build, a test suite, a backup, anything. Such as
`wakeshot run -- npm test`. The command always runs, gets its arguments exactly as typed and keeps its exit code.

**Hold files.** Any script can hold the PC awake by writing a small JSON file to `%LocalAppData%\Wakeshot\holds\`.
The hold lasts while the process named in it runs. Format and examples: [docs/hold-files.md](docs/hold-files.md).

**Keep awake.** From the menu or `wakeshot keep-awake`: 30 minutes, 1 hour, 2 hours, 4 hours, or until you turn it
off, with Keep screen on if you want it. It works whether Automatic is on or off, and the lid and battery rules below
apply to it the same way. It ends at sign-out or restart, so a forgotten "Until I turn it off" never survives a
reboot.

**Automatic** (on by default) is the switch for the first three. Off: only Keep awake holds.

</details>

### What it changes on your PC

<details>
<summary>One power request, three power settings, and how your own values come back</summary>

When nothing holds, Wakeshot changes nothing: your PC behaves as if Wakeshot were not there.

While something holds:

1. **A power request.** Wakeshot asks Windows to keep the system running. The screen still turns off on its usual
   timer, unless you turn on Keep screen on.
2. **Three settings of your active power plan**, written when the hold starts and put back when it ends:

| Setting | While holding | Why |
|---|---|---|
| Lid close, plugged in | Do nothing | No power request can stop a lid close; only this setting keeps a closed laptop working |
| Lid close, on battery | Do nothing | The same |
| Sleep after, on battery | Never | On battery, Windows ends every power request about 5 minutes after this timeout, lid open or closed |

The facts and our tests behind the table: [Windows power facts](docs/research.md#windows-power-facts). Wakeshot writes
them also when plugged in, so unplugging, or a USB-C dock that drops power for a few seconds, never meets the battery
rule first. Sleep while plugged in is never touched: there a power request is enough. While a hold runs, Windows
Settings shows these values; that is Wakeshot, and they go back when the hold ends.

How your values come back:

- Before any write, Wakeshot saves your value in `%LocalAppData%\Wakeshot\journal.json` and remembers what it wrote.
- It puts back only what is still its own. Change one of these settings during a hold, and your change wins at once:
  Wakeshot leaves that setting alone on every plan until the current work ends, and a notice says so.
- Switch power plans, and it puts the old plan back and applies to the new one.
- It puts them back when the work ends, at Quit, at sign-out, at its next start and at uninstall. If Wakeshot crashes
  or is killed, a small guard process, which runs only while settings are changed, puts them back. If even that does
  not run, your next sign-in does.
- **Restore my power settings** in the menu, or `wakeshot restore`, puts them back now and stops changing them until
  the current work ends. The PC still stays awake for the work, but only while the lid is open.
- If a company policy blocks the change, Wakeshot keeps the PC awake with the power request only, and tells you.
- Every write goes to the log with the old and the new value.

Everything else it keeps, all under your user:

- `%LocalAppData%\Wakeshot`: `config.json` (Automatic on or off), `holds\` (hold files), `state.json` (what Wakeshot
  is doing, for `wakeshot status`), `journal.json` (your saved values, only while settings are changed),
  `restore-request.json` (a short-lived request from `wakeshot restore`), `logs\` (the log).
- Registry, under `HKCU\Software\Microsoft\Windows\CurrentVersion`: `Run\Wakeshot` (start at login) and
  `RunOnce\WakeshotRestore` (only while settings are changed: the restore at your next sign-in).
- Your user `Path`: the app's folder, for the `wakeshot` command.
- Windows itself keeps a record of every tray icon it has seen, Wakeshot's too (`HKCU\Control Panel\NotifyIconSettings`).

Wakeshot changes no other setting, never touches other apps and never needs admin rights.

</details>

### Lid closed, lock and battery

<details>
<summary>Close the lid, unplug, run low: what happens, and why it never lowers your security</summary>

Wakeshot never lowers your security.

- **Close the lid while something holds.** A short chime says the laptop keeps running, and Wakeshot locks the PC.
  Windows would have slept, and with the lid set to Do nothing it would not lock by itself.
- **Docked, with your own lid action already Do nothing.** Wakeshot changes nothing there: no lock, no chime.
- **Unplug a closed laptop that runs only because of Wakeshot.** It locks once the laptop has been on battery for
  30 seconds, so a dock's short power drop does not lock you while you work on the external screen.
- **If the lock fails.** Wakeshot tries once more. If that fails too, it stops holding, puts your settings back and
  hibernates the laptop after 5 minutes with no input, so a closed laptop never runs unlocked in a bag.
- **Work ends with the lid closed.** Your settings come back at once. After 5 minutes with no input, a short "going to
  sleep" sound plays, then Wakeshot hibernates the laptop. Open the lid during the sound to stop it. Typing on an
  external screen counts as input, so a docked laptop is not put to sleep under your hands. If hibernate is off or
  fails, a notice says so, and Windows will most likely sleep it at your usual sleep time; until the lid opens,
  Wakeshot locks it whenever it is left idle for 5 minutes.
- **Open the lid and sign in.** A notice says how long the PC ran with the lid closed and what held it, such as "The
  PC kept running for 2 h 15 min with the lid closed. What held it: Claude Code."
- **Battery floor.** Wakeshot uses Windows' own energy saver level (20% when energy saver is set to Never or Always);
  there is no Wakeshot setting for it. On battery below it, Wakeshot steps back at once: it ends its power request,
  puts your settings back, plays a sound and says "Battery low: the PC may sleep now". If the lid is closed, it then
  hibernates the laptop after 5 minutes with no input, unless your own battery lid action is Do nothing. Plug in, and
  the hold comes back.
- **Keep screen on.** The screen never turns off, so Windows does not lock the PC at screen off; Wakeshot locks it
  instead once you have been away longer than your own sleep timeout.
- **Start > Sleep.** Your own sleep always wins. Windows drops every power request then, and Wakeshot takes its
  request again after you wake the PC.

Sounds: three short ones, for a lid close while holding, for "going to sleep", and for the battery floor. They follow
your Windows sound scheme ("No sounds" means none). They play on your default audio device, unless that is a
screen's audio (HDMI or DisplayPort, often with no speakers); then on the laptop speakers.

</details>

### The menu, notices and start at login

<details>
<summary>Every menu item, every notice, and how to stop Wakeshot starting at login</summary>

Click the tray icon:

```text
Holding awake: Claude Code (2 sessions)
---------------------------------------
Automatic                           [x]
Keep awake                           >    Off
---------------------------------------   30 minutes
Restore my power settings                 1 hour
Open log                                  2 hours
Quit                                      4 hours
                                          Until I turn it off
                                          -------------------
                                          Keep screen on
```

- **The top line** says what holds right now, in the same words as the tooltip.
- **Automatic**: watch Claude Code, `wakeshot run` and hold files. On by default, and kept across restarts.
- **Keep awake**: hold for a time, or until you turn it off. **Keep screen on** keeps the screen on too, while Keep
  awake is on.
- **Restore my power settings**: put your lid and sleep settings back now (see above).
- **Open log**: opens the log with your default app.
- **Quit**: stop holding, put your settings back, end Keep awake, exit.

The menu follows your taskbar's light or dark mode, like Windows' own tray menus. Other top lines you may see:
`Holding awake, screen on`, `Holding awake, lid open only` (Wakeshot could not change the power settings, so closing
the lid sleeps the laptop as usual), `Battery low, not holding` and `Cannot hold awake` (Windows refused the power
request).

A routine hold that starts or ends shows only in the icon and the tooltip. Anything Wakeshot does on its own gets a
notice:

| Notice | When |
|---|---|
| Wakeshot is running | First start: where the icon is and how to pin it |
| Wakeshot keeps your change | You changed a setting Wakeshot had changed |
| Your power settings are back | You picked Restore my power settings |
| The PC ran with the lid closed | You are back after a closed-lid hold |
| Battery low: the PC may sleep now | The battery fell below the floor |
| Wakeshot locked the PC | Keep screen on, and you were away past your sleep timeout |
| Wakeshot updated to (version) | After an update, with no sound |
| Update Wakeshot | Claude Code changed its files, and Wakeshot cannot fully read them |
| Wakeshot cannot change power settings | Something, such as a company policy, blocked the change |
| Wakeshot cannot keep the PC awake | Windows did not accept the power request |
| Could not put back your power settings | A restore failed; Wakeshot keeps trying, and the log has your old values |
| Check your lid and sleep settings | The file with your saved values was damaged; the log has your old values |
| Wakeshot could not lock the PC | Windows did not lock; with the lid closed Wakeshot tries once more |
| Wakeshot could not hibernate the PC | Hibernate is off or failed with the lid closed |

Notices raised while the PC is locked, or while another user is on the screen, wait until you are back. Sounds never
wait.

**Start at login** is on after the install. To turn it off: Settings > Apps > Startup, then switch Wakeshot off.
Uninstall removes it.

</details>

### Commands

<details>
<summary>The <code>wakeshot</code> command, its help and what each command does</summary>

`wakeshot --help`:

```text
Wakeshot keeps this PC awake while something needs it.

Usage: wakeshot <command>

Commands:
  run -- <command>                           Run a command, awake until it ends
  keep-awake 30m|1h|2h|4h|on|off [--screen]  Keep awake for a time, or until off
  automatic on|off                           Watch AI agents and 'run' commands
  status                                     Show what Wakeshot is doing
  restore                                    Restore your power settings
  log                                        Open the log

'keep-awake on' lasts until you turn it off. '--screen' keeps the screen on too.

Test aid:
  --dry-run   Before the command: use the dry-run data folder, as the tray's
              --dry-run does. Such as: wakeshot --dry-run status
```

- `run`: everything after `--` is your command, exactly as typed, such as `wakeshot run -- npm test`. It runs in the
  same console, and `wakeshot` exits with its exit code. It always runs the command, and warns when Wakeshot is not
  running or Automatic is off. Its own codes, as GNU `env` and `timeout` use them: 127 the command was not found, 126
  it cannot start (a `.ps1` file, for example: run `pwsh -File <script>` instead), 125 it lost the command. The hold
  follows the command's own process, not the `wakeshot` wrapper.
- `keep-awake`: the same lengths as the menu. It needs Wakeshot running, and says so when it is not. `--screen` does
  not go with `off`.
- `automatic on` and `automatic off`: the same as the menu's Automatic.
- `status`: what holds, what is changed, and whether Wakeshot runs at all.
- `restore`: the same as Restore my power settings.
- `log`: opens the log.
- Words match in any case, as in Windows' own commands.

For your own lengths and labels, write a hold file: [docs/hold-files.md](docs/hold-files.md).

</details>

### Updates and privacy

<details>
<summary>Automatic updates between holds, two checks a day, no telemetry</summary>

- Wakeshot updates itself from GitHub Releases. It checks 5 minutes after it starts, then every 12 hours, and
  downloads a new version at once.
- It applies the update only when nothing holds (no agent at work, no `wakeshot run`, no hold file, no Keep awake),
  none of your settings are changed and no hibernate is pending. So an update never cuts your work short.
- Then Wakeshot quits and the new version starts. One quiet notice says "Wakeshot updated to (version)", with no
  sound.
- A failed check or download waits for the next check, with no notice. Pre-releases are skipped.
- The only network calls are those update checks and downloads, to GitHub, with no account and no token.
- No telemetry. The log stays on your PC: `%LocalAppData%\Wakeshot\logs\wakeshot.log`, plain English, rolled over at
  start once it passes 1 MB, three files kept. It never holds your prompts, transcript text or full command lines;
  `wakeshot run` keeps only the program's name.

</details>

### Limits

<details>
<summary>What v1 does not do, and the rare cases where your settings stay changed for a while</summary>

**After a crash, your settings can stay changed for a while.** In these cases nothing puts them back right away: the
lid action stays Do nothing and battery sleep Never, so closing the lid does not sleep the laptop, and a closed laptop
at the sign-in screen does not sleep either. A restore before sign-in would most likely need a task that runs at
boot, and that needs admin rights, which Wakeshot never asks for.

- Until your next sign-in:
  - Windows crashes, or the power is cut, during a hold;
  - you pick "Shut down anyway", or Windows ends your session, before the restore at sign-out finishes;
  - End task ends Wakeshot and its guard together, if their separate starts do not keep them apart (not tested yet).
- Until Wakeshot next starts (from the Start menu, or at the next sign-in when start at login is on), because Windows
  deletes the one-time sign-in entry before it runs it:
  - the restore at sign-in fails for 2 minutes, and the Wakeshot it then starts fails to start too;
  - the restore at sign-in crashes.
- Until you reinstall, with no warning: you uninstall during a hold, and the restore fails for the uninstall's whole
  25 seconds. Your saved values stay, so a reinstall puts them back.
- Until you set the values by hand: the file with your saved values is damaged. Wakeshot sets it aside and warns
  you, and the log holds your old values. Until then a lid close gets no lock either.

**A failed restore brings Wakeshot back.** When the guard or the restore at sign-in cannot put your settings back for
2 minutes, it starts Wakeshot, even with start at login off, so you see the warning and the retries go on. That
Wakeshot also holds for agents as usual, so with start at login off it is back and changes your settings at the next
agent turn. After a Quit whose restore failed, Wakeshot comes back 2 minutes later.

**One signed-in user at a time.** Power plans belong to the whole PC. When another user takes the screen (switch
user), Wakeshot puts your values back and keeps only its power request, then applies them again when you come back.
If one user's changes were left behind (that session ended without a restore), another user's Wakeshot takes Do
nothing for that user's own choice: no lock, no hibernate. A second signed-in user's Wakeshot keeps the PC awake with
the power request only. Remote Desktop into your session most likely counts as
leaving the screen, so it gets the power request only; not tested yet.

**`winget upgrade` turns Wakeshot off until you sign in again.** Once Wakeshot is on winget: `winget upgrade` runs
the new Setup silently. That ends Wakeshot and any running `wakeshot run` wrapper; the command itself goes on. Setup
puts your power settings back, but a silent Setup does not start the app, so Wakeshot stays off until your next
sign-in or until you start it from the Start menu. While it is off, nothing keeps the PC awake, also not for that
command. If winget installed Wakeshot, `winget upgrade --all` skips it, since Wakeshot updates itself. If you
installed from GitHub, `winget pin add HaikAsatryan.Wakeshot` makes `--all` skip it too (not tested on Wakeshot yet).

**Claude Code only, in v1.** Codex CLI and Copilot CLI come next. Until then, `wakeshot run -- <agent>` holds for as
long as the agent runs, idle or not; or write a hold file.

**Some Claude Code waits do not hold.** Waiting for your answer holds only with Remote Control on, for at most 1
hour; without it the PC may sleep while Claude waits for you. A scheduled run more than 1 hour away does not hold: the
PC may sleep, and the job runs once it wakes.

**`CLAUDE_CONFIG_DIR`.** Wakeshot follows it when it is set as a user variable in Windows, so Wakeshot gets it too.
A value set only in one terminal is not seen, and those sessions do not hold.

**Not in v1:** Claude Code inside WSL (its sessions carry Linux process ids, which Windows cannot check); rules for
any app, such as "while ffmpeg runs"; macOS and Linux; languages other than English.

**Not tested live yet:** the Claude Code extension for VS Code and the Desktop app.

**With the lid closed, no time limit and no heat guard.** Wakeshot holds as long as the work runs. The hardware's own
protection applies, the same as for a lid set to Do nothing by hand: the CPU slows itself when it gets too hot (not
tested by us), and Windows shuts down at the critical temperature.

**If a company policy blocks the settings.** Wakeshot then keeps the PC awake with the power request only: closing
the lid sleeps the laptop as usual, and on battery Windows ends the request about 5 minutes after your sleep timeout.

**Unsigned in v1.** See [Install](#install).

</details>

### Uninstall

<details>
<summary>What it puts back, what it removes, and what stays</summary>

Uninstall Wakeshot like any app, from Settings > Apps. The uninstall:

1. removes start at login (with its switch in Settings > Apps > Startup), the sign-in restore entry and the `Path`
   entry first;
2. puts your power settings back if a hold was running, trying for up to 25 seconds;
3. deletes `%LocalAppData%\Wakeshot` whole, after a good restore or when there was nothing to restore;
4. deletes the app folder, the Start menu shortcut and the app's entry in the Apps list.

What stays:

- After a restore that failed: `journal.json` and `logs\` in `%LocalAppData%\Wakeshot`, so a reinstall can still put
  your values back; the log holds them if you prefer to set them by hand. Until then the lid action stays Do nothing
  and battery sleep Never (see [Limits](#limits)).
- The installer's own logs: `%LocalAppData%\velopack\velopack.log` and
  `%LocalAppData%\velopack\velopack_HaikAsatryan.Wakeshot.log`. Delete them by hand if you like.
- Windows' own record of the tray icons it has seen: `HKCU\Control Panel\NotifyIconSettings\<id>` and the
  notification key `...\Notifications\Settings\NotifyIconGeneratedAumid_<id>`. Windows writes these itself, for
  every tray icon it shows, and Wakeshot leaves them in place.

</details>

## Credits and license

Wakeshot ports no code from the tools above, so no credit line is due. The date math in hold files follows Howard
Hinnant's public domain [date algorithms](https://howardhinnant.github.io/date_algorithms.html). Built on
[`windows-rs`](https://github.com/microsoft/windows-rs); installs and updates with
[Velopack](https://github.com/velopack/velopack).

For the curious: [docs/product.md](docs/product.md) (what it does and why), [docs/architecture.md](docs/architecture.md)
(how it is built), [docs/research.md](docs/research.md) (every fact, and how we checked it).

MIT license, see [LICENSE](LICENSE). Made by Haik Asatryan.
