# Build the tray icon: add, update, clicks, re-add, theme and display changes

Phase: 5. Status: blocked: waiting for the screen slot (session locked). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K
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

## Steps for Haik (Part B)

The screen steps need an unlocked session, and Windows locks it 5 minutes after the last input (display off).

1. When you are back at the laptop, unlock it.
2. Tell the orchestrator the screen card may run, then leave the mouse and keyboard alone for about 20 minutes. The
   worker first sets a display request, so the screen stays on and Windows does not lock. During the run Windows
   switches to Light mode for about a minute and then back to Dark; a demo icon shows in the tray.

## Found outside scope

Problems seen but not fixed.

- The orchestrator's file list for this round left out `crates/wakeshot-windows/examples/ux_icon.rs`; this card's
  "Owns" and the phase file's table give it to this card, so it was made here. No other card owns it.
- A display scale change on the taskbar's screen with no display change: if Windows sends the hidden window neither
  `WM_DISPLAYCHANGE` nor `TaskbarCreated`, the icon keeps the old size until the next trigger. Not tested;
  `p5-haik-check` covers scale changes (track note 3.10).
- An elevated tray: only `TaskbarCreated` is let through UIPI. Whether Explorer's callback messages reach an elevated
  window: not verified. The tray never runs elevated by design.
- For the orchestrator (Part B, 2026-11-04 02:18): the screen-slot rule "Haik away at least 15 minutes" cannot meet a
  usable screen on this laptop. On AC the display turns off 300 s after the last input and the session locks at once
  (`DelayLockInterval` 0); no agent can unlock it. Every screen card (`p5-menu-view` Part B, `p5-notices` Part B, and
  any sound card that needs the screen) hits the same wall. Default: the screen card starts a display request holder
  (Phase 0 `tray-screen-holder.ps1` pattern) while the session is still unlocked, and Haik steps away after that.
- Also for those cards: `GetLastInputInfo` most likely counts the worker's own synthetic input (`mouse_event`,
  `keybd_event`; expected from how Windows treats injected input, not verified here: no input was sent while locked).
  An idle check after the worker's own clicks must compare the last input time with the end of its own last
  injection, or use a low-level hook that skips input flagged as injected.

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
- 2026-11-04 02:11, Part B worker: started (screen slot given by the orchestrator). Idle 940 s at 02:11:17.
- 2026-11-04 02:14, Part B worker: **the session is locked**, so no picture, click or Settings step can run. Read at
  02:12:59 (`<scratchpad>\p5-tray-icon\lockstate.ps1`): `WTSSessionInfoEx` SessionFlags 0 (`WTS_SESSIONSTATE_LOCK`),
  `LogonUI` running, started 02:00:38. Last input 01:55:37 (02:11:17 minus 940 s), so it locked 301 s after the last
  input: the AC display timeout (`VIDEOIDLE` AC 0x12c = 300 s, read with `powercfg /q`) with `DelayLockInterval` 0
  (lock when the display turns off). No display request was held (`powercfg /requests`: only the orchestrator's two
  SYSTEM requests). On AC, 82%. Primary display is the DELL (2560x1440 logical, so 3840x2160 at 150%), the laptop screen
  1536x960 logical (1920x1200 at 125%). Theme read 02:14:26: `AppsUseLightTheme` 0, `SystemUsesLightTheme` 0. No demo
  `NotifyIconSettings` entry and no `NotifyIconGeneratedAumid_` key of ours before the run (the one such key,
  `11595054434325676090`, is Explorer's own). Plan: run the steps that need no screen now (build, Medium start, add,
  `TaskbarCreated`, slot rect, stop, leftovers), then block for the screen steps.
