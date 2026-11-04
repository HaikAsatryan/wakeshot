# Fixes after the sleep route review; shorter notices; the reset covers the short timeouts and uninstall

Phase: 2 to 6 (core and power). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens.
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

1. **`README.md`** (not mine) is now wrong in three places: line 305 names the old title "Check your lid and sleep
   settings" (now "Check your power settings"); lines 401 to 406 say the reset takes "lid Do nothing, sleep Never"
   (now also the 1-minute sleep and screen timeouts); the uninstall section does not say that a damaged journal is
   now reset at uninstall, with `logs\` kept.
2. **`docs/product.md`** (orchestrator): rule 7 names only "lid Do nothing, sleep Never"; the orchestrator's decision
   for major 2 adds the 1-minute timeouts (for Haik's review). Rule 5 and the crash-gap paragraph: an uninstall with a
   damaged journal now resets instead of leaving the values.
3. **Two more notices would be cut on the taskbar** (`text.rs`, not among the card's four): measured with the same wrap
   model as deliverable 5, `HibernateFailed` (words written by `p2-6-sleep-route` after the screen run, 183
   characters) and the closed-lid report with many holders (its list is sized to 200 units) each wrap to five body
   lines, one more than the banner shows. For `p5-haik-check` or a follow-up; a fitting `HibernateFailed`, for
   example: "It did not hibernate either. Windows should sleep it once nothing keeps it awake. Opening the lid puts your
   screen and sleep settings back." (138, four lines).
4. **`docs/research.md` "Tray UX: notices"** says "at most five body lines"; the pictures show the title and four body
   lines, so a body gets four lines under a one-line title.
5. **A hold that starts right after a damaged file went aside, with every hold key still at 0** (pre-existing,
   `p2-6-record-lost` decision 3): its apply skips all three as the user's own and writes no journal, so the reset
   (`damaged`, no file) puts them to the defaults while the hold holds, and nothing applies again: that hold runs with
   no Do nothing, and a lid close sleeps the laptop. Safe, but the hold is lost for the episode. Lean fix: after a lost
   reset that reset a key while the core applies, apply again.
6. **`CLAUDE.md`** fact "Agents here run elevated (High, `SeDebugPrivilege` enabled)": the Bash tool's shell also has
   `SeBackupPrivilege` and `SeRestorePrivilege` enabled (MSYS2), PowerShell's has not (research "A deny ACE and the
   backup privilege"). A test that expects a denial passes for the wrong reason, or fails, depending on the tool.
7. **Balanced's Windows defaults of the short keys** (read about 13:15, read only): AC sleep 300, screen off AC 300
   and battery 180 by the API (registry table 1800, 600, 300). They belong in research "Windows defaults of the three
   keys"; my `research.md` scope was deliverable 6 only, so they are in this card's Log.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 12:40: worker started; read the card, the review, both earlier cards' Results and decisions, product
  rules.
- 2026-11-04 12:55: read the code (`core/lid.rs`, `core/settings.rs`, `core.rs`, `state.rs`, `ownership/reset.rs`,
  `restore.rs`, `hooks.rs`, `power/executor.rs`, `power/journal.rs`, `text.rs`, `sim.rs` World, lid scenarios).
  Deliverable 6 cause found: the Bash tool's shell (Git Bash, MSYS2 runtime) runs with `SeBackupPrivilege` and
  `SeRestorePrivilege` **enabled** (`whoami /priv` there), PowerShell's with them disabled; the same test binary fails
  from Bash (`left: Ok([...])`) and passes from PowerShell. A directory opened with backup semantics while the backup
  privilege is enabled lists past the deny ACE.
- 2026-11-04 13:08: core done: window lock kept until lid open, `Locked` or `Unlocked` and settled 30 s (`core/lid.rs`,
  `state.rs`), `check_lock` treats a window hold as ours, a failed reset locks a laptop closed in the window, hold-key
  count (`core/settings.rs`, props model), reset of the short keys at exactly 60 s (`ownership/reset.rs`), case B: a
  damaged reset that finds this tray's own fresh entries restores them, resets again, applies again (`done_reset`).
  Notice words shortened and measured (a Segoe UI wrap model calibrated on the screen-run pictures: every new body
  wraps to four lines, also 6% narrower). Scenarios: 6 lid, 2 World cases, reset routine; props model and the lock
  property now judge a hold after the window. `cargo test -p wakeshot-core --lib`: 658 passed. Next: hook, executor,
  docs.
- 2026-11-04 13:20: Windows side: uninstall hook runs the reset on a journal that does not parse (`hooks.rs`), a
  `.corrupt` that cannot be renamed is not reported (`power/executor.rs`), the listing test drops the backup privilege
  on its thread (`watch/agents.rs`), reset log test (`power/journal.rs`). `cargo test -p wakeshot-windows --lib` from
  the Bash tool: 563 passed; the listing test passes from both tools. Real, read only
  (`real_defaults_of_the_active_plan_read`, now all six keys): Balanced's defaults by the API are lid 1/1, battery
  sleep 180, AC sleep 300, screen off AC 300 and battery 180 (registry table 1800, 600, 300); `powercfg /qh` and
  `/list` identical before and after.
- 2026-11-04 13:30: `docs/architecture.md` sections 3 (window lock, command block, retry line, the reset paragraph), 5
  (restore step 1, "Gap left") and 6 (hooks), the failure table; `docs/research.md` new section "A deny ACE and the
  backup privilege". Slip: I ran one read-only `git show HEAD:docs/architecture.md | head -0` while checking a line; it
  printed nothing and changed nothing, and no other git command ran.
- 2026-11-04 13:10 to 13:40: hand mutants on a scratch copy (one build job, one test thread, shrinking off, 32 property
  cases, a time limit, a stop under 10 GB free). Two runner bugs cost time: a pipe nobody read blocked the child on a
  failing test's long output (fixed: output to a file). M4 (`Locked` does not end the wait) made the World loop on
  lock, `Locked`, lock; the memory guard stopped it at 10.0 GB free, as meant. Fix from it: the window lock also needs
  the session unlocked, so a `Lock` into a locked session is never sent again at each pass. Found in review meanwhile:
  after a window lock failed twice, the step back must not meet a third lock on our Do nothing (guard and test).
- 2026-11-04 13:49: M4 to M13 again, M12 again after its test: 13 of 13 killed. Final checks (Result): fmt, clippy x64
  and ARM64, workspace tests 1355 passed, props at 4096. Scratch and build folders deleted, checked gone. Status done.

## Result

**Done (2026-11-04).** Contract changes: none (no new command, event, outcome or `RestoreWhy`); one trait method on the
hooks' `HookEnv` (`reset_lost`), which only `hooks.rs` implements.

### What was done, by deliverable

1. **Review major 1** (`core/lid.rs`, `state.rs`). The lock a lid close in the unknown window did not get
   (`LockWatch::window_close`) now waits until the lid opens or the session reports `Locked`, not until the window
   ends. `Core::settle_window_lock` locks, with the chime, once the journal is known, Wakeshot keeps the closed laptop
   running (a hold holds, also one that starts after the window or holds again after a step back; or Wakeshot's own Do
   nothing is in force), and a read shows the user's own lid action as not Do nothing. A close while the session is
   locked owes no lock. Scenarios: `a_hold_that_starts_after_the_window_locks_a_laptop_closed_in_it` (main path; the
   lock waits for the apply's journal when the copy still shows the old Do nothing; a lock in between ends it),
   `a_hold_held_back_at_the_windows_end_locks_once_it_holds_again` (second path, AC back), and a third case in
   `once_the_journal_reads_its_entries_decide_the_lock` (Wakeshot's own Do nothing, no hold). The props model knows
   the owed lock, and `closed_laptop_is_locked_in_time` now starts a stretch at any later step that finds the laptop
   running unlocked on our Do nothing, not only at a lid close or unplug.
2. **Review major 2** (orchestrator's decision). Case A: `ownership::reset` takes all six keys; a key resets when it
   holds exactly a value Wakeshot writes (hold keys 0, short keys 60 s; battery sleep either). Case B
   (`Core::done_reset`): a damaged file's reset that finds entries, while this tray owns the settings, found its own
   apply that ran between the set-aside and the reset (it journaled battery sleep with 60 s as the user's); the core
   sends `Restore { plan: None, why: WorkEnded }`, the reset again, the hold's apply again and, if in force, the short
   apply, in that order on the IO queue; once only (`State::reset_rerun`). Tests: reset routine
   `the_sleep_routes_one_minute_is_reset_on_each_short_key_and_no_other_value`; World cases through the real routines
   `one_minute_timeouts_a_crash_left_end_at_the_defaults` (A) and
   `a_damaged_journal_met_as_a_hold_starts_over_the_one_minute_timeouts_ends_at_the_defaults` (B: battery sleep's
   entry ends with the default as its original, and every short key ends at the default).
3. **Review major 3** (`hooks.rs`). On a journal that does not parse, the uninstall hook runs
   `power::journal::reset_locked` (`damaged: true`), logs a summary under the reset's per-key lines, and keeps
   `logs\`. Install and updated leave it to the tray. Tests: the fake hook order and line, and
   `real_uninstall_resets_a_journal_that_does_not_parse` (temp data folder, a corrupt `journal.json`, `FakePlans`:
   lid DC, battery sleep and AC sleep at Wakeshot's values go to the defaults, the user's lid AC stays, the file goes,
   only `logs` is left).
4. **Review minors.** 4: `applies` and `take_back` count only hold keys, and so does the props model (as the review
   says); test `short_keys_the_user_changes_in_a_step_back_never_stop_the_hold`. 5: the leaner of the review's two: a
   failed reset locks once, with the chime, a laptop whose lid closed in the window (`Core::lost_reset_failed`); no
   retry of the reset; test `a_failed_reset_locks_a_laptop_whose_lid_closed_in_the_window_once`. 6: the review's first
   fix: a `.corrupt` that cannot be renamed to `.seen` is logged and not reported (`PowerExecutor::mark_seen`); the
   late report of a file `wakeshot restore` set aside while a tray ran is kept, as nothing else would ever reset the
   values it named; test `startup_does_not_report_a_corrupt_file_it_cannot_mark_seen`. 7: as the review says: the
   window lock waits for the power source to have lasted 30 s, and `check_lock` treats a window lock as ours while a
   hold holds (so a requests-only hold gets the retry and the step back); tests
   `a_window_lock_waits_out_a_short_ac_drop_for_a_docked_user`,
   `a_window_lock_the_session_never_confirms_is_sent_again_then_steps_back`. Found while testing: after that step back
   the window lock must not fire a third time on our Do nothing; guarded and tested
   (`a_window_lock_on_wakeshots_do_nothing_that_fails_twice_steps_back_with_no_third_lock`).
5. **Notice words** (`text.rs`). Measured, not guessed: a greedy wrap of Segoe UI text (GDI `TextRenderer`), its line
   width set between the widest line the screen-run pictures show whole and the narrowest they wrap, reproduces every
   wrap in the pictures; each new body wraps to four lines, also with a 6% narrower line. The test
   `the_notices_the_screen_run_cut_fit_four_lines` holds each form to 150 UTF-16 units. Old and new:

   | Notice | Old | New |
   |---|---|---|
   | Restored by user | Wakeshot put back your lid and sleep settings and leaves them alone until the current work ends. It still keeps the PC awake for the work, but only while the lid is open. | Wakeshot put back your lid and sleep settings and leaves them alone until the work ends. It still keeps the PC awake, but only with the lid open. |
   | Lock failed | Windows did not lock the PC. Lock it with Windows key + L. With the lid closed, Wakeshot tries once more; if that fails, it stops keeping the PC awake and makes it sleep after 5 minutes with no input. | Lock it with Windows key + L. With the lid closed, Wakeshot tries again; if that fails too, it puts the PC to sleep after 5 minutes with no input. |
   | Update Wakeshot | Wakeshot cannot fully read what {agent} is doing, most likely because {agent} was updated. Until you update Wakeshot, it may not keep the PC awake for {agent} as it should. | {agent} was likely updated, so Wakeshot cannot fully read what it is doing. Until you update Wakeshot, it may not keep the PC awake as it should. |
   | Journal lost | Wakeshot could not read its record of your settings, so it set {the lid setting / the battery sleep setting / the lid and battery sleep settings} to {the Windows default / Windows defaults}. If you had your own, set {it / them} again. Click to see the old values in the log. | Wakeshot could not read its record, so it reset your {lid / screen / sleep; joined: "lid and sleep", "screen and sleep", "lid, screen and sleep"} {setting / settings}. If you had your own, set {it / them} again; click to see the old values. |
   | Journal lost, failed (title) | Check your lid and sleep settings | Check your power settings |
   | Journal lost, failed (body) | Wakeshot could not read its record of your settings, and could not set them all back to Windows defaults. Set the lid and sleep settings by hand; click to see the old values in the log. | Wakeshot could not read its record or reset every setting. Set your lid, screen and sleep settings by hand; click to see the old values. |

   The lost-journal groups follow Windows: the two lid keys are one lid setting, sleep on AC and battery one sleep
   setting, screen off on AC and battery one screen setting. Titles are unchanged but the failed one.
6. **The listing test** (`watch/agents.rs`, tests only). Cause: the Bash tool's shell (Git Bash, MSYS2) runs with
   `SeBackupPrivilege` and `SeRestorePrivilege` enabled, PowerShell's does not; with the backup privilege enabled a
   folder opened for listing lists past the deny ACE. The same binary failed from Bash and passed from PowerShell. The
   test thread now impersonates a copy of its token with `SeBackupPrivilege` off (`NoBackup`), so it checks the
   denial in any shell; it passes from both tools. Research: "A deny ACE and the backup privilege".
7. **`docs/architecture.md`**: section 3 (the window lock, the `ResetLost` comment, the retry line's hold keys, the
   lid-close line, the reset paragraph with the short keys and case B), section 5 (restore step 1, "Gap left": the
   short keys, the uninstall reset, the `.seen` rule and the kept late report), section 6 (hooks), and the failure
   table row "Journal corrupt".

### Decided by the worker, for Haik's review

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | The window lock also comes on Wakeshot's own Do nothing with no hold, not only when a hold holds | The lid-close rule locks on our Do nothing; the window lock is that lock, delayed | Never likely |
| 2 | The wait ends at `Locked` or the lid opening; a close while the session is locked owes nothing, and no window lock goes into a locked session | A locked session is the goal; every unlock follows a lock; a `Lock` into a locked session is not watched, so it would repeat at every pass | A log shows a lost `Locked` event |
| 3 | The 30 s settle applies after any source change, AC back too, as `relock` does | One rule; 30 s is harmless | Never likely |
| 4 | A failed reset locks once (with the chime), only for a lid closed in the window; the reset is not retried | Covers every failure cause for the lock; a policy failure would fail again | A log shows a failed reset that a retry would fix: retry it on the 2-minute clock |
| 5 | Case B is answered from the reset's own answer (entries after a damaged set-aside, this tray the owner): restore, reset again, apply again, once | No timing and no new contract; the IO queue's order makes it exact. Its restore uses `RestoreWhy::WorkEnded` | A log shows it where the entries' originals were right (it costs one restore and one apply) |
| 6 | The uninstall reset runs with `damaged: true` (the review said `false`), and keeps `logs\` | A file the restore routine set aside a moment ago is gone at the reset's load, and `false` would reset nothing then; the log holds the old values | Never likely |
| 7 | Minor 6: no report when the `.seen` rename fails; the late report of a CLI set-aside is kept | Repeated resets cost more than a missed one; the late reset is the only thing that takes those values back | A log shows a late reset over the user's own values: skip a `.corrupt` older than the tray's last clean quit |
| 8 | Notice words as listed; "Check your power settings" as the failed title | Fits four lines with room; the title keeps one line (a 41-character title risks wrapping) | Haik's screen check shows a cut, or he prefers other words |
| 9 | The listing test drops the backup privilege on its own thread, not a skip | It then tests the denial in every shell | `ImpersonateSelf` fails on a runner: then skip with a printed reason |
| 10 | The Windows defaults of the short keys are read, not pinned, in the real read-only test | Nothing measured them before; read now: 300, 300, 180 | A second read on another laptop: pin them per laptop |

### Files changed

`crates/wakeshot-core/src/`: `core/lid.rs`, `core/settings.rs`, `state.rs`, `command.rs` (doc only),
`ownership/reset.rs`, `text.rs`, `scenarios/lid.rs`, `scenarios/props.rs`, `scenarios/settings.rs`.
`crates/wakeshot-windows/src/`: `hooks.rs`, `power/executor.rs`, `power/journal.rs` (test), `power/plan.rs` (real
read-only test), `watch/agents.rs` (test). Docs: `docs/architecture.md`, `docs/research.md` (new section for
deliverable 6), this card. No example needed a change; `notify.rs` none (it shows the `text.rs` words).

### Checks (2026-11-04, build folder `<scratchpad>\p2-6-sleep-route-fixes\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (`cargo fmt --all` ran first; nothing else runs in parallel, and the tree was formatted before, so only my files changed) |
| clippy x64 `--workspace --all-targets -D warnings`, and without `--all-targets` | both exit 0 |
| clippy ARM64 `-p wakeshot-core -p wakeshot-windows -p wakeshot-cli`, with and without `--all-targets` | both exit 0; the tray's ARM64: CI only |
| `cargo test --workspace --locked` (debug), from the Bash tool (the shell with the backup privilege on) | exit 0: 1355 passed, 0 failed (cli 75 + 3 + 5, core 660 + 11 + 25, tray 8 + 5, windows 563); the listing test also passed from PowerShell |
| Core scenario props at `PROPTEST_CASES=4096`, once | 234 passed, 1 ignored, 37 s; 36.4 GB free after |
| Hand mutants on the core fixes (scratch copy; one build job, one test thread, shrinking off, 32 property cases, 600 s limit, stop under 10 GB free) | 13 run, 13 killed in the end: M1 to M11 and M13 at once; M4 first looped the World until the guard stopped it at 10.0 GB free (that led to the `!session_locked` guard), then killed; M12 (the once-only guard) survived, then killed by the test added for it. Windows-side lines: no mutants (each has its own failing-before test: the hook, the executor rename, the listing test from Bash) |
| Real, read only: `real_defaults_of_the_active_plan_read` (all six keys now) | passed; Balanced by the API: lid 1/1, battery sleep 180, AC sleep 300, screen off AC 300, battery 180 |
| Haik's plan never written; no tray started | yes: fakes, `DryRunPower` and temp folders only; `powercfg /qh` and `/list` identical before and after the real read |
| Build folder deleted | yes: the build folder, the scratch copy, its build folder and my scripts deleted (checked gone); the ACL probe folder got its access back first; my leftover `tail` processes stopped by PID |

### Open items

- For the orchestrator: Found outside scope 1 to 7 (README and `product.md` lines, two more notices that would be
  cut, a research line, a hold lost after a damaged set-aside, a `CLAUDE.md` fact, the short keys' defaults).
- For Haik (`p5-haik-check`): the new notice words above; the window lock and failed-reset lock behavior (decisions
  1 to 4); the uninstall reset (decision 6).
- Not tested on the laptop: the uninstall reset on a real install, and a real damaged journal in a run.
