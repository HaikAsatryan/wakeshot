# Build the tray menu: owner-drawn Win32 menu with round corners

Phase: 5. Status: done (Part A 2026-11-04 02:26; Part B screen run 11:47 to 11:51). Needs Haik: no.
His menu look answer (2026-11-03 17:32): "I like your new ui, you might need some icons for each row as examples are
using. Everything else is good." The pick stands; every item gets the icon that `p5-menu-paint` draws. Agent:
`wakeshot-worker`. Size: about 130K tokens.
Real laptop: **screen** in Part B (one screen or sound card at a time, across all tracks), and **settings**: the
Windows mode is switched to light and back through Settings. Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-menu-view\target`.
Runs after: `p5-menu-paint`, `p5-tray-icon` (Part B clicks the real demo icon). Parallel with (Part A): any card that
does not own `tray/menu.rs`; Part B only with cards that use no screen, sound or power settings.

## Goal

`OwnerDrawnMenu`, the product's `MenuView`: it shows a `MenuModel` as given, painted by `menu_paint`, with Windows'
own menu loop (keyboard, mnemonics, light dismiss, submenus, screen readers), round corners from DWM, colours that
follow the Windows mode, and the pick posted to the loop as `Event::Menu`.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2 to 4).
- `docs/architecture.md` section 2 "Boundary traits" (`MenuView`), section 5 "Tray, notices, sounds" (menu bullet).
- `docs/research/tray-menu-look.md`: "Pick", "Side by side on screen", "How the pick behaves" (DWM call timing,
  `ExcludeClipRect`, keyboard, `MSAAMENUINFO`, theme switch), "Which mode the tray menus follow".
- `docs/research.md`, "Tray, menu and notifications": "Dark menus", "Menu details" (`SetForegroundWindow`, outside
  click).
- `docs/tasks/p1-tray-menu-look.md` Result, "Found outside scope" (measure the menu code alone), and Haik's answer.
- `spikes/tray/src/look.rs` (the menu half: item data, `WM_MENUCHAR`, the CBT and `WM_ENTERIDLE` DWM calls, the
  forced theme) and `spikes/tray/src/theme.rs` (uxtheme ordinals). Port by reading and rewriting.
- `crates/wakeshot-windows/src/tray.rs` (`MenuView` and its doc), `tray/menu_paint.rs`, `tray/theme.rs`, the event
  sink (contracts card); `crates/wakeshot-core/src/view.rs`, `text.rs`.

## Owns

`crates/wakeshot-windows/src/tray/menu.rs`; `crates/wakeshot-windows/examples/ux_menu.rs`; new files
`docs/research/img/ux/menu-screen-*.png`.

## Deliverables

Part A (no laptop):

1. `OwnerDrawnMenu::new(sink: Arc<dyn EventSink>) -> Result<Self, WinError>`: creates one hidden owner window on the
   loop thread (track note 2; change it when step 7 fails).
2. `MenuView::show(&mut self, menu, anchor)`:
   - reads the theme (`theme::read`, a failed read means dark) and the DPI of the anchor's monitor (`MonitorFromPoint`,
     `GetDpiForMonitor` with `MDT_EFFECTIVE_DPI`);
   - builds a fresh `HMENU` from the model: owner-drawn items and separators, submenus, command ids from a table
     (`Label` gets no command), `MSAAMENUINFO` first in each item's data with the text less its access key marks,
     `MENUINFO.hbrBack` from the palette;
   - sets the classic menu frame to the same mode (`SetPreferredAppMode` `ForceDark` or `ForceLight`, then
     `FlushMenuThemes`, uxtheme ordinals 135 and 136 found by `GetProcAddress`; if either is missing, go on without);
   - `SetForegroundWindow(owner)`, `TrackPopupMenuEx` with `TPM_RETURNCMD | TPM_RIGHTBUTTON`, alignment from
     `SM_MENUDROPALIGNMENT` and the anchor, then `PostMessage(owner, WM_NULL)`;
   - posts the picked command as `Event::Menu` through the sink; nothing on Esc or an outside click;
   - frees the menu, fonts and brushes.
3. Owner window procedure: `WM_MEASUREITEM` and `WM_DRAWITEM` through `menu_paint`; on the first `WM_DRAWITEM` of each
   menu window, `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND)` on `WindowFromDC(hDC)`, and the
   WinUI border colour through `DWMWA_BORDER_COLOR` (worked in dark in Phase 1), with `WM_ENTERIDLE` as the fallback;
   `ExcludeClipRect` over the chevron; `WM_MENUCHAR` maps a key to the item with that access key (`MNC_EXECUTE`,
   which also opens a submenu) or `MNC_IGNORE`.
4. Pure helpers with unit tests, written first: `command_ids` and `command_for(id)`, `access_key(text)`,
   `strip_access_keys(text)`, `menu_char(key, items)`. Tests: `ids_round_trip_every_command`, `labels_get_no_id`,
   `access_key_found_and_double_ampersand_ignored`, `msaa_name_drops_access_key_marks`,
   `menu_char_picks_the_item_or_ignores`, `builds_the_hmenu_from_the_model` (headless: `GetMenuItemCount`,
   `GetMenuItemInfoW` types, ids, submenu; no `TrackPopupMenuEx`), `pick_posts_menu_event` (the path from a returned id
   to the sink, with the fake sink).
5. Example `ux_menu`: `TrayIcon` (demo kind) with `OwnerDrawnMenu`, a sample `TrayView` built from `text.rs` (the
   product sketch, with a long Armenian hold reason in the status line), and a sink that prints each event with the
   wall time. Flags `--contrast` (forced contrast palette) and `--minutes N`.

Part B (screen slot; set Status `blocked: waiting for the screen slot` after Part A and stop):

6. Start `ux_menu` at Medium (track note 4); pin the demo icon as in `p5-tray-icon`. Pictures on a plain backdrop,
   like Phase 1: the menu and the open submenu, dark and light (switch through Settings, read before, put back, read
   after). Compare with `docs/research/img/tray-look-*-explorer-new.png` and the Phase 1 pick pictures; measure the
   corner, row pitch, font and glyph sizes at 125%.
7. Open by left click, right click and keyboard; an outside click closes it every time; Esc closes; Down, Right, Left,
   Enter and access keys work; the picked command arrives as `Event::Menu`. `AccessibleObjectFromWindow` on the menu
   window gives every item's name without `&` and its checked, disabled and submenu state.
8. A theme switch shows at the next open without a restart. The Armenian label draws with real glyphs.
9. Cost: release size of `ux_menu` against `ux_icon` (the menu code's weight, Phase 1 "Found outside scope"), and
   working set and private bytes after 20 opens.
10. Leftovers removed and checked (track note 4). Demo stopped by its PID.
11. `docs/research.md`: a new section "Tray UX: menu on screen".

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows tray::menu`: all pass (this also runs `tray::menu_paint`); list this card's
  names in the Result.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-windows` for both targets: each clean in the
  owned files.
- Part B steps 6 to 8: each passes with a picture or a log line; theme reads before and after are equal.
- Leftovers gone at two reads a minute apart; no demo process left.
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Painting (`p5-menu-paint`; a paint bug found here goes to its owner or, once that card is done, into this card's
  Result as a request). Which items, order and checks (`p2-present`). Wording (`p5-text`). A menu opened by code
  (v1 opens it only from the icon). Icons in items, acrylic (product call; "change it when" in the research Pick).

## Questions for Haik

Each with the worker's default.

None. The calls this card made are listed in the Result as "decided by the worker, for Haik's review".

## Found outside scope

Problems seen but not fixed.

- `docs/phases/05-tray-ux.md` track note 5 writes `TrayIcon::new(kind, Box::new(OwnerDrawnMenu::new(sink)))`, but
  `OwnerDrawnMenu::new` returns a `Result` (as the card asks), so the wiring card needs a `?` there. A doc line only.
- Part B (11:48): the "Keep awake" row still draws a cup glyph, while the brand (2026-11-04 11:15) replaced the Cup
  with the Sunrise for the tray icon. Maybe intended (a row glyph, not the brand); not checked against `docs/brand.md`.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 02:04: Part A started by `wakeshot-worker`. Scope this round: Part A only; Part B waits for a screen slot.
- 2026-11-04 02:14: read done (card list, `menu_paint.rs`, `icon.rs` API, spike `look.rs` and `theme.rs`). One change
  from the card: the spike gave `MSAAMENUINFO` the raw text "&Keep awake" (`spikes/tray/out/log-look.txt` draws
  `'&Keep awake'`, and owner-drawn items have no other string) and the screen reader read "Keep awake", so Windows'
  proxy drops the marks itself. Stripping them first would most likely make a status line "R&&D" read as "RD"
  (inferred from that, not verified). So the item data
  keeps the menu text as is; no `strip_access_keys`. Command ids come from a table built per menu (a fixed table
  could not hold any `Minutes(n)`). Writing `menu.rs`.
- 2026-11-04 02:20: `menu.rs` written; `cargo test --locked -p wakeshot-windows tray::menu`: 25 passed (10 of this
  card, 15 of `menu_paint`), 1 ignored (`menu_paint`'s render), at the first run. Item data is one boxed slice made
  before the first insert (no `Vec<Box<_>>`, which clippy flags). Writing `examples/ux_menu.rs`.
- 2026-11-04 02:25: `ux_menu.rs` written. rustfmt (edition 2024) reformatted only my two files; then `--check` clean.
  Clippy `-D warnings`, x64 and ARM64, with and without `--all-targets`: clean at the first run, no hits in other
  cards' files. Release builds of the lib (x64, ARM64) and of `ux_menu` and `ux_icon` (x64): clean. Tests again
  after the format: 25 passed. LF only, ASCII only in both files. No failure from another card's file this round.
  Build folder (2,068 MB) deleted and checked gone.
- 2026-11-04 02:26: Part A done. Status `blocked: waiting for the screen slot`.

- 2026-11-04 11:47, Part B worker (session `wakeshot-12`): started the screen run (session unlocked, orchestrator's
  DISPLAY request held). `ux_menu` debug x64 built from the f47391d export (debug only, orchestrator rule).
- 2026-11-04 11:47, Part B worker: plain grey backdrop window (own pwsh, PID 10452) behind the menu area.
  `explorer.exe menu1.cmd`: token line Medium (S-1-16-8192), demo PID 36200, `Added { attempt: 1 }`; the
  `NotifyIconSettings` id was `12880751163260563509` again (the id follows the GUID, not the exe); `IsPromoted` = 1.
- 2026-11-04 11:47 to 11:49, Part B worker: MSAA (`AccessibleObjectFromWindow`, `OBJID_CLIENT`) on the open `#32768`
  windows: main menu names "Holding awake: Claude Code (2 sessions), <Armenian> & <Armenian>" (one literal `&`,
  unavailable), "Automatic" (checked), "Keep awake" (has popup), "Restore my power settings", "Open log", "Quit";
  submenu "Off" (checked), "30 minutes", "1 hour", "2 hours", "4 hours", "Until I turn it off", "Keep screen on"
  (unavailable). No `&` access mark in any name. Opens: left, right, keyboard (Win+B, Right, Enter) each opened one
  menu; outside clicks on the backdrop closed it 4 of 4; Esc closes the submenu, then the menu. `a` gave
  `event: Menu(Automatic(false))`; Down x4 then Enter gave `Menu(Restore)` (arrows also stop on the greyed status
  line, so the 4th Down is Restore); `k` opens the submenu, `s` (disabled) keeps it open, `1` gave
  `Menu(KeepAwake(Minutes(60)))`; `r` gave `Menu(Restore)`; Down x3 then Right opened the submenu, Left closed it.
  Memory (debug build) before 20 opens: working set 17,227,776, private 2,433,024 bytes; after: 17,272,832 and
  2,408,448. 31 `Opened`, 31 `Closed`. `Styled` came `at: Idle` on most main-menu opens, `at: FirstPaint` on the
  first open and on submenus; `corners` and `border` `Ok(())` every time.
