# Design the tray icons and the app icon as code

Phase: 5. Status: ready. Needs Haik: one look at the icon sheet, not blocking (batched for the plan sign-off or
`p5-haik-check`). Agent: `wakeshot-worker`. Size: about 120K tokens.
Real laptop: no (pure code; pictures are rendered, not captured). Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-icon-art\target`.
Runs after: the contracts change card (it creates `tray/art.rs` as a stub and a placeholder `assets/wakeshot.ico`
that both exes embed). Parallel with: `p5-theme`, `p5-text`, Part A of `p5-sounds` and `p5-notices`, and any card of
another track.

## Goal

The art for the three tray icon states (holding, idle, off) and the app icon, as pure Rust that draws crisp pixels at
any size, plus the files built from it: `assets/wakeshot.ico` and a preview sheet Haik can judge at a glance.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 1, 3 and 4).
- `docs/product.md` UX: "Tray icon states: holding, idle, off. It must look good, be crisp at every display scale, and
  be correct on light and dark taskbars and in high contrast."
- `docs/research.md`, "Tray, menu and notifications": "DPI", "`LoadIconMetric` follows the system DPI", "Size".
- Pictures of Windows' own tray glyphs on this laptop: `docs/research/img/tray-icon-switch3-light-taskbar.png`,
  `tray-icon-switch4-dark-taskbar.png`, `tray-taskbar-after-promote.png`.
- `spikes/tray/src/glyph.rs` and `spikes/tray/build.rs` (the placeholder ring, supersampling, the ICO writer).
- Microsoft: "App icon design" and "Iconography in Windows" (learn.microsoft.com/windows/apps/design/style/
  iconography), current pages, for sizes, padding and the system tray glyph style.

## Owns

`crates/wakeshot-windows/src/tray/art.rs`; `assets/wakeshot.ico`; new files `docs/research/img/ux/icon-*.png`.
The placeholder `assets/wakeshot.ico` exists and both exes embed it through `crates/build-resources.rs`; replacing
the file needs no build change. No card edits `build.rs` or `build-resources.rs` (`p1-contracts-split` edit 8).

## Deliverables

1. **The design.** Two or three concepts for the tray glyph, each with all three states, in the style of Windows 11's
   own tray glyphs: monochrome, simple strokes, readable at 16 px. The states must differ in shape, not only in
   fill or colour (high contrast and colour blindness). Off is Wakeshot switched off (Automatic off and nothing
   holds); idle is on but nothing holds, the PC sleeps as usual; holding is keeping the PC awake. Pick one and say
   why; keep the others drawable until Haik has looked (a `Concept` value), then they go (`p5-haik-check`). The app
   icon: a coloured form of the same mark, for Explorer, Start, the installer and the notice banner.
2. **Drawing code**, no Windows API in this file (it could move to the core later):
   - `pub fn tray_mask(state: IconState, size: u32) -> Mask` (alpha only, top-down rows), and
     `pub fn colorize(mask: &Mask, rgb: [u8; 3]) -> Rgba` (straight alpha). The colour comes from the caller
     (`p5-tray-icon` picks white, near black or the contrast colour).
   - `pub fn app_icon(size: u32) -> Rgba`.
   - Shapes from a few primitives (circle, ring, arc, rounded rect, round-capped line, polygon) with supersampling,
     and strokes snapped to the pixel grid at small sizes, so 16, 20, 24 and 32 px are sharp, not blurred.
   - `pub fn ico(frames: &[Rgba]) -> Vec<u8>` (32-bit frames with an AND mask) and `pub fn png(img: &Rgba) -> Vec<u8>`
     (stdlib only: stored deflate, CRC-32, Adler-32), so other cards' tests can write previews too.
   - Avoid `sin` and `cos` in the shapes where you can: their results may differ in the last bit between x64 and
     ARM64, and the golden test runs on both. Where they stay, the golden test compares with a tolerance of 2 per
     channel.
3. **Glyph colours**, measured from the pictures above with a stdlib script (scratchpad): the taskbar background and
   Windows' own glyph colour, light and dark. Record them; the spike used white and `1c1c1c`.
4. **Files**: `assets/wakeshot.ico` with frames 16, 20, 24, 32, 40, 48 and 64 (no 256 frame: track note 3.2), and
   `docs/research/img/ux/icon-sheet-dark.png`, `icon-sheet-light.png`, `icon-sheet-contrast.png` (a sample contrast
   pair, for example yellow on black), `icon-app.png`. Each sheet: every concept, every state, at 16, 20, 24, 32, 40 and
   48 px on the measured taskbar colour, at real size and enlarged 8 times (nearest neighbour). An ignored test writes
   them: `cargo test --locked -p wakeshot-windows tray::art::write_art_files -- --ignored`.
5. **Unit tests**, written first:
   - `every_state_draws_at_every_size` (16, 20, 24, 28, 32, 36, 40, 48, 56, 64: the sizes for 100% to 400%);
   - `states_differ_in_shape` (alpha masks differ pairwise in at least a share of the glyph's pixels you set, at 16 px);
   - `strokes_are_crisp_at_small_sizes` (share of half-covered pixels under a bound you set, at 16, 20, 24 and 32);
   - `glyph_contrast_on_taskbars` (WCAG contrast of the glyph colour on the measured taskbar colours, at least 3:1);
   - `glyph_stays_inside_its_box` (the padding rule you take from Microsoft's guidance);
   - `ico_frames_parse_back` (count, sizes, offsets, pixels) and `png_is_valid` (signature, chunk CRCs, Adler-32);
   - `app_icon_file_matches_the_art` (golden: decode `assets/wakeshot.ico`, compare with a fresh render).
6. `docs/research.md`: a new section "Tray UX: icon art": measured colours, the guidance used, sizes, the pick.
7. In the Result, for Haik: one paragraph per concept and the pick, pointing at the sheets. His answer is a one-word
   pick or "redo"; it changes one value.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows tray::art`: all pass; list the names in the Result.
- The ignored writer ran once and its files are in place; `app_icon_file_matches_the_art` passes after it.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-tray -p wakeshot-cli` for both targets (the
  tray's ARM64 build is CI only once `velopack` is in and this laptop has no clang): each clean in the owned files.
- `tray/art.rs` calls no Windows API (a grep for `windows::` finds none).
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Turning the art into an `HICON`, picking the size for the taskbar, the contrast colour pick (`p5-tray-icon`).
- `build.rs` (the contracts card already embeds `assets/wakeshot.ico`), the installer icon (Phase 6), the web page.
- Icons inside menu items (a product call; not in v1).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and the paragraph for
Haik.
