# Fix the privacy findings before the first public push

Phase: 1. Status: dropped (2026-11-03, `p1-design-review-fixes`), with `p1-privacy-check`. Haik's answer 1 (15:29)
makes the repo fully public; a real secret found by the orchestrator's secret scan gets its own small fix card.
Kept for the record only; do not run it. Agent: `wakeshot-worker`.
Runs after: `p1-privacy-check`, and Haik's answers to its "Haik decides" items (or his "take the defaults").
Parallel with: none that writes `docs/`, `spikes/`, `.claude/` or `CLAUDE.md`. This card may edit any file on the
list, so `p1-lid-sound` and `p1-tray-menu-look` must be done or paused first.

## Goal

Make every file on the public list safe to publish, without losing a fact the docs rest on.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/reviews/p1-privacy.md`: every finding, the grep patterns, and Haik's answers.
- Each file a finding names.

## Deliverables

1. Every **must fix** item fixed, and every **Haik decides** item done as Haik answered. Ways to fix, in this order:
   - Replace an identifier with a neutral placeholder that keeps the meaning: `C:\Users\<user>`, `<computer>`,
     `<user SID>`, `<session id>`. Keep the fact; drop the identifier.
   - Remove a line that is private and carries no fact the design needs.
   - Keep a whole file local (add it to `.gitignore` with a comment why), when it cannot be cleaned. Then mark each
     doc line that cites it "(local file, not in the repo)".
   - Images: never edit pixels by hand. If an image shows private content, keep it local as above, unless Haik
     decided to crop or blur it; then do it with a script you keep in your scratch folder, and view the result.
2. The review is the reviewer's file: do not edit it. Write the fix for each finding in this card's Log (finding,
   file, what changed).

## Acceptance checks

- Re-run every grep pattern from the review over the public list: zero hits, except hits the review marked fine
  (list them).
- Every changed image viewed after the change.
- No doc lost a fact: for each removed line, the Log says why nothing the design rests on went with it.
- The reviewer then re-checks the fixes (no card; the orchestrator starts it).

## Out of scope

- Git of any kind (the orchestrator re-lists the files with git after this card).
- Wording or content changes not needed for privacy. `docs/product.md` unless a finding names it.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p1-detailed-design`.

## Result

What was done, files changed, checks and outcomes, open items.
