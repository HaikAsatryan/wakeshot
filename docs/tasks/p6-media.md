# Speed numbers, pictures, the icon loop and the 30 second video

Phase: 6. Status: ready. Needs Haik: yes: he watches the video and the loop and says yes or what to change; he stays
off the PC for about 10 minutes while the captures run. Agent: `wakeshot-worker`. Size: about 120K tokens.
Real laptop: yes: screen captures of the real tray during one real Claude Code test session, light and dark; the real
app holds and restores the power settings during it. One card at a time on the laptop.
Runs after: `p6-launch-research` (video tool, storyboard, shot list, specs, sound sources); the tray track's
`p5-haik-check` (final icon and menu words); integration's `p6-int-real-e2e` (the real app works end to end).
Parallel with: any card that does not use the screen, the sound, a Claude Code test session or the power settings.
Build folder: `<scratchpad>\p6-media\` (exes, raw captures, renders).

## Goal

Make what the README, the Pages page and the Store listing show (Haik, 2026-11-04): measured speed numbers that prove
Wakeshot is very fast, pictures of the menu and tooltip, a short silent loop of the icon, and a 30 second promo video
with sound and effects, animated in code. Everything shows only what Wakeshot really does.

## Read first

- `docs/tasks/p6-launch-research.md`, Result; `docs/research/launch.md` parts 1 and 4.
- `docs/product.md`: "The job", "Distribution" (the README and video lines).
- The tray track's icon art and menu (`p5-icon-art`, `p5-haik-check` Results).

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

## Log

- 2026-11-04: card written from Haik's launch request (speed shown, pictures, a 30 s animated video with sound and
  effects).

## Result

What was done, files changed, checks and outcomes, open items.
