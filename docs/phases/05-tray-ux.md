# Phase 5: Tray UX (track)

Goal: everything the user sees and hears, built and tested on its own before integration: the tray icon (art for
holding, idle and off; light and dark taskbar, high contrast, every scale), the menu as Windows 11 draws its own, the
notices, the three chimes, every user-facing word, and the first-run pin hint. Plan written by `p5-plan-tray-ux`
(2026-11-03). Design: `docs/architecture.md` sections 2, 4, 5 and 8; decisions: `docs/product.md` UX and the sound
lines. This file holds only what the design leaves open for this track.

## Track notes

### 1. Files and owners

Every file below exists after the contracts change card (the orchestrator runs it alone, before any build). No card
of this track edits a module list, `lib.rs`, `main.rs`, `Cargo.toml`, `Cargo.lock`, `tray.rs`, `testing.rs` or a
`build.rs`. A missing signature is a contract change for the orchestrator, never a side edit.

| File | Card | What |
|---|---|---|
| `crates/wakeshot-core/src/text.rs` | `p5-text` | every user-facing string; the signatures are contract (Phase 2 calls them) |
| `crates/wakeshot-windows/src/tray/theme.rs` | `p5-theme` | Windows mode, high contrast and its colours; change compare |
| `crates/wakeshot-windows/src/tray/art.rs` | `p5-icon-art` | icon art as code: tray glyphs, app icon, ICO and PNG writers |
| `assets/wakeshot.ico` | `p5-icon-art` | the app icon file both exes and the installer use |
| `crates/wakeshot-windows/src/tray/menu_paint.rs` | `p5-menu-paint` | menu layout, colours, item painting (no window) |
| `crates/wakeshot-windows/src/tray/icon.rs` | `p5-tray-icon` | `TrayIcon`: its own hidden window, the notify icon, clicks, re-add, theme and display changes |
| `crates/wakeshot-windows/src/tray/menu.rs` | `p5-menu-view` | `OwnerDrawnMenu`, the `MenuView` the product uses |
| `crates/wakeshot-windows/src/notify.rs` | `p5-notices` | `BalloonNotifier`; `FakeNotifier` is already here (the contracts card moved it; its API is contract) |
| `crates/wakeshot-windows/src/sound.rs` and `sound/*.rs` it declares | `p5-sounds` | chimes, device rule, WASAPI player; `FakeSoundPlayer` is already here (moved by the contracts card; its API is contract) |
| `crates/wakeshot-windows/examples/ux_icon.rs`, `ux_menu.rs`, `ux_notices.rs`, `ux_sounds.rs`, `ux_all.rs` | the card of the same name (`ux_all`: `p5-haik-check`) | on-screen and by-ear harnesses, run before integration |
| `docs/research/img/ux/<prefix>-*.png` | prefix `icon` (`p5-icon-art`), `taskbar` (`p5-tray-icon`), `menu` (`p5-menu-paint`, `p5-menu-view`: `menu-render-*` and `menu-screen-*`), `notice` (`p5-notices`), `check` (`p5-haik-check`) | pictures |

Shared, additions only, re-read right before each edit: `docs/research.md` (one new section per card).

Not this track's, used as is: `crates/wakeshot-windows/src/shell.rs` (Phase 4, `p4-cli-commands`:
`open_with_default_app`) for Open log and the first-run click; `crates/wakeshot-core/src/present.rs` (Phase 2,
`p2-present`) builds the `TrayView` this track draws; the event sink (contracts card); the tray bin (integration).

### 2. How the parts fit

- `TrayIcon` lives on the loop thread and owns its own hidden top-level window. All icon messages go there: the icon
  callback (left click, right click, keyboard select, balloon click), `TaskbarCreated`, `WM_DISPLAYCHANGE`,
  `WM_SETTINGCHANGE`, `WM_SYSCOLORCHANGE`, and its own timers (add retry, theme settle). It opens the menu itself with
  the menu of the last `TrayView`. So the loop's window procedure never sees a tray message, and the review's window
  hook (`p1-contracts-domain` M1) is not needed for this track. The loop's message pump dispatches to it, since both
  windows are on one thread.
