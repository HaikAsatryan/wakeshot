# Rewrite the design docs without planning history

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics` (it may change a contract in `architecture.md`). Parallel with: every card that does
not own these files (group 1; no build). If `p4-live-claude-desktop` is still open, its later edits to these files are
additions that `p8-final-cleanup` sweeps.
Build folder: none.

## Goal

`docs/architecture.md`, `docs/product.md` and `docs/hold-files.md` read as the design, the product rules and the hold
file format of a finished v1: what is true and why, with no trace of cards, tracks, phases as process, sessions or who
said what when.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership".
- The three files, whole. `architecture.md` is about 1,580 lines: read it in parts.
- What points at them, before you rename anything: grep `crates/`, `packaging/`, `.github/`, `site/`, `media/`,
  `README.md`, `CONTRIBUTING.md` and the docs that stay for `architecture`, `product` and `hold-files`.

## Deliverables

1. `docs/architecture.md`:
   - The opening paragraph: what the doc is and how "not tested" is marked; no card names, no sign-off story.
   - Remove card codes, track ownership, `Owner:` explanations, "from `p1-contracts`", "the orchestrator runs alone",
     phase references as process, Haik attributions and dated decision notes. Keep the design and its reasons, and
     every "change it when".
   - Keep the section numbers 1 to 12, their subsection names, the "Tray startup" step numbers and the decision
     numbers 0 to 44 in "Decisions and where they live" (code cites all of them). In that table, drop the Phase
     column and the attributions; keep the decision text and where it lives.
   - `spikes/` is removed in Phase 8 (`p8-repo-config`): section 2's workspace note and "Ported from the spikes" keep
     what was ported and why, without pointers into `spikes/`.
   - Section 9 (Testing) and section 10 (CI): describe the tests and checks as they are, not which card wrote them.
     The manual checklist stays if it is a real release checklist; drop "Haik's" in its name.
2. `docs/product.md`:
   - Decisions as present-tense rules. Drop "(Haik, date)", "for Haik's review", "decided by the worker", "on
     2026-11-03 v1 was ...", and pointers into task cards and phases.
   - "Answers to the track plan questions": fold each answer into its topic section, or keep the section under a plain
     name such as "Detailed rules"; the README and code cite no part of it by name today (check). Record the rename.
   - Keep every section name and the "User settings safety" item numbers (code cites them).
   - The agents out of v1 (VS Code Local chats, Antigravity, Cursor) keep their one-line reason and "revisit when".
3. `docs/hold-files.md`: check it is current against `crates/wakeshot-core/src/holds/file.rs` and the CLI's `run`;
   fix only what is stale (it is user-facing and the README links to it).
4. Result: "Heading renames" (file, old heading, new heading), even if empty; sizes before and after; the decisions you
   folded or merged; anything a code comment cites that you could not keep.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on the three files: zero hits.
2. No em dash (U+2014) in the three files.
3. Every heading, section number, step number, decision number and "User settings safety" number that a file outside
   these three cites still exists, or is in "Heading renames". Show the grep you used.
4. Every relative link in the three files resolves to a file that stays (not `docs/plan.md`, `docs/phases/`,
   `docs/tasks/`, `docs/reviews/`, `spikes/`, a brand report).
5. No decision lost: list the `product.md` decisions before (one line each) and check each against the new text.

## Out of scope

Code, `README.md`, `media/`, `docs/research.md` and `docs/research/`, `docs/brand.md`, `docs/agent-hooks.md`. Git.
A wrong fact you find in a doc you do not own goes under "Found outside scope".

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
