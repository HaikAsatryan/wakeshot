# Show notices as tray balloons

Phase: 5. Status: done (Part A 2026-11-04 02:03; Part B screen run 11:51 to 12:04). Needs Haik: no (the locked-session check is in `p5-haik-check`). Agent: `wakeshot-worker`.
Size: about 110K tokens.
Real laptop: **screen** in Part B (one screen or sound card at a time, across all tracks), and **settings**: Do not
disturb is switched on and back. Build folder: `CARGO_TARGET_DIR=<scratchpad>\p5-notices\target`.
Runs after: Part A: the contracts change card (`IconIdentity`, the `text.rs` stub). Part B: `p5-tray-icon` (the demo
icon), `p5-text` (real wording in the pictures), `p5-icon-art` (the large icon). Parallel with (Part A): any card that
does not own `notify.rs`; Part B only with cards that use no screen, sound or power settings.

## Goal

`BalloonNotifier`, the product's `Notifier`: it shows each `Notice` as a balloon on Wakeshot's own icon (Windows 11
shows it as a banner, with no app registration), with the words, warning look, sound and click action from
`text::notice`, and finds out on screen what happens to a balloon when it is clicked, when two come close together,
and under Do not disturb.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2 to 5).
- `docs/architecture.md` section 5 "Tray, notices, sounds" (balloons; never count balloon events; a successful call
  never proves the user saw it; when to change to registered toasts); section 8 (which failures notify).
- `docs/research.md`, "Tray, menu and notifications": "Balloon", "Balloon on Windows 11", "Notification centre",
  "Toast" (only for the comparison), "Windows writes `Notifications\Settings\<aumid>` only for the registered app id".
- `docs/tasks/p1-lid-sound.md` Result item 4 (toasts are hidden on the lock screen by default; not tested).
- `spikes/tray/src/main.rs` (the `NIF_INFO` path). Port by reading and rewriting.
- `crates/wakeshot-windows/src/notify.rs` (`Notifier` and its doc), `tray.rs` (`IconIdentity`), `tray/art.rs`;
  `crates/wakeshot-core/src/text.rs` (`NoticeText`, `NoticeClick`), `view.rs` (`Notice`).

## Owns

`crates/wakeshot-windows/src/notify.rs` (with `FakeNotifier` if the contracts card moved it there; its public API
stays as the contracts card left it); `crates/wakeshot-windows/examples/ux_notices.rs`; new files
`docs/research/img/ux/notice-*.png`.

## Deliverables

`FakeNotifier` already lives in `notify.rs` (the contracts card put it there, re-exported from `testing.rs`); its API
and the `Notifier` trait are contract: add, never change (`p1-contracts-split` edit 7).

Part A (no laptop):

1. `BalloonNotifier` (`Send`; it runs on the IO thread):
   - `new(icon: IconIdentity) -> Self`.
   - `Notifier::show(&self, notice)`: `text::notice(notice)` into a `NOTIFYICONDATAW` with `NIF_INFO | NIF_GUID`,
     `szInfoTitle` (at most 63 UTF-16 units) and `szInfo` (at most 255), `dwInfoFlags` from the text: the large app
     icon (`NIIF_USER | NIIF_LARGE_ICON` with `hBalloonIcon` drawn from `art::app_icon` at the large icon size) or the
     info or warning icon, as Part B decides; `NIIF_NOSOUND` when `silent`. Then `Shell_NotifyIconW(NIM_MODIFY)`;
     FALSE gives `WinError { call: "Shell_NotifyIconW", code }`. Free the icon after the call if the shell copies it
     (check in Part B; else keep it until the next notice).
   - `take_click(&self) -> NoticeClick`: the click action of the last balloon shown, then `Nothing`, so a late click
     on an old banner cannot open the wrong thing. `TrayIcon`'s balloon click hook calls it; integration opens the
     result with `shell::open_with_default_app` (Phase 4).
2. Pure helpers with unit tests, written first: `fill(&NoticeText, IconIdentity) -> NOTIFYICONDATAW` and
   `info_flags(&NoticeText, IconChoice)`. Tests: `fills_title_and_body_within_limits` (long text cut on a character
   boundary), `warning_and_silent_set_the_flags`, `uses_the_guid_identity`, `click_is_kept_for_the_last_notice_only`,
   `take_click_clears_it`, `notifier_is_send` (compile-time). No test calls `Shell_NotifyIconW`.
