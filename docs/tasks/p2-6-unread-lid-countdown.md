# A closed lid still counts down when an unread journal's restore lands

Phase: 2 to 6 (core). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-int-wiring-fixes` "Found outside scope" 3 and 5.
Runs after: `p6-int-wiring-fixes` (done; it changed `core/lid.rs` and `scenarios/lid.rs`). Parallel with: cards that
own none of the files below. It may run beside `p6-int-dry-run-e2e` (that card fixes only `wakeshot-windows` `app/*`
files, the tray tests and the CLI command files).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-unread-lid-countdown\target`.

## Goal

With an unread startup journal (`Event::Started { journal_unread: true }`), a lid closed before the `Leftover`
restore lands must end the same way as with a journal that read: once the restore puts the user's own lid action back
and nothing holds, the closed laptop must not run on until the user's battery sleep timeout.

Today (`p6-int-wiring-fixes`, Found outside scope 3): the lock rules now treat the unread case as "may be ours", but
the work-end countdown reads the Do nothing in force as the user's own (`work_ended_on` uses `in_force` with the
empty copy), so no countdown starts. When the retry then restores the user's Sleep, a laptop whose lid is already
closed does not sleep by itself. With a journal that read, the same case counts down (the copy has the `original`).

## Read first

- `docs/tasks/p6-int-wiring-fixes.md`: deliverable 5, its Result, Found outside scope 3 and 5.
- `docs/tasks/p2-6-unread-journal.md`: Goal 1 and its Result.
- `crates/wakeshot-core/src/core/lid.rs` (`in_force`, `work_ended_on`, the countdown rules, the unread guess from
  `p6-int-wiring-fixes`); `crates/wakeshot-core/src/scenarios/lid.rs`; `crates/wakeshot-core/src/scenarios/props.rs`
  (`WorldStart`, `startup_unread`).
- `docs/product.md` (lid rules); `docs/architecture.md` (grep "countdown" and "unread").

## Deliverables

1. The leanest fix in `core/lid.rs` (for example: a countdown when a `Leftover` restore lands with the lid closed and
   nothing holding, or the unread guess also covering `work_ended_on`), and why that one. It must start no countdown
   when the user's own lid action, once known, is Do nothing (the plan read tells), and must not change the read
   journal cases. Tests first, in `scenarios/lid.rs`: the case above fails before the fix.
2. `scenarios/props.rs`: try drawing `startup_unread` in `WorldStart` now that the lock rules have a guess for it.
   Keep it if the properties pass at their usual case counts; if not, say why in the Result and leave it as it is.
3. `docs/architecture.md`: only the line or lines that state this rule, if one changes.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and
  without `--all-targets`, both targets (core needs no clang); `cargo test --locked -p wakeshot-core`; then
  `cargo test --workspace --locked` once at the end.
- Mutants on the changed lines if the card's time allows (as `p2-core-lid` did); record the count.
- Build folder deleted at the end.

## Out of scope

- `wakeshot-windows` files, `core.rs`, `state.rs`, `event.rs` (a contract change: describe it and stop if the fix
  needs one).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. **Flaky property, pre-existing: fixed here at the orchestrator's request (05:21), see Result.**
   `scenarios::props::no_lock_when_the_lid_action_in_force_is_the_users_own` failed 6 of 60 runs at the usual case
   count with this card's lid change switched off and `startup_unread` never drawn, and 3 of 3 runs at 4096 cases.
   Every minimal case has a restore that fails to load the journal (code 2 at call +0 or +1) while the copy is
   empty, so `journal_unknown()` is true and `lock_view`'s guess (`p6-int-wiring-fixes` item 5) locks and chimes on
   the user's own Do nothing (for example after a switch to the docked plan). That card took this trade-off on
   purpose (its decision 7, "errs toward a lock"); the property did not excuse it. Two cases: "start on Balanced, lid
   open, on battery at 10 %, automatic-off, not-owner, off-console: Keep awake 30, IO call +1 of the next Restore
   fails (code 2), user switches to other plan, menu Restore twice, IO runs three, lid closes"; "start on other plan,
   same flags: session ending, run 1 s, IO call +0 of the next command fails (code 2), IO flush, lid closes". Still
   open for Haik: decision 7 itself (a lock and chime on a user's own Do nothing while the journal is unknown).
2. **The `Floor` countdown has the same gap.** `floor_countdown_starts` reads the in-force view while the journal is
   unknown, and `floor_waits` waits only before the first plan read. So a step back with the lid closed on a crashed
   tray's Do nothing (unread journal) starts no `Floor` countdown, and after the restore the closed laptop runs on
   battery under the floor until the user's battery sleep timeout. Rare (a hold with an unread journal, on battery
   under the floor, lid closed). Same fix shape as this card: wait while the journal is unknown, and a want that
   waited takes no Do nothing as final.
3. **`state.rs` doc lines (contract file).** `LockWatch` says "the countdown rules that wait for the first plan read",
   and `work_end_waits` says "Work ended with the lid closed before the first plan read, on these power sources: the
   read decides." The want now also waits while the journal is unknown, and once it waited it stays until a read
   shows the user's own action as not Do nothing. Suggested `work_end_waits` text: "Work ended with the lid closed
   while the user's own lid action was not known (no plan read yet, or the journal unknown), on these power sources:
   it starts the countdown once a read shows their own action as not Do nothing (`core::lid::work_end_waited`)."

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 04:38: worker started; reading the card's files.
- 2026-11-04 04:57: read `core/lid.rs`, `core.rs`, `core/settings.rs` (`tick`, `take_plan`, `done_restored`),
  `state.rs`, the two cards. Root cause: after the retry's restore of an unread journal lands, the plan copy still
  shows the crashed tray's Do nothing until the read the same tick sent behind the retry is answered, and no journal
  entry can put the `original` back (`plan_copy`). So the want must wait for that read. Rejected: dropping the plan
  copy at the retry (the floor falls back to 20 % until the read, a step back could start), the guess in
  `work_ended_on` (a countdown before the read, on a user's own Do nothing). Picked: the want waits while the journal
  is unknown, and once known a Do nothing decision stays open until the next tick that reads the plan. Writing the
  tests first.
- 2026-11-04 05:03: three new tests in `scenarios/lid.rs`. On the original logic two failed (no `Hibernate`), the
  own Do nothing guard passed. While coding the tick bound I found it wrong: a hold-end or step-back restore has no
  read right behind it, so the next read tick would still decide on the old copy. Switched to: a want that waited
  stays until a read shows the user's own action as not Do nothing, the lid opens or a hold starts (no tick bound).
  Fix in `core/lid.rs` (`work_ended_on` waits while the journal is unknown; new `work_end_waited`). `cargo test -p
  wakeshot-core`: 599 + 11 + 25 passed, 1 ignored. Next: props `startup_unread`, docs, checks.
- 2026-11-04 05:12: drew `startup_unread` (weight 0.2). One run at the usual count passed; at 4096 cases
  `no_lock_when_the_lid_action_in_force_is_the_users_own` failed. Measured that property alone, 60 runs each at the
  usual count: 6 of 60 fail with the draw off and my lid change switched off too (pre-existing, Found outside scope
  1), 12 of 60 with the draw on (5 of the 12 minimal cases start unread: the guess locks on the docked plan's own AC
  Do nothing). So the draw stays off; comment updated. Next: architecture lines, mutants, checks.
- 2026-11-04 05:21: architecture lines done; `work_end_waited` lost a redundant lid check (a lid open always clears
  the want first). The orchestrator relayed a workspace run of another card where
  `no_lock_when_the_lid_action_in_force_is_the_users_own` failed at a lid close at run 0 ms. That can be my draw (on
  from about 05:06 to 05:12) or the pre-existing failure above; both are the same guess locking on a user's own Do
  nothing while the mirror may not know the journal. Fixed in the property, not the guess: the core cannot tell a
  user's Do nothing from a crashed tray's then, and `p6-int-wiring-fixes` decision 7 chose a lock. `Model` now keeps
  the journal mirror's keys and the restore retry from the events (section 3 and 5 rules), and that one property
  skips a lock while `retry` is wanted and the mirror is empty. With that, `startup_unread` is drawn (weight 0.2).
  All 22 props at `PROPTEST_CASES=4096`: passed 3 of 3 runs. The first mutant run was void (the scratch copy lacked
  `docs/hold-files.md`, so no mutant compiled); rerunning.
- 2026-11-04 05:24: the orchestrator reported free memory at 0 GB: the test binary of my mutant rerun (pid 38452,
  started 05:18) had a 35 GB working set. Stopped it, the runner and its cargo processes by PID; none left; free
  memory 36.3 GB of 55.6 GB after. It was mutant 6 of 7, "`work_ended_on` never says it started" (every waiting want
  stays, also after its countdown started). Cause not diagnosed (not verified): most likely a property case where the
  mutant makes the World fire timers without end, so steps and trace lines pile up. Mutant 4 has the same effect and
  finished, so random cases decide. Mutants are not run again (no memory cap at hand): 5 of 7 run, 5 killed. Mutant
  folder deleted.
- 2026-11-04 05:28: final checks (Result): fmt, four clippy runs, core tests, props at 4096 cases, the workspace test
  once (exit 0). Build folder deleted and checked gone. Status done.

## Result

**Done (2026-11-04).** No contract change.

### What was done

1. **The fix, `core/lid.rs`.** `work_ended_on` (now gives whether it wants the countdown): while the journal is
   unknown (`journal_unknown()`), a Do nothing in force decides nothing, and the want waits in `work_end_waits`. New
   `work_end_waited`, which `settle_lid` calls for a waiting want once a plan read worked: when it starts no
   countdown, the want stays. So a want that waited (before the first plan read, or on an unknown journal) takes no
   Do nothing as final: it starts the countdown once a read shows the user's own lid action for its power source as
   not Do nothing, and ends when the lid opens or a hold starts (the existing cancels). With the user's own Do nothing
   no countdown ever starts. Why this one: after the retry's restore of an unread journal, the plan copy shows the
   crashed tray's Do nothing until a later read, and no mirror entry can put the `original` back (`plan_copy`); a
   hold-end or step-back restore has no read right behind it at all; and with the current state no settle pass can
   tell a read taken after the restore from an older one. Waiting is the only lid-only rule that never decides on
   the old copy. Read journal cases: unchanged when the journal is known at work end; a want that waited for the
   first plan read and found the user's own Do nothing now stays instead of ending (decision 1).
2. **Tests first, `scenarios/lid.rs`:** `with_an_unread_journal_a_closed_lid_counts_down_once_the_retry_puts_the_users_sleep_back`
   (the tick sends the retry before the read; a loop timer and a battery reading come between the restore and the
   read; also the lid closed before any read), `with_an_unread_journal_the_countdown_waits_through_failed_retries`,
   `with_an_unread_journal_the_users_own_do_nothing_gives_no_countdown` (also: the lid opening ends the want). On the
   original logic the first two failed (no `Hibernate`), the third passed, as a guard should.
3. **`scenarios/props.rs`:** `startup_unread` is drawn now (weight 0.2). That needed the property fix the
   orchestrator asked for at 05:21: `Model` keeps the journal mirror's keys and the restore retry from the events
   (section 3 and 5 rules, `take_journal`), and `no_lock_when_the_lid_action_in_force_is_the_users_own` skips a lock
   while a retry is wanted and the mirror is empty (`Model::journal_unknown`), the window where the lock rules lock
   on any Do nothing by design. That also ends a pre-existing flake of the same property (Found outside scope 1: 6 of
   60 runs failed at the usual count before this card).
4. **`docs/architecture.md`:** section 3, the `LockWatch` line and the paragraph on countdown rules that wait.

### Decided by the worker, for Haik's review

Haik's rule for open calls: with no work going on, the laptop behaves as if Wakeshot were not there.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | A want that waited never takes a Do nothing as final; it stays until the lid opens or a hold starts | The only rule in `core/lid.rs` that never decides on an old copy (see 1 above). Cost: if the user's own action is Do nothing and, with the lid still closed and nothing holding, they set Sleep or switch to a plan with Sleep, the countdown starts then and hibernates after 5 minutes without input. Only after a crashed tray (unread journal, or a `Started` journal before the first read) | A core card may change `state.rs` and `core/settings.rs`: mark the copy "behind" when a restore of an unknown journal lands (`done_restored`), clear it at the next good read (`take_plan`), and decide the want once, on that read, as with a journal that read |
| 2 | While the journal is unknown, a copy that shows anything but Do nothing starts the countdown at once | A key the crashed tray wrote reads Do nothing; any other value is the user's own (a restore keeps a value the user changed) | Never likely |
| 3 | The flaky property is fixed in the property, not in the guess | The core cannot tell the user's Do nothing from a crashed tray's while the journal is unknown, and `p6-int-wiring-fixes` decision 7 chose a lock then | Haik rejects that decision 7: narrow the guess in `core/lid.rs` and drop the excuse |
| 4 | Rejected: dropping the plan copy at the retry; the guess in `work_ended_on`; deciding at the next tick that reads the plan | The first sets the floor to 20 % until the read (a step back could start); the second starts a countdown before the read, at once after a long idle, on a docked user's own Do nothing; the third is wrong for a hold-end restore, which has no read behind it | - |

### Files changed

`crates/wakeshot-core/src/core/lid.rs`, `crates/wakeshot-core/src/scenarios/lid.rs`,
`crates/wakeshot-core/src/scenarios/props.rs`, `docs/architecture.md`, this card. No `docs/research.md` entry: no new
outside fact (the test measurements are here).

### Checks (2026-11-04, build folder `<scratchpad>\p2-6-unread-lid-countdown\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy -p wakeshot-core --all-targets --target <t> -- -D warnings`, and without `--all-targets`, x64 and ARM64 | all four clean |
| `cargo test -p wakeshot-core` | 599 + 11 + 25 passed, 1 ignored (3 new tests) |
| Props at `PROPTEST_CASES=4096`, `startup_unread` drawn | all 22 passed, 3 of 3 runs |
| Same property, 60 runs at the usual count, before the property fix | 6 of 60 failed with the draw off and the lid change off; 12 of 60 with the draw on |
| Hand mutants on the changed lines (scratch copy) | 5 of 7 run, 5 killed; the 6th grew to 35 GB and was stopped (05:24 Log); the 7th not run. The 6th has the same effect as the 4th (killed), the 7th as the 3rd (killed). Not run again: no memory cap at hand |
| `cargo test --workspace --locked`, once (05:25 to 05:28) | exit 0, nothing failed: cli 75 + 3 + 5, core 599 + 11 + 25, tray 8 + 5 (the e2e and smoke tests ignored), windows 549; 1280 passed, 52 ignored |
| Build folder and mutant folder deleted | yes, checked gone 05:28; free memory 36.4 GB after |

### Open items

- Found outside scope 1: decision 7 of `p6-int-wiring-fixes` (a lock and chime on a user's own Do nothing while the
  journal is unknown) is still Haik's call.
- Found outside scope 2: the `Floor` countdown has the same gap; a core lid card.
- Found outside scope 3: two `state.rs` doc lines; a contract card.
