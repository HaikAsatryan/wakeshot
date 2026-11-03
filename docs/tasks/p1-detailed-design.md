# Write the detailed design and the Phase 1 cards

Phase: 1. Status: done. Needs Haik: yes, at the end: he signs off the design (you write the summary and questions;
the orchestrator relays them). Agent: `wakeshot-worker`.
Runs after: Phase 0 sign-off (done 2026-11-03). Parallel with: `p1-lid-sound`, `p1-tray-menu-research` and the tray
Result review. Shared file: `docs/research.md` (the lid sound worker writes it too): re-read it right before each
edit and only add lines in your own place.

## Goal

Turn the high-level `docs/architecture.md` into the detailed design the build phases follow, and split the rest of
Phase 1 (repo, Cargo workspace, CI) into task cards. Haik signs off this design before any product code.

## Read first

- `CLAUDE.md`, `docs/plan.md` (How we work, Phases).
- `docs/product.md`: every decision. The design must serve each one.
- `docs/architecture.md`: the high-level design today.
- `docs/research.md`: the facts. The design leans only on these, or marks a point "not tested".
- `docs/research/distribution.md` (Velopack, winget, signing) and the rules section of
  `docs/research/claude-code-signals.md`.
- `docs/phases/00-facts.md`: the design inputs near the top of the work log. The open items below come from it.
- Spike code you plan to port: `spikes/power-settings/`, `spikes/lid-sleep/`, `spikes/tray/`,
  `spikes/claude-watch/`. Read at the level of modules and function signatures; do not change them.

Open items Phase 0 left for this design:

1. Battery sleep while holding: the Phase 0 close says pick Never (a finite value acts as a time limit, against
   decision 3). M1 tested Never with the lid open only.
2. Sign-in after sleep "Never": the stored value is untested on an account without Windows Hello (delta review m7).
   Place that test.
3. A power request held across a user sleep is not honored after resume; a fresh one taken after resume is (H4).
   Tested form: clear, then set on the same handle. "New before old" is not tested.
4. Settings and the Control Panel write some battery values to the user's real plan even while a copy is active
   (power card A, A4). The ownership rules must handle it.
5. Velopack kills the app without warning on update, uninstall and reinstall. Setup.exe over an install wipes the
   install folder with no hook. Data lives outside the install folder.
6. Phase 3 checklist: closed lid with no request on battery past the 600 s timeout; one sign-in delay other than
   "Every Time"; re-take on battery; "new before old" if used; a Settings or Control Panel change during a test with
   a copy writes the user's real plan.
7. Phase 4: Claude signals rule 11 (transcript growth when the status file is unknown) contradicts the
   unknown-status rule in `CLAUDE.md`; rule 5 does not cover `--resume` in a new process.
8. Two parts stay open on purpose until their cards end: how the tray menu is drawn (`p1-tray-menu-look`; its answer
   may change the UI stack) and how sounds play (`p1-lid-sound`). Design a seam for each; do not pick.

## Deliverables

1. `docs/architecture.md`, rewritten as the detailed design. Replace the high-level text; no history. Cover at
   least:
   - crates and modules, with the main types and traits at each boundary;
   - the core: events, state, commands, and a table from each product rule to where it lives;
   - processes (tray app, CLI, guard), how they talk, threads and the message loop;
   - Windows layer: power requests and their renewal after resume; power settings ownership, the journal, restore
     and the guard; OS events;
   - files: the data folder, every file format with a version field, config;
   - sources: the interface one agent source implements, and how an unknown format shows "update Wakeshot";
   - errors, logging, the user-visible log;
   - testing: the core with a fake clock, parser fixtures, Windows layer tests that never touch the user's real
     settings, the manual checklist for Haik;
   - CI and the build targets (x64, ARM64);
   - dependencies with versions checked today on crates.io or docs.rs (`windows` and any other crate), with the
     date.
   Mark each point that rests on something not tested. Keep it as short as the design allows; say each thing once.
2. `docs/phases/01-design-skeleton.md`: fill "Design summary for Haik" (what gets built, in plain terms, about 40
   lines at most), then "Questions for Haik" as a numbered list, each with your default. Fill "Orchestrator steps
   (git and GitHub)": the order of `git init`, the first commit, `gh repo create HaikAsatryan/wakeshot --public` and
   the push, with the gates before the push. Add your new cards to its card table.
3. Phase 1 task cards in `docs/tasks/`, from `_template.md`. At least:
   - repo files: LICENSE (MIT), a short README stub, `.gitignore`, `.gitattributes`, and which of `docs/`, `spikes/`,
     `.claude/` go public (a question for Haik, with your default);
   - a privacy check of every file before the first public push (a reviewer): personal paths, names, screenshots,
     transcript lines, machine IDs, anything from Haik's own sessions;
   - the Cargo workspace skeleton: builds, one test per crate, fmt and clippy clean;
   - CI on GitHub Actions: fmt, clippy, tests, x64 and ARM64 release builds.
