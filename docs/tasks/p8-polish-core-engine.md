# Polish the core engine (wakeshot-core)

Phase: 8. Status: done (2026-11-07 05:17). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics`, and any Rust fix card open when it ends (a later one waits for this card; phase
file, "Order"). Parallel with: any card that does not own these files (group 1; a building card, at most four at once).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-polish-core-engine\target`; delete it when done.

## Goal

Polish the core's engine in `wakeshot-core`: the state machine, policies, events, commands, files, tray view and text:
comments that say why in a few words, no planning history, clear names, no dead code, files in the right place. No
behavior change, and the checks prove it.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "Pointers between files", "Polish rules" (Code, Every card), "Checks for a code
  card", "The no-code-change check", "File ownership".
- Your files' module docs, then every comment in them. Read comments, not whole files: a short script that prints each
  comment line with its file and line number is the fast way; open the code around a comment when you need it to
  judge.
- `docs/architecture.md`, `docs/product.md` and `docs/research.md` only where a comment points at them.

## Owned files

- `crates/wakeshot-core/src/`: `lib.rs`, `command.rs`, `core.rs`, `core/` (all), `event.rs`, `files.rs`, `holds.rs`,
  `holds/` (all), `present.rs`, `process.rs`, `settings.rs`, `state.rs`, `text.rs`, `time.rs`, `view.rs`, `policy/`
  (all).
- `crates/wakeshot-core/proptest-regressions/` (keep the files; they are seeds, not comments).

About 1,850 comment lines today. New files that `p6-logs-diagnostics` or another open card added in these folders
are yours too (phase file, "File ownership").

## Deliverables

1. Every owned file polished by rules C1 to C10: what-comments deleted, why-comments cut to a line or two, module docs
   short, no planning history, pointers into removed paths replaced by the fact, names by the API guidelines, dead
   code removed.
2. `lib.rs`: a short crate doc (a pure core: `(Now, Event)` in, commands out, no OS code, builds and tests on any OS;
   its public types are what the Windows layer and the exes use). No tracks, contract cards or `Owner:` lines. You
   own `lib.rs`; only you format it.
3. `state.rs`: its module doc speaks of later cards filling fields; say what the state is and drop the rest.
4. `text.rs` holds every word users see, approved by Haik: no string changes at all, comments only.
5. Pointers to decision numbers (`architecture.md` "Decisions and where they live") and to `product.md` sections
   ("User settings safety" N, "Battery floor", "Never lower security") stay as written; "track note N" and phase file
   pointers become the fact.
6. Result: comment lines before and after; renames (old, new, file); dead code removed; files moved; "Rename
   candidates" and "Structure candidates" for `p8-repo-config`, each with one line of reason; anything left.

## Acceptance checks

The eight checks in the phase file's "Checks for a code card", for `wakeshot-core`. Record each with its outcome:

1. Files copied to `before`, test list saved, before the first edit.
2. `rustfmt --edition 2024` on each changed owned file; `lib.rs` by hand.
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

- `cargo doc -p wakeshot-core` warns in `crates/wakeshot-core/src/ownership/reset.rs:68`: the public doc of `keys`
  links to the private `KEYS` (`rustdoc::private_intra_doc_links`). Owner: `p8-polish-core-ownership` (a comment fix).

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 04:58: started (worker, session wakeshot-4e). Read CLAUDE.md, plan "How we work", phase file, card.
- 05:00: 27 owned files copied to `<scratchpad>\p8-polish-core-engine\before\`; 1,525 comment lines. Test list build
  started 05:00 (my first comment edits landed while it compiled; comments cannot change test names). 05:01: list
  saved, 932 tests (`list-before.txt`).
- 05:11: planning history out of all 27 files (`Owner:` lines, card codes, track notes, review finding numbers,
  "Haik said", phase pointers); facts kept, pointers into removed paths replaced by the fact (`phase 1, "Values I
  set"` became product "Battery floor"; the sleep route card became decision 23). Ragged paragraphs reflowed by a
  script (comments only). No-code-change check: no code change. Sweep: 0 hits. Comment lines 1,525 to 1,391.
- 05:14: names checked against the API guidelines (no `get_`, conversions named right): no rename. Dead code: every
  public item is used (script over `crates/` and `packaging/`); private dead code would fail clippy. No file qualifies
  for a rule C7 test move (`core/lid.rs` has 1,518 lines and no tests). In `core/lid.rs` a comment that sat above the
  wrong statement moved to the `if` it explains; "as before the sleep route" wording out. Deleting the doc line
  `Holds hold.` made rustfmt reflow the next variant, so it stays (comment-only changes must not move code).
  rustfmt check on the 25 changed files (lib.rs left out): clean. Clippy, tests, list and doc running.
