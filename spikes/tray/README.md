# Tray spike

Phase 0 throwaway code for card `docs/tasks/p0-spike-tray.md`. It answers: can a Rust tray app look native in light
and dark mode, show notifications and play a sound, and how big is it. Findings live in `docs/research.md`, section
"Tray, menu and notifications".

Not product code. Icon shapes are placeholders (icon design is Phase 5).

## What it does

- Tray icon through `Shell_NotifyIconW` (version 4) from a hidden top-level window, plus a message-only window that
  only logs, to see which one gets `TaskbarCreated`.
- Icons from resources through `LoadIconMetric`, a glyph for dark taskbars and one for light taskbars. `build.rs`
  draws them (16 to 64 px) and a short WAV, then embeds them with a manifest (Common Controls 6, per-monitor v2 DPI)
  and version info through `embed-resource`.
- Popup menu like the `docs/product.md` sketch, with a disabled status line, a check mark, a radio submenu and test
  items. Dark menus through the undocumented uxtheme ordinals 135 and 136.
- Notifications: a `Shell_NotifyIcon` balloon, and WinRT toasts with a registered and an unregistered app id.
- Sounds: the user's "SystemNotification" sound, and the embedded WAVE resource.
- Logs everything to `out/log-<mode>.txt` and saves menu screenshots to `docs/research/img/`. Screenshots keep only
  the menu windows; everything else in the frame is painted grey.

## Build

```
%USERPROFILE%\.cargo\bin\cargo.exe build --release --target x86_64-pc-windows-msvc
%USERPROFILE%\.cargo\bin\cargo.exe build --release --target aarch64-pc-windows-msvc
```

`--no-default-features` drops the PNG encoder, only to measure its share of the exe size.

## Run

`target\x86_64-pc-windows-msvc\release\tray-spike.exe` with one of:

| Argument | What happens |
| --- | --- |
| (none) | Interactive: icon and menu for a person. Follows the system theme. Saves a screenshot each time the open menus change, and after a theme switch opens the menu by itself twice (before and after `FlushMenuThemes`). Quit from the menu. |
| `--auto` | About 25 s of unattended checks: add, modify, menus in forced light and dark, balloon, two toasts, two sounds, memory. Pops up notifications and plays sounds. |
| `--menus` | Only the menu screenshots of `--auto`. |
| `--idle N` | Icon only, then logs memory after N seconds and quits. |
| `--guid-test` | Adds and removes an icon identified by `NIF_GUID`. Run it from two folders to test the exe path binding. |
| `--cleanup` | Deletes the toast registry keys the spike made. Quit does this too. |
| `--token` | Logs this process's elevation and integrity level to `out/log-token.txt` and quits. No icon, no keys. |
| `--look` | Interactive, logs to `out/log-look.txt` and saves every picture to `out/look/` (git-ignored: raw captures can show parts of the screen). Takes the test messages below. |
| `--classic` | The Phase 0 classic menu instead of the owner-drawn one. |
| `--render` | No window: draws the owner-drawn items at 96, 120 and 144 dpi, light, dark and high contrast, into `out/look/render-*.png`. |
| `--allow-elevated` | Interactive mode refuses an elevated token (it would not get `TaskbarCreated` the normal way), with a message box. This flag overrides that. |

Every mode logs its token at start. For Haik's steps, start it from `spikes\morning-shell.cmd` (see
`docs/phases/00-morning-steps.md`).

Sending `WM_CLOSE` to the window (class `WakeshotTraySpike`) also quits cleanly.

The menu is owner-drawn by default (`src/look.rs`, card `docs/tasks/p1-tray-menu-look.md`): WinUI 3 MenuFlyout
sizes and colours, `DWMWCP_ROUND` on the menu window at the first `WM_DRAWITEM`, colours from `SystemUsesLightTheme`.
Test messages to the main window (`WM_APP` + n), used by the look card's scripts: 2 style (wParam bit 0 owner-drawn,
bit 1 forced high contrast, bits 8 and up corner bits: 1 first paint, 2 `WM_ENTERIDLE`, 4 border colour), 3 open the
menu by code and capture it (bit 0 submenu, bits 4 to 7 Down presses, lParam a point on the monitor to use), 4 remove
the icon, 5 backdrop window (0 off, 1 light, 2 dark), 6 capture the backdrop rect. Messages 3 and 6 name the picture
from `out/look/next-tag.txt`.

## Leftovers on the PC

- `HKCU\Control Panel\NotifyIconSettings\<id>` entries for each exe path and the GUID. Windows makes these for every
  tray icon; harmless.
- Toast keys `HKCU\Software\Classes\AppUserModelId\Wakeshot.TraySpike*` and
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\Wakeshot.TraySpike*`: removed on quit or with
  `--cleanup`.
- Windows makes `NotifyIconGeneratedAumid_<NotifyIconSettings id>` under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings` for the balloon. Quit and `--cleanup` do not
  remove it.
- The spike's toasts stay in the notification database after quit and expire after 3 days. Deleting the toast keys
  does not remove them.