3. Example `ux_notices`: `TrayIcon` (demo kind) plus `BalloonNotifier`; flags `--all` (every `Notice` variant with
   sample data, 10 s apart), `--one <variant>`, `--pair` (two notices 1 s apart), `--icon user|info` (the large icon
   choice), `--cycle` (the icon changes state while balloons show, to check the two threads do not clash). The balloon
   click hook prints the action from `take_click`; it opens nothing.

Part B (screen slot; set Status `blocked: waiting for the screen slot` after Part A and stop):

4. Start `ux_notices` at Medium (track note 4); pin the demo icon as in `p5-tray-icon`.
5. `--all`: a picture of each banner (crop to the banner). Check every text fits, the warning ones read as warnings,
   and the silent one makes no sound (no loopback needed: `NIIF_NOSOUND` set is enough, plus your note).
6. `--icon user` against `--icon info`: which reads better on the banner, light and dark (switch the Windows mode as
   in `p5-tray-icon`, read before, put back, read after). Pick, with the reason.
7. Click a banner (synthetic click, as the spike's synthetic right-click): `NIN_BALLOONUSERCLICK` arrives (never tested
   before) and the hook prints the right action. Click the first-run banner too.
8. `--pair`: both banners show, or the second replaces the first? Record.
9. Do not disturb: read its state, turn it on (Settings by UI Automation, or the notification centre), show one
   notice, open the notification centre (Win+N) for a picture, turn it back, read again. Is the balloon shown later,
   kept in the centre, or lost? If lost, write the trigger in the Result: move to registered toasts (stored in the
   centre), which needs Phase 6's uninstall to clear them (architecture section 5).
10. Leftovers removed and checked (track note 4). Demo stopped by its PID.
11. `docs/research.md`: a new section "Tray UX: notices" (icon choice, click, pair, Do not disturb, icon lifetime).

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows notify`: all pass; list the names in the Result.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-windows` for both targets: each clean in the
  owned files.
- Part B steps 5 to 9: each passes, or its finding is recorded with a picture or a log line; the settings reads before
  and after are equal.
- Leftovers gone at two reads a minute apart; no demo process left.
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The wording (`p5-text`; a text that reads badly on the banner goes to its owner or into the Result). When a notice
  is raised and holding notices while the session is locked (core track, `p2-core-lid` item 8a; this track's
  proposal, signed off 2026-11-03 18:09). Opening the click target (Phase 4 `shell.rs`, integration). Registered
  toasts (only on the trigger above). The lock-screen check (`p5-haik-check`).

## Questions for Haik

Each with the worker's default.

None blocking. Part A's open calls were decided by the worker (Result, "Decisions for Haik's review").

## Found outside scope

Problems seen but not fixed.

0. Part B (12:03): Haik's notification centre still holds two "Wakeshot tray spike" toasts from Tuesday ("Toast with a
   registered app id", Phase 0 spike, registered AUMID). Left alone (not this card's; Haik's centre). Whoever owns the
   Phase 0 leftovers can clear them, or Haik with "Clear all".
1. `docs/phases/05-tray-ux.md` track note 5 says "`BalloonNotifier::new(icon.identity(), ..)` runs `Command::Notify`".
   As built, `new` takes only the identity, and the notifier must be shared by two threads: `Arc::new(...)`, one
   clone boxed as the IO thread's `Box<dyn Notifier>`, one in the balloon click hook (Result, "API"). For the
   orchestrator (phase file; workers never edit it) and `p6-int-tray-mode`.
2. `taskbar_dpi` is now in both `tray/icon.rs` (private there, `p5-tray-icon`) and `notify.rs`: eight lines, the same
   rule. One `pub(crate)` copy in `icon.rs` would serve both; that is an `icon.rs` edit, so it waits for that file's
   owner.

## Needs another card

None from Part A. Wording that reads badly on the banner can only show in Part B.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 01:50: Part A started (worker). Part B waits for the screen slot.
- 2026-11-04 01:58: `notify.rs` written: `BalloonNotifier` (`Send + Sync`, `Arc` is a `Notifier` too), `IconChoice`,
  `fill`, `info_flags`, the app icon through `art::ico` and `CreateIconFromResourceEx`. 10 tests, 9 passed at the
  first run; the one failure was a wrong count in the test itself (6 notices, not 7), fixed.
