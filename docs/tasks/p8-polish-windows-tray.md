# Polish the tray icon, menu, notices and sounds (wakeshot-windows)

Phase: 8. Status: done. Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 2; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-windows-tray\target`; delete it when done.

## Goal

Polish the tray UI of `wakeshot-windows` (icon art, icon, menu, menu painting, theme, notices, sounds) and the `ux_*`
examples: comments that say why in a few words, no planning history, clear names, no dead code, files in the right
place. No behavior change, and the checks prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-windows/src/`: `tray.rs`, `tray/` (all: `art.rs`, `icon.rs`, `menu.rs`, `menu_paint.rs`,
  `theme.rs`), `notify.rs`, `sound.rs`, `sound/` (all: `chime.rs`, `device.rs`, `player.rs`).
- `crates/wakeshot-windows/examples/`: `ux_all.rs`, `ux_icon.rs`, `ux_menu.rs`, `ux_notices.rs`, `ux_sounds.rs`.

About 1,580 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. Old-brand pointers: `art.rs` cites `brand-review.md` (T2, T4, T7) and `brand-directions-2.md`, `menu_paint.rs` cites
   `brand-windows.md`. `p8-cleanup` removes those reports. Point at `docs/brand.md` or state the reason in a few
   words.
3. `sound/chime.rs` is read by `media/scripts/make-sounds.mjs` with regular expressions (`const NAME: Shape = Shape {`
   up to `\n};`, and `key: number` inside). Keep those blocks' code shape, and write no `word: number` text in a
   comment inside them.
4. `art.rs` writes `assets/`, `site/` and `docs/research/img/ux/icon-sheet-*.png`, and its golden tests compare them.
   The SVG text it writes names `write_art_files` (a string literal): do not rename that function.
