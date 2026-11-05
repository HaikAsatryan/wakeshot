# Brand review of the current app, and three drawn brand directions for Haik to pick from

Phase: 6 (brand review). Status: ready. Needs Haik: yes, after this card: he picks a direction. Agent:
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

## Log

- 2026-11-05: card written by the orchestrator.

## Result
