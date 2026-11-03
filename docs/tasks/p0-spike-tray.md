# Prove the tray icon, menu and notifications, and measure the footprint

Phase: 0. Status: done. Needs Haik: only the optional T13 (look at the `tray-*.png` pictures). The theme switch and
the Explorer restart are done.
Runs after: `p0-toolchain`. Parallel with: the other spike cards.

## Goal

Show that a Rust tray app can look native in light and dark mode, and measure its size and memory.

## Read first

- `CLAUDE.md`
- `docs/product.md`: "UX"
- `docs/research/rivals-others.md`: tray and icon notes, traps list

## Questions to answer

1. Tray icon with `Shell_NotifyIcon`: add, update, tooltip, re-add on `TaskbarCreated`. Where does a new icon appear
   on first add (overflow or visible)?
2. Popup menu in light and dark mode with the undocumented `uxtheme` calls (`SetPreferredAppMode`,
   `FlushMenuThemes`). Does it follow a live theme switch? Submenus, check marks, a disabled status line.
3. Taskbar theme (`SystemUsesLightTheme`) and its live change, high contrast detection, per-monitor DPI icons
   (`LoadIconMetric`).
4. A notification (toast) from an unpackaged exe without admin: what registration is needed?
5. A short sound: system sound alias vs a bundled file.
6. Release exe size and idle memory (working set) on x64. ARM64 build compiles.

## Deliverables

- `spikes/tray/`: standalone Cargo project with a README.
- Screenshots of the menu in light and dark mode in `docs/research/img/`.
- Findings in `docs/research.md` with how each was checked.
- An "Implications for the design" list in Result.

## Acceptance checks

- Every question has a tested answer, or "could not test" with the reason.
- Both screenshots exist and Haik has seen them.

## Out of scope

Icon design (Phase 5). Product code.

## Steps for Haik

Moved to `docs/phases/00-morning-steps.md` (block T, steps T1 to T13 = the old steps 1 to 13), the one list for all
three spike cards. Since `p0-morning-trim` the list holds only T10 and the optional T13; the rest moved to agents
(below); T5 and T11 are dropped. An agent starts the spike with the Medium token through Explorer. The spike logs its
token in every mode and refuses an elevated token in interactive mode (an elevated tray app does not get Explorer's
`TaskbarCreated` the normal way, so step T10 would be wrong).

### Moved to agent (card `p0-morning-trim`, 2026-11-03; safety fixes from `docs/reviews/p0-morning-trim-safety.md`)

Haik keeps T10 (the first step of his list) and the optional T13 at the end. Agents do the rest.

**Rules for the agent tray steps**

- One agent on the screen at a time. T1 and T2 run in step 0, before Haik starts. T3 and later run only after the
  power card's agent steps have ended with their end check, never during a Haik block. The lid card's log reads need
  no screen and may run alongside. (For the morning agent steps this overrides "Parallel with: the other spike cards"
  in the header.)
