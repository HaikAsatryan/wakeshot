# Apply the held light in Moss to the Windows app: tray glyphs, app icon, notice icon

Phase: 6 (brand apply). Status: done (review fixes too, `docs/reviews/p6-brand-apply.md`). Needs Haik: no (his 8
brand decisions, 2026-11-05 ~15:00, all defaults).
Agent: `wakeshot-worker`. Size: about 200K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: Haik's brand decisions (`docs/phases/06-ship.md` work log, 2026-11-05 ~15:00).
Parallel with: Part A of `p6-brand-apply-web` and `p6-brand-apply-media` (no Rust, no cargo). Nothing else that
edits `wakeshot-windows` or `assets/`.
Real laptop: no power settings, no screen, no installs. Offscreen renders only (the session may be locked).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-brand-apply-art\target`; delete it when done. Never set `RUSTFLAGS`.
Owns: `crates/wakeshot-windows/src/tray/art.rs`, `tray/icon.rs`, `tray/theme.rs`, `notify.rs` (its notice icon
constants, writer and test only), `tray/menu_paint.rs` (only if a render writer must change), `assets/wakeshot.ico`,
`assets/wakeshot-notice.png`, `site/favicon.ico` (from now on written by `art.rs`), the review sheets
`docs/research/img/ux/icon-sheet-*.png` and `icon-app.png`, the menu renders `docs/research/img/ux/menu-render-*.png`,
new pictures `docs/research/img/brand/apply-*.png`, additions to `docs/research.md`.

## Goal

The app still draws the Sunrise in night violet and sun gold. Haik picked the held light in Moss, plated on Windows,
brand colour in the tray only while holding. Draw it in `art.rs` with the craft of round 2, so `art.rs` is the one
source every other surface ports from (the page, the media, the README lockup, the social card). Quality bar: a senior
icon designer would ship it. Render every frame, look at it at 100% and enlarged, fix, look again.

## Read first

- `docs/research/brand-directions-2.md`, whole: the design, the Moss column, "What applying the pick changes".
- `docs/phases/06-ship.md` work log, line "2026-11-05 ~15:00": Haik's decisions.
- `docs/research/brand-review.md`: findings T1 to T8, N1, N2, A1 to A5, B1, X2, "The top five should-fix items".
- `docs/research/brand-windows.md`: section 1 (the half rule, icon sizes), 2 (2025 icon craft), 6 (notices); any ICO
  size list it cites. Check Microsoft's current app icon size list yourself (learn.microsoft.com, "App icon
  construction" or its 2026 successor) and cite it.
- `crates/wakeshot-windows/src/tray/art.rs` whole; in `icon.rs` `glyph_colour`, the redraw path and their tests;
  `theme.rs`; in `notify.rs` `NOTICE_ICON_*`, `notice_icon_png` and its tests.
- Round 2 design sources, copied for you to `<scratchpad>\brand-src\` (copy what you edit into your own folder; leave
  `brand-src` as it is):
  - `p6-brand-directions-2\tools\`: `marks2.py` (tray glyphs per pixel, uses round 1's rasterizer), `icons2.py` (app
    icons as SVG per size), `pal2.py` (palettes), `checks2.py` (WCAG ratios, CIEDE2000, Machado colour-blind),
    `measure.py` (vertical position on the taskbar), `render_icons.ps1`, `render2.ps1`, `shot2.ps1` (headless Edge at
    device scale 1); `out\` (`checks.md`, `look\*.png`) and `board\` hold what Haik approved.
  - `p6-brand-directions\tools\` (round 1 rasterizer `raster.py`, `colour.py`, `palette_check.py`), its `edge\` and
    `out\` (real taskbar crops of this laptop).
- Approved pictures: `docs/research/img/brand/dir2-tray-held.png`, `dir2-icon-moss.png`, `dir2-compare.png`.

## Deliverables

1. **Tray glyph** (`tray_mask`, every `TRAY_SIZES` size, three states), as `marks2.py` draws mark A:
   - holding: a filled round light over a line; idle: the same light as an outline at the same place (the light does
     not jump between states); off: the idle outline struck through, the slash inside the glyph's box (T7);
   - ink mass centre level with Windows' own glyphs (T4): target 0.455 of the box, measured with `measure.py` on the
     rendered taskbar against the chevron, Wi-Fi, volume and battery glyphs; record the offset per size;
   - crisp at 16, 20 and 24 px: whole-pixel strokes, the line solid, the ring as even as the line (T1 to T3);
     hand-tuned per-pixel frames where geometry alone does not give a clean result;
   - states differ by shape at 16 px (keep or tighten the 25% ink test).
2. **Colour while holding**: in the approved picture only the light takes the colour; the line keeps the glyph
   colour. Moss `#B6E05A` on a dark taskbar, `#3B7A2A` on a light one. Idle and off stay one colour (white, near
   black). High contrast: one colour, the window text colour, for every state and every part. Route it through
   `glyph_colour` and the theme (`icon.rs`, `theme.rs`); the glyph needs two parts (light and line), so `tray_mask`
   or `colorize` grows a second channel or a second mask: your design, kept small. Check contrast on the taskbar and
   on the hover colours (`HOVER_DARK`, `HOVER_LIGHT`), 3.0 at least (WCAG 1.4.11).
