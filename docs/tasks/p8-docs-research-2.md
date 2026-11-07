# Clean research.md, part 2, and group the whole file by topic

Phase: 8. Status: waits for `p8-docs-research-1` and `p6-end-card-numbers` (new 2026-11-07). Needs Haik: no (his go of
2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 170K tokens.
Runs after: `p8-docs-research-1`, `p6-end-card-numbers` (it writes the speed numbers section). The orchestrator starts
it when no running card still appends to `research.md`, or names the ones that do. Parallel with: every card that does
not own `docs/research.md` (group 2; no build).
Build folder: none.

## Goal

The second half of `docs/research.md` gets the same cleaning as the first. Then the whole file is grouped by topic,
with a short intro and a contents list, so a reader finds the fact they need.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership", calls 1, 3 and 4.
- `docs/tasks/p8-docs-research-1.md`, Result (its heading renames and removed sections).
- `docs/tasks/p8-cleanup.md`: the old-brand reports and pictures it removes, and its Result.
- Your range: from the section "Tray UX: tray icon" to the end of the file (about 3,200 lines, 290 KB today, plus what
  other cards appended). Read it in parts.
- What points at your sections: grep the repo outside `docs/tasks/`, `docs/reviews/`, `docs/phases/` for
  `research.md` (the README and `site/` link `#speed-numbers-final-v1-build-2026-11-07`; `media/` cites "Speed
  numbers").

## Deliverables

1. Every section in your range cleaned exactly as `p8-docs-research-1.md` deliverables 1 to 3 say (same rules, same
   records).
2. Old brand (`p8-cleanup` removes the reports and pictures): "Tray UX: tray icon", "Tray UX: menu on screen", "Tray
   UX: notices" and "Integration dry-run e2e" cite old-art pictures. Remove the picture links, keep the facts, and say
   the captures are not kept because they show the old icon. The notice header colours in "Brand apply: the held light
   in art.rs" stay, marked "measured on 2026-11-04 captures, not kept (they show the old icon)" (call 4). Pointers to
   the brand reports go to `docs/brand.md` or become the fact.
3. "Integration checklist with Haik" and its preparation become "Manual checklist on the test laptop"; "Tray UX:
   Haik's check" becomes a plain name. Record the renames.
4. Group the whole file by topic, as the last step, in one scripted pass right after a fresh read:
   - Topics as `##` headings and today's sections as `###` under them. The heading level does not change a GitHub
     anchor, so pointers keep working. A starting set, change it if the sections say so: The problem; Windows power;
     Agents; Tray, menu, notices and sounds; Files, processes and the log; Install, updates and release; Build, CI and
     toolchain; Rivals and launch; Brand; Speed.
   - A short intro at the top: what the file is, how a fact is marked (tested here, with the date; from a source, with
     its date; "not verified"), and that longer reports live in `docs/research/`.
   - A contents list of the topics, each linking its anchor.
   - Proof that the pass only moved text: the sorted list of non-heading lines before the pass equals the one after,
     minus the intro and contents lines you added.
   - No two headings with the same text (GitHub numbers repeated anchors in order, so a move would shift `-1` and
     `-2`), and no topic heading equal to a section heading.
5. Sections another card appended after you started: clean them too if their card is done; otherwise leave them, list
   them in the Result, and `p8-final-cleanup` cleans them.
6. Result: "Heading renames" (yours; part 1's stay valid since the grouping keeps anchors); removed sections and where
   their facts went; the topic list; size of the file before and after.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on the whole file: zero hits, except sections listed under
   deliverable 5.
2. No em dash (U+2014) in the file.
3. Every section name or anchor that a file outside `docs/tasks/`, `docs/reviews/`, `docs/phases/` cites still exists,
   or is in part 1's or your "Heading renames". Show the grep.
4. The grouping pass only moved text, and no heading text repeats (deliverable 4, last two points).
5. Every relative link and picture path in the file resolves to a file that stays (after `p8-cleanup`'s removals).
6. "not verified" and "not tested" counts: none lost outside removed sections, each listed.

## Out of scope

Every other file, code, the README, `media/`, `site/`. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
