# A journal that cannot be read: try for 2 minutes, then reset Wakeshot's own values to Windows defaults

Phase: 2 to 6 (core and power). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens.
Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from Haik's answer of 2026-11-04 (`docs/product.md`,
"User settings safety" 7). It replaces `p6-int-wiring-fixes` decision 7 (a lock on a Do nothing that may be ours
while the journal is unknown).
Runs after: nothing. Parallel with: `p3-idle-sleep-check` (laptop only, no shared files). Not with
`p2-6-sleep-route` (same files; it runs after this card).
Real laptop: no (reads of the real plan's defaults are fine; writes only to a plan copy, never Haik's plan).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-record-lost\target`.

## Goal

Haik's rule: with no work going on, the laptop behaves as if Wakeshot were not there. A journal that cannot be read
most likely means Wakeshot's values are still in force and their originals are lost. So, after a short retry, put
every value that looks like Wakeshot's back to Windows' default, tell the user once, and log it. Haik's words: "put it
to windows defaults and notify the user and log the error".

## Read first

- `docs/product.md`: "User settings safety" 1 to 7, the lid rules ("Never lower security", work end).
- `docs/tasks/p6-int-wiring-fixes.md` Result item 5 and decision 7; `docs/tasks/p2-6-unread-journal.md`,
  `p2-6-unread-lid-countdown.md`, `p2-6-unread-floor-countdown.md` (Result and "Found outside scope").
- `crates/wakeshot-core/src/core.rs` (start, `Leftover` retry), `core/settings.rs` (restore results, `retry`,
  `RestoreFailed`), `core/lid.rs` (`journal_unknown`, `lock_view`, the waited countdowns), `state.rs`, `command.rs`,
  `event.rs`, `text.rs` (`JournalCorrupt`, `RestoreFailed`).
- `crates/wakeshot-windows/src/power/journal.rs` (load: missing, corrupt, failed; the `.corrupt` set-aside),
  `power/plan.rs`, `power/executor.rs`.
- `docs/architecture.md` sections 3 and 4 (grep "unread", "corrupt", "journal").

## Deliverables

1. Lost journal, in the core. A journal load that fails (any read error) and keeps failing on each retry for
   2 minutes from the first failure (core clock, not tick count), or a load that finds the file damaged, makes the
   journal "lost". Then the core sends one command (new, in `command.rs`) to reset the values that look like
   Wakeshot's, and treats the journal as known and empty once the result comes back.
2. The reset, in `wakeshot-windows`. On the plan in use, for each key Wakeshot writes (`LidAc`, `LidDc`, `SleepDc`):
   if the value now is exactly the value Wakeshot writes (lid Do nothing, sleep Never), write the plan's Windows
   default (`PowerReadACDefaultIndex` or `PowerReadDCDefaultIndex`; check the current Microsoft docs), then
   `PowerSetActiveScheme`; any other value is the user's: leave it. Log each key with its old and new value, or why it
   was left. Set the bad file aside the way a damaged one is set aside today; if that fails, log it and go on (the
   next save replaces the file). `DryRunPower` does the same on its copy.
3. Notice. One notice, only when at least one value was reset (nothing reset: log only, no notice). Draft words
   (the worker may improve them within `text.rs` rules): title "Power settings set to Windows defaults", body
   "Wakeshot could not read its record of your power settings, so it put the lid and sleep settings that looked like
   its own back to Windows defaults. If you had your own, set them again. Click to see the old values in the log."
   It replaces `JournalCorrupt` (a damaged file now takes the same path). Click opens the log.
4. The unknown window. While the journal is unknown (before it reads or is lost), no lock at a lid close and no
   countdown: remove the `p6-int-wiring-fixes` item 5 lock guess and keep the countdown waits. When the journal
   becomes known or lost and the lid is closed with nothing holding and the user's lid action (after any reset) not Do
   nothing, start the same countdown a work end with the lid closed starts (today it ends in a hibernate;
   `p2-6-sleep-route` turns it into sleep).
5. A hold that starts while the journal is unknown: keep what is built today, and say in the Result what happens (one
   paragraph); change it only if it breaks rule 7.
6. Tests first: core scenarios (retry then lost at 2 minutes; damaged file lost at once; a read that works at 1:59
   ends it; reset of only Wakeshot-looking values; no notice when nothing reset; no lock in the window; the countdown
   at the end with the lid closed; a user's own Do nothing reset and noticed); Windows unit tests with the fakes; one
   real read of Balanced's defaults (read only); a real reset on a plan copy only if the harness makes it easy.
7. `docs/architecture.md`: the lines of sections 3 and 4 that state these rules.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64 and ARM64 (the tray's ARM64 needs clang: CI only); `cargo test --workspace --locked` (debug
  only, never `--release`); the core props at 4096 cases once.
- Mutants only with one job, a low timeout and a memory cap (a mutant once grew to 35 GB); stop if free memory drops
  under 10 GB; skip and say so if you cannot cap it.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The sleep route and `SettingKey::SleepAc` (`p2-6-sleep-route`). Keys other than the three. Other plans than the
  plan in use (say in the Result if a lost journal can leave Wakeshot values on another plan, and how likely).
- `README.md`, phase files, `docs/product.md` (the orchestrator owns them; tell it if a product line is wrong).

## Questions for Haik

None (his answer is in `docs/product.md`, "User settings safety" 7). Decide the rest yourself and record each choice
as "decided by the worker, for Haik's review".

## Found outside scope

1. **`crates/wakeshot-windows/examples/ux_notices.rs` changed (2 lines), not in my file list.** Its `name` match has
   no wildcard on purpose, so replacing `Notice::JournalCorrupt` (a contract change the card asks for) does not build
   without it: the arm is now `JournalLost { .. } => "journal-lost"` and the sample has all three keys. Nothing else
   there changed. The `--one journal-corrupt` name is gone.
2. **`examples/power_check.rs` `Timed<T>` does not forward `PlanIo::read_default`.** It builds (the trait method has a
   default body), but a harness reset through it would answer "not supported" and reset nothing. No harness flow sends
   `ResetLost` today. Fix when one does: one forwarding method, like its other ones. `recovery.rs`'s test fake `Flaky`
   is the same and needs nothing.
3. **`docs/architecture.md` sections 5 and 8 still say a corrupt journal's values stay.** Section 5, "Restore routine"
   step 1 ("values stay as they are. That override then stays until the user sets the values by hand") and the
   section 8 table row "Journal corrupt | moved aside; warning notice; values left as they are". True for the guard,
   restore mode, the hooks and `wakeshot restore`, which only set the file aside; but the tray now resets at its next
   start (or at once for its own restore). Not my sections.
4. **The uninstall hook can leave a corrupt journal's values.** The hooks set a corrupt file aside and no tray starts
   after an uninstall, so no reset follows. Rare (a corrupt journal at the moment of uninstall). Same shape for the
   guard and restore mode, but there the tray's next start resets.
5. **`docs/product.md` line check.** Rule 7 fits what is built. Two details the orchestrator may want in it: the
   2 minutes are run time (a sleep pauses them), and Windows' default for battery sleep on this laptop is 3 minutes
   (Microsoft's newer Modern Standby default), so a user whose own battery sleep was Never meets 3 minutes after a lost
   journal. Its "up to about 7 minutes" holds with nothing holding (2 minutes, then the 5-minute countdown); with a hold
   that ran through the window the closed laptop runs until the hold ends plus 5 minutes (Result, deliverable 5).

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 10:20: worker started; reading the card, product rule 7 and the earlier unread cards.
- 2026-11-04 10:24: read the code (`core.rs`, `core/settings.rs`, `core/lid.rs`, `state.rs`, `command.rs`, `event.rs`,
  `text.rs`, `ownership/{restore,io,fake}.rs`, `power/{journal,executor,plan}.rs`, `app/{io_thread,dispatch}.rs`,
  `sim.rs`). Microsoft docs: `PowerReadDCDefaultIndex` takes the scheme *personality* GUID, not the scheme GUID.
  Registry on this laptop (read only): `DefaultPowerSchemeValues` is keyed by the three personality GUIDs; a plan's
  personality is the hidden setting `245d8541-...` (`GUID_POWERSCHEME_PERSONALITY`, index 0 power saver, 1 high
  performance, 2 balanced; Balanced reads 2). Plan: the reset as a core ownership routine over the seams (like
  restore), `PlanIo::read_default` with a default body so the harness example and the recovery test fake still build.
- 2026-11-04 10:28: design set. Core: `Command::ResetLost { damaged }` / `Outcome::ResetLost { reset, result,
  journal }`; the journal is unknown while a retry is wanted with an empty copy, or a reset is out; a tick 2 minutes
  (run time) after it became unknown sends the reset instead of the retry; `JournalCorrupt` sends one at once
  (`damaged: true`). The routine loads the file once more under the lock: known entries mean it read after all (no
  reset); missing or no known entry means nothing to reset unless `damaged`. No `RestoreFailed` while unknown. The
  unread lock guess goes; at the window's end the countdown hook sets the waiting want. `Notice::JournalLost` replaces
  `JournalCorrupt` (so `examples/ux_notices.rs`, not in my list, needs its two lines: a contract change must build).
- 2026-11-04 10:40: core done. New `ownership/reset.rs` (routine and 9 tests over the fake; the fake gained
  `ReadDefault`, `defaults` and the `SetBy::Default` label), `command.rs`, `view.rs` (`JournalLost`), `text.rs`,
  `state.rs`, `core.rs`, `core/settings.rs`, `core/lid.rs` (guess removed, hook `Core::unknown_journal_ended`),
  `sim.rs` (the World runs the reset; its `Leftover` now answers an unreadable file as the executor does and sends a
  corrupt one to the routine), scenarios (settings 7 new, lid 6 new or rewritten, props model and 3 named World
  cases). `cargo test -p wakeshot-core --lib`: 624 passed, 1 ignored. Next: the Windows side.
- 2026-11-04 10:47: Windows side done: `power/plan.rs` (`read_default` through the plan's personality, forwarded by
  `DryRunPower` and `CopyAsActive`), `power/journal.rs` (`reset_locked`, a log line per key; the set-aside line no
  longer says the values stay), `power/executor.rs` (runs `ResetLost`, marks the set-aside file seen),
  `power/stop_points.rs` (forwards), `app/io_thread.rs`, `app/dispatch.rs`, `notify.rs` (test), and the two lines in
  `examples/ux_notices.rs`. `cargo test -p wakeshot-windows --lib -- power:: app:: notify::`: 220 passed. fmt clean;
  clippy x64 with and without `--all-targets`: exit 0. Next: docs, then the full checks.
- 2026-11-04 10:55: real tests, once each (`--ignored`): the first run failed on my oracle: `PowerReadDCDefaultIndex`
  gives Balanced's battery sleep default as 180, the registry's `DefaultPowerSchemeValues` table 900. Microsoft's
  "Power settings in Windows 11" page: Modern Standby battery sleep default is now 3 minutes. So the API is right and
  the tests now expect 180 (lid keys still match the table). Rerun: both pass. Plan copies gone, Haik's Balanced read
  the same before and after (lid 1/1, sleep 18000/18000). Fake default sleep set to 180 to match. `docs/research.md`
  new section "Windows defaults of the three keys"; `docs/architecture.md` sections 3 and 4 done.
- 2026-11-04 11:02: checks: `cargo fmt --all --check` exit 0; clippy x64 and ARM64 (core, windows, cli), with and
  without `--all-targets`: exit 0; `cargo test --workspace --locked`: 1312 passed, 0 failed; core scenario props at
  `PROPTEST_CASES=4096`: 207 passed, 34 s. Hand mutants started on a scratch copy (`<scratchpad>\p2-6-record-lost\
  mutants.ps1`: one build job, one test thread, the test process stopped by its PID at 4 GB, under 10 GB free, or
  after 150 s; results to `mutants.txt`).
- 2026-11-04 11:06: mutants, first run (11:03 to 11:06), 14 on the changed lines plus the unchanged base: 13 killed,
  each test process under 0.03 GB, none stopped, free memory 32.5 GB after. Survivor M2 ("a reset out does not count
  as unknown"): the damaged path still worked, because the hook then fires when the reset goes out and the want waits
  for a read anyway. It showed a real gap: a damaged reset whose answer is lost was sent again as `damaged: false`,
  which finds no file and resets nothing. Fixed: `State::resetting` is now `Option<bool>` (the `damaged` of the reset
  out), the resend asks the same, and `a_reset_whose_answer_never_comes_goes_out_again` gained the damaged case.
  Found on the way: the scratch copy shared my build folder, and Cargo hashes workspace members by their path inside
  the workspace, so the last mutant's build stood in for the real tree's (6 reset tests failed on correct code).
  Cleaned the four workspace crates; core tests pass again. The earlier full checks ran before the mutants, on clean
  builds. M2 is checked again with its own build folder (`mutants-target`).
- 2026-11-04 11:10: M2 again (11:08 to 11:10, base plus M2, own build folder): killed by
  `a_reset_whose_answer_never_comes_goes_out_again`. All checks rerun after the fix (Result). Scratch folder (build
  folders, the copy, the script) deleted and checked gone 11:11; plan list only Balanced; free memory 33.1 GB.
  Status done.

## Result

**Done (2026-11-04).** Contract change in this card: `Command::ResetLost`, `Outcome::ResetLost`, `Notice::JournalLost`
(replaces `JournalCorrupt`), `PlanIo::read_default` (with a default body), new module `ownership::reset`.

### What was done

1. **Lost journal, core** (`core/settings.rs`, `state.rs`). The journal is unknown while a restore retry is wanted
   with an empty copy, or a `ResetLost` waits for its answer (`Core::journal_unknown`, moved from `core/lid.rs`). The
   settle pass keeps when it became unknown (`unknown_since`) and sets a deadline 2 minutes of run time later; that
   tick sends `ResetLost { damaged: false }` in place of the retry, and no retry goes out while a reset is out.
   `JournalCorrupt` sends `ResetLost { damaged: true }` at once. The answer (`done_reset`): known entries mean the file
   read after all, so the copy takes them and the retry goes on; otherwise the journal is known and empty (retry
   cleared), one `JournalLost` notice when a key was reset or a call failed, the log only when nothing was reset, then
   a `ReadPlan`. A failed restore while unknown shows no `RestoreFailed`. A reset whose answer never comes goes out
   again 2 minutes later, with the same `damaged` (`State::resetting` keeps it).
2. **The reset** (`ownership/reset.rs`, called through the new `power::journal::reset_locked` under the journal
   mutex; `power/executor.rs` runs it). It loads the file once more: known entries, nothing reset; missing or no known
   entry, nothing reset unless `damaged`; still unreadable or corrupt, the reset runs. On the plan in use, each of
   `LidAc`, `LidDc`, `SleepDc` at exactly 0 gets the plan's Windows default, then `PowerSetActiveScheme` once anything
   was written; any other value is kept; a default that is 0 itself is kept. The bad file goes aside after the
   writes; a failed move is logged and does not fail the reset. The executor marks the set-aside file seen, so the next
   start does not reset again. One log line per key with old and new value, or why it was kept, and a summary line.
   The default is `PowerRead{AC,DC}DefaultIndex` of the plan's *personality* (the hidden setting
   `GUID_POWERSCHEME_PERSONALITY`, index 0, 1, 2 mapped to the three GUIDs; research "Windows defaults of the three
   keys"). `DryRunPower` reads the real default and writes only its overlay; `CopyAsActive` and the stop-point wrapper
   forward it.
3. **Notice** (`text.rs`, `view.rs`). Title "Power settings set to Windows defaults", body "Wakeshot could not read its
   record of your settings, so it set {the lid setting | the battery sleep setting | the lid and battery sleep
   settings} to {the Windows default | Windows defaults}. If you had your own, set {it | them} again. Click to see the
   old values in the log." (at most 195 characters). When a call failed: "Check your lid and sleep settings" / "Wakeshot
   could not read its record of your settings, and could not set them all back to Windows defaults. Set the lid and
   sleep settings by hand; click to see the old values in the log." Warning, click opens the log, kept for the user's
   return when shown to a closed laptop.
4. **The unknown window** (`core/lid.rs`). The lock guess of `p6-int-wiring-fixes` item 5 is gone: while unknown there
   is no lid-close lock and no source lock (an apply in flight still counts as ours, as before). The countdown waits
   stay. The countdown hook is **`Core::unknown_journal_ended`** (`core/lid.rs`), called by `Core::settle_journal`
   (`core/settings.rs`) in the settle pass, before `settle_lid`, when the journal stops being unknown (known or lost):
   with the lid closed and no episode it sets the work-end want waiting (`work_end_waits`), so the next plan read that
   shows the user's own lid action as not Do nothing starts the same countdown a work end starts; with a stepped-back
   episode it sets the `Floor` want waiting the same way. `p2-6-sleep-route` can change what the countdown does
   without touching the hook.
5. **A hold that starts while the journal is unknown** (kept as built; one paragraph as asked). Its requests go on at
   once. Its apply loads the journal too, fails at the safety net, and the episode holds with requests only
   (`RequestsOnly` notice); nothing is restored for it. A lid close in the window neither locks nor counts down, so if
   the lid action in force is Do nothing (a crashed tray's, or the user's own) the closed laptop runs unlocked on the
   requests. At 2 minutes the reset puts the Do nothing back to the default, but the lid is already closed, the hold
   still holds, and no lock comes after the window, so the laptop runs unlocked until the hold ends; then the work-end
   rule counts down from the user's own action and it hibernates 5 minutes after the last input (sleeps once
   `p2-6-sleep-route` lands). This does not break rule 7, which speaks of the 2 minutes and of nothing holding; it is
   longer than its "about 7 minutes". Tested in `a_hold_in_the_window_runs_on_requests_and_counts_down_when_it_ends`.
   For Haik's review: a lock at the window's end when the lid is closed and a hold runs would close it (one line in the
   hook); I left it because the card says to keep what is built.
6. **Tests** (new or rewritten): `ownership::reset` 9 (order of calls, corrupt, missing with and without `damaged`,
   read after all, nothing at Wakeshot's value, a default that is 0, each failed call, a failed set-aside, a stop at
   every call then a second run); `scenarios::settings` 7 (lost at 2:00 to the millisecond on the core's clock and off
   the tick grid, a read at 1:59 ends it, nothing reset gives no notice, damaged lost at once at start and in a run,
   read after all keeps the retry, a failed reset warns once, an answer that never comes, also for a damaged file);
   `scenarios::lid` 6 (no lock
   in the window, entries decide once read, lost with the lid closed counts down after the reset and the notice comes
   again at the open laptop, a damaged file with the lid closed, a Do nothing still in force gives no countdown, a hold
   in the window); `scenarios::props` model and properties updated (no guess excuse; no lock and no stretch in the
   window; no `RestoreFailed` while unknown; the reset's notice expected) and 3 named World cases through the real
   routines (a crashed tray's unreadable journal ends at the defaults; the user's own battery Do nothing reset with the
   notice; a damaged leftover). Windows: `power::journal` (log lines, a failed set-aside, a busy mutex),
   `power::executor` (answer, set-aside marked seen, `damaged`), `power::plan` (personality map, dry-run reset; real:
   Balanced's defaults read, a reset on a plan copy), `app::io_thread` and `app::dispatch` (routing), `notify`, `text`.
7. **Docs**: `docs/architecture.md` section 3 (event, command and outcome blocks, the notice list, state, the lid lock
   and countdown lines, "may be owned", a new paragraph "A journal that cannot be read") and section 4 (startup steps 6
   and 8); `docs/research.md` new section "Windows defaults of the three keys".

### Decided by the worker, for Haik's review

Haik's rule for open calls: with no work going on, the laptop behaves as if Wakeshot were not there.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | The 2 minutes are run time from the moment the copy became unknown, so a sleep pauses them | The window is there for tries; a laptop asleep makes none | Haik wants wall time |
| 2 | "Unknown" means an empty copy with a retry wanted, or a reset out; a failed load while the copy knows entries keeps today's retry and `RestoreFailed` | The core cannot tell a failed load from other failures, and with a known copy the lid rules know what is ours | A log shows a known journal unreadable for minutes: then count it too, and restore from the copy's originals rather than defaults |
| 3 | The reset loads the file once more under the mutex and resets nothing when it reads, or is gone (unless `damaged`) | Only that run sees the truth; a file restored elsewhere, or our own apply's fresh journal, must not be reset | Never likely |
| 4 | A failed reset is not retried; the journal counts as known and empty, with the "Check your lid and sleep settings" notice | The card's rule; a failed power call (policy) would fail again | A log shows a failed reset: retry it on the 2-minute clock |
| 5 | No `RestoreFailed` while the journal is unknown | Rule 7 says one notice; the window ends in a read (silent, as before) or the reset (its own notice) | Users miss a word during the 2 minutes |
| 6 | Notice `JournalLost { reset, failed }`, a warning, the two lid keys named as one "lid setting", kept for the user's return | Says what was reset within the 200-character English limit; a closed laptop's screen may show it to nobody | Translations need more room |
| 7 | The hook also covers a stepped-back hold with the lid closed (the `Floor` want) | "Nothing holding" includes a hold the battery floor stopped | Never likely |
| 8 | The default comes from the API (Balanced battery sleep 180 s here), not the registry table (900) | The API gives Windows' current default (Microsoft's new Modern Standby default is 3 minutes) | Microsoft documents another source |
| 9 | The plan's kind from the hidden setting's index (0, 1, 2) mapped to the three GUIDs; any other index fails | Verified on this laptop; `PowerReadPossibleValue` would read the same bytes with more code | A PC shows another index |
| 10 | `PlanIo::read_default` has a default body that answers "not supported" | Implementors outside my files (`power_check.rs`, `recovery.rs`'s test fake) still build; the failure is safe (nothing written) | `Timed` in `power_check.rs` forwards it (Found outside scope 2) |
| 11 | The bad file is set aside even when a write failed; RunOnce and the guard are left alone | So the next apply can write a fresh journal; the restore routine's own set-aside does the same | Never likely |
| 12 | The fake labels a write of the plan's default with no journal entry `SetBy::Default` | The World's properties must tell a reset from an override | A routine writes a default with no entry on purpose |

### Files changed

`crates/wakeshot-core/src/`: `command.rs`, `event.rs`, `view.rs`, `text.rs`, `state.rs`, `core.rs`, `core/settings.rs`,
`core/lid.rs`, `sim.rs`, `ownership/mod.rs`, `ownership/io.rs`, `ownership/fake.rs`, `ownership/props.rs`,
`ownership/reset.rs` (new), `scenarios/settings.rs`, `scenarios/lid.rs`, `scenarios/props.rs`.
`crates/wakeshot-windows/src/`: `power/plan.rs`, `power/journal.rs`, `power/executor.rs`, `power/stop_points.rs`,
`app/io_thread.rs`, `app/dispatch.rs`, `notify.rs`; `crates/wakeshot-windows/examples/ux_notices.rs` (Found outside
scope 1). `docs/architecture.md`, `docs/research.md`, this card.

### Checks (2026-11-04, build folder `<scratchpad>\p2-6-record-lost\target`, no `RUSTFLAGS`, all `--locked`)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 (run before and after the M2 fix) |
| `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings`, and without `--all-targets` | both exit 0 (after the fix) |
| ARM64: `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli` with and without `--all-targets` | both exit 0 (after the fix); the tray's ARM64: CI only (clang) |
| `cargo test --workspace --locked` (debug) | exit 0 (after the fix): cli 75 + 3 + 5, core 624 + 11 + 25, tray 8 + 5, windows 556; 1312 passed, 0 failed |
| Core scenario props at `PROPTEST_CASES=4096`, once after the fix | 207 passed, 1 ignored, 36 s; free memory 32.8 GB after |
| Real, read only: `real_defaults_of_the_active_plan_read` | passed (lid AC 1, lid DC 1, battery sleep 180; personality Balanced) |
| Real, plan copy only: `real_reset_on_a_copy_puts_the_balanced_defaults` | passed (lid AC 0 to 1, lid DC kept, sleep DC 0 to 180; Windows never switched plans) |
| Haik's plan never written | `powercfg /qh` before and after both real runs: lid AC and DC 1, sleep AC and DC 18000; plan list only Balanced after |
| Hand mutants on the changed core lines (scratch copy; one build job, one test thread; stop by PID at 4 GB, under 10 GB free, or 150 s) | 14 run, 14 killed in the end: 13 at once, M2 after the fix it showed (Log 11:06, 11:10). Peak test process 0.02 GB, none stopped. Windows-side lines: no mutants (the core holds the logic; the executor and journal lines have their own tests) |
| No tray started; build folder deleted | yes; scratch folder gone 11:11 |

### Open items

- For Haik's review: a hold that ran through the window with the lid closed runs unlocked until it ends (deliverable
  5 above). One line in the hook would lock then; left out, as the card says to keep what is built.
- For the orchestrator: Found outside scope 1 (the example file I changed), 3 (stale lines in architecture sections 5
  and 8), 5 (product rule 7 details).
- Not mine, seen: ten `%TEMP%\wakeshot-*-32688-*` test folders from a run at 04:16 (another card's), left as they are.