- 2026-11-04 02:03: `examples/ux_notices.rs` written; clippy `--all-targets` x64 clean; rustfmt applied to both owned
  files (one assert joined). Full check run started (both targets, tests, release builds).
- 2026-11-04 02:03: every Part A check passed (Result). `docs/research.md` section "Tray UX: notices" added (Part A
  facts: the two Microsoft pages, `CreateIconFromResourceEx` alpha). Part A done; Status set to blocked, waiting for
  the screen slot. Build folder kept for Part B.
- 2026-11-04 11:51, Part B worker (session `wakeshot-12`): started the screen run (unlocked, orchestrator's DISPLAY
  request; debug build of the f47391d export; grey backdrop window behind the banner area). `--all --icon user` at
  Medium (token line Medium, PID 42456, `icon App`), icon promoted; each banner grabbed 2.5 s after its `notice <name>:
  Ok(())` line (12 of 14; first-run and user-changed came before the watcher and were grabbed in later runs).
  Four bodies cut by Windows at five lines (Result). Stopped by PID, exit 0.
- 2026-11-04 11:56, Part B worker: banner clicks (synthetic left click on the body): restore-failed ->
  `balloon click: OpenLog` (PID 40652); first-run -> `balloon click: OpenTaskbarSettings` (PID 37004). Each demo
  stopped by PID.
- 2026-11-04 11:57 to 12:01, Part B worker: icon compare. Banners from a demo that had quit still showed later (no
  input, so no timeout), so the first light grabs showed stale banners; redone with stale banners closed by their X
  and one `--all` run per icon (a new balloon of the same icon replaces the old). Valid pairs: dark closed-lid-report
  app (`--all`) and info (PID 30080); light first-run info (PID 41264) and app (PID 27380). Mode reads before 11:57:13
  and after 12:00:59 equal (`AppsUseLightTheme=0 ColorPrevalence=0 EnableTransparency=1 SystemUsesLightTheme=0`).
- 2026-11-04 12:01, Part B worker: `--pair` (PID 22952): first-run at 12:01:43.366, restore-failed 1.0 s later. Grabs
  every 0.7 s for 8 s: only the first-run banner. Click on it 12:01:53: no balloon click logged, the banner closed;
  the restore-failed banner then showed; click 12:01:57 -> `balloon click: OpenLog`.
- 2026-11-04 12:02, Part B worker: Do not disturb read Off (Settings toggle
  `SystemSettings_Notifications_QuietHours_MuteNotification_Enabled_ToggleSwitch`), turned On 12:02:54; restore-failed
  sent `Ok(())` 12:02:58 (PID 22348): no banner; Win+N centre: "Do not disturb is on", no entry for the demo; DND Off
  12:03:07; 8 grabs over 8 s with the demo running: no banner. DND read Off at 12:03:19. Demo stopped by PID.
