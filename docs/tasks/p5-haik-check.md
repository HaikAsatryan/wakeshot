# Haik's look, listen and click check of the tray UX

Phase: 5. Status: blocked: Part A done (2026-11-04 12:25); Part B waits for the screen slot with Haik. Needs Haik: **yes**, about 25 minutes at the laptop (eyes, ears, hands), plus his answers to
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
`art.rs` and `text.rs` name this hand-over (`p1-close-contracts`).

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

## Steps for Haik (about 25 minutes; written in Part A, run in Part B)

Not run yet. The Part B worker checks the values in brackets against its own reads before it hands these over.

Before you start: an agent has started the demo `ux_all` at Medium from `ux-all.cmd`, pinned its icon next to the
clock, and read your Windows mode, Do not disturb, audio device and volume, sound scheme and display scale. The icon
is a sun: holding is a risen half sun with rays, idle the empty half sun, off the empty half sun struck through. A
console window named after `ux-all.cmd` prints one line per event; it takes the keys below. **Keys work only while
that window has focus**: click its title bar first, and again after each use of the menu or Settings. Pick menu
items with the mouse unless a step says the keyboard. Do not close Settings or Notepad windows for the agent; close
your own as you like.

Write each answer under its step, or tell the orchestrator.

1. **Icon states.** The demo starts holding for "Claude Code (2 sessions)".
   1. Look at the icon. Hover over it: the tooltip says "Wakeshot", then what holds.
   2. Click the console window, press `a` (the sample sessions stop): the icon turns idle.
   3. Left-click the icon, click **Automatic**: the icon turns off.
   4. Left-click the icon, **Keep awake** > **30 minutes**: the icon turns holding; the menu's top line shows Keep
      awake with the minutes left.
   5. Left-click the icon, **Keep awake** > **Off**: off again. Left-click, **Automatic**: idle. Click the console,
      press `a`: holding again.
   Answer: crisp at this size? Clear which state is which?
2. **Menu.** Open it three ways, then press Esc each time:
   1. Left-click the icon. 2. Right-click the icon. 3. Press Win+B, press the Right arrow until the sun icon has a
   focus box, press Enter; in the menu use the arrow keys (Right opens Keep awake), then Esc.
   Then right-click an empty spot of the desktop and compare that Windows menu with ours: corners, colours, spacing,
   font, icons, the check marks. Answer: anything off?
