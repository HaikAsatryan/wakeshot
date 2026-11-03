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
   a golden test; embedded by both exes and used by the installer. No 256 frame yet. `art::png` compresses since
   `p5-icon-art`, so a 256 frame needs no crate; its size in the exe is not measured. Change it when the installer or
   Explorer's large view looks blurry: add the frame in `art.rs` and the golden test.
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

- Loop thread: `TrayIcon::new(kind, Box::new(OwnerDrawnMenu::new(sink)?))?` (both return a `Result`) at tray startup
  step 8; `Command::Tray(view)` runs `icon.show(&view)`; `icon.on_balloon_click(..)` takes `notifier.take_click()` and
  opens it with `shell::open_with_default_app`.
- IO thread: `let n = Arc::new(BalloonNotifier::new(icon.identity()));` the IO thread gets `Box::new(Arc::clone(&n))`,
  runs `Command::Notify` and gives `Outcome::Notified`; the loop thread's `icon.on_balloon_click` calls `n.take_click()`.
  The first `Command::Tray` runs before the first `Command::Notify`, or that notice fails as `Outcome::Notified(Err)`.
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
| `p5-theme` | no | no | contracts change card | done (00:24): 7 tests; `Theme { light, contrast }`, `ThemeWatch` with `current()` |
| `p5-icon-art` | one look at the icon sheet, not blocking | no | contracts change card | done (committed 01:29): three glyph sets (Cup, Eye, Sky) with holding, idle and off; app icon; ICO 16 to 64; 12 tests |
| `p5-text` | one read of the text list, not blocking | no | contracts change card | done (00:02): every word in `text.rs`, 552 core tests; the text list for Haik is in its Result |
| `p5-menu-paint` | no (menu look answered 17:32: an icon per item) | no | `p5-theme` | done (00:50): 15 tests; colours and row sizes match the Phase 1 renders by pixel; nine renders in `docs/research/img/ux/` |
| `p5-tray-icon` | no | screen, settings (light and dark mode) | `p5-theme`, `p5-icon-art` | blocked: waiting for the screen slot (Part A done, 16 tests; Part B partly run 02:19: launch at Medium, TaskbarCreated, leftovers passed; the session was locked) |
| `p5-menu-view` | no (menu look answered 17:32) | screen, settings (light and dark mode) | `p5-menu-paint`, `p5-tray-icon` | blocked: waiting for the screen slot (Part A done, committed 02:27: 10 tests) |
| `p5-notices` | no | screen, settings (Do not disturb) | Part A: contracts change card; Part B: `p5-tray-icon`, `p5-text`, `p5-icon-art` | blocked: waiting for the screen slot (Part A done, committed 02:07: 10 tests) |
| `p5-sounds` | no (the "No sounds" question: honor it, Haik 2026-11-03 18:09) | sound | contracts change card | done (committed 02:41): Part B in a locked session: all nine chimes seen in loopback at full length; reads equal |
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
- 2026-11-04 00:51: the `p5-theme` + `p5-menu-paint` bundle done and committed (orchestrator session after the
  handover). Decided by the worker, for Haik's review: a missing `SystemUsesLightTheme` reads as light; icons on the
  five main rows only (eye, cup, restore arrow, page, cross), none on the Keep awake lengths, Keep screen on or the
  status line; the status line uses the secondary text colour; text capped at 480 epx (a long status ends in "...");
  marks and chevron use WinUI's secondary colour. For `p5-menu-view` Part B: check on screen that each row is as wide
  as measure plus `windows_adds()` (14, 16, 20 px at 96, 120, 144 dpi; only 120 was confirmed on screen). Stale line:
  `p5-menu-paint`'s "Out of scope" still lists icons in items, which Haik's 17:32 answer overrides (next docs pass).
