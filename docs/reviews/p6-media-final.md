# Review: the final promo video (p6-media, round 4)

Reviewer: `wakeshot-reviewer`, fresh eyes, 2026-11-06.

Reviewed: `media/out/wakeshot-promo.mp4` (7,096,189 bytes) and `media/out/wakeshot-promo-poster.png`. Judged against
`docs/research/promo-video.md`, `docs/product.md`, `docs/research.md` "Speed numbers" and Haik's notes on earlier
drafts. Nothing was rendered again; no file outside this review was changed.

**Counts: 0 blockers, 1 major, 10 minors.**

## How it was checked

- All 900 frames decoded with Remotion's bundled ffmpeg: at full size (grey) for frame-to-frame differences, cuts,
  odd frames and still stretches; at quarter size for six contact sheets of my own (every 6 frames); about 30 frames
  and crops looked at in full size. Times below are seconds and frames (30 fps).
- Audio decoded to WAV: level every 0.25 s, `loudnorm` and `silencedetect`.
- Event density per 2 s window counted from `media/src/timeline.ts` (read only, my own script).
- I cannot play the video in real time or listen to it. Anything that needs an eye at speed or an ear says so.

## Major

### M1. The lid shot is again the densest stretch: Haik's "a little fast" note is at risk (10.5 to 13.5 s)

- **Where:** 10.5 to 13.5 s (frames 315 to 405), `src/timeline.ts` `T.lid` (`close`, `shut`, `work`, `workEvery`,
  `lock`, `done`).
- **What is wrong:** round 2 closed Haik's note by spreading the densest stretches, measured as "6 events in 2 s",
  the worst being the lid shot ("lid cut, close, shut, lock and two lines within 1.5 s"). Round 4 rebuilt that same
  shot and filled it again: the lid closes and "Close the lid." starts (10.5), the lid shuts with the chime (11.0),
  the work chip rises (11.5), the lock badge, "Work goes on, locked." and the next chip step (12.0), then a new chip
  line every beat (12.5, 13.0) and "Done. All 128 tests pass." (13.5), the slide to the card (14.0), the card and
  "Work done. Your settings, back." (14.5). That is 6 non-streamed events in 1.5 s (frames 315 to 360), the same as
  the stretch Haik called fast, plus 4 lines of mono text (13 words) in 2 s. By my count 10.5 to 12.5 s has 8 new
  things, the most of any 2 s window before the end card. The pace check has no upper limit, so it stays green.
- **Why it matters:** the README plays the video muted. In 4 s a viewer must read two captions, follow a chip that
  changes every 0.5 s, then a third caption and a card. Haik's note is meant to stay done. Not judged at real speed
  by me: this needs Haik's eye, but by the measure used to close his note it is back.
- **Fix:** keep the motion, cut the reading. Change the chip's step once a bar, not every beat (two steps, for example
  "run npm test" then the done line), and start the chip with the lock line at 12.0 s instead of a beat before it.
  Optional: pop the lock badge with the chime at 11.0 s (Wakeshot locks at the lid close, so it is also closer to the
  truth), which leaves 12.0 s for the second caption and the chip only. The 2 s gap rule still holds.

## Minor

### m1. The terminal in the tray shot is 19 px, under the spec's 30 to 40 px (5.0 to 10.0 s)

- **Where:** frames 150 to 299; `src/promo/world.ts` `TRAY_TERMINAL.scale: 0.56` over `Terminal.tsx` `FONT = 34`.
- **What is wrong:** the terminal text is 19 px, the title bar text about 12 px. `promo-video.md` "Text": "Terminal
  text: JetBrains Mono 30 to 40 px". At the spec's own check width (900 px) the typed `claude` is about 9 px.
- **Why it matters:** this terminal is the cause in the cause-and-effect shot Haik asked for. The caption and the real
  tooltip still say "Claude Code", so the meaning survives, but the cause itself cannot be read.
- **Fix:** the terminal cannot get wider (see m5: after the drift its left edge is 30 px from the frame edge, and the
  capture starts at x 641). Show fewer, larger lines in a window about 600 px wide: the prompt with `claude`, then one
  status line (Working, then the done line), at 30 px. The streamed tool lines can stay in the hook and the laptop.

