# Speed numbers, pictures, the icon loop and the 30 second video

Phase: 6. Status: Part A done (2026-11-04: the loop, the video pipeline with stand-ins, the media docs, the speed script); Part C round 3 done (2026-11-05 23:30: real captures in, `draft-3.mp4` ready for Haik's look); round 4 (the final) see its own status below; Part B after Part C: the installed build, the screen slot with a real Claude Code session, 10 idle minutes and Haik's look. Needs Haik: yes: he watches the video and the loop and says yes or what to change; he stays
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
4. Round 4 (2026-11-06): a repo grep finds the name of another product Haik does not want in any Wakeshot file in
   `docs/plan.md` line 259 and `docs/tasks/p6-repo-files.md` lines 47 and 129 (the latter two as a "no mention" rule
   and its check). Not this card's files; for their owners.

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

Status: round 3 done 2026-11-05 23:30 (round 1 21:58, round 2 22:40); round 4, the final, below. Agent: `wakeshot-worker`. Size: about 200K tokens.
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
- 2026-11-05 20:55: Part C started (worker). Read the card, `promo-video.md` and the current `media/` sources. Build
  folder `<scratchpad>\p6-media-c\`.
- 2026-11-05 21:00: Remotion's skills read from GitHub (`remotion-dev/skills`, router, timing, motion blur, audio,
  sfx, 3d, video layout) into the build folder; no skill file in the repo. APIs checked against the installed 4.0.540
  types: `Easing.bezier`, `Easing.spring` (mass, damping, stiffness), `interpolate` (`output: 'perceptual-scale'`
  exists), `random`, `Folder`; the CLI's `--frames` takes a comma list (read in `@remotion/renderer` `options/frames.js`).
- 2026-11-05 21:10: motion parts built (`src/beat.ts`, `src/motion/`: Camera, DirectionalBlur, Focus, Words, Cursor,
  Pop, Counter, Stage), each on its own composition in the Parts folder. Contact sheets: camera slide and whip blur
  ok; focus ok; CSS 3D laptop renders. Found: Chrome's backdrop filter (the focus dim) draws mirrored copies when
  Remotion renders at `--scale=0.25`, fine at full size, so `scripts/contact-sheet.mjs` renders the picked frames at
  full size and shrinks them itself.
- 2026-11-05 21:18: the 9 shots built on the new timeline (`src/timeline.ts`, `src/promo/`), each checked on a contact
  sheet in order; fixes after the sheets: the hook opens with lines already there, the faint terminal blurred behind
  the name, the card leaves with the slide back, the laptop a quarter larger, the cursor fades after its last click,
  a "still working" chip in the lid shot (the pace check found 2.1 s with nothing new there).
- 2026-11-05 21:21: soundtrack rewritten: a 120 BPM sequencer in code (kick, hats, bass, pluck, pad) shaped by the
  storyboard, whooshes peaking on each move's first frame, the chimes from `chime.rs`, the music ducked 4 dB under
  each chime, BS.1770 loudness in the script. ffmpeg `loudnorm` reads -15.1 LUFS, true peak -1.4 dBTP.
  `scripts/check-pace.mjs` passes and runs in `npm run check`.
- 2026-11-05 21:24: first full draft rendered (1 min 51 s at concurrency 4): 7.83 MB at CRF 16, `check-video --draft`
  ok. Then the laptop resized to fit the open lid above the captions (scale 1.12) and a second draft rendered at
  21:35 (1 min 52 s): 7.84 MB.
- 2026-11-05 21:33: music: OpenGameArt and the Free Music Archive searched for CC0 tracks (Freesound needs a login);
  14 tracks downloaded into the build folder and their tempo and lift found by `scripts/music-candidate.mjs`; three
  in 110 to 130 BPM kept, each licence read on its own page; each fitted to the storyboard and mixed with the effects
  and chimes; each muxed onto the draft's picture with Remotion's ffmpeg. The other eleven files deleted.
- 2026-11-05 21:36: `docs/research/launch.md` part 4: the old storyboard and shot list replaced by a pointer to
  `promo-video.md`. `promo-video.md` changed to match the build (worker's choices): 10 framings (the settings slide
  starts at 12 s, the menu shot pans once), a capture pops by rising not scaling, captions leave at once on a cut,
  the ring narrows to the session's name at 7 s, the blur strength rule, digit boxes in the counter, contact sheets at
  full size, the measured draft size, how a found track is fitted, the files built. `media/README.md` and
  `media/CREDITS.md` follow.
- 2026-11-05 21:45: `check-rebuild` failed on the draft video: two renders of the same source differed in a few
  frames by one to five levels (frame by frame with Remotion's ffmpeg and a small diff script). Cause: CSS `filter`
  layers (the word blur, the faint terminal's blur, the cursor's drop shadow) render a level apart from run to run on
  the GPU. Software GL (`--gl=swangle`) was 4 times slower and differed more. Fix: the blurs are text shadows and the
  cursor's shadow a plain offset path; the camera's SVG blur and the focus backdrop filter stayed (they matched in
  every repeat). After the word fix, one frame (636) still differed: the cursor's shadow, now fixed too.
- 2026-11-05 21:58: round 1 Result written. Haik's five notes arrived (~21:50); round 2 started.
- 2026-11-05 22:03 (round 2, note 1, music): candidate 3 put into `media/public/music/a-chill-fever.mp3` (the
  original file, unchanged, SHA-256 in `CREDITS.md`); `data/music.json` names it with its fit (130 BPM, lift
  44.33 s); `make-sounds.mjs` fits it on every build (`scripts/lib/fit-music.mjs`, split out of
  `music-candidate.mjs`); the code bed stays as the fallback. `CREDITS.md`: its line with the licence from its own
  page (License(s): CC0; "all my music is public domain"). Candidates 1 and 2 and their drafts deleted from the build
  folder. Mix: -15.2 LUFS, -1.6 dBTP (`loudnorm`).
- 2026-11-05 22:10 (note 2, the ending): found the cause: the picked track is 59 s and its lift late (44.3 s), so in
  the round 1 candidate draft the music ran out at 25.2 s and the end card started on silence, then a synthetic
  rising sweep, a whoosh and two pops. Now: the loopable track is played three times over before it is cut, so it
  never runs out; on the end card it plays on for a beat, fades over 2.5 s into a soft ping-pong echo (dotted
  eighth, darker each repeat) and is silent 0.1 s before the end (levels every 0.5 s from 26 s: -15, -19, -15, -19,
  -21, -30, -40, -56 dB); the light at the lockup has one soft swell of air (unpitched, so it cannot clash with the
  track's key, which the script could not pin down: G minor in its first bars, unclear later); no whoosh, sweep or
  pops there. The code bed's fallback resolve lost its drum. The hit under the first light is lower (the limiter
  pulled 4.9 dB there). Mix: -15.3 LUFS, -1.6 dBTP (`loudnorm`).
- 2026-11-05 22:20 (note 3, pace): measured the density from the timeline (events in each 2 s window, caption holds):
  the peaks were 6 events in 2 s at 8 s (lid cut, close, shut, lock and two lines within 1.5 s), at 12 s (slide,
  light off, card, line, two rolls) and at 19.5 s (menu hover, pan, hover, click, two lines); the shortest holds were
  "No second tool." 1.5 s and the tagline 1.6 s. Changes, on the beat grid, 30 s kept: the speed numbers fold into
  the end card as one line (the spec's own lever: their 3 s go to the other shots), the menu shot gets 6 s (hover,
  pan and click 1.5 s apart, not 1), the lock comes a beat later and its line is shorter ("Work goes on, locked."),
  the settings rows roll 1 s apart, the "still working" chip is gone, and the tray shot gets 5 s for note 5. The
  pace check still passes (10 framings of 2 to 4 s, longest gap 1.8 s); the end card may show three lines (the check
  now allows that there only).
- 2026-11-05 22:35 (note 4, quality), judged on full-size frames, not only sheets. Laptop rebuilt
  (`LaptopTilt.tsx`): neutral graphite lit from the top left instead of flat olive, a real keyboard (six rows of
  keys, a space bar), a touchpad, base edge and side faces, a lid with thickness, a shell and a front edge face that
  carries the green light when shut, a screen with bezel, camera dot and a soft glass sheen, the terminal filling the
  display, a soft floor shadow; sized and placed so the open lid and the shut laptop both clear the captions.
  Terminal (`Terminal.tsx`): a window title bar with a tab icon and thin window marks, a drawn turning spinner
  (not ASCII), a drawn check for the done line, a waiting caret. Settings card: a "Power settings" title with a
  drawn plug, values in pills (green while Wakeshot holds, neutral once back), softer rim and top light. Chips and
  the lock badge share one panel look (soft top light, hairline rim, deep shadow); the padlock redrawn. The counter
  is no longer in the cut (note 3 folded the numbers into the end card).
- 2026-11-05 22:35 (note 5, the cause and effect): the tray shot (5 to 10 s) now shows it exactly: a terminal (the
  cause) left of the real tray rectangle (the effect, the 200% capture set at 2x). The glyph flies down from the
  title and goes out as it lands on the idle icon (S1); at 5.5 s Enter is pressed, Claude Code starts work, and at
  that same frame the icon turns holding (S2) with a ring of green spreading from it; the cursor comes to rest on the
  icon and the holding tooltip shows (S3, ringed, at 6.5 s); at 8 s the terminal says done and at that same frame the
  icon goes back to idle with the idle tooltip (S4). Lines: "Claude Code starts: it turns green." and "Done: it lets
  go." The ring does not dim the terminal here, so cause and effect stay in one view. Built with stand-in sketches in
  a new `tray` set in `data/shots.json` (600 x 300 at 200%, boxes glyph, tip, tipName, tipIdle); the old 3x tooltip
  set is gone. The real S1 to S4 from the capture worker are not in yet (no `captures.json` at 22:35).
- 2026-11-05 22:45: `draft-2.mp4` rendered (8.50 MB), its sheet looked at; checks run (Result, round 2);
  `check-rebuild`: the soundtrack is the same bytes with the found track, the draft video still differs slightly.
  `promo-video.md` (storyboard rows 3 to 8, counts, captures table, music) and `media/README.md` follow round 2.
  Scratch files in the build folder deleted.
- 2026-11-05 22:55 (round 3): the real captures of `p6-media-captures` swapped in. 16 files copied into
  `media/public/shots/` (the video's tray S1 to S4 and menu S1, S5a to S5d, dark; the README's menu, holding and idle
  tooltip in light and dark, and the build tooltip S7); `data/shots.json` rebuilt from its `captures.json` (scale
  200, its boxes, tooltip boxes, menu frames and the real rows; the placeholder rows gone; boxes grown by the shadow
  for the parts that rise in; the icon's 32 px square from the glyph box and the port's ink offset 4, 4);
  `data/settings.json` from S9 (no plugged-in timer: Wakeshot does not write it). Each file credited in
  `CREDITS.md`. The battery (96% in S1, 97% in S5a to S5d) is held at S5a's value by drawing its box over every
  menu frame; the covered TrafficMonitor and clock read as plain taskbar in the files. The sets float on the stage
  (shadow, rim, square corners) so the capture's plain backdrop reads as a picture, not a box. After the menu click:
  a ring on the icon without dimming and a lens with the holding glyph above it. CRF 18 (round 2 was 8.5 MB at 16).
- 2026-11-05 23:25 (round 3, the rebuild): `check-rebuild` with the real captures: the menu corner difference is gone
  (no menu frame differs). New differences, found by decoding both videos frame by frame and by rendering ranges
  twice and comparing decoded pixels (Chrome's PNG files of equal pixels differ in bytes, so a byte compare misleads):
  1. the hook (frames 0 to 59): the terminal's SVG spinner under the 3D tilt, up to 4 levels; fixed: the terminal
  draws no SVG (text dots for the spinner, boxes and text for its marks); two repeat renders of 0 to 59 then equal,
  and the whole rebuild shows no hook frame different. 2. the tray shot from 5.5 s: in repeat renders one frame now
  and then shows its neighbour's state at the icon and the terminal's caret (3791 pixels, up to 21 levels, at 164,
  165, 166 or 167 in different runs), the frames where the capture swaps from S1 to S2; and a 1 to 2 level shimmer at
  the terminal tab's 1.5 px border under the 0.56 scale. 3. 14.2 to 17.2 s: small differences at the entering words'
  shadow blur. Not fixed: causes 2 and 3 need more runs to pin down (2 looks like a frame captured before the swapped
  capture painted).

### Result

**Round 1 done (2026-11-05 21:35, checks to 21:58): one full-size draft and three music candidates for Haik.**
Round 2 (Haik's five notes of ~21:50) follows below.

#### Round 1: built

`media/` (owned whole):

- `src/beat.ts` (120 BPM grid), `src/timeline.ts` rewritten: the 9 shots, every shared moment (`T`), the camera keys,
  the focus keys, the caption blocks, the pace events, the sound cues and the music's shape, all on the grid.
  `src/copy.ts`: every word, drafts.
- `src/motion/`: `ease.ts`, `Camera.tsx` (world, view, velocity, whole-pixel offset), `DirectionalBlur.tsx`,
  `Focus.tsx`, `Words.tsx`, `Cursor.tsx`, `Pop.tsx`, `Counter.tsx`, `Stage.tsx` (lift and fixed dither).
- `src/promo/`: `Promo.tsx` (the 9 shots in one world), `Capture.tsx` (a set's shot at whole zoom, or its tagged
  stand-in sketch; the final render throws on a stand-in), `Terminal.tsx`, `LaptopTilt.tsx` (CSS 3D),
  `SettingsCard.tsx`, `world.ts`, `draft.ts`. The old `scenes.tsx` and `parts.tsx` removed.
- `src/parts/Parts.tsx` and the Parts folder in `src/Root.tsx`. `src/art/`: the tray fade can draw on a clear
  background (`motion.ts`, `Glyph.tsx`).
- `data/shots.json`: two sets (tooltip at 3x, menu at 1x) with boxes and rows; stand-ins until Part B.
- `scripts/`: `check-pace.mjs` (in `npm run check`), `contact-sheet.mjs`, `music-candidate.mjs`, `lib/audio.mjs`,
  `make-sounds.mjs` rewritten, `build.mjs` (the sets, poster frame 120), `lib/png.mjs` (a writer). `package.json`:
  `check-pace`, `sheet`.
- `README.md` (commands, checks, decision 1 rewritten, decision 11 with the Part C choices), `CREDITS.md`.

Docs: `docs/research/launch.md` part 4 points to `promo-video.md`; `promo-video.md` changed to match the build (Log).

#### Round 1: outputs (build folder `<scratchpad>\p6-media-c\`)

- `draft.mp4`: full size, DRAFT mark with second and frame, stand-in captures and numbers, code bed. 7.84 MB at
  CRF 16, H.264 and AAC, 1920 x 1080, 30 fps, 30.02 s, -15.1 LUFS, true peak -1.3 dBTP. `draft-poster.png`,
  `draft-sheet.png` (121 tiles, every 7.5 frames).
- Music: `draft-music-1-Hope_On_Repeat.mp4`, `draft-music-2-better_days_0.mp4`, `draft-music-3-a_chill_fever_0.mp4`
  (the same picture with each candidate); fitted tracks and mixes in `music/`, the originals in `music-raw/`. Links,
  licences and fit numbers in `media/CREDITS.md`.

#### Round 1: checks

| Check | Outcome |
|---|---|
| `npm run check` (dashes, emoji, credits, colour pairs, pace) | ok: 49 starts on the beat, 10 framings of 2.0 to 4.0 s, 109 new things before the end card, longest gap 1.80 s |
| `npm run typecheck` | ok |
| `check-art.mjs` | ok, all 9,458,740 pixels |
| `check-video --draft` | ok: h264, 1920 x 1080, 30 fps, yuv420p, aac, 30.02 s, 7.84 MB; both loops ok |
| Loudness (ffmpeg `loudnorm`) | draft -15.1 LUFS, -1.3 dBTP; candidates -15.3, -15.3, -15.2 LUFS; true peak -1.6, -0.7 (over -1), -1.5 dBTP |
| `check-rebuild` | loops, soundtrack and poster the same bytes; the draft video differs (see open items) |

#### Round 1: open items and findings

1. `check-rebuild` on the draft video still fails, now by very little: after the filter fixes, two renders differ
   only in 4 to 13 pixels by one level at the stand-in menu's rounded corner (frames 583 on, under a `clip-path`),
   and the encoder carries that to the end of the menu shot. The loops, the soundtrack and the poster rebuild to the
   same bytes. Part B's real captures replace the sketch; check again then, and if a clipped capture still differs,
   draw the pops with whole-picture swaps instead of clips.
2. Candidate 2's true peak is -0.7 dBTP after AAC (target -1 or lower); the pick's mix gets its own check.
3. The tempos were found by the script, not by ear. Freesound needs a login to download, so no Freesound track.
4. The draft is 7.84 MB at CRF 16, close to the spec's 8 MB line; real captures may push it over (then CRF 18).
5. After the menu click (21 to 23 s) the whole frame dims to 35% around a small ring on the icon, which may read as
   empty: for Haik's look.

### Round 2 (Haik's notes, 2026-11-05 ~21:50)

Notes, in order: 1. music: candidate 3; 2. the ending sounds are not good; 3. a little fast at some point; 4. the
drawn parts look like a draft (the laptop first); 5. show exactly that Claude Code starts work and the icon turns
green, then goes back when it finishes (real 200% captures coming from a capture worker; stand-ins meanwhile).
Log in Part C's Log above; result here when done.

#### Round 2: what changed per note (2026-11-05 22:40)

1. **Music: candidate 3.** "A Chill Fever (Loopable)" by Pro Sensory, CC0 (licence read on its own page), is in
   `media/public/music/a-chill-fever.mp3` with its line in `media/CREDITS.md`; `data/music.json` holds its fit and
   `make-sounds.mjs` fits it on every build (`scripts/lib/fit-music.mjs`). Candidates 1 and 2 deleted from the build
   folder. The code bed stays as the fallback.
2. **The ending sounds.** Cause found: the track ran out at 25.2 s, so the end card opened on silence, then a sweep, a
   whoosh and two pops. Now the loopable track never runs out, plays on a beat into the end card, fades over 2.5 s
   into a soft echo and is silent 0.1 s before the end; the lockup has one soft swell of air; no sweep, whoosh or pops.
3. **Pace.** The densest stretches (6 events in 2 s at 8, 12 and 19.5 s) and the shortest holds (1.5 and 1.6 s) were
   measured from the timeline. The speed numbers fold into the end card (one line, build and date in small print), the
   menu shot gets 6 s, the lid and settings beats are spread, the extra chip is gone. 30 s and the beat grid kept.
4. **Quality.** The laptop drawn again (graphite, keys, touchpad, edges, a lid with thickness, the light on its front
   edge, glass, floor shadow), the terminal as a finished window (title bar, drawn spinner and check), the settings
   card with a title and pills, one panel look for chips and badge. Checked on full-size frames. Nothing needed a
   different medium; the stand-in sketches of the captures stay plain on purpose (the real captures replace them).
5. **Claude Code turns it green, then it lets go.** The tray shot (5 to 10 s): terminal left, the real 200% tray
   rectangle at 2x right; at the frame Claude Code starts the icon turns holding (a green ring spreads from it), the
   holding tooltip shows under the cursor, and at the frame the work ends the icon goes back to idle with the idle
   tooltip. Built on stand-ins (set `tray`, S1 to S4); the real captures swap in when the orchestrator says they are
   ready (file, w, h, scale and the boxes from the capture's `captures.json`).

#### Round 2: outputs and checks

- `<scratchpad>\p6-media-c\draft-2.mp4`: 8.50 MB at CRF 16, H.264 and AAC, 1920 x 1080, 30 fps, 30.02 s, DRAFT
  mark; -15.4 LUFS, true peak -1.2 dBTP (`loudnorm`). `draft-2-sheet.png` (121 tiles), `draft-2-poster.png`.
- `npm run check` ok (63 files, 7 assets credited, pace: 49 starts on the beat, 10 framings of 2 to 4 s, longest
  gap 1.8 s); `npm run typecheck` ok; `check-art.mjs` ok; `check-video --draft` ok.

#### Round 2: open items and findings

1. The draft is now 8.5 MB, over the spec's 8 MB line for CRF 16 (still under GitHub's 10 MB); by the spec's rule
   the final goes to CRF 18, to be checked on the real captures for blocking on the plate (Part A saw it at CRF 20).
2. The real S1 to S4 are not in yet; the swap is a data change in `data/shots.json` plus a look at the boxes.
3. The track's tempo (130) and lift (44.33 s) were found by script, not by ear; the key could not be pinned down, so
   nothing pitched is added over it.
4. Round 1's open items 1 (`check-rebuild` on the draft video) and 5 (the dim after the menu click) still stand;
   the round 2 rebuild result is in the line below.
5. `check-rebuild` after round 2 (22:45): both loops, the soundtrack (now with the found track through ffmpeg) and
   the poster rebuild to the same bytes; the draft video still differs, as in round 1 item 1.

#### Round 3: real captures (2026-11-05 23:30)

1. **Captures in.** 16 files from `p6-media-captures` in `media/public/shots/` (video: tray S1 to S4, menu S1, S5a
   to S5d; README: menu, holding and idle tooltip in light and dark, the build tooltip S7), each credited;
   `data/shots.json` rebuilt from `captures.json` (scale 200, its boxes, menu frames and real rows; no placeholder
   rows); `data/settings.json` from S9 (Lid close: Do nothing while holding, Sleep back; Battery sleep: Never while
   holding, 5 hours back; no plugged-in timer, Wakeshot does not write it).
2. **Stand-ins gone.** The tray shot shows S1, S2, S3, S4 in one rectangle at 2x (the hover look and the tooltip
   arrive with the cursor; S4 at the frame the work ends); the menu shot S1, S5a to S5d at 1x, the menu and submenu
   rising in with their shadows. Both sets float on the stage (shadow, rim, square corners).
3. **Battery** held at S5a's 97% over every menu frame (S1 had 96%). TrafficMonitor and the clock read as plain
   taskbar in the files (checked on full-size frames).
4. **After the menu click:** a ring on the icon, no dimming, and a round lens above it with the tray glyph at 128 px,
   its light coming on; a soft rise in the sound.
5. **Size:** CRF 18 (round 2 was 8.5 MB at 16): `draft-3.mp4` is 7.17 MB; no blocking seen on the end card's light.
6. **Checks:** `npm run check` ok (79 files, 23 assets credited; pace ok); `typecheck` ok; `check-art` ok;
   `check-video --draft` ok (h264, 1920 x 1080, 30 fps, aac, 30.02 s, 7.17 MB); loudness -15.4 LUFS, -1.15 dBTP;
   `check-rebuild`: loops, soundtrack and poster the same bytes; the video differs (Log, 23:25: menu corner gone, hook
   fixed, two causes left in the tray shot and at 14 to 17 s).

Output: `<scratchpad>\p6-media-c\draft-3.mp4` (7.17 MB), `draft-3-sheet.png`, `draft-3-poster.png`.

## Round 4: the final (2026-11-06)

Written by the orchestrator (overnight run, Haik asleep). Status: done 2026-11-06 04:29; review fix round done 05:19 (waits for Haik's look and the reviewer's fix check). Agent: `wakeshot-worker`. Owns what Part C
owns, plus a new `## Speed numbers` section at the end of `docs/research.md` (additions only; re-read right before
the edit; the README links to `docs/research.md#speed-numbers`, so keep that heading). Real laptop: no screen, no
power settings, no Claude Code test session. Build folder: `<scratchpad>\p6-media-final\`. Round 3's files are in
the old scratchpad: `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\d7052579-3169-4874-8004-3ff3befa980b\scratchpad\p6-media-c\`.

1. **Speed numbers.** Measured 2026-11-06 02:09 to 02:32 on Haik's installed build (0.1.0, local x64 release of
   c9da3dd; it lacks only the high contrast menu fix, which does not touch speed). Raw files: `speed-size.json`,
   `speed-idle.json`, `speed-hold.json`, `speed-delays.json`, `idle-check.json`, `before.json`, `after.json`,
   `run.log` in `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\de37e39e-ac9f-4f08-8aa0-3d773dd2f757\scratchpad\speed\`;
   the wrapper that ran them is `speed-run.ps1` there. Fill `data/speed.json` from them and write the research.md
   section: method (launch.md part 1, and where the wrapper differs), values, date, build.
   - Idle, 10 min, 0 holds: tray private memory median 4.27 MB (max 4.30), working set median 17.14 MB, CPU 109 ms in
     10 min (0.018% of one core). Task Manager's default Memory column (private working set) read 3.4 MB for the tray
     at 03:55.
   - Hold, 10 min (`wakeshot run -- ping -n 600`): tray the same; guard private 1.67 MB, working set 11.66 MB; guard
     CPU not recorded (the ping ended about 15 s before the last sample, so the guard had exited).
   - Start delay, five `claude -p --model haiku` runs: status file seen (50 ms poll) to "Holding starts": 298, 114,
     155, 104, 108 ms; median 114, max 298.
   - End: in 4 of 5 runs "Holding ends" came about 0.75 s before the status file was removed, in one 2 ms after. The
     launch.md end method (the `idle` write's `statusUpdatedAt`) was not used, so give no end delay number.
   - Size: Setup.exe 12,310,500 bytes; `wakeshot-tray.exe` 2,907,648; `wakeshot.exe` 700,416; install folder
     14,269,755.
   - Before and after: the power settings the same (lid 1 and 1, sleep 18000 and 18000).
   - Orchestrator decision, for Haik's review: the video's memory number is the private memory (4.3 MB), the one
     Task Manager's Memory column comes close to and the PowerToys figure uses; the working set (17.1 MB) counts
     shared Windows files. Say "about 0.1 s" or the median for the start, never "instant".
2. **Haik's note, 2026-11-06 00:56** (`docs/phases/06-ship.md` work log): no 5:00 countdown after the closed-lid
   work. Show the closed laptop working behind the scenes, then the work clearly done, then it sleeps as usual. No
   countdown on screen and no timing claim the app does not keep (in v1 it sleeps after 5 minutes with no input,
   `docs/product.md`; a time cut is fine, a claim that it sleeps at once is not).
3. **`check-rebuild`**: fix what is still open: a frame showing its neighbour's state at the S1 to S2 swap near 5.5 s,
   the terminal tab border, and the word blur at 14 to 17 s. Compare decoded pixels, not PNG bytes.
4. **Final render**: DRAFT mark off, CRF 18, under 8 MB; `check-video` in its final mode and every other check
   (`npm run check`, typecheck, `check-art`, `check-rebuild`, loudness) passing. Output
   `<scratchpad>\p6-media-final\wakeshot-promo.mp4` plus its sheet and poster; anything `media/README.md` says goes
   into the repo goes there. Look at full-size frames, not only sheets.
5. Haik's notes so far, all done: music 3; a little fast in one place; the ending sounds; the laptop quality (he says
   it is OK now); Claude Code starts and the real icon goes green, then back. Keep them.
6. A fresh review follows (`wakeshot-reviewer`, against `promo-video.md`); its blockers and majors come back to you.

### Log (round 4)

- 2026-11-06 04:00 (UTC+4, as all round 4 times; times from the files' write times): round 4 started (worker).
  Read the card (Part C Log and Result), `promo-video.md`, `media/README.md`. Build folder
  `<scratchpad>\p6-media-final\`.
- 04:03 (item 1, speed): `data/speed.json` filled from the raw files (shown: 4.3 MB private memory, 109 ms CPU in 10
  minutes; start median 0.114 s; every raw value under `raw`); `docs/research.md` "Speed numbers" appended (method,
  where the wrapper differs from launch.md, every value, build, date). Installed exe sizes, version and the install
  folder sum read again: the same as the raw file.
- 04:09 (item 2, Haik's 00:56 note): 10 to 20 s rebuilt. Lid shuts at 11 s; at 11.5 s a work chip rises above the
  shut laptop (the terminal's spinner and the agent's next step, one a beat: edit, read, lint, test), the lock badge
  beside it at 12 s; at 13.5 s the chip says "Done. All 128 tests pass." and at that frame the edge light (the hold)
  goes off; the settings card as before (the chip stays in view at the left with its done line; camera and card moved
  so neither is cut). At 17 s a time cut (the frame dips dark into the cut and back), "Later, it sleeps as usual.",
  the power light fades at 18 s with the going-to-sleep chime, the moon at 18.5 s. The 5:00 countdown chip and its
  ticks are gone. Pace check ok (119 new things, longest gap 1.8 s). Full-size frames 390, 405, 450, 485, 570 looked
  at.
- 04:17 (item 3, `check-rebuild`): ranges rendered twice and compared by decoded pixels (`<build>\tools\det.sh`,
  `pixdiff.mjs`). Before any fix: 150 to 299 differed in 9 frames by 1 to 3 levels in the tray terminal's title bar
  (its gradient, not only the tab border); 300 to 599 in 40 frames by 1 level: the laptop's floor shadow (a radial
  gradient), the card's soft shadow at its lower edge (a 3 px band at y 870), and the floor shadow again under the
  slow push. All are soft paints in parts of the frame that did not change since the render tab's previous frame.
  Cause (found by test, the mechanism inside Chrome not verified): Chrome repaints only what changed since the frame
  that tab drew before, and which frame that was depends on how the 4 tabs share the work, so a soft edge keeps a
  level of a different old frame. Test 1: the stage repainted in full every frame (its lift moved 1/64 px on odd
  frames): the floor shadow then equal in 380 to 419, the card band still differed in 13 frames. Test 2: a new tree
  every frame (`key={f}` on the frame's root), so every frame is painted from nothing.
- 04:15 (the S1 to S2 swap): `Capture` keys its image by the file, so a swap mounts a new image instead of changing
  the source of the one on screen (the frames that showed the shot before were all at a source change; the mounts at
  195 and 240 never did).
- 04:12 (Haik's add-on, via the orchestrator): the README state table's icons: `npm run states` writes
  `out/state-<holding|idle|off>-<light|dark>.png`, 32 x 32, see-through, `tray_mask` at 32 px from the port;
  `check-video` lays each on its taskbar colour and compares it with art.rs's own review sheet cell: all 6 equal
  (a negative test, the idle icon on the holding row, differs in 672 channels). Credited; in `check-rebuild`.
- 04:19: test 2 passed: frames 150 to 299, 380 to 470 and 570 to 599 (271 frames) rendered twice, all equal by
  pixels. Test 1's stage change dropped (one mechanism, the new tree, covers it). `media/README.md` decision 11 and
  `promo-video.md` (storyboard rows 4 to 6 and 8, a time cut row in "Motion", counts) follow; `launch.md` part 4
  notes the sound sources as built.
- 04:20: `npm run draft`: 7.33 MB, `check-video --draft` ok; -15.39 LUFS, -1.23 dBTP.
- 04:22: `npm run video` (the final: no DRAFT mark, CRF 18): `out/wakeshot-promo.mp4` 7,096,189 bytes and
  `out/wakeshot-promo-poster.png`; `check-video` ok (all lines, state icons included); -15.39 LUFS, -1.23 dBTP. 24
  full-size frames taken from the MP4 and looked at (hook, light, tray at 170 and 200, lid, chip, done, card, dip,
  sleep, menu, end card), and the whole cut on a 121-tile sheet rendered with the final props
  (`contact-sheet.mjs` takes `--final` now). Copies and the sheet in the build folder.
- 04:25: `measure-speed.ps1` hold mode: the load is now `ping -n <minutes x 60 + 60>` (660 for 10 minutes), so the
  guard still runs at the last sample and its CPU time is read; parse checked, not run (no power settings this
  round). The 2026-11-06 numbers stay as measured with `-n 600`.
- 04:26: a slip: one `git status --short` ran inside a listing command (read only, its output thrown away). No other
  git command this round.
- 04:28: `npm run check-rebuild -- --final`: ok, a clean copy rebuilds all 15 outputs to the same bytes, the draft
  and the final video included (the open item since round 1 is closed). `npm run check` ok after the last doc edits.
  Result written.
- 04:45 (fix round): the review `docs/reviews/p6-media-final.md` read (0 blockers, 1 major, 10 minors).
- 04:53: m2: the dark captures' stretched covers refilled (row mean of the real taskbar beside them plus a seeded
  grain measured on the same taskbar); originals kept in `<build>\shots-before\` for comparison. Battery patch from
  S5d over S1 only.
- 04:56: M1 and m6 (one lid caption, badge with the chime, one chip step, caption leaves at done), the density limit
  in `check-pace` (negative test: the old lid fails it), m7 (`--color-space=bt709`, tags in `check-video`), m4 (card
  framing zoom 0.92), m1 and m5 (tray terminal 30 px, narrower), m3 (glyph drawn per size, lands on the moving icon,
  fades), m8 (speed line 40 px, note 28 px), m10 (README loudness line, faint terminal blur 7 px). Full-size frames
  of each changed part rendered and looked at.
- 05:01: draft and final rendered; the final checked (7.17 MB, BT.709 tags, -15.39 LUFS, -1.36 dBTP, the green
  decoded with BT.709 within 3 levels of the source). Frame 120 still showed the faint terminal's words if one looked
  for them: its opacity to 7%, and the rebuild check stopped (its temp copy deleted, no process of it left).
- 05:03: draft, final and `check-rebuild --final` run again in one go.
- 05:08: `check-rebuild --final`: 14 of 15 the same bytes; the final video differed (the draft did not). A second
  render of the final, decoded and compared frame by frame: frames 0 to 149 equal, 150 differs first in the faint
  terminal's area (y 815 to 980), and the encoder carries it to the cut at 300. Frames 140 to 175 rendered twice in a
  range: equal, so it is rare. The faint terminal (hundreds of soft letters at 7% under the whip's blur) now goes as
  the whip starts, where it cannot be seen anyway.
- 05:12: draft, final, `check-rebuild --final` and one more independent full render, in one go.
- 05:18: `check-rebuild --final` ok, all 15 the same bytes; the independent render the same bytes as `media/out`'s
  final. Frames 150 to 156 rendered and looked at (the glyph's one-frame dip at 151 noted in the Result). The final
  sheet rendered again; `check`, `typecheck`, `check-art`, `check-video` ok. Fix-round Result written.

### Result (round 4)

**Done (2026-11-06 04:29): the final video is built and every check passes.** Waits for Haik's look and yes, and the
fresh review (item 6).

#### Outputs

- `C:\Users\haika\code\wakeshot\media\out\wakeshot-promo.mp4`: the final after the review's fixes, 7,137,594 bytes
  (7.14 MB), H.264 CRF 18, BT.709 (converted and tagged), AAC, 1920 x 1080, 30 fps, 30.02 s, no DRAFT mark, -15.39
  LUFS, true peak -1.36 dBTP. (Before the fixes: 7,096,189 bytes, untagged BT.601, -1.23 dBTP.) `out/` is not
  git-ignored, so it goes into the repo with the commit; GitHub plays it only as an uploaded attachment (not
  uploaded).
- `C:\Users\haika\code\wakeshot\media\out\wakeshot-promo-poster.png`: frame 120, 2,512,195 bytes.
- Copies and the review sheet in the build folder:
  `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\de37e39e-ac9f-4f08-8aa0-3d773dd2f757\scratchpad\p6-media-final\`
  `wakeshot-promo.mp4`, `wakeshot-promo-poster.png` (both after the fixes), `wakeshot-promo-sheet.png` (121 tiles,
  every 7.5 frames, final props), `final-frames\` (26 full-size frames from the fixed MP4), `shots-before\` (the
  captures before the cover refill).
- The README state table's icons, 32 x 32 PNG, see-through, `tray_mask` drawn at 32 px (the tray at 200%); show them
  at 16 px: `C:\Users\haika\code\wakeshot\media\out\state-holding-light.png`, `...\state-holding-dark.png`,
  `...\state-idle-light.png`, `...\state-idle-dark.png`, `...\state-off-light.png`, `...\state-off-dark.png` (221
  to 341 bytes each). "light" and "dark" are the taskbar the glyph is drawn for (dark ink on light, white ink on dark).
- `media/out/draft/wakeshot-promo-draft.mp4` (7.43 MB after the fixes) and its poster, git-ignored, built for
  `check-rebuild`.
- `media/data/speed.json` (measured, every raw value under `raw`) and `docs/research.md` "Speed numbers" (method, where
  the run differs from launch.md, every value, build 0.1.0, date 2026-11-06).

#### What changed

1. **Speed numbers** in (item 1). The end card reads "Idle: 4.3 MB of memory, 109 ms of CPU in 10 minutes." (after
   the review; first "Light: 4.3 MB of memory idle, ...") and "Measured on Wakeshot 0.1.0, 2026-11-06, x64." No start
   or end delay on screen.
2. **Haik's 00:56 note** (item 2): 10 to 20 s rebuilt. The lid shuts; a work chip above the shut laptop shows the
   agent's next step every beat (the work goes on behind the lid), with the lock badge beside it; at 13.5 s the chip
   says "Done. All 128 tests pass." and the edge light (the hold) goes off at that frame; the settings card; then a
   time cut (a dip to dark) and "Later, it sleeps as usual.", the power light out, the going-to-sleep chime, the moon.
   No countdown, no clock, no claim that it sleeps at once.
3. **`check-rebuild`** (item 3): cause found by test (Chrome's inner workings not verified): soft edges (gradients,
   soft shadows) kept a level of whichever frame a render tab drew before, which differs from run to run. Fix: a new
   tree every frame (`key={f}` on the frame's root), and each capture's image keyed by its file (the S1 to S2 swap
   mounts a new image). 271 frames rendered twice: equal by pixels; then the clean rebuild below.
4. **Final render** (item 4): `build.mjs video`; DRAFT off; CRF 18; 7.10 MB.
5. **Add-on (orchestrator, for the README):** `npm run states` and its check in `check-video` against art.rs's own
   review sheets; credited; in `check-rebuild`.
6. Small: `contact-sheet.mjs --final`; `measure-speed.ps1` hold load outlasts the sampling (guard CPU next time).
   Docs: `media/README.md` (state row, speed source, decisions 6 and 11), `media/CREDITS.md` (final video, poster,
   state icons), `promo-video.md` (rows 4 to 6 and 8, a time cut row, counts, final size), `launch.md` part 4 (sound
   as built).

#### Checks (before the review; after it, see "Fix round (review)")

| Check | Outcome |
|---|---|
| `npm run check` (dashes, emoji, credits, colour pairs; pace) | ok: 87 files, 31 assets credited, 18 colour pairs; 49 starts on the beat, 10 framings of 2 to 4 s, 119 new things, longest gap 1.80 s, 15 lines of words |
| `npm run typecheck` | ok |
| `node scripts/check-art.mjs` | ok, all 9,458,740 pixels |
| `npm run check-video` (final) | ok: h264, 1920 x 1080, 30 fps, yuv420p, aac, 30.02 s, 7.10 MB; both loops ok; the 6 state icons equal art.rs's sheet cells, every pixel |
| Loudness (ffmpeg `loudnorm`, Remotion's ffmpeg) | -15.39 LUFS integrated, -1.23 dBTP true peak (target -14 to -16, -1 or lower) |
| `npm run check-rebuild -- --final` (clean copy, `npm ci`, every build command) | ok: all 15 outputs the same bytes: both loops, the 6 state icons, the soundtrack (twice), the draft video and poster, the final video and poster |
| Every speed number has its method, build and date | ok (`docs/research.md`, "Speed numbers") |
| Power settings the same before and after | ok for the speed run (`compare`: SAME); this round touched none |
| No banned product name, no style reference in the repo files of this round | ok (grep) |
| Haik's yes on the video and the loop | waits |

#### Open items

1. Haik's look and yes on `wakeshot-promo.mp4` (and the loop, unchanged since round 1).
2. The fresh review against `promo-video.md` (item 6).
3. Upload: `gh issue comment <n> --attach out/wakeshot-promo.mp4` on a pinned "Media" issue, the URL into
   `media/README.md` and the README; and the same MP4 and poster into `site/` for Pages (the page's owner).
4. Guard CPU during a hold: not measured (the script is fixed; one more `-Mode hold` run gives it). End delay: no
   number (launch.md's method not run).
5. The new tree each frame costs a little more per frame (images decoded again each frame); the final still
   rendered in about 2 minutes at concurrency 4 (from the draft's and the final's write times, 04:20:39 and
   04:22:27).

#### Decisions for Haik's review (principal designer)

1. **The work behind the lid is a chip, not a terminal.** One line in 32 px mono reads at README size; a small
   terminal's lines would not. Change it when Haik wants to see the whole terminal there.
2. **"Later, it sleeps as usual." with a dip to dark** as the time cut, not "5 minutes later": the 5 minutes count
   from the last input, which the video cannot show, and Haik asked for no timing claim. Change it when Haik wants
   the 5 minutes named.
3. **The settings framing moved**: the camera rests 150 px further left (the card stays where it was in the world,
   so it sits 150 px further right on screen), so the badge, the work chip with its done line and the card all fit
   without a cut edge.
4. **A new tree every frame** for same-bytes renders, over rendering at concurrency 1 (4 times slower). Change it when
   a render gets too slow.
5. **State icons at 32 px shown at 16 px**, see-through, so they sit on GitHub's page in both themes; light and dark
   follow the taskbar the glyph is drawn for. Change it to 48 px (shown at 24) if the table looks too small.

#### Fix round (review)

The fresh review `docs/reviews/p6-media-final.md` (0 blockers, 1 major, 10 minors), fixed 2026-11-06 04:45 to
05:19. Each finding and what was done:

- **M1, the lid shot is the densest stretch again (10.5 to 13.5 s): fixed.** One caption, "Close the lid: work goes
  on." (315), in place of two; the lock badge comes with the lid chime at 11 s (330; the app locks at the lid close),
  with no lock sound so the chime sits alone; the work chip rises at 12 s (360) with one step, "run npm test", under
  the turning spinner, then its done line at 13.5 s. No step every beat, no tick per step. The pace check now has an
  upper limit: at most 5 things to take in within any 2 s before the end card (cuts, moves, pops, clicks, lights,
  counts, rings, swaps, line starts; a cut counts once with what it opens on, a line that flows on from the one before
  not counted). The video's densest window is now 5; the round 4 lid shape, put back in a copy, fails the check (6
  within 2 s from 10.0 s), so it cannot come back unnoticed.
- **m6, "Work goes on" while the chip says done: fixed.** The lid caption fades at 13.5 s (405), the frame the chip
  says done and the edge light starts to go out; the light is out 9 frames into the slide.
- **m7, BT.601 matrix with no tags: fixed.** `--color-space=bt709` in `build.mjs`: ffprobe reads bt709 matrix,
  primaries and transfer; `check-video` checks the three tags. Frame 120 decoded with the BT.709 matrix: the hero
  light's middle 180 221 87 against the source `#B6E05A` (182 224 90); the old file decoded the same way gave 178 210
  83.
