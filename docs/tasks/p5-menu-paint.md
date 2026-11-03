# Paint the menu items the Windows 11 way

Phase: 5. Status: done. Needs Haik: no (his menu look answer came 2026-11-03 17:32: the pick stands, plus an icon
per menu item; see Read first). Agent: `wakeshot-worker`. Size:
about 110K tokens.
Real laptop: no (draws into memory bitmaps, no window, no screen). Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-menu-paint\target`.
Runs after: `p5-theme` (the `Theme` type). Parallel with: `p5-icon-art`, `p5-text`, `p5-tray-icon` Part A, Part A of
`p5-sounds` and `p5-notices`, and any card of another track.

## Goal

The drawing half of the owner-drawn menu picked in Phase 1: item sizes at any DPI, the colours for light, dark and high
contrast, and painting one item into a device context. No window and no menu loop: `p5-menu-view` calls this from
`WM_MEASUREITEM` and `WM_DRAWITEM`.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2 and 4).
- `docs/research/tray-menu-look.md`: "WinUI 3 `MenuFlyout` template", "Colours", "Differences and what each costs"
  (rows 2 to 4, 8 to 16), "Side by side on screen", "How the pick behaves".
- `docs/tasks/p1-tray-menu-look.md` Result and Haik's answer if it has come (the orchestrator relays it).
  Haik's answer (2026-11-03 17:32): "I like your new ui, you might need some icons for each row as examples are
  using. Everything else is good." So the pick stands, and every menu item gets a small icon like Windows 11's own
  menus (Segoe Fluent Icons glyphs are one way; pick with the reason), in light, dark and high contrast, at every
  scale. If this card cannot hold the icons within about 150K tokens, split them into a new card.
- `spikes/tray/src/look.rs`: `palette`, the font and glyph setup, `measure`, `render`, `draw`, `render_menu`, and its
  tests. Port by reading and rewriting, never copy whole.
- `crates/wakeshot-core/src/view.rs` (`MenuItem`, `MenuCommand`, the `&` access key rule);
  `crates/wakeshot-windows/src/tray/theme.rs` (`Theme`, `Contrast`).

## Owns

`crates/wakeshot-windows/src/tray/menu_paint.rs`; new files `docs/research/img/ux/menu-render-*.png`.

## Deliverables

1. `Layout` for a DPI: row height (33 epx at 96 dpi), separator slot, inner padding, glyph column, text start, chevron
   column, the 4 epx hover inset and radius, all scaled by the DPI given (never the system DPI). Widths come from the
   text measured with the item font. Windows adds 16 px at 120 dpi to the width asked (research): account for it.
2. `Palette::for_theme(&Theme)`: the WinUI tokens for light and dark from the research table (text, disabled text,
   hover, separator, background), and in high contrast the system colours (`Contrast`) in the MenuFlyout high
   contrast mapping the spike used.
3. `Fonts::new(dpi)`: Segoe UI Variable Text at 14 epx for text and Segoe Fluent Icons at 12 epx for glyphs (Segoe
   MDL2 Assets as the spike's fallback), deleted on drop. A label with characters the text font lacks (Armenian,
   Cyrillic outside its range, CJK) is drawn with a font that has them: check with `GetGlyphIndicesW` and
   `GGI_MARK_NONEXISTING_GLYPHS`, then fall back (Segoe UI first). Hold reasons are user text and Haik's own language
   is Armenian.
4. `paint_item(dc, rect, item, state, &palette, &fonts, &layout)`: text with the access key underlined only when
   Windows asks for keyboard cues (`ODS_NOACCEL` off), check mark `E73E` for Automatic and Keep screen on, radio
   bullet `E915` for the Keep awake choice (`mark_for(command)`), chevron `E974` for a submenu, rounded hover fill on
   `ODS_SELECTED` (not on a `Label`, which reads as a status line and never highlights), disabled colour, owner-drawn
   separators. Return the rect of the chevron so `p5-menu-view` can clip it with `ExcludeClipRect` (Windows then draws
   no second arrow, research).
5. Unit tests, written first: `layout_matches_winui_at_96_dpi`, `layout_scales_at_120_144_and_192_dpi`,
   `palette_light_and_dark_match_the_tokens`, `palette_contrast_uses_system_colours`, `marks_follow_the_command`,
   `access_key_underline_only_with_keyboard_cues`, `label_never_highlights`, `paints_offscreen` (each item kind into a
   32-bit DIB in a memory DC at 96 and 144 dpi; check the hover fill pixel, text pixels present, the glyph present, the
   separator row colour), `non_latin_labels_get_a_font_that_has_them` (an Armenian and a Cyrillic label).
6. Renders for review: an ignored test writes `docs/research/img/ux/menu-render-<96|120|144>-<light|dark|contrast>.png`
   of the product menu (sample texts from `text.rs`), using `tray::art::png` if `p5-icon-art` is done (else BMP in the
   scratchpad only). Compare by eye and by pixel with `docs/research/img/tray-look-render-*.png` from Phase 1.
7. `docs/research.md`: a new section "Tray UX: menu painting" with what changed from the spike and why.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows tray::menu_paint`: all pass; list the names in the Result.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-windows` for both targets: each clean in the
  owned files.
- The renders match the Phase 1 pick at 96 and 144 dpi (row 33 and 50 px, font 14 and 21 px, glyphs 12 and 18 px), or
  the Result says what Haik's answer changed.
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The menu loop, `TrackPopupMenuEx`, the owner window, `MSAAMENUINFO`, `WM_MENUCHAR`, the DWM corner call, the menu
  theme flush (`p5-menu-view`). Theme reads (`p5-theme`). Wording (`p5-text`).

## Questions for Haik

None open. Decided by the worker, for Haik's review (pictures: `docs/research/img/ux/menu-render-*.png`):

1. Icons on the five main rows: Automatic an eye (`E890`, it watches agents), Keep awake a cup (`EC32`, the usual
   stay-awake picture), Restore my power settings the restore arrow (`E777`), Open log a page (`E8A5`), Quit a cross
   (`E711`; a power button would read as shutting the PC down). No icon on the Keep awake lengths, Keep screen on or
   the status line. Why: Microsoft's menu guidance, "Don't feel obligated to provide icons for commands that don't
   have a standard visualization"; the lengths are one choice with a bullet. Change it when Haik wants every row of
   the submenu to have one too: one line each in `icon_for`.
2. The status line is drawn in WinUI's secondary text colour (about 6:1 contrast in light, 9:1 in dark) instead of
   the disabled grey (2.5 to 2.7:1), never highlighted, and starts where the item texts start. Change it when Haik
   prefers it greyer or flush left.
3. An item asks for at most 480 epx of text; a longer status line ends in "...". Two hold groups fit. Change it when
   status lines are cut that people need: raise `TEXT_MAX`.
4. Check, bullet and chevron in WinUI's secondary colour, as Windows 11's own menus draw them (the spike used the
   text colour).

## Found outside scope

1. Full `cargo test -p wakeshot-windows` at 00:44: 6 failures, all in `crates/wakeshot-windows/src/watch/data.rs`
   tests (`a_bad_expires_is_left_out_like_a_bad_file`, `a_holds_folder_that_cannot_be_listed_is_logged_once`,
   `a_bad_file_is_retried_once_then_logged_once` with `left: []`, `right: ["job.json"]`,
   `a_bad_file_fixed_before_its_retry_counts_with_no_line`, `an_unreadable_file_is_retried_then_logged`,
   `temp_names_and_folders_are_never_opened`). Not this card's files; most likely the Claude review fixes in
   `holds/file.rs` running now (not checked). See the last Log line for the rerun.
2. This card's "Out of scope" still lists "Icons in items (product call)"; Haik's answer of 17:32 (Read first) put
   them in, and this card drew them.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 00:25: started by `wakeshot-worker` as the second card of the `p5-theme` + `p5-menu-paint` bundle
  (`p5-theme` done 00:24). Build folder for the bundle: `<scratchpad>\p5-theme\target`.
- 2026-11-04 00:33: read the WinUI template again (`gh api`, last commit 2026-05-30): icon 16 epx, mark and icon
  columns 28 epx each (text at 28 or 56), marks and chevron in `TextFillColorSecondary`. Microsoft's menu page
  (updated 2026-10-14): icons only where a standard picture exists. Wine's `menu.c`: Windows adds twice the menu
  font's average character width to an owner-drawn item. Fonts here: Segoe UI Variable lacks Armenian, Segoe UI has
  it; both have Cyrillic. Icon picks from a candidate sheet (scratchpad `icon-candidates.png`).
- 2026-11-04 00:38: `menu_paint.rs` and its tests written in one pass (tests planned first, not run red first); 15
  pass. Windows adds 14, 16 and 20 px at 96, 120 and 144 dpi by the Wine rule: 16 at 120 matches Phase 1's screen
  measure. Renders written and viewed; the status line of two groups was cut at a 400 epx cap, so the cap is 480.
- 2026-11-04 00:42: renders compared with Phase 1's by pixel: same fill, hover and separator colours, same hover band
  (27 rows at 96 dpi, 42 at 144). PNGs in `docs/research/img/ux/`. Research section added. A missing glyph face now
  falls back to GDI's substitute instead of failing the menu.
- 2026-11-04 00:44: fmt, clippy (4 runs) and release builds (2) clean; `tray::` 22 pass. Full `cargo test -p
  wakeshot-windows`: 366 pass, 6 fail, all in `watch::data::tests` (`a_bad_expires_is_left_out_like_a_bad_file` and
  5 more), another card's file while the Claude review fixes and `p4-watch-folders` run. Waiting, then again.
- 2026-11-04 00:50: rerun of the full crate tests: 367 pass, the same 6 `watch::data` tests fail (Found outside
  scope 1). Every `tray::` test passes. Build folder deleted. Card done.

## Result

What was done: `crates/wakeshot-windows/src/tray/menu_paint.rs` holds the drawing half of the owner-drawn menu: sizes
at any DPI, colours for light, dark and high contrast, fonts with a fallback for scripts the text font lacks, item
icons (Haik's ask), and painting one item offscreen into any DC. Nine review renders in `docs/research/img/ux/`;
research section "Tray UX: menu painting" in `docs/research.md` (what changed from the spike and why).

Files changed: `crates/wakeshot-windows/src/tray/menu_paint.rs`; nine new
`docs/research/img/ux/menu-render-<96|120|144>-<light|dark|contrast>.png`; the new `docs/research.md` section; this
card.

Public API as built (`wakeshot_windows::tray::menu_paint`):

- `scale(epx, dpi) -> i32`.
- `Columns { marks, icons }`, `Columns::of(&[MenuItem])`: a menu reserves the mark and icon columns for every item
  once any item has one (WinUI).
- `Layout::new(dpi, columns)`: `row` (33 epx), `separator`, `separator_top`, `separator_line`, `inset_x`, `inset_y`,
  `radius` (4 epx hover inset and radius), `line_top`, `line`, `mark_left`, `icon_left`, `text_left` (15, 43 or 71
  epx), `right`, `chevron_right`, `mark` (12), `icon` (16), `chevron` (12), `font` (14), all in pixels for the DPI
  given; `width_for(text_px, submenu)`.
- `Palette::for_theme(&Theme)`: `background` (for `MENUINFO.hbrBack`), `text`, `secondary`, `disabled`, `hover`,
  `hover_text`, `hover_secondary`, `separator`, `border` (for `DWMWA_BORDER_COLOR`), each a `theme::Rgb`.
- `Mark { Check, Radio }` with `glyph()`; `mark_for(MenuCommand) -> Option<Mark>`; `icon_for(&MenuItem) ->
  Option<u16>`.
- `ItemState { selected, keyboard_cues }`, `ItemState::from_ods(dis.itemState.0)`.
- `Fonts::new(dpi) -> Result<Fonts, WinError>` (deleted on drop; loop thread only): `font_for(&[u16]) -> HFONT`,
  `text_face()`, `glyph_face()`, `windows_adds()`.
- `measure(&MenuItem, &Fonts, &Layout) -> (u32, u32)`: what to put in `MEASUREITEMSTRUCT`, the width less what
  Windows adds.
- `paint_item(dc, rect, &MenuItem, ItemState, &Palette, &Fonts, &Layout) -> Result<Option<RECT>, WinError>`:
  `Some(rect)` for a submenu, the rect to `ExcludeClipRect`. This is the whole item, not only the chevron as the card
  said: Phase 1 tested the whole item, and Windows places its own arrow by its own metrics.

For `p5-menu-view`: one `Fonts::new(dpi)` per open, one `Layout::new(dpi, Columns::of(items))` per menu level, one
`Palette::for_theme(&theme)`; `measure` in `WM_MEASUREITEM`, `paint_item(dis.hDC, dis.rcItem, ..)` in `WM_DRAWITEM`,
then `ExcludeClipRect` on the returned rect. Part B should check on screen that `rcItem` is as wide as `measure`'s
width plus `windows_adds()` (the Wine rule gives 14, 16 and 20 px at 96, 120 and 144 dpi; only 120 matches a screen
measure, from Phase 1). `paint_item` never highlights a label or a disabled item, whatever `MFS_DISABLED` says.

Checks:

- `cargo test --locked -p wakeshot-windows tray::menu_paint`: 15 passed, 1 ignored. The card's nine:
  `layout_matches_winui_at_96_dpi`, `layout_scales_at_120_144_and_192_dpi`, `palette_light_and_dark_match_the_tokens`,
  `palette_contrast_uses_system_colours`, `marks_follow_the_command`, `access_key_underline_only_with_keyboard_cues`,
  `label_never_highlights`, `paints_offscreen`, `non_latin_labels_get_a_font_that_has_them`; plus
  `icons_only_for_commands_with_a_known_picture`, `columns_follow_marks_and_icons`,
  `visible_text_drops_access_key_marks`, `paints_at_the_rect_given`, `measure_asks_less_what_windows_adds`,
  `a_long_label_is_capped`; ignored `renders_for_review` (run once, it wrote the renders).
- `cargo test --locked -p wakeshot-windows tray::`: 22 passed (both cards), 1 ignored.
- `rustfmt --edition 2024 --check` on `menu_paint.rs` and `theme.rs`: no diff.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and without
  `--all-targets`, x64 and ARM64: exit 0 (4 runs).
- `cargo build --release --locked -p wakeshot-windows --target <t>`, x64 and ARM64: exit 0.
- Renders against the Phase 1 pick: rows 33 and 50 px, text 14 and 21 px, glyphs 12 and 18 px at 96 and 144 dpi;
  fill, hover and separator pixels equal `tray-look-render-96dpi-dark.png` and `-light.png`; hover band 27 and 42
  rows in both. What Haik's answer changed: five icons, so the main menu's text starts at 71 epx (was 43).
- The renders are BMP from the test, made PNG by a stdlib Python script (scratchpad `bmp2png.py`): `art::png`
  (`p5-icon-art`) was not there yet. Once it is, the test can write PNG itself.
- Only the owned files, the new research section and this card changed.

- Full `cargo test --locked -p wakeshot-windows` (00:50): 367 passed, 6 failed, 20 ignored. All 6 are in
  `watch/data.rs`, another card's file (Found outside scope 1); none is in `tray/`.

Status: done (2026-11-04 00:50).

Open items: an on-screen check of the widths and the icons (`p5-menu-view` Part B); the `watch::data` failures for
their owner.
