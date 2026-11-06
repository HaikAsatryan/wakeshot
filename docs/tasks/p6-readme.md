# The v1 README

Phase: 6. Status: blocked: draft done; waits for `p5-haik-check` and `p6-media`. Needs Haik: yes: he reads the result at the Phase 6 sign-off. The README states his answers
to Phase 1 Questions 10 to 12 (`docs/phases/01-design-skeleton.md`) and to Questions 1 to 3 of `p6-plan-ship`, plus
sign-off item 12: all answered with the defaults (`docs/phases/02-06-signoff.md`, Haik 2026-11-03 18:09). Agent:
`wakeshot-worker`. Size: about 70K tokens.
Real laptop: no.
Runs after: `p6-pack-and-release` (install steps) and `p6-install-e2e` (what an uninstall really
leaves); the sources track's `p4-hold-files` (`docs/hold-files.md`), `p4-cli-args` (the command help), `p4-live-claude`
(what is tested live; v1 supports Claude Code only, Codex and Copilot come after v1, Haik 2026-11-03); the tray
track's `p5-haik-check` (final menu words; review m2); the brand cards `p6-brand-apply-*` (the icon, the header
lockup and the social card, done 2026-11-05: `docs/brand.md`); `p6-launch-research` (rival pick and table facts,
README shape; its Result may change this card, the orchestrator routes that first); `p6-media` (speed numbers,
pictures, loop, video). Parallel with: any card that does not own `README.md`.
The README also says that Windows keeps its own record of the tray icons it has seen (`NotifyIconSettings`), which an
uninstall leaves (`p6-hooks`, review m17).
Build folder: none (no Rust).

## Goal

A README for users that sells Wakeshot and is never boring (Haik, 2026-11-04: long and detailed is fine), and still
says how to install it, what it changes on their PC and every case where a change can stay behind (decision 41).

## Read first

- `docs/product.md` (all of it: it is short); `docs/architecture.md` section 3, "Decisions and where they live", row
  41; section 5, "Gap left"; section 6, "Velopack hooks".
- `docs/phases/06-ship.md`: "Package and install values", "Hooks", "Updates".
- The answers named above (`docs/phases/02-06-signoff.md`); `docs/hold-files.md`; `crates/wakeshot-cli/src/args.rs`
  (the help text).
- `README.md` as it is now (`p1-repo-files`).

## Deliverables

`README.md`, owned whole, in plain short English, no em dashes, no emojis. Shape (`product.md`, Distribution,
README line):