- **m2, retouched patches on the captures: fixed.** The stretched covers of the 12 dark captures (5 tray-set files,
  the 2 dark README tooltips, 5 menu-set files) are refilled: each covered pixel is the mean of the real taskbar beside
  it on the same row plus a fixed seeded grain measured on a clean part of the same taskbar; no stretched column is
  left (checked by script) and the crops at 3x gain show no streaks or edge. The battery patch now comes from S5d
  (menu closed, like S1) and only over S1, so the taskbar's top border runs on unbroken (checked at frame 605). The
  two light README stills keep their first cover (their taskbar has a sideways gradient that a row mean does not
  match): left, their fill needs its own method, and the README shows them small.
- **m4, the laptop cut by the left edge in the settings framing: fixed.** The card framing widens to zoom 0.92 (drawn
  parts only): the laptop, the badge, the chip and the card all in frame (frame 450).
- **m1, the tray terminal's text is 19 px: fixed.** The tray terminal shows `claude`, a shorter prompt ("fix the
  failing tests") and one status line at 30 px (scale 0.88, 660 px wide); no tool lines there and no key ticks for
  lines that are no longer shown.
- **m5, the tray terminal overlaps the capture's corner: fixed** with m1: the narrower terminal ends 31 px left of the
  capture before and after the drift, level with the tooltip.
