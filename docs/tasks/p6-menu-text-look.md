# Menu text and glyphs in grey like Windows' own menus, and a menu width that matches WinUI

Phase: 6 (brand review fixes). Status: done. Needs Haik: one question, the menu width (the default is in place). Agent: `wakeshot-worker`. Size: about 80K tokens.
Written by the orchestrator of Haik's brand session, 2026-11-05, from `docs/research/brand-review.md` findings M1 and M2
(both "should fix", neither depends on the brand pick).
Runs after: `p6-brand-cleanup` (done). Parallel with: the brand direction cards (no code). Nothing else that edits
`crates/wakeshot-windows`.
Real laptop: no power settings, no screen. Offscreen renders only (the session may be locked).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-menu-text-look\target`.

## Goal

The owner-drawn menu copies WinUI 3's `MenuFlyout` so it looks like Explorer's and the taskbar's menus
(`docs/research/tray-menu-look.md`, "Pick"). Two things break that: it draws text and glyphs with ClearType (colour
fringes; the grey check looks red-brown), where Windows' own XAML menus draw grayscale; and a long status line makes it
about 853 px wide where Explorer's menu is about 410 px.

## Read first

- `docs/research/brand-review.md` rows M1 and M2, and the picture `docs/research/img/brand/review-menu-glyph-fringes.png`.
- `crates/wakeshot-windows/src/tray/menu_paint.rs`: the font creation (about line 591, `lfQuality`), `TEXT_MAX`, and
  how text is drawn into the DIB (alpha handling matters: grayscale text over the item fill must stay correct in light,
  dark and high contrast).
- The WinUI 3 source the menu copies (`microsoft-ui-xaml`, `MenuFlyout_themeresources.xaml` and the presenter's
  template): is there a max width for a `MenuFlyoutPresenter` or its items? Read it at the source, not from memory.

## Deliverables

1. **M1.** Text and glyphs drawn grayscale (for example `ANTIALIASED_QUALITY` instead of `DEFAULT_QUALITY`, or the way
   the code draws allows), in light, dark and high contrast, at 96, 120 and 144 dpi. Check that the result has no
   chroma in the text and glyph pixels (measure, as the review did) and still looks as sharp as Explorer's menu.
2. **M2.** If WinUI sets a max width for the menu, use it (in epx, scaled like the other sizes) instead of
   `TEXT_MAX = 480`, so a long status line ends in an ellipsis at WinUI's width. If WinUI sets none, keep `TEXT_MAX`, and
   write the finding and a proposal under Questions for Haik.
3. Rewrite the offscreen renders `docs/research/img/ux/menu-render-*.png` and add one render with a long status line.
4. `docs/research.md`: one short dated line under the menu look facts (M1 measured before and after; M2 source).

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy -p wakeshot-windows --all-targets --locked --target x86_64-pc-windows-msvc
  -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-windows --locked`.
- Chroma in text and glyph pixels of the light and dark renders: 0 (or within 2), numbers in the Result.
- Every item and the hold line still fit or end in an ellipsis; Armenian text still draws (the fallback faces).

## Out of scope

The glyph choice, colours, the brand (the brand cards). Menu words. The tray icon. Screen pictures (next screen
session).

## Questions for Haik

1. **Menu width (M2).** WinUI's menu has no maximum width: the template sets only a minimum, and the menu stops at the
   monitor's edge (source in Result). So `TEXT_MAX` stays 480 epx, as this card says. Options:

   | Option | Long status line | The usual line (two groups, 406 epx of text) |
   |---|---|---|
   | Keep 480 epx of text (default, in place) | menu stops at 566 epx: 850 px at 150% | fits whole |
   | Whole menu at 456 epx, WinUI's cap for its plain `Flyout` | menu stops at 456 epx: 684 px at 150% | cut: "...Keep awake (25 min l..." |

   My pick: keep 480. The cap only acts when three or more things hold the PC, and 456 would cut the line people see
   most. Explorer's menu (410 px) is narrow only because its words are short. Nothing to undo either way: a one-line
   change of `TEXT_MAX`.

## Found outside scope

- `docs/research/brand-review.md` rows M1 and M2 still describe the old state (fringes, "should fix"); its owner may
  mark M1 fixed and M2 waiting on Haik's answer above.
- The screen pictures `docs/research/img/ux/menu-screen-*.png` still show ClearType text (and the cup); the next screen
  session redoes them (out of scope here, as the card says).

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started. Only other card in progress is `p6-brand-directions-2` (docs only), so no conflict.
- 2026-11-05: before, measured on the committed renders by a stdlib Python script (largest R, G, B spread over all
  pixels): light 158, dark 147 at 96, 120 and 144 dpi; 6230, 8799 and 10186 pixels over 2.
- 2026-11-05: WinUI read at the source (`gh api`, microsoft-ui-xaml main): `DefaultMenuFlyoutPresenterStyle` sets
  `MinWidth` (`FlyoutThemeMinWidth` 96) but no `MaxWidth`, in `MenuFlyout_themeresources.xaml` and the `_perf2026`
  copy; items trim with `Clip`. `FlyoutBase_partial.cpp` `UpdateTargetPosition` caps the presenter only at the
  monitor or window. Only the plain `FlyoutPresenter` has `MaxWidth` `FlyoutThemeMaxWidth` 456. So M2 goes to Haik.
