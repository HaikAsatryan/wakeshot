# Clean research.md, part 1: from the top through "Update thread checks"

Phase: 8. Status: ready (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics`. Parallel with: every card that does not own `docs/research.md` (group 1; no build).
Never at the same time as `p8-docs-research-2`. Other open cards may append sections at the end of the file while you
work: they are not yours; re-read the part you edit right before each edit.
Build folder: none.

## Goal

The first half of `docs/research.md` keeps every fact and how it was checked, and loses the planning history: each
section is named by its fact, not by the card that checked it, and points at nothing that Phase 8 removes.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership", call 1 (`spikes/` goes) and call 3 (`research.md` stays one file).
- Your range of `docs/research.md`: from the top through the section "Update thread checks" (about lines 1 to 2,990
  today, 270 KB). Read it in parts, section by section.
- What points at your sections: grep the repo outside `docs/tasks/`, `docs/reviews/` and `docs/phases/` for
  `research.md` (code cites sections by quoted name, for example `docs/research.md` "Process module checks"; the README
  links `#windows-power-facts` and `#story-checks-powertoys-awake-and-claudes-own-keep-awake`).

## Deliverables

1. Every section in your range cleaned by rules D1 to D6:
   - No card codes, sessions, "for card `docs/tasks/...`", "Haik said", pointers into `docs/plan.md`, `docs/phases/`,
     `docs/tasks/`, `docs/reviews/`. Keep the date, the machine ("the test laptop", Windows build), the command or test
     that checked it, the source and its date, and every "not verified" or "not tested" mark.
   - Pointers into `spikes/` (removed by `p8-repo-config`): keep what was run and what it showed; say "a throwaway
     test program (not kept in the repo)" instead of the path.
   - Tests in the crates stay as evidence by name (`store::tests`, `atomic_write_replaces_a_file_a_reader_has_open`).
2. Headings named by fact: "Store checks" becomes, for example, "Files, the registry and named mutexes". Keep the
   README's two anchors unchanged. Record every rename (old heading, old anchor, new heading, new anchor) in the Result;
   `p8-final-cleanup` fixes the pointers.
3. Sections that only record planning (the track plan checks, "Contracts split checks", "Track plan fixes checks",
   "Design verify checks" and the like): remove them when they hold no fact about Windows, an agent, a tool or the
   product; move a fact they do hold into the section of its topic. List each removed section and where its facts went.
4. Do not reorder sections and do not touch the file's opening paragraph: `p8-docs-research-2` groups the whole file by
   topic and writes the intro and contents list. Do not edit sections from "Tray UX: tray icon" on.
5. Result: "Heading renames"; removed sections and where their facts went; sections you left as they are and why;
   size of your range before and after.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on your range: zero hits (`spikes/` included).
2. No em dash (U+2014) in your range.
3. Every section name or anchor in your range that a file outside `docs/tasks/`, `docs/reviews/`, `docs/phases/` cites
   still exists, or is in "Heading renames". Show the grep.
4. "not verified" and "not tested" counts in your range: none lost (count before and after, minus those inside
   sections you removed, each listed).
5. Every relative link and picture path in your range resolves to a file that stays.

## Out of scope

The rest of `docs/research.md`, every other file, code, the README. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
