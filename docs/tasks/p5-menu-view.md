# Build the tray menu: owner-drawn Win32 menu with round corners

Phase: 5. Status: ready. Needs Haik: his one look at the menu pictures (`p1-tray-menu-look`); if his answer changes
the pick (for example a XAML flyout), stop and ask the orchestrator for a new plan of this card. Agent:
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.
