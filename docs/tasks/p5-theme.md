# Read the Windows theme for the tray icon and menu

Phase: 5. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.