5. Pointers to `research.md` sections ("Tray UX: menu painting", "Live theme switch through Settings", "Menu text in
   grey") stay as written.
6. `art.rs` and `menu_paint.rs` (about 2,900 and 2,400 lines) are candidates for rule C7 if their inline tests are
   most of them.
7. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
   candidates" and "Structure candidates" for `p8-repo-config`, each with one line of reason; anything left.

## Acceptance checks

The eight checks in the phase file's "Checks for a code card", for `wakeshot-windows`. Record each with its outcome:

1. Files copied to `before`, test list saved, before the first edit.
2. `rustfmt --edition 2024` on each changed owned file.
3. Both clippy runs on `wakeshot-windows`, x64: pass.
4. `cargo test` on `wakeshot-windows`: pass; the test list has the same count, names changed only by listed renames.
5. `cargo doc --no-deps` on `wakeshot-windows`: no warning in your files.
6. The no-code-change check: every difference is a listed rename, removal or move.
7. The internal-name sweep on your files: zero hits.
8. Comment line count before and after, recorded.

## Out of scope

- Files you do not own, docs, every `Cargo.toml`, `build.rs`, test fixture data.
- String literals, signatures used by files you do not own, behavior, test expectations.
- Git. A bug goes under "Found outside scope".

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

- `cargo doc -p wakeshot-windows` gives 10 warnings (public docs linking private items), none in my files:
  `agent_hooks.rs` (4), `app/window.rs`, `sqlite.rs`, `store.rs`, `update.rs`, `watch/thread.rs` (2). For the cards
  that own them.
- The same `win_error` helper is copied in 12 files of `wakeshot-windows` (and `last_error` in 4); see "Structure
  candidates".

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 05:06: started (worker, session wakeshot-4e). Read `CLAUDE.md`, the phase file and this card.
- 2026-11-07 05:07: the 16 owned files copied to `before`; 1,309 comment lines (lines starting `//`). Test list saved:
  773 tests in the package, 125 in my modules. The list build ran while my first comment edits began; comments cannot
  change a test name, so it stands as the before list.
- 2026-11-07, after 05:07 (not clocked): `tray.rs`, `sound.rs`, `sound/chime.rs`, `sound/device.rs`,
  `sound/player.rs` polished (`Owner:` lines, card codes, spike paths out; "this laptop" and "Haik heard" now "the test
  laptop" and "tested").
- 2026-11-07, before 05:27 (not clocked): `theme.rs`, `notify.rs`, `icon.rs`, `menu.rs` and the five `ux_*` examples
  polished. Rename: `Rgb::colorref` to `to_colorref` (API guidelines: a by-value conversion of a `Copy` type takes
  `to_`; every use is in `theme.rs`, `menu.rs`, `menu_paint.rs`). For Haik's review: the demos' "Start it at Medium
  (track note 4)" now says why in words: as a user's tray runs.
- 2026-11-07, before 05:27 (not clocked): `menu_paint.rs` and `art.rs` polished: brand report pointers
  (`brand-review.md` T2, T4, T7, `brand-directions-2.md`, `brand-windows.md`) replaced by the fact or by
  `docs/brand.md`; "round 2", review finding codes (M1, A4, T1) and "Haik's pick" dates out.
- 2026-11-07, before 05:27 (not clocked): sweep clean; no dead item (each defined name counted across `crates/`,
  `packaging/`, `media/`, `site/`: only test functions appear once); seven what-comments deleted. Rule C7: the inline
  tests of `tray/menu_paint.rs` (1,432 of 2,524 lines) moved by script to `tray/menu_paint/tests.rs`; the moved body's
  stripped text equals the new file's. rustfmt then dropped one closure's braces and added one trailing comma there
  (one level less deep, the body fits a line). `art.rs` stays whole: its `mod tests` is 1,140 of 3,052 lines, not
  most.
- 2026-11-07 05:27: checks done (Result). One more comment fix: `menu.rs` linked the private `placement` from a public
  doc (a `cargo doc` warning that was there before); now plain code text. Build folder deleted. The Log times between
  05:07 and 05:27 were not clocked; corrected from estimates to "not clocked".

## Result

Done 2026-11-07 05:27. Every owned file polished by rules C1 to C10; no behavior change, proven below.

**Comment lines** (lines whose first non-blank characters are `//`): 1,309 before, 1,267 after (the moved tests
counted in `menu_paint/tests.rs`). What went: 18 `Owner:` lines, every card code, "track note", "Phase N" as process,
review finding codes (T2, T4, T7, M1, A4, T1), "round 1/2", "Haik's pick/check/heard/DELL", spike paths, a pointer
into `docs/tasks/`, six pointers into three removed brand reports, seven what-comments, and wordy parts of a few long
docs. Every `// SAFETY:` comment kept (189 before, 189 after). Most remaining comments are real reasons (Windows
quirks, measured values, contracts), so the count drops less than the history did.

**Renames** (old, new, file):

- `Rgb::colorref`, `Rgb::to_colorref`, `tray/theme.rs` (uses in `tray/menu.rs`, `tray/menu_paint.rs`; no use outside
  my files). Other files need not follow.

**Dead code removed**: none found. Every item defined in my files is used (name counts across `crates/`,
`packaging/`, `media/`, `site/`; both clippy runs clean).

**Files moved** (rule C7): the inline `mod tests` of `crates/wakeshot-windows/src/tray/menu_paint.rs` to the new
`crates/wakeshot-windows/src/tray/menu_paint/tests.rs`, by script (body dedented four spaces), leaving `#[cfg(test)]
mod tests;`. Test paths unchanged (`tray::menu_paint::tests::*`). Moved body stripped == new file stripped. Then
rustfmt, one level less deep, turned one closure body `|row| { expr }` into `|row| expr` and added one trailing comma
(in the test helper `contrast`); same meaning.

**Pointers**: brand reports now point at `docs/brand.md` ("Colours", "Tray icon") or state the fact. Pointers to
`research.md` sections stay as written (no new one added). `menu.rs` and `menu_paint.rs` keep their pointer to
`docs/research/tray-menu-look.md`, "Pick". `chime.rs` `Shape` blocks untouched (make-sounds.mjs reads them); its
module doc now says so. `write_art_files` not renamed.

**Calls for Haik's review**:

1. `Rgb::colorref` renamed to `to_colorref` (API guidelines). Change it when: never needed; mechanical to undo.
2. The menu_paint tests moved to `menu_paint/tests.rs` (call 7 of the phase). `art.rs` (3,052 lines) kept whole:
   its tests are not most of it and its test-only helpers sit outside `mod tests`, so a cut would be hand edits.
3. The demos' "Start it at Medium (track note 4)" became "as a user's tray runs (`explorer.exe <launch .cmd>` from an
   elevated shell)".
4. `IconChoice::PRODUCT`'s doc, which said the pick waited on a later comparison, now states the result recorded in
   `research.md` (the app icon names the sender; both read clearly).
5. `icon.rs` `ICON_UID`: "(which one: Part B logs it)" became "(which one: not verified)", as `research.md` lists it.
6. Seven what-comments deleted (doc lines that only repeated the item's name, on `tray_mask`, `TASKBAR_DARK`,
   `TASKBAR_LIGHT`, both `NoteHook`s, `NOTICE_NAME`, `Rgb`).

**Rename candidates** (for `p8-repo-config`): none that cross my files' borders. `PickWhy` (`sound/device.rs`, only in
my files) reads oddly but breaks no guideline; left as is.

**Structure candidates** (for `p8-repo-config`):

- One shared `win_error` and `last_error` in `error.rs`: the same `win_error` is copied in 12 files (`app/window.rs`,
  `localtime.rs`, `log.rs`, `session.rs`, `power/hibernate.rs`, `power/requests.rs`, `sound/device.rs`,
  `tray/icon.rs`, `tray/theme.rs`, `tray/menu.rs`, `tray/menu_paint.rs`, `examples/power_check.rs`). Reason: one
  rule ("Win32 code if any, else the HRESULT") in one place.
- `tray/art.rs` to `wakeshot-core` (its module doc says it uses no Windows API), or its tests to `tray/art/tests.rs`
  with the test-only helpers. Reason: the largest file in the crate (3,052 lines); a move needs hand edits.

**Acceptance checks** (`wakeshot-windows`, x64, own `CARGO_TARGET_DIR`):

1. Files copied to `before` and the test list saved before the first edit: done (05:07).
2. `rustfmt --edition 2024` on `tray.rs`, `sound.rs`, `notify.rs` and the five examples (it follows `tray/`,
   `sound/` and `menu_paint/tests.rs`): `--check` exit 0.
3. `cargo clippy -p wakeshot-windows --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`: pass;
   the same without `--all-targets`: pass.
4. `cargo test -p wakeshot-windows --locked`: 756 passed, 0 failed, 28 ignored. Test list: my modules 125 before,
   125 after, names identical. The package went 773 to 784 only by 11 new tests in other cards' files (`watch/`,
   `sqlite.rs`, `app/io_thread.rs`, from `p6-log-silent-decisions`). `ux_all` example tests: 7 passed.