3. **App icon** (`app_icon`, `APP_ICON_SIZES`), plated Moss as `icons2.py` draws the pick: the 120 degree plate ramp
   `#628C2B` to `#3E7C3B`, a radial glow round the light, the emissive light (core `#FFFBE8`, edge `#FFEA9C`) with a
   soft bloom from 32 px up, the white bar, a soft vignette at the bottom, a soft drop shadow under the plate from 32
   px up and none at 24 px and below. 16, 20 and 24 px are simpler flat drawings of the same mark, tuned per pixel:
   no loose single pixels (A4). Frames up to 256 (B1), the size list from Microsoft's current guidance; the 256 frame
   is stored as PNG inside the ICO.
   - Draw it in `art.rs` with plain arithmetic. `sqrt` is fine; no `sin`, `cos`, `exp`, `powf` or `ln`: they may differ
     in the last bit between x64 and ARM64, and the golden tests compare exactly. A soft blur as repeated box blurs on
     integers is exact. No new crate and no build-time renderer: a `Cargo.toml` or `Cargo.lock` change needs the
     orchestrator. If the quality bar cannot be met this way, stop and say why under Questions for Haik.
   - Compression: check what `art.rs`'s `deflate` does today. The 256 PNG frame must be really compressed (LZ77 plus
     Huffman as needed). The ICO is embedded in both exes (`crates/build-resources.rs`): record the ICO size and both
     release exe sizes before and after (release tray was 2.71 MB, CLI 0.56 MB).
4. **Notice icon** `assets/wakeshot-notice.png` (`NOTICE_ICON_SIZE`, 48 px today): from the new app icon. The notice
   header draws it at about 16 logical px (N1): downscale it to 16, 20 and 24 px as the shell would and look at it on
   both notice themes (colours in `brand-windows.md` section 6). If the full drawing reads badly that small, pick the
   better source (for example the small flat drawing at 48 px) and record why.
5. **Files the art writes**: `assets/wakeshot.ico`, `assets/wakeshot-notice.png`, `site/favicon.ico` (16, 32 and 48
   px frames of the app icon, added to the art writer so the page's favicon is the same art), the review sheets
   (`icon-sheet-dark.png`, `-light.png`, `-contrast.png` with a row for holding in the brand colour; `icon-app.png`).
   Check whether the menu renders (`menu_paint.rs` writer) show any art; regenerate them only if they do.
6. **No legacy in the files you own**: remove the old brand's code, constants and tests (`APP_LIGHT`, `APP_DARK`,
   `SUN_GOLD`, `sunrise`, `risen_sun`, `empty_sun`, `SUNRISE_GREY` and the like) and every comment that names the
   Sunrise, sun, rays, horizon, violet or gold. Module docs name the held light and Moss and point to `docs/brand.md`
   (rewritten by `p6-brand-docs-sweep`). Add this card to the `Owner:` line of each module doc you change.
7. **Tests**: golden (every asset equals what the art writes; keep the existing ones working), shape (states differ;
   slash inside the box; ink centre within 0.5 px of the target per size; no loose single ink pixel at 16, 20, 24),
   contrast (glyph parts on both taskbars and hovers per state, including the holding colours; the light's edge on the
   plate at every app icon size; the plate on light and dark Start; Microsoft's half rule at 48 px on both Start
   colours: at least 50% of the icon at 3:1).