- **m3, the whip lands as two icons, the glyph a scaled bitmap: fixed.** The flying glyph is drawn from the port at
  each whole size (no CSS scale), flies to where the capture's icon is on screen at that frame, lands on it at 5.27 s
  (frame 158) and fades into it by 5.4 s, so no second icon shows beside the real one (frames 150 to 160 looked at).
  Left for Haik's eye: as it aims at the icon, which is still below the frame when the whip starts, the glyph dips to
  the frame's bottom edge for one frame (151) before it rises with the tray. A path that aims first at where the icon
  will rest would keep it in the frame; not done, as it needs another full render and rebuild for one frame of a whip.
- **m8, the end card's small lines: fixed.** The speed line is shorter and larger: "Idle: 4.3 MB of memory, 109 ms of
  CPU in 10 minutes." at 40 px (was 34 px with 12 words); the build and date line at 28 px (was 24). The numbers are as
  measured.
- **m10, doc and poster points: fixed.** `media/README.md` decision 1 now gives the measured -15.4 LUFS and -1.2 dBTP
  after AAC (sample peaks up to -1.6 dBFS). The faint terminal behind the name is softer (blur 7 px, was 3) and
  fainter (7%, was 10%): a texture of work under the name, its words no longer read through it at a glance (frame
  120 looked at full size).
