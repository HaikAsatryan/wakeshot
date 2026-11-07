# Polish the two exes (wakeshot-cli and wakeshot-tray)

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 2; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-bins\target`; delete it when done.

## Goal

Polish the `wakeshot` CLI and the `wakeshot-tray` exe: their sources and tests: comments that say why in a few words, no
planning history, clear names, no dead code, files in the right place. No behavior change, and the checks prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-cli/src/` (all) and `crates/wakeshot-cli/tests/` (all).
- `crates/wakeshot-tray/src/` (all) and `crates/wakeshot-tray/tests/` (all, `e2e/mod.rs` included).
- Not the `Cargo.toml` files, `build.rs` or the `.exe.manifest` files (`p8-repo-config`).

About 1,030 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. `wakeshot-tray/src/main.rs`: "the integration cards own these arms" and the owner lines go; keep why the exe is a
   GUI exe and why a bad command line shows only in the exit code.
3. `wakeshot-cli/src/args.rs` holds the help text, and `status.rs`, `run.rs` and `restore.rs` the messages users see:
   no string changes.
4. `wakeshot-tray/tests/e2e/mod.rs` has `#![allow(dead_code)]` because each test binary uses part of it: keep it, with
   that one-line reason.
5. `wakeshot-cli/src/hook.rs` (about 2,100 lines) is a candidate for rule C7 if its inline tests are most of it.
6. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
   candidates" and "Structure candidates" for `p8-repo-config`, each with one line of reason; anything left.

## Acceptance checks

The eight checks in the phase file's "Checks for a code card", for both packages (`-p wakeshot-cli -p wakeshot-tray`).
Record each with its outcome:

1. Files copied to `before`, test list saved, before the first edit.
2. `rustfmt --edition 2024` on each changed owned file.
3. Both clippy runs on both packages, x64: pass.
4. `cargo test` on both packages: pass; the test list has the same count, names changed only by listed renames.
5. `cargo doc --no-deps` on both packages: no warning in your files.
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
