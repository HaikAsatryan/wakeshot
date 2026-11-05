# The menu's status row shows the tray's state glyph in its icon column

Phase: 6 (Haik's review of the installed build). Status: done. Needs Haik: no (Haik 2026-11-05 ~20:10: "1 yes, 2 yes,
go ahead"). Agent: `wakeshot-worker`. Size: about 80K tokens.
Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-brand-apply-art` (done). Parallel with: nothing that edits `crates/wakeshot-windows` or
`crates/wakeshot-core/src/view.rs`.
Real laptop: no power settings, no screen. Offscreen renders only. An installed Wakeshot runs on this laptop (Haik's
local build): do not start, quit or test against it; tests that need the real tray lock or real settings refuse beside
it (`p6-real-install-guard`), which is expected.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-menu-status-glyph\target`.

## Goal

Haik on the installed build: the menu's top row ("Holding awake: ...") starts after a wide empty space. That space is
WinUI's two columns, the check (28 epx) and the icon (28 epx), which every row keeps once any row has a check or an
icon (`menu_paint.rs`, `COLUMN`). Haik's decision: put the current state glyph, the same held light as the tray icon
(holding, idle or off), in the status row's icon column. The text stays where it is, so every row's text still lines
up. Haik also decided that several holds stay one status line (as now: groups by commas, `and N more`, an ellipsis
past `TEXT_MAX`); that needs no code, only the doc line in deliverable 4.

## Read first

- `crates/wakeshot-windows/src/tray/menu_paint.rs`: the layout constants (`COLUMN`, `ICON`, `TEXT_MAX`), how item
  icons (Segoe Fluent glyphs) are placed and coloured, and the `Label` row.
- `crates/wakeshot-windows/src/tray/art.rs`: the tray glyph per state and size (`TRAY_SIZES`, the hand-tuned 16, 20 and
  24 px frames). `crates/wakeshot-windows/src/tray/icon.rs`: `glyph_colours(theme)` (ink and light per theme, one
  colour in high contrast).
- `crates/wakeshot-windows/src/tray/menu.rs` and `crates/wakeshot-core/src/view.rs` (`MenuItem::Label`) and
  `crates/wakeshot-core/src/present.rs`: how the menu gets its rows, so you can pick how the painter learns the state.
- `docs/brand.md` (tray glyph and Windows menu parts) and `docs/research/tray-menu-look.md` ("Pick").

## Deliverables

1. **The glyph in the status row.** The `Label` row draws the tray's glyph for the current state in the icon column,
   at the menu's icon size (`ICON`, 16 epx, scaled by dpi like the other sizes: 16, 20, 24, 28, 32 px at 96 to 192
   dpi), from `art.rs` (the same frames the tray uses at that pixel size, the hand-tuned ones at 16, 20 and 24), centred
   in the column like the other row icons. Colours: as the tray icon for the menu's theme (`glyph_colours`), so the
   light is Moss while holding; in high contrast one colour, the colour the menu uses for that row's text. Check the
   ink and the light keep 3:1 against the menu background in light and dark (drawn pixels, as `art.rs` measures).
   The text, its column, the ellipsis and the screen reader text do not change.
2. **How the painter learns the state:** your pick, the smallest change that keeps the contracts clean. A new field or
   variant in `wakeshot-core` `view.rs` is allowed if needed; say why in the Result. Fakes behind `test-support` and
   every match on `MenuItem` keep compiling.
3. **Tests:** the status row has glyph ink in the icon column for each state and none of it in the check column; the
   text starts at the same x as before; light, dark and high contrast; 96, 120 and 144 dpi. Golden tests that already
   hold the menu renders or review sheets are updated by their writer, not by hand.
