# Paint the menu items the Windows 11 way

Phase: 5. Status: ready. Needs Haik: his one look at the menu pictures (`p1-tray-menu-look`) decides the details;
start on the current pick if no answer has come, and apply his answer when it comes. Agent: `wakeshot-worker`. Size:
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
  theme flush (`p5-menu-view`). Theme reads (`p5-theme`). Wording (`p5-text`). Icons in items (product call).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.
