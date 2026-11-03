# The v1 README

Phase: 6. Status: ready. Needs Haik: yes: he reads the result at the Phase 6 sign-off. The README states his answers
to Phase 1 Questions 10 to 12 (`docs/phases/01-design-skeleton.md`) and to Questions 1 to 3 of `p6-plan-ship`, plus
sign-off item 12: all answered with the defaults (`docs/phases/02-06-signoff.md`, Haik 2026-11-03 18:09). Agent:
`wakeshot-worker`. Size: about 70K tokens.
Real laptop: no.
Runs after: `p6-pack-and-release` (install steps) and `p6-install-e2e` (what an uninstall really
leaves); the sources track's `p4-hold-files` (`docs/hold-files.md`), `p4-cli-args` (the command help), `p4-live-claude`
(what is tested live; v1 supports Claude Code only, Codex and Copilot come after v1, Haik 2026-11-03); the tray
track's `p5-haik-check` (final menu words and icon)
(review m2). Parallel with: any card that does not own `README.md`.
The README also says that Windows keeps its own record of the tray icons it has seen (`NotifyIconSettings`), which an
uninstall leaves (`p6-hooks`, review m17).
Build folder: none (no Rust).

## Goal

A README for users that says what Wakeshot does, how to install it, what it changes on their PC and every case where
a change can stay behind (decision 41).

## Read first

- `docs/product.md` (all of it: it is short); `docs/architecture.md` section 3, "Decisions and where they live", row
  41; section 5, "Gap left"; section 6, "Velopack hooks".
- `docs/phases/06-ship.md`: "Package and install values", "Hooks", "Updates".
- The answers named above (`docs/phases/02-06-signoff.md`); `docs/hold-files.md`; `crates/wakeshot-cli/src/args.rs`
  (the help text).
- `README.md` as it is now (`p1-repo-files`).

## Deliverables

`README.md`, owned whole, in plain short English, no em dashes, no emojis:

- What Wakeshot does, in three or four lines; Windows 11, x64 and ARM64, per user, no admin.
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
- A credit line for each port from a rival (none so far: `docs/phases/04-sources.md`, note 16); license.

## Acceptance checks

- A checklist in the Result: every item above and every README promise in `product.md` and `architecture.md`
  (search both for "README"), each with the line that covers it.
- Every fact in the README is in `product.md`, `architecture.md` or `research.md`; the Result lists the source for
  each limit line.
- A script checks that every relative link resolves, and that the file has no em dash and no emoji.

## Out of scope

- The Pages page (`p6-pages`); the Phase 8 trim; any doc but `README.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
