# Sleep, not hibernate, after work ends with the lid closed (and at the battery floor)

Phase: 2 to 6 (core and power). Status: done. Needs Haik: no (his lid test comes later, in `p6-int-haik-checks`).
Agent: `wakeshot-worker`. Size: about 180K tokens. Written by the orchestrator (session `wakeshot-12`, 2026-11-04)
from Haik's answer of 2026-11-04 (`docs/product.md`, work end rule, battery floor, failed lock).
Runs after: `p2-6-record-lost` (same files) and `p3-idle-sleep-check` (its answers decide how Wakeshot sees that the
laptop slept). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-sleep-route\target`.

## Goal

Haik's rule: when Wakeshot is done, the laptop ends up where Windows would have put it; with the lid closed, that is
sleep. His words: "just make it sleep but if technically it's impossible to control then think about your best
effort but do not go to critical". On Modern Standby no app call starts sleep, so Wakeshot uses Windows' own idle
sleep, with hibernate only as the fallback and the built lock as the last step.

## Read first

- `docs/product.md`: the work end rule, battery floor, "Never lower security" (failed lock), sounds; "User settings
  safety".
- `docs/tasks/p3-idle-sleep-check.md` Result (how long idle sleep takes with the lid closed; what a desktop app sees
  at sleep and wake).
- `docs/tasks/p2-6-record-lost.md` Result (the countdown it starts after a lost journal).
- `crates/wakeshot-core/src/core/lid.rs` (work-end and floor countdowns, relock, hibernate result), `policy/lid.rs`,
  `policy/floor.rs`, `settings.rs` (`SettingKey`), `command.rs` (`Hibernate`, `HibernateEnd`), `event.rs`, `state.rs`,
  `text.rs` (`HibernateFailed`).
- `crates/wakeshot-windows/src/power/hibernate.rs`, `power/plan.rs` (`PlanSetting`, writes limited to `SettingKey`),
  `power/executor.rs`, `power/journal.rs` (key names in the file), `power/system.rs`, `app/dispatch.rs`.
- `docs/architecture.md` sections 3, 5 and 6.

## Deliverables

1. `SettingKey::SleepAc`, `VideoIdleAc`, `VideoIdleDc` (journal keys `sleepAc`, `videoIdleAc`, `videoIdleDc`):
   written, journaled and restored like the other three. An older journal without them still loads.
2. The step after the countdown. Where the work-end countdown (5 minutes with no input, the checks, the "going to
   sleep" sound) now sends `Hibernate`, it instead sets the sleep timeout of the plan in use to 60 s on AC and on DC,
   each only where the value in force (the user's own after the restore) is longer or Never, as Wakeshot-owned
   journal entries. The same for the battery floor countdown and for the failed-lock step back. Added after
   `p3-idle-sleep-check` (orchestrator decision 2026-11-04, for Haik's review): the screen timeout too
   (`VideoIdleAc`, `VideoIdleDc`, 60 s, same rule), because on this laptop the sleep came only when the screen
   timeout turned the screen off, not at the 60 s sleep timeout; a user with a long or Never screen timeout would
   otherwise not sleep.
3. Fallback. If 5 minutes after that write the laptop has not slept, the lid is still closed and no input came, send
   `Hibernate` as today (its own checks stay). "Slept" means the suspend seen through the suspend-resume callback
   (`p3-idle-sleep-check`: window messages did not show it), confirmed by the clock gap at the next tick. If
   hibernate is not allowed or fails: the built relock and notice. The short timeouts stay in force, so Windows
   sleeps the laptop once nothing holds it.
4. Restore of the short timeouts: at lid open, at the first unlock after a wake (the user is there), when a hold
   starts, and at every other restore point (quit, sign-out, next start, uninstall, guard). Not at a wake alone: a
   wake with the lid still closed must sleep again. Input after the write (someone at an external screen): restore
   and wait again for 5 minutes with no input. A user's own change to a timeout wins as today.
5. Words: `text.rs` notices that say "hibernate" for this path now say what happens (sleep, or hibernate as the
   fallback). Keep `text.rs` rules.
6. Tests first: core scenarios for work end, floor and failed lock (write, sleep seen, fallback, lid open, unlock
   after wake, a wake with the lid closed, a hold that starts, a user change); props updated; Windows unit tests with
   the fakes; journal round trip with `sleepAc`.
7. `docs/architecture.md` sections 3, 5 and 6: the lines that state these rules. Also sections 5 and 8, which still
   say a damaged journal's values stay (`p2-6-record-lost` for the orchestrator 4).
8. From `p2-6-record-lost` (orchestrator decision 2026-11-04, for Haik's review): a hold that runs through the
   unknown window with the lid closed now keeps the laptop running unlocked until the hold ends, because the lid
   close came while no lock was allowed. When the window ends (`Core::unknown_journal_ended`) with the lid closed
   while Wakeshot holds on its own Do nothing, lock as at a lid close, with the chime ("Never lower security").
9. `examples/power_check.rs`: its `Timed` wrapper does not pass the new default read through
   (`p2-6-record-lost` for the orchestrator 3). Pass it through, or say why it never matters.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets`, x64 and ARM64 (the tray's ARM64: CI only);
  `cargo test --workspace --locked` (debug only); the core props at 4096 cases once.
- Mutants only with one job, a low timeout and a memory cap; stop if free memory drops under 10 GB.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The real lid test (`p6-int-haik-checks` gets it). `README.md`, phase files, `docs/product.md` (tell the
  orchestrator if a product line is wrong).

## Questions for Haik

None (his answer is in `docs/product.md`). Decide the rest yourself and record each choice as "decided by the
worker, for Haik's review".

## Found outside scope

1. **`crates/wakeshot-cli/src/status.rs`** (not my file): `setting_words` has no wildcard, so the three new keys need
   their words to build (the same as `text::setting`), and `setting_words_match_the_notices` now loops over
   `SettingKey::ALL`. **`crates/wakeshot-tray/tests/e2e_real.rs`** (not my file): one `unreachable!` arm in
   `Values::of` (those runs check only the hold keys). Both are build fixes a contract change forces.
2. **`app/tray_mode.rs`**: changed for more than the suspend callback the orchestrator named: the runner of the new
   thread command (`start_sleep_check`, `Hibernator::start_check`, one test). The callback route needed nothing:
   `power::system` already posts `Suspending` and `Resumed` from it.
3. **`"sleepAc"` as a test's unknown key.** Eleven test files used the journal name `"sleepAc"` as "a key from a newer
   version"; it is a known key now, and two of them parse it from text, so they would test the wrong thing (the CLI's
   `restore.rs` test would fail). Renamed to `"hibernateAc"` in tests only: mine (`ownership/{apply,check,journal,
   props,restore}.rs`, `power/{executor,journal,stop_points}.rs`) and not mine (`wakeshot-cli/src/restore.rs`,
   `wakeshot-windows/src/hooks.rs`, `wakeshot-windows/src/recovery.rs`). No product code changed there.
