# Wiring fixes: a lost first-run notice, a stuck hibernate flag, zone errors, one log file name, the unread lid guess

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens. Written by the orchestrator
(session `wakeshot-46`, 2026-11-04) from `p6-int-tray-mode` "Found outside scope" items 1 to 3 and its "Needs another
card" (`log::log_file`).
Runs after: `p6-int-tray-mode` Part A (done), `p6-velopack-dependency` (it runs alone), `p6-int-update-wiring` (it
owns `tray_mode.rs` before this card). Parallel with: any card that owns none of the files below.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-wiring-fixes\target`.

## Goal

Close four small gaps the tray wiring found, each with the leanest fix and a test.

## Read first

- `docs/tasks/p6-int-tray-mode.md`: "Found outside scope" 1 to 3, "Needs another card".
- `crates/wakeshot-windows/src/app/dispatch.rs` (the `hibernating` flag), `crates/wakeshot-windows/src/notify.rs`
  (`BalloonNotifier`), `crates/wakeshot-core/src/core.rs` (`take`, `settle`: the `Started` batch order),
  `crates/wakeshot-windows/src/localtime.rs` (`SystemZone`, `take_error`), `crates/wakeshot-windows/src/log.rs`.

## Deliverables

1. First-run notice: when the first icon add fails (a start at sign-in before the taskbar exists), the notice must
   still show once the icon is added. Pick the leanest place (the core's batch order, `BalloonNotifier` keeping a
   notice that failed for want of an icon, or the tray holding it until `Note::Added`) and say why. Test with the fake
   shell.
2. Hibernate flag: a `Done(Hibernate)` whose post fails (queue full, 1816) must not leave the dispatcher's
   `hibernating` flag set for the rest of the run. Pick the leanest fix (a short retry of the post on the hibernate
   thread, or a clear on `Resumed` or on the next lid open) and say why. It must never start two hibernate threads at
   once. Test.
3. `SystemZone` errors: once the zone is boxed into `ClaudeSource`, a failing zone read is never logged. Log it (once
   per new error, not per read) without a contract change if you can; if not, describe the change and stop.
4. `log::log_file(logs_dir) -> PathBuf` in `log.rs`, used by `io_thread.rs`, `hooks.rs` and `tray_mode.rs` instead of
   their own copies of "wakeshot.log". Remove the copies and `hooks.rs`'s agreement test if it becomes moot.

5. Added after docs pass round 3: `core/lid.rs` treats an unread startup journal as "may be ours" until the first
   read lands, so a lid close in the first seconds after a start with an unread journal still gets the lid lock
   rules (`p2-6-unread-journal` "Found outside scope" 3). Test in the core scenarios.
6. Log wording seen in the first smoke run (`p6-int-tray-mode` "Found outside scope" 6 and 7): `journal.rs` logs
   "restored" for the save made before any write; the startup restore logs nothing when there is no journal, while
   the restore at Quit does. Make each line say what happened; keep any line a test or `docs/` quotes, or update both.

## Acceptance checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc`; ARM64 the same if clang is on the laptop, else "ARM64: CI only".
- `cargo test --workspace --locked`: all pass; record the counts.
- Only the files the deliverables name and this card changed.

## Out of scope

- Any real power setting, screen, sound or tray start. Tests use fakes only.
- The update thread wiring (`p6-int-update-wiring`).

## Questions for Haik

None expected. Haik is away: decide with your default and list it as "decided by the worker, for Haik's review".

## Found outside scope

1. `crates/wakeshot-cli/src/open_log.rs` keeps a fourth copy of "wakeshot.log" (`LOG_FILE`, `wakeshot restore`'s Open
   log). It can use `wakeshot_windows::log::log_file` now. Not my file.
2. `watch::thread::tests::a_process_exit_posts_the_session_and_the_hold_as_gone_at_once` (real child processes, a
   settle wait) failed once at `watch/thread.rs:1070` (`rig.settled().len()` not 2) in the last full workspace run,
   while the install card worked on the laptop. It passed 5 of 5 alone right after, the full `wakeshot-windows` run
   after that, and the two full runs before. Flaky under load; not my file.
3. Unread journal, the part this card does not close: a lid close before the retry lands now locks, but the work-end
   countdown still reads the Do nothing in force as the user's own (`work_ended_on` uses `in_force` with the empty
   copy), so no countdown starts. When the retry then restores the user's Sleep, a laptop whose lid is already closed
   does not sleep by itself, so it runs until the user's battery sleep timeout. With a journal that read, the same
   case counts down (the copy has the `original`). Minor; a core lid card (a countdown when a `Leftover` restore
   lands with the lid closed and nothing holding).
4. Quotes of the old journal text "journal saved ... restored": `docs/research.md` "Install end to end checks" and
   `docs/tasks/p6-install-e2e.md` (Log, 156) record what those runs logged; `docs/phases/06-integration.md` (325) and
   `docs/tasks/p6-int-tray-mode.md` (Found outside scope 6) describe the old text. Left: records of past runs, a phase
   file, and other cards' files. The new text is "being changed".
