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
| `public/sound/soundtrack.wav` | Made in code by `scripts/make-sounds.mjs`: the music bed and the effects from sine waves and seeded noise; Wakeshot's own two chimes rendered from the numbers in `crates/wakeshot-windows/src/sound/chime.rs`. Not kept in git; every build makes it again | MIT, ours |

No outside sound or music is used. The launch research picked Kenney (CC0) effects and a Freesound (CC0) music loop;
the worker made everything in code instead, so there is no third-party file at all (reason in `README.md`).

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
