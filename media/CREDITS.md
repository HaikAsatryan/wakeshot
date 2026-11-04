# Credits and licences

Every file the media use or make, where it comes from, and its licence. `node scripts/check.mjs` fails when a file
under `public/` or `out/` has no line here. Wakeshot's own work is MIT (`../LICENSE`).

## Pictures

| File | Source | Licence |
|---|---|---|
| `public/shots/tray-dark-idle.png` | Stand-in: `docs/research/img/ux/taskbar-dark-idle.png`, a screen capture of Wakeshot's demo icon on Haik's taskbar | MIT, ours |
| `public/shots/tray-dark-holding.png` | Stand-in: `docs/research/img/ux/taskbar-dark-holding.png`, same | MIT, ours |
| `public/shots/tooltip-dark-idle.png` | Stand-in: `docs/research/img/ux/taskbar-tooltip-dark.png`, Wakeshot's tooltip on Haik's taskbar | MIT, ours |
| `public/shots/menu-light-main.png` | Stand-in: `docs/research/img/ux/menu-screen-light-main.png`, Wakeshot's menu on Haik's screen | MIT, ours |
| `public/shots/menu-light-sub.png` | Stand-in: `docs/research/img/ux/menu-screen-light-sub.png`, same | MIT, ours |
| `public/shots/menu-dark-main.png` | Stand-in: `docs/research/img/ux/menu-screen-dark-main.png`, same | MIT, ours |
| `public/shots/menu-dark-sub.png` | Stand-in: `docs/research/img/ux/menu-screen-dark-sub.png`, same | MIT, ours |

The captures show parts of the Windows taskbar around Wakeshot's own icon and menu. Microsoft's terms for showing
Windows screenshots: not checked.

## Sound

| File | Source | Licence |
|---|---|---|
| `public/sound/soundtrack.wav` | Made in code by `scripts/make-sounds.mjs`: the music bed and the effects from sine waves and seeded noise; Wakeshot's own two chimes rendered from the numbers in `crates/wakeshot-windows/src/sound/chime.rs`. Not kept in git; every build makes it again | MIT, ours |

No outside sound or music is used. The launch research picked Kenney (CC0) effects and a Freesound (CC0) music loop;
the worker made everything in code instead, so there is no third-party file at all (reason in `README.md`).

## Fonts

| Package | Font | Licence |
|---|---|---|
| `@fontsource/inter` 5.3.0 | Inter, "Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter)" | SIL Open Font License 1.1 |
| `@fontsource/jetbrains-mono` 5.3.0 | JetBrains Mono, "Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono)" | SIL Open Font License 1.1 |

The licence lines are quoted from each package's `LICENSE`. The OFL allows using the fonts in pictures and video; the
font files are not shipped in any output, only drawn into it.

## Outputs

| File | Made from | Licence |
|---|---|---|
| `out/icon-loop-light.gif` | `src/loop/IconLoop.tsx` and the Sunrise art (`src/art/sunrise.ts`, a checked port of `crates/wakeshot-windows/src/tray/art.rs`) | MIT, ours |
| `out/icon-loop-dark.gif` | Same | MIT, ours |
| `out/wakeshot-promo.mp4` | `src/promo/`, the pictures and the sound above, Inter and JetBrains Mono. Not built yet: waits for the real captures and the speed numbers | MIT, ours |
| `out/wakeshot-promo-poster.png` | A still of the video (frame 75). Not built yet | MIT, ours |

The drafts in `out/draft/` are not kept in git and carry a DRAFT mark on every frame.

## Tools (not shipped in any output)

| Tool | Licence |
|---|---|
| Remotion 4.0.540 (`remotion`, `@remotion/cli`, `@remotion/media`, `@remotion/fonts`) | Remotion License: free for an individual and for companies with up to 3 employees; source-available, not open source (`LICENSE.md` in `remotion-dev/remotion`, read for `p6-launch-research`) |
| FFmpeg and ffprobe, as bundled in `@remotion/compositor-win32-x64-msvc` 4.0.540 | Not checked: the package ships no licence text of its own |
| Chrome Headless Shell, downloaded by Remotion into `node_modules/.remotion` on the first render | Not checked |
| React and React DOM 19.3.1, `@types/react` 19.3.0, TypeScript 7.0.2 | MIT, MIT, Apache-2.0 (each `package.json`) |
