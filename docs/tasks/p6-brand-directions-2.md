# Brand directions, round 2: one sharper mark idea, colour chosen apart from the mark, icons with 2026 craft

Phase: 6 (brand review). Status: done. Needs Haik: yes, after this card: he picks. Agent: `wakeshot-worker`. Size:
about 200K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05 13:45, after looking at round 1.
Runs after: `p6-brand-directions` (round 1, done). Parallel with: any card (no code, no shipped asset).
Real laptop: no screen, no tray, no installs; offscreen renders only (the session may be locked).
Scratch folder: `<scratchpad>\p6-brand-directions-2\`; round 1's sources are in `<scratchpad>\p6-brand-directions\`
(reuse its scripts, taskbar crops, contrast script and stand-ins).

## Goal

Round 1 (`docs/research/brand-directions.md`, pictures `docs/research/img/brand/dir-*.png`) got the method right and the
look not yet. Haik asked for excellent work that follows 2026 trends; he judges with his eyes. Round 2 gives him a
choice he can make in two steps: first the mark, then the colour, each shown well enough to judge.

## The orchestrator's look at round 1 (art direction; check each against the pictures, disagree with reasons)

1. **The app icons look like 2015 flat clip-art.** A flat disc and a flat bar on a flat plate. `brand-windows.md` found
   Microsoft's own 2026 icons are free shapes with soft depth and rich gradients, and its guide prefers a transparent
   background. Next to the Start stand-ins (folder, gear), ours look out of place. The brand idea is a light that stays
   on while work runs, so the light should glow: soft light, gentle depth, a lit surface, as a Windows 11 icon would
   draw it. Not glass on Windows (`brand-macos.md`: no cross-platform app puts glass on its Windows icon).
2. **Pilot's app icon reads as a flag** (a disc on a plain field: Japan, Bangladesh, Palau), and olive with lime reads
   drab (army, olive oil) rather than calm and trusted.
3. **Pilot's idle glyph is the same dome** that `brand-review.md` says can read as a cloche or a bell.
4. **Daybreak glues two ideas** (a prompt and a sunrise); at 16 px it is noise. Braces fails 3:1 on dark Start.
5. What works and must stay: per-pixel 16 px tray glyphs; the held light's crisp holding glyph; the side-by-side test
   against the look-alikes; the contrast and colour-blind script.

## Deliverables

All in `docs/research/brand-directions-2.md` (write it as you go) and pictures `docs/research/img/brand/dir2-*.png`.

1. **Marks: two candidates.** (a) The held light, refined. Idle must not be a dome: try at least three idle forms at
   16 px (for example the unlit light as an outline above the line, a small low ember, the light half set) and pick the
   one that reads as "light off" and not as a cloche, bell or "eject". Keep the story simple: the light is on while
   work runs, off when it is done. (b) One more mark, not a sunrise (Shut the Lid owns the sunrise on the Mac), from
   `brand-identity.md` question 6 or better, held to the same 16 px test. Show both at 16, 20, 24, 32 px in the
   three states on a dark and a light taskbar beside Windows' own glyphs, and fix the vertical position
   (`brand-review.md`: today's glyph sits about 2 px lower than Windows' own).
2. **Colour: three palettes, chosen apart from the mark.** Each is shown on the same mark so only the colour changes.
   Not violet or a violet-blue gradient (AI purple; Shut the Lid is blue-indigo), not orange or near Claude's
   `#D97757`, not Windows default blue. Round 1's olive may stay as one of the three only if it can be made to look
   calm and warm rather than drab. Each palette passes the checks of `brand-identity.md` question 8 (contrast script
   numbers, colour-blind worst case) and names what it feels like in three words.
