# Credits and licences

Every file the media use or make, where it comes from, and its licence. `node scripts/check.mjs` fails when a file
under `public/` or `out/` has no line here. Wakeshot's own work is MIT (`../LICENSE`).

## Pictures

Real screen captures of Wakeshot's own tray icon, tooltip and menu, made by `p6-media-captures` on 2026-11-05 from the
installed local 0.1.0 build (commit c9da3dd) on the DELL U2723QE at 200%, cut unchanged from its grabs (its
`captures.json` is the manifest; `data/shots.json` carries the boxes and rows). The captures show parts of the Windows
taskbar around Wakeshot's icon (the chevron, OneDrive, ENG, network, volume, battery); TrafficMonitor and the clock are
covered with plain taskbar. Wakeshot's own icon, menu and words: MIT, ours. Microsoft's terms for showing Windows
screenshots: not checked.

| File | What | Source |
|---|---|---|
| `public/shots/work-S1-dark.png` | Set `tray`, S1: idle icon, no tooltip | p6-media-captures 2026-11-05 22:07:06.485, DELL 200%, screen rect 2986,1920 564x240, from grab S1-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/work-S2-dark.png` | Set `tray`, S2: holding icon (a real Claude Code session), no tooltip | p6-media-captures 2026-11-05 22:09:23.556, DELL 200%, screen rect 2986,1920 564x240, from grab S2-dark.png; tray: Holding awake: Claude Code (1 session) |
| `public/shots/work-S3-dark.png` | Set `tray`, S3: holding tooltip of a real Claude Code session, the icon hovered | p6-media-captures 2026-11-05 22:09:25.889, DELL 200%, screen rect 2986,1920 564x240, from grab S3-dark.png; tray: Holding awake: Claude Code (1 session) |
| `public/shots/work-S4-dark.png` | Set `tray`, S4: idle tooltip, the icon hovered | p6-media-captures 2026-11-05 22:07:33.434, DELL 200%, screen rect 2986,1920 564x240, from grab S4-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/menu-S1-dark.png` | Set `menu`, S1: idle icon, menu closed (battery 96%) | p6-media-captures 2026-11-05 22:07:06.485, DELL 200%, screen rect 2870,1346 970x814, from grab S1-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/menu-S5a-dark.png` | Set `menu`, S5a: menu open, no hover | p6-media-captures 2026-11-05 22:08:28.607, DELL 200%, screen rect 2870,1346 970x814, from grab S5a-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/menu-S5b-dark.png` | Set `menu`, S5b: Keep awake hovered, its submenu open | p6-media-captures 2026-11-05 22:08:30.722, DELL 200%, screen rect 2870,1346 970x814, from grab S5b-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/menu-S5c-dark.png` | Set `menu`, S5c: 1 hour hovered | p6-media-captures 2026-11-05 22:08:32.343, DELL 200%, screen rect 2870,1346 970x814, from grab S5c-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/menu-S5d-dark.png` | Set `menu`, S5d: menu closed after the click, icon holding (Keep awake, 1 h left) | p6-media-captures 2026-11-05 22:08:35.342, DELL 200%, screen rect 2870,1346 970x814, from grab S5d-dark.png; tray: Holding awake: Keep awake (1 h left) |
| `public/shots/readme-menu-dark.png` | README: the menu open (dark) | p6-media-captures 2026-11-05 22:08:28.607, DELL 200%, screen rect 3140,1610 700x550, from grab S5a-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/readme-menu-light.png` | README: the menu open (light) | p6-media-captures 2026-11-05 22:11:59.431, DELL 200%, screen rect 3140,1610 700x550, from grab S5a-light.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/readme-holding-tooltip-dark.png` | README: the holding tooltip of a real Claude Code session (dark) | p6-media-captures 2026-11-05 22:09:25.889, DELL 200%, screen rect 2986,1920 564x240, from grab S3-dark.png; tray: Holding awake: Claude Code (1 session) |
| `public/shots/readme-holding-tooltip-light.png` | README: the holding tooltip of a real Claude Code session (light) | p6-media-captures 2026-11-05 22:12:09.093, DELL 200%, screen rect 2986,1920 564x240, from grab S3-light.png; tray: Holding awake: Claude Code (1 session) |
| `public/shots/readme-idle-tooltip-dark.png` | README: the idle tooltip (dark) | p6-media-captures 2026-11-05 22:07:33.434, DELL 200%, screen rect 2986,1920 564x240, from grab S4-dark.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/readme-idle-tooltip-light.png` | README: the idle tooltip (light) | p6-media-captures 2026-11-05 22:13:43.491, DELL 200%, screen rect 2986,1920 564x240, from grab S4-light.png; tray: Not holding, the PC sleeps as usual |
| `public/shots/work-S7-dark.png` | README and a longer cut: the tooltip while wakeshot run -- npm run build holds (dark) | p6-media-captures 2026-11-05 22:08:46.310, DELL 200%, screen rect 2986,1920 564x240, from grab S7-dark.png; tray: Holding awake: npm |

## Sound

| File | Source | Licence |
|---|---|---|
| `public/music/a-chill-fever.mp3` | "A Chill Fever (Loopable)" by Pro Sensory, from https://opengameart.org/content/a-chill-fever-loopable (file `a_chill_fever_0.mp3`, 59.1 s, MP3 192 kbit/s, SHA-256 `9ae1326374674c1de8719b4d952394992ad819f12cf7730705be2ad05a1dfe49`), unchanged. Haik's pick of 2026-11-05 from three CC0 candidates. `make-sounds.mjs` fits it to the video on every build with the numbers in `data/music.json` (130 BPM conformed to 120, its lift at 44.33 s on the light) | CC0: the page's License(s) block reads "CC0", and the author writes "all my music is public domain" (read 2026-11-05) |
| `public/sound/soundtrack.wav` | Made by `scripts/make-sounds.mjs`: the music above fitted to the video, the effects from sine waves and seeded noise, Wakeshot's own two chimes rendered from the numbers in `crates/wakeshot-windows/src/sound/chime.rs`. Not kept in git; every build makes it again | MIT, ours, over the CC0 music |

The code bed (a 120 BPM sequencer in `make-sounds.mjs`) is the fallback when `data/music.json` names no file. The
launch research picked Kenney (CC0) effects; the code effects are used instead, so there is no third-party effect
file. The other two music candidates were not used and are not in the repo. The tempo was found by the script's
autocorrelation, not checked by ear.

## Fonts

| File or package | Font | Licence |
|---|---|---|
| `public/fonts/InstrumentSans-Regular.woff2` | Instrument Sans Regular, Version 1.000, from `fonts/webfonts/` of https://github.com/Instrument/instrument-sans at commit `7fa22308a3d0c94ee2b3cd537a1196b65db34a3e` (the commit Google Fonts also ships), unchanged. "Copyright 2022 The Instrument Sans Project Authors (https://github.com/Instrument/instrument-sans)" | SIL Open Font License 1.1, no Reserved Font Name |
| `public/fonts/InstrumentSans-SemiBold.woff2` | Instrument Sans SemiBold, the same source, commit and copyright | Same |
| `public/fonts/OFL.txt` | The licence file of the same commit, unchanged | Same |
| `@fontsource/jetbrains-mono` 5.3.0 | JetBrains Mono, "Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono)" | SIL Open Font License 1.1 |

The copyright lines are quoted from `OFL.txt` and from the package's `LICENSE`. The OFL allows using the fonts in
pictures and video; the font files are not shipped in any output, only drawn into it.

## Outputs

| File | Made from | Licence |
|---|---|---|
| `out/icon-loop-light.gif` | `src/loop/IconLoop.tsx` and the tray glyph, the held light (`src/art/`), written by `scripts/lib/gif.mjs` | MIT, ours |
| `out/icon-loop-dark.gif` | Same | MIT, ours |
| `out/wakeshot-promo.mp4` | `src/promo/`, the pictures, the sound and the fonts above, the speed numbers of `data/speed.json` | MIT, ours, over the CC0 music and the OFL fonts |
| `out/wakeshot-promo-poster.png` | A still of the video (frame 120) | MIT, ours |
| `out/state-holding-light.png` | The tray glyph, holding, for the light taskbar: `tray_mask` at 32 px from the art port (`src/art/heldLight.ts`), written by `npm run states` | MIT, ours |
| `out/state-holding-dark.png` | Same, for the dark taskbar | MIT, ours |
| `out/state-idle-light.png` | Same, idle, light taskbar | MIT, ours |
| `out/state-idle-dark.png` | Same, idle, dark taskbar | MIT, ours |
| `out/state-off-light.png` | Same, off, light taskbar | MIT, ours |
| `out/state-off-dark.png` | Same, off, dark taskbar | MIT, ours |

The drafts in `out/draft/` are not kept in git and carry a DRAFT mark on every frame.

## Tools (not shipped in any output)

| Tool | Licence |
|---|---|
| Remotion 4.0.540 (`remotion`, `@remotion/cli`, `@remotion/media`, `@remotion/fonts`) | Remotion License: free for an individual and for companies with up to 3 employees; source-available, not open source (`LICENSE.md` in `remotion-dev/remotion`, read for `p6-launch-research`) |
| FFmpeg and ffprobe, as bundled in `@remotion/compositor-win32-x64-msvc` 4.0.540 | Not checked: the package ships no licence text of its own |
| Chrome Headless Shell, downloaded by Remotion into `node_modules/.remotion` on the first render | Not checked |
| React and React DOM 19.3.1, `@types/react` 19.3.0, TypeScript 7.0.2 | MIT, MIT, Apache-2.0 (each `package.json`) |
