# The promo video: style, storyboard and build plan

Written 2026-11-05 for `p6-media`. It replaces the storyboard and shot list in `docs/research/launch.md` part 4; the
specs, the GitHub facts and the sound source table there still hold. Words on screen are drafts: `p5-text` and
`p6-readme` own the final wording, and every claim is something Wakeshot v1 does (`docs/product.md`).

## Why a new plan

The current draft (`npm run draft`, 30 s) shows the right facts but sits still: eight scenes that fade in and out in
place, small pictures in the middle of a large dark frame, captions of 8 to 10 words, and no camera. Measured on the
draft at 2 fps: no cut, one framing per scene (2 to 5 s each), something new about every 2 s (counted by eye on a
contact sheet). Good product videos of this kind change something every 0.5 to 1 s, move the camera between ideas,
and show one thing at a time, large. This plan keeps the draft's truth rules, art, fonts, colours and pipeline, and
changes how it moves.

## Style rules

Numbers are targets for the 30 s main cut at 1920 x 1080, 30 fps. Measured from well-made product films of this
kind (counted frame by frame and from their sound), then set for Wakeshot.

### Pace

| Rule | Target |
| --- | --- |
| Beat grid | 120 BPM: one beat every 0.5 s (15 frames), one bar every 2 s (60 frames). Every cut, camera move, pop and caption start lands on a beat, within 1 frame |
| Framing (one camera place) | 2 to 4 s, about 3 s on average; 10 framings in 30 s (built: the settings slide starts at 12 s and the menu shot pans once) |
| Something new on screen | on average every 0.5 to 1 s (a move, a pop, a click, a word, a number); never more than 2 s without one, except on the end card |
| Longest still hold | 3.5 s, and even then a slow push runs (below) |
| First 2 s | a full picture from frame 0, the problem shown, sound from frame 0. Frame 0 is never black: the README player may show it before play |
| Name on screen | by 3 s |
| End card | the last 4 s |

### Text

- One idea per line. 2 to 6 words. At most two lines on screen.
- Two kinds of text:
  - **Big words** on an empty stage (hook, speed, end card): Instrument Sans SemiBold 120 to 160 px, centred.
  - **Captions** while a picture is on screen: Instrument Sans Regular 60 px (never under 56), centred, baseline at 86%
    of the frame height (928 px), white text `#EEF2E8`.
- One emphasis word per line, in the holding colour `#B6E05A` and SemiBold. Never more than one.
- Entry, word by word: each word fades 0 to 1, from 8 px blur to 0, and rises 16 px, over 9 frames
  (`Easing.bezier(0.16, 1, 0.3, 1)`), the next word 3 frames later. A 5 word line is fully in after 21 frames (0.7 s).
  Built with the blur as the word's own text shadow, not a CSS filter: filtered frames rendered a level apart from
  run to run, which breaks the same-bytes rebuild (`media/README.md`, decision 11).
- Exit: the whole line fades out with 6 px of blur in 6 frames, starting with the next camera move; on a hard cut it
  goes with the cut.
- A caption stays at least 1.4 s after its last word lands (most get 1.7 s); big words of 1 to 3 words at least
  0.5 s. A viewer with sound off reads every line once.
- About 10 lines in 30 s is the most that fits: each needs about 0.7 s to come in and 1.4 s to be read.
- UI words come only from the real captures. The video never types or redraws a menu or tooltip word.
- Terminal text: JetBrains Mono 30 to 40 px; typing at 1 character per frame (30 per second) for prompts, 2 per frame
  for commands the viewer does not need to read.

### Motion

