# Review: Phase 1 design, power settings and facts

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only; no git, no registry, power or screen use.

Scope (narrow brief): paths where the user's power settings are not restored or are restored wrongly; holds with no
checked fact or that outlive it; facts with no source or that contradict a tested fact. Read: `docs/architecture.md`,
`docs/phases/01-design-skeleton.md` (summary, questions), `docs/product.md`, `docs/research.md`, `CLAUDE.md`,
`docs/tasks/p1-detailed-design.md`, `docs/research/distribution.md`, `docs/research/other-agent-signals.md`,
`docs/research/rivals-others.md` (traps), `docs/reviews/p0-architecture.md` and `p0-close-proposals.md` (guard and
panic items), spike logs `spikes/power-settings/out/copy-test-limited-20261103-021755.log` and `hold-20261103-131303.log`.
`research.md` changed while I read it (lid sound results); it is cited by section name, as the design does.

Counts: 0 blocker, 6 major, 13 minor. All are fixes to the design text; none needs a new test before sign-off.

## Major

### 1. Ownership checks can drop the wrong plan's entries, or entries we are about to write

Where: `architecture.md:139-140` (`Apply { plan, .. }`, `Release(Vec<SettingKey>)`), `:52` (`SettingKey` has no plan),
`:150` (`released` keys), `:177-179` (10 s read and plan switch as two rules with no order), `:360-364`.

What fails:

- Plan switch. Hold on Balanced; journal has Balanced `lidAc`, `lidDc`, `sleepDc`, written 0. The user picks another
  plan. The next read posts `Plan` with the new plan id and its own values (1, 1, 900). Both rules match: "differs from
  what we wrote: Release" and "plan switch: Restore, Apply". If Release runs first, Balanced's entries leave the
  journal. Balanced keeps Do nothing and Never forever: no journal, so no guard, RunOnce or restore at start.
- Stale read at a normal hold start. The safety tick reads the plan every 30 s even when nothing is owned (`:361-362`).
  A read taken on the IO thread just before an `Apply` it has not run yet reaches the core after the core issued the
  `Apply`. If the core's journal mirror counts entries from the moment it issues `Apply`, the old values "differ from
  what we wrote" and the core sends `Release`. The IO thread then runs `Apply` (writes 0) and `Release` (empties the
  journal). Same end state: overridden, no journal.
