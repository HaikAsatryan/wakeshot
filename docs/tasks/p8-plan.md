# Plan the repo polish for launch (Phase 8)

Phase: 8. Status: done (new 2026-11-07). Needs Haik: no (his go for the night, below; decide and log "for Haik's
review").
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: none. Parallel with: every card; this card writes only planning files and builds nothing.
Build folder: none.

## Goal

Haik, 2026-11-07 ~02:39, his go for the night: "you can do cleanup, you can even do code cleanup session if needed
(names, structure, places, comments (I mean deleting idiotic wordy comments). Like full polish of the repo so it would
not look like fully vibe coded repo. Structured and following best practices repo, with good page and so on."
Haik, ~02:40: yes to the Phase 8 history squash once the polish is done and every check and CI is green.

Turn that into a phase file and task cards the orchestrator can run tonight, at most four building cards at a time,
each owning its own files, so the whole polish finishes in about two rounds.

## Read first

- `CLAUDE.md`, `docs/plan.md` (How we work, Phases row 8), `docs/tasks/p8-cleanup.md` (brand history list),
  `docs/tasks/p8-store.md` header.
- The repo tree (`Get-ChildItem`, `Glob`), each crate's module list and a sample of files per crate, enough to size
  the work. Read samples, not every file.
- Haik's global rules on comments and docs: comments say why, never what; a CLAUDE.md is a short cache of facts.

## Deliverables

1. `docs/phases/08-launch-cleanup.md`: the design (what stays, what goes, what gets renamed or moved, and why), a card
   table (card, needs Haik, runs after, status), the file ownership table, and an empty work log.
2. Task cards `docs/tasks/p8-*.md` from `docs/tasks/_template.md`. Expected shape, change it if the code says so:
   - Code polish, split by crate or module group so no two cards own one file: names (Rust API guidelines), file
     places, dead code, comments (delete wordy and "what" comments, keep short "why" comments; no internal card codes,
     session names, review finding numbers or "Haik said" in code that stays), test layout. No behavior change; the
     tests prove it. Contracts and every `Cargo.toml` and `Cargo.lock` change only in one card that runs alone.
   - Repo hygiene: Cargo metadata, `.github/` workflows tidy, `.gitignore`, `.gitattributes`, editor and lint config,
     `packaging/`, `media/` and `site/` scripts in the right places, best-practice files that are missing.
   - Docs: which docs stay for contributors (likely `docs/architecture.md`, `docs/product.md`, `docs/brand.md`,
     `docs/research.md` and some of `docs/research/`), each rewritten without internal planning history; the brand
     history in `p8-cleanup.md`; links that would break; `spikes/` (keep or remove, with the reason).
   - The last card, run alone right before the squash: remove internal planning (`docs/plan.md`, `docs/phases/`,
     `docs/tasks/`, `docs/reviews/`, and `.claude/` agents and skills unless one skill really helps contributors),
     write the short `CLAUDE.md`, and fix every pointer into removed files. The orchestrator archives everything
     removed outside the repo first, so nothing is lost.
   - README: Haik approved it (2026-11-06 and 2026-11-07). Only stale facts and pointers into removed files change;
     no rewrite, no new sections, his "Why I built it" and "What comes next" stay as written.
3. Each card: goal, read first, deliverables, acceptance checks (`cargo fmt`, clippy and tests on its own package
   while others run; the orchestrator runs the whole-workspace checks on x64 and ARM64 when a round closes), out of
   scope, rules (below), size.
4. In the Result: the card list with sizes, the order and which run in parallel, and every call you made for Haik's
   review.

## Rules for the cards you write

- Workers never run git, not even read-only. Never read token files. Never set `RUSTFLAGS`. Plain short English, no
  em dashes, no emojis. No mention anywhere of the product the project memory names. Haik's style reference videos
  stay out of the repo.
- No behavior change in a polish card. A bug found goes to "Found outside scope", not into a quiet fix.
- The video, its end card and Haik's approved README text are not polish targets.

## Out of scope

Doing the polish itself. Git. The squash. The winget submission and the first release (Haik's).

## Questions for Haik

None tonight: decide and log "for Haik's review".

## Found outside scope

1. `docs/plan.md`, Phases row 8, says the removal list is in `docs/tasks/p8-cleanup.md`; it now lives in
   `docs/phases/08-launch-cleanup.md` ("What stays and what goes"). The orchestrator's file.
2. `media/scripts/make-sounds.mjs` reads `crates/wakeshot-windows/src/sound/chime.rs` with regular expressions, so a
   comment edit inside a `Shape` block can break the media build. Handled for tonight by a limit in
   `p8-polish-windows-tray`; the coupling itself stays (after v1: read the numbers from a data file both use).
3. `.claude/scheduled_tasks.lock` is not in `.gitignore` and was not in the start-of-session untracked list, so it is
   probably tracked (not verified: no git). `p8-repo-hygiene` adds the ignore line; untracking it is a git step for the
   orchestrator.
4. `crates/wakeshot-windows/examples/agent-watch.rs` is the only example with a hyphen in its name (the others use
   `_`). Kept, since research docs cite its command; a rename after v1.
5. This card's own Rules named the product the project memory says never to name; reworded in place (04:15).

## Log

- 2026-11-07 03:50: card written by the orchestrator (session wakeshot-4e).
- 2026-11-07 03:47 (clock time from `Get-Date`; the line above used the orchestrator's estimate): started by a
  `wakeshot-worker`. Reading the repo tree, crates and docs to size the polish.
- 2026-11-07 04:02: sized: 131.8K lines of code and scripts outside `media/`'s `node_modules`, 14.7K comment lines
  (about 13.3K in the crates), about 450 card codes, 240 `Owner:` lines and 400 doc pointers in the crates; docs that
  stay about 1.5 MB (`research.md` 568 KB). (Phase file numbers are the exact ones: about 430 lines with card codes.)
  Wrote and self-tested a comment-stripping compare script for the code
  cards (it goes in the phase file). Writing the phase file and 16 cards next.
- 2026-11-07 04:07: phase file `docs/phases/08-launch-cleanup.md` written; the eight `p8-polish-*` code cards written
  from one template. Found while sizing: `media/scripts/make-sounds.mjs` parses `sound/chime.rs` with regular
  expressions, and `art.rs` writes files others compare, so the tray card carries both limits. Next: the docs,
  hygiene, config and final cards, then `p8-cleanup.md`.
- 2026-11-07 04:15: all cards written; `p8-cleanup.md` filled as the brand history card. Checks run (Result). No
  `docs/research.md` addition: the facts checked here are planning facts, recorded with how they were measured in the
  phase file, "Facts behind the plan". Status done.

## Result

Done 2026-11-07 04:15. Phase file `docs/phases/08-launch-cleanup.md`: the design (the repo after Phase 8, what is
removed and why, what is rewritten by which card, the pointer rules), the polish rules for code and docs, the checks,
a tested no-code-change script, the card table, the order, the file ownership table, 15 calls for Haik's review, the
sizing facts, and an empty work log.

Cards (estimates; real use runs 2 to 3 times higher; about 2.3M in all):

| Group | Card | Size | Owns |
|---|---|---|---|
| 1, building | `p8-polish-core-engine` | 150K | `wakeshot-core`: state machine, policies, events, commands, files, text, `lib.rs` |
| 1, building | `p8-polish-core-sources` | 150K | `wakeshot-core`: `sources/`, `tests/` |
| 1, building | `p8-polish-windows-power` | 150K | `wakeshot-windows`: `power/`, recovery, process, session, `lib.rs`, `power_check` |
| 1, building | `p8-polish-windows-watch` | 140K | `wakeshot-windows`: `watch/`, `agent_hooks.rs`, `sqlite.rs`, `lockowner.rs`, `agent-watch` |
| 1, beside | `p8-docs-design` | 150K | `architecture.md`, `product.md`, `hold-files.md` |
| 1, beside | `p8-docs-research-reports` | 140K | `docs/research/*.md` (not brand), `agent-hooks.md` |
| 1, beside | `p8-docs-research-1` | 150K | `research.md`, top through "Update thread checks" |
| 1, beside | `p8-cleanup` | 90K | `brand.md`, the six brand reports, old-art pictures |
| 2, building | `p8-polish-core-ownership` | 150K | `wakeshot-core`: `ownership/`, `sim.rs`, `testing.rs`, `scenarios/` |
| 2, building | `p8-polish-windows-tray` | 140K | `wakeshot-windows`: `tray/`, `notify.rs`, `sound/`, `ux_*` |
| 2, building | `p8-polish-windows-app` | 150K | `wakeshot-windows`: `app/`, store, log, update, install, hooks, new top-level files |
| 2, building | `p8-polish-bins` | 120K | `wakeshot-cli` and `wakeshot-tray` sources and tests |
| 2, beside | `p8-docs-research-2` | 170K | `research.md`, the rest, then topic groups and contents (after part 1 and `p6-end-card-numbers`) |
| 2, beside | `p8-repo-hygiene` | 140K | `.github/`, `packaging/`, `site/`, community files, `.gitignore`, `.gitattributes`, `.editorconfig` |
| alone (building) | `p8-repo-config` | 110K | Cargo and build files, `spikes/`, listed cross-crate renames, whole-workspace checks |
| alone, last | `p8-final-cleanup` | 160K | planning removal, short `CLAUDE.md`, every pointer (README and `media/` pointers only) |

Order: group 1 starts when `p6-logs-diagnostics` is done (three of its docs cards could start earlier); group 2 fills
slots as they free; the orchestrator runs whole-workspace checks after each group's building cards; then
`p8-repo-config` alone among building cards; then the history bundle and `p8-final-cleanup` alone, after every other
card of the night (`p4-live-uninstall` and the video upload included); then push, CI, a release dry run, the squash.

Calls for Haik's review (phase file, numbered there): `spikes/` removed; no Claude skill or agent stays; `research.md`
stays one file, grouped by topic; old-brand pictures removed without new captures (the notice colours stay, marked);
the unwired Cursor, Antigravity and VS Code Local chat code stays; cross-crate renames capped at about 10; big test
modules may move to `tests.rs`; Dependabot monthly for Actions and Cargo; no CHANGELOG; `publish = false`; machine notes
leave `CLAUDE.md`; `media/` only pointer-fixed; README pointer fixes only (phase-named research anchors renamed with
it); `Owner:` lines go; no behavior change proven by the compare script, equal test lists and the CI release dry run.

`p8-cleanup.md`: not folded away; it became the brand history card (Read first, Deliverables, checks, scope, and how
Phase 8 splits its list across cards). Its list stays the source.

Checks run:

- Ownership coverage (a script applying the ownership table to every file under `crates/`): 329 files, 0 unowned, 0
  owned twice.
- The no-code-change script, extracted from the phase file as written and run on a copy of `crates/`: comments only
  gave "no code change"; a changed type, a changed string letter and a removed file were each reported.
- All Phase 8 planning files: no em dash, no line over 120 columns outside tables, no forbidden name; every card has
  the template's sections.

Files created: `docs/phases/08-launch-cleanup.md`; `docs/tasks/p8-polish-core-engine.md`,
`p8-polish-core-ownership.md`, `p8-polish-core-sources.md`, `p8-polish-windows-power.md`, `p8-polish-windows-tray.md`,
`p8-polish-windows-app.md`, `p8-polish-windows-watch.md`, `p8-polish-bins.md`, `p8-docs-design.md`,
`p8-docs-research-reports.md`, `p8-docs-research-1.md`, `p8-docs-research-2.md`, `p8-repo-hygiene.md`,
`p8-repo-config.md`, `p8-final-cleanup.md`. Changed: `docs/tasks/p8-cleanup.md`, `docs/tasks/p8-plan.md`.

Open for the orchestrator: the `docs/plan.md` row 8 pointer (Found outside scope 1); untrack
`.claude/scheduled_tasks.lock` at commit if tracked (3); name the archive folder `p8-final-cleanup` writes machine notes
to; make the history bundle right before `p8-final-cleanup`; after it, `/orchestrate` and the agent files load only
from the archive.
