# Rewrite the design docs without planning history

Phase: 8. Status: done (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics` (it may change a contract in `architecture.md`). Parallel with: every card that does
not own these files (group 1; no build). If `p4-live-claude-desktop` is still open, its later edits to these files are
additions that `p8-final-cleanup` sweeps.
Build folder: none.

## Goal

`docs/architecture.md`, `docs/product.md` and `docs/hold-files.md` read as the design, the product rules and the hold
file format of a finished v1: what is true and why, with no trace of cards, tracks, phases as process, sessions or who
said what when.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership".
- The three files, whole. `architecture.md` is about 1,580 lines: read it in parts.
- What points at them, before you rename anything: grep `crates/`, `packaging/`, `.github/`, `site/`, `media/`,
  `README.md`, `CONTRIBUTING.md` and the docs that stay for `architecture`, `product` and `hold-files`.

## Deliverables

1. `docs/architecture.md`:
   - The opening paragraph: what the doc is and how "not tested" is marked; no card names, no sign-off story.
   - Remove card codes, track ownership, `Owner:` explanations, "from `p1-contracts`", "the orchestrator runs alone",
     phase references as process, Haik attributions and dated decision notes. Keep the design and its reasons, and
     every "change it when".
   - Keep the section numbers 1 to 12, their subsection names, the "Tray startup" step numbers and the decision
     numbers 0 to 44 in "Decisions and where they live" (code cites all of them). In that table, drop the Phase
     column and the attributions; keep the decision text and where it lives.
   - `spikes/` is removed in Phase 8 (`p8-repo-config`): section 2's workspace note and "Ported from the spikes" keep
     what was ported and why, without pointers into `spikes/`.
   - Section 9 (Testing) and section 10 (CI): describe the tests and checks as they are, not which card wrote them.
     The manual checklist stays if it is a real release checklist; drop "Haik's" in its name.
2. `docs/product.md`:
   - Decisions as present-tense rules. Drop "(Haik, date)", "for Haik's review", "decided by the worker", "on
     2026-11-03 v1 was ...", and pointers into task cards and phases.
   - "Answers to the track plan questions": fold each answer into its topic section, or keep the section under a plain
     name such as "Detailed rules"; the README and code cite no part of it by name today (check). Record the rename.
   - Keep every section name and the "User settings safety" item numbers (code cites them).
   - The agents out of v1 (VS Code Local chats, Antigravity, Cursor) keep their one-line reason and "revisit when".
3. `docs/hold-files.md`: check it is current against `crates/wakeshot-core/src/holds/file.rs` and the CLI's `run`;
   fix only what is stale (it is user-facing and the README links to it).
4. Result: "Heading renames" (file, old heading, new heading), even if empty; sizes before and after; the decisions you
   folded or merged; anything a code comment cites that you could not keep.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on the three files: zero hits.
2. No em dash (U+2014) in the three files.
3. Every heading, section number, step number, decision number and "User settings safety" number that a file outside
   these three cites still exists, or is in "Heading renames". Show the grep you used.
4. Every relative link in the three files resolves to a file that stays (not `docs/plan.md`, `docs/phases/`,
   `docs/tasks/`, `docs/reviews/`, `spikes/`, a brand report).
5. No decision lost: list the `product.md` decisions before (one line each) and check each against the new text.

## Out of scope

Code, `README.md`, `media/`, `docs/research.md` and `docs/research/`, `docs/brand.md`, `docs/agent-hooks.md`. Git.
A wrong fact you find in a doc you do not own goes under "Found outside scope".

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

1. `docs/agent-hooks.md` line 132 and `docs/research/other-agent-signals.md` line 663 cite `product.md` "Answers to
   the track plan questions", now "Detailed rules" (`p8-final-cleanup`).
2. `p6-winget-upgrade-restart-fix` (running tonight) may change what happens after `winget upgrade`. If it does, two
   lines become stale: `product.md` "Detailed rules" ("After `winget upgrade`, Wakeshot is off until the next sign-in
   or a manual start") and the end of `architecture.md` section 6 "Velopack hooks" ("Accepted ... Wakeshot is off
   until the next sign-in or a manual start"). The architecture line is that card's (hook lines); the product line
   needs whoever closes it, or `p8-final-cleanup`.
3. `architecture.md` cites two `research.md` names that sit in the half `p8-docs-research-2` still renames: "Power
   track manual checks" (twice, section 5 "Logon restore") and "The spike's log clock drifted 16 s" (section 8 "Log").
   Left as written, since both exist today; `p8-final-cleanup` applies that card's renames.
4. `research.md` (second half) still says "Haik's checklist" in "Integration real e2e" (lines about 3776 and 3780);
   the checklist is now named "Manual checklist" in `architecture.md` section 9, numbers unchanged
   (`p8-docs-research-2`).
5. `.github/workflows/release.yml` comments name "Phase 8" and "Haik sets the repository variable" (lines 206 to 208)
   (`p8-repo-hygiene`).

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 04:46: started (worker, session wakeshot-4e). Sizes before: `architecture.md` 151,092 bytes and 1,612
  lines; `product.md` 31,327 bytes and 348 lines; `hold-files.md` 5,415 bytes and 115 lines. Orchestrator added one
  item: `wakeshot diagnostics` joins `run` as a CLI-only exception in `product.md`.
- 2026-11-07 about 04:50: read the three files and what cites them (scripts `cites.py`, `cites2.py`, `rcites.py` in the
  scratch folder). Outside files cite architecture section numbers 2 to 11, 33 paragraph names, "Tray startup" steps
  1, 3, 5, 6, decisions 0 to 44 and checklist items 4, 6, 9; product.md section names, bold labels and "User settings
  safety" 3, 4, 5, 7; no anchors into the three files. Twelve research pointers in `architecture.md` name headings
  renamed tonight. `hold-files.md` checked against `holds/file.rs`, `files.rs`, `watch/data.rs` and `run.rs`:
  current, no change needed.
- 2026-11-07 04:58: `architecture.md` sections 1 to 6 done (opening, section 2 workspace and "Ported from the test
  programs", section 3 attributions and review numbers, the decisions table without its Phase column, sections 4 to
  6). Research pointers moved to tonight's new heading names. Calls for Haik's review so far: "not tested" marks that
  research shows as done now say "tested too" with the checklist number (checklists 1, 3, 6 and the session lock
  notices, all 2026-11-04); the journal's note on builds from before the sleep route keys is cut (no release has
  them); decision 30 and product UX add `diagnostics` beside `run`.
- 2026-11-07 05:01: `architecture.md` sections 7 to 12 done. Stale facts fixed with code checks: stub and "until a
  card" lines (hook command, sources, `HookEnv::remove_agent_hooks`), "all automated tests run in CI" (real-laptop
  tests are `#[ignore]`d), "six commands" (seven plus `hook`), decision 12's Codex note. Section 12 rewritten as the
  design with v1's state last. Next: `product.md`, then line wrap and checks.
- 2026-11-07 05:05: `product.md` rewritten: attributions, card codes, quotes of Haik's words and the status line out;
  "Answers to the track plan questions" kept whole as "Detailed rules"; `wakeshot diagnostics` added to UX. Calls for
  Haik's review: revisit-when lines added for Antigravity and Cursor (derived from their reasons); the author line
  became the repo name (the sweep allows "Haik Asatryan" only in named files; the README names him); the promo video
  is "short (the main cut runs 38 s)", not "30 second"; superseded plans dropped (the cleanup and squash plan, "fully
  public as they are", "OpenCode 1.x if cheap", "Codex and Copilot after v1"). Paragraphs with a line over 120
  columns rewrapped by script (words checked unchanged).
- 2026-11-07 05:07: acceptance checks run (Result). A word diff of each file against its before copy was read in
  full: only the intended removals and rewrites; no line from another card appeared in `architecture.md` while I
  worked.

## Result

Done 2026-11-07 05:08. `architecture.md` and `product.md` read as the design and product rules of v1, with no card
codes, phases as process, sessions, review numbers or "who said what when"; every fact keeps how and when it was
checked, and every "not tested" or "not verified" mark stays unless research shows the check done (below).
`hold-files.md` was checked against the code and is current: not changed.

What changed in `architecture.md`:

- Opening paragraph: what the doc is and how "not tested" is marked; no sign-off story.
- Section 2: the `spikes/` workspace note, the "no track changes `Cargo.lock`" note and the `Owner:` paragraph are
  gone; "Ported from the spikes" became "Ported from the test programs", naming what was tested, not spike paths.
- Section 3: every "(Haik, date)", review or verify number and card code out; the decisions table lost its Phase
  column and attributions; decision 9 lists the v1 agents and the ones out of v1; decision 12 now says only the Codex
  fallback for a Restart Manager failure is not tied to its session (section 7 says so); decision 30 adds
  `diagnostics`; decisions 33, 36, 41, 43, 44 point at real places (`text`, `site/`, `packaging/winget/`,
  `release.yml`) instead of phases.
- Sections 4 to 12: the same clean-up. Stale lines made current, each checked in the code or `research.md`: the
  `hook` command (built, always prints `{}` and exits 0), the sources (no stubs; the tray starts Claude Code, Codex,
  Copilot CLI and OpenCode, `tray_mode.rs` lines 577 to 582), `HookEnv::remove_agent_hooks` (no stub), "all
  automated tests run in CI" (tests that need the real laptop are `#[ignore]`d), "six commands" (seven plus `hook`),
  the sound wait (at most 3 s, `play_and_wait`), icon art (`tray::art`).
- "Not tested" marks that `research.md` "Power track manual checks" shows done on 2026-11-04 now say "tested too"
  with the checklist number: re-take on battery (checklist 3), battery sleep Never with the lid closed (checklist 1),
  a real sign-out (checklist 6), session lock and unlock notices. Console left and back stays "no (checklist 11)".
- Section 9: Phase labels and the checklist's Phase column out; "Manual checklist for Haik" is "Manual checklist".
- Section 10: the spike build note and "the tray spike's settings" out.
- Section 12: the design first, v1's state (no hooks added) last; the stub list became the list of shared shapes.
- Twelve research pointers follow tonight's renames: "Power rules checks", "Power track real checks" (six times),
  "Process module checks", "Store checks", "Log writer checks", "Phase 1 design checks", "Cron checks", and
  `claude-code-signals.md` "Phase 4 live shapes" (now "Live shapes"). `agent-hooks.md` "contract change 6" points
  at "The hook command" step 5 and "Known risks".

What changed in `product.md`: the status line, every attribution, card code, quote of Haik's words and pointer into
`phases/` out; `wakeshot diagnostics` added to UX as a CLI-only support command beside `run`; "Answers to the track
plan questions" kept whole as "Detailed rules", its stale first line ("apply when they are built, after v1") dropped;
"Haik's laptop" and "this laptop" became "the test laptop".

### Heading renames

| File | Old | New | Cited by |
|---|---|---|---|
| `architecture.md` | `### Ported from the spikes` | `### Ported from the test programs` | nothing outside |
| `architecture.md` | paragraph label "Manual checklist for Haik on the test laptop." | "Manual checklist: checks that need a person at the test laptop." | `research.md` cites "checklist 4", "6", "9" and "Haik's checklist"; numbers 1 to 16 unchanged |
| `product.md` | `### Answers to the track plan questions (Haik, 2026-11-03 18:09: all defaults)` | `### Detailed rules` | `docs/agent-hooks.md` line 132, `docs/research/other-agent-signals.md` line 663 |
| `product.md` | bullet label "No guessing (Haik, ...)" | "No guessing:" | the words "No guessing" unchanged |

Kept as they were: section numbers 1 to 12 and every subsection name, the paragraph names code cites (33 of them,
"Tray threads" to "State is one struct"), "Tray startup" steps 1 to 8, Apply steps 1 to 6, restore steps 1 to 7,
decisions 0 to 44, checklist 1 to 16, every `product.md` section name and bold label, "User settings safety" 1 to 7.
Nothing a code comment cites was lost.

### Sizes

| File | Before | After |
|---|---|---|
| `architecture.md` | 151,092 bytes, 1,612 lines | 145,663 bytes, 1,579 lines |
| `product.md` | 31,327 bytes, 348 lines | 28,813 bytes, 322 lines |
| `hold-files.md` | 5,415 bytes, 115 lines | unchanged |

### Decisions folded, merged or dropped

None folded or merged: "Detailed rules" keeps all 13 answers. Dropped as superseded: "OpenCode 2.x first, 1.x only if
cheap" (v1 supports 2.x only, kept); "fully public from the first push: docs, planning, reviews and spikes as they
are" and "a cleanup phase ... squashes the history" (the launch cleanup carries them out; "secrets never, a secret
scan before each push" kept); "the two Codex and Copilot answers apply when they are built, after v1" (both are in
v1). Changed: the promo video is "short (the main cut runs 38 s)", from `promo-video.md`. Added: `diagnostics`
beside `run`; a revisit-when line for Antigravity and for Cursor.

### Acceptance checks

1. Internal-name sweep (`sweep.py`: the phase file's patterns plus `phases/`, `reviews/`, `plan.md`, "Phase N",
   "card", "worker", "Question N"): one hit, "the social card" in `product.md` UX, plain English (the README's social
   card picture). The phase file's own patterns: zero hits. Pass.
2. Em dash (U+2014): zero in the three files. Pass.
3. Cited names and numbers: `check3.py` checks 46 `architecture.md` names, 17 `product.md` names, sections 1 to 12,
   decisions 0 to 44, startup steps 1 to 8, checklist 1 to 16, "User settings safety" 1 to 7, Apply and restore
   steps: 0 missing. The names came from `cites.py` and `cites2.py`, which grep every kept file outside the planning
   folders for quoted names after "architecture", "product" or "section N", and for decision, step and safety
   numbers. The one rename others cite ("Detailed rules") is in the table above. Pass.
4. Links: the three files have no Markdown links; every path they name (`docs/agent-hooks.md`, `docs/brand.md`, the
   six `research/*.md` reports, `research.md`, `README.md`, `CLAUDE.md`, `packaging/` scripts and `winget/`, the
   two e2e test files, `assets/wakeshot.ico`, the fixtures folder) exists; none points into `docs/plan.md`,
   `docs/phases/`, `docs/tasks/`, `docs/reviews/`, `spikes/` or a brand report (`CLAUDE.md` was dropped as a pointer
   for the unknown-status rule, since it gets cut later; `product.md` UX holds it). Research pointers: `rcites.py`
   finds every quoted `research.md` and report name. Pass.
5. No decision lost: the 79 decision lines of the old `product.md`, each checked against the new text with a word
   diff (`product.wdiff.txt` in the scratch folder): The job 1 to 3, Platform 4 to 6, Sources 7 to 29 (Claude Code
   rules, other agents in and out, OpenCode, logos, Gemini, live tests, `run`, Keep awake, facts not time caps, no
   guessing, the Antigravity code, later, no plugins, hooks), Power 30 to 38 (requests, re-take, two settings, never
   lower security, sounds, closed-lid report, battery floor, no heat guard, work end), User settings safety 39 to 45
   and the crash gap 46, UX 47 to 58, Distribution 59 to 65, Detailed rules 66 to 78: all present; the four dropped
   as superseded are listed above. Pass. The list, old text in short, each one present in the new file unless marked:

   1 awake only while something needs it, keep awake on demand, lid closed a feature; 2 the user always understands
   (icon, notice or sound, routine holds in icon and tooltip only); 3 no work: the user's settings decide; 4 Windows
   11, x64 and ARM64, English; 5 Rust with windows-rs, runner-up and switch trigger; 6 macOS and Linux long term;
   7 Claude Code from status files and transcript, listed fields, no hooks; 8 `busy` holds; 9 timer within 1 hour
   holds; 10 `waiting` only with Remote Control, 1 hour; 11 `shell` and background tasks hold while the process
   lives; 12 task ends with no transcript line; 13 Codex CLI, Copilot CLI, Copilot in VS Code, OpenCode 2.x in v1;
   14 VS Code Local chats out, revisit when; 15 Antigravity out; 16 Cursor out; 17 OpenCode 2.x only (dropped: "1.x
   if cheap"); 18 no agent logos; 19 `opencode.db` read only, two tables, with a test; 20 Gemini dropped, Aider, Amp,
   Kiro not planned, ports with credit; 21 every agent tested live or marked; 22 `wakeshot run` and the hold file;
   23 Keep awake; 24 signals checked against facts, two time limits only; 25 no guessing, Claude's cancelled wakeup;
   26 the Antigravity code kept, `agy` log rules; 27 app rules and WSL later; 28 no plugins or scripting; 29 hooks
   rule, none in v1; 30 SYSTEM plus EXECUTION, display for Keep screen on; 31 re-take after resume; 32 lid action and
   battery sleep Never only, also on AC; 33 never lower security (lid lock, Keep screen on lock, unplug lock, failed
   lock step back, no lock in docked use or for a hold that starts closed); 34 sounds and the device rule;
   35 closed-lid report; 36 battery floor and its 30 seconds; 37 no time limit, no heat guard; 38 work end with the
   lid closed (5 minutes, 1-minute timeouts, hibernate, relock); 39 to 45 "User settings safety" 1 to 7; 46 crash gap
   accepted; 47 no window, menu covers the CLI except `run` (now also `diagnostics`); 48 CLI commands (now with
   `diagnostics`); 49 Automatic; 50 Keep awake choices; 51 menu sketch; 52 icon states and menu look; 53 tooltip and
   menu name what holds; 54 first run pin notice; 55 "update Wakeshot"; 56 restore fails warning; 57 no Pause and
   the like; 58 no telemetry; 59 public repo, MIT, secrets never, Pages (dropped: "planning, reviews and spikes
   public as they are" and the cleanup and squash plan); 60 README rules; 61 promo video (now 38 s); 62 pack id and
   data folder; 63 Velopack per user, hooks restore, `run` wrapper on upgrade; 64 winget, scoop, Store; 65 signing;
   66 Codex waiting holds; 67 Copilot `permission.requested` does not; 68 chimes honor the sound scheme; 69 notices
   wait, sounds never; 70 tray banners, not toasts; 71 off after `winget upgrade`; 72 uninstall data folder rule;
   73 update notice; 74 `NotifyIconSettings` left; 75 a pre-release tests the release workflow; 76 two users;
   77 failed guard or sign-in restore starts the tray; 78 Remote Desktop not tested; 79 "Codex and Copilot answers
   apply after v1" (dropped: both are in v1).

### Calls for Haik's review

1. `wakeshot diagnostics` is CLI-only, like `run` (orchestrator's call; `product.md` UX and decision 30).
2. "Answers to the track plan questions" stays one section, renamed "Detailed rules", rather than folded into the
   topics: two docs cite it, and its answers are short rules with no single home.
3. Stale "not tested" marks with research evidence now say "tested too" (checklists 1, 3, 6; session lock notices).
4. The journal note on builds from before the sleep route keys is cut: no release has such a build.
5. New revisit-when lines for Antigravity ("Esc fires a hook, or the cancel line stays the same across `agy`
   releases") and Cursor ("a chat deleted mid-turn sends its own end").
6. `product.md` no longer names the author (the sweep allows "Haik Asatryan" only in listed files); it names the
   repo. The README names him.
7. The promo video line follows the built cut (38 s), not the planned 30 s.

Files changed: `docs/architecture.md`, `docs/product.md`, this card. Scratch scripts, before copies and word diffs
are in the session's scratch folder (`p8-docs-design\`). Open items: "Found outside scope" 1 to 5.
