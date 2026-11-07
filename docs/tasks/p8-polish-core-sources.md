# Polish the agent sources and the core's integration tests (wakeshot-core)

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 1; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-core-sources\target`; delete it when done.

## Goal

Polish the agent sources in `wakeshot-core` and the crate's integration tests and fixture notes: comments that say why
in a few words, no planning history, clear names, no dead code, files in the right place. No behavior change, and the
checks prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-core/src/`: `sources.rs`, `sources/` (all: `antigravity.rs`, `codex.rs`, `copilot.rs`, `cron.rs`,
  `cursor.rs`, `hooked.rs`, `opencode.rs`, `vscode.rs`, `claude/`).
- `crates/wakeshot-core/tests/`: every `.rs` file, the `README.md` files under `fixtures/`, and
  `vscode_source.proptest-regressions` (kept as it is). Fixture data files are never edited.

About 1,980 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. Agent facts stay with their evidence: the agent version seen, "seen live", "not verified". Drop the card or session
   that saw it. A fact written twice (module doc and function doc) stays once.
3. Cursor, Antigravity and VS Code Local chats are out of v1 but their code stays unwired (phase file, rule C6): trim
   comments, keep the code and the one line that says why it is not wired.
4. Fixture `README.md` files: how each fixture was captured and what it shows, without card codes; pointers to the
   research reports stay as written (their headings may be renamed; `p8-final-cleanup` fixes them).
5. `sources/claude/transcript.rs` (about 2,800 lines) is a candidate for rule C7 (tests to `transcript/tests.rs`) if
   its inline tests are most of it.
6. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
   candidates" and "Structure candidates" for `p8-repo-config`, each with one line of reason; anything left.

## Acceptance checks

The eight checks in the phase file's "Checks for a code card", for `wakeshot-core`. Record each with its outcome:

1. Files copied to `before`, test list saved, before the first edit.
2. `rustfmt --edition 2024` on each changed owned file.
3. Both clippy runs on `wakeshot-core`, x64: pass.
4. `cargo test` on `wakeshot-core`: pass; the test list has the same count, names changed only by listed renames.
5. `cargo doc --no-deps` on `wakeshot-core`: no warning in your files.
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