| Move | How | Length | Easing |
| --- | --- | --- | --- |
| Camera move between near places | the world slides and scales under a fixed frame | 12 to 18 frames (0.4 to 0.6 s) | `Easing.bezier(0.16, 1, 0.3, 1)`: fast start, long settle |
| Whip | the same, faster, with a directional blur that peaks mid-move (up to 40 px along the move) | 9 to 12 frames | same |
| Hard cut | to a far place, always on a beat, with a whoosh into it | 0 | none |
| Slow push during a hold | scale +3% over 2 s, on drawn parts only. While a capture is in view the camera rests at a whole-number zoom; the motion then comes from the cursor, pops and the ring | while holding | linear |
| Pop (chip, badge, card) | scale 0.94 to 1 from its anchor, opacity 0 to 1 in 4 frames. A capture (tooltip, menu, submenu) rises 16 px on whole pixels instead, so it is never scaled | 10 frames | spring, mass 0.6, damping 12, stiffness 200 (about 4% overshoot) |
| Focus | everything but the target drops to 35% brightness and 4 px blur; a 2 px ring in the light's edge colour `#FFEA9C` with a soft 24 px glow wraps the target, 6 px outside it | 8 frames; the ring glides to the next target with the camera easing | |
| Cursor | a drawn arrow; glides on a slight curve, then clicks: scales to 0.9 and back in 6 frames, a ring spreads from the tip in 8 frames; the target answers within 3 frames | glide 10 to 15 frames | `Easing.inOut(Easing.cubic)` |
| The light comes on | the brand's own fade (`src/art/motion.ts`): 13 frames, ease out; a bloom spreads from it over 18 frames | | |
| The light goes off | 24 frames, ease in and out (the loop's off curve) | | |
| Tilt | only two places use 3D: the hook's terminal (8 degrees about Y) and the laptop (three-quarter view, the lid turns on its hinge) | | |

Never: bounces on text, spins, zoom-blur flashes, more than three flashes a second (WCAG 2.3.1), or a picture of a real
capture scaled by anything but a whole number (decision 4 in `media/README.md`).

### Colour and stage

- Stage `#121610` with a soft radial lift `#1A2117` (about 900 px wide) that follows the focal point.
- The one accent is the holding green `#B6E05A`: the light while it holds, and the one emphasis word. The light's warm
  core `#FFFBE8` and edge `#FFEA9C` are for the focus ring and the bloom only.
- Pictures float: big soft shadow (`0 40px 120px rgba(0,0,0,0.55)`) and a 1 px rim `#2D3628`.
- A fixed, seeded dither on the stage background (about 1.5% monochrome noise that does not move with the camera), so
  dark gradients do not band in H.264. Not animated grain: moving noise costs bits on every frame.
- Dark only. The light captures are for the README stills, not the video.

### Sound

- One music track at 120 BPM (or conformed to it), calm electronic: sparse in the hook, full when the light comes on,
  thinned to let the lid chime sit alone, lifted at the menu, resolved on the end card (the last chord rings 1.5 s).
- Every camera move has a whoosh: 0.3 to 0.6 s of filtered noise, loudest at the move's fastest frame. Every click has
  a click. Typing has soft key ticks (at most 12 a second, not one per character). Counters tick at most 8 a second.
- Wakeshot's own chimes where the app plays them: the lid chime at the lid close, the going-to-sleep chime when the
  laptop sleeps.
- Mix: integrated -14 to -16 LUFS, true peak -1 dBTP or lower (ffmpeg `loudnorm`, which Remotion's ffmpeg has). The
  music ducks 4 dB under each chime. No voice.
- The video must make sense with no sound (the README plays it muted); sound is the bonus.

## Storyboard: the 30 s main cut

Frames at 30 fps; a beat is 15 frames. "Drawn" means made in code; "capture" means a real screen picture from the
capture session (shot ids in the next section). The camera works over one large dark world: the terminal, the tray,
the laptop, the settings card and the menu each have a place in it, so near moves are slides and far ones are cuts.

| # | Seconds (frames) | On screen | Motion | Text | Sound |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.0 to 2.0 (0 to 59) | **Hook.** A drawn terminal, large (1300 px wide), tilted 8 degrees, the prompt already typed: `> migrate billing to API v2 and run all tests`. "Agent working." already on screen at frame 0, so the first frame is a full picture | Enter at frame 0; tool lines stream in from frame 3, one every 3 frames, under a "Working" spinner; slow push. Frame 30: the stage dims to 10% in 9 frames, the spinner stops, a small "Sleeping" chip with a moon pops at the top right | Big words over the terminal: "Agent working." (from frame 0), then "PC sleeping." (frame 30, in by frame 42) | Enter key at frame 0; soft hat pulse; from frame 30 the music falls in pitch to a low hum; frames 56 to 59 silent |
| 2 | 2.0 to 5.0 (60 to 149) | **The light.** Black stage, the tray glyph idle (the art port at 320 px, white outline; the loop already draws it at 128 px, so check 320 once). Behind it the terminal comes back, faint, its lines moving again | Frames 60 to 73: the light comes on, the bloom spreads; frame 75: "Wakeshot" rises under it, and the line under the name follows word by word | "Wakeshot" (SemiBold 140 px), then "keeps it **awake** while agents work." (in by frame 102) | Low hit and the full beat at frame 60; a soft rising tone on the light |
| 3 | 5.0 to 10.0 (150 to 299) | **Claude Code turns it green, then lets go** (round 2, Haik's note: show exactly that Claude Code starts work and the icon turns green, then goes back when it finishes). A drawn terminal (the cause) left of the real tray rectangle (the effect): the 200% tray set (S1 to S4) at 2x | Whip, 12 frames: the big glyph flies down and goes out as it lands on the idle icon (S1). Frame 165: Enter, Claude Code starts, and at that frame the icon turns holding (S2), a ring of green spreading from it. The cursor comes to rest on the icon; frame 195 the holding tooltip (S3) rises, ringed (no dimming: the terminal stays in view). Frame 210 the camera drifts 90 px (15 frames). Frame 240 the terminal shows done and at that frame the icon is idle again with the idle tooltip (S4) | "Claude Code starts: it turns **green**." (frame 165, in by 189), then "Done: it lets **go**." (frame 240, in by 258) | Enter and a soft rise at 165; key ticks; pop; a soft whoosh; a soft fall at 240 |
| 4 | 10.0 to 14.0 (300 to 419) | **Lid.** Cut to a drawn laptop (graphite, keys, touchpad, the terminal on its screen), three-quarter view; then shut, a thin holding-green light on the lid's front edge, breathing (2 s period); a lock badge | Camera settles back 15 frames; frames 315 to 330 the lid closes (ease in, a 2 degree settle); frame 360 the lock badge pops; slow orbit of 6 degrees from frame 360 | "Close the **lid**." (frame 315, in by 330), then "Work goes **on**, locked." (frame 360, in by 378) | Whoosh; soft thump and Wakeshot's lid chime at frame 330; the music dips under the chime |
| 5 | 14.0 to 17.0 (420 to 509) | **Done, settings back.** Frame 420: the edge light goes off (24 frames). Frame 435: a drawn card pops beside the laptop: "Power settings", two rows from the capture log (S9) | Camera slides right to the card from frame 420 (15 frames); each value rolls from Wakeshot's to the user's own (8 frames) at frames 465 and 480; the ring moves row to row | "Work done. Your settings, **back**." (frame 435, in by 456) | Soft fall at 420; whoosh; two ticks |
| 6 | 17.0 to 20.0 (510 to 599) | **Then it sleeps.** Back to the shut laptop, its edge dark; a chip "5:00 with no input" | Camera slides back (15 frames); the chip counts 5:00 to 0:00 from frame 525 to 555; frame 555 the power light fades (24 frames); frame 570 a moon rises | "Then it sleeps, as **usual**." (frame 525, in by 546) | Whoosh; Wakeshot's going-to-sleep chime at frame 555 |
| 7 | 20.0 to 26.0 (600 to 779) | **Keep awake on demand.** The menu set (one screen rect, five captures) at rest zoom, the drawn cursor | Cursor glides in and clicks the icon at frame 615; the menu (S5a) rises from the taskbar; frame 645 the cursor reaches Keep awake: S5b (hover, submenu open); frame 690 it reaches "1 hour": S5c, and the camera pans 150 px to the submenu (12 frames); click at frame 720; the menu closes and the icon turns holding (S5d); ring on the icon; the cursor fades | "Keep awake on **demand**." (frame 645, in by 663), then "No second **tool**." (frame 720, in by 735) | Clicks at 615 and 720; pops |
| 8 | 26.0 to 30.0 (780 to 899) | **End card.** The stacked lockup (icon 256 px): the app icon unlit, its light comes on at frames 780 to 793, "Wakeshot" under it. The speed numbers fold in here (round 2) | The lines under the lockup pop in whole (no word reveal); still for the last 3 s | Frame 795: "Free and open source. Windows 11, x64 and ARM64." Frame 810: "Light: {ram} MB of memory idle, {cpu} ms of CPU in 10 minutes." from `data/speed.json`, with "Measured on Wakeshot <build>, <date>, x64." in small print. Frame 825: the install line in mono | One soft swell of air as the light comes on; no whoosh, sweep or pops; the music plays on a beat, fades over 2.5 s into a soft echo and is silent 0.1 s before the end |

Counts (round 2): 8 shots, 10 framings in 30 s (3 s on average), 12 caption lines plus the big words; `check-pace.mjs`
counts 120 new things before the end card (tool lines and words included), the longest gap 1.8 s. Every caption
holds 1.4 s or more after it is in; every start is on a beat.

Round 2 (Haik's look at the first draft): "at some point maybe a little fast". The densest stretches were 8, 12 and
19.5 s (6 events in 2 s each); the speed numbers folded into the end card, as planned, and their 3 s went to the
menu shot (6 s) and to the new tray shot (5 s). A longer cut (about 45 s) can add, in this order: builds too
(`wakeshot run -- npm run build` and its tooltip), the morning notice after a closed-lid night, the battery floor.

Rules kept from the old storyboard: no fake numbers (shot 8 waits for the measurements, and the final render refuses
stand-ins); no agent shown that v1 does not detect; the terminals are illustrations, not copies of Claude Code's
screen, and use only our colours; nothing private in any capture. One more: every scene is our own idea; no scene,
layout or line is taken from another app's video.

## Real captures and drawn parts

### What Part B captures from the real app

Dark taskbar, at the highest scale the DELL screen offers (see question 6), cursor hidden, plain wallpaper, nothing
private in view. Stills only: no screen video is needed, because every motion is made in Remotion.

Captures come in **sets**: several pictures of one screen rectangle at one scale, so the video swaps them in place
and only the drawn layers move.

| Id | Capture | Why real | Used in shot |
| --- | --- | --- | --- |
| Tray set: S1, S2, S3, S4 | One fixed rect over the tray icon and the space above it, at 200% on the DELL screen (round 2): idle icon (S1), holding icon (S2), the holding tooltip of a real Claude Code session, "Holding awake: Claude Code (1 session)" (S3), the idle tooltip (S4) | proof that it sees Claude Code and lets go when the work ends; the words are the app's | 3 |
| Menu set: S1, S5a to S5d | One rect over the tray icon and the space the menu and submenu take: idle icon, menu closed (its own S1); menu open, no hover (S5a); Keep awake hovered with its submenu open (S5b); "1 hour" hovered (S5c); menu closed after the click, icon holding (S5d) | the menu words never change; the real hover look, so no drawn highlight is needed | 7 |
| S9 | Before and after values of the lid action and sleep timers, from the capture session's log, as text | the card shows only real values | 5 |
| Speed | The numbers from `measure-speed.ps1` | | 8 (end card) |
| S6, S7 | The morning notice after a closed-lid hold; the tooltip while `wakeshot run -- npm run build` holds | README stills and the longer cut | none in this cut |

For each menu capture, record the rows' boxes in capture pixels in `data/shots.json` (a `rows` list: name, x, y, w,
h), measured once by a script from the picture, so the cursor and the ring land on the real rows at any zoom. The pop
of the menu in shot 7 is the menu's part of S5a, masked out and scaled from the icon over S1.

A real Claude Code session is needed only to make S3 (and to measure the hold delays). Its screen is not shown.

### What is drawn in Remotion, and why

| Part | Why drawn |
| --- | --- |
| The tray glyph and the app icon at hero size | the art port draws the app's exact pixels at any whole size, so the light can fill the frame and stay sharp; a capture zoomed that far would turn to blocks |
| The terminals (Claude Code and the build) | an illustration avoids showing another company's screen and anything private; it types per character and stays sharp at any zoom |
| The laptop and its lid | there is no lid to film in code; a simple drawing in our colours reads at once |
| The settings card | Windows shows the two values in different places; one card with the logged values says it in one look |
| Counters, big words, captions, end card | text and numbers belong to the video, not to the app |
| Cursor, focus ring, dimming, blur, bloom, moon, lock badge, chips | overlays that direct the eye; the real UI under them stays untouched |

Real captures carry the proof (the real words, the real taskbar); drawn parts carry the motion. Real captures are
shown at whole-number zoom only (1x, 2x, 3x), so a capture is never blurred into something it was not.

## The build

### Remotion parts to add (in `media/src/`)

Keep `art/`, `fonts.ts`, `theme.ts`, `copy.ts`, the `Shot` component, the build scripts and the checks. Add:

| File | What it does |
| --- | --- |
| `beat.ts` | `BPM = 120`, `beat(n)`, `bar(n)`: frame numbers. `timeline.ts` places every scene and sound with them |
| `motion/Camera.tsx` | keyframes `{at, x, y, zoom, ease}` over the world; returns the transform and the velocity per frame |
| `motion/DirectionalBlur.tsx` | an SVG Gaussian blur along the camera's velocity (deterministic, cheap; built: 0.25 px of smear per px a frame of speed, capped at 40 px, so with this easing it is strongest at the move's start, not its middle). Fallback: `<HtmlInCanvasMotionBlur>` from `@remotion/motion-blur` 4.0.540 if the cheap blur looks fake (needs a spike: render cost and byte-for-byte rebuild) |
| `motion/Focus.tsx` | dims and blurs all but a rectangle; draws and glides the ring |
| `motion/Words.tsx` | the word-by-word caption and the big-word variant, with one emphasis word |
| `motion/Cursor.tsx` | the drawn arrow, its glide path and click |
| `motion/Pop.tsx` | the spring pop from an anchor |
| `motion/Counter.tsx` | count-up with tabular digits (built: each digit in a fixed-width box, so the font's figures do not matter; Instrument Sans's own tabular figures not checked) |
| `motion/Stage.tsx` | the stage colour, the radial lift that follows the focus, the fixed dither |
| `promo/LaptopTilt.tsx` | the three-quarter laptop with CSS 3D (perspective about 1600 px), the lid on its hinge, the edge light |
| `promo/SettingsCard.tsx` | the two rows and the odometer roll |
| `scripts/check-pace.mjs` | reads the timeline and fails when an event is off the beat grid by more than 1 frame, a gap between events is over 2 s (end card excepted), a framing is over 4 s, a caption holds less than 1.4 s after its last word, or a caption has more than 6 words |

Also built (2026-11-05): `motion/ease.ts` (the easings in one place), `promo/Capture.tsx` (a shot of a set at whole
zoom, or its stand-in sketch), `promo/Terminal.tsx`, `promo/world.ts` (where each thing sits in the world),
`parts/Parts.tsx` (the parts' own compositions), `scripts/contact-sheet.mjs`, `scripts/music-candidate.mjs` and
`scripts/lib/audio.mjs` (WAV, BS.1770 loudness, a look-ahead limiter). `data/shots.json` now holds the two sets with
their boxes and rows.

Animate only with `useCurrentFrame()` and `interpolate()` (CSS animations do not render). Use `Easing.spring()` and
`Easing.bezier()`; both are in Remotion 4.0.540 (read in `node_modules/remotion/dist/cjs/easing.d.ts`). Any new
`@remotion/*` package is pinned to 4.0.540, as Remotion needs all its packages on one version.

### Tools besides Remotion

| Need | Tool | Status |
| --- | --- | --- |
| Stills of the tray, tooltip and menu | the spike's `BitBlt` capture (`spikes/tray/src/capture.rs`), a screen rect for the tooltip; per-monitor DPI aware | as in `launch.md` part 4; Part B |
| Row boxes of the menu captures | a small Node script in `media/scripts/` | new |
| Loudness and conforming a found track to 120 BPM | Remotion's own ffmpeg: it has `loudnorm` and `atempo` (read in its build line, 2026-11-05) | nothing to install |
| Contact sheets for review | `scripts/contact-sheet.mjs`: renders only the picked frames (`--frames` takes a comma list) at full size, shrinks and tiles them. Not `--scale=0.25`: Chrome's backdrop filter (the focus dimming) draws mirrored copies at that scale (seen 2026-11-05) | built |
| Screen video, cursor logs, scene detection | not needed | none |

No new system-wide install. Remotion's bundled ffmpeg lacks most filters (no `select`, `fps` or `tile`), so do not
plan on it for analysis; the pace check reads the timeline data instead of pixels.

### How to brief the agent that builds it

One `wakeshot-worker` on `p6-media`, with this file as the spec. The brief says:

1. Install Remotion's agent skills for this project (`npx remotion skills add` in `media/`; the official skills,
   `remotion-dev/skills`, version 4.0.541 on 2026-11-05) or read them from GitHub. Where a skill names an API, check it
   against the installed 4.0.540 types before using it.
2. The style rules above are acceptance criteria, with numbers. `check-pace.mjs` holds the timeline to them.
3. Build the motion parts first and show each in Remotion Studio on its own composition (`Focus`, `Words`, `Camera`
   with a test world), then the scenes, one at a time, in storyboard order.
4. Self-review loop after each scene: render that scene's frames at quarter size, tile them at 4 a second into a
   contact sheet, look at it, and compare with the storyboard row (what is on screen, when, how big). Fix, then the
   next scene. Full-size renders only for Haik's looks and the final.
5. Keep every existing rule: captures only from `data/shots.json`, whole-number zoom for captures, the art port's
   pixels, no em dash or emoji, `npm run check`, `check-video`, `check-rebuild`, and the DRAFT mark on drafts.
6. One change at a time after Haik's first look; each of his notes is one change with its own draft.
7. Laptop: renders at the default concurrency of 4; never two renders at once.

### Music and effects

| Source | Licence | In the repo and on social media | Note |
| --- | --- | --- | --- |
| Wakeshot's own chimes (`chime.rs`, rendered by `make-sounds.mjs`) | MIT, ours | yes | the lid and going-to-sleep sounds |
| Made in code: whooshes (filtered noise sweeps), clicks, ticks, and the current music bed | MIT, ours | yes | deterministic; follows the timeline exactly. Risk: the bed sounds thin for this pace |
| A 120 BPM sequencer in code (kick, hats, bass, pluck, pad) | MIT, ours | yes | the upgrade of the current bed; still no third-party file |
| Kenney audio packs (Interface Sounds, UI Audio) | CC0 | yes | clicks and pops if the code ones sound cheap |
| Freesound, filtered to CC0 | CC0 per sound | yes | whooshes and music loops; check each page's licence and that the uploader made it |
| Free Music Archive, OpenGameArt, itch.io packs, filtered to CC0 | CC0 per track | yes | music; check each track's page |
| CC BY 4.0 music (for example incompetech.com) | CC BY | yes, with the credit line in `CREDITS.md` and in every post's text | the site generates the credit line per track |
| Pixabay music | Pixabay Content License: "You cannot sell or distribute Content ... on a Standalone basis" | no: a raw file in a public repo is standalone distribution | |
| Remotion's sample effects (`remotion.media/*.wav`) | no licence stated in the skill | no | several are meme sounds |
| Sonniss bundles, YouTube Audio Library | redistribution limits, YouTube terms | no | |

Pick, for Haik's yes: the agent finds three CC0 tracks between 110 and 130 BPM (calm electronic, a clear lift after
a sparse intro), cuts each to 30 s on the storyboard, conforms it to 120 BPM with `atempo`, and Haik picks one by ear.
Effects are made in code, Kenney CC0 where a code sound is weak; the chimes are ours. If none of the three tracks
pleases him, the 120 BPM sequencer in code is the fallback. Every file, its link and its licence go into
`media/CREDITS.md`.

Built 2026-11-05 (Part C): the code bed is a 120 BPM sequencer in `make-sounds.mjs`, shaped as the "Sound" rules say.
A found track is fitted by `scripts/music-candidate.mjs`: its tempo found by autocorrelation (not checked by ear),
conformed with `atempo`, cut so the moment it lifts out of its intro lands on the light (2 s); the bars before the lift
play softly under the hook with a short silence before the light (a found track cannot fall in pitch like the code
bed), a 5 dB dip under the lid chime, a fade over the end card. The music stem is set to -20 LUFS before the mix, so
every candidate sits at the code bed's level; the mix is -15 LUFS with sample peaks under -2 dBFS. Freesound needs a
login to download, so the three candidates come from OpenGameArt and the Free Music Archive (`media/CREDITS.md`).

Round 2 (Haik's notes, 2026-11-05): his pick is candidate 3, "A Chill Fever (Loopable)" by Pro Sensory (CC0), now in
`media/public/music/` and fitted on every build (`data/music.json`). "The ending sounds are not good": the track is
59 s with its lift at 44 s, so it ran out at 25.2 s and the end card opened on silence, then a synthetic sweep, a
whoosh and two pops. Now the loopable track plays three times over before it is cut, so it never runs out; on the end
card it plays on for a beat, fades over 2.5 s into a soft ping-pong echo and is silent 0.1 s before the end; the
lockup has one soft unpitched swell of air (the track's key could not be pinned down, so no added chord).

## Formats

| Where | What plays | File |
| --- | --- | --- |
| README | only an uploaded attachment plays inline: the bare `github.com/user-attachments/assets/...` URL on its own line shows a player, muted, with controls, no autoplay, no loop. Limits: "10MB for videos" on a free plan; `.mp4`, `.mov`, `.webm`; "we recommend using H.264" (GitHub docs, read 2026-11-05) | MP4, H.264 and AAC, 1920 x 1080, 30 fps, 30 s, under 10 MB |
| Pages | `<video autoplay muted loop playsinline poster>` with the same MP4 in `site/` | the same file and its poster PNG |
| X, LinkedIn, YouTube | the 16:9 master plays on all three. Reported limits (third-party guides, not checked on the sites): X up to 140 s for a standard account, any shape from 1:3 to 3:1; LinkedIn 16:9, 1:1 or 9:16 | the master |
| Optional 1:1 cut | 1080 x 1080, for LinkedIn and X feeds on phones, where a square shows larger; captions at 72 px | a second composition over the same world and timeline; the camera frames each place for the square |
| 9:16 | not worth it now: Wakeshot is a desktop tool and its viewers come from GitHub, X and LinkedIn | none |
| Store (later) | trailer 1920 x 1080, 60 s or less (`launch.md` part 3) | the master |

Size: the dynamic cut will be larger than the 2.5 MB draft. Keep CRF 16 if the file stays under 8 MB; else CRF 18,
then less dither. Measured on the first full draft (2026-11-05, stand-in captures): 7.8 MB at CRF 16, so CRF 16 stays
for now; the real captures may push it over 8 MB.

## Effort and risks

### Effort

| Round | Who | Work | Laptop |
| --- | --- | --- | --- |
| 1 | one `wakeshot-worker` (about 200K tokens) | motion parts, the 9 shots with stand-in captures, the beat timeline, `check-pace.mjs`, effects in code, three music candidates; a full draft for Haik | about 20 quarter-size scene renders (under 30 s each) and 2 or 3 full drafts (2 to 4 min each at concurrency 4) |
| Haik | | watches the draft (10 min), picks the music (10 min), answers the questions below | |
| 2 | the same worker, Part B as already planned | captures at the agreed scale, row boxes, speed numbers, final render, all checks, Haik's yes | the capture session (Haik off the PC about 10 min) and one final render |
| 3 (only if wanted) | one worker (about 60K tokens) | the 1:1 cut | one render |
| Review | one `wakeshot-reviewer` | fresh-eyes pass on the final against this file | none |

So: 2 agents in 2 rounds for the main cut, plus a reviewer; 3 rounds with the square cut.

### Risks

1. **Too fast for a muted README viewer.** Check: watch the draft at 900 px wide with sound off; every line read once?
   Lever: fold the speed numbers into the end card.
2. **File over 10 MB.** Motion, blur and dither cost bits. Lever: CRF 18, less dither, 720p as the last resort.
3. **Motion blur.** The cheap SVG blur may look fake; Remotion's own motion blur needs Chrome's HTML-in-canvas and
   renders each blurred frame several times; whether it rebuilds to the same bytes (`check-rebuild`) is not known.
   Spike it on one whip before using it.
4. **Captures at 150% limit the zoom.** At whole-number zoom the menu set fits the 1080 p frame only at 1x (2x is too
   wide); a capture at 200% (question 6) shows it a third larger and sharper at the same 1x.
5. **Overlay alignment.** The cursor and ring must land on the real rows; the row boxes in `shots.json` fix that,
   measured per capture.
6. **Truth.** Every number from the measurement, every setting value from the log, every UI word from a capture, no
   agent but Claude Code. The final render already refuses stand-ins and missing numbers.
7. **Music taste.** Only Haik can judge it; three candidates keep that to one short listen.
8. **Skill and version drift.** The skills track Remotion's newest version (4.0.541) and we pin 4.0.540; an API may be
   newer than ours. The brief says to check the installed types first.

## Questions for Haik

Each with the default the worker takes if there is no answer. All seven defaults taken on 2026-11-05 ~21:00 as
orchestrator decisions for Haik's review (Haik away: "can start media ... even without question"); question 6 is
asked again before the capture session.

1. **Music:** a found CC0 track that you pick from three, or music made in code? Default: three CC0 candidates, you
   pick one; code is the fallback. Answered by Haik, 2026-11-05: candidate 3, "A Chill Fever (Loopable)".
2. **Length:** keep 30 s, as the card says? Default: yes. Builds (`wakeshot run`), the morning notice and the battery
   floor wait for a longer cut; if 30 s still feels rushed, the speed numbers fold into the end card.
3. **Hook:** open on the problem (an agent at work, then the PC going to sleep) and show the name at 2 s? Default: yes.
4. **Terminal:** a drawn illustration in our colours, or a real capture of a Claude Code session? Default: drawn.
5. **Social cuts:** a 1:1 cut for LinkedIn and X? Default: only the 16:9 master now; the square later if you plan to
   post on LinkedIn.
6. **Capture scale:** can you set the DELL screen to 200% (or its highest) for the capture session, by hand? Default:
   yes; else 150%, and the menu stays at 1x.
7. **Where this plan lives:** default `docs/research/promo-video.md`, with the storyboard in `launch.md` part 4
   replaced by a pointer to it.

## Sources

Read 2026-11-05.

- Remotion's agent skills: `github.com/remotion-dev/skills` (last commit 2026-11-05, skill version 4.0.541): the
  router `remotion-best-practices`, and in `remotion-markup` the files on timing, transitions, motion blur and sound
  effects; `remotion-create/video-layout.md` ("Decide what the viewer should notice first in each scene").
- Remotion docs: "Claude Code" (`remotion.dev/docs/ai/claude-code`: `npx remotion skills add`), "Motion blur"
  (`remotion.dev/docs/motion-blur`: `<HtmlInCanvasMotionBlur>`, `<CameraMotionBlur>`, `<Trail>`).
- Briefing an agent for a Remotion video: Tella, "How to Use Remotion Agent Skills With Claude Code", 2026-10-12
  ("Treat the first prompt as a brief, not a magic render button"; one change at a time; keep the preview running).
- GitHub docs, "Attaching files" (limits and formats quoted above).
- Pixabay licence summary (`pixabay.com/service/license-summary/`); Kenney Interface Sounds (`kenney.nl`, CC0).
- Social video limits: third-party guides only, not checked on X or LinkedIn.
- Our own reads: Remotion 4.0.540 types in `media/node_modules` (`Easing.spring`, `perceptual-scale`), the build line
  of Remotion's bundled ffmpeg (filters it has and lacks), and the current draft measured at 2 fps.
