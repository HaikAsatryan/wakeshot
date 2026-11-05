# Apply the new brand to media/: colours, font, the art port, the icon loop, the poster

Phase: 6 (brand apply). Status: done (Parts A and B, 2026-11-05). Needs Haik: no (his 8
brand decisions, 2026-11-05 ~15:00, all defaults).
Agent: `wakeshot-worker`. Size: about 150K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: Part A: Haik's brand decisions. Part B: `p6-brand-apply-art` done (the orchestrator resumes you, or a new
worker reads this card's Log).
Parallel with: `p6-brand-apply-art` (Part A only), `p6-brand-apply-web`. No cargo builds in this card.
Real laptop: no power settings, no screen, no installs. Offscreen renders only (Remotion and headless Chrome or Edge,
one render at a time).
Scratch folder: `<scratchpad>\p6-brand-apply-media\`.
Owns: `media/` (all of it: `src/`, `scripts/`, `public/`, `out/`, `README.md`, `CREDITS.md`, `package.json` and its
lock only if a font package must change). Additions to `docs/research.md`.

## Goal

`media/` still draws the Sunrise in violet and gold with Inter, and its icon loop has a brown haze, a helmet frame and
boxes that do not match GitHub (L1 to L4). Bring it onto the held light in Moss with Instrument Sans, ported exactly
from `art.rs`, so `p6-media` Part B only adds the real captures. Quality bar: a senior motion designer would ship it.
Render, look at frames, fix, look again.

## Read first

- `docs/research/brand-directions-2.md`: "The answer in short", "Step 2: the colour" (Moss column), "What applying the
  pick changes" (`media/` row: the loop becomes the light fading on and off, no rising).
- `docs/phases/06-ship.md` work log, line "2026-11-05 ~15:00": Haik's decisions.
- `docs/research/brand-review.md`: L1 to L4, X1, X2, B2.
- `docs/tasks/p6-media.md`: the Goal, Part A's Result and Part B's list (what Part B still needs from you).
- `media/README.md`, `media/src/theme.ts`, `fonts.ts`, `art/sunrise.ts`, `art/Glyph.tsx`, `loop/IconLoop.tsx`,
  `media/scripts/check-art.mjs`, `check-rebuild.mjs`; the rest of `media/src/` as needed.
- Round 2 sources in `<scratchpad>\brand-src\p6-brand-directions-2\tools\` (`pal2.py`, `checks2.py`, `marks2.py`).

## Deliverables

### Part A (now; nothing here waits for the art)

1. **Colours**: `theme.ts` and every other colour in `media/src` to the Moss set (plate `#628C2B` to `#3E7C3B`, light
   core `#FFFBE8`, edge `#FFEA9C`, holding `#B6E05A` on dark and `#3B7A2A` on light, pages `#FBFCF7` and `#121610`).
   Text pairs WCAG AA in both themes. Nothing violet or gold stays.
2. **Font**: Inter out, Instrument Sans in (`fonts.ts`), self-hosted from its official source with `OFL.txt`; SemiBold
   for the wordmark, the same family for the rest of the video text. Remove Inter's files and its `CREDITS.md` lines.
   No font fetched from the network at render time.
3. **The loop's design (L1 to L4)**: the held light going idle, holding, idle: the light fades on and off in place
   (no rising, no half-way dome), holding in the brand colour as the tray draws it, no glow haze on either theme. The
   background is GitHub's own page colour so the GIF has no visible box: check GitHub's current light and dark page
   colours from GitHub's own CSS (Primer) and cite them; if GitHub's dark has more than one variant (dark, dimmed,
   high contrast), pick the default and say so. 4 to 6 s, seamless, under 1 MB each.
4. **Stale stand-ins out**: delete every file in `media/public/shots/` that shows the cup or the old icon, and any
   other old-brand output in `media/out/` (the draft video and poster). Make the draft render show a neutral
   placeholder box naming the missing shot (for example "S5: menu with Keep awake open") where a stand-in was, so the
   pipeline still runs and nothing old is drawn. The final render keeps refusing placeholders.

### Part B (after `p6-brand-apply-art`)

