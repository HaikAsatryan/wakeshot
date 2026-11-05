# Screen session: the new brand on real Windows screens

Phase: 6 (brand apply). Status: done (2026-11-05 16:5x; open items in Result). Needs Haik: no for this run: Haik (2026-11-05 15:40, away at least an
hour): "Do the screen session as well". The worker does every step itself; a step that truly needs his hands is
marked "not shown" with the reason and listed for him. Agent: `wakeshot-worker`. Size: Part A about 50K tokens; Part
B about 80K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-brand-apply-art` (done). The page and README block runs last, after `p6-brand-apply-web` is done.
Parallel with: `p6-brand-apply-web` Part B and `p6-brand-apply-media` Part B (offscreen only, no screen use).
Real laptop: the screen, a real install and uninstall of a local build, theme and high contrast switches, display
scale changes (Haik's own settings: each one read before, put back and read again after).
Display: the orchestrator holds the display on with a display-only power request (a hidden `pwsh` running
`<scratchpad>\display-hold.ps1`, PID in `<scratchpad>\display-hold.pid`), so the session does not lock while Haik is
away. Do not stop it; the orchestrator ends it after this card. It shows in `powercfg /requests` under DISPLAY.
Owns: this card; its pictures `docs/research/img/brand/screen-*.png`; its scratch folder
`<scratchpad>\p6-brand-screen-session\` (build folder `target` inside it, deleted when done).

## Goal

The new brand has been seen only in offscreen renders. Before `p6-media` Part B, the README and Pages finish, Haik
and a worker look at every place Windows shows it, on real screens, and capture each one. Part A writes the exact
checklist; Part B runs it with Haik.

## Read first

- `docs/research/brand-review.md`: "Next screen session: what to capture" (8 items) and the findings it names.
- The Results of the three apply cards (what changed, the splash decision, the notice icon choice).
- `CLAUDE.md` "Test laptop" (quit an installed Wakeshot first; no other keep-awake app), `docs/tasks/p6-screen-reruns.md`
  and `docs/tasks/p6-real-install-guard.md` (how earlier screen and install runs were done safely), `packaging/README.md`
  and `packaging/pack.ps1` (a local pack for the install).

## Deliverables

### Part A (now, no screen)

1. A checklist in this card, in run order, each step with: what Haik does (exact clicks), what the worker captures
   (tool, region, file name), what to look for (the finding ID or rule it checks), and how to put the setting back.
   It covers at least:
   - the tray glyph in its three states (holding in Moss, idle, off) beside Windows' own glyphs, on the dark and the
     light taskbar and in high contrast (Night sky and Desert), at 100% and 150% scale (T1 to T8);
   - the menu with the clock, light and dark, main and submenu, a long status line (M2, M4);
   - a notice from the installed build: header icon and name, light and dark (N1, N2);
   - Start: pinned grid, All list, a search for "Wakeshot", light and dark Start (A1 to A5);
   - Settings: Apps > Installed apps and Apps > Startup;
   - Explorer: the install folder in Large and Extra large icons and Details, light and dark (B1);
   - Task Manager: Processes and Startup apps;
   - the installer from a double click (not `--silent`): SmartScreen, the splash or whatever Velopack shows, then the
     first-run notice (I1, I2);
   - the README header and the page in a browser, light and dark (the lockup, the colours), and the social card as a
     link preview if one can be made without posting anywhere public.
2. Before and after checks for Haik's own settings each step touches (theme, high contrast, scale, startup entries),
   with the exact command or Settings path to read each value.
3. The order that keeps the session short (group by theme and scale), an estimate per block, and where Haik can stop
   and resume.
4. Which build to install: a local x64 release pack of the current commit into a scratch folder, installed as the
   real-install guard allows; after the session, uninstall it (default) or keep it for the Phase 7 field test (a
   question for Haik with that default).

### Part B (with Haik)

5. Run the checklist with Haik. Captures go to `docs/research/img/brand/screen-*.png`. Each finding: what was seen,
   its severity, and the card that fixes it. Put every setting back and check it.

## Acceptance checks

- Part A: every item of brand-review's 8-item list and of deliverable 1 has a step; every touched setting has a
  before and after read.
- Part B: every step captured or marked "not shown" with the reason; Haik's settings read equal before and after.

## Out of scope

Fixing anything found (new cards); `p6-media` Part B captures (they follow this session); any code change. Do not run
git.

## Part A: the checklist

Written 2026-11-05 16:0x. `<S>` is `<scratchpad>\p6-brand-screen-session`. Captures go to
`docs/research/img/brand/screen-*.png`; raw grabs and enlargements stay in `<S>\grabs`.

### Ground rules

- The laptop as found at 15:56: **only the laptop screen** (1920x1200, 125%, DPI 120, primary, taskbar and tray on
  it), **on battery** (86%, no dock). So "the monitor with the tray" is the laptop screen; its scales are 100%, 125%
  (Haik's own) and 150%. If the DELL comes back mid-run, note it and keep the tray on the screen it is on.
- Who does what: Haik is away, so the worker does every step by automation from the elevated agent shell: Settings
  pages through `ms-settings:` and UI Automation (Expand, SelectionItem.Select, Collapse, Invoke), keys with
  `keybd_event`, clicks with `mouse_event`, the tray by its CLI (`wakeshot automatic on|off`, `wakeshot keep-awake
  on|off|1h`), captures with `Graphics.CopyFromScreen` in a per-monitor DPI aware pwsh (helpers in `<S>\lib.ps1`).
  Each step below names the Settings path, so Haik can do it by hand in a rerun.
- Before every capture: no `LogonUI` process (else stop and report). Never stop the display hold (PID 2728).
- **Privacy, the repo is public:** crop every capture to the surface. Never keep the Recommended part of Start, other
  apps' window titles, Task Manager's other rows beyond a few neighbours, or the terminal behind.
- Start the tray at Medium only: `explorer.exe "<file>"` (a `.cmd` for env values). Quit it with `WM_CLOSE` posted to
  its `Wakeshot.Loop` window (`p6-notice-app-icon` decision 9), never by name.
- A throwaway plan copy is active from before the install to the end, so every write of the tray and the hooks lands
  on the copy (`p6-notice-app-icon` step 3).
- The three tray states: holding (Keep awake, or a Claude Code session), idle (Automatic on and nothing to hold), off
  (Automatic off). For idle, the tray starts with `CLAUDE_CONFIG_DIR` set to an empty folder in `<S>`, so the agents'
  own working sessions (this one included) do not hold.

### Run order

| Block | Scale | Theme | Steps | Estimate | Stop and resume |
|---|---|---|---|---|---|
| 0 Prepare | 125% | dark | 1 to 4 | 10 min | anywhere; nothing installed yet |
| 1 Install and dark surfaces | 150% | dark | 5 to 14 | 20 min | after 14 (app installed, tray quit) |
| 2 Light surfaces | 150% | light | 15 to 20 | 15 min | after 20 |
| 3 High contrast at 150% | 150% | Desert, Night sky | 21, 22 | 8 min | after 22 (contrast on) |
| 4 The 100% tray | 100% | Night sky, Desert, light, dark | 23 to 26 | 12 min | after 26 |
| 5 Back to Haik's 125% | 125% | dark | 27 | 3 min | after 27 |
| 6 Teardown | 125% | dark | 28 to 30 | 8 min | no: finish 28 to 30 in one go |
| 7 Page and README | 125% | light, dark | 31 to 33 | 10 min | after `p6-brand-apply-web` is done |

Grouped so each switch happens once: scale 125 to 150 once, light once, contrast on once (Desert, then Night sky),
scale 150 to 100 once, contrast off once (back to light), dark once, scale back to 125 once. About 85 minutes.

### Steps

| # | What is done (exact path) | Captured (region, file) | Look for | Put back |
|---|---|---|---|---|
| 1 | Guard: no `Local\Wakeshot.Tray`, no install traces (as `packaging/e2e.ps1` refuses), no journal, no other keep-awake app. | none | | |
| 2 | BEFORE: `<S>\read-state.ps1 before` (list below). Copy `Custom.theme`, Task Manager's `settings.json` and the Wakeshot `NotifyIconSettings` entry (`reg export`) to `<S>\backup`. | `<S>\state-before.json` | | |
| 3 | `powercfg /duplicatescheme 381b4222-...` named "Wakeshot screen session", `/setactive` it. | | | step 29 |
| 4 | Pack: `packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir <S>\target\x86_64-pc-windows-msvc\release -OutDir <S>\pack -Icon assets\wakeshot.ico` (build already done: `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p wakeshot-cli`, `CARGO_TARGET_DIR=<S>\target`). Copy `Setup.exe` to `<S>\dl\` and give the copy a download mark (`Zone.Identifier`, `ZoneId=3`), so Windows treats it as a file from the internet, as a user's download is. | | | `<S>` deleted at the end |
| 5 | Scale to 150%: Settings > System > Display > Scale (`ms-settings:display`), the laptop screen. | | | step 27 |
| 6 | Install as a double click: `explorer.exe "<S>\dl\...Setup.exe"`. SmartScreen: if "Windows protected your PC" shows, "More info", then "Run anyway". Grab loop from before the start: each new window of SmartScreen and Setup, every 0.3 s, until the tray runs and its notice has gone. | `screen-install-smartscreen.png`, `screen-install-smartscreen-more.png`, `screen-install-progress.png`, then the first-run notice: `screen-notice-dark.png` (the notice and the tray below it) | I1 (what Velopack shows), I2 (SmartScreen), N1 (header icon and name, dark), N2 (body icon) | uninstall in 28 |
| 7 | Keep the icon on the taskbar: set `IsPromoted` 1 in its `NotifyIconSettings` entry (research, "Tray UX: tray icon"). | | | step 28: the entry back as exported |
| 8 | Tray, dark, 150%: the tray from the install (real Claude Code watched: holding). Then quit it and start it with `CLAUDE_CONFIG_DIR=<S>\noclaude` (idle); `wakeshot automatic off` (off); `wakeshot automatic on` and `wakeshot keep-awake on` (holding, Moss). | right part of the taskbar, chevron to clock, each state; one sheet `screen-tray-dark-150.png` (three rows, 1:1 and 4x) | T1 to T7: crisp, centred beside Windows' glyphs, states tell apart, Moss light on dark, off slash inside | |
| 9 | Menu, dark: click the icon. Main menu, then hover Keep awake for the submenu (with the clock). Then the long line: `wakeshot keep-awake 1h` and `wakeshot run -- <S>\nightly-integration-tests.exe ...` (a copy of `ping.exe`), menu again. | menu and the tray below it: `screen-menu-dark-main.png`, `screen-menu-dark-sub.png`, `screen-menu-dark-long.png` | M1 (no colour fringes), M2 (width with two names), M4 (the clock, not the cup) | holds ended in 14 |
| 10 | Start, dark: pin Wakeshot (Start > All > Wakeshot, right click, Pin to Start), open Start (pinned grid); then All; then type "Wakeshot" (search). | the Start window, without Recommended: `screen-start-dark-pinned.png`, `screen-start-dark-all.png`, `screen-start-dark-search.png` | A1 to A5 among Microsoft's icons; B1 (sharp, full size) | step 28: Unpin from Start; `start2.bin` read |
| 11 | Settings > Apps > Installed apps (`ms-settings:appsfeatures`, search "Wakeshot"); Settings > Apps > Startup (`ms-settings:startupapps`). | the Wakeshot row and a few neighbours: `screen-settings-installed.png`, `screen-settings-startup.png` | A5: icon at Settings' size, name, publisher | Settings closed |
| 12 | Explorer: `explorer.exe "%LocalAppData%\HaikAsatryan.Wakeshot\current"`, then Ctrl+Shift+2 (Large icons), Ctrl+Shift+1 (Extra large), Ctrl+Shift+6 (Details). | the Explorer window: `screen-explorer-dark-large.png`, `-xlarge.png`, `-details.png` | B1 (the 256 frame fills the box), A1 | window closed; its view bag stays (Windows' own, the folder goes) |
| 13 | Task Manager (`taskmgr`): Processes (Wakeshot under Background processes), Startup apps. | the Wakeshot row and neighbours: `screen-taskmgr-processes.png`, `screen-taskmgr-startup.png` | A5 | Task Manager closed; its `settings.json` copied back |
| 14 | End the holds (`wakeshot keep-awake off`, the run ends by itself). | | | |
| 15 | Light: Settings > Personalization > Colors > Choose your mode > Light (`ms-settings:colors`, `SystemSettings_Personalize_Color_ColorMode_ComboBox`). Wait for the taskbar to turn light (T8: it once stayed dark). | | T8 | step 26 |
| 16 | Tray, light, 150%: the three states as in 8. | `screen-tray-light-150.png` | T2 (arc on light), T8, the dark Moss `#3B7A2A` | |
| 17 | Menu, light: main, sub, long line as in 9. | `screen-menu-light-main.png`, `-sub.png`, `-long.png` | M1, M2, M4 | |
| 18 | Notice, light: quit the tray, delete `%LocalAppData%\Wakeshot\config.json` (this install made it), start the tray again: the first-run notice. | `screen-notice-light.png` | N1, N2 in light | |
| 19 | Start, light: pinned, All, search as in 10. | `screen-start-light-*.png` | A1 to A5 | |
| 20 | Explorer, light: Large, Extra large, Details as in 12. | `screen-explorer-light-*.png` | B1 | |
| 21 | Contrast Desert: Settings > Accessibility > Contrast themes (`ms-settings:easeofaccess-highcontrast`), Desert, Apply. Tray three states. | `screen-tray-desert-150.png` | T1 to T7 in the system colours | step 25 |
| 22 | Contrast Night sky, Apply. Tray three states. | `screen-tray-nightsky-150.png` | the same | step 25 |
| 23 | Scale 100% (as in 5). Night sky: tray three states. | `screen-tray-nightsky-100.png` | T1 to T3 at 16 px | step 27 |
| 24 | Desert: tray three states. | `screen-tray-desert-100.png` | T1 to T3 | |
| 25 | Contrast themes > None, Apply (Windows goes back to the theme before it: light mode). | | | read: `Control Panel\Colors`, `CurrentTheme`, `Custom.theme` |
| 26 | Light: tray three states. Then Choose your mode > Dark: tray three states. | `screen-tray-light-100.png`, `screen-tray-dark-100.png` | T1 to T3, T8 | mode Dark again |
| 27 | Scale back to 125% (Haik's). Tray three states (20 px, a size not seen before). | `screen-tray-dark-125.png` | T1 to T3 at 20 px | |
| 28 | Quit the tray (`WM_CLOSE`). Unpin from Start. Uninstall as Apps and Features does: the uninstall key's `QuietUninstallString` at Medium. The `NotifyIconSettings` entry back to its export. Task Manager's `settings.json` back. | | | |
| 29 | `/setactive` Balanced, delete the copy. | | | |
| 30 | AFTER: `<S>\read-state.ps1 after`, compare with BEFORE. Delete `<S>\target` and `<S>\pack`. | `<S>\state-after.json` | equal, or each difference explained | |
| 31 | The page in Edge, light and dark: `site/index.html` from a local file (no server needed). | `screen-page-light.png`, `screen-page-dark.png` (first screen) | the lockup, Moss colours, the wordmark font | Edge window closed |
| 32 | The README header: GitHub renders the pushed README only; the local `README.md` is shown through GitHub's own markdown API render if it works without posting, else the pushed page. | `screen-readme-light.png`, `screen-readme-dark.png` | R1, the lockup in both themes | |
| 33 | The social card as a link preview: no tool shows one without posting the link somewhere public, so the card picture itself (`site/social-card.png`) is looked at at preview sizes. | `screen-social-preview.png` if made | R2 | |

### Reads before and after (step 2 and 30)

`<S>\read-state.ps1` writes one JSON file. Each value and how it is read:

| Setting | How it is read |
|---|---|
| Theme mode | `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`: `SystemUsesLightTheme`, `AppsUseLightTheme`, `EnableTransparency`, `ColorPrevalence` (Settings > Personalization > Colors) |
| Theme and accent | `HKCU\...\Themes` `CurrentTheme`; SHA-256 of `%LocalAppData%\Microsoft\Windows\Themes\Custom.theme`; `HKCU\Software\Microsoft\Windows\DWM` (all values); `HKCU\...\Explorer\Accent` (all values); `HKCU\Control Panel\Desktop` `WallPaper` |
| High contrast | `HKCU\Control Panel\Accessibility\HighContrast` `Flags`, `High Contrast Scheme`; `SystemParametersInfo(SPI_GETHIGHCONTRAST)` on or off; `HKCU\Control Panel\Colors` (all values, contrast rewrites them) |
| Scale per monitor | `GetDpiForMonitor` for each monitor (Settings > System > Display > Scale); `HKCU\Control Panel\Desktop\PerMonitorSettings\*` `DpiValue`; `HKCU\Control Panel\Desktop` `LogPixels`, `Win8DpiScaling` |
| Startup entries | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, `RunOnce`, `...\Explorer\StartupApproved\Run` (all values); `HKCU\Environment` `Path` (raw, with its type) |
| Power | `powercfg /getactivescheme`, `/list`; Balanced's lid action and sleep after, AC and DC (`/q` of `SUB_BUTTONS LIDACTION`, `SUB_SLEEP STANDBYIDLE`); SHA-256 of Balanced's `/qh` |
| Tray icon entry | `HKCU\Control Panel\NotifyIconSettings\*`: each entry naming Wakeshot (values; `IconSnapshot` as a hash) |
| Start and taskbar pins | SHA-256 of `start2.bin` (`StartMenuExperienceHost` LocalState); `HKCU\...\Explorer\Taskband` `Favorites` hash |
| Task Manager | SHA-256 of `%LocalAppData%\Microsoft\Windows\TaskManager\settings.json` |
| Notices | `HKCU\...\Notifications\Settings\velopack.HaikAsatryan.Wakeshot` (values) |
| Install traces | the guard's list: install folder, uninstall key, Run `Wakeshot`, RunOnce `WakeshotRestore`, the switch value, Path entry, Start shortcut, `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`, `%LocalAppData%\Wakeshot`, the tray lock |

### Build and install (deliverable 4)

A local x64 release build of the current tree (no Rust file differs from the last commit; the other workers' open
changes are in `site/`, `media/` and `README.md`), packed by `pack.ps1` as version 0.1.0 into `<S>\pack`, installed
per user at Medium from a double click. After the session it is uninstalled (question 1, default taken).

## Questions for Haik

1. After the session, uninstall the local build or keep it for the field test? Default (taken, orchestrator decision
   for Haik's review): uninstall; the field test installs the first real release.

## Decisions

Worker decisions (test design), each with its reason and when to change it.

1. **The laptop screen stood in for the DELL.** The dock was out and the laptop on battery at the start; the tray was
   on the laptop screen, so its 100%, 125% and 150% were used. Change it when a DELL-only picture is wanted (the
   DELL ran at 150% before, so the 150% pictures match it in size).
2. **The Setup copy got a download mark** (`ZoneId=3`), so SmartScreen showed as a user's download would; the second
   install, from the unmarked file, showed what a local file gets (nothing). Change it never: both are recorded.
3. **A plain backdrop window of mine (dark 32 or light 243 grey, topmost, under the menus) sat behind the menus and
   the light notice**, so no other app shows in a public picture. Change it when a picture on a real wallpaper is
   wanted.
4. **States were switched with the installed CLI at Medium** (`wakeshot automatic`, `keep-awake`, `run`), not by
   menu clicks: the same commands, fewer clicks to go wrong. The menu was still opened by a real click on the icon.
5. **Idle came from a tray started with `CLAUDE_CONFIG_DIR` set to an empty folder**: the agents' own sessions work
   all the time, so a tray watching the real folder never goes idle.
6. **Settings and Task Manager were captured in dark only, Start's List view in dark only**: the card asks light and
   dark for Start and Explorer, which were done; the List view needs Haik's View setting changed and back each time.
7. **Haik's own settings that Windows changed as a side effect were put back to the BEFORE values** (`Custom.theme`,
   `StartupApproved\Run`, the `HighContrast` key, `PerMonitorSettings`, two classic colours), each from a backup or a
   value read before; the session exports of the changed state stay in `<S>\backup`. `start2.bin` was not copied
   back: Start had it open, and the pinned items read back the same.

## Found outside scope

Findings from the captures, by severity, with the ID from `docs/research/brand-review.md` where one fits. None is
fixed here.

| # | Finding | Severity | Picture | File that would change |
|---|---|---|---|---|
| 1 | **Task Manager shows the old violet Sunrise icon** on Processes and Startup apps. Its own cache of this install path, which held older builds: the shell's icon read of the same exe is Moss (research, "Brand screen session"). Not what ships. Retake both pictures after Haik's next sign-out; no code change. | screen session (retake) | `screen-taskmgr-processes.png`, `screen-taskmgr-startup.png` | none (pictures only) |
| 2 | **Idle reads as a person glyph** (judgement): a ring over a line is close to Windows' "account" glyph (head over shoulders), and off then reads as "person blocked". Holding, a filled light, does not have this. Belongs in Haik's 2 to 3 person check (T6). | should check (T6) | `screen-tray-*-*.png`, idle and off rows | `crates/wakeshot-windows/src/tray/art.rs` if the check fails |
| 3 | **The tray button's screen reader name keeps the old tooltip**: after a state change UI Automation gives "Wakeshot / Not holding, the PC sleeps as usual Wakeshot / Holding awake: ..." (old text, then new), while the tooltip on screen shows only the new text. Whether Narrator reads the old part, and whether Windows or our `NIM_MODIFY` causes it: not verified. | should check | none (UI Automation read, `<S>\session.log`) | `crates/wakeshot-windows/src/tray/icon.rs` if ours |
| 4 | **In Night sky the menu's status line is button text yellow, like the commands** (`Palette::for_theme`: `secondary` = `button_text`), so a line that cannot be picked looks like one that can. Windows' `WINDOWTEXT` (white) or `GRAYTEXT` would set it apart; which one WinUI uses there is not verified. | nice to have | `screen-menu-nightsky-main.png` | `crates/wakeshot-windows/src/tray/menu_paint.rs` |
| 5 | **Off at 20 px (125%): the slash runs down through the horizon line**, so off looks busier than idle and holding (T7 at one size; at 16 and 24 px it stays inside). | nice to have (T7) | `screen-tray-dark-125.png` | `art.rs` |
| 6 | **Two green plates in Start's pinned grid**: on Haik's Start, Wakeshot sits under Xbox's green plate. A plate is still a plate among Microsoft's free shapes (A1), though the depth and glow answer A2. Judgement, for the next brand look. | nice to have (A1) | `screen-start-dark-pinned.png`, `screen-start-light-pinned.png` | none now |
| 7 | **Velopack's progress window is up about half a second**, drawn light in dark Windows, with a generic information icon and an OK button. Not ours to change; it also means a splash would show for half a second, so "no splash" (`p6-brand-apply-web` decision 7) stands. | nice to have (I1, closed) | `screen-install-progress.png` | none (Velopack) |
| 8 | **SmartScreen: "Unknown publisher"** for a downloaded `Setup.exe`, as expected for v1 without signing (I2, decision 44). | outside scope (I2) | `screen-install-smartscreen*.png` | none in v1 |
| 9 | **The second install's first-run banner came 6 s after the tray logged it** (the first install's came at once). Cause not checked; if it repeats on a clean PC, a user may think nothing happened. | note | none (`<S>\grabs\install2-notice`) | none yet |