3. **App icon with 2026 craft, Windows.** For the stronger mark in the best palette and one other palette: a plated
   and an unplated (free shape) version, at 256 px and at true 16, 24, 32, 48 px, next to Windows 11 style stand-ins in
   a Start grid and on the taskbar, light and dark. Soft depth, a lit light, a gentle gradient; still crisp and simple at
   16 px (the small frames may be simpler drawings of the same mark, as Microsoft's own icons do).
4. **App icon, Mac.** The same mark as a layered Liquid Glass mockup (default, dark, clear, tinted), as round 1 did,
   with the lit light as its own layer.
5. **One comparison board** (`dir2-compare.png`): both marks, the three palettes, the plated and free icons, at the
   sizes that decide it, light and dark, beside today's icon and the look-alike stand-ins.
6. **HTML board**: `<scratch>\board\index.html`, offline, self-contained or with pictures beside it, showing round 2
   first, and round 1's comparison board at the end for reference. Short captions in plain words; Haik is not a
   designer.
7. **Recommendation**: your pick of mark, palette, plated or free; the runner-up; Haik's decisions as a numbered list
   with defaults; what applying the pick changes (round 1's list, updated).

## Acceptance checks

- Each 16 px glyph and each true-size icon looked at by you at 100% zoom; the Log says what you fixed.
- Contrast and colour-blind numbers from the script for every palette and both icon versions.
- The HTML board opens offline and shows every picture.
- Every one of the five points above is answered (done, or disagreed with a reason).

## Out of scope

Code, shipped assets, `docs/brand.md`, `docs/research.md`, `docs/plan.md`, phase files, round 1's files. No art copied
from other products; stand-ins only.

## Questions for Haik

Collect them in deliverable 7.

## Found outside scope

- Round 1's Windows 11 stand-ins (`pages.py` `win_standins`, reused here) are drawn smaller inside their box than
  Microsoft's real icons, so in the Start rows our plated icon looks a little heavier than it would beside the real
  ones. Not fixed (round 1's files); a real Start screenshot in the apply card settles it.

## Log

- 2026-11-05 13:45: card written by the orchestrator.
- 2026-11-05: worker started; reading round 1 report, its tools and pictures.
- 2026-11-05 14:00: tools in scratch `tools\` (marks2, icons2, pal2, checks2, glyphs2). Idle trials at 16 px looked
  at 100% and x8: ring picked; ember too weak (a period), half set and dome read as cloche or eject, ring on the line
  merges into a "Q", dashed ring is noise. Second mark trials: lamp kept; lantern read as a padlock, beacon as a
  joystick or pin, light between two bars as a divide sign. Fixed: off slash kept inside the glyph box (T7); lamp
  outline at 16 and 20 px redrawn as a pixel staircase (the anti-aliased slope was faint grey); vertical position set
  so the ink's mass centre sits at 0.455 of the box (measured neighbours on the real 150% taskbar).
- 2026-11-05 14:15: app icons v1 to v3 looked at 256 and at true 16 to 64. Fixed: tennis-ball look of the free light
  (wide green rim, now a narrow rim); plated light read as a ball (now emissive, near-white with a bloom); ground band
  made the plate a sunrise landscape (removed, a soft vignette instead); half rule failed for moss and graphite plated
  (plate deepened, glow kept modest below 64 px). Free icons fail the half rule on light Start in every palette.
- 2026-11-05 14:40: pages shot and looked at. Fixed: pages cut off at the bottom (heights); Mac outline was a pillow
  (now a superellipse); lamp's light cone a brown block on dark Start (now a soft glow); Rose's peach light 16 dE00
  from Claude's orange (now paler); Moss and Graphite light edge 2.98 and 2.74 on the plate's light end (light paler,
  plates one step deeper; now every palette passes every check). Vertical position measured on the rendered
  taskbar: within about 1 px of Windows' glyphs at every scale.
- 2026-11-05 14:55: report, HTML board (opened offline in Edge, all nine pictures load) and card result written.

## Result

Done 2026-11-05. Report `docs/research/brand-directions-2.md`; pictures `docs/research/img/brand/dir2-marks.png`,
`dir2-tray-held.png`, `dir2-tray-lamp.png`, `dir2-palettes.png`, `dir2-icon-moss.png`, `dir2-icon-rose.png`,
`dir2-mac.png`, `dir2-compare.png`; HTML board `<scratchpad>\p6-brand-directions-2\board\index.html` (pictures in
`board\img\`, round 1's board at the end). Tools in the same scratch folder, `tools\`.

- Pick: the held light (filled light over a line; idle the same light as an outline; off struck inside its box), in
  Moss (`#628C2B` to `#3E7C3B`, warm light `#FFEA9C`), plated on Windows. Runner-up: the same mark in Rose. Second
  mark: a table lamp.
- Deliverables 1 to 7: all done. Six idle forms and four second-mark trials drawn at 16 px; both marks at 16, 20, 24,
  32 px, three states, both taskbars, real 150% crop; vertical position measured within about 1 px of Windows'
  glyphs. Three palettes on one mark; plated and free icons at 256 and true 16 to 48 px in two palettes; Mac
  layered mockup; comparison board; HTML board; recommendation with decisions.
- Checks: every palette passes C1 to C13 (tightest 3.0 and 3.1); half rule at 48 px: plated 62 to 67% light Start, 79
  to 88% dark; free 24 to 26% light Start (fails). States differ 36 to 54% (held light), 26 to 50% (lamp). Board
  opened offline in Edge: all nine pictures load. Every 16 px glyph and true-size icon looked at at 100%; fixes in
  the Log.
- Disagreement with the art direction: point 1, free shape. The free version fails Microsoft's half rule on light
  Start in every palette; recommended plated, with the glow and depth asked for.

Questions for Haik (defaults): 1. mark: held light; 2. colour: Moss; 3. plated; 4. brand colour in the tray while
holding; 5. a five-minute 16 px check with two or three people first: yes.

Open items: no live screen test (offline renders only); the Mac looks are a page mockup, not Icon Composer.
