# Docs pass for Phases 2 to 6: make the docs match the built code

Phase: 6. Status: done. Needs Haik: no.
Runs after: the Phase 2 to 6 track cards named below. Parallel with: `p6-int-window`, `p6-int-startup`, `p5-text`,
the power fix round (core), the Claude parsing review.

## Goal

Make `docs/architecture.md`, `docs/research/claude-code-signals.md`, a few code doc comments and two phase-file
sections match the code as built, using the items the track cards left for a docs pass.

## Read first

- The code each item names, and the named section of each source card in `docs/tasks/`.
- `docs/architecture.md` sections 5 to 10.

## Deliverables

Items (source card in brackets):

1. `docs/architecture.md` section 7 "Waiting limit": the run-time cap (`p2-policy-holds` Result, decision 3).
2. Section 5 Apply steps 3 and 4 and the accepted cases under "Ownership" (`p2-ownership-apply` Found outside scope
   1, 2, 6), and the restore's own window between steps 3 and 4.
3. Plan reads and retries happen at most once per 10 s at a tick; the restore at hold end runs only when something
   may be owned (`p2-core-settings` Found outside scope 2).
4. Section 5 hibernate order: the privilege and `IsPwrHibernateAllowed` checks run before the sound (`p3-hibernate`
   Result).
5. The Guard line: the guard leaves a corrupt journal for the next sign-in's restore (`p3-recovery` Result).
6. Section 8: `watch/read.rs` returns `io::Result`, not `WinError` (`p4-watch-reads` Result).
7. Section 6: two stale lines and the old pointer in the `files.rs` module doc (`p4-hold-files` Result); two Velopack
   facts now checked but still called unverified (`p6-velopack-facts` Result).
8. Section 10 wording with `p1-contracts`' corrections (`docs/phases/01-design-skeleton.md` line 478).
9. `docs/research/claude-code-signals.md`: "15 s early" for `*/5` (`p4-cron` Result).
10. Code doc comments: `policy::lid` module doc, `TestPlan::sweep`'s doc, the docs pass lines in `p2-core-lid`.
11. Two phase-file sections: track note 4 in `docs/phases/03-windows-power.md` (`p3-stop-points` Found outside scope
    1); the Updates part of `docs/phases/06-ship.md` (`p6-velopack-facts` Result).
