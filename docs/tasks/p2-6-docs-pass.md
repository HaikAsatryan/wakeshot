# Docs pass for Phases 2 to 6: make the docs match the built code

Phase: 6. Status: done (rounds 1, 2 and 3). Needs Haik: no.
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