- 2026-11-05: two tests added first (`text_and_glyphs_draw_grayscale`, `high_contrast_text_has_no_fringes`); both
  failed on the ClearType code (chroma 105; 51 off the blend), so they see fringes.
- 2026-11-05: tried `ANTIALIASED_QUALITY`: chroma 0, but "Open" at 120 dpi has ink mass 120 against Explorer's 155
  (old ClearType 152), visibly thin. A Python prototype that folds ClearType's three subpixel coverages into one grey
  keeps 151.5 and looks like Explorer's. Going with the fold.
- 2026-11-05: fold built in `draw` (fonts at `CLEARTYPE_QUALITY`); 21 menu_paint tests pass. Renders now PNG straight
  from the test (`art::png`), copied to `docs/research/img/ux/`: chroma 0 in all light and dark renders; "Open" ink
  mass 151.6 (Explorer 155.3); high contrast renders: every pixel within 1 of a blend of two main colours (before: up
  to 58 off, 1470 to 2493 pixels over 2). Two-group status line: 406 epx of text, main menu 492 epx.
- 2026-11-05: research line added; acceptance checks run (all pass); build folder deleted. Done.

## Result

**M1, done.** Menu text, marks, icons and the chevron now draw grey, with the same weight as before.

- How: fonts ask for `CLEARTYPE_QUALITY`. After each `DrawTextW`, every pixel that the text changed is set to one
  blend of its fill and its ink, by the mean of its three subpixel coverages (`fold`). Each pixel is measured against
  what was under it, so a neighbour's ink is never touched. This works in light, dark and high contrast: no alpha is
  involved (the DIB is filled first, then `BitBlt` `SRCCOPY` to a plain menu DC).
- Why not `ANTIALIASED_QUALITY`: chroma 0 too, but the text came out about a fifth lighter than Explorer's and looked
  thin. Ink of "Open" at 120 dpi: Explorer 155.3, old ClearType 151.7, GDI grayscale 119.9, fold 151.6. Change it
  when: a user asks for unsmoothed menu text, or XAML menus turn out to follow the "smooth edges of screen fonts"
  setting; then use `DEFAULT_QUALITY` (the fold leaves grey or plain text alone).
- Chroma (largest R, G, B spread over every pixel): light 158 and dark 147 before; 0 after in all six light and dark
  renders, the long render and the Armenian checks. High contrast renders: before, up to 58 off any blend of two main
  colours (1470 to 2493 pixels over 2); after, at most 1 (0 pixels over 2).

**M2, finding.** WinUI sets no maximum width for a menu. `DefaultMenuFlyoutPresenterStyle` sets `MinWidth`
(`FlyoutThemeMinWidth` 96) and no `MaxWidth` (in `controls/dev/CommonStyles/MenuFlyout_themeresources.xaml`, last commit
2026-05-30, and its `_perf2026` copy); items trim with `Clip`. `FlyoutBase::UpdateTargetPosition`
(`dxaml/xcp/dxaml/lib/FlyoutBase_partial.cpp`) caps the presenter only at the monitor or window. Only the plain
`FlyoutPresenter` has `MaxWidth` `FlyoutThemeMaxWidth` 456 (`FlyoutPresenter_themeresources.xaml`,
`dxaml/xcp/dxaml/themes/generic.xaml`). All read with `gh api` from microsoft-ui-xaml main. `TEXT_MAX` stays 480; its
comment now says this. The proposal is under Questions for Haik.

**Files.**
- `crates/wakeshot-windows/src/tray/menu_paint.rs`: `CLEARTYPE_QUALITY`, `draw` takes the canvas and folds, new
  `fold` and `rgb_of`, the `TEXT_MAX` comment. Tests: new `fold_takes_the_mean_coverage`,
  `text_and_glyphs_draw_grayscale` (light and dark, 96, 120, 144 dpi, every item kind, at rest and hot; also checks
  the text is smoothed, not aliased), `high_contrast_text_has_no_fringes` (coloured system colours) and
  `a_long_status_line_ends_in_its_column` (English and Armenian, three DPIs). The two fringe tests failed on the old
  code (chroma 105; 51 off the blend). `renders_for_review` now writes PNG through `art::png` (no BMP step) and adds
  the long render.
- `docs/research/img/ux/menu-render-<96|120|144>-<light|dark|contrast>.png` redone; new
  `menu-render-long-144-light.png` (three hold groups and Keep awake: the line ends in "--rel...", menu 850 px).
- `docs/research.md`: "Menu text in grey" bullet under "Tray UX: menu painting".

**Checks.** `cargo fmt --all --check` pass. `cargo clippy -p wakeshot-windows --all-targets --locked --target
x86_64-pc-windows-msvc -- -D warnings` pass, and without `--all-targets` pass. `cargo test -p wakeshot-windows --locked`:
596 passed, 0 failed, 24 ignored. Chroma: 0 (above). Every item and the hold line fit or end in an ellipsis (renders,
`a_long_status_line_ends_in_its_column`, `a_long_label_is_capped`); Armenian draws with Segoe UI
(`non_latin_labels_get_a_font_that_has_them`, `menu-check-armenian-*` in scratch).

**Open.** Haik's answer on the menu width. Not verified: what `CLEARTYPE_QUALITY` draws when the user turns font
smoothing off (not tried: it is a live user setting); the fold handles any result. The menu on screen (next screen
session). Scratch scripts and comparisons (`measure.py`, `weight.py`, `blend.py`, `cmp-*.png`) are in
`<scratchpad>\p6-menu-text-look\`.