- The top, before any folded section: one line of what we solve (the PC stays awake while an AI agent works, then
  sleeps as usual with the user's own settings back); one line that keep awake on demand comes with it, so nobody
  needs a second tool; the silent icon loop from `p6-media`; the speed numbers from `p6-media` (each linked to its
  method in `docs/research.md`); the install command; Windows 11, x64 and ARM64, per user, no admin; macOS planned
  for later (no date).
- Then the side by side table against the 2 or 3 rivals `p6-launch-research` picked: stack, size, speed where
  published, features. Every rival cell has a source link and the date it was read; no claim the research does not
  back.
- Then the 30 s video (the way the research found GitHub plays it), and the menu and tooltip pictures, light and dark
  (`<picture>`).
- Then a short contents list, and the details below in folded `<details>` sections, each with a plain title:
- Install: `Setup.exe` from GitHub Releases (unsigned in v1: what SmartScreen shows and how to go on); winget once it
  is there (Phase 8 adds the line).
- What it changes and when: power requests while something holds; the lid action and battery sleep (Never) while
  holding, written at the start and put back after; the saved original values; `wakeshot restore` and the menu item.
- The menu, Automatic, Keep awake; start at login and how to turn it off (Settings > Apps > Startup).
- The commands (from the CLI help) and a link to `docs/hold-files.md`.
- Updates: automatic from GitHub Releases, applied only while nothing holds; one notice after an update (Question
  3: yes); GitHub is asked twice a day; no telemetry; the log stays on the PC.
- Limits, each as the answers leave it: the crash gap and every path into it (`architecture.md` section 5, "Gap
  left", and Question 11); one signed-in user per PC (Question 10); a failed restore outside the tray (Question 12);
  `winget upgrade` ends a running `wakeshot run` wrapper while its command goes on; after `winget upgrade` Wakeshot is
  off until the next sign-in (Question 1).
- Uninstall: what it puts back and removes, and what stays (Question 2), including Windows' own record of the tray
  icons (`NotifyIconSettings`, sign-off item 12: left in place).
- A credit line for each port from a rival (none so far: `docs/phases/04-sources.md`, note 17); license.

## Acceptance checks

- A checklist in the Result: every item above and every README promise in `product.md` and `architecture.md`
  (search both for "README"), each with the line that covers it.
- Every fact in the README is in `product.md`, `architecture.md` or `research.md`; the Result lists the source for
  each limit line.
- A script checks that every relative link resolves, and that the file has no em dash and no emoji.
- The top (everything before the rival table) is at most 15 lines of Markdown, not counting image tags.
- Every speed number and rival cell matches `docs/research.md` or `docs/research/launch.md` (the Result lists each).

## Out of scope

- The Pages page (`p6-pages`); the Phase 8 trim; any doc but `README.md`.

## Questions for Haik

Each with the worker's default.

None that blocks. Every open point was decided by the worker, for Haik's review (Log, 04:20).

## Found outside scope

Problems seen but not fixed.

1. This card's deliverable cites `docs/phases/04-sources.md` "note 16" for "no rival port"; the note is 17.
2. Phase 1 Question 11 (`docs/phases/01-design-skeleton.md`) says the uninstall restore fails "for its whole 30 s";
   `architecture.md` section 5 ("Gap left", step 7) and section 6 give the hook's inner limit as 25 s. The README
   says 25 seconds.
3. `architecture.md` section 5, restore step 7, still ends "`hooks.rs` is built and tested with fakes; `velo.rs` does
   not call it yet." Stale: `p6-velopack-dependency` wired the callbacks and `p6-install-e2e` ran them for real. It
   may be the "section 5 ('the hooks are not built yet')" item already queued for docs pass round 3 (`06-ship.md`
   work log 02:05).
4. `docs/product.md` Distribution still says "Whether Velopack accepts a dot in the pack id: not verified; Phase 6
   checks it"; `architecture.md` section 6 records it as tested (research "Velopack checks for Wakeshot", point 1).
5. No doc records what SmartScreen shows for an unsigned `Setup.exe` or how to go on, though this card asks the README
   to say it. A capture at the first public pre-release (`v0.1.0-rc.1`) would settle it; the README holds a `todo:`
   comment at that spot.
6. Not verified anywhere: whether start at login stays off after `winget upgrade` or a new `Setup.exe` over the
   install, which writes the Run value again (`06-ship.md` "Package and install values" marks it not verified). The
   README only says how to turn it off.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04: scope widened by Haik's launch request: sells, never boring, speed shown, rival table, macOS later,
  pictures, loop and video; details stay, folded.
- 2026-11-04 04:02: started as a draft by the orchestrator's decision (session wakeshot-12): `p5-haik-check` and
  `p6-media` are not done. Product text comes from the code as it is; each media spot is an HTML comment.
- 2026-11-04 04:11: read the card's sources, `launch.md` parts 1, 2 and 4, `text.rs`, `present.rs` (menu order),
  `args.rs`, `run.rs`, `hold-files.md`, the `p6-install-e2e`, `p4-live-claude` and `p6-launch-research` Results and
  `research.md` "Install end to end checks" and "Velopack checks". `gh release list`: no release yet. Writing the
  README now.
- 2026-11-04 04:20: draft written; checks run (Result). Decided by the worker, for Haik's review:
  1. Order as this card says: top, rival table, then the video and pictures. `p6-launch-research` item 7 proposed the
     video right after the install line. Change it when Haik picks item 7.
  2. Each folded section has a visible `###` heading above its `<details>`, so the contents links and GitHub's own
     outline work; the `<summary>` line says what is inside. Headings inside `<summary>`: anchors not verified.
  3. The top's line count is non-blank lines: blank lines are spacing. Media spots count as the lines they will take
     (loop 1, speed numbers 1).
  4. One read date for all rival cells, in the caption ("all read 2026-11-04"), and each cell's text is its link.
     Stars not read again: the research read them today (00:45), the day this README is written. A final pass on a
     later day reads them again with the pick rule (`launch.md` part 1).
  5. Rival cells that rest on our own code reading (LidGuard's crash restore, battery floor, lock) link to our report
     `docs/research/rivals-lidguard.md`, the source the research cites.
  6. Table changes from the research's draft: a "Runs on" row (the rivals are cross-platform: an honest gap); "web
     UI" without "WebView2" for Agents Don't Sleep (`launch.md`: not checked on a PC); PowerToys "Locks a closed
     laptop" is "n/a: no lid support", not "no"; PowerToys battery floor "none listed" (the page lists none).
  7. Added for selling, no new facts: the icon state table with tooltip words (`launch.md` part 4 pattern), "A night
     with Wakeshot" (five steps from `product.md`), and a "where the others lead, where Wakeshot differs" paragraph.
  8. A "Not released yet" line at the top with a `release:` comment, since no release exists: the README must not
     promise a download that is not there.
  9. SmartScreen: only "may warn"; the words and steps wait in a `todo:` comment (no doc has them; never from memory).
  10. winget: "Coming" plus `winget:` comments for Phase 8; the `winget upgrade` limits say "Once Wakeshot is on
      winget".
  11. "What stays" after uninstall also names Velopack's own logs (`research.md`, "Velopack's own logs"), which
      `p6-velopack-facts` asked the README to say.
  12. Credits: no rival port; thanks to Howard Hinnant's public domain date algorithms (`research.md` says no credit
      line is needed), and `windows-rs` and Velopack named. Links to `product.md`, `architecture.md` and
      `research.md` "for the curious"; the Phase 8 cleanup decides whether they stay.
- 2026-11-04 04:22: Result filled; both scripts pass again on the final file. Status set to blocked: draft done; waits
  for `p5-haik-check` and `p6-media`. Scratch test files deleted; the two check scripts stay for the final pass.

## Result

Status: blocked: draft done; waits for `p5-haik-check` and `p6-media` (orchestrator decision, session wakeshot-12).

**Done.** `README.md` rewritten whole as a draft, 474 lines: the top (pitch, keep awake, loop spot, speed spot,
install, platforms, agents, a "not released yet" line), the rival table, "See it work" (video, menu and tooltip
spots, the icon state table), "A night with Wakeshot", a contents list, nine folded sections (Install, What keeps
the PC awake, What it changes on your PC, Lid closed lock and battery, The menu notices and start at login,
Commands, Updates and privacy, Limits, Uninstall), then Credits and license. Product words come from the code as it
is now. Ten `<!-- media: ... -->` spots, each naming what goes there and the card that supplies it; one `todo:`
(SmartScreen), two `winget:` and one `release:` comment. Decisions for Haik's review: Log, 04:20.

Files changed: `README.md`; this card. Nothing else. Scripts (kept for the final pass, outside the repo):
`<scratchpad>\p6-readme\check_readme.py`, `check_help.py`.

**Checks.**

| Check | Outcome |
|---|---|
| Relative links resolve, anchors included (GitHub's slug rule), script `check_readme.py` | pass: 23 of 23 |
| No em dash, no en dash, no emoji, no non-ASCII; UTF-8, LF, no BOM (same script) | pass |
| The script fails on a broken copy (bad anchor, em dash, emoji, CRLF planted) | pass: all four caught |
| External links answer (`--web`, one GET each, 2026-11-04 04:18) | pass: 19 of 19 unique URLs gave 200 |
| Rendered by GitHub's Markdown API (`gh api markdown`, gfm) | 4 tables, 9 `<details>`, comments in cells give empty cells, Markdown inside `<details>` renders |
| Help block equals `USAGE` in `args.rs` (`check_help.py`) | pass |
| Menu order and words equal `present.rs` and `text.rs` | pass (read) |
| Top at most 15 lines, image tags not counted | pass: 13 non-blank lines before the table rows (9 before the "How it compares" heading); 12 once the "not released" line goes. Total with blank lines: 21 and 15 |
| Every fact in `product.md`, `architecture.md` or research | pass, by reading; sources below |
| Every speed number matches research | left: there are none yet; all are `p6-media` spots |
| Product words match Haik's pick | left: waits for `p5-haik-check`; lines below |
| Pictures, loop, video in place; the video URL is a `github.com/user-attachments` URL | left: waits for `p6-media` |

**Checklist: card items and every README promise** (README line numbers).

| Item | Lines |
|---|---|
| One line what we solve | 5 |
| One line keep awake on demand comes with it | 6 |
| Silent icon loop (`p6-media`) | 3 (spot) |
| Speed numbers, each linked to its method (`p6-media`) | 8 (spot); table cells 27, 29 (spots) |
| Install command | 10 (Setup.exe; winget comment for Phase 8) |
| Windows 11, x64 and ARM64, per user, no admin; macOS later | 11 |
| Rival table: stack, size, speed where published, features; link and date per rival cell | 16 to 43 (date: 18 to 20) |
| 30 s video | 47 (spot) |
| Menu and tooltip pictures, light and dark | 49, 51 (spots); icons 57 to 59 (spots) |
| Short contents list | 76 to 87 |
| Details folded, each with a plain title | 89 to 462 |
| Install: Setup.exe from Releases; unsigned and SmartScreen; winget once there | 89 to 118 (SmartScreen words: `todo:` 110; winget 114 to 116) |
| What it changes: power requests; lid action and battery sleep Never at start, put back after; saved originals; `wakeshot restore` and the menu item | 166 to 167; 168 to 179; 183 to 189; 190 to 191 |
| Menu, Automatic, Keep awake; start at login and how to turn it off | 148 to 153, 254 to 273; 303 |
| Commands from the CLI help; link to `docs/hold-files.md` | 313 to 346; 146, 348 |
| Updates: automatic, only while nothing holds; one notice after; twice a day; no telemetry; log stays on the PC | 357 to 367 |
| Limits: crash gap and every path; one user per PC; failed restore outside the tray; `winget upgrade` ends `wakeshot run` while its command goes on; off until the next sign-in | 376 to 392; 399 to 404; 394 to 397; 406 to 409; 406 to 409 |
| Uninstall: what it puts back, removes, what stays, `NotifyIconSettings` | 439 to 461 (458 to 460) |
| Credit line per port (none), license | 466, 474 |
| `NotifyIconSettings` left by uninstall (card header) | 203, 458 to 460 |
| `product.md` "User settings safety": the crash gap, "The README says so" | 376 to 392 |
| `product.md` Distribution, README line (top, speed, rivals, folded, macOS, pictures) | 3 to 59 |
| `product.md` Distribution: `winget upgrade` during `wakeshot run` kills the wrapper | 406 to 409 |
| `product.md` Answers: off after `winget upgrade` until sign-in; `NotifyIconSettings` | 406 to 409; 458 to 460 |
| `architecture.md` decision 9: credit line per port | 466 |
| `architecture.md` decision 41: crash gap and paths, one user, `winget upgrade` kills `wakeshot run`, credit | 376 to 392, 399 to 404, 406 to 409, 466 |
| `architecture.md` section 5 "Gap left" and section 6 (winget off until sign-in) | 376 to 392; 406 to 409 |

**Source of each limit line.**

| Lines | Source |
|---|---|
| 376 to 392, crash gap | `architecture.md` section 5 "Gap left" (25 s: section 5 step 7, section 6); `product.md` "User settings safety"; Phase 1 Question 11 (default, sign-off item 11) |
| 394 to 397, failed restore starts the tray | Phase 1 Question 12; `architecture.md` section 5 restore step 7; `product.md` Answers |
| 399 to 404, one user at a time | Phase 1 Question 10; `architecture.md` section 4 "Processes" and section 3 "Episode flow" (console left, Remote Desktop not tested); `product.md` Answers |
| 406 to 411, `winget upgrade` | `architecture.md` section 6; `product.md` Distribution and Answers; `06-ship.md` Question 1; `research.md` "Velopack checks for Wakeshot" point 7; `launch.md` part 2 rule 9 and "A catch in the client" |
| 413 to 414, Claude Code only | `product.md` "Other agents" |
| 416 to 418, waits that do not hold | `product.md` "What keeps the PC awake" |
| 420 to 421, `CLAUDE_CONFIG_DIR` | `research/claude-code-signals.md` "Other facts"; `p4-live-claude` decision 2 |
| 423 to 424, not in v1 | `product.md` (WSL and rules for any app: "Later, not v1"; macOS and Linux long term; English only) |
| 426, not tested live | `research/claude-code-signals.md` "Phase 4 end to end" ("Not run") |
| 428 to 430, no time limit, no heat guard | `product.md` "Power behavior" |
| 432 to 433, policy blocks | `product.md` "User settings safety" rule 6; `research.md` "Windows power facts" (battery rule) |
| 435, unsigned | `product.md` "Signing" |

**Rival cells, each against `launch.md` part 1** ("Candidates" for stars and platforms, "Facts per rival" for the
rest; all read 2026-11-04).

| Row | PowerToys Awake | LidGuard | Agents Don't Sleep |
|---|---|---|---|
| Stars | 140,491, for all of PowerToys | 13 | 10 |
| Runs on | Windows | Windows, Linux, macOS | Windows, macOS, Linux |
| Built with | C# on .NET 10, inside PowerToys | C# .NET 10 NativeAOT, a .NET tool | Rust and Tauri, with a web UI |
| Download, x64 | 323.9 MB, all of PowerToys | 25.3 MB, plus the .NET SDK | 1.9 MB |
| ARM64 | yes | yes | no |
| Idle RAM and CPU | not published; 14.3 MB private memory in a maintainer's test harness | not published | not published |
| Knows when an agent works | no: a timer, a time or a process id | Codex, Claude Code, Copilot CLI, OpenCode; tested: Codex | 8 agents; tested end to end: Claude Code |
| Edits your agent's config | no | yes, adds hooks | yes, adds hooks |
| Lid closed | no | yes, optional | yes |
| Puts settings back | changes none | yes; after a crash, only at its next run | yes, also after a crash |
| Battery floor | none listed | no | 15% by default |
| Locks a closed laptop | n/a: no lid support | no | yes, by default |
| Updates itself | checks, then shows a notice | no: `dotnet tool update` by hand | no updater found |
| Install | per user or machine-wide | per user | per user |
| License, price | MIT, free | MIT, free | MIT, free |

Speed numbers in the README: none (spots only). One size statement, "Agents Don't Sleep's installer is smaller"
(line 41), rests on `research.md` "Install end to end checks" (our `Setup.exe` 11,347,640 bytes, dev build cd3d498)
against `launch.md` (1,897,125 bytes).

**Lines that quote product text** (for the check against Haik's pick in `p5-haik-check`): 51 (tooltip, in the media
comment); 57 to 59 (status lines); 98 to 100 (first-run notice); 139 ("Update Wakeshot"); 148 to 150 and 153 (Keep
awake lengths, Keep screen on, "Until I turn it off", Automatic); 167; 190 to 191 (Restore my power settings, and the
"only while the lid is open" body); 229 (closed-lid report, an example built from `text.rs`); 232 (battery floor
title); 235; 254 to 264 (the menu); 267 to 273 (menu items); 276 to 277 (other status lines); 285 to 298 (every
notice title); 313 to 333 (the CLI help, verbatim, checked by script); 340 to 341 (`--screen` with `off`); 344; 361
("Wakeshot updated to (version)").

**Open items.**

- `p6-media`: the ten spots (loop line 3, speed line 8, table cells 27 and 29, video 47, menu 49, tooltip 51, icons 57
  to 59); then the speed-number check and the user-attachments check.
- `p5-haik-check`: compare the lines above with Haik's final words; fix any that changed.
- Brand (2026-11-05, `docs/brand.md`): the icon is the held light; the README's media comments already name it and
  the loop files in `media/out/`. The header lockup is `p6-brand-apply-web`'s (304 x 80 px); if the loop right under
  it competes with it, the lockup drops to 64 px tall (that card's decision 10).
- First public pre-release: the SmartScreen words and steps (`todo:` line 110).
- Phase 8: the winget lines (`winget:` lines 10 and 116); drop the "not released" line 14 at the first release; read
  the rival stars again if the final pass is on a later day.
- Found outside scope 1 to 6.

## Round 2: the comparison, Haik's ask (2026-11-04)

Status: done (round 2). Written by the orchestrator (session `wakeshot-12`). Needs Haik: he reads the new table. Owns
`README.md` (all of it this round), a dated note in `docs/research/launch.md` part 1, and this section.

Haik's words (2026-11-04): "I do not like that comparison table a lot. If we compare ourselves then we should only
compare with multi thousand star and extremely popular 2-3 options, and it is not important whether its agent or
program specific or not. I want to compete with popular options. And comparison should be on features to show that
besides simple insomnia or keep awake PowerToys feature we give this this and this." `docs/product.md` (README line)
now says so.

1. Pick 2 or 3 keep-awake options that are extremely popular: many thousands of GitHub stars, or for tools not on
   GitHub a verifiable large user count (downloads, store ratings). Agent aware or not does not matter. Tools that run
   on Windows (Wakeshot is Windows only; macOS is planned). PowerToys Awake is the obvious first; research the others
   (for example Caffeine by Zhorn Software, Don't Sleep, Mouse Jiggler; check what Haik may mean by "insomnia"; say
   what you checked and why each is in or out). Read each fact from the tool's own page or repo, and give one read
   date. The small agent-aware tools (LidGuard, Agents Don't Sleep) leave the table; drop them from the
   README unless one line elsewhere still helps the reader (say which, or none).
2. The table compares features, rows as the reader's questions, so it shows plainly: the popular tools keep the PC
   awake while you tell them to; Wakeshot also does this, this and this. Wakeshot's rows come from `docs/product.md`
   as it is now (note: after a closed-lid work end Wakeshot puts the laptop to sleep, not hibernate; a lost journal
   goes to Windows defaults; both 2026-11-04). Parity rows (keep awake on demand, timers, keep screen on) stay short.
   Honest gaps too (for example: Windows only, new and small).
3. Also fix the icon and brand lines (the Sunrise replaced the Cup, `docs/brand.md`), which docs pass round 5 handed
   over. Keep the media spots and the lid and sleep text as they are unless a line is now wrong by `product.md`.
4. Same checks as the draft (links, help block equal to `args.rs`, top line count); lines in the card at most 120
   characters; plain short English, no em dashes.

### Log

- 2026-11-04: round 2 written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 11:33: started by `wakeshot-worker`. Reading `product.md`, `brand.md`, `launch.md` part 1.
- 2026-11-04 11:39: read so far (`gh api` at 11:36): PowerToys 140,521 stars; no other Windows keep-awake repo has
  thousands of stars (Mouse Jiggler 1,459; KeepingYouAwake 7,029 is macOS only). Chocolatey downloads: Caffeine
  367,663, Don't Sleep 38,324 plus 20,340 portable, Mouse Jiggler 26,452 (page; 26,163 in a cached list). Leaning to
  PowerToys Awake, Caffeine, Don't Sleep. Orchestrator note: also fix the media comments at README lines 3 and 57 to
  59 (art is decided: the Sunrise).
- 2026-11-04 11:45: rival facts read from each tool's own page (Awake and install pages, Zhorn's Caffeine page and home
  page, SoftwareOK's Don't Sleep page, FAQ 1 and History). README lines now wrong by `product.md` found: hibernate
  at closed-lid work end, battery floor and failed lock (now sleep); the damaged journal limit (now Windows defaults);
  the timeouts Wakeshot writes after a closed-lid work end; three notice titles (`text.rs` as it is now). Editing.
- 2026-11-04 11:48: README edited; every rival fact checked again in the raw page text (`curl`); dated note added
  to `launch.md` part 1 ("Round 2: popular tools only"). Checks run (Result).
- 2026-11-04 11:50: decided by the worker, for Haik's review:
  1. Rivals: PowerToys Awake, Caffeine (Zhorn Software), Don't Sleep (SoftwareOK). Rule: Windows keep-awake tools
     with the largest public count, GitHub stars or else Chocolatey downloads. Change it when a tool with a larger
     count anyone can check turns up (Microsoft Store rating counts were not checked), or Haik names another.
  2. Out: Mouse Jiggler (fewer downloads than either, and it fakes mouse moves); "Insomnia" (two tools by that name,
     neither with a count: `launch.md`); LidGuard and Agents Don't Sleep leave the README whole. No line
     elsewhere keeps them: next to the popular three they only send readers to tiny rivals. Change it when Haik wants
     an "other agent-aware tools" line.
  3. The first row is how popular each is, in its own unit, with "new in 2026" for Wakeshot: it shows why these three,
     and Wakeshot's honest gap.
  4. Rows are the reader's questions. Parity rows say "yes". Two rows in bold, what only Wakeshot does: awake while
     an AI agent works, and a closed laptop working. Lock, battery, settings and updates are not bold: a rival has
     something close.
  5. "no" only where the tool's own page says so, or where none of its listed methods can do it for a reason Microsoft
     documents (the lid row, explained under the table); "not listed" where the page says nothing. Caffeine's "Keep the
     screen on" says why ("it fakes a key press"): the page names only the lock and the screensaver.
  6. Rows dropped: stars as such, "Built with", "ARM64 build" (now in "Runs on"), "Idle RAM and CPU" (no rival
     publishes one; Wakeshot's numbers stay in the top line) and "Edits your agent's config" (no popular rival touches
     an agent). Kept a "Download" row: a fair question and an honest gap; its Wakeshot cell stays a media spot. Media
     spots: 9, was 10.
  7. Below the table: why no rival keeps a closed laptop working, and "Where the others lead" (years of use, Windows
     10, tiny downloads, Don't Sleep's shutdown features; Wakeshot new, Windows 11 only, Claude Code only).
  8. Lines wrong by `product.md`, fixed: closed-lid work end, battery floor and failed lock now sleep, with hibernate
     only as the fallback ("A night with Wakeshot" 3 and 5; "Lid closed, lock and battery"); "What it changes" gets
     the 1 minute screen and sleep timeouts and its summary says "a few power settings"; the damaged journal leaves
     the crash list for its own paragraph (Windows defaults); updates wait for no "closed-lid sleep" pending; "one
     user" says "no lock, no sleep"; the credit line says "from other tools".
  9. Notice table follows `text.rs` as it is now, which `p2-6-sleep-route` is still changing: "The PC did not go to
     sleep" replaces "Wakeshot could not hibernate the PC"; "Power settings set to Windows defaults" added; "Check
     your lid and sleep settings" now means the reset failed too.
  10. Media comments (orchestrator note): the loop shows the sun rising and setting (`docs/brand.md`, "The idea") with
      new alt text; the three icon spots name the Sunrise art in `tray/art.rs` and carry alt text from `product.md`.
- 2026-11-04 11:51: orchestrator note 2 (the "hibernates" lines): already fixed under decision 8; trimmed them and
  added two `<!-- sleep-route: -->` comments that name every line to check again after `p2-6-sleep-route`. Checks run
  again; scratch files of this round deleted (the two check scripts stay).

### Result (round 2)

Status: done. Needs Haik: he reads the new table and decisions 1 to 10 (Log, 11:50). The card as a whole stays
blocked on `p5-haik-check` and `p6-media`, as before.

**Done.** The comparison is now Wakeshot against PowerToys Awake (140,521 GitHub stars, for all of PowerToys),
Caffeine (367,663 Chocolatey downloads) and Don't Sleep (38,324 plus 20,340 portable), on 13 feature rows asked as
the reader's questions. LidGuard and Agents Don't Sleep are gone from the README. Lines wrong by
`product.md` fixed (sleep, not hibernate; lost journal to Windows defaults; the 1 minute timeouts; notice titles).
Sunrise media comments in place. README: 492 lines, 9 media spots.

Files changed: `README.md`; `docs/research/launch.md` (part 1, new "Round 2: popular tools only"); this section.

| Check | Outcome |
|---|---|
| Relative links and anchors (`check_readme.py`) | pass: 22 of 22 |
| No em or en dash, no emoji or other non-ASCII; UTF-8, LF, no BOM (same script) | pass |
| External links (`--web`, one GET each, 11:47) | pass: 16 of 16 unique URLs gave 200; none added since |
| Help block equals `USAGE` in `args.rs` (`check_help.py`) | pass |
| Top at most 15 lines, image tags not counted | pass: 13 non-blank before the table rows, 9 before the heading |
| GitHub render (`gh api markdown`, gfm) | 4 tables, 9 `<details>`; comparison 14 rows of 5 cells; no comment shows |
| Every rival fact in the raw page text (`curl`), not only a fetch summary | pass; Chocolatey counts from page HTML |
| Round 2 lines in this card and the `launch.md` note at most 120 characters, ASCII only | pass |

**Each table row and its source.** Rival cells: `launch.md` part 1, "Round 2: popular tools only" (Caffeine and
Don't Sleep bullets; PowerToys there and in "Facts per rival"). Wakeshot cells: `product.md`.

| Row | Rival source | Wakeshot source (`product.md`) |
|---|---|---|
| How popular is it? | gh stars; Chocolatey page counts | not released (README top) |
| Keep awake for a set time, or until I turn it off? | Awake modes; `-activefor`; Don't Sleep timer | UX, Keep awake |
| Keep the screen on too? | Keep screen on; F15 key press (reason in the cell); monitor off option | UX |
| Awake while my AI agent works (bold) | none on any page | What keeps the PC awake |
| Awake while a build or a script runs? | `--pid`; `-watchwindow`, `-cpu`; CPU and network load | `wakeshot run` |
| Lid closed (bold) | Awake page, FAQ 1: no setting changed; Caffeine: no switch; lid fact | Power behavior |
| Still locks when I walk away? | Awake "lock your computer"; Caffeine "won't lock"; none | Never lower security |
| Step back when the battery runs low? | none; `-onac`; History 5.11 and 6.77 | Battery floor |
| Does it change my power settings? | Awake page; none listed; FAQ 1 | Power behavior; User settings safety |
| Updates itself? | install page; none listed; History "optional Auto Update Service" | Distribution, Velopack |
| Runs on | install page; "x86 and x64" download; "Suitable for" | Platform and stack |
| Download | v0.102.3001 assets (gh); caffeine.zip 306k; "Download (267 KB)" | media spot (`p6-media`) |
| Price | MIT (gh); Zhorn "freeware" line; "small Freeware" | MIT, free |

Under the table: the lid fact (`research.md`, "Windows power facts"); "Where the others lead" from the same note
(Windows 10 lines, sizes, "Schedule computer to shut down", "prevent system shutdown").

**Open items.**

- After `p2-6-sleep-route`: the two `sleep-route:` comments in the README ("What it changes" paragraph on the 1
  minute timeouts; lid bullets, "A night with Wakeshot" 3 and 5, the notice table).
- From the draft, unchanged: `p6-media` spots (9 now; the Download cell is one), `p5-haik-check` words, the
  SmartScreen `todo:`, the `winget:` and `release:` comments. Read the counts again if the final pass is on a later
  day.

**Found outside scope (round 2).**

1. `docs/product.md` line 3, "Status on 2026-11-03: ... no product code yet", is stale. Its README line is right.
2. The draft's "Found outside scope" 4 (the dot in the pack id) is fixed in `product.md` now.
3. The crash gap text (`product.md` "User settings safety", README "Limits") names only lid Do nothing and battery
   sleep Never. A crash during a closed-lid sleep step would leave the 1 minute timeouts too (inferred from
   `product.md`; not verified). `architecture.md` "Gap left" is `p2-6-sleep-route`'s this round; README not changed.

## Round 3 (Haik's table comments, 2026-11-04)

Status: done (round 3). Written by the orchestrator. Owns `README.md` (the comparison table and the sentences right
around it), `site/index.html` (only if it has the same comparison) and this section.

Haik's comments (his words, 2026-11-04 about 16:05):

1. "popularity row not needed." Remove the "How popular is it?" row.
2. "powertoys awake keep awake time is limited (30mins, 1 hour and 2 hour)". Check the current Microsoft docs for
   PowerToys Awake first; write the cell to match them, short, with the link.
3. "locking part can be bold. I think only we do that". Bold the lock row, if the other three cells support it.
4. "Does it change my power settings remove please". Remove that row.
5. Not decided yet: a tech stack or lightness row. Not added this round.

### Log

- 2026-11-04 16:05: started by `wakeshot-worker`. `site/index.html` has no comparison yet (only a `waits for the
  README rework` comment at line 72), so only `README.md` changes.
- 2026-11-04 16:08: PowerToys Awake docs read in the raw page (`curl`, page `ms.date` 2026-10-14, "Last updated on
  2026-10-21"). Settings modes, word for word: "Keep awake indefinitely: The computer stays awake indefinitely until
  you explicitly put the machine to sleep or close/disable the application."; "Keep awake for a time interval: Keep
  machine awake for a predefined limited time."; "Keep awake until expiration: Keep machine awake until a defined
  date and time is hit."; a note on "the hours or minutes" of the interval. Tray: "a pre-defined time interval (for
  example, 30 minutes)"; the tray's times can be changed with `customTrayTimes` in `settings.json`. CLI:
  `--time-limit` in seconds, `--expire-at`, and no argument means indefinitely. The docs do not list the tray's times.
- 2026-11-04 16:10: PowerToys source at the latest release tag `v0.102.3001` (`gh api`; same file as `main`):
  `Manager.cs` lines 508 to 517, `GetDefaultTrayOptions`, gives 30 minutes, 1 hour and 2 hours; `TrayHelper.cs`
  lines 510 to 519 add "Keep awake indefinitely" and a greyed "Keep awake until expiration" to the tray menu. So
  Haik is right about the tray menu, but Awake is not limited to those: Settings and the command line take any
  length, a date and time, or no end.
- 2026-11-04 16:12: lock row. The three rival cells, read again in the raw pages: PowerToys lists no lock and says
  "configure your Windows power plan settings directly and lock your computer for security"; Caffeine "won't lock the
  screen" (its key presses), and `-notwhenlocked` only goes inactive when locked; Don't Sleep's page and FAQ 1 list no
  lock (it blocks the screen saver, monitor off and logoff). Only doubt: Don't Sleep's History has two fix lines
  "related to Smart Screen Lock" (9.81, 9.91), with no word on what that is; a web search found nothing. No rival
  lists a feature that locks the PC, so the row is bold.
- 2026-11-04 16:14: README edited; checks run (Result).
- 2026-11-04 16:15: decided by the worker:
  1. PowerToys timer cell: the docs' three modes in one link to the docs, then the tray's 30 min, 1 h and 2 h linked
     to the source at the release tag (the docs do not list them). Change it when the docs list the tray times: link
     the docs only.
  2. Caption: "(first row)" dropped with the popularity row; "two rows in bold" is now "three"; "page or package" is
     now "page or repo" (the Chocolatey links left with the row). The counts stay in `launch.md` part 1 for the pick.
  3. Wakeshot's own cell in the timer row stays "yes": not part of Haik's comments.
- 2026-11-04 16:25: orchestrator note: Haik agreed (about 16:10) to no stack row and no size row, and one line under
  the table instead ("A small native app: one exe of about 3 MB, no runtime to install", in spirit). Facts read:
  release x64 tray 2,706,432 bytes and CLI 557,056 (`p6-tray-size` Result; `research.md` "Tray size"), 3.26 MB
  together; the shipped ARM64 tray size was never measured (CI only, no clang here), so the line says "on x64";
  both exes on both targets import only `KERNEL32.dll`, `ntdll.dll` and `api-ms-win-core-synch-l1-2-0.dll`
  (`research.md` "CRT check"); the install holds `Update.exe`, a root stub `Wakeshot.exe`, both exes under `current\`
  and a package copy in `packages\` (`research.md` "Velopack checks", point 2), so "one exe" is not true. Velopack's
  `Update.exe`: 5.2 MB (`research.md` "Size"; `research/distribution.md`), 4.7 MB in the ARM64 package. Line added
  under the table, linked to "Tray size". `check_readme.py`: pass, 24 of 24 relative links.

### Result (round 3)

Status: done. The card as a whole stays blocked on `p5-haik-check` and `p6-media`, as before.

**Done.** In `README.md`, "How it compares": the "How popular is it?" and "Does it change my power settings?" rows
are gone; "Does my PC still lock when I walk away?" is bold; the PowerToys cell of the timer row now says "yes: until
you turn it off, until a date and time, or for hours and minutes you set in Settings; the tray menu lists 30 min, 1 h
and 2 h". The caption above the table follows (no "first row", "three rows in bold", "page or repo"). The table is
11 rows, 3 bold. No tech stack, lightness or size row (Haik, about 16:10). One line under the table instead: "A
small native app: Wakeshot's two exes come to about 3 MB on x64 and need no .NET or other runtime. Velopack's
updater (about 5 MB) comes with them." (facts: Log 16:25). `site/index.html` not changed: it has no comparison yet.

What the PowerToys docs say (page updated 2026-10-21): three keep-awake modes, "Keep awake indefinitely", "Keep awake
for a time interval" ("a predefined limited time", set in hours and minutes) and "Keep awake until expiration" ("until
a defined date and time is hit"). The docs do not list the tray's times; the source at `v0.102.3001` does: 30
minutes, 1 hour, 2 hours by default, which `customTrayTimes` can change. Haik is right about the tray menu only.

Files changed: `README.md` (lines 18 to 37); this section.

| Check | Outcome |
|---|---|
| Relative links and anchors (`check_readme.py` from round 2) | pass: 24 of 24 (the new "Tray size" link) |
| No em or en dash, no emoji or other non-ASCII; UTF-8, LF, no BOM (same script) | pass |
| External links (`--web`, one GET each, 16:13) | pass: 14 of 14 unique URLs gave 200, the new source link included |
| Top at most 15 lines, image tags not counted | pass: 13 non-blank before the table rows, 9 before the heading |
| GitHub render (`gh api markdown`, gfm) | 4 tables, 9 `<details>`; comparison 11 rows of 5 cells, 3 bold |
| New rival facts in the raw page or source, not a fetch summary | pass: `curl` (Awake page), `gh api` (source) |
| Round 3 lines in this card at most 120 characters, ASCII only | pass |

Help block not checked again: not touched.

**Found outside scope (round 3).**

1. `docs/research/launch.md` part 1 and `docs/research.md` do not hold the Awake modes or the tray's default times;
   this round owned neither file. The facts and how they were checked are in this round's Log (16:08, 16:10).
2. On timers PowerToys offers more than Wakeshot: Wakeshot has 30 min, 1 h, 2 h, 4 h or until off (`args.rs` line
   21), with no length of your own and no end time. The timer row stays a parity "yes" for both; Haik may want to
   know before he calls Awake "limited".
3. Don't Sleep's History names a "Smart Screen Lock" twice (fix lines 9.81 and 9.91) without saying what it is. If it
   is a lock that Don't Sleep starts, the bold lock row is wrong for Don't Sleep. Not verified.

## Round 4: the story, the media and the numbers (Haik, 2026-11-06)

Written by the orchestrator (overnight run, Haik asleep). Status: done. Agent: `wakeshot-worker`. Owns `README.md`;
the copy in `site/` only where it repeats the README (keep the two in line; `site/check_page.py` must still pass);
additions to `docs/research.md` for the rival facts (re-read right before each edit). Runs beside `p6-media` round 4
(it owns `media/` and writes `docs/research.md#speed-numbers`) and `p6-repo-files` (it writes `CONTRIBUTING.md`,
`SECURITY.md`, `CODE_OF_CONDUCT.md` and `.github/` templates; link to them by those names). Build folder: none.

1. **The story.** A short, honest section in Haik's voice (plain, first person, his facts, no sales words) on why he
   built Wakeshot and why the built-in or Windows tools did not fit him. His words below, from voice input (read for
   intent; the paste lost a few words at line ends, shown in brackets). Keep it short: the README is long already.
   Unverified parts are his own experience ("I", "for me"), never a fact about another tool. Every claim about
   another tool is checked against its current docs or source, with the link and the date read in
   `docs/research.md`: the PowerToys download size (the table has 323.9 MB for v0.102.3001: still current?);
   whether PowerToys Awake allows longer or custom periods and where; whether it shows the time left and can extend a
   running timer; what "Claude's built-in tool" is (a keep-awake setting in a Claude app? find it in Anthropic's own
   docs; if none exists, leave that part out and say so in the Log). Round 3's Found outside scope 2 applies: our own
   timers are 30 min, 1 h, 2 h, 4 h or until off, so do not mock PowerToys for fixed periods.
2. **The future, from his words**: Windows only for now because he built it for himself; the design is ready for other
   systems; macOS if it gets popular or gets donations (it needs a Mac to build and test on); Linux is fragmented
   (desktops, distributions, package managers), so not planned now. PRs welcome; Haik reviews the architecture (link
   `CONTRIBUTING.md`). No date, no promise beyond his words. No donation link yet (that waits for his account choice).
3. **Media**: fill every `<!-- media: ... -->` spot that has a committed file: the icon loop
   (`media/out/icon-loop-dark.gif`, `-light.gif`), the menu and tooltip stills (`media/public/shots/readme-*.png`),
   the state icons if committed PNG or SVG files exist (else leave that spot and say so). Use `<picture>` with
   `prefers-color-scheme`, alt text from the spot. The video plays inline only from a
   `https://github.com/user-attachments/assets/<id>` URL, and that upload waits for Haik: leave one clearly marked
   spot for it (a visible line is wrong; an HTML comment saying "Haik: paste the video URL here" is right), and link
   nothing else for the video.
4. **Numbers** (`p6-media` round 4 has the raw values): the speed line at the top and the Download cell. Use:
   private memory 4.3 MB idle (median), CPU 109 ms in 10 minutes idle (0.018% of one core), start of a hold about
   0.1 s after Claude Code starts work (median of 5 runs, 114 ms; max 298 ms), Setup.exe 12.3 MB, measured
   2026-11-06 on 0.1.0, x64. Link each to `docs/research.md#speed-numbers`. Give no end delay number.
5. Plain short English, no em dashes, no emojis, the house voice of the rest of the README. Read the result once as
   a stranger: the top must still sell in the first screen.

Haik's words (2026-11-06, voice input, as pasted):

> Our readme and repo should follow best practices! We should write a story of why [I] created this software and why
> claude's built-in tool or windows ones are not fit to us. The story is follow[ing]: [before] that then using agents
> or downloading long files usually I use keep awake of windows power toys. I download[ed] this 300mb tool only for
> ke[ep] awake feature. BTW it does not allow to set longer periods maybe hidden in settin[gs] but not a good ux. And
> when you click for instance the max (2 hours), you do not see how much left and cant refresh it. Other tools hav[e]
> problems as well. So I ended up constantly switching my laptop settings for long sessions. And not only ai w[ork],
> sometimes download of a big items and so on. Another bigger problem is that sometimes you are forced al[so to
> close] the lid. In this case you should change extra settings. This is constant micro stresses that starting to
> m[ess] with you, but it's not critical. The critical part is that sometimes you forget t[o] return back the
> settings an[d] that's it. You feel how your bag become hot piece of thing or at the important ti[me] you have died
> battery and so on. That's why I decided to create something that first of all helps me, that's w[hy] it's only on
> windows now. I have made its architecture ready for cross platform but as it's only for me and I['m a] windows user
> I did only for windows. If this software will become popular, or get some donations, I will develop mac os version
> as well. To [be] honest for now it's too time consuming, as project is heavily relying on OS specifics and APIs and
> even [...] that's why it's a long job for a windows guy. + need to borrow apple laptop and so on :) Linux is another
> beas[t]. This OS is used for a very few people compared to windows and mac. And these few peoples use different
> desktop environments like KDE and Gnome, and also different OS with different package managers. This one is very
> [...] one. Everything in short. If this software will become mainstream, everything will be in place. And also as
> th[is is] open source, if someone like me wants to do something it can do itself as well, as I welcome PRs, and
> only will review architecture. Hope this software will help community, as it is crafted very carefully. It's doing
> very simple thing, very trivial thing. It's just for certain processes and conditions does not allow laptop to
> sl[eep] but under the hood it's too complicated. Dozen of cases of careful crafted works and so on. Even
> Microsoft's kee[p] awake has lots of bugs time to time that I faced, but this one was crafted with a great care of
> many nuances. Every notification and sound has been carefully designed. Every scenario.

### Log (round 4)

- 2026-11-06: started by `wakeshot-worker`. Read the card top, Results of rounds 2 and 3, `README.md` (506 lines) and
  `site/index.html`.
- 2026-11-06 04:00: rival facts checked in raw pages and source (details and links go to `docs/research.md`, "Story
  checks: PowerToys Awake and Claude's own keep awake"):
  1. PowerToys latest release is still `v0.102.3001` (published 2026-10-27); `PowerToysUserSetup-0.102.3001.0-x64.exe`
     is 323,884,640 bytes, so the table's 323.9 MB stands.
  2. Longer periods: yes. Settings take any hours (NumberBox, minimum 0, no maximum) and minutes; the docs add
     "until expiration" and `--time-limit` in seconds; `customTrayTimes` in `settings.json` adds tray entries. The
     tray's own list is 30 min, 1 h, 2 h (`GetDefaultTrayOptions`).
  3. Time left: at `v0.102.3001` the tray tooltip shows "<time> remaining" in timed mode, updated each second
     (`Manager.cs` 404 to 417); an older form was in the source by 2025-09-24. Extend: the tray's time items are
     always enabled, and picking one again cancels the timer and starts it again from now (source read, not run).
     So Haik's "you do not see how much left and cant refresh it" does not hold for the current release: left out.
  4. "Claude's built-in tool": Anthropic's docs name a desktop app setting, **Keep computer awake** (Settings > Desktop
     app or Settings > This computer > System), "To prevent idle-sleep", and add "Closing the laptop lid still puts it
     to sleep." Claude Code CLI 2.1.318 on this laptop: the sleep inhibitor returns `caffeinate` on macOS and `null`
     on every other system; no Windows power API name in the binary.
- 2026-11-06 04:05: media on disk: `media/out/icon-loop-{dark,light}.gif` (480 x 240), `media/public/shots/readme-*`
  (menu 700 x 550; holding and idle tooltip 564 x 240; all at 200%). No state icon file of the held light exists:
  `docs/research/img/ux/taskbar-*` and `int-icon-*` show the old sunrise glyph, and the `icon-sheet-*` files are
  review sheets. The three icon cells stay media spots. The menu still shows the main menu, not the Keep awake
  submenu, so its alt text says what it shows.
- 2026-11-06 04:15: `docs/research.md`: new section "Story checks: PowerToys Awake and Claude's own keep awake" before
  "CLI run checks" (away from the end, where `p6-media` appends). `## Speed numbers` is in place (p6-media).
- 2026-11-06 04:30: README edited: icon loop, speed line, Download cell 12.3 MB, exes 3.6 MB and 14.3 MB installed
  (both from "Speed numbers"), the video spot as one "Haik: paste the video URL here" comment, menu and both tooltip
  pictures, "Why I built it" and "What comes next". `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` do not
  exist yet (`p6-repo-files` runs beside); linked by those names.
- 2026-11-06 04:35: `site/index.html`: the speed spot filled with the README's line (links to the research section on
  github.com). Other site spots left: they need files copied into `site/media/` or new copy, not a README repeat.
- 2026-11-06 04:08 to 04:40: checks run (Result). All three community files exist by 04:40. `softwareok.com` answers
  503 for every page, its home page too (IIS "Service Unavailable"), at 04:07 and 04:08: the site is down, the Don't
  Sleep links are unchanged since round 3.

### Result (round 4)

Status: done. The card as a whole stays blocked on the video URL (Haik's upload), the three state icon files and
`p5-haik-check`, plus the SmartScreen `todo:`, the `winget:` and `release:` comments as before.

**Done.** In `README.md`: the icon loop (`<picture>`, dark and light GIF, 240 px) under the header; the speed line at
the top, each number linked to `docs/research.md#speed-numbers`; the Download cell (12.3 MB `Setup.exe`); the size
line under the table (3.6 MB exes, 14.3 MB installed, from "Speed numbers"; was "about 3 MB" from "Tray size"); the
caption says PowerToys was read again on 2026-11-06; one HTML comment "Haik: paste the video URL here" in "See it
work"; the menu picture and both tooltip pictures (holding and idle), each a `<picture>`, light and dark; a new "Why I
built it" with "What comes next", which links `CONTRIBUTING.md`, `SECURITY.md` and `CODE_OF_CONDUCT.md`. In
`site/index.html`: the speed line, the same words and numbers. In `docs/research.md`: a new section "Story checks:
PowerToys Awake and Claude's own keep awake" (before "CLI run checks").

Files changed: `README.md` (506 to 562 lines), `site/index.html` (hero), `docs/research.md` (new section), this card.

| Check | Outcome |
|---|---|
| Relative links and anchors (`check_readme.py`, round 2 script plus a top count and alt check) | pass: 42 of 42 |
| No em or en dash, no emoji or other non-ASCII; UTF-8, LF, no BOM (same script) | pass |
| Top at most 15 lines, image tags and comments not counted | pass: 12 before the table rows, 7 before the heading |
| Every `<img>` has alt text; 5 `<picture>`, 5 `<source>`, 5 `<img>` | pass |
| External links (`--web`, one GET each, 04:08) | 38 of 41 gave 200; 3 `softwareok.com` links 503: site down (Log) |
| Help block equals `USAGE` in `args.rs` (`check_help.py`) | pass |
| `site/check_page.py site` | pass: 0 errors, 7 spots wait (was 8) |
| GitHub render (`gh api markdown`, gfm) | 4 tables, 9 `<details>`, 5 `<picture>`; no comment shows |
| Story and speed claims against their sources | pass: list below |
| Round 4 lines in this card at most 120 characters, ASCII only (the brief excepted) | pass |

**Each number and its source** (`docs/research.md`, "Speed numbers"): 4.3 MB private memory (median 4.27); 109 ms of
CPU in 10 idle minutes, 0.018% of one core; about 0.1 s to start holding (median 114 ms); `Setup.exe` 12.3 MB
(12,310,500 bytes); exes 3.6 MB (2,907,648 + 700,416); 14.3 MB installed (14,269,755). No end delay. 0.1.0, x64,
2026-11-06.

**Facts verified for the story** (all in research.md, "Story checks", read 2026-11-06):

1. PowerToys `v0.102.3001` is still the latest; user setup x64 323.9 MB:
   https://github.com/microsoft/PowerToys/releases/tag/v0.102.3001
2. Awake allows longer times in Settings (hours, no maximum) and `settings.json`; the tray list is 30 min, 1 h, 2 h:
   https://learn.microsoft.com/en-us/windows/powertoys/awake and the source at the tag.
3. Awake shows the time left in its tooltip and re-picking a time starts it over (source; not run).
4. Claude's desktop app: **Keep computer awake**; "Closing the laptop lid still puts it to sleep":
   https://code.claude.com/docs/en/desktop-scheduled-tasks
5. Claude Code 2.1.318 on Windows holds nothing (binary grep; anthropics/claude-code#96679 still open).

**Decisions for Haik's review.**

1. Left out "you do not see how much is left and can't refresh it": the current Awake shows the time left in its
   tooltip, and picking a time again starts it over (fact 3). Change it when you see it fail on the current version.
2. Kept "Now and then I also ran into its bugs" and "Other tools I tried had gaps of their own" as your experience.
   PR #44795 fixed two real Awake bugs this year. Change it when you want no line against PowerToys.
3. Linux: kept the many desktops, distributions and package managers; dropped "very few people use it" (not checked).
4. "Claude's built-in tool" is the desktop app's Keep computer awake; the story names it with its docs' lid line, and
   adds that the terminal CLI holds nothing on Windows.
5. Both tooltip pictures, holding and idle, side by side under the menu picture: they show the icon states that the
   table's empty icon cells cannot yet.
6. The menu picture shows the main menu, not the Keep awake submenu the old spot asked for; the alt text says what it
   shows. Change it when a light and dark submenu still exists.
7. The speed line's lead-in is "Small and quiet:" (not "Light", which reads as the icon's light).

**Left.**

- Haik: upload the video and paste its URL at the "Haik: paste the video URL here" comment in "See it work".
- The three icon cells of the state table stay `media:` spots: no PNG or SVG of the held light per state exists
  (`docs/research/img/ux/taskbar-*` and `int-icon-*` show the old sunrise glyph; `icon-sheet-*` are review sheets).
- `site/`: the icon loop, pictures, video and comparison spots still wait (they need files in `site/media/`).
- Check the `softwareok.com` links again once the site is back.

**Found outside scope (round 4).**

1. The README's PowerToys timer cell is a plain "yes", not the longer text round 3's Result records; changed after
   round 3 by someone else. Not touched.
2. `docs/research.md` "Tray size" (2,706,432 bytes) is an earlier build; "Speed numbers" has 0.1.0's sizes.
3. `docs/research/img/ux/taskbar-*.png` and `int-icon-*.png` still show the old sunrise glyph, with another app's tray
   icon beside it. Fine as history; not for the README.

## Round 5: a new README from scratch (Haik, 2026-11-06)

Written by the orchestrator. Status: done. Agent: `wakeshot-worker`. Owns: `README.md` (whole),
`docs/research/readme.md` (new), this card's Round 5 Log and Result; `site/index.html` only where its words now say
the opposite of the new README (list the rest for `p6-pages`). Real laptop: no. Build folder:
`<scratchpad>\p6-readme-r5\`.

Haik's words, verbatim (voice input): "I do not like the readme.md. It's focus is on comparison and some numbers from
start. I guess it's focus should be what it does and how easy to install and the installation part should be super
simple to read. Then all other stuff can come. Maybe your agent should research how perfect readme.md should be and
craft it from scratch. No need to do so many external links to claude and etc. Do not forget that we are going to add
more and more features and options which will hold awake. Table of comparison is too big in rows. Not sure people
care on velopack or too technical things there related to size. Lightweighness is enough. Also claude keep awake is
only for claude desktop, you are right that claude code is out of it but also other agents or software are out. Go
on research how perfect readme should be wich engage reading. First of all installation easiest way is winget path
which has 0 words about it."

What that means for the build:

1. **Research first**, into `docs/research/readme.md` (short, sources linked, dates): what makes a README people read
   to the end and act on. GitHub's own docs on READMEs, a few loved READMEs of small desktop utilities (read them,
   do not copy layouts or lines), and what readers look for first. Rules with reasons, then the outline it leads to.
2. **Write `README.md` from scratch** on that outline. Top: what Wakeshot does in plain words, the video spot, then
   install, super simple. Everything else after.
3. **Install: winget first.** `winget install HaikAsatryan.Wakeshot` is the main path; Setup.exe from Releases is the
   second. Fact to keep honest: the winget package and the first release are not live yet (Phase 8 submits the first
   version, `packaging/winget/README.md`). Write for launch day and leave one HTML comment where Phase 8 checks the
   line. Check in `packaging/winget/` whether winget picks x64 or ARM64 by itself; say only what the manifest does.
4. **Built to grow.** Present what holds the PC awake as a list of sources that will grow (today: Claude Code, any
   command through `wakeshot run`, Keep awake from the menu; more later). No Claude-only framing of the product.
5. **Lightweight is enough.** No Velopack, no MB numbers, no size cells near the top or in the table. The measured
   numbers may stay as one line deep in the details (link `docs/research.md#speed-numbers`) or go: your call, say
   which.
6. **Small comparison table**, few rows, low on the page. The Claude desktop app's "Keep computer awake" covers only
   the Claude desktop app: Claude Code, other agents and other software are not covered. Fewer external links overall,
   especially to Claude pages.
7. **Keep what must stay** (lower down, folded is fine): decision 41's list (`docs/architecture.md` row 41: the crash
   gap and every path into it, one signed-in user per PC, `winget upgrade` ends `wakeshot run`, a credit line for
   each port from a rival), what it changes on the PC and what an uninstall leaves (with `NotifyIconSettings`), the
   commands, privacy, limits, Haik's story (`## Why I built it`, his words from Round 4, shorter is fine, keep it his
   voice and true), the links to CONTRIBUTING, SECURITY and CODE_OF_CONDUCT, the icon loop, the menu and tooltip
   stills, the state icons, and the "Haik: paste the video URL here" comment.
8. Every fact still true to the code and docs; nothing new claimed without a source. Plain short English, no em
   dashes, no emojis. Render check: the GitHub markdown API, light and dark pictures.
9. Result: the new outline, the choices you made for Haik's review, what you cut and why, and the site lines that
   now differ.

### Log (round 5)

- 2026-11-06: started by `wakeshot-worker`. Status: in progress. Read the Round 5 and Round 4 sections, the current
  `README.md` (562 lines) and the plan's "How we work".
- 2026-11-06 10:49: research read (GitHub docs, makeareadme, standard-readme, Art of README via the Wayback Machine,
  NN/g, the 2017 Open Source Survey, Prana et al. 2018) and 10 READMEs of small desktop utilities (`gh api`, raw, into
  the build folder). Facts: no release yet (`gh release list` empty); the winget manifest lists one installer per
  CPU (x64, arm64); a silent Setup does not start the app (`research.md` 3610, 4370), so a winget install needs a
  Start menu start. Claude desktop's Keep computer awake: the docs give no scope; issue anthropics/claude-code#94420
  (a user's log) shows a hold per Code turn of the desktop app. No check script reads `README.md` besides our own.
- 2026-11-06 about 10:52: `docs/research/readme.md` written: sources, 12 rules with reasons, the outline, and the facts
  checked for the new install lines. Writing `README.md` now.
- 2026-11-06 about 10:56: `README.md` written from scratch on that outline (579 lines; old copy in the build folder as
  `README-round4.md`). Check scripts copied from round 4 into the build folder and changed: the install line's place
  and the table's row count replace the old top count. First runs: links 46 of 46, help block equal, render has 5
  tables, 9 folds, 8 pictures each with a dark source, the note alert in gfm mode, no comment shows.
- 2026-11-06 10:58: read once as a stranger: the system line moved above the command, MIT into the pitch, one
  repeated line cut. All checks run again (Result). `site/index.html`: no line says the opposite of the new README,
  so not edited; the lines that now differ are listed for `p6-pages`. Scratch downloads deleted; the two check scripts
  and the old README copy stay in the build folder for the final pass.

### Result (round 5)

Status: done. The card as a whole stays blocked as before: the video URL (Haik's upload), `p5-haik-check`, the
SmartScreen `todo:`, and the `release:` and `winget:` comments that wait for the first release and Phase 8.

**Done.** `docs/research/readme.md` (new): 8 sources and 6 utility READMEs, read 2026-11-06, 12 rules with reasons,
the outline, and the facts checked for the new install lines. `README.md` rewritten from scratch on that outline
(578 lines, was 562; everything below "Details" is folded).

**The new outline.**

1. Header lockup, light and dark.
2. What it does: one bold sentence; one line of what holds the PC awake (an agent, a build or any command, Keep
   awake), the lid, your settings back; "small, lightweight", MIT.
3. The video spot ("Haik: paste the video URL here", unchanged).
4. `## Install`: one `[!NOTE]` (not released yet, with a `release:` comment), the system line,
   `winget install HaikAsatryan.Wakeshot` (with a `winget:` comment for Phase 8), "start it once from the Start
   menu", `Setup.exe` from Releases as the other way; one fold: what the install sets up, the first notice,
   SmartScreen (`todo:` kept), signing, updates.
5. `## What keeps your PC awake`: a 4-row table of today's holds (Claude Code, any command, any script, you), "the
   list grows", Codex CLI and Copilot CLI next.
6. `## How it works`: a night with Wakeshot, five steps, agent-neutral.
7. `## What you see`: the icon loop, the three state icons, the menu, the two tooltips.
8. `## How it compares`: 5 rows, 3 rivals; one line on the Claude desktop app's own setting; where the others lead.
9. `## Why I built it` and `### What comes next`: Haik's story, shorter, his voice.
10. `## Details`, folded: when each hold counts, what it changes, lid and battery, menu and notices, commands,
    updates and privacy, limits, uninstall.
11. `## Credits and license`.

**Choices for Haik's review.**

1. winget is the first install path; the README is written for launch day, with one `[!NOTE]` saying nothing is
   released yet. Change it when you prefer no install command until winget has the package.
2. After `winget install`, the README says "start Wakeshot once from the Start menu": a winget install runs Setup
   silently, and a silent Setup does not start the app (`docs/research/readme.md`, "Facts checked").
3. "winget picks the Setup for your PC, x64 or ARM64": the manifest has one installer per CPU and Microsoft's docs
   say winget selects the installer for the system. How it ranks them on an Arm PC is not verified.
4. The icon loop moved from the top to "What you see", so the top is words, the video, then install. Change it when
   the video is not uploaded by launch: put the loop back under the pitch.
5. Measured numbers: one line kept, deep in "Updates and privacy" (memory and CPU only, linked to "Speed numbers").
   The 0.1 s start delay, the `Setup.exe` size, the exe sizes and the Velopack size are cut.
6. Table: 11 rows to 5 (set time; on its own for an agent; builds and scripts; lid closed; lock). Cut rows: keep
   screen on (all do it), battery, updates, runs on, download size, price. Windows 10 support and the tiny
   downloads of the others stay in one "where the others lead" line, without numbers.
7. Links: each rival is linked once, in the table head; the cell sources are one link to `launch.md` part 1. One link
   to a Claude page in the whole README (was 2); external links 8 (was 41, 14 unique).
8. The Claude desktop app's Keep computer awake: "It holds for that app's own sessions, not for Claude Code in a
   terminal, other agents or other programs", plus its docs' lid line. The scope rests on a user's log in
   anthropics/claude-code#94420, not on Anthropic's docs (marked **reported** in `docs/research/readme.md`).
   Change it when Anthropic documents the scope.
9. Story: kept your facts and voice; "over 300 MB" without a link; the Claude part is one sentence with no link; one
   relative link at the end to the "Story checks".
10. Holds are a table with "Today these can hold" and "The list grows with each version", so new sources add a row.

**Kept** (decision 41 and the brief's item 7): the crash gap and every path into it, one signed-in user per PC,
`winget upgrade` ends `wakeshot run` and leaves Wakeshot off until sign-in, the credit line (no port), what it changes
and what an uninstall leaves (with `NotifyIconSettings` and Velopack's own logs), the commands (help block equal to
`USAGE`), privacy, limits, the story, the three community links, the icon loop, menu and tooltip stills, state icons,
the video comment. The fold texts are the round 4 texts; only "Once Wakeshot is on winget:" and "Claude Code only, in
v1" were reworded for launch day and for more sources.

| Check | Outcome |
|---|---|
| Relative links and anchors (`check_readme.py`) | pass: 46 of 46 |
| No em or en dash, no emoji or other non-ASCII; UTF-8, LF, no BOM | pass (README, `docs/research/readme.md`, this card's round 5 lines) |
| Install command place | line 26; 9 counted lines before it (image tags, comments, blanks not counted) |
| Comparison table | 5 rows; no "Velopack", "MB" or "KB" above its end |
| External links (`--web`, one GET each, 10:58) | 8 of 8 gave 200 (`softwareok.com` is back) |
| Help block equals `USAGE` in `args.rs` (`check_help.py`) | pass |
| GitHub render (`gh api markdown`, gfm, repo context) | 5 tables, 9 folds, 8 `<picture>` each with a dark `<source>` and alt text, 1 note alert, no comment text shows; a repo README renders alerts (Ice's, `application/vnd.github.html`) |
| `site/check_page.py site` | pass: 0 errors, 7 spots wait (unchanged) |
| Other checks that read `README.md` | none (`media/scripts`, `site/check_page.py` and CI grepped) |

**Site lines that now differ** (for `p6-pages`; none says the opposite, so `site/index.html` is not edited):

1. Hero lead: "Your PC stays awake while your AI agent works"; the README leads with "while something needs it" and
   names the holds.
2. Hero speed line with three numbers; the README keeps two numbers, deep in the folds.
3. Install: the site leads with the `Setup.exe` button and keeps winget for a Phase 8 comment; the README leads with
   winget and the Start menu step.
4. "A night with Wakeshot" step 1: "You give Claude Code a long task"; the README: "You start a long job".
5. The site's comparison spot waits for "the reworked README": the 5-row table is ready to take.

**Found outside scope (round 5).**

1. `docs/product.md` "README (Haik, 2026-11-04)" still says the top shows the speed numbers, then the side by side
   table, and that the README says macOS is planned. Haik's 2026-11-06 words change the order and the numbers; the
   README now says macOS comes if Wakeshot gets popular or gets donations (his round 4 words). The line needs his new
   words.
2. This card's "Deliverables" and "Acceptance checks" (top at most 15 lines before the rival table, a source link in
   every rival cell, speed numbers at the top) are the round 1 rules; round 5 replaces them.
3. Art of README and common-readme (`hackergrrl/*`) are gone from GitHub (404); read in the Wayback Machine.