- Two plans in one journal (old plan's restore failed and is kept for retry, new plan applied): `Release(LidDc)`
  carries no plan, so it drops both plans' entries.

Fix: decide ownership on the IO thread, which owns the journal and runs commands in order: for each entry, read that
entry's own plan, compare with `written`, and post `UserChanged { plan, key }`. Make `Release` and `released` use
`(plan, key)`. Handle a changed active plan id before any release check. Add plan switches and reads in flight to the
property test generator (`:575`).

### 2. Read and apply errors count as "the user's" or as "done"

Where: restore routine `architecture.md:368-373`; `Event::Plan` `:129`; failure table `:543-554`; research, "Power
settings spike", "Stored at once, applied only on `PowerSetActiveScheme`".

What fails:

- Step 2: "plan gone, drop it; live equals written, write original; otherwise drop it". "Plan gone" is not defined, and
  a failed read has no branch. Coded as "anything but equal means drop", one failed read drops the entry, then step 4
  deletes the journal, the HKCU copy and RunOnce and stops the guard. The override stays for good.
- The 10 s read: `PlanValues` has no error case. A failed read that reaches the core as any value other than 0 is a
  "user change": `Release`, and the original is gone.
- `PowerSetActiveScheme` fails in step 3. Step 5 keeps the journal, but step 2 already stored the originals. On the
  retry each stored value equals `original`, not `written`, so step 2 drops it, nothing is written, step 3 does not
  run, and the journal is deleted. The applied (live) lid action stays Do nothing until something re-applies the plan
  or the PC restarts, with no journal left.

Fix: "plan gone" only when the GUID is missing from `PowerEnumerate`. Any read error is a failure: keep the entry, retry
on the tick. The 10 s read posts a failure as an error, never as values. Keep a "needs apply" flag in the journal; an
entry is done only when the stored value equals `original` and a `PowerSetActiveScheme` of the active plan succeeded
after that write.

### 3. The panic hook's restore races the IO thread and can remove every safety net

Where: `architecture.md:311`, `:554`; restore step 4 `:372`; Apply order `:354-356`.

What fails: the core runs on the loop thread, so a core panic runs the hook there while the IO thread keeps going. A bug
in handling a new hold is a likely trigger, and it comes right after the core issued `Apply`. The IO thread has written
the journal but not yet called `PowerWrite*`. The hook's restore sees each live value equal to `original`, not
`written`, drops every entry, deletes the journal, the HKCU copy and RunOnce, and sets `GuardStop`, so the guard exits.
The IO thread then writes Do nothing and Never and calls `PowerSetActiveScheme` before the abort ends the process.
Result: overridden, with no journal, no guard and no RunOnce. `Apply` never takes `Local\Wakeshot.Restore`, so that
mutex does not stop this.

Fix: the panic hook logs and aborts only. The guard already runs whenever anything is overridden and restores when the
tray dies; the next start covers the rest. If a hook restore is kept, `Apply` and every restore must take one shared
lock, and the hook waits for it with a timeout.

### 4. The guard is the tray's child again, so End task on the group likely ends both

Where: `architecture.md:268` (started by the tray), `:377` ("not a child in a job with the tray"); Phase 0 close item
15, applied (`reviews/p0-close-proposals.md:278-287`): "Start the guard detached, not as a child of the tray".
Summary `phases/01-design-skeleton.md:28-30` says the guard restores on "crash or End task".

What fails: "not in a job" is not "not a child". Task Manager lists child processes under their parent, and End task on
the group likely ends both (not verified; checklist 4 tests it). Then nothing restores until the next tray start or
sign-in. A laptop with lid Do nothing and battery sleep Never, closed in a bag, keeps running. The summary tells Haik
the opposite.

Fix: say how the guard starts outside the tray's process tree, for example `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS` with
the shell's Explorer as parent, or a short-lived launcher that exits (neither is verified for Task Manager grouping).
Keep checklist 4 and write the fallback if it fails. In the summary, mark End task "not tested".

### 5. A hold file without `procStart` can hold forever on a reused pid

Where: `architecture.md:469-471`, `:476`; decision 12.

What fails: "Wakeshot reads the start time when it first sees the file". If the tray first sees the file after its
writer died (tray not running, or a file left over from before a restart), the pid may already belong to another
process. Example: a nightly script started by Task Scheduler writes `holds\backup.json` with its pid and dies before
deleting it; after a restart that pid belongs to a long-lived process; at sign-in the tray records that process's start
time and holds while it lives, with the lid set to Do nothing. "Keeps it" is also not said to survive a tray restart.

Fix: with `procStart` missing, count the hold only if the process started before the file's last write time (a later
process cannot be the writer). Write the learned start time back into the file (atomic write), so a tray restart keeps
it.

### 6. `Apply`'s `PowerSetActiveScheme` can undo the user's plan switch

Where: `architecture.md:139` (`Apply { plan, .. }`), `:354-356` ("one `PowerSetActiveScheme`"), `:363-364`.

What fails: the target of that call is not specified. If it is `plan` and the user (or an OEM tool) switched plans while
the `Apply` waited in the IO queue, the call switches the user back to the old plan. Wakeshot never journals the active
plan, so nothing switches it back. "AC back: hold again" (`:187`) fires on an AC change, the same moment some OEM tools
switch plans (not verified for Lenovo).

Fix: right before writing, the IO thread reads the active plan. If it is not `plan`, it writes nothing and returns
`Done(PlanChanged)`. `PowerSetActiveScheme` always takes the GUID just read, never a stored one. Same rule for restore
step 3.

## Minor

### 7. `wakeshot restore` with a tray running writes the plan beside the tray's IO thread

Where: `architecture.md:285`.

What fails: the CLI and the tray's `Apply` share no lock. The CLI loads the journal, sees a key still at `original` (the
tray journaled it but has not written it yet), skips it; the tray then writes it. "Restore my power settings" leaves
that key overridden. "Tray running" comes from `state.json`; if that file is stale, the CLI runs the whole routine
(deletes the journal and RunOnce, sets `GuardStop`) while the tray lives.

Fix: with a live journal owner, the CLI only asks (a request file or a `config.json` field) and the tray restores on its
IO thread. Judge "tray running" by the journal owner's pid and start time, not by `state.json`.

### 8. No rule when a safety-net step fails; guard lifecycle gaps

Where: Apply order `:354-356`; failure table `:543-554`; `GuardStop` `:286`, `:372`, `:380`.

What fails: the table has no row for a failed journal write, HKCU copy, RunOnce value or guard start; decision 24 needs
"no `PowerWrite*` then". The reset type of `GuardStop` and who resets it are not written: a manual-reset event left set
(the tray holds a handle, so it persists) makes the next episode's guard exit at once. The tray does not watch the
guard, so a guard ended on its own is not restarted, and a later tray crash has no guard.

