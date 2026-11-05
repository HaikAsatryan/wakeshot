# Brand review of the current app

Part A of `docs/tasks/p6-brand-directions.md`: every surface a user sees, as it stands on 2026-11-05, judged for
looks and brand. Plain and critical. Severity: **blocker** (fix before launch), **should fix**, **nice to have**.
Each finding names the picture it is seen in. `review-*` pictures are in `img/brand/` (new, made for this review from
the repo's own pictures and renders); the others are in `img/ux/` or `img/`. Picture paths below are relative to
`docs/research/`; other paths start at the repo root. Facts carry how they were checked; everything else is judgement and says so.

Status: done 2026-11-05. Surfaces with no picture and no offscreen render are listed in "Next screen session".

## Verdict in short

Judgement, built on the findings below.

- **The Windows-native parts are good.** The menu, the tooltip, the notices and the monochrome tray glyph follow
  Windows 11 closely and look like they belong. Keep how they are built.
- **The brand parts are generic and foreign to Windows.** The app icon is a flat violet rounded square with a stock
  sunrise glyph on it. Next to Windows 11's own icons it looks like a phone app or a web favicon, and it is the
  "AI purple" of 2026. The page, the poster and the loop repeat the same look, so the whole public face reads as a
  template, not as Wakeshot.
- **The tray glyph is fine at 150% and 200%, weak at 100%.** At 16 px the rays break into loose specks, the idle arc
  goes soft, and the off glyph becomes a smudge. It also sits about 2 logical px below Windows' own glyphs. At a
  glance it can read as a siren, a room-service cloche and a muted bell (judgement; test it with people).
- **Two blockers**, both cheap or already planned: the app icon file has no frame above 64 px, so Explorer shows it at
  less than half the size of other apps; and the page and README have no picture of the product yet (planned in
  `p6-media` Part B, which should wait for the brand pick).

## Blockers for launch

| # | Finding | Seen in |
|---|---|---|
| B1 | **The ICO stops at 64 px.** In Explorer's Large icons view at 150% the item box is 142 px wide and our icon is drawn at 64 px inside it, under half the size of an app with a 256 px frame. Extra large icons and Start at high scale will show it small or blurry too (not seen yet). Microsoft's minimum set is 16, 24, 32, 48 and 256 (`brand-windows.md` section 1). Cheap: add 256 (and 96, 128) to `APP_ICON_SIZES`. | `img/ux/int-exe-icon.png` (measured: plate 64 by 64, item box 142 by 160) |
| B2 | **No product picture on the page or the README.** The first screen of both is an icon, a name and text; a visitor never sees the tray, the menu or a notice. Planned: `p6-media` Part B fills the placeholders. Run it after the brand pick, or its shots are redone. | `img/brand/review-page-hero-light.png`, `review-page-hero-dark.png`, `review-page-full-*.png`; `README.md` lines 3, 50 to 62 (HTML comments only) |

## Findings by surface

### 1. Tray glyph (three states; dark, light and high contrast; 100%, 150%, 200%)

What works: one colour like Windows' own glyphs (white, `#1A1A1A`, the high contrast text colour); outline for idle
and filled for holding, which is Fluent's own regular and filled rule for "on"; three states that differ; crisp at
24 and 32 px; the high contrast colours come from the system.

| # | Finding | Severity | Seen in |
|---|---|---|---|
| T1 | At 16 px (100%) the holding glyph is a stepped blob with two loose 2-pixel specks for the diagonal rays. It reads as noise, not rays. 100% is the normal scale of a desktop monitor and of many external screens. | should fix | `img/brand/review-tray-states.png` (first column of each enlarged row), `img/ux/icon-sheet-*.png` |
| T2 | At 16 px the idle arc is drawn in grey anti-aliased pixels while the horizon is solid, so the arc looks fainter than the line. On the light taskbar it nearly disappears. | should fix | `img/brand/review-tray-states.png` (idle, 16 px, light row) |
| T3 | At 16 px the off glyph is a soft grey diagonal over two crumbs of arc. Off exists to warn a user who forgot Automatic is off; at 100% it is the hardest state to read. | should fix | `img/brand/review-tray-states.png` (off, 16 px) |
| T4 | The glyph sits low. On the real dark taskbar at 150% its ink centre is at y 39.2 (holding), 38.5 (idle) of 72, while the chevron is at 35.3 and the next app's icon at 34.9; the taskbar centre is 36. About 3 px low at 150%, 2 logical px. The ink starts 5 px below the top of the 24 px box and ends 2 px above the bottom. | should fix | `img/brand/review-taskbar-dark-150.png`, `img/ux/taskbar-dark-*.png` (measured by script) |
| T5 | The idle glyph is wide and flat: 11 px of ink height at 150% against 19 px for the next app's icon, so it looks smaller than its neighbours. | nice to have | `img/brand/review-taskbar-dark-150.png` |
| T6 | Lookalikes at tray size (judgement): holding reads close to a siren or alarm light (a dome on a base with rays), idle to a room-service cloche, off to a muted bell. A siren in the tray while all is well, or a "muted" sign, sends the wrong message. Test with three to five people before keeping it. | should fix | `img/brand/review-tray-lookalikes.png` (ours left in each pair; the stand-ins right are drawn for this review from the generic idea of each sign) |
| T7 | The off slash runs past the glyph's box, from above the rays to below the horizon, so off looks bigger and busier than idle and holding. Windows' own struck-through glyphs keep the slash inside the shape. | nice to have | `img/ux/icon-sheet-dark.png` (third row), `img/ux/taskbar-keyboard-focus.png` |
| T8 | No true picture of the current glyph on a light taskbar exists: in `taskbar-light-*.png` Windows left the taskbar dark after the theme switch (`docs/research.md`, "Tray UX: tray icon"), and the menu covers it in `menu-screen-light-sub.png`. The render on the light taskbar colour is in `icon-sheet-light.png`. | screen session | `img/ux/taskbar-light-holding.png` |

### 2. Tooltip

What works: Windows draws it, so it is native in both themes; the first line is the name and the second the state.
No look or brand finding. Only a dark picture exists, from a demo build ("Wakeshot demo").

| # | Finding | Severity | Seen in |
|---|---|---|---|
| TT1 | None for the look. | | `img/ux/taskbar-tooltip-dark.png` |

### 3. Menu and submenu

What works: the most native surface of the app. WinUI 3 `MenuFlyout` sizes, Segoe UI Variable, Segoe Fluent Icons,
rounded corners, border and shadow from DWM, light, dark and high contrast palettes from Windows' own tokens. No brand
in the menu, which is right: Windows' own tray menus carry none. The new clock for Keep awake reads as "for a time".

| # | Finding | Severity | Seen in |
|---|---|---|---|
| M1 | Text and glyphs are drawn with ClearType, so thin glyphs get colour fringes. The check next to Automatic looks red-brown in light and at 1:1 (the mark colour is grey `secondary`; the red comes from the fringes). Windows 11's own XAML menu draws grayscale: measured chroma 0 in its text against up to 158 in ours. | fixed 2026-11-05 (`p6-menu-text-look`: grey text, chroma 0) | `img/brand/review-menu-glyph-fringes.png` (light top, dark below, 6x), `img/ux/menu-render-144-light.png`, `img/ux/menu-screen-light-sub.png`; native: `img/tray-look-light-explorer-new.png` |
| M2 | The menu grows to the text cap of 480 epx plus the gutters, about 570 logical px, when the status line is long (two hold names). That is twice the width of Windows 11's own Explorer menu in a picture from this laptop (about 410 px against our 853 px). A long status line makes the menu a banner across the screen. | waits for Haik (`p6-menu-text-look`: WinUI sets no menu max width; default keeps 480 epx) | `img/ux/menu-screen-light-sub.png`, `menu-screen-dark-sub.png`; native: `img/tray-look-light-explorer-new.png` |
| M3 | An eye for Automatic: in Windows an eye means "show" or "reveal" (password boxes, visibility). Here it is meant as "watches agents", which may also read as "watches me". | nice to have | `img/ux/menu-render-*.png` |
| M4 | The screen pictures still show the cup; only the renders show the clock. | screen session | `img/ux/menu-screen-*.png` |

### 4. Notices

What works: Windows draws them; a warning shows Windows' own warning triangle, other notices show the app icon in
the body; the header names Wakeshot once the app id is registered.

| # | Finding | Severity | Seen in |
|---|---|---|---|
| N1 | The header icon of the shipped app (the registered 48 px PNG, drawn at about 16 logical px) has never been seen on screen: the one installed picture shows a stale cup from Windows' cache, and the dev pictures show the tray glyph because the harness is not registered. Judging from the 16 and 24 px frames, the rays become specks and the icon reads as a violet square with a yellow blob. | screen session; then nice to have | `img/ux/int-first-run-balloon.png` (cup), `img/ux/notice-dark-first-run.png` (header = tray glyph, "ux_notices.exe"), `img/brand/review-app-icon-small.png` |
| N2 | The app icon in the body is the flat violet plate (see A1, A2): the most frequent place a user sees the brand after the tray. | should fix (follows the brand pick) | `img/ux/notice-dark-first-run.png`, `notice-light-first-run.png`, `notice-dark-updated.png` |
| N3 | The first-run notice writes the hidden-icons arrow as a bare "^". In the notice it looks like a stray character. | nice to have | `img/ux/notice-dark-first-run.png` |

### 5. App icon where Windows shows it

What works: one mark in every place; the plate keeps the gold sun readable on light and dark (contrast in
`docs/brand.md`); the frames are drawn by code per size, so the edges are sharp.

| # | Finding | Severity | Seen in |
|---|---|---|---|
| A1 | Foreign to Windows 11 (judgement on `brand-windows.md` facts): all 20 Microsoft app icons on this laptop are free shapes with soft depth and no backdrop. Ours is a full square of saturated violet. Among them it is the heaviest, flattest item, and reads as a phone app or a web favicon. | should fix (the main brand issue) | `img/brand/win-microsoft-app-icons.png` beside `img/ux/icon-app.png`; `img/ux/int-exe-icon.png` |
| A2 | Flat and dated: the plate's gradient (`#7957F4` to `#6B49F3`) is too small to see, there is no highlight or depth, and the sun and rays are flat fills. Microsoft's 2025 icons went to "depth, and highlights" (`brand-windows.md`). | should fix | `img/ux/icon-app.png` |
| A3 | Generic: the mark is the stock "sunrise" of most icon sets (Material `wb_twilight` is nearly the same drawing, `brand-identity.md` section 3), and violet is 2026's crowded AI colour. | should fix (Part B) | `img/ux/icon-app.png` |
| A4 | At 16, 20 and 24 px the diagonal rays are single loose pixels: noise in Start's list, the notice header and small Explorer views. | should fix | `img/brand/review-app-icon-small.png` |
| A5 | Start (pinned and All), Settings (Installed apps, Startup), Task Manager and the taskbar (when pinned) were never captured. | screen session | none |
| (B1) | No 256 px frame: see Blockers. | blocker | `img/ux/int-exe-icon.png` |

### 6. Installer

What works: one click, no wizard, no admin prompt (Velopack; `docs/research/distribution.md`).

| # | Finding | Severity | Seen in |
|---|---|---|---|
| I1 | No picture of a real, visible install exists. `pack.ps1` passes no `--splashImage`; Velopack's docs say the splash is "shown while installing" and do not say what shows without one. So the user may see nothing for a few seconds, then a notice. A splash with the brand is the one place the installer can carry it. | screen session; then should fix | none (`packaging/pack.ps1` lines 124 to 140 read) |
| I2 | The first thing a new user sees is SmartScreen's "Windows protected your PC", unknown publisher (v1 is not signed, the page says so). Not a design matter; see "Found outside scope". | outside scope | none |

### 7. Web page (`site/index.html`, light and dark)

What works: clean, readable, accessible (the text pairs pass 6:1, `p6-pages`), system fonts, no scripts, follows
the system theme, fits a 390 px phone without overflow (checked in a real 390 px frame).

| # | Finding | Severity | Seen in |
|---|---|---|---|
| W1 | No product picture (B2). | blocker (planned) | `img/brand/review-page-hero-*.png` |
| W2 | Generic: a centred icon, a big name, a violet button, a soft violet glow at the top. This is the common 2023 to 2025 SaaS and AI landing page; nothing on it is Wakeshot's own. | should fix (follows the brand pick) | `img/brand/review-page-hero-light.png`, `review-page-hero-dark.png` |
| W3 | A lone 128 px gold bar under the hero has no job; it reads as a leftover. | nice to have | `img/brand/review-page-hero-*.png` |
| W4 | Below the hero the page is text, cards and one table for about 3,600 px with nothing to look at (gold step numbers aside). | should fix (with B2's pictures) | `img/brand/review-page-full-light.png`, `review-page-full-dark.png` |
| W5 | On a phone the two buttons stack at different widths. | nice to have | `img/brand/review-page-phone-light.png`, `review-page-phone-dark.png` |

### 8. README and the GitHub repo card

| # | Finding | Severity | Seen in |
|---|---|---|---|
| R1 | No icon, name lockup or picture at the top; on GitHub the first screen is a heading and text, then a large comparison table. The loop and the pictures are planned (B2). | blocker (planned) | `README.md` lines 1 to 14 |
| R2 | The repo has no description, no website link, no topics and no custom social preview (GitHub API, 2026-11-05: `usesCustomOpenGraphImage` false, `description` null, no topics). A shared link shows GitHub's generic card; search shows no one-line pitch. Launch posts link the repo (`p6-pages` decision 10). | should fix | GitHub repo settings (no picture) |

### 9. Icon loop (`media/out/icon-loop-*.gif`)

What works: within its spec (480 by 240, 125 frames, 5.00 s, 459 KB dark and 269 KB light, under 1 MB); first and
last frame equal, so no jump; it shows the product's real behaviour.

| # | Finding | Severity | Seen in |
|---|---|---|---|
| L1 | The glow goes wrong in both themes: on dark the gold at low opacity over near-black turns into a brown haze; on light it is a beige stain around a black sun. A glowing black sun is a contradiction. | should fix | `img/brand/review-icon-loop-frames.png` (right column) |
| L2 | As the README's first picture it shows only the monochrome tray glyph: no app icon, no colour, no name. The face of the project becomes the stock glyph (A3). | should fix (follows the brand pick) | `img/brand/review-icon-loop-frames.png` |
| L3 | The half-way frame is a dome with a small hole, which reads as a helmet or a mailbox, not a rising sun. | nice to have | `img/brand/review-icon-loop-frames.png` (middle column) |
| L4 | The GIF backgrounds (`#14121F`, `#F3F3F3`) are not GitHub's page colours, so the loop sits in a visible box on GitHub in both themes (GitHub's colours `#FFFFFF` and about `#0D1117`: not verified). | nice to have | `img/brand/review-icon-loop-frames.png` |

### 10. Across surfaces

| # | Finding | Severity | Seen in |
|---|---|---|---|
| X1 | Two wordmarks: Segoe UI Variable Display Semibold on the page (and on Mac or Linux whatever `system-ui` is), Inter SemiBold in the video and its poster (`media/src/fonts.ts`). Side by side the letters differ ("a", "k", the spacing). `brand-identity.md` picks one free face for the wordmark. | should fix | `img/brand/review-page-hero-light.png`, `media/out/draft/wakeshot-promo-poster-draft.png` |
| X2 | Two shape styles of one mark: square-ended rays and horizon in the tray art and the ICO; round caps in `site/wakeshot.svg`, the loop and the poster. | nice to have | `img/ux/icon-sheet-dark.png` beside `img/brand/review-icon-loop-frames.png` |
| X3 | Violet and gold is also the pairing of Shut the Lid, a Mac keep-awake app (`brand-macos.md`), and violet is the most crowded colour among new AI tools (`brand-identity.md` section 4). | should fix (Part B) | `img/ux/icon-app.png` |

## The top five should-fix items

Ranked by how much each changes what a user sees, cheapest first where equal.

1. **App icon: foreign, flat and generic (A1, A2, A3).** The single biggest brand problem; Part B's directions answer it.
2. **Tray glyph at 100% (T1, T2, T3) and its low placement (T4).** A 16 px pass in `art.rs`: drop or merge the
   diagonal rays at 16 and 20, a solid 1 px arc, a crisp off slash, the ink centred 1 to 2 px higher.
3. **Tray lookalikes (T6).** Siren, cloche, muted bell: check with people; a new mark in Part B may solve it.
4. **Menu colour fringes (M1).** Draw the check, the bullet and the icons in grayscale, as Windows' XAML menus do.
5. **Repo card and wordmark (R2, X1).** A description, topics and a social preview card; one wordmark face everywhere.

## Next screen session: what to capture

None of these can be rendered offscreen. Quit any installed Wakeshot first (`CLAUDE.md`). Capture at 150% and, where
a monitor allows, at 100%, light and dark.

1. **Tray on a real light taskbar**, three states, beside Windows' own glyphs; also high contrast (Night sky and
   Desert). The old light pictures show a dark taskbar (T8).