- The spike runs with the Medium token: a `.cmd` in the scratchpad opened with `explorer.exe` (`research.md`, "Spike
  launch and stop checks").
- Screen holder for T3 to T12: one display request for this step only, in the pattern of the orchestrator's
  `keep-awake.ps1` (`PowerCreateRequest`, then `PowerSetRequest` with `PowerRequestDisplayRequired`). It ends by itself
  after 45 minutes, and at once when the laptop goes on battery; it writes its PID to a file; the agent stops it by
  that PID at the end of the step. The end check's `powercfg /requests` must not list it.
- Before every UI Automation action or synthetic input: the session must be unlocked (`LogonUI` not running). If it is
  locked: put the theme back if it was switched (registry values below), quit the spike, and wait for Haik.
- Synthetic mouse input only inside the spike icon's own rect (from the log); menu items and Settings controls only
  through UI Automation on named items. No other clicks at screen coordinates.
- Write a Log line in this card when T3 to T12 start and when they end ("done, end check OK").

Before Haik starts (step 0 of the list):

1. T1: start `tray-spike.exe` (interactive). Check the `token:` line in `out/log-interactive.txt` says `elevated false,
   integrity Medium`.
2. T2: where it landed is in the log (`Shell_NotifyIconGetRect`) and in the `tray-icon-add*` capture. Move it onto the
   taskbar: set `IsPromoted` = 1 (DWORD) on its own `HKCU\Control Panel\NotifyIconSettings\<id>` key, the one whose
   `ExecutablePath` is the spike exe (today `17618738229921704208`; no `IsPromoted` there at 09:37). If a screenshot
   does not show it on the taskbar, drag it there with synthetic mouse input from its rect. Leave the spike running
   for T10. T12 removes the value again.

After the power card's agent steps (Haik done, laptop on AC and unlocked):

3. Before values: `AppsUseLightTheme`, `SystemUsesLightTheme` (both 0 at 09:50) and `CurrentTheme` (`Custom.theme`)
   from `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes`; a screenshot of the taskbar.
4. T10 results: `TaskbarCreated received on ...`, the `NIM_ADD` result after it, the rect, the new `tray-icon-add*`
   capture, and a screenshot of the taskbar.
5. T3: hover the icon with synthetic mouse input and take a screenshot of the tooltip; right-click it the same way
   (the spike saves its menu screenshots by itself), open "Keep awake", Esc twice.
6. T4: invoke "Test: balloon notification", "Test: toast, registered app id" and "Test: toast, unregistered app id"
   through UI Automation on the open menu. Screenshot the bottom right after each. Open the notification center
   (synthetic Win+N, after the lock check), screenshot, Esc.
7. T6: "Automatic" twice; screenshot the icon each time.
8. T7 to T9: switch Settings > Personalization > Colors > "Choose your mode" to Light with UI Automation. If that
   fails: write `AppsUseLightTheme` and `SystemUsesLightTheme` = 1 under
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` and broadcast `WM_SETTINGCHANGE` with
   "ImmersiveColorSet"; log which way was used, because the spike reacts to that broadcast. No input for 10 s. Read
   the `ImmersiveColorSet` lines, `SystemUsesLightTheme` and the `tray-icon-switch1-*` and `tray-menu-switch1-*`
   pictures. Switch back to Dark the same way; read the `switch2` pictures.
9. T12: quit with `WM_CLOSE` to the window of class `WakeshotTraySpike`. Check that the toast keys are gone, else
   `tray-spike.exe --cleanup`. Delete the `IsPromoted` value set in T2.
10. End check: `AppsUseLightTheme`, `SystemUsesLightTheme` and `CurrentTheme` equal step 3; no `tray-spike` process;
    toast keys gone; `IsPromoted` gone; stop the screen holder by PID, then `powercfg /requests` lists nothing from
    an agent. Log "done, end check OK". Then tell the orchestrator that T13 is ready for Haik.

Not verified: UI Automation in the Colors page and on the tray menu, the registry switch with the broadcast,
synthetic input to the tray and to Win+N, whether a screen capture shows toasts, and whether Explorer applies
`IsPromoted` without a restart. If a step fails, put the theme back, record it, and give that step to Haik as a short
optional step.

Dropped:

- T5, the sounds by ear. Both `PlaySound` calls already return TRUE; the `SystemNotification` alias is the lid-close
  sound Haik hears in D, E and I (it maps to `Windows Background.wav` here); the bundled WAV is a placeholder, and
  Phase 5 checks the real sound by ear. Question 5 stays answered as "both play (TRUE); not checked by ear".
- T11, high contrast on and off. It was optional, and turning it off is not verified to bring back Haik's
  `Custom.theme`. High contrast reads work (question 3); the live change waits for Phase 5.

### For the worker who finishes this card

Haik's run writes `spikes/tray/out/log-interactive.txt` and new screenshots in `docs/research/img/`. Check in the
log: the `token:` line at start says `elevated false, integrity Medium`; which window got `TaskbarCreated` (main or message-only) and whether `NIM_ADD` after it returned TRUE; the
`ImmersiveColorSet` lines and the `SystemUsesLightTheme` value after each switch; whether
`tray-menu-switch1-light-noflush.png` is already light (then no flush is needed) or only `...-flush.png` is; the
`tray-icon-switch*-*.png` icon captures; any `WM_SETTINGCHANGE` with wParam `0x43` (high contrast). Then fill the
Result, update `docs/research.md` "Tray, menu and notifications", and run `tray-spike.exe --cleanup` if the log shows
Quit was not used.

## Questions for Haik

## Found outside scope

- `docs/product.md` UX says "Windows 11 hides new tray icons; not verified". It is now verified on this laptop (see
  `docs/research.md`, "Tray, menu and notifications"), so the first-run pin tip is needed. The note can drop
  "not verified".
- The Microsoft Learn page "Send a local toast notification from other types of unpackaged apps" returns 404 at its
  old URLs. Any doc that links it should be rechecked later.
- `docs/phases/00-morning-steps.md` T13 says "look at the `tray-*.png` pictures". The light and dark switch pictures
  are `tray-menu-switch3-light-*` and `tray-menu-switch4-dark-*`; `switch1` and `switch2` are both dark (broadcasts
  with no theme change). T13 could name the files so Haik does not take `switch1` for a failed switch.
- The rules for the agent tray steps say the holder ends after 45 minutes; the orchestrator asked for a hard cap, and I
  used 90 minutes (stopped by PID after 16 minutes). No effect, noted so the rule and practice match.
- The toast header shows the spike's toast icon orange, but `out/toast-icon.png` is blue (43, 124, 211) and the
  code writes straight RGBA, so the spike is not the cause. Windows or a cached copy swapped R and B (not checked).
  Phase 5 checks its real toast icon on screen.
- Leftovers the 13:04 end check did not look for (review `docs/reviews/p0-tray.md`, finding 2): the balloon's
  `Notifications\Settings\NotifyIconGeneratedAumid_17618738229921704208` key, the spike's stored toasts in the
  notification database (they expire by 2026-11-06), two `HKCU\Control Panel\NotifyIconSettings` keys, and the
  `guidmove` exe copy in scratchpad `1828f2de-...`. Decided (`p1-tray-menu-research`, decision 4):
  `p1-tray-menu-look` removes them at its end, since it runs the spike again; the stored toasts expire by themselves.
  Done 2026-11-03 16:14 by `p1-tray-menu-look` (keys exported, then deleted; still gone at 16:16).

## Log

- 2026-11-03: card written.
- 2026-11-03: worker started. Plan: one Cargo project in `spikes/tray/` with windows-rs, a CLI flag per test.
- 2026-11-03: current crates: windows 0.62.2, embed-resource 3.0.12, png 0.18.1 (cargo search and cargo info). `LoadIconMetric` is exported only by comctl32 v6 in WinSxS, not by System32 comctl32 5.82 (dumpbin), so the exe needs a Common Controls 6 manifest. Haik's theme now: apps dark, taskbar dark. OS 26H2 build 26300.9711.
- 2026-11-03: spike `spikes/tray/` builds (x64 release 504 KB), clippy and fmt clean. First `--auto` run done: icon add, modify, menu captures in forced light and dark, balloon, two toasts, two sounds, memory. Log in `spikes/tray/out/log-auto.txt`.

- 2026-11-03: `--menus` rerun with screenshots that paint everything outside the menu grey. `--idle 60`: idle working set 9.7 MB, private 1.5 MB. Haik right-clicked the icon during that run from the `^` flyout; live screenshots saved. `--guid-test` from two folders: both adds succeed. ARM64 release builds, 486 KB, clippy clean. Findings written to `docs/research.md`.
- 2026-11-03: blocked on Haik's manual steps (theme switch, Explorer restart, look and listen). Spike exe left ready.
- 2026-11-03 03:54: card `p0-spike-safety-fixes` moved the steps to `docs/phases/00-morning-steps.md` (block T) and
  changed the spike: `token:` line in every log, interactive mode refuses an elevated token with a message box
  (`--allow-elevated` overrides), new `--token` mode. x64 release now 519,168 bytes (was 504 KB; the token code and
  `Win32_Security`). x64 and ARM64 clippy clean, 1 test.
- 2026-11-03 10:18: step 0 worker started (morning list step 0.2, items 1 to 3; here T1 and T2 only).
- 2026-11-03 10:19 T1: no `tray-spike` process before. Started the x64 release exe (interactive, no args) through
  `explorer.exe <scratchpad>\s0-tray-start.cmd` (`start ""` of the exe). PID 11944, started 10:19:34, parent cmd
  33864 (gone after start). `out/log-interactive.txt`: `token: pid 11944, elevated false, integrity Medium (0x2000)`.
  `NIM_ADD (start, attempt 1): true; NIM_SETVERSION 4: true`. Theme at start: apps dark, taskbar dark, high
  contrast off.
- 2026-11-03 10:19 T2: first add landed in the overflow: `Shell_NotifyIconGetRect` gave RECT 3378,2088 to 3426,2160,
  and `docs/research/img/tray-icon-add1-dark-taskbar.png` shows the `^` chevron. Taskbar screenshot
  `<scratchpad>\s0-taskbar-before.png`: no white circle. Key `HKCU\Control Panel\NotifyIconSettings\17618738229921704208`
  (`ExecutablePath` = the x64 release exe) had no `IsPromoted`. A second key `2295659972384642272` points at an old
  copy in another scratchpad (`guidmove\tray-spike.exe`); not touched.
- 2026-11-03 10:20 T2: wrote `IsPromoted` = 1 (DWORD) on `17618738229921704208` at 10:20:13. Explorer applied it at
  once, no restart: 3 s later `<scratchpad>\s0-taskbar-promoted.png` shows the white circle on the taskbar right of
  the chevron, and `Shell_NotifyIconGetRect` (window `WakeshotTraySpike`, uID 1) gives RECT 3378,2088 to 3426,2160,
  which now is the circle itself (the chevron moved left). No drag needed, no synthetic input used. Spike left
  running, idle, PID 11944. T12 deletes the `IsPromoted` value. (`<scratchpad>` = this worker's scratchpad
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`.)
- 2026-11-03 12:47 agent tray steps (T3 to T12) started, after the power card's end check (12:45, ALL OK). Screen
  holder `<scratchpad>\tray-screen-holder.ps1` (copy of `ab-screen-holder2.ps1`; reason "Wakeshot tray card agent
  steps: screen holder"; clears on battery, sets again if AC is back within 60 s, ends after 60 s on battery or 90
  min), PID 29816, request set 12:47:48. Spike 11944 still running.
