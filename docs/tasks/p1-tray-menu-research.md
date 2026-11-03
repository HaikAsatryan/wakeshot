# Tray menu look: desk research

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`.
Runs after: none. Parallel with: `p0-close-results` and the tray Result review (this card writes only its own new
files).

## Goal

Do the desk part of `p1-tray-menu-look` now, while Phase 0 closes: find what makes current Windows 11 tray menus look
modern, what our spike menu lacks, and the leanest way to match. No screen use. The pictures of other apps' menus and
the spike change stay in `p1-tray-menu-look`, which builds on this file.

## Read first

- `docs/tasks/p1-tray-menu-look.md`: Haik's words and the full deliverables.
- `docs/tasks/p0-spike-tray.md` Result. In `docs/research.md` (read only): "Tray, menu and notifications" and
  "Tray menus, synthetic input and the theme switch".
- Our pictures: `docs/research/img/tray-menu-switch3-light-*.png`, `tray-menu-switch4-dark-*.png`, and the other
  `tray-menu-*.png` if useful.
- Spike menu code: `spikes/tray/src/`, `spikes/tray/tray-spike.exe.manifest`, `spikes/tray/build.rs`.
- `docs/architecture.md` (Crates, Processes) and `docs/product.md` (UX: tray menu items, size goals). Read only.

## Deliverables

1. `docs/research/tray-menu-look.md` (new file):
   - Our menu today, measured from our pictures: corners, border, shadow, padding, item height, font and size, check
     marks, submenu arrow, separators, dark colors. And why, from the spike code: which API, flags, manifest entries,
     theme calls.
   - For each app Haik named (Telegram, Wi-Fi quick settings, Windows Security, Chrome, Copilot, and the Explorer
     right-click menu as the Windows reference): which UI stack draws its menu, with a source. Reading the installed
     apps' files to identify the stack (DLL names, package manifests) is fine, read only.
   - Each expected difference: how the others get it, and what it costs us (code, binary size, dependencies, startup
     time, memory, Windows 10 risk if that still matters), with a source.
   - The options in a small table, for example: plain Win32 menu with the right flags, manifest and theme calls;
     owner-drawn Win32 menu; our own popup window; a XAML flyout (WinUI 3 or XAML Islands); WebView2.
   - One pick with the reason, and a "change it when" trigger for the stronger option.
   - "To check on screen": exactly what the `p1-tray-menu-look` worker must confirm with pictures, and which spike
     change to try first.
2. This card's Log and Result.

## Acceptance checks

- Every claim has a source (current Microsoft docs link, app file, or a measured value from our pictures) or is
  marked "not verified".
- Every app Haik named has a stack line or a reason why not.
- The pick names its cost in binary size and dependencies, and says whether it keeps the tray card's checks working:
  `TaskbarCreated` re-add, theme switch without restart, Medium token.

## Out of scope

Strict, because another worker edits the docs right now:

- Write only `docs/research/tray-menu-look.md` and this card. Do not edit `docs/research.md`, `docs/product.md`,
  `docs/architecture.md`, `CLAUDE.md`, `docs/plan.md`, `docs/phases/*`, `docs/reviews/*` or any other card. Facts go
  into the new file; the orchestrator links them from `research.md` later.
- No screen use: do not open menus, run the spike or other apps, take screenshots, send input, or switch the theme.
  Haik is using the laptop.
- No installs, no builds, no power settings, no registry writes, no git.
- Icon design (Phase 5). Menu items and wording (`product.md`).

## Questions for Haik

None. Haik (2026-11-03): design calls are Claude's as principal designer; compare with the tray icons he has now.
Decided:

1. Copilot: Microsoft 365 Copilot (`M365Copilot.exe`), the one in Haik's tray.
2. Wi-Fi: picture both the left-click quick settings flyout and the right-click menu of the network icon.
3. The tray menu follows the Windows mode (`SystemUsesLightTheme`, the taskbar's), like the tray icon. Why: the menu
   opens from the taskbar, next to Windows' own tray flyouts, which follow the taskbar; with a dark taskbar and light
   apps, an app-mode menu would clash with the bar it opens from. Change it when the on-screen pictures show Windows'
   own tray right-click menus follow the app mode.
4. Spike leftovers (two `NotifyIconSettings` entries, `NotifyIconGeneratedAumid_17618738229921704208`, the
   `guidmove` exe copy): remove them at the end of `p1-tray-menu-look`, not now, because that card runs the spike
   again and Windows would write them again. The 4 stored spike toasts expire 2026-11-06 by themselves.

## Found outside scope

Problems seen but not fixed.

- `HKCU\Control Panel\NotifyIconSettings` still lists two spike exe paths: the build output and an old scratchpad
  copy (`...\1828f2de-...\scratchpad\guidmove\tray-spike.exe`). Windows writes these entries itself; they are
  harmless. Decided: decision 4 above.
- `product.md` UX says "The menu follows light and dark mode too" without saying which mode (apps or Windows).
  Decided: decision 3 above.

## Log

- 2026-11-03: card written from Haik's request to run safe work in parallel with the orchestrator's Phase 0 close.
- 2026-11-03: worker started. Read CLAUDE.md, plan "How we work", both tray menu cards.
- 2026-11-03: measured our light and dark pictures with a stdlib PNG reader (no installs). Findings in the new file,
  "Our menu today": corners are rounded (about 4 px at 100%), shadow is masked by `capture.rs`, font Segoe UI 9 pt,
  item pitch 26 px at 125%. Next: Microsoft docs, then the named apps' files.
- 2026-11-03: read Microsoft docs (geometry, typography, rounded corners, DWM attributes) and the WinUI 3 MenuFlyout
  XAML from GitHub. Main gap so far: 8 px vs our 4 px corners, 14 px vs 12 px font, about 33 vs 21 epx item pitch,
  acrylic vs flat background. Written to the new file. Next: identify each named app's UI stack from its files.
- 2026-11-03: stacks found (read-only file and source checks): Telegram own Qt popup; quick settings and taskbar menus
  system XAML MenuFlyout; Windows Security classic Win32 menu (`SecurityHealthSSO.dll`); Chrome Views menu, 12 px
  radius; Copilot here is `M365Copilot.exe`, WinUI 3 MenuFlyout likely; Explorer "XAML context menu". Next: costs of
  each option and the pick.
- 2026-11-03: costs gathered: owner-draw docs (WM_MEASUREITEM, WM_DRAWITEM, WM_MENUCHAR, MSAAMENUINFO), WM_ENTERIDLE
  gives the menu window, Windows App SDK deployment page, Windows App Runtime 2 on disk here 119.5 MB.
  `windows-reactor` already has a tray icon plus WinUI 3 MenuFlyout, but no check or radio items, and needs the
  runtime installed. Writing differences, options, pick.
- 2026-11-03: wrote differences (16 rows), options table (A to E), pick (B, try A first), "change it when", and "To
  check on screen". Next: acceptance checks and Result.
- 2026-11-03: acceptance checks run (below), small fixes to sources and units, Result written. Status done.

## Result

Status: done.

What was done: desk research only, no screen use, no installs, builds, registry or power writes, no git. All facts,
with how each was checked, are in `docs/research/tray-menu-look.md` (new).

Findings in short:

- Our menu is the classic Win32 menu drawn by user32. Measured from our pictures (125%): corners are rounded, but the
  small 4 px radius Windows gives menus; 1 px border; Segoe UI 9 pt (12 px); rows about 21 px apart at 100%; flat
  background; ClearType text. The shadow is not in any picture: `capture.rs` greys everything outside the window rect.
- Modern Windows 11 menus (XAML `MenuFlyout`): 8 px corners, Segoe UI Variable 14 px, rows about 33 px apart,
  rounded hover, acrylic background (Microsoft docs and the WinUI 3 source).
- Stacks: Telegram own Qt popup; Wi-Fi quick settings and the taskbar menus system XAML; Windows Security the same
  classic menu as ours; Chrome its own Views menu (12 px corners); Copilot here is `M365Copilot.exe`, most likely a
  WinUI 3 `MenuFlyout`; Explorer "the XAML context menu" (Microsoft's words), with the classic one behind "Show more
  options".
- Pick: owner-drawn items in the classic menu, plus one DWM call (`DWMWCP_ROUND`) on the menu window; try the DWM
  call alone first. Cost: about 300 to 500 lines, about 10 to 30 KB (est.), no crate, no runtime. Keeps the tray
  checks (`TaskbarCreated` re-add, theme switch without restart, Medium token). Gives up acrylic. Change to a WinUI 3
  `MenuFlyout` (`windows-reactor`) when Wakeshot needs a real window, or `windows-reactor` is stable with check and
  radio items and the runtime installs without a prompt, or the side-by-side shows acrylic still matters.

Files changed: `docs/research/tray-menu-look.md` (new), this card. Scratch only (not in the repo): `png.py`,
`measure.py`, `rows.py`, `detail.py`, `strings.py`, and downloaded source files in
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\7c8299f5-9b89-452b-a899-d64d015f5d2d\scratchpad`.

Acceptance checks:

- Every claim has a source (Microsoft Learn link with its update date, GitHub source path with its last commit date,
  an installed file, or a value measured from our pictures) or is marked "not verified" or "est.": passed, by a full
  read of the new file.
- Every app Haik named has a stack line: passed. Telegram, Wi-Fi quick settings (plus the network icon menu),
  Windows Security, Chrome, Copilot, Explorer. Copilot and the Explorer menu DLL carry "not verified" parts.
- The pick names its cost in binary size and dependencies and says whether it keeps the tray card's checks: passed
  ("Pick" and the "Options" table).

Open items:

- Everything marked "not verified" in the new file, mainly: whether DWM rounds a `#32768` window at 8 px, whether
  Windows draws its own submenu arrow over an owner-drawn item, the real shadow, hover colours. The "To check on
  screen" section lists the steps for `p1-tray-menu-look`.
For the orchestrator (after `p0-close-results` is done, since these touch its files):

- Link `docs/research/tray-menu-look.md` from `docs/research.md` ("Tray, menu and notifications").
- Apply the "Edits for the orchestrator" at the end of `docs/reviews/p0-tray.md` (review of the tray card's Result,
  item 5 of "Before Haik signs off Phase 0" in `docs/reviews/p0-delta.md`).
- `product.md` UX: the menu follows the Windows mode (decision 3).
- `p1-tray-menu-look`: builds on `docs/research/tray-menu-look.md` ("To check on screen"); uses decisions 1 to 4; its
  on-screen test of the owner-drawn menu must cover check marks, keyboard use, high contrast and DPI scaling, the
  things the classic menu gives us for free today.
