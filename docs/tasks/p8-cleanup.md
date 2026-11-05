# Phase 8 cleanup: remove or rewrite the old brand's history

Phase: 8. Status: later. Needs Haik: decided when Phase 8 is planned.
Runs after: the Phase 7 field test. Parallel with: decided when Phase 8 is planned.

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

## Read first

Written when Phase 8 is planned.

## Deliverables

Written when Phase 8 is planned.

## Acceptance checks

Written when Phase 8 is planned.

## Out of scope

Written when Phase 8 is planned.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-05: card created by `p6-brand-docs-sweep` with the brand history list.
- 2026-11-05: `p6-brand-apply-art` review fix round (`docs/reviews/p6-brand-apply.md` findings 4 and 5) added the
  2026-11-04 12:07 work log line of `05-tray-ux.md` and the two Task Manager captures.

## Result
