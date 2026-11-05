# Remove the cup from the menu and the dropped icon designs from the code

Phase: 6 (brand review). Status: ready. Needs Haik: no (answered 2026-11-05: the clock). Agent:
`wakeshot-worker`. Size: about 80K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-polite-quit-notice-retry` (another session's card; it edits `wakeshot-windows`, and a crate builds
whole) and `p6-brand-research-windows` (its question 5 verifies the glyph code points).
Parallel with: the brand research cards and `p6-brand-directions` (they edit no code). Nothing else that edits
`wakeshot-windows` or `media/`.
Real laptop: no power settings, no screen. Offscreen renders only.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-brand-cleanup\target`.

## Goal

Haik (2026-11-05): "Remove obsolete elements from UI like cup and orange". The brand dropped the Cup on 2026-11-04
(`docs/brand.md`), but the menu still shows a coffee cup on Keep awake (`menu_paint.rs`, `ICON_KEEP_AWAKE = 0xEC32`),
and `tray/art.rs` still draws the dropped Cup, Eye and Sky designs (its own doc says `p5-haik-check` would remove them;
it did not). Orange is already gone from all code and pages (checked by the orchestrator with a search, 2026-11-05):
only `docs/brand.md` and research history name it, as the reason it was dropped. Leave those.

## Read first

- `docs/brand.md`; `docs/research/brand-windows.md` question 5 (glyph candidates with verified code points).
- `crates/wakeshot-windows/src/tray/menu_paint.rs` lines 55 to 70 and its tests.
- `crates/wakeshot-windows/src/tray/art.rs`: `Concept`, `PICK`, `concept_mask`, `concept_app_icon`, `glyph`, `cup`,
  `steam`, `eye`, `closed_eye`, `sky`, `moon`, `sheet`, `app_sheet`, `write_art_files`, and the tests that loop over
  `Concept::ALL`.
- `media/scripts/check-art.mjs` and `media/src/art/sunrise.ts` (they read the review sheets that `art.rs` writes).

## Deliverables

1. **Menu.** Keep awake gets a new glyph instead of the cup: the one Haik picks (see Questions), else the default.
   Update the comment above the glyph constants, the tests, and `docs/research.md` line 2571 (the menu glyph list).
2. **Art.** Remove the Cup, Eye and Sky designs and every function only they use. The Sunrise stays exactly as it
   draws now: `assets/wakeshot.ico` and `assets/wakeshot-notice.png` must come out byte-equal when rewritten. Simplify
   what only existed to choose between designs (`Concept`, `PICK`, `concept_*`) where that makes the code plainer;
   keep the public names other files use, or update their callers in this crate. Review sheets show only the Sunrise.
3. **Pictures.** Rewrite the review sheets (`write_art_files`) and the offscreen menu renders
   (`docs/research/img/ux/menu-render-*.png`, however the code writes them). The screen pictures
   (`menu-screen-*.png`, `media/public/shots/menu-*.png`) need the screen session: list them under Found outside scope.
4. **Media check.** If the sheet layout changed, update `media/scripts/check-art.mjs` so it still compares the port with
   the sheets, and run it.
5. **Docs.** `docs/research.md` "Tray UX: icon art": one "Now" line that the dropped designs are gone from the code.
   `docs/brand.md` keeps its reasons (the Cup history is the why); fix only lines that are now wrong.

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy -p wakeshot-windows --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`, and the
  same without `--all-targets`.
- `cargo test -p wakeshot-windows --locked`.
- `assets/wakeshot.ico` and `assets/wakeshot-notice.png` byte-equal before and after (hashes in the Result).
- `node media/scripts/check-art.mjs` passes.
- A search for `cup`, `Cup`, `EC32`, `eye(`, `sky(`, `moon(` in `crates/` finds nothing left but the Automatic eye glyph.

## Out of scope

The brand itself (colours, mark, tray style): `p6-brand-directions` and a later apply card may change all of it. Do not
redraw anything. `docs/plan.md` and phase files. Files of `p6-polite-quit-notice-retry` (`tray/icon.rs`, `tray/menu.rs`,
`app/window.rs`, `notify.rs`) unless that card is done.

## Questions for Haik

1. The Keep awake menu glyph instead of the cup. Default: a clock from Windows' own icon font (every Keep awake choice
   is a length of time; the code point from `brand-windows.md` question 5). Other choices: a sun, a lightbulb, or our
   own sun mark drawn by `art.rs` (on brand, a little more code).

## Found outside scope

## Log

- 2026-11-05: card written by the orchestrator. Haik's answer to question 1: not yet asked when written.
- 2026-11-05: Haik answered question 1 with the default: a clock from Segoe Fluent Icons ("All default answers are
  good"). `brand-windows.md` question 5: use `E823` ("Recent", a clock, in Segoe Fluent Icons and Segoe MDL2 Assets);
  `E917` ("Clock") is missing from MDL2, our fallback font.

## Result
