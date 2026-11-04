# Power safety check: p2-6-runonce-keep Part A

Reviewer: `wakeshot-reviewer`, 2026-11-04. Brief: the orchestrator's narrow check of commit 76a44cc (`keep_guard`
sets the RunOnce value again while this tray owns a known entry). Read: the card, the diff, `power/executor.rs`,
`power/journal.rs` (`FileNet`), core `ownership/apply.rs` and `ownership/restore.rs` (take-back, clean-up),
`core/settings.rs` (plan read cadence), `recovery.rs` (guard, restore mode), `wakeshot-cli/src/restore.rs`,
`hooks.rs` (restore, uninstall), `app/io_thread.rs` and `app/tray_mode.rs` (who owns the executor),
`docs/architecture.md` sections 4, 5 and 8, `docs/research.md` "Power track manual checks", the README's gap list.
No build and no test run (brief). One slip: I ran one read-only `git diff --stat` by mistake; it changed nothing.

Counts: 0 blockers, 0 majors, 6 minors.

## Findings

### 1. Minor: a dry-run hold now logs "dry run: RunOnce value not set" at every plan read

- Where: `crates/wakeshot-windows/src/power/journal.rs:546-549` (`FileNet::set_run_once`, `RunOnce::Off`), now
  called from `power/executor.rs:265` before every command while a dry-run journal is owned.
- What breaks: a dry-run hold writes one Info line every 10 s (about 360 an hour), the per-load noise that
  `journal.rs:373-375` (`unknown_logged`) was written to avoid. Caused by this change. The card lists it (Found
  outside scope 1) but leaves it.
- Fix: log the dry-run line once per `FileNet` (one bool field, like `unknown_logged`). Do it in this card's fix
  round: `journal.rs` is already touched.

### 2. Minor: a journal that does not read skips the value, so "back within one plan read" does not hold while reads fail

- Where: `power/executor.rs:251-263` (`JournalRead::Failed` gives `owned = false` and returns before
  `set_run_once`); the new Gap left paragraph `docs/architecture.md:933-935`; test `no_safety_net_when_nothing_owned`
  (`executor.rs:1317-1331`) has no unreadable start, and no executor test uses `FakeJournal::Unreadable`.
- What breaks: the sign-in pass deletes the value; the tray's next plan reads find `journal.json` unreadable (a
  scanner holding it, a changed ACL); each one skips the value, so it stays missing for as long as reads fail, not
  10 s. A crash, a power cut or End task on both in that time leaves nothing for the next sign-in. The doc names a
  failed write as the only longer case. Likelihood low: such reads fail rarely and briefly. Not tested.
- Fix, lean: add "or while journal.json does not read" to the Gap left line, and an `Unreadable` start to
  `no_safety_net_when_nothing_owned` so the behavior is pinned. Stronger: keep the last answer in a bool on the
  executor and, on a failed read after an owned one, still set the value (a stale value is harmless: restore mode
  finds no journal or a live owner). Change to the stronger one when a log shows reads failing past one plan read.

### 3. Minor: the README's gap list is now behind architecture's "Gap left"