### m2. Retouched patches show on the real captures (5 to 10 s and 20 to 26 s)

- **Where:** tray set, all frames 160 to 299: taskbar left of the `^` (video x 735 to 1065, y 625 to 810, before the
  drift). Menu set, frames 600 to 779: taskbar left (x 625 to 900) and right of the battery (x 1415 to 1595). Battery
  patch in frames 600 to 614 and 720 to 779.
- **What is wrong:** where TrafficMonitor and the clock were covered, the fill has horizontal streaks and no acrylic
  grain, with a clear edge where the real taskbar starts (seen in `public/shots/work-S1-dark.png` itself, columns 0 to
  165). The battery box copied from S5a breaks the taskbar's top border line for about 100 px: on row 698 the border
  reads 68 outside the patch and 29 inside it (frames 600, 725, 760, 779). With the menu open (S5a to S5c) the menu
  hides the gap; with it closed (S1, S5d) it shows, right next to the ringed icon at 24 to 26 s.
- **Why it matters:** "real captures carry the proof". A visible paste on a real capture invites doubt about the rest.
- **Fix:** take the battery patch only from below the border row (or copy the border row from the frame it lands on);
  fill the covered areas with a plain sample of the real taskbar texture (a tiled crop of an empty part), or crop the
  sets so they start right of the covered areas.

### m3. The whip lands as two icons, and the flying glyph is a scaled bitmap (5.2 to 5.4 s)

- **Where:** frames 156 to 159; `FlyingGlyph` in `src/promo/Promo.tsx` (`transform: scale(${p.s})`).
- **What is wrong:** the flying glyph is sharp and lands 50, 20, 10 and 5 px off the capture's own idle icon, which is
  already visible, so two idle glyphs show for 4 frames. The glyph is the hero bitmap shrunk by CSS `scale`, which
  `media/README.md` decision 7 rules out ("draw the port at each size, never scale a bitmap").
- **Why it matters:** small at speed, but it is the one moment the brand glyph meets the real tray.
- **Fix:** hide the capture's icon box until the glyph lands (frame 162), and fade the flying glyph out over its last
  3 frames, or draw it at whole sizes from the port as decision 7 says.

### m4. The laptop is cut by the left edge in the settings framing (14.3 to 17.0 s)

- **Where:** frames about 426 to 509; `PLACES.card` in `src/timeline.ts` (round 4 decision 3 moved it 150 px).
- **What is wrong:** the laptop's left side runs off the frame (column 2 is laptop on rows 562 to 610 at frames 430 to
  460). Decision 3 says the badge, the chip and the card fit "without a cut edge", but the laptop does not.
- **Why it matters:** the closed laptop is the subject of Haik's note ("show the closed laptop working"); cut, it
  reads as a framing slip.
- **Fix:** zoom the card framing out slightly (drawn parts only, so 0.92 is allowed), or move the card 100 px closer
  to the laptop in the world.

### m5. The tray terminal overlaps the tray capture's corner (5.0 to 10.0 s)

- **Where:** frames 162 to 299: the terminal's lower right (x up to 847, y up to 368 at frame 200; 757, 350 after the
  drift) sits over the capture's upper left (732, 330; 641, 310).
- **What is wrong:** two floating panels collide by about 115 x 38 px, shadow over picture.
- **Why it matters:** looks cramped in the shot that carries Haik's note; not a truth issue.
- **Fix:** make the terminal narrower so it ends left of the capture (about 600 px wide at this scale; its text ends
  near x 655 today, so the right part of the window is empty). Moving it left does not work: after the drift its
  left edge is at x 30.

### m6. "Work goes on, locked." stays while the chip says "Done" (13.5 to 14.0 s)

- **Where:** frames 405 to 420.
- **What is wrong:** for half a second the caption says the work goes on while the chip says it is done, and the edge
  light's 24 frame fade (405 to 429) runs mostly under the camera move at 420, so "the hold ends" is easy to miss.
- **Why it matters:** Haik's note asks for the work "clearly done".
- **Fix:** end the lid caption block at 405 with a fade (or start the slide at 405 and the card a beat later), so the
  done line and the light going out have the frame to themselves.

