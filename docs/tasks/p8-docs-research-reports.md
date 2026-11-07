# Clean the research reports and the agent hooks doc

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: `p6-logs-diagnostics`. Parallel with: every card that does not own these files (group 1; no build). Open
cards (`p4-live-claude-desktop`, the winget research) may still append to a report: re-read before each edit.
Build folder: none.

## Goal

The longer research reports keep their facts and sources and lose the planning history: a reader learns what each
agent writes, how Velopack and winget behave, what the rivals do, how the tray menu should look, and how each was
checked, without card codes, sessions, "Haik said" or questions that were answered long ago.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership".
- Each owned file's headings, then the lines the internal-name sweep hits, with enough context to rewrite them. Read a
  section whole when you decide whether it is process only. About 450 KB in all: do not read every line.
- What points at them: grep the repo for `research/<name>.md` and for `agent-hooks.md`. The README links to five
  anchors: `launch.md#1-rivals-for-the-readme-table`, `claude-code-signals.md#phase-4-end-to-end-2026-11-04`,
  `other-agent-signals.md#phase-4-live-test-2026-11-06` and `#phase-4-live-test-2026-11-06-1`,
  `copilot-vscode-signals.md#stop-approve-and-window-close-live-test-2026-11-06`,
  `opencode-signals.md#phase-4-live-test-2026-11-06`.

## Owned files

- `docs/research/`: `antigravity-signals.md`, `claude-code-signals.md`, `copilot-vscode-signals.md`,
  `cursor-gemini-signals.md`, `distribution.md`, `launch.md`, `opencode-signals.md`, `other-agent-signals.md`,
  `promo-video.md`, `readme.md`, `rivals-lidguard.md`, `rivals-others.md`, `tray-menu-look.md`.
- `docs/agent-hooks.md`.
- Not the six `brand-*.md` reports (`p8-cleanup`), not `docs/research.md`, not the pictures.

## Deliverables

1. Each owned file cleaned by rules D1 to D6: no card codes, sessions, "Haik said", "for Haik's review", pointers
   into removed paths; facts keep their source and date.
2. Cut sections that are process only: "Questions for Haik" that are answered, "Effort and risks" of finished work,
   "How to brief the agent", "Rules for `<card>`", drafts that the README or the code replaced. Keep a section that
   holds a fact the code, the README or a kept doc relies on.
3. Headings named after a phase or a card ("Phase 4 live test (2026-11-06)", "Rules for `p6-winget-manifest`") get a
   plain name ("Live test (2026-11-06)"). Record every rename, with the old and new GitHub anchor, for
   `p8-final-cleanup` (it fixes the README and code pointers). Keep the date in a live test's heading.
4. `promo-video.md` and `readme.md` record how the video and the README were made. Keep the facts others rely on
   (music and font licences, the agent logo rules, what GitHub renders); cut the plan. If nothing of a report is
   left that anyone relies on, say so in the Result: the orchestrator decides whether to remove it. Haik's style
   reference videos must not appear (rule D6).
5. `docs/agent-hooks.md`: the hook design for the agents out of v1 (their code stays, unwired). Keep "What Wakeshot
   adds", the Cursor and Antigravity design, "Adding", "Removing", "Failure cases", "Privacy" and the facts; cut
   "Contract changes the build needs", "Live test (for the live card)" and the review-risk list if the code and the
   security review already settled them (keep any risk that is still open, as a plain "Known risks" list). Code cites
   its headings "Adding", "Antigravity", "Cursor", "Events and states", "Privacy", "README lines", "Removing", "The
   folder", "The hook command", "The files it writes": keep them or record the rename.
6. Result: "Heading renames" (file, old heading, old anchor, new heading, new anchor); per file the sections cut and
   why; reports you suggest removing; sizes before and after.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on the owned files: zero hits.
2. No em dash (U+2014) in the owned files.
3. Every anchor or heading that a file outside this card cites still exists, or is in "Heading renames". Show the grep.
4. Every relative link and picture path in the owned files resolves to a file that stays (not a removed path or a
   brand report; check `docs/research/img/` paths exist).
5. Every "not verified" or "not tested" mark that was there is still there (count before and after).

## Out of scope

`README.md` (its anchors are fixed by `p8-final-cleanup`), code, `media/`, `docs/research.md`, the brand reports,
`docs/research/img/`. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