3. **Light mode.** Start > Settings (Win+I) > Personalization > Colors > "Choose your mode" > **Light**. Look at the
   icon and open the menu. Then set "Choose your mode" back to what it was (**Dark** at the 2026-11-04 02:14 read; the
   agent's read before this check rules). Answer: icon and menu right in light mode?
4. **Notices.** Click the console, press `n`, read the banner at the bottom right; press `n` again for the next one.
   There are 15; the console prints "1 of 15" and so on, with each one's name. Then press `f` and click its banner:
   Settings opens the Taskbar page. Open "Other system tray icons" and find the demo there (it may be named `ux_all`
   rather than Wakeshot: not verified). If no banner shows at all, tell the orchestrator (Do not disturb may be on).
   Answer: each one readable, short, clear? Which ones would you word otherwise?
5. **Locked.** Click the console, press `l`. The PC locks at once; the demo shows a notice 10 s later ("Wakeshot
   locked the PC"). Wait 30 s, then sign in. Answer: did the banner show on the lock screen, right after sign-in,
   only in the notification centre (Win+N), or nowhere?
6. **Chimes.**
   1. Unplug the DELL dock cable (the laptop goes on battery; the console window may move to the laptop screen).
   2. Click the console, press `1`, wait 3 s, press `2`, wait 3 s, press `3`. Repeat as you like. `1`: lid closed
      while holding (two notes rising), `2`: going to sleep (two notes falling), `3`: battery low (two short pulses
      on one note).
   3. Plug the DELL back in, wait 30 s, click the console, press `1`: the chime should come from the laptop
      speakers, not the screen. The console's `sound` line names the device kind it played on.
   Answer: clear? Told apart? Too loud or too soft at your usual volume? Step 3: laptop or screen?
7. **Open log.** Left-click the icon, click **Open log**: a sample log opens in your default app for `.log` files.
   Close it when done. Answer: did it open, and in which app?
8. **Optional, high contrast.** Skip if in doubt: Phase 0 never checked that turning contrast off restores your
   theme, so the save comes first.
   1. Settings > Personalization > Themes. If the current theme shows a **Save** button, click it and name it "Haik
      before contrast". If there is no Save button, note which theme is selected now.
   2. Settings > Accessibility > Contrast themes > pick one (for example Night sky) > **Apply**. Look at the icon and
      open the menu.
   3. Settings > Personalization > Themes > click "Haik before contrast" (or the theme you noted).
   Answer: icon and menu right in high contrast? Your theme back as it was? Or "skipped".
9. **Optional, scale.** The tray is on the DELL screen (150% at the 2026-11-04 02:14 read; the laptop screen, 125%,
   shows no tray). Settings > System > Display, click the DELL screen's box at the top, Scale > **100%**. Look at
   the icon and open the menu. Then Scale back to the value it had. Answer: crisp at 100%? Or "skipped".
10. **Optional, Explorer restart.** Only while no other agent runs (the orchestrator says when). Ctrl+Shift+Esc >
    Processes > right-click **Windows Explorer** > **Restart**. The taskbar goes and comes back (open File Explorer
    windows close). Answer: did the demo icon come back by itself, still pinned? Or "skipped".
11. **Your answers.**
    1. The icon: the Sunrise is the pick (`docs/brand.md`). Confirm it now that you saw it on your taskbar, or pick
       another from the sheets (`docs/research/img/ux/icon-sheet-dark.png`, `icon-sheet-light.png`: rows Sunrise,
       Cup, Eye, Sky; each holding, idle, off).
    2. The text list (`docs/tasks/p5-text.md`, "Text list for Haik"): any wording to change. Since
       `p2-6-sleep-route`, two notices have new words (the PC did not go to sleep, could not lock) and the "you
       changed a setting" notice names three more settings; the agent shows you the current list from `text.rs`
       before this step.
    3. Click the console, press `q`: the demo icon goes away.

After your steps the agent reads your settings again, removes the demo's leftovers and records your answers
(deliverable 4).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

1. `docs/tasks/p5-icon-art.md` Result still names the Cup as the pick; `docs/brand.md` and `art.rs` say the Sunrise
   (f47391d). A reader of that card alone gets the old pick.
2. The text list in `docs/tasks/p5-text.md` ("Text list for Haik") is stale once `p2-6-sleep-route` lands: that card
   changes the words of `HibernateFailed` and `LockFailed` and adds three settings to `UserChanged`. Step 11.2 asks
   the Part B worker to show Haik the current list from `text.rs`.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 11:40: Part A started (session `wakeshot-12`): deliverable 1 (`ux_all`) and deliverable 3 (steps for
  Haik). No screen, sound or settings use in this stop. The repo tree does not compile (`p2-6-sleep-route` mid-change),
  so the example is built and checked in a clean export of commit f47391d in the scratchpad, then copied in.
- 2026-11-04 11:55: `ux_all.rs` written in the export (`present::build` over a sample state, `OwnerDrawnMenu`,
  `BalloonNotifier` on a notice thread, `WasapiPlayer`, `shell::open_with_default_app`, console keys through
  `ReadConsoleInputW`). Debug build of the example passes (x64); `rustfmt --edition 2024 --check` clean.
- 2026-11-04 12:10: the sample state split out of `Demo` as `Sample` (no Windows call), with 7 tests that check what
  each of Haik's icon steps shows. Clippy clean on both targets, with and without `--all-targets`, and on the
  example's tests (`--profile test`). ARM64 debug build of the example passes. Copied into the repo; repo copy
  byte-equal and `rustfmt --check` clean. Not built in the repo (tree mid-change by `p2-6-sleep-route`).
- 2026-11-04 12:25: "Steps for Haik" written. Part A done; Part B waits for the screen slot.

## Result

What was done, files changed, checks and outcomes, open items. Include Haik's answers per step.

**Part A done (2026-11-04 12:25). Part B waits for the screen slot with Haik** (deliverables 2, 4 and 5). No screen,
sound, settings change or demo run in Part A.

### Part A

`crates/wakeshot-windows/examples/ux_all.rs` (new, 800 lines with tests): `ux_all [--minutes N] [--logs <dir>]`.

- The demo icon (`IconKind::Demo`) with the real `OwnerDrawnMenu`. The menu is `present::build` over a sample state,
  so the status line, tooltip, checks and enabled rows are the product's own words from `text.rs`. Starts as the
  product's menu sketch: Automatic on, two Claude Code sessions, Keep awake off (holding).
- Picks: Automatic, Keep awake (with its minutes left and its run-out) and Keep screen on change the sample, and the
  icon and status line follow; Restore shows `RestoredByUser`; Open log opens the sample log through
  `shell::open_with_default_app`; Quit quits.
- Keys in the console (one key, no Enter): `n` the next of 15 notices (every `Notice` kind, plus the failed journal
  loss), `f` first run, `1` `2` `3` the chimes (`WasapiPlayer::play`), `l` `LockWorkStation` then `IdleLocked` 10 s
  later, `a` the sample sessions stop or start, `q` quit; any other key prints the key list.
- As the product: notices and opens run on their own thread through `Box<dyn Notifier>`; a balloon click takes
  `take_click` and opens `ms-settings:taskbar` or the log; a notice refused while the taskbar was gone shows again at
  `Note::Added` (`icon_added`). Every event, pick, note and result prints with the wall time.
- Sample log: the product's `LogWriter` in `ux-all-logs` next to the exe (the build folder), with a start line and
  the chimes' own log lines.
- Quits on `q`, Quit, Ctrl+C, closing the console, `WM_CLOSE` (`taskkill /PID` without `/F`) or after N minutes
  (default 60), removing the icon each time. Changes no setting.

Picks (principal designer), each with its trigger:

1. Menu from `present::build`, not hand-built, so Haik sees the product's rows. Change it when the real tray exe runs
   (integration): then the check moves to it.
2. Extra key `a` (not in the card): the only way to see idle with Automatic on, since the demo has no real agents.
   Change it when Haik finds it confusing: start with the sessions stopped instead.
3. The locked notice is `IdleLocked`, the one the product shows while the PC is locked (Keep screen on locked it). It
   is silent. Change it when the locked check should also test a notice with a sound: use `RestoreFailed`.
4. Keys through `ReadConsoleInputW` on a thread, so no Enter is needed; Ctrl+C and a console close quit through the
   loop, so no dead icon stays (the close handler waits up to 4 s for the icon to go).
5. Other threads reach the main thread through a channel, a `WM_APP` thread message to wake it, and a 250 ms timer.
   Reason: a modal loop (the open menu) drops thread messages (Raymond Chen's blog, from memory, not verified here),
   so the timer drains what a lost wake left. Change it when the demo moves onto the product's loop window, whose
   window messages survive the modal loop.
6. Keep screen on clears when Keep awake ends, as the core reads it from the Keep awake file, which goes then
   (`core/view.rs`: `screen_on: found.screen`).
7. Default run time 60 minutes (the other demos: 10), for about 25 minutes of steps plus setup.
8. Tests inside the example (`Sample` has no Windows call): they check what each icon step shows, with words built
   from `text`, never literals.

Checks, run in the clean export of f47391d (the repo tree is mid-change by `p2-6-sleep-route`), build folder
`<scratchpad>\p5-haik-check\target`:

- `rustfmt --edition 2024 --check` on `ux_all.rs`: clean (export and repo copy, which is byte-equal).
- `cargo clippy --locked -p wakeshot-core -p wakeshot-windows --all-targets --target <t> -- -D warnings`: clean, x64
  and ARM64. The same without `--all-targets`: clean, both. `cargo clippy --locked -p wakeshot-windows --example
  ux_all --profile test` (the example's tests): clean, both.
- `cargo test --locked -p wakeshot-windows --example ux_all`: 7 passed.
- `cargo build --locked -p wakeshot-windows --example ux_all`: passes, x64 and ARM64 (debug).
- Not run: the card's release builds of `wakeshot-tray` and `wakeshot-cli` (the orchestrator's rule for this stop:
  debug builds only), and every check in the repo itself. **The in-repo build waits for `p2-6-sleep-route`**; the
  orchestrator reruns these checks in the repo after that card. The example uses no item that card changes (checked
  12:25 by comparing the two trees: `view.rs` and `present.rs` are the same, `text.rs` keeps its public functions,
  `SettingKey::LidAc`, `LidDc`, `SleepDc` stay; in `wakeshot-windows/src` only `power/`, `hooks.rs` and
  `recovery.rs` differ, which the example does not use).

Files: `crates/wakeshot-windows/examples/ux_all.rs` (new), this card. My copy of `ux_all.rs` also sits in the export
(`<scratchpad>\head-f47391d\crates\wakeshot-windows\examples\`). The build folder is kept for Part B; delete it when
the card ends.

Open items for Part B:

- Deliverable 2: `ux-all.cmd` with the token line, start at Medium, pin, the reads. Also read "Show notifications on
  the lock screen" (Settings > System > Notifications) before step 5: the locked result depends on it (not in the
  card's read list; its registry place not verified).
- Check the bracketed values in the steps (Dark mode, DELL at 150%) against the fresh reads.
- Not verified until the screen run: the demo's name under "Other system tray icons", and that Win+B, the arrows and
  Enter reach the pinned icon.
