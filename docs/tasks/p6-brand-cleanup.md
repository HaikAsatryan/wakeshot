# Remove the cup from the menu and the dropped icon designs from the code

Phase: 6 (brand review). Status: done. Needs Haik: no (answered 2026-11-05: the clock). Agent:
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

- The screen pictures still show the cup and need the screen session:
  `docs/research/img/ux/menu-screen-dark-main.png`, `-dark-sub.png`, `-light-main.png`, `-light-sub.png`, and their
  stand-in copies `media/public/shots/menu-dark-main.png`, `menu-dark-sub.png`, `menu-light-main.png`,
  `menu-light-sub.png` (named in `media/data/shots.json` and
  `media/CREDITS.md`). Anything `media/` builds from those shots (the draft video and poster) needs a rebuild after.
- `docs/research/brand-windows.md` line 424 calls `EC32` "today's glyph"; the menu now draws `E823`. Not this card's
  file.
- `docs/phases/05-tray-ux.md` log lines (around 250 and 314) say Cup, Eye and Sky stay drawable in `art.rs`. They are
  dated history; phase files are out of scope.

## Log

- 2026-11-05: card written by the orchestrator. Haik's answer to question 1: not yet asked when written.
- 2026-11-05: Haik answered question 1 with the default: a clock from Segoe Fluent Icons ("All default answers are
  good"). `brand-windows.md` question 5: use `E823` ("Recent", a clock, in Segoe Fluent Icons and Segoe MDL2 Assets);
  `E917` ("Clock") is missing from MDL2, our fallback font.
- 2026-11-05: worker started. Hashes before: `assets/wakeshot.ico` sha256 `17f7bc3e...a23ba`,
  `assets/wakeshot-notice.png` sha256 `8f3b516c...618fb`. Only other card in progress: `p6-brand-research-macos`
  (docs only).
- 2026-11-05: code done. `art.rs`: Cup, Eye, Sky and their helpers gone; `Concept`, `PICK`, `concept_mask` gone,
  `concept_app_icon` folded into `app_icon`; sheets draw only the Sunrise. `menu_paint.rs`: Keep awake `E823`, new test
  `item_glyphs_are_in_both_icon_fonts`. fmt, both clippy runs and the art, menu and notice icon tests pass.
- 2026-11-05: pictures rewritten. Icon and notice icon byte-equal (trial in scratch first, then the repo). Sheets
  1892x1248 (were 1892x4960), the Sunrise rows pixel-equal to the old ones; app sheet 1508x576, equal to the old top
  rows. Menu renders: only the Keep awake icon cell differs from the old ones. `check-art.mjs` passes, worst 0.
- 2026-11-05: docs done, every acceptance check run (Result), build folder deleted. Done.

## Result

Done 2026-11-05.

**What changed**

- Menu: Keep awake draws the clock `E823` ("Recent", Haik's pick) instead of the cup `EC32`. The comment above the
  glyph constants says why. Tests: the icons test pins `E823`; a new test `item_glyphs_are_in_both_icon_fonts` checks
  that all eight menu glyphs are in Segoe Fluent Icons and Segoe MDL2 Assets (when installed), with `E917` missing
  from MDL2 as a control that the check can see a gap.
- Art: Cup, Eye and Sky are gone from `art.rs` with every helper only they used (`cup`, `steam`, `eye`,
  `closed_eye`, `sky`, `moon`, `frame`, `glyph`). `Concept`, `PICK` and `concept_mask` are gone; `tray_mask` draws the
  Sunrise directly and `concept_app_icon` folded into `app_icon`. No public name another file uses changed. The tests
  that looped over the concepts now check the Sunrise alone and print its measured shares. Review sheets show only the
  Sunrise; the pick bar is gone.
- Pictures: `icon-sheet-dark|light|contrast.png` (1892x1248, were 1892x4960; the Sunrise rows pixel-equal to the old
  ones), `icon-app.png` (1508x576, equal to the old top rows), and the nine
  `menu-render-<96|120|144>-<light|dark|contrast>.png` (only the Keep awake icon cell differs from the old ones).
- Docs: `docs/research.md` menu glyph list (Tray UX: menu painting) and one "Now" bullet in "Tray UX: icon art".
  `docs/brand.md`: no line was wrong, so unchanged.
- Scope call: the sheet layout kept the Sunrise rows where they were, so `check-art.mjs` needed no logic change; I
  fixed its one comment that said "the Sunrise is the first concept", and three comments in `media/src/art/sunrise.ts`
  that named `concept_app_icon` (now `app_icon`). Comments only; no code in `media/` changed.

**Files changed:** `crates/wakeshot-windows/src/tray/art.rs`, `crates/wakeshot-windows/src/tray/menu_paint.rs`,
`media/scripts/check-art.mjs` (comment), `media/src/art/sunrise.ts` (comments), `docs/research/img/ux/icon-sheet-*.png`
(3), `docs/research/img/ux/icon-app.png`, `docs/research/img/ux/menu-render-*.png` (9), `docs/research.md`, this card.
`assets/wakeshot.ico` and `assets/wakeshot-notice.png` were rewritten byte-equal.

**Checks**

- `cargo fmt --all --check`: pass.
- `cargo clippy -p wakeshot-windows --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`: pass;
  without `--all-targets`: pass.
- `cargo test -p wakeshot-windows --locked`: 592 passed, 0 failed, 24 ignored.
- Asset hashes (sha256), before and after the rewrite, the same: `assets/wakeshot.ico`
  `17f7bc3e57017e3ed4cacf1127d6eab7a014e33108ba3fde5e4b060ae13a23ba`; `assets/wakeshot-notice.png`
  `8f3b516c5382ee0b45b973f72cbd5b65b9fb8dd70cc0520745b75bd0227618fb`. Written first into a scratch folder
  (`WAKESHOT_ART_DIR`), then into the repo.
- `node media/scripts/check-art.mjs`: pass, worst diff 0, before and after.
- Search for `cup`, `Cup`, `EC32`, `eye(`, `sky(`, `moon(` in `crates/`: no match. The only eye left is the Automatic
  glyph `E890` and its comment.

**Open items:** the screen pictures under Found outside scope, for the screen session.

**Decisions (mine):** the sheets keep the Sunrise rows at the same places, so the media port check needs no change;
change it when the sheets get a new layout. The font test skips a face that is not installed (a CI image may lack
them) instead of failing; change it when CI images are known to carry both fonts.
