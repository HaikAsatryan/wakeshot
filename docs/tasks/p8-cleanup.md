# Phase 8 cleanup: remove or rewrite the old brand's history

Phase: 8. Status: done (planned 2026-11-07 by `p8-plan`; done 2026-11-07 04:27). Needs Haik: no (his rule of 2026-11-05, "no legacy
anywhere", and his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p6-logs-diagnostics` (the orchestrator's start for Phase 8; this card touches no code). Parallel with:
every card that does not own these files (group 1; no build).
Build folder: none.

## Goal

Leave nothing of the old brand in the repo: remove or rewrite every file and line listed under "Brand history to
remove" (Haik, 2026-11-05: no legacy anywhere; the research history stays only until this cleanup).

## Brand history to remove

Listed by `p6-brand-docs-sweep` (2026-11-05), the files and lines its whole-repo search still finds. The search terms
and the script: that card, deliverable 3 and its Log.

Research reports, `docs/research/` (old-brand words throughout; what still matters is in `docs/brand.md`):

- `brand-directions.md` (round 1), `brand-directions-2.md` (round 2, the pick's reasons), `brand-identity.md`,
  `brand-macos.md`, `brand-review.md`, `brand-windows.md`.

Pictures:

- `docs/research/img/brand/`: `dir-*.png` (round 1, 23 files), `dir2-*.png` (round 2, 8), `review-*.png` (the brand
  review, 12), `win-*.png` (the Windows report, 3). Keep `apply-*.png` (today's brand) and the screen session's
  `screen-*.png`, except `screen-taskmgr-processes.png` and `screen-taskmgr-startup.png`: both show the old violet
  icon from Task Manager's own cache of the install path (`p6-brand-screen-session.md`, finding 1), not today's icon.
  Retake them after the cache clears (sign out or reboot; not tested which is enough) or remove them.
- `docs/research/img/ux/`, screen pictures of the old art: `int-exe-icon.png`, `int-first-run-balloon.png`,
  `int-icon-holding.png`, `int-icon-idle.png`, `int-smoke-holding.png`; `menu-screen-dark-main.png`,
  `menu-screen-dark-sub.png`, `menu-screen-light-main.png`, `menu-screen-light-sub.png`; every `notice-*.png` except
  `notice-dnd-centre.png` (18 files); `taskbar-dark-holding.png`, `taskbar-dark-idle.png`, `taskbar-dark-off.png`,
  `taskbar-light-holding.png`, `taskbar-light-idle.png`, `taskbar-light-off.png`, `taskbar-keyboard-focus.png`,
  `taskbar-tooltip-dark.png`. Keep: `taskbar-pattern-150.png`, `notice-dnd-centre.png` (no art), and the files today's
  code writes (`icon-sheet-*.png`, `icon-app.png`, `menu-render-*.png`).

Done task cards that name the old brand (`docs/tasks/`):

- `p2-6-docs-pass.md`, `p5-haik-check.md`, `p5-icon-art.md`, `p5-menu-paint.md`, `p5-menu-view.md`,
  `p5-notices.md`, `p6-brand-apply-art.md`, `p6-brand-apply-media.md`, `p6-brand-apply-web.md`,
  `p6-brand-cleanup.md`, `p6-brand-directions.md`, `p6-brand-directions-2.md`, `p6-brand-docs-sweep.md`,
  `p6-brand-research-identity.md`, `p6-brand-research-macos.md`, `p6-brand-research-windows.md`,
  `p6-brand-screen-session.md` (running when this list was made), `p6-int-dry-run-e2e.md`,
  `p6-int-haik-checks.md`, `p6-int-tray-mode.md`, `p6-menu-text-look.md`, `p6-notice-app-icon.md`,
  `p6-screen-reruns.md`, `p6-velopack-dependency.md`.

Dated lines inside files that stay:

- `docs/tasks/p6-media.md`: the Log lines of 2026-11-04 (the old port and font).
- `docs/tasks/p6-readme.md`: "Round 2" (its item 3, Log and Result name the old art).
- `docs/phases/05-tray-ux.md`: the `p5-icon-art` row of the card table; the work log lines of 2026-11-04 on the
  icon pick, the menu glyphs and `p5-haik-check`, and the 2026-11-04 12:07 line (the screen slot, "built from an
  export of f47391d (the Sunrise)").
- `docs/phases/06-integration.md`: the `p6-int-tray-mode` row; the work log line on its pictures.
- `docs/phases/06-ship.md`: the work log lines 2026-11-04 14:08 and 2026-11-05 14:20.
- `docs/plan.md`: the status lines 2026-11-04 14:09 and 2026-11-05 14:20.

Before removing (or links break):

- Live pointers into the reports: `docs/brand.md` (`brand-directions-2.md`, `brand-identity.md`, `brand-macos.md`);
  `docs/research.md` ("Brand on macOS", "Brand apply: the held light in art.rs", "Brand colours under colour-blind
  simulation"); `media/README.md`, `media/src/fonts.ts`, `media/src/theme.ts`, `site/style.css`; comments in
  `crates/wakeshot-windows/src/tray/art.rs` (`brand-review.md` T2, T4, T7; `brand-directions-2.md`) and
  `tray/menu_paint.rs` (`brand-windows.md`). Point them at `docs/brand.md` or `docs/research.md`, or keep the
  reports rewritten without the old brand.
- `docs/research.md` cites the old-art pictures as evidence ("Tray UX: tray icon", "Tray UX: menu on screen", "Tray
  UX: notices", "Integration dry-run e2e"). `notice-dark-first-run.png` and `notice-light-first-run.png` are also the
  source of the notice header colours in "Brand apply: the held light in art.rs": measure them again on new captures
  first.

Not old brand, keep (the search words match for another reason): the chimes' rising pitch
(`crates/wakeshot-windows/src/sound/chime.rs`, `spikes/lid-sleep/src/sound.rs`, `docs/research.md` "Tray UX: chimes",
`docs/tasks/p5-sounds.md`, the `p5-sounds` line in `05-tray-ux.md`); a caption that rises into place
(`media/src/promo/parts.tsx`); facts about other products (`docs/research.md` "Brand on macOS",
`docs/research/rivals-others.md`); "golden" tests and macOS "Golden Gate".

## How Phase 8 splits this list

Decided by `p8-plan` (2026-11-07, for Haik's review; phase file `docs/phases/08-launch-cleanup.md`, calls 4 and 12):

- This card: the six reports, the old-art pictures, `docs/brand.md`.
- `p8-docs-research-2`: the `research.md` lines above. The two Task Manager captures are removed, not retaken, and the
  notice header colours are not measured again: they stay in `research.md`, marked "measured on 2026-11-04 captures,
  not kept (they show the old icon)". No screen session tonight.
- `p8-polish-windows-tray`: the comments in `art.rs` and `menu_paint.rs`.
- `p8-repo-hygiene`: `site/style.css`.
- `p8-final-cleanup`: `media/README.md`, `media/src/fonts.ts`, `media/src/theme.ts` (pointers only), and a last
  repo-wide search for the report and picture names.
- The done task cards and the dated lines inside `docs/tasks/`, `docs/phases/` and `docs/plan.md`: nothing to do;
  `p8-final-cleanup` removes those files (the history bundle keeps them).

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership", call 4.
- This card's list above (the source of truth) and `docs/tasks/p6-brand-docs-sweep.md`, deliverable 3 and its Log
  (the search terms).
- `docs/brand.md`, whole. The six reports: headings, and a section only where you need to check that a fact still
  used is in `brand.md`.
- What points at the reports and the pictures: grep the repo outside `docs/tasks/`, `docs/reviews/`, `docs/phases/`
  and `docs/plan.md` for each report name and each removed picture name.

## Deliverables

1. Before deleting a report: every fact from it that a kept file still uses (`brand.md`, a code comment, `media/`,
   `site/`, `research.md`) is in `docs/brand.md`, short, with its source. Add what is missing.
2. Delete the six reports and the old-art pictures listed above, the two Task Manager captures included. Keep every
   picture the list says to keep.
3. `docs/brand.md`: no pointer to a removed report or picture, no history ("round 1", "the pick", "Haik chose"):
   today's colours, rules, assets and where each is made. Keep its headings (others may cite them) or record renames.
4. The pointers in files you do not own: list each (file, line, what it points at, a suggested new target) in the
   Result, grouped by the card that fixes it (section "How Phase 8 splits this list").
5. Result: removed files (count, bytes); facts added to `brand.md`; the pointer list; "Heading renames" for
   `brand.md` (even if empty).

## Acceptance checks

1. None of the removed files exists; every picture the list keeps still exists (list both counts).
2. The `p6-brand-docs-sweep` search terms find nothing in `docs/brand.md`.
3. The internal-name sweep (phase file, "Every card") on `docs/brand.md`: zero hits. No em dash.
4. Every relative link and picture path in `docs/brand.md` resolves.
5. A repo-wide grep for the six report names and the removed picture names: every remaining hit is in the Result's
   pointer list with the card that fixes it, or is inside `docs/tasks/`, `docs/reviews/`, `docs/phases/` or
   `docs/plan.md`.

## Out of scope

`docs/research.md`, code, `media/`, `site/`, `README.md`, the kept pictures, the "Not old brand, keep" items. Git.

## Questions for Haik

## Found outside scope

- `media/README.md` decision 10 (about line 176) still says `STACKED` is in `src/promo/scenes.tsx` and that the title
  and the outro use the stacked lockup; line 159 names "the title and outro icons". Today `STACKED` is in
  `media/src/promo/Promo.tsx` and only the end card uses it. For tonight's media cards, else `p8-final-cleanup`.
- The six reports held facts nothing kept uses, now only in the history bundle: the Store asset sizes
  (`brand-windows.md` section 1, "Store assets", written for the blocked `p8-store`), the Mac keep-awake survey table,
  the 2026 icon trends and Microsoft's 2025 icon refresh. Read them from the bundle when the Store card or a Mac
  version starts.

## Log

- 2026-11-05: card created by `p6-brand-docs-sweep` with the brand history list.
- 2026-11-05: `p6-brand-apply-art` review fix round (`docs/reviews/p6-brand-apply.md` findings 4 and 5) added the
  2026-11-04 12:07 work log line of `05-tray-ux.md` and the two Task Manager captures.
- 2026-11-07: Read first, Deliverables, checks and scope written by `p8-plan`; the split across the Phase 8 cards in
  "How Phase 8 splits this list". Status ready.
- 2026-11-07 04:17: worker started (session wakeshot-4e). Status in progress. Read `CLAUDE.md`, the phase file, this
  card, `docs/brand.md`.
- 2026-11-07 04:24: lists made by script (scratch `p8-cleanup\lists.py`): 89 files to remove (6 reports, 83
  pictures, 7,565,453 bytes), 75 pictures to keep. Repo grep for each removed name: 28 hits outside the planning
  folders and the reports themselves. `brand.md` rewritten: history and report pointers out, facts the kept files use
  added with their sources (ring anti-aliasing, taskbar level as measured, menu glyphs and E917, colour rules and
  method, font FAQ, Mac sources); the stacked lockup and video lines fixed to today's `media/src/promo/Promo.tsx`.
- 2026-11-07 04:25: 89 files deleted (script `delete.py`, only paths under `docs/research/`); `check` mode: 89 gone,
  75 kept pictures present. Acceptance checks 1 to 5 run (Result). Status done.

## Result

Status: done, 2026-11-07 04:27.

### What was done

- **Removed 89 files, 7,565,453 bytes**: the six reports `docs/research/brand-directions.md`, `brand-directions-2.md`,
  `brand-identity.md`, `brand-macos.md`, `brand-review.md`, `brand-windows.md` (198,637 bytes); 48 pictures in
  `docs/research/img/brand/` (`dir-*.png` 23, `dir2-*.png` 8, `review-*.png` 12, `win-*.png` 3,
  `screen-taskmgr-processes.png`, `screen-taskmgr-startup.png`); 35 in `docs/research/img/ux/` (`int-*.png` 5,
  `menu-screen-*.png` 4, `notice-*.png` except `notice-dnd-centre.png` 18, the eight `taskbar-*.png` named in the list).
- **Kept 75 pictures**: `img/brand/` 57 (`apply-*.png` 13, `screen-*.png` 44), `img/ux/` 18 (`icon-sheet-*.png` 3,
  `icon-app.png`, `menu-render-*.png` 12, `taskbar-pattern-150.png`, `notice-dnd-centre.png`). The two folders now hold
  exactly these.
- **`docs/brand.md` rewritten** (17,645 bytes): no pointer to a removed report or picture; no history (the decision
  quote and its work log pointer, "round 2", "Haik, date", "as Haik approved it" are out). Media lines brought to
  today's code: the stacked lockup is in `media/src/promo/Promo.tsx` (the old `scenes.tsx` path was broken) and the
  end card uses it; the video's light comes on at 12 s; the npm scripts are `loop`, `video`, `poster`, `draft`.

### Facts added to `brand.md` (each with its source; facts a kept file used from a removed report)

| Fact | Used by | Where in `brand.md` |
|---|---|---|
| A 1 px ring drawn as a plain curve looks fainter than the line and nearly vanishes on a light taskbar (16 px renders, 2026-11-05) | `art.rs` (T2) | Tray icon, "Drawn per pixel" |
| The holding glyph sat within 0.3 px of OneDrive's icon at 100%, 125%, 150% on the test laptop (2026-11-05) | `art.rs`, `research.md` (T4) | Tray icon, "Level with its neighbours" |
| Menu glyphs E890, E823, E777, E8A5, E711; E917 "Clock" is missing from Segoe MDL2 Assets (cmap of both fonts and Microsoft's lists, 2026-11-05; test `item_glyphs_are_in_both_icon_fonts`) | `menu_paint.rs` | Tray icon, new bullet "Menu glyphs are Windows' own" |
| The contrast rules: WCAG 2.2 1.4.3 and 1.4.11, Microsoft's half rule (quoted), and the colour-blind rule with its method (Machado, Oliveira and Fernandes 2009, full strength, linear sRGB) | `research.md` (section 8, C4) | Colours, "The rules" |
| Moss's hue band (OKLCH 100 to 150) held none of 29 coloured brands sampled from Simple Icons (2026-11-05); the 20 dE00 rule beside the measured 34.8 | `brand.md` itself | Colours, "Why Moss" |
| Segoe UI Variable's licence quote and the no-web-font rule, Microsoft's "Font redistribution FAQ" (updated 2024-11-19, read 2026-11-05) | `media/src/fonts.ts`, `media/README.md`, `brand.md` | Wordmark |
| The Mac rules' sources: Apple's HIG pages, the Icon Composer article, WWDC25 220 and 361 (read 2026-11-05, no Mac) | `research.md` "Brand on macOS", `brand.md` | Mac (later work) |
| Why the held light: not a weather sign (Shut the Lid uses one), not coffee, the picture most of about 65 Mac keep-awake apps use (2026-11-05) | `brand.md` itself | The idea |

The Moss colours (`art.rs`, `media/src/theme.ts`, `media/README.md`, `site/style.css`), the 0.455 target and the slash
inside the box (`art.rs` T4, T7) were already in `brand.md`.

### Heading renames in `brand.md`

None. All eight `##` headings kept as they were.

### Pointers in files I do not own (26, plus 5 history notes)

Line numbers as of 04:27; other cards edit these files, so the section name is the anchor.

`p8-polish-windows-tray` (6):

| File, line | Points at | Suggested fix |
|---|---|---|
| `crates/wakeshot-windows/src/tray/art.rs:100` | `docs/research/brand-directions-2.md`, the Moss column; also "Haik's pick 2026-11-05" | `docs/brand.md`, "Colours"; drop the pick and date |
| `art.rs:155` | `brand-review.md` T4 | drop (the comment states the fact) or `docs/brand.md`, "Tray icon" |
| `art.rs:205` | `brand-review.md` T2 | drop (the comment states the fact) |
| `art.rs:263` | `brand-review.md` T7 | "as Windows' own struck glyphs do", or `docs/brand.md`, "Tray icon" |
| `art.rs:2042` | `brand-review.md` T4 | drop or `docs/brand.md`, "Tray icon" |
| `crates/wakeshot-windows/src/tray/menu_paint.rs:1584` | `docs/research/brand-windows.md` (E917 cmap) | `docs/brand.md`, "Tray icon" (the menu glyphs bullet), or just "its cmap" |

`p8-docs-research-2` (15, all in its half of `docs/research.md`):

| Line, section | Points at | Suggested fix |
|---|---|---|
| 3037, "Tray UX: tray icon" | `docs/research/img/ux/taskbar-*.png` (the eight old-art files are gone; `taskbar-pattern-150.png` stays); the line also names the old art's export | "pictures not kept (they show the old icon)" |
| 3052, same | `taskbar-light-*.png` | same |
| 3056, same | `taskbar-keyboard-focus.png` | same |
| 3058, same | `taskbar-tooltip-dark.png` | same |
| 3065, "Tray UX: menu on screen" | `menu-screen-{dark,light}-{main,sub}.png` | not kept; today's menu on screen is in `docs/research/img/brand/screen-menu-*.png` |
| 3256, "Tray UX: notices" | `docs/research/img/ux/notice-*.png` (only `notice-dnd-centre.png` stays) | not kept; today's notice: `img/brand/screen-notice-dark.png`, `-light.png` |
| 3274 to 3275, same | `notice-dark-closed-lid-report*.png`, `notice-light-first-run*.png` | not kept |
| 3754, "Integration dry-run e2e" | `int-first-run-balloon.png` | not kept |
| 3761, same | `int-icon-idle.png`, `int-icon-holding.png` | not kept |
| 3765, same | `int-exe-icon.png` | not kept |
| 4760, "Brand on macOS" | `docs/research/brand-macos.md` | drop; the section keeps its own sources, the rules are in `docs/brand.md`, "Mac (later work)" |
| 4871, "Brand apply: the held light in art.rs" | `brand-review.md` T4 | `docs/brand.md`, "Tray icon", or drop |
| 4914, same | `img/ux/notice-dark-first-run.png`, `notice-light-first-run.png` | "measured on 2026-11-04 captures, not kept (they show the old icon)" (phase call 4) |
| 5008, "Brand colours under colour-blind simulation" | `brand-identity.md` section 8, rule C4 | `docs/brand.md`, "Colours" (the rules paragraph) |
| 5033, "Brand screen session: what Windows shows" | `docs/research/img/brand/screen-*.png`, which no longer holds the two Task Manager captures | say those two are not kept (they show the old icon) |

History notes for `p8-docs-research-2` (old-brand words, not file pointers): 4872 "round 2's method"; 4882 "round
2's rule, approved"; 5025 "round 1's `de2000`"; 5055 "Task Manager shows the old violet Sunrise icon" (keep the fact,
its captures are gone); 5100 "round 2's approved frames".

`p8-repo-hygiene` (1):

| File, line | Points at | Suggested fix |
|---|---|---|
| `site/style.css:1` | `docs/research/brand-directions-2.md`, "Step 2: the colour" | `docs/brand.md`, "Colours" |

`p8-final-cleanup` (4, `media/` pointers only):

| File, line | Points at | Suggested fix |
|---|---|---|
| `media/README.md:82` | `docs/research/brand-directions-2.md` | `docs/brand.md`, "Colours" |
| `media/README.md:129` | `docs/research/brand-identity.md` (section 7) | `docs/brand.md`, "Wordmark, lockup and social card" |
| `media/src/fonts.ts:5` | `docs/research/brand-identity.md` section 7 | same |
| `media/src/theme.ts:1` | `docs/research/brand-directions-2.md`, "Step 2: the colour"; also "Haik's pick of 2026-11-05" | `docs/brand.md`, "Colours"; drop the pick and date |

### Acceptance checks

| # | Check | Outcome |
|---|---|---|
| 1 | Removed files gone, kept pictures present (`lists.py check`) | pass: 89 of 89 gone; 75 of 75 kept present; `img/brand/` 57 files, `img/ux/` 18, no `brand-*.md` left |
| 2 | Sweep terms in `brand.md` (`check_brand.py`: the hex codes and phrases anywhere, words as whole words, plus risen, sun, horizon) | pass: 0 hits. Substring-only matches read and left: "golden tests" (plain English, on the card's keep list), "fainter", "interactive", "Interface" |
| 3 | Internal-name sweep on `brand.md`, plus "round 1/2", "the pick", "approved"; em and en dash | pass: 0 hits, no dash |
| 4 | Every path in `brand.md` resolves | pass: 0 broken (backticked repo paths by script; the folder-less names `-light.gif`, `-dark.svg`, `OFL.txt`, `icon-app.png` checked by hand) |
| 5 | Repo-wide grep for the report and removed picture names, outside the planning folders | pass: 22 named hits plus 8 wildcard mentions in `research.md`; each is in the pointer list above |

### Calls for Haik's review

1. **Menu glyph fact moved into `brand.md`** (new bullet "Menu glyphs are Windows' own"), since `menu_paint.rs` cited
   the removed Windows report for E917. Change it when the menu glyphs move to a doc of their own.
2. **`brand.md`'s media lines follow today's media code** (stacked lockup in `Promo.tsx` used by the end card, the
   light at 12 s, the npm scripts); the old "drawn frames match within a pixel" was dropped, since nobody checked it
   on the new video. Change it when the media cards change the video again.
3. **"Why the held light" is told without the old picture**: not a weather sign, not coffee, with the survey's date.
4. **Only facts a kept file uses were carried over**; the rest of the reports (Store asset sizes, the Mac survey,
   trends) lives in the history bundle (see "Found outside scope").

### Files

- Changed: `docs/brand.md`, `docs/tasks/p8-cleanup.md`.
- Deleted: the six `docs/research/brand-*.md`; `docs/research/img/brand/` `dir-*.png`, `dir2-*.png`, `review-*.png`,
  `win-*.png`, `screen-taskmgr-*.png`; `docs/research/img/ux/` `int-*.png`, `menu-screen-*.png`, `notice-*.png` (not
  `notice-dnd-centre.png`), `taskbar-*.png` (not `taskbar-pattern-150.png`).
- Created: none in the repo. Scratch scripts in `<scratchpad>\p8-cleanup\` (`lists.py`, `heads.py`, `check_brand.py`,
  `delete.py`, the lists).
- Open items: the 26 pointers and 5 history notes above, for the cards named.