8. **Look**: render every tray frame and every app icon frame at 100% and enlarged: tray on dark, light and high
   contrast taskbars (round 2's taskbar rebuilds and the real dark 150% crop), app icon on light and dark Start, the
   taskbar and Explorer backgrounds, the notice header on both themes. Put them side by side with the approved round 2
   pictures. Fix what a senior icon designer would fix, then look again. Keep 3 to 5 final pictures as
   `docs/research/img/brand/apply-*.png` and cite them in the Result.
9. **`docs/research.md`**: additions only, a short "Brand apply: the held light in art.rs" section with what you
   measured and how (vertical offsets, contrast numbers, half rule, ICO and exe sizes, the source of the size list).

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`; the same two for `aarch64-pc-windows-msvc` with the Build Tools `VC\Tools\Llvm\x64\bin`
  folder on that command's PATH only.
- `cargo test --workspace --locked` (no other card builds Rust while this one runs, so whole-workspace checks are
  yours to run at the end).
- `cargo build --release --workspace --locked` for both targets (ARM64 with the Llvm folder on PATH).
- The art tests above pass; a second run of the writer leaves every asset byte-equal.
- Every look picture checked by eye; the Result says what you changed after looking.
- Contrast and half-rule numbers in a table in the Result, each against its minimum.

## Out of scope

Menu words, menu glyphs (the clock stays), any user-facing text; `site/` except `favicon.ico`; `media/`;
`README.md`; `docs/brand.md` and the other docs (`p6-brand-docs-sweep`); anything Mac; power and core code. Do not
run git.

## Questions for Haik

None expected: Haik gave every decision. A design call inside the brief: decide as a senior icon designer and list it
under "Decisions for Haik's review" in the Result.

## Found outside scope

- `media/scripts/check-art.mjs` (p6-brand-apply-media Part B): the review sheets now have four rows (holding in one
  colour, holding in its colours, idle, off; layout in `sheet()` of `art.rs`), and `assets/wakeshot.ico` has 14
  frames with the 256 frame as PNG, so its `readIco` must read PNG frames. The script fails until Part B ports it.
- `site/index.html` line 9 (p6-brand-apply-web Part B): `sizes="16x16 20x20 24x24 32x32 40x40 48x48 64x64"` lists the
  old frames; `site/favicon.ico` now holds 16, 32 and 48.
- For the ports (web, media): the plate's ramp is Microsoft's 120 degrees as CSS reads it (direction (sqrt 3 / 2,
  1/2), corner to corner), not round 2's SVG `x2=0.87 y2=1`; all numbers are constants and `Plated::at` in `art.rs`.
- Stale docs for `p6-brand-docs-sweep`: `docs/phases/05-tray-ux.md` lines 68 to 69 ("frames 16 ... 64", "No 256
  frame yet"); `docs/research/brand-windows.md` lines 140 to 143 (seven frames, 43,006 bytes); `docs/brand.md` still
  the Sunrise; `docs/research.md` "Tray UX: icon art" names `SUNRISE_GREY` (history; the new section supersedes it).
- Not verified: whether Velopack's `Setup.exe` (`pack.ps1 -Icon`) carries an ICO with a PNG frame well. Windows
  itself loads it (LoadImageW, PrivateExtractIconsW); the next install run (`packaging/e2e.ps1` or a screen session)
  shows the installer's icon.
- I ran `cargo fmt -p wakeshot-windows` once before switching to `rustfmt` on my own files. If another file of that
  crate was not formatted, it is now; `cargo fmt --all --check` passes. Not checked with git (workers do not run it).
- Review fix round (2026-11-05):
  - `docs/tasks/p6-pages.md` item 2 (lines 195 to 197) says to port `site/wakeshot.svg` and the lockups by hand
    ("its port scripts are not in the repo"). Stale now: `write_art_files` writes all three and golden tests hold
    them; after an art change only `python site/social_card.py` stays a hand step. For `p6-pages` or the orchestrator.
  - `media/README.md` line 62 ("0 of 9,458,740 pixels differ") and `docs/brand.md`'s media row are true again only
    after the media worker ports the two changes in the Log (bloom, strike box); `check-art.mjs` fails until then, and
    so will the new CI job `art-port`. The media's pictures of the three icon states (`p6-media`) show the new off
    glyph only after that port.
  - Whether `actions/setup-node@v7` with Node 26 and the `.ts` import work on the CI runner: not verified (no CI run
    from a worker).

## Log

- 2026-11-05: started. Read CLAUDE.md, plan "How we work", this card.
- 2026-11-05: read brand-directions-2, the decisions line, brand-review T1 to X2, brand-windows 1, 2, 6, `art.rs`,
  `icon.rs`, `theme.rs`, `notify.rs`, round 2 tools (`marks2.py`, `icons2.py`, `pal2.py`, `checks2.py`,
  `measure.py`). Microsoft's size list read again (app-icon-construction, ms.date 2026-07-21, updated 2026-08-05):
  AppList target sizes 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256 (the union of its scale table); bare
  minimum 16, 24, 32, 48, 256. Before: `assets/wakeshot.ico` 43,006 bytes, `wakeshot-notice.png` 1,886 bytes,
  `site/favicon.ico` a copy of the ICO. Release x64 build of the old art started for the exe sizes.
- 2026-11-05: before, release x64: `wakeshot-tray.exe` 2,745,856 bytes, `wakeshot.exe` 558,080 bytes. Windows' own
  glyphs (Segoe Fluent Icons E70E, E701, E767, E83F) measured in headless Edge at every tray size: ink centre 0.477 to
  0.479 of the box (scratch `tools\sysglyphs.py`). Tray glyph prototyped in Python per size (scratch `tools\glyph.py`):
  one formula for every size (light half the box, line 3/4, gap 1/8, light placed so the holding ink centre is at
  0.455), hand-drawn 1 px rings at 16 and 20 px (the anti-aliased ring looked fainter than the line, T2).
- 2026-11-05: `art.rs` rewritten: the held light (tray, two parts: light and line), plated Moss app icon (ramp,
  glow, shade, bloom, rim, shadow by integer box blurs), 14 ICO frames with the 256 frame as PNG, `favicon_file`,
  real deflate (hash-chain LZ77 with lazy matching, fixed or dynamic Huffman per block). `icon.rs`: `glyph_colours`
  returns `GlyphColours` (ink, light); `Drawn` and `Note::Redrawn` carry it. `notify.rs`: the notice PNG comes from
  art's writer. Old Sunrise code, constants and tests removed. A temporary `look_frames` test in `art.rs` writes look
  frames to scratch: remove it before the end.
- 2026-11-05: looked at every frame (scratch `out\look-*.png`), fixed: off slash at 1 px now clears all 8 neighbours
  (4 left the ring touching the slash; idle to off at 20 px went 24% to 33%); the lit light's rim back to round 2's
  `#F7DC7F` (the card's `#FFEA9C` edge looked flat beside the approved picture); gap at least bar height + 1 (24 px
  matched round 2 again); the shadow cut at the icon's edge (9% black on the last row at 48 px) now a 96th down and a
  64th deviation (5% at 48 px, 1% at 256). Notice header: the full drawing scaled to 16 to 24 px blurred the
  light and left a grey shadow rim on the light notice; the flat drawing at 48 px reads clean, so the notice PNG is
  `app_icon_flat(48)`. Rendered vertical position matches the Rust test to 0.01 px.
