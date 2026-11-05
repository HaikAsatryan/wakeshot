# Brand

What Wakeshot looks like, and why: the held light in Moss. Haik's eight brand decisions, 2026-11-05, all the
defaults ("I have reviewed and fully agree with you"; `docs/phases/06-ship.md` work log, 2026-11-05 ~15:00). Every
drawing comes from one source, `crates/wakeshot-windows/src/tray/art.rs`; the page, the media and the README port
from it. The reasons in full: `docs/research/brand-directions-2.md`. Measured numbers and how they were checked:
`docs/research.md`, the "Brand apply" sections and "Brand colours under colour-blind simulation".

## The idea

A light left on while work runs. A round light held over a line: filled while something holds the PC awake, the same
light as an outline when nothing does, and the outline struck through when Automatic is off. The light never moves: it
is on while work runs and off when it is done. In motion (the README loop, the video's title) the light fades on and
off in place.

Why: it is what Wakeshot does, in one picture, and it follows Fluent's own rule (filled is on, outline is off). It is
not a weather sign, and not the picture another keep-awake app already shows (Shut the Lid on the Mac,
`docs/research/brand-macos.md` section 6).

## Colours

| Name | Hex | Where |
|---|---|---|
| Plate | `#628C2B` to `#3E7C3B` | The app icon's plate, top left to bottom right, at 120 degrees as CSS reads the angle |
| Plate shade | `#2C5A26` | The bottom of the plate, lit frames only |
| Light core | `#FFFBE8` | The middle of the light; the bar's right end (the bar runs from white) |
| Light edge | `#FFEA9C` | The light's edge in small frames, its glow and bloom; the page's step numbers and dark focus ring |
| Lit rim | `#F7DC7F` | The light's outermost ring in lit frames (30 px and up), so the lit light keeps a warm edge |
| Holding on dark | `#B6E05A` | The tray light while holding, dark taskbar; links and buttons on the dark page; the video's accent |
| Holding on light | `#3B7A2A` | The tray light while holding, light taskbar |
| Link on light | `#33701F` | Links, buttons and focus ring on the light page; the step number discs on both pages |
| Glyph ink | `#FFFFFF`, `#1A1A1A` | The tray line and outline on a dark and on a light taskbar, as Windows' own glyphs |

Page, light: page `#FBFCF7`, text `#1A1D17`, muted `#525C49`, panel `#F0F4E8`, line `#D8E0CC`, button text white.
Page, dark: page `#121610`, text `#EEF2E8`, muted `#B3BCA8`, panel `#1A2017`, line `#2E3829`, button text
`#121610`. The video and the social card are dark on `#121610`; the video's own tokens (dim `#A3AD99`, panel
`#1B2118`, terminal `#0C0F0B`) are in `media/src/theme.ts`. The README loop sits on GitHub's own page colours,
`#FFFFFF` and `#0D1117`.

Why Moss: the hue band 100 to 150 is the emptiest among AI and developer brands (`docs/research/brand-identity.md`
question 4); every brand colour is far from Claude's orange (CIEDE2000 34.8 or more), so Wakeshot never looks like an
official Claude tool; green is the colour of "on", so the holding colour means what it says; and a warm light glows
best against a cool green.

Contrast (WCAG 2) and the worst of protan, deutan and tritan simulation (Machado 2009, full strength):