- 2026-11-04 02:15, Part B worker: `ux_icon` x64 release built (1 min 4 s), clean.
- 2026-11-04 02:16:39, Part B worker: idle 1253 s, session still locked. Started `ux_icon --state holding --minutes 20`
  with `explorer.exe <scratchpad>\p5-tray-icon\run1.cmd` (the `.cmd` writes `whoami /groups` "Mandatory Label" to the
  log, then runs the demo with stdout to `run1.log`). Token line: `Mandatory Label\Medium Mandatory Level ...
  S-1-16-8192`. Demo PID 19948 (parent cmd 31592), window `0x14e0306`, GUID `1e8c66d40fa34a5495a161bb89b171e9`.
  `02:16:39.216 icon: Added { attempt: 1 }`. Windows wrote `NotifyIconSettings\12880751163260563509` with only
  `IconGuid` (`{1E8C66D4-...-61BB89B171E9}`), `ExecutablePath` and `IconSnapshot`: no `UID`, no `IsPromoted` (so the
  first add is in the overflow, as in Phase 0). `Shell_NotifyIconGetRect` by GUID (per-monitor v2 caller): 3371,2088 to
  3419,2160, 48x72 physical. `Shell_TrayWnd`: DPI 144, rect 0,2088 to 3840,2160 (the DELL at 150%), so the demo draws
  24 px (`SM_CXSMICON` at 144). Not promoted: no picture can be taken while locked, and the entry is deleted at the end.
- 2026-11-04 02:17:00, Part B worker: `TaskbarCreated` (registered message `0xc07f`) posted to `0x14e0306` (window
  owner checked: PID 19948). Log: `02:17:00.139 icon: TaskbarCreated`, `02:17:00.153 icon: Added { attempt: 1 }`;
  the rect 2 s later was the same. Which add path ran (a plain `NIM_ADD`, or `NIM_ADD` failed on the GUID that was
  still there and the delete-and-add ran) is not in the log: not verified.
- 2026-11-04 02:17:12, Part B worker: `taskkill /PID 19948` (no `/F`): "SUCCESS: Sent termination signal". Log
  `02:17:13.152 WM_CLOSE: quitting`, `02:17:13.296 icon removed`, `exit 0`; PID 19948 and cmd 31592 gone 2 s later;
  `Shell_NotifyIconGetRect` then `0x80004005` (icon gone). No terminal process left from the launch.
- 2026-11-04 02:17:24, Part B worker: leftovers. Exported `NotifyIconSettings\12880751163260563509` to
  `<scratchpad>\p5-tray-icon\leftover-notifyiconsettings-run1.reg` and deleted it; `NotifyIconGeneratedAumid_12880751163260563509`
  was never made (no balloon). First read 02:17:24: both absent, no `ux_icon` process.
- 2026-11-04 02:18:29, Part B worker: second read, 65 s after the first: entry and `NotifyIconGeneratedAumid_` key
  absent, no `NotifyIconSettings` entry with the demo GUID or an `ux_icon` path, no `ux_icon` process. Theme read
  02:18:23 (never switched): `AppsUseLightTheme` 0, `SystemUsesLightTheme` 0, the same as 02:14:26. Still locked at
  02:18:24. Research section extended with these facts.
