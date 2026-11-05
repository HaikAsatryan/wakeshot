# The menu's status row shows the tray's state glyph in its icon column

Phase: 6 (Haik's review of the installed build). Status: in progress. Needs Haik: no (Haik 2026-11-05 ~20:10: "1 yes, 2 yes,
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

## Log

- 2026-11-05 ~20:10: card written by the orchestrator.

## Result
