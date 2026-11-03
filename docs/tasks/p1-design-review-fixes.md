# Fix the design review findings and apply Haik's answers

Phase: 1. Status: done. Needs Haik: only for new judgment calls (write them under Questions for Haik with a
default; do not block on them). Agent: `wakeshot-worker`. Size: about 150K tokens used (estimated).
Runs after: `p1-detailed-design` and the general design review (`docs/reviews/p1-design-general.md`), both done.
The power design review (`docs/reviews/p1-design-power.md`) is still running; the orchestrator sends it when done.
Parallel with: `p1-lid-sound` (finishing) and `p1-tray-menu-look`. Shared file: `docs/research.md` (both write it):
re-read it right before each edit and match on text.

## Goal

Make the Phase 1 design ready for Haik's sign-off: fix both design reviews' findings and write in Haik's answers to
the 8 design questions.

## Read first

- `docs/reviews/p1-design-general.md` (1 blocker, 4 major, 13 minor), then `docs/reviews/p1-design-power.md` when
  the orchestrator says it is done.
- `docs/architecture.md`; `docs/phases/01-design-skeleton.md` (summary, questions, card table, git steps).
- The Phase 1 cards: `p1-repo-files`, `p1-cargo-workspace`, `p1-ci`, `p1-privacy-check`, `p1-privacy-fixes`.
- `docs/tasks/p1-detailed-design.md`: Result and "Found outside scope".
- `docs/plan.md` "How we work" (updated 2026-11-03 15:30: fully public, Phase 8 cleanup, parallel agents).

## Haik's answers (2026-11-03, 15:29)

1. Question 1, his words: "We need to go fully public. If something is weird and internal like our conversation or
   our planning, this can be deleted in the end. When we make it 1 single commit history will be lost as well.
   imagine a perfect repo with simple claude.md and readme.md and maybe a claude skill if it's too important. This
   cleanup phase should be seperate phase. Temporary junk in remote is ok. Noone actively viewing my profile."
   So: everything goes public now, including docs, planning, reviews, spikes and their logs. Phase 8 (in `plan.md`)
   does the cleanup and the squash. What still never goes public: secrets (tokens, keys, credentials, the Claude
   `.key` files, anything that grants access) and build output (`target/`). The privacy check shrinks to a secret
   scan of the exact staged state right before each push.
2. Questions 2 to 8: the defaults, as written in the phase file.
3. Commit identity (general review M4): Haik's usual git identity stays. His Gmail address is already on 6,683 of his
   public GitHub commits (`gh` search, 15:29), so it adds nothing new. No noreply address.
4. Standing rule: run agents in parallel whenever it is safe, at full effort. Plan the card order for that.

## Deliverables

1. Every finding of both reviews fixed in `architecture.md`, the phase file and the cards, or listed in the Log with
   the reason it was changed or not applied. With answer 1, general review B1 and M3 become: one lean secret scan
   (card or orchestrator step) on the staged files right before the push, with all cards that write files paused
   from scan to push. Merge or drop `p1-privacy-check` and `p1-privacy-fixes` to fit.
2. Haik's answers written in: each "Question N" pointer in `architecture.md` replaced by the answer; the phase file's
   questions marked answered; `docs/product.md` updated where an answer or the design settles an open value (battery
   sleep Never; 5 minutes with no input before hibernate; the menu commands). Replace lines; no history.
3. A short "Values I set" list in the phase file summary (general review m5): every value the design picked that
   Haik has not seen, each with one line of why.
4. The design card's outside-scope items: mark Claude signals rule 11 as dropped in
   `docs/research/claude-code-signals.md`; move the five tray facts that live only in `docs/reviews/p0-tray.md`
   (findings 2, 6, 12, 13, 14) into `research.md`'s tray section, so the design cites `research.md`.
5. Every Phase 1 card fits about 150K tokens of one worker (the design card used about 434K). Split any that do not.
   Fill "Runs after" and "Parallel with" for the most parallel safe order.
6. Worktree isolation (general review M1): propose the simplest rule that fits this repo's git flow. Haik's global
   rule gives fan-out agents worktree isolation because many repos deploy on push; this repo only runs CI on push.

## Acceptance checks

- Each finding of both reviews is fixed or listed with a reason.
- No "Question N" pointer is left in `architecture.md`.
- The git steps have no commit before the secret scan and no file change between the scan and the push.
- Every card names its size estimate, and none is over about 150K tokens.
- No product decision made silently: new judgment calls are under Questions for Haik with defaults.

