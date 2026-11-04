# The Floor countdown waits while the journal is unknown; two state.rs doc lines; the empty-copy retry interval

Phase: 2 to 6 (core). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p2-6-unread-lid-countdown` "Found outside scope" 2 and 3 and
`p2-6-unread-journal` "Found outside scope" 5.
Runs after: `p2-6-unread-lid-countdown` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-floor-countdown\target`.

## Goal

Close the same gap for the battery floor's countdown that `p2-6-unread-lid-countdown` closed for the work-end
countdown, fix two stale doc lines in a contract file, and look at how often an unread journal is retried.

## Read first

- `docs/tasks/p2-6-unread-lid-countdown.md`: Result, decisions, "Found outside scope" 2 and 3.
- `docs/tasks/p2-6-unread-journal.md`: "Found outside scope" 5.
- `crates/wakeshot-core/src/core/lid.rs` (`floor_countdown_starts`, `floor_countdown_wanted`, `floor_waits`,
  `work_end_waited`, `journal_unknown`); `crates/wakeshot-core/src/state.rs` (`LockWatch`, `work_end_waits`);
  `crates/wakeshot-core/src/core.rs` (`plan_read_due`).
- `docs/architecture.md` (grep "Floor" and "countdown").

## Deliverables

1. `core/lid.rs`: a step back with the lid closed on a Do nothing in force while the journal is unknown starts no
   `Floor` countdown today, so after the restore the closed laptop runs on battery under the floor until the user's
   battery sleep timeout. Use the same fix shape as the work-end case (wait while the journal is unknown; a want that
   waited takes no Do nothing as final), unless a leaner one exists; say why. Tests first in `scenarios/lid.rs`.
