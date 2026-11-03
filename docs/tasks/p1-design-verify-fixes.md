# Fix the verify findings and write Haik's sign-off list

Phase: 1. Status: done. Needs Haik: no (new judgment calls go under Questions for Haik with a default). Agent:
`wakeshot-worker`. Size: about 100K tokens.
Runs after: both verify reviews (`docs/reviews/p1-design-general-verify.md`, `docs/reviews/p1-design-power-verify.md`).
Parallel with: `p1-cargo-workspace` (it owns `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/`, `crates/`
and one `CLAUDE.md` line) and `p1-tray-menu-look` (it owns `spikes/tray/`, `docs/research/tray-menu-look.md`,
`docs/research/img/`, its card). This card owns `docs/architecture.md`, `docs/product.md` and the phase file sections
named below. `docs/research.md` is shared: additions only, re-read right before each edit.

## Goal

Close the findings of both verify reviews, so the design is ready for Haik, and write the short list he signs off
from.

## Read first

- `docs/reviews/p1-design-power-verify.md` (2 major, 12 minor) and `docs/reviews/p1-design-general-verify.md`
  (1 major, 9 minor).
- `docs/architecture.md`; `docs/phases/01-design-skeleton.md` (summary, "Values I set", questions, git steps).
- `docs/tasks/p1-design-review-fixes.md` Log (what the last round changed and why).

## Deliverables