5. `scenarios/props.rs` (`WorldStart`, about line 1108) does not draw `startup_unread` because "the lock rules here
   would need that excuse". With item 5 the lock rules have a guess for it, so drawing it may work now: not tried.
6. `docs/tasks/p6-int-tray-mode.md` "Notes for the Part B run" picks the `NotifyIconSettings` entry by `IconGuid`.
   In this run the entry held only `IconSnapshot` (research, "Wiring fixes checks"), so compare ids before and after.

## Log

- 2026-11-04 03:55: started (worker). Read the card, `CLAUDE.md`, `plan.md` "How we work", the cited Found outside
  scope items.
- 2026-11-04 04:06: picks made (see Result). Edited `log.rs` (`log_file`), `io_thread.rs`, `hooks.rs` (copies gone,
  the agreement assert dropped, the test kept), `notify.rs` (refused notices kept for the next add), `dispatch.rs`
  (`start_hibernate` returns `Result`, a failed start is fed at once). Next: `tray_mode.rs`, `localtime.rs`,
  `core/lid.rs`, `journal.rs`, `startup.rs`, then the build.
- 2026-11-04 04:14: all six items coded with tests; x64 clippy of core and windows clean (one `type_complexity` fixed
  with a `ZoneLog` struct); rustfmt on the 11 changed files. Next: tests.
- 2026-11-04 04:21: `cargo fmt --all --check` exit 0; x64 workspace clippy (both forms) clean; ARM64 clippy of core,
  windows and cli (both forms) clean; `cargo test --workspace --locked` all pass. Before the smoke run: 16
  `NotifyIconSettings` entries, none with the dry-run GUID; no Wakeshot process. Next: the dry-run smoke test.
- 2026-11-04 04:22: smoke test run 1 passed (6.05 s, tray pid 34360). Its log showed the write-ahead and the restore's
  mark both saved before the writes, so the interim wording "plan holds the original" was wrong for the restore:
  changed to "being changed". Entry `15983757285637097514` exported and deleted 04:23:23; gone at 04:24:37.
- 2026-11-04 04:27: final checks (all below). One flaky watch test in the last full run (Found outside scope 2).
  Build folder deleted and checked gone. `docs/research.md` section "Wiring fixes checks" added.

## Result

**Done (2026-11-04 04:27).** All six items, each with tests. No contract change; `LoopRunners::start_hibernate`
(not a contract, `dispatch.rs`) now returns `Result`.

Files: `crates/wakeshot-windows/src/log.rs`, `app/io_thread.rs`, `hooks.rs`, `notify.rs`, `app/dispatch.rs`,
`app/tray_mode.rs`, `localtime.rs`, `power/journal.rs`, `app/startup.rs`; `crates/wakeshot-core/src/core/lid.rs`,
`scenarios/lid.rs`; `docs/research.md` (new section "Wiring fixes checks"); this card.

### What each item does

1. **First-run notice.** `BalloonNotifier` keeps each notice the shell refused (in order, once each) and gives them
   back from `icon_added()`; the tray's icon note hook calls it at every `Note::Added` and pushes them to the IO queue
   as `Command::Notify`, with one line. An add that lands while a refused call runs gets a second try at once.
   `icon_added` takes only its own lock, never the shell, so the loop thread never waits on Explorer.
2. **Hibernate flag.** The hibernate thread posts its end again while the post fails with 1816, every 1 s, up to 60
   tries; any other error (1400: the window is gone) stops at once. One Info line when a later try worked, one Warn at
   the give-up ("... at try N; no other hibernate starts in this run"). A thread that cannot start no longer posts:
   `start_hibernate` returns the error and the dispatcher feeds `Done(Hibernate(Failed))` at once. The flag still
   clears only when the end is fed, and the post is the thread's last act, so two threads never run at once.
3. **Zone errors.** `SystemZone::current_logged(log)`: each distinct error (call and code) is logged once per run,
   Warn, area `watch`: "the time zone does not read: X failed with N; cron times use the offset read last (M min)".
   `tray_mode::sources` uses it with the direct log. `take_error` and `current()` stay as they were.
4. **Log file name.** `log::log_file(logs_dir) -> PathBuf`; `LogWriter::open` and `roll` use it, and so do
   `io_thread.rs` (Open log), `hooks.rs` (the last delete) and `tray_mode.rs` (balloon click). The three copies are
   gone. In `hooks.rs` only the agreement assert went; its test stays, since it also proves the open log is deleted.
5. **Unread journal.** `Core::journal_unknown()`: a restore is still wanted and the copy names no entry. Then
   `lock_view` counts each lid key in force at Do nothing that no entry names as ours, and before the first plan read
   both lid keys (a stand-in plan at Do nothing). It ends when a restore or apply reads the file: the copy gets
   entries, or the `Leftover` retry ends with `Ok`.
6. **Log wording.** The journal's `restored` mark prints as ", being changed" (saved before an apply's first write
   and before a restore writes the original back; research, "Wiring fixes checks"). Step 6 logs one Info line unless
   the journal did not read (that case has its own lines): "startup restore done: no journal entry is left", or
   "startup restore done: N journal entries left for the retry on the tick".

