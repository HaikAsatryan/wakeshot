# Docs pass for Phases 2 to 6: make the docs match the built code

Phase: 6. Status: rounds 1 to 4 done. Needs Haik: no.
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
7. `docs/tasks/p6-hooks.md` line 40 gives the updated hook's inner limit as 12 s; `p6-velopack-facts` set 10 s, which
   section 5 now states. The card should follow before `p6-hooks` runs.

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
- 2026-11-04 00:08: orchestrator round 2, Found outside scope 1, 2 and 5. (1) Section 11 "cron next time" row: our
  own scheduler (`sources/cron.rs` module doc; no `croner` or `jiff` in any `Cargo.toml`). (2) `06-ship.md`
  Updates, the `ApplyUpdate` bullet, from `core/update.rs` `update_may_run` and `done_update`; rewrapped. (5)
  Section 5 restore step 7: measured hook limits (research "Velopack checks for Wakeshot", point 6) and the inner
  limits from `p6-velopack-facts`, marked not built (`hooks.rs` is still a stub). For consistency, the 30 s in the
  "Gap left" uninstall line, section 6's hook paragraph and the section 8 "Restore fails" row now read 25 s.

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

Round 2 (2026-11-04 00:08, orchestrator): Found outside scope 1, 2 and 5 fixed (Log 00:08). Same checks: no em dash
or emoji, no new line over 120 characters outside tables, no do-not-touch line edited, `06-ship.md` only in the
Updates bullet. No `.rs` file changed in this round.

Open items: Found outside scope 3 and 4 (the orchestrator sends them to the line owners), 6 and 7.

## Round 2 (2026-11-04)

Status: done.

Items (from the orchestrator):

1. `docs/architecture.md` section 5, "Folder changes" row: several folders on one thread is built and tested
   (`watch/thread.rs`, `p4-watch-thread` Result).
2. `docs/product.md` lines 41 to 46: a UI stop of a background task ends it at its queued `enqueue` notice (M1 part
   b), text from `p4-claude-transcript.md` newest round.
3. `docs/architecture.md` lines 899 to 900: the `timers` and `open_tasks` comments against the Monitor rules.
4. `docs/architecture.md` section 4: `wakeshot run` exit codes 127, 126, 125 (`wakeshot-cli/src/run.rs`).
5. `docs/tasks/p5-menu-paint.md` "Out of scope": icons per menu item (Haik, 17:32).
6. `docs/research.md` "Competitors" table from `docs/research/launch.md`.
7. Haik (2026-11-04 01:14): no paid certificate, free paths only: `product.md` Distribution, `research/launch.md`
   Certum question, `p8-store.md` routing.
8. Haik's sleep values (his change, 2026-11-04 00:31): read on the laptop, then `CLAUDE.md` "Test laptop" and
   `architecture.md` lines 652, 656, 1017. Read only; no power setting changed.
9. `docs/research/claude-code-signals.md` rule 3 cites `product.md:44-50` for "No time cap".

Rules: Edit tool only (other agents edit the same files); never section 3 of `architecture.md`, the lid lines of
`product.md`, the new `claude-code-signals.md` section of `p4-live-claude`, or other cards' `research.md` sections.

### Log

- 2026-11-04 01:33: round 2 started, Status in progress. Item 8 read on the laptop (read only, `powercfg /q`
  Balanced `SUB_SLEEP STANDBYIDLE`): AC and DC index `0x00004650` = 18000 s (5 h); Balanced is the active plan.
