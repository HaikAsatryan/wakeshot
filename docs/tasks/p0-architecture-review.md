# Review the design against the Phase 0 facts

Phase: 0. Status: ready. Needs Haik: no. Agent: `wakeshot-reviewer`.
Runs after: all other Phase 0 cards. Parallel with: none.

## Goal

A fresh, critical read of the product and architecture docs before Phase 1 builds on them.

## Read first

- `CLAUDE.md`
- `docs/product.md`, `docs/architecture.md`
- `docs/research.md` and the Result sections of all `docs/tasks/p0-*.md` cards
- Reports in `docs/research/` only where a finding needs the detail

## Brief

Find:
- Decisions the facts do not support, or facts that change a decision.
- Failure modes with no answer: for each input and each command, what if it fails, is late, or lies? Special focus:
  any path where Haik's power settings stay changed, or a closed laptop keeps running without a limit.
- Places where the design is more complex than the job needs (KISS).
- Contradictions between docs.

## Deliverables

`docs/reviews/p0-architecture.md`, most severe first.

## Acceptance checks

Each finding cites the doc line or fact it rests on.

## Out of scope

Editing the docs. The orchestrator turns findings into decisions with Haik.

## Log

- 2026-11-03: card written.

## Result
