# Apply the tray review and the tray menu research to the docs

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`.
Runs after: the tray Result review (`docs/reviews/p0-tray.md`) and `p1-tray-menu-research`, both done 2026-11-03.
Parallel with: `p1-detailed-design` and `p1-lid-sound`. Shared file: `docs/research.md` (both of them may write it):
re-read it right before each edit and match on text, not line numbers.

## Goal

Fix the tray card's record with the review's findings, and carry the research card's decisions into the docs and
into `p1-tray-menu-look`, so that card starts from correct facts.

## Read first

- `docs/reviews/p0-tray.md`: findings 1 and 2, then "Edits for the orchestrator" (exact old and new text).
- `docs/tasks/p1-tray-menu-research.md`: "Questions for Haik" (decisions 1 to 4, taken under Haik's delegation of
  design calls) and Result ("For the orchestrator").
- `docs/tasks/p1-tray-menu-look.md`.

## Deliverables

1. Every edit in "Edits for the orchestrator" of `docs/reviews/p0-tray.md`, to `docs/research.md` and
   `docs/tasks/p0-spike-tray.md`, plus the two optional ones (`spikes/tray/README.md` leftovers, the `product.md`
   pointer). If an old text no longer matches, apply the intent and say so in the Log.
2. `docs/research.md` "Tray, menu and notifications": a link to `docs/research/tray-menu-look.md`.
3. `docs/product.md` UX: the menu follows the Windows mode (`SystemUsesLightTheme`), like the tray icon, with the
   one-line reason from research decision 3. Replace the current line; do not add history.
4. `docs/tasks/p1-tray-menu-look.md`: add research decisions 1 to 4; the on-screen test of the owner-drawn menu must
   cover check marks, keyboard use, high contrast and DPI scaling; remove the spike leftovers (two
   `NotifyIconSettings` entries, the `NotifyIconGeneratedAumid_...` key, the `guidmove` exe copy) at the card's end;
   the Phase 5 toast icon check from review finding 1, unless the review's edits already put it in `research.md`.
   Also add to its header: runs after `p1-lid-sound` and this card (screen and sound cards run one at a time).

## Acceptance checks

- Each of the review's numbered edits is applied or listed in the Log with the reason it was changed or skipped.
- No edit to `docs/architecture.md`, any phase file, `docs/tasks/p1-lid-sound.md` or `docs/tasks/p1-detailed-design.md`.
- Nothing run on the laptop: no registry writes, no spike runs, no deletes outside your scratch folder.

## Out of scope

- Removing the spike leftovers now (`p1-tray-menu-look` does it at its end).
- Any new research or screen work.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

- Stale line-number pointers into `research.md`: `product.md:91` cited `research.md:485` for the 113 C trip point
  (fixed since: it now cites the bullet "Limits: Kernel-Power 125"; noted by `p1-tray-menu-look`); `architecture.md:41` cites `research.md:381` and
  `architecture.md:51` cites `research.md:378`, both now inside the tray section. Cite bullet names instead.
  `architecture.md` is being rewritten by `p1-detailed-design`.
- Review points with no numbered edit, so not applied here:
  - Finding 13: the evidence for `IsPromoted` live, the unregistered toast "no banner" and the tooltip lives only in
    the temp folder `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\06511897-9625-4d7a-b494-37b2b0bd9eb2\scratchpad`
    (still there at 15:06: `s0-taskbar-before.png`, `s0-taskbar-promoted.png`, `tray-shots\125822-T4-toast-unreg-a.png`).
    The review asks to copy non-private crops into `docs/research/img/` before it is cleaned. Also: Phase 1 logs
    should stamp wall time on every line (the spike's ms clock drifted 16 s).
  - Finding 14: `NIN_BALLOONTIMEOUT` arrives twice at the same ms (I saw it in `log-auto.txt` 15696 ms and
    `log-interactive.txt` 9522219 ms), so Phase 1 must not count balloon events; Phase 5 should try a different
    `DisplayName` once; the `SND_SYSTEM` conflict belongs to the lid-cue pick (`p1-lid-sound`).
  - Finding 2: a balloon is filed under `NotifyIconGeneratedAumid_<NotifyIconSettings id>` and is not stored (0 rows).
    Finding 6: a code-opened menu may not close on an outside click (KB135788). Finding 12: reload the icon on
    `WM_DISPLAYCHANGE` and `TaskbarCreated`, not on `WM_DPICHANGED`.

## Log

- 2026-11-03 14:58: card written by the orchestrator after Haik's parallel session finished both tray agents.
- 2026-11-03 14:59: worker started. Read CLAUDE.md, plan "How we work", this card, the review, both tray menu cards.
- 2026-11-03 15:01: all 19 old texts still match (`research.md` edits 1 to 11, `p0-spike-tray.md` 12 to 18; 19 is an
  add). Spot-checked review facts read-only: `out/toast-icon.png` decodes to opaque RGB (43, 124, 211) only (stdlib
  PNG decoder in my scratchpad); x64 exe 519,168 bytes, ARM64 498,176 bytes (01:43); `log-interactive.txt` has
  `SetForegroundWindow=true` at 9753822, 9755691, 9783915, 9785768 ms and false at 4675236 to 6315311 ms; nine
  `0x7e`, no `0x2e0`, and `main.rs:1110` would log both; `toast.rs:115` calls `GetHistoryWithId`. Next: research.md.
- 2026-11-03 15:04: applied review edits 1 to 19, the README and `product.md` pointer edits, the `research.md` link
  and the `product.md` menu theme line. Lines rewrapped to 120 where a new text made them long. Next: the
  `p1-tray-menu-look` card.
- 2026-11-03 15:05: `p1-tray-menu-look` updated: "Decided" (research decisions 1 to 4), owner-drawn test items in
  deliverable 5, leftover removal as deliverable 6 (full key names and the exe path, found by read-only `ls`), two
  acceptance checks, new "Runs after". The Phase 5 toast icon check stays in `research.md` (edit 1), with the
  review's method added there, so it is not in that card.
- 2026-11-03 15:06: acceptance checks run (Result). Status done.

## Result

Status: done.

What was done: applied the tray review's 19 numbered edits and its 2 optional ones, linked the tray menu research,
set the menu theme rule in `product.md`, and carried the research decisions into `p1-tray-menu-look`. Docs only.

The 19 review edits:

- As written (only rewrapped): 3, 4, 5, 6, 8, 9, 11, 12 to 18. That is 14.
- Changed: 5.
  - 1: added the review's Phase 5 check method (pure red test icon, fresh `IconUri` path) from finding 1, so the
    check lives in `research.md` and not in `p1-tray-menu-look`.
  - 2: also corrected `GetHistory` to `GetHistoryWithId` (finding 3; `toast.rs:115`).
  - 7: "(03:54)" became "(built 03:41)": 03:54 is the card's Log time, the exe's file time is 03:41. Also marked the
    ARM64 486 KB as the 01:43 build from before the token code (finding 8; file time read).
  - 10: "without `FlushMenuThemes`" became "without another `FlushMenuThemes`", since the new text adds one at start.
  - 19: the review let Haik decide on the leftovers; research decision 4 already decided, so the line records it.
- Skipped: 0. Optional edits (`spikes/tray/README.md` leftovers, `product.md` pointer): both applied.

Before editing, I spot-checked the review's key facts read-only (Log 15:01): all held.

Files changed: `docs/research.md` (tray section only), `docs/tasks/p0-spike-tray.md`, `docs/product.md` (UX: menu
theme line and the overflow pointer), `spikes/tray/README.md`, `docs/tasks/p1-tray-menu-look.md`, this card. Scratch
only: `pngpix.py`, `check_edits.py` in my scratchpad.

Acceptance checks:

- Each numbered edit applied or listed with its reason: passed. `check_edits.py` finds every new text and none of
  the replaced old texts (27 of 27, including the card's own deliverables).
- No edit to `docs/architecture.md`, any phase file, `docs/tasks/p1-lid-sound.md` or `docs/tasks/p1-detailed-design.md`:
  passed, by my own edit record (their file times cannot prove it, since other workers edit them now).
- Nothing run on the laptop that changes state: passed. Only file reads, `ls`, `Get-Date` and two read-only Python
  scripts in my scratchpad. No registry access, no spike run, no deletes.

Open items: see "Found outside scope": stale `research.md:<line>` pointers in `product.md` and `architecture.md`,
and review points with no numbered edit (the temp-folder evidence to copy before it is cleaned is the urgent one).