4. Each open item above placed: resolved in the design, put in a card, or put in the later phase it belongs to (say
   which).

## Acceptance checks

- Every decision in `product.md` maps to a place in the design or to a named later phase (a table in
  `architecture.md` or in this card's Result).
- Every fact the design leans on points to `research.md`, or is marked "not tested".
- You made no product decision: each one is under "Questions for Haik" with a default.
- Cards follow the template; each fits about 150K tokens of one worker; "Runs after" and "Parallel with" are filled;
  no two parallel cards touch the same files or the real power settings.
- Nothing created outside docs: no repo, no git, no Cargo files.

## Out of scope

- Code, Cargo files, git, creating the repo.
- `docs/product.md`: do not edit. Contradictions or gaps go to Questions for Haik.
- Files another agent writes now: `docs/research/tray-menu-look.md`, `docs/reviews/p0-tray.md` and
  `docs/tasks/p1-tray-menu-research.md` (Haik's parallel session); `docs/tasks/p1-lid-sound.md` and
  `spikes/lid-sleep/` (the lid sound worker). Read the first three only if they say "Status: done"; never edit any
  of them. Do not edit `docs/tasks/p1-tray-menu-look.md`.
- The power settings, the screen and sound: this card only reads on the laptop.

## Questions for Haik

Eight, written out with reasons in `docs/phases/01-design-skeleton.md`, "Questions for Haik". Defaults in short:

1. Public: `docs/`, `spikes/` source, `.claude/`, `CLAUDE.md`; raw tool logs stay local.
2. Pack id `HaikAsatryan.Wakeshot`, data folder `%LocalAppData%\Wakeshot`.
3. 5 minutes with no input before hibernating a closed laptop.
4. CLI: `run`, `keep-awake`, `automatic`, `status`, `restore`, `log`.
5. Accept the crash gap (no restore before the next sign-in after an OS crash); README says so.
6. No test account for the sign-in Never mapping; v1 always locks at lid close while holding.
7. "Update Wakeshot" once per start per agent.
8. Routine hold start and end show only in the icon and tooltip; notices for everything else.

## Found outside scope

Problems seen but not fixed.

- `CLAUDE.md` says "Status: Phase 0 (facts) ... no git repo yet": stale since the Phase 0 sign-off. `p1-cargo-workspace`
  rewrites the status line; the orchestrator fixes the git part after `git init` (orchestrator step 3).
- `docs/plan.md` Status: the "Parallel session" and "Next" lines still describe the tray review and menu research as
  running; both are done.
- `docs/product.md` (not mine to edit): "Value in Phase 1: Never, or a value Settings cannot set" can now say Never
  (design section 5), and "a few minutes (value in Phase 1)" waits for Question 3.
- `docs/research/claude-code-signals.md` still lists rule 11 as a recommendation; the design drops it (section 7).
  Mark it there when Phase 4 starts, or now.
- Five tray facts the design uses live only in `docs/reviews/p0-tray.md` (findings 2, 6, 12, 13, 14: balloon not
  stored, code-opened menu and KB135788, no `WM_DPICHANGED`, 16 s counter drift, double `NIN_BALLOONTIMEOUT`). The
  design cites the review. Moving them into `research.md` is the orchestrator's call.
- After Haik answers, each "Question N" pointer in `architecture.md` should be replaced by the answer.

## Log

- 2026-11-03 14:35: card written by the orchestrator after Haik's Phase 0 sign-off.
- 2026-11-03 14:37: worker started. Reading product, architecture, research, phase files and spikes.
- 2026-11-03 15:06: reads done (spikes at signature level). Checked today: crate versions (crates.io API), windows-rs
  0.100 status (release 74, issue #4867), Actions versions and the ARM64 runner image (gh api), and three Microsoft
  pages (PostThreadMessage, Run and RunOnce keys, kernel object namespaces). Orchestrator note: tray research and tray
  review are done; menu pick B (owner-drawn classic menu plus `DWMWCP_ROUND`) is the expected answer behind the seam;
  the menu follows the Windows mode. Writing `architecture.md` next.
- 2026-11-03 15:09: `architecture.md` rewritten (11 sections, decision table with 44 rows). Added the tray review
  points (findings 2, 6, 12, 13, 14) the orchestrator listed. Research cited by section name only. New section
  "Phase 1 design checks" at the end of `research.md`. Writing the Phase 1 cards next.
- 2026-11-03 15:15: five cards written (`p1-repo-files`, `p1-cargo-workspace`, `p1-ci`, `p1-privacy-check`,
  `p1-privacy-fixes`); phase file filled (summary, 8 questions, card rows, 11 orchestrator steps). Read-through fixes
  to `architecture.md`: lock waits for the lid sound (lid sound card's start delay), floor blocks new holds, `screen`
  only in `keep-awake.json`, CLI restore steps, overlapped folder watch was tested on one folder, row 0 and row 9 of
  the decision table, Question 8. Checks run (Result).
- 2026-11-03 15:15: Result written, Status done.

## Result

Done 2026-11-03 15:15. The design waits for Haik's sign-off and his answers to 8 questions (relayed by the
orchestrator).

What was done:

- `docs/architecture.md` rewritten as the detailed design: crates and modules with boundary traits, the core (events,
  state, commands, clocks, episode flow), a 45-row table from every `product.md` decision to its place and phase,
  processes and threads, the Windows layer (requests and re-take, settings ownership, journal, restore routine, guard,
  RunOnce logon restore, lock, hibernate, OS events table, tray and notices), files with versioned formats, the source
  interface and the "update Wakeshot" path, errors and logging, testing (fake clock, fixtures, a never-active plan
  copy, a 14-item manual checklist), CI and targets, dependencies checked today. Each untested point says so.
- Seams, not picks: `MenuView` (expected answer from the menu research: owner-drawn classic menu plus
  `DWMWCP_ROUND`) and `SoundPlayer` (the lid-close lock waits for the sound or a cap that `p1-lid-sound` sets).
- Design values I set (technical, open to Haik at sign-off): Claude timer margin 2 minutes, safety tick 30 s, wait at
  end of session 4 s, log 1 MB with three files, hold `reason` cut to 60 characters.
- Phase 1 cards: `p1-repo-files`, `p1-cargo-workspace`, `p1-ci`, `p1-privacy-check` (reviewer), `p1-privacy-fixes`.
  Also suggested in the phase file: a domain review of the design before sign-off, and a general review of the
  workspace and CI before the first push (reviewers, no cards).
- Phase file: design summary (43 lines with blanks), 8 questions with defaults, card rows, 11 orchestrator steps for
  `git init`, the privacy gate, the first commit, `gh repo create`, the push and the CI check.

Open items from Phase 0, placed:

| # | Item | Where it went |
|---|---|---|
| 1 | Battery sleep while holding | Resolved: Never on DC, written at hold start also on AC (section 5). Lid closed with Never: Phase 3 checklist 1 |
| 2 | Sign-in Never stored value | Design always locks until a test shows it (section 5); test: Phase 3 or later, checklist 12, gated by Question 6 |
| 3 | Re-take after resume | Resolved: clear, then set, same handle, the tested form (section 5). "New before old" is not used, so no test. On battery: Phase 3 checklist 3 |
| 4 | Settings and Control Panel write the real plan while a copy is active | Resolved: edit in place, never a copy (section 5); rule for every Phase 3 test with a copy (section 9) |
| 5 | Velopack kills the app; Setup.exe wipes the install folder | Data folder outside the install folder (section 6, Question 2); hooks run the restore routine; guard death covered by the hooks; Phase 6 checklist 14 |
| 6 | Phase 3 checklist | Section 9, items 1 to 3 and 12, plus the copy rule; "new before old" dropped (not used) |
| 7 | Claude rule 11; rule 5 and `--resume` | Rule 11 dropped (section 7); `--resume` in a new process: Phase 4 live test (section 7) |
| 8 | Menu drawing and sounds | Seams `MenuView` and `SoundPlayer` (section 2); picks stay with `p1-tray-menu-look` and `p1-lid-sound` |

Files changed: `docs/architecture.md` (rewritten), `docs/phases/01-design-skeleton.md` (summary, questions, card rows,
orchestrator steps, my row's status), `docs/research.md` (new last section "Phase 1 design checks"), this card. New:
`docs/tasks/p1-repo-files.md`, `p1-cargo-workspace.md`, `p1-ci.md`, `p1-privacy-check.md`, `p1-privacy-fixes.md`.
Scratch only: `crate-versions.ps1` in my scratchpad.

Acceptance checks:

- Every `product.md` decision maps to a place or a later phase: pass. Table "Decisions and where they live", rows 0 to
  44, made by going through `product.md` section by section.
- Every fact points to research or is marked: pass, with one note. Facts cite `research.md` by section name (grep: no
  `research.md:<line>` pointers left), the research reports in `docs/research/`, or Microsoft pages recorded in
  "Phase 1 design checks". Five tray facts cite `docs/reviews/p0-tray.md` because no research line holds them (Found
  outside scope).
- No product decision made: pass. Eight questions with defaults; the technical values above are listed for Haik.
- Cards follow the template, fit one worker, have Runs after and Parallel with: pass. No two parallel cards share a
  file: repo files (4 root files) vs workspace (Cargo files, `crates/`, one `CLAUDE.md` line) vs CI (`.github/`);
  privacy fixes runs alone. None touches power settings, the screen or sound.
- Nothing outside docs: pass. Repo root has no `.git` and no `Cargo.toml` (checked 15:15); no git run.
- Style: no em or en dashes and no emojis in the nine files written (script), LF line endings.

Open items: Haik's sign-off and answers; the suggested design review; the outside-scope notes above.
