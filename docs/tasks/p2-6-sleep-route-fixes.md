# Fixes after the sleep route review; shorter notices; the reset covers the short timeouts and uninstall

Phase: 2 to 6 (core and power). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens.
Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from `docs/reviews/p2-6-sleep-route-review.md`
(0 blockers, 3 majors, 5 minors), the screen runs of `p5-notices` (four notices cut after five lines) and
`p2-6-sleep-route` "Found outside scope".
Runs after: `p2-6-sleep-route` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-sleep-route-fixes\target`.

## Goal

Close the review's findings in the leanest way, each with a test, and three small items found around it. Haik's
rules: with no work going on, the laptop behaves as if Wakeshot were not there; a closed laptop never runs unlocked
with no path to sleep or lock ("Never lower security"); a lost journal goes to Windows defaults, with a notice.

## Read first

- `docs/reviews/p2-6-sleep-route-review.md` (each finding names files, lines and a fix).
- `docs/tasks/p2-6-sleep-route.md` and `docs/tasks/p2-6-record-lost.md` (Results and decisions: do not undo them
  without a reason in your Result).
- `docs/product.md`: "Never lower security", the work end rule, "User settings safety" 1 to 7.

## Deliverables

1. Review major 1 (a closed lid in the unknown window, then a hold that starts or resumes): keep the pending window
   lock until the lid opens or the session locks; lock once a hold holds on Wakeshot's Do nothing. One scenario per
   path.
2. Review major 2 (a lost journal while the 60 s timeouts are in force). Orchestrator decision, for Haik's review, in
   line with his rule 7 ("put it to windows defaults"): the reset also puts back each of the four short keys that
   holds exactly Wakeshot's short value; the notice words cover it; case B (a damaged file seen while the short
   timeouts are in force) treats them as lost too. Tests for case A and case B.
3. Review major 3 (uninstall with a damaged journal): the uninstall hook runs the same reset (log only; a hook shows
   no notice). One hook test with a corrupt file.
4. Review minors 4 to 7, each as the review suggests or leaner; say which in the Result. Minor 8 (`product.md`) is
   done by the orchestrator.
5. `text.rs`: the four notices cut after five lines on the real taskbar (`p5-notices` Part B, 2026-11-04: restored by
   user, lock failed, update Wakeshot, journal lost; pictures in `docs/research/img/ux/notice-*.png`) get shorter
   words that fit, keeping their meaning and `text.rs` rules. List old and new words in the Result; Haik reads them at
   `p5-haik-check`.
6. `watch::agents::tests::a_folder_that_cannot_be_listed_reads_its_known_files` fails in agent shells since about
   12:00 on 2026-11-04 (also on f47391d; CI passed it there): the listing is not denied. Find out why (the deny ACL,
   elevation, the temp folder) and make the test check what it means to check, or skip it with a printed reason when
   the deny does not take. Not a product change.
7. `docs/architecture.md`: the lines that state these rules (section 5 "Gap left" among them).

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets`, x64 and ARM64 (the tray's ARM64: CI only);
  `cargo test --workspace --locked` (debug only, never `--release`); the core props at 4096 cases once.
- Mutants only with one job, a low timeout and a memory cap; stop if free memory drops under 10 GB.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The dim timeout (review unverified note): it goes to Haik's installed-app check. `README.md`, `site/`, phase files,
  `docs/product.md` (tell the orchestrator if a product line is wrong).

## Questions for Haik

None. Decide the rest yourself and record each choice as "decided by the worker, for Haik's review".

## Found outside scope

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result
