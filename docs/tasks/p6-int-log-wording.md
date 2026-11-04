# Log wording: journal key names, a false WARN at quit, one log file name in the CLI

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-int-dry-run-e2e` "Found outside scope" 1 and 2 and
`p6-int-wiring-fixes` "Found outside scope" 1 and 2.
Runs after: `p6-int-wiring-fixes` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-log-wording\target`.

## Goal

Three small log fixes, each with a test where one fits.

## Read first

- `docs/tasks/p6-int-dry-run-e2e.md` "Found outside scope" 1 and 2; `docs/tasks/p6-int-wiring-fixes.md` "Found outside
  scope" 1 and 2.
- `docs/architecture.md` section 8 (log lines: fields, plain English, no file content).

## Deliverables

1. `crates/wakeshot-windows/src/power/journal.rs`: the apply line prints the written keys as a Rust debug list
   (`wrote [LidAc, LidDc, SleepDc]`), and the "journal saved" lines spell them `LidAc` while the plan lines use
   `lidAc`. Use plain words and one spelling, the one the journal file and the plan lines use (`lidAc, lidDc,
   sleepDc`). Update the unit tests that assert the text.
2. `crates/wakeshot-windows/src/watch/thread.rs`: a post that fails with 1400 (the loop window is gone, as after a
   quit) logs a WARN "the next tick posts everything again", though no tick comes. Log it at Info, as the IO thread
   logs the same case ("as after a quit"). Test with the fake poster if the file has one.
3. `crates/wakeshot-cli/src/open_log.rs`: use `wakeshot_windows::log::log_file` instead of its own "wakeshot.log".
4. Optional, only if the cause is clear: `watch::thread::tests::a_process_exit_posts_the_session_and_the_hold_as_gone_at_once`
   failed once under load (`p6-int-wiring-fixes` Found outside scope 2: `rig.settled().len()` not 2 at
   `watch/thread.rs` about line 1070). If its settle wait is too tight for a loaded runner, widen it the way other
   tests in the file wait; otherwise leave it and say what you saw.
5. Docs lines that quote a changed log line (grep `docs/` for the old text): change only lines that describe what the
   app logs now; leave records of past runs (work logs, card Logs, dated research results) as they are.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64; ARM64 for `wakeshot-windows` and `wakeshot-cli` (the tray's ARM64 needs clang: CI only);
  `cargo test --locked -p wakeshot-windows` and `-p wakeshot-cli`; `cargo test --workspace --locked` once at the end.
- Build folder deleted at the end.

## Out of scope

- Any other log line; `app/*` files; core files.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. Core tests of the lid card that runs at the same time (`p2-6-unread-lid-countdown`), in my workspace test runs:
   - Run 1: `scenarios::lid::with_an_unread_journal_the_countdown_waits_through_failed_retries` and
     `..._a_closed_lid_counts_down_once_the_retry_puts_the_users_sleep_back` failed (`scenarios/lid.rs:615`). Alone at
     05:11: both pass, with the other two `with_an_unread_journal` tests. Likely their edits were half done.
   - Run 2: the property test `scenarios::props::no_lock_when_the_lid_action_in_force_is_the_users_own`
     failed: "a lid lock while the user's own Do nothing is in force (User), at run 0 ms, event Lid { closed: true }";
     the trace's first event is the lid close, answered with `Lock` and the chime. Alone at 05:12: passes (random
     cases, so a pass alone does not clear it). Run 3 (`--no-fail-fast`): all pass. The lid card should look at this
     case: it reads like the new "an unread journal may be ours" guess locking over a user's own Do nothing.
2. The phase file `docs/phases/06-integration.md` (about line 365) still names the two log problems this card fixes;
   the orchestrator owns it.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 04:51: worker started; read CLAUDE.md, plan "How we work", this card.
- 2026-11-04 04:55: `journal.rs` changed: known keys print by their journal names through `PlanSetting::name`
  (`lidAc`), the apply line prints `wrote lidAc, lidDc` or `wrote nothing`; tests updated. Tray tests grepped for the
  old texts: only `e2e_dry_run.rs` 186 and 219 wait on "apply to plan", which stays.
- 2026-11-04 04:59: `watch/thread.rs` (1400 at Info, a new test, events shown when the exit test fails) and
  `open_log.rs` (`log::log_file`) changed. rustfmt check on the three files and clippy x64 and ARM64 for
  `wakeshot-windows` and `wakeshot-cli`, with and without `--all-targets`: clean. Tests next.
