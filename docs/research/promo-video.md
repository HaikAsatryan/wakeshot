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
| Framing (one camera place) | 2 to 4 s, about 3 s on average; 9 framings in 30 s since Part C (the menu shot and its pan left); 11 in 34 s since Part D; 12 in 38 s since Part E (the pain has four: typing close, working close, the laptop, sinking) |
| Something new on screen | on average every 0.5 to 1 s (a move, a pop, a click, a word, a number); never more than 2 s without one, except on the end card |
| Most to take in | at most 5 things within any 2 s before the end card: cuts, moves, pops, clicks, lights, counts, rings, swaps and line starts (tool lines and single words not counted; a cut counts once with what it opens on). Added in round 4 after the review found the lid shot back at 6 to 8 in 2 s, the measure Haik's "a little fast" note was closed by |
| Longest still hold | 3.5 s, and even then a slow push runs (below) |
| First 2 s | a full picture from frame 0, sound from frame 0. Frame 0 is never black: the README player may show it before play. Since Part E frame 0 is the terminal with its command line and the empty input box, the caret blinking; the first key lands at 0.4 s. The problem comes at 7.5 to 8.5 s, after the prompt and the waiting (Haik: the first scene was too short, then: a person types, the agent loads) |
| Name on screen | by 9 s (was 3 s, then 5 s in Part C of `p6-agents-video-readme`, 2026-11-06, when the pain and Haik's question "Tired of sleepy laptops?" came first in 4 s; Part D gave the pain 8 s at Haik's word; Part E, his typing opening, 12 s, so the name lands at 12.5 s and the rule is by 13 s). Change it when a viewer test shows people leave before the name |
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
  for commands the viewer does not need to read. Since Part E the pain's prompt is typed by a person: about 11 keys a
  second, 2 to 5 frames apart (`KEYS` in `timeline.ts`), the terminal's 36 px type seen at zoom 1.45 to 1.5 (about
  52 px); the agent then shows mostly a spinner and a thinking line, a tool line only now and then.

### Motion

| Move | How | Length | Easing |
| --- | --- | --- | --- |
| Camera move between near places | the world slides and scales under a fixed frame | 12 to 18 frames (0.4 to 0.6 s) | `Easing.bezier(0.16, 1, 0.3, 1)`: fast start, long settle |
| Whip | the same, faster, with a directional blur that peaks mid-move (up to 40 px along the move) | 9 to 12 frames | same |
| Hard cut | to a far place, always on a beat, with a whoosh into it | 0 | none |
| Time cut (built in round 4, once, at 17 s; reshaped in round 6) | a hard cut back to the same place, later, hidden in a fade to full black: 8 frames down, 7 black with the cut inside, 9 back, the picture fully back on the next beat; "Later," lands on the black (the line starts at the cut, drawn over the black) and the picture comes back under the rest of the line; a soft whoosh at the cut. Full black, because the shot before is already dimmed to 35% by the focus and the round 4 dip (85% for 0.2 s, the words only after it) read as a dropped picture (Haik, 2026-11-06) | 24 frames (0.8 s) | sine in and out |
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

- No music before the light (Part E, Haik, 2026-11-06 23:02: "First music is not required ... we only here that typing
  sounds and sleeping process and we start music when starting to introducing our app"). The opening is a quiet room:
  the keys as the prompt is typed, a faint room tone and the laptop's fan, then the laptop going to sleep (a soft
  tick, a low glide falling two octaves, the fan winding down), the question in near silence, and a 0.5 s riser of
  air into the light.