- 2026-11-04 12:04, Part B worker: Settings window (opened by my script) closed; backdrop (own pwsh 10452) stopped by
  PID; entry `12880751163260563509` and `NotifyIconGeneratedAumid_12880751163260563509` (made by the balloons:
  `PeriodicNotificationCount` 30) exported to `<scratchpad>\p5-screen\` and deleted. Reads 12:04:02, 12:04:58,
  12:05:36: icon ids equal to before, only the two old `NotifyIconGeneratedAumid_` keys, no demo process.
- 2026-11-04 12:06, Part B worker: build folder `<scratchpad>\p5-screen\target` deleted (checked gone); no demo
  process. Scripts and raw grabs stay in `<scratchpad>\p5-screen\`. Status `done`.

## Result

**Done (2026-11-04 12:08).** Part A 02:03; Part B in the screen slot of 11:51 to 12:04 (session `wakeshot-12`). No
code change. Facts in `docs/research.md` "Tray UX: notices" (Part B block). Pictures `docs/research/img/ux/notice-*.png`
(19).

| Step | Outcome |
| --- | --- |
| 4. Medium start, promote | Passed (token line Medium; `IsPromoted` = 1). |
| 5. `--all` pictures | Done, with a finding: 4 of 14 bodies are cut by Windows after five lines: `restored-by-user`, `lock-failed`, `update-wakeshot`, `journal-lost` (`notice-dark-<name>.png`). Warnings show the warning triangle. Silent ones sent with `NIIF_NOSOUND` (flag line), no sound check. |
| 6. App icon or info icon | Pick: the app icon (`IconChoice::PRODUCT = App` stays). Both read clearly in light and dark; the app icon tells who speaks, the info "i" adds nothing. Change it when Haik finds the violet square too loud next to Windows' own banners. Mode reads before and after equal. |
| 7. Banner click | Passed. `NIN_BALLOONUSERCLICK` arrives: `OpenLog` for restore-failed, `OpenTaskbarSettings` for first-run. |
| 8. `--pair` | The second does not replace the first: it waits until the first is gone (no input, so the first stayed). A click on the first gave no click message at all (its action is lost); the second's click gave its own action. No wrong action seen. |
| 9. Do not disturb | Lost: no banner, no centre entry, nothing after DND off. Trigger to move to registered toasts (stored in the centre; Phase 6 uninstall must clear them): if Haik wants notices kept through Do not disturb. DND read Off before and after. |
| 10. Leftovers, stop by PID | Passed (Log 12:04). |
| 11. Research | Done. |

For Haik / the next text pass (not fixed here: `text.rs` is not this card's file): shorten the four cut notices to
fit five lines at this width (about 45 characters a line), or move the rest of the words to the log. Banners wait
for real input before they time out, so a banner of a run that has ended can still show later.

Acceptance: `cargo` checks not rerun (no code change since Part A). Files changed in Part B: this card,
`docs/research.md`, 19 new `docs/research/img/ux/notice-*.png`.

Part A record:

### What was done

- `crates/wakeshot-windows/src/notify.rs`: `BalloonNotifier` and `IconChoice` added; the `Notifier` trait and
  `FakeNotifier` unchanged. `show` turns `text::notice` into a `NOTIFYICONDATAW` with `NIF_INFO | NIF_GUID` only (the
  icon, tooltip and callback stay as `TrayIcon` set them), title and body cut to 63 and 255 UTF-16 units on a
  character boundary, then `Shell_NotifyIconW(NIM_MODIFY)`; FALSE gives `WinError { call: "Shell_NotifyIconW", code }`
  (last error cleared first, so 0 means Windows gave no code). The large app icon is `art::app_icon` at `SM_CXICON`
  for the primary taskbar's DPI, made with `CreateIconFromResourceEx` from one frame of `art::ico` (so it reuses the
  tested ICO writer instead of copying `icon.rs`'s private `make_icon`).
- `crates/wakeshot-windows/examples/ux_notices.rs`: demo icon on the main thread, notices from a second thread as the
  IO thread does. Flags `--all` (14 notices, 10 s apart, first after 2 s), `--one <name>`, `--list`, `--pair`
  (first-run then restore-failed, 1 s apart: two different click actions), `--icon user|info`, `--cycle` (icon state
  every 3 s), `--minutes N` (default 10). Prints each notice's warning, silent and click, each `show` result, every
  icon note, and `balloon click: <take_click>`; opens nothing. Quits on time or `WM_CLOSE`, removing the icon.
- `docs/research.md`: new section "Tray UX: notices" (Part A facts only).

### API (for `p6-int-tray-mode`)

```rust
pub enum IconChoice { App, Info }            // IconChoice::PRODUCT = App until Part B decides
pub struct BalloonNotifier;                   // Send + Sync
impl BalloonNotifier {
    pub fn new(icon: IconIdentity) -> Self;
    pub fn with_choice(icon: IconIdentity, choice: IconChoice) -> Self;   // demos
    pub fn take_click(&self) -> NoticeClick;
}
impl Notifier for BalloonNotifier {}
impl Notifier for Arc<BalloonNotifier> {}
```

Wiring: `let n = Arc::new(BalloonNotifier::new(icon.identity()));` the IO thread gets `Box::new(Arc::clone(&n))` as
its `Box<dyn Notifier>`; `icon.on_balloon_click(Box::new(move || open(n.take_click())))` on the loop thread. The icon
must be added (the first `Command::Tray`) before the first `Command::Notify`, else that notice fails with
`Shell_NotifyIconW` and comes back as `Outcome::Notified(Err)`; the notifier does not retry. While Explorer is
restarting, a notice fails the same way.

### Checks (2026-11-04 01:59 to 02:03, build folder `<scratchpad>\p5-notices\target`)

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check` on `notify.rs`, `ux_notices.rs` | clean (after one `rustfmt` of both owned files) |
| `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings`, x64 and ARM64 | clean |
| the same without `--all-targets`, x64 and ARM64 | clean |
| `cargo test --locked -p wakeshot-windows notify` | 10 passed |
| `cargo test --locked -p wakeshot-windows` | 513 passed, 21 ignored, 0 failed |
| `cargo build --release --locked -p wakeshot-windows --target <t>`, x64 and ARM64, lib and `--example ux_notices` | built |