2. **The menu with the clock**, light and dark, main and submenu, with a long status line (M2, M4).
3. **A notice from the installed build** (registered app id), light and dark: the header icon and name (N1).
4. **Start**: the pinned grid, the All list and a search result for "Wakeshot" (A5).
5. **Settings**: Apps > Installed apps and Apps > Startup (A5).
6. **Explorer**: the install folder in Large and Extra large icons and in Details (B1).
7. **Task Manager**: Processes and Startup apps (A5).
8. **The installer from a double click** (not `--silent`): SmartScreen, whatever Velopack shows with no splash image,
   then the first-run notice (I1, I2). This installs on the real laptop, so it runs as an install card does.

## Facts checked for this review (2026-11-05)

- Explorer, Large icons, 150%: our plate is 64 by 64 px inside a 142 by 160 px item box. Checked by a script that
  found the violet pixels and the selection box in `img/ux/int-exe-icon.png`. `assets/wakeshot.ico` frames 16, 20,
  24, 32, 40, 48, 64 (read from its header by script); `site/favicon.ico` is the same file (equal SHA-256).
- Tray placement at 150%: ink centres from `img/ux/taskbar-dark-*.png` by script (pixels brighter than 120): chevron
  35.3, ours 39.2, 38.5 and 37.3 (holding, idle, off), the next app's icon 34.9, in a 72 px taskbar.