- 2026-11-05: art written into the repo (`assets/wakeshot.ico` 185,835 bytes, `wakeshot-notice.png` 1,798,
  `site/favicon.ico` 15,086, sheets, `icon-app.png`); Python's own zlib read every PNG and ICO frame back. Five look
  pictures copied to `docs/research/img/brand/apply-*.png`. Temporary `look_frames` test removed. Menu renders show
  no art (text and Segoe glyphs only), so not regenerated.
- 2026-11-05: every acceptance check passed on the final code (table in Result); writer run twice, byte-equal; exes
  checked for all 14 frames; `docs/research.md` section added. Build folder deleted. Status done.
- 2026-11-05: review fix round started (`docs/reviews/p6-brand-apply.md` findings 1, 2, 3, 5, 7; the orchestrator's
  brief). Build folder `<scratchpad>\p6-brand-art-fixes\target`. The orchestrator added one item: the 20 px off
  glyph, whose slash the screen session saw cutting through the line.
- 2026-11-05: finding 1, measured. A numpy port of `draw_plated` (scratch `p6-brand-art-fixes\proto.py`) equals the
  ICO's 14 frames to 1 level. Drawn pairs (every fully covered bar pixel against the first clean plate pixel 1 or 2
  steps away): bar 3.27 to 3.36 at 30 to 60 px but 3.05 at 36 and 2.78 to 2.95 from 64 up; under the simulations 2.73
  at worst, and 2.98 already at 36. Light's core against the plate past the bloom (1.9 r): 3.25 to 3.76, simulations
  3.16 or more. Matches the review. Painting the bar over the bloom alone gives 2.85 at 64: not enough.
- 2026-11-05: finding 1, the fix tried. The bloom's tail squeezed so it ends at E r instead of 1.9 r, its value at the
  light's edge (0.44) kept: E 1.75 gives bar 2.96, 1.7 3.00, 1.65 3.05 (simulations 2.98), 1.6 3.12 (simulations
  3.05), 1.5 3.27; the core past the shorter bloom stays 3.12 / 3.04 at 1.6. Scaling the whole bloom to 1.6 r changes
  more (mean 4.3 levels, max 25) than squeezing the tail (mean 3.0, max 17). Picked stops in light radii (0, 0.8),
  (1.0, 0.44), (1.28, 0.144), (1.6, 0): mean difference to today 2.2 to 3.0 levels per frame, max 17; half rule at 48
  px 69% / 91% (was 63% / 91%). Looked at 64 px x5 and 256 px x1 on both Starts beside today: the halo a little
  tighter, the bar cleaner; same look.
- 2026-11-05: finding 1 in `art.rs`: `BLOOM_END` 1.6 and the new stops; `app_icon_contrast` now measures the pixels
  of `app_icon(size)` (bar edge pairs; flat frames the light's edge pairs; lit frames the core against the 1 px ring
  just past the bloom), plain and under Machado protan, deutan and tritan; the glyph test and the half rule take the
  simulations too. `plate_colour`'s `glow` switch and `round_the_clock` are gone (no caller left). Test output: bar
  3.12 to 4.34 (colour-blind 3.05 to 4.20), core past the bloom 3.11 to 3.60 (3.03 to 3.48), flat light edge 3.75
  to 3.95 (3.53 to 3.76), lit light edge as drawn 1.40 to 2.03 (record only), half rule 69% / 91% (colour-blind 68% /
  87%).
- 2026-11-05: the 20 px off glyph. Text dumps of the sheet: 16, 20 and 24 px all end the slash on the line's last
  pixel, with a 2 px cut before it, so the line reads as broken with a stray end pixel; 20 px shows it most (3 rows
  between light and line). The slash also ran 1 to 2 px left of the light's centre. A senior icon designer fixes
  this. Tried in a Python port (scratch `offglyph.py`): "above" (from the light's top left corner through its centre
  to a stroke above the line) and "even" (the same, overshooting both ends; fails
  `the_slash_stays_inside_the_idle_glyph` since it rises above the light). Picked "above": line whole, slash centred, glyph no larger than idle. States
  differ idle/off 28% at 16 px (was 43%), 31% at 20 (33%), 35% at 24 (48%); minimum 25%. New test
  `the_slash_leaves_the_line_whole`.
