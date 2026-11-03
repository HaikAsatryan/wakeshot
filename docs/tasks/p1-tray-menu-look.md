# Make the tray menu look like current Windows 11 menus

Phase: 1. Status: done (Haik's look 2026-11-03 17:32: "I like your new ui, you might need some icons for each row as
examples are using. Everything else is good."). Needs Haik: done. Agent: `wakeshot-worker`.
Runs after: Phase 0 sign-off, `p1-tray-menu-research` (done; it wrote `docs/research/tray-menu-look.md`),
`p1-lid-sound` and `p1-tray-review-fixes` (screen and sound cards run one at a time). Parallel with: any Phase 1 card
that does not use the screen or sound.

## Goal

Haik's verdict on the tray spike menu (2026-11-03, T13): it looks a little old: no rounded corners, and something
else is off. Find what makes current Windows 11 tray menus look modern and pick the leanest way to match them. This
runs in Phase 1, not Phase 5, because the answer may change the stack (for example plain Win32 menus versus a XAML
flyout). Phase 5 does the final polish on top of the pick.

## Read first

- `docs/research/tray-menu-look.md`, mainly "Pick" and "To check on screen" (your step list).
- `docs/tasks/p0-spike-tray.md` Result, and the tray section of `docs/research.md`.
- Our pictures: `docs/research/img/tray-menu-switch3-light-*` and `tray-menu-switch4-dark-*`.
- `docs/product.md` (tray menu items) and `docs/architecture.md` (tray process, binary size goals).
- `docs/reviews/p0-tray-verify.md`: 5 minor doc findings on the tray record. Fix them in the same pass as your own
  edits to those files. Finding 1 matters for your build: the old dark menu recipe (`AllowDark`) follows the app
  mode, but decision 3 below says the Windows mode.

## Decided

From `p1-tray-menu-research`, taken under Haik's delegation of design calls (2026-11-03):

1. Copilot: Microsoft 365 Copilot (`M365Copilot.exe`), the one in Haik's tray.
2. Wi-Fi: picture both the left-click quick settings flyout and the right-click menu of the network icon.
3. The tray menu follows the Windows mode (`SystemUsesLightTheme`, the taskbar's), like the tray icon. Why: the menu
   opens from the taskbar, next to Windows' own tray flyouts, which follow the taskbar; with a dark taskbar and light
   apps, an app-mode menu would clash with the bar it opens from. Change it when the on-screen pictures ("To check on
   screen", step 6) show Windows' own tray right-click menus follow the app mode. Note: the spike's `AllowDark`
   follows the app mode. With owner-drawn items the colours are ours; a plain classic menu needs `ForceDark` or
   `ForceLight` from `SystemUsesLightTheme` plus `FlushMenuThemes` (`research.md`, "Dark menus").
4. Spike leftovers: remove them at the end of this card (deliverable 6), not earlier, because this card runs the
   spike again and Windows would write them again. The 4 stored spike toasts expire 2026-11-06 by themselves.

## Deliverables

1. Pictures of the tray menus Haik named, in light and in dark, taken on this laptop: Telegram, Wi-Fi (quick
   settings flyout and the network icon's right-click menu), Windows Security (Defender), Chrome, Copilot
   (`M365Copilot.exe`), and the Explorer right-click menu as the Windows reference. Open each menu, take the picture,
   close it with Esc. Never click a menu item in another app.
   The repo goes public (orchestrator, 2026-11-03 16:22): before a picture lands in `docs/research/img/`, crop or
   blank out Wi-Fi network names, account names, email addresses, chat names or message text, and anything else
   personal. Keep only what the comparison needs (menu shape, corners, spacing, colours, font). Raw shots stay in the
   worker's scratch folder.
2. A side-by-side page, added to `docs/research/tray-menu-look.md` (start from what `p1-tray-menu-research` wrote
   there; do not redo its toolkit and cost research, check it against the pictures): ours next to each, with every
   visible difference named (corners, shadow, border, padding, item height, font and size, check marks, submenu
   arrow, separators, hover and pressed color, icons in items, dark mode colors, high contrast).
3. For each difference: how the other apps get it and what it costs us (code, binary size, dependencies, startup
   time, memory, risk on Windows 10 if that still matters). Use current Microsoft docs, not memory.
4. One pick with the reason, and a "change it when" trigger for the stronger option.
5. The pick shown working in the tray spike (`spikes/tray/`), with new pictures in light and dark, at 100% and at
   the laptop's own scale. If the pick owner-draws the items, the on-screen test also covers what the classic menu
   gives for free today:
   - Check marks and radio bullets: drawn, in light and dark.
   - Keyboard use: Up and Down, Right and Left into and out of the submenu, Enter, Esc, and mnemonics through
     `WM_MENUCHAR`.
   - High contrast: Haik's theme must not be switched (T11: turning it off may not bring back his `Custom.theme`).
     When `SPI_GETHIGHCONTRAST` is on, draw with system colours (`GetSysColor`), not the fixed WinUI colours; show
     that branch with a spike flag that forces it, and say in the Result that a real high contrast switch was not
     tested.
   - DPI scaling: item height, font, glyphs and corners measured at 100% and at the laptop's own scale.
6. At the end, after the last spike run: remove the spike leftovers. They are the two spike entries under
   `HKCU\Control Panel\NotifyIconSettings` (`17618738229921704208`, the release exe, and `2295659972384642272`, the
   `NIF_GUID` icon), the key
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\NotifyIconGeneratedAumid_17618738229921704208`,
   the copy `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\guidmove\tray-spike.exe`,
   and any new spike entries this card's runs add. Export each key to your scratchpad (`reg export`) before you
   delete it, then check that each is gone and still gone a minute later (Explorer keeps its own copy; whether it
   writes them back is not verified). Leave the stored toasts: they expire by 2026-11-06.
7. Tray review finding 13 (`docs/reviews/p0-tray.md`): crop the non-private parts of `s0-taskbar-before.png`,
   `s0-taskbar-promoted.png` and `tray-shots\125822-T4-toast-unreg-a.png` into `docs/research/img/` and point the
   matching `research.md` lines at them. The orchestrator copied that old scratchpad to
   `%USERPROFILE%\.claude\wakeshot-keep\2026-11-03-tray-scratchpad\` (2026-11-03 15:08); read from there. Crop out
   anything personal (other apps' content, names, chats); the repo will be public.

## Acceptance checks

- Every app Haik named has a light and a dark picture, or a reason why not.
- Each difference in the table has a source (doc link or a measured value from the pictures).
- The new spike pictures fix every difference the pick claims to fix (compare by eye and by pixel measures).
- The spike still passes the tray card's checks: `TaskbarCreated` re-add, theme switch without restart, Medium
  token.
- If the items are owner-drawn: check marks, keyboard use, the high contrast branch and DPI scaling pass as in
  deliverable 5, each with a picture or a log line.
- The leftovers in deliverable 6 are gone, checked by registry and file reads after the last spike run.

## Out of scope

- Icon design (Phase 5). Menu items and wording (product decisions, `product.md`).
- Changing Haik's theme or any setting, except a theme switch through Settings that the tray card already allows,
  put back exactly, and the leftover removal in deliverable 6.

## Questions for Haik

Each with the worker's default.

## Found outside scope

- My mistake, not fixable: the 15:47 run overwrote `docs/research/img/tray-icon-add1-dark-taskbar.png` (Phase 0,
  10:19, chevron at 150%) with a 15:47 capture (chevron at 125%). Same meaning (first add lands in the overflow), other
  scale. `p0-spike-tray.md` Log 10:19 T2 describes the old one.
- Chrome's tray icon has an empty UI Automation name in the overflow; a product note only if Wakeshot ever drives
  other apps' icons (it does not).
- Windows Security's classic menu has 32 epx rows where ours had 21; why (another menu flag or owner-draw) was not
  checked.
- The spike exe grew from 580,608 (15:45) to 676,864 bytes (16:13) with little code change; cause not checked. Phase 5
  should measure the menu code alone in the product build.
- The orchestrator's screen holder and spike hold (PIDs 13064, 15652) were left running, as told.

## Log

- 2026-11-03: card written by the orchestrator from Haik's T13 feedback.
- 2026-11-03 15:05: `p1-tray-review-fixes` added the research decisions ("Decided"), the owner-drawn test items, the
  leftover removal (deliverable 6) and the new "Runs after".
- 2026-11-03 15:31: worker started. Plan: no-screen work first (owner-drawn menu and DWM call in the spike, build,
  tests, the 5 doc fixes), then block for Haik before any screen step.
- 2026-11-03 15:45: spike change built (no screen). New `spikes/tray/src/look.rs`: owner-drawn items with WinUI 3
  MenuFlyout sizes and colours (template values read from microsoft-ui-xaml with `gh api`), `MSAAMENUINFO` item data,
  `WM_MENUCHAR` mnemonics, `DWMWCP_ROUND` on the `#32768` window from a thread CBT hook (and again at
  `WM_ENTERIDLE`), forced high contrast branch, `--render` (draws the items at 96, 120 and 144 dpi into PNGs with no
  window), and test messages for scripts (style, capture, backdrop, shot). `main.rs`: one entry list for the classic
  and the owner-drawn builder; the owner-drawn menu follows `SystemUsesLightTheme` and sets `ForceDark` or
  `ForceLight` plus `FlushMenuThemes` at each open, so the theme-drawn frame matches. x64 release 580,608 bytes (was
  519,168). `cargo fmt --check`, clippy `-D warnings` on x64 and ARM64, 7 tests: all clean.
- 2026-11-03 15:46: orchestrator relayed Haik's go for the screen part while he is away (private place). Laptop on
  battery 86%, only the laptop screen (1920x1200 at 125%), unlocked, theme apps dark and taskbar dark. Orchestrator
  holds DISPLAY (pwsh 13064) and a spike hold with battery sleep Never (15652) until about 18:13; not mine, not
  touched. I add no holder of my own: theirs covers the window, and mine would have to run on battery.
- 2026-11-03 15:47 to 16:12: screen part (scripts and step log `look-steps.log` in my scratchpad
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\4fa582c2-fd10-483c-8e4d-a373ec22347e\scratchpad`).
  Spike started at Medium through `explorer.exe look-start.cmd` (token line: `elevated false, integrity Medium`).
  Mistake: the first `--look` start (15:47, PID 10328) still saved its icon capture into `docs/research/img`, so
  `tray-icon-add1-dark-taskbar.png` from 10:19 (chevron at 150%) was overwritten with today's 15:47 capture (chevron
  at 125%); no backup existed. Fixed at once: a `--look` run writes every picture to `spikes/tray/out/look/` (now in
  `.gitignore`). Pictures: ours in 8 variants, Telegram, network right-click, quick settings, Windows Security,
  Chrome, Copilot, Explorer new and classic (Shift+right-click), in dark, light and Custom (Windows dark, apps
  light). Theme switches through Settings by UI Automation (Settings was minimized; restored it, then minimized it
  again); the snapshot after equals the 15:47 snapshot. Two raw Copilot shots showed Haik's terminal (it came in
  front of my backdrop): deleted at once, backdrop made topmost, and every shot now deletes itself if its corners are
  not the backdrop colour. Keyboard, mnemonic, Enter and MSAA checks on a real right-click; `TaskbarCreated` re-add
  simulated. Spike quit by `WM_CLOSE` (PID 30700 gone), cursor put back.
- 2026-11-03 16:13: dead code removed after the spike quit; fmt, clippy `-D warnings` x64 and ARM64, 7 tests, both
  release builds clean (x64 676,864 bytes, ARM64 648,192).
- 2026-11-03 16:14: deliverable 6. Exported with `reg export` to `leftover-1.reg` to `leftover-3.reg` in my scratchpad,
  then deleted: `NotifyIconSettings\17618738229921704208`, `NotifyIconSettings\2295659972384642272`,
  `Notifications\Settings\NotifyIconGeneratedAumid_17618738229921704208`; deleted the `guidmove\tray-spike.exe` copy.
  This card's runs added no new spike entry (same exe path). All four read as gone at 16:14:48 and again at 16:16:40.
  Toast keys: none. Stored toasts left to expire.
- 2026-11-03 16:15: deliverable 7. Crops in `docs/research/img/`: `tray-taskbar-before-promote.png`,
  `tray-taskbar-after-promote.png` (taskbar rows only; the strip above showed a terminal),
  `tray-toast-unregistered-none.png` (taskbar and spike tooltip; the area above the taskbar showed other apps and is
  greyed). `research.md` points at them. Review finding 5: who added deliverable 7 is not in this Log; most likely the
  orchestrator with the 15:08 scratchpad copy (inferred, not verified).
- 2026-11-03 16:20: docs. `tray-menu-look.md` "Side by side on screen"; `research.md` review findings 2, 3, 4 and
  picture pointers; `p0-spike-tray.md` findings 1, 2 and the optional pointers of 5; `p1-tray-review-fixes.md` the
  stale half of finding 5; spike README.
- 2026-11-03 16:21: moved the 107 raw captures from `spikes/tray/out/look/` to my scratchpad `raw-look\` (some show
  the Wi-Fi name, Haik's media panel or a strip of terminal), so none can be committed. Orchestrator reports AC since
  16:20 and Balanced back with Haik's values; my screen part had ended at 16:12, and I wrote no power setting.
- 2026-11-03 16:24: orchestrator's privacy rule added to deliverable 1. Checked the curated pictures against it by
  eye: Wi-Fi name greyed in both quick settings pictures, the Chrome extension line greyed, quick settings' media panel
  cut away; Telegram, Copilot, network, Security and Explorer pictures show only fixed menu text and tray icons. The
  network menu shows no network name. No account, email or chat text in any picture.

## Steps for Haik (one look, about 5 minutes, any time, no setting changes)

Open these files in `docs/research/img/` and compare:

1. `tray-look-dark-explorer-new.png` (Windows' own new menu) next to `tray-look-dark-ours-owner.png` (the pick) and
   `tray-look-dark-ours-classic.png` (the menu you called old).
2. The same three with `light` in the name.
3. Optional: `tray-look-dark-network-menu.png`, `...-security.png`, `...-copilot.png`, `...-chrome.png`,
   `...-telegram.png`, and `tray-look-custom-ours-owner.png` next to `tray-look-custom-network-menu.png` (Windows dark,
   apps light: both dark, as decided).
4. Tell the orchestrator one of: "pick is fine", or what still looks off. The known gaps are the flat fill (no
   acrylic) and no icons in items (icons are a product call).

## Result

Status: blocked only on Haik's one look (steps above). All agent work is done.

What was done:

- Pictures (deliverable 1): Telegram, network right-click, quick settings, Windows Security, Chrome, Copilot
  (`M365Copilot.exe`) and Explorer (new and classic), in dark and light, plus Custom mode for the theme rule; 36 curated
  files `docs/research/img/tray-look-*.png`, private bits greyed. Raw captures (some show private bits) were moved
  out of the repo to my scratchpad `raw-look\`.
- Side by side (deliverables 2 to 4): `docs/research/tray-menu-look.md` "Side by side on screen": measured table,
  theme rule result, how the pick behaves. Pick unchanged: **B**, owner-drawn items plus `DWMWCP_ROUND`; change it
  when the triggers in "Pick" hold. Proposal for the design (not edited, not mine): `architecture.md` can state that
  the tray menu is owner-drawn with `MSAAMENUINFO`, `WM_MENUCHAR`, `ForceDark`/`ForceLight` from
  `SystemUsesLightTheme`, and the DWM call at the first `WM_DRAWITEM`.
- Spike (deliverable 5): `spikes/tray/src/look.rs` (new), `src/main.rs`, `Cargo.toml` (`Win32_Graphics_Dwm`,
  `Win32_UI_Input_KeyboardAndMouse`), `.gitignore` (`out/look/`), `README.md`.
- Leftovers (deliverable 6) removed and checked twice. Finding 13 crops (deliverable 7) done.
- Review `p0-tray-verify.md` minors 1 to 5 fixed in `research.md`, `p0-spike-tray.md`, `p1-tray-review-fixes.md`.

Checks:

- Every named app has a light and a dark picture: pass.
- Each difference has a source: measured values from the pictures (dark) or the Microsoft and WinUI sources above:
  pass. Light values not measured separately.
- The pick fixes what it claims: corner 4 to 8 epx, font 12 to 14 epx Segoe UI Variable, rows 21 to 33 epx (Explorer:
  32), Fluent glyphs, rounded hover, WinUI colours: pass by measure and eye.
- Tray checks: Medium token pass; theme switch without restart pass; `TaskbarCreated` re-add pass, simulated only
  (no real Explorer restart).
- Owner-drawn checks: check and radio in light and dark pass (pictures); keyboard Down, Right, Left, Enter, Esc and
  mnemonic pass (spike log and step log); high contrast branch pass with the forced flag (real switch not tested);
  DPI: 125% on screen and 96 and 144 dpi renders pass, 100% on screen not possible (only the 125% screen here).
- Leftovers gone by registry and file reads at 16:14:48 and 16:16:40: pass.
- Code: `cargo fmt --check`, clippy `-D warnings` x64 and ARM64, 7 tests (layout, colours, coverage, mnemonic,
  hover shape, a render with no screen): pass.

Open items: Haik's look; a real Explorer restart, a real high contrast switch and 100% on screen remain untested;
the overwritten Phase 0 icon picture (Found outside scope).
