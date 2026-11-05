# Apply the new brand to the page, the README header, the social card and the installer splash

Phase: 6 (brand apply). Status: in progress (Part A done; Part B waits for p6-brand-apply-art). Needs Haik: no (his 8 brand decisions, 2026-11-05 ~15:00, all defaults).
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
- `site/favicon.ico` and `site/wakeshot.svg` are still violet and gold until `p6-brand-apply-art` and Part B.

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

## Result