- From the light, one music track at 120 BPM (or conformed to it): it comes in on its lift, a downbeat, with the low
  hit, and plays at one level to the end card (Part D, Haik: "music integration is little weird because it kind of
  becomes more silent at some points"). It fades into a soft echo on the end card. The plan asked for calm
  electronic; Haik's pick (2026-11-06) is a funky disco loop, with three options of the same type in Part D.
- Level, measured on the music alone (`make-sounds.mjs`, 3 s short-term loudness every 0.5 s, only windows wholly
  after the light): within 2 LU of its median to the end card; the build stops past that. A slow leveler
  (`lib/fit-music.mjs`, at most 6 dB, the gain moving along a raised cosine every 0.25 s) evens a track out first.
- The opening's levels (set by script, not heard): typing about -25 LUFS short-term, 9 to 10 LU under the music; the
  room and fan about -36; the power down about -29; the keys peak about -9 dBFS.
- From the light on, every camera move has a whoosh: 0.3 to 0.6 s of filtered noise, loudest at the move's fastest
  frame (the opening's cut and moves are silent: only keys and the sleep there). Every click has a click. Typing is one
  real keystroke per key (CC0 takes, "Music and effects"), about 11 a second. Counters tick at most 8 a second.
- Wakeshot's own chimes where the app plays them: the lid chime at the lid close, the going-to-sleep chime when the
  laptop sleeps.
- Mix: integrated -14 to -16 LUFS, true peak -1 dBTP or lower (ffmpeg `loudnorm`, which Remotion's ffmpeg has). Under
  each chime the music gives way only in the chime's band: a 6 dB bell cut at 1.5 kHz (Q 0.8, so 1 to 3 kHz: C6, G6
  and their overtones), eased in over 0.1 s and out over 0.3 s, the way a mix makes room for a voice. Until Part D
  the whole music ducked 4 dB under each chime and 5 dB more under the lid chime, one of the dips Haik heard. No
  voice.
- The video must make sense with no sound (the README plays it muted); sound is the bonus.

## Storyboard: the 38 s main cut

Part C of `docs/tasks/p6-agents-video-readme.md` (Haik, 2026-11-06 18:45: "show how one of agents working and laptop
sleeps and tell tired of sleepy laptops and start our logo", the agents "dynamically", designed by us). It replaces
the storyboard Haik signed off in round 6 (git history has it): the hook is now the pain on a laptop and the question,
a new agents shot follows the name, the tray, lid, settings and sleep shots carry over 180 frames later, and the menu
shot left. The brand, the music, the beat grid and the checks are kept. Part D (Haik, 2026-11-06 20:45: "First sleep
scene is too short, it's ok if we pass 30 second threshold, just make it more professional") made the pain 8 s, so
every shot after it started 4 s (8 beats) later and the cut was 34 s. Part E (Haik, 2026-11-06 23:02: "Imagine
terminal is open and someone is typing something and we only here that typing sounds and sleeping process and we
start music when starting to introducing our app. Usually agents in terminal are not typing so fast, usually they
loading a lot.") made the pain a person's story in 12 s: every shot after it starts 4 s later again, the cut is 38 s,
and the music starts on the light.

Frames at 30 fps; a beat is 15 frames. "Drawn" means made in code; "capture" means a real screen picture from the
capture session (shot ids in the next section). The camera works over one large dark world: the pain's laptop, the
light and the agent tiles, the tray, the lid's laptop, the settings card and the end card each have a place in it, so
near moves are slides and far ones are cuts.

| # | Seconds (frames) | On screen | Motion | Text | Sound |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.0 to 12.0 (0 to 359) | **The pain** (Part E: a person's story). First a drawn terminal window seen close and straight on (1500 x 906, our colours, not a copy of any app): `PS C:\code\shop> claude` already run, the agent's input box empty with its caret blinking, so frame 0 is a full picture. Then the same screen on the drawn laptop of the lid shot, three-quarter view, in the middle of the frame | Frames 0 to 89: close on the input box (zoom 1.45 to 1.5, a slow push), the prompt's 36 px type at about 52 px; from frame 12 a person types "migrate billing to API v2", one key every 2 to 5 frames (about 11 a second, a little slower after a space and before "API"), the caret solid while keys come. Frame 90: Enter; the box's frame goes, the prompt stays as sent, a turning spinner and "Thinking" with a slow shimmer and the agent's clock come in, and the camera moves out to most of the window (zoom 1.25 to 1.29). The clock runs in real time to frame 126, then speeds up over 1.5 s to a time lapse (150 clock seconds a second): about 4 minutes at the cut, 9 when the laptop sleeps. Tool lines now and then (frames 132, 171, 219, 243), never evenly. Frame 195: a cut to the laptop with the same screen (zoom 1 to 1.03). Frame 225: the idle timer dims the screen to 40% over 10 frames; the agent still works. Frame 255: the screen goes off over 6 frames, the work stops, the power light starts to breathe (2 s), a "Sleeping" chip with a moon pops beside the lid. Frame 285: the camera eases back (zoom 0.88) while the laptop, its chip and the stage's lift fade out over 12 frames, so from frame 297 the stage is plain and empty. Frame 300: the question, alone | No words over the terminal or the laptop: the typed prompt, the thinking line and the chip tell it (big words would cover the prompt as it is typed). "Tired of sleepy laptops?" (frame 300, in by 318, holds 1.4 s; no green word: green means awake) | No music. From frame 0 a faint room tone and the laptop's fan. One real keystroke per typed key, each from its own side of the keyboard; shift just before "API"; a firmer Enter at 90. The waiting is silent but for the room and fan. At 255 the laptop's own sound of going to sleep: a soft tick, a low glide falling two octaves, a breath of air, and the fan winding down over 1.6 s. The question in near silence; over the beat before the light a riser of air (345 to 360) |
| 2 | 12.0 to 15.0 (360 to 449) | **The light.** A hard cut to the black stage: the tray glyph idle at 320 px, alone | Frames 360 to 373: the light comes on, the bloom spreads; frame 375: "Wakeshot" rises under it and the line under the name follows word by word | "Wakeshot" (SemiBold 140 px), then "keeps it **awake** while agents work." (in by 402) | The music starts here on its lift, a downbeat, with a low hit and the end of the riser; a soft rising tone on the light |
| 3 | 15.0 to 19.0 (450 to 569) | **The agents.** The lit glyph at the top; under it one tile per agent whose live test passed (`media/src/agents.ts`, A to Z): the name in Instrument Sans SemiBold 46 px, an example task under it in the terminal's face (`copy.ts`, by name), a turning spinner. One row up to three tiles, two rows from four; at most six, since the two Copilots share one tile once both pass. Names only, no owner's logo ("Agent logos" below). An agent that drops out takes its tile with it; the grid centres itself | Frame 450: the camera moves down 210 px (15 frames) while the glyph rises with it and shrinks from 320 to 160 px, drawn from the port at each whole size; the name and its line fade. Frame 465: the tiles come in one after another, 5 frames apart, each rising 28 px with the pop's spring (one group, like the words of a line); a slow push to zoom 1.03 until the whip. Frame 525: a ring of the holding green spreads from the light: it holds for every one of them | "Stays **awake** for these agents." (frame 495, in by 516, holds 1.8 s). Not "works with": OpenAI asks non-partners not to write "worked with", and the line says what Wakeshot does | Whoosh at 450; one soft tick per tile (6 a second at most); a soft rise at 525 |
| 4 | 19.0 to 24.0 (570 to 719) | **The agent turns it green, then lets go** (round 2, Haik's note, kept). The drawn terminal (`claude`, "fix the failing tests", one status line, 30 px) left of the real tray rectangle, the 200% tray set (S1 to S4) at 2x | Whip, 12 frames: the glyph flies from the top of the agents shot to the capture's idle icon (S1), going out as it flies, lands on it at frame 578 and fades into it by 582. Frame 585: Enter, the agent starts, and at that frame the icon turns holding (S2), a ring of green spreading from it. The cursor comes to rest on the icon; frame 615 the holding tooltip (S3) rises, ringed. Frame 630 the camera drifts 90 px. Frame 660 the terminal shows done and at that frame the icon is idle again with the idle tooltip (S4) | "Your agent starts: it turns **green**." (frame 585, in by 609), then "Done: it lets **go**." (frame 660, in by 678) | Enter and a soft rise at 585; pop; a soft whoosh; a soft fall at 660 |
| 5 | 24.0 to 28.0 (720 to 839) | **Lid, and the work goes on behind it** (round 4, kept). Cut to the drawn laptop, three-quarter view; then shut, a thin holding-green light on the lid's front edge, breathing. Above it, in one row: a lock badge and a work chip | Camera settles back 15 frames; frames 735 to 750 the lid closes; frame 750 the lock badge pops with the lid chime; frame 780 the work chip rises: "run npm test" under the turning spinner; frame 825 the chip says "Done. All 128 tests pass." and at that frame the edge light goes off: the hold ends; slow orbit of 6 degrees from 780 | "Close the lid: work goes **on**." (frame 735, in by 759), gone with a fade at 825 | Whoosh; soft thump and Wakeshot's lid chime at 750, the music keeps its level and gives way only in the chime's band ("Sound"); a pop at 780; a soft fall at 825 |
| 6 | 28.0 to 31.0 (840 to 929) | **Done, settings back** (kept). Frame 855: the "Power settings" card pops right of the laptop, two rows from the capture log (S9); the work chip stays in view with its done line | Camera slides right from 840 (15 frames), zoom 0.92 (drawn parts only); each value rolls back to the user's own at 885 and 900; the ring moves row to row | "Work done. Your settings, **back**." (frame 855, in by 876) | Whoosh; pop; two ticks |
| 7 | 31.0 to 34.0 (930 to 1019) | **Later, it sleeps** (round 6, kept). A time cut back to the shut laptop, its edge dark, the lock badge alone above it; the same laptop that slept under the agent in shot 1, now asleep once the work is done | Black from 921, full 929 to 935 (the cut at 930 inside it), back by 945; slow push of 3%; frame 960 the power light fades and the badge goes; frame 975 a moon rises | "Later, it sleeps as **usual**." (frame 930, on the black, "Later," in by 939) | Soft whoosh at the cut; Wakeshot's going-to-sleep chime at 960 |
| 8 | 34.0 to 38.0 (1020 to 1139) | **End card** (kept). The stacked lockup, the app icon's light coming on at 1020 to 1033; the lines under it; the small print at the bottom now ends "Agent names are trademarks of their owners; Wakeshot is not affiliated with them." (Google asks for a trademark line where its names show) | The lines pop in whole; still for the last 3 s | Frame 1035: "Free and open source. Windows 11, x64 and ARM64." Frame 1050: the speed line from `data/speed.json` with the measured line and the trademark line at 28 px. Frame 1065: the install line in mono | One soft swell of air as the light comes on; the music fades over 2.5 s into a soft echo and is silent 0.1 s before the end |

Counts (Part E, 2026-11-06, with the final v1 list: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code and
OpenCode, so four tiles: Claude Code, Codex CLI, Copilot, OpenCode, the two Copilots sharing one): 8 shots, 12
framings in 38 s (2 to 4 s each), 12 lines of words (the question, the name and its line, 6 captions, the end card's
3); `check-pace.mjs` counts 47 starts on the beat, 90 new things before the end card, the longest gap 1.9 s, at most
5 things to take in within any 2 s; the name at 12.5 s. The poster is frame 420 (the light on, the name and its line
in).

Out of this cut: the menu shot ("Keep awake on demand", 6 s). Its time paid for the longer pain (4 s, was 2 s; 8 s since
Part D, 12 s since Part E) and the agents shot (4 s). Keep awake stays in the README and the site. Change it when Haik wants Keep awake in the video: in
a longer cut, or in place of the settings shot (the menu's clicks need 4 to 5 s, so the cut grows by 1 to 2 s). A
longer cut (about 45 s) can add, in this order: Keep awake from the menu, builds (`wakeshot run -- npm run build` and
its tooltip), the morning notice after a closed-lid night, the battery floor.

Rules kept from the old storyboard: no fake numbers (the end card waits for the measurements, and the final render
refuses stand-ins); no agent shown whose live test did not pass (`agents.ts`; the `preview` prop of the video exists
only for stills of a layout and no build passes it); the terminals and the tiles are illustrations, not copies of any
agent's screen, and use only our colours; no owner's logo without its rules allowing it; nothing private in any
capture. Every scene is our own idea; no scene, layout or line is taken from another app's video.

## Real captures and drawn parts

### What Part B captures from the real app

Dark taskbar, at the highest scale the DELL screen offers (see question 6), cursor hidden, plain wallpaper, nothing
private in view. Stills only: no screen video is needed, because every motion is made in Remotion.

Captures come in **sets**: several pictures of one screen rectangle at one scale, so the video swaps them in place
and only the drawn layers move.

| Id | Capture | Why real | Used in shot |
| --- | --- | --- | --- |
| Tray set: S1, S2, S3, S4 | One fixed rect over the tray icon and the space above it, at 200% on the DELL screen (round 2): idle icon (S1), holding icon (S2), the holding tooltip of a real Claude Code session, "Holding awake: Claude Code (1 session)" (S3), the idle tooltip (S4) | proof that it sees Claude Code and lets go when the work ends; the words are the app's | 4 |
| Menu set: S1, S5a to S5d | One rect over the tray icon and the space the menu and submenu take: idle icon, menu closed (its own S1); menu open, no hover (S5a); Keep awake hovered with its submenu open (S5b); "1 hour" hovered (S5c); menu closed after the click, icon holding (S5d) | the menu words never change; the real hover look, so no drawn highlight is needed | none since Part C (was the menu shot); kept for a longer cut, and still referred to by `data/shots.json` (set `menu`), `media/CREDITS.md` (its rows and the grain source), `src/promo/Capture.tsx` and `world.ts`; `build.mjs` checks every set's files before a final |
| S9 | Before and after values of the lid action and sleep timers, from the capture session's log, as text | the card shows only real values | 5 |
| Speed | The numbers from `measure-speed.ps1` | | 8 (end card) |
| S6, S7 | The morning notice after a closed-lid hold; the tooltip while `wakeshot run -- npm run build` holds | README stills and the longer cut | none in this cut |

For each menu capture, record the rows' boxes in capture pixels in `data/shots.json` (a `rows` list: name, x, y, w,
h), measured once by a script from the picture, so the cursor and the ring land on the real rows at any zoom. The pop
of the menu in the menu shot was the menu's part of S5a, masked out and scaled from the icon over S1.

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
conformed with `atempo`, cut so the moment it lifts out of its intro lands on the light (2 s then, 4 s since Part C, 8 s since Part D);
the bars before the lift play softly under the hook with a short silence before the light (a found track cannot fall
in pitch like the code bed), a 5 dB dip under the lid chime, a fade over the end card. The music stem is set to -20 LUFS before the mix, so
every candidate sits at the code bed's level; the mix is -15 LUFS with sample peaks under -2.5 dBFS (-2 until the
Part C draft read -1.01 dBTP after AAC, too close to the limit). Freesound needs a login to download, so the three
candidates come from OpenGameArt and the Free Music Archive (`media/CREDITS.md`).

Round 2 (Haik's notes, 2026-11-05): his pick is candidate 3, "A Chill Fever (Loopable)" by Pro Sensory (CC0), now in
`media/public/music/` and fitted on every build (`data/music.json`). "The ending sounds are not good": the track is
59 s with its lift at 44 s, so it ran out at 25.2 s and the end card opened on silence, then a synthetic sweep, a
whoosh and two pops. Now the loopable track plays three times over before it is cut, so it never runs out; on the end
card it plays on for a beat, fades over 2.5 s into a soft ping-pong echo and is silent 0.1 s before the end; the
lockup has one soft unpitched swell of air (the track's key could not be pinned down, so no added chord).

Round 5 and 6 (2026-11-06): Haik asked to hear other music on the final picture. Four new CC0 options (OpenGameArt
and the Free Music Archive, each licence read on the track's own page) were fitted and muxed onto the final; he
picked option 4, "Funky Disco Beats to Boogie/Woogie to" by Fupi (CC0), now in `media/public/music/` with its numbers
in `data/music.json` (110.25 BPM, conformed with `atempo` 1.0884; its lift at 45.75 s on the light). "A Chill Fever"
left the repo. The fit and the shape are unchanged: the same storyboard cut, the dip under the lid chime and the end
card's fade and echo. Tempo and key were found by script (A minor, so the C6 and G6 chimes are in key), not by ear.
The options, their pages and their readings are in `docs/tasks/p6-media.md`, round 5.

Part D of `p6-agents-video-readme` (Haik, 2026-11-06 20:45): "music integration is little weird because it kind of
becomes more silent at some points. Also I like this sound and the type of this sound and want to see alternative
options as well (not older ones) that are around the same type and can have little bass as well. But entertaining."
The fit's shape changed ("Sound" above): one level from the first frame, a slow leveler, the muffle while the laptop
sleeps, the chimes' band cut in place of the ducks. The old shape is gone from the fit (the code bed keeps its own).

Three new options, found by a helper agent on the Free Music Archive and OpenGameArt (ten tracks downloaded, nine
CC0 and one CC BY 4.0), picked by numbers, not by ear: same type (disco or funk tags), 105 to 130 BPM, a steady
level, a key with C and G in it (the chimes are C6 and G6), and more bass than the current track where small speakers
play it (100 to 250 Hz). Each licence read on the track's own page, 2026-11-06 (the helper, then again by curl).

| # | Track | Page and licence | BPM, key (by script) | Energy in 40-100 / 100-250 / 250-2k / 2k-8k Hz | Lift (orig. s) | After AAC: LUFS, dBTP, LRA | Music 3 s loudness: outside the muffle, in it |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | "Funky Disco Beats to Boogie/Woogie to", Fupi (the current pick) | https://opengameart.org/content/funky-disco-beats-to-boogiewoogie-to, "CC0" | 110.25, A minor | 78 / 11 / 5 / 3% | 45.75 | -15.25, -1.45, 5.4 | -0.5 to +0.4 LU, -2.2 LU |
| 1 | "Can't Stop My Feet !", Loyalty Freak Music (album "ROLLER DISCO DANCE DANCE") | https://freemusicarchive.org/music/Loyalty_Freak_Music/ROLLER_DISCO_DANCE_DANCE/Loyalty_Freak_Music_-_ROLLER_DISCO_DANCE_DANCE_-_04_Cant_Stop_My_Feet_/, "licensed under a CC0 1.0 Universal License"; the page says "Instrumental: No" | 130, A minor | 52 / 26 / 11 / 7% | 59.07 (bar 32; a 2-bar break at 44 to 48 s kept out of the cut) | -15.26, -1.82, 4.4 | -0.3 to +0.4 LU, -1.1 LU |
| 2 | "Lost In The Rythm", HoliznaCC0 (album "Lost") | https://freemusicarchive.org/music/holiznacc0/lost/lost-in-the-rythm/, "licensed under a CC0 1.0 Universal License"; "Instrumental: Yes" | 105, A minor | 56 / 20 / 5 / 0% | 27.43 (bar 12) | -15.27, -1.58, 5.0 | -0.2 to +0.2 LU, -0.1 LU |
| 3 | "Funked Up", Joth | https://opengameart.org/content/funked-up, "License(s): CC0"; its notice asks for credit to Joth (CC0 does not require it; `CREDITS.md` would give it) | 116, A minor | 24 / 45 / 30 / 1% | 16.55 (bar 8; a 66 s loop, so no loop seam in the cut) | -15.23, -2.27, 5.0 | -0.5 to +0.4 LU, -0.3 LU |

Files: options 1 and 2 are MP3s from the Free Music Archive's stream link (the download button needs a login), 185.6
and 160.0 s, SHA-256 `5cedc66e...` and `53a5c022...`; option 3 the page's MP3, 66.3 s, SHA-256 `57cd8125...`. None is
in the repo; the build folder of `p6-agents-video-readme` holds them. To use one: copy it unchanged into
`media/public/music/`, set `data/music.json` (`file`, `bpm`, `lift` as above, title, author, page), write its
`CREDITS.md` line, then `npm run video` and the checks. Each was rendered once that way, every check passing (card
Result, Part D). Not picked: "Make Funk" (HoliznaCC0, 98.75 BPM, too far from 120), "Rhythm Garden" (tempo unsure),
"Raspberry Jam" (C sharp major), "High Funktioning ..." (D sharp minor, and its title), "Roller Fever" and "Grab A
Partner" (Loyalty Freak Music, less bass), "Funky Chunk" (Kevin MacLeod, CC BY 4.0: a credit in every post; also very
widely used).

Part E (Haik, 2026-11-06 23:02: no music until Wakeshot comes in; the opening is typing and the laptop going to
sleep). The fit now starts each track on its lift at the light, with a 10 ms fade in, and nothing before; the muffle
is gone, since nothing plays while the laptop sleeps. The leveler and the evenness rule count only the windows wholly
after the light (a window that reached back into the silence read low and would have pushed the first bars up). The
same four tracks, lifts and tempos as Part D, so the music from the light on is the same cut of each. Haik's pick,
2026-11-06 about 23:50, after listening: the current track, "Funky Disco Beats" ("ok our music was the best"). Options
1 to 3 in the table above were offered and not picked, since Haik picked the current track; none of them is in the
repo. The new cut was rendered with it and with options 1 and 2 (every check passing); the run with option 3 was
stopped at Haik's pick.

The keys are real keystrokes. A helper agent searched OpenGameArt, Wikimedia Commons, Kenney, BigSoundBank, Freesound
and others (2026-11-06; its notes are in the card's build folder) and downloaded three, each licence read on its own
page:

| Recording | Page and licence | Why picked or not |
| --- | --- | --- |
| "Keyboard Soundpack #1 [Typing and Single Keystrokes]", unicaegames | https://opengameart.org/content/keyboard-soundpack-1-typing-and-single-keystrokes, "License(s): CC0" and "Free to use however you like." | picked: 32 single keys of 0.25 s each (WAV, 44.1 kHz), so one take per typed key; a Cherry KC 1000, a low-profile office keyboard, close and dry (the floor between strokes about -70 dBFS, by script). 18 takes are in `media/public/keys/`, unchanged |
| A laptop typing take, johndekale, OpenGameArt | CC0 | not picked: one 5 s OGG (lossy), its floor about 20 dB higher |
| A laptop on a wooden desk, Wikimedia Commons | public domain | not picked: rumble and birdsong (the page says so), from a 128k MP3 |

Not downloaded: BigSoundBank's two "Slow MacBook Pro keyboard" takes (CC0 on the page, but the site blocks scripted
downloads; a person can fetch them by hand) and Freesound's CC0 MacBook takes (a login is needed for the originals;
its terms say nothing either way about the public previews, so none was used). Rejected by licence: SoundBible
(personal use or CC BY 3.0), a Keychron take (CC BY 4.0, and mechanical), an Internet Archive item labelled CC0 whose
own text says its files come from Mixkit, Pixabay and SoundBible.

Which take plays which key was picked by numbers, not by ear (`make-sounds.mjs`, `KEY_TAKES`): letters from 15
mid-level takes that have their key-up after them; the space bar the one deep take (a fifth of its energy under
300 Hz); Enter a firm take with more low end; shift a soft take with no key-up. Each take is cut 3 ms before its
stroke, its top softened a little (half of it through a 6.5 kHz low-pass, heard from the seat, not the close
microphone), its peak set, then each key varies by up to 1.5 dB. The room, the fan, the power down and the riser are
made in code.

## Agent logos

Step 8 of Part C: before any agent's logo goes in, each owner's own brand or trademark page was read for our use (a
"works with" row in the video and the README of a free MIT app, the file committed to the public repo, drawn unchanged
on the dark stage, no endorsement meant). Read 2026-11-06 by a helper agent of `p6-agents-video-readme` (WebFetch, and
curl where WebFetch got 403); quotes are exact. A press kit download is not a permission.

| Agent | Owner | Page read | The rule that decides it | Verdict |
| --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `anthropic.com/legal/trademark-guidelines` (PDF "Effective Aug 1, 2024"); the press kit at `anthropic.com/press-kit` | "You may only use our trademarks as specifically permitted by us and only in materials we approve beforehand." Trademarks include "our names, logos, brand elements". The press kit has a Claude Code logo (Ivory, for dark) but no usage text | forbidden without Anthropic's prior approval |
| Codex | OpenAI | `openai.com/brand` (with its "Non-partnerships" and "API Developers" boxes) | "Don't: Use the logo without permission or outside OpenAI's terms." "All co-branded materials must undergo an approval process by both brands". There is no Codex logo, only the OpenAI wordmark and Blossom. Non-partners: "please don't use "collaborated with," "worked with," or "partnered with," in any form." Allowed: "you may truthfully identify the OpenAI technology you use" | unclear, treated as no |
| GitHub Copilot | GitHub | `brand.github.com/foundations/logo` (from `github.com/logos`), its Copilot and co-branding pages, `docs.github.com` logo policy | "Show integration: Use a permitted GitHub logo to inform others that your project integrates with GitHub." Also: less prominent than our own name, unchanged, never "combining with other words or design elements". But the same page: "Do not use GitHub trademarks, logos, or artwork without GitHub's prior written permission." Since 2025 Copilot has no standalone icon logo; the white "GitHub Copilot" lockup is in `brand.github.com/GitHub_Logos.zip` | allowed, with one open point (the written-permission line; not asked of GitHub) |
| Cursor | Anysphere | `cursor.com/brand`, `cursor.com/terms-of-service` (no trademark or press page exists) | The brand page gives files "to represent Cursor" and nothing on third-party use; the terms: "There are no implied licenses". Name rule: "Refer to us as Cursor. Not Cursor AI" | unclear, treated as no |
| Antigravity | Google | `antigravity.google/press`, Google's Partner Marketing Hub ("How to show that your business uses or works with Google technology", the trademark guidelines and terms) | Plain text is welcome: "you could say your product or service "works with," "is compatible with," or "integrates with" a Google product." Icons are not: "To use a Google product icon in your work, create a Partner Marketing Hub account ... and request permission through our approval form." With Google's names: "include a legal line in the footer or other appropriate area" | forbidden without Google's approval; plain text allowed with a legal line |
| OpenCode | Anomaly (`anomalyco/opencode`; `sst/opencode` redirects there) | `opencode.ai/brand`, its terms; the next brand text in the repo (`services/site/src/lib/brand-content.ts`, not live) | The live page has assets and no rules; MIT covers the code, not the name. The repo's coming text allows "works with" only as an OpenCode badge or a partner lockup, and says do not "put other words next to the logo" | unclear today, treated as no |

Our call (principal designer): **names only, in our own type, for every agent**; no logo file goes into `media/`.
Why: four owners say no or say nothing a stranger can rely on, and the one that allows it (GitHub) also asks for
written permission in the same page. One logo among five names would also make an uneven row and put one agent
first by look, which the A to Z order is there to avoid. Three owners (GitHub, Google, the coming OpenCode text)
forbid words next to their mark, so a logo would have to replace the name, not sit beside it.

What the video does instead, from the same pages: the line over the tiles says "Stays awake for these agents." (not
"works with", for OpenAI's non-partner rule); each name is written the owner's way ("Cursor", "OpenCode"); and the end
card's small print says "Agent names are trademarks of their owners; Wakeshot is not affiliated with them." (Google's
legal line; the others' no-endorsement rules).

Change it when every owner of a shown agent allows its logo for this use, in writing or in rules a stranger can rely
on: then the official file goes in `media/public/logos/` unchanged, credited in `media/CREDITS.md`, the logo takes the
name's place in its tile (not beside it), smaller than the Wakeshot name, and this table gets the new source.

Outside the logo question (not a legal reading, not verified how Google applies it): the Antigravity terms say "You
must not abuse, harm, interfere with, or disrupt the Service. This includes, but is not limited to, using the Service
in connection with products not provided by us." Their example is reaching the service with its sign-in, which
Wakeshot does not do. Listed for Haik before a public "Antigravity" claim. Moot since 2026-11-06 23:21: Antigravity
and Cursor left v1, so neither is named in the video or the docs.

## Formats

| Where | What plays | File |
| --- | --- | --- |
| README | only an uploaded attachment plays inline: the bare `github.com/user-attachments/assets/...` URL on its own line shows a player, muted, with controls, no autoplay, no loop. Limits: "10MB for videos" on a free plan; `.mp4`, `.mov`, `.webm`; "we recommend using H.264" (GitHub docs, read 2026-11-05) | MP4, H.264 and AAC, 1920 x 1080, 30 fps, 38 s since Part E (34 s in Part D, 30 s before), under 10 MB |
| Pages | `<video autoplay muted loop playsinline poster>` with the same MP4 in `site/` | the same file and its poster PNG |
| X, LinkedIn, YouTube | the 16:9 master plays on all three. Reported limits (third-party guides, not checked on the sites): X up to 140 s for a standard account, any shape from 1:3 to 3:1; LinkedIn 16:9, 1:1 or 9:16 | the master |
| Optional 1:1 cut | 1080 x 1080, for LinkedIn and X feeds on phones, where a square shows larger; captions at 72 px | a second composition over the same world and timeline; the camera frames each place for the square |
| 9:16 | not worth it now: Wakeshot is a desktop tool and its viewers come from GitHub, X and LinkedIn | none |
| Store (later) | trailer 1920 x 1080, 60 s or less (`launch.md` part 3) | the master |

Size: the dynamic cut will be larger than the 2.5 MB draft. Keep CRF 16 if the file stays under 8 MB; else CRF 18,
then less dither. Measured on the first full draft (2026-11-05, stand-in captures): 7.8 MB at CRF 16; the round 2 draft was
8.5 MB at CRF 16, so from round 3 (real captures) the build uses CRF 18. The final (round 6, 2026-11-06: the new
music and the reshaped time cut) is 7.02 MB at CRF 18, BT.709 (converted and tagged; Remotion 4's default was BT.601
with no tags), -15.3 LUFS, true peak -1.1 dBTP. The round 4 final was 7.14 MB, -15.4 LUFS, -1.4 dBTP. The Part C
draft (2026-11-06, four agent tiles) is 7.49 MB and 30.02 s, -15.3 LUFS, -1.62 dBTP; the same cut rendered as a
final, 7.37 MB (a check only; the round 6 final stays in `media/out/` until Haik's look). The Part D cut (2026-11-06, 34 s) is 8.45 MB (8,447,495 bytes)
with each of the four tracks: the same picture, AAC at 192 kbit/s. Over the 8 MB line above, under GitHub's 10 MB and
`media/README.md` decision 9's 9 MB trigger. The Part E cut (2026-11-06, 38 s, the typing opening, the music from the
light) with "Funky Disco Beats" is 8.50 MB (8,497,349 bytes), 38.02 s, -15.32 LUFS, -1.47 dBTP, LRA 16.2 LU (the quiet
opening widens it; the music alone sits within 0.5 LU of its median), and is the final in `media/out/` since Haik's
yes to it. Its draft, with the DRAFT mark, is 9.10 MB. Measure the true peak on
the rendered MP4 (Remotion's ffmpeg, `loudnorm=print_format=json`, `input_tp`): the same WAV encoded by that ffmpeg
directly (`-c:a aac -b:a 192k`) read -0.55 dBTP where Remotion's render of it read -1.01 (2026-11-06), so a quick
encode is no stand-in for the render.

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
   agent whose live test did not pass (`media/src/agents.ts`), no owner's logo. The final render already refuses
   stand-ins and missing numbers.
7. **Music taste.** Only Haik can judge it; three candidates keep that to one short listen.
8. **Skill and version drift.** The skills track Remotion's newest version (4.0.541) and we pin 4.0.540; an API may be
   newer than ours. The brief says to check the installed types first.

## Questions for Haik

Each with the default the worker takes if there is no answer. All seven defaults taken on 2026-11-05 ~21:00 as
orchestrator decisions for Haik's review (Haik away: "can start media ... even without question"); question 6 is
asked again before the capture session.

1. **Music:** a found CC0 track that you pick from three, or music made in code? Default: three CC0 candidates, you
   pick one; code is the fallback. Answered by Haik, 2026-11-05: candidate 3, "A Chill Fever (Loopable)". Changed by
   Haik, 2026-11-06: option 4 of four new ones, "Funky Disco Beats to Boogie/Woogie to" by Fupi.
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
