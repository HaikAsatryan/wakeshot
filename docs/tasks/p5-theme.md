# Read the Windows theme for the tray icon and menu

Phase: 5. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens.
Real laptop: no (registry and system reads only, no window, no switch). Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-theme\target`.
Runs after: the contracts change card (it creates `tray/theme.rs` as a stub). Parallel with: `p5-icon-art`,
`p5-text`, Part A of `p5-sounds` and `p5-notices`, and any card of another track.

## Goal

One small module that tells the icon and the menu which colours to use: the Windows mode (`SystemUsesLightTheme`, the
taskbar's), whether high contrast is on and its system colours, and whether any of that changed since the last read.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2, 3 and 4).
- `docs/architecture.md` section 5: "Tray, notices, sounds" (pick the glyph from the registry, not cached uxtheme
  values) and the OS events row "Theme".
- `docs/research.md`, "Tray, menu and notifications": "Theme and contrast reads", "Live theme switch through
  Settings", "ImmersiveColorSet also comes without a theme change".
- `docs/research/tray-menu-look.md`: difference 15 (high contrast colours) and "How the pick behaves" (the high
  contrast branch's colour list).
- `spikes/tray/src/theme.rs` (the spike's reads; port by reading and rewriting, never copy whole).

## Owns

`crates/wakeshot-windows/src/tray/theme.rs`. Nothing else in `crates/`.

## Deliverables

1. `pub struct Theme { pub light: bool, pub contrast: Option<Contrast> }` (`Clone`, `Copy`, `Debug`, `PartialEq`,
   `Eq`). `light` is `SystemUsesLightTheme` under `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`
   (`RegGetValueW`, `RRF_RT_REG_DWORD`). A missing value: take the Windows 11 default mode, with the source of that
   default in research (not verified until then).
2. `pub struct Contrast` with the system colours the art and the menu need, as RGB: window, window text, highlight,
   highlight text, gray text, button face, button text, hot light (`GetSysColor`), read only when
   `SystemParametersInfoW(SPI_GETHIGHCONTRAST)` has `HCF_HIGHCONTRASTON`.
3. `pub fn read() -> Result<Theme, WinError>`: one read of all of it. A failed read is an error, never a guess.
4. `pub struct ThemeWatch` with `new(first: Theme)`, `check(&mut self) -> Option<Theme>` (reads and returns the new
   theme only when it differs from the last one), and `check_with(read: impl FnOnce() -> Result<Theme, WinError>)`
   so tests drive it. A failed read keeps the last theme and returns `None`. Why: `WM_SETTINGCHANGE`
   "ImmersiveColorSet" comes twice per switch, the first time with stale values, and also with no change at all
   (research), so the icon compares values and never trusts the broadcast.
5. Unit tests, written first: `watch_reports_only_changes`, `watch_keeps_last_on_read_failure`,
   `watch_reports_contrast_on_and_off`, `contrast_colours_only_with_contrast_on` (pure, from a flag and a colour
   source), `reads_this_pc` (reads only; asserts `Ok`; prints the values for the Log).
6. `docs/research.md`: a new section "Tray UX: theme reads", with the default for a missing value and its source.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows tray::theme`: all pass; list the names in the Result.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-windows --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- `cargo build --release --locked -p wakeshot-windows --target x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`:
  exit 0.
- `reads_this_pc` gives the laptop's current values (expected now: dark, no contrast; Haik's theme is not switched).
- Only `tray/theme.rs` and the new `docs/research.md` section changed.

## Out of scope

- Reacting to the broadcast, timers, redrawing (`p5-tray-icon`). Colours for the menu (`p5-menu-paint`) and the
  glyph (`p5-icon-art`). Switching any setting. The app mode (`AppsUseLightTheme`): the menu and icon follow the
  Windows mode (product UX).

## Questions for Haik

None open. Decided by the worker, for Haik's review:

1. A missing `SystemUsesLightTheme` (value or key) reads as light, the mode Windows 11 installs with (`InstallTheme`
   is `aero.theme`, `SystemMode=Light`; research "Tray UX: theme reads"). Why: the card asks for the Windows 11
   default; Windows 11 writes the value, so a missing one is rare. Change it when a user report shows a dark taskbar
   with no value.

## Found outside scope

None.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-04 00:17: started by `wakeshot-worker` as the first card of the `p5-theme` + `p5-menu-paint` bundle. One
  build folder for the bundle: `<scratchpad>\p5-theme\target`.
- 2026-11-04 00:20: default for a missing value checked: `HKLM\...\Themes` `InstallTheme` = `aero.theme`, which has
  `SystemMode=Light`. No Microsoft page says what Explorer does with a missing value (web search). Tests and code
  written in one pass (the tests were planned first, not run red first).
- 2026-11-04 00:24: `tray::theme` 7 tests pass; `reads_this_pc` gives dark, no contrast. fmt, clippy (both targets,
  with and without `--all-targets`) and release builds (both targets) exit 0. Research section added.

## Result

Status: done (2026-11-04 00:24).

What was done: `crates/wakeshot-windows/src/tray/theme.rs` reads the Windows mode, high contrast and its colours,
and compares reads so the icon redraws only on a real change. Research: `docs/research.md`, new section "Tray UX:
theme reads" (the missing-value default and its source).

Public API as built (`wakeshot_windows::tray::theme`):

- `Rgb { r, g, b }` (`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`): `new(r, g, b)`, `from_colorref(u32)`,
  `colorref() -> u32` (GDI `0x00BBGGRR`), `From<Rgb> for [u8; 3]` (what `art::colorize` takes).
- `Theme { light: bool, contrast: Option<Contrast> }`; `Contrast { window, window_text, highlight, highlight_text,
  gray_text, button_face, button_text, hot_light }`, each an `Rgb`. Both `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`.
- `read() -> Result<Theme, WinError>`: `RegGetValueW` (`RRF_RT_REG_DWORD`) of `SystemUsesLightTheme`; a missing
  value or key is light, any other failure is `Err` with call `RegGetValueW`. `SystemParametersInfoW
  (SPI_GETHIGHCONTRAST)` failure is `Err` with call `SystemParametersInfoW`. `GetSysColor` only with high contrast on.
- `ThemeWatch`: `new(first)`, `current() -> Theme` (added: the last good theme, so `p5-tray-icon` needs no second
  copy for a re-add or a display change), `check() -> Option<Theme>`, `check_with(read) -> Option<Theme>`. A failed
  read keeps the last theme and returns `None`.

Checks:

- `cargo test --locked -p wakeshot-windows tray::theme`: 7 passed: `watch_reports_only_changes`,
  `watch_keeps_last_on_read_failure`, `watch_reports_contrast_on_and_off`, `contrast_colours_only_with_contrast_on`,
  `reads_this_pc`, plus `colorref_is_red_in_the_low_byte` and `missing_value_reads_as_the_windows_11_default`.
- `reads_this_pc`: `Theme { light: false, contrast: None }`, as expected (dark, no contrast).
- `rustfmt --edition 2024 --check` on `theme.rs`: no diff.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for x64 and ARM64: exit 0 (4 runs).
- `cargo build --release --locked -p wakeshot-windows --target <t>`, x64 and ARM64: exit 0.
- Changed: only `theme.rs` and the new research section (plus this card).

Open items: none. A real high contrast switch is `p5-haik-check`'s, optional.