Fix: any failed step before `PowerWrite*` means no write, requests only, one notice. Give each guard its own event name
(with the guard's pid), or reset the event before each start. The tray waits on the guard's handle and restarts it while
anything is owned.

### 9. A failed restore outside the tray is silent, and in restore mode it is final

Where: `architecture.md:385-388`, `:491-494`, decision 38; research, "Phase 1 design checks", "RunOnce" (value deleted
before the run); `distribution.md:29-30` (hooks must not show UI).

What fails: the guard, restore mode and the hooks have no tray icon, so decision 38's warning never shows. With start at
login off, a restore-mode failure (for example the power service not ready at sign-in) is never retried: Windows already
deleted the value, and nothing runs until the user starts Wakeshot. The uninstall hook removes RunOnce and the HKCU copy
even when its restore failed; a later reinstall's hook may then apply that stale `journal.json` over values the user set
since.

Fix: restore mode retries for a few minutes; if it still fails, it starts the tray, which retries on each tick and warns.
The uninstall hook deletes `journal.json` only after a successful restore, and logs the failure.

### 10. Two users: plans are per machine, the journal and RunOnce are per user

Where: `architecture.md:272-275`, `:459`, `:385`.

What fails: if user A's session ends without a restore (A's tray and guard killed, or A logged off by B's shutdown while
switched out; Windows behavior there not verified), A's override stays until A signs in again. User B's tray reads Do
nothing as B's own value: no lock at lid close ("docked use"), no countdown, and the laptop runs in a bag. During fast
user switching, B's lid close is locked by nobody: A's tray locks A's own session.

Fix: a Question for Haik: document "one user per PC", or keep an owner marker every user can read in a machine-wide place
(for example `%ProgramData%\Wakeshot`; ACLs not verified), so any tray can tell a leftover override from a user's own
value.

### 11. `--dry-run` can delete the real journal

Where: `architecture.md:309`, `:312`, `:372`, `:588`.

What fails: startup step 5 runs in every mode, and `DryRunPower` only logs writes, but routine step 4 still deletes the
journal, the HKCU copy and RunOnce. A dry run started after a crash wipes the only record of a real override.

Fix: dry run uses its own data folder and never loads, writes or deletes the real journal or RunOnce.

### 12. Journal `seq` and version edge cases

Where: `architecture.md:368`, `:448-449`, `:481`, `:485`.

What fails: a deleted journal's `seq` is not kept. If a new journal restarts at 1 while a stale HKCU copy with 7 is left
(an earlier delete failed), step 1 picks the stale copy and ignores the current file. "A newer journal is left
untouched": a downgrade (winget can install an older version) leaves the override with nothing that restores it.

Fix: `seq` keeps growing across journals (for example wall time in ms). Freeze the entry fields so any version restores
the entries it understands.

### 13. The end-of-session restore waits behind the IO queue

Where: `architecture.md:293`, `:303-304`.

What fails: the IO thread runs commands in order, so queued notices, state and log writes run first. A balloon call
while Explorer shuts down may block (not verified), and the 4 s window can pass before the restore starts.

Fix: on `Session(Ending)` the IO thread runs the restore next and drops queued notices.

### 14. Liveness error mapping is not written down

Where: `architecture.md:65`, `:476`, `:312`, `:397`; research, "Reading elevated Claude Code sessions", "Gone vs no
access" (`0x80070057` gone, `0x80070005` denied); research, "Lid and sleep spike", "Lid notification".

