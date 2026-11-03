# Build the tray icon: add, update, clicks, re-add, theme and display changes

Phase: 5. Status: blocked: waiting for the screen slot. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K
tokens. Split trigger (review m13): if Part A ends above about 100K, the worker stops there (Status `blocked: waiting for the screen slot`, as
usual) and the orchestrator runs Part B as its own card with a fresh worker, which reads this card's Log and Result.
Real laptop: **screen** in Part B (one screen or sound card at a time, across all tracks), and **settings**: the
Windows mode is switched to light and back through Settings. Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-tray-icon\target`.
Runs after: `p5-theme`, `p5-icon-art` (`art.rs` has no functions until `p5-icon-art` is done; verify review Minor
4). Parallel with (Part A): any card that does not own `tray/icon.rs`; Part B only with cards that use no
screen, sound or power settings.

## Goal

`TrayIcon`, the one object the tray loop holds for the notify icon: it adds the icon (and keeps trying until the
taskbar exists), shows the state and tooltip of each `TrayView`, opens the menu on a click, re-adds the icon after an
Explorer restart, and redraws it when the theme or the displays change, all on its own hidden window.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2 to 5, all of them).
- `docs/architecture.md` section 4 "Tray threads" and "Tray startup" step 8; section 5 "Tray, notices, sounds", OS
  events rows "Theme" and "Taskbar restart"; section 8 row "Tray icon add fails at sign-in".
- `docs/research.md`, "Tray, menu and notifications": "Add and update", "First add lands in the overflow", "Explorer
  applies `IsPromoted` live", "`NIF_GUID` and the exe path", "DPI", "Theme and contrast reads", "Explorer restart",
  "Live theme switch through Settings", "Tooltip", "Synthetic right-click", "`LoadIconMetric` follows the system DPI".
- `docs/tasks/p0-spike-tray.md` Result and Log (how agents started the spike at Medium, pinned it, switched the theme
  by UI Automation and put it back); `docs/tasks/p1-tray-menu-look.md` deliverable 6 (leftover removal).
- `spikes/tray/src/main.rs`: the window, `Shell_NotifyIconW` calls, `TaskbarCreated`, the click handling. Port by
  reading and rewriting.
- `crates/wakeshot-windows/src/tray.rs` (`Anchor`, `MenuView`, `IconIdentity`), `tray/theme.rs`, `tray/art.rs`;
  `crates/wakeshot-core/src/view.rs` (`TrayView`, `IconState`), `text.rs` (`dry_run`).

## Owns

`crates/wakeshot-windows/src/tray/icon.rs`; `crates/wakeshot-windows/examples/ux_icon.rs`; new files
`docs/research/img/ux/taskbar-*.png`.

## Deliverables

Part A (no laptop):

1. `TrayIcon` (loop thread only, not `Send`):
   - `new(kind: IconKind, menu: Box<dyn MenuView>) -> Result<Self, WinError>`: registers its window class and creates
     its own hidden top-level window (never shown), registers `TaskbarCreated` and allows it through
     `ChangeWindowMessageFilterEx` (in case the tray ever runs elevated), and reads the theme.
     `IconKind { Product, DryRun, Demo }` picks one of three fixed GUIDs made once in this card (record them).
   - `show(&mut self, view: &TrayView)`: the first call adds the icon (`NIM_ADD` with `NIF_GUID`, `NIF_MESSAGE`,
     `NIF_ICON`, `NIF_TIP`, `NIF_SHOWTIP`, then `NIM_SETVERSION` 4); later calls send `NIM_MODIFY` only when the state,
     the theme or the tooltip changed. It keeps `view.menu` for the next click. Dry run marks the tooltip with
     `text::dry_run`. Last-guard cut of the tooltip at 127 UTF-16 units on a character boundary.
   - A failed add retries on its own timer (`retry_delay(attempt)`: 1, 2, 4, 8, 16, then every 30 s) and on
     `TaskbarCreated`. `TaskbarCreated` re-adds, sets version 4 again and re-applies the last view.
   - Theme: on `WM_SETTINGCHANGE` "ImmersiveColorSet", `WM_SETTINGCHANGE` with `SPI_SETHIGHCONTRAST` and
     `WM_SYSCOLORCHANGE`, check `ThemeWatch` at once and again 1 s later (own timer); redraw only on a real change.
   - Size: draw the glyph at `GetSystemMetricsForDpi(SM_CXSMICON, dpi)` with `dpi` from the taskbar window
     (`Shell_TrayWnd`, `GetDpiForWindow`), else the system DPI; redraw on `WM_DISPLAYCHANGE` and on `TaskbarCreated`,
     never on `WM_DPICHANGED` (research). Part B checks the pick on screen.
   - Colour: white on a dark taskbar, the measured near black on a light one (`p5-icon-art` Result), and in high
     contrast the system colour Microsoft documents for taskbar text (cite the page; "not verified" if none says).
   - `HICON` from `art::colorize(art::tray_mask(..))` through `CreateDIBSection` and `CreateIconIndirect`; the old
     icon is destroyed after the `NIM_MODIFY` that replaced it; every GDI object freed on drop.
   - Clicks: left click (`NIN_SELECT`), keyboard select (`NIN_SELECT | NINF_KEY`; `windows` 0.62.2 has no
     `NIN_KEYSELECT` constant) and right click (`WM_CONTEXTMENU`) call `menu.show(&last_menu, anchor)` with the anchor
     from `wParam` (version 4). A click while the menu is open does nothing.
   - Balloon messages: `NIN_BALLOONUSERCLICK` calls the hook set by `on_balloon_click(Box<dyn FnMut()>)`;
     `NIN_BALLOONSHOW`, `NIN_BALLOONTIMEOUT` and `NIN_BALLOONHIDE` are ignored, never counted (research: the timeout
     came twice).
   - `identity(&self) -> IconIdentity` for `BalloonNotifier`. Drop: `NIM_DELETE`, then the window.
   - Re-entrancy: `MenuView::show` runs a modal loop that dispatches this window's messages and the loop's events
     (which can call `show` with a new view) before it returns. Make that safe by design (for example state behind a
     `RefCell`, with work that arrives during the menu queued until it closes) and test it.
2. Pure helpers with unit tests, written first: `decode(msg, wparam, lparam) -> Option<Input>`, `cut_tooltip`,
   `retry_delay`, `glyph_size(dpi)`, `glyph_colour(&Theme)`. Tests: `decode_version_4_messages` (select, key select,
   context menu with its anchor, balloon click, balloon timeout ignored, other icon id ignored),
   `tooltip_cut_keeps_whole_characters` (a surrogate pair at the limit), `short_tooltip_is_unchanged`,
   `retry_backs_off_then_holds_at_30_s`, `glyph_size_per_dpi` (96, 120, 144, 192), `glyph_colour_per_theme`,
   `hicon_from_art_reads_back` (headless: `GetIconInfo` and the bitmap size), `show_during_menu_is_queued` (a fake
   `MenuView` that calls back in). No test calls `Shell_NotifyIconW`: it would put icons on Haik's taskbar.
3. Example `ux_icon`: sets per-monitor v2 DPI at start (`SetProcessDpiAwarenessContext`), uses `IconKind::Demo`, a
   `MenuView` that prints the anchor, and flags: `--cycle` (holding, idle, off every 3 s, with tooltips), `--pattern`
   (a test glyph with a one-pixel border, to see scaling), `--contrast` (forced contrast colours), `--minutes N`.
   Prints each event with the wall time.

Part B (screen slot; set Status `blocked: waiting for the screen slot` after Part A and stop):

4. Start `ux_icon` at Medium (track note 4). The first add lands in the overflow: write `IsPromoted` = 1 on the demo's
   `NotifyIconSettings` entry (the Phase 0 method) so it shows on the taskbar.
5. Pictures (taskbar strip only): each state, dark taskbar; the same after switching to Light mode through Settings
   (read the `Personalize` values before; switch back to Dark; read after; both reads equal). The icon must change
   within about 1 s, without a restart. `--pattern` at 125%: is the drawn size shown 1:1, or scaled? If scaled,
   change the size pick and say why.
6. Tooltip shows; left click, right click and keyboard (Win+B, arrows to the icon, Enter; then Shift+F10) each reach
   the printing `MenuView` with an anchor on the icon. `TaskbarCreated` simulated by posting the registered message
   (no Explorer restart): the icon comes back. Token line Medium.
7. Leftovers removed and checked (track note 4). Demo stopped by its PID.
8. `docs/research.md`: a new section "Tray UX: tray icon" (GUIDs, size pick and the screen result, contrast colour
   source, clicks, theme switch timing).

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows tray::icon`: all pass; list the names in the Result.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-windows` for both targets: each clean in the
  owned files.
- Part B steps 5 and 6: each passes with a picture or a log line; the theme reads before and after are equal.
- Leftovers gone at two reads a minute apart; no demo process left.
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The menu itself (`p5-menu-view`; use a printing `MenuView`), balloons (`p5-notices`), sounds. The art
  (`p5-icon-art`), theme reads (`p5-theme`). Wiring into the tray exe (integration). A real Explorer restart, high
  contrast and other display scales (Haik, `p5-haik-check`).

## Questions for Haik

Each with the worker's default.

None. The calls this card made are listed in the Result as "decided by the worker, for Haik's review".

## Found outside scope

Problems seen but not fixed.

- The orchestrator's file list for this round left out `crates/wakeshot-windows/examples/ux_icon.rs`; this card's
  "Owns" and the phase file's table give it to this card, so it was made here. No other card owns it.
- A display scale change on the taskbar's screen with no display change: if Windows sends the hidden window neither
  `WM_DISPLAYCHANGE` nor `TaskbarCreated`, the icon keeps the old size until the next trigger. Not tested;
  `p5-haik-check` covers scale changes (track note 3.10).
- An elevated tray: only `TaskbarCreated` is let through UIPI. Whether Explorer's callback messages reach an elevated
  window: not verified. The tray never runs elevated by design.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-03 20:12, orchestrator, from `p2-present`'s "Found outside scope": the dry-run mark can push a full tooltip
  past Windows' 128-unit limit (`present.rs` already cuts to fit without it).
- 2026-11-04 01:30, worker: started Part A; reading the card's list.
- 2026-11-04 01:37, worker: read done. GUIDs made (PowerShell `[guid]::NewGuid()`): Product
  `7db9403e-d4b9-4165-a99e-a69b8ea56744`, DryRun `c0d07dcd-dfd4-4a9d-b139-ad1e2ce9f987`, Demo
  `1e8c66d4-0fa3-4a54-95a1-61bb89b171e9`. Contrast colour: Microsoft "Contrast themes" page (dated 2026-08-29) says
  bars use `SystemColorWindowColor` as background, paired with `SystemColorWindowTextColor`; no page names the
  taskbar's own colour. Writing `icon.rs`.
- 2026-11-04 01:45, worker: `icon.rs` and `examples/ux_icon.rs` written; 16 tests pass at the first run. rustfmt
  (edition 2024) reformatted only my two files. Clippy found a complex type and four `chunks_exact` uses; fixed
  (`NoteHook` alias, `as_chunks`). Then clippy clean on x64 and ARM64, with and without `--all-targets`; release
  builds of the lib on both targets and of `ux_icon` on x64 clean.
- 2026-11-04 01:47, worker: `GetLastInputInfo`: last input 174 s ago, so Haik may be at the laptop. Part B not run
  (orchestrator rule: at least 15 minutes with no input). Part A also ends near the 100K split trigger. Research
  section "Tray UX: tray icon" added with the Part A facts. Build folder deleted (checked gone).
- 2026-11-04 01:48, worker: Status `blocked: waiting for the screen slot`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and the three GUIDs.

**Part A done (2026-11-04 01:48). Part B not run**: Haik's last input was 174 s before the check, and Part A ended
near the split trigger. Next: a fresh worker runs Part B in a screen slot (steps 4 to 8), reading this Log and Result.

Files: `crates/wakeshot-windows/src/tray/icon.rs` (new body), `crates/wakeshot-windows/examples/ux_icon.rs` (new),
`docs/research.md` (new section "Tray UX: tray icon", Part A facts only), this card.

Public API (`wakeshot_windows::tray::icon`):

- `TrayIcon::new(kind: IconKind, menu: Box<dyn MenuView>) -> Result<TrayIcon, WinError>`: class `Wakeshot.TrayIcon`,
  hidden top-level `WS_EX_TOOLWINDOW` window, `TaskbarCreated` registered and allowed through
  `ChangeWindowMessageFilterEx`, theme read (a failed read draws for dark), size read. Not `Send` (holds an `Rc`).
- `show(&mut self, &TrayView)`: the first call adds (`NIM_ADD` with `NIF_GUID | NIF_MESSAGE | NIF_ICON | NIF_TIP |
  NIF_SHOWTIP`, then `NIM_SETVERSION` 4); later calls send `NIM_MODIFY` only when the state, the glyph colour, the
  size or the tooltip changed (a new `HICON` only when the picture changed). Keeps `view.menu` for the next click.
  Dry run marks the tooltip with `text::dry_run`; a last-guard cut at 127 UTF-16 units on a character boundary.
- `on_balloon_click(&mut self, Box<dyn FnMut()>)`; `on_note(&mut self, NoteHook)` with
  `NoteHook = Box<dyn FnMut(&Note)>`; `identity(&self) -> IconIdentity` (window handle and GUID).
- Demo only: `force_theme(&mut self, Option<Theme>)`, `show_pattern(&mut self, bool)`.
- `IconKind { Product, DryRun, Demo }` with `guid() -> u128`. `Note { Added { attempt }, AddFailed { attempt,
  retry_in }, Redrawn { icon, size, colour }, ModifyFailed, TaskbarCreated, ThemeChanged(Theme), SizeChanged(u32) }`.
  `Input` and `OpenBy` (what the callback decodes to).
- Drop: `NIM_DELETE` (when any add was sent), then the window; each `HICON` and bitmap is freed by its owner type,
  the old icon right after the call that replaced it.

GUIDs: Product `7db9403e-d4b9-4165-a99e-a69b8ea56744`, DryRun `c0d07dcd-dfd4-4a9d-b139-ad1e2ce9f987`, Demo
`1e8c66d4-0fa3-4a54-95a1-61bb89b171e9`.

For `p5-menu-view`, `p5-notices` and `p6-int-tray-mode`: `TrayIcon::new(IconKind::Demo, Box::new(menu))`, then
`show(&view)`; `BalloonNotifier::new(icon.identity())` sets `hWnd` and `guidItem` (with `NIF_GUID`) from the
identity. Integration logs the `Note`s it wants (`AddFailed`, `ModifyFailed`) through `on_note`. A `MenuView` may pump
messages: `show` calls that arrive while the menu is open are kept and drawn when it closes, and a click while it is
open does nothing.

Checks (build folder in the scratchpad, deleted at the end and checked gone):

- `cargo test --locked -p wakeshot-windows tray::icon`: 16 passed: `decode_version_4_messages`,
  `tooltip_cut_keeps_whole_characters`, `short_tooltip_is_unchanged`, `retry_backs_off_then_holds_at_30_s`,
  `glyph_size_per_dpi`, `glyph_colour_per_theme`, `hicon_from_art_reads_back`, `show_during_menu_is_queued`,
  `first_show_adds_then_modifies_only_on_change`, `forced_theme_redraws_only_on_a_colour_change`,
  `failed_add_retries_then_taskbar_created_re_adds`, `dry_run_marks_the_tooltip`,
  `clicks_open_the_menu_and_balloon_click_calls_the_hook`, `pattern_has_a_border_and_a_checker`,
  `and_mask_marks_clear_pixels`, `theme_settings_are_recognised`. No test calls `Shell_NotifyIconW` (fake shell).
- `rustfmt --edition 2024 --check` on `icon.rs` and `ux_icon.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --target <t> -- -D warnings`, with and without `--all-targets`, x64 and
  ARM64: clean, with no hits in other cards' files either.
- `cargo build --release --locked -p wakeshot-windows`, x64 and ARM64: clean; `--example ux_icon` x64 release: clean.
- Part B checks (pictures, theme reads, clicks, `TaskbarCreated`, leftovers): not run yet.

Decided by the worker, for Haik's review (technical; each with a "change it when"):

1. While the menu is open no shell call is made; a new view, theme or size waits and is drawn when the menu closes.
   Why: one shell call at a time, and no borrow is held while the modal loop runs. Change it when Part B or Haik finds
   the icon changes too late after the menu closes.
2. A failed `NIM_ADD` deletes the GUID and adds once more before the backoff. Why: a run that died without
   `NIM_DELETE`, or an add that timed out but landed, may block the next add of our own GUID (not verified). Change it
   when a field log shows that delete removing an icon it should not.
3. A failed `NIM_SETVERSION` or `NIM_MODIFY` counts as "not added", so the icon is added again. Why: clicks decode only
   in version 4, and a failed modify most likely means the icon is gone. Change it when Part B shows a modify failing
   while the icon is still there.
4. The high contrast glyph colour is `COLOR_WINDOWTEXT` (Microsoft's pairing for bars; the taskbar's own colour is not
   verified). Change it when `p5-haik-check` shows the glyph hard to see in a contrast theme.
5. Theme and display triggers read at once and again 1 s later, and the size is read on the same triggers. `WM_CLOSE`
   on the icon window is ignored: the loop window owns quitting.
6. `ux_icon` has `--state holding|idle|off` (one fixed state for pictures) beside the card's flags.

Needs another card: none.

Notes for Part B: build `ux_icon` (x64 release), start it at Medium through `explorer.exe <launch .cmd>` with stdout to a
log. It prints its pid, window handle and GUID first, a `menu: anchor` line per click and an `icon: ...` line per
`Note`. If no click prints, the callback's high word is not 0 for a GUID icon: fix the id check in `decode`.
`TaskbarCreated` can be posted to the printed window handle (`RegisterWindowMessage("TaskbarCreated")`, then
`PostMessage`). It quits after `--minutes` or on `taskkill /PID` without `/F`, and removes its icon. Leftovers: the
Demo GUID's `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_` key (track note 4).
