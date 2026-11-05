# Apply the new brand to the page, the README header, the social card and the installer splash

Phase: 6 (brand apply). Status: done (Part A and Part B). Needs Haik: no (his 8 brand decisions, 2026-11-05 ~15:00, all defaults).
Agent: `wakeshot-worker`. Size: about 150K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: Part A: Haik's brand decisions. Part B: `p6-brand-apply-art` done (the orchestrator resumes you, or a new
worker reads this card's Log).
Parallel with: `p6-brand-apply-art` (Part A only), `p6-brand-apply-media`. No cargo builds in this card.
Real laptop: no power settings, no screen, no installs. Offscreen renders only (headless Edge).
Scratch folder: `<scratchpad>\p6-brand-apply-web\`.
Owns: `site/` except `site/favicon.ico` (written by `art.rs`), the top of `README.md` (the heading block above the
first paragraph, Part B), `packaging/pack.ps1` and `packaging/README.md` (installer splash only), a new
`assets/wakeshot-splash.png` if the splash is needed (Part B). Additions to `docs/research.md`.

## Goal

Haik picked the held light in Moss; the wordmark outside the Windows app is Instrument Sans SemiBold. The page is
still violet and gold with a stray gold bar, the README starts with a bare heading, the repo has no social card, and
nobody knows what the installer shows. Bring all four onto the new brand, from the same art as the app. Quality bar: a
senior brand designer would ship it. Render, look at every result, fix, look again.

## Read first

- `docs/research/brand-directions-2.md`: "Step 2: the colour" (the Moss column) and "What applying the pick changes".
- `docs/phases/06-ship.md` work log, line "2026-11-05 ~15:00": Haik's decisions.
- `docs/research/brand-review.md`: W1 to W5, R1, R2, I1, X1, X2.
- `docs/research/brand-identity.md`: the wordmark part (Instrument Sans, why).
- `site/index.html`, `site/style.css`, `site/check_page.py`, `site/wakeshot.svg`, `.github/workflows/pages.yml`.
- `README.md` lines 1 to 10 only.
- `packaging/pack.ps1`, `packaging/README.md`.
- Round 2 sources in `<scratchpad>\brand-src\p6-brand-directions-2\tools\` (`pal2.py`, `checks2.py`, `icons2.py`,
  `render2.ps1`) and `out\checks.md`; round 1's `fonts\instrumentsans.ttf` in `<scratchpad>\brand-src\p6-brand-directions\`
  (only to compare: take the shipped font from its official source). Copy what you edit into your own folder.

## Deliverables

### Part A (now; nothing here waits for the art)

1. **Page colours**, `site/style.css`, light and dark: the Moss set from round 2 (plate `#628C2B` to `#3E7C3B`, light
   `#FFEA9C`, link `#33701F` light and `#B6E05A` dark, page `#FBFCF7` light and `#121610` dark; text, muted, panel,
   line, button and focus colours chosen in the same family). Every text pair passes WCAG AA (4.5, large text 3.0),
   focus and non-text UI 3.0, in both themes. Nothing violet or gold stays. Update `check_page.py` to the new colours.
2. **The violet look goes** (W2): the violet glow and button become Moss. Drop the stray gold bar (W3). No layout or
   copy change beyond that: product pictures come with `p6-media` Part B.
3. **Wordmark font**: Instrument Sans SemiBold, self-hosted as WOFF2 under `site/fonts/`, from its official source
   (the designer's repo or Google Fonts' own GitHub repo; cite the URL and version), with its licence file (`OFL.txt`)
   beside it. Check the licence for a Reserved Font Name before you subset or rename. Only the wordmark uses it; body
   text stays as it is. No other font host, no request leaves the page (the page has none today; keep it so).
4. **Installer splash (I1)**: find out what Velopack's `Setup.exe` shows during install when `pack.ps1` passes no
   `--splashImage`, from Velopack's own source of the version we pack with (cite file and line). Record it in
   `docs/research.md`. If it shows nothing, Part B adds a splash.

### Part B (after `p6-brand-apply-art`)

5. **`site/wakeshot.svg`**: the app icon as a vector, ported from `art.rs`'s numbers (plate, ramp, glow, light, bar,
   caps). Render it at 256 in headless Edge and compare with the 256 frame of `assets/wakeshot.ico`: record the mean
   and max channel difference and fix what a person would see. One stroke style with the art (X2).
6. **Wordmark lockup**: the app icon beside "Wakeshot" in Instrument Sans SemiBold, the letters as outlines (SVG paths,
   so GitHub and any viewer draw it without the font), a light and a dark version: `site/wakeshot-lockup-light.svg`,
   `site/wakeshot-lockup-dark.svg`. The page's hero uses the same lockup or the same font and spacing (X1: one
   wordmark).
