# Polish the tray icon, menu, notices and sounds (wakeshot-windows)

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
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

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