5. **The art port**: replace `src/art/sunrise.ts` with a port of the new `art.rs` (tray glyph three states and both
   parts of the holding colour; the app icon if the media uses it), named for what it is, not the Sunrise. Update
   `Glyph.tsx`, `IconLoop.tsx` and every import. `check-art.mjs`: the port against the review sheets and
   `assets/wakeshot.ico`, 0 pixel difference as today (or an exact, written tolerance only where `art.rs` itself
   blurs, with the reason). `check-rebuild.mjs`: its input list matches the new files.
6. **Render**: `media/out/icon-loop-dark.gif` and `-light.gif` again; the draft video and its poster again with the new
   brand (placeholders for missing shots). Look at the loop's key frames enlarged and at 100% on GitHub's colours, the
   poster at full size. Fix, look again. Keep 2 or 3 pictures as `docs/research/img/brand/apply-media-*.png`.
7. **`media/README.md`**: the new art, font, colours and loop; no Sunrise, Inter, violet or gold left in it.

## Acceptance checks

- `npm run` the media checks that exist today (`check-art`, `check-rebuild`, type check, lint if any) pass.
- No `#7250F4`, `#7957F4`, `#6B49F3`, `#5B3FD6`, `#A996FF`, `#FFCC4D`, `#14121F`, "sunrise", "Sunrise", "Inter" or
  `EC32` left in `media/` outside `node_modules` (grep, case-insensitive where it makes sense).
- Each GIF: size, length, frame count and the background colour checked by reading the file, not by eye.
- The final render still refuses to run with a placeholder.

## Out of scope

`p6-media` Part B (real captures, speed numbers, the final video, Haik's look); the README; `site/`; the Rust code;
`docs/brand.md` and other docs. Do not run git.

## Questions for Haik

None expected. Design calls inside the brief: decide and list them under "Decisions for Haik's review".

## Decisions for Haik's review

Made by the worker as principal designer; reasons and "change it when" triggers also in `media/README.md`.

1. **Our own GIF writer for the loop** (`media/scripts/lib/gif.mjs`), not Remotion's GIF codec: Remotion's one
   palette for the whole clip moved the old background by a level, so the GIF would sit in a faint box. Ours keeps
   every colour exact (checked from the file). Change it when the README loop moves to a video format, or Remotion
   gets an exact-palette GIF mode.
2. **The loop's motion**: the outline takes the light colour in the first 35% of the fade and the inside brightens
   over all of it (one dim light, not a rim round a dark button); on in 0.44 s ease out (a lamp coming on), off in
   0.8 s ease in and out; idle 1.6 s, holding about 2.2 s; colours mixed in OKLab. Change it when Haik finds it slow
   or fast.
3. **Loop backgrounds**: GitHub's default `light` `#FFFFFF` and `dark` `#0D1117`. Dark dimmed (`#212830`) and dark
   high contrast (`#010409`) show the dark GIF as a slightly different dark box; a README `<picture>` can only choose
   by light or dark. Change it when GitHub changes its defaults (`check-video` then fails).
4. **Loop canvas kept at 480 x 240** with the glyph drawn at 128 px (64 px when shown 240 px wide). Change it when
   the README layout (`p6-media` Part B) wants another size.
5. **Video accent is the holding green `#B6E05A`** (working line, held values, speed numbers, the wipe line, the
   install line): green means "on" in this brand. The warm light edge `#FFEA9C` is kept for the light itself (the
   laptop's lit screen strip now, the app icon's light in Part B). Change it when the green competes with the real
   tray captures in the same frame.
6. **Instrument Sans for every word of the video**, Regular and SemiBold, self-hosted in `media/public/fonts/` from
   the designer's repo (byte-equal to `site/fonts/`); JetBrains Mono only in the drawn terminal and the install
   command, where a code face is expected. Change it when the install line becomes plain words.
7. **The video stays dark**; the only light colours (`LIGHT` in `theme.ts`) are for the placeholders of light
   captures, so the light/dark wipe still reads in a draft. Both sets' text pairs are held to 4.5:1 by `npm run check`.
8. **Placeholders**: a plain panel box with a dashed edge, "S5: menu with Keep awake open (light)" over "no capture
   yet", light or dark per shot; the draft banner now says "DRAFT: captures and speed numbers missing". The final
   render refuses twice: `build.mjs` before rendering ("has no capture yet") and the `Shot` component throws outside
   a draft.
9. **(Part B) The port is exact, no tolerance**: every f32 step of `art.rs` rounded with `Math.fround` in Rust's
   order, so `check-art` allows 0 difference (the old port allowed 8 levels). Change it when `art.rs` takes a
   function JavaScript cannot match bit for bit (`sin`, `exp`): then a written tolerance for those pixels only.
10. **(Part B) The art is drawn from the port's pixels on a canvas**, 1:1 at whole-pixel places, never redrawn as
    SVG: the loop's ends are `tray_mask(128)` and the title and outro icons `app_icon(256)` and `app_icon(192)`, byte
    for byte. The loop glyph's ink box is centred in the frame (4 art px lower than the square's centre). Change it
    when the video needs the icon zoomed: then draw the port at each size, never scale a bitmap.
