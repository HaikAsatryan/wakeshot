# In high contrast, the menu's status line in the colour WinUI gives secondary text

Phase: 6 (screen session finding). Status: in progress. Needs Haik: no (orchestrator decision for his review; Haik
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

## Log

- 2026-11-05 ~21:00: card written by the orchestrator.

## Result
