# Screen session: the new brand on real Windows screens

Phase: 6 (brand apply). Status: in progress. Needs Haik: no for this run: Haik (2026-11-05 15:40, away at least an
hour): "Do the screen session as well". The worker does every step itself; a step that truly needs his hands is
marked "not shown" with the reason and listed for him. Agent: `wakeshot-worker`. Size: Part A about 50K tokens; Part
B about 80K tokens. Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-brand-apply-art` (done). The page and README block runs last, after `p6-brand-apply-web` is done.
Parallel with: `p6-brand-apply-web` Part B and `p6-brand-apply-media` Part B (offscreen only, no screen use).
Real laptop: the screen, a real install and uninstall of a local build, theme and high contrast switches, display
scale changes (Haik's own settings: each one read before, put back and read again after).
Display: the orchestrator holds the display on with a display-only power request (a hidden `pwsh` running
`<scratchpad>\display-hold.ps1`, PID in `<scratchpad>\display-hold.pid`), so the session does not lock while Haik is
away. Do not stop it; the orchestrator ends it after this card. It shows in `powercfg /requests` under DISPLAY.
Owns: this card; its pictures `docs/research/img/brand/screen-*.png`; its scratch folder
`<scratchpad>\p6-brand-screen-session\` (build folder `target` inside it, deleted when done).

## Goal

The new brand has been seen only in offscreen renders. Before `p6-media` Part B, the README and Pages finish, Haik
and a worker look at every place Windows shows it, on real screens, and capture each one. Part A writes the exact
checklist; Part B runs it with Haik.

## Read first

- `docs/research/brand-review.md`: "Next screen session: what to capture" (8 items) and the findings it names.
- The Results of the three apply cards (what changed, the splash decision, the notice icon choice).
- `CLAUDE.md` "Test laptop" (quit an installed Wakeshot first; no other keep-awake app), `docs/tasks/p6-screen-reruns.md`
  and `docs/tasks/p6-real-install-guard.md` (how earlier screen and install runs were done safely), `packaging/README.md`
  and `packaging/pack.ps1` (a local pack for the install).

## Deliverables

### Part A (now, no screen)

1. A checklist in this card, in run order, each step with: what Haik does (exact clicks), what the worker captures
   (tool, region, file name), what to look for (the finding ID or rule it checks), and how to put the setting back.
   It covers at least:
   - the tray glyph in its three states (holding in Moss, idle, off) beside Windows' own glyphs, on the dark and the
     light taskbar and in high contrast (Night sky and Desert), at 100% and 150% scale (T1 to T8);
   - the menu with the clock, light and dark, main and submenu, a long status line (M2, M4);
   - a notice from the installed build: header icon and name, light and dark (N1, N2);
   - Start: pinned grid, All list, a search for "Wakeshot", light and dark Start (A1 to A5);
   - Settings: Apps > Installed apps and Apps > Startup;
   - Explorer: the install folder in Large and Extra large icons and Details, light and dark (B1);
   - Task Manager: Processes and Startup apps;
   - the installer from a double click (not `--silent`): SmartScreen, the splash or whatever Velopack shows, then the
     first-run notice (I1, I2);
   - the README header and the page in a browser, light and dark (the lockup, the colours), and the social card as a
     link preview if one can be made without posting anywhere public.
2. Before and after checks for Haik's own settings each step touches (theme, high contrast, scale, startup entries),
   with the exact command or Settings path to read each value.
3. The order that keeps the session short (group by theme and scale), an estimate per block, and where Haik can stop
   and resume.
4. Which build to install: a local x64 release pack of the current commit into a scratch folder, installed as the
   real-install guard allows; after the session, uninstall it (default) or keep it for the Phase 7 field test (a
   question for Haik with that default).

### Part B (with Haik)

5. Run the checklist with Haik. Captures go to `docs/research/img/brand/screen-*.png`. Each finding: what was seen,
   its severity, and the card that fixes it. Put every setting back and check it.

## Acceptance checks

- Part A: every item of brand-review's 8-item list and of deliverable 1 has a step; every touched setting has a
  before and after read.
- Part B: every step captured or marked "not shown" with the reason; Haik's settings read equal before and after.

## Out of scope

Fixing anything found (new cards); `p6-media` Part B captures (they follow this session); any code change. Do not run
git.

## Questions for Haik

1. After the session, uninstall the local build or keep it for the field test? Default (taken, orchestrator decision
   for Haik's review): uninstall; the field test installs the first real release.

## Found outside scope

## Log

## Result