- 2026-11-04 01:34: item 1 done (OS events table: `watch/thread.rs` watches the data folder, `holds\` and each agent
  folder on one thread, tested; `watch/dir.rs` `MAX_FOLDERS` 60). Item 2 done (`product.md` UI stop line, the
  proposed text, checked in `transcript.rs` `queued`, lines 297 to 306). Item 3 done (section 7 struct comments,
  from `transcript.rs` `timers`, `task_timers`, `open_tasks` and `claude/mod.rs` lines 212 to 227).
- 2026-11-04 01:35: item 4 done (section 4 CLI: 127, 126, 125 and the child's code passed whole; `run.rs` lines 23
  to 26, 77 to 134 and its tests).
- 2026-11-04 01:36: item 5 done (`p5-menu-paint.md` "Out of scope": the icons entry removed). Item 6 done
  (`research.md` "Competitors": table, stars note and demand line from `launch.md` part 1; the reviews below it
  left). Item 7 done (`product.md` Signing bullet; `launch.md` Certum paragraph and "What this means"; `p8-store.md`
  Status and "Runs after"; Haik's words from `docs/phases/06-ship.md` lines 243 to 245).
- 2026-11-04 01:37: item 8, `architecture.md`: wrong as given, skipped. No line there holds Haik's sleep values (grep
  for 14400, 10800, 18000, "3 h", "4 h", hours). The line numbers come from `p3-real-checks` Found outside scope 3,
  which names three stale "not tested" lines (the two "No apply" points and the side effects of many
  `PowerSetActiveScheme` calls), not sleep values; `03-windows-power.md` line 219 mixed them up. Listed below.
  Item 8, `CLAUDE.md`: not edited. It is agent configuration, and the change came through agents (a peer session,
  then the orchestrator); `03-windows-power.md` lines 225 and 226 say this edit waits for Haik's yes in the
  orchestrator's own chat. Exact text in Result for the orchestrator.
- 2026-11-04 01:37: item 9 done: rule 3 cites `product.md:38-40`. Same table, same kind of dead cite (Haik's rule:
  a cleanup pass fixes dead references): rule 4 `product.md:33` to `:37`, rule 5 `:30-32` to `:35-36`, rule 9
  `:30-31` to `:34-35` (each read in `product.md` after items 2 and 7).
- 2026-11-04 01:38: checks run (Result). Status done.

### Found outside scope

1. `docs/architecture.md` section 5, three lines `p3-real-checks` (Found outside scope 3) found stale: "Plan switch",
   "'No apply' rests on two points not tested"; "Apply and restore run at every hold start ... not known either way";
   section 9, "That a write to an inactive plan changes only that plan is not tested yet". Research "Power track real
   checks" now has both points tested and 50 cycles with brightness unchanged. Not on this round's list.
2. `docs/phases/03-windows-power.md` line 219 calls those lines "sleep values, dock"; they are not (item 8).
3. `crates/wakeshot-core/src/sim.rs` line 79: the `plan` doc says "Haik's own values ... 3 h on AC and 10 min on
   battery, energy saver at 20 %". His values are 5 h and 5 h, energy saver 30 (`03-windows-power.md` line 221). Code
   doc comment, not named here.
4. Rule 2 of `claude-code-signals.md` cites `research.md:82-130` for the unelevated check; line 81 of `research.md`
   now starts the Phase 4 live shapes. Where the unelevated check sits now: not checked.
5. `p6-launch-research` Result proposes more edits that are not in: `product.md` Distribution (a) the Store line, (b)
   the winget line, (c) the README line; and `p8-store` deliverables and checks for when it re-opens. This round
   recorded only Haik's answer and the card's routing.

### Result

Done 2026-11-04 01:38. Every item was checked in the code or on the laptop first.

| # | Outcome |
|---|---|
| 1 | Done: OS events, "Folder changes": several folders on one thread tested (`watch/thread.rs`, `MAX_FOLDERS` 60) |
| 2 | Done: `product.md` UI stop line, the text proposed in `p4-claude-transcript.md`, "not checked live" kept |
| 3 | Done: section 7 comments: `timers` adds Monitor timeouts, `open_tasks` only tasks with no known end |
| 4 | Done: section 4 CLI: 127, 126, 125, and the child's code passed whole |
| 5 | Done: `p5-menu-paint.md` "Out of scope" no longer lists icons |
| 6 | Done: `research.md` "Competitors" table, stars note and demand line from `launch.md` part 1 |
| 7 | Done: `product.md` Signing, `launch.md` Certum paragraph and "What this means", `p8-store.md` Status and "Runs after" |
| 8 | Read on the laptop: AC and DC 18000 (5 h). `architecture.md`: skipped, no sleep values in it (Log 01:37). `CLAUDE.md`: not edited (Log 01:37) |
| 9 | Done: rule 3 cites `product.md:38-40`; the dead cites of rules 4, 5 and 9 in the same table fixed too |

For the orchestrator, item 8's `CLAUDE.md` "Test laptop" text, once Haik says yes in its own chat: replace "sleep
after 3 h on AC and 4 h on battery (Haik raised battery from 10 min on 2026-11-03; read 20:41: DC 14400, AC 10800)"
with "sleep after 5 h on AC and on battery (Haik's own change, 2026-11-04 00:31; read 2026-11-04 01:33: AC and DC
18000)". The dock sentence stays.

Files changed: `docs/architecture.md` (sections 4, 5 OS events, 7 struct comments), `docs/product.md` (Claude Code
UI stop line, Signing bullet), `docs/research.md` ("Competitors" only), `docs/research/launch.md` (part 3 Certum
paragraph and "What this means"), `docs/research/claude-code-signals.md` (rules table cites only),
`docs/tasks/p5-menu-paint.md` ("Out of scope" line), `docs/tasks/p8-store.md` (Status, "Runs after"), this card.
No code file. No power setting written (one `powercfg /q` read).

Checks:

1. Each item changed, or skipped with the reason (table, Log).
2. No em dash, en dash or emoji in the changed lines; no changed line over 120 characters outside tables; LF only in
   every changed file (scratch script over the changed lines).
3. Not touched: `architecture.md` section 3, the lid lines of `product.md`, `p4-live-claude`'s section in
   `claude-code-signals.md`, other cards' `research.md` sections, `docs/phases/`, every code file. No git, no build.

Phase file items for the orchestrator: `03-windows-power.md` line 219 (Found outside scope 2); `06-ship.md` line 243
can say the answer is recorded.

Open items: `CLAUDE.md` sleep values (Haik's yes); Found outside scope 1 to 5.

## Round 3 (2026-11-04)

Status: done.

Items (from the orchestrator):

1. `docs/architecture.md`: the lines `p3-real-checks` Found outside scope 3 lists as stale (the two "No apply"
   points, the side effects of many `PowerSetActiveScheme` calls): tested on the laptop, as that card's Result says.
2. Section 6 uninstall order as built in `hooks.rs` (`p6-hooks` Result); section 5 still says the hooks are not built.
3. `sim.rs` `plan` doc comment: describe the test values as they are, not as Haik's.
4. `claude-code-signals.md` rule 2: cite the `research.md` section by heading, not `research.md:82-130`.
5. `command.rs` `Outcome::Update` doc and `core/update.rs`: "the next `UpdateReady` tries again" is wrong
   (`p6-update-schedule` Result).
6. Docs pass items in the Results of the cards finished since round 2.

### Log

- 2026-11-04 02:52: round 3 started, Status in progress.
- 2026-11-04 02:55: item 1 done: section 5 "Plan switch" (both "No apply" points tested) and the many-calls bullet
  (50 cycles: brightness unchanged, no error or warning, one event 12 per call; flicker, callbacks and AC not
  verified); section 9 names `real_write_to_inactive_copy_changes_only_the_copy` (`power/plan.rs` line 1235);
  section 4 gets the `Global\` create at Medium and the journal lock across High and Medium (all from research "Power
  track real checks"). Item 2 done from `hooks.rs` (`on_install`, `on_updated`, `on_uninstall`, `restore`, limits):
  section 6 hook paragraph rewritten in the built order; section 5 step 7 says `hooks.rs` is built and `velo.rs` does
  not call it yet (`velo.rs` is still a stub); section 4 restore row no longer names the hooks as a starter of
  `--restore`; section 8 "Restore fails" row names every hook's limit. Item 3 done (`sim.rs` `plan` doc). Item 4 done
  (rule 2 cites the `research.md` heading "Reading elevated Claude Code sessions from an unelevated process", where
  the Medium reader check is). Item 5 done from `update.rs` `run` (`posted`) and `apply`, and `event.rs` line 50.
- 2026-11-04 02:57: item 6, the ten cards' Results and "Found outside scope" read. Fixed in `architecture.md`:
  section 4 "Tray startup" step 6 names the unread journal (`p2-6-unread-journal` Found outside scope 4, checked in
  `app/startup.rs` `JOURNAL_RETRY_MS` and `power/executor.rs` `Startup::unread`); section 10 gets the release
  workflow (`p6-pack-and-release` Result, checked in `release.yml`); section 2 `SoundPlayer` gets the built device
  rule with no default device, the chimes made in code and the "No sounds" skip (`p5-sounds` Result, checked in
  `sound/device.rs` `pick`). Already done by others: `command.rs` `WriteConfig` line and section 6 "never written
  over" (`p2-core-settings` round B, Needs another card 1). Nothing for these files in `p6-winget-manifest`,
  `p5-notices`, `p5-menu-view`, `p5-tray-icon`, `p4-cli-restore` CI fix. Three paragraphs in sections 2, 6 and 7 that
  my edits or older edits left over 120 characters rewrapped (a scratch script, words unchanged).
- 2026-11-04 02:58: checks run (Result); build folder deleted. Status done.

### Found outside scope

1. Test fixtures in other files still call their plan values "Haik's values", some citing `CLAUDE.md` "Test laptop":
   `fn haiks` or such comments in `wakeshot-windows` `hooks.rs` (10800, 14400, 20), `app/startup.rs`,
   `app/io_thread.rs`, `power/executor.rs`, `power/plan.rs`, `recovery.rs`, `app/dispatch.rs`,
   `examples/power_check.rs`; `wakeshot-cli` `restore.rs`; `wakeshot-core` `ownership/apply.rs`, `check.rs`,
   `fake.rs`, `restore.rs`, `scenarios/lid.rs`, `policy/lid.rs`. Which of them hold old numbers: not checked one by
   one. A code card can rename them the way item 3 did.
2. `docs/architecture.md` lines 387 and 406 (section 3, the AC drop and floor lines) are over 120 characters; left,
   since round 1 kept those section 3 lines for their owner. `docs/research/claude-code-signals.md` has six lines over
   120 characters (78, 331, 393, 487, 491, 496); left, older text of other cards.
3. Round 1 Found outside scope 4 (`claude-code-signals.md` rule 5 lacks the resume and reminder texts) is still open;
   not on this round's list.

### Result

Done 2026-11-04 02:58. Every item was checked in the code or the source card's Result first.

| # | Outcome |
|---|---|
| 1 | Done: section 5 "Plan switch" (both "No apply" points tested) and the many-calls bullet (50 cycles; flicker, other apps' callbacks and AC not verified); section 9's inactive plan line names the real test; section 4 gets the `Global\` create at Medium and the journal lock across High and Medium (tested with probe names, not the Rust exes) |
| 2 | Done: section 6 hook paragraph in the built order (registry first, restore, then the uninstall's data folder step; tries every 2 s up to 25 s or 10 s; which answers end the tries); section 5 step 7: `hooks.rs` built and tested with fakes, `velo.rs` does not call it yet; section 8 "Restore fails" row; section 4 restore row no longer names the hooks |
| 3 | Done: `sim.rs` `plan` doc describes test values, not Haik's; no value changed |
| 4 | Done: rule 2 cites `research.md` "Reading elevated Claude Code sessions from an unelevated process" |
| 5 | Done: `command.rs` `Outcome` doc and `core/update.rs` `done_update` doc: after `Update(Err)` the core waits for the next `UpdateReady`, which comes once per version per run, so the failed version waits for a newer one or a tray restart |
| 6 | Done for these files: section 4 step 6 unread journal, section 10 releases, section 2 sound rule; `WriteConfig` lines already in. Rest listed below |

Files changed: `docs/architecture.md` (sections 2, 4, 5, 6, 7 rewrap only, 8, 9, 10),
`docs/research/claude-code-signals.md` (rule 2 cite), `crates/wakeshot-core/src/sim.rs`, `command.rs`,
`core/update.rs` (doc comments only), this card.

Checks:

1. Each item changed, with the code read first (table, Log).
2. `cargo fmt --all --check`: exit 0, no diff. `rustfmt --check --edition 2024` on `sim.rs`, `command.rs`,
   `core/update.rs`: exit 0.
3. `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings`: exit 0
   (own build folder, deleted after).
4. Every changed file: no CR byte, no em dash, en dash or emoji; no line of mine over 120 characters outside tables
   (scratch script).
5. Not touched: phase files, `docs/plan.md`, other cards' files, `research.md`, any code outside the three doc
   comments. No git.

Decided by the worker, for Haik's review:

1. Section 4's process table no longer lists the Velopack hooks as starters of `--restore`: `hooks.rs` calls
   `restore_locked` in the hook's own process. Change it when a hook starts restore mode instead.
2. Item 6 fixes went beyond lines a card named (section 10 releases, section 2 sound rule), under the audit rule: stale
   docs found in a docs pass are fixed in the same pass. Change it when the orchestrator wants such lines listed only.

For the orchestrator (phase files and `docs/plan.md`, not edited):

1. `docs/phases/03-windows-power.md` line 219: the architecture lines it names (652, 656 and 1017) are now fixed by
   item 1; they hold no sleep values (round 2 Found outside scope 2), so the line can close.
2. `docs/phases/06-ship.md` "Not in this track": could point at `packaging/winget/README.md` for the Phase 8 steps
   (`p6-winget-manifest` Result). The "Hooks" 10 s, the "Release" pre-release and winget rules and the `05-tray-ux.md`
   track note 5 fixes are already in.

For code cards (not done):

1. `velo.rs` (`p6-velopack-dependency`): the four hook callbacks (`p6-hooks` "Needs another card"), `GithubSource`
   with `prerelease: false` (`p6-pack-and-release`). Optional: `log.rs` `LOG_FILE` public; a `restore_locked` form
   with the caller's lock wait (`p6-hooks`).
2. `tray/icon.rs`: one shared `taskbar_dpi` (`p5-notices` Found outside scope 2).
3. `core/lid.rs`: treat an unread journal as "may be ours" before the first read (`p2-6-unread-journal` Found
   outside scope 3); its item 5 (retry with an empty copy at the 30 s tick) was left as it is by that card.
4. `wakeshot-cli/src/run.rs` `held_while_running`: child lifetime on a slow runner (`p4-cli-restore` CI fix, not seen
   failing).
5. Test fixture names, Found outside scope 1.

Open items: Found outside scope 1 to 3; the lists above.

## Round 4 (2026-11-04)

Status: done. Written by the orchestrator (session `wakeshot-12`). Runs after `p2-6-unread-floor-countdown` (it
edits `core/lid.rs`, `scenarios/lid.rs`, `state.rs` and `docs/architecture.md`).

Items (from the orchestrator):

1. `docs/architecture.md` line 751 (section 5, restore step 7): "`velo.rs` does not call it yet" is stale:
   `p6-velopack-dependency` wired the hooks, and `p6-install-e2e` and `p6-int-update-e2e` ran them for real.
2. `docs/product.md` line 208 (Distribution): "Whether Velopack accepts a dot in the pack id: not verified" is stale:
   `architecture.md` section 6 and research "Velopack checks for Wakeshot" point 1 record it as tested.
3. `docs/tasks/p6-readme.md`: its deliverable cites `docs/phases/04-sources.md` "note 16" for the rival port line; it
   is note 17 (that card's Found outside scope 1). Fix the citation only.
4. Round 3 Found outside scope 1: rename the test fixtures that call their plan values "Haik's values" (names and
   comments only, no value change; describe them as test values, the way round 3 item 3 did `sim.rs`). Keep every
   test passing.
5. Round 3 Found outside scope 3: `claude-code-signals.md` rule 5 lacks the resume and reminder texts, if the
   research has them; if not, say so.
6. Docs pass items in the Results and "Found outside scope" lists of the cards finished since round 3:
   `p6-int-wiring-fixes`, `p6-int-log-wording`, `p2-6-unread-lid-countdown`, `p2-6-unread-floor-countdown`,
   `p6-int-dry-run-e2e`, `p6-int-real-e2e`, `p6-int-update-e2e`, the `p6-install-e2e` rerun, `p6-readme` (its Found
   outside scope 2 to 6: only doc lines that state what the code or a test shows; anything that needs Haik or a new
   test goes to the list for the orchestrator).
7. Not to change: Phase 1 Question 11's "30 s" (a record of Haik's answer; the README and `architecture.md` give the
   hook's inner limit, 25 s); records of past runs (work logs, card Logs, dated research results); `CLAUDE.md` (its
   test laptop line already says 5 h, line 48).

### Log

- 2026-11-04 05:59: round 4 started, Status in progress.
- 2026-11-04 06:02: item 1 done (section 5 step 7: `velo.rs` `startup` calls `hooks::on_install`, `on_obsolete`,
  `on_updated`, `on_uninstall`; real runs cited by research heading). Item 2 done (`product.md` Distribution, the
  dotted pack id tested, research "Velopack checks for Wakeshot" point 1). Item 3 done (`p6-readme.md` cites note 17;
  `04-sources.md` note 17 is "No rival code is ported"). Item 4: `haiks` renamed `user_values` in 11 files (it sits
  beside the existing `user_value(key)` helpers); comments that called plan values Haik's now say test or user values,
  also `sim.rs` `world_plan`'s "Balanced: Haik's own", which round 3 missed. `power/plan.rs` reformatted by rustfmt
  (one call grew past the width); `cargo fmt --all --check` exit 0. Values and logic unchanged.
- 2026-11-04 06:07: item 5 already in: rule 5 in the rules table holds the resume text (at `procStart`, as
  `claude/mod.rs` builds `since` from `proc_start`) and the reminder line; the "Rule changes" closing note now says
  the rule uses `procStart`, where the proposal said `startedAt`. Item 6, the nine cards read: in
  `architecture.md` section 3 "Lid closes" the unread journal lock guess (`core/lid.rs` `lock_view`,
  `journal_unknown`); section 9's lock property names its exception (`scenarios/props.rs`
  `no_lock_on_the_users_own_do_nothing`); section 5 one hibernate thread at a time and its post retry
  (`dispatch.rs`, `tray_mode.rs` `post_end`); section 5 notices: refused ones shown again at the icon add
  (`notify.rs` `icon_added`, `tray_mode.rs` `show_again`); section 4 end of session and section 5 guard: what the
  real e2e tested; section 6 hooks: what the update e2e tested; section 9 Integration: the four suites as built.
  Already done by the cards themselves: the countdown waits (section 3), `state.rs` lines, `open_log.rs`. Long lines
  rewrapped with a scratch script (words unchanged).
- 2026-11-04 06:11: item 6, two more lines from the same cards: section 4's failed post line (1400 after a quit is
  Info on the IO and watch threads, `p6-int-log-wording`); section 3's retry paragraph (with an empty mirror the
  retry comes at the 30 s safety tick: `core/settings.rs` `plan_read_due`, `p2-6-unread-floor-countdown` Result 4).
  Decided by the worker, for Haik's review: (1) the fixture name is `user_values`, next to the existing
  `user_value(key)` helpers, and the comments say "the test's user values" or "user values"; change it when a
  fixture reads real values (then a name like `stop_points.rs` `users_values`). (2) Section 3 states
  `p6-int-wiring-fixes` decision 7 as built and marks it open for Haik, rather than leaving the lock rule unstated
  until he decides; change it when he rules on it. (3) Item 7: the phase file lines that quote the old journal text
  are work log records, so they stay.
- 2026-11-04 06:11: checks run (Result); build folder deleted. Status done.

### Found outside scope

For Haik (decisions, not doc lines):

1. `p6-int-wiring-fixes` decision 7: while the journal is unknown, the lid-close lock and chime also come over a
   user's own Do nothing. Tied to it: a journal that stays unreadable keeps both waited countdowns waiting
   (`p2-6-unread-floor-countdown` Found outside scope 2); the `may_own()` condition on the first-read wait, which a
   surviving mutant (M10) shows no test pins (its Found outside scope 3). Section 3 now states decision 7 as built
   and marks it open for Haik.

Needs a test (not a doc change):

2. Start at login after Setup.exe over the install (`p6-readme` Found outside scope 6): `hooks.rs` `on_install` calls
   `set_run` every time, and `winget upgrade` runs Setup.exe (section 6). Whether the user's off switch in Settings >
   Apps > Startup keeps it off is not verified (`06-ship.md` line 43). Test: switch it off, run Setup.exe over the
   install, read the Run value and `HKCU\...\Explorer\StartupApproved\Run` (most likely where that switch is kept;
   not verified), sign out and in.
3. SmartScreen's text for an unsigned `Setup.exe` (`p6-readme` Found outside scope 5): a capture at the first public
   pre-release.
4. With the session unlocked: the "Wakeshot updated to (version)" notice after an update (`p6-int-update-e2e` Found
   outside scope 1); the `UserChanged` notice at an unlock (`p6-int-real-e2e` Open items); the dry-run pictures
   (`p6-int-dry-run-e2e` "Steps left"), whose clean-up also takes the dry-run icon keys `p6-int-real-e2e` left (its
   Found outside scope 1).

For the orchestrator (files this round may not edit):

5. `p6-int-tray-mode.md` "Notes for the Part B run" picks the icon entry by `IconGuid`, which the entry may lack;
   compare ids before and after instead (`p6-int-wiring-fixes` Found outside scope 6). The old journal text and log
   problems that card's Found outside scope 4 and `p6-int-log-wording` Found outside scope 2 name in
   `docs/phases/06-integration.md` (lines 326 and 365) are work log records, so they stay (item 7).
6. `docs/research/claude-code-signals.md` "Rule changes" keeps the proposal's `startedAt` (a dated record); its
   closing note now says the rule as built uses `procStart`.

Minor, left:

7. Lines over 120 characters that are not mine: `scenarios/lid.rs` 868, `power/executor.rs` 126,
   `examples/power_check.rs` 3332, `wakeshot-cli/src/restore.rs` 277; `docs/architecture.md` 396 and 415 (round 3
   Found outside scope 2); `docs/product.md` 24.
8. The release tray exe grew from 2.0 to 3.95 MB with `p6-int-update-wiring` (`url`, `idna`, `icu_*` now built;
   `p6-install-e2e` rerun Found outside scope 2). If size matters, the `velo.rs` owner can check which Velopack
   feature pulls them.

### Result

Done 2026-11-04 06:11. Every item was checked in the code or the source card first.

| # | Outcome |
|---|---|
| 1 | Done: section 5 step 7, `velo.rs` calls `hooks.rs` from Velopack's hook callbacks; real installs, updates and uninstalls ran it (research "Install end to end checks", "Integration update e2e") |
| 2 | Done: `product.md` Distribution, the dotted pack id is tested (research "Velopack checks for Wakeshot", point 1) |
| 3 | Done: `p6-readme.md` cites `04-sources.md` note 17 |
| 4 | Done: `haiks` renamed `user_values` in 11 files; every comment that called plan values Haik's now says test or user values (22 comments, also `sim.rs` `world_plan`); no value or logic changed; tests pass |
| 5 | Already in: rule 5 holds the resume text (at `procStart`, as built) and the reminder line; the "Rule changes" note now names `procStart` |
| 6 | Done: section 3 the unread journal lock guess and the empty-mirror retry at 30 s; section 4 failed posts (1400 is Info) and the tested end of session; section 5 the tested guard, one hibernate thread at a time and its post retry, refused notices shown again; section 6 what the update e2e tested; section 9 the lock property's exception and the four e2e suites. Rest listed under Found outside scope |
| 7 | Kept: Phase 1 Question 11's 30 s, work logs and dated research, `CLAUDE.md` |

Files changed: `docs/architecture.md` (sections 3, 4, 5, 6, 9), `docs/product.md` (Distribution, item 2),
`docs/research/claude-code-signals.md` ("Rule changes" closing note), `docs/tasks/p6-readme.md` (the citation),
this card. Rust, names and comments only: `wakeshot-core` `ownership/apply.rs`, `check.rs`, `fake.rs`, `restore.rs`,
`policy/lid.rs`, `sim.rs`, `scenarios/lid.rs`; `wakeshot-windows` `power/executor.rs`, `power/plan.rs` (also
rustfmt), `recovery.rs`, `hooks.rs`, `app/io_thread.rs`, `app/startup.rs`, `app/dispatch.rs`,
`examples/power_check.rs`; `wakeshot-cli` `restore.rs`.

Checks (build folder `<scratchpad>\p2-6-docs-pass-r4\target`, no `RUSTFLAGS`, all `--locked`):

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (after rustfmt on `power/plan.rs`, where one renamed call grew past the width) |
| `cargo clippy --workspace --locked --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | exit 0 |
| The same without `--all-targets` | exit 0 |
| ARM64 clippy, `-p wakeshot-core -p wakeshot-windows -p wakeshot-cli`, with and without `--all-targets` | exit 0 both; tray ARM64: CI only (clang) |
| `cargo test --workspace --locked`, once at the end | exit 0, nothing failed (99 s) |
| Changed files: no CR byte, em dash, en dash or emoji; final newline | pass (scratch script, 21 files) |
| No line of mine over 120 characters outside tables | pass; older long lines listed under Found outside scope 7 |
| Not touched: `docs/phases/`, `docs/plan.md`, `CLAUDE.md`, `README.md`, other cards but `p6-readme.md`'s citation | pass, from my edit record (no git) |
| Build folder and scratch scripts deleted | yes, checked gone 06:11 |

Decided by the worker, for Haik's review: Log 06:11, items 1 to 3.

Open items: Found outside scope 1 (Haik), 2 to 4 (tests), 5 (orchestrator), 7 and 8 (minor).

## Round 5 (2026-11-04)

Status: done. Written by the orchestrator (session `wakeshot-12`). Runs beside `p2-6-sleep-route`, which owns all
code, `docs/architecture.md` and the card files it names; this round owns none of them.

Items (from the orchestrator):

1. Branding (Haik's yes, 2026-11-04; `docs/brand.md`, commit f47391d): the Sunrise replaced the Cup. Bring
   `docs/research.md` "Tray UX: icon art" up to date (concepts and pick, the app icon colours `e06a22` to `c2501a`,
   the "transparent corner and the disc's orange" line) without erasing what was measured then: say what changed and
   point to `docs/brand.md`. Add a dated note under the `p5-icon-art` Result that the pick changed (do not rewrite the
   Result itself).
2. `README.md` draft: only the lines that describe the icon art (cup, steam, orange) or the brand; keep its media
   spots and everything about the lid and sleep (that waits for `p2-6-sleep-route`). List the lines you changed.
3. `docs/product.md` "User settings safety" 7: say the 2 minutes are run time, and that after a reset battery sleep
   is Windows' default for the plan (180 s on this laptop, `research.md` "Windows defaults of the three keys").
4. Docs items in the Results and "Found outside scope" lists of `p2-6-record-lost` and `p3-idle-sleep-check` that are
   not code and not `docs/architecture.md` (that file is `p2-6-sleep-route`'s this round): fix them or list them.
5. Not to change: work logs, card Logs, dated research results, `CLAUDE.md`, any code.

### Log

- 2026-11-04: round 5 written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 11:40: round 5 started, Status in progress.
- 2026-11-04 11:50: item 1 done. Checked first: `art.rs` (`PICK = Concept::Sunrise`, four concepts, `APP_LIGHT`
  7957f4, `APP_DARK` 6b49f3, `SUN_GOLD` ffcc4d, mark 72%); `assets/wakeshot.ico` read with a stdlib Python ICO reader
  (seven frames, transparent corners, violet plate, gold and white, no orange); sheet sizes from the PNG headers; the
  new contrast numbers recomputed with a Python WCAG script (match `brand.md`). Research "Tray UX: icon art" got a
  dated "Changed later" paragraph and a "Now" line in four points; the old measurements stay. `p5-icon-art.md` got a
  dated note after its Result.
- 2026-11-04 11:52: orchestrator dropped item 2 (a README worker owns all of `README.md`). I had not edited it; lines
  it may want are under Found outside scope.
- 2026-11-04 11:55: item 3 done (`product.md` rule 7: run time, the 180 s battery default, and "with nothing holding"
  on the 7 minutes, from `p2-6-record-lost` Found outside scope 5). Item 4: the two cards read; nothing else of theirs
  is a doc line this round may edit; listed below.
- 2026-11-04 11:58: checks run (Result); scratch scripts deleted. Status done.

### Found outside scope

For the README worker (item 2, dropped):

1. `README.md` has no line on a cup, steam or orange. Lines 3 and 57 to 59 (media comments) say the art comes "from
   `p5-haik-check`"; the art is decided now (`tray/art.rs`, `docs/brand.md`). Line 3's alt text ("wakes while an agent
   works and rests when it is done") could follow `brand.md` "The idea" (the sun rises and sets).

For `p2-6-sleep-route` or the orchestrator (`docs/architecture.md`, code):

2. `p2-6-record-lost` Found outside scope 3: `architecture.md` section 5 "Restore routine" step 1 and the section 8
   row "Journal corrupt" still say a corrupt journal's values stay; the tray now resets them at its next start.
3. `p2-6-record-lost` Found outside scope 2: `examples/power_check.rs` `Timed<T>` does not forward
   `PlanIo::read_default` (code; matters once a harness flow sends `ResetLost`).

For Haik (decisions, not doc lines):

4. `p2-6-record-lost` Found outside scope 4: the uninstall hook sets a corrupt journal aside and no tray starts after
   it, so its values are not reset. Product rule 5 lists uninstall as a restore point.
5. `p2-6-record-lost` Open items: a hold that ran through the 2 minutes with the lid closed runs unlocked until it
   ends, longer than rule 7's "about 7 minutes" (now marked "with nothing holding").
6. `p3-idle-sleep-check` Open: the proposed 15-minute follow-up run (display already off at the release; a display
   timeout longer than the sleep timeout).

For the orchestrator (lines this round may not edit):

7. `docs/product.md` lines 126 and 127 (lid section, not rule 7): "after a release its apps most likely run until the user's
   own sleep timeout (inferred; not tested with the lid closed)". `p3-idle-sleep-check` tested a release with the lid
   closed on battery: the sleep came at the display timeout, not the sleep timeout. Same for `docs/research.md` line
   722 to 724, a dated 2026-11-03 result: left as it is; a pointer to "Idle sleep with the lid closed" would do.

Minor, left: `docs/product.md` line 204 is over 120 characters (not mine; round 4 listed line 24 only).

### Result

Done 2026-11-04 11:58. Every new line was checked in the code, the files or a script first.

| # | Outcome |
|---|---|
| 1 | Done: research "Tray UX: icon art" has a dated "Changed later" paragraph and "Now" lines on contrast, stroke, concepts and pick, and the ICO and PNG files; old numbers kept. `p5-icon-art.md` has a dated note after its Result |
| 2 | Dropped by the orchestrator (README worker); `README.md` not edited. Lines for that worker: Found outside scope 1 |
| 3 | Done: rule 7 says the 2 minutes are run time, the battery default is 180 s on the test laptop (3 minutes for a user's own Never), and the 7 minutes assume nothing holds |
| 4 | Nothing more to edit in my files; Found outside scope 2 to 7 |
| 5 | Kept: work logs, card Logs, dated research results, `CLAUDE.md`, code, `architecture.md`, `site/` |

Files changed: `docs/research.md` ("Tray UX: icon art" only), `docs/tasks/p5-icon-art.md` (a dated note),
`docs/product.md` (rule 7 only), this card.

Checks:

| Check | Outcome |
|---|---|
| `assets/wakeshot.ico` matches the new art (stdlib Python ICO reader) | pass: 7 frames, 43,006 bytes, transparent corners, 7957f4 and 6b49f3, gold and white, no orange |
| New contrast numbers (Python WCAG 2) | match `brand.md`; the dark end on 202020 is 3.004 |
| Changed files: no CR byte, em dash, en dash or emoji; final newline | pass (scratch script, 4 files) |
| No line of mine over 120 characters | pass; `product.md` 24 and 204 and older research lines are not mine |
| Not touched: code, `architecture.md`, `site/`, `README.md`, `CLAUDE.md`, phase files, work logs | pass, from my edit record (no git) |
| Scratch scripts deleted | yes |

No cargo run: this round changed no code.

Open items: Found outside scope 1 (README worker), 2 and 3 (`p2-6-sleep-route` or a code card), 4 to 6 (Haik), 7
(orchestrator).

## Round 6 (2026-11-04)

Status: done. Written by the orchestrator (session `wakeshot-12`). Runs after `p2-6-sleep-route-fixes`
(done, local commit 9aa92f9). Owns: `README.md`, `site/index.html` (only its sleep wording spot),
`crates/wakeshot-core/src/text.rs` (two notices), `docs/research.md` (the two lines below), and this section.

Items (from the orchestrator; sources: `p2-6-sleep-route-fixes` Found outside scope 1, 3, 4, 7 and
`p2-6-sleep-route` Found outside scope):

1. `README.md`: the old notice title "Check your lid and sleep settings" (now "Check your power settings"); the reset
   list (now also the 1-minute sleep and screen timeouts); uninstall now resets a damaged journal and keeps `logs\`;
   the two `sleep-route:` comments: check those lines against the code now and remove the comments; the crash gap
   text may leave out the 1-minute timeouts (`docs/product.md` crash gap paragraph now names them).
2. `site/index.html`: the spot marked for the sleep wording after `p2-6-sleep-route`: check it against `product.md`
   and the code, fill it, remove the mark.
3. `text.rs`: two notices wrap to five body lines on the taskbar (one more than shows): `HibernateFailed` and the
   closed-lid report with many holders. Use the same wrap model as `p2-6-sleep-route-fixes` deliverable 5 (its Result)
   and make both fit four lines, keeping the meaning and `text.rs` rules (a suggested `HibernateFailed` is in that
   card's Found outside scope 3). `cargo test --locked -p wakeshot-core` and clippy on the core after the change.
4. `docs/research.md`: "Tray UX: notices" says "at most five body lines"; the pictures show a title and four body
   lines. Add Balanced's Windows defaults of the short keys (AC sleep 300, screen off AC 300 and battery 180 by the
   API; registry table 1800, 600, 300; read 2026-11-04 by `p2-6-sleep-route-fixes`) to "Windows defaults of the three
   keys" as a dated addition.
5. Not to change: `docs/product.md` (the orchestrator updated rules 5 and 7), work logs, card Logs, dated results,
   `CLAUDE.md`, other code.

### Log

- 2026-11-04: round 6 written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 13:50: round 6 started, Status in progress.
- 2026-11-04 about 13:55: item 3 measured. The fixes card's scripts were gone, so I rebuilt its model in a scratch
  PowerShell script: GDI `TextRenderer` widths of Segoe UI at 18 px and 21 px, greedy wrap, width taken from the 13
  `p5-notices` pictures (41 body lines read off them): widest line shown whole 343 px, narrowest line that wrapped
  344 px at 18 px (399 and 404 at 21 px), so every picture's wraps come back. It agrees with the fixes card: its five
  bodies take four lines, `HibernateFailed` and the many-holds report five. `text.rs` changed (Result); new test.
- 2026-11-04 about 13:59: core tests: one run had one property failure outside my files (Found outside scope 1), the
  rerun and five runs of that test at 4096 cases passed; clippy on the core clean.
- 2026-11-04 about 14:02: items 1 and 2 done after reading `core/lid.rs` (`end_short` callers, `take_unlocked`,
  `end_lock_step_back`), `settings.rs` (`HOLD`, `SHORT`), `hooks.rs` (`on_uninstall`, `reset_lost`, the uninstall test
  that leaves only `logs`), `power/hibernate.rs` (`run_sleep_check`: the lid once more after the sound).
- 2026-11-04 14:05: item 4 done. The short keys' defaults read again myself, read only (research entry says how);
  same values as the fixes card. `powercfg /qh` and `/list` the same before and after.
- 2026-11-04 14:06: checks run (Result); build folder and scratch scripts deleted, checked gone. Status done.

### Found outside scope

1. **A core property failed once** (`crates/wakeshot-core/src/scenarios/props.rs`, not mine, nothing of mine feeds it):
   `scenarios::props::a_released_key_is_never_ours_in_force_again_in_its_episode`, at the default 256 cases, in my
   first `cargo test -p wakeshot-core` run: "SleepDc was released in this episode, yet ours is in force on
   381b4222-..., at run 300000 ms". Not seen again: one rerun at 256 and five runs at 4096 cases passed. The shrunk
   input was lost (my output filter dropped it, and these props save no seed: `failure_persistence: None`). SleepDc is
   both a hold key and a short key, so the sleep route fixes are the likely area (inferred, not checked). For a core
   worker: run it in a loop with full output to catch the case.
2. **`docs/product.md` lines 128 and 164** are over 120 characters (the orchestrator's edits to the work end rule and
   rule 7). Not mine this round.
3. **`docs/tasks/p5-haik-check.md`** lists notice words for Haik's read (around lines 164 and 183); it does not have the
   new `HibernateFailed` words or the shorter many-holds report yet.
4. **Not checked in code:** the "Restore my power settings" menu line in `README.md` and the `RestoredByUser` notice
   say "lid and sleep settings"; after a closed-lid work end the restore may also put back the screen timeouts.
5. **`docs/research.md` heading "Windows defaults of the three keys"** now covers six keys. Left as it is, since
   `product.md` rule 7 names the heading.

### Result

Done 2026-11-04 14:06. Each line was checked in the code or by a read first.

| # | Outcome |
|---|---|
| 1 | `README.md`: notice table title now "Check your power settings"; Limits reset list adds the 1-minute screen and sleep timeouts ("battery sleep Never", "1 minute yourself") and the uninstall reset; crash gap paragraph adds the 1-minute timeouts (as `product.md` says the README does); Uninstall step 2 says "if Wakeshot had changed them" and the damaged-file reset, and "What stays" has `logs\` after that reset. Both `sleep-route:` comments checked and removed; fixed on the way: "unlock after a wake" is now any unlock (code: `take_unlocked`), "not when the laptop only wakes", "During a hold, sleep while plugged in is never touched", and the failed-lock bullet says the hold comes back once the lid opens or the PC locks (`end_lock_step_back`). Steps 3 and 5, the floor bullet and the rest of the notice table matched the code |
| 2 | `site/index.html`: steps 3 and 5 already said "goes to sleep" and match `product.md` and the code; the mark removed, words kept |
| 3 | `text.rs`: both notices fit four lines in the wrap model (table below); new test `hibernate_failed_and_the_closed_lid_report_fit_four_lines` |
| 4 | `docs/research.md`: "Text fit" corrected to four body lines, with a dated note; a new "wrap model" point says how it was built; "Windows defaults of the three keys" has a dated "short keys" point, read again by me |
| 5 | Not touched: `product.md`, work logs, other card Logs, dated results, `CLAUDE.md`, other code |

Notice words (titles unchanged):

| Notice | Old body | New body |
|---|---|---|
| `HibernateFailed` ("The PC did not go to sleep") | The lid is closed, but the PC did not sleep and could not hibernate. Windows should sleep it once nothing else keeps it awake. Opening the lid puts your screen and sleep settings back. (5 lines) | It could not hibernate either. Windows should sleep it once nothing else keeps it awake. Opening the lid puts your screen and sleep settings back. (4 lines) |
| Closed-lid report, 3 days 5 h, nine holds | The PC kept running for 3 days 5 h with the lid closed. What held it: nightly backup 1, nightly backup 2, nightly backup 3, nightly backup 4, nightly backup 5, nightly backup 6 and 3 more. (5 lines) | The PC kept running for 3 days 5 h with the lid closed. What held it: nightly backup 1, nightly backup 2, nightly backup 3 and 6 more. (4 lines) |

The report's list is now fitted to a 150-unit body (`BODY_SHOWN_UTF16`, which replaces the 200-unit
`BODY_ENGLISH_UTF16`); a one- or two-hold report reads as before.

Decided by the worker, for Haik's review:

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | `HibernateFailed` drops "The lid is closed" and says "could not hibernate either" | "Opening the lid" already says the lid is closed; every form with the lid sentence took five lines; "could not" fits the not allowed and failed cases | Haik's screen check shows room, or he prefers other words |
| 2 | The report keeps its unit budget, now 150, not a cap on the number of names | Same stand-in as the fixes card; a 60-character reason is still named | The screen check shows a cut (a name in capitals can take a fifth line), or Haik wants at most N names |

Files changed: `README.md`, `site/index.html` (one comment removed), `crates/wakeshot-core/src/text.rs`,
`docs/research.md` ("Tray UX: notices" and "Windows defaults of the three keys"), this card's round 6 section.

Checks (build folder `<scratchpad>\docs-r6\target`, debug, no `RUSTFLAGS`, `--locked`):

| Check | Outcome |
|---|---|
| `rustfmt --check --edition 2024` on `text.rs` | exit 0 |
| `cargo test --locked -p wakeshot-core` | first run: 660 passed, 1 failed (Found outside scope 1, not mine); second run exit 0: 661 passed, 1 ignored, then 11 and 25 |
| `cargo clippy -p wakeshot-core --target x86_64-pc-windows-msvc -- -D warnings`, with and without `--all-targets` | both exit 0 |
| The new test fails on the old words | by count only: both old bodies are over 150 units (not run on the old code) |
| Wrap model reproduces the pictures | yes: 343 px fits every line shown whole, 344 px is the narrowest wrap (18 px font); 21 px agrees |
| Changed files: em or en dash, emoji, CR byte, final newline | pass (scratch script, 5 files) |
| Lines over 120 characters | none of mine; older README table and comment lines stay |
| Haik's plan | only read; `powercfg /qh` and `/list` the same before and after |
| Build folder and scratch scripts deleted | yes, checked gone |

Open items: Found outside scope 1 (core worker), 2 to 5 (orchestrator). Haik reads the two new bodies at
`p5-haik-check`; neither was shown on the real taskbar yet.