4. **A lost journal while the 1-minute timeouts are in force leaves them.** Product rule 7 resets only lid Do nothing
   and sleep Never. A crash in the minutes the short timeouts are written, then a journal that never reads, leaves
   screen and sleep at 1 minute until the user sets them (rare; written in `architecture.md` section 5 "Gap left").
   Extending the reset to the short keys at exactly 60 s is one key list and the notice words; for the orchestrator
   and Haik (product rule 7 is his).
5. **`watch::agents::tests::a_folder_that_cannot_be_listed_reads_its_known_files` fails in this shell** (twice, the same
   way): the listing that the test denies by ACL is not denied (`left: Ok([...])`, `right: Err(Some(5))`). Not my file
   and nothing of this card touches it; most likely the elevated agent shell (CLAUDE.md: a test that expects a denial
   must run unelevated). `p2-6-record-lost` saw 0 failures, so it may depend on how the session was started.
6. **README** (`README.md` lines 230 to 236) already describes this route and the notice "The PC did not go to sleep"
   matches the new words; its comment asks to check it against this Result. One detail it does not say: the screen
   also turns off after 1 minute, and the user's timeouts come back at input or an unlock.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 11:20: worker started; read the card, product rules, both earlier cards' Results.
- 2026-11-04 11:25: read the code (`core/lid.rs`, `core/settings.rs`, `core.rs`, `state.rs`, `command.rs`, `event.rs`,
  `settings.rs`, `policy/ending.rs`, `policy/lock.rs`, `ownership/apply.rs`, `ownership/fake.rs`, `text.rs`,
  `power/hibernate.rs`, `power/plan.rs` outline, `app/dispatch.rs` send, `app/tray_mode.rs` Hibernator). Design:
  countdown due sends a new `SleepCheck` (the hibernate thread's checks and sound, no hibernate); its `Ok` sends an
  `Apply` of the four short keys at 60 (the apply routine skips a key already at or under 60 and not Never); a state
  `short` holds the fallback deadline (5 min of run time, set again at a resume or a sleep seen by the clock gap);
  input (raw idle going down), an unlock, the lid open, a hold start or the countdown ending restore them
  (`RestoreWhy::ShortTimeouts`). Needs `app/tray_mode.rs` (the runner of the new thread command) and two build-only
  arms outside my files (CLI `setting_words`, tray `e2e_real.rs`), since `SettingKey` grows.
- 2026-11-04 11:47: core done: `settings.rs` (three keys, `HOLD`, `SHORT`, `SHORT_TIMEOUT_S`), `command.rs`
  (`SleepCheck`, `Outcome::SleepCheck`, `RestoreWhy::ShortTimeouts`), `state.rs`, `core/lid.rs`, `core/settings.rs`,
  `core.rs`, `policy/{ending,lock}.rs`, `ownership/{apply,fake,props}.rs` (the ceiling rule `apply::meets`), `text.rs`
  (two notices, three key words), `sim.rs`, scenarios (lid: helpers now name the sleep check; 26 new or rewritten for
  the sleep route; props: an apply with no hold may be the sleep route's with the lid closed; the World opens the lid
  at a `LastHold` end). Props found two real gaps, fixed: no short write while the session ends, and a key the user
  released in a stepped-back episode stays theirs. Added an unlock clamp: the countdown counts idle from the last
  unlock too (a face sign-in gives no input). `"sleepAc"` was the test name of an unknown key in eleven test files; now
  `"hibernateAc"`. `cargo test -p wakeshot-core --lib`: 647 passed. Next: Windows side.
- 2026-11-04 11:57: Windows side done: `power/plan.rs` (`PlanSetting::VideoIdle{Ac,Dc}` with
  `GUID_VIDEO_SUBGROUP`/`GUID_VIDEO_POWERDOWN_TIMEOUT`, checked with `powercfg /aliases`, read only), `power/hibernate.rs`
  (`run_sleep_check`), `app/dispatch.rs`, `app/io_thread.rs`, `app/tray_mode.rs` (`Hibernator::start_check`),
  test arms in `power/{executor,stop_points}.rs`, `app/startup.rs`; `examples/power_check.rs` `Timed` forwards
  `read_default`; build-only arms in `wakeshot-cli/src/status.rs` and `wakeshot-tray/tests/e2e_real.rs`.
  `examples/ux_all.rs` (orchestrator note) builds unchanged. fmt (my files only, by `rustfmt`), clippy x64 and ARM64
  clean. Full test run: one failure not mine (`watch::agents`, Found outside scope 5).
- 2026-11-04 12:03: docs: `docs/architecture.md` sections 2, 3 ("The sleep route", the window lock, clocks), 4, 5
  (keys table, hibernate thread, restore step 1, "Gap left"), 6 (journal keys), 8, 9; `docs/research.md` new section
  "The screen timeout setting". Core props at `PROPTEST_CASES=4096`: 223 passed, 50 s. Hand mutants started.
- 2026-11-04 12:11: hand mutants (12:04 to 12:11): 13 of 13 killed. Added the window lock checks to the window test and
  a docked window test; final checks (Result). Scratch deleted, Status done.

## Result

**Done (2026-11-04).** Contract changes: `SettingKey::{SleepAc, VideoIdleAc, VideoIdleDc}` (with `HOLD`, `SHORT`,
`ALL`, `is_timeout`, `SHORT_TIMEOUT_S`), `Command::SleepCheck`, `Outcome::SleepCheck`, `RestoreWhy::ShortTimeouts`,
`ownership::apply::meets`, `power::hibernate::run_sleep_check`, the runner method `LoopRunners::start_sleep_check`.

### What was done, by deliverable

1. **Keys.** Journal names `sleepAc`, `videoIdleAc`, `videoIdleDc`; written, journaled and restored by the same
   routines as the hold keys (RunOnce, guard, restore mode, hooks and CLI all go through them). An older journal
   loads as before (`files` test `the_sleep_route_keys_round_trip_and_an_older_journal_still_loads`).
   `power::plan` reads and writes the screen timeout through `GUID_VIDEO_SUBGROUP`/`GUID_VIDEO_POWERDOWN_TIMEOUT`.
2. **The step after the countdown** (`core/lid.rs`). Due: `SleepCheck` (the hibernate thread's lid, idle and floor
   checks, the going to sleep sound, the lid once more; no hibernate). `Ok`: `Apply` of sleep after and screen off, AC
   and DC, at 60 s. The apply writes a key only where the value in force is longer or Never (the ceiling rule), as
   Wakeshot entries. The same for the floor countdown and the failed-lock step back (both use this countdown).
3. **Fallback.** 5 minutes of run time after the write (again after each resume): `Hibernate` as before, its checks
   unchanged. A sleep is `Suspending` then `Resumed`, or `Suspending` and a tick whose wall clock moved 5 s or more past
   run time when the `Resumed` post was lost. Hibernate not allowed, failed or lid unknown: `HibernateFailed` (new
   words), the relock, no second hibernate in this countdown; the short timeouts stay.
4. **Restore** (`RestoreWhy::ShortTimeouts`): at an unlock, input (a reading under the one before; then 5 minutes
   again), the lid open, a hold start (before its apply), the countdown's end (AC back for Floor, a pause, a resumed
   hibernate) and every other restore point through the journal. Not at a wake. A user's change to a short key wins
   and is not written again in that countdown. A plan switch moves them.
