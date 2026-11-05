# Real captures of the installed app for the video and the README (`p6-media` Part B, captures only)

Phase: 6. Status: done. Needs Haik: he stays off the PC while it runs (Haik 2026-11-05 ~21:50: "Dell is
connected go on"). Agent: `wakeshot-worker`. Size: about 100K tokens.
Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-media` Part C's first draft (done); `p6-menu-status-glyph` (in the installed build).
Parallel with: `p6-media` Part C round 2 (Remotion renders in `media/`, no screen). Nothing else that uses the screen,
the theme, the scale or the power settings.
Real laptop: yes. The screen, theme and scale switches, the installed Wakeshot's states, its real holds on a throwaway
plan copy. Haik's own settings read before and put back after.
Build folder: `<scratchpad>\p6-media-captures\` (all pictures and logs; nothing in `media/` this round).

## Goal

The video (`docs/research/promo-video.md`, "What Part B captures from the real app") and the README need real stills
of the installed app. The video's drawn parts move; the captures carry the proof. Haik's note on the first draft
(2026-11-05): "Show exactly that Claude Code starts to work and it becomes green, then it finishes it goes back."
So the captures must let the video show, in one fixed screen rectangle, the icon idle, then holding (green) when a
real Claude Code session works, with its tooltip, then idle again.

## Read first

- `docs/research/promo-video.md`: "Real captures and drawn parts" (the sets, ids S1 to S9, whole-number zoom, rows).
- `docs/tasks/p6-brand-screen-session.md`: how a screen session runs safely here (guard, the throwaway plan copy, the
  tray started at Medium through `explorer.exe`, quit by `WM_CLOSE` to its `Wakeshot.Loop` window, never by name, the
  idle state with `CLAUDE_CONFIG_DIR` pointing at an empty folder, states by the installed CLI, theme and scale
  switches read before and restored after, per-monitor DPI aware capture). Its scripts are in
  `<scratchpad>\p6-brand-screen-session\*.ps1`: reuse them, copied into your build folder.
- `CLAUDE.md` "Test laptop" and "Facts to keep in mind" (elevated agent shells: start the tray and the CLI at Medium).
- `media/data/shots.json` (the format Part C uses for captures and row boxes) and `media/README.md` decision 4.

## Deliverables

1. **State before and after**: theme, scale of each monitor, the active power plan and Balanced's lid action and sleep
   timers (AC and DC), the tray icon's `NotifyIconSettings` entry, the installed app's `config.json`. Equal after, or
   each difference explained.
2. **The video sets, dark, at 200% on the DELL** (Haik's answer; set by script and put back if it is not already;
   if the tray is not on the DELL's taskbar, use the screen that has it at its highest scale and say so). One fixed
   screen rectangle per set, cursor not in the picture, nothing private in view (no names, no message counts; crop or
   cover):
   - **Work set** (Haik's note): the tray icon idle (S1), holding while a real Claude Code session works (S2), the
     holding tooltip "Holding awake: Claude Code (1 session)" (S3), and the idle tooltip (S4); S1 also serves as
     "idle again". This session's own Claude Code process is the real session (it stays busy while any agent runs);
     the idle pictures come from a tray started with `CLAUDE_CONFIG_DIR` at an empty folder, as the screen session
     did. Same rectangle and scale for all, so the video swaps them in place.
   - **Menu set**: menu closed, icon idle (S1); menu open, no hover (S5a); Keep awake hovered with its submenu open
     (S5b); "1 hour" hovered (S5c); menu closed after the click, icon holding (S5d). Then Keep awake off again.
   - **S7**: the tooltip while `wakeshot run -- <a short command>` holds (the command name is what shows; pick a neutral
     one such as `npm run build` in a scratch folder with a stub script, nothing private).
   - **S9**: before and after values of the lid action and the sleep timers from the hold's log (on the plan copy).
   - For each menu capture, the rows' boxes in capture pixels (name, x, y, w, h), measured by a script.
3. **README stills**: the menu open, the holding tooltip, the idle tooltip, light and dark, at the same scale.
4. A manifest `<build folder>\captures.json`: each file, set, id, monitor, scale, rectangle, theme, time, and the rows.
   One contact sheet of all pictures for the orchestrator.
5. Log as you go; Result with the before and after table, the files, and anything that did not work.

## Rules

- Keep the screen awake for the session with a display-only request (`SetThreadExecutionState` with
  `ES_CONTINUOUS | ES_DISPLAY_REQUIRED` from a process you start; stop it by PID at the end).
- Kill only processes you started, by PID. Never quit the installed tray by name; never kill Claude Code.
- If Haik comes back to the PC mid-session (real input you did not make), pause and note it in the Log.
- Never run git. No change to `media/`, the app code or any doc but this card.

## Out of scope

The video build and its `media/` files (Part C), speed numbers (Haik starts them with no Claude Code session), the
morning notice (S6), the Task Manager retake.

## Questions for Haik

None.

## Found outside scope

- `<scratchpad>\review-install\quit-tray.ps1` (the orchestrator's helper) never quits the tray: it passes `$null` as
  the window name to `FindWindowEx`, PowerShell marshals that as "", and the `Wakeshot.Loop` window has a title, so
  it posts to 0 windows and ends "still running" (exit 1). Fix: `[NullString]::Value`, as
  `<scratchpad>\p6-media-captures\quit.ps1` does.
- `media/data/shots.json` (Part C's file, not touched): its stand-in rows (`lidClosed`, `settings`, `about`) and
  scale 150 do not match the real app. The real menu is status, Automatic, Keep awake, Restore my power settings,
  Open log, Quit; the submenu Off, 30 minutes, 1 hour, 2 hours, 4 hours, Until I turn it off, Keep screen on; the
  captures are at 200%. `captures.json` has the real rows and rects for Part C to take over.

## Log

- 2026-11-05 ~21:50: card written by the orchestrator.
- 2026-11-05 21:58: worker started; reading the screen session's card and scripts.
- 2026-11-05 22:10: found: DELL U2723QE is `\\.\DISPLAY2`, primary, 3840x2160 at 150% (its recommended), taskbar
  and tray on it; laptop screen `\\.\DISPLAY1` at 125%; on AC 93%; dark mode. The installed tray (pid 40884, guard
  40936) holds for Claude Code and has Balanced at lid Do nothing and DC sleep Never (its own journal). The Wakeshot
  icon sits in the overflow (no `IsPromoted`). TrafficMonitor (Haik's taskbar plug-in) sits left of the tray chevron.
  Scripts copied from the screen session into the build folder; display hold pid 16868 (mine). Scale switch by
  `DisplayConfigSetDeviceInfo` (the call Settings makes; read back by its get call, which matched `GetDpiForMonitor`
  on both screens), so no Settings window has to pick the monitor.
- 2026-11-05 22:04 to 22:06: phase A. **The first quit missed**: the orchestrator's `quit-tray.ps1` passes `$null`
  as the window name to `FindWindowEx`, PowerShell sends "" and no window matched ("posted to 0 windows"); my script
  went on and made the plan copy while the tray still had its own values on Balanced, so the copy got lid 0 and DC
  sleep 0. The tray saw the plan switch and put Balanced back itself (log 22:05:16: lid 0 to 1, DC sleep 0 to 18000;
  "apply to plan <copy>: wrote nothing"). Fixed: quit by my `quit.ps1` (`[NullString]`), tray ended 22:06:04 (code
  0, journal gone), Balanced active, first copy deleted, BEFORE read again with the tray off, fresh copy
  `8c83c295-7be7-4fec-b238-a9e88f9c7d43` "Wakeshot capture session" active (lid 1/1, sleep 18000/18000, as
  Balanced). `IsPromoted` 1 written to the icon entry `2334163346184210159` (it had only `IconSnapshot`).
- 2026-11-05 22:06:43: DELL to 200% (DPI 192 read back); idle tray (`CLAUDE_CONFIG_DIR` empty) started by
  `explorer.exe`, Medium; dark backdrop window of mine over the right part of the DELL. Grab region 1536,400 to
  3840,2160 for every shot, so every set is cut from the same screen pixels.
- 2026-11-05 22:07 to 22:09: dark. S1; S4 (the first try showed no tooltip: the XAML tray wants real moves, so the
  cursor now steps in from 20 px left); menu S5a, S5b, S5c, click on "1 hour", S5d (holding "Keep awake (1 h
  left)"); `wakeshot keep-awake off` (exit 0, idle again); S7 (`wakeshot run -- npm run build` on a 25 s stub,
  tooltip "Holding awake: npm"; ended by itself); real tray: S2, S3 ("Holding awake: Claude Code (1 session)").
- 2026-11-05 22:09 to 22:14: light. **The first light grabs showed Haik's terminal behind the tray** (the light
  backdrop came up under the dark one, which had been clicked and so sat first among topmost windows): deleted at
  once, never used; `Grab` now refuses and deletes a picture whose backdrop pixels are wrong. Retaken with the
  backdrop checked and raised (`SetWindowPos` HWND_TOPMOST, no activate): S1, S4, S5a, S2, S3 light. Mode by
  `SystemUsesLightTheme`/`AppsUseLightTheme` plus the `ImmersiveColorSet` broadcast; taskbar read 216 light, 32 dark.
- 2026-11-05 22:14: teardown. Tray quit, backdrops stopped, DELL back to 150% (DPI 144), the
  `PerMonitorSettings\DEL42798...` key the change made deleted, `IsPromoted` deleted, Balanced active, copy deleted,
  `Custom.theme` unchanged. AFTER (tray off) equals BEFORE but battery, time and `IsPromoted` (BEFORE was read at
  22:06:05, after my first phase A had written it at 22:05:16; PRE at 22:04:46 shows it absent, as AFTER). Tray
  started normally by `explorer.exe` (pids 26888, 37696, Medium, holding "Claude Code (1 session)", Automatic):
  equal to PRE but pids, time and battery. Display hold pid 16868 stopped. Screen use 22:04 to 22:15, about 11
  minutes. No input that was not mine (the cursor check before each grab never fired; no `LogonUI`).
- 2026-11-05 22:16 to 22:40: built the files (no screen). The dark S4 of 22:08:25 has no tooltip (kept as
  `grabs\S4-dark-notip.png`); the tooltip test grab of 22:07:33 (idle, dark, same region, tooltip up) is S4 dark.
  Tooltip boxes measured by a pixel diff against the same state without a tooltip; menus and rows from
  `GetMenuItemRect` and MSAA names at grab time. TrafficMonitor (its CPU and MEM figures change every second) and the
  clock covered in every file: each row blended between the plain taskbar pixels left and right of the box (a copied
  block showed a lighter box on the light acrylic). First contact sheet showed a half chevron at the README menu's
  left edge and a cut volume icon at the work set's right edge: both rects widened to whole buttons. The battery
  figure in the menu set reads 96% in S1 and 97% in S5a to S5d (real charging): not covered, its box is in the
  manifest. A section of facts went into `docs/research.md` first and was taken out again (the card allows no doc
  but this card; the file is back to its 5141 lines); the facts are below under Result.

## Result

Done 2026-11-05 22:40. All five deliverables made; Haik's settings back and checked; the installed tray running as
found.

**Files** (build folder `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\d7052579-3169-4874-8004-3ff3befa980b\scratchpad\p6-media-captures\`):

- `captures.json`: every file with set, id, theme, what, time, tray state, monitor, scale, rect, source grab; per
  set the rect, taskbar top, button and glyph boxes; tooltip boxes; menu frames and rows (name, text, x, y, w, h)
  in each file's own pixels; the covered boxes; S9.
- `contact-sheet.png`: all 22 files.
- `out\`: work set `work-S1/S2/S3/S4/S7-dark.png` and `work-S1/S2/S3/S4-light.png` (564x240, screen rect
  2986,1920); menu set `menu-S1/S5a/S5b/S5c/S5d-dark.png`, `menu-S1/S5a-light.png` (970x814, rect 2870,1346);
  README `readme-menu-{dark,light}.png` (700x550), `readme-holding-tooltip-{dark,light}.png`,
  `readme-idle-tooltip-{dark,light}.png` (the work rect).
- `grabs\`: the raw region grabs (1536,400 to 3840,2160, nothing covered), for re-cuts. `s9-log.txt`, `session.log`,
  `state-pre/before/after/after-tray.json`, the scripts (`phase-*.ps1`, `cap.ps1`, `scale.ps1`, `build.ps1`).

All on the DELL U2723QE (`\\.\DISPLAY2`, primary, the tray's screen) at 200%, cursor never in the picture (GDI
grab), a plain window of mine behind the tray (dark 32, light 243), TrafficMonitor and the clock covered.

**S9** (the throwaway copy of Balanced, from `wakeshot.log`; the same for Claude Code, Keep awake and `wakeshot run`):

| Setting | Before | While holding | After |
| --- | --- | --- | --- |
| Lid close, plugged in and on battery | Sleep (1) | Do nothing (0) | Sleep (1) |
| Sleep after, on battery | 5 hours (18000) | Never (0) | 5 hours (18000) |
| Sleep after, plugged in | 5 hours (18000) | not written | 5 hours (18000) |

**Before and after** (`compare.ps1`; PRE 22:04:46 with the tray holding, AFTER 22:14:13 tray off, AFTER-TRAY 22:14:52):

| What | PRE | AFTER (tray off) | AFTER-TRAY |
| --- | --- | --- | --- |
| Theme | dark (both values 0), `Custom.theme` hash | same | same |
| Scale | DELL 150% (DPI 144), laptop 125% (120), no `PerMonitorSettings` key | same | same |
| Active plan | Balanced only | same, copy deleted | same |
| Balanced lid AC/DC | 0/0 (the tray's hold) | 1/1 (Haik's) | 0/0 (the tray's hold) |
| Balanced sleep AC/DC | 18000/0 (the tray's hold) | 18000/18000 (Haik's) | 18000/0 (the tray's hold) |
| Icon entry `2334163346184210159` | `IconSnapshot` only | same (hash equal) | same |
| `config.json` | `{"version":1,"automatic":true}` | same bytes | same bytes |
| Tray | pids 40884, 40936, Medium, holding Claude Code | not running | pids 26888, 37696, Medium, holding Claude Code (1 session), Automatic |

AFTER-TRAY differs from PRE only in pids, time and battery (95% to 98%, on AC).

**Checks**

1. State before and after: equal, as the table (pass).
2. Video sets dark at 200% on the DELL: work S1 to S4 (and S7 in the same rect), menu S1, S5a to S5d, S7, S9, rows
   for every menu file (pass). Keep awake off again after S5d (`wakeshot keep-awake off`, exit 0).
3. README stills, light and dark, 200%: menu open, holding tooltip, idle tooltip (pass).
4. Manifest and contact sheet (pass).
5. Log as I went; this Result.

**Choices** (principal designer):

- Scale by `DisplayConfigSetDeviceInfo` (the private call Settings makes), not Settings by UI Automation: with two
  screens, Settings first needs the monitor picked, and the get call reads the value back exactly. Change it when
  Windows changes those private types, or for a rerun by hand: Settings > System > Display, pick display 2, Scale.
- Light and dark by the two registry values and the `ImmersiveColorSet` broadcast: no `Custom.theme` rewrite. Change
  it when a surface does not follow; then use Settings > Personalization > Colors.
- Covers by row blends between the plain pixels on each side, so the swap shows nothing that changes on its own and
  no other app. Change it when Part C wants the raw taskbar: `grabs\` are uncovered.
- S7 cut with the work set's rect, so it swaps with S2 and S3 if the longer cut uses it.

**Facts checked here** (kept in the card, as the card allows no other doc; for `docs/research.md` when the
orchestrator wants them):

- The tray tooltip is a XAML popup (`Xaml_WindowedPopupClass` "PopupHost" of `explorer.exe`). It needs real moves:
  `SetCursorPos` plus one injected move gave the hover box but no tooltip; ten 2 px injected moves from 20 px left
  gave it in 0.5 s; even then two of six hovers gave none, so poll for the popup before grabbing. **Verified.**
- `DisplayConfigGetDeviceInfo` type -3 read the same scales as `GetDpiForMonitor`; type -4 set 200% (DPI 192) and back
  to 150% (144) within 3 s; the change creates `PerMonitorSettings\<monitor>` (`DpiValue` 0 after going back).
  **Verified.**
- Registry light/dark plus `ImmersiveColorSet`: taskbar 32 to 216 and back within 2 s; tray icon, tooltip and menu
  followed; `Custom.theme` not rewritten. **Verified.**
- A new topmost window from a background process came up under an older topmost window that had been clicked;
  `SetWindowPos(HWND_TOPMOST, no move, no size, no activate)` raised it. **Verified.**
- The tray follows a plan switch during a hold: it put the old plan back and applied to the new one (log 22:05:16).
  **Verified.**
- `wakeshot run -- npm run build` shows "Holding awake: npm". The 200% menu: rows 66 px, separators 6 px, main menu
  613x414, submenu 320x474 to its left. **Verified.**

**Open items and findings for Haik**

- The menu set's battery figure changes 96% to 97% between S1 and S5a (real charging during the set); its box is in
  the manifest if Part C wants to mask it.
- The holding icon in S5d is "Keep awake (1 h left)", in S2 and S3 "Claude Code (1 session)"; the icon looks the same.
- `media/data/shots.json` needs Part C to take the real rows, rects and scale from `captures.json` (see Found
  outside scope). Nothing in `media/` was touched.
- No screen pictures were copied into the repo; where they go (`media/public/shots/`, the README) is Part C's and the
  orchestrator's call.