Seen and fine (no finding): T1, T2, T3 at 16 px (a solid ring, no loose specks), T4 (ink centre with Windows'
glyphs at 100%, 125% and 150%), T8 (a true light taskbar, the dark Moss light reads), the high contrast glyphs in
system colours, M1 (grey text, no fringes), M4 (the clock), N1 (header icon is the Moss plate, sharp at 150% and
125%), N2 (lit body icon), B1 (the 256 frame fills Extra large and Large), A4 (the 36 px list icon is clean), A5
(Start, Settings and the taskbar button show the new icon; Task Manager see 1). M2: the long line makes the menu 708
px wide at 150% (472 epx), against about 410 px for Explorer's own menu; it stays Haik's call (`p6-menu-text-look`).

Outside the brand: closing the menu with Esc leaves a keyboard focus box on the icon until another taskbar click;
that is Windows returning focus to the notification area, as its guidance asks. Not a finding.

## Log

- 2026-11-05 15:5x: worker started (Part A, then Part B without Haik, as the orchestrator relayed). Read the card,
  `brand-review.md`, the apply cards, `p6-real-install-guard`, `p6-screen-reruns`, `p6-notice-app-icon` Part B,
  `packaging/`.
- 2026-11-05 15:56: found only the laptop screen (1920x1200, 125%, DPI 120, primary), on battery 86%, no dock; theme
  dark, contrast off. Release x64 build into `<S>\target` done 15:58 (exit 0, 2 min).