5. **Words** (`text.rs`): `HibernateFailed` "The PC did not go to sleep" / "The lid is closed, but the PC did not
   sleep and could not hibernate. Windows should sleep it once nothing else keeps it awake. Opening the lid puts your
   screen and sleep settings back."; `LockFailed` ends "makes it sleep after 5 minutes with no input" (200
   characters); `UserChanged` words for the three keys (also in the CLI).
6. **Tests.** Core: 22 lid scenarios new or rewritten for the route (write, sleep seen, clock gap, fallback, lid open,
   unlock, input, a wake, a hold that starts, a user change, floor, failed lock, not writable, failed write, plan
   switch, each sleep check and fallback skip), 8 earlier ones now reach the fallback first, the window test checks the
   lock and a new one checks no lock on the user's own Do nothing; props: an apply with no
   hold may only be the sleep route's with the lid closed, and the World opens the lid at a `LastHold` end; apply: the
   ceiling, a short apply, a hold taking it over, a leftover Never under the user's 30 s, a stop at every call of a
   short apply then a restore. Windows: `run_sleep_check` (4), the thread (`tray_mode`), dispatch routing and the
   countdown test, io_thread strays and restore reasons, plan GUIDs. Journal round trip with `sleepAc` (`files`).
7. **`docs/architecture.md`**: sections 2, 3 ("The sleep route", the window lock, clocks), 4, 5 (keys table, ceiling,
   hibernate and sleep check thread, restore step 1 and "Gap left" for a damaged journal), 6 (journal keys), 8 (two
   rows), 9.
