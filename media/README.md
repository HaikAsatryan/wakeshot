# Wakeshot media

The icon loop for the README top, the 30 second promo video and its poster, built in code with
[Remotion](https://www.remotion.dev/) so they can be made again when the icon or the menu changes. Outside the Cargo
workspace; no Rust here. Card: `docs/tasks/p6-media.md`. Specs: `docs/research/launch.md` part 4. Style rules,
storyboard and captures: `docs/research/promo-video.md`.

## Setup

Node 26 (built with v26.13.0), then once in this folder:

```
npm ci
```

Nothing is installed system-wide. Remotion brings its own FFmpeg and ffprobe (`@remotion/compositor-win32-x64-msvc`)
and downloads a Chrome Headless Shell into `node_modules/.remotion` on the first render. With no licence key Remotion
sends no usage event (`@remotion/renderer` 4.0.540, `render-media.js`: the event is sent only when a key is set).

## One command per file

| File | Command | Notes |
|---|---|---|
| `out/icon-loop-light.gif`, `out/icon-loop-dark.gif` | `npm run loop` | 480 x 240, 5 s, loops forever, on GitHub's page colours; the tray glyph drawn at 128 px. Remotion renders 125 PNG frames (25 fps); `scripts/lib/gif.mjs` writes the GIF with each frame's exact colours. Show at 240 px wide for sharp HiDPI |
| `out/wakeshot-promo.mp4`, `out/wakeshot-promo-poster.png` | `npm run video` | Refuses while any picture is missing or a stand-in, or a speed number is missing |
| `out/draft/wakeshot-promo-draft.mp4` and its poster | `npm run draft` | A missing capture shows as a plain sketch of its set with a stand-in tag, missing speed numbers as tagged stand-ins (88.8, 888, 8.8); DRAFT, the second and the frame on every frame; not kept in git |
| `out/wakeshot-promo-poster.png` alone | `npm run poster` | Frame 180 of the video (the light on, the name and its line in), for the Pages `<video poster>` |
| `out/state-<holding\|idle\|off>-<light\|dark>.png` | `npm run states` | The README's state table: the tray glyph per state for the light and the dark taskbar, see-through, 32 x 32 (`tray_mask` drawn at 32 px, the tray at 200%); show at 16 px, the tray's size at 100%, so it stays sharp on a HiDPI screen. No Remotion |
| `public/sound/soundtrack.wav` | `npm run sounds` | Also run by `draft` and `video`; not kept in git. `-- --music <file.wav>` puts a fitted found track in place of the code bed, `-- --out <file.wav>` writes elsewhere |
| A found track fitted to the video | `node scripts/music-candidate.mjs <track> --out <file.wav> [--bpm N] [--lift s]` | Finds the tempo and the lift, conforms to 120 BPM with `atempo`, cuts 30 s on the storyboard; prints the numbers to rebuild it with. The lift it finds is the biggest 2 s rise in the first minute: when that rise comes just before a breakdown (a quiet stretch would then fall under the lid and settings shots), set `--lift` by hand to where the track's steady part starts (round 5, "Beach") |
| A contact sheet for review | `npm run sheet -- <composition> <from> <to> <out.png>` | About 4 frames a second, rendered at full size and shrunk to a quarter, tiled; add `--final` for frames without the DRAFT mark |
| Speed numbers | `pwsh scripts/measure-speed.ps1 -Mode <mode>` | See "Speed numbers" below |
| The live preview | `npm run studio` | Opens Remotion Studio in the browser |

`MEDIA_CONCURRENCY` sets how many frames render at once (default 4, so other work keeps its cores). The draft video
takes about 2 minutes on the test laptop. The motion parts each have a small composition in Studio's Parts folder
(`src/parts/Parts.tsx`).

## Checks

| Check | Command |
|---|---|
| The art port draws exactly what `art.rs` draws, no tolerance: every pixel of the three review sheets (tray glyphs at 16 to 48 px in four rows, dark, light and high contrast), the 14 frames of `assets/wakeshot.ico` and the notice picture | `node scripts/check-art.mjs`; CI runs it on every push (job `art-port`, Node 26, no `npm ci`: it needs only `node:` modules and the `.ts` port) |
| No em dash, en dash or emoji in the text files; every asset credited; every text colour pair in `src/theme.ts` at 4.5:1 or more and every graphic pair at 3:1; the pace check; and the agent lists in `README.md` and `site/index.html` equal to `src/agents.ts`, with a rules paragraph in the README for each agent that passed and none for one that did not | `npm run check` |
| Pace (`scripts/check-pace.mjs`, from the timeline data): every cut, move, pop, click and caption start on the 120 BPM grid within 1 frame; something new at least every 2 s before the end card; framings 2 to 4 s; captions hold 1.4 s after their last word (big words of 1 to 3 words 0.5 s); at most 5 things to take in within any 2 s (cuts, moves, pops, clicks, lights, counts, rings, swaps, line starts); 2 to 6 words a caption line; at most two lines on screen; the name by 3 s; the end card the last 4 s; frame 0 not empty | `npm run check-pace` |
| Video: H.264 and AAC, 1920 x 1080, 30 fps, 30 s plus or minus 2, under 10 MB; loops, read frame by frame: GIF, 480 x 240, 4 to 6 s, forever, under 1 MB, last frame equal to the first, a 40 px border of every frame exactly GitHub's page colour, the first frame the port's idle glyph and a held stretch its holding glyph, every pixel; the six state icons, each laid on its taskbar colour, equal to the cell of art.rs's own review sheet for that state at 32 px, every pixel, and see-through around the glyph | `npm run check-video` (add `-- --draft` for the draft) |
| A clean checkout rebuilds the same bytes | `npm run check-rebuild` (add `-- --final` once the final video exists) |
| Types | `npm run typecheck` |

## When something changes

- **The icon** (`crates/wakeshot-windows/src/tray/art.rs`): run its `write_art_files` test to refresh the sheets,
  `assets/wakeshot.ico` and the notice picture, then `node scripts/check-art.mjs`. It fails, and CI with it, until
  `src/art/heldLight.ts` draws the same bytes: port the change in the same change, step by step, each f32 operation
  wrapped in `F` in Rust's order (the file's head says why). Then `npm run loop` and `npm run video`.