11. **(Part B) The title's light comes on**: the icon slides in unlit (plate, shade, bar as the app icon, no glow, the
    light a white outline as heavy as the bar, the tray's idle look), then at frame 20 the light comes on in 0.44 s
    (the loop's curve): the inside leads, the edge warms after, the glow spreads. The sound beat moved with it (frame
    14 to 20; `title.rise` renamed `lightOn`, the sounds `lightOn` and `lightOff`). Change it when Haik wants the title
    to open on the lit icon.
12. **(Part B) H.264 at CRF 16**, not 20: at 20 the plate's edge broke into blocks while the light came on; the draft
    is 2.5 MB of the 10 MB limit. Change it when the final video with captures passes about 8 MB.

## Found outside scope

- Root `README.md` media comments still describe the old brand: line 3 (the loop: "the sun rises over the horizon",
  its alt text, "the art is the Sunrise"), lines 59 and 60 (icon alt texts "A risen sun with rays", "An empty sun on
  the horizon"). Owner: the README's media placeholders (`p6-media` Part B or a docs sweep), not this card.
- `docs/tasks/p6-media.md` Part B list and Result still speak of 8 stand-in shots in `media/public/shots/`; they are
  now gone (this card), and every shot is a named placeholder until the capture session. Its Part B wording needs a
  touch when it runs. Its Part A Result also names `src/art/sunrise.ts` and the Sunrise `Glyph.tsx` (history; now
  `src/art/heldLight.ts`, `motion.ts`, `Glyph.tsx`).
- `check-art` has no `npm run` script (`node scripts/check-art.mjs`, as before). Adding one changes `package.json`,
  which this card may touch only for a font package; for whoever next owns it.
- Nothing wrong found in `art.rs` or `assets/`: the port matches every file it writes.

## Log

- 2026-11-05: started Part A (deliverables 1 to 4). Part B waits for `p6-brand-apply-art`.
- 2026-11-05: facts. GitHub's page colours from github.com's own theme CSS (fetched with curl from the repo page's
  links): `light` `--bgColor-default:#fff`, `dark` `#0d1117`, `dark_dimmed` `#212830`, `dark_high_contrast` `#010409`;
  a logged-out page has `data-color-mode="auto" data-light-theme="light" data-dark-theme="dark"`, so the defaults are
  light and dark. Instrument Sans: the designer's repo `Instrument/instrument-sans` at commit `7fa2230` (2023-06-14),
  the same commit Google Fonts' `ofl/instrumentsans/METADATA.pb` names; static `webfonts/` WOFF2 Regular and SemiBold,
  "Version 1.000" (name ID 5, read from the TTFs of the same commit); `OFL.txt` declares no Reserved Font Name.
- 2026-11-05: the old loop GIF's dark background is `#14111E`, not the `#14121F` it was drawn on (read with a new GIF
  reader, `media/scripts/lib/gif.mjs`): Remotion's one GIF palette (`palettegen` over the whole clip) moves colours. So
  the loop gets its own GIF writer with an exact palette per frame (design call, below).
- 2026-11-05: `theme.ts` rewritten to Moss: `MOSS`, `GITHUB`, `DARK`, and the text and graphic pairs the check holds
  to 4.5:1 and 3:1. Picked: dim `#A3AD99` (7.0 to 8.3), panel `#1B2118`, terminal `#0C0F0B`, accent `#B6E05A`.
- 2026-11-05: fonts. Instrument Sans Regular and SemiBold WOFF2 and `OFL.txt` in `media/public/fonts/` (loaded with
  `staticFile`); `fonts.ts` rewritten; `npm uninstall @fontsource/inter` (package.json and lock lose only that
  package); `CREDITS.md` font lines replaced. JetBrains Mono stays for the drawn terminal and the install command.