8. **Window lock** (`Core::unknown_journal_ended`, `settle_window_lock`): a lid close inside the unknown window with a
   hold holding sets a wait; once a read shows the user's own lid action as not Do nothing, `Lock` with the chime, as
   at a lid close. Tested in `a_hold_in_the_window_runs_on_requests_and_counts_down_when_it_ends`.
9. **`power_check.rs`**: `Timed` forwards `read_default`, logged like its other calls.

### Decided by the worker, for Haik's review

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | The sound and checks run on the hibernate thread (`SleepCheck`), then the core writes through the existing apply | The write gets the journal, RunOnce and guard for free; the sound ends before the write, so a sleep at once cannot cut it | A test shows the write must come with the sound |
| 2 | A timeout target other than Never is a ceiling in the apply; a hold's leftover Never over the user's shorter value goes back to that value | "Only where the user's own is longer or Never", decided on the live value at the write | Never likely |
| 3 | The fallback counts run time from the write and again from each resume | A laptop that wakes with the lid closed gets the same 5 minutes to sleep again | A log shows wakes that never sleep again |
| 4 | Input is a raw reading under the one before, not the clamped idle | A resume is not input; readings come every 30 s or less, far under 5 minutes | Readings ever come further apart |
| 5 | Any unlock puts them back, not only the first after a wake | An unlock means the user is there | Never likely |
| 6 | The countdown counts idle from the last unlock too | A face sign-in may give no input Windows counts; the user would hear the sound and lose the screen in a minute | A test shows Hello sign-in counts as input |
| 7 | A failed fallback no longer ends the countdown: no second hibernate or notice in it, and after input the short timeouts come back (with the sound) | The laptop still ends where Windows would put it | Haik prefers the old end |
| 8 | A hibernate that resumed ends the countdown and puts them back | A resume from hibernate is a deliberate wake | Never likely |
| 9 | Not writable (another session owns settings, policy, session ending, every key the user's): plain hibernate at once; a failed write: the fallback at once | Nothing else can sleep the laptop | Never likely |
| 10 | One blocked key (policy on the screen timeout) blocks the whole short write | The apply is all or nothing by design | A company PC report |
| 11 | A sleep check thread that cannot start answers `Ok` | The core checked lid, idle and floor; only the sound is lost | Never likely |
| 12 | An unknown lid in the sleep check goes on | The core's own reading says closed, and the timeouts go back at lid open | A test shows a wrong lid state |
| 13 | A key the user changed in a stepped-back episode stays theirs in the floor route | "Until the current work ends" (rule 3); found by the props | Never likely |
| 14 | No short write while the session ends | The end-of-session restore must be the last word; found by the props | Never likely |
| 15 | Deliverable 8's lock waits for a read showing the user's own lid action as not Do nothing | The copy may still show the old Do nothing after the reset; with their own Do nothing no lock, as at a lid close | Never likely |
| 16 | The fallback hibernate plays the going to sleep sound again | The thread is unchanged; it is the hint right before the PC goes down | Haik finds two sounds too many |
| 17 | The lost-journal reset stays on the hold keys | Product rule 7 names those; Found outside scope 4 | Haik extends rule 7 |

### Files changed

`crates/wakeshot-core/src/`: `settings.rs`, `command.rs`, `event.rs`, `files.rs`, `state.rs`, `core.rs`,
`core/lid.rs`, `core/settings.rs`, `policy/ending.rs`, `policy/lock.rs`, `text.rs`, `sim.rs`,
`ownership/{apply,fake,props,check,journal,restore}.rs`, `scenarios/{lid,props}.rs`.
`crates/wakeshot-windows/src/`: `power/{plan,hibernate,executor,journal,stop_points}.rs`, `app/{dispatch,io_thread,
tray_mode,startup}.rs`, `hooks.rs` and `recovery.rs` (test key name only); `examples/power_check.rs`.
Outside my list (Found outside scope 1, 3): `crates/wakeshot-cli/src/{status,restore}.rs`,
`crates/wakeshot-tray/tests/e2e_real.rs`. Docs: `docs/architecture.md`, `docs/research.md`, this card.
`notify.rs` needed nothing (no notice variant changed); `examples/ux_all.rs` builds unchanged.

### Checks (2026-11-04, build folder `<scratchpad>\p2-6-sleep-route\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (my files formatted by `rustfmt --edition 2024` only, so no other card's file was touched) |
| clippy x64 `--workspace --all-targets -D warnings`, and without `--all-targets` | both exit 0 |
| clippy ARM64 `-p wakeshot-core -p wakeshot-windows -p wakeshot-cli`, with and without `--all-targets` | both exit 0; the tray's ARM64: CI only |
| `cargo test --workspace --locked` (debug) | exit 101 only for `watch::agents::tests::a_folder_that_cannot_be_listed_reads_its_known_files` (Found outside scope 5, not mine); with that one skipped and `--no-fail-fast`: exit 0, 1340 passed (cli 75 + 3 + 5, core 648 + 11 + 25, tray 8 + 5, windows 560) |
| Core scenario props at `PROPTEST_CASES=4096` | 223 passed, 1 ignored, 50 s |
| Hand mutants on the new core lines (scratch copy, own build folder, one build job, one test thread, stop by PID at 4 GB, under 10 GB free, or 240 s) | 13 run, 13 killed (input, session end, episode releases, no fallback, clock gap, rearm at resume, unlock, Never and the ceiling, the release under it, countdown end, a second fallback, window lock, unlock clamp); peak test process 0.02 GB, none stopped, free memory 34.2 GB after. Windows-side lines: no mutants (routing and thread code, each with its own tests) |
| Haik's plan never written; no tray started | yes: only fakes and `DryRunPower`; one read-only `powercfg /aliases` |
| Build folder deleted | yes: build folders, the scratch copy and my scripts deleted at the end (checked gone) |

### Open items

- For the orchestrator: Found outside scope 1 to 6. Product lines: `product.md` work end says the timeouts go back
  "at the first unlock after a wake"; built: at any unlock (decision 5). Rule 7 does not cover the 1-minute timeouts
  (Found outside scope 4).
- Not tested on the laptop: a real write of the screen timeout, and how soon Windows sleeps after it (`p6-int-haik-checks`).
