# Show notices as tray balloons

Phase: 5. Status: ready. Needs Haik: no (the locked-session check is in `p5-haik-check`). Agent: `wakeshot-worker`.
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
  is raised and holding notices while the session is locked (core track; this track's proposal is in
  `p5-plan-tray-ux`). Opening the click target (Phase 4 `shell.rs`, integration). Registered toasts (only on the
  trigger above). The lock-screen check (`p5-haik-check`).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.
