# In high contrast, the menu's status line in the colour WinUI gives secondary text

Phase: 6 (screen session finding). Status: done. Needs Haik: no (orchestrator decision for his review; Haik
2026-11-05 ~20:30: "do anything you want"). Agent: `wakeshot-worker`. Size: about 50K tokens.
Written by the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-menu-status-glyph` (done). Parallel with: `p6-media` Part C (`media/` only).
Real laptop: no screen, no power settings. Offscreen renders only. Haik's local Wakeshot is installed and running:
do not start, quit or test against it.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-menu-hc-status\target`.

## Goal

`p6-brand-screen-session` finding 4: in a high contrast theme (Night sky) the menu's status line is drawn in
`ButtonText`, the colour of the commands (`Palette::for_theme`: `secondary` is `c.button_text`), so a line that cannot
be picked looks like one that can. In the normal themes the status line is WinUI's secondary text (grey). Use, in high
contrast, the system colour WinUI's own high contrast dictionary gives that secondary text, so the status line sets
itself apart the way Windows' own controls do.

## Read first

- `crates/wakeshot-windows/src/tray/menu_paint.rs`: `Palette` and `Palette::for_theme` (the high contrast branch),
  where `secondary` and `status` are used (marks at rest, the chevron, the status line, its glyph).
- `docs/research/tray-menu-look.md` and `docs/research.md` (menu look facts).
- WinUI 3 at the source (`microsoft/microsoft-ui-xaml`): the high contrast entries of `TextFillColorSecondary` (or the
  brush the status line copies) and of the `MenuFlyoutItem` foregrounds. Read it at the source, not from memory.

## Deliverables

1. In high contrast, the status line's text and its glyph take the colour WinUI's high contrast dictionary gives the
   secondary text (likely `WindowText` or `GrayText`; the source decides). Marks at rest and the chevron keep what
   WinUI gives them; change them only if the source shows ours differ. If `secondary` serves both, split the field.
2. Tests: the high contrast palette for each of Windows' four contrast themes (their system colours as data) gives the
   status line the chosen colour, and it differs from the commands' colour where the theme's colours differ.
3. Rewrite the contrast renders `docs/research/img/ux/menu-render-*-contrast.png` with their example.
4. One dated line in `docs/research.md` under the menu look facts: the source file and entry, and the choice.

## Acceptance checks