- 2026-11-05: video colours. `parts.tsx`, `scenes.tsx`, `Promo.tsx` take `DARK` and `MOSS` tokens: text `#EEF2E8`,
  dim `#A3AD99`, accent `#B6E05A` (working line, held values, speed numbers, split line, install line), laptop in
  dim and panel with the screen strip in the light edge `#FFEA9C`, padlock in text colour, moon in dim; background
  `#1A2117` to `#121610`. App icon plate in the Moss ramp (shape stays the old one until Part B's port).
- 2026-11-05: stale stand-ins out. Deleted all 7 files in `media/public/shots/` (each showed the cup glyph in the
  menu or the old tray icon; looked at each) and the folder; deleted `out/draft/wakeshot-promo-draft.mp4` and its
  poster. `data/shots.json`: every shot `file: null` with a `what` line; `Shot` draws a plain panel box "S5: menu
  with Keep awake open (light)" over "no capture yet", and throws outside the draft. `CREDITS.md` pictures section
  now says none yet.
- 2026-11-05: loop design in code: `Glyph.tsx` has `HeldLight` (provisional geometry: `marks2.py` `HELD[32]`
  scaled, until Part B's port), colours mixed in OKLab; `IconLoop.tsx` on GitHub's page colours, on at frame 25 over
  11 frames (ease out), off at 90 over 20 (ease in and out). Old Sunrise `Glyph`, the glow and `set` removed.
- 2026-11-05: GIF writer `writeGif` in `media/scripts/lib/gif.mjs` (per-frame palette of exact colours, changed box
  only with unchanged pixels transparent, repeated frames merged; over 255 colours the rarest edge colour joins its
  nearest and the move is reported). Self-tested in scratch: writer, reader and the bundled ffmpeg decode agree to
  the bit, including a 300 x 200 noise frame that fills the LZW table many times. `build.mjs loop` now renders a PNG
  sequence (`--sequence`) to a temp folder and writes the GIF with it.
- 2026-11-05: looked at render 1 (scratch `look\loop1-dark.png`): mid-fade the rim stayed near white round an olive
  inside: a pale ring round a dark button, two objects. Fix: the rim takes the light colour in the first 35% of the
  fade, the inside over all of it. Render 2 (`look\loop2.png`, `look\loop2-100.png` at 100% and at the README's
  half size): mid frames read as one dim light; holding is one solid disc, no seam; edges clean at 100%.
- 2026-11-05: `check-video.mjs` now reads each loop with `readGif`: size, 4 to 6 s, frame count, loops forever,
  under 1 MB, last frame equals first, a 40 px border of every frame exactly GitHub's colour. Both loops pass:
  5.00 s, 29 frames (125 rendered), 28 KB each, every border pixel `#0D1117` and `#FFFFFF`.
- 2026-11-05: `check.mjs` holds `TEXT_PAIRS` to 4.5:1 and `GRAPHIC_PAIRS` to 3:1 (16 pairs pass); Inter dropped from
  its font list. Draft stills at frames 75, 200, 300, 470, 700, 800, 880 rendered to scratch `look\still-*.png` and
  looked at: Instrument Sans loads (SemiBold wordmark, Regular lines), colours coherent. Fixed after looking: the draft
  banner said "stand-in pictures" (now "DRAFT: captures and speed numbers missing"); the light/dark wipe showed two
  dark boxes, so light shots now draw a light placeholder (`LIGHT` set, `theme` per shot in `shots.json`, 2 more text
  pairs checked).
- 2026-11-05: `media/README.md` updated for what Part A changed (loop row, draft row, checks, sources, fonts and
  colours rows, decisions 2 and 3); the art port lines wait for Part B. `docs/research.md`: new section "Media:
  GitHub's page colours, the video font and exact GIF colours". The media SemiBold WOFF2 is byte-equal to
  `site/fonts/` (same SHA-256 as the web card's).
- 2026-11-05: **Part A done.** Result of Part A:
  - Files changed in `media/`: `src/theme.ts`, `src/fonts.ts`, `src/copy.ts` (draft banner), `src/art/Glyph.tsx`,
    `src/loop/IconLoop.tsx`, `src/promo/parts.tsx`, `scenes.tsx`, `Promo.tsx`, `data/shots.json`,
    `scripts/build.mjs`, `scripts/check.mjs`, `scripts/check-video.mjs`, `package.json`, `package-lock.json`,
    `README.md`, `CREDITS.md`. New: `scripts/lib/gif.mjs`, `public/fonts/InstrumentSans-Regular.woff2`,
    `InstrumentSans-SemiBold.woff2`, `OFL.txt`. Deleted: the 7 files of `public/shots/` and the folder,
    `out/draft/wakeshot-promo-draft.mp4`, `out/draft/wakeshot-promo-poster-draft.png`. Rebuilt:
    `out/icon-loop-light.gif`, `out/icon-loop-dark.gif` (new design, provisional geometry). Outside `media/`:
    `docs/research.md` (one new section).
  - Checks run: `npm run typecheck` ok; `npm run check` ok (37 files, 6 assets credited, 16 colour pairs pass);
    `check-video --draft`: all 14 loop checks ok (5.00 s, 29 frames, 28,109 and 27,918 bytes, loops forever,
    seamless, every border pixel `#FFFFFF` and `#0D1117`), the draft video check fails as expected (deleted; Part B
    renders it); `build.mjs video` and `poster` refuse (exit 1, every shot "has no capture yet" and the speed
    fields); a `draft:false` still throws "shot S1 tray-dark-idle has no capture; the final video needs it"; the loop
    rebuild is byte-identical; `check-art` still passes only because `art.rs` has not rewritten its files yet.
    `check-rebuild` not run: it needs the draft, which Part B renders.
- **Notes for Part B** (for whoever continues):
  - Port the new `art.rs` to a file named for the held light (replacing `src/art/sunrise.ts`): tray glyph three
    states and both parts (light, line), the holding colours, the glyph ink colours, the app icon. Then in
    `Glyph.tsx` replace `heldLayout` (provisional, `marks2.py` `HELD[32]` scaled) with the port's geometry at 128 px;
    if `art.rs` has only per-size tables, take its largest size scaled and say so. `TRAY_SIZES` in `art.rs` was 16 to
    64 before the art card.
  - `HeldLight` must stay pixel-true to the port at `on` 0 and 1; keep the rim-first fade (decision 2).
  - `IconLoop.tsx` imports `GLYPH_ON_DARK`, `GLYPH_ON_LIGHT` from `sunrise.ts`: move to the port. If `art.rs` names
    the holding colours, make `theme.ts` `MOSS.holdingOn*` come from the port (one source) and add the glyph ink on
    GitHub's colours to `GRAPHIC_PAIRS`.
  - App icon: `AppIcon`, `SunriseParts`, `Phase`, `IDLE`, `HOLDING` in `Glyph.tsx` and `rise` in `scenes.tsx` are the
    old mark in Moss colours; replace with the ported plated icon, and the title's motion with the light fading on.
  - `check-art.mjs`: new sheet layout and the holding colour; `check-rebuild.mjs` `REPO_INPUTS`; `media/README.md`
    lines on the icon art (the "When something changes" icon item and the "Icon art" source row).
  - Then `npm run loop`, `npm run draft` (one render at a time), `check-video --draft`, `check-rebuild`, the legacy
    grep of the acceptance checks, and 2 or 3 pictures to `docs/research/img/brand/apply-media-*.png`. Scratch
    tools for looking: `<scratchpad>\p6-brand-apply-media\tools\sheet.mjs` (enlarged key frames),
    `sheet100.mjs` (100% and README size), `gifinfo.mjs`, `gifframes.mjs`.
- 2026-11-05: Part B started (a new worker; `p6-brand-apply-art` done). Read this card's Log, the art card's Result.
- 2026-11-05: plan. Port `art.rs` to `src/art/heldLight.ts` with every f32 step rounded by `Math.fround` in Rust's
  order (a double rounded once to f32 after each +, -, *, / or sqrt is the f32 result), so `check-art` can hold it to
  0 difference instead of the old 8. The loop and the title draw the port's own pixels on a canvas (no SVG
  redraw), with the in-between frames mixed per pixel from the two exact end pictures, so both ends are the port
  byte for byte.
- 2026-11-05: `src/art/heldLight.ts` written (tray glyph, hand rings, off slash, `colorize`, plated app icon with
  glow, bloom, shade and shadow, flat frames, `mix`, `paint`, `sheet` from art.rs's tests). `scripts/lib/png.mjs`
  reads PNG frames inside an ICO. `check-art.mjs` rewritten: all three sheets whole (dark, light, contrast; 1892 x
  1648, four rows, real size and enlarged), the 14 ICO frames and their list, the notice picture (flat, 48 px):
  9,458,740 pixels, 0 differ, 1.5 s. Control run in scratch with `F` as a no-op (f64 math): 11 app icon frames
  differ by 1 level in 4 to 186 pixels, so the f32 rounding is needed and the check sees a 1-level slip.
- 2026-11-05: media on the port. `src/art/motion.ts` (new): OKLab mixing, `fadeAt` (per pixel from one exact end
  picture to the other; the outline's share at the rim's pace, the rest at `on`), `trayFade` (idle and holding
  composed as the sheets are), `appIconUnlit` (plate, shade, shadow and bar as `app_icon`, no glow or bloom, the
  light as a white outline one bar height thick) and `appIconFade`. `Glyph.tsx` rewritten: `Pixels` (canvas, 1:1),
  `TrayGlyph`, `AppIcon` (`on` 1 is `app_icon(size)`). `IconLoop.tsx` draws `TrayGlyph` at 128 px with `ON_DARK` and
  `ON_LIGHT`, ink box centred on whole pixels. Title and outro: a `Lockup` that sets the icon's top on a whole pixel
  (the text's line heights are fixed) so the icon is never resampled at rest; the title's light comes on at frame 20
  over 13 frames (the loop's 0.44 s, ease out); beat `title.rise` renamed `lightOn` (sound follows). `theme.ts` takes
  `MOSS` from the port and holds the glyph ink on GitHub's pages to 3:1 (18 pairs pass). `sunrise.ts` deleted.
  Typecheck ok.
- 2026-11-05: `npm run loop` (18 s): 29 GIF frames each, at most 40 colours in a frame, nothing merged; 20,343 and
  21,561 bytes. `check-video` now also holds the whole frame to the port: frame 0 is the idle glyph and a 2.28 s
  stretch is the holding glyph, every pixel, both themes (pass: the canvas path through Chrome is exact). Looked at
  key frames enlarged (scratch `partb\look\on-sheet.png`, `edge-sheet.png`) and at 100% and README size
  (`loop-100.png`): edges clean, no seam between outline and inside, holding one solid disc, glyph centred.
- 2026-11-05: title stills (frames 12 to 28, 75) and outro (870). The title and outro icons are the port's
  `app_icon(256)` and `(192)`, 0 of 52,952 and 29,920 opaque pixels differ (scratch `iconexact.mjs`). Fixed after
  looking (`title-on.png` to `title-on4.png`): with the outline leading, as in the loop, the in-between frames showed
  a gold ring round a grey-green middle; the inside now leads instead (`Fade.lead`, generalised from the loop's
  `ring`), and as a step that left a seam (the lit middle's warm edge against a near-white outline, and a dark
  hairline), so the lead falls off across the outline. Now one light warming up, glow spreading over 13 frames.
- 2026-11-05: `npm run draft` (78 s): 1.9 MB video and poster. Poster looked at full size and at 100% beside video
  frame 75 (`look\poster-100.png`), outro frame 870. Found in the video: at CRF 20 the plate's top edge broke into
  blocks while the light came on (the whole plate changes at once; scratch `look\vid-22-top.png`). Title rendered at
  CRF 20 and 16 side by side (`look\crf.png`): 16 is clean, 1.5 times the bytes. `build.mjs` now uses CRF 16.
- 2026-11-05: draft again at CRF 16: 2.48 MB, `check-video --draft` all ok. Loops rebuilt: byte-identical.
  `check-rebuild.mjs`: `REPO_INPUTS` now the three sheets, the ICO, the notice picture and `chime.rs`. Run: `npm ci`
  in a temp copy, `check-art` 0 differ, loop and draft rebuilt, all 5 outputs the same bytes (2 min).
- 2026-11-05: final render still refuses: `build.mjs video` and `poster` exit 1 ("has no capture yet"), a
  `draft:false` still throws "shot S1 tray-dark-idle has no capture". Legacy grep clean. Old names out: sounds
  `rise`/`fall` now `lightOn`/`lightOff`, `.gitignore` comment, `TRAY_SIZES` (unused) dropped. `media/README.md`:
  art rows, checks, "When something changes", decisions 7 to 9. Pictures kept: `apply-media-loop.png`,
  `apply-media-title.png`, `apply-media-poster.png`. `docs/research.md`: section "Media: the art port in f32, and
  the loop and title drawn from it". Final `check-rebuild`: all 5 outputs the same bytes. **Part B done.**

## Result

Done 2026-11-05 (Part A by the first worker, see its Result in the Log; Part B below).

Part B, what changed:

- **The art port**: `media/src/art/heldLight.ts` replaces `sunrise.ts`: tray glyph in three states with both parts
  (light and line), hand rings, off slash, `colorize`, the plated app icon (ramp, glow, shade, bloom, rim, shadow,
  flat frames), and the sheets' `mix`, `paint`, `sheet`. Every f32 step rounded as Rust does, so it is exact.
- **In-between states**: `media/src/art/motion.ts`: OKLab mixing, `fadeAt` from one exact end picture to the other,
  the loop's fade (outline leads) and the title's (inside leads, falling off across the outline), the unlit icon.
- **Drawing**: `Glyph.tsx` (`Pixels` canvas, `TrayGlyph`, `AppIcon`), `IconLoop.tsx`, `scenes.tsx` (title and outro
  `Lockup` on whole pixels, the light coming on), `theme.ts` (`MOSS` from the port, glyph ink pairs), `timeline.ts`
  (`LOOP.art`, `LIGHT_ON_FRAMES`, beat `lightOn`), `make-sounds.mjs` (names), `build.mjs` (CRF 16).
- **Checks**: `check-art.mjs` (whole sheets, ICO list and frames, notice picture, 0 tolerance), `lib/png.mjs` (PNG
  frames in an ICO), `check-video.mjs` (loop ends equal the port), `check-rebuild.mjs` (inputs).
- **Rendered**: `media/out/icon-loop-dark.gif` (21,561 bytes), `icon-loop-light.gif` (20,343), the draft video
  (2.48 MB) and poster (not in git).

Files: `media/src/art/heldLight.ts` (new), `motion.ts` (new), `Glyph.tsx`, `src/loop/IconLoop.tsx`,
`src/promo/scenes.tsx`, `src/theme.ts`, `src/timeline.ts`, `scripts/check-art.mjs`, `check-video.mjs`,
`check-rebuild.mjs`, `build.mjs`, `make-sounds.mjs`, `lib/png.mjs`, `README.md`, `.gitignore`, `out/*.gif`;
deleted `src/art/sunrise.ts`. Outside `media/`: `docs/research.md` (one section), new
`docs/research/img/brand/apply-media-loop.png`, `apply-media-title.png`, `apply-media-poster.png`.

Checks (final code):

| Check | Outcome |
|---|---|
| `npm run typecheck` | pass |
| `npm run check` | pass: 40 files, 6 assets credited, 18 colour pairs |
| `node scripts/check-art.mjs` | pass: 9,458,740 pixels, 0 differ (control with f64 math: 1-level misses, so the check is sharp) |
| `npm run check-video -- --draft` | pass, 25 checks: draft H.264, AAC, 1920 x 1080, 30 fps, 30.02 s, 2.48 MB; each loop GIF89a, 480 x 240, 5.00 s in 29 frames, loops forever, 20 to 22 KB, seamless, every border pixel GitHub's colour, first frame the port's idle glyph and 2.28 s the holding glyph, every pixel |
| `npm run check-rebuild` | pass: clean `npm ci` copy, loops, sound, draft video and poster all the same bytes |
| Final render refuses placeholders | pass: `build.mjs video` and `poster` exit 1; a `draft:false` frame throws |
| Legacy grep (`#7250F4` ... `EC32`, sunrise, Inter) in `media/` | no hits |

Looked at (scratch `<scratchpad>\p6-brand-apply-media\partb\look\`): loop key frames at 2 and 5 times and at 100%
and README size; title frames 12 to 28 and the poster at full size and at 100%; outro; video frames against stills.
Changed after looking: the title's fade (warm ring round a grey-green middle, then a seam, now one light), CRF 16.

Pictures for Haik: `docs/research/img/brand/apply-media-loop.png`, `apply-media-title.png`, `apply-media-poster.png`;
the files themselves: `media/out/icon-loop-dark.gif`, `icon-loop-light.gif`, `media/out/draft/wakeshot-promo-draft.mp4`,
`wakeshot-promo-poster-draft.png`.

Open items: `p6-media` Part B (captures, speed numbers, final video); the root README's media alt texts (docs sweep).
