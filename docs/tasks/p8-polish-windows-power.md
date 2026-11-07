# Polish the Windows power layer (wakeshot-windows)

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 1; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-windows-power\target`; delete it when done.

## Goal

Polish the power layer of `wakeshot-windows` (power requests and settings, journal, hibernate, recovery, process and
session) and the `power_check` example: comments that say why in a few words, no planning history, clear names, no dead
code, files in the right place. No behavior change, and the checks prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-windows/src/`: `lib.rs`, `error.rs`, `sink.rs`, `testing.rs`, `power/` (all: `executor.rs`,
  `hibernate.rs`, `journal.rs`, `mod.rs`, `plan.rs`, `requests.rs`, `stop_points.rs`, `system.rs`), `recovery.rs`,
  `process.rs`, `session.rs`.
- `crates/wakeshot-windows/examples/power_check.rs`.

About 1,830 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. `lib.rs`: a short crate doc (every Windows call lives here; the traits are the seams where a real, dry-run or fake
   implementation is swapped). No contract card or `Owner:` lines. You own `lib.rs`; only you format it, by hand
   (rustfmt would follow every `mod` line into other cards' files).
3. This code writes the user's real power settings: keep every safety reason and every Windows fact with its source
   ("Microsoft docs", "tested on the test laptop, date"). "Power safety review finding N" becomes the fact.
4. `examples/power_check.rs`: its top doc says what it checks and how to run it safely (plan copy, quit Wakeshot and
   any keep-awake app first); the history of which card added which check goes.
5. `testing.rs` serves other crates' tests (`test-support`): list renames of its public items as candidates.
6. `get_active` in `power/plan.rs` is private: a rename (for example `active_scheme`) is yours if you think it reads
   better.
7. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
   candidates" and "Structure candidates" for `p8-repo-config`, each with one line of reason; anything left.

## Acceptance checks

The eight checks in the phase file's "Checks for a code card", for `wakeshot-windows`. Record each with its outcome:

1. Files copied to `before`, test list saved, before the first edit.
2. `rustfmt --edition 2024` on each changed owned file; `lib.rs` by hand.
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