What fails: other open errors and a failed process list call have no meaning in the design. If they count as "gone", a
transient error deletes a live `wakeshot run` hold file for good (the wrapper never rewrites it). For the journal owner,
"alive" on an error leaves a dead tray's override; "dead" restores under a live tray. For the hibernate check, the lid
callback may never come (Microsoft: "The callback won't be called until a lid device is found and its current state is
known").

Fix: write the mapping. Invalid parameter or denied: check the system process list. List fails: unknown; keep the last
state, log, never delete a file. Lid state unknown: do not hibernate.

### 15. Codex liveness is not tied to the session's process

Where: `architecture.md:501`, `:533-534`; `research/other-agent-signals.md:61-66`, `:79-83`, `:529-531`.

What fails: "a `codex.exe` is alive and the thread lock file exists". A crash leaves the lock file until the next Codex
start, and a shared `codex app-server` daemon can live for days. A session killed during a turn (last event
`task_started`) then holds while any `codex.exe` lives.

Fix: Phase 4 maps the lock file to the process holding it open (for example Restart Manager `RmGetList`; not verified
for this lock) and checks that pid and start time. Until then, mark the Codex rule "can outlive its session".

### 16. Facts with no source or no mark

Where and fix:

- `architecture.md:584-585`: "A write to an inactive plan changes only that plan (tested in the spikes)". No
  `research.md` line says so. The spike logs I read wrote to the copy while it was active (copy-test) or activated it at
  once (hold). Phase 3's test safety rests on this. Add the source or mark it "not tested", and make the first Phase 3
  test read the real plan before and after a write to the inactive copy.
- `:401-402`: "settings are already restored, so Windows' own timers apply". Research calls this inferred and not
  tested with the lid closed ("Lid and sleep spike", morning results, "Screen off is not suspend"). Mark it; point to
  checklist 2.
- `:390-391`: "a boot-time task needs admin". No source. Likely true; mark it.
- Apply and restore run at every hold start and end, which Question 8 says is each agent turn, each with a
  `PowerSetActiveScheme`. Nothing in research covers side effects of re-applying the active plan that often (other
  apps' setting callbacks, display brightness on some devices; not verified either way). Add a Phase 3 check: about 50
  apply and restore cycles, watching brightness and the System log.

### 17. Lid already closed, AC to DC: the lid action in force becomes ours with no lock

Where: `architecture.md:180-184`; `research/rivals-others.md:325-327` (trap 12, Windows not checked); research, "Lid
and sleep spike", morning results, "Hold with the lid closed" (unplug tested only with Do nothing on both AC and DC).

What fails: a common docked set-up is the user's own AC lid action Do nothing and DC Sleep. Wakeshot writes only lid DC.
Closed on the dock: the user's own action, no lock (right). The user unplugs the closed laptop and bags it: the action
in force is now ours, there is no lid event, so no sound and no lock, and the hold keeps it running unlocked. What
Windows does with the user's own values at that moment is not verified; if it would have slept, this lowers security.

Fix: also check the lock rule on a power source change while the lid is closed. Add the case to checklist 8 or a new
item.

### 18. The lid-close lock waits for a sound that the lock does not stop

Where: `architecture.md:180-183` ("The lock waits because ... in Phase 0's silent run the lock came first"); research,
"Lid-close sound", "Lid-close results" (added after the design, 15:16 to 15:20).

What fails: the new results say "neither the lock nor the closed lid stops or mutes a sound on this laptop": in close 1
the lock came 0.2 ms after `PlaySound` returned and the sound was heard. The wait now has no tested reason, and it
leaves the session unlocked with the lid closed for up to the cap (sound start 0.3 to 0.8 s plus its length).

Fix: lock at once after starting the sound; drop the wait and the cap, unless `p1-lid-sound` finds a reason on another
device.

### 19. Question 5 understates the gap

Where: `phases/01-design-skeleton.md:70-72`; `architecture.md:390-391`.

What fails: "nothing restores before the next sign-in" also follows End task on the group (finding 4), "Shut down
anyway" before the end-of-session restore ends, and a failed restore-mode run with start at login off (finding 9). Haik
answers Question 5 seeing only an OS crash and a power cut.

Fix: list every path in Question 5 and in the architecture's "Gap left".

## Checked and fine

- Tested facts the design follows: re-take after every resume as clear then set on the same handle (H4); battery sleep
  Never written also on AC, lid closed marked not tested; edits in place because Settings writes the real plan; no
  judging by `powercfg /requests` or `SystemExecutionState`; idle clamped at resume; settings restored before the
  hibernate countdown.
- Restore order across processes at sign-in (tray step 5, RunOnce, hooks) is serialized by `Local\Wakeshot.Restore`,
  and the "owner dead" check keeps restore mode off a live tray's journal.
- A crash mid-`Apply` with the guard alive restores correctly: entries not yet written fail the "live equals written"
  test and are dropped, which is right.

## Seen outside the brief

- `winget upgrade` runs Setup.exe silent, which does not start the app, and hooks cannot start it
  (`distribution.md:43-49`). After an upgrade Wakeshot is off until the next sign-in, so agent work may sleep. Neither
  the design nor the README plan says so.
- A tray that could not take `Global\Wakeshot.Settings` never retries (`architecture.md:272-274`); when the owner exits
  it stays requests-only, and on battery the battery rule then ends its requests.