- 2026-11-04 11:49 to 11:50, Part B worker: Light through Settings 11:49:52, menu opened 11:49:57 with `theme: light:
  true` (no restart), pictures; Dark back 11:50:01. Reads before 11:49:50 and after 11:50:06 equal
  (`AppsUseLightTheme=0 ColorPrevalence=0 EnableTransparency=1 SystemUsesLightTheme=0`). `taskkill /PID 36200`:
  quit, `icon removed`, exit 0.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.

**Done (2026-11-04 12:07).** Part A 02:26; Part B in the screen slot of 11:47 to 11:51 (session `wakeshot-12`,
unlocked, orchestrator's DISPLAY request; debug build of the f47391d export). No code change. Facts in
`docs/research.md` "Tray UX: menu on screen".

| Step | Outcome |
| --- | --- |
| 6. Medium start, pictures | Passed. Token line Medium; icon promoted. `menu-screen-{dark,light}-{main,sub}.png` on a plain grey window. Against Explorer's new menu: same style; row pitch 50 px at 150% (33 DIP) against about 32 DIP; small rounded corner. Font and glyph sizes not measured in pixels (time); the pictures are there to measure. Measured at 150%, not 125%: the taskbar is on the 150% screen. |
| 7. Opens, closes, keys, MSAA | Passed. Left, right and keyboard opens; outside click closed 4 of 4; Esc closes submenu then menu; Down, Right, Left, Enter and access keys `a`, `k`, `1`, `r` work (`s` disabled: nothing); picks arrive as `Event::Menu(...)`. MSAA: every name without `&` marks, the status line's literal `&` kept; checked, unavailable and has-popup states right. |
| 8. Theme, Armenian | Passed. The next open after the Light switch drew light (`theme: light: true`), no restart. Armenian status line in real glyphs, cut with an ellipsis. Reads before (11:49:50) and after (11:50:06) equal. |
| 9. Cost | Working set and private bytes flat over 20 opens (debug: 17.2 MB and 2.4 MB). Release sizes: Part A's (`ux_menu` 361,472, `ux_icon` 264,192 bytes); no release build in this slot (orchestrator rule). |
| 10. Leftovers, stop by PID | Passed. Demo 36200 stopped by `taskkill /PID` (exit 0); backdrop window (own pwsh 10452) stopped by PID. Shared entry removed after `p5-notices`; reads 12:04:02, 12:04:58 and 12:05:36 equal to before, no demo process. |
| 11. Research | Done (new section). |

Notes from the run, for Haik's review: arrow keys also stop on the greyed status line (normal Win32 behaviour); the
menu's bottom sits at the click point, so it overlaps the taskbar by 36 px, like Windows' own tray menus; in light mode
the check mark is drawn in the accent colour (reddish on this laptop), in dark it is grey; `Styled` mostly arrives
`at: Idle` with the corners applied (`Ok(())`) and every picture has the rounded frame.

Acceptance: `cargo` checks not rerun (no code change since Part A). Files changed in Part B: this card,
`docs/research.md`, 4 new `docs/research/img/ux/menu-screen-*.png`.

Part A record:

Files: `crates/wakeshot-windows/src/tray/menu.rs` (new body), `crates/wakeshot-windows/examples/ux_menu.rs` (new),
this card. Nothing else changed; no `docs/research.md` section yet (step 11 is Part B).

Public API (`wakeshot_windows::tray::menu`):

- `OwnerDrawnMenu::new(sink: Arc<dyn EventSink>) -> Result<OwnerDrawnMenu, WinError>`: class `Wakeshot.TrayMenu`,
  one hidden top-level `WS_EX_TOOLWINDOW` owner window on the calling thread; loads uxtheme ordinals 135 and 136 once.
  Not `Send` (holds an `Rc`). Drop clears the window slot, then destroys the window.
- `impl MenuView`: `show(&mut self, &MenuModel, Anchor)` reads the theme (`theme::read`, a failed read is dark), the
  DPI and rect of the anchor's monitor (`MonitorFromPoint`, `GetDpiForMonitor` `MDT_EFFECTIVE_DPI`, `GetDpiForSystem`
  if that fails), sets the frame mode (`ForceDark`, `ForceLight`, or `Default` in high contrast, then
  `FlushMenuThemes`; skipped if either ordinal is missing), builds a fresh `HMENU`, `SetForegroundWindow(owner)`,
  `TrackPopupMenuEx` with `TPM_RETURNCMD | TPM_RIGHTBUTTON` and the alignment, `PostMessage(owner, WM_NULL)`, frees the
  menu, brush and fonts, then posts the pick as `Event::Menu` (nothing on Esc or an outside click). An empty model
  opens nothing.