7. **README header (R1)**: replace the `# Wakeshot` heading with the lockup in one centred `<picture>`
   (`prefers-color-scheme`, alt text "Wakeshot"), keeping a heading for screen readers (for example the picture inside
   an `<h1>`). Nothing else in the README changes: the media comment on line 3 and the rest belong to
   `p6-brand-docs-sweep` and `p6-readme`.
8. **Social card (R2)**: the repo's social preview, `site/social-card.png`, at the size GitHub's docs give today
   (check: about 1280 x 640, under 1 MB), with a safe margin for cropping. The lockup plus the README's existing first
   line as the only text (no new copy). Keep its source (SVG or HTML) and the script that renders it under `site/`,
   so it can be made again. Add `og:image` and `twitter:image` (with the page's absolute Pages URL, from `pages.yml`
   or the repo's Pages settings) and alt text to `index.html`.
9. **Splash, if Part A found it needed**: `assets/wakeshot-splash.png` drawn from the app icon (the 256 frame or the
   lockup; sized and shaped by Velopack's rules for the splash window), passed by `pack.ps1` as `--splashImage`;
   `packaging/README.md` says so. If Velopack shows its own progress window, no splash: record the reason.
10. **Look**: render the page light and dark at desktop width and at 390 px, the README header as GitHub shows it
    (light `#FFFFFF`, dark `#0D1117`: check GitHub's current colours), the social card at full size and at 400 px wide,
    the splash if any. Fix, look again. Keep 3 to 5 pictures as `docs/research/img/brand/apply-web-*.png`.

## Acceptance checks

- `python site/check_page.py` passes (and `--final` fails only for the spots other cards fill, as today).
- WCAG numbers for every colour pair, both themes, in a table in the Result.
- No `#7250F4`, `#7957F4`, `#6B49F3`, `#5B3FD6`, `#A996FF`, `#FFCC4D`, `#14121F` or other violet or gold left in
  `site/`.
- The page makes no network request outside its own folder (headless Edge network log, or a grep for `http` in CSS
  and fonts).
- `wakeshot.svg` against the ICO's 256 frame: numbers recorded, no visible difference at 100%.
- If `pack.ps1` changed: `pwsh -NoProfile -File packaging/pack.ps1` runs on a local release build into a scratch
  folder (no install), and actionlint is not needed (no workflow change).

## Out of scope

`site/favicon.ico` (the art writes it); `media/`; any README change below the heading; any new or changed copy beyond
the alt texts; the Rust code; `docs/brand.md` and other docs (`p6-brand-docs-sweep`); the GitHub repo settings (the
orchestrator sets them); anything Mac. Do not run git.

## Questions for Haik

None expected. Design calls inside the brief: decide and list them under "Decisions for Haik's review".

## Decisions for Haik's review

Part A (each with its reason and when to change it):

1. **Hero glow: the warm light round the icon, over a faint moss haze**, not a moss glow alone. A moss-only glow read
   as a grey-green smudge on the light page and as olive on the dark one; a light glowing round the mark is the brand
   idea (a light left on), so the glow is ours and not the generic top-of-page glow W2 names. Light theme: cream at
   45% in a 15rem circle on the icon, moss at 8% along the top; dark: cream 10%, moss 26%. Change it when the hero
   layout moves the icon (Part B's lockup): re-centre the circle on the icon.
2. **Step numbers: the cream light on a moss disc** (`#FFEA9C` on `#33701F`, 5.03), both themes, replacing the gold
   discs. It is a small copy of the icon (light on plate). Change it when it reads as decoration that fights the
   pictures `p6-media` adds next to the steps.
3. **One interactive colour per theme**: buttons, links and the light focus ring are the link colour (`#33701F`
   light, `#B6E05A` dark), as the old page did with violet. Not the plate's `#3E7C3B` for the button: two near greens
   side by side look like a mistake. Change it when a second button style is needed.
4. **Dark focus ring: the cream light `#FFEA9C`** (15.3 on the page), so the ring stands apart from the lime button it
   circles. Change it when a person finds it hard to see on the cream step numbers (it never sits on them today).
5. **Font shipped unchanged, 36 KB, `font-display: swap` plus a preload.** Measured: one font request, reused by the
   CSS. If the font fails, the heading falls back to the old system stack. Change it when the page needs a smaller
   first load or more weights: subset to Latin with fontTools.
6. **Wordmark tracking stays -0.01em** (0, -0.01em and -0.02em compared at 44 px: 0 is loose, -0.02em crowds "ho" and
   "ot"). Part B's lockup uses the same.
7. **No installer splash** (decides Part B deliverable 9): Velopack already shows its own window with words ("Installing
   Wakeshot", "Setting up Wakeshot 0.1.0, please wait..."), a title bar and a progress bar. A splash would replace it
   with a borderless picture that has no words, a flat 12 px bar and no taskbar button, for the few seconds an
   install takes. Change it when the screen session (`brand-review.md`, "Next screen session", item 8) shows that
   window blank, broken or confusing.

Part B:

8. **The page hero is the lockup row** (icon beside the name), not the icon stacked over the name: the same lockup as
   the README and the social card (X1), and it drops the "centred icon over a big name" template W2 called generic.
   The glow moved to the icon. Change it when `p6-media`'s pictures need a taller hero or a larger icon: the stacked
   form is one CSS rule.
9. **Lockup proportions**: the word's cap height 0.46 of the plate, centred on it; 0.26 of the plate between plate
   and word; tracking -0.01em; the word in the page's text colours (`#1A1D17`, `#EEF2E8`). 0.38 looked timid, 0.56
   heavy. Change it when the lockup sits next to other apps' lockups and looks small.
10. **README lockup 304 x 80 px**: the word comes out at 44.6 px, the page's 44. Change it when the loop GIF lands
    right under it and the two compete: then 64 px tall.
11. **Social card dark**, the line in Instrument Sans Regular (the video's face for words that are not the name), the
    warm glow at 0.16. Dark carries "a light left on" and stands out in a white feed. Change it when a dark feed shows
    it as a dark block: the light version is a few colour values in `social-card.html`.
12. **Full share tag set**: `og:type`, `og:url`, `og:title` and `og:description` (the page's own title and
    description, so no new copy; `check_page.py` fails if they drift), `og:image` with size and alt,
    `twitter:card` `summary_large_image`, `twitter:image` with alt. The card asked for the two image tags; without
    the others the Open Graph set is incomplete, and X shows a small card without `twitter:card`. Change it when a
    platform's card checker asks for more.
13. **The social card's source is HTML with live text**, not outlines: `p6-readme` will rework the README's first
    line, and then the card is one edit and `python site/social_card.py`. It adds
    `site/fonts/InstrumentSans-Regular.woff2` (35 KB, never loaded by the page). Change it when the page's folder must
    hold only what the page loads: move the source and script out with a `pages.yml` change.
14. **README picture by `<picture>`**, as the card asks. GitHub wraps it in its `themed-picture` element; a community
    report (not verified) says it follows the system's theme now, which would show the light-text lockup on white for
    a reader whose GitHub and Windows themes differ. Change it when Haik's check (Result, open items) shows that:
    then two `<img>` with the `#gh-light-mode-only` and `#gh-dark-mode-only` fragments, which GitHub's own CSS hides by
    GitHub's theme.

## Found outside scope

- `site/index.html` copy still describes the old sun: line 58 ("the sun is up while something holds, an empty sun
  when nothing does, and the empty sun struck through") and line 67 ("The icon shows the sun up"). With the held light
  it is a filled light, an outline, and the outline struck through. Copy is outside this card: `p6-brand-docs-sweep`
  or `p6-readme`.
- `README.md` line 3 (media comment) describes the Sunrise loop and names the Sunrise in `art.rs`
  (`p6-brand-docs-sweep`, `p6-readme`, `p6-media`).
- `.gitattributes` has no `*.woff2 binary` line. Git's own test (a NUL byte in the first 8,000 bytes) already treats
  the WOFF2 as binary (its header has NUL bytes), so nothing breaks; the line would make it explicit.
- W5 stays: on a phone the two hero buttons stack at different widths (a layout change, not in this card).
- `site/favicon.ico` and `site/wakeshot.svg` are still violet and gold until `p6-brand-apply-art` and Part B. (Part B:
  both are Moss now.)
- Part B: `.github/workflows/pages.yml` removes only `site/check_page.py` before upload, so `site/social_card.py` and
  `site/social-card.html` (marked noindex) go live too. Harmless; to keep the site to what it serves, make that step
  `rm site/check_page.py site/social_card.py` (and keep or drop the HTML source the same way). The workflow is not
  this card's.
- Part B: three headless Edge browsers started by Part A's `crop.ps1` at 14:53 still run (PIDs 33032, 2596, 12196,
  each with child processes), holding scratch `edge\c2ea62dca`, `ca2b8c95c`, `c3270d87b`. I did not start them, so
  I did not end them; the orchestrator may end them by PID tree.
- Part B, stale docs for `p6-brand-docs-sweep`: `docs/phases/06-ship.md` line 430 lists `site/` without the lockups,
  the social card, its source and script, and the second font; `docs/brand.md` should carry the lockup rules (cap
  0.46, gap 0.26, tracking -0.01em, colours), the stacked versus row hero, and the social card.
- Part B, for the orchestrator (GitHub settings, R2): upload `site/social-card.png` as the repo's social preview, and
  set the description and website link.
- Part B: `README.md` line 8 (was line 3) still has the Sunrise media comment (Part A's note above).

## Log

- 2026-11-05: started Part A (deliverables 1 to 4). Part B waits for `p6-brand-apply-art`.
- 2026-11-05: font source found. The designer's repo `Instrument/instrument-sans` ships a static SemiBold WOFF2
  (`fonts/webfonts/InstrumentSans-SemiBold.woff2`, 35,812 bytes, version 1.000, OFL 1.1, no Reserved Font Name, fsType
  0). Google Fonts' `ofl/instrumentsans/METADATA.pb` names the same upstream commit `7fa22308`. Downloaded at that
  commit into scratch `dl-font\`; the WOFF2's sfnt size equals the static TTF's (87,004), so it is that font. Shipped
  unmodified: no subset, no rename (subsetting needs fontTools, not installed; 36 KB is fine for one font).
- 2026-11-05: deliverable 4 done (installer splash). Velopack 1.2.161 source, tag commit `92d6a1c9`: with no
  `--splashImage`, `Setup.exe` shows its own progress window (xdialog, info icon, "Wakeshot Setup", "Installing
  Wakeshot", "Setting up Wakeshot <version>, please wait...", a progress bar), not nothing. File and line chain in
  `docs/research.md`, "Installer splash and the wordmark font". So Part B deliverable 9 adds no splash; the reason is
  recorded there and in the Result. `pack.ps1` and `packaging/README.md` stay unchanged.
- 2026-11-05: deliverables 1 to 3 drafted. `style.css`: Moss palette both themes, gold bar gone, step discs now the
  cream light on a moss disc, `@font-face` for `fonts/InstrumentSans-SemiBold.woff2` (with `fonts/OFL.txt`), the hero
  `h1` only in Instrument Sans; `index.html` preloads the font. Glow: tried three (moss only; warm core in a top
  ellipse; warm light centred on the icon over a faint moss haze) and two dark tunings; kept the centred one, it reads
  as the icon's own light, not a generic top glow. `check_page.py` now checks 16 colour pairs per theme from the CSS
  custom properties, the hero pairs under each mix of its two glows; negative tests (low muted, missing property,
  dark haze) fail with exit 1. Scratch tools: `pal_web.py`, `render_site.ps1` (retries: headless Edge can exit before
  it writes the picture), `sheet.ps1`.
- 2026-11-05: looked at the page light and dark at 1280 px and in a 390 px frame (headless Edge clips a 390 px window,
  so the phone render is an iframe). Hero, cards, steps, table and footer read well in both themes. Wordmark tracking
  compared at 0, -0.01em and -0.02em; kept -0.01em.
- 2026-11-05: **Part A done** (deliverables 1 to 4). Checks:
  - `python site/check_page.py --colours site`: 0 errors, 8 spots wait; every pair passes (table below).
    `--final`: fails only on those 8 spots, as before. No "not linked" notes (`OFL.txt` is exempt by name).
  - Violet and gold (scratch `colour_scan.py`, every colour in `site/` text files with its OKLCH): `style.css` has
    none. Only `wakeshot.svg` still has `#7957F4`, `#6B49F3`, `#FFCC4D` (Part B replaces it); `favicon.ico` is the
    art card's.
  - Network: scratch `netcheck.ps1` serves a copy of `site/` on `127.0.0.1:8765`, loads it in headless Edge with a
    network log, and `netlog_initiators.py` splits requests by initiator. Started by the page: `style.css`, the font
    (once), `wakeshot.svg`, `favicon.ico`, all on 127.0.0.1; 0 elsewhere. The other entries have no page initiator:
    Edge's own services (network time, experiments, Bing list). No `http` in `style.css`.
  - Splash: Velopack source read, see the deliverable 4 line above and `docs/research.md`.
  - Files changed: `site/style.css`, `site/index.html` (one preload line), `site/check_page.py`; new
    `site/fonts/InstrumentSans-SemiBold.woff2` and `site/fonts/OFL.txt`; `docs/research.md` (new section at the end).
    `packaging/` unchanged.

WCAG 2.2 contrast, both themes (from `check_page.py --colours`; "hero" is the worst of the page bare, under each glow
and under both at full strength):

| Pair | Needs | Light | Dark |
|---|---|---|---|
| text on page / panel / hero | 4.5 | 16.53 / 15.27 / 14.67 | 16.12 / 14.65 / 9.34 |
| muted on page / panel / hero | 4.5 | 6.82 / 6.30 / 6.05 | 9.30 / 8.45 / 5.39 |
| link on page / panel / hero | 4.5 | 5.86 / 5.41 / 5.20 | 12.02 / 10.93 / 6.96 |
| button text on button | 4.5 | 6.04 | 12.02 |
| step number on step disc | 4.5 | 5.03 | 5.03 |
| button edge on page / hero | 3.0 | 5.86 / 5.20 | 12.02 / 6.96 |
| focus ring on page / panel / hero | 3.0 | 5.86 / 5.41 / 5.20 | 15.25 / 13.86 / 8.83 |
| step disc on page (not required) | none | 5.86 | 3.03 |
| line on page / panel (decorative) | none | 1.32 / 1.22 | 1.49 / 1.36 |
| panel on page (cards have a line too) | none | 1.08 | 1.10 |

Colours: light page `#FBFCF7`, text `#1A1D17`, muted `#525C49`, panel `#F0F4E8`, line `#D8E0CC`, link, button and
focus `#33701F`, button text `#FFFFFF`, step `#33701F` with `#FFEA9C`, glow `rgba(98,140,43,.08)` and core
`rgba(255,234,156,.45)`. Dark page `#121610`, text `#EEF2E8`, muted `#B3BCA8`, panel `#1A2017`, line `#2E3829`, link
and button `#B6E05A`, button text `#121610`, focus `#FFEA9C`, glow `rgba(62,124,59,.26)` and core
`rgba(255,234,156,.1)`.

Notes for Part B (scratch `<scratchpad>\p6-brand-apply-web\`):

- Render: `pwsh -NoProfile -File render_site.ps1 -Width 1280 -Height 3900 -Tag desk`; phone: `-Width 600 -Height 7000
  -Frame 390 -Tag phone`; try CSS without editing the page: `-Extra var\x.css`. Side by side: `sheet.ps1`; enlarged
  crops: `crop.ps1` (regions `file|x|y|w|h`). Network: `netcheck.ps1`. Colours: `colour_scan.py <folder>`.
- The hero glow is centred on the icon at `50% 7rem` (4rem padding plus half the 6rem icon). If the lockup changes
  the hero, move the centre with it and rerun `check_page.py --colours`.
- Wordmark: Instrument Sans SemiBold, tracking -0.01em; font file and licence already in `site/fonts/`. For outlines
  (deliverable 6), the static TTF from the same commit is in scratch `dl-font\` (fontTools is not installed; the TTF's
  `glyf` table can be read with the standard library, and kerning sits in `GPOS`).
- Deliverable 9: no splash (decision 7 above); `pack.ps1` stays unchanged, so its acceptance check does not apply.

- 2026-11-05: started Part B (deliverables 5 to 10); `p6-brand-apply-art` is done. Orchestrator adds: pack a local x64
  release build with `pack.ps1` into scratch (no install) and check that `Setup.exe` carries every frame of the new
  ICO; fix the favicon `sizes=` in `index.html` (now 16, 32, 48).
- 2026-11-05: tools. Scratch `venv\` with fontTools 4.68.0, uharfbuzz 0.57.0, Pillow, numpy (outlines, shaping,
  picture maths; nothing installed outside scratch). Instrument Sans Regular TTF downloaded from the same commit
  `7fa22308` (`fonts/ttf/`, 86,232 bytes, name "Version 1.000", weight 400, fsType 0); the SemiBold TTF downloaded
  again equals Part A's. GitHub's page colours from Primer primitives 11.10.0 (`@primer/primitives` on unpkg,
  `dist/css/functional/themes/{light,dark}.css`): `--bgColor-default` `#ffffff` and `#0d1117`, the same as
  `p6-brand-apply-media` read from github.com's own CSS.
- 2026-11-05: deliverable 5 done. `site/wakeshot.svg` written by scratch `gen_icon.py`, which computes
  `Plated::at(256)` with Rust's rounding (margin 11, side 234, corner 43.875, light at 128,106 r 48, bar 58..198 x
  175..191, pill) and ports every layer: shadow (plate moved down 2.667, blurred to the box blurs' deviation 4.472,
  30% black), the 120 degree ramp as a userSpace gradient from the plate's top left to (287.825, 170.825), glow
  (r 145.08, 0.6 / 0.21 / 0), bottom shade (35% `#2C5A26` from y 139.7 to 245), all three in one masked group so the
  edge is drawn once as in `art.rs`; the bar's white to core ramp; bloom (r 91.2, never reaches the plate's edge, so
  no clip); the light's focal gradient (focus 117.44, 92.56; stops 0.5 core, 0.86 edge, 1 rim `#F7DC7F`). Against
  the ICO's 256 frame (scratch `compare_icon.py`, Edge at scale 1, transparent): over white mean 0.64, max 41; over
  `#121610` mean 0.47, max 29; alpha mean 0.23, max 52. Inside the shapes at most 3 (Chrome dithers gradients);
  the larger ones are single edge pixels where Chrome's anti-aliasing of a curve near its tangent differs (plate
  corners up to 41, bar ends 29, light edge 14). `art.rs` matches the exact coverage there (checked by sampling).
  Clip-path, mask and three separate plates give the same numbers. Looked at 100% on white and GitHub dark and at 6x:
  no difference a person sees at 100%; at 6x only Chrome's gradient dither and the edge pixels.
- 2026-11-05: deliverable 6 done. Scratch `textpath.py` shapes with HarfBuzz as Chrome does (default features; with
  letter spacing, spacing after each glyph and no optional ligatures) and draws fontTools outlines of the TTF; checked
  against Edge drawing the WOFF2 at 200 px: same ink box to the pixel, every glyph in the same place (kerning
  matches); only Chrome's vertical hinting moves some x-height edges by a pixel. Scratch `gen_lockup.py` writes
  `site/wakeshot-lockup-light.svg` and `-dark.svg` (304 x 80 px, 6.2 KB each): icon as in `wakeshot.svg`, the word's
  cap height 0.46 of the plate, centred on it, ink 0.26 of the plate after it, word `#1A1D17` light and `#EEF2E8` dark
  (the page's text colours). Compared cap 0.38 to 0.56 and gap 0.24 to 0.30 on GitHub's two colours at scale 1 and 2:
  0.38 looks timid, 0.56 heavy; kept 0.46 and 0.26. At 80 px the word is 44.6 px, the page's 44.
- 2026-11-05: the page hero now uses the same lockup in live text: `<h1 class="brand">` holds the icon (alt "") and
  the word in a centred flex row, icon 1.712em, gap 0.294em, line height 1 (with this font's 970/-250 metrics that
  centres the cap height on the icon). Compared with the stacked hero (icon over the name) in both themes: the row
  is the same lockup as the README and the social card, and drops the "centred icon over a big name" template W2
  called generic. The warm glow moved with the icon (circle 12rem at `calc(50% - 6.85rem) 6.35rem`). Fits a 390 px
  phone (295 px wide). Favicon `sizes` now "16x16 32x32 48x48".
- 2026-11-05: deliverable 7 done. `README.md` line 1 `# Wakeshot` is now `<h1 align="center">` around one
  `<picture>` (dark source by `prefers-color-scheme`, light `<img>` with alt "Wakeshot", 304 x 80); nothing else
  changed. GitHub's own Markdown API (`gh api markdown`, mode gfm) keeps the `h1`, its `align`, the `picture`, `source`
  and the img's alt, width and height, and wraps the picture in GitHub's `<themed-picture>` element (which picks the
  source by GitHub's theme). Drawn with that HTML in github-markdown-css 5.9.0 on `#ffffff` and `#0d1117`, each theme's
  source pinned: reads well at scale 1 and 2. A first render without pinning showed the dark lockup on white (Edge
  followed the system's dark theme): what a reader would see if `themed-picture` failed. A GitHub discussion
  (community #210983, 2026-10-22) reports exactly that; not verified, noted in the Result.
- 2026-11-05: deliverable 8 done. GitHub's docs ("Customizing your repository's social media preview", read
  2026-11-05): "PNG, JPG, or GIF file under 1 MB", "at least 640 by 320 pixels (1280 by 640 pixels for best
  display)". Drafts in scratch `sc\` (dark and light; tagline Regular or SemiBold, muted or text colour; glow 0.1,
  0.16, 0.16 wider): picked dark (the light left on at night, and it stands out in a white feed), the lockup 700 px
  wide, the README's first line in two lines of Instrument Sans Regular 40 px in the text colour (muted read faint at
  400 px wide; SemiBold heavy), warm glow 0.16 round the icon (0.1 was too faint, a wider one turned muddy). Source
  `site/social-card.html` (noindex, uses `wakeshot-lockup-dark.svg` and the new `site/fonts/InstrumentSans-Regular.woff2`,
  byte-equal to `media/public/fonts/`, SHA-256 `f28af62f...`), drawn by `site/social_card.py` (standard library;
  finds Edge or Chrome, retries, kills only its own tree, re-deflates at level 9, fails unless 1280 x 640 and under
  1 MB): `site/social-card.png` 285,642 bytes. Content box x 108 to 1169, y 164 to 482: at least 108 px from every
  edge. `index.html` has `og:type`, `og:url`, `og:title` and `og:description` (the page's own title and description),
  `og:image` with width, height and alt, `twitter:card` `summary_large_image`, `twitter:image` and alt; the Pages
  address `https://haikasatryan.github.io/wakeshot/` is GitHub's project site pattern (Pages is off until v1:
  `gh api repos/HaikAsatryan/wakeshot/pages` gives 404, so not verified live). `check_page.py` now checks share tags
  (under the Pages address, file exists, alt present, title and description equal the page's) and `url()` inside
  `<style>`; five negative tests on copies fail with exit 1 as they should.
- 2026-11-05: deliverable 9: no splash, as Part A found (decision 7); `pack.ps1` and `packaging/README.md` unchanged.
- 2026-11-05: deliverable 10. Rendered the whole page light and dark at 1280 px and in a 390 px frame, the hero
  enlarged 1.5x, the README top (GitHub's API HTML in GitHub's CSS) at scale 1 and 2, the social card at 1280 and
  400 px wide, the icon against the ICO at 100% and 6x. Changed after looking: the hero from the stacked icon and name
  to the lockup row; the README lockup from cap 0.50 / gap 0.28 drafts to 0.46 / 0.26; the card's tagline from muted
  to the text colour and Regular 38 to 40 px; the card's glow from 0.1 to 0.16. Kept four pictures:
  `docs/research/img/brand/apply-web-hero.png`, `apply-web-readme.png`, `apply-web-social.png`, `apply-web-icon.png`.
  Acceptance so far: `check_page.py` 0 errors, 8 waits, no "not linked" note; `--final` fails only on the 8 waits;
  colour scan of `site/` 0 flagged and no old hex anywhere in `site/` or `README.md`; network check (page served on
  127.0.0.1, Edge net log): the page started 3 requests (CSS, font, icon), all local, 0 elsewhere; no `http` in
  `style.css`. Local x64 release build for the Setup.exe icon check started (`CARGO_TARGET_DIR` in scratch).
- 2026-11-05: Setup.exe icon check (orchestrator's item). Release x64 build (2m 17s; tray 2,907,648 bytes, CLI
  701,440, as the art card measured), then `pwsh -NoProfile -File packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir
  <scratch target> -OutDir <scratch>\pack-x64 -Icon assets/wakeshot.ico`: exit 0, Setup.exe 12,327,360 bytes.
  Scratch `exe_icons.py` (ctypes): one icon group, 14 frames, each byte-equal to the ICO's (256 as PNG), each loads
  at its size with `CreateIconFromResourceEx`, and `PrivateExtractIconsW` gives 16 to 256 px. The portable zip's
  `Update.exe` and `Wakeshot.exe` the same. Build folder and pack output deleted. Nothing installed.
- 2026-11-05: on a 320 px phone the lockup (6.7em) pushed into the side padding (12 px left); the word's size is now
  `min(2.75rem, calc((100vw - 2.5rem) / 6.8))`, so it shrinks only below 340 px (320 px: 25 and 22 px margins; 390
  px and up unchanged). Second `social_card.py` run: byte-equal PNG. Facts added to `docs/research.md`, "Brand apply:
  the page, the README header and the social card". Final checks below; Part B done.

## Result

Done 2026-11-05 (Part A and Part B). The page, the README header and the social card are on the held light in Moss,
from the same numbers as `art.rs`, with one wordmark (Instrument Sans SemiBold, -0.01em) everywhere.

What was done:

- **Page** (Part A): Moss colours in both themes, gold bar gone, step numbers as the cream light on a moss disc, the
  wordmark font self-hosted. Part B: the hero is the lockup row (icon beside the live-text name, the same proportions
  as the SVG lockup), the glow centred on the icon; favicon `sizes` 16, 32, 48; full share tags.
- **`site/wakeshot.svg`**: the 256 px app icon ported layer by layer from `art.rs` (shadow, 120 degree ramp, glow,
  shade, bar, bloom, light with its focal gradient). Against the ICO's 256 frame: over white mean 0.64, max 41 of 255
  (edge pixels only; inside the shapes 3 at most); no difference a person sees at 100%.
- **Lockups** `site/wakeshot-lockup-light.svg` and `-dark.svg`: icon plus "Wakeshot" as outlines (HarfBuzz shaping,
  checked against Edge's own text), 304 x 80 px.
- **README**: line 1 is a centred `<h1>` with one `<picture>` (dark source, light `<img>`, alt "Wakeshot"). Nothing
  else changed.
- **Social card** `site/social-card.png`, 1280 x 640, 285,642 bytes, dark, the lockup and the README's first line;
  source `site/social-card.html`, drawn by `site/social_card.py` (byte-equal on a second run).
- **Installer splash**: none. Velopack 1.2.161 shows its own progress window with words, a title bar and a progress
  bar when no `--splashImage` is given (source chain in `docs/research.md`); a splash would replace it with a
  wordless picture. `pack.ps1` and `packaging/README.md` unchanged.
- **Setup.exe icon** (orchestrator's item): packed locally, Setup.exe holds all 14 ICO frames byte-equal, the 256 PNG
  included, and Windows loads each.

Files: `site/index.html`, `site/style.css`, `site/check_page.py`, `site/wakeshot.svg`; new `site/wakeshot-lockup-light.svg`,
`site/wakeshot-lockup-dark.svg`, `site/social-card.html`, `site/social_card.py`, `site/social-card.png`,
`site/fonts/InstrumentSans-SemiBold.woff2`, `site/fonts/InstrumentSans-Regular.woff2`, `site/fonts/OFL.txt`;
`README.md` (heading only); `docs/research.md` (two sections added); pictures
`docs/research/img/brand/apply-web-hero.png`, `apply-web-readme.png`, `apply-web-social.png`, `apply-web-icon.png`.
Not changed: `packaging/`, `site/favicon.ico`, `assets/`, `art.rs`, `media/`.

Checks:

| Check | Outcome |
|---|---|
| `python site/check_page.py site` | pass: 0 errors, 8 spots wait, no "not linked" note |
| `python site/check_page.py --final site` | fails only on those 8 spots (exit 1), as before |
| New `check_page.py` checks, negative tests on copies | missing og:image file, og:title drift, missing alt, wrong og:url, missing `<style>` font: each exit 1 |
| No violet or gold in `site/` | scratch `colour_scan.py`: 0 flagged; no `#7250F4`, `#7957F4`, `#6B49F3`, `#5B3FD6`, `#A996FF`, `#FFCC4D`, `#14121F` in `site/` or `README.md` |
| No request leaves the page | Edge net log, page on 127.0.0.1: 4 requests by the page (CSS, font, icon, favicon), all local, 0 elsewhere; no `http` in `style.css` |
| `wakeshot.svg` against the ICO's 256 frame | numbers above; no visible difference at 100% (looked on white and GitHub dark, and at 6x) |
| `pack.ps1` | unchanged, so its check does not apply; still ran it on a local x64 release into scratch for the icon check: exit 0 |
| Social card | 1280 x 640, 285,642 bytes (under 1 MB), content at least 108 px from every edge |
| Look | page light and dark at 1280, 390 and 320 px; README top in GitHub's CSS at scale 1 and 2; card at 1280 and 400 px; icon at 100% and 6x |

WCAG 2.2 contrast (page pairs from `check_page.py --colours`, unchanged since Part A; "hero" is the worst of the bare
page, under each glow and under both):

| Pair | Needs | Light | Dark |
|---|---|---|---|
| text on page / panel / hero | 4.5 | 16.53 / 15.27 / 14.67 | 16.12 / 14.65 / 9.34 |
| muted on page / panel / hero | 4.5 | 6.82 / 6.30 / 6.05 | 9.30 / 8.45 / 5.39 |
| link on page / panel / hero | 4.5 | 5.86 / 5.41 / 5.20 | 12.02 / 10.93 / 6.96 |
| button text on button | 4.5 | 6.04 | 12.02 |
| step number on step disc | 4.5 | 5.03 | 5.03 |
| button edge on page / hero | 3.0 | 5.86 / 5.20 | 12.02 / 6.96 |
| focus ring on page / panel / hero | 3.0 | 5.86 / 5.41 / 5.20 | 15.25 / 13.86 / 8.83 |
| lockup word on GitHub `#ffffff` / `#0d1117` | 4.5 | 17.04 | 16.67 |
| card line `#EEF2E8` on card, haze, glow, both | 4.5 | none | 16.12 / 12.33 / 10.51 / 7.82 |
| plate ends on GitHub's page (non-text) | 3.0 | 3.96 to 5.06 | 3.74 to 4.78 |

Open items:

- Haik's check after the push: on github.com set the theme opposite to Windows' (Settings > Appearance), open the
  repo and see whether the header follows GitHub's theme (decision 14).
- The orchestrator: upload `site/social-card.png` as the social preview; the `pages.yml` step and the stale docs
  under "Found outside scope".
- Not verified: the Pages address live (Pages is off until v1); how Velopack's progress window shows the icon (a real
  install, screen session).
