# Clean the research reports and the agent hooks doc

Phase: 8. Status: done (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
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

1. `p8-final-cleanup` (orphan pictures): `tray-menu-look.md` names its 36 `docs/research/img/tray-look-*.png`
   pictures by pattern (`tray-look-<theme>-ours-classic.png` and so on), not one by one. An orphan check that matches
   whole file names would call them orphans; they are the report's evidence. It names
   `tray-menu-switch3-light-flush.png` and `tray-menu-switch4-dark-flush.png` in full.
2. `p8-final-cleanup` (pointers): the reports cite `docs/research.md` sections by name ("Velopack checks for
   Wakeshot", "Cursor hook payload form", "Antigravity hook payload form", "Cron checks", "Claude transcript
   checks", "Claude UI stop notice checks", "Reading elevated Claude Code sessions from an unelevated process",
   "Medium reader, High `claude.exe`", "Story checks", "Install end to end checks", "Tray size", "OpenCode SQLite
   reader checks", "Live checks", "Windows power facts", "Tray, menu and notifications"). They stay as written;
   the renames of `p8-docs-research-1` and `-2` apply to these files too.
3. Open card `p4-live-claude-desktop` may still append its live part to `claude-code-signals.md`, "Claude Desktop
   (2026-11-07)" (this card cleaned that section: no card path, "the test laptop", "not run" in place of "needs
   Haik"). What it adds must follow the polish rules, or `p8-final-cleanup` sweeps it.

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 04:17: started (worker, session wakeshot-4e). Status in progress.
- Step 1: baseline saved in the scratch folder (copies, sizes, sweep hits 232, "not verified"/"not tested" counts per
  file). Pointers from kept files listed: code and `media/` cite many headings of `agent-hooks.md` and
  `promo-video.md`, plus numbered items ("The hook command" steps 4 to 6, "contract change 6", "risk 19",
  `distribution.md` sections 1 and 2, `launch.md` parts 1, 2 and 4 with rules 2, 3 and 5, `rivals-others.md` trap 12).
- Step 2: `agent-hooks.md` done: 74.5 KB to about 62 KB, sweep 0. Cut "Contract changes the build needs" (its one fact
  code cites, contract change 6, now sits in "The hook command" step 5 and "Known risks"), "Live test (for the live
  card)" (plan; results are in the live test section), the 25-row risk table (settled; four open or accepted risks kept
  as "Known risks"). Call for Haik's review: "risk 19" (cited by `wakeshot-cli/tests/hook.rs`) goes with the table; its
  fact is "The hook command" step 4.
- Step 3: `claude-code-signals.md` done (65.6 to 60.8 KB, sweep 0): cut "Proposed rule changes" and "Rule changes"
  (drafts already applied to the rules table; the one fact kept moved into the text), trimmed the step list written for
  a person in "Live tests still open". `other-agent-signals.md` done (sweep 0); its two live test headings got distinct
  names. Call for Haik's review: `Codex live test (2026-11-06)` and `Copilot CLI live test (2026-11-06)` instead of
  two `Live test (2026-11-06)`, so neither anchor depends on order (`-1`).
- Step 4: `copilot-vscode-signals.md`, `opencode-signals.md`, `antigravity-signals.md` done (sweep 0 each). Cut:
  opencode "What is left" (a live-test plan the live test ran), antigravity "Sign-in steps for Haik's one sign-in
  sitting" (steps for a person). "For the source card" becomes "Source sketch" in both reports that had it.
