---
name: orchestrate
description: Run Wakeshot work as the orchestrator. Use after every /clear.
---

# Wakeshot orchestrator

You plan and coordinate. Workers do the task work. You never do task work yourself. Haik clears you often, so all
state lives in files. The source of truth is `docs/plan.md` "How we work"; if this file and the plan disagree, follow
the plan and fix this file.

## Each run

1. Read `CLAUDE.md`, `docs/plan.md` (How we work, Status) and the current phase file in `docs/phases/` (card table
   and work log).
2. From the task cards in `docs/tasks/`, read only the header lines: Status, Needs Haik, Runs after, Parallel with.
   Read a card's Result section only when the card is done or blocked.
3. Pick the next cards: Status `ready` and every "Runs after" card `done`. Cards that touch the same files or the same
   real power settings run one at a time. Run cards in parallel only where their "Parallel with" lines allow it.
4. Spawn one `wakeshot-worker` agent per card, or per bundle (the "Bundles" line in the phase file: one worker, both
   cards in order, once both cards' Runs after are met), in the background. Never start a card with Status `later`.
   The prompt names the card path (or both, in order) and adds any answer or context from Haik that the card needs,
   and the files it owns this round. No worktrees in this repo: parallel
   cards own different files (`docs/plan.md`, "Git"). Never ask agents to inspect the tree with git.
5. When an agent ends, read its short final reply and the card's Result. Update the card table in the phase file.
6. Blocked cards: relay the card's "Questions for Haik" and its manual steps (lid, battery, theme switch) to Haik in
   chat. Copy the steps exactly. When he answers, resume the card: SendMessage to the same agent if it is still
   available, otherwise a new worker that reads the card's Log.
7. Reviews: only the two in `docs/phases/02-06-lighter.md` ("Reviews"), each one `wakeshot-reviewer` with its narrow
   brief, at the point named there. Findings go to `docs/reviews/`. The worker that built the card fixes them
   (SendMessage, or a new worker that reads the card's Log); the same reviewer checks the fixes of blockers and majors
   only. No phase-end review loop, and no plan, close or fix cards.
8. Before you stop: add dated lines to the phase file work log and update the Status section of `docs/plan.md`.

## Git

- Start with `git pull --rebase` once the first push has happened (before it, `main` has no upstream: follow the git
  steps in the Phase 1 file). If the tree is dirty, the branch has no upstream, or the rebase conflicts, abort and
  tell Haik.
- Commit after each finished card and after review fixes. Only Haik's name: no AI mention, no co-author lines, no
  "Generated with" line.
- Push after a phase passes review.
- Never force push without asking Haik. The history squash before launch needs his go right before the force push.
- Workers and reviewers never run git.

## Rules

- Haik's global CLAUDE.md applies. Plain short English. No em dashes, no emojis.
- Ask questions as a short numbered list in chat, each with your default. Never use the AskUserQuestion tool. Keep
  working on whatever does not depend on the answer.
- Keep your context small: read agents' final replies and card Result sections, not transcripts or whole reports,
  unless a decision depends on them.
- Power settings on the test laptop are Haik's real settings. Before any card that changes them, make sure Nightcrew
  is uninstalled (see `CLAUDE.md`, Test laptop). If it still runs, ask Haik to quit it from its tray. Never kill it
  by name.
- At about 150K to 250K tokens, update the phase file and the plan Status, then stop at a clean point.

## End every run with

1. What is left.
2. What blocks it.
3. How many agents in how many rounds will finish it.
4. Haik's actions, in order.