1. Every finding of both verify reviews fixed, or listed in the Log with the reason. Must fix:
   - power verify major 1: a failed read of the plan list or the active plan must never look like "plan gone"; the
     journal, RunOnce entry and guard stay until a restore really succeeds;
   - power verify major 2: Question 10 covers fast user switching (B closes the lid while A's override is in force);
   - general verify major: Haik's last yes right before `git init` and the first public push is back in the git
     steps as its own gate, and Gate A no longer reads as passed.
2. A section "Sign-off list for Haik" at the top of the phase file's "Design summary for Haik", at most about 25
   lines, plain words: what the four reviews found in short, what changed in the design because of them, the values
   he has not seen ("Values I set", by reference), and the open questions with defaults (10, 11, 12 and any new
   one). Haik reads only this list to sign off.

## Acceptance checks

- Each finding of both verify reviews is fixed or listed with a reason.
- No restore path deletes the journal, RunOnce entry or guard after a failed read or write.
- The git steps have a gate with Haik's yes before `git init` and before the first push.
- The sign-off list stands alone: someone who read nothing else can sign off from it.

## Out of scope

- Code, Cargo files, git, `spikes/`, other cards.
- The phase file's work log and card status column (the orchestrator writes them).

## Questions for Haik

Each with the worker's default. Full text in `docs/phases/01-design-skeleton.md`, "New questions for Haik"; the
sign-off list at the top of its summary has the short form. None blocks the build cards.

10. Two users on one PC. New, safer default for fast user switching: a tray overrides only while its session is on the
    PC's own screen; when another user takes it, it restores and keeps only the power request, and applies again when
    its user comes back (Remote Desktop most likely counts as leaving; not tested). For an override left behind after
    a session ended: v1 assumes one signed-in user at a time; the README says so.
11. Longer gaps, now with their length: until the next sign-in (OS crash, power cut, "Shut down anyway", End task on
    tray and guard); until Wakeshot next starts (sign-in restore fails 2 minutes and the tray fails too, or the
    sign-in restore crashes); until a reinstall (uninstall whose restore fails for 30 s). Default: accept; README lists
    them.
12. A failed restore starts the tray. Default: yes. Now names the side effects (it holds for agents as usual, also with
    start at login off; it comes back 2 minutes after a Quit whose restore failed) and the other option (that tray only
    retries and warns, then exits).

No new question: the Gate B yes before `git init` and the push is a gate in the steps, not a question.

## Found outside scope

Problems seen but not fixed.

- `p1-contracts` (running in parallel) writes the boundary types. This card changed them in `architecture.md`
  section 3: `Started` gains `on_console`; `SessionChange` gains `EndCancelled`, `ConsoleLeft`, `ConsoleBack`;
  `Command::Restore { plan: Option<Guid>, why }` (new `RestoreWhy::ConsoleLeft`); `Outcome::Applied` and
  `Outcome::Restored` carry `journal: Vec<JournalEntry>` (and `Restored` its `plan`); `ApplyError::PlanChanged` can
  now come after writes; the episode's `released` is `Set<SettingKey>`. The orchestrator should relay this.
- `docs/tasks/p1-tray-menu-look.md` deliverable 1 still has no crop line for the new pictures (Wi-Fi flyout network
  names, account names). The phase file now says the orchestrator adds it.
- `.claude/skills/orchestrate/SKILL.md:21` (worktree isolation once the repo exists) and `:33` (`git pull --rebase`,
  "no upstream: abort") still contradict Question 9's decision and step 3. Orchestrator's file.
- `docs/tasks/p1-lid-sound.md:177-178` (done card) still says the lock waits until the stream runs; the design now
  locks at once.
- `.gitignore` (done card `p1-repo-files`): general verify minor 2's optional ignore lines for `*.key`, `*.pem`,
  `*.pfx`, `*.p12`, `.env*`, `id_rsa*`, `id_ed25519*` are not added. The scan before every commit covers the risk.
- `spikes/power-settings/src/power.rs` `schemes`: stops on any `PowerEnumerate` error as if the list ended. Harmless in
  a spike; the design now says the port must not copy it.
- `docs/product.md:21` is a 203-character line (old, harmless; not rewrapped).
- For the power and core track planners (round 3): a rarer case of the same lateness as R2-1. An entry left on plan P
  by a failed restore while P was inactive, for a key the user released on another plan in this episode, then a switch
  back to P: the hold's `Apply` to P leaves that key out (released), so it never takes the entry over, and while the
  core applies the retry skips P. Nothing is dropped; the restore comes at the hold's end. A fix needs either
  `Restore` with keys (a contract change) or `Apply` restoring entries on its plan for keys outside its writes (Apply
  step text). Not changed here: the reviewer is re-checking section 5.
- Correction 12 of `p1-contracts` (journal entries with a key this version does not know) stays open for the power
  track planner, as the orchestrator said.

## Log

- 2026-11-03 16:03: card written by the orchestrator after both verify reviews.
- 2026-11-03 16:06: worker started; read both verify reviews.
- 2026-11-03 16:10: checks done (no writes): `PowerEnumerate` ends the plan list with 259 (`ERROR_NO_MORE_ITEMS`,
  read-only script); the spike's `schemes` loop stops on any error as if the list ended; Microsoft pages for
  `WM_WTSSESSION_CHANGE` and `WAIT_ABANDONED`, Rust `set_hook`; the new hook pattern rejects 6 AI forms and passes 3
  plain messages (GNU grep 3.0). Next: `architecture.md`.
- 2026-11-03 16:15: `architecture.md` done for all power verify findings (restore step 2 fails on a short list or a
  failed id read; write-ahead mark; `Restore { plan }` and retry rule; journal owner re-check; no `Apply` after session
  end; second plan read before `PowerSetActiveScheme`; journal last in the safety net; `released` by key; abort in
  every profile and `WAIT_ABANDONED`; 5 s AC to DC lock; alias vs chime wording and checklist 16; gap list with
  lengths; console rule for fast user switching) and the README line (general minor 9). Orchestrator notes from
  `p1-cargo-workspace` applied: section 10 CRT caveat replaced, `RUSTFLAGS` rule; section 2 bins add the libs later.
  Next: `product.md`, then the phase file.
- 2026-11-03 16:17: `product.md` (status line, AC sound exception, unplug lock, rule 3 on every plan) and the phase
  file done: sign-off list at the top of the summary, summary and "Values I set" updated, Questions 10 to 12
  rewritten, git steps (Gate A text, Gate B with freeze rules, tested hook pattern, scan self-test, scan before every
  commit, remedy for a hit in an unpushed commit, Gate C failure path, crop claim fixed). Next: `research.md`, style
  and acceptance checks.
- 2026-11-03 16:19: `research.md` section "Design verify checks" added. Acceptance pass over every restore path found
  two more gaps, now fixed: a journal load error other than "missing" now fails the run (it was not named), and a
  failed second plan read before `PowerSetActiveScheme` keeps the entries. The `SafetyNet` undo takes back RunOnce and
  the guard only when the journal is known to be absent. Style check (script): no dashes, emojis or CR; no new line
  over 120.
- 2026-11-03 16:32: reopened by the orchestrator for round 2: `docs/reviews/p1-design-final-check.md` (0 blocker, 1
  major, 8 minor) and `p1-ci`'s section 10 wording.
- 2026-11-03 16:35: round 2 done in `architecture.md`, `product.md` and the phase file (table below). Stop-point walk
  for the major, on the new text. Notation: `o` original, `w` written (0), "in force" is the applied value, "marked" is
  `restored`. Restore step 3 drops only "unmarked, live `o`" (never written, so in force is `o` too) or a value that
  is neither `o` nor `w` (the user's).
  - Entry states an `Apply` can take over: E1 marked, live `o`, in force `w` (restore wrote, call failed); E2 marked,
    live `w` (restore stopped after its mark); E3 unmarked, live `w` (restore failed on a read or on its mark).
  - A1, stop before Apply step 3's journal write: journal unchanged; each E is finished by the next restore (E1 apply
    only, E2 and E3 write `o` then apply).
  - A2, stop after step 3, before step 5: E1, E2, E3 keep their state (the mark is no longer cleared here), so as A1.
    A new entry F: unmarked, live `o`, in force `o`: dropped as never written, correct.
  - A3, stop during or after step 5: a written E1 is now marked, live `w`: "write never happened" branch, restored
    again; an unwritten E1 as A2. F written: unmarked, live `w`: restored. F unwritten: as A2.
  - A4, a write or read-back fails in step 5, the second plan read fails, or the call fails: the restore routine runs
    on this plan's entries, so every case above is restored or kept; a failure keeps the entries.
  - A5, stop after a successful call, before the mark-clearing journal write: E marked, live `w`, in force `w`:
    restored again, correct. The clearing write fails: marks stay; the ownership check skips them (a user change is
    not noticed until the next restore), and that restore handles marked entries right.
  - A6, plan switched between step 5 and the call: no call; E marked, F unmarked on a now inactive plan; the plan
    switch restore handles both (marked live `w` and unmarked live `w`: write `o`; inactive: no apply).
  - Restore routine, unchanged: R1 stop before its mark: nothing changed. R2 after the mark, before the write: marked,
    live `w`, restored again. R3 after the write, before the call: marked, live `o`, apply. R4 after the call, before
    the journal rewrite: marked, live `o`, apply again (harmless) and drop. R5 journal emptied, stop before the
    clean-up: step 1 now runs the clean-up on a journal with no entries. R6 stop inside the clean-up: no journal, so
    RunOnce and the guard find nothing to do.
  - Ownership check: drops only on a successful read that differs from `w` on an unmarked entry; a stop between the
    drop and the `UserChanged` post loses only the notice.
  Result: no stop point and no failed read or write drops an entry whose value is still in force.
- 2026-11-03 16:38: reopened for round 3: apply `p1-contracts` design corrections 1 to 11 and 13 to
  `architecture.md` so the text matches `crates/`. Correction 12 (unknown journal keys) is left to the power track
  planner. No restore step text to change unless a correction needs it.
- 2026-11-03 16:41: round 3 done. `p1-contracts` corrections 1 to 11 and 13 applied to `architecture.md`, each checked
  against `crates/` (section 2 crate table, deps paragraph, core rows `time`, `process` (new), `settings`, `files`,
  `view`, Windows rows `error` (new), `clock`, `power::plan`, `process`; boundary trait block with the full `PowerApi`,
  `Notifier` returning `Result`, `WantedFile` and `FileId`; section 3 `Started.tray`, `Sources` with `Alive`,
  `Hibernate { reason, floor_pct, last_resume_mono_ms }`, `WriteHold`, `CountdownReason`, `HibernateSkip`, `FileKind`,
  `Notice` data; section 6 `VersionProbe` and `HoldFile.expires`; section 7 `Liveness`, `Waiting`, `Unreadable`;
  section 11 table and the `test-support` line). Correction 12 not touched. No restore step text changed by them.
  Two more items from the round 2 re-check (`p1-design-final-check.md`, "Round 2"): R2-2, the sign-off list's
  Question 11 line now ends the damaged-file case with "until then a lid close gets no lock"; R2-1, the retry rule
  in the episode flow (section 3) adds "no `Apply` failed in this episode" to "while the core applies", so after a
  failed apply the retry is `Restore { plan: None }` and does not wait for an apply that never comes (property test
  line added). Stop points: unchanged, since the retry runs the same restore routine already walked (16:35); it only
  runs sooner. Style check: clean.

How each finding was handled. "Fixed": as the review asked. "Changed": done another way, reason given.

Power verify (`docs/reviews/p1-design-power-verify.md`):

| # | Outcome | Where and why |
|---|---|---|
| Major 1 | Fixed | Restore step 2: the id read and the whole plan list (`PowerEnumerate` until 259) must succeed before any entry is dropped or written; tests added; the port note in section 2 |
| Major 2 | Fixed | Question 10 names fast user switching; safer default (override only while on the console, `ConsoleLeft` restores, `ConsoleBack` applies); episode flow, section 4, decision 18, OS events, checklist 11, "Values I set" |
| Minor 3 | Fixed | Restore step 4 writes ahead (mark, then write); step 3 treats "`restored` and live equals `written`" as a write never made; test added |
| Minor 4 | Fixed | `Restore { plan: Option<Guid>, why }`; retry rule in the episode flow (while applying, only other plans' entries); `Done(Applied)` and `Done(Restored)` carry the journal; `Apply` takes over kept entries (step 3); the skip of the user's own 0 moved to the IO thread, which knows the journal |
| Minor 5 | Fixed | Restore step 7: each retry reloads the journal and exits when another tray owns it; Unknown in restore mode is a failed run |
| Minor 6 | Fixed | Section 4: queued `Apply` dropped at session end; no `Apply` after `Session(Ending)`; `EndCancelled` lifts both |
| Minor 7 | Fixed | Apply step 6: second active plan read right before `PowerSetActiveScheme`; changed: no call, `PlanChanged`, the plan switch restores. Same in restore step 5 |
| Minor 8 | Changed | Safety net order: RunOnce and guard first, journal last, so a failure never leaves entries for unwritten keys (the review's "undo the entries" could itself fail); the undo takes back RunOnce and guard only when the journal is known to be absent. Clean-up whenever the last entry is dropped (step 6). New guard when the current one's stop event is set |
| Minor 9 | Fixed | `released: Set<SettingKey>`; "Restore my power settings" releases all three keys; `product.md` rule 3 says "on every power plan" |
| Minor 10 | Fixed | Panic hook calls `std::process::abort()` in every profile (tray startup step 5, sections 8 and 10); `WAIT_ABANDONED` counts as taken (section 4) |
| Minor 11 | Fixed | Episode flow: AC to battery with the lid closed and our Do nothing in force locks and chimes after 5 s; checklist 8; "Values I set"; `product.md` |
| Minor 12 | Fixed | Lid sentence names the alias and marks the chime in the v1 order not tested; checklist 16; the two "no apply" points marked under "Plan switch" |
| Minor 13 | Fixed | "Gap left" split by length (next sign-in, next start, reinstall); Question 11; uninstall hook retries within 30 s |
| Minor 14 | Fixed | Question 12 names both side effects and the other option |

General verify (`docs/reviews/p1-design-general-verify.md`):

| # | Outcome | Where and why |
|---|---|---|
| Major 1 | Fixed | Gate A: signs off after the sign-off list; the 15:45 sign-off allows only local build cards. Gate B back in step 3, right before `git init`: Haik's yes covers steps 4 to 9 unless a scan hit is put to him. Sign-off list line. Steps intro reworded |
| Minor 1 | Changed | Freeze covers the orchestrator's notes (scratchpad until the push) and repo-writing processes. Gate C failure: re-scan and a second commit with a plain message, not an amend (Haik's rule prefers new commits) |
| Minor 2 | Fixed | Step 11: scan before every commit; remedy for a hit in an unpushed commit (never push; Haik's go for a history rewrite). Optional `.gitignore` lines not added (not my file; Found outside scope) |
| Minor 3 | Fixed | Step 5 self-test on the known fine hits; zero hits stops; note for when Phase 8 removes them |
| Minor 4 | Fixed | Hook pattern as proposed, split into a variable; tested on nine samples (`research.md`) |
| Minor 5 | Not mine | `p1-cargo-workspace` card; it installed the 1.99.1 toolchain itself (`research.md`, "Workspace build") |
| Minor 6 | Fixed in part | Phase file claim corrected; the crop line in `p1-tray-menu-look` is the orchestrator's (Found outside scope); in the sign-off list |
| Minor 7 | Fixed | `product.md` sound exception; sign-off list |
| Minor 8 | Fixed in part | `product.md:3` fixed. Already fixed by others: `plan.md` Git (Question 9 decided), `CLAUDE.md` status. Not mine: `SKILL.md:21`, `:33`, `p1-lid-sound.md:177-178` (Found outside scope) |
| Minor 9 | Fixed | Decision 41's row lists what the v1 README must say |

Orchestrator notes (16:13, from `p1-cargo-workspace`): section 10 CRT caveat replaced by the checked fact and a
"never set `RUSTFLAGS`" rule; section 2 says the skeleton bins depend on neither lib yet.

Round 2, final check (`docs/reviews/p1-design-final-check.md`):

| # | Outcome | Where and why |
|---|---|---|
| Major 1 | Fixed | Apply step 3: a taken-over entry keeps its `restored` mark; step 6 clears marks only after a successful `PowerSetActiveScheme`; any failure in steps 5 and 6 runs the restore routine on this plan's entries (`ApplyError::Write` comment, section 8 row). Stop-point paragraph under Apply; restore step 3 names the takeover; restore step 1 cleans up a journal with no entries; tests name a failed write, a failed call and a stop after a takeover. Walk in the Log (16:35) |
| Minor 1 | Fixed | Sign-off list Q12: works as usual, overrides at the next agent turn; other option |
| Minor 2 | Fixed | Sign-off list Q10: Remote Desktop, the accepted risk. Full Q10 and the episode flow: a failed `ConsoleLeft` restore warns in the session that left |
| Minor 3 | Fixed | First line: "Read this list and 'Values I set' below" |
| Minor 4 | Fixed | Restore step 1 says what a corrupt journal leaves; "Gap left" (until set by hand); section 8 row; Q11 full and short |
| Minor 5 | Fixed | List: "(exceptions in Question 11)"; review counts now 5 reviews, 1 blocker, 14 major, 55 minor |
| Minor 6 | Fixed | `ConsoleLeft` drops the screen request too; `ConsoleBack` takes it again; property test; Q10. Whether Windows honors a screen request from a non-console session: not verified (marked) |
| Minor 7 | Fixed | Apply step 1: a failed read writes nothing and reports `SafetyNet`; section 8 row |
| Minor 8 | Fixed | `product.md`: "or longer in the cases of Question 11"; phase summary: repo files, workspace and CI done under the ahead sign-off, then their review, Gate B, scan, push |

`p1-ci` wording (its card, "Found outside scope"): section 10 CI bullet now names `--target <triple>` on each cargo
command, the `dumpbin /dependents` step and its fail list, and that CI never sets `RUSTFLAGS`.

## Result

Done 2026-11-03 16:21; round 2 done 16:36; round 3 done 16:42 (`p1-contracts` corrections 1 to 11 and 13 in
`architecture.md`, matching `crates/`; final check R2-1 retry rule and R2-2 Question 11 clause; Log 16:41).

What was done: every finding of both verify reviews handled (power: 2 major and 12 minor, all fixed, minor 8 another
way; general: 1 major and 9 minor, 6 fixed, 1 changed, 2 in part, 1 not mine), plus the two orchestrator notes and
two more restore gaps found in the acceptance pass. Sign-off list for Haik written. Round 2: all 9 final check
findings fixed (major: a taken-over entry keeps its `restored` mark until `PowerSetActiveScheme` succeeds, and any
apply failure runs the restore routine; stop-point walk in the Log), `p1-ci`'s section 10 wording applied. The
sign-off list is now 26 lines with the review's additions. Round 2 changed no type shapes; it changed what
`ApplyError::Write` means (the restore routine ran) and that a failed read at Apply step 1 is `SafetyNet`.

Files changed:

- `docs/architecture.md`: sections 2 to 6, 8 to 10 (restore routine, Apply, retries, console rule, session end,
  panic hook, gap list, tests, checklist 8, 11, 16, CRT and `RUSTFLAGS`, README line).
- `docs/product.md`: status line, unplug lock, AC sound exception, rule 3.
- `docs/phases/01-design-skeleton.md`: sign-off list, summary, "Values I set", Questions 10 to 12, git steps, scan
  section, one line of the card order paragraph. Work log and status column untouched.
- `docs/research.md`: new section "Design verify checks" (additions only).

Checks:

- Each finding fixed or listed with a reason: pass (tables above).
- No restore path deletes the journal, RunOnce entry or guard after a failed read or write: pass by reading every
  path (restore steps 1 to 7, ownership check, `SafetyNet`, guard and restore mode retries, hooks, CLI, session end).
  Round 1 missed the takeover path (final check major 1); round 2 fixed it and walked every stop point of `Apply` and
  the restore routine (Log, 16:35).
  One reasoned exception, now written in section 6: uninstall removes the RunOnce value even after a failed restore,
  because the exe it names is being removed; the journal stays. A corrupt journal (reads, does not parse) is still
  moved aside, as both first reviews accepted.
- Git steps have a gate with Haik's yes before `git init` and before the first push: pass (Gate B, step 3; a scan
  hit put to Haik needs his yes again before the push).
- Sign-off list stands alone: pass by reading (26 lines; terms like "guard" explained; its first line now sends Haik
  to "Values I set" as well, as the card allows; Questions 10 and 12 carry their risks and options).
- Style: no em or en dashes, emojis or CR in the four files; no new line over 120 characters.

Open items: Haik's sign-off and answers to Questions 10 to 12; the orchestrator relays the type changes to
`p1-contracts` and adds the crop line to `p1-tray-menu-look`; the other Found outside scope lines.
