# Phase 8 cleanup: remove or rewrite the old brand's history

Phase: 8. Status: ready (planned 2026-11-07 by `p8-plan`). Needs Haik: no (his rule of 2026-11-05, "no legacy
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

## Log

- 2026-11-05: card created by `p6-brand-docs-sweep` with the brand history list.
- 2026-11-05: `p6-brand-apply-art` review fix round (`docs/reviews/p6-brand-apply.md` findings 4 and 5) added the
  2026-11-04 12:07 work log line of `05-tray-ux.md` and the two Task Manager captures.
- 2026-11-07: Read first, Deliverables, checks and scope written by `p8-plan`; the split across the Phase 8 cards in
  "How Phase 8 splits this list". Status ready.

## Result