| Pair | Ratio | Worst colour-blind | Needs |
|---|---|---|---|
| Glyph ink white on dark taskbar `#202021` / its hover `#2D2D2E` | 16.28 / 13.76 | 16.27 / 13.75 | 3.0 |
| Glyph ink `#1A1A1A` on light taskbar `#D8DADC` / its hover `#ECEDEE` | 12.42 / 14.85 | 12.40 / 14.84 | 3.0 |
| Holding `#B6E05A` on dark taskbar / hover | 10.70 / 9.04 | 10.53 / 8.90 | 3.0 |
| Holding `#3B7A2A` on light taskbar / hover | 3.74 / 4.48 | 3.58 / 4.28 | 3.0 |
| Plate ends on light Start `#F3F3F3` | 3.57, 4.56 | 3.48, 4.35 | 3.0 |
| Plate ends on dark Start `#202020` | 4.12, 3.22 | 4.04, 3.11 | 3.0 |
| App icon, drawn: the light's edge on the plate next to it, flat frames 16 to 24 px | 3.75 to 3.95 | 3.53 to 3.76 | 3.0 |
| App icon, drawn: the light's core `#FFFBE8` on the plate just past its bloom, lit frames 30 to 256 px | 3.11 to 3.60 | 3.03 to 3.48 | 3.0 |
| App icon, drawn: the lit light's edge on the plate next to it (its soft edge; for the record) | 1.40 to 2.03 | 1.35 to 1.97 | none |
| App icon, drawn: the bar (white to `#FFFBE8`) on the plate next to it, every frame | 3.12 to 4.34 | 3.05 to 4.20 | 3.0 |
| Half rule at 48 px (share of the icon at 3:1), light / dark Start | 69% / 91% | 68% / 87% | 50% |
| Page text on page, light / dark | 16.53 / 16.12 | 16.47 / 16.05 | 4.5 |
| Page link on page, light / dark | 5.86 / 12.02 | 5.58 / 11.79 | 4.5 |
| Step number `#FFEA9C` on its disc `#33701F` (the page's lowest text pair) | 5.03 | 4.61 | 4.5 |
| Lockup word on GitHub `#FFFFFF` / `#0D1117` | 17.04 / 16.67 | 17.02 / 16.67 | 4.5 |

The app icon is measured on its drawn pixels, `app_icon(size)` at every frame: each fully covered pixel of a shape
against the nearest plain plate pixel one or two steps away, and the half rule over the whole icon. The rule for the
light: in a lit frame its core keeps 3:1 against the plate just past the bloom, plain and under protan, deutan and
tritan. The bloom and the glow are the light's own soft edge, so its edge as drawn sits under 3:1 by design; the warm
rim `#F7DC7F` is a detail of that edge, round 2's look as Haik approved it, and is not counted. A flat frame has no
bloom, so there the light's edge itself keeps 3:1. The bloom ends 1.6 light radii from the centre, so at the bar's
top edge it is down to 0.08 or less and the bar keeps 3:1 as drawn at every frame. Checked by the `art.rs` tests
`glyph_contrast_on_taskbars`, `app_icon_contrast` and `half_the_icon_keeps_3_to_1_on_start`, plain and colour-blind
(the simulation is not rounded to whole levels, so these numbers differ from a rounded run by up to 0.03); by `python
site/check_page.py --colours site` for the page (every pair, both themes, under the hero's glows too); and by a script
for the page's colour-blind column (`docs/research.md`).

## Tray icon

- **Three states, by shape.** Holding: the filled light over the line. Idle: the same light as an outline, in the
  same place. Off: the idle outline with the light struck through, the Windows sign for off; the slash runs from the
  light's top left corner through its centre and stops a stroke above the line, so the line stays whole and off is no
  larger than idle (Windows' own off glyphs keep the slash inside the shape). Why three: with two,
  idle and off look the same, so a user who turned Automatic off and forgot sees nothing wrong, starts a long agent
  run, and the PC sleeps mid-run (Haik, 2026-11-04). Change it when users say the off icon confuses them.
- **Brand colour only while holding.** The line and the outline are Windows' glyph colour (white on a dark taskbar,
  `#1A1A1A` on a light one); while holding, the light alone is Moss. Idle and off stay one colour. High contrast draws
  every part in the window text colour (`COLOR_WINDOWTEXT`). Why: green means "on" exactly when something holds; the
  states still differ by shape, so colour is never the only sign (WCAG 1.4.1); high contrast stays the system's.
- **Drawn per pixel at the taskbar's size** (16 to 64 px, 100% to 400%): the light half the box, the line 3/4 of it,
  a gap of 1/8, whole-pixel strokes of size / 16. The 1 px rings at 16 and 20 px are drawn by hand, so the ring is as
  solid as the line. The states differ in 28% to 58% of their ink at 16 to 24 px (the test asks 25%).
- **Level with its neighbours.** The filled glyph's ink centre sits at 0.455 of the box, where the next app's icon
  sits on the taskbar: within 0.5 px at every size. Idle and off sit 1 to 4 px lower by ink, because the light
  keeps its place in every state.

## App icon (Windows)

- **Plated.** The lit light over a white bar on a rounded Moss square: the plate inset by size / 24, its corners 3/16
  of its side, the ramp at Microsoft's default 120 degrees. From 30 px up the frames are lit: a glow round the light,
  a bloom that fades out by 1.6 light radii (nearly gone where the bar starts), the warm rim, a shade at the bottom
  and a soft drop shadow (a 96th of the icon down, blurred by a 64th, 30% black). Below 30 px they are flat drawings
  of the same mark, tuned to whole pixels. Why plated: a light is pale and needs dark round it to glow; without a
  plate the icon keeps 3:1 on only 24% of its area on light Start, under Microsoft's half rule (round 2's drawing).
  Why flat when small: blur and shadow turn to mud there (Microsoft's old guide: no shadows at 24 px or below).
  Change it when a free shape shows well on light Start in a screen test, or when an MSIX package lets us ship light
  and dark icons.
- **All 14 of Microsoft's target sizes** in `assets/wakeshot.ico`: 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96
  and 256, the 256 frame as PNG (31,114 bytes), the rest 32-bit bitmaps; 184,720 bytes, every frame of it in each exe,
  and Setup.exe carries it whole. Why: Windows finds an exact frame at every scale instead of shrinking a larger one.
  Change it when exe size matters more: PNG frames from 64 px up, after a Start and Explorer check.
- **Notice header picture** `assets/wakeshot-notice.png`: the flat drawing at 48 px (1,798 bytes), shipped beside the
  tray exe. Why: the shell shows it at about 16 px, and the lit drawing blurred there. Change it when a screen session
  shows the header soft or wrong.
- **No letters in the icon** (Microsoft: "Icons should not include typography"); Windows shows the name beside it.
- **No installer splash.** Velopack's `Setup.exe` shows its own window with words and a progress bar (its 1.2.161
  source); a splash would replace it with a picture that has no words. Change it when a screen session shows that
  window blank or confusing.

## Wordmark, lockup and social card

- **Instrument Sans SemiBold**, tracking -0.01em, version 1.000 from the designer's repo (SIL Open Font License 1.1,
  no Reserved Font Name), self-hosted as WOFF2 in `site/fonts/` and `media/public/fonts/` with `OFL.txt`. Used for the
  name in the lockup, the page hero, the social card, the video and any later Mac or store picture; Instrument Sans
  Regular for the other words of the social card and the video. Not used in the Windows app (its UI is the system's
  Segoe UI Variable, like Windows' own apps), in the page's body text (the system font stack) or in the app icon. Why:
  a free font can go into pictures, the video, a web font and a Mac app, while Segoe UI Variable may not be used
  outside Microsoft products or turned into a web font (Microsoft's font FAQ, `docs/research/brand-identity.md`
  section 7); it is compact and calm, and not the face every other tech brand uses. Change it when the lockup looks
  cramped next to the icon at social card size.
- **The lockup**: the app icon beside "Wakeshot", the word's cap height 0.46 of the plate and centred on it, 0.26 of
  the plate between them, the word `#1A1D17` on light and `#EEF2E8` on dark. `site/wakeshot-lockup-light.svg` and
  `-dark.svg` (304 x 80 px) carry the letters as outlines, so any viewer draws them. The README header is the lockup
  in a centred `<h1>`, one `<picture>` with alt "Wakeshot"; the page hero is the same lockup in live text, a row, not
  the icon over the name. Change it when the lockup sits beside other apps' lockups and looks small.
- **The stacked lockup**: the app icon over "Wakeshot", for 16:9 frames, where a row leaves the frame empty above and
  below; the video's title, outro and poster use it. The word's cap height 0.36 of the plate (the word set at half the
  plate's side), 0.26 of the plate from the plate's bottom to the word's cap line (the row lockup's gap), the word's
  ink centred under the plate, the same word colours. Lines under it keep the plate as their unit: 0.2 of it from a
  baseline to the next cap line. Set in `media/src/promo/scenes.tsx` (`STACKED`); the drawn frames match within a pixel.
  Change it when a frame needs the name larger than the icon allows, or the page wants a stacked form too.
- **The social card** `site/social-card.png`: 1280 x 640, 285,193 bytes, on `#121610` with the warm light round the
  icon; the lockup and the README's first line, every part at least 108 px from each edge. It is the page's
  `og:image` and the repo's social preview. Why dark: a light left on at night, and it stands out in a white feed.
  Change it when a dark feed shows it as a dark block.

## Page and media

- The page uses Moss in both themes (colours above), the warm light glowing round the hero's icon, the step numbers as
  the light on a Moss disc, one interactive colour per theme. No request leaves the page.
- The README loop (`media/out/icon-loop-dark.gif`, `-light.gif`): the tray glyph at 128 px going idle, holding,
  idle; the light comes on in 0.44 s and goes off in 0.8 s, in place; 5 s, on GitHub's page colours, so it shows no
  box. The video's title opens on the app icon unlit, and its light comes on.

## Mac (later work)

The rules a Mac version must meet (`docs/research/brand-macos.md` section 5; round 2's mockup in
`docs/research/brand-directions-2.md`, "The Mac"):

- **One layered Icon Composer `.icon`**, made once on a Mac (macOS 26.4 or later), three layers back to front: the
  background, the bar, the light (the layer the system lifts as glass). Square and full bleed, no drawn corners
  (macOS masks them); centred, with air at the edges; flat fills with clear edges, no shadow, glow, bevel or blur of
  our own (the system adds them); bold, no hairlines, since one vector is scaled to 16 px with no per-size drawing.
- **It reads with no colour**: in the clear and tinted looks only the shape and two or three greys remain, and the
  light is the white element in mono. The same picture in every appearance; the dark look keeps a Moss background,
  a darker shade at most.
- **Same mark, each platform's finish**: glass on the Mac, the flat plate on Windows; never glass on Windows.
- **Menu bar glyph**: a one-colour template image (the system tints it), so on the Mac the holding state shows by
  shape alone; the three Windows states, about 16 pt in a 22 pt box, strokes as heavy as Apple's own extras (both
  estimates, not checked on a Mac).