5. `cargo doc -p wakeshot-windows --no-deps --locked`: no warning in my files (10 in others, "Found outside scope").
6. The no-code-change check: plain run lists `menu.rs`, `menu_paint.rs`, `theme.rs` (the rename and the move);
   normalized run (rename applied to `before`, the tests put back inside `mod tests { }`, rustfmt's closure braces
   restored): "no code change".
7. Internal-name sweep on my 17 files: zero hits for the phase file's patterns. My extra pattern `Phase [0-9]` hits
   one string literal, an assertion message in `menu_paint/tests.rs` ("#33000000, the Phase 1 border"): rule C1
   forbids changing it; left for whoever may change test text.
8. Comment lines: 1,309 before, 1,267 after.

**Files changed**: `crates/wakeshot-windows/src/tray.rs`, `crates/wakeshot-windows/src/tray/{art,icon,menu,
menu_paint,theme}.rs`, `crates/wakeshot-windows/src/notify.rs`, `crates/wakeshot-windows/src/sound.rs`,
`crates/wakeshot-windows/src/sound/{chime,device,player}.rs`, `crates/wakeshot-windows/examples/ux_{all,icon,menu,
notices,sounds}.rs`, this card. **Created**: `crates/wakeshot-windows/src/tray/menu_paint/tests.rs`. **Deleted**: the
build folder in the scratchpad.

**Left**: nothing in scope.
