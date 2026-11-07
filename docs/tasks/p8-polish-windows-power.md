# Polish the Windows power layer (wakeshot-windows)

Phase: 8. Status: done (2026-11-07 05:16). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
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

1. String literals in my files still carry planning names; rule C1 keeps every string, so I left them. For
   `p8-repo-config` or `p8-final-cleanup`, as a listed exception (none is shipped behavior):
   - `#[ignore = "..."]` reasons: `process.rs` two "needs Explorer as the shell (track note 3)"; `session.rs` one
     "real: locks the screen; p3-haik-checks only"; `power/system.rs` two "... (card p3-system-events)".
   - `examples/power_check.rs`: the `USAGE` text names `docs/tasks/p3-harness.md` (a removed path) and the manual
     check numbers ("Checks 1, 2 and 3", "Check 6", "Check 11", "Check 15"); two console lines say "sign out to run
     check 6" and "for checks 4 and 5". The test `usage_names_every_command_and_option` checks only command and option
     names, so a text change there does not break it.
   - `examples/power_check.rs` test fixtures use `C:\Users\Haik\...` paths: fixture data, allowed by the sweep rules.
2. Failures in files I do not own, from the parallel fix cards: `sqlite::tests::nothing_read_from_the_database_reaches_a_log`
   and `watch::agents::tests::image_since_counts_a_process_of_the_image_that_started_early_enough` failed at 05:13
   (see Log for the rerun). Both files belong to `p6-log-silent-decisions`, which changes log lines on purpose.
3. No bug found in the power code while reading its comments.

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 04:58: started. Read CLAUDE.md, the phase file and this card. `diagnostics.rs` (new, top-level) is not
  mine: the phase file gives new top-level files to `p8-polish-windows-app`, and `p6-log-silent-decisions` edits it now.
- 04:59: check 1: 16 owned files copied to `<scratchpad>\p8-polish-windows-power\before\`; `cargo test -p
  wakeshot-windows --locked -- --list` started 04:59 (773 tests, 191 in my modules). It built while I made the first
  comment-only edits in five small files (`lib.rs`, `error.rs`, `sink.rs`, `testing.rs`, `power/mod.rs`); no test name
  is in a comment, so the list is the before list. 1,428 comment lines (`//` first) in my files before.
- 05:01: small files done (`Owner:` lines out, `lib.rs` crate doc short, `sink.rs` names the real sink
  `app::window::WindowSink`).
- 05:07: `session.rs`, `process.rs`, `recovery.rs` and all of `power/` done: card codes, track notes, review finding
  numbers, "correction 12", "walk R6", "Haik" and phase questions replaced by the fact or dropped. Edits go through a
  script that refuses any line range that is not all comment lines. 05:08: example test list saved before its edits
  (29 tests, `--features test-support --example power_check`).
- 05:09: `power_check.rs` done: new top doc (what it checks, how to run it safely, how to build). Renames, for Haik's
  review: `get_active` to `active_scheme` (private, `power/plan.rs`; matches `PowerGetActiveScheme`, no `get_` per
  the API guidelines); test `setting_guids_match_the_spike` to `setting_guids_match_the_checked_literals` and its local
  `spike` to `checked` (the spike folder leaves the repo, so the old name points at nothing); example test
  `gaps_are_classified_like_the_spike` to `gaps_are_classified_by_unbiased_time` (what it proves). Three comments that
  only repeat the item name deleted (`process::current`, `process::current_session`, one test line in `session.rs`).
- 05:10: `rustfmt --edition 2024 --check` on the 15 changed files (not `lib.rs`): clean; then `rustfmt`: no change.
  No-code-change check: only `power/plan.rs` and `examples/power_check.rs` differ, and with the listed renames applied
  to a copy of `before` the check prints "no code change". No dead code found (every pub item has a use).
- 05:12: clippy, tests and doc started (own target folder).
- 05:14: three comment blocks I had made over 120 columns rewrapped (`executor.rs` `keep_guard`, `plan.rs` `sweep`;
  comment lines only). Clippy, all three runs: pass. `cargo test`: 745 passed, 2 failed, both in files of
  `p6-log-silent-decisions` (Found outside scope 2); my 191 tests pass. Test list: my modules 191 before and after,
  the one difference is my listed rename; the crate went 773 to 776 from three new tests in other cards' files.
  Example: 29 tests pass, list equal but for the listed rename.
