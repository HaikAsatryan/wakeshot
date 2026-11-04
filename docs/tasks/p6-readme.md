# The v1 README

Phase: 6. Status: blocked: draft done; waits for `p5-haik-check` and `p6-media`. Needs Haik: yes: he reads the result at the Phase 6 sign-off. The README states his answers
to Phase 1 Questions 10 to 12 (`docs/phases/01-design-skeleton.md`) and to Questions 1 to 3 of `p6-plan-ship`, plus
sign-off item 12: all answered with the defaults (`docs/phases/02-06-signoff.md`, Haik 2026-11-03 18:09). Agent:
`wakeshot-worker`. Size: about 70K tokens.
Real laptop: no.
Runs after: `p6-pack-and-release` (install steps) and `p6-install-e2e` (what an uninstall really
leaves); the sources track's `p4-hold-files` (`docs/hold-files.md`), `p4-cli-args` (the command help), `p4-live-claude`
(what is tested live; v1 supports Claude Code only, Codex and Copilot come after v1, Haik 2026-11-03); the tray
track's `p5-haik-check` (final menu words and icon)
(review m2); `p6-launch-research` (rival pick and table facts, README shape; its Result may change this card, the
orchestrator routes that first); `p6-media` (speed numbers, pictures, loop, video). Parallel with: any card that does
not own `README.md`.
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
     `docs/research/rivals-nightcrew-lidguard.md`, the source the research cites.
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
- `p5-haik-check`: compare the lines above with Haik's final words and icon; fix any that changed.
- First public pre-release: the SmartScreen words and steps (`todo:` line 110).
- Phase 8: the winget lines (`winget:` lines 10 and 116); drop the "not released" line 14 at the first release; read
  the rival stars again if the final pass is on a later day.
- Found outside scope 1 to 6.
