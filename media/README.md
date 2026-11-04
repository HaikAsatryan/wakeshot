# Wakeshot media

The icon loop for the README top, the 30 second promo video and its poster, built in code with
[Remotion](https://www.remotion.dev/) so they can be made again when the icon or the menu changes. Outside the Cargo
workspace; no Rust here. Card: `docs/tasks/p6-media.md`. Specs and storyboard: `docs/research/launch.md` part 4.

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
| `out/icon-loop-light.gif`, `out/icon-loop-dark.gif` | `npm run loop` | 480 x 240, 25 fps, 5 s, loops forever. Show at 240 px wide for sharp HiDPI |
| `out/wakeshot-promo.mp4`, `out/wakeshot-promo-poster.png` | `npm run video` | Refuses while any picture is a stand-in or a speed number is missing |
| `out/draft/wakeshot-promo-draft.mp4` and its poster | `npm run draft` | Stand-ins allowed; DRAFT on every frame; not kept in git |
| `out/wakeshot-promo-poster.png` alone | `npm run poster` | Frame 75 of the video, for the Pages `<video poster>` |
| `public/sound/soundtrack.wav` | `npm run sounds` | Also run by `draft` and `video`; not kept in git |
| Speed numbers | `pwsh scripts/measure-speed.ps1 -Mode <mode>` | See "Speed numbers" below |
| The live preview | `npm run studio` | Opens Remotion Studio in the browser |

`MEDIA_CONCURRENCY` sets how many frames render at once (default 4, so other work keeps its cores). The draft video
takes about a minute on the test laptop.

## Checks

| Check | Command |
|---|---|
| The art port draws exactly what `art.rs` draws (tray glyphs at 16 to 48 px, light and dark, and every app icon frame) | `node scripts/check-art.mjs` |
| No em dash, en dash or emoji in the text files; every asset credited | `npm run check` |
| Video: H.264 and AAC, 1920 x 1080, 30 fps, 30 s plus or minus 2, under 10 MB; loops: GIF, 4 to 6 s, forever, under 1 MB | `npm run check-video` (add `-- --draft` for the draft) |
| A clean checkout rebuilds the same bytes | `npm run check-rebuild` (add `-- --final` once the final video exists) |
| Types | `npm run typecheck` |

## When something changes

- **The icon** (`crates/wakeshot-windows/src/tray/art.rs`): run its `write_art_files` test to refresh the sheets and
  `assets/wakeshot.ico`, then `node scripts/check-art.mjs`. It fails until `src/art/sunrise.ts` draws the same; then
  `npm run loop` and `npm run video`.
- **The chimes** (`crates/wakeshot-windows/src/sound/chime.rs`): nothing to port; `make-sounds.mjs` reads the numbers
  from that file. If the file changes shape, the script stops with the name of what it could not find.
- **The menu or tooltip words**: new captures into `public/shots/`, then update `data/shots.json` (file, size, and
  `standIn: false`).
- **The words on screen**: `src/copy.ts`. **The timing**: `src/timeline.ts`; the sounds follow it.
- **The speed numbers**: `data/speed.json`, from the measure script's output.
- **The install line**: `COPY.install` in `src/copy.ts`; the winget command once the package is live.

## Where each input comes from

| Input | From |
|---|---|
| Icon art | `src/art/sunrise.ts`, a port of the Sunrise in `art.rs`, checked pixel for pixel by `check-art.mjs` (exact at every size, 2026-11-04) |
| Screen pictures | `public/shots/`, listed in `data/shots.json` with their source. Today all stand-ins (see the card) |
| Settings values in the video | `data/settings.json` (stand-in: the test laptop's own values) |
| Speed numbers | `data/speed.json` (empty until measured) |
| Sound | Made in code, `scripts/make-sounds.mjs` |
| Fonts | npm packages, `src/fonts.ts` |

Licences: `CREDITS.md`.

## Speed numbers

`scripts/measure-speed.ps1` follows the method in `docs/research/launch.md` part 1, on the installed release build:

1. `-Mode snapshot -Out before.json` (power settings, read only).
2. `-Mode size -Setup <Setup.exe>`: exe sizes and the install folder.
3. `-Mode idle`: 10 minutes, nothing holding; tray working set and private bytes every 10 s, CPU time.
4. `-Mode hold`: 10 minutes of `wakeshot run -- ping -n 600 127.0.0.1`; tray and guard.
5. `-Mode delays -Minutes 30`: start and finish 5 short Claude Code turns; pairs `statusUpdatedAt` with Wakeshot's
   `Holding starts` and `Holding ends` log lines.
6. `-Mode compare -Before before.json`: the settings equal the snapshot.

Idle and hold refuse to start while `cargo`, `rustc`, `claude`, `node` or another build runs. That includes the
Claude Code session of an agent: its own busy session makes Wakeshot hold, so these two modes run from a plain
PowerShell window with no Claude Code session working.

## Decisions (principal designer, for Haik's review)

1. **Sound made in code, not Kenney and Freesound files.** The bed (C major, so Wakeshot's C6 and G6 chimes sit in
   tune) and every effect are sine waves and seeded noise; the chimes come from `chime.rs`. Why: no third-party file,
   no licence question, the same bytes on every build. Change it when Haik finds the bed or the effects thin: then a
   CC0 loop from Freesound, committed with its link.
2. **Inter and JetBrains Mono from npm, not Segoe UI.** Microsoft allows Windows fonts in pictures and video ("We
   view this as the same as graphic files, providing the captions or text is rendered as a bitmap image", Font
   redistribution FAQ, learn.microsoft.com, read 2026-11-04), but a system font makes the build depend on the
   machine. Inter is the brand's named fallback. Change it when the video must match Windows' own text exactly and
   every build runs on Windows 11.
3. **The tray glyph in the loop keeps its taskbar colours** (ink on light, white on dark), with a soft gold light
   behind the risen sun: the brand's holding highlight, not a recoloured icon. Change it when Haik wants the gold sun
   of the app icon.
4. **Screen pictures zoom by whole numbers with hard pixels**, other zooms smooth, so a capture is never blurred into
   something it was not, and uneven pixels never garble text.
5. **The final render refuses stand-ins and missing numbers** (`build.mjs`), and every draft frame says DRAFT, so a
   draft cannot be mistaken for the real video.
6. **After the lid beat, the video says the closed laptop sleeps** once nothing has happened for 5 minutes (the
   2026-11-04 behaviour in `docs/product.md`), with Wakeshot's own going-to-sleep chime.

## Where the video lives

GitHub plays a video inline only as an uploaded attachment: `gh issue comment <n> --attach out/wakeshot-promo.mp4` on
a pinned "Media" issue; the README holds the bare `github.com/user-attachments` URL. Not uploaded yet; the URL goes
here once it is.