- **Menu**: the native menu, no brand colour, no item icons (macOS 27 hides them); the same words and order as on
  Windows.
- **Source**: `art.rs` should write each layer as an SVG on a 1024 px canvas, numbered back to front.

## Where each asset is made

| Asset | Made by |
|---|---|
| Tray glyphs | `art.rs` `tray_mask` and `colorize`, at run time, at the size the taskbar asks |
| `assets/wakeshot.ico`, `assets/wakeshot-notice.png`, `site/favicon.ico` (16, 32, 48), `site/wakeshot.svg`, `site/wakeshot-lockup-light.svg`, `-dark.svg`, the review sheets `docs/research/img/ux/icon-sheet-*.png` and `icon-app.png` | `art.rs`: `cargo test --locked -p wakeshot-windows tray::art::write_art_files -- --ignored`; golden tests hold each file equal to the art. The three glyph sheets are what `media/scripts/check-art.mjs` holds the media's tray glyphs to. The SVGs are the 256 px frame as shapes, from the same layout and constants; the lockups add the word's outlines, kept in `art.rs` as fixed path data that changes only with the font |
| `site/social-card.png` | `python site/social_card.py`, from `site/social-card.html` (which shows the dark lockup): run it again after an art change |
| `media/out/icon-loop-*.gif`, the video and its poster | `npm run loop` and `npm run draft` in `media/`, drawn from `media/src/art/heldLight.ts`, an exact port of `art.rs`: `node scripts/check-art.mjs` fails on any pixel that differs, and CI runs it (job `art-port`) |
| `site/fonts/`, `media/public/fonts/` | the designer's repo, `Instrument/instrument-sans` at commit `7fa22308`, unchanged |