12. Added by the orchestrator (Claude parsing review m8, bullet 1): section 7's "Rule 5 does not cover `claude
    --resume`" line is done in the code.
13. Added by the orchestrator (m8, bullet 2): `claude-code-signals.md` rule 3 closes on any `<status>`; "Short answer"
    is stale. m8's third bullet stays for a later fix worker.

## Acceptance checks

1. Each item is either changed to match the code, or recorded as already fine, with the code read first.
2. Every edited `.rs` file passes `rustfmt --check --edition 2024 <file>`.
3. No em dashes and no emojis in the changed text.
4. No edit in the do-not-touch parts: `docs/architecture.md` section 3 lines on the battery floor, the lid lock and
   the closed-lid report, the section 8 "Lock fails" row; `crates/wakeshot-core/src/core.rs`, `core/`, `scenarios/`,
   `sim.rs`, `text.rs`; `crates/wakeshot-windows/src/app/`; `docs/phases/` beyond item 11.

## Out of scope

Code behavior; any doc not named above (problems go under "Found outside scope"). No build, no git.

## Questions for Haik

None.

## Found outside scope

1. `docs/architecture.md` section 11, the "cron next time" row: still "open", "candidates `croner` 4.0.2 and `jiff`
   0.2.39; Phase 4 picks or writes its own". `sources/cron.rs` is our own code, and no `Cargo.toml` or `Cargo.lock`
   names `croner` or `jiff` (grep). Stale row.
2. `docs/phases/06-ship.md` Updates, the bullet before the one I edited: "applies only when no hold counts, no hold
   file names a live process ... and no key is owned", "on success the core issues `Quit`". The code also waits for
   no countdown, work end or hibernate and no session end, skips an expired Keep awake file, and takes the menu's
   Quit path (`core/update.rs`). Section 3 now says so; the phase file was outside item 11's wording, so left.
3. `p2-core-lid` Found outside scope 4, "a lid close on our Do nothing with nothing holding": I wrote the countdown
   half in section 3's Countdown paragraph. The lock and chime half belongs in the "Lid closes" bullet, which is in
   the do-not-touch lines; the worker who owns those lines can add "also with nothing holding (a failed restore)".
4. `docs/research/claude-code-signals.md` rules table, rule 5: still lacks the resume and reminder texts proposed in
   "Phase 4 live shapes", "Rule changes" (the code follows them). Noted in the file; not on this card's list.
5. `docs/architecture.md` section 5 restore step 7 says the Velopack hooks "retry within their 30 s limit", and
   section 6 calls the uninstall limit 30 s. Research "Velopack checks for Wakeshot", point 6, measured install 30 s,
   updated 15 s, uninstall 60 s (the Velopack docs say 30 s); `p6-velopack-facts` set inner limits of 25 s for
   install and uninstall and 10 s for updated. So the updated hook's 15 s is not named, and the uninstall figure is
   the documented one, not the measured one.
6. `docs/phases/03-windows-power.md` work log lines 199 to 203 still list the track note 4 text and the
   `TestPlan::sweep` doc as open; both are done here. Orchestrator lines, so left.

## Log

- 2026-11-03 23:44: card created, Status in progress.
- 2026-11-03 23:45: item 1 already fine (section 7 has the run-time cap; matches `sessions.rs` lines 170 to 178).
  Item 2: Apply steps 3 and 4 and the step 2 to 5 window already fine; added the restore's step 3 to 4 window under
  "Ownership" (checked in `ownership/restore.rs` steps 3 and 4).
- 2026-11-03 23:48: item 3 done from `core/settings.rs` (`PLAN_READ_EVERY_MS`, `tick`, `may_own`, `restore_all`):
  the `ReadPlan` comment, the safety tick line, "Holding ends", the retry paragraph and the console-left bullet in
  section 3; the `ReadPlan` bullet and restore step 7 in section 5; the "Restore fails" row in section 8. The user's
  Restore, Quit and session end restores go out whatever the mirror says (lines 229 to 288), so the "may be owned"
  rule is written only for the hold end, console left and floor restores.
- 2026-11-03 23:49: item 4 done (section 5 hibernate order, from `power/hibernate.rs` `run_hibernate` lines 103 to
  129). Item 5 done (section 5 Guard line, from `recovery.rs` `try_once` lines 728 to 745: a corrupt journal is left
  by the guard, set aside by restore mode; a journal naming another tray ends the guard).
- 2026-11-03 23:51: item 6 done (section 8 opening: `watch::read` is the one `io::Result` module; grep of `crates/`
  finds `io::Result` only in `watch/read.rs`). Item 7: "at or before" and the no-`jiff` line were already fixed in
  section 6; the file table's "documented for users in Phase 4" now names `docs/hold-files.md`; `files.rs` `HoldFile`
  doc names `docs/hold-files.md` and `holds::file` (rustfmt check exit 0); section 6's dotted id and `current` path
  facts now cite research "Velopack checks for Wakeshot", points 1 and 3.
- 2026-11-03 23:52: item 8: `p1-ci`'s section 10 words and `p1-contracts`' corrections (sections 3, 6, 7, 11) were
  already in. Checked section 10 against `.github/workflows/ci.yml`: added the clippy run without `--all-targets`,
  `--locked`, and the Linux job's `cargo tree` check, which the doc did not name.
- 2026-11-03 23:53: item 9 done: `claude-code-signals.md` "Timers" jitter bullet now gives the 285 s rule (checked in
  `sources/cron.rs` lines 320 to 336 and research "Cron checks"); the "Phase 4 live shapes" line that pointed at the
  old "4 min 45 s gap" words points at the 285 s rule.
- 2026-11-03 23:56: item 10 done. `policy/lid.rs` module doc names the in-force view (`core/lid.rs` `in_force`,
  lines 37 to 69). `TestPlan::sweep` doc names `power_check cleanup` as a caller (`examples/power_check.rs` line
  1159; the other callers, `plan.rs` and `stop_points.rs`, are `real_` tests). Both rustfmt checks exit 0.
  `p2-core-lid` Found outside scope 4, from `core/lid.rs`, `core/update.rs`, `core.rs` `notify`: section 3 now has
  the in-force view (after "State"), held notices (after `Notice`), the countdown pause off the console, the pause
  on the user's own Do nothing with its 30 s restart, a lid close on our Do nothing with nothing holding, the full
  `ApplyUpdate` wait and `Update(Ok)` taking the Quit path. The closed-lid report wording was already right (and is
  in the do-not-touch lines). Its item 1 (5 s settle in other docs): `p6-int-haik-checks.md` already fixed; the
  rest are history or phase files, left.
- 2026-11-03 23:58: item 11 done. `03-windows-power.md` track note 4 takes `p3-stop-points`' proposed text (checked
  against `stop_points.rs` `assert_released`, lines 868 to 898). `06-ship.md` Updates: Velopack kills a tray that
  has not quit 60 s after `wait_exit_then_apply_updates` (research "Velopack checks for Wakeshot", point 4).
  Orchestrator added items 12 and 13 (Claude parsing review m8, first two bullets); starting them.
- 2026-11-04 00:01: item 12 done: section 7's "Rule 5 does not cover `claude --resume`" line now states the resume
  rule as built (`claude/mod.rs` line 211 passes `procStart`; `transcript.rs` `timers`, `open_tasks`, `Job::due`).
  Item 13 done: rule 3 in the rules table closes on any `<status>` and on `queued_command` prompts (`transcript.rs`
  `notice_ids`, lines 608 to 633); Phase 0 change 1 keeps its text plus a pointer; "Short answer", the "two outside
  methods" item 1 and the "Wakeup armed" row now say a resume rebuilds `CronCreate` jobs, and wakeups only for `bg`
  sessions (research "Claude transcript checks"); the "Resume findings" note says so. m8's third bullet left alone.
- 2026-11-04 00:02: acceptance checks run (Result); long lines I made rewrapped. Status done.

## Result

Done 2026-11-04 00:02. Every item was checked in the code first.

| # | Outcome |
|---|---|
| 1 | Already fine: section 7 has the run-time cap (`policy/sessions.rs` lines 170 to 178) |
| 2 | Apply steps 3 and 4 and the step 2 to 5 window already fine; added the restore's own window between its steps 3 and 4 under "Ownership" |
| 3 | Done: plan reads and restore retries at most once per 10 s at a tick (sections 3, 5, 8); the hold-end restore only when something may be owned, and which restores ignore that rule |
| 4 | Done: section 5 hibernate order, checks before the sound |
| 5 | Done: section 5 Guard line, a corrupt journal is left for the next sign-in's restore mode; a journal naming another tray ends the guard |
| 6 | Done: section 8, `watch::read` returns `io::Result` (the only such module) |
| 7 | "at or before" and the `jiff` line already fixed; file table and `files.rs` `HoldFile` doc point at `docs/hold-files.md`; dotted pack id and `current` path now marked tested |
| 8 | `p1-ci` and `p1-contracts` wording already in; section 10 now also names the clippy run without `--all-targets`, `--locked` and the Linux `cargo tree` check (from `ci.yml`) |
| 9 | Done: the 285 s `*/5` rule in `claude-code-signals.md`, two places |
| 10 | Done: `policy::lid` module doc, `TestPlan::sweep` doc; section 3 gets the in-force view, held notices, the countdown pauses and restart, the full `ApplyUpdate` wait, `Update(Ok)` as Quit. Closed-lid report wording already right |
| 11 | Done: `03-windows-power.md` track note 4; `06-ship.md` Updates, the 60 s kill |
| 12 | Done: section 7 resume line now states the built rule (at `procStart`) |
| 13 | Done: rule 3 closes on any `<status>` and `queued_command` prompts; "Short answer", "two outside methods" and the "Wakeup armed" row fixed for resume |

Files changed: `docs/architecture.md`, `docs/research/claude-code-signals.md`, `docs/phases/03-windows-power.md`
(track note 4 only), `docs/phases/06-ship.md` (Updates only), `crates/wakeshot-core/src/files.rs`,
`crates/wakeshot-core/src/policy/lid.rs`, `crates/wakeshot-windows/src/power/plan.rs` (doc comments only), this card.

Checks:

1. Each item changed or recorded as already fine: pass (table above, Log).
2. `rustfmt --check --edition 2024` on `files.rs`, `policy/lid.rs`, `power/plan.rs`: exit 0 each.
3. No em dash, en dash or emoji in any changed file (grep and a script): pass. New lines are at most 120 characters
   outside tables.
4. Do-not-touch parts: not edited, from my own edit record (no git): section 3's floor, lid lock and closed-lid
   report lines, section 8's "Lock fails" row, `core.rs`, `core/`, `scenarios/`, `sim.rs`, `text.rs`, `app/`;
   `docs/phases/` only in item 11's two sections. No build run.

Open items: "Found outside scope" 1 to 6.