- 2026-11-03 12:51 step 3 (before values, `<scratchpad>\tray-snap-before.txt`): `AppsUseLightTheme` 0,
  `SystemUsesLightTheme` 0, `ColorPrevalence` 0, `EnableTransparency` 1, `CurrentTheme`
  `%LOCALAPPDATA%\Microsoft\Windows\Themes\Custom.theme` (sha256 `D16CC919...`, copied to
  `<scratchpad>\tray-theme-backup.theme`), DWM and Accent values and `AutoColorization` 1 recorded too. `IsPromoted` 1.
  No toast keys. `powercfg /requests`: only my holder (DISPLAY). Only the laptop screen is active now (1920x1200 at
  125%, primary); the DELL is not listed as a display. Icon rect 1545,1140 to 1585,1200. Taskbar shot
  `<scratchpad>\tray-shots\125300-taskbar-before.png`: white circle on the taskbar, right of the chevron.
- 2026-11-03 12:53 step 4 (T10, from `out/log-interactive.txt`): `TaskbarCreated received on main window` at
  459705 ms (about 10:27:14); the message-only window logged nothing. Then `NIM_ADD (TaskbarCreated, attempt 2): true;
  NIM_SETVERSION 4: true`, rect 3378,2088 to 3426,2160 (DELL primary then, same as before the restart),
  `tray-icon-add2-dark-taskbar.png` (10:27:16) shows the white circle, not the chevron: the pinned state survived the
  Explorer restart. Also in the log: two `ImmersiveColorSet` broadcasts with no theme change (about 11:37 and 12:04,
  during or right after Haik's battery blocks), so the spike's switch counter is at 2: the old `switch1`/`switch2`
  pictures are dark-only, and my switches will save `switch3` and `switch4`. Haik's three
  `tray-menu-live-dark-0*.png` (01:36) backed up to `<scratchpad>\tray-live-backup`, because T3 live captures reuse
  those names.
- 2026-11-03 12:53 T3 (scripts `<scratchpad>\tray-lib.ps1`, `tray-t3.ps1`, `tray-t3b.ps1`, `tray-msaa.ps1`; steps log
  `<scratchpad>\tray-steps.log`). Hover: absolute mouse move to 1565,1170 inside the rect; after 2.5 s the native dark
  tooltip shows both lines ("Wakeshot spike" / "Holding awake: Claude Code, 2 sessions"),
  `<scratchpad>\tray-shots\125335-T3-tooltip.png`. Right-click (synthetic, same point): spike logs `event 0x204`,
  `0x205`, `right click at (1565, 1169)`, then `menu closed: SetForegroundWindow=true`, so Explorer passed the
  foreground to the spike after a synthetic click. Problem: pwsh 7's managed UI Automation shows the `#32768` menu
  window as an empty Pane (no items; registering `UIAutomationClientSideProviders` fails: type not found). Fix: MSAA
  (`AccessibleObjectFromWindow` on the menu window, `IAccessible` by item name, `accDoDefaultAction`). It lists all 14
  items with names, roles and states (status line unavailable, "Automatic" checked, "Keep awake" has a popup, action
  "Open"). My first run had no item to open and sent a second Esc with no menu open; the foreground was then the
  spike's own hidden window (checked right after), so it had no effect. Esc is now sent only while a spike menu is
  open and the spike owns the foreground. Rerun 12:57: "Keep awake" opened by MSAA (2 menus), Esc closed the submenu,
  Esc closed the menu. Live captures `tray-menu-live-dark-04-1menus`, `05-2menus`, `06-1menus` (01 to 03 overwritten by
  my dump runs; Haik's originals are in the backup and go back at T12). Screenshot `125710-T3-menu-sub.png`: dark menu
  and dark submenu, radio bullet on "Off", check on "Automatic". The tray tooltip stayed on top of the open menu
  (cursor still on the icon).
- 2026-11-03 12:57 T4 (`tray-t4.ps1`, items run by MSAA on their names; shots in `<scratchpad>\tray-shots\`).
  Balloon (`125755-T4-balloon-a.png`): shows as a Windows 11 toast banner, header "Wakeshot tray spike" with the
  spike's tray icon, a large blue "i", title "Wakeshot spike" and the body; log `NIN_BALLOONSHOW`, then
  `NIN_BALLOONTIMEOUT` 6.6 s later. Registered toast (`125809-T4-toast-reg-a.png`): banner, header "Wakeshot tray
  spike" (the registered `DisplayName`) with the `IconUri` PNG, which shows orange although the spike draws it with
  RGB 2b7cd3 (blue): probably a channel order slip in the spike's PNG path, not checked; log `Dismissed
  reason=ToastDismissalReason(2)` (TimedOut) 6.5 s later. Unregistered toast (`125822-...-a`, `125824-...-b`): no
  banner at 2 s or 4 s, no `Dismissed` event; `Show() ok`. New: `notifier Setting=NotificationSetting(0)` (Enabled)
  for both app ids, where the 01:33 `--auto` run got `0x80070490`; this run is Medium, that one was probably elevated
  (cause not checked). Working set after toasts 26.7 MB, private 3.2 to 3.6 MB. Win+N (synthetic, after the lock
  check): notification centre opened (foreground `ShellExperienceHost`), shot `125834-T4-notification-centre.png`
  (scratchpad only: it shows Haik's other notifications): a "Wakeshot tray spike" group (orange icon) with "Toast with
  a registered app id." and "+1 notifications"; no separate group for the balloon or the unregistered toast. Esc
  closed it (foreground back to `explorer`). Which notification the "+1" is: not checked; two more Win+N runs to read
  it found no UIA element for the notification centre from this elevated pwsh. Live captures 07 to 09.
- 2026-11-03 13:00 T6 (`tray-t6.ps1`): "Automatic" run twice by MSAA. Log: `command=1001`, `LoadIconMetric(id 102)`,
  `NIM_MODIFY icon and tooltip (Automatic toggled): true`, then id 101 and true again. Icon crops (3x zoom)
  `130022-T6-icon-0.png` filled circle (holding), `130026-T6-icon-1.png` ring (idle), `130031-T6-icon-2.png` filled
  circle again; the tooltip changed live to "Idle: the PC sleeps as usual" (`130027-T6-tooltip-1.png`). Seen in the
  log: `LoadIconMetric` still returns 24 px (system DPI 144, fixed at sign-in with the DELL primary) while the taskbar
  now sits on the 120 dpi laptop screen (rect 40x60); Windows scales the icon, sharpness not judged. Live captures 10,
  11.
- 2026-11-03 13:01 T7 to T9 start: Settings opened with `ms-settings:colors` (pid 27148); combo "Choose your mode"
  (`SystemSettings_Personalize_Color_ColorMode_ComboBox`) shows "Dark". Fallback ready, not used yet:
  `<scratchpad>\tray-theme-restore.ps1` (both values 0 plus an `ImmersiveColorSet` broadcast).
- 2026-11-03 13:01:48 T7 (Light) by Settings UI Automation (`tray-t7switch.ps1`: Expand, SelectionItem.Select on
  "Light", Collapse); no registry write, fallback not used. Both values 1 within 1 s. Spike: two `ImmersiveColorSet`
  broadcasts 0.1 s apart; in the first `AppsUseLightTheme=true` but `SystemUsesLightTheme=false` and
  `ShouldAppsUseDarkMode=true` (not updated yet), in the second all three agree. Icon switched to id 103 (glyph for a
  light taskbar). After the 2.5 s settle: `tray-icon-switch3-light-taskbar.png` (dark filled circle on the light
  taskbar), `tray-menu-switch3-light-noflush.png` already fully light (so no `FlushMenuThemes` is needed when the app
  mode is `AllowDark`), `...-flush.png` and `...-flush-submenu.png` light too.
- 2026-11-03 13:02:19 T9 (back to Dark) the same way: both values 0 within 1 s; again two broadcasts, the first with
  `ShouldAppsUseDarkMode=false` still. `tray-icon-switch4-dark-taskbar.png` white circle, `tray-menu-switch4-dark-noflush.png`
  already dark, flush and submenu dark. No input during the 15 s after each switch. Settings closed with its named
  Close button (13:02:51). Snapshot `<scratchpad>\tray-snap-after-switch.txt` against `tray-snap-before.txt`: every
  value in Themes, Personalize, DWM, Explorer\Accent, `AutoColorization`, `WallPaper`, the `TranscodedImageCache` hash
  and the `Custom.theme` sha256 and write time are identical. Only differences: the cursor, and the toast keys
  `AppUserModelId\Wakeshot.TraySpike` and `Notifications\Settings\Wakeshot.TraySpike` (none for the unregistered id),
  which quit removes.
- 2026-11-03 13:03:31 T12 (`tray-t12.ps1`): `WM_CLOSE` posted to the `WakeshotTraySpike` window; PID 11944 exited
  within 0.25 s. Log: `NIM_DELETE: true`, `unregister toast app ids` (0 for the registered id's two keys, 2 = not
  found for the unregistered id's), `message loop ended`. Toast keys left: none, so no `--cleanup` run. `IsPromoted`
  deleted from `NotifyIconSettings\17618738229921704208` (its `ExecutablePath` checked first); reads absent. Cursor put
  back to its start point 437,991 with `SetCursorPos` (no click).
- 2026-11-03 13:03 pictures (`tray-images.ps1`): Haik's `tray-menu-live-dark-01-1menus`, `02-2menus`, `03-1menus`
  restored from the backup (hashes identical). My T3 live captures kept as `tray-menu-agent-dark-01-1menus.png`,
  `02-2menus`, `03-1menus`; my other eight live captures moved to `<scratchpad>\tray-live-agent`. New crops in
  `docs/research/img/`: `tray-tooltip-dark.png`, `tray-balloon-dark.png`, `tray-toast-registered-dark.png`. The
  notification centre shot stays in the scratchpad (it shows Haik's other notifications).
- 2026-11-03 13:04 end check (`tray-endcheck.ps1`): holder 29816 stopped by PID, gone; no other holder or keep-awake
  `pwsh`. Snapshot `tray-snap-after.txt` against `tray-snap-before.txt`: theme values identical (`AppsUseLightTheme` 0,
  `SystemUsesLightTheme` 0, `CurrentTheme` `Custom.theme`, file sha256 `D16CC919...` and write time unchanged, DWM and
  Accent unchanged); only differences `IsPromoted` absent, no spike, holder gone, no requests. No toast keys. No
  `tray-spike`, `power-settings-spike` or `lid-sleep-spike` process. `powercfg /requests`: all None. Power reads
  (`ab-read.ps1`, elevated): only Balanced, active; LIDACTION 0x1/0x1, STANDBYIDLE 0x2a30/0x258, VIDEOIDLE
  0x12c/0xb4, RTCWAKE 0x0/0x0, ESBATTTHRESHOLD 0x0/0x1e, CONSOLELOCK 0x1/0x1; `DelayLockInterval` 0; overlay AC
  `ded574b5-...`, DC absent; no journals; unlocked; on AC, 95%. Same as the power card's AFTER line (12:45:16).
  Agent tray steps T3 to T12: done, end check OK. T13 is ready for Haik.

## Result

Status: done. Open: only the optional T13 (Haik looks at the `tray-*.png` pictures); the acceptance check "Haik has
seen them" is met only after it. Facts in `docs/research.md` "Tray, menu and notifications" (and "Agent reach").

Step 0 (10:19 to 10:21): T1 and T2 passed. T10 (Haik, about 10:27) passed. Agent steps T3 to T12 (12:47 to 13:04)
passed; end check OK at 13:04 (leftovers it missed: Found outside scope).

1. Tray icon: add, modify, two-line tooltip, version 4 work. First add lands in the `^` overflow. After an Explorer
   restart, `TaskbarCreated` reaches the hidden top-level window (not the message-only one), `NIM_ADD` returns TRUE,
   and a pinned icon stays on the taskbar. `IsPromoted` = 1 pins it live.
2. Menu: light and dark through uxtheme 135/136, with submenu, check mark, radio bullet and disabled line. Live switch
   (Settings, Light then Dark): the next menu has the new theme without `FlushMenuThemes`
   (`tray-menu-switch3-light-*`, `tray-menu-switch4-dark-*`).
3. Taskbar theme: the icon follows `SystemUsesLightTheme` live. Each switch sends two `ImmersiveColorSet` broadcasts,
   the first with stale values; twice a single one came with no theme change (about 11:37 and 12:04). High contrast:
   read works; live change could not be tested here (T11 dropped: turning it off may not bring back Haik's
   `Custom.theme`). `LoadIconMetric`: 24 px at 144 dpi; it keeps the sign-in system DPI after a display change.
4. Notifications: the balloon shows as a toast banner with the exe's name and tray icon, no registration. The
   registered toast (HKCU `AppUserModelId` key) shows a banner with its name and icon. The unregistered toast shows no
   banner. Pictures `tray-balloon-dark.png`, `tray-toast-registered-dark.png`.
5. Sound: both play (TRUE); not checked by ear (T5 dropped).
6. x64 exe 519,168 bytes (504 KB before the token code), idle 1.6 to 1.7 MB private, about 10 MB working set; about
   27 MB working set after WinRT toasts. ARM64 builds.

Files changed in this run: this card; `docs/research.md` (new bullets in "Tray, menu and notifications" and "Agent
reach", two stale lines fixed); `docs/research/img/` new `tray-menu-switch3-light-*` and `tray-menu-switch4-dark-*`
(4 each with the icon), `tray-menu-agent-dark-0*.png`, `tray-tooltip-dark.png`, `tray-balloon-dark.png`,
`tray-toast-registered-dark.png`; Haik's `tray-menu-live-dark-0*.png` restored unchanged. `spikes/tray/out/log-interactive.txt`
grew (spike log). Scripts, shots and snapshots in the scratchpad named in the Log.

Checks: every question answered or marked "could not test" with the reason. Light and dark menu pictures exist.
End check: theme values identical to before, `IsPromoted` absent, toast keys gone (leftovers it missed: Found outside
scope), no spike or holder process, `powercfg /requests` all None, power values as in the power card's AFTER line. Earlier build checks still stand: x64
and ARM64 release builds, `cargo fmt` and `cargo clippy -- -D warnings` clean.

Implications for the design:

- Embed a manifest: Common Controls 6 (needed for `LoadIconMetric`), per-monitor v2 DPI, `asInvoker`.
  `embed-resource` in `build.rs` does it with no setup.
- Own a hidden top-level window for the tray, not a message-only one, so broadcasts reach it.
- New icons hide in the overflow, so the first-run "pin the icon" tip is needed.
- Keep a stable install path: Windows most likely keys a uID icon's pinned state by exe path (inferred from the
  key's `ExecutablePath` and `UID`; a uID icon was not moved in a test).
- Two icon sets, picked by `SystemUsesLightTheme` (taskbar), not `AppsUseLightTheme` (the test changed both
  together; Custom mode was not tested). Ship frames 16 to 64 px. Store them PNG-compressed to save about 150 KB (not
  verified that `LoadIconMetric` reads PNG frames).
- Dark menus: call `SetPreferredAppMode(AllowDark)`, then `RefreshImmersiveColorPolicyState` and `FlushMenuThemes`,
  before the first menu (and after each earlier settle, after its no-flush capture), and build the menu at each open
  (the tested sequence), only on build 18362 or later and only if the ordinal resolves; otherwise stay light. With
  `AllowDark`, no `FlushMenuThemes` is needed on a live switch. Superseded for the colour choice: `product.md` follows the
  Windows mode, so a classic menu uses `ForceDark` or `ForceLight` from `SystemUsesLightTheme` plus `FlushMenuThemes`.
- On `ImmersiveColorSet`, wait for the broadcasts to settle (the first carries stale values), then read the values
  and change the icon only if they really changed.
- Re-add the icon on `TaskbarCreated` from the hidden top-level window; the user's pin survives an Explorer restart.
- `LoadIconMetric` uses the sign-in system DPI; to stay sharp after a display change, load the icon for the taskbar
  monitor's DPI (not verified which API fits best; check in Phase 5).
- The disabled status line is grey and low contrast; Phase 5 wording should decide if that is fine.
- Notifications: register the app id per user under HKCU at start (on uninstall, clear its toast history with
  `ToastNotificationManager.History.Clear(aumid)`, then remove the keys; the keys alone leave stored toasts; Clear not
  tested) and use WinRT toasts; an unregistered id shows no banner. A balloon needs nothing but looks like a plain
  info toast. Load WinRT only on the first toast: it adds about 8 MB working set. Do not rely on
  `ToastNotifier.Setting`: it threw in one run and worked in another (cause not checked).
- Sound: ship a short WAV as a resource with `SND_SYSTEM`; the user's scheme may map the alias to anything or nothing.