- Whole workspace, both targets: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked --target
  <t> -- -D warnings` and the same without `--all-targets`; `cargo test --workspace --locked`; `cargo build --release
  --workspace --locked --target <t>`; `<t>` is `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (ARM64 needs the
  Build Tools `VC\Tools\Llvm\x64\bin` on that command's PATH only, `CLAUDE.md`). Never set `RUSTFLAGS`.

## Out of scope

The normal themes, menu words, the tray icon, screen pictures, `media/`.

## Questions for Haik

## Found outside scope

- `docs/research.md`, "Tray UX: menu painting", "Renders match the Phase 1 pick": "The contrast render uses this PC's
  system colours, as Phase 1's did (`ffffff`, `0078d7`, `000000`)" is now out of date; the contrast renders use Night
  sky (my new bullet says so). The orchestrator may replace that clause.
- `docs/research.md`, "Brand screen session": "Which colour WinUI uses for secondary text in contrast themes: **not
  verified**" is now answered by the new bullet ("High contrast status line in `WindowText`"); a pointer there would
  help.
- `crates/wakeshot-windows/examples/ux_menu.rs` `--contrast` uses a sample set where `window_text` and `button_text`
  are both yellow, so the on-screen harness cannot show the difference. Harmless; change it only if someone checks
  this on screen.

## Log

- 2026-11-05 ~21:00: card written by the orchestrator.
- 2026-11-05: worker started; read the card, `CLAUDE.md`, "How we work" and `Palette::for_theme`.
- 2026-11-05: read WinUI at the source (`gh api`, microsoft-ui-xaml main, commit `8463f45`, 2026-05-30).
  `Common_themeresources_any.xaml` HighContrast: `TextFillColorSecondaryBrush` is `SystemColorWindowTextColor`
  (primary and tertiary too; disabled `GrayText`). `MenuFlyout_themeresources.xaml` HighContrast: the menu's own
  secondary text, `MenuFlyoutItemKeyboardAcceleratorTextForeground` (Light and Default: `TextFillColorSecondaryBrush`),
  is `SystemColorWindowTextColorBrush`; item foreground `ButtonText`; `MenuFlyoutSubItemChevron` `ButtonText`, which
  the check (`DefaultToggleMenuFlyoutItemStyle`) and the radio bullet (`RadioMenuFlyoutItem_themeresources.xaml`) also
  use at rest. So marks and chevron stay `ButtonText` (ours match); the status line goes to `WindowText`.
- 2026-11-05: Microsoft's "Contrast themes" page (dated 2026-08-29): `WindowText` is for "any UI that can't be
  interacted with", `ButtonText` for "any UI that can be interacted with"; "Do not use SystemColorGrayTextColor for
  secondary body text". Agrees.
- 2026-11-05: the four themes' colours from `C:\Windows\Resources\Ease of Access Themes\*.theme`. Aquatic is
  `hcblack.theme` (the page's Aquatic swatches read `#202020`, `#8EE3F0`, `#75E9FC`, `#FFFFFF`); Desert `hcwhite`
  and Night sky `hc2` from the screen session; Dusk `hc1` by elimination.
- 2026-11-05: code done in `menu_paint.rs`: new `Palette::label` (status line text; `WindowText` in high contrast,
  `secondary` otherwise), status glyph one `WindowText`; tests updated, new
  `contrast_status_line_is_window_text_in_every_theme`. `cargo test -p wakeshot-windows --lib menu_paint`: 24 passed.
  Mutation check (paint the line in `secondary` again): the new test fails, as it should; restored.
- 2026-11-05: renders written to scratch, then the three contrast renders and the new
  `menu-render-themes-contrast.png` copied into `docs/research/img/ux/`; light, dark, long and glyph renders came out
  byte-equal with the committed ones (left as they are). One bullet added to `docs/research.md`, "Tray UX: menu
  painting". Next: the full checks on both targets.
- 2026-11-05: x64 checks all exit 0 (fmt, clippy with and without `--all-targets`, `cargo test --workspace --locked`,
  release build). Then one comment reworded in `for_theme` (no code change); ARM64 checks next.
- 2026-11-05: ARM64 checks all exit 0; x64 checks rerun on the final file, all exit 0. Scratch build folder (6.3 GB)
  and downloads deleted.

## Result

Done 2026-11-05.

**Colour chosen: `WindowText`** for the status line's text and its glyph in high contrast. Source: microsoft-ui-xaml
main (commit `8463f45`, 2026-05-30), `controls/dev/CommonStyles/Common_themeresources_any.xaml`, HighContrast
dictionary: `TextFillColorSecondaryBrush` is `SystemColorWindowTextColor`; and
`controls/dev/CommonStyles/MenuFlyout_themeresources.xaml`, HighContrast: the menu's own secondary text,
`MenuFlyoutItemKeyboardAcceleratorTextForeground`, is `SystemColorWindowTextColorBrush`. Microsoft's "Contrast themes"
page agrees (`WindowText` for "any UI that can't be interacted with"). Marks at rest and the chevron stay `ButtonText`:
WinUI's `MenuFlyoutSubItemChevron` is `ButtonText` in high contrast, and the check and radio marks take it at rest, so
ours already matched.

What changed:
- `crates/wakeshot-windows/src/tray/menu_paint.rs`: `secondary` split; new `Palette::label` (the status line; equal to
  `secondary` in light and dark, `WindowText` in high contrast); `Palette::status` in high contrast is one `WindowText`.
  Tests: `palette_light_and_dark_match_the_tokens` and `palette_contrast_uses_system_colours` updated;
  `high_contrast_text_has_no_fringes` now expects the status line in `WindowText`; new
  `contrast_status_line_is_window_text_in_every_theme` takes Windows' four themes as data (Aquatic, Desert, Dusk, Night
  sky, from the `.theme` files) and checks the palette and the drawn pixels. A mutation (status line back in
  `secondary`) makes it fail. `live_system_colours` and its imports removed (unused).
- Renders: `renders_for_review` draws the contrast renders in Night sky instead of this PC's classic colours, and adds
  `menu-render-themes-contrast.png` (all four themes, 120 dpi). Rewritten: `docs/research/img/ux/menu-render-96-,
  120-, 144-contrast.png`; new `menu-render-themes-contrast.png`. Light, dark, long and glyph renders came out
  byte-equal, so they were not touched.
- `docs/research.md`: one bullet under "Tray UX: menu painting".

Result per theme (status line, commands): Aquatic `#FFFFFF`, `#FFFFFF` (same, as in WinUI); Desert `#3D3D3D`,
`#202020`; Dusk `#FFFFFF`, `#B6F6F0`; Night sky `#FFFFFF`, `#FFEE32`. Status line on the fill: 16.3, 10.4, 13.0, 21:1.

Checks (all exit 0): `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked --target <t> -- -D
warnings` and without `--all-targets`, and `cargo build --release --workspace --locked --target <t>`, for
`x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (ARM64 with the Build Tools LLVM folder on that command's PATH);
`cargo test --workspace --locked` on x64: every suite ok, 0 failed (`wakeshot-windows` lib 616 passed).

Design choices (mine, for Haik's review):
- `WindowText` over `GrayText`: WinUI uses `WindowText`, and Microsoft says not to use `GrayText` for secondary text
  (it means disabled). Change it when WinUI's HighContrast entries above change, or a screen check in a contrast theme
  shows Windows' own XAML menus draw secondary text otherwise.
- A new field rather than reusing `secondary`: marks and the chevron must stay `ButtonText`. Change it when WinUI gives
  them and secondary text one colour again; then the fields can merge.
- Night sky data for the contrast renders, not live system colours: outside a contrast theme those are the classic
  black on white, where the status line and the commands are one black. Change it when renders are made on a PC in a
  real contrast theme.

Not verified: a real switch to a contrast theme with this build (offscreen renders only); Dusk is `hc1.theme` by
elimination. Open items: the three under "Found outside scope". No test outside this file had to change.