- Menu text rendering: the largest channel spread (max minus min of R, G, B) in a text crop is 0 in Windows 11's XAML
  menu (`img/tray-look-light-explorer-new.png`, 226 dark text pixels) and `img/tray-look-dark-explorer-new.png`;
  158 in our render and our screen picture. So Windows' menu draws grayscale and ours draws ClearType.
- Menu width: `TEXT_MAX` 480 epx in `crates/wakeshot-windows/src/tray/menu_paint.rs`; the main menu in
  `menu-screen-*-sub.png` is about 850 px wide at 150%.
- Loop: 125 frames, 500 centiseconds, 480 by 240 px, decoded by a stdlib GIF reader written for this review.
- Velopack installer: "`--splashImage {path}` sets the (possibly animated) splash image to be shown while installing";
  the page does not say what shows without one (`https://docs.velopack.io/packaging/installer`, read 2026-11-05).
  `packaging/pack.ps1` passes no splash image (read).
- GitHub repo: `usesCustomOpenGraphImage` false, `description` null, `homepageUrl` null, no topics (`gh api graphql`,
  2026-11-05).
- The page fits a 390 px phone: rendered in a 390 px iframe. A plain 390 px headless window clips the page, but that
  is headless Edge's minimum window width, not the page.

Not verified: how Start, Settings, Task Manager and the installer show the icon; the shipped notice header; the light
taskbar; GitHub's exact page colours; whether people read the glyphs as siren, cloche or muted bell.