4. **Docs and renders:** rewrite the offscreen menu renders `docs/research/img/ux/menu-render-*.png` with their
   example, and make one sheet for Haik: the menu in holding, idle and off, dark and light, 100% and 150%
   (`docs/research/img/ux/menu-render-status-glyph.png`). One short line where the docs describe the Windows menu
   (`docs/brand.md` or `docs/architecture.md`, wherever the menu's look is written) for the glyph and for the one-line
   status with several holds (Haik's two decisions, 2026-11-05).

## Acceptance checks

- Whole workspace, both targets: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked --target
  <t> -- -D warnings` and the same without `--all-targets`; `cargo test --workspace --locked`; `cargo build --release
  --workspace --locked --target <t>`; `<t>` is `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (ARM64 needs the
  Build Tools `VC\Tools\Llvm\x64\bin` on that command's PATH only, `CLAUDE.md`). Never set `RUSTFLAGS`.
- The sheet shows the glyph centred in the icon column at both scales, the text column unchanged.

## Out of scope

Menu words, the order of rows, the tray icon itself, the status line's structure, `TEXT_MAX`, the Night sky status
colour (a separate finding), screen pictures, the Mac.

## Questions for Haik

## Found outside scope

1. `docs/product.md` (UX, tray icon states): "The brand colours are otherwise only in the app icon, the page, the
   README header, the social card and the media." Moss now also shows in the menu, as the same tray glyph on the
   status line while holding. Suggested words: "... only while holding does the light take the brand colour, Moss,
   in the tray and on the menu's status line". Not changed: `product.md` is not this card's file.

## Log

- 2026-11-05 ~20:10: card written by the orchestrator.
- 2026-11-05: worker started; reading `menu_paint.rs`, `art.rs`, `icon.rs`, `menu.rs`, `view.rs`, `present.rs`.
- 2026-11-05: design pick: the status row carries its own state, `MenuItem::Label { text, icon: Option<IconState> }`
  in `view.rs`; `present` sets `icon` from the same `IconState` as `TrayView.icon`. Why: the painter stays one item
  at a time (`paint_item(item, ..)`), `Columns::of` stays a per-item rule, and the trait `MenuView` and its fakes keep
  their signature. Passing the state through `MenuView::show` instead would touch as many files and make every
  `Label` draw a state it was not given. `Option`, since a label in general (tests, a submenu) has no state. Colours:
  a new `Palette::status` (`art::ON_LIGHT` or `ON_DARK`, high contrast one colour = the row's text colour), so
  `icon.rs` and `art.rs` stay as they are. Change it when the menu gets a second row with a picture of its own.
- 2026-11-05: built. `view.rs` `Label { text, icon }`; `present` sets it; `menu_paint` `Icon::{Font, Tray}`,
  `tray_glyph` (the tray's `tray_mask` + `colorize` at `layout.icon`, laid over the row by alpha, box at
  `icon_left`, centred on the text line), `Palette::status`; match sites in `menu.rs`, `testing.rs`, `ux_menu`,
  `ux_all`. x64 clippy all targets clean; `tray::menu` tests 36 passed. Pick on the glyph's place: the box the font
  icons take (the column's left edge, as WinUI puts a row icon), not the middle of the 28 epx column, so it lines up
  with the icons under it; measured, its ink centre equals the eye's at 96 to 192 dpi.
- 2026-11-05: renders written by `renders_for_review` into scratch, `menu-render-*.png` (11, the sheet included)
  copied to `docs/research/img/ux/`; the `menu-check-armenian-*` scratch checks stay out of the repo. Sheet viewed,
  zoomed at 100% and 150%: glyph in the icon column, centred with the row icons, text column unchanged.
- 2026-11-05: docs: one bullet in `docs/brand.md` (Tray icon, "The menu shows it too"), the contract cell in
  `docs/architecture.md` (`Label { text, icon }`, which the change made stale), a `docs/research.md` section ("Menu
  status glyph: place and contrast").
- 2026-11-05 20:39 to 20:41: x64 checks all exit 0: fmt check, clippy all targets, clippy libs, release build, `cargo
  test --workspace --locked` (core 706 passed 1 ignored; windows 615 passed 23 ignored; the rest pass). Nothing
  refused beside Haik's installed Wakeshot. Then a test comment reworded; ARM64 run started (its fmt check covers
  that edit).
- 2026-11-05 20:41 to 20:43: ARM64 checks all exit 0 (fmt, clippy all targets, clippy libs, release build; Build
  Tools LLVM bin on that run's PATH only). x64 clippy all targets and the `menu_paint` tests run again after the
  comment edit: clean, 23 passed. Scratch build folder deleted.

## Result

Done. The menu's status row draws the tray's held light for the current state in its icon column.

- **Contract** (`crates/wakeshot-core/src/view.rs`): `MenuItem::Label(String)` became `Label { text, icon:
  Option<IconState> }`. Why: the glyph belongs to the row, so `paint_item` still paints one item from its own data,
  `Columns::of` stays a per-item rule, and `MenuView` and its fakes keep their signature. `present.rs` sets `icon`
  from the same state as `TrayView.icon`; its test helper checks both agree in every state.
- **Painter** (`menu_paint.rs`): `icon_for` gives `Icon::Font(u16)` or `Icon::Tray(IconState)`; `tray_glyph` draws
  `art::tray_mask` + `colorize` at `layout.icon` (16, 20, 24, 28, 32 px at 96 to 192 dpi), alpha-blended in the box
  the font icons take (column's left edge, centred on the text line). New `Palette::status`: `art::ON_LIGHT` /
  `ON_DARK`; high contrast one colour, the row's text colour. Text, its column, ellipsis, screen reader text unchanged.
  `art.rs` and `icon.rs` not changed.
- **Match sites**: `menu.rs`, `testing.rs` (test), examples `ux_menu.rs`, `ux_all.rs`.
- **Tests**: `status_row_draws_the_tray_glyph` (every pixel: the tray frame in the box, the rest equal to the line
  without a glyph; ink in the icon column, none in the check column, text starts at the same x; Moss only while
  holding; light, dark, high contrast; 96 to 192 dpi), `status_glyph_sits_where_the_row_icons_sit` (same ink centre
  as the Automatic eye at every scale), `status_glyph_keeps_3_to_1` (drawn pixels, plain and colour-blind: lowest
  4.98, colour-blind 4.76, the deep Moss on the light menu), plus updated palette, icon, column and grayscale tests.
- **Renders**: `renders_for_review` rewrote `docs/research/img/ux/menu-render-*.png` (10) and adds the sheet
  `docs/research/img/ux/menu-render-status-glyph.png` (holding, idle, off; dark, light; 100%, 150%; menus built by
  `present::build`). Sheet viewed and zoomed: glyph centred in the icon column at both scales, text column unchanged.
- **Docs**: `docs/brand.md` one bullet (the glyph and the one-line status, Haik's two decisions);
  `docs/architecture.md` contract cell updated; `docs/research.md` "Menu status glyph: place and contrast".
- **Checks**: x64 and ARM64: fmt, clippy all targets, clippy libs, release build all clean; `cargo test --workspace
  --locked` all pass (core 706, windows 615). Nothing refused beside the installed Wakeshot.
- **Open**: Found outside scope 1 (`product.md` line on where Moss shows). Visual note for Haik: the glyph's ink is
  narrower than the Fluent icons below it (12 of 16 px at 100%), since it keeps the tray frame's padding; it is the
  same frame the taskbar shows.
