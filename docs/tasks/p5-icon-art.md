# Design the tray icons and the app icon as code

Phase: 5. Status: done. Needs Haik: one look at the icon sheet, not blocking (batched for the plan sign-off or
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

None blocking. His one look at the sheets is batched for `p5-haik-check`: the default is the Cup (see Result).

## Found outside scope

Problems seen but not fixed.

- 2026-11-04 01:24 to 01:26: `crates/wakeshot-windows/src/install.rs` (another card's file, mid-edit) failed to compile
  in clippy, then failed clippy's `chunks_exact` with a constant size lint. Gone at 01:27; noted in case it comes back.
- Track note 3.2 (`docs/phases/05-tray-ux.md`) leaves out a 256 px frame because "a small one needs a deflate encoder,
  which means a crate". `tray::art::png` is now a stdlib deflate (fixed codes, runs, palette), so a PNG-compressed
  256 frame needs no crate. Its size in the exe was not measured. The orchestrator decides; this card did not add one.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 00:55: started by a `wakeshot-worker`. Read the card, the plan, track notes 1, 3, 4 and 7.
- 2026-11-04, between 00:55 and 01:20 (clock not read for these two lines): measured the pictures with a stdlib
  Python script (scratchpad `p5-icon-art\pngread.py`, `measure*.py`): dark taskbar 202021, light taskbar d8dadc,
  hover plates 2d2d2e and ecedee, Windows' glyph ffffff on dark; on light, Windows' glyphs in the Quick Settings
  picture are 191919 to 1b1b1b (they fit black at alpha e4 over the plate, worked out from the numbers). Windows' own
  tray glyphs at 24 px: about 1.5 px strokes, wifi 24 px wide. Read Microsoft's app icon pages (sizes table: tray 16
  to 64 px for 100% to 400%) and the Fluent icon keyline (2 px at 20 px).
- Same span: wrote the unit tests and stub bodies first; 11 of 12 failed as expected (only the CRC check passed on
  its own).
- 2026-11-04 01:20: built the art (three concepts, app icon, ICO, PNG). Trial renders in the scratchpad showed a
  soft cup handle at 16 and 20 px (tightened its corners) and stubby steam at 24 px (steam now runs to a gap above
  the rim). A stored PNG of a sheet would be 28 MB, so `png` deflates with fixed codes and runs, and uses a palette
  when an image has at most 256 colours: sheets 107 KB, the app sheet 333 KB. Python's zlib inflates the streams and
  every chunk CRC matches. Wrote `assets/wakeshot.ico` and the four PNGs with the ignored writer; all 12 art tests
  pass.
- 2026-11-04 01:26: rustfmt clean; release builds of `wakeshot-tray` and `wakeshot-cli` pass for x64 and ARM64, and
  both x64 exes carry the new icon (.NET `ExtractAssociatedIcon`: orange disc, transparent corner). Clippy fails at
  once in `crates/wakeshot-windows/src/install.rs` (another card's file, being edited: its lines moved between runs;
  first unresolved functions, then `chunks_exact` with a constant size); nothing in `art.rs`. Waiting to run again.
- 2026-11-04 01:27: clippy again, with and without `--all-targets`, x64 and ARM64: all four clean (`install.rs` had
  settled). Art tests at 01:28: 12 passed, 1 ignored (the writer).
- 2026-11-04 01:29: Result written, Status done. Deleted the build folder; the measuring scripts stay in the
  scratchpad (`p5-icon-art\*.py`).

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and the paragraph for
Haik.

**Done.** Three tray glyph concepts as code, each with holding, idle and off; the app icon; ICO and PNG writers; the
icon file and the review sheets.

Files changed (all owned):
- `crates/wakeshot-windows/src/tray/art.rs`: the art, the writers, 12 unit tests and the ignored writer
  `tray::art::write_art_files` (set `WAKESHOT_ART_DIR` to write a trial set elsewhere and leave the repo alone).
- `assets/wakeshot.ico`: frames 16, 20, 24, 32, 40, 48 and 64, 43,006 bytes, made by the writer.
- `docs/research/img/ux/icon-sheet-dark.png`, `icon-sheet-light.png`, `icon-sheet-contrast.png` (107 KB each),
  `icon-app.png` (333 KB).
- `docs/research.md`: new section "Tray UX: icon art" (measured colours, Microsoft's guidance, sizes, padding, stroke,
  the pick, the ICO and PNG checks).

**For Haik** (one look, not blocking). Sheets: in each tray sheet, rows are Cup, Eye and Sky, each with holding, idle
and off; a grey bar at the left marks the pick. Columns: 16, 20, 24, 32, 40 and 48 px at real size, then 8 times
larger. Dark and light use the measured taskbar colours; contrast is a sample yellow on black. `icon-app.png` shows
each concept's app icon at every frame size on light and dark, real size then 4 times larger.
- **Cup (picked).** Holding: a full cup with three wisps of steam. Idle: an empty cup. Off: the empty cup struck
  through. Why: a cup is the plain, widely known sign for "stay awake", and it ties to the name (a wake shot). The
  idle cup is quiet, so with no work going on the icon says "nothing happening". Its strokes are straight lines on
  whole pixels, so it is the sharpest of the three at 16 to 32 px (at most 13% grey pixels). Risk: PowerToys Awake
  also uses a cup (from memory, not checked here).
- **Eye.** Holding: an open eye. Idle: a closed eye with lashes (the PC sleeps). Off: the closed eye struck through.
  Clear when holding, but idle and off read alike, and it is all curves, so it blurs at 16 px (up to 42% grey).
- **Sky.** Holding: a sun. Idle: a moon. Off: the moon struck through. The meaning is the clearest (awake, sleeps as
  usual), but it looks like Windows' own brightness and night light glyphs, and the struck-through moon is messy at
  16 px.

His answer is one word (cup, eye or sky) or "redo". A new pick changes `PICK` in `art.rs`; then run the writer
(`cargo test --locked -p wakeshot-windows tray::art::write_art_files -- --ignored`) so the icon file follows, and the
golden test checks it.

Decided by the worker, for Haik's review (each with its trigger):
1. The Cup as the pick (above). Change it when Haik picks another.
2. App icon: the holding cup in white on an orange disc (e06a22 at the top left to c2501a at the bottom right).
   Why: a plate keeps 3:1 on light, dark and accent backgrounds and stays readable at 16 px; orange stands apart from
   the many blue icons. Change it when Haik wants an unplated cup or another colour.
3. Strokes in whole pixels: 1 px at 16 and 20 px, 2 px at 24 to 36. Windows' own glyphs use about 1.5 px at 24 px,
   blurred; ours are sharp and a little heavier. Change it when the cup looks heavy next to Windows' icons at 150%.
4. Padding size / 16 (1 px at 100%), between Fluent's keyline and Windows' tray glyphs, which fill the slot. Change it
   when the cup looks too large or too small next to its neighbours on the real taskbar (`p5-tray-icon` Part B).
5. Glyph colour on light: solid 1a1a1a, what Windows' own light glyphs measure. White on dark.
6. Off is the idle glyph struck through, with a gap each side of the bar, as Windows marks its own off states.
7. `png` compresses (fixed-code deflate with runs, and a palette up to 256 colours) instead of the card's stored
   deflate: a stored sheet would be 28 MB. Still stdlib only.
8. No polygon primitive: no shape needed one (disc, ring, arc, rounded rect and round-capped line cover all three
   concepts). Add one when a design needs it.

**Public API as built** (`wakeshot_windows::tray::art`, no Windows API in the file):
- `Mask { size: u32, alpha: Vec<u8> }` (top-down rows); `Rgba { width: u32, height: u32, pixels: Vec<u8> }` (straight
  alpha RGBA, top-down).
- `tray_mask(state: IconState, size: u32) -> Mask`; `colorize(mask: &Mask, rgb: [u8; 3]) -> Rgba`;
  `app_icon(size: u32) -> Rgba`; `app_icon_file() -> Vec<u8>` (the bytes of `assets/wakeshot.ico`);
  `ico(frames: &[Rgba]) -> Vec<u8>` (square frames, 1 to 256 px; panics otherwise); `png(img: &Rgba) -> Vec<u8>`;
  `padding(size: u32) -> u32`.
- Constants: `TRAY_SIZES` (16 to 64), `APP_ICON_SIZES`, `GLYPH_ON_DARK` (ffffff), `GLYPH_ON_LIGHT` (1a1a1a),
  `TASKBAR_DARK` (202021), `TASKBAR_LIGHT` (d8dadc), `HOVER_DARK` (2d2d2e), `HOVER_LIGHT` (ecedee).
- Review only, removed by `p5-haik-check`: `Concept { Cup, Eye, Sky }`, `Concept::ALL`, `PICK`, `concept_mask`,
  `concept_app_icon`.
- For `p5-tray-icon`: `colorize(&tray_mask(view.icon, size), colour)` with `GLYPH_ON_DARK`, `GLYPH_ON_LIGHT` or the
  high contrast text colour (`theme::Rgb` converts to `[u8; 3]`). A 32-bit DIB for `CreateIconIndirect` is BGRA, so
  swap R and B. For `p5-notices`: the exes' icon resource 1 holds the same seven frames; `app_icon(size)` draws any
  size. `menu_paint` can write its renders with `png`.

**Checks** (build folder `<scratchpad>\p5-icon-art\target`, deleted at the end):
- `cargo test --locked -p wakeshot-windows tray::art`: 12 passed, 1 ignored (the writer). Names:
  `every_state_draws_at_every_size`, `states_differ_in_shape`, `strokes_are_crisp_at_small_sizes`,
  `glyph_contrast_on_taskbars`, `glyph_stays_inside_its_box`, `ico_frames_parse_back`, `png_is_valid`,
  `app_icon_file_matches_the_art`, and four more: `colorize_keeps_alpha_and_sets_colour`,
  `app_icon_draws_at_every_frame_size`, `png_compresses_flat_areas`, `crc_table_matches_the_bitwise_crc`. Written first
  against stubs: 11 of 12 failed then.
- The writer ran just after 01:20 and wrote the five files; `app_icon_file_matches_the_art` passes after it (exact
  compare: no `sin` or `cos` in the shapes).
- `rustfmt --edition 2024 --check` on `art.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows` with and without `--all-targets`, `-D warnings`, x64 and ARM64: all four
  clean at 01:27 (earlier runs stopped on `install.rs`, see Found outside scope).
- `cargo build --release --locked -p wakeshot-tray -p wakeshot-cli`, x64 and ARM64: pass (no `velopack` yet, so no
  clang needed). Both x64 exes carry the new icon (.NET `ExtractAssociatedIcon`).
- `windows::` in `art.rs`: none.
- Only the owned files and the new research section changed.
- Outside the tests: Python's `zlib` inflated all four PNGs and every chunk CRC matched; .NET `System.Drawing.Icon`
  loaded all seven ICO frames with a transparent corner.

**Open items:** Haik's look (`p5-haik-check`); after it, remove the other concepts. Not tested on ARM64 hardware (CI
or a device runs the golden test there). The 256 px frame note under Found outside scope.