- `OwnerDrawnMenu` (the `MenuView`) owns one hidden owner window for `TrackPopupMenuEx` and its owner-draw messages,
  and posts the pick as `Event::Menu` through the event sink.
- `BalloonNotifier` is `Send`, runs on the IO thread, and modifies the same icon through its identity (window handle
  and GUID, a contract type in `tray.rs`). It keeps the click action of the last balloon; `TrayIcon` calls a hook on
  `NIN_BALLOONUSERCLICK`, and the hook (integration) takes that action and opens it through `shell.rs`.
- The WASAPI player is `Send + Sync`: `play` spawns one short thread per sound and posts `Done(Outcome::Sound)`
  through the event sink; `play_and_wait` blocks (hibernate thread only).
- `text.rs` (core) words everything. `present.rs` (Phase 2) decides which items, order, checks and icon state, and
  calls `text.rs` for every word.

Why own windows: this track builds and tests alone, and integration only creates the objects and runs three commands.
Change it when `p5-menu-view` Part B shows `SetForegroundWindow` on the menu's own window does not give a menu that
closes on an outside click: then the menu uses the icon's window (one handler call from `icon.rs`).

### 3. Picks this track makes

Design calls the design leaves to Phase 5 (worker as principal designer). Each has a reason and a trigger.

1. **Tray icons are drawn at run time** from `art.rs` at the size the taskbar wants, with `CreateIconIndirect`; no
   icon resources and no `LoadIconMetric`. Why: exact pixels at every scale (`LoadIconMetric` follows the system DPI
   fixed at sign-in, research "`LoadIconMetric` follows the system DPI"), contrast colours from the live theme, and
   no resource weight (the spike's four icons were 180 KB of its 519 KB). Change it when `p5-tray-icon` Part B shows
   the shell blurs or drops run-time icons.
2. **App icon** `assets/wakeshot.ico`, frames 16, 20, 24, 32, 40, 48 and 64, made by the same art code and checked by
   a golden test; embedded by both exes and used by the installer. No 256 frame: a small one needs a deflate encoder,
   which means a crate. Change it when the installer or Explorer's large view looks blurry: add `png` as a dev
   dependency through the contracts card.
3. **Glyphs are monochrome** like Windows' own tray glyphs (white on a dark taskbar, near black on a light one, the
   contrast text colour in high contrast), and the three states differ in shape, not only in colour.
4. **Left click, right click and keyboard select all open the same menu.** There is no window to open, and a left
   click that does nothing looks broken.
5. **Notices** are balloons (design). Title, body, warning, silent and click come from `text::notice`. A notice that
   comes with a chime is silent (`NIIF_NOSOUND`), so two sounds never overlap. The large icon is the app icon if it
   reads well on the banner, else the info or warning icon (`p5-notices` Part B decides).
6. **First run** (method left to Phase 5, decision 36): the balloon says how to pin the icon, and its click opens
   `ms-settings:taskbar` (Other system tray icons). Not the undocumented `IsPromoted` write, which touches a key
   Windows owns. Change it when Haik wants one-click pinning and accepts an undocumented key.
7. **Chimes are made in code** (no WAV resource), three shapes told apart by pitch direction and rhythm, each under
   1.2 s. No audio stream kept ready during a hold: the 0.3 to 0.8 s start (research, "Lid-close sound") is accepted.
   Change it when Haik says the lid-close chime comes too late. `play_and_wait` waits at most 3 s (the value the
   design leaves to Phase 5, section 5 "Lock and hibernate").
8. **Tooltip**: `p2-present` cuts whole groups to fit; `TrayIcon` cuts only as a last guard, at 127 UTF-16 units on a
   character boundary.
9. **Dry run**: its own icon GUID, so a dry-run tray can run next to the real one; `text::dry_run` marks its tooltip.
   Demos use a third GUID.
10. **High contrast** draws with system colours. No agent switches high contrast or the display scale (Haik's own
    settings; Phase 0 T11): forced-branch renders cover it, and Haik's optional steps in `p5-haik-check` (theme saved
    first) cover the real switch.

### 4. Rules for every laptop card of this track