## How the pictures were made

Scripts in the scratch folder `<scratchpad>\p6-brand-review\` (not in the repo): `png.py` (a stdlib PNG reader and
writer, no Pillow on this laptop), `gif.py` (a stdlib GIF frame reader), `review_pics.py` (crops and enlargements of
`img/ux/` pictures, nearest neighbour), `render_site.ps1` and `shot.ps1` (headless Edge, offscreen, own profile per
run, `--force-device-scale-factor=1`; the page copy pins light or dark by rewriting its one
`prefers-color-scheme` query), `lookalikes.html` (the generic stand-ins for T6, drawn here). No tray, no screen, no
installs, no power settings.

## Found outside scope

- SmartScreen's unknown-publisher wall is the installer's first screen (I2). Signing costs money: Haik's call.
- `p6-media` Part B (README and page shots) should run after the brand apply card, or its pictures are redone (B2).
- `docs/research.md` has no pointer to this review; the orchestrator may add one.

## Log

- 2026-11-05 13:10: started. Read the card (Goal, Haik's answers, Log, Part A), the Result sections of
  `p6-brand-research-windows`, `-macos`, `-identity` and `p6-brand-cleanup`, `docs/brand.md`.
- 2026-11-05 13:12: no Pillow, no installs allowed: wrote a stdlib PNG reader and writer, then a GIF reader. Looked at
  every picture in `img/ux/` that the card names, enlarged where small.
- 2026-11-05 13:15: rendered the page light, dark, desktop and phone with headless Edge. The first dark render failed
  silently (two runs shared one Edge profile); fixed with a profile per run. The first phone render clipped the page
  (headless minimum window width); redone in a 390 px iframe.
- 2026-11-05 13:20: built the `review-*` pictures, checked each by eye at 1:1 and enlarged; measured the Explorer
  icon, the tray placement and the menu fringes by script.
- 2026-11-05 13:30: report written; no code, no shipped asset, no card edited.
- 2026-11-05 13:35: checked every surface the card lists against the report (tray three states in three themes and
  three scales, tooltip, menu and submenu, notices, app icon in Windows' places, installer, page light and dark,
  README, loop): each has findings with a severity and a picture, or a screen-session entry. No dashes or emojis.
  Edge profiles deleted; no Edge process left from these renders (checked by command line).
