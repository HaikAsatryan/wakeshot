# Haik's look, listen and click check of the tray UX

Phase: 5. Status: ready. Needs Haik: **yes**, about 25 minutes at the laptop (eyes, ears, hands), plus his answers to
the icon sheet and the text list if he has not given them at the sign-off. Agent: `wakeshot-worker`. Size: about 80K
tokens.
Real laptop: **screen** and **sound** (one screen or sound card at a time, across all tracks), and **settings**: Haik
himself switches the Windows mode, and optionally high contrast and the display scale. Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-haik-check\target`.
Runs after: `p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds`, `p5-text`, `p5-icon-art` (all done), and
`p4-cli-commands` (`shell.rs`, for Open log and the first-run click). Parallel with: only cards that use no screen,
sound or power settings.

## Goal

Architecture checklist item 13 for this track, before integration: Haik sees the icon, menu and notices on his own
taskbar in both modes, hears the three chimes, clicks through the menu and the first-run notice, and tells us what is
off. Also the one check an agent cannot do alone: a notice shown while the PC is locked.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 3 to 6).
- The Results of `p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds`, `p5-icon-art` (the concepts paragraph),
  `p5-text` (the text list).
- `docs/architecture.md` section 9, checklist 13.
- `docs/tasks/p1-lid-sound.md` "Steps for Haik" (the form of steps that worked).

## Owns

`crates/wakeshot-windows/examples/ux_all.rs`; new files `docs/research/img/ux/check-*.png`. After Haik answers, and
only for his answers: `crates/wakeshot-windows/src/tray/art.rs` and `assets/wakeshot.ico` (the concept he picks
becomes the default concept and the `.ico` is written from it; keep the `art` API and the other concepts, so
`ux_icon`, `ux_menu` and `ux_notices` of done cards keep building; review m16) and `crates/wakeshot-core/src/text.rs`
(his wording changes). Those cards are done by then, so no other card owns these files. The owner doc lines in
`art.rs` and `text.rs` name only `p5-icon-art` and `p5-text`; this hand-over is the planned exception (listed for the
orchestrator in `p1-track-plan-fixes`).

## Deliverables

1. Example `ux_all` (Part A, no laptop): the demo icon with the real `OwnerDrawnMenu` and a sample product menu from
   `text.rs`; picks act like the product would look (Automatic and Keep awake change the icon state and the status
   line; Restore shows its notice; Open log opens a sample log file through `shell::open_with_default_app`; Quit
   exits). The console window takes keys: `n` next notice (every variant in turn), `1` `2` `3` the three chimes, `f`
   the first-run notice, `l` lock the PC and show a notice 10 s later, `q` quit. Prints each event with the wall time.
2. Agent steps before Haik (screen slot): checks below; start `ux_all` at Medium (track note 4) from a launch file
   `ux-all.cmd` in your scratchpad; pin the demo icon; read and record the Windows mode, Do not disturb, the default
   audio device and volume, the sound scheme, the display scale. Set Status `blocked` with the steps below for Haik.
3. **Steps for Haik** (write them into this card with exact clicks before blocking; the outline):
   1. Icon: look at the demo icon on the taskbar in each state (the menu's Automatic and Keep awake change it).
      Crisp? Clear which state is which?
   2. Menu: open it with a left click, a right click, and the keyboard (Win+B, arrows, Enter). Compare with Explorer's
      right-click menu. Anything off?
   3. Theme: Settings > Personalization > Colors > Choose your mode > Light. Look at the icon and open the menu. Then
      back to Dark (or to what it was).
   4. Notices: press `n` until every notice has shown. Each one readable, short, clear? Press `f`, click the banner:
      the taskbar settings page opens; find Wakeshot under "Other system tray icons".
   5. Locked: press `l`, wait until the screen locks, wait 30 s, sign in. Did a banner show on the lock screen, right
      after sign-in, only in the notification centre (Win+N), or nowhere?
   6. Chimes: press `1`, `2`, `3` with the laptop alone. Clear? Told apart? Too loud or soft? Then plug in the DELL,
      wait 30 s, press `1` again: the chime should come from the laptop speakers, not the screen.
   7. Open log: pick Open log in the menu: the sample log opens in your default app.
   8. Optional, high contrast: first Settings > Personalization > Themes > Save (name it "Haik before contrast"); then
      Settings > Accessibility > Contrast themes > a theme > Apply; look at the icon and the menu; then back through
      Personalization > Themes > "Haik before contrast". Skip if in doubt: Phase 0 never checked that turning contrast
      off restores your theme, so the save comes first.
   9. Optional, scale: Settings > System > Display > Scale 100%, look at the icon and menu, then back to 125%.
   10. Optional, Explorer restart: Task Manager > Windows Explorer > Restart: the demo icon comes back.
   11. Your answers: the icon concept (from the sheet) and the text list, if not given yet. Quit the demo with `q`.
4. Agent steps after Haik: reads again (equal to before, or what Haik changed on purpose), leftovers removed and
   checked (track note 4), no demo process left; apply his icon pick and wording changes (rewrite the art files with
   the ignored writer; rerun the art and text tests); record everything.
5. `docs/research.md`: a new section "Tray UX: Haik's check" (his words, the locked-notice result, what changed).

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows
  --all-targets` with `-D warnings` for both targets; `cargo build --release --locked -p wakeshot-tray -p wakeshot-cli`
  for both targets (the tray's ARM64 build is CI only once `velopack` is in and this laptop has no clang): each clean in
  the owned files, before Haik's steps and again after his changes.
- Each of Haik's steps has his answer recorded, or "skipped" for an optional one.
- The settings reads after equal the reads before, except what Haik changed himself on purpose.
- Leftovers gone at two reads a minute apart; no demo process left.
- If the locked notice was lost: the Result says so and points at the core proposal in `p5-plan-tray-ux` (notices
  wait while the session is locked).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Fixing what Haik finds beyond the icon pick and wording: each finding goes to the Result for a fix card. The real
  tray exe (integration). Any change to Haik's settings by the agent.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include Haik's answers per step.