- Where: `README.md:401-404`; `docs/architecture.md:919` says "the README says so", and `:933-935` adds a case.
- What breaks: the README does not list the new case (crash, power cut or End task on both in the up to 10 s after
  the sign-in's RunOnce pass, or while the value cannot be written back), so the user-facing list and the design
  differ. `README.md` has uncommitted edits in the working tree (not from this commit), so merge with those.
- Fix: one bullet under "Until Wakeshot next starts", for example: "Windows crashes, the power is cut, or End task
  ends both, in the few seconds after sign-in between Windows using the one-time sign-in entry and Wakeshot setting it
  again."

### 4. Minor: "a value that is there is only read" is not quite what the code does

- Where: `docs/architecture.md:912`; code `power/journal.rs:557-563`.
- What breaks: a value that is there but differs (another exe path, or a value that does not read) is overwritten.
  The executor doc has it right ("writes only a missing or wrong value", `executor.rs:70`). Doc precision only.
- Fix: "a value that is there and right is only read".

### 5. Minor: architecture section 8's row promises a notice that `keep_guard` never gives

- Where: `docs/architecture.md:1188` ("Journal load, lock or write, RunOnce value or guard start fails ... one
  notice"); `executor.rs:273-279` only logs a Warn.
- What breaks: a RunOnce write that fails before a plan read gives a Warn line per command and no notice, so the user
  is not told the sign-in restore is missing. This matches the card's Decision 4 and is already its Found outside
  scope 2; listed so the fix round does not drop it.
- Fix: the card's wording, "(in an apply; before each command they only log)".

### 6. Minor, pre-existing: a leftover of another owner that this tray cannot restore has no RunOnce value

- Where: `executor.rs:251-263` (the owner must be `self.me`); reason in the doc at `executor.rs:241-243`.
- Scenario: the previous session's tray crashed. At sign-in, this tray's start and the restore mode run by the
  RunOnce pass both fail to restore it, or its owner's liveness is Unknown. Windows used the value in that pass. This
  tray retries on each tick but sets no value and starts no guard for that journal. If this tray then crashes or is
  ended together with its guard, and start at login is off, nothing restores until Wakeshot next starts. Not caused
  by this change. While a hold of this tray owns entries, the journal always names this tray (apply step 3 takes it
  over, `ownership/apply.rs:144-162`), so the brief's case is closed.
- Fix, optional: set the value for any journal with a known entry, whatever its owner, and keep the guard for this
  tray's own journal only. Restore mode checks liveness, so a value for a live or Unknown owner costs only a restore
  mode start that exits. Or list the case in Gap left. Haik's call; likelihood very low (a restore failing for
  minutes, then a crash).

## Answers to the brief

1. **Does it close the gap?** Yes, on every path where this tray's hold owns entries and the journal reads. While
   anything is owned the core sends `ReadPlan` every 10 s (`core/settings.rs:40`, `plan_read_due` at `:209`), and
   `ReadPlan`, `Apply` and `Restore` each run `keep_guard` first (`executor.rs:137-147`), which sets a missing value
   (`journal.rs:557-567`). Per path:
   - Dry run: never has a value, by design (`RunOnce::Off`). Only the log noise of finding 1.
   - Another owner's journal: a hold of this tray takes the journal over at apply step 3, which sets the value, so it
     cannot own entries in a journal that names another owner. Leftovers it does not own: finding 6.
   - Unknown entries only: nothing this version can restore, and restore mode would only clean up
     (`restore.rs:66-73`), so no value is right. Known plus unknown entries: set.
   - Journal does not read: skipped while reads fail (finding 2).
   - Failed write: a Warn line, retried on the next command (10 s); listed in Gap left.
   - Quit, end-of-session restore: both are `Restore` commands on the same IO thread, so `keep_guard` puts the
     value back before the restore starts. If the session end cuts the restore short, the value is there. Better
     than before.
2. **Bad value, value with nothing owned, race with a clean restore?** No.
   - Bad time: the value runs only at the next sign-in's pass. Restore mode then restores only a journal whose owner
     is Gone, exits for a live owner (`recovery.rs:756`), and never restores on Unknown. A value with no journal ends
     in "no journal, nothing to restore". There is no bad time.
   - Nothing owned: `keep_guard` writes only after a read that names this tray with a known entry
     (`executor.rs:251-263`).
   - The tray's own clean restore: it runs on the same IO thread after `keep_guard` (`executor.rs:145-147`), so the
     order is always read, set, restore, clean-up; the next command reads Missing and writes nothing.
   - Outside the lock: it does not matter. The only bad order is: (a) `keep_guard` reads a journal naming this tray;
     (b) another process takes the lock, restores, deletes the journal and clears the value; (c) `keep_guard` writes
     the value (and starts a guard, as it already did before this change). Step (b) needs another process to restore
     the journal of a tray that still runs, and none does: the guard acts only after the tray's process ends
     (`recovery.rs:741`); restore mode, the CLI (`wakeshot-cli/src/restore.rs:117-127`) and the hooks
     (`hooks.rs:621-641`) restore only when the owner is Gone; Velopack kills the tray before its hooks (research
     line 227). Even then the end state is a value with no journal, which the next sign-in reads as nothing to do
     (walk R6). This assumes one tray per user (Windows' default of one session per user; the `Local\` mutexes are
     per session). Not verified for a PC set to allow two sessions per user.
3. **Do the tests prove it?** Mostly.
   - `run_once_set_again_while_owned` (`executor.rs:1123`) fails if the call is missing, comes after
     `ensure_guard`, is skipped for any of the four commands, makes the command fail, or stops the guard start after
     a failed write. It checks the Info line only when the value was missing.
   - `no_safety_net_when_nothing_owned` (`:1317`) fails if the owner test or the known-entry test is dropped (another
     owner's leftover; only unknown entries with owner `ME`; no journal; corrupt).
   - `run_once_comes_back_with_the_real_net` (`:1213`) shows the real `FileNet` reads first, writes only a missing
     value, and that nothing comes back after a clean restore.
   - `plan_read_stop_points` (`:1263`) shows a stop at any call of a plan read keeps the entries and the user's
     originals, and the next start restores.
   - Gaps: no unreadable-journal test (finding 2); no test for the dry-run line (finding 1); the outside-the-lock order
     is argued by reading, not tested (it cannot happen while the tray lives, so a test would only show the R6 end
     state).
   - Not verified by me: that the tests pass, and the card's mutation check (no cargo run, per the brief).
4. **Are the changed `architecture.md` lines true?** Mostly.
   - "Logon restore" (`:905-916`) matches the code and research (39.0 and 64.6 s; restore mode's start and owner check
     verified, its restore at sign-in not yet). One wording slip: finding 4.
   - The new Gap left paragraph (`:933-935`) is true but leaves out the unreadable journal (finding 2). The README and
     section 8 now lag it (findings 3 and 5).
   - Code docs `executor.rs:69-72`, `:241-248` and `journal.rs:347-349` match the code.

## Checked, no finding

- Apply's take-back is unaffected: `keep_guard` sets the value only when the journal holds a known entry, and apply
  takes back only what it added when the journal held none (`ownership/apply.rs:87`, `:151`, `:164`).
- The uninstall hook removes the value (`hooks.rs:201`) after Velopack has killed the tray, so a live tray cannot put
  it back. If a Medium Update.exe could not kill an elevated tray (not tested, research "Elevation" under the
  Velopack checks), the tray would set the value again within 10 s, naming the old exe; at the next sign-in that
  either fails to start (harmless) or restores a gone tray's journal (wanted).
- Explorer's pass: research shows it ends in 17 ms and the tray writes up to 10 s later, so restore mode does not run
  twice in one pass. Whether a write inside those 17 ms is picked up: not verified; harmless either way (restore mode
  exits for a live owner).
- Session end: one registry read, and maybe a write, now come before the end-of-session restore, inside its 4 s
  wait. Not measured; registry calls take well under that.