- 05:16: checks pass (Result). Over-long comment lines from my edits reflowed after the first run, so clippy, doc,
  rustfmt check, the compare and the sweep ran again on the final text: all clean.
- 05:17: build folder deleted. Calls for Haik's review: (1) long contract docs (`Event`, `Command`, `Outcome`) and
  the closed-lid rule docs in `core/lid.rs` stay long: each line is a rule a caller or a later change relies on;
  change it when a doc repeats `architecture.md` word for word, then point there instead. (2) Card-code facts were
  turned into plain dates ("one run, 2026-11-04", "screen captures, 2026-11-04", "on the test laptop's dock") taken
  from `research.md`, never invented. (3) `Holds hold.` stays (see 05:14).

## Result

Status: done (2026-11-07 05:17).

What was done: all 27 owned files in `crates/wakeshot-core/src/` polished, comments only. Planning history out:
`Owner:` lines, card codes, track notes, review finding numbers, "Haik said", phase and question pointers, "as before"
history wording. Contract text kept, rewritten without tracks ("The core may rely on", "Callers may rely on").
`lib.rs` has the short crate doc the card asked for; `state.rs` doc says what the state is. `text.rs`: comments only,
no string touched. Pointers to decisions and `product.md` sections stay; `research.md` pointers stay as written.

Files changed (26 of 27; `proptest-regressions/` untouched): `crates/wakeshot-core/src/` `lib.rs`, `command.rs`,
`core.rs`, `core/{holds,lid,settings,update,view}.rs`, `event.rs`, `files.rs`, `holds.rs`, `holds/file.rs`,
`present.rs`, `process.rs`, `settings.rs`, `state.rs`, `text.rs`, `time.rs`, `view.rs`,
`policy/{ending,floor,hold_files,lid,lock,mod,requests,sessions}.rs`. Unchanged byte for byte: none besides the
regression seeds (every `.rs` file had planning history).

Checks:

1. Before copies and the test list: 27 files in `<scratchpad>\p8-polish-core-engine\before\`, 932 tests listed at
   05:01 (the list build overlapped my first comment edits; comments cannot change test names, and the after list is
   identical).
2. `rustfmt --edition 2024` on the 26 changed files except `lib.rs` (owns modules of other cards): no change; `lib.rs`
   checked by hand (3 doc lines, under 120 columns).
3. Clippy `-p wakeshot-core --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`: pass; without
   `--all-targets`: pass (run twice, the second on the final text).
4. `cargo test -p wakeshot-core --locked`: pass, 931 passed, 1 ignored, 0 failed. Test list after: 932, identical to
   before (no name changed).
5. `cargo doc -p wakeshot-core --no-deps --locked`: no warning in my files. One warning in `ownership/reset.rs:68`
   (not mine; "Found outside scope").
6. No-code-change check (`code_diff.py`, the phase file's text): "no code change". No rename, removal or move.
7. Internal-name sweep (phase file regexes) on my files: 0 hits. An extra sweep (track, phase, question, review,
   spike, card) leaves one hit: `policy/floor.rs` cites research section "Power settings spike", a pointer into a kept
   doc, kept as written for `p8-final-cleanup` to rename if `research.md` renamed it.
8. Comment lines (first non-blank characters `//`): 1,525 before, 1,394 after.

Renames: none. Dead code removed: none found (every public item has a use; clippy finds no private dead code).
Files moved: none (no owned file over 1,500 lines is mostly tests).

Rename candidates (for `p8-repo-config`): none worth a cross-crate change. Names follow the guidelines; the closest
is `files::Read<T>`, which shares its name with `std::io::Read` (rename to `ReadOutcome` only if a reader ever needs
both in scope).

Structure candidates (for `p8-repo-config`):

- `core/lid.rs` (1,518 lines, no tests) holds four rule areas: lid and source locks, the battery floor step back,
  the countdown with the sleep route, and the Keep screen on idle lock. A split into `core/lid.rs`, `core/floor.rs`
  and `core/countdown.rs` would make each one readable alone. Not needed for v1.

Pointers for `p8-final-cleanup`: my files cite these `research.md` sections as written: "Idle sleep with the lid
closed", "Balloon on Windows 11", "Clock", "`LockWorkStation`", "Lock", "Power setting IDs", "Power rules checks",
"Power settings spike", "Stored at once, applied only on `PowerSetActiveScheme`". Check them against the heading
renames of `p8-docs-research-1` and `p8-docs-research-2`.

Open items: none.
