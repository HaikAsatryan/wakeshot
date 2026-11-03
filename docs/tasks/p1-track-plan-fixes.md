# Fix the track plans from the cross-track review

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: the cross-track review (`docs/reviews/p2-6-cross-track.md`, done). Parallel with: `p1-contracts-split`
(it owns `crates/`, `Cargo.*`, `.github/`, `CLAUDE.md`, `docs/architecture.md`; it lists card edits in its Result,
which the orchestrator sends you when it ends). This card owns `docs/phases/02-core.md` to `docs/phases/06-ship.md`
and every `docs/tasks/p2-*.md` to `docs/tasks/p6-*.md` card, plan cards included.

## Goal

Make the five track plans safe to build in parallel: zero shared files, no stale links, every laptop and Haik card
marked, and one short sign-off list Haik reads to approve all five.

## Read first

- `docs/reviews/p2-6-cross-track.md` (2 blockers, 10 majors, 19 minors, totals).
- `docs/plan.md`: "How we work", "Git", the note under the Phases table (the orchestrator updated the check rule).
- The five phase files and their cards.

## Deliverables

1. Every finding fixed in the phase files and cards, or listed in the Log with the reason. Blockers first:
   - B1: no card requires whole-workspace checks while other cards run. Each card checks only its own package and
     files, as `docs/plan.md` "How we work" now says.
   - B2: `p3-haik-checks` (sign-out) runs only when no other agent runs; say so in its header.
   - M1: replace every `p3-restore` link with `p3-journal` plus `p2-ownership-restore`, and add `p3-power-plan` where
     the review says.
2. The card edits `p1-contracts-split` lists in its Result, when the orchestrator sends them.
3. `docs/phases/02-06-signoff.md` (new, at most about 40 lines): what the five tracks build, card count and token
   total per track, Haik's minutes per track, laptop sessions, the critical path, and every question the planners
   left for Haik, each with its default. Haik signs off all five plans from this one file.

## Acceptance checks

- Each finding is fixed or listed with a reason.
- No card names a card that does not exist; no two cards that can run at once own one file.
- The sign-off file stands alone.

## Out of scope

- Code, `crates/`, `docs/architecture.md`, `docs/product.md`, `docs/plan.md`, the Phase 1 file. Git.
- The integration plan (the orchestrator plans it after the tracks).

## Questions for Haik

Each with the worker's default.

None that block. The planners' questions, with defaults, are in `docs/phases/02-06-signoff.md` (12 items). One there
is this card's: the uninstall leaves Windows' `NotifyIconSettings` record of the tray icons (review m17); default:
leave it, the README says so.

## Found outside scope

Problems seen but not fixed.

- `docs/plan.md` step 2 (out of scope here): review m14's "the orchestrator caps parallel builds" is not written
  anywhere; the phase notes carry only "delete the card's target folder when done". Review M5's ARM64 rule went into
  the five phase notes instead of `plan.md`.
- Owner doc lines in `crates/` (contract files): `text.rs` names only `p5-text` and `art.rs` only `p5-icon-art`, but
  `p5-haik-check` takes both over after Haik's answers (planned hand-over); the `p4-live-*` cards fix done files
  (`sources/*`, `cron.rs`, `holds/file.rs`, `watch/*`, `lockowner.rs`) that their doc lines do not name. Suggest the
  next contracts change adds "then `p5-haik-check`" and "fixes by the `p4-live-*` cards" to those lines.
- `docs/architecture.md`: line 612 still names `restore_from_journal`; line 943 names the test plan copy "Wakeshot
  test" (cards now use "Wakeshot test <pid>" and "Wakeshot test e2e"); section 6 has no line for `store::AutoStart`
  as the one Run value writer (review m12).
- Plan cards keep stale text in their own "Found outside scope" and Results (`p3-plan-windows-power` line 99,
  `p6-plan-ship` lines 91 and 209: `p3-restore`, "the Run value stays out of `store`"). Kept as history; each of the
  four plan cards got a dated Log line that says how to read it.
- `p3-session`'s `register_on_hidden_window` creates a window in plain `cargo test`, so CI runs it; whether a GitHub
  Windows runner allows that: not verified. The first CI run shows it.

## Log

- 2026-11-03 17:40: card written by the orchestrator after the cross-track review.
- 2026-11-03 17:43: worker started. Read the card, `docs/plan.md`, the cross-track review.
- 2026-11-03 17:51: B1 card part done by a script (scratchpad `b1_checks.py`): 56 cards' Acceptance checks now use
  `rustfmt --edition 2024 --check <owned files>`, `cargo clippy -p` and `cargo test -p`, and end with one "Check rule"
  bullet. `p6-velopack-dependency` keeps whole-workspace runs (it runs alone). Checked first: a bare `rustfmt` uses
  the 2015 edition and also checks child modules; `cargo test a b` is an error, filters go after `--`.
- 2026-11-03 17:55: B1 phase notes added (02 note 20, 03 note 2, 04 note 18, 05 section 7, 06 subsection). B2 done
  (`p3-haik-checks` header, 03 table, `p3-real-checks` steps 2 and 5). Card parts done: M1 (`p4-cli-restore`,
  `p6-hooks`, `p3-journal` forms), M3, M7, M8 (`p3-store` `ScratchKey` helper as the one rule), M9, m5 (test moved
  to `p3-recovery`), m11, m12 (Run value: `store::AutoStart` only), m15 (module filters), m17 (`p6-hooks`: leave the
  icon entries). Next: phase file parts, M2, M4 to M6, M10, the rest of the minors.