- 2026-11-05 16:0x: Part A written (section "Part A: the checklist"). Part B starts.
- 2026-11-05 15:59: steps 1 and 2. Guard clear (no tray lock, no install trace, no journal, no keep-awake app).
  BEFORE in `<S>\state-before.json`; backups in `<S>\backup` (`Custom.theme`, Task Manager `settings.json`,
  `start2.bin`, the Wakeshot `NotifyIconSettings` entry, `Control Panel\Colors`, `Taskband`). Seen open: Haik's own
  Task Manager window (pid 42376) and a Notepad; left alone.
- 2026-11-05 15:59 to 16:01: step 4 pack exit 0 (Setup.exe 12,327,360 bytes); step 3 copy
  `0fcc6190-1bed-4347-a97a-e28c233590dd` "Wakeshot screen session" active; step 5 scale 125% to 150% through Settings
  (DPI 144 read back).
- 2026-11-05 16:01:20: step 6. `explorer.exe` on the marked Setup copy: SmartScreen "Windows protected your PC"
  (captured), More info at 16:02:22 (captured: "Unknown publisher"). **A real input came at 16:02:31.97 (not mine:
  my scripts only used UI Automation by then) and SmartScreen closed: someone clicked Run anyway; Haik may be back.**
  Install 16:02:37, install hook 16:02:39.0, the tray started (pid 19344), FirstRun notice 16:02:39.3, held for
  Claude Code (writes on the copy only). The notice was grabbed (`screen-notice-dark.png`). The Velopack progress
  window was not grabbed: my window loop got the process list as one string (fixed); a second install after the
  uninstall will grab it.