- 2026-11-04 02:18:48, Part B worker: build folder deleted, checked gone. Scripts kept for the next run in
  `<scratchpad>\p5-tray-icon\` (`lockstate.ps1`, `p5-lib.ps1`, `run1.cmd`). Status `blocked: waiting for the screen
  slot (session locked)`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and the three GUIDs.

**Part A done (2026-11-04 01:48). Part B partly done (2026-11-04 02:18), blocked: the session was locked** (Windows
locked it 5 minutes after Haik's last input, when the display turned off), so no picture, click or theme switch could
run. Haik's steps are in "Steps for Haik (Part B)". Next: a worker runs the rest of Part B in an unlocked screen slot.

Part B so far (Log 02:14 to 02:18; facts in `docs/research.md` "Tray UX: tray icon"):

| Step | Outcome |
| --- | --- |
| 4. Start at Medium | Passed. `explorer.exe <scratchpad>\p5-tray-icon\run1.cmd`; token line "Medium Mandatory Level" (S-1-16-8192); `Added { attempt: 1 }`. `IsPromoted` not written (nothing to see while locked; the entry was deleted at the end). |
| 5. Pictures, theme switch, `--pattern` | Not run (locked). No theme switch: reads 02:14:26 and 02:18:23 both `AppsUseLightTheme` 0, `SystemUsesLightTheme` 0. Slot 48x72 physical, taskbar DPI 144 (DELL, 150%), drawn 24 px. 125% is only the laptop screen, which has no tray: check at 150% and note it for Haik. |
| 6. Tooltip, clicks, keyboard | Not run (locked). `TaskbarCreated` passed by log: posted `0xc07f`, then `icon: TaskbarCreated` and `Added { attempt: 1 }` 14 ms later. Token line Medium (step 4). |
| 7. Leftovers, stop by PID | Passed for this run. `taskkill /PID 19948` (no `/F`): quit in 0.2 s, `icon removed`, exit 0. Entry `NotifyIconSettings\12880751163260563509` exported and deleted; no `NotifyIconGeneratedAumid_` key was made; both absent at 02:17:24 and 02:18:29; no `ux_icon` process. |
| 8. Research | Extended with the facts above; the screen facts come with the next run. |

No code change in Part B. Build folder deleted (checked gone 02:18:48). No other file changed.

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
- Part B checks: `TaskbarCreated` (log), Medium token, stop by PID and leftovers passed (table above); pictures, the
  theme switch and its reads, tooltip and clicks not run yet (session locked).

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
7. Part B: with the session locked, the steps that need no screen ran anyway (Medium start, add, `TaskbarCreated`,
   quit, leftovers), and the screen steps wait. Why: it checks the launch and the add path now, so a launch problem
   does not eat the next screen slot. Change it when a locked-session result turns out to differ from an unlocked
   one: then redo those steps in the next slot (it starts the demo again anyway).
8. Part B: the next slot sets a display request first (default under "Found outside scope"), so the session stays
   unlocked once Haik steps away. Change it when the orchestrator picks a different way to keep the screen usable.

Needs another card: none. The orchestrator: the lock and idle-check notes under "Found outside scope" apply to every
screen card (`p5-menu-view` Part B and `p5-notices` Part B, which use this demo icon next).

Notes for Part B: build `ux_icon` (x64 release), start it at Medium through `explorer.exe <launch .cmd>` with stdout to a
log. It prints its pid, window handle and GUID first, a `menu: anchor` line per click and an `icon: ...` line per
`Note`. If no click prints, the callback's high word is not 0 for a GUID icon: fix the id check in `decode`.
`TaskbarCreated` can be posted to the printed window handle (`RegisterWindowMessage("TaskbarCreated")`, then
`PostMessage`). It quits after `--minutes` or on `taskkill /PID` without `/F`, and removes its icon. Leftovers: the
Demo GUID's `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_` key (track note 4).

Notes for the rest of Part B (02:18):

- First check the lock: `pwsh -NoProfile -File <scratchpad>\p5-tray-icon\lockstate.ps1` must say `SessionFlags 1
  (unlocked)`. Then start a display request holder (Phase 0 `tray-screen-holder.ps1`, in the old scratchpad below)
  and stop it by its PID at the end; `powercfg /requests` must not list it after.
- Ready in `<scratchpad>\p5-tray-icon\`: `run1.cmd` (Medium start with the token line; build `ux_icon` into
  `<scratchpad>\p5-tray-icon\target` first), `p5-lib.ps1` (rect by GUID, taskbar DPI, demo entries, post
  `TaskbarCreated` after checking the window's PID). The entry id was `12880751163260563509` and will be new again.
- Phase 0 scripts to port (hover and right-click only inside the icon rect, MSAA on `#32768` menus, `Save-Rect`,
  Settings mode switch by UI Automation on `SystemSettings_Personalize_Color_ColorMode_ComboBox`):
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad\tray-lib.ps1`,
  `tray-t7open.ps1`, `tray-t7switch.ps1`, `tray-t7close.ps1`.
- The idle check after your own clicks: see "Found outside scope" (own input counts).
- The demo's `Added` note has no size; the size is `SM_CXSMICON` at `GetDpiForWindow(Shell_TrayWnd)` (24 at 144).