- Part A (code, tests, no laptop) first. At its end set Status to `blocked: waiting for the screen slot` (or sound
  slot) and stop; the orchestrator resumes the card when no other screen or sound card runs, in any track.
- Start a demo at Medium through `explorer.exe <launch .cmd>` and check its token line, as the Phase 0 spikes did.
- Demos use the demo GUID. After the last run: export (`reg export`, to the scratchpad) and delete the demo's
  `HKCU\Control Panel\NotifyIconSettings\<id>` entry and its
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\NotifyIconGeneratedAumid_<id>` key; check both
  are gone, and still gone a minute later.
- Pictures show only the taskbar, the menu or the banner. Crop or grey anything personal; raw shots stay in the
  scratchpad. Use a plain backdrop window for menus, as `p1-tray-menu-look` did.
- Haik's settings a card may switch (Windows mode light or dark; Do not disturb): read them before, put them back
  exactly, read them after, and record both reads. Never switch high contrast, the display scale, the default audio
  device, the volume or the sound scheme. Never close or kill a program the card did not start (Settings, Notepad,
  Explorer); kill only its own demo, by PID.
- Each card builds into `CARGO_TARGET_DIR=<scratchpad>\<card name>\target`; never sets `RUSTFLAGS`.
- Checks before done: as section 7 says (own package and files only; no `--workspace` run).

### 5. Integration notes (for the wiring cards, planned later)

- Loop thread: `TrayIcon::new(kind, Box::new(OwnerDrawnMenu::new(sink)))` at tray startup step 8; `Command::Tray(view)`
  runs `icon.show(&view)`; `icon.on_balloon_click(..)` takes `notifier.take_click()` and opens it with
  `shell::open_with_default_app`.
- IO thread: `BalloonNotifier::new(icon.identity(), ..)` runs `Command::Notify` and gives `Outcome::Notified`.
- `Command::Play(cue)`: `player.play(cue)` on the loop thread (never blocks). The hibernate thread (Phase 3) holds an
  `Arc<dyn SoundPlayer>` and calls `play_and_wait(Cue::GoingToSleep)` after its last check.
- `Command::OpenLog`: `shell::open_with_default_app(<log path from store>)` off the loop thread.
- After wiring, check in the real exe what demos cannot show: the balloon header name (`FileDescription`), the exe
  icon in Explorer and Start, the product GUID's `NotifyIconSettings` entry, and that log lines read as plain English.

### 6. Waiting on Haik

- His one look at the menu pictures (`p1-tray-menu-look`): answered 2026-11-03 17:32, the pick stands and every
  menu item gets an icon (`p5-menu-paint` draws it, `p5-menu-view` shows it).
- Question in `p5-plan-tray-ux`: do the chimes honor the Windows "No sounds" scheme. Answered: yes, the default
  (`docs/phases/02-06-signoff.md` item 3, Haik 2026-11-03 18:09).
- Non-blocking looks, batched for the sign-off or `p5-haik-check`: the icon sheet (`p5-icon-art`), the text list
  (`p5-text`).
- `p5-haik-check`: about 25 minutes of his eyes, ears and hands.
- Questions 10 to 12 (`phases/01-design-skeleton.md`): answered with the defaults (18:09); no card of this track
  depends on them.

### 7. Checks while other cards run (every card)

From the cross-track review (B1); the rule is `docs/plan.md` "How we work" step 2. Every card of this track checks only
its own package and files:
- format: `rustfmt --edition 2024 --check <owned files>`. A bare `rustfmt` uses the 2015 edition (the repo has no
  `rustfmt.toml`) and also checks the child modules a file declares (both checked 2026-11-03, `docs/research.md` "Track
  plan fixes checks"), so a diff in another card's child file is not this card's. Format only owned files (`rustfmt
  --edition 2024 <file>`), never a file whose child modules another card owns.
- lint: `cargo clippy --locked -p <own package> --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets` (the libs as they ship, without `test-support`; `CLAUDE.md` "Checks"), for both targets: nothing in
  the owned files.
- test: `cargo test --locked -p <own package> <filter>`: the card's own tests pass. Several filters go after `--`
  (`cargo test --locked -p wakeshot-windows -- a:: b::`); two filters before `--` are a cargo error.
- Every cargo command takes `--locked`: it fails at once if a build would change `Cargo.lock`, which only
  `p6-velopack-dependency` changes, alone.
- A failure in a file another card owns goes in the Log: wait and run again; never edit, format or `--fix` that file.
  Never run `cargo fmt` without `--check`, `cargo fix` or `cargo clippy --fix`.
- Every save of an owned file compiles (`todo!()` bodies are fine): one compile error fails every `-p` run of that crate
  and of the crates that use it.
- No card runs `--workspace` or `--all` as a done check; the orchestrator runs the whole-workspace check when it closes
  a wave.
- Once `p6-velopack-dependency` is done, ARM64 builds of `wakeshot-tray` need clang (only the tray depends on
  `velopack`). Haik said yes to the Build Tools clang component (sign-off item 8, Haik 2026-11-03 18:09);
  `p6-velopack-dependency` step 0 has Haik install it, elevated. Without it the tray's ARM64 builds are CI only
  (review M5).
- Delete the card's own target folder when the card is done (review m14).

## Where other tracks meet this one

- Contracts change card (before any build): everything in `p5-plan-tray-ux` Result, "Contract changes".
- Phase 2: `p2-present` calls `text.rs`; until `p5-text` is done, the contracts card's stub wording stands in. This
  track draws hand-built `TrayView`s until `p2-present` is done. Proposal for the core, signed off (sign-off item 4,
  Haik 2026-11-03 18:09; `p2-core-lid` item 8a): notices raised while the session is locked or off the console wait
  until it is back (`p5-plan-tray-ux` Result).
- Phase 3: the hibernate thread uses `SoundPlayer::play_and_wait` (fake until `p5-sounds`); checklist 16 (chime at a
  real lid close) needs `p5-sounds` done.
- Phase 4: `shell.rs` (`p4-cli-commands`) opens the log and the Settings page; `p5-haik-check` runs after it.
- Phase 6: the installer and the Pages site use `assets/wakeshot.ico` (placeholder until `p5-icon-art`); uninstall
  may want the three icon GUIDs (`p5-tray-icon` Result).

## Task cards (`docs/tasks/`)

Laptop: "screen" or "sound" marks a Part B that runs one card at a time across all tracks; "settings" means Haik's
own Windows settings switched and put back.

| Card | Needs Haik | Laptop | Runs after | Status |
|---|---|---|---|---|
| `p5-theme` | no | no | contracts change card | in progress |
| `p5-icon-art` | one look at the icon sheet, not blocking | no | contracts change card | ready |
| `p5-text` | one read of the text list, not blocking | no | contracts change card | done (00:02): every word in `text.rs`, 552 core tests; the text list for Haik is in its Result |
| `p5-menu-paint` | no (menu look answered 17:32: an icon per item) | no | `p5-theme` | in progress |
| `p5-tray-icon` | no | screen, settings (light and dark mode) | `p5-theme`, `p5-icon-art` | ready |
| `p5-menu-view` | no (menu look answered 17:32) | screen, settings (light and dark mode) | `p5-menu-paint`, `p5-tray-icon` | ready |
| `p5-notices` | no | screen, settings (Do not disturb) | Part A: contracts change card; Part B: `p5-tray-icon`, `p5-text`, `p5-icon-art` | ready |
| `p5-sounds` | no (the "No sounds" question: honor it, Haik 2026-11-03 18:09) | sound | contracts change card | ready |
| `p5-haik-check` | yes, about 25 minutes: eyes, ears, hands | screen, sound, settings | every card above, `p4-cli-commands` | ready |

Sizes, one worker's context: `p5-theme` about 40K tokens, `p5-icon-art` 120K, `p5-text` 80K, `p5-menu-paint` 110K,
`p5-tray-icon` 140K, `p5-menu-view` 130K, `p5-notices` 110K, `p5-sounds` 120K, `p5-haik-check` 80K. Total about 930K.
Wave 1 of the other tracks peaked at 2 to 3 times such estimates (`docs/phases/02-06-lighter.md`).

Bundle (one worker, both cards in order; `docs/phases/02-06-lighter.md`): `p5-theme` + `p5-menu-paint`. So
`p5-tray-icon` starts when the bundle is done. No separate review for this track: the acceptance checks,
`p6-int-dry-run-e2e` and Haik's checks cover it.

## Order inside the track

1. Right after the contracts change card, in parallel, no laptop: `p5-theme`, `p5-icon-art`, `p5-text`, and Part A of
   `p5-sounds` and `p5-notices`.
2. When `p5-theme` is done: `p5-menu-paint`; when `p5-theme` and `p5-icon-art` are done: `p5-tray-icon` Part A.
3. Laptop slots, one at a time, in this order when they are free: `p5-tray-icon` Part B, `p5-sounds` Part B,
   `p5-menu-view` (Part A may run beside any card; Part B needs the slot), `p5-notices` Part B.
4. Last: `p5-haik-check`, when Haik is at the laptop.

## Coverage

| Item | Card |
|---|---|
| Icon states holding, idle, off; crisp; light, dark, high contrast; every scale (decision 34; product UX) | `p5-icon-art`, `p5-tray-icon`, `p5-haik-check` |
| Icon add, version 4, modify, re-add on `TaskbarCreated`, reload on `WM_DISPLAYCHANGE`, retry with backoff at sign-in (sections 4, 5, 8) | `p5-tray-icon` |
| Theme from `SystemUsesLightTheme`, compared after the broadcasts settle (section 5, OS events) | `p5-theme`, `p5-tray-icon` |
| Menu: `MenuView`, owner-drawn items plus `DWMWCP_ROUND`, follows the Windows mode, built fresh on each open, opens only from the icon (sections 2, 5; decisions 33, 34) | `p5-menu-paint`, `p5-menu-view` |
| Menu wording, tooltip and status wording (decisions 1, 33, 35; product menu sketch) | `p5-text` (structure: `p2-present`) |
| Notices as balloons, never counting balloon events, click (section 5) | `p5-notices` |
| Notice wording for every `Notice` (decisions 0, 20, 29, 36, 37, 38; section 8 table) | `p5-text` |
| First run pin offer, method (decision 36) | `p5-text`, `p5-notices`, `p5-haik-check` |
| Sounds: three chimes, device rule, own thread, never blocks (decision 19; section 2 `SoundPlayer`) | `p5-sounds` |
| How long the hibernate thread waits for its sound (section 5) | `p5-sounds` (`play_and_wait`) |
| Manual checklist 13: theme switch, high contrast, scales, sounds by ear (section 9) | `p5-haik-check` |
| Open log (section 8; product "the log stays on the PC") | Phase 4 `shell.rs`; integration wires `Command::OpenLog` |
| Checklist 16 (chime at a real lid close with the lock) | integration: `p6-int-haik-checks` block 2, after `p5-sounds` |
| Toast icon colours, toast `DisplayName` (section 5) | none in v1: balloons only; only if a notice ever needs buttons |
| Wiring into the tray exe; real-exe checks (balloon header, exe icon) | integration |

## Work log

- 2026-11-03 17:00: phase file and nine cards written by `p5-plan-tray-ux`. Waiting for the cross-track check, the
  contracts change card and Haik's sign-off of all five plans.
- 2026-11-03: Haik signed off with every default (18:09); `p1-close-contracts` wrote the answers into this file and
  the cards, and added `Notice::Updated { version }` for `p5-text` to word.
- 2026-11-03 23:36: `p5-text` running (core only, beside the app.rs contract change).
- 2026-11-04 00:02: `p5-text` done and committed. Its Result holds the text list for Haik (`p5-haik-check`). The fix round now words the LockFailed notice for the new step back (`text.rs` handed to it). Next tray cards: `p5-theme` + `p5-menu-paint`, `p5-icon-art`, `p5-sounds` (sound is fine at night, Haik 23:45).
- 2026-11-04 00:17: the `p5-theme` + `p5-menu-paint` bundle running. `p5-text`'s card table now quotes the new LockFailed words.
