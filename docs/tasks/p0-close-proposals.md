# Draft the Phase 0 close: proposed doc changes and Haik's decisions

Phase: 0. Status: done. Needs Haik: no (he answers the result later). Agent: `wakeshot-worker`.
Runs after: the early architecture review. Parallel with: none needed.

## Goal

Turn the review findings into one short list Haik can answer, so the Phase 0 close takes minutes, not hours.

## Read first

- `docs/reviews/p0-architecture.md` (all findings).
- `docs/research/claude-code-signals.md`: the ten proposed rule changes only.
- Results of `docs/tasks/p0-unelevated-liveness.md` and `docs/tasks/p0-distribution-facts.md`.
- `docs/product.md` and `docs/architecture.md`, to word each change against the current text.
- `docs/phases/00-facts.md` work log lines marked "for the close".

## Deliverables

`docs/reviews/p0-close-proposals.md` with three parts:

1. **Decisions for Haik**: only product, money or hard-to-undo choices. Each: the question in one or two plain
   sentences, the worker's default with the reason, and the one concrete consequence of the other choice. Numbered,
   most important first. Aim for as few as possible.
2. **Changes the orchestrator can apply after Haik's yes**: each a concrete edit (file, section, old idea, new idea)
   that follows from facts, not from taste. Group by file.
3. **Waits for the morning results**: findings that depend on a step in `docs/phases/00-morning-steps.md`, with the
   step.

## Acceptance checks

- Every architecture review finding and every proposed Claude rule appears in exactly one part, or is listed as
  "dropped" with a reason.
- Each item cites the review finding number or doc line it comes from.

## Out of scope

Editing `product.md`, `architecture.md`, `research.md` or any review file. Running spikes. Power settings.

## Questions for Haik

None that block this card. The two decisions for Haik are in part 1 of the deliverable.

## Found outside scope

- `CLAUDE.md:7` says "No code, no git repo yet". Spike code exists in `spikes/`, so "No code" is stale. (This
  session's environment reports no git repo, so that half still holds.)
- `docs/phases/00-facts.md:71` refers to "product.md Open 3 and 6". `product.md` now has only Open 1 and 2. It is a
  log line, so it was left as history; the close proposals remove both Open items anyway.
- The review's `research.md` line numbers after line 82 are about 50 lower than the file today (the unelevated
  section was added after the review). The proposals cite current lines and say so.

## Log

- 2026-11-03: card written by the orchestrator.
- 2026-11-03: worker started. Reading the review, the ten Claude rules, the two task results, product and architecture.
- 2026-11-03 05:20: inputs read. Plan: 2 decisions (docked time limit, heat guard), about 25 doc edits, 8 morning
  items, and a coverage table. The review's `research.md` line numbers after line 82 are about 50 lower than today;
  the proposals cite current lines. Fixed decisions kept where no fact contradicts them (docked lock rule, public hold
  file format).
- 2026-11-03 05:35: `docs/reviews/p0-close-proposals.md` written. Cited lines checked against the current files.
  Acceptance checks run with an awk count over the file (results in Result). Card done.

## Result

Done. Deliverable: `docs/reviews/p0-close-proposals.md`.

What it holds:

1. **Decisions for Haik: 2.** (1) Does the lid-closed time limit apply to a docked laptop? Default no: only with the
   lid closed and no external screen. (2) Add a heat guard? Default yes, lean: lid closed, no external screen, zone
   above one fixed value for a few minutes, then hibernate; no setting; value from morning step I.
2. **Changes after Haik's yes: 25 items**, grouped by file: `product.md` 11, `architecture.md` 9,
   `claude-code-signals.md` 1 (a table of rule edits that adopts all ten changes, two of them as "dropped for v1"),
   `CLAUDE.md` 2, `00-facts.md` 1, and one new-files item (the live-test fixtures, review test 7, not done yet).
   Open 1 and Open 2 are decided there, because Haik handed them over at `product.md:114`.
3. **Waits for the morning results: 8 items**, each with its step (battery timer C and I, hibernate fallback F and
   E, idle lock H, plan copy B, clock E to I, sound D E I, lock and re-take E H I, screen off E to I).

Not adopted, with the reason in the file: the review's "an external screen means docked, so no lock" and "keep the
hold format internal in v1". Both contradict fixed `product.md` decisions, and no fact contradicts those.

Files changed: `docs/reviews/p0-close-proposals.md` (new), this card.

Acceptance checks:

- Every review finding and every proposed Claude rule in exactly one part, or dropped with a reason: pass. An awk
  count over the file shows findings 1 to 22 each in one part. Finding 23 is nine separate doc fixes; eight are in
  part 2 and the clock bullet is in part 3, each listed on its own row in the coverage table. All ten Claude changes
  are in part 2 (changes 6 and 7 as "dropped for v1"). Review tests 1 to 7 are listed too (six done earlier, test 7
  open).
- Each item cites the review finding number or doc line: pass. Every part 1, 2 and 3 item names its finding, test
  or doc line.

Open items: the fixtures card (part 2 item 25) should run before anyone deletes the `cctest*` transcript folders.