## Out of scope

- Code, Cargo files, git, creating the repo.
- `docs/tasks/p1-lid-sound.md`, `docs/tasks/p1-tray-menu-look.md`, `docs/research/tray-menu-look.md`, `spikes/`
  (other workers own them).
- The phase file's work log and card status column (the orchestrator writes them). Add or change rows only for
  cards you create, split or drop.

## Questions for Haik

Each with the worker's default. Full text in `docs/phases/01-design-skeleton.md`, "New questions for Haik". None
blocks Phase 1.

9. Worktrees in this repo. Default: none, for workers or reviewers (only the orchestrator runs git, a push only runs
   CI, parallel cards write different files). Change it when an agent must run git or two cards must edit one file.
10. Two users on one PC. Default: v1 assumes one signed-in user at a time; the README says so.
11. More ways into the crash gap ("Shut down anyway" before the sign-out restore ends; End task ending tray and guard
    together, not tested; a sign-in restore that keeps failing). Default: your "accept" covers them; README lists them.
12. A restore that fails outside the tray for 2 minutes starts the tray, even with start at login off, so the warning
    shows. Default: yes.

## Found outside scope

Problems seen but not fixed.

- `docs/plan.md` "Git" says "Reviewers run with worktree isolation once the repo exists", and `SKILL.md` step 4 says
  the same for parallel agents. Both conflict with the Question 9 default. The phase file says Phase 1 runs without
  worktrees (they cannot work before the first commit). After Haik answers, the orchestrator updates both lines.
- `SKILL.md` "Git": `git pull --rebase` at each start, and "no upstream: abort and tell Haik". Phase file step 3 says
  to skip it until the first push (general review m12); the skill line itself is the orchestrator's to change.
- The secret scan patterns live only in the phase file. From Phase 2 every push needs them; move them to `plan.md`
  "Git" or the skill when Phase 1 closes.
- `docs/tasks/p1-tray-menu-look.md` (not mine): no size estimate, and its Wi-Fi quick settings pictures list nearby
  network names (general review m13). Under Haik's answer 1 these are not secrets; a crop is the orchestrator's call.
- `docs/product.md` line 3, "Status on 2026-11-03: decisions fixed, Phase 0 (facts) running": Phase 0 is done.
- `CLAUDE.md` "Status: Phase 0 ... no git repo yet" is stale; `p1-cargo-workspace` and orchestrator step 4 fix it.

## Log

- 2026-11-03 15:30: card written by the orchestrator after the general design review and Haik's answers.
- 2026-11-03 15:30: worker started. General review and Haik's answers first; power review when the orchestrator
  sends it.
- 2026-11-03 15:33: orchestrator input: `p1-lid-sound` done; put its pick into the sound seam. Done in `research.md`:
  "Morning results" Sound bullet now points to "Lid-close sound"; four tray facts added to the tray section (code-opened
  menu and `TrackPopupMenuEx` Remarks, read today; double `NIN_BALLOONTIMEOUT`, grep of both logs today; balloon not
  stored; 16 s log clock drift). Finding 12 (no `WM_DPICHANGED`) was already there. Rule 11 marked dropped in
  `research/claude-code-signals.md`.
- 2026-11-03 15:36: power review arrived (0 blocker, 6 major, 13 minor). Orchestrator: power minor 18 wins over the
  lid sound card's wait; lock at once, sound in parallel; resolve general m6 the same way. Saved copies of
  `architecture.md` and the phase file in my scratchpad to map the reviews' line numbers.
- 2026-11-03 15:39: `product.md` updated: Haik's sound moments (15:35, "default"), device rule, battery sleep Never,
  5 minutes, CLI commands, "update Wakeshot" once per start per agent, routine hold start and end in the icon only,
  crash gap accepted, pack id and data folder, fully public repo. Next: `architecture.md` rewrite with every finding.
- 2026-11-03 15:44: `architecture.md` rewritten with both reviews' design findings and Haik's answers (no "Question N"
  pointer left). Next: phase file (summary, Values I set, answered questions, new questions, card table, git steps,
  worktree rule), then the cards.
- 2026-11-03 15:48: phase file done (summary, "Values I set", answers, new Questions 9 to 12, card rows, order, sizes,
  git steps with freeze and secret scan). Dry run of the scan patterns with GNU grep over the tree (no git): parse
  fine; only redacted or public values hit (recorded in `research.md`, "Phase 1 design checks", and the phase file).