- `on_note(&mut self, NoteHook)` with `NoteHook = Box<dyn FnMut(&Note)>`; `Note { Opened { dpi, theme, foreground,
  frame }, Styled { at: StyledAt, corners, border }, Closed { command }, Failed(WinError) }`, `StyledAt { FirstPaint,
  Idle }`. Integration should log `Failed` (a build, paint or post failure; a failed post loses the pick) and
  `Opened` with `foreground: false`.
- Demo only: `force_theme(&mut self, Option<Theme>)`.
- Inside: owner-drawn items and separators (`MFT_OWNERDRAW`, `MFT_SEPARATOR`, `MFT_RADIOCHECK` for the Keep awake
  lengths, `MFS_CHECKED`, `MFS_GRAYED`); the status line is grayed with no id; `MENUINFO.hbrBack` from the palette
  with `MIM_APPLYTOSUBMENUS`; each menu has its own `Layout` (columns per menu). Item data is `#[repr(C)]` with
  `MSAAMENUINFO` first; `dwItemData` is used only when it is one of the open menu's own items. The owner window
  answers `WM_MEASUREITEM` and `WM_DRAWITEM` through `menu_paint`, sets `DWMWCP_ROUND` and `DWMWA_BORDER_COLOR`
  (palette border) once per menu window at its first `WM_DRAWITEM` (`WindowFromDC`), with `WM_ENTERIDLE` as the
  fallback, calls `ExcludeClipRect` over a submenu item, answers `WM_MENUCHAR`, and ignores `WM_CLOSE`.

