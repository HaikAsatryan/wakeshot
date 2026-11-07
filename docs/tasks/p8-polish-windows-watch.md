# Polish the folder watch and agent IO (wakeshot-windows)

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 1; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-windows-watch\target`; delete it when done.

## Goal

Polish the folder watch and agent IO of `wakeshot-windows` (watch thread, folder notices, reads, agent sources and
liveness, hook folders, the SQLite reader, Restart Manager) and the `agent-watch` example: comments that say why in a
few words, no planning history, clear names, no dead code, files in the right place. No behavior change, and the checks
prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-windows/src/`: `watch/` (all: `agents.rs`, `data.rs`, `dir.rs`, `mod.rs`, `read.rs`, `thread.rs`),
  `agent_hooks.rs`, `sqlite.rs`, `lockowner.rs`.
- `crates/wakeshot-windows/examples/agent-watch.rs` (keep its name: research docs cite the command).

About 1,580 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. `watch/agents.rs` has a 37-line module doc listing the cards that touched it; keep what the module does and its
   non-obvious rules (what is opened and when, why Restart Manager is asked rarely), in fewer words.
3. `sqlite.rs` opens OpenCode's database, which also holds its login tokens: keep every comment that explains why
   only the two tables are read, and the test that proves it.
4. `agent_hooks.rs` serves the unwired Cursor and Antigravity hooks and the removal of folders an earlier build added
   (phase file, rule C6): trim comments, keep the code.
5. `watch/agents.rs` (about 4,300 lines) and `agent_hooks.rs` (about 3,100) are candidates for rule C7 if their
   inline tests are most of them.
6. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
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