- 2026-11-03 15:51: cards updated: `p1-repo-files` (answer 1, m10, size, runs after this card), `p1-cargo-workspace`
  (m11, size), `p1-ci` (size); `p1-privacy-check` and `p1-privacy-fixes` marked dropped (not deleted: not my files).
- 2026-11-03 15:53: style check (script): no em or en dashes, no emojis, LF only in the 11 files I changed; long lines
  I made rewrapped.

How each finding was handled. "Fixed": done as the review asked. "Changed": done another way, reason given.

General review (`docs/reviews/p1-design-general.md`):

| # | Outcome | Where and why |
|---|---|---|
| B1 | Changed | Haik's answer 1 turned the privacy check into a secret scan. Phase file steps 3 to 9: freeze from step 3 to the push, `git add`, save `git ls-files -s`, scan the index with `git grep --cached`, commit, Gate C compares `git status --porcelain` and `git ls-files -s` with the saved file |
| M1 | Changed | Proposal, not a rule change: Question 9 (default no worktrees). `plan.md` and `SKILL.md` lines wait for Haik (Found outside scope) |
| M2 | Fixed | Answer 1 makes the fixture log public; `p1-repo-files` ignore list and architecture section 9 updated |
| M3 | Changed | Card not split but dropped, with `p1-privacy-fixes`; the secret scan replaces both |
| M4 | Fixed | Haik's answer 3: his usual identity; step 4 checks it; phase file notes it |
| m1 | Changed | Kept two bin crates with the true reason (only the tray takes `velopack` from Phase 6; resources are not the reason). One package only if the dependency lists end up the same |
| m2 | Fixed | Journal is one file; registry copy, `seq` and its merge rule removed |
| m3 | Fixed | With power 3: the panic hook logs and aborts; no restore |
| m4 | Fixed | With power 7: `restore-request.json`; tray runs the menu path; tray running judged by the journal owner |
| m5 | Fixed | "Values I set" in the phase summary; Never trade-off in Question 5's answer |
| m6 | Changed | Orchestrator, from power 18: lock at once, sound in parallel, no wait |
| m7 | Fixed | `HoldFacts`, `Outcome`, `ApplyError`, `PlanValues`, `OwnedKey`, `Failure`, `Notice` variants defined; `ReadPlan` command; exit waits owned by the watch thread; data folder watch named |
| m8 | Fixed | Rows 11 and 32: phases 2, 4, 5 |
| m9 | Fixed | With power 5: count the process only if it started before the file's last write time |
| m10 | Fixed | `.claude/worktrees/` and `.claude/settings.local.json` in `p1-repo-files` |
| m11 | Changed | Note in `p1-cargo-workspace`, architecture section 10 and the `CLAUDE.md` status line it writes; not in the spikes' READMEs (other cards own `spikes/`) |
| m12 | Fixed | Step 3 runs after step 2 ends; skip `git pull --rebase` until the push |
| m13 | Fixed in part | Fix row for the skeleton review added; parallel lines agree (privacy cards dropped; freeze in the steps); local `commit-msg` hook in step 4, matching trailers and "generated with", not the bare word "Claude" (messages may name Claude Code, the product). Wi-Fi crop note: not applied (card not mine; not a secret under answer 1; Found outside scope) |

Power review (`docs/reviews/p1-design-power.md`):