### Decided by the worker, for Haik's review

None of these makes the laptop behave differently when no work is going on.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | Item 1 in the notifier and the icon hook, not in the core's batch order | The core cannot see a failed add (`show_tray` answers nothing), so no order saves the notice then; the notifier is the one place that sees the refusal, the icon hook the one that sees the add | Notices move to registered toasts |
| 2 | Every refused notice is kept, not only the first-run one; each once | The same code; a notice refused during an Explorer restart is lost the same way | A field log shows an old notice shown late at an add |
| 3 | A notice shown again skips the core's "hold while locked" rule | It goes straight to the IO queue; only an add while the session is locked (Explorer restart behind the lock screen) shows it to nobody | That case shows up in a log: then hold it until `Unlocked` in the tray |
| 4 | Item 2: retry on the hibernate thread (1 s, 60 tries, 1816 only), and a failed start fed at once | A `Resumed` or lid open can come while the thread still runs its checks or sound (a user's own sleep), so clearing the flag then could start a second thread; only the thread's own end proves it is done | A log shows "hibernate result not posted ... at try 60": then also clear the flag when the thread's handle shows it finished, at a safety tick |
| 5 | Item 3: one line per distinct error per run, kept in the zone itself | The cron search reads the zone many times per search; no contract change | A log needs to show a failure that comes back after a good read |
| 6 | Item 5 derived from the state (`retry` set, copy empty), no new `State` field | `state.rs` is not this card's; the rule also covers a failed load of a copy known empty, and a `UserChanged` that emptied the copy while a retry waits; each lasts until the next retry works | A core card adds a "journal unknown" field, or a log shows the guess locking a user's own Do nothing |
| 7 | Item 5 errs toward a lock | In that window a user whose own lid action is Do nothing gets a lock and the chime (decision 18 says neither), against a crashed tray's Do nothing that would run a closed laptop unlocked | Never likely: the window ends at the first good read |
| 8 | Item 6: "being changed", and a start-up line that does not tell "no journal" from "all restored" | The mark is saved before the writes in both routines; `Startup` carries no more, and the restore's own line already names what it found | `PowerExecutor::startup` reports what it found |

### Checks (2026-11-04, build folder `<scratchpad>\p6-int-wiring-fixes\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (after `rustfmt` of the changed files only) |
| `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | clean |
| the same without `--all-targets` | clean |
| ARM64: `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli` with and without `--all-targets` | clean; ARM64 tray: CI only (no clang here, `ring`) |
| `cargo test --workspace` | all pass: cli 83 (5 ignored), core 632 (1 ignored), tray 13 (15 ignored), windows 548 (21 ignored); 1276 passed. One flaky watch test failed once in the last full run and passed on every rerun (Found outside scope 2) |
| Smoke test, `cargo test -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored --test-threads=1` | passed, run 1 of 1, 04:22:28, 6.05 s, tray pid 34360; the new start-up line in its log. Run before the last wording change in `journal.rs` (a format string, checked by `save_load_round_trip`) |
| Dry-run `NotifyIconSettings` entry | id `15983757285637097514` (new against the 16 ids listed before), exported to `<scratchpad>\p6-int-wiring-fixes\notifyicon-15983757285637097514.reg`, deleted 04:23:23, still gone 04:24:37; no `NotifyIconGeneratedAumid_15983757285637097514` key |
| Left behind | no Wakeshot process, no `%TEMP%\wakeshot-tray-smoke-*`, no `%LocalAppData%\Wakeshot`; no power setting, Run, RunOnce or Path value touched; build folder deleted and checked gone |
| Only the card's files changed | yes: the 11 code files above, one new `research.md` section, this card |

New tests: `notify` `a_refused_notice_waits_for_the_next_add`, `an_add_during_the_shell_call_tries_the_notice_again_at_once`;
`dispatch` `a_hibernate_thread_that_does_not_start_answers_at_once_and_frees_the_next`; `tray_mode`
`a_hibernate_end_is_posted_again_while_the_queue_is_full`, `notices_refused_before_the_add_go_to_the_io_thread_again`,
`a_hibernate_end_that_cannot_be_posted_is_logged` (rewritten: 60 tries at 1816, 1 at 1400); `localtime`
`a_logged_zone_logs_each_new_error_once`; `scenarios::lid` `a_lid_close_before_any_read_locks_when_the_journal_did_not_read`,
`with_an_unread_journal_a_do_nothing_in_force_may_be_ours_until_a_read_lands`,
`an_unread_journal_keeps_the_guess_until_a_retry_reads_it`. New asserts in four `startup` tests, `log`
`open_creates_the_folder_and_appends` and `journal` `save_load_round_trip`.

### Needs another card

- `wakeshot-cli` (`open_log.rs`): use `log::log_file` (Found outside scope 1).
- Core lid card: the countdown with an unread journal (Found outside scope 3).
- Orchestrator: the old journal text in records and the phase file (Found outside scope 4); the `IconGuid` clean-up
  note in `p6-int-tray-mode` (Found outside scope 6).
