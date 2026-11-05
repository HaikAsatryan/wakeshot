# Speed numbers, pictures, the icon loop and the 30 second video

Phase: 6. Status: Part A done (2026-11-04: the loop, the video pipeline with stand-ins, the media docs, the speed script); Part C in progress (2026-11-05: the dynamic cut with stand-ins, `docs/research/promo-video.md`); Part B after Part C: the installed build, the screen slot with a real Claude Code session, 10 idle minutes and Haik's look. Needs Haik: yes: he watches the video and the loop and says yes or what to change; he stays
off the PC for about 10 minutes while the captures run. Agent: `wakeshot-worker`. Size: about 120K tokens.
Real laptop: yes: screen captures of the real tray during one real Claude Code test session, light and dark; the real
app holds and restores the power settings during it. One card at a time on the laptop.
Runs after: `p6-launch-research` (video tool, storyboard, shot list, specs, sound sources); the tray track's
`p5-haik-check` (menu words); `p6-brand-apply-art` and `p6-brand-apply-media` (the final icon and its port into
`media/`, both done 2026-11-05); integration's `p6-int-real-e2e` (the real app works end to end).
Parallel with: any card that does not use the screen, the sound, a Claude Code test session or the power settings.
Build folder: `<scratchpad>\p6-media\` (exes, raw captures, renders).

## Goal

Make what the README, the Pages page and the Store listing show (Haik, 2026-11-04): measured speed numbers that prove
Wakeshot is very fast, pictures of the menu and tooltip, a short silent loop of the icon, and a 30 second promo video
with sound and effects, animated in code. Everything shows only what Wakeshot really does.

## Read first

- `docs/tasks/p6-launch-research.md`, Result; `docs/research/launch.md` parts 1 and 4.
- `docs/product.md`: "The job", "Distribution" (the README and video lines).
- The brand: `docs/brand.md`; what `p6-brand-apply-media` changed in `media/` (its Result and decisions); the menu:
  `p5-haik-check` Result.

## Deliverables

`media/`, owned whole, outside the Cargo workspace (no Rust):

- Speed numbers, measured on the installed release build and written to `docs/research.md` with the method (additions
  only): idle RAM (working set) and CPU time over 10 minutes idle and during a hold, for `wakeshot-tray.exe` and the
  guard; exe and install size; time from Claude Code starting work to the hold starting, and from its end to the
  hold ending (log timestamps). The same numbers for x64 here; ARM64 "not measured" unless a device is at hand.
- Screenshots: the menu, the tooltip while holding and while idle, a notice; light and dark; at 100 % and 150 %
  scale; Store sizes as the research lists them. Real captures, cropped to the tray area, nothing private in view.
- A short silent loop (the format the research picks): the icon going from idle to working and back.
- The 30 second video with sound and effects: built in code with the tool the research picks, from the real icon art
  and the screenshots, following its storyboard. MP4 at the sizes GitHub, Pages and the Store need. Music and effects
  only with a license that allows it, each credited in `media/CREDITS.md`.
- `media/README.md`: how to rebuild every file with one command each (for when the icon changes), and the source of
  each asset.

## Acceptance checks

- Each speed number in `docs/research.md` has its method, the build version and the date.
- Every rebuild command runs from a clean checkout of `media/` and gives the same files (script).
- The video is 30 s (plus or minus 2 s), has an audio track, and matches the research's specs (ffprobe).
- No asset without a license line in `media/CREDITS.md`; no em dash and no emoji in `media/` text files (script).
- The power settings after the capture session equal the values before it (the same check the real e2e uses).
- Haik's yes on the video and the loop is in the Log.

## Out of scope

- The README, the Pages page and the Store listing text (`p6-readme`, `p6-pages`, `p8-store`); any app code.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

1. `docs/research/launch.md` line 151 ("Wakeshot's row") still says a closed laptop on battery "hibernates" after 5
   minutes; since 2026-11-04 it sleeps, with hibernate only as the fallback (`docs/product.md`). `p6-readme` drafts
   its rival table from that row.
2. `docs/brand.md`, "Logo and wordmark", says "Not checked: whether Segoe UI may be used in rendered pictures and the
   video. `p6-media` checks it". Checked 2026-11-04 (Part A): Microsoft's Font redistribution FAQ
   (learn.microsoft.com/en-us/typography/fonts/font-faq, fetched with curl) says "We view creating graphic files as
   being essentially the same as printing from an output device" and, for videos, "We view this as the same as
   graphic files, providing the captions or text is rendered as a bitmap image, and the font files themselves are not
   embedded within the video file". The line in `brand.md` and a `research.md` entry are for their owners (this card
   may not edit `research.md` before its speed numbers).
3. `p5-haik-check` may change the menu words or the icon after Part A: the loop then needs `npm run loop` again (and
   `check-art.mjs` first if the art changed).

## Log

- 2026-11-04: card written from Haik's launch request (speed shown, pictures, a 30 s animated video with sound and
  effects).
- 2026-11-04 13:55: Part A started (orchestrator, session `wakeshot-12`): the icon loop, the video pipeline with
  stand-in pictures, `media/README.md`, `media/CREDITS.md`, the speed script (not run). Started before
  `p5-haik-check` on the orchestrator's call, for Haik's review.
- 2026-11-04 13:57 (Part A): Remotion 4.0.540 installed inside `media/` with npm (versions pinned, lock file).
  Nothing system-wide; Remotion bundles FFmpeg and ffprobe and put its Chrome Headless Shell into
  `media/node_modules/.remotion`. npm 11.20 skipped esbuild's install script (its `allowScripts` rule); the build
  works without it. No usage event without a licence key (read in `@remotion/renderer` `render-media.js`).
- 2026-11-04 13:58 (Part A): the Sunrise art ported to `media/src/art/sunrise.ts`; `check-art.mjs` rasters the port
  as `art.rs` does and compares it with the Rust-written sheets and `assets/wakeshot.ico`: 0 difference at every size
  (tray glyphs 16 to 48 px in three states on light and dark; app icon frames 16 to 64 px).
- 2026-11-04 13:59 (Part A): soundtrack made in code, the same bytes on two runs; loops rendered (light 0.27 MB,
  dark 0.46 MB).
- 2026-11-04 14:00 (Part A): draft video rendered in 57 s at concurrency 4: 2.2 MB, all `check-video` lines ok;
  audio -15.6 LUFS integrated, true peak -1.0 dBTP (ffmpeg `loudnorm` measure).
- 2026-11-04 14:01 (Part A): Segoe UI checked: Microsoft's Font redistribution FAQ allows Windows fonts in pictures
  and video text rendered as bitmaps; the video uses Inter anyway, for builds that match on any machine.
- 2026-11-04 14:03 (Part A): `check-rebuild.mjs` (clean copy, `npm ci`, every command): all 5 outputs the same
  bytes. Stills reviewed by eye; fixed the open lid running off the top, the jagged 1.25x menu zoom, a speck in the
  nearly filled sun and the glow's hard edge.
- 2026-11-04 14:12 (Part A): after the last source edits, the outputs rebuilt and `check-rebuild.mjs` run again: the
  art check and 5 of 5 outputs the same bytes. Scratch folder `<scratchpad>\p6-media\` deleted. No git run.

## Result

**Part A done (2026-11-04 14:10). Part B waits** for an installed release build, the screen slot with a real Claude
Code test session, 10 idle minutes and Haik's look.

### Part A: built

`media/` (owned whole; all new):

- `package.json`, `package-lock.json`, `tsconfig.json`, `.gitignore` (`node_modules/`, `.remotion/`,
  `public/sound/`, `out/draft/`).
- `src/art/heldLight.ts`: an exact port of `art.rs` (tray glyph, app icon, the review sheets); `src/art/motion.ts`:
  the states in between (the light coming on); `src/art/Glyph.tsx`: draws the port's pixels on a canvas. Since
  `p6-brand-apply-media` (2026-11-05).
- `src/loop/IconLoop.tsx`: the README loop, idle to holding and back, tray colours, light and dark.
- `src/promo/Promo.tsx`, `scenes.tsx`, `parts.tsx`: the 30 s video, eight scenes on the storyboard of
  `research/launch.md` part 4; `src/timeline.ts` (frames for picture and sound), `src/copy.ts` (every word on
  screen, drafts), `src/fonts.ts`, `src/theme.ts`.
- `data/shots.json` (each picture, its size, stand-in or not, its source), `data/speed.json` (empty),
  `data/settings.json` (stand-in: the laptop's own Sleep and 5 hours).
- No screen pictures yet: every shot in `data/shots.json` draws a named placeholder box in the draft, and the final
  render refuses it (`p6-brand-apply-media` deleted the old stand-ins).
- `scripts/`: `build.mjs` (loop, draft, video, poster), `make-sounds.mjs`, `measure-speed.ps1` (not run; parse
  checked only), `check.mjs`, `check-art.mjs`, `check-video.mjs`, `check-rebuild.mjs`, `lib/png.mjs`, `lib/gif.mjs`
  (the loop's own GIF writer, exact colours).
- `out/icon-loop-light.gif`, `out/icon-loop-dark.gif`: the held light fading on and off; final unless the icon
  changes.
- `README.md` (one command per file, sources, what to do when the icon, chimes or words change) and `CREDITS.md`.
- Not kept: `out/draft/wakeshot-promo-draft.mp4` and its poster (git-ignored), for Haik's first look.

### Part A: checks

| Check | Outcome |
|---|---|
| Art port equals `art.rs` (`check-art.mjs`) | ok, 43 comparisons, worst difference 0 |
| Rebuild from a clean copy gives the same files (`check-rebuild.mjs`) | ok, 5 of 5 the same bytes: both loops, soundtrack, draft video, draft poster |
| Video specs (`check-video.mjs --draft`) | ok: h264, yuv420p, 1920 x 1080, 30 fps, 30.02 s, aac track, 2.18 MB; loops GIF89a, 5 s, forever, 0.27 and 0.46 MB |
| No em dash, en dash or emoji; every asset credited (`check.mjs`) | ok, 10 assets; every text file in `media/` is plain ASCII |
| Final render refuses stand-ins (`build.mjs video`) | ok: refuses and lists the 8 stand-in shots, the 6 missing speed fields and the stand-in settings |
| Types (`tsc --noEmit`) | ok |
| Speed numbers in `docs/research.md`; power settings before equal after; Haik's yes | waits for Part B |

### Decisions (principal designer, for Haik's review; reasons and triggers in `media/README.md`)

1. All sound made in code (bed in C major so the C6 and G6 chimes sit in tune, effects, chimes read from
   `chime.rs`), not the research's Kenney and Freesound files: no third-party file, same bytes every build. Change it
   when Haik finds it thin.
2. Instrument Sans for every word of the video, self-hosted in `public/fonts/`, and JetBrains Mono from npm for the
   drawn terminal and the install line; not Segoe UI (`p6-brand-apply-media` decision 6). Change it when the install
   line becomes plain words.
3. The loop is the tray glyph as the tray draws it: the held light fades on and off in place, on GitHub's own page
   colours, no glow; 480 x 240, 25 fps (GIF delays are whole hundredths), 5 s (`p6-brand-apply-media` decisions 2
   to 4). Change it when Haik finds it slow or fast.
4. The art as an exact port of `art.rs`, drawn from its pixels and never redrawn as vectors (`p6-brand-apply-media`
   decisions 9 and 10). Change it when the video needs the icon at a size between whole pixels.
5. Screen pictures zoom by whole numbers with hard pixels, else smooth.
6. The final render refuses stand-ins and missing numbers; every draft frame says DRAFT.
7. The story follows today's behaviour: after the lid beat, "Work done" says a closed laptop goes to sleep once
   nothing has happened for 5 minutes, with Wakeshot's going-to-sleep chime; the lid scene plays the lid chime.
8. Settings scene shows the laptop's real values (Sleep, 5 hours), not the storyboard's "4 hours".
9. Install line on the end card: the GitHub address until winget is live.
10. Store-size screenshots dropped, as the launch research says (no Store at launch).

### Part B: what waits

1. Speed numbers (installed release build): `measure-speed.ps1` modes snapshot, size, idle, hold, delays, compare;
   results into `docs/research.md` (method, build, date) and `media/data/speed.json`. Idle and hold refuse to start
   while any `claude`, `node`, `cargo` or `rustc` runs; an agent's own busy Claude Code session makes Wakeshot hold,
   so Haik starts these two from a plain PowerShell window with no session working.
2. Final captures S1 to S6 at 150 % and 125 %, light and dark (100 % only by Haik's hand), into `media/public/shots/`
   with `data/shots.json` updated; S9 values from the capture session's log into `data/settings.json`. Nothing private
   in view.
3. Real Claude Code session captures (the holding tooltip, S3).
4. `npm run video`, `npm run check-video`, `npm run check-rebuild -- --final`; the power settings check after.
5. Haik's look at the loop and the video; his yes in the Log.
6. Upload with `gh issue comment <n> --attach` to a pinned "Media" issue; the URL into `media/README.md`.

Open: the Windows screenshot terms of Microsoft and the bundled FFmpeg and Chrome licences are not checked
(`CREDITS.md` says so).

## Part C: the dynamic cut (2026-11-05)

Written by the orchestrator of Haik's brand session. Haik wants the 30 s video dynamic: things change every 0.5 to
1 s, the camera moves between ideas, one thing at a time, large. The spec is `docs/research/promo-video.md` (style
rules with numbers, a 9 shot storyboard, the captures Part B needs, the parts to build, music and formats). Its seven
questions were answered with their defaults as orchestrator decisions for Haik's review (Haik away: "can start media
... even without question"). Part C builds the cut with stand-in captures and stand-in numbers; Part B then swaps in
the real captures and the measured numbers and renders the final.

Status: in progress. Needs Haik: after it, he watches the draft (about 10 min) and picks the music by ear from three
candidates (about 10 min). Agent: `wakeshot-worker`. Size: about 200K tokens.
Real laptop: no screen, no power settings, no Claude Code test session. Renders only, one at a time (concurrency 4).
Owns: `media/` and `docs/research/launch.md` part 4 (the storyboard and shot list there become a pointer to
`promo-video.md`). `docs/research/promo-video.md` only to keep it true to what was built (say so in the Log).
Build folder: `<scratchpad>\p6-media-c\` (renders, contact sheets, music candidates).

### Deliverables

1. The motion parts and scenes of `promo-video.md` "The build", in its order: the parts first, each on its own
   composition, then the 9 shots in storyboard order, each checked by its own contact sheet before the next.
2. `scripts/check-pace.mjs` as the spec says, run by `npm run check` (or its own script), passing.
3. Effects made in code; Kenney CC0 sounds only where a code sound is weak. Wakeshot's own chimes where the app plays
   them.
4. Music: three CC0 candidates (110 to 130 BPM, calm electronic, a lift after a sparse intro), each cut to 30 s on the
   storyboard and conformed to 120 BPM, kept in the build folder until Haik picks one (the pick then goes into
   `media/` with its line in `CREDITS.md`); the 120 BPM sequencer in code as the fallback and the draft's default
   bed. Licence checked on each track's own page.
5. One full-size draft with the DRAFT mark, at `<scratchpad>\p6-media-c\draft.mp4`, and its contact sheet; its file
   size and the CRF used in the Result.
6. `media/README.md` and `media/CREDITS.md` follow every change.

### Rules

- `promo-video.md` "How to brief the agent that builds it" applies. Remotion's agent skills: read them from GitHub
  (`remotion-dev/skills`) or install them outside the repo; add no skill files to the repo. Check every API against
  the installed 4.0.540 types. A new `@remotion/*` package only pinned to 4.0.540, and only after a spike shows it
  renders the same bytes twice (`check-rebuild`).
- Every claim on screen is something v1 does (`docs/product.md`); words on screen are drafts. UI words come only from
  captures. No agent but Claude Code.
- No scene, layout or line from another app's video. Describe the style only in our own words and numbers.
- Keep `npm run check`, `check-video` (as it applies to a draft) and `check-rebuild` passing.

### Log

- 2026-11-05 ~21:00: Part C written by the orchestrator.

### Result