### m7. The video is encoded with the BT.601 matrix and carries no colour tags (whole video; effect unverified)

- **Where:** `scripts/build.mjs` `VIDEO_FLAGS` has no `--color-space`; Remotion 4.0.540's default is `default`
  (BT.601, untagged; `node_modules/@remotion/renderer/dist/options/color-space.js`). ffprobe: `color_space`,
  `color_primaries`, `color_transfer` all `unknown`.
- **What is wrong:** decoding frame 120 by hand, the holding green (source `#B6E05A`, 182 224 90) comes back as 181 224
  89 with the BT.601 matrix, and 179 212 84 with BT.709. A player that assumes BT.709 for untagged HD shows the brand
  green and the light about 12 levels duller. Which players do that (GitHub's, Chrome, Safari) is not verified.
- **Why it matters:** the brand colour is the video's one accent.
- **Fix:** add `--color-space=bt709` to `VIDEO_FLAGS` (Remotion's recommended value, and its v5 default), render once,
  run `check-rebuild` and compare the green in a browser.

### m8. The end card's small lines are small and dense (26.5 to 30.0 s)

- **Where:** frames 795 to 899; `STYLE.line` 44 px, `STYLE.small` 34 px, the measured note 24 px (`Promo.tsx`).
- **What is wrong:** the speed line is 34 px and 12 words, the build and date line 24 px. At 900 px wide they are about
  16 px and 11 px; on a phone they cannot be read. The spec's own text rules (2 to 6 words, at most two lines) are
  broken here, though the storyboard row 8 asks for these lines.
- **Why it matters:** the measured numbers are one of the README's main claims; this is the only place the video
  shows them.
- **Fix:** shorter and larger: "4.3 MB of memory idle." and "109 ms of CPU in 10 minutes." on two lines at 44 px, or
  one line "4.3 MB idle, 109 ms CPU per 10 min" at 44 px; the build and date line at 28 px. Keep the numbers as
  measured.

### m9. "while agents work" says more than v1 does (2.5 to 5.0 s, and the poster)

- **Where:** frames 84 to 156, `COPY.tagline` in `src/copy.ts`; also the poster (frame 120).
- **What is wrong:** v1 detects Claude Code only (`docs/product.md`: other agents after v1). "agents" in the plural
  reads as any agent; a Codex or Copilot user would expect it to work. The next shot names Claude Code, so the video as
  a whole is honest, but the poster alone is not.
- **Why it matters:** the poster is what Pages shows before play; the truth rule is "every claim is something v1 does".
- **Fix (Haik's call, words are drafts):** "keeps it awake while Claude Code works." or "keeps it awake while your
  agent works." The README wording should match.

### m10. Small doc and poster points

- `media/README.md` decision 1 says "true peak about -1.5 dBTP"; the final measures -1.23 dBTP (`loudnorm`), and the
  AAC file's sample peaks reach -1.6 dBFS (10.0 s), not under -2. Fix: say "-1.2 dBTP after AAC".
- The poster (frame 120, equal to the video's frame 120 within encoding noise) shows the faint terminal crisp enough
  to read (`src/billing/webhooks.ts`, `docs/api-v2.md`) straight through "Wakeshot" and the tagline. Fix: more blur on
  the faint terminal (`soft` 6 to 8 instead of 3) or lower its opacity under the name.

## Checked and fine

- Format: H.264 High, yuv420p, 1920 x 1080, 30 fps, 900 frames, 30.0 s; AAC LC 48 kHz stereo 192 kbit/s; 7.10 MB
  (under 8 MB and GitHub's 10 MB); `moov` before `mdat`, so it starts playing before the whole file loads.
- No DRAFT mark on any frame. Frame 0 is a full picture ("Agent working." and the tilted terminal). The name is in by
  2.8 s. The end card runs the last 4 s and is still for the last 2.3 s (frames 830 to 899).
- Cuts at 60, 300, 510, 600 and 780 are clean: one large change, no blended or neighbour frame. No odd frame anywhere
  (no frame that differs from both neighbours while they match), except frame 334, the turning point of the lid's
  2 degree settle (`lidShut` bounce), which is meant. The S1 to S2 swap at 165 and the S3 to S4 swap at 240 are clean
  (round 3's neighbour frame is gone).
- Still stretches: at most 70 frames, all planned (after "Done: it lets go.", after the moon, after "No second tool.",
  the end card).
- Text on screen matches `src/copy.ts`; no em dash or emoji; captions at 60 px on the 928 px baseline; one emphasis
  word per line at most. Menu and tooltip words come only from the captures.
- Truth: the tooltip reads "Holding awake: Claude Code (1 session)" and "Not holding, the PC sleeps as usual"; the
  settings card shows Lid close Do nothing to Sleep and Battery sleep Never to 5 hours (`data/settings.json`, Haik's
  own values); lock at lid close, work end, settings back, then "Later, it sleeps as usual." with the going-to-sleep
  chime all match `docs/product.md`. No countdown, no clock, no timing claim. ARM64 is built by `release.yml`.
- Speed numbers equal `docs/research.md` "Speed numbers": "Light: 4.3 MB of memory idle, 109 ms of CPU in 10 minutes."
  and "Measured on Wakeshot 0.1.0, 2026-11-06, x64."
- Privacy: the terminal path is `C:\code\shop`; no user name, no clock, no other app's name in any capture; the tray
  icons are Windows parts (OneDrive cloud, language, network, volume, battery).
- Audio: -15.39 LUFS integrated, -1.23 dBTP true peak (targets -14 to -16 and -1 or lower); sound from frame 0;
  silent 1.86 to 2.00 s (the planned gap before the light); the end card's music falls from -18.6 dB RMS at 27.5 s to
  -76 dB at 29.75 s and is silent from 29.74 s; after 26 s the only effect is the soft swell at 26.0 s, no sweep,
  whoosh or pops. Music file per `data/music.json`: "A Chill Fever (Loopable)", not checked by ear.
- Haik's notes: music 3 (by config), the ending sounds (by level), the laptop quality (graphite, keys, touchpad, edge
  light; looked at full size), Claude Code turns the real icon green and back (real S1 to S4 at 2x), and the 00:56
  note (work behind the lid, done, then sleep) are all in. "A little fast" is M1.
- Poster: equal to the video's frame 120 (mean difference 0.5 levels).

## Fix check (2026-11-06)

Checked the re-rendered `media/out/wakeshot-promo.mp4` (7,137,594 bytes) against the worker's "Fix round (review)"
in `docs/tasks/p6-media.md`. Scope as asked: M1, plus a look for new blockers or majors in the changed parts. No
render, no git.

- **M1: fixed.** The lid shot now runs: cut (10.0), the lid closes with one caption "Close the lid: work goes on."
  (10.5), the lid shuts with the chime and the lock badge (11.0), the work chip rises with one step, "run npm test"
  (12.0), its done line (13.5). From 10.5 to 13.5 s (frames 315 to 405) the timeline now has 6 events and caption
  starts, where round 4 had 10; my count gives at most 5 in any 2 s there (was 8). One 6-word caption replaces two, and the chip no longer changes every beat. The
  caption fades at 13.5 s, as the chip says done. Seen on a sheet of every 6th frame (300 to 444) and in full-size
  frame 370.
- **Colour (BT.601 to BT.709):** ffprobe reads `bt709` for matrix, primaries and transfer. Frame 120 decoded with
  the BT.709 matrix gives the light's middle 182 224 89 against the source `#B6E05A` (182 224 90). The stage is
  unchanged (17 22 17). No new issue.
- **Tray shot (5 to 10 s):** the terminal reads at 30 px and ends left of the capture (frame 200). The taskbar
  covers show no streaks or edge at full size. The S1 to S2 swap (165) and the S3 to S4 swap (240) are clean. No new
  issue.
- **Settings framing:** at zoom 0.92 the laptop clears the left edge (frames 430 to 500).
- **Frame 151:** the small glyph sits at the bottom edge for one frame, inside a 12-frame blurred whip. Not a major.
  Also seen, and not a major: the name and tagline fade over the incoming terminal and tray in frames 152 to 156.

Result: M1 is closed, and I found no new blocker or major in the changed parts. The minors were not re-checked one by
one, as asked.