Checks (build folder in the scratchpad, deleted at the end and checked gone):

- `cargo test --locked -p wakeshot-windows tray::menu`: 25 passed, 1 ignored (`menu_paint`'s `renders_for_review`).
  This card's 10: `ids_round_trip_every_command`, `labels_get_no_id`, `access_key_found_and_double_ampersand_ignored`,
  `msaa_text_is_the_menu_text` (in place of `msaa_name_drops_access_key_marks`, decision 1 below),
  `menu_char_picks_the_item_or_ignores`, `builds_the_hmenu_from_the_model`, `pick_posts_menu_event`,
  `alignment_follows_the_setting_and_the_anchor`, `frame_mode_follows_the_theme`,
  `owner_window_measures_draws_and_maps_keys` (real owner window, `SendMessageW` of measure, draw into a DIB, menu
  char, idle, close; no `TrackPopupMenuEx`).
- `rustfmt --edition 2024 --check` on `menu.rs` and `ux_menu.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --target <t> -- -D warnings`, with and without `--all-targets`, x64 and
  ARM64: clean.
- `cargo build --release --locked -p wakeshot-windows`, x64 and ARM64: clean; `--example ux_menu --example ux_icon`
  x64 release: clean.
- Only the owned files changed: yes (two Rust files and this card).
- Part B checks (pictures, clicks, keyboard, screen reader, theme switch, cost, leftovers): not run yet.

Decided by the worker, for Haik's review (technical; each with a "change it when"):

1. **The screen reader text keeps the access key marks** (the card said to strip them). Why: Phase 1 gave
   `MSAAMENUINFO` the raw "&Keep awake" (spike log draws `'&Keep awake'`; owner-drawn items have no other string) and
   `AccessibleObjectFromWindow` read "Keep awake", so Windows' proxy strips them itself; stripping first would most
   likely turn a status line's literal `&` ("R&&D") into an access key mark and lose it (inferred, not verified). So
   no `strip_access_keys` helper. Change it when Part B step 7 reads a name with a `&` mark left in it: then
   `ItemData::new` takes the stripped text (one place) and the test changes with it.
2. **Command ids come from a table built per menu**, depth first, id = place + 1 (0 is "nothing picked"). Why: a
   fixed table cannot hold any `KeepAwake::Minutes(n)` the core may offer. Change it when a command must keep one id
   across opens (a menu that sends `WM_COMMAND` later, not in v1).
3. **The status line is grayed** (`MFS_GRAYED`, no id): Enter on it does nothing and a screen reader hears
   "unavailable". The keyboard still stops on it (Phase 1). Change it when Haik finds that stop annoying: an
   owner-drawn `MFT_SEPARATOR` item is skipped by the keyboard, but a screen reader would then hear a separator.
4. **High contrast leaves the frame mode to Windows** (`Default`), since Windows turns dark mode off under a contrast
   theme; what `ForceDark` does there is not verified. Change it when `p5-haik-check` shows the frame in the wrong
   colours under a contrast theme.
5. **Alignment**: left or right from `SM_MENUDROPALIGNMENT` (Microsoft's notification icon sample does this); above
   the anchor when it is in the lower half of its monitor, else below. Change it when Part B shows the menu covering
   the icon or off the work area.
6. **Access keys**: the first enabled item with the key is executed (no cycling through items that share a key); a
   disabled item's key and an unknown key give `MNC_IGNORE` (the usual beep). Change it when the menu gets two items
   with one key (`text.rs`: all are unique today).
7. **A note hook** (`on_note`, like `TrayIcon`'s) is the menu's only way to report: `MenuView::show` returns nothing
   and the sink contract says a failed post must be logged by the caller.
8. **`ux_menu`'s sample**: Keep awake off (Off has the bullet, Keep screen on is disabled, as `present` builds it),
   Automatic on, and a 52-letter Armenian hold reason with a literal `&` ("Nightly backup & whole disk check"), so one
   status line covers the Armenian glyphs, the ellipsis and the screen reader's `&`. `--contrast` forces the same
   contrast set as `ux_icon` on both the icon and the menu.

Needs another card: none.

Notes for the Part B worker:

- Build: `CARGO_TARGET_DIR=<scratchpad>\p5-menu-view\target`, `cargo build --release --locked -p wakeshot-windows
  --example ux_menu --example ux_icon --target x86_64-pc-windows-msvc` (about 1.5 minutes from clean; the Part A
  folder was deleted).
- Start `ux_menu` at Medium through `explorer.exe <launch .cmd>` with stdout to a log; pin the demo icon as
  `p5-tray-icon` does (demo GUID `1e8c66d4-0fa3-4a54-95a1-61bb89b171e9`). First lines: pid, icon window and GUID,
  then `status line: Label(...)`. Each open prints `menu: Opened { dpi, theme, foreground, frame }`, one
  `menu: Styled { at: FirstPaint, corners: Ok(()), border: Ok(()) }` per menu window (main, then the submenu when it
  opens), `event: Menu(...)` for a pick and `menu: Closed { command }`. `at: Idle` means the first paint had no
  window; an `Err` in `corners` means DWM refused the window.
- Step 7, screen reader: expect "Automatic", "Keep awake", "Restore my power settings", "Open log", "Quit", and the
  status line with one `&` before the Armenian words; checked on Automatic and Off, unavailable on the status line
  and Keep screen on, has popup on Keep awake. If a name keeps a `&` mark, apply decision 1's switch. Access keys:
  `a`, `k` (opens the submenu), `r`, `o`, `q`; in the submenu `o`, `3`, `1`, `2`, `4`, `u`; `s` beeps (disabled).
- Step 9, size: already measured in Part A (x64 release, 02:24): `ux_menu.exe` 361,472 bytes, `ux_icon.exe` 264,192,
  so 97,280 bytes more. That counts `menu.rs`, `menu_paint.rs` and the `text.rs` words the sample uses, so it is
  above the research's 10 to 30 KB estimate; Part B records it and measures working set and private bytes after 20
  opens.
- The process's uxtheme mode changes only inside the demo; nothing of it persists. Leftovers are the demo GUID's
  `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_` key (track note 4).
- `taskkill /PID <pid>` without `/F` ends the demo through its pump; both its windows ignore a sent `WM_CLOSE`. Else
  `--minutes` ends it.