| # | Outcome | Where and why |
|---|---|---|
| 1 | Fixed | IO thread decides ownership per entry's own plan; `UserChanged(OwnedKey)`; `Release` command removed; `released` keyed by plan and key; plan id handled first; property and Windows layer tests list the cases |
| 2 | Fixed | "Plan gone" only when missing from `PowerEnumerate`; read errors keep the entry; `Plan(Result)`; `restored` flag per entry until `PowerSetActiveScheme` succeeds |
| 3 | Fixed | Panic hook logs and aborts (also general m3). Every apply and restore also runs under `Local\Wakeshot.Journal` |
| 4 | Fixed | Guard started with `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS` (Explorer); launcher as fallback; checklist 4; summary marks End task not tested |
| 5 | Changed | Start-before-write check applied (also general m9). Write-back of the learned start not applied: the check keeps no state, so it survives a tray restart, and a write-back would move the write time it relies on |
| 6 | Fixed | Apply step 1 reads the active plan and writes nothing if it changed; `PowerSetActiveScheme` takes the id just read; same in restore step 3 |
| 7 | Fixed | Same fix as general m4 |
| 8 | Fixed | Apply step 4: a failed safety step means no write; per-guard stop event; tray waits on the guard and restarts it |
| 9 | Fixed | Restore mode and guard retry 2 minutes, then start the tray (Question 12); uninstall deletes the journal only after a successful restore |
| 10 | Question | Question 10, default one user per PC, README says so (the review asked for a question) |
| 11 | Fixed | Dry run: own data folder and mutex names; never touches the real journal, Run or RunOnce |
| 12 | Changed | `seq` gone with the registry copy; entry fields frozen so any version restores what it knows |
| 13 | Fixed | End-of-session restore goes to the front of the IO queue; queued notices dropped |
| 14 | Fixed | Section 5 "Liveness": any open error reads the process list; list failure is Unknown, keeps the last state, deletes nothing; unknown lid never hibernates |
| 15 | Fixed | Section 7: Codex rule marked "can outlive its session"; Phase 4 maps the lock file to its process |
| 16 | Fixed | Inactive-plan write marked not tested and made the first Phase 3 test; Windows timers after a failed hibernate marked inferred; boot-time admin marked not verified; checklist 15 (50 apply and restore cycles) |
| 17 | Fixed | Episode flow: lock and sound on a power source change with the lid closed; checklist 8 |
| 18 | Fixed | Lock at once, sound in parallel; "change it when" a test shows the lock cuts a sound |
| 19 | Fixed | "Gap left" lists every path; Question 5's answer points to it; Question 11 asks about the new paths |
| Outside brief | Placed | `winget upgrade` leaves Wakeshot off until the next sign-in: architecture section 6, open for Phase 6. Settings mutex retried on each tick: sections 3, 4, 8 |

## Result

Done 2026-11-03 15:55.

What was done:

- `docs/architecture.md` rewritten with both reviews' findings, Haik's answers and the `p1-lid-sound` pick (674
  lines). No "Question N" pointer left.
- `docs/phases/01-design-skeleton.md`: summary updated; "Values I set" (15 lines); Questions 1 to 8 marked answered;
  new Questions 9 to 12; card rows (privacy cards dropped, skeleton review fix row, `p1-repo-files` runs after this
  card); most parallel order; size estimates; git steps rewritten (freeze, hook, stage, secret scan with patterns,
  Gate C on `git ls-files -s`); no Gate B wait for Haik, since his "go fully public" is the go (he is asked only about
  a scan hit the orchestrator cannot judge).
- `docs/product.md`: battery sleep Never; 5 minutes; CLI commands; sound moments and device rule (Haik 15:35); hold
  start and end in the icon only; "update Wakeshot" once per start per agent; v1 always locks; crash gap accepted;
  pack id and data folder; fully public repo with the Phase 8 cleanup.
- `docs/research.md`: four tray facts moved in from the tray review (code-opened menu with Microsoft's
  `TrackPopupMenuEx` Remarks, double `NIN_BALLOONTIMEOUT`, balloons not stored, 16 s log clock drift); "Morning
  results" Sound bullet points to "Lid-close sound"; secret scan dry run in "Phase 1 design checks".
- `docs/research/claude-code-signals.md`: rule 11 marked dropped.
- Cards: `p1-repo-files`, `p1-cargo-workspace`, `p1-ci` updated with sizes and lines; `p1-privacy-check` and
  `p1-privacy-fixes` marked dropped.

Counts: general review 18 findings: 11 fixed, 6 changed, 1 fixed in part (m13 crop note). Power review 19: 16 fixed,
2 changed, 1 became a question. Four pairs overlapped (general m3, m4, m6, m9 with power 3, 7, 18, 5); each was fixed
once.

Checks:

- Each finding fixed or listed with a reason: pass (tables above).
- No "Question N" pointer in `architecture.md`: pass (`Select-String 'Question \d'`: 0).
- Git steps: no commit before the scan (commit is step 7, scan step 5); no file change between scan and push (freeze
  from step 3; Gate C compares `git ls-files -s` with the saved list and needs a clean status): pass by reading. The
  scan commands themselves are not run (no repo); the patterns parse in GNU grep.
- Every card names a size, none over about 150K: pass for `p1-repo-files` (60K), `p1-cargo-workspace` (120K), `p1-ci`
  (80K). This card used about 150K (estimated, not measured). `p1-tray-menu-look` has no size line (not my card).
- No silent product decision: new judgment calls are Questions 9 to 12 with defaults; design values are in "Values I
  set".

Open items: Haik's sign-off and Questions 9 to 12; the `plan.md` and `SKILL.md` lines in "Found outside scope".