- 2026-11-03 18:02: `p1-contracts-split` card edits applied (orchestrator relayed them): 1 `p2-ownership-restore`
  (extend the working `FakePlans`; API add-only; `Stop` is a panic); 2 `p3-power-plan` (no `PowerApi`/`FakePower`,
  `PlanIo` directly); 3 `p3-journal` (locked stubs exist); 4 `p4-cli-args` (remove the one `#[expect(dead_code)]`);
  5 `p4-cli-restore`, `p6-hooks`, 04 and 06 (with M1); 6 `p4-hold-files`, `p4-watch-data`, `p6-hooks`
  (`files::read`); 7 `p5-sounds`, `p5-notices` (fakes in their files); 8 `p5-icon-art` (placeholder icon, no build
  change); 9 `--locked` on every cargo command in 54 cards (script `locked.py`) plus the clippy run without
  `--all-targets` in the rule bullet and the five phase notes; 10 `02-core.md` module layout. Also: `p2-core-lid` 9a
  and 02 Coverage now say the update contract was taken, `Updated` waits on ship Question 3; 05 file table says the
  fakes moved. Done since 17:55: M2, M4, M5, M6, m1 to m4, m6, m7, m10, m13, m16, m19. M10 not done: out of scope
  (the card says the orchestrator plans integration); `p4-watch-thread` now says "integration".
- 2026-11-03 18:06: plan card Logs noted (history kept); `docs/research.md` "Track plan fixes checks" added; sign-off
  file written; long lines re-wrapped; acceptance checks run. Status done.

## Result

Done 2026-11-03 18:06.

Findings of `docs/reviews/p2-6-cross-track.md`, each fixed or listed:

| Finding | Outcome |
|---|---|
| B1 | Fixed. One "Checks while other cards run" note in each phase file (02 note 20, 03 note 2, 04 note 18, 05 section 7, 06 subsection), pointing at `docs/plan.md` step 2; every card's checks use own files and `-p` only, and end with a "Check rule" bullet. `p6-velopack-dependency` keeps whole-workspace runs (it runs alone). |
| B2 | Fixed: `p3-haik-checks` runs alone on the whole laptop (no agent, no cargo); 03 table; `p3-real-checks` step 5. |
| M1 | Fixed in `p4-cli-restore`, `p6-hooks`, `p3-journal` forms, 04 and 06 (table and order). |
| M2, M3, M4, M6, M7, M8, M9 | Fixed as the review says. M8 uses one helper: `p3-store`'s `ScratchKey` and `sweep_scratch_keys`. |
| M5 | Fixed in `p6-velopack-dependency` (runs alone) and 06 order (clang install as its own step). The `plan.md` rule went into the five phase notes (plan.md is out of scope). |
| M10 | Not done: the card puts the integration plan out of scope (the orchestrator plans it). `p4-watch-thread` now says "integration". |
| m1 to m8, m10 to m13, m15 to m17, m19 | Fixed as the review says. m12: `store::AutoStart` is the one Run value writer; `install.rs` keeps only the Path. m17: decided, leave the icon entries (sign-off item 12). |
| m9 | Not done here: the contracts card chose to wait for ship Question 3; it is sign-off item 7. |
| m14 | Half: "delete your target folder" is in the phase notes; the build cap belongs in `plan.md` (Found outside scope). |
| m18 | `p6-velopack-dependency` fixed; `p1-contracts-split.md` is not this card's, and it is done (no `FakePower` left). |

`p1-contracts-split` card edits 1 to 10: all applied (Log 18:02). Edit 9 added `--locked` to every cargo command in
54 cards and the clippy run without `--all-targets`.

Files changed: `docs/phases/02-core.md` to `06-ship.md`; new `docs/phases/02-06-signoff.md` (42 lines); 65 of the
68 `p2-*` to `p6-*` cards (all but `p5-plan-tray-ux`, `p6-pack-and-release`, `p6-winget-manifest`; four plan cards
only got a Log line); `docs/research.md` (new section "Track plan fixes
checks"). Scripts in the scratchpad (`b1_checks.py`, `b1_notes*.py`, `locked.py`, `check_links.py`,
`critical_path.py`, `widths.py`, `fix_widths.py`).

Checks:

- Each finding fixed or listed: the table above.
- No card names a missing card: `check_links.py` over every `p<N>-name` in the track cards, phase files and sign-off
  file. Left: history in the 03 work log and in two plan cards' "Found outside scope" (the dropped `p3-restore`), and
  three globs (`p2-ownership-*`, `p4-cli-*`, `p4-live-*`). Every "Runs after" name is a real card
  (`critical_path.py`: none missing).
- No two cards that can run at once own one file: the review found none; this card's ownership changes are each
  sequential or own-file (`tests/claude_status.rs` to `p4-claude-transcript` after `p4-claude-status`; one attribute
  line in the CLI `command.rs` to `p4-cli-args`, edit 4; a test moved into `p3-recovery`'s own file; `ScratchKey` in
  `store.rs`; the Run value code dropped from `install.rs`). Shared resources fixed: test keys per pid and test, plan
  copies per pid, the dry-run folder, `Cargo.lock`.
- The sign-off file stands alone: tracks, cards, tokens, Haik's minutes, laptop sessions, critical path (recomputed:
  730K, 8 cards, through the two agent sessions with Haik), 12 questions with defaults, ship Question 3 included.
- No em or en dashes in the changed files; no new line over 120 characters outside tables.

Open items: the integration plan (orchestrator); the `plan.md` build cap; owner doc lines and `architecture.md` text
(Found outside scope).