- 2026-11-05 16:05 to 16:13: step 7 `IsPromoted` 1 put the icon on the taskbar within 3 s. Step 8 tray dark 150%:
  holding (Claude Code), then tray restarted with `CLAUDE_CONFIG_DIR` empty: idle, off, holding (Keep awake), each
  grabbed with ink centres measured (`<S>\session.log`). Step 9 menus on a plain dark backdrop window of mine (so no
  other app shows): main, submenu (opened by hovering Keep awake), long line (`wakeshot run` of a `ping` copy named
  `nightly-integration-tests` plus Keep awake 1 h). The minimized `wakeshot run` console also showed the app icon as a
  taskbar button ("Wakeshot command line"): grabbed. The ping copy stopped by its pid.
- 2026-11-05 16:13 to 16:27: step 10 Start, dark: search "Wakeshot" grabbed (crop without the Photos and Documents
  results), Options > Pin to Start, pinned grid grabbed (crop without Recent), All in Haik's Category view grabbed;
  View set to List for the W rows, grabbed, then set back to Category (seen; the Start key's `Config` hash equal to
  before). Step 11 Settings: Installed apps (filtered "Wakeshot") and Startup grabbed, crops without the account
  panel. Step 12 Explorer, install folder: Details, Large icons, Extra large icons grabbed (crops without the nav
  pane); window closed. Step 13 Task Manager: Haik's own open window (pid 42376, was on Performance) used for
  Processes and Startup apps, then put back on Performance. **Task Manager shows the old violet Sunrise icon** for
  both pages; the shell's own read of the same exe (`SHGetFileInfo`, small) and the file's icon are Moss (average
  colour 120,158,88 and 124,158,86). So it is Task Manager's own cache of this path, which held older builds.