- 2026-11-05: finding 2, the art side. `art.rs` writes `site/wakeshot.svg` and both lockups (`svg_icon_parts`,
  `app_icon_svg`, `lockup_svg`, `site_svgs`), a port of the web card's `gen_icon.py` and `gen_lockup.py` that reads
  `Plated::at(256)` and the same constants the drawing uses: `glow()`, `focus()`, `shadow_drop()`, `shadow_blur()`,
  `bar_corner()` and the consts `SHADE_FROM`, `SHADE`, `LIT_RAMP`, `BLOOM`, now shared by the drawing and the SVG. The
  word's outlines are fixed path data from today's lockups (`WORDMARK`, ink right 998.47), with an assert that the
  256 plate is still margin 11, side 234. The deviation of the SVG blur is now the exact one of the box passes,
  sqrt(passes x (w^2 - 1) / 12) (`gen_icon.py` doubled it for small sizes; only 256 is written, where both give
  4.472). Golden test `site_svgs_match_the_art`. Checks: every raster file written before the refactor (bloom and
  slash only) and after it byte-equal (7 of 7); the three SVGs equal today's but for the header comment and the
  bloom (r 76.8, stops 0.625 and 0.8). Headless Edge (scratch `compare_icon.py`, the web card's) against the new 256
  frame: over white mean 0.62, max 41; over `#121610` mean 0.46, max 29; alpha mean 0.23, max 52 (today's pair: 0.64,
  0.47, 0.23); the larger values are the plate's corner pixels, as before.
- 2026-11-05: art written into the repo: `assets/wakeshot.ico` 184,720 bytes (256 frame 31,114), notice PNG
  1,798 (unchanged bytes), `site/favicon.ico` 15,086 (new bytes), the three SVGs, the four sheets. Art and notify
  tests: 47 passed. Slip: I ran one read-only `git show HEAD:site/favicon.ico` to compare a hash; nothing changed.
- 2026-11-05: looked again. A temporary test wrote the look frames (removed later); the earlier worker's
  `look.py`, `compare.py` and `shoot.ps1` (copied to my scratch) rebuilt the five `apply-*.png` pages, headless Edge
  at scale 1. Old and new frames compared: only the lit app icon frames (30 px up) and the off glyphs changed; the
  other 120 look frames are byte-equal. Mean difference to round 2's approved frames: lit frames 3.5 to 5.0 levels
  (2 to 3 before), 16 to 24 px unchanged. Looked at all five at 100% and enlarged, and at a new before/after sheet
  (`apply-fixes.png`): the halo a little tighter, the bar cleaner, the light still glows; the off glyph reads as a
  struck light over a whole line beside Windows' glyphs. Two captions updated (tray detail, compare). Replaced
  `apply-tray.png`, `apply-tray-detail.png`, `apply-icon.png`, `apply-compare.png`, `apply-notice.png`; added
  `apply-fixes.png`.
- 2026-11-05: `site/social-card.png` re-rendered with `python site/social_card.py`: 285,193 bytes (was 285,642); a
  second run gave the same SHA-256 (`1c166fd6...`). Looked at it: right.
- 2026-11-05: finding 2, the media side. `check-art.mjs` imports `node:url`, `node:path`, `./lib/png.mjs` (only
  `node:zlib`, `node:fs`) and `../src/art/heldLight.ts` (no imports): no `npm ci`. New CI job `art-port` on
  ubuntu-latest: checkout, `actions/setup-node@v7` (latest release v7.1.0, 2026-10-08, read with `gh api`) with Node
  26 (26.13.0 LTS is in setup-node's manifest; `media/README.md` asks Node 26) and `package-manager-cache: false`,
  then `node media/scripts/check-art.mjs`. Runs in about 2 s here. actionlint 1.7.12 (the copy in session
  `ca4fed46...` scratch; its zip matches the release checksum list and the exe equals the zip's): 0 findings, without
  shellcheck. The check fails now, as expected: app icon 30 to 256 px differ (bloom) and the off glyphs.
  **For the media port** (`media/src/art/heldLight.ts`), two changes in `art.rs`, both exact f32 as before:
  1. `bloom`: `t = distance / r` (was `distance / (1.9 r)`), then `stops(BLOOM, t)` with
     `BLOOM = [(0.0, 0.8), (1.0, 0.44), (1.28, 0.144), (1.6, 0.0)]` (`BLOOM_END` 1.6).
  2. `strike_box(h, size)`: `left = (size - d) / 2` (integer), `bottom = line_y - t`, box `(left, top, left + (bottom
     - top), bottom)`; used by `struck` (2 px strokes up) and `strike_pixels` (1 px) as before.
  Nothing else changes a pixel: `plate_colour` lost its `glow` switch (always on), and the shade and the light's ramp
  now read named constants whose f32 sums equal the old literals (`1.0 - 0.55` is `0.45`, `0.86 - 0.5` is `0.36`;
  the rim keeps `/ 0.14` written out, since `1.0 - 0.86` is not `0.14` in f32). The SVG side needs nothing from media.
- 2026-11-05: finding 5 (`p8-cleanup.md`: the 12:07 work log line and the two Task Manager captures) and the docs:
  `docs/brand.md` (contrast rows from the test output, the light's rule, off glyph, bloom, sizes, asset table),
  `docs/research.md` (my section's rows corrected with a note, the web and colour-blind sections pointed at the new
  section "Brand apply review fixes", social card bytes). Temporary look test removed; module doc names the SVGs.
- 2026-11-05: final checks all pass (table in Result); writer run twice, 10 files byte-equal; check_page 0 errors.
  Build folder deleted. Status done.
- 2026-11-05: fix check gap started (`docs/reviews/p6-brand-apply.md`, "Fix check", finding 2's gap): the four review
  sheets have no golden test, yet `docs/brand.md`'s asset table says they do. Build folder
  `<scratchpad>\p6-brand-sheet-golden\target`. Read: `check-art.mjs` reads the three `icon-sheet-*.png` (line 44),
  not `icon-app.png` (it reads the ICO for the app icon). `p8-cleanup.md` line 33 to 34 already lists all four under
  "Keep", not under removal.
- 2026-11-05: `art.rs`: `review_sheets()` (test code) lists the four sheets by file name, and the writer loops over it,
  so the writer and the test cannot drift apart (a design call: the test alone was the brief; one list is the house
  way, as `site_svgs()`). New golden test `review_sheets_match_the_art`, the same "run the writer" message. It passed
  on the files in the repo, so the shared list writes the same bytes as the old writer. Mutation check: the dark sheet
  drawn with the light taskbar's colours failed the test with `icon-sheet-dark.png differs from the art: run ...`;
  reverted. The four sheets take 2.8 s in a debug test run.
- 2026-11-05: `docs/brand.md` asset table: the "golden tests" claim is true now (ICO, favicon, SVGs, sheets in
  `art.rs`; the notice PNG in `notify.rs`); added that the three glyph sheets are what `check-art.mjs` holds the media's
  tray glyphs to. `p8-cleanup.md` not changed: the sheets are on its Keep list already. Whole-workspace checks running
  (script `<scratchpad>\p6-brand-sheet-golden\checks.ps1`, a log per check).
- 2026-11-05: every check passed, 0 warnings: fmt; clippy x64 and ARM64 (Llvm folder on PATH), with and without
  `--all-targets`; `cargo test --workspace --locked` (wakeshot-windows lib 612 passed, 23 ignored; every crate 0
  failed); release builds x64 and ARM64; `node media/scripts/check-art.mjs` 9,458,740 pixels, 0 differ. Build folder
  deleted. Status done.

## Result

Done 2026-11-05. The held light in Moss is drawn in `art.rs`, the one source for the ports.

What changed:

- **Tray glyph** (`tray_mask`, every size 16 to 64 and any other): holding a filled light over a line, idle the same
  light as an outline at the same place, off the outline struck inside its box. One formula for all sizes (light half
  the box, line 3/4, gap 1/8, whole-pixel strokes), the light placed so the filled glyph's ink centre is at 0.455;
  hand-drawn 1 px rings at 16 and 20 px; a pixel slash at 1 px with all eight neighbours cleared. `Mask` has a second
  channel, `light`.
- **Colours**: `GlyphColours { ink, light }`; `icon.rs` `glyph_colours(theme)` gives white + `#B6E05A` on dark,
  `#1A1A1A` + `#3B7A2A` on light, the window text colour for every part in high contrast. `Drawn` and
  `Note::Redrawn` carry it. `theme.rs` needed no change.
- **App icon** (`app_icon`, `app_icon_flat`): Moss plate at 120 degrees, glow, bottom shade, emissive light with bloom
  and warm rim, white bar, soft shadow; frames under 30 px flat. ICO: Microsoft's 14 target sizes, 256 as PNG.
- **Writers**: real deflate (LZ77 + dynamic Huffman); `ico()` writes PNG for 256; `favicon_file()`; the writer test
  writes `assets/wakeshot.ico`, `assets/wakeshot-notice.png` (flat drawing, 48 px), `site/favicon.ico` and the sheets.
- **Removed**: the Sunrise code, `APP_LIGHT`, `APP_DARK`, `SUN_GOLD`, `SUNRISE_GREY`, `risen_sun`, `empty_sun`,
  `notify`'s own writer.

Files: `crates/wakeshot-windows/src/tray/art.rs`, `tray/icon.rs`, `notify.rs`; `assets/wakeshot.ico` (185,835 bytes,
was 43,006), `assets/wakeshot-notice.png` (1,798), `site/favicon.ico` (15,086); `docs/research/img/ux/icon-sheet-*.png`,
`icon-app.png`; new `docs/research/img/brand/apply-tray.png`, `apply-tray-detail.png`, `apply-icon.png`,
`apply-compare.png`, `apply-notice.png`; `docs/research.md` section "Brand apply: the held light in art.rs".
`tray/theme.rs`, `tray/menu_paint.rs` and the menu renders unchanged (the menus show no art).

Checks (all on the final code):

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | pass |
| clippy x64, with and without `--all-targets`, `-D warnings` | pass, pass |
| clippy ARM64 (Llvm folder on PATH), with and without `--all-targets` | pass, pass |
| `cargo test --workspace --locked` | pass (wakeshot-windows lib 608 passed, 23 ignored; all crates 0 failed) |
| release build x64, ARM64 | pass, pass |
| writer run twice | all 7 files byte-equal |
| Golden tests (ICO, favicon, notice PNG equal the art) | pass |
| Independent reads | Python zlib inflated every PNG; Windows `LoadImageW` loaded all 14 ICO frames and 3 favicon frames; `PrivateExtractIconsW` gave 16 to 256 px from both exes; all 14 frames byte for byte in all four exes |
| Exe sizes x64 | tray 2,745,856 to 2,907,648; CLI 558,080 to 701,440. ARM64 after: 2,565,120, 675,840 |

Numbers against their minimums (more in `docs/research.md`):

| Check | Value | Minimum |
|---|---|---|
| Filled glyph ink centre from 0.455, 16 to 64 px | -0.18 to +0.46 px | within 0.5 px |
| Idle, off ink centre from 0.455 (light kept in place) | +1.0 to +4.0, +0.7 to +2.9 px | printed only |
| States differ at 16, 20, 24 px | 33% to 62% | 25% |
| Half-covered pixels, 16 to 32 px | 23% at most | 25% |
| Glyph parts on taskbars and hovers (ink, holding light) | 3.74 (`#3B7A2A` on d8dadc) to 16.28 | 3.0 |
| Plate ends on light / dark Start | 3.57 to 4.56 / 3.22 to 4.12 | 3.0 |
| Light's rim on the plate, every frame | 3.13 to 3.51 (2.03 to 2.58 inside its glow) | 3.0 (glow not counted) |
| Bar on the plate, every frame | 3.46 to 4.15 | 3.0 |
| Half rule at 48 px, light / dark Start | 63% / 91% | 50% |
| 256 PNG frame | 32,229 bytes (12% of the pixels) | under 36,000 (test) |

Looked at every frame at 100% and enlarged (scratch `out\`), beside round 2's approved pictures; changed after
looking: the 1 px slash gap (eight neighbours), the lit rim back to round 2's `#F7DC7F`, the light-to-bar gap at 24 px,
a smaller shadow that no longer ends in a hard line at the icon's edge, the flat drawing for the notice header. Mean
difference to the approved frames: 2 to 4 levels of 255 (24 px: 7.5, now flat as the card asks).

Decisions for Haik's review (my calls as designer; each with its trigger):

1. **All 14 ICO frames** (Microsoft's target list), not the 5-frame minimum: sharp at every scale, about 143 KB in
   each exe. Change it when exe size matters more: PNG frames from 64 px up after a Start and Explorer check.
2. **Notice header PNG is the flat drawing at 48 px**: the lit drawing blurred at 16 to 24 px. Change it when a
   screen session shows the header soft or wrong.
3. **The lit light keeps round 2's warm rim `#F7DC7F`**, not the card's `#FFEA9C` edge; its 3:1 is counted against
   the plate without the glow (inside the glow 2.0 to 2.6, as round 2 drew it).
4. **Shadow smaller than round 2's** (a 96th down, a 64th blur): round 2's was cut at the icon's edge. At 30 and 32 px
   (1 px margin) it is a faint hairline round the plate.
5. **30 px is a lit frame**, 24 px and below flat (round 2's 24 had a glow).
6. **Idle sits lower by ink** (1 to 4 px from 16 to 64): the light keeps one place in all states; only the filled
   glyph is held to 0.455.
7. **Ramp at 120 degrees as CSS reads it**, as the old `art.rs` did; round 2's SVG used about 139 degrees. Barely
   visible.

Open items: the media and web ports (Part B of their cards; see Found outside scope); a screen session to see the
tray, Start, the installer and the notice header live; Haik's 2 to 3 person check at 16 px.

### Review fixes (2026-11-05)

Review `docs/reviews/p6-brand-apply.md`; the orchestrator's brief: findings 1, 3, 7, 2 (art and media sides), 5, the
`brand.md` rows, and the 20 px off glyph.

- **Finding 1 (major), fixed.** `app_icon_contrast` measures the pixels of `app_icon(size)`: the bar's and the flat
  light's edges as pairs of a fully covered pixel and the nearest plain plate pixel, the lit light's core against the
  ring just past the bloom; plain and under protan, deutan and tritan (Machado 2009). The bar was 2.78 to 2.95 as drawn
  from 64 px up: the bloom's tail now ends at 1.6 light radii instead of 1.9 (stops (0, 0.8), (1.0, 0.44), (1.28,
  0.144), (1.6, 0)), the least change that keeps 3:1 with the simulations (1.65 gave 2.98). Half rule re-run: 69% /
  91%, colour-blind 68% / 87%.
- **Finding 3, as decided** (orchestrator, under Haik's "However you want"): the rim stays `#F7DC7F`; the rule is the
  light's core against the plate past the bloom, 3:1 plain and colour-blind, in the test, `docs/brand.md` and
  `docs/research.md`, with the drawn edge (1.40 to 2.03) as information.
- **Finding 7, fixed**: the rim comment says round 2's `#F7DC7F`, no derivation.
- **Finding 2, art side, fixed**: `art.rs` writes `site/wakeshot.svg` and both lockups from `Plated::at(256)` and the
  drawing's own constants (now shared helpers); the word is fixed path data with an assert on the plate it was set
  for; golden test `site_svgs_match_the_art`. The new SVG against the new ICO in headless Edge: mean 0.62 over white,
  as the old pair (0.64). `site/social-card.png` re-rendered, byte-stable.
- **Finding 2, media side, done on our side**: CI job `art-port` (ubuntu, setup-node v7, Node 26) runs
  `node media/scripts/check-art.mjs`, no npm install needed; actionlint 0 findings. It fails until the media worker
  ports the bloom and the strike box (exact changes in the Log, 2026-11-05 "finding 2, the media side").
- **Finding 5, fixed**: `docs/tasks/p8-cleanup.md` lists the 12:07 work log line and both Task Manager captures.
- **The 20 px off glyph, fixed**: the slash ended on the line's last pixel at 16, 20 and 24 px (most visible at 20)
  and ran left of the light's centre. It now runs from the light's top left corner through its centre and stops a
  stroke above the line; new test `the_slash_leaves_the_line_whole`.
- **`docs/brand.md`**: the contrast rows (drawn numbers, colour-blind column from the tests), the light's rule, the off
  glyph, the bloom, ICO and social card sizes, and the asset table (SVGs written by `art.rs`, CI check for media).
- **Fix check gap, fixed**: golden test `review_sheets_match_the_art` holds the three `icon-sheet-*.png` and
  `icon-app.png` to the art (one list, `review_sheets()`, for it and the writer), so a tray glyph change without a
  writer run fails `cargo test` and cannot leave `check-art` green on old sheets; `brand.md`'s "golden tests" line is
  true now and names the sheets' link to `check-art.mjs`; `p8-cleanup.md` already keeps the sheets. All checks pass
  (Log, last line).

Files: `crates/wakeshot-windows/src/tray/art.rs`; `assets/wakeshot.ico` (184,720 bytes), `site/favicon.ico`,
`site/wakeshot.svg`, `site/wakeshot-lockup-light.svg`, `-dark.svg`, `site/social-card.png` (285,193);
`docs/research/img/ux/icon-sheet-*.png`, `icon-app.png`; `docs/research/img/brand/apply-tray.png`,
`apply-tray-detail.png`, `apply-icon.png`, `apply-compare.png`, `apply-notice.png`, new `apply-fixes.png` (before and
after); `.github/workflows/ci.yml`; `docs/brand.md`; `docs/research.md` (rows corrected, new section "Brand apply
review fixes"); `docs/tasks/p8-cleanup.md`. `notify.rs`, `icon.rs` and `assets/wakeshot-notice.png` unchanged.

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | pass |
| clippy x64, with and without `--all-targets`, `-D warnings` | pass, pass |
| clippy ARM64 (Llvm folder on PATH), with and without `--all-targets` | pass, pass |
| `cargo test --workspace --locked` | pass (wakeshot-windows lib 611 passed, 23 ignored; every crate 0 failed) |
| release build x64, ARM64 | pass, pass; tray 2,906,624 / 2,564,096, CLI 700,416 / 674,816 bytes; 14 of 14 ICO frames in all four exes |
| `python site/check_page.py site` | 0 errors, 8 waits |
| writer run twice | all 10 written files byte-equal |
| refactor neutral | every raster file byte-equal before and after sharing the constants (7 of 7) |
| actionlint 1.7.12 on `ci.yml` | 0 findings (no shellcheck) |
| `social_card.py` twice | same SHA-256 |
| `node media/scripts/check-art.mjs` | fails as expected until the media port (lit frames and off glyphs) |

Decisions for Haik's review:

1. **Bloom ends at 1.6 light radii** (was 1.9) so the bar keeps 3:1 as drawn. The lit frames now differ from round 2's
   approved ones by 3.5 to 5 levels of 255 on average (2 to 3 before); the halo is a little tighter. Change it when a
   screen look finds the light glowing less than approved: then the other way is a bar rule that counts the bloom.
2. **The off slash strikes the light only** and stops above the line, through the light's centre. Round 2's approved
   slash crossed the whole glyph. Idle and off now differ in 28% of their ink at 16 px (43% before; the test asks
   25%). Change it when Haik's 2 to 3 person check at 16 px mixes up idle and off.
3. **The colour-blind rule is in the tests** for every tray and app icon pair and the half rule, stricter than
   Microsoft's plain half rule. Change it when a colour-blind check needs its own tolerance.
4. **The SVG writer is test code** (like the ICO writer's sheets) and the word's outlines are fixed data, made again
   with HarfBuzz and fontTools only when the font, the lockup ratios or the 256 plate change (an assert catches the
   plate). Change it when the lockup needs kerning work: then move the web card's scripts into the repo.
5. **CI `art-port` runs on ubuntu-latest with Node 26**, apart from the Rust jobs, about 2 s of work. Change it when the
   media check needs npm packages (then `npm ci` in `media/`).

Pictures to look at: `docs/research/img/brand/apply-fixes.png` (before and after: 256, 64, 48 px icons on both
Starts; off at 16, 20, 24 px), then `apply-tray.png` and `apply-compare.png`.

Open items: the media port of the two changes (next worker), then `check-art` green in CI; the `p6-pages` line above.