- Step 5: `cursor-gemini-signals.md` (cut "6. Product question for Haik", answered; "What is left" rewritten to what is
  still untested), `distribution.md`, `launch.md` done. `launch.md`: cut "Draft table for `p6-readme`" (the README
  table replaced it; every cell's fact is in "Facts per rival"), merged the superseded sound pick into "As built".
  Its two sweep hits are `Haik Asatryan` as the winget `Publisher` value, a data value like the winget templates:
  kept on purpose.
- Step 6: `promo-video.md` (55.5 to 43.5 KB; cut "Why a new plan", "How to brief the agent that builds it", "Effort and
  risks", "Questions for Haik"; the round-by-round music history folded into one present-tense account),
  `readme.md` (cut "The outline it leads to", replaced by the README itself), `rivals-*.md`, `tray-menu-look.md` (cut
  the instructions for the on-screen worker) done. "this laptop" became "the test laptop" in every owned report.
- 2026-11-07 04:43: steps 1 to 6 ran between 04:17 and 04:43; the clock was read only at those two times, so the
  step lines carry no time. Then: paragraphs with a new line over 120 columns rewrapped (a check showed the words
  unchanged in every file), the acceptance checks run, Result written. Status done.

## Result

Done. The 13 reports and `docs/agent-hooks.md` keep their facts, sources, dates and marks, and lose card codes,
sessions, "Haik said", review numbers, pointers into removed paths and the plans that the code or the README replaced.
"Haik's laptop", "this laptop" and the like became "the test laptop" (or "the user's own" where it meant his apps and
files). No other file was edited.

### Heading renames

For `p8-final-cleanup`. "Cited by" lists the kept files that point at the old name today (from a grep of the repo).

| File | Old heading | Old anchor | New heading | New anchor | Cited by |
|---|---|---|---|---|---|
| `claude-code-signals.md` | Phase 4 live shapes (2026-11-03) | `#phase-4-live-shapes-2026-11-03` | Live shapes (2026-11-03) | `#live-shapes-2026-11-03` | `architecture.md`, `research.md`, `crates/wakeshot-core/tests/fixtures/claude/live/README.md` |
| `claude-code-signals.md` | Phase 4 end to end (2026-11-04) | `#phase-4-end-to-end-2026-11-04` | End to end (2026-11-04) | `#end-to-end-2026-11-04` | README (anchor), `research.md`, `crates/wakeshot-core/src/sources/claude/mod.rs` |
| `other-agent-signals.md` | Phase 4 live test (2026-11-06), under Codex | `#phase-4-live-test-2026-11-06` | Codex live test (2026-11-06) | `#codex-live-test-2026-11-06` | README line 226 (anchor), `research.md`, `crates/wakeshot-core/tests/fixtures/codex/live/README.md` |
| `other-agent-signals.md` | Phase 4 live test (2026-11-06), under GitHub Copilot CLI | `#phase-4-live-test-2026-11-06-1` | Copilot CLI live test (2026-11-06) | `#copilot-cli-live-test-2026-11-06` | README line 234 (anchor), `research.md` (twice), `crates/wakeshot-core/tests/fixtures/copilot/live/README.md` |
| `copilot-vscode-signals.md` | Phase 4 live test (2026-11-06) | `#phase-4-live-test-2026-11-06` | Live test (2026-11-06) | `#live-test-2026-11-06` | `research.md`, `crates/wakeshot-core/tests/fixtures/vscode/live/README.md` |
| `copilot-vscode-signals.md` | Built (`p4-vscode-source`, 2026-11-06) | `#built-p4-vscode-source-2026-11-06` | Built (2026-11-06) | `#built-2026-11-06` | `research.md` cites "Built": still matches |
| `copilot-vscode-signals.md` | For the source card | `#for-the-source-card` | Source sketch | `#source-sketch` | none ("Fixture sketch" under it is unchanged) |
| `opencode-signals.md` | Phase 4 live test (2026-11-06) | `#phase-4-live-test-2026-11-06` | Live test (2026-11-06) | `#live-test-2026-11-06` | README (anchor), `research.md`, `crates/wakeshot-core/tests/fixtures/opencode/live/README.md` |
| `opencode-signals.md` | Built (Part B, 2026-11-06) | `#built-part-b-2026-11-06` | Built (2026-11-06) | `#built-2026-11-06` | none |
| `antigravity-signals.md` | Phase 4 live test (2026-11-06) | `#phase-4-live-test-2026-11-06` | Live test (2026-11-06) | `#live-test-2026-11-06` | `research.md` |
| `antigravity-signals.md` | For the source card | `#for-the-source-card` | Source sketch | `#source-sketch` | none |
| `agent-hooks.md` | Phase 4 live test (2026-11-06) | `#phase-4-live-test-2026-11-06` | Live test (2026-11-06) | `#live-test-2026-11-06` | `research.md` ("Phase 4 live test") |
| `agent-hooks.md` | Exact end (`p4-antigravity-exact-end`, 2026-11-06) | `#exact-end-p4-antigravity-exact-end-2026-11-06` | Exact end (2026-11-06) | `#exact-end-2026-11-06` | none outside (cited as "Exact end", still matches) |
| `agent-hooks.md` | Risks for the security review | `#risks-for-the-security-review` | Known risks (most rows cut) | `#known-risks` | `crates/wakeshot-cli/tests/hook.rs` "risk 19": the row is cut; its fact is "The hook command", step 4 |
| `launch.md` | Speed numbers: how `p6-media` measures each | `#speed-numbers-how-p6-media-measures-each` | Speed numbers: how each is measured | `#speed-numbers-how-each-is-measured` | `research.md`, `media/scripts/measure-speed.ps1` |
| `launch.md` | Rules for `p6-winget-manifest` and `release.yml` | `#rules-for-p6-winget-manifest-and-releaseyml` | Rules for the winget manifest and `release.yml` | `#rules-for-the-winget-manifest-and-releaseyml` | `packaging/winget/README.md` line 13; rule numbers 1 to 9 unchanged |
| `launch.md` | Round 2: popular tools only (2026-11-04, `p6-readme` round 2) | `#round-2-popular-tools-only-2026-11-04-p6-readme-round-2` | Round 2: popular tools only (2026-11-04) | `#round-2-popular-tools-only-2026-11-04` | none |
| `launch.md` | Haik's three conditions | `#haiks-three-conditions` | Three conditions for the Store | `#three-conditions-for-the-store` | none |
| `promo-video.md` | What Part B captures from the real app | `#what-part-b-captures-from-the-real-app` | What is captured from the real app | `#what-is-captured-from-the-real-app` | none ("Real captures and drawn parts" unchanged) |
| `promo-video.md` | Remotion parts to add (in `media/src/`) | `#remotion-parts-to-add-in-mediasrc` | Remotion parts (in `media/src/`) | `#remotion-parts-in-mediasrc` | `media/src/parts/Parts.tsx` cites the cut "How to brief the agent that builds it", step 3: point it here (the `parts/Parts.tsx` line says what step 3 said) |
| `tray-menu-look.md` | Why it looks like that (spike code) | `#why-it-looks-like-that-spike-code` | Why it looks like that | `#why-it-looks-like-that` | none |
| `tray-menu-look.md` | How the pick behaves (spike, on screen) | `#how-the-pick-behaves-spike-on-screen` | How the pick behaves (on screen) | `#how-the-pick-behaves-on-screen` | none |
| `tray-menu-look.md` | To check on screen | `#to-check-on-screen` | Not checked on screen | `#not-checked-on-screen` | none |
| `distribution.md` | Trial on this laptop (2026-11-03, all verified) | `#trial-on-this-laptop-2026-11-03-all-verified` | Trial on the test laptop (2026-11-03, all verified) | `#trial-on-the-test-laptop-2026-11-03-all-verified` | none |
| `rivals-lidguard.md` | 1.3 Claude Code status file today (this laptop, ...) | `#13-claude-code-status-file-today-this-laptop-claude-code-21314-and-21316` | 1.3 ... (the test laptop, ...) | `#13-claude-code-status-file-today-the-test-laptop-claude-code-21314-and-21316` | none |
| `promo-video.md` (H1) | The promo video: style, storyboard and build plan | | The promo video: style, storyboard and build | | none |

Numbered items others cite, moved or cut:

- `agent-hooks.md` "contract change 6" (cited by `crates/wakeshot-windows/src/store.rs` line 142 and
  `docs/architecture.md` line 1082): the section is cut; the fact is now in "The hook command", step 5 (the temp file
  shape and `CREATE_NEW`) and "Known risks" (the junction gap). Point both there.
- `agent-hooks.md` "risk 19" (cited by `crates/wakeshot-cli/tests/hook.rs` line 3): see the table.
- Kept unchanged: "The hook command" steps 0 to 6, "Source turned off", "How the build checks it", "The `wakeshot hook`
  command form is frozen", every heading the code cites; `distribution.md` sections 1 and 2 and "x64 and ARM64";
  `launch.md` parts 1 to 4 and `#1-rivals-for-the-readme-table`; `rivals-others.md` trap 12; `promo-video.md`
  "Motion" (with its Whip and Focus rows), "Sound", "Text", "Colour and stage", "Storyboard: the 38 s main cut",
  "Real captures and drawn parts", "Music and effects", "Agent logos", "Formats"; `opencode-signals.md` "Probe";
  the last section of `copilot-vscode-signals.md` and `antigravity-signals.md`.

New pointers this card added, both into `product.md` (no new pointer into `research.md`): `other-agent-signals.md` to
"Answers to the track plan questions" (that heading may be renamed by `p8-docs-design`); `copilot-vscode-signals.md`
to "What keeps the PC awake" (it replaced a line-number pointer, `product.md:33`). Line-number pointers into
`product.md` and `research.md` in `claude-code-signals.md` and `rivals-*.md` became section or file pointers.

### Sections cut, per file

- `agent-hooks.md`: "Contract changes the build needs" (all six built; the shapes are in the code), "Live test (for
  the live card)" (the plan; results are in "Live test (2026-11-06)"; the one item not run, a folder named with `ё`,
  `ւ` and a space, is now a "not tested" line), the 25-row risk table (settled by the design and its tests; four open
  or accepted risks kept as "Known risks"). Also cut inside sections: "Review fixes" labels, product question
  numbers (answered or moot), the Antigravity build-card plan paragraph. `tray_mode::hooked_sources` no longer exists
  in the code, so the doc says the tray starts neither hooked source in v1. Facts made current: a chat deleted while
  it runs is now "seen live" (it was "not run"; the doc's own "Out of v1" already said so), and both payload forms
  passed the Privacy gate.
- `claude-code-signals.md`: "Proposed rule changes" and "Rule changes" (drafts applied to the rules table; the
  `updatedAt` fact and the resume divider kept in the text); the step list written for a person in "Live tests still
  open" (the open cases kept as facts).
- `copilot-vscode-signals.md`: none whole; process lines out.
- `opencode-signals.md`: "What is left" (the live-test plan; the live test ran it; what was not tested is in "Not
  tested live").
- `antigravity-signals.md`: "Sign-in steps for Haik's one sign-in sitting" (steps for a person).
- `cursor-gemini-signals.md`: "6. Product question for Haik" (answered in `product.md`); the superseded helper-exe
  paragraph of section 5 (the per-agent hook facts kept); "What is left" rewritten to what is still untested.
- `launch.md`: "Draft table for `p6-readme`" (the README table replaced it; each cell's fact is in "Facts per rival");
  the superseded sound pick (merged into "As built"). The video spec row now says the cut runs 38 s.
- `promo-video.md`: "Why a new plan" (history), "How to brief the agent that builds it" (its one fact others use,
  step 3, now sits with `parts/Parts.tsx` in "Remotion parts"; the render limit moved under "Tools besides
  Remotion"), "Effort and risks", "Questions for Haik" (all answered); the round-by-round music history, folded into
  one present-tense account with every licence, number and source kept.
- `readme.md`: "The outline it leads to" (the README itself replaced it).
- `tray-menu-look.md`: the instructions for the on-screen worker at the end ("To check on screen" list); its first
  paragraph kept as "Not checked on screen".
- `distribution.md`, `other-agent-signals.md`, `rivals-lidguard.md`, `rivals-others.md`: none whole; process lines out.

### Reports suggested for removal

None. `readme.md` is the only one no kept file cites (the grep hits for it were the card name
`p6-agents-video-readme.md`), but it holds the sources and rules behind the README's shape and four checked facts behind
its install lines, so I suggest keeping it. The orchestrator decides.

### Sizes (bytes)

| File | Before | After |
|---|---|---|
| `agent-hooks.md` | 74,453 | 62,151 |
| `antigravity-signals.md` | 39,644 | 38,249 |
| `claude-code-signals.md` | 65,618 | 60,845 |
| `copilot-vscode-signals.md` | 48,057 | 48,028 |
| `cursor-gemini-signals.md` | 25,332 | 24,222 |
| `distribution.md` | 21,218 | 21,222 |
| `launch.md` | 56,158 | 53,926 |
| `opencode-signals.md` | 28,590 | 28,015 |
| `other-agent-signals.md` | 53,364 | 53,380 |
| `promo-video.md` | 55,485 | 43,507 |
| `readme.md` | 11,597 | 10,491 |
| `rivals-lidguard.md` | 20,226 | 20,219 |
| `rivals-others.md` | 24,487 | 24,460 |
| `tray-menu-look.md` | 38,186 | 35,095 |
| All 14 | 562,415 | 523,810 |

### Acceptance checks

1. Internal-name sweep (the phase file's patterns) on the 14 files: 232 hits before, 2 after, both in `launch.md`:
   `Haik Asatryan` as the winget `Publisher` value (the "AppsAndFeaturesEntries" row and rule 4). It is a data value,
   the same text as the winget templates, which the phase file allows; changing it would make the rule wrong. Pass
   with that exception.
2. No em dash (U+2014) in any of the 14 files: pass (0 before, 0 after).
3. Cited headings and anchors: a script listed every citation of the 14 files from kept files and checked the quoted
   name or anchor. 19 point at an old name, all in "Heading renames" above (the four README anchors, the live test
   and live shapes headings, "Phase 4 end to end", and `Parts.tsx`'s cut section); every other citation resolves.
   Citations the script does not parse (numbered items, single quotes, names before the file name) were checked by
   hand: steps 4 to 6, "Source turned off", "The `wakeshot hook` command form is frozen", "Real captures and drawn
   parts", "Probe", `launch.md` parts and rules, `distribution.md` sections, trap 12: present. Pass.
4. Links and picture paths: no Markdown link or picture in the 14 files points at a missing file, a removed path or a
   brand report; the two pictures named in full (`img/tray-menu-switch3-light-flush.png`,
   `img/tray-menu-switch4-dark-flush.png`) exist; no mention of `spikes/`, `docs/tasks`, `docs/phases`,
   `docs/reviews` or `docs/plan.md` is left. Backtick paths the script flagged are third-party paths (the Antigravity,
   VS Code and Gemini docs, other repos' crates) and `media/public/logos/`, a folder the doc says is used only once a
   logo is allowed. Pass.
5. Marks: "not verified" / "not tested" 230 before, 236 after; "unverified" / "untested" counts unchanged per file. A
   script listed each old mark whose words changed: all five are still present with new words around them. Pass.

Also: every paragraph that got a new line over 120 columns was rewrapped by a script, and a check showed the words of
each file unchanged by it.

### Calls for Haik's review

1. Distinct live-test headings in `other-agent-signals.md` ("Codex live test", "Copilot CLI live test") instead of two
   "Live test (2026-11-06)", so neither anchor depends on order. Change it when a plain name is preferred: both
   anchors then become `#live-test-2026-11-06` and `-1`.
2. `agent-hooks.md` keeps only four open or accepted risks of the 25-row security list; the settled ones live in the
   design text and the tests. Change it when a reviewer wants the full table back: it is in the history bundle.
3. `readme.md` kept, not suggested for removal (see above).
4. Spike programs are now described as throwaway programs, not kept (the phase file's wording for `research.md`), in
   `tray-menu-look.md`, `launch.md`, `claude-code-signals.md`, `other-agent-signals.md` and `opencode-signals.md`.
5. Superseded research stays where its facts are still the only source: the old OpenCode section in
   `other-agent-signals.md` (marked superseded, and `opencode-signals.md` cites it) and the per-agent hook table in
   `cursor-gemini-signals.md` section 5.

### Files

Changed: `docs/agent-hooks.md`; `docs/research/antigravity-signals.md`, `claude-code-signals.md`,
`copilot-vscode-signals.md`, `cursor-gemini-signals.md`, `distribution.md`, `launch.md`, `opencode-signals.md`,
`other-agent-signals.md`, `promo-video.md`, `readme.md`, `rivals-lidguard.md`, `rivals-others.md`,
`tray-menu-look.md`; this card. Created or deleted in the repo: none. Scratch scripts and copies are in the session's
scratch folder.