2. `state.rs`: doc lines only, no type or field change: `LockWatch` ("the countdown rules that wait for the first plan
   read") and `work_end_waits` (suggested text in `p2-6-unread-lid-countdown` "Found outside scope" 3), plus the floor
   field if deliverable 1 makes its doc stale.
3. `core.rs`: a retry with an empty copy (an unread journal, or a failed restore that read nothing) runs at the 30 s
   safety tick, not every 10 s, because `plan_read_due` sets a deadline only while the copy holds entries. Decide
   whether the 10 s retry is worth it now that a waited countdown starts only after a read; if yes, the leanest change
   and a test; if no, say why in the Result and change nothing.
4. `docs/architecture.md`: only the lines that state these rules, if one changes.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and
  without `--all-targets`, both targets; `cargo test --locked -p wakeshot-core`; the props at 4096 cases once; then
  `cargo test --workspace --locked` once at the end.
- Mutants only with one job and a low timeout while you watch free memory (a mutant of the last card grew to 35 GB and
  Claude Code then ended other processes for low memory); stop at once if free memory drops under 10 GB. If you cannot
  cap it, skip mutants and say so.
- Build folder deleted at the end.

## Out of scope

- `wakeshot-windows`; `event.rs`; any field or type change in `state.rs` (describe it and stop).
- `p6-int-wiring-fixes` decision 7 (a lock on a user's own Do nothing while the journal is unknown): Haik's call.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. **The card names the wrong file for deliverable 3.** `plan_read_due` lives in `core/settings.rs`, not `core.rs`
   (`core.rs` only calls it in `settle`). Nothing changed there, so no file outside the card was touched.
2. **A journal that stays unreadable keeps every waited countdown waiting (not new).** While each retry fails to load
   `journal.json`, the crashed tray's Do nothing and battery sleep Never stay in force, and no read can show the user's
   own value, so neither a work-end nor (now) a `Floor` countdown starts. A closed laptop then runs, locked by the
   unread guess, until the lid opens, AC comes back (`Floor` only) or Windows' own critical battery action. Before
   this card and `p2-6-unread-lid-countdown` no countdown started either, so nothing got worse. The only way out is a
   guess, which would hibernate a user whose own lid action is Do nothing; that is the same trade-off as
   `p6-int-wiring-fixes` decision 7, Haik's call. Seen only in reasoning, not on the laptop (not verified how long a
   real load failure lasts).
3. **Before the first plan read, a step back with the lid closed and nothing owned never counts down; after it, it
   does.** `floor_countdown_wanted` (and `work_ended_on`) wait for the first read only when `may_own()`: "nothing owned,
   so Wakeshot changed nothing". Once a read is in, the same step back starts the `Floor` countdown on the user's own
   Sleep with nothing owned (the floor rule has no ownership condition). So a step back in the first seconds after the
   tray starts, before its first read, ends differently from the same step back a minute later. Rare and harmless
   either way (Windows' own timers still run). This is also why mutant M10 (drop `may_own()` from the wait) survives:
   no test pins the condition, and I did not add one, because which side is right is a design call (Haik's rule
   favors "no countdown", the floor rule favors "countdown"). Fix when decided: one condition in each of the two
   functions, plus a test.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 05:29: worker started; Status in progress. Reading the two source cards and the code.
- 2026-11-04 05:35: read `core/lid.rs`, `core/settings.rs`, `core.rs` (`settle`, `Started`), `state.rs`, `sim.rs`,
  the floor and unread tests. Note: `plan_read_due` lives in `core/settings.rs`, not `core.rs`. Three tests first in
  `scenarios/lid.rs` (`hibernated_after_idle` now takes the reason): on the original logic the two countdown cases
  fail (no `Hibernate`), the own Do nothing guard passes. Next: the fix in `core/lid.rs`.
- 2026-11-04 05:36: fix in `core/lid.rs`, the work-end shape: `floor_countdown_wanted` (now gives whether it starts)
  also waits while the journal is unknown; new `floor_countdown_waited` keeps a waited want that starts nothing.
  `cargo test -p wakeshot-core`: 602 + 11 + 25 passed, 1 ignored. `state.rs` doc lines (`LockWatch`, `work_end_waits`,
  `floor_waits`) and the architecture section 3 paragraph done. Next: deliverable 3, props at 4096, checks.
- 2026-11-04 05:41: deliverable 3 measured in a scratch copy of the workspace
  (`<scratchpad>\p2-6-unread-floor-countdown\copy`, never the shared tree): with `plan_read_due` also due while `retry` is wanted, 3 core tests fail, all pinning one
  retry per 30 s safety tick (`scenarios::settings::unread_journal_is_retried_on_the_next_tick`,
  `unread_journal_that_keeps_failing_keeps_the_retry`, and my file's
  `with_an_unread_journal_a_closed_lid_counts_down_once_the_retry_puts_the_users_sleep_back`). The failing run's trace
  before the fix also showed the retry at 1 s, 30 s, 60 s. Decided: no change (Result). Hand mutants started in the
  copy: one build job, one test thread, the test process stopped by its PID at 4 GB, at free memory under 12 GB, or at
  150 s (`mutants.ps1`, results to `mutants.txt` as they come). Baseline: passes, 21 s, under 0.1 GB.
- 2026-11-04 05:52: mutants, first run (05:41 to 05:46): 10 run, 6 killed, 4 survived, none stopped, free memory 35.9
  GB after. Survivors: M3 (the wait ignores the lid), M6 and M8 (a want that started stays: after a `Floor` hibernate
  resumes with the lid still closed, a second countdown), M10 (the wait ignores `may_own`, a condition from before
  this card). Added a test for M3 (now named `with_an_unread_journal_a_step_back_with_the_lid_open_wants_no_countdown`)
  and a `Resumed` step to the first new test. Rerun: M3 and M6 killed; M8 hit the 4 GB cap at 32 s and the watchdog stopped
  it by its PID (free memory stayed 36 GB). M8 does the same as M6, so random property cases decide which way it
  fails, as in the last card. M10 left (Found outside scope 3). `cargo test -p wakeshot-core`: 603 + 11 + 25 passed.
- 2026-11-04 05:57: final checks (Result). `cargo fmt --all --check` first showed diffs only in my test file (three
  test names over 100 columns): names shortened, the file formatted alone with `rustfmt`. Build folder, scratch copy
  and scripts deleted and checked gone; free memory 36.3 GB. Status done.

## Result

**Done (2026-11-04).** No contract change: `state.rs` got doc lines only.

### What was done

1. **The `Floor` gap, `core/lid.rs`, the work-end fix shape.** `floor_countdown_wanted` now gives whether it starts,
   and when it does not, it also waits (`floor_waits`) while the journal is unknown (`journal_unknown()`), not only
   before the first plan read. New `floor_countdown_waited`, which `settle_lid` calls for a waiting want once a plan
   read worked: when it starts no countdown, the want stays. So a `Floor` want that waited takes no Do nothing as
   final: it starts the countdown once a read shows the user's own battery lid action as not Do nothing, and ends when
   AC comes back or the lid opens (the existing cancels). With the user's own battery Do nothing no countdown ever
   starts (power safety review finding 7). Why this one and no leaner: after a restore of an unknown journal, the plan
   copy shows the crashed tray's Do nothing until a later read, and no mirror entry can put the `original` back; a
   guess (an unnamed Do nothing counts as ours while unknown) would start at once with no wait state, but would
   hibernate a user whose own battery lid action is Do nothing. The change is two conditions and a four-line function,
   no state change. A known journal behaves as before, except decision 1 below.
2. **Tests first, `scenarios/lid.rs`:** `with_an_unread_journal_a_closed_step_back_counts_down_once_the_users_sleep_is_back`
   (the step back's own restore reads the file; a loop timer and a battery reading come before the read; after the
   hibernate resumes with the lid closed, no second one), `with_an_unread_journal_a_close_after_the_step_back_counts_down_once_a_retry_works`
   (the restore fails, the lid close locks on the unread guess, a second failed retry and a read in between),
   `with_an_unread_journal_the_users_own_battery_do_nothing_gives_no_floor_countdown` (also: the lid opening ends the
   want), and, from the mutants, `with_an_unread_journal_a_step_back_with_the_lid_open_wants_no_countdown`. On the
   original logic the first two failed (no `Hibernate`), the guard passed. `hibernated_after_idle` now takes the
   reason.
3. **`state.rs`, doc lines only:** `LockWatch` ("countdown rules that wait for a plan read that tells the user's own
   lid action"), `work_end_waits` (the suggested text) and `floor_waits` (the same rule for the battery lid action).
4. **Deliverable 3, the retry with an empty copy: no change.** Measured: the retry with an empty copy runs once per
   30 s safety tick (trace of the failing run: 1 s, 30 s, 60 s). Why not 10 s: (a) the `Hibernate` time comes from the
   last input, not from when a waited countdown starts (the two new countdown tests start it about 1 s and about 1
   minute after the last input, and both hibernate at exactly 5 minutes), so 30 s against 10 s moves a hibernate only when the journal stays unreadable past
   about 4.5 minutes of idle, and then by at most 20 s; (b) each retry tick sends its plan read right behind the
   retry, so once a retry works the read that decides comes in the same tick at either cadence; (c) the unread guess
   window (a lock on any Do nothing, `p6-int-wiring-fixes` decision 7) and the crashed tray's values in force last at
   most 20 s longer per failed try, and the 500 ms start-up retry already makes an unread journal rare; (d) cost: the
   function is in `core/settings.rs` (not `core.rs`), and in a scratch copy the one-condition change failed 3 tests
   that pin one retry per 30 s, 2 of them in `scenarios/settings.rs` (`p2-6-unread-journal`'s file).
5. **`docs/architecture.md`, section 3:** the paragraph on countdowns that wait now covers work end and Floor.

### Decided by the worker, for Haik's review

Haik's rule for open calls: with no work going on, the laptop behaves as if Wakeshot were not there.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | A `Floor` want that waited never takes a Do nothing as final; it stays until AC or the lid opens. This also covers a want that waited for the first plan read with a journal that read: it used to end when that read showed Do nothing | Same rule and reason as `p2-6-unread-lid-countdown` decision 1, so both countdowns behave alike. Cost: with the user's own battery Do nothing, if they set Sleep while the lid stays closed and the laptop is still on battery under the floor, the countdown starts then | The same trigger as that card's decision 1: a core card may mark the copy "behind" when a restore of an unknown journal lands and decide both wants once, on the next good read |
| 2 | Deliverable 3: the retry with an empty copy stays at the 30 s safety tick | See 4 above: it barely moves any outcome, and the change touches another card's tests | Haik keeps decision 7's lock guess and someone meets a lock on their own Do nothing after a failed journal load, or logs show a journal unreadable for minutes: then `plan_read_due` also sets its deadline while `retry` is wanted (one condition), and the three tests count retries per 10 s |

### Files changed

`crates/wakeshot-core/src/core/lid.rs`, `crates/wakeshot-core/src/scenarios/lid.rs`,
`crates/wakeshot-core/src/state.rs` (doc lines), `docs/architecture.md` (one paragraph), this card. No
`docs/research.md` entry: no new outside fact (the measurements are in this card).

### Checks (2026-11-04, build folder `<scratchpad>\p2-6-unread-floor-countdown\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (after shortening three of my test names) |
| `cargo clippy -p wakeshot-core --all-targets --target <t> -- -D warnings`, and without `--all-targets`, x64 and ARM64 | all four exit 0 |
| `cargo test -p wakeshot-core` | 603 + 11 + 25 passed, 1 ignored (4 new tests) |
| Scenario props at `PROPTEST_CASES=4096`, once | 195 passed, 1 ignored, 34 s; free memory 36.1 GB after |
| Hand mutants on the changed lines (scratch copy, one build job, one test thread, test process stopped by its PID at 4 GB, at free memory under 12 GB or at 150 s) | 10 run. Killed: 8 (M1, M2, M4, M5, M7, M9; M3 and M6 after the two test additions). M8 (same effect as M6) hit the 4 GB cap at 32 s on its rerun and was stopped; its first run survived, so random property cases decide. M10 survives: it drops `may_own()`, a condition from before this card (Found outside scope 3). Free memory 35.5 GB or more at each start and end; the 12 GB floor never triggered |
| `cargo test --workspace --locked`, once (05:54 to 05:57) | exit 0, nothing failed: cli 75 + 3 + 5, core 603 + 11 + 25, tray 8 + 5, windows 549; 1284 passed, 52 ignored |
| Build folder, scratch copy and scripts deleted | yes, checked gone 05:57 |

### Open items

- Found outside scope 2: a journal that stays unreadable keeps both waited countdowns waiting; tied to
  `p6-int-wiring-fixes` decision 7 (Haik's call).
- Found outside scope 3: the ownership condition on the first-read wait (M10); a design call, then one condition in
  each of two functions plus a test.
