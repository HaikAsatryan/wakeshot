# Recheck the Phase 0 close list against Haik's three answers

Phase: 0. Status: done. Needs Haik: no (he says yes to the result later). Agent: `wakeshot-worker`.
Runs after: `p0-close-proposals` (done) and Haik's answers to decisions 1 to 3 (given 2026-11-03).
Parallel with: Haik's morning steps and the spike cards. This card changes docs only, never power settings.

## Goal

Make part 2 of `docs/reviews/p0-close-proposals.md` match Haik's answers, so his single "yes" covers exactly the edits
that will be made. Also say which morning steps lost their purpose because of the answers.

## Read first

- `docs/reviews/p0-close-proposals.md`: part 1 (decisions 1 to 3 and Haik's answers), part 2 (items 1 to 25),
  part 3.
- `docs/product.md` and `docs/architecture.md`, to word each edit against the current text.
- `docs/phases/00-morning-steps.md`, only to map steps to what they feed.
- `docs/research/claude-code-signals.md` proposed rules table (part 2 item 21 points to it).

## Deliverables

All in `docs/reviews/p0-close-proposals.md`. Do not edit any other file.

1. Part 2, each item 1 to 25: add one tag line under it: **Keep**, **Change** (with the new wording), or **Drop**
   (with the reason). The rule from decision 3: drop or reword any item that exists only to guard against another
   program's bug (a lost signal, a hung process, a stale file). Keep the 1 hour limit on waiting for approval and the
   battery floor. Do not renumber; Haik may refer to items by number.
2. New items in part 2, numbered from 26, for the edits that decisions 1 to 3 themselves need: at least
   `product.md:58` (drop the fixed lid-closed time limit), `product.md:40-41` (every signal checked against a fact,
   not capped by time), the docked rule (decision 1, now mostly moot without the limit: say what is left of it), and
   "no heat guard" wherever the docs still propose one. Each item: file, line, old text, new text.
3. Part 3: tag each item **Still needed** or **Moot** after the answers, with the reason.
4. A new short section "Morning steps without a purpose now": each step or sub-step of `00-morning-steps.md` whose
   only consumer was the time limit or the heat guard. Say whether Haik can skip it. Do not edit the morning list:
   Haik may be using it.
5. Item 25 (live-test fixtures): the five test transcript folders in `~/.claude/projects` were deleted this morning
   and `spikes/claude-watch/fixtures/` does not exist. Check what real shapes still exist (for example
   `spikes/claude-watch/out`, quotes in the research report and cards). Rewrite item 25 to what can still be done.
   Never read or copy a `.key` file. Redact prompts and personal paths.
6. A summary at the top of part 2: how many items are Keep, Change, Drop, and new.

## Acceptance checks

- Every part 2 item 1 to 25 has exactly one tag line. Every part 3 item has a tag.
- No kept or new item adds a time cap whose only job is to cover another program's bug.
- Every new item quotes the current text of the line it changes (checked against the file today).
- The morning steps file is unchanged.

## Out of scope

- Applying any edit to `product.md`, `architecture.md`, `research.md`, `CLAUDE.md` or the phase file. The orchestrator
  does that after Haik's yes.
- Any power setting, spike run or process.

## Questions for Haik

Each with the worker's default. Neither blocks: Haik's yes to part 2 takes both defaults. Full text in the proposals
file, "Questions from the recheck".

1. An unknown Claude `status` value (items 21, 22). Default: counts as working while the session process lives,
   plus the "update Wakeshot" notice. Other choice: no signal, so a renamed `busy` lets the PC sleep mid-task.
2. A hang check for our own tray app (item 14). Default: none, item 14 dropped, because item 13 removes the cause.
   Other choice: keep the guard's 30 s `WM_NULL` ping.

## Found outside scope

Problems seen but not fixed.

- `docs/research/claude-code-signals.md:327-328` says the live-test watcher log was "not kept". It still exists in
  the old session scratch folder. Item 25's new text fixes the line after the copy.
- The battery floor still hibernates a closed laptop first (`product.md:56-57`), even with an external screen on and
  the user typing on it, against "never hibernates the PC in front of the user" (`architecture.md:36-37`). Decision 1
  covered only the time limit. Phase 1 can add the same "no input for a few minutes" wait as item 28, or leave it,
  since below the floor Windows' own rules stop the PC soon anyway (not verified).
- Signals rule 11 (`claude-code-signals.md:235`, transcript growth as a fallback when the status file is missing or
  unknown) is a guess-based signal that no part 2 item touches. With question 1's default its "unknown" case is moot;
  the "missing file" case is for Phase 4.
- `architecture.md:25` lists "limits" among the core policies. After decision 3 only Keep awake times and the 1 hour
  approval limit are left. Harmless wording; no item proposed.
- `00-facts.md` changes while workers run (two card rows were added during this card), so line numbers into it go
  stale within hours. Items 24 and 33 now say to find the lines by text.

## Log

- 2026-11-03: card written by the orchestrator.
- 2026-11-03: worker started. Read the card, plan "How we work", and the proposals file (parts 1 to 3).
- 2026-11-03: morning steps file SHA256 at start `616571B2...79FD6` (written 04:38:55), to prove it unchanged at the
  end. Read product.md, architecture.md, the morning steps, the signals rules table, review findings 3 to 18.
- 2026-11-03: first pass on items 1 to 25 (plan, not yet written): Drop 3 (busy cap), 7 (replaced by new item for
  product.md:40-41), 14 (hang check for our own tray: item 13 removes the cause). Change 5 (shell: no cap), 21 (rule
  3 rows, rule 4 product rule), 22 (unknown status, no cap), 24 (line 45 is now 46), 25 (fixtures). Rest Keep.
- 2026-11-03: scope change from the orchestrator: skip deliverable 4 ("Morning steps without a purpose now"); card
  `p0-morning-trim` does a fuller cut of the morning list and reads the part 3 tags, so part 3 is tagged first.
- 2026-11-03: part 3 tagged in the proposals file: items 1 to 8 all Still needed (5 with less at stake, 7 with a
  decision 3 note on the re-take). Added a "Moot after Haik's answers" list for the trim card: D3 heat run, the step
  I heat value, the closed-lid screen count (D, E, D5 "DELL stayed on"). D6 and D7 still test the core lid feature.
- 2026-11-03: item 25 check (counts and file names only, no content copied): in `~/.claude/projects` 0 timer lines
  left (CronCreate, ScheduleWakeup, scheduled_task_fire, session_cron_carry, nextFireAt, scheduledFor), but 26
  `backgroundTaskId` results, 82 `<task-notification>` lines, 6 Monitor calls. The live-tests watcher log still
  exists in the old session scratch folder (`...\1828f2de-...\scratchpad\live\watch.log`, 1078 lines, all test
  shapes; first 12 lines read to confirm the format), so `claude-code-signals.md:327-328` "(not kept)" is wrong.
  Live-tests worker transcript also left (`agent-a9478976b79b588cb.jsonl`, meta says "Do card p0-claude-live-tests").
- 2026-11-03: 00-facts.md changes while I work (the orchestrator added two table rows), so 00-facts.md items quote
  text, not only line numbers. Item 24's line is 48 now.
- 2026-11-03: part 2 summary and tags for items 1 to 25 written (Keep 17, Change 5, Drop 3). Next: new items 26 to
  33, questions, coverage note, checks.
- 2026-11-03: new items 26 to 33, two questions, coverage note written. Checks run (see Result). Done.

## Result

Done. Deliverable 4 skipped on the orchestrator's instruction (card `p0-morning-trim` does it).

What was done, all in `docs/reviews/p0-close-proposals.md`:

- Part 2 summary table at the top: Keep 17 (1, 2, 4, 6, 8 to 13, 15 to 20, 23), Change 5 (5, 21, 22, 24, 25),
  Drop 3 (3, 7, 14), new 8 (26 to 33).
- One "Recheck:" tag line under each item 1 to 25. Drops: 3 (busy cap, named by decision 3), 7 (replaced by item
  27), 14 (hang check for our own tray; item 13 removes the cause). Changes: 5 (`shell` and non-terminal background
  tasks hold while the process lives, no cap), 21 (two rule 3 rows lose their caps, rule 4 gets the 1 hour product
  rule), 22 (unknown status counts as working while the process lives), 24 (the line is now 48; find by text), 25
  (rewritten for what still exists).
- New items: 26 `product.md:58` (no time limit, no heat guard), 27 `product.md:40-41` (signals checked against facts),
  28 `product.md:59` (what is left of the docked rule: end-of-work sleep waits for no input; Do nothing means release
  only; no v1 rule counts screens), 29 to 33 (heat guard and time cap proposals in the rivals reports, `research.md`,
  the lid spike card and the phase log, marked not adopted).
- Part 3: all 8 items tagged Still needed (5 with less at stake; 7 notes that the re-take rests on another
  program's bug, so the delta review asks Haik if H shows the request honored). A "Moot after Haik's answers" list
  for the trim card: D3 heat run, the heat value from I, the closed-lid screen count in D and E. D6 and D7 still test
  the core lid feature.
- Item 25: the live-test watcher log still exists
  (`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\live\watch.log`,
  1078 lines, every test shape), and so does the live-tests worker's transcript. Haik's own transcripts hold no timer
  lines. New item 25: copy from the log first, because it sits under `%TEMP%`.
- Two questions for Haik, each with a default his yes takes (above).
- Coverage section: one paragraph on the homes that moved.

Files changed: `docs/reviews/p0-close-proposals.md`, this card. Scratch scripts only in my scratchpad.

Checks:

- Every part 2 item 1 to 25 has exactly one tag line: pass (script count: 25 tag lines, one per item).
- Every part 3 item has a tag: pass (8 of 8).
- No kept or new item adds a time cap that only covers another program's bug: pass by review. Left on purpose: the
  timer due time plus margin (decision 3 names it), the 1 hour approval limit, the end-of-work wait for no input (it
  waits for the user, not for a bug).
- Every new item quotes today's text: pass (script: 18 of 18 quotes found in the files, line numbers match).
- Morning steps file unchanged: pass (SHA256 `616571B2...79FD6` and write time 04:38:55, same as at the start).
- No em or en dashes in the proposals file: pass.

Open items: Haik's yes to part 2 (with the two defaults). Item 25's copy should run soon.