- 2026-11-04 05:06: package tests pass (windows 549, cli 83). Flaky exit test: reproduced, 5 of 40 runs failed with
  8 copies at once; the extra event is the session posted as gone before its process ended. A temporary probe (removed)
  showed the cause: right after the first round, a notice in the data folder names `holds`, in every run; when the
  test clears the fake before that read, the read posts the session gone. Fix: settle before the clear. After it, 40 of
  40 with the probe and 60 of 60 without (12 at once) pass.
- 2026-11-04 05:15: final checks (Result). Decided by the worker, for Haik's review (none changes the product):
  1. The ownership check line ("the user changed sleepDc on ...") also takes the journal spelling, though the card
     names only the apply and "journal saved" lines: the card asks for one spelling, and that line is in the same file.
     Change it when: never, unless the journal names change.
  2. An unknown key (from a newer version's journal) stays quoted and escaped (`"hibernateDc"`): the name comes from
     the file, and escaping keeps a stray quote or line break from breaking the one-line format.
  3. An apply that wrote no key says `wrote nothing`, not an empty list.
  4. The watch thread's 1400 line: "the loop window is gone (PostMessageW error 1400), as after a quit: this event and
     every later one are dropped", Info, once per run of failures, close to the IO thread's words. Other codes keep the
     WARN line as it was.
  5. The CLI's open-log tests keep the literal "wakeshot.log", so a change of the shared name fails a test there.
  6. The flaky test: the cause was clear (not a short wait), so it is fixed by a settle before the fake changes, and
     the two "nothing came" asserts now print the events when they fail. Facts in research "Log wording checks".

## Result

Done.

- `crates/wakeshot-windows/src/power/journal.rs`: key names in log lines come from `PlanSetting::name` (`lidAc`,
  `lidDc`, `sleepDc`), as in the journal file and the plan lines. Apply line: `apply to plan <guid> done; wrote lidAc,
  lidDc, sleepDc; 3 journal entries` (or `wrote nothing`). "journal saved" and "ownership check" lines use the same
  names. Tests `save_load_round_trip` (now an exact line) and `outcome_lines_name_the_result` updated.
- `crates/wakeshot-windows/src/watch/thread.rs`: a post that fails with 1400 logs one Info line (decision 4); other
  codes keep the WARN. The test sink now fails with a chosen code; new test
  `a_post_after_the_loop_window_is_gone_is_logged_once_as_info`; the 1816 test also checks the WARN level. Flaky test
  `a_process_exit_posts_the_session_and_the_hold_as_gone_at_once` fixed (Log 05:06).
- `crates/wakeshot-cli/src/open_log.rs`: uses `wakeshot_windows::log::log_file`; its own `LOG_FILE` and `log_path`
  are gone.
- `docs/research.md`: new section "Log wording checks" (the `holds` notice after start).
- Docs quoting the old lines: none describes what the app logs now; all are records of past runs (research
  3596-3601, card Logs), left as they are. The phase file is the orchestrator's (Found outside scope 2).
- Needs another card: none. Tray tests (`e2e_dry_run.rs`, `e2e_real.rs`, `e2e/mod.rs`, `smoke.rs`, `recovery.rs`)
  match no changed text; `e2e_dry_run.rs` 186 and 219 wait on "apply to plan", which is unchanged.

Checks:

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy x64, `--workspace`, with and without `--all-targets`, `-D warnings` | clean |
| clippy ARM64, `-p wakeshot-windows -p wakeshot-cli`, with and without `--all-targets` | clean; tray ARM64: CI only (no clang) |
| `cargo test --locked -p wakeshot-windows` | 549 passed, 21 ignored (548 before, plus the new test) |
| `cargo test --locked -p wakeshot-cli` | 83 passed, 5 ignored |
| exit test stress, test binary copies at once | before the fix 5 of 40 failed (8 at once); after it 0 of 40 (probe) and 0 of 60 (12 at once) |
| `cargo test --workspace --locked` | runs 1 and 2 failed in core tests of the lid card running at the same time (Found outside scope 1; each passes alone); run 3 (`--no-fail-fast`): all pass, cli 83, core 635, tray 13, windows 549; 1280 passed |
| build folder | deleted, with the scratch folder |

Open items: Found outside scope 1 (lid card) and 2 (phase file line).