- 2026-11-04 01:30: `p5-icon-art` done and committed. Sheets: `docs/research/img/ux/icon-sheet-dark.png`,
  `-light.png`, `-contrast.png`, `icon-app.png` (Haik's look, not blocking). Decided by the worker, for Haik's review:
  the pick is the Cup (steam while holding, empty when idle, struck through when off; Eye and Sky stay in `art.rs` as
  the other choices); the app icon is a white cup on an orange disc; whole-pixel strokes (2 px at 24 px, a little
  heavier than Windows' 1.5 px); padding size/16; the glyph on a light taskbar is 1a1a1a like Windows' own; `png`
  compresses (a stored sheet would be about 28 MB); no polygon primitive. Outside scope: track note 3.2 says a 256 px
  frame needs a crate, which no longer holds (`png` is a stdlib deflate now); size not measured (next docs pass or
  Haik's call). `p5-tray-icon` running; its Part B (screen, the light/dark switch) runs only after 15 minutes with no
  user input, else it stops after Part A.
- 2026-11-04 01:41: `p5-sounds` Part A running; the worker stops before Part B until no screen or sound card runs.
- 2026-11-04 01:50: `p5-tray-icon` Part A done and committed (16 tests, a fake shell; `examples/ux_icon.rs`). Part B
  did not run: input 174 s before 01:47, so Haik may be at the laptop. API: `TrayIcon::new(IconKind, Box<dyn MenuView>)`,
  `show`, `on_balloon_click`, `on_note` (log `AddFailed` and `ModifyFailed` through it in `p6-int-tray-mode`),
  `identity()`; GUIDs: product `7db9403e-d4b9-4165-a99e-a69b8ea56744`, dry run `c0d07dcd-dfd4-4a9d-b139-ad1e2ce9f987`,
  demo `1e8c66d4-0fa3-4a54-95a1-61bb89b171e9`. Decided by the worker, for Haik's review: no shell call while the menu
  is open (a new view, theme or size waits; a click does nothing); a failed add deletes our own GUID and adds once more
  before backing off (not verified as needed); a failed version 4 or later change adds the icon again; in high
  contrast the icon takes the window text colour (Windows' own taskbar colour there not verified); theme and display
  messages are checked at once and 1 s later; `WM_CLOSE` to the icon's window is ignored; `ux_icon --state`. Not
  tested: a display scale change with no display message may leave the old size (`p5-haik-check`). The screen lane
  runs one Part B at a time, each only after 15 minutes with no input: tray icon, then menu view, then notices; sounds
  Part B after them. `p5-notices` Part A running. The laptop is on AC again (01:49).
- 2026-11-04 02:01: `p5-sounds` Part A done and committed (chimes in code: lid closed rising 1046.5 to 1568 Hz at
  -6 dBFS, going to sleep the same notes falling at -9 dBFS, battery floor two 659 Hz pulses; `ux_sounds` example).
  Part B goes to a fresh worker (Part A used about 110K; notes for it in the card's Log; build folder kept). API:
  `WasapiPlayer::new(sink, log)` (`Send + Sync`); `play(cue)` returns at once and posts `Done(Sound(..))`;
  `play_and_wait(cue)` gives `Played`, `Skipped`, or error 1460 after 3 s; `p6-int-tray-mode` builds one player and
  shares it with the hibernate thread. Decided by the worker, for Haik's review: the player takes a log too; going to
  sleep is 3 dB softer; the battery floor chime has a soft second harmonic; the lid chime peak is exactly -6 dBFS;
  log lines name the device type, never its name; unusual mix formats fall back to 16-bit mono; an unreadable or
  changed sound scheme still plays; on a 3 s timeout the sound plays on and the hibernate cuts it; the device is the
  one Windows plays system sounds on. "No sounds" is read as the scheme `.None` (inferred from this PC's registry and
  third-party pages; no Microsoft page says so).
- 2026-11-04 02:07 (orchestrator, session `wakeshot-46`): `p5-notices` Part A done and committed (10 tests; whole
  package 513 passed); Part B waits for the screen slot, fresh worker, build folder kept in session `wakeshot-9d`'s
  scratchpad. Track note 5 now gives the `Arc` wiring. Decided by the worker, for Haik's review: warnings always show
  Windows' warning icon; Windows' quiet-time flag is not set (with it a new user's first hour drops notices); a notice
  that comes during another balloon still shows, later; a click on an older banner may open the newer notice's action
  (Part B's `--pair` run settles it). `p5-menu-view` Part A started 02:05.
- 2026-11-04 02:20 (orchestrator, session `wakeshot-46`): `p5-tray-icon` Part B ran in a locked session: the session
  locked at 02:00:38 (seen at 02:12:59), 301 s after Haik's last input at 01:55:37 (on AC the display turns off
  at 300 s, and `DelayLockInterval` 0 locks the session with it). No agent can unlock it. Passed without the screen: the demo starts at Medium and adds
  its icon, a posted `TaskbarCreated` re-adds it, `taskkill /PID` without `/F` removes it, both leftovers removed and
  checked. Not run: pictures, the light/dark switch (mode never switched; reads 02:14 and 02:18 both Dark), tooltip,
  clicks, keyboard. The taskbar is on the DELL screen at 150% (24 px icon), so `--pattern` is checked at 150%, not
  125%. So every screen Part B (`p5-tray-icon`, `p5-menu-view`, `p5-notices`) needs Haik: he unlocks, a worker starts a
  DISPLAY request so the screen stays on, then he leaves the mouse and keyboard alone for about 20 minutes. Worker
  note, not verified: `GetLastInputInfo` may count the worker's own synthetic input, so the idle check must leave it
  out. Helper scripts: session `wakeshot-46`'s scratchpad, `p5-tray-icon\`.
- 2026-11-04 02:27 (orchestrator, session `wakeshot-46`): `p5-menu-view` Part A done and committed (10 new tests, 25 in
  `tray::menu`); Part B waits for the screen slot (needs Haik, see 02:20). Decided by the worker, for Haik's review: the
  screen-reader text keeps the `&` marks (Windows drops them itself, seen in Phase 1; stripping them would lose a real
  `&` in a status line; inferred), so the card's `strip_access_keys` is not built; command ids from a table built per
  menu; the status line is greyed out (Enter does nothing, a screen reader says "unavailable"); in high contrast the
  menu frame is left to Windows. Track note 5 now has the `?` on both `new` calls.
- 2026-11-04 02:41 (orchestrator, session `wakeshot-46`): `p5-sounds` done and committed. Part B ran in the locked
  session: sound still plays and loopback still sees it. All nine chimes on the Speakers (Realtek, the only active
  endpoint), full tone length; stream start about 370 to 420 ms cold, about 100 ms warm; `--wait` returns 52 to 55 ms
  after the tone ends (under 3 s); the 20 reads before and after are equal; no code change. Decided by the worker, for
  Haik's review: the -6 dBFS peaks stay, though loopback shows the two -6 dBFS chimes at -0.2 and -0.1 dBFS (why is not
  verified; Haik's ear check in `p5-haik-check` says if they are too loud); no warm-up for the slow first stream; the
  screen-audio branch of the device rule was not run live (no DELL audio endpoint was active).
