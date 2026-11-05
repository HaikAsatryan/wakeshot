# Real captures of the installed app for the video and the README (`p6-media` Part B, captures only)

Phase: 6. Status: in progress. Needs Haik: he stays off the PC while it runs (Haik 2026-11-05 ~21:50: "Dell is
connected go on"). Agent: `wakeshot-worker`. Size: about 100K tokens.
Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-media` Part C's first draft (done); `p6-menu-status-glyph` (in the installed build).
Parallel with: `p6-media` Part C round 2 (Remotion renders in `media/`, no screen). Nothing else that uses the screen,
the theme, the scale or the power settings.
Real laptop: yes. The screen, theme and scale switches, the installed Wakeshot's states, its real holds on a throwaway
plan copy. Haik's own settings read before and put back after.
Build folder: `<scratchpad>\p6-media-captures\` (all pictures and logs; nothing in `media/` this round).

## Goal

The video (`docs/research/promo-video.md`, "What Part B captures from the real app") and the README need real stills
of the installed app. The video's drawn parts move; the captures carry the proof. Haik's note on the first draft
(2026-11-05): "Show exactly that Claude Code starts to work and it becomes green, then it finishes it goes back."
So the captures must let the video show, in one fixed screen rectangle, the icon idle, then holding (green) when a
real Claude Code session works, with its tooltip, then idle again.

## Read first

- `docs/research/promo-video.md`: "Real captures and drawn parts" (the sets, ids S1 to S9, whole-number zoom, rows).
- `docs/tasks/p6-brand-screen-session.md`: how a screen session runs safely here (guard, the throwaway plan copy, the
  tray started at Medium through `explorer.exe`, quit by `WM_CLOSE` to its `Wakeshot.Loop` window, never by name, the
  idle state with `CLAUDE_CONFIG_DIR` pointing at an empty folder, states by the installed CLI, theme and scale
  switches read before and restored after, per-monitor DPI aware capture). Its scripts are in
  `<scratchpad>\p6-brand-screen-session\*.ps1`: reuse them, copied into your build folder.
- `CLAUDE.md` "Test laptop" and "Facts to keep in mind" (elevated agent shells: start the tray and the CLI at Medium).
- `media/data/shots.json` (the format Part C uses for captures and row boxes) and `media/README.md` decision 4.

## Deliverables

1. **State before and after**: theme, scale of each monitor, the active power plan and Balanced's lid action and sleep
   timers (AC and DC), the tray icon's `NotifyIconSettings` entry, the installed app's `config.json`. Equal after, or
   each difference explained.
2. **The video sets, dark, at 200% on the DELL** (Haik's answer; set by script and put back if it is not already;
   if the tray is not on the DELL's taskbar, use the screen that has it at its highest scale and say so). One fixed
   screen rectangle per set, cursor not in the picture, nothing private in view (no names, no message counts; crop or
   cover):
   - **Work set** (Haik's note): the tray icon idle (S1), holding while a real Claude Code session works (S2), the
     holding tooltip "Holding awake: Claude Code (1 session)" (S3), and the idle tooltip (S4); S1 also serves as
     "idle again". This session's own Claude Code process is the real session (it stays busy while any agent runs);
     the idle pictures come from a tray started with `CLAUDE_CONFIG_DIR` at an empty folder, as the screen session
     did. Same rectangle and scale for all, so the video swaps them in place.
   - **Menu set**: menu closed, icon idle (S1); menu open, no hover (S5a); Keep awake hovered with its submenu open
     (S5b); "1 hour" hovered (S5c); menu closed after the click, icon holding (S5d). Then Keep awake off again.
   - **S7**: the tooltip while `wakeshot run -- <a short command>` holds (the command name is what shows; pick a neutral
     one such as `npm run build` in a scratch folder with a stub script, nothing private).
   - **S9**: before and after values of the lid action and the sleep timers from the hold's log (on the plan copy).
   - For each menu capture, the rows' boxes in capture pixels (name, x, y, w, h), measured by a script.
3. **README stills**: the menu open, the holding tooltip, the idle tooltip, light and dark, at the same scale.
4. A manifest `<build folder>\captures.json`: each file, set, id, monitor, scale, rectangle, theme, time, and the rows.
   One contact sheet of all pictures for the orchestrator.
5. Log as you go; Result with the before and after table, the files, and anything that did not work.

## Rules

- Keep the screen awake for the session with a display-only request (`SetThreadExecutionState` with
  `ES_CONTINUOUS | ES_DISPLAY_REQUIRED` from a process you start; stop it by PID at the end).
- Kill only processes you started, by PID. Never quit the installed tray by name; never kill Claude Code.
- If Haik comes back to the PC mid-session (real input you did not make), pause and note it in the Log.
- Never run git. No change to `media/`, the app code or any doc but this card.

## Out of scope

The video build and its `media/` files (Part C), speed numbers (Haik starts them with no Claude Code session), the
morning notice (S6), the Task Manager retake.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-05 ~21:50: card written by the orchestrator.
- 2026-11-05 21:58: worker started; reading the screen session's card and scripts.

## Result
