# Close contracts: update notice, owner lines, answered questions, verify findings

Phase: 1. Status: done. Needs Haik: no.
Runs after: the verify review `docs/reviews/p1-fixes-verify.md` (reviewer, no card). Parallel with: only
`p4-claude-live-shapes` (no build; it owns its fixtures folder, a section of `docs/research/claude-code-signals.md`
and its own card, so never edit those). It changes contract files and many cards, so no other card and no other
cargo command runs beside it.

## Goal

The last contracts and plan change before the track builds start: add the update notice Haik said yes to, fix the
owner doc lines and stale design lines that `p1-track-plan-fixes` left open, record Haik's answers in the cards, and
fix the findings of the verify review.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/phases/02-06-signoff.md`: the 12 questions. Haik answered 2026-11-03 18:09: all defaults.
- `docs/tasks/p6-plan-ship.md` "Contract changes for the contracts card", item 4.
- `docs/tasks/p1-contracts-split.md` Result (owner doc lines, what each file holds).
- `docs/tasks/p1-track-plan-fixes.md` "Found outside scope" and Result "Open items".
- `docs/reviews/p1-fixes-verify.md` (written by the reviewer before this card starts).

## Deliverables

1. Ship Question 3 (sign-off item 7) is yes. Add `Event::Updated { version: String }` in
   `crates/wakeshot-core/src/event.rs` (posted once at start when Velopack restarted the tray after an update) and
   `Notice::Updated { version: String }` in `crates/wakeshot-core/src/view.rs` (the core answers with `Notify`; the
   tray track words it), each with a doc comment that names its owner track, in the style of the other variants. If
   `crates/wakeshot-tray/src/text.rs` has a signature per notice, add the one for `Updated`, so `p5-text` words it
   without a contract edit. Every match on these enums still compiles. The only `Cargo` changes allowed are the
   ones the verify review's Major 1 asks for (`proptest` as a dev-dependency of `wakeshot-cli`, the
   `Win32_System_Environment` feature of `windows` in `wakeshot-windows`), each checked against current docs.
2. Owner doc lines (`p1-track-plan-fixes`, Found outside scope): `text.rs` and `art.rs` add "then `p5-haik-check`"
   (planned hand-over); the done files that the `p4-live-*` cards fix (`sources/*`, `cron.rs`, `holds/file.rs`,
   `watch/*`, `lockowner.rs`) add "fixes by the `p4-live-*` cards". Change only the first doc line of each file.
3. `docs/architecture.md`: line 612 (`restore_from_journal`), line 943 (the test plan copy names "Wakeshot test
   <pid>" and "Wakeshot test e2e"), section 6 gets one line for `store::AutoStart` as the one Run value writer, and
   the two new variants where sections 2 and 3 list the enums. Re-read each spot first; line numbers may have moved.
4. Haik's answers in the cards and phase files: every header, table row or step that still says a card waits on one
   of the 12 questions, or builds "the default until he answers", now says the answer (default, Haik 2026-11-03
   18:09). Known spots: `p2-core-lid` items 8a (build it: Question 4 yes) and 9a (build the `Updated` sentence:
   Question 7 yes), `p2-core-settings`, `p3-recovery`, `p4-cli-restore`, `p5-sounds`, `p5-notices` if it mentions
   Question 4, `p6-velopack-dependency`, `p6-readme`, the 02 to 06 phase files (tables, Questions sections, 02
   Coverage, 06 lines 185 to 191). Grep for "Question" and "answer" in `docs/phases/0[2-6]-*.md` and the p2 to p6
   cards to find the rest. Work that the answers add goes into the card that owns that file (for example the tray's
   wording of `Notice::Updated` in `p5-text`, and the core rule `Updated` gives `Notify(Updated)` in `p2-core-lid`).
5. Every finding in `docs/reviews/p1-fixes-verify.md`: fixed, or listed in the Result with the reason.
6. `docs/research.md`: one short section "Close contracts checks" (additions only) with what you checked and how.

## Acceptance checks

This card runs alone, so it runs the whole-workspace checks. `CARGO_TARGET_DIR` is
`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\e79ea594-d540-4910-be2d-33ca362d9e70\scratchpad\p1-close-contracts\target`.
Never set `RUSTFLAGS`.

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- `cargo test --workspace --locked`.
- `cargo build --release --workspace --locked --target <t>` for both targets.
- `Cargo.lock`: the diff holds only what Major 1 needs (no new package downloaded, no version change); record the
  diff. Then every check above passes with `--locked`.
- Each of the 12 questions: no card or phase file still waits on it (grep, recorded).
- Every finding of the verify review: fixed or listed.
- No em or en dashes in changed files; no new line over 120 characters outside tables.
- Delete the build folder at the end.

## Out of scope

- The integration plan (the orchestrator writes it).
- `velopack` and any new dependency beyond Major 1 (`p6-velopack-dependency`).
- Product logic in any track's files: stubs and signatures only.
- `docs/plan.md` and `docs/phases/01-design-skeleton.md` (the orchestrator's).
- Git: never run it.

## Questions for Haik

Each with the worker's default.

None. Every design call here was mine (below, in Result).

## Found outside scope

Problems seen but not fixed.

- `CLAUDE.md` Status still says each file names its owner "in its first doc line". The owner is in the module doc's
  `Owner:` line (usually the third line), as `lib.rs` and `architecture.md` now say. Not edited: `CLAUDE.md` is not
  this card's file. Suggest: "each file names its one owner in its module doc's `Owner:` line".
- The same old claim stands in `p1-contracts-split.md` Result (a done card, history) and in the Read first of
  `p6-plan-integration.md` (running beside this card).
- This card's deliverable 1 names `crates/wakeshot-tray/src/text.rs`; the file is `crates/wakeshot-core/src/text.rs`
  (verify review Minor 6). Left as written; the work used the right file.
- "Runs after: Haik's sign-off of the track plans" in `p3-log`, `p3-power-plan`, `p3-process`, `p3-requests`,
  `p3-session`, `p3-store`, `p3-system-events`, `p6-velopack-facts` is now met; left as it is (true, harmless). The
  same for "batched for the plan sign-off or `p5-haik-check`" in `p5-text` and `p5-icon-art`: the sign-off passed
  without those looks, so they fall to `p5-haik-check`.
- `p6-int-update-wiring.md:3` (owned by `p6-plan-integration`) says "uses his answers 7 and 9" but names only item 7's
  answer; item 9 is the public pre-release. Not checked further.

## Log

- 2026-11-03 18:13: card written by the orchestrator after Haik's answers (18:09) and the first push.
- 2026-11-03: worker started. Read the card, `plan.md` How we work, the verify review.
- 2026-11-03: Major 1 Cargo part done. `Win32_System_Environment` added to `wakeshot-windows` (checked: local
  `windows-0.62.2` source gates `Environment` behind it, `GetCommandLineW` has no other cfg; Microsoft's 0.62.2 docs
  put it in `Win32::System::Environment`). `proptest.workspace = true` in `wakeshot-cli` dev-dependencies (crates.io:
  1.12.0 is the newest, not yanked). `cargo metadata --offline`: `Cargo.lock` gains one line, ` "proptest",` in the
  `wakeshot-cli` entry (backup diff in the scratchpad). Slip: one PowerShell line held a `git diff --no-index --stat`
  by mistake; it failed with a usage error (exit 129) and changed nothing.
- 2026-11-03: deliverable 1 code done: `Event::Updated { version }` (event.rs, owner list plus a rely-on bullet),
  `Notice::Updated { version }` (view.rs). No exhaustive match on either enum in `crates/` (grep). `text.rs` is in
  the core (`crates/wakeshot-core/src/text.rs`, verify Minor 6) and has one `notice(&Notice)` for all notices, so no
  new signature. Deliverable 2 and verify Minor 2 code done (scratchpad `owner_edits.py`): live-card fix names on 13
  done files, `p5-haik-check` on `text.rs` and `art.rs`, an `Owner:` line in the 17 files the review lists (plus
  windows `lib.rs`), the two `lib.rs` claims now say "the module doc names the owner in its `Owner:` line",
  `install.rs` line 1 without the Run value.
- 2026-11-03: deliverable 3 done in `architecture.md`: header; section 2 line 45 (owner claim) and the `install` row
  (no Run value); section 3 `Event::Updated` in the enum block and `Updated { version }` in the `Notice` list; section 5
  restore routine named `ownership::restore` through `power::journal::restore_locked`; section 6 one line for
  `store::AutoStart` as the one Run and RunOnce writer; section 9 test plan names "Wakeshot test <pid>" and "Wakeshot
  test e2e". Next: deliverable 4 (answers in cards and phase files), Major 2, the minors.
- 2026-11-03: Major 2 done: `02-core.md` note 20 has the scratch copy rule (with `assets\`, which the exes' build
  scripts read, and its own build folder); `p2-ownership-props`, `p2-core-props` point at it, and the four own-file
  mutant cards (`p2-ownership-restore`, `p2-ownership-apply`, `p2-core-settings`, `p2-core-lid`) got one line. Answers
  done in `p2-core-lid` (8a build it; 9a whole item with `Updated`), `p2-core-settings`, `02-core.md` (note 18, table,
  Coverage, note 20 clang bullet). Next: 03 to 06 phase files and their cards.
- 2026-11-03: answers written in 03 to 06 phase files (tables, Questions sections, value notes, clang bullets, a work
  log line each) and cards `p3-recovery` (plus `p3-power-plan` in Runs after: Minor 3), `p3-haik-checks`,
  `p4-cli-restore`, `p4-codex-source`, `p4-copilot-source`, `p4-live-codex`, `p4-live-copilot`, `p5-sounds`,
  `p5-notices`, `p6-hooks`, `p6-pack-and-release`, `p6-readme`, `p6-velopack-dependency` (step 0 owns the clang
  install, Haik elevates; `velo::feed()`: Minor 5). `p5-text` words `Notice::Updated` (quiet, `silent` true; the
  `NoticeText` doc in `text.rs` now allows a quiet notice). `p5-tray-icon` (Minor 4), `p5-haik-check` (owner lines).
  03 note 3 `--locked` (Minor 6).
- 2026-11-03: re-grep found more stale "still open" lines: `architecture.md` (Question 10 twice, Question 12, the
  winget note) and `product.md` (Question 11); fixed to the answer. 02 Coverage got a row for item 8a. `p4-cli-args`
  and `p4-cli-run` now say their `proptest` and feature are declared. Line widths and dashes checked
  (`textcheck.py`). `cargo fmt --all --check` exit 0; whole-workspace clippy, test, release builds running.
- 2026-11-03: clippy (4 runs) exit 0; `cargo test --workspace --locked` exit 0 (36 tests: core 24, windows 10, cli 1,
  tray 1); x64 release build exit 0. ARM64 release build running.
- 2026-11-03: ARM64 release build exit 0. Scratch copy check of note 20 passed (`mutant_copy.ps1`: copy tests pass,
  a planted mutant fails `read_ignores_newer_files`, the repo file unchanged); copy and its build folder deleted.
  `docs/research.md` section "Close contracts checks" added. Build folder deleted. Done.

## Result

Done 2026-11-03.

### What was done

1. Update notice (sign-off item 7 yes): `Event::Updated { version }` (`event.rs`: ship track posts it, at most once,
   at start, after `Started` and `JournalCorrupt`; the core answers `Notify(Notice::Updated)`) and
   `Notice::Updated { version }` (`view.rs`). No `match` lists all variants, so nothing else changed. `text.rs` has one
   `notice(&Notice)`, so no new signature; its `NoticeText` doc now allows a quiet notice, because ship Question 3 says
   "no sound". `Cargo` changes, only Major 1's: `Win32_System_Environment` in `wakeshot-windows`, `proptest` as a
   dev-dependency of `wakeshot-cli` (both checked, research "Close contracts checks").
2. Owner lines: `text.rs`, `art.rs` add `p5-haik-check`; 13 done files name the live card that may fix them
   (`sources/claude/*`, `codex.rs`, `copilot.rs`, `cron.rs`, `holds/file.rs`, `watch/{agents,data,dir,read,thread}.rs`,
   `lockowner.rs`). The card names each live card instead of `p4-live-*`, so link checks keep working. `watch/mod.rs`
   (a module list) is left alone.
3. `architecture.md`: header; section 2 (owner claim, `install` row without the Run value); section 3 (`Updated` in
   the event block and the notice list); section 5 (`ownership::restore` through `restore_locked`); section 6 (one line:
   `store::AutoStart` is the one Run and RunOnce writer); section 9 ("Wakeshot test <pid>", "Wakeshot test e2e"). Also
   four stale "still open" lines (Phase 1 Questions 10 and 12, the winget note), and one in `product.md` (Question 11).
4. Answers: 02 to 06 phase files (tables, Questions sections, value notes, clang bullets, work log lines, 02 Coverage
   rows for the update rule and item 8a, sign-off file line 28) and 16 cards. New work went to its owner:
   `p2-core-lid` 8a (build it) and 9a (`Updated` gives `Notify(Updated)`, two named tests); `p5-text` words
   `Notice::Updated` (quiet, `silent` true); `p6-readme` states the update notice and `NotifyIconSettings`;
   `p6-velopack-dependency` step 0 owns the clang install.
5. Verify review, every finding:

| Finding | Outcome |
|---|---|
| Major 1 | Fixed: the feature and the dev-dependency; `Cargo.lock` gains one line; `p4-cli-args`, `p4-cli-run` say so. |
| Major 2 | Fixed: `02-core.md` note 20 (scratch copy with `assets\`, own build folder, `--locked`, delete after); `p2-ownership-props`, `p2-core-props` and the four own-file mutant cards point at it. Checked runnable (research). |
| Minor 1 | Fixed: every spot listed, plus `p3-haik-checks`, `p6-pack-and-release`; the clang install is `p6-velopack-dependency` step 0 (Haik runs it elevated; the worker writes the exact steps). |
| Minor 2 | Fixed: an `Owner:` line in all 17 files (plus windows `lib.rs`); `clock.rs` names `p3-system-events`, `core.rs` the three-card chain; `install.rs:1` and `architecture.md` without the Run value; the claim in both `lib.rs` and `architecture.md` changed. `CLAUDE.md` not edited (Found outside scope). |
| Minor 3 | Fixed: `p3-power-plan` in `p3-recovery`'s Runs after and the 03 table row. |
| Minor 4 | Fixed: `p5-tray-icon` no longer offers stub art. |
| Minor 5 | Fixed: `velo::feed() -> impl UpdateFeed` in `06-ship.md` and `p6-velopack-dependency`; `startup()` keeps its signature. |
| Minor 6 | Fixed: `--locked` in 03 note 3. The wrong `text.rs` path in this card: listed (Found outside scope). |
| Known open m9, `architecture.md` 612 and 943 | Fixed (items 1 and 3). M10 stays with `p6-plan-integration`. |

### Design calls

- `Notice::Updated` is quiet (`silent` true): ship Question 3 says "no sound". Change it when Haik wants the balloon
  sound.
- Clang install owner: a step 0 of `p6-velopack-dependency`, not a new card. It already runs alone, and the install
  needs Haik's elevation, so the worker writes exact steps and blocks. Change it when the install needs more than one
  command.
- Mutants for all six mutant cards run in a scratch copy, not only the two the review named: one rule, and a stopped
  session never leaves a mutant in the repo. Change it when a card needs a mutant to reach a real-laptop run.

### Files changed

- Code: `crates/wakeshot-cli/Cargo.toml`, `crates/wakeshot-windows/Cargo.toml`, `Cargo.lock`; core `event.rs`,
  `view.rs`, `text.rs`, `lib.rs`, `command.rs`, `core.rs`, `files.rs`, `holds.rs`, `process.rs`, `settings.rs`,
  `sources.rs`, `testing.rs`, `time.rs`, `holds/file.rs`, `sources/claude/{mod,status,transcript}.rs`,
  `sources/{codex,copilot,cron}.rs`; windows `lib.rs`, `clock.rs`, `error.rs`, `install.rs`, `lockowner.rs`,
  `watch/{agents,data,dir,read,thread}.rs`, `tray/art.rs`; tray `src/mode.rs`, `build.rs`; cli `build.rs`. Doc lines
  and the two variants only; no logic.
- Docs: `architecture.md`, `product.md`, `research.md` (new section); `phases/02-core.md`, `03-windows-power.md`,
  `04-sources.md`, `05-tray-ux.md`, `06-ship.md`, `02-06-signoff.md`; cards `p2-core-lid`, `p2-core-settings`,
  `p2-core-props`, `p2-ownership-props`, `p2-ownership-restore`, `p2-ownership-apply`, `p3-recovery`,
  `p3-haik-checks`, `p4-cli-args`, `p4-cli-run`, `p4-cli-restore`, `p4-codex-source`, `p4-copilot-source`,
  `p4-live-codex`, `p4-live-copilot`, `p5-sounds`, `p5-notices`, `p5-text`, `p5-tray-icon`, `p5-haik-check`,
  `p6-hooks`, `p6-pack-and-release`, `p6-readme`, `p6-velopack-dependency`, and this card.

### Checks

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings`, x64 and ARM64 | exit 0, both |
| the same without `--all-targets`, x64 and ARM64 | exit 0, both |
| `cargo test --workspace --locked` | exit 0, 36 tests (core 24, windows 10, cli 1, tray 1) |
| `cargo build --release --workspace --locked --target <t>`, x64 and ARM64 | exit 0, both |
| `Cargo.lock` | one line added: ` "proptest",` in the `wakeshot-cli` entry; no new package, no version change (`cargo metadata --offline`, then diff against a copy taken first); every check above ran with `--locked` |
| Each of the 12 questions | grep for question, answer, "if Haik", "until he", "still open", "may add" and similar over 02 to 06 and the p2 to p6 build cards: no card or phase file waits on one. Left: the questions' own text in `02-06-signoff.md` (answered at line 27) |
| Verify review findings | all fixed or listed (table above) |
| Dashes, line width | no en or em dash in the changed files; no new line over 120 characters outside tables (`textcheck.py`) |
| Build folder | deleted; the scratch copy and its build folder too |

Open items: the `CLAUDE.md` owner claim (Found outside scope). Slip: one PowerShell line ran `git diff --no-index
--stat` by mistake (exit 129, usage error, changed nothing). Scripts in the scratchpad `p1-close-contracts\`.