Tests (all in `notify::tests`): `fills_title_and_body_within_limits`, `warning_and_silent_set_the_flags`,
`uses_the_guid_identity`, `click_is_kept_for_the_last_notice_only`, `take_click_clears_it`, `notifier_is_send`
(compile-time `Send + Sync`, and both `BalloonNotifier` and `Arc<BalloonNotifier>` are a `Notifier`),
`show_sends_the_words_flags_and_app_icon`, `first_frame_of_one_frame_ico`, `large_icon_reads_back`,
`large_icon_size_is_the_large_metric`. No test calls `Shell_NotifyIconW` (a fake shell behind a private seam). No
other card's file showed a fmt or clippy hit. Only the owned files, this card and the new `research.md` section
changed.

### Design calls (worker as designer; each with its trigger)

1. A warning always shows Windows' warning icon (`NIIF_WARNING`); `IconChoice` picks only the icon of the other
   notices (app icon or `NIIF_INFO`). Why: the goal asks for a warning look in either case. Change it when Part B
   shows the warning icon does not show on the banner.
2. `NIIF_LARGE_ICON` on every notice, stock icons included (the page allows it). Change it when Part B shows the
   stock icon looks wrong large.
3. The large app icon is kept for the notifier's life and redrawn only when the size changes (the old one freed after
   the next call). So the card's "free it after the call if the shell copies it" check is not needed for safety:
   one handle costs nothing, and freeing early could blank a queued balloon. Change it when a leak check shows more
   than one icon handle per notifier.
4. Size `SM_CXICON` at the primary taskbar's DPI, the size `NIIF_LARGE_ICON` names. Change it when Part B shows a blurry icon on the
   banner.
5. If the app icon cannot be made (no handles left), the notice shows with the info icon instead of failing.
6. The click action is set only after Windows took the notice; a refused notice leaves the earlier click in place.
7. `Arc<BalloonNotifier>` implements `Notifier` (an addition to the contract module, nothing changed): the one way to
   serve both threads with the IO thread's `Box<dyn Notifier>` as it is.
8. The large-icon lock is held through the shell call; `take_click` uses a separate lock, so a slow Explorer never
   holds up the loop thread.

### Decisions for Haik's review (decided by the worker)

1. **No quiet time flag** (`NIIF_RESPECT_QUIET_TIME`). With it, Windows silently drops a notice in a new user's first
   hour (or after an OS upgrade). Every Wakeshot notice tells of something it did or could not do (decision 0), most
   in answer to the user's own act, which Microsoft exempts. Change it when a notice is pure news (only `Updated` is
   close), or if Haik wants `Updated` held back then.
2. **No "real time only" flag** (`NIF_REALTIME`): a notice queued behind another balloon still shows, later. A late
   notice beats a lost one. Change it when Part B shows a queued notice shows words that are no longer true.
3. **Warnings keep the warning icon** even if Part B picks the app icon for the rest (design call 1).
4. **A click on an older banner** may give the newer notice's action: Windows' click message does not say which
   balloon. If Part B `--pair` shows two banners at once, the safe fix is to drop the click action when two notices
   with different actions come close together. Part B decides with the picture.

### Open for Part B (steps 4 to 11)

- Part B steps unchanged, except the "does the shell copy the icon" check (design call 3: not needed).
- To record: does `NIM_MODIFY` from the second thread work (never tried before: the spike called it from the
  window's thread); does `--cycle` show any clash; the large icon's sharpness at 150% (design call 4); the high word
  of the callback `lParam` (from `p5-tray-icon`, if not settled there).
- Then IconChoice: set `IconChoice::PRODUCT` to the pick and record why.

Context used by Part A: about 110K tokens.
