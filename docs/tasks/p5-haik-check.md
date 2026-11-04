# Haik's look, listen and click check of the tray UX

Phase: 5. Status: done (2026-11-04 16:40). Needs Haik: **yes**, about 25 minutes at the laptop (eyes, ears, hands), plus his answers to
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

Ready since 2026-11-04 15:58 (Part B worker): values in brackets checked against the reads at 15:51 to 15:53. The
demo quits by itself at about 17:53 (`--minutes 120`).

Before you start: an agent has started the demo `ux_all` at Medium from `ux-all.cmd`, pinned its icon next to the
clock (on the DELL taskbar, just right of the ^ arrow), and read your Windows mode, Do not disturb, audio device and
volume, sound scheme and display scale. The icon is a sun: holding is a risen half sun with rays, idle the empty half
sun, off the empty half sun struck through. A Windows Terminal window titled **"ux-all.cmd: Wakeshot demo"** (on the
DELL screen, upper left) prints one line per event; it takes the keys below. **Keys work only while that window has
focus**: click its title bar first, and again after each use of the menu or Settings. Pick menu items with the mouse
unless a step says the keyboard. Quit the demo only with `q` (step 11.3), not with the window's X. Do not close
Settings or Notepad windows for the agent; close your own as you like.

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
   icon and open the menu. Then set "Choose your mode" back to what it was (**Dark** at the agent's read, 2026-11-04
   15:52). Answer: icon and menu right in light mode?