- **The chimes** (`crates/wakeshot-windows/src/sound/chime.rs`): nothing to port; `make-sounds.mjs` reads the numbers
  from that file. If the file changes shape, the script stops with the name of what it could not find.
- **The menu or tooltip words**: new captures into `public/shots/`, then update the set in `data/shots.json` (file,
  `standIn: false`, the set's size and scale, and the boxes and rows measured from the pictures).
- **The supported agents** (a live test passed, or an agent is dropped): its line in `src/agents.ts`, then
  `npm run agent-lists` (writes the lists in `README.md` and `site/index.html`), then `npm run check`, which names any
  README rules paragraph to add or delete. `live` is `passed`, `partly` (the card is done but no working hold was seen
  live) or `pending`; only `passed` shows anywhere. The video's agent tiles (shot 3) follow on the next render and
  lay themselves out for any count from 1 to 6.
- **The words on screen**: `src/copy.ts`. **The timing**: `src/timeline.ts` (on the grid of `src/beat.ts`); the
  sounds and `check-pace` follow it.
- **The speed numbers**: `data/speed.json`, from the measure script's output.
- **The install line**: `COPY.install` in `src/copy.ts`; the winget command once the package is live.

## Where each input comes from

| Input | From |
|---|---|
| Icon art | `src/art/heldLight.ts`, a port of the held light in `art.rs` (tray glyph, app icon, the sheets' composition), equal to every file `art.rs` writes (`check-art.mjs`, 0 of 9,458,740 pixels differ, 2026-11-05). The states in between (the light coming on) are the media's own: `src/art/motion.ts` |
| Screen pictures | `public/shots/`, real captures of `p6-media-captures` (2026-11-05, the DELL at 200%), listed in `data/shots.json`: two sets, each one screen rectangle at one scale with its boxes and rows from the capture's manifest: `tray` (S1 to S4 in one rectangle) for the video, and `menu` (S1, S5a to S5d), out of the 30 s cut since `p6-agents-video-readme` Part C and kept for a longer cut (still in `shots.json`, `CREDITS.md`, `Capture.tsx` and `world.ts`, so a final render checks its files), and the README stills (menu, holding and idle tooltip, light and dark; the build tooltip) |
| Settings values in the video | `data/settings.json`, from the capture session's log (S9): lid close Do nothing while holding, Sleep before and after; battery sleep Never while holding, 5 hours before and after. The plugged-in timer is not written by Wakeshot, so it is not shown |
| Speed numbers | `data/speed.json`: measured 2026-11-06 on Wakeshot 0.1.0, x64 (`docs/research.md`, "Speed numbers", has the method and every raw value). The end card shows the tray's private memory idle (4.3 MB) and its CPU time in 10 idle minutes (109 ms) |
| Sound | `scripts/make-sounds.mjs`: the music Haik picked (`public/music/`, named with its fit in `data/music.json`, fitted on every build by `scripts/lib/fit-music.mjs`), the effects made in code, the chimes from `chime.rs`. The code bed (a 120 BPM sequencer) when `data/music.json` names no file |
| Fonts | Instrument Sans in `public/fonts/` with its `OFL.txt` (from the designer's repo); JetBrains Mono from npm; loaded in `src/fonts.ts` |
| Colours | `src/theme.ts`: Moss (`docs/research/brand-directions-2.md`), GitHub's page colours for the loop |

Licences: `CREDITS.md`.

## Speed numbers

`scripts/measure-speed.ps1` follows the method in `docs/research/launch.md` part 1, on the installed release build:

1. `-Mode snapshot -Out before.json` (power settings, read only).
2. `-Mode size -Setup <Setup.exe>`: exe sizes and the install folder.
3. `-Mode idle`: 10 minutes, nothing holding; tray working set and private bytes every 10 s, CPU time.
4. `-Mode hold`: 10 minutes of `wakeshot run -- ping -n 660 127.0.0.1` (it outlasts the sampling, so the guard's
   CPU time can be read; the 2026-11-06 run used `-n 600` and has no guard CPU); tray and guard.
5. `-Mode delays -Minutes 30`: start and finish 5 short Claude Code turns; pairs `statusUpdatedAt` with Wakeshot's
   `Holding starts` and `Holding ends` log lines.
6. `-Mode compare -Before before.json`: the settings equal the snapshot.

Measured 2026-11-06 on 0.1.0 (x64): the values, and where that run's wrapper differed (the delays from five headless
`claude -p` runs, by the status file appearing and going), are in `docs/research.md`, "Speed numbers".

Idle and hold refuse to start while `cargo`, `rustc`, `claude`, `node` or another build runs. That includes the
Claude Code session of an agent: its own busy session makes Wakeshot hold, so these two modes run from a plain
PowerShell window with no Claude Code session working.

## Decisions (principal designer, for Haik's review)

1. **Effects made in code; the music is Haik's pick.** Every effect (whooshes, clicks, pops, ticks, the low hit, the
   end card's swell of air) is sine waves and seeded noise, and the chimes come from `chime.rs`. The music is "Funky
   Disco Beats to Boogie/Woogie to" by Fupi (CC0), Haik's pick of 2026-11-06 from four new options (it replaced "A
   Chill Fever (Loopable)" by Pro Sensory, his pick of 2026-11-05): conformed from 110.25 to 120 BPM, cut so its
   lift (45.75 s into the track) lands on the light at 4 s, softer under the hook with a short silence before the
   light, 5 dB down under the lid chime, and on the end card it plays on for a beat and fades over 2.5 s into a soft
   echo, silent 0.1 s before the end. The fit plays any track three times over, so it never runs out; this one is
   133 s long and the cut ends at about 76 s of it. A minor, so both chimes (C6, G6) are in its key (by script, not
   heard). The music ducks 4 dB under each chime; the mix is set to -15 LUFS with sample peaks under -2.5 dBFS in the
   WAV; after AAC the Part C draft measures -15.3 LUFS and -1.6 dBTP true peak (`loudnorm`, 2026-11-06). The 120 BPM
   sequencer in code (C major, so the C6 and G6 chimes sit in tune) stays as the fallback when `data/music.json`
   names no file. Why: a real track sounds full where the code bed sounded thin, and every build still makes the
   same bytes. Change it when Haik wants another track: try it with `music-candidate.mjs`, then `data/music.json`.
2. **Instrument Sans for every word of the video, self-hosted; JetBrains Mono only in the drawn terminal and the
   install command.** Instrument Sans SemiBold is the brand's wordmark face (`docs/research/brand-identity.md`
   section 7); the other lines use the same family so the name and the words read as one. The WOFF2 files and the
   licence sit in `public/fonts/`, copied unchanged from the designer's repo, so a render never fetches a font and
   draws the same letters on any machine. Segoe UI is not used: a system font makes the build depend on the machine.
   Change it when the video must match Windows' own text exactly and every build runs on Windows 11.
3. **The loop is the tray glyph as the tray draws it, on GitHub's own page colour.** The held light goes idle,
   holding, idle: it fades on and off in place, only the light takes the brand colour (`#B6E05A` on dark, `#3B7A2A`
   on light) and the line keeps the glyph colour; no glow. The outline takes the light colour in the first 35% of
   the fade and the inside brightens over all of it, so the frames in between read as one dim light. The idle and
   holding frames are `tray_mask` at 128 px, every pixel (`check-video` holds them to it). On in 0.44 s
   (ease out, a lamp coming on), off in 0.8 s (ease in and out), idle 1.6 s and holding about 2.2 s. Backgrounds are
   GitHub's default light and dark page colours (`#FFFFFF`, `#0D1117`; dark dimmed `#212830` and high contrast
   `#010409` show a faint box: the README's `<picture>` can only choose by light or dark). The GIF is written by our
   own writer: Remotion's single GIF palette moved the old loop's background by one level. Change it when GitHub
   changes its page colours (the loop check then fails) or when Haik wants the app icon in the loop.
4. **Screen pictures zoom by whole numbers with hard pixels**, other zooms smooth, so a capture is never blurred into
   something it was not, and uneven pixels never garble text.
5. **The final render refuses stand-ins and missing numbers** (`build.mjs`, and `Capture` and the speed shot throw),
   and every draft frame says DRAFT with its second and frame, so a draft cannot be mistaken for the real video and
   a note on it can name the moment.
6. **After the closed-lid work, a time cut, then "Later, it sleeps as usual"**, with Wakeshot's own going-to-sleep
   chime. No clock and no countdown on screen (Haik, 2026-11-06 00:56: the 5:00 count read as confusing); the app
   sleeps a closed laptop once nothing has happened for 5 minutes (`docs/product.md`), so the words say "Later" and
   claim no time. Before it, the work goes on behind the shut lid (a chip with the agent's next step) and is clearly
   done (the chip's done line, the edge light off at that frame). The time cut is a fade to full black (0.8 s:
   8 frames down, 7 black with the cut inside, 9 back): "Later," lands on the black, and the picture comes back
   under the rest of the line (round 6). Before, a 0.2 s dip to 85% black came out of a shot the focus had already
   dimmed, and Haik read it as a dropped picture. Black with a word on it is the usual sign that time passed; a plain
   slide would make the sleep look immediate. Change it when Haik wants the 5 minutes named.
7. **The art is drawn from the port's pixels, never redrawn as vectors.** `src/art/Glyph.tsx` puts them on a canvas
   1:1 at whole-pixel places, so the title and outro icons are `app_icon(256)` and `app_icon(192)` and the loop's
   ends are `tray_mask(128)`, byte for byte. The port rounds to f32 at every step, as `art.rs` computes, so it draws
   the same bytes, not bytes a level off. Why: a vector redraw anti-aliases its own way and drifts from the app. Change
   it when the video needs the icon at a size between whole pixels (a zoom): then draw the port at each size, never
   scale a bitmap.
8. **The title's light comes on.** It opens on the app icon unlit (plate, shade, shadow and bar as the app icon, no
   glow, the light as a white outline as heavy as the bar, the tray's idle look) and ends on the app icon itself. The
   inside of the light leads (lit by 35% of the fade) and the lead falls off across the outline, so the light fills
   at once and its edge warms after; the glow and bloom spread over the whole 0.44 s (ease out, the loop's curve).
   Why: with the outline leading, as in the loop, the frames in between showed a warm ring round a grey-green middle,
   and a hard edge between the two parts left a seam. Change it when Haik wants the title to open on the lit icon.
9. **H.264 at CRF 18.** CRF 16 kept the round 1 drafts clean but the round 2 draft was 8.5 MB, over the spec's
   8 MB line, so the spec's next step, CRF 18. Not 20: at 20 the app icon's plate broke into blocks while its light
   came on (Part A). Change it when the final passes about 9 MB: then less dither, then 720p as the last resort.
   BT.709, converted and tagged (`--color-space=bt709`, Remotion's own advice and its 5.0 default): with 4.0.540's
   default the file used the BT.601 matrix with no tags, and a player that takes untagged HD as BT.709 shows the
   holding green about 12 levels duller (review m7, round 4); `check-video` checks the tags.
10. **The title and the outro are one stacked lockup** (`docs/brand.md`), the icon over the name, sized by the
    plate's side (`STACKED` in `src/promo/scenes.tsx`): the name's cap height 0.36 of it (the name at half the side,
    117 px under the 256 px icon, 88 px under 192), 0.26 of it from the plate's bottom to the cap line, the name's ink
    centred under the plate, tracking -0.01em as the wordmark. Each line is placed by its cap line and baseline (CSS
    `text-box`), baselines on whole pixels. Why: a 16:9 title wants the stacked form, and with the sizes set by hand
    the title and the outro were two lockups. Change it when a scene needs the name larger than the icon allows.

11. **The dynamic cut (Part C) follows `docs/research/promo-video.md`**, with these choices of the worker. Since
    `p6-agents-video-readme` Part C (2026-11-06) the cut has 9 framings and no menu shot; the menu notes below stay
    for a longer cut.
    - **Ten framings, not nine.** The settings slide starts a beat after the work ends (14 s), so the lid framing is
      4 s; the menu shot (6 s) is two framings with a short pan to the submenu at 23 s. Every framing is 2 to 4 s.
      Change it when Haik wants fewer camera moves.
    - **A capture pops by rising, not by scaling.** Tooltip, menu and submenu fade in over 4 frames and rise 16 px on
      whole pixels, so a capture is never shown at a zoom between whole numbers. Drawn things (chips, badges, the
      card) pop with the spec's scale and spring. Change it when a real capture looks wrong rising.
    - **The camera's offset is rounded to whole pixels at zoom 1**, so captures and the art stay sharp at rest, and
      the move blur is an SVG Gaussian stretched along the move, off while the camera rests. Change it to Remotion's
      `@remotion/motion-blur` only after a spike shows it rebuilds the same bytes.
    - **Round 3: the real captures are in** (2026-11-05). The menu set's battery figure read 96% in S1 and 97% in
      S5a to S5d (it was charging): S5d's battery box (menu closed, as in S1) is drawn over S1 only, so the figure
      never jumps; S5a's box, used before, carried the open menu's shadow and showed as a darker patch (review m2).
      Each set floats on the stage as a picture (deep shadow, 1 px rim, square corners, nothing clipped). The tooltip,
      menu and submenu rise in with their shadows (each box grown 12 to 16 px). After the menu click the frame had
      no subject: now a ring sits on the icon (no dimming) and a round lens rises above it with the tray glyph at
      128 px, its light coming on. Change it when Haik wants a different subject there.
    - **Stand-in captures are plain sketches of each set** (dark taskbar, menu and tooltip boxes, gray bars where words
      go, the port's real tray glyph), tagged in red, not empty boxes, so the cursor, the pops and the ring can be
      judged before the captures exist. The words never appear: UI words come only from the captures.
    - **Round 4: a work chip above the shut laptop (12 to 17 s)** with the terminal's spinner and one step ("run npm
      test"), then its done line; the lock badge beside it comes with the lid chime at 11 s. Round 2 had dropped a
      "still working" chip for pace; Haik's note of 2026-11-06 asks to see the work go on behind the lid, so the chip
      is back. After the review (M1) the shot has one caption and one chip step, and `check-pace` allows at most 5
      things within 2 s, so a shot cannot fill up again unnoticed. Change it when Haik wants the steps back.
    - **The end card's icon is 256 px** (the name 117 px), larger than Part A's 192, so the end card fills the frame.
    - **Round 2, Haik's notes (2026-11-05).** The speed numbers fold into the end card as one line ("Light: ... MB of
      memory idle, ... ms of CPU in 10 minutes.") with the build and date in small print; their 3 s go to a 6 s menu
      shot and a 5 s tray shot. The tray shot shows the cause and the effect in one view: a terminal where Claude
      Code starts and finishes, and the real 200% tray capture at 2x where the icon turns holding and back at those
      same frames (set `tray` in `data/shots.json`, S1 to S4). The laptop is drawn again in graphite with keys,
      touchpad, edges, a lid with thickness and the edge light on its front edge; the terminal, the card, the chips
      and the badge share one finished panel look. Change it when Haik's next look asks.
    - **Contact sheets render at full size and shrink in the script**: Chrome's backdrop filter (the focus dimming)
      drew mirrored copies at Remotion's `--scale=0.25`.
    - **No CSS `blur()` or `drop-shadow()` filters.** A word's blur is its own text shadow (fill hidden) faded
      against the sharp word, the faint terminal behind the name is blurred the same way, and the cursor's shadow is
      a plain offset copy of the arrow. With those filters, a few frames came out one to five levels apart from one
      render to the next (a filter gets its own GPU layer), and the clean rebuild differed. The camera's SVG blur and
      the focus dimming's backdrop filter matched in every repeat and stay. Software GL (`--gl=swangle`) was slower
      (80 s for 116 frames, not 20) and differed more. The terminal also draws no SVG (round 3): under the hook's 3D
      tilt its SVG spinner, check and window marks came out up to 4 levels apart; its spinner is now three text
      dots and its marks plain boxes and text. Compare renders by decoded pixels, not file bytes: Chrome's PNG files
      of equal pixels differ. Change it when Chrome renders filters and SVG the same on every run.
    - **Round 4: a new tree every frame.** The video's root is keyed by the frame, so Chrome paints each frame from
      nothing. Kept from frame to frame, Chrome repainted only what had changed since the frame that render tab drew
      before, and which frame that was depends on how the four tabs share the work: soft shadows and gradients (the
      laptop's floor shadow, the card's shadow, the terminal's title bar) kept a level of the old frame at their edges,
      and two renders differed in 49 of 450 frames tested (that reading of Chrome is ours, not verified; the fix is
      tested). With the key, 271 frames rendered twice were equal. A
      capture's image is keyed by its file, so a swap of shots mounts a new image: a changed source let a render now
      and then show the shot before (round 3, frame 165). Change it when a render gets slow: the remount costs each
      frame its images' decode and the stage's dither again. After the review's fixes one full render still came out
      different at the whip (frame 150 on, in the faint terminal's area): the faint terminal, hundreds of soft letters
      at 7% under the move's blur, now goes as the whip starts.

## Where the video lives

GitHub plays a video inline only as an uploaded attachment: `gh issue comment <n> --attach out/wakeshot-promo.mp4` on
a pinned "Media" issue; the README holds the bare `github.com/user-attachments` URL. Not uploaded yet; the URL goes
here once it is.
