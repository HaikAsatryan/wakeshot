# Credits and licences

Every file the media use or make, where it comes from, and its licence. `node scripts/check.mjs` fails when a file
under `public/` or `out/` has no line here. Wakeshot's own work is MIT (`../LICENSE`).

## Pictures

None yet. The video's screen pictures come from the capture session of `p6-media` Part B, each with a line here
(file, what it shows, where it was taken). Until then the draft draws a plain box naming each missing shot
(`data/shots.json`). The captures will show parts of the Windows taskbar around Wakeshot's own icon and menu.
Microsoft's terms for showing Windows screenshots: not checked.

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
| `out/wakeshot-promo.mp4` | `src/promo/`, the pictures, the sound and the fonts above. Not built yet: waits for the real captures and the speed numbers | MIT, ours |
| `out/wakeshot-promo-poster.png` | A still of the video (frame 75). Not built yet | MIT, ours |

The drafts in `out/draft/` are not kept in git and carry a DRAFT mark on every frame.

## Tools (not shipped in any output)

| Tool | Licence |
|---|---|
| Remotion 4.0.540 (`remotion`, `@remotion/cli`, `@remotion/media`, `@remotion/fonts`) | Remotion License: free for an individual and for companies with up to 3 employees; source-available, not open source (`LICENSE.md` in `remotion-dev/remotion`, read for `p6-launch-research`) |
| FFmpeg and ffprobe, as bundled in `@remotion/compositor-win32-x64-msvc` 4.0.540 | Not checked: the package ships no licence text of its own |
| Chrome Headless Shell, downloaded by Remotion into `node_modules/.remotion` on the first render | Not checked |
| React and React DOM 19.3.1, `@types/react` 19.3.0, TypeScript 7.0.2 | MIT, MIT, Apache-2.0 (each `package.json`) |