- 05:16: reran after the last edits: clippy x3 pass; `cargo test` 746 passed, the same 2 fail (still in
  `sqlite.rs:1342` and `watch/agents.rs:4690`, log-line asserts; reported, not mine); `cargo doc`: 11 warnings, none
  in my files. No-code-change check against `before` with the renames applied: "no code change". Sweep: zero hits in
  comments. Build folder deleted. Done.

## Result

Status: done (2026-11-07 05:16).

What was done: all 16 owned files polished by rules C1 to C10. Planning history out of every comment: 28 `Owner:`
lines, card codes, track notes, review finding numbers, "correction 12", "walk R6", phase questions, "Haik", and the
pointers into `spikes/` and `docs/tasks/`, each replaced by the fact in a few words or dropped. Every safety reason,
Windows fact and source kept; all 154 `// SAFETY:` comments kept. `lib.rs` has a short crate doc (formatted by hand).
`power_check.rs` has a new top doc: what it checks, how to run it safely (quit Wakeshot and any keep-awake app, plan
copy, exit code 3, `--plan-only`, `cleanup`, one RunOnce command at a time) and how to build it. Three comments that
only repeated the item name went.

Comment lines (`//` first): 1,428 before, 1,379 after. The files were already dense; most of the change is history
out, not length.

Renames (old, new, file):
- `get_active`, `active_scheme`, `crates/wakeshot-windows/src/power/plan.rs` (private fn).
- test `setting_guids_match_the_spike`, `setting_guids_match_the_checked_literals`, same file; its local `spike`,
  `checked`. `docs/research.md` names the old test twice (lines 1498 and 4075 today): `p8-final-cleanup` follows.
- example test `gaps_are_classified_like_the_spike`, `gaps_are_classified_by_unbiased_time`,
  `crates/wakeshot-windows/examples/power_check.rs`. Only a removed task card named it.

Dead code removed: none found (every pub item in my files has a use). Files moved: none (no file over 1,500 lines
with a test module that is most of it, beyond `power_check.rs`, an example).

Rename candidates for `p8-repo-config`: none. `testing.rs` re-exports and `FakeMenuView` follow the naming guidelines;
no `get_`, `to_`, `as_` or `into_` misuse in my files.

Structure candidates for `p8-repo-config`: none needed. Several pub items are used only in their own file
(`recovery::{RecoveryOs, Recovered, GuardEnd, TrayStarted}`, `process::Spawned`, `executor::OWNER_UNKNOWN` and
others), but they sit in pub signatures or serve the test fakes, so narrowing them is not worth a change.

Files changed: `crates/wakeshot-windows/src/{lib,error,sink,testing,process,recovery,session}.rs`,
`crates/wakeshot-windows/src/power/{mod,executor,hibernate,journal,plan,requests,stop_points,system}.rs`,
`crates/wakeshot-windows/examples/power_check.rs`, this card.

Checks:
1. Files copied to `before`, test lists saved before the edits (lib 773, mine 191; example 29): done.
2. `rustfmt --edition 2024` on the 15 changed files: no change; `lib.rs` by hand.
3. Clippy `-p wakeshot-windows --all-targets`, without `--all-targets`, and `--all-targets --features test-support`
   (to cover `power_check`), x64, `-D warnings`: pass.
4. `cargo test -p wakeshot-windows`: my 191 tests pass; 2 fail in files of `p6-log-silent-decisions` (Found outside
   scope 2), the same on a rerun 3 minutes later. Lists: same counts in my modules and the example; names differ only
   by the listed renames. Example tests: 29 pass.
5. `cargo doc --no-deps`: no warning in my files (11 in other cards' files).
6. No-code-change check: against `before` it lists only `power/plan.rs` and `power_check.rs`; against `before` with
   the listed renames applied it prints "no code change".
7. Internal-name sweep: zero hits in comments. 11 hits remain in string literals (C1): 5 `#[ignore]` reasons, the
   `USAGE` line naming `docs/tasks/p3-harness.md`, 5 test fixture paths `C:\Users\Haik\...` (fixture data).
8. Comment lines 1,428 before, 1,379 after.

Open items: the string literals in Found outside scope 1, for `p8-repo-config` or `p8-final-cleanup`; the two failing
tests in Found outside scope 2, for `p6-log-silent-decisions`. Scratch evidence (`before`, scripts, test lists) stays
in `<scratchpad>\p8-polish-windows-power\`; the build folder is deleted.