- 2026-11-05 16:24 to 16:27: step 15 Light through Settings: the taskbar turned light at once (T8 seen).
  Steps 16 to 20: tray three states, menus (main, sub, long) on a light backdrop, notice light (tray quit, the test
  install's `config.json` deleted, tray started: FirstRun 16:25:50.45), Start pinned, All, search, Explorer Extra
  large, Large, Details (the folder's view was Details when first opened, and is left on Details).
- 2026-11-05 16:27 to 16:35: steps 21 to 27. Desert applied (`hcwhite.theme`), tray three states and the menu;
  Night sky applied, the same. Closing a menu with Esc leaves a focus box on the icon (Windows' own keyboard focus
  after the menu); a click on an empty part of the taskbar clears it, so later grabs do that first. Scale 150% to
  100%: Night sky, Desert. Contrast None at 16:32: **Windows went back to `Custom.theme` in dark mode, not to the light
  mode set before** (`Custom.theme` hash unchanged, so the light switch was never saved in it). So dark 100% came
  first, then Light, light 100%, then Dark again. Scale back to 125% (DPI 120 read back): dark 125% tray. After
  contrast: two classic colours differ from BEFORE (`Control Panel\Colors` `Hilight` 0 120 212 to 0 120 215,
  `MenuHilight` 0 120 212 to 51 153 255); put back in teardown.
- 2026-11-05 16:36 to 16:41: step 28 to 30. Holds ended, tray quit (journal and RunOnce gone, exit 0), Unpin from
  Start (pinned grid seen as before, 11 items), uninstall by `QuietUninstallString` at Medium (exit 0; folder, key,
  data folder gone). Second install from the unmarked `Setup.exe` by `explorer.exe`: no SmartScreen at all; Velopack's
  progress window grabbed (up 16:37:41.7, gone by 16:37:42.1: about half a second); FirstRun logged 16:37:42.2, the
  banner on screen only from 16:37:48.2 (6 s later; cause not checked). Tray quit, second uninstall (exit 0). Then
  put back: `IsPromoted` deleted from the icon entry; `Hilight` and `MenuHilight` (registry and `SetSysColors`);
  Balanced active, copy deleted. AFTER showed more Windows writes made during the session, all put back to the
  BEFORE values: `Custom.theme` (Windows had rewritten it to `SystemMode=Light` at the 100% light switch and did not
  write Dark back; copied back from the backup), `StartupApproved\Run` (viewing the Startup pages added a value for
  each of Haik's six enabled Run apps and turned Docker's `01` into `03`; the six deleted, Docker back to `01`, the
  BEFORE hash matched by computing it), the `HighContrast` key (Windows kept "High Contrast White" as the last scheme;
  values set back), `PerMonitorSettings\LEN41B70...` (`DpiValue` 0, made by the scale change; deleted, as before).
  The second AFTER (16:40:34) equals BEFORE but for `start2.bin` (rewritten by pin and unpin; same pinned items),
  Windows' notice counts for the app id (19 to 22) and the battery. `<S>\target`, `<S>\pack`, `<S>\dl` deleted.
- 2026-11-05 16:42 to 16:44: steps 31 to 33 (`p6-brand-apply-web` read done). The page from its local file in an Edge
  app window on a throwaway profile in `<S>`, light (`--blink-settings=preferredColorScheme=1`) and dark. The new
  README header is not pushed (GitHub's copy still starts `# Wakeshot`), so GitHub cannot show it: a local stand-in
  page with GitHub's page colours and the README's header block as written was shown light and dark instead. The
  social card has no link preview without posting the link somewhere public: not shown; the card looked at 400 px
  wide instead (lockup and line readable). The throwaway Edge profile pulled in one of Haik's extensions (188 MB);
  deleted. Facts in `docs/research.md`, "Brand screen session: what Windows shows".
- 2026-11-05 16:5x: orchestrator note taken: the Task Manager pictures show the old icon from Task Manager's cache;
  no sign-out or reboot here; retake after Haik's next sign-out. The art fix round (bloom over the bar, large frames)
  that starts now does not call for a retake in this card.

## Result

**Done 2026-11-05**, 15:55 to 16:5x, Part A then Part B, all by automation (Haik away; one real click on SmartScreen's
Run anyway at 16:02:32 was not mine: Haik's, he confirmed at 19:50). Laptop screen only, on battery 86% to 72%.
Build: local x64 release of the tree at 15:58 (rim `#F7DC7F`; the later rim and bloom changes are not in it).

Captured (46 pictures, 4.8 MB, `docs/research/img/brand/screen-*.png`):

| Surface | Pictures |
|---|---|
| Tray, three states each, 1:1 and enlarged | `screen-tray-{dark,light,desert,nightsky}-{150,100}.png`, `screen-tray-dark-125.png` |
| Menu | `screen-menu-{dark,light}-{main,sub,long}.png`, `screen-menu-dark-100.png`, `screen-menu-{desert,nightsky}-main.png` |
| Notice (first run) | `screen-notice-dark.png`, `screen-notice-light.png` |
| Start | `screen-start-{dark,light}-{pinned,all,search}.png`, `screen-start-dark-list.png` |
| Settings | `screen-settings-installed.png`, `screen-settings-startup.png` |
| Explorer, install folder | `screen-explorer-{dark,light}-{large,xlarge,details}.png` |
| Task Manager | `screen-taskmgr-processes.png`, `screen-taskmgr-startup.png`: **old icon from Task Manager's cache, not what ships** (Found outside scope 1) |
| Taskbar button (the `wakeshot run` console) | `screen-taskbar-button-{dark,light}.png` |
| Installer | `screen-install-smartscreen.png`, `screen-install-smartscreen-more.png`, `screen-install-progress.png` |
| Page and README header | `screen-page-{light,dark}.png`, `screen-readme-{light,dark}.png` (a local stand-in, not GitHub) |

Not shown, with the reason: the README on GitHub itself (the new header is not pushed); the social card as a link
preview (no tool shows one without posting the link publicly; the card was looked at 400 px wide); the taskbar pin
(the running `wakeshot run` console shows the same icon at the same size, so no pin was made); Settings, Task
Manager and Start's List view in light (dark only, decision 6).

Findings: see "Found outside scope" (1 retake, 2 should check, 5 nice to have, 1 note). No blocker. The brand
review's T1 to T4, T8, M1, M4, N1, N2, B1, A4 and A5 read as fixed on screen.

Haik's settings: BEFORE 15:59:34, AFTER 16:40:34 (`<S>\state-before.json`, `<S>\state-after2.json`): equal for the
theme mode, `Custom.theme`, accent and DWM values, wallpaper, contrast (off, key values), `Control Panel\Colors`, the
scale (125%, DPI 120; no `PerMonitorSettings` key), Run, RunOnce, `StartupApproved\Run`, user Path (type and value),
the active plan (Balanced, the only plan), Balanced lid 1 and 1, sleep 18000 and 18000, its `/qh` hash, the Wakeshot
`NotifyIconSettings` entry, `Taskband`, Task Manager's `settings.json`, the Start view (Category) and no install
trace. Differ, explained: `start2.bin` (rewritten by pin and unpin; the same 11 pinned items seen), Windows' notice
counts for the app id (19 to 22), the battery. Several of the equal values needed putting back after Windows changed
them as side effects (Log 16:36 to 16:41, decision 7).

Acceptance checks:

- Part A: every item of `brand-review.md`'s 8-item list and of deliverable 1 has a step (steps 6 to 33); every
  touched setting has a before and after read ("Reads before and after"): **pass**.
- Part B: every step captured or marked "not shown" with its reason (above): **pass**. Haik's settings equal before
  and after: **pass**, with the three explained differences above.

Files: this card; `docs/research.md` (new section "Brand screen session: what Windows shows", at the end); the 46
pictures. No code. Scratch: `<S>` keeps the scripts, logs, reads, backups and raw grabs; `target`, `pack`, `dl` and
the Edge profile deleted.

Open items:

1. For Haik: retake the two Task Manager pictures after his next sign-out (Found outside scope 1).
2. For Haik's 2 to 3 person check: idle as a person glyph (Found outside scope 2, T6).
3. To check: the tray button's screen reader name (Found outside scope 3), with Narrator.
4. Fix cards to weigh: Night sky status line colour (4), off slash at 20 px (5).
5. The README header on GitHub and the social card preview: look once they are pushed.

Pictures to look at first: `screen-tray-dark-100.png` (16 px, the hardest size), `screen-start-dark-pinned.png`
(beside Microsoft's icons), `screen-explorer-dark-xlarge.png` (the 256 frame), `screen-notice-light.png` (header and
body icon), `screen-install-smartscreen-more.png` with `screen-install-progress.png` (a new user's first seconds).