4. **Notices.** Click the console, press `n`, read the banner at the bottom right; press `n` again for the next one.
   There are 15; the console prints "1 of 15" and so on, with each one's name. Then press `f` and click its banner:
   Settings opens the Taskbar page. Open "Other system tray icons" and find the demo there (it may be named `ux_all`
   rather than Wakeshot: not verified). If no banner shows at all, tell the orchestrator (Do not disturb was off at
   the agent's read, 15:52; check that you did not turn it on). The words are also listed below the steps.
   Answer: each one readable, short, clear? Which ones would you word otherwise?
5. **Locked.** Click the console, press `l`. The PC locks at once; the demo shows a notice 10 s later ("Wakeshot
   locked the PC"). Wait 30 s, then sign in. Your "Show notifications on the lock screen" is on (agent's read, 15:52).
   Answer: did the banner show on the lock screen, right after sign-in, only in the notification centre (Win+N), or
   nowhere?
6. **Chimes.**
   1. Unplug the DELL dock cable (the laptop goes on battery; the console window may move to the laptop screen).
   2. Click the console, press `1`, wait 3 s, press `2`, wait 3 s, press `3`. Repeat as you like. `1`: lid closed
      while holding (two notes rising), `2`: going to sleep (two notes falling), `3`: battery low (two short pulses
      on one note).
   3. Plug the DELL back in, wait 30 s, click the console, press `1`: the chime should come from the laptop
      speakers, not the screen. The console's `sound` line names the device kind it played on.
   The chimes play on the laptop speakers (volume 67% at the agent's read, 15:52), not on your default output, the
   DELL screen (20%). After the replug Windows picks the default output itself; the agent reads it after your steps
   and tells you if it moved.
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
9. **Optional, scale.** The tray is on the DELL screen (150% at the agent's read, 2026-11-04 15:52; the laptop
   screen, also 150% at that read, shows no tray). Settings > System > Display, click the DELL screen's box at the top, Scale > **100%**. Look at
   the icon and open the menu. Then Scale back to the value it had. Answer: crisp at 100%? Or "skipped".
10. **Optional, Explorer restart.** Only while no other agent runs (the orchestrator says when). Ctrl+Shift+Esc >
    Processes > right-click **Windows Explorer** > **Restart**. The taskbar goes and comes back (open File Explorer
    windows close). Answer: did the demo icon come back by itself, still pinned? Or "skipped".
11. **Your answers.**
    1. The icon: the Sunrise is the pick (`docs/brand.md`). Confirm it now that you saw it on your taskbar, or pick
       another from the sheets (`docs/research/img/ux/icon-sheet-dark.png`, `icon-sheet-light.png`: rows Sunrise,
       Cup, Eye, Sky; each holding, idle, off).
    2. The text list: "Current words for step 11.2" below, from `text.rs` today (it replaces the stale list in
       `p5-text.md`). Since `p2-6-sleep-route`, notices 8 and 9 have new words and notice 2 names six settings. Any
       wording to change?
    3. Click the console, press `q`: the demo icon goes away.

After your steps the agent reads your settings again, removes the demo's leftovers and records your answers
(deliverable 4).

### Current words for step 11.2

From `crates/wakeshot-core/src/text.rs`, read 2026-11-04 15:58. The underlined access key is each row's first
letter, except Keep **s**creen on.

Menu, top to bottom:

- Status line, for example "Holding awake: Claude Code (2 sessions)". Other heads: "Holding awake, screen on",
  "Holding awake, lid open only", "Cannot hold awake", "Not holding, the PC sleeps as usual", "Not holding",
  "Automatic is off, the PC sleeps as usual", "Battery low, not holding". Groups after the colon: "Claude Code (2
  sessions)", "wakeshot run", "a hold file", "Keep awake (45 min left)", "Keep awake (until turned off)", "and 2 more".
- Automatic
- Keep awake > Off, 30 minutes, 1 hour, 2 hours, 4 hours, Until I turn it off, (line), Keep screen on
- Restore my power settings
- Open log
- Quit

Tooltip: "Wakeshot" on the first line, the status line on the second.

Notices, in the order `n` shows them (title: body). W: warning icon. Silent: no Windows sound.

1. Wakeshot is running: Its icon may hide under the ^ arrow on the taskbar. To keep it in view, click this notice,
   then turn on Wakeshot under "Other system tray icons". (Click opens the taskbar settings.)
2. Wakeshot keeps your change: You changed what closing the lid does when plugged in. Wakeshot leaves it as you set it
   until the current work ends. (The other five: what closing the lid does on battery; when the PC sleeps when plugged
   in; when the PC sleeps on battery; when the screen turns off when plugged in; when the screen turns off on
   battery.)
3. Your power settings are back: Wakeshot put back your lid and sleep settings and leaves them alone until the work
   ends. It still keeps the PC awake, but only with the lid open.
4. W, silent (a chime plays with it). Battery low: the PC may sleep now: The battery is under 20%, so Wakeshot stopped
   keeping the PC awake and put back your power settings. Plug in to keep it awake again.
5. W. Wakeshot cannot change power settings: Something, such as a company policy, blocked the change. Wakeshot still
   keeps the PC awake, but only while the lid is open.
6. W. Wakeshot cannot keep the PC awake: Windows did not accept Wakeshot's request, so the PC may sleep while your work
   runs.
7. W. Could not put back your power settings: Wakeshot keeps trying. Until it works, closing the lid may not sleep the
   PC, and on battery it may not sleep at all. Click to see your old values in the log. (Click opens the log.)
8. W, new. The PC did not go to sleep: It could not hibernate either. Windows should sleep it once nothing else keeps
   it awake. Opening the lid puts your screen and sleep settings back.
9. W, new. Wakeshot could not lock the PC: Lock it with Windows key + L. With the lid closed, Wakeshot tries again; if
   that fails too, it puts the PC to sleep after 5 minutes with no input.
10. The PC ran with the lid closed: The PC kept running for 2 h 15 min with the lid closed. What held it: Claude Code
    and cargo.
11. W. Update Wakeshot: Claude Code was likely updated, so Wakeshot cannot fully read what it is doing. Until you
    update Wakeshot, it may not keep the PC awake as it should.
12. W. Power settings set to Windows defaults: Wakeshot could not read its record, so it reset your lid and sleep
    settings. If you had your own, set them again; click to see the old values. (Click opens the log.)
13. W. Check your power settings: Wakeshot could not read its record or reset every setting. Set your lid, screen and
    sleep settings by hand; click to see the old values. (Click opens the log.)
14. Silent. Wakeshot locked the PC: Keep screen on is on, and the PC had no input for your usual sleep time, so
    Wakeshot locked it. Your work keeps running.
15. Silent. Wakeshot updated to 1.1.0: The new version is running.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

1. `docs/tasks/p5-icon-art.md` Result still names the Cup as the pick; `docs/brand.md` and `art.rs` say the Sunrise
   (f47391d). A reader of that card alone gets the old pick.
2. The text list in `docs/tasks/p5-text.md` ("Text list for Haik") is stale once `p2-6-sleep-route` lands: that card
   changes the words of `HibernateFailed` and `LockFailed` and adds three settings to `UserChanged`. Step 11.2 asks
   the Part B worker to show Haik the current list from `text.rs`.

3. `docs/tasks/p5-menu-view.md` pick 5 ("above the anchor when it is in the lower half of its monitor ... Change it
   when Part B shows the menu covering the icon") is out of date: its trigger fired in this check, and `placement` in
   `menu.rs` replaces it. Not edited (that card's file).

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
- 2026-11-04 15:48: Part B started, first half (deliverable 2). Haik is at the laptop, session unlocked. Another worker
  (`p2-6-props-rare`) builds core tests in the background (no screen, sound or power settings). Build folder
  `<scratchpad>\p5-haik-check\target` in this session's scratchpad (`469e45e6-...`); Part A's export is in another
  session's scratchpad and is not used: the build is from the repo tree.
- 2026-11-04 15:50: debug x64 build of `ux_all` from the repo tree (with `p2-6-props-rare`'s uncommitted core edits in
  it): clean, 43 s. Copied to `<scratchpad>\p5-haik-check\run\ux_all.exe`, so the running demo locks no file in the
  build folder and its sample log sits in `run\ux-all-logs`.
- 2026-11-04 15:51 to 15:53, reads before (`reads.ps1`, `notif-read.ps1`; files `reads-before.txt`,
  `notif-before.txt`): Windows mode Dark (`AppsUseLightTheme` 0, `SystemUsesLightTheme` 0, `ColorPrevalence` 0,
  `EnableTransparency` 1); high contrast off (`Flags` 126); sound scheme `.Default`; default output (console,
  multimedia and communications) "4 - DELL U2723QE (AMD High Definition Audio Device)", form factor 9 (screen audio),
  20%, not muted; laptop "Speakers (Realtek(R) Audio)" 67%, not muted; Do not disturb Off; "Show notifications on
  the lock screen" On (`SystemSettings_Notifications_ToastsAboveLock_CheckBox`; no `NOC_*` registry values, so the
  registry place stays not verified); Notifications On; "Allow notifications to play sounds" On; displays: DELL
  3840x2160 at 150% (primary, the tray), laptop 2304x1440 physical at 150%; on AC, 100%; not locked. Settings was
  opened by the script and closed by it (none was open before).
- 2026-11-04 15:53:10: `explorer.exe <scratchpad>\p5-haik-check\ux-all.cmd` (`--minutes 120`, stdout through Git's
  `tee -a -i` to `ux-all.log` and the window). Token line `Medium Mandatory Level S-1-16-8192`. Demo PID 34284
  (parent cmd 28424; tee 15684; its own Windows Terminal, PID 15416, window titled "ux-all.cmd: Wakeshot demo").
  `15:53:12.875 icon: Added { attempt: 1 }`. One stray `key 'm'` came in at the start, before anyone typed in it:
  not explained (a terminal start-up sequence, or a key Haik typed as the window opened); it only prints the key
  list, the sample state stayed at the start.
- 2026-11-04 15:53:37: new `NotifyIconSettings\12880751163260563509` (only `IconSnapshot`); `IsPromoted` = 1 written.
  15:53:54: `Shell_NotifyIconGetRect` 3312,2088 to 3360,2160 (48x72) on the DELL taskbar (DPI 144); the strip grab
  shows the holding sun right of the ^ arrow (`strip-pinned.png`, scratchpad only).
- 2026-11-04 15:58: bracketed values in the steps checked: Dark mode and DELL 150% hold; the laptop screen is 150%,
  not 125% (step 9 fixed). Steps 4, 5 and 6 now name the DND, lock-screen and speaker reads. The current words from
  `text.rs` put under the steps for step 11.2.
- 2026-11-04 15:54 to 15:57: the card's checks in the repo tree (`checks.ps1`, `checks-before.log`), all exit 0: see
  the Result.
- 2026-11-04 15:58: demo alive, icon still at the same rect, `IsPromoted` 1, no key since the stray `m`. Status
  `blocked`: waiting for Haik's steps. Deliverables 4 and 5 follow his answers.
- 2026-11-04 16:30: Haik's answers, relayed by the orchestrator. He checked the demo his own way, not step by step:
  "I checked demo in my way and everything seems ok besides 2 comments below: 1. Notification header has weird name
  ux_all.exe 2. When you click tray icon the the panel opens on click place but it should open above taskbar for not
  hiding taskbar icons." Recorded as: icon (Sunrise) and words OK as they are, no wording change. The orchestrator
  extends deliverable 4: find what names the banner header and fix it in the tray build and `ux_all`; open the menu
  above the taskbar (menu code, with placement tests); rebuild and restart the demo for Haik. Status `in progress`.
- 2026-11-04 16:21: the demo's log (`ux-all-run1.log`) shows what Haik ran: notices 1 to 15 twice and part of a
  third round (keys `n`, 16:06 to 16:19), `a` four times, menu picks Open log (opened, `Ok`), Keep awake 30 minutes,
  Off, 4 hours, Restore, Automatic off and on; Light mode at 16:14:46 (glyph redrawn near black in 60 ms) and back to
  Dark at 16:15:46; quit with `q` at 16:19:57 (`icon removed`). No chime key, no `l`, no `f`, no balloon click. Two
  more stray keys (`k` 16:06:30, `s` 16:15:04) printed the key list only. Demo processes gone (PID 34284, cmd, tee).
- 2026-11-04 16:22: reads after Haik's run (`reads-after-haik.txt`, `notif-after-haik.txt`): every value equal to the
  reads before (Dark, DND Off, lock-screen notifications On, DELL default output 20%, speakers 67%, `.Default`, both
  screens 150%, high contrast off). Settings was open this time (Haik's, PID 27148): the read script moved it to the
  Notifications page and unfolded the Notifications row, and left it open.
- 2026-11-04 16:23: banner name. `HKCU\Software\Classes\AppUserModelId\NotifyIconGeneratedAumid_12880751163260563509`
  had `DisplayName` "ux_all.exe": `ux_all.exe` has no version resource. The tray exe gets `FileDescription` and
  `ProductName` "Wakeshot" from `crates/build-resources.rs`, and research (`p6-int-dry-run-e2e`) saw the dry-run
  tray's header say "Wakeshot" on screen. No Microsoft page names the source of the header (web search 2026-11-04).
- 2026-11-04 16:25: menu place. `menu.rs` aligned the menu's bottom to the click point (`TPM_BOTTOMALIGN` at the
  click's y), so a menu opened from the middle of the taskbar covered its upper half. New `placement`: a point outside
  the monitor's work area (on the taskbar) moves to the work area's edge that faces the taskbar; a point inside it
  stays. `Note::Opened` gets `at`. Owner line now names this card for the change. 4 new tests replace the old
  alignment test; `tray::menu` tests 28 passed.
- 2026-11-04 16:27: `ux_all.rc` in the scratchpad (the tray's `VERSIONINFO` with `OriginalFilename` "ux_all.exe", and
  the app icon), compiled with the SDK's `rc.exe` (10.0.26100.0), linked with `cargo rustc --locked -p
  wakeshot-windows --example ux_all --target x86_64-pc-windows-msvc -- -C link-arg=<ux_all.res>`: exe reads
  `FileDescription` and `ProductName` "Wakeshot". `ux_all.rs` doc says how. The three demo keys of run 1 exported
  (`leftover-*-run1.reg`) and deleted, checked gone.
- 2026-11-04 16:27:31: run 2 started the same way: token line Medium (S-1-16-8192), PID 39952, `Added { attempt: 1 }`
  16:27:34; `IsPromoted` = 1 at 16:27:35. No stray key this time.
- 2026-11-04 16:28: self-check (`verify.ps1`, `verify.log`). Synthetic left click on the icon centre (3336,2124):
  `menu: Opened { ..., at: Anchor { x: 3335, y: 2088 } }`; the menu window `#32768` at 3327,1772 to 3840,2088, its
  bottom on the taskbar's top edge (2088), slid 8 px left by Windows to stay on the monitor; Esc closed it
  (`menu-placed.png`, scratchpad only). Then `n` in the console: the first-run banner's header reads "Wakeshot" with
  the sun glyph (`banner-name.png`), and the AUMID key's `DisplayName` is "Wakeshot".
- 2026-11-04 16:30: checks after the fixes (`checks-after-fix.log`), all exit 0: see the Result. 16:30:40, the Keep
  awake submenu by a synthetic hover: it opened left of the menu and upward, 3087,1572 to 3330,1933, clear of the
  taskbar (`submenu-placed.png`). Research section "Tray UX: Haik's check" added. Status `blocked`: Haik looks at the
  menu place and one banner header in run 2 (PID 39952); then the final reads and leftovers.
- 2026-11-04 16:35: Haik's words on run 2, relayed: "It's perfect fixed the place and also tested step 5 and 6 which
  missed. l locks without any sound and after 10 seconds notification comes but as locked we cant see content is
  private. in step 6 all 3 sounds I hear both docked and undocked. BTW sometimes notification disappears so fast
  that I cant manage to read, but not sure it is windows stuff or yours." Run 2's log (`ux-all-run2.log`): `l` at
  16:31:34 and 16:34:04, each `idle-locked` shown 10 s later (`Ok`, silent by design); chimes undocked 16:33:01 to
  16:33:07 "on speakers (the default device)", docked again 16:33:33 to 16:33:36 "on speakers (speakers instead of
  the default screen audio)"; the icon redrew at 20 px while undocked (16:32:53, the laptop taskbar) and at 24 px
  docked (16:33:27); quit with `q` 16:34:30, `icon removed`. Haik did not comment on the banner header.
- 2026-11-04 16:36: fast banners. `SystemParametersInfoW(SPI_GETMESSAGEDURATION)` and `MessageDuration` under
  `HKCU\Control Panel\Accessibility`: 5 s (read only). Run 3 (16:37:42, PID 34240, `pair.ps1`): two notices 1.7 s
  apart; grabs every 0.4 s: the first banner stayed about 6 s, the second did not replace it and showed after it,
  for about 6 s; `q` 16:38:03.
- 2026-11-04 16:38: the IDE's errors in `menu.rs` were from the middle of the edit; on the files now: `rustfmt
  --check` clean, clippy x64 with and without `--all-targets` clean, `cargo test --locked -p wakeshot-windows` 566
  passed, 23 ignored (`checks-final.log`).
- 2026-11-04 16:39: final reads: all equal to before except the DELL output, now "3 - DELL U2723QE" (a new endpoint
  id) at 18%, where it was "4 - DELL U2723QE" at 20%: Windows made a new device at Haik's dock replug in step 6; still
  the default, still screen audio. Settings was Haik's (left open). Three demo keys exported (`leftover-*-final.reg`)
  and deleted; no `ux_all` process. Build folder and `run\` deleted, checked gone. Left: the shell's
  `%TEMP%\NotifyIconGeneratedAumid_12880751163260563509.png` (not created by this card's code; outside the
  scratchpad).
- 2026-11-04 16:40:03: second leftover read, 64 s after the removal: the three keys absent, no `ux_all` process, only
  Explorer's own `NotifyIconGeneratedAumid_11595054434325676090`. Research section extended with run 2 and 3. Status
  `done`.

## Result

What was done, files changed, checks and outcomes, open items. Include Haik's answers per step.

**Done 2026-11-04 16:40.** Part A 12:25; Part B 15:48 to 16:40: Haik checked the demo, two findings fixed (banner
name, menu place), steps 5 and 6 run after the fix, leftovers gone. No screen, sound, settings change or demo run in
Part A.

### Part B, first half (deliverable 2)

- Demo running: `ux_all` PID 34284 (debug x64 from the repo tree), started 15:53:10 at Medium (token line S-1-16-8192)
  through `explorer.exe <scratchpad>\p5-haik-check\ux-all.cmd`; its own Windows Terminal (PID 15416) titled
  "ux-all.cmd: Wakeshot demo"; lines also in `<scratchpad>\p5-haik-check\ux-all.log`; quits by itself about 17:53.
  Pinned: `IsPromoted` = 1 on `NotifyIconSettings\12880751163260563509`; icon on the DELL taskbar right of the ^ arrow.
- Reads before (15:51 to 15:53; the Log has every value): Dark mode; Do not disturb Off; lock-screen notifications On;
  default output the DELL screen audio at 20% (the chimes pick the laptop speakers, 67%); sound scheme `.Default`;
  high contrast off; DELL and laptop both at 150%; on AC, 100%.
- Card fixed: the laptop screen is 150%, not 125% (step 9); steps 3 to 6 name today's reads; the current words from
  `text.rs` are under the steps ("Current words for step 11.2").
- Checks before Haik's steps, in the repo tree with `p2-6-props-rare`'s uncommitted core edits present, build folder
  `<scratchpad>\p5-haik-check\target`, all exit 0: `rustfmt --edition 2024 --check` on `ux_all.rs`; `cargo clippy
  --locked -p wakeshot-core -p wakeshot-windows --all-targets --target <t> -- -D warnings` for x64 and ARM64, and the
  same without `--all-targets`, both; `cargo test --locked -p wakeshot-windows --example ux_all`: 7 passed; `cargo
  build --release --locked -p wakeshot-tray -p wakeshot-cli` x64 (1 min 23 s); `cargo build --release --locked -p
  wakeshot-cli` ARM64 (the tray's ARM64 build is CI only, as the card says).

### Part B, Haik's answers and the two fixes (deliverable 4, extended by the orchestrator)

Haik, 2026-11-04 about 16:30, checked the demo his own way: "I checked demo in my way and everything seems ok besides
2 comments below: 1. Notification header has weird name ux_all.exe 2. When you click tray icon the the panel opens
on click place but it should open above taskbar for not hiding taskbar icons."

| Step | Answer |
|---|---|
| 1. Icon states | OK (Haik). The log shows idle, off and holding (`a`, Automatic, Keep awake 30 minutes and 4 hours). |
| 2. Menu | Opened on the click, covering the taskbar: **fixed** (below); run 2: "It's perfect fixed the place". The Win+B keyboard open: not checked step by step; Haik saw no problem. |
| 3. Light mode | OK (Haik). Light 16:14:46, Dark again 16:15:46; reads after equal. |
| 4. Notices | Words OK, no change. Header "ux_all.exe": **fixed** for the demo (the agent's grab in run 2 shows "Wakeshot"; Haik did not comment on it); the tray exe already says "Wakeshot". The `f` click and "Other system tray icons": not checked step by step; Haik saw no problem. Some banners went "so fast that I cant manage to read": Windows' 5 s setting (below). |
| 5. Locked | Run 2: `l` locks with no sound, and 10 s later the banner showed on the lock screen with its content hidden as private (Windows' lock screen privacy). No sound is by design: `IdleLocked` is silent (`text.rs`, Phase 2 track note 11). The locked notice is **not lost**. |
| 6. Chimes | Run 2: all three heard, docked and undocked. Undocked they played on the speakers as the default device; after the replug "on speakers (speakers instead of the default screen audio)", so on the laptop, as designed. Loud or soft: not said. |
| 7. Open log | OK: Open log opened the sample log (`Ok`); the app was not named. |
| 8. High contrast | Not checked step by step; Haik saw no problem. |
| 9. Scale | Not checked step by step; Haik saw no problem. |
| 10. Explorer restart | Not checked step by step (no `TaskbarCreated` in the log). |
| 11. Answers | Icon: the Sunrise stays. Words: OK as they are, no wording change. Quit with `q` 16:19:57. |

So `art.rs`, `assets/wakeshot.ico` and `text.rs` are unchanged.

Fix 1, banner header name:

- What names it: the shell writes the name it read as `DisplayName` under
  `HKCU\Software\Classes\AppUserModelId\NotifyIconGeneratedAumid_<id>`: "ux_all.exe" for the demo, which has no
  version resource; Explorer's own shows as "Windows Explorer", its `FileDescription`. No Microsoft page names the
  source (web search).
- The real tray exe (installed or dry run) already has `FileDescription` and `ProductName` "Wakeshot"
  (`crates/build-resources.rs`), and `p6-int-dry-run-e2e` saw its header say "Wakeshot" on screen. The installed app:
  "Wakeshot", inferred (same exe and resource; not checked on an install). No change to the tray build.
- The demo: `ux_all.rc` (scratchpad) with the same `VERSIONINFO` and the app icon, linked through `cargo rustc ... --
  -C link-arg=<ux_all.res>`; the `ux_all.rs` doc says how. The run-1 name key deleted first. Verified at 16:28: header
  "Wakeshot" with the sun glyph, key `DisplayName` "Wakeshot".

Fix 2, menu place (`crates/wakeshot-windows/src/tray/menu.rs`, Owner line now also names this card):

- `placement`: a click or key-select point outside the monitor's work area, so on the taskbar, moves to the work
  area's edge that faces the taskbar (bottom: the menu's bottom on the taskbar's top edge; top, left, right the same
  way); along that edge it stays at the point, and Windows keeps the menu inside the monitor. A point inside the work
  area (the overflow flyout; an auto-hidden taskbar) stays as before. Left or right from `SM_MENUDROPALIGNMENT` as
  before. `Note::Opened` now carries `at`, the aligned point.
- Tests (4, replacing the alignment test): `a_bottom_taskbar_menu_sits_on_the_taskbar_top_edge` (Haik's DELL numbers,
  click and key anchors, first and last taskbar row, right-hand drop),
  `top_and_side_taskbars_anchor_to_the_edge_facing_the_work_area`, `a_point_inside_the_work_area_is_kept` (flyout,
  auto-hide, failed monitor read),
  `a_monitor_above_the_primary_keeps_its_negative_edges`.
- Verified at 16:28 on run 2: the menu window's bottom at 2088, the taskbar's top; the Keep awake submenu opened
  upward, clear of the taskbar.

Fast banners (Haik: "sometimes notification disappears so fast that I cant manage to read"): Windows, not our code.

- His "Dismiss notifications after this amount of time" (Settings > Accessibility > Visual effects) is 5 s
  (`SPI_GETMESSAGEDURATION` and `MessageDuration`, read only). 5 s is Windows' default per third-party guides
  (ElevenForum, GeekRewind); Microsoft's `SystemParametersInfoW` page names no default.
- Run 3: a banner stays about 6 s with input going on; a second notice does not replace it but shows after it.
  Without input a banner stays up (`p5-notices`). `uTimeout` is ignored since Vista (Microsoft's
  `NOTIFYICONDATAW` page), so the app cannot ask for longer.
- A banner that went is gone: Microsoft's `Shell_NotifyIconW` page says a Windows 11 banner that timed out or was
  dismissed is not shown in the notification centre, and `p5-notices` found no entry there.
- What helps: Haik can set that setting to 7 s or more (his own setting, not changed here). For the product, a fix
  card could shorten the four-line bodies, or move notices to registered WinRT toasts, which may stay in the
  notification centre (not verified; Phase 0 saw a registered toast show, not whether the centre kept it). Product
  call for Haik; not in this card's scope.

Final reads (16:39): equal to the reads before, except the DELL output: a new device "3 - DELL U2723QE" at 18%
(was "4 - DELL U2723QE" at 20%), made by Windows at the dock replug; still the default output. Not changed by the
agent; Haik may want to set its volume.

Checks after the fixes (16:28 to 16:30, `checks-after-fix.log`), all exit 0: `cargo test --locked -p wakeshot-windows
--lib tray::` 63 passed, 2 ignored; `rustfmt --edition 2024 --check` on `ux_all.rs` and `menu.rs`; clippy `-D
warnings` for `wakeshot-core` and `wakeshot-windows`, x64 and ARM64, with and without `--all-targets`; `cargo test
--locked -p wakeshot-windows --example ux_all` 7 passed; release builds of `wakeshot-tray` and `wakeshot-cli` (x64) and
`wakeshot-cli` (ARM64).

Picks (principal designer):

1. The demo runs from a copy in `run\`, not from the build folder, so the checks can rebuild while it runs. Change
   it when a check must run the very exe Haik sees.
2. Its lines go to the window and a log through Git's `tee -a -i` in the `.cmd` pipeline: the demo's stdin stays the
   console (cmd pipes only stdout), and `-i` keeps Ctrl+C from killing the tee under the demo. Change it when the
   demo writes its own event log.
3. The demo's version resource is linked from the scratchpad (`cargo rustc` link arg), not by a new `build.rs`: an
   embedded resource for examples needs `embed-resource` as a build dependency of `wakeshot-windows`, which changes
   `Cargo.toml` and `Cargo.lock` (a card the orchestrator runs alone). Change it when a demo is run again by people
   other than an agent: then add the `build.rs` with `rustc-link-arg-examples` through such a card.
4. The menu meets the taskbar edge with no gap, as asked. Change it when Haik finds it too tight next to Windows' own
   tray menus: add a gap of a few DIPs in `placement`.
5. The taskbar's edge comes from the monitor's work area, not `SHAppBarMessage(ABM_GETTASKBARPOS)`. Change it when an
   auto-hidden taskbar shows the menu over it (its work area is the whole monitor, so the click point is kept).

Ending (16:34 to 16:40): Haik quit run 2 with `q`; run 3 (the banner test) quit by its own `q`. Final reads done
(above). The three demo keys exported and deleted, gone at 16:38:59 and 16:40:03; no demo process. Build folder and
`run\` deleted. Haik's words and runs 2 and 3 are in `docs/research.md`, "Tray UX: Haik's check". After the IDE showed
errors from mid-edit, `menu.rs` and `ux_all.rs` passed again (16:38): fmt, clippy x64 with and without
`--all-targets`, `cargo test --locked -p wakeshot-windows` 566 passed, 23 ignored.

Open items for other cards: the fast-banner choice (a setting for Haik, or shorter bodies or registered toasts:
product call); `p5-menu-view.md` pick 5 is stale (Found outside scope 3); the steps not checked step by step (the
keyboard open, the `f` click, high contrast, 100% scale, Explorer restart) are left to `p6-int-haik-checks` if
wanted.

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
