# Brand review of the current app, and three drawn brand directions for Haik to pick from

Phase: 6 (brand review). Status: done (Parts B and C, 2026-11-05; Part A by the second worker). Needs Haik: yes, after this card: he picks a direction. Agent:
`wakeshot-worker`. Size: about 220K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-brand-research-windows`, `p6-brand-research-macos`, `p6-brand-research-identity`.
Parallel with: any card (this card edits no code and no shipped asset).
Real laptop: no power settings, no installs. Renders run offscreen; reading the current tray and menu pictures is
enough. Do not start the tray.
Scratch folder: `<scratchpad>\p6-brand-directions\`.

## Goal

Haik (2026-11-05): "review our small app in terms of UI and the branding", "I am not sure if our logo is good or not,
not sure about colors and white black tray icon. Also tray icon and app icon difference. Or does the brand and icon
should be the same or not", "if we later introduce macOS, we need to design our panel and icons for it as well right?
Like should it differ and be glassy and Tahoe compliant", "as we are not live yet we can change everything and
anything", "follow 2026 visual trends", "this work should be excellent". He is a backend architect, not a designer:
show, do not tell. Every option must be a real picture at real sizes in real places, so he can judge it with his eyes.

Haik's answers (2026-11-05, "All default answers are good"): the name stays Wakeshot; the Mac version is the Mac menu
bar app (not iOS), the brand must work on the Mac now, Mac pictures are mockups only and nothing ships for the Mac yet;
the Keep awake glyph becomes a clock from Segoe Fluent Icons (`p6-brand-cleanup`), unless a direction makes a better
case, shown in its menu picture; no budget: free fonts (SIL Open Font License or similar) and our own drawing code.

## Read first

- `docs/research/brand-windows.md`, `docs/research/brand-macos.md`, `docs/research/brand-identity.md` (this round's
  research; their Result sections first).
- `docs/brand.md`, `docs/product.md` first screen, `docs/research/tray-menu-look.md` "Pick".
- The current app, as pictures: `docs/research/img/ux/` (`icon-app.png`, `icon-sheet-*.png`, `taskbar-*.png`,
  `menu-screen-*.png`, `menu-render-*.png`, `notice-*.png`, `taskbar-tooltip-dark.png`, `int-*.png`); the page
  `site/index.html` with `site/style.css` and `site/wakeshot.svg`; `README.md`; `media/out/icon-loop-*.gif`.
- The art code, to know what can be drawn and at which sizes: `crates/wakeshot-windows/src/tray/art.rs` (top 190
  lines), `menu_paint.rs` lines 55 to 70.

## Deliverables

All in `docs/research/brand-directions.md` (write it as you go) and pictures in `docs/research/img/brand/` with names
starting `dir-` or `review-`.

### Part A: review of the current UI and brand

Split off to a second worker (orchestrator, 2026-11-05, for size): Part A goes to `docs/research/brand-review.md` with
pictures named `review-*`; the Part B and C worker reads it for Part C when it is done, and does not wait for it.

Every surface a user sees: the tray glyph in its three states (dark, light and high contrast taskbar; 100%, 150%,
200%), the tooltip, the menu and submenu, the notices, the app icon where Windows shows it (Start, taskbar, Settings,
Explorer, the notice header), the installer, the web page (light and dark), the README, the icon loop. For each: what
works, what does not, how bad (blocker for launch, should fix, nice to have), with the picture it is seen in. Include
anything that looks dated, foreign to Windows 11, generic, or off brand. Plain and critical; Haik wants the truth.

### Part B: three brand directions

Three directions, real and different. One may be the current Sunrise refined, if the research supports it; at least
two must be new marks (from `brand-identity.md` question 6 or better). For each direction draw, as PNG renders at true
pixel sizes plus enlarged views:

1. The mark and the palette (hex, with the WCAG numbers that `brand-identity.md` question 8 asks for).
2. The Windows app icon at 16, 24, 32, 48 and 256 px, on light and dark Start-like backgrounds, and next to 4 or 5
   real Windows 11 app icons for scale and style (draw stand-ins, do not copy others' art into the repo).
3. The tray glyph in its three states (holding, idle, off) at 100% and 150%, on a dark and a light taskbar crop with
   Windows' own glyphs beside it (crops from `docs/research/img/`), following `brand-windows.md`'s pick on monochrome
   or colour. If the research says colour while holding, show both variants.
4. The menu with the new Keep awake glyph (the cup is going away, `docs/tasks/p6-brand-cleanup.md`).
5. The macOS twin: the app icon as a Tahoe-style layered icon (default, dark, clear and tinted looks, as
   `brand-macos.md` describes them), and the menu bar glyph on a light and a dark menu bar. Mockups, not shipped files.
6. The wordmark lockup, the favicon at 16 and 32 px, and a GitHub social preview card at the size GitHub asks for.

Then one comparison board: the three directions side by side at the sizes that decide it (16 px tray, 32 px and 48 px
app icon, the Mac icon), light and dark.

Draw with code you can rerun: SVG sources in your scratch folder, rendered to PNG (headless Edge, or the Remotion
setup in `media/` if it helps; do not change `media/`). Look at every render yourself before you show it; fix blurry
16 px glyphs (snap strokes to whole pixels as `art.rs` does). Also write one self-contained HTML board
(`<scratch>\board\index.html`, pictures inline or beside it) that shows Part A's key findings and all of Part B, so
the orchestrator can publish it for Haik.

### Part C: recommendation

- Your pick of direction, with the reasons, and the runner-up.
- Plain answers to Haik's questions, one short paragraph each: is the current logo good; the colours; white or black
  tray icon versus colour; tray icon versus app icon (same or different, and why); brand mark versus icons; the Mac
  icon and panel (different, glassy, Tahoe); which 2026 trends to follow and which to skip.
- The decisions only Haik can make, as a numbered list with your default for each.
- What applying the pick would change (files: `tray/art.rs`, `menu_paint.rs`, `assets/`, `notify.rs` notice icon,
  `site/`, `media/src/theme.ts` and `media/src/art/`, README pictures, `docs/brand.md`), and a size estimate for the
  apply card.

## Acceptance checks

- Part A covers every surface listed, each finding with a picture and a severity.
- Part B has all six items for each of the three directions, as PNGs at true sizes; the comparison board exists.
- Every 16 px tray render was checked at 100% zoom by the worker (say so in the Log, with what was fixed).
- Every palette has its contrast numbers from a script (keep the script in the scratch folder, name it in the Log).
- The HTML board opens offline and shows all pictures.
- Part C answers each of Haik's questions and lists his decisions with defaults.

## Out of scope

Code, shipped assets (`assets/`, `site/`, `media/`), `docs/brand.md`: the apply card changes those after Haik picks.
`docs/plan.md` and phase files (another orchestrator session owns them). No art copied from other products into the
repo; stand-ins only.

## Questions for Haik

Collect them in Part C. Write here only a question that blocks this card, with your default, and go on.

## Found outside scope

- `docs/brand.md` says "no keep-awake rival uses violet"; true on Windows only (the Mac research found four violet
  plates). The apply card rewrites the file anyway.
- Part A finding T4 (the tray ink sits about 2 px below Windows' own glyphs) also applies to Daybreak and Pilot as
  drawn here (horizon on row 13, as today). For the apply card.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: research done (`brand-windows.md`, `brand-macos.md`, `brand-identity.md`); cup removed
  (`p6-brand-cleanup`, the menu now shows the clock `E823`). `brand-macos.md` question for Haik 1 (Shut the Lid, a Mac
  app of 2026-11-04, already uses a gold half sun over a white line on a blue-indigo plate): its default applies here
  until Haik answers: a sunrise direction must get a feature of its own, and every direction is shown side by side with
  stand-ins of Shut the Lid's icon, SF Symbols `sun.horizon.fill` and Material `wb_twilight` (drawn from their
  descriptions, not copied). Part A split off to a second worker (`docs/research/brand-review.md`).
- 2026-11-05: Parts B and C worker started. Read the card, the three research reports (Results and the parts on
  marks, colour, tray, menu, Mac rules), `docs/brand.md`, `art.rs` (grid and snapping), `menu_paint.rs` glyph table.
- 2026-11-05: tooling in `<scratch>\p6-brand-directions\tools\`: `raster.py` (a Python port of `art.rs`'s method:
  coverage of 8 x 8 sub-samples, the 16-unit grid that snaps edges to whole pixels, 1 px diagonals placed pixel by
  pixel), `marks.py`, `sketch.py`, `colour.py` (copied from the identity card's scratch), `shot.ps1` (headless Edge,
  own profile in the scratch folder). Sketch round at 16, 20, 24, 32 px on both taskbars (`out\sketch1.png`,
  `out\sketch2.png`): the current Sunrise, sun from a prompt, held light (a disc above a line), lit window (square and
  arched), two-pane window, pilot ring, square brackets, curly braces. Dropped after looking at 100%: both windows
  (read as a monitor, a stop button or a tombstone; a square outline, which the tray guide warns against), the pilot
  ring (reads as record or a radio button), square brackets (a frame: reads as a screenshot or record tool, the risk
  the identity report names). Kept: sun from a prompt, held light, braces.
- 2026-11-05: three directions set: Daybreak (sun from a prompt, dawn rose), Pilot (light held above a line, olive
  and lime), Braces (light in curly braces, near-black and a warm light). Palettes in `tools\palette.py`, checked by
  `tools\palette_check.py` (C1 to C14 of `brand-identity.md` question 8, on `colour.py`); `tools\assets.py` also
  prints the 16 px shape test (C6) and the share of the 48 px icon that keeps 3:1 on each Start (Microsoft's "half").
  Design calls: one plate shape for all three (art.rs's margin and corner), so the comparison is fair; small icons
  drawn per size (16 px by hand, 20 and 24 px from the tray drawing); the colour while holding is a light tone on the
  dark taskbar and a deep tone on the light one, not an outline. Change it when a direction moves to a free shape.
- 2026-11-05: looked at every 16 px tray render at 100% and zoomed, and the app icons at 16 to 256 px. Fixed: Braces
  idle (a ring) differed from holding in only 21% of the ink at 16 px, under the art.rs test's 25%: now empty braces
  (55%). Pilot's app icon and free mark read as a profile picture: four variants tried (`out\pilot_try*.png`); kept a
  horizon across the whole plate with darker ground (a scene), and a thin horizon twice the light's width for the free
  mark. Pilot's pale lime read as a moon: saturated, then lightened to `#E8F86C` after the colour-blind check failed
  on the plate's light end (2.96). Daybreak's 16 px app icon lost the prompt and had a sun pixel against the chevron:
  drawn by hand. Shut the Lid's stand-in glyph was scaled and faint: drawn per size. Mac pictures first rendered empty
  (masks from file URLs need `--allow-file-access-from-files`): fixed in `shot.ps1`.