- **m9, "while agents work": left as is**, on the orchestrator's word: Haik's brand line, listed for him.

Renders and checks after the fixes (2026-11-06 05:19):

| Check | Outcome |
|---|---|
| `npm run check` | ok: 87 files, 31 assets credited, 18 colour pairs; pace: 48 starts on the beat, 10 framings of 2 to 4 s, 103 new things, longest gap 1.80 s, at most 5 things to take in within 2 s, 14 lines of words |
| `npm run typecheck`, `check-art.mjs` | ok; ok, all 9,458,740 pixels |
| `npm run check-video` (final) | ok, 32 lines: h264, 1920 x 1080, 30 fps, yuv420p, tagged BT.709 (matrix, primaries, transfer), aac, 30.02 s, 7.14 MB; both loops; the 6 state icons |
| Loudness (`loudnorm`) | -15.39 LUFS, -1.36 dBTP |
| The green, frame 120 decoded with BT.709 | 180 221 87 against `#B6E05A` (182 224 90) |
| `npm run check-rebuild -- --final` | ok: all 15 outputs the same bytes. The run before the last change failed on the final video only (frame 150 on; see the Log); after it, the rebuild and one more independent full render gave the same bytes as the final in `media/out` (three renders equal) |
| Full-size frames of each changed part | looked at: 120, 150 to 160, 200, 250, 320, 330, 345, 360, 375, 405, 410, 440, 450, 485, 512, 545, 560, 570, 605, 770, 850, 896; and the whole cut on a 121-tile sheet |

Final files after the fixes: `media/out/wakeshot-promo.mp4` 7,137,594 bytes (7.14 MB), `media/out/wakeshot-promo-poster.png`
2,512,195 bytes; the draft 7.43 MB. Copies, the sheet and 26 full-size frames in the build folder.