- 2026-11-05: pages (`tools\pages.py`, shot by `tools\render.ps1`) copied to `docs/research/img/brand/dir-*.png`
  (23 files, 2.4 MB). Part A was done by then; read its verdict and top five for Part C and the board. Report written:
  `docs/research/brand-directions.md`. Board: `<scratch>\p6-brand-directions\board\index.html` with `board\img\`
  (32 pictures), shot offline with headless Edge to check every picture loads (`out\board-check.png`).

## Result

Status: Parts B and C done 2026-11-05 (Part A done by the second worker, `docs/research/brand-review.md`).

What was done:

- Ten marks sketched at 16 to 32 px; three kept and drawn in full: **Daybreak** (the sun rises from a prompt, dawn
  rose), **Pilot** (one light held above a horizon, olive and lime), **Braces** (a light in code braces, near-black).
  Each has the six items: mark and palette with contrast numbers, Windows app icon at 16 to 256 px among Windows 11
  style stand-ins on light and dark Start and taskbar, tray glyph in three states at 100% and 150% on both taskbars
  (one real taskbar crop from this laptop), one colour and colour while holding, the menu with the clock, the Mac
  twin (four Liquid Glass looks and the menu bar, mockups), wordmark lockup and favicon, and a 1280 x 640 social card.
- Comparison board `dir-compare.png`: the three beside today's icon and stand-ins of Shut the Lid,
  `sun.horizon.fill` and `wb_twilight`, at 16 and 24 px tray, 32 and 48 px icon on both Starts, Mac default and dark.
- Part C in the report: **pick Pilot, runner-up Daybreak**; answers to each of Haik's questions; five decisions with
  defaults; what applying the pick changes, about 150K to 200K tokens plus a media rerun.

Files: `docs/research/brand-directions.md` (new); `docs/research/img/brand/dir-*.png` (23 new); this card. Scratch:
`tools\` (scripts), `board\index.html` and `board\img\`.

Acceptance checks:

- Part A: done by the second worker (not checked here beyond reading its verdict and top five).
- Part B has all six items for each direction as PNGs at true sizes, plus the comparison board: pass (23 pictures).
- Every 16 px tray render checked at 100% by me, with fixes: pass (Log, 2026-11-05 entries).
- Every palette has contrast numbers from a script: pass (`tools\palette_check.py`; one known fail stated: Braces'
  plate on dark Start, 1.0 to 1.3).
- The HTML board opens offline and shows all pictures: pass (headless Edge, no network, `out\board-check.png`).
- Part C answers each question and lists decisions with defaults: pass.

Open items: no live screen check (no screen this session): the new icons were never seen in a real Start menu,
notice or taskbar; the Mac looks are drawn, not made in Icon Composer; how people read the 16 px glyphs is untested
(decision 5). The apply card should also lift the tray ink about 1 px (Part A, T4). Facts and numbers are in the
report, not in `docs/research.md` (this card does not own that file).
